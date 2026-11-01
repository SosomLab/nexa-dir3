//! 전송 작업 큐(T-202 ④ 슬라이스 2 · docs/26 §12.2~12.4 · 사용자 결정 11-01 ①④): 전송 중 새 전송은 **큐에 추가**(`transfer.queue` on ·
//! off = 종전 거부 `ops.busy`) · 워커 1개(순차 · 머리 작업만 `App::transfer`로 돈다) · 충돌은 실행 차례에 인라인 질문(대기 중엔 묻지 않음).
//!
//! - 저장 = 설정 폴더 `queue/<id>.job.json`(JobSpec JSON · cdylib과 같은 인코딩) + `<id>.journal`(ndir-xfer `journal` 키 → started/done 줄) ·
//!   Done/Failed/Canceled = 두 파일 삭제(결과는 토스트 · 로그) · 폴더 `None`(시험 · 둘째 인스턴스) = 메모리만(실제 설정 폴더를 건드리지 않는다).
//! - **UTF-8이 아닌 경로가 든 작업은 저장하지 않는다**(`to_string_lossy`라 저장본 경로가 달라진다) → 메모리 큐에서만 · 재시작 복원 제외 · 토스트 1회.
//! - `cut`(붙인 뒤 클립보드 비우기)은 저장하지 않는다(재시작 뒤 클립보드는 없다). 재시작 복원 · `owner.lock` · `transfer.queue_restore` = 슬라이스 3.

use ndir_i18n::{tr, trf};
use ndir_ops::Op;
use ndir_xfer::JobSpec;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 대기 중인 작업 1건.
#[derive(Debug)]
pub(crate) struct QueuedJob {
    pub id: u64,
    pub spec: JobSpec,
    /// 끝난 뒤 앱 내 클립보드를 비운다(잘라내기 → 붙여넣기).
    pub cut: bool,
}

/// `push`가 작업을 어디에 두었나.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Saved {
    /// `queue/<id>.job.json` + 저널 경로 지정.
    File,
    /// 저장 폴더 없음(시험 · 둘째 인스턴스).
    MemoryOnly,
    /// 경로가 UTF-8이 아니라 저장하지 않음(메모리만 · 안내 1회).
    NotUtf8,
}

/// 전송 대기 큐 — 머리 1건은 호출자(`App::pump_queue`)가 꺼내 돌린다.
#[derive(Debug)]
pub(crate) struct TransferQueue {
    pending: VecDeque<QueuedJob>,
    dir: Option<PathBuf>,
    last_id: u64,
}

impl TransferQueue {
    pub(crate) fn new(dir: Option<PathBuf>) -> Self {
        TransferQueue {
            pending: VecDeque::new(),
            dir,
            last_id: 0,
        }
    }

    /// 저장 폴더(설정 폴더 `queue/`) — 기동 뒤 설정 폴더가 정해지면 한 번.
    pub(crate) fn set_dir(&mut self, dir: Option<PathBuf>) {
        self.dir = dir;
    }

    pub(crate) fn len(&self) -> usize {
        self.pending.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// 대기 순서대로 id.
    pub(crate) fn ids(&self) -> Vec<u64> {
        self.pending.iter().map(|j| j.id).collect()
    }

    /// 진행 창 대기 줄 라벨(순서대로): "복사: 3 → /대상" 꼴(`ops.queueRow`).
    pub(crate) fn labels(&self) -> Vec<String> {
        self.pending
            .iter()
            .map(|j| {
                let kind = tr(if j.spec.op == Op::Move {
                    "menu.edit.cut"
                } else {
                    "menu.edit.copy"
                });
                trf(
                    "ops.queueRow",
                    &[
                        &kind,
                        &j.spec.sources.len().to_string(),
                        &j.spec.dest.display().to_string(),
                    ],
                )
            })
            .collect()
    }

    /// 뒤에 붙인다 — 저장 폴더가 있고 경로가 전부 UTF-8이면 `<id>.job.json`을 쓰고 `spec.journal`을 `<id>.journal`로 둔다.
    pub(crate) fn push(&mut self, mut spec: JobSpec, cut: bool) -> (u64, Saved) {
        let id = self.next_id();
        let saved = match &self.dir {
            None => Saved::MemoryOnly,
            Some(_) if !utf8_paths(&spec) => Saved::NotUtf8,
            Some(dir) => {
                spec.journal = Some(journal_path(dir, id));
                match std::fs::create_dir_all(dir)
                    .and_then(|()| std::fs::write(job_path(dir, id), spec.to_json()))
                {
                    Ok(()) => Saved::File,
                    Err(_) => {
                        // 못 쓰면 메모리만(저널도 두지 않는다 — 열지 못하는 저널은 작업을 막는다).
                        spec.journal = None;
                        Saved::MemoryOnly
                    }
                }
            }
        };
        self.pending.push_back(QueuedJob { id, spec, cut });
        (id, saved)
    }

    /// 머리를 꺼낸다(실행용 · 파일은 끝난 뒤 `forget`).
    pub(crate) fn pop_front(&mut self) -> Option<QueuedJob> {
        self.pending.pop_front()
    }

    /// 대기 중인 작업을 지운다(파일도).
    pub(crate) fn remove(&mut self, id: u64) -> bool {
        let Some(i) = self.pending.iter().position(|j| j.id == id) else {
            return false;
        };
        self.pending.remove(i);
        self.forget(id);
        true
    }

    pub(crate) fn move_up(&mut self, id: u64) -> bool {
        match self.pending.iter().position(|j| j.id == id) {
            Some(i) if i > 0 => {
                self.pending.swap(i, i - 1);
                true
            }
            _ => false,
        }
    }

    pub(crate) fn move_down(&mut self, id: u64) -> bool {
        match self.pending.iter().position(|j| j.id == id) {
            Some(i) if i + 1 < self.pending.len() => {
                self.pending.swap(i, i + 1);
                true
            }
            _ => false,
        }
    }

    /// 끝난(또는 지운) 작업의 파일 두 개를 치운다 — 없으면 그만.
    pub(crate) fn forget(&self, id: u64) {
        if let Some(dir) = &self.dir {
            let _ = std::fs::remove_file(job_path(dir, id));
            let _ = std::fs::remove_file(journal_path(dir, id));
        }
    }

    /// id = 기동 뒤 단조 증가(밀리초 시각 기반 · 같은 ms면 +1) — 파일 이름 · 재시작 복원 열쇠.
    fn next_id(&mut self) -> u64 {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let id = ms.max(self.last_id + 1);
        self.last_id = id;
        id
    }
}

fn job_path(dir: &Path, id: u64) -> PathBuf {
    dir.join(format!("{id}.job.json"))
}

fn journal_path(dir: &Path, id: u64) -> PathBuf {
    dir.join(format!("{id}.journal"))
}

/// 저장해도 경로가 달라지지 않는가(원본 · 대상 전부 UTF-8).
fn utf8_paths(spec: &JobSpec) -> bool {
    spec.dest.to_str().is_some() && spec.sources.iter().all(|p| p.to_str().is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(n: usize, dest: &Path) -> JobSpec {
        JobSpec {
            sources: (0..n).map(|i| dest.join(format!("s{i}"))).collect(),
            dest: dest.to_path_buf(),
            ..JobSpec::default()
        }
    }

    /// 저장 폴더가 있으면 `<id>.job.json` + 저널 경로 · 순서 편집(위/아래/삭제) · `forget` = 파일 삭제 · 폴더 없음 = 메모리만.
    #[test]
    fn saves_edits_and_forgets() {
        ndir_i18n::activate(ndir_i18n::load("en", Path::new("nowhere")));
        let d = std::env::temp_dir().join(format!("ndir-queue-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut q = TransferQueue::new(Some(d.join("queue")));
        assert!(q.is_empty());
        let (a, sa) = q.push(spec(2, &d), false);
        let (b, sb) = q.push(spec(1, &d), true);
        let (c, _) = q.push(spec(3, &d), false);
        assert_eq!((sa, sb), (Saved::File, Saved::File));
        assert!(a < b && b < c, "단조 증가 id");
        assert!(d.join("queue").join(format!("{a}.job.json")).is_file());
        let text = std::fs::read_to_string(job_path(&d.join("queue"), b)).unwrap();
        assert!(
            text.contains("\"op\":\"copy\"") || text.contains("\"op\": \"copy\""),
            "{text}"
        );
        assert_eq!(
            q.pending[0].spec.journal.as_deref(),
            Some(journal_path(&d.join("queue"), a).as_path())
        );
        assert_eq!(q.ids(), vec![a, b, c]);
        assert!(q.move_up(b) && q.ids() == vec![b, a, c]);
        assert!(!q.move_up(b), "맨 앞은 더 못 올림");
        assert!(q.move_down(b) && q.ids() == vec![a, b, c]);
        assert!(!q.move_down(c));
        assert!(q.remove(b) && q.ids() == vec![a, c]);
        assert!(!job_path(&d.join("queue"), b).exists(), "삭제 = 파일도");
        assert!(!q.remove(b));
        let labels = q.labels();
        assert_eq!(labels.len(), 2);
        assert!(
            labels[0].contains('2') && labels[0].contains(&d.display().to_string()),
            "{labels:?}"
        );
        let head = q.pop_front().unwrap();
        assert_eq!((head.id, head.cut), (a, false));
        q.forget(a);
        assert!(!job_path(&d.join("queue"), a).exists());
        // 폴더 없음 = 메모리만.
        let mut m = TransferQueue::new(None);
        let (_, s) = m.push(spec(1, &d), false);
        assert_eq!(s, Saved::MemoryOnly);
        assert!(m.pending[0].spec.journal.is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// UTF-8이 아닌 경로 = 저장하지 않음(메모리만 · 저널 없음).
    #[cfg(unix)]
    #[test]
    fn non_utf8_paths_stay_in_memory() {
        use std::os::unix::ffi::OsStringExt;
        let d = std::env::temp_dir().join(format!("ndir-queue-u8-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mut q = TransferQueue::new(Some(d.join("queue")));
        let bad = PathBuf::from(std::ffi::OsString::from_vec(vec![b'/', b't', 0xff, 0xfe]));
        let s = JobSpec {
            sources: vec![bad],
            dest: d.clone(),
            ..JobSpec::default()
        };
        let (id, saved) = q.push(s, false);
        assert_eq!(saved, Saved::NotUtf8);
        assert!(!job_path(&d.join("queue"), id).exists());
        assert!(q.pending[0].spec.journal.is_none());
        let _ = std::fs::remove_dir_all(&d);
    }
}
