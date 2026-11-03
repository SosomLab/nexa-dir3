//! 전송 작업 큐(T-202 ④ 슬라이스 2 · docs/26 §12.2~12.4 · 사용자 결정 11-01 ①④): 전송 중 새 전송은 **큐에 추가**(`transfer.queue` on ·
//! off = 종전 거부 `ops.busy`) · 워커 1개(순차 · 머리 작업만 `App::transfer`로 돈다) · 충돌은 실행 차례에 인라인 질문(대기 중엔 묻지 않음).
//!
//! - 저장 = 설정 폴더 `queue/<id>.job.json`(JobSpec JSON · cdylib과 같은 인코딩) + `<id>.journal`(ndir-xfer `journal` 키 → started/done 줄) ·
//!   Done/Failed/Canceled = 두 파일 삭제(결과는 토스트 · 로그) · 폴더 `None`(시험 · 둘째 인스턴스) = 메모리만(실제 설정 폴더를 건드리지 않는다).
//! - **UTF-8이 아닌 경로가 든 작업은 저장하지 않는다**(`to_string_lossy`라 저장본 경로가 달라진다) → 메모리 큐에서만 · 재시작 복원 제외 · 토스트 1회.
//! - `cut`(붙인 뒤 클립보드 비우기)은 저장하지 않는다(재시작 뒤 클립보드는 없다).
//! - **재시작 복원**(슬라이스 3 · §12.2 · §12.4 · 사용자 ③ "물어보고 재개"): 기동 때 `owner.lock`(pid + 기동 시각 · `platform::procid`)으로 소유를
//!   판정 — 다른 인스턴스가 살아 있으면 **메모리 큐만**(`set_dir(None)`) · 아니면 인수하고 남은 `*.job.json`을 `transfer.queue_restore`(ask · auto ·
//!   discard)대로: 재개 = 같은 id · `resume` = `journal` = `<id>.journal`(끊긴 덮어쓰기 사본은 질문에 "중단된 사본" 줄 · ndir-xfer `ask_interrupted`).

use crate::app::dialogs::DlgReply;
use crate::dlg_win::DlgSpec;
use crate::platform::procid::{process_alive, process_start_time};
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

/// `owner.lock` 판정.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Claim {
    /// 우리 것(없었거나 · 죽었거나 · pid가 재사용됐거나 · 우리 자신).
    Owned,
    /// 다른 인스턴스가 살아 있다(pid) — 이 인스턴스는 메모리 큐만.
    OtherAlive(u32),
}

const OWNER: &str = "owner.lock";

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

    /// 저장 폴더가 없다(시험 · 둘째 인스턴스) — 작업은 메모리에만.
    pub(crate) fn memory_only(&self) -> bool {
        self.dir.is_none()
    }

    /// 소유 표시(§12.2): `owner.lock` = "pid\n기동시각\n". 적힌 pid가 우리가 아니고 살아 있고 기동 시각이 같으면(시각을 모르는 OS · 0 = pid만)
    /// 다른 인스턴스 → `OtherAlive` · 아니면 우리 것으로 다시 쓴다. 폴더 없음 = Owned(메모리).
    pub(crate) fn claim(&mut self) -> Claim {
        let Some(dir) = &self.dir else {
            return Claim::Owned;
        };
        let me = std::process::id();
        if let Some((pid, start)) = read_owner(dir) {
            if pid != me && owner_alive(pid, start) {
                return Claim::OtherAlive(pid);
            }
        }
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(
            dir.join(OWNER),
            format!("{me}\n{}\n", process_start_time(me).unwrap_or(0)),
        );
        Claim::Owned
    }

    /// 종료 때 — 우리 pid의 잠금만 지운다(남의 것은 건드리지 않는다).
    pub(crate) fn release_owner(&self) {
        if let Some(dir) = &self.dir {
            if read_owner(dir).is_some_and(|(pid, _)| pid == std::process::id()) {
                let _ = std::fs::remove_file(dir.join(OWNER));
            }
        }
    }

    /// 지난 실행이 남긴 작업 파일(`<id>.job.json` · id 순). 못 읽는 파일은 지운다(우리 형식 · 깨졌으면 복원할 수 없다).
    pub(crate) fn leftovers(&self) -> Vec<(u64, JobSpec)> {
        let Some(dir) = &self.dir else {
            return Vec::new();
        };
        let Ok(rd) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for e in rd.flatten() {
            let name = e.file_name();
            let Some(id) = name
                .to_str()
                .and_then(|n| n.strip_suffix(".job.json"))
                .and_then(|s| s.parse::<u64>().ok())
            else {
                continue;
            };
            match std::fs::read_to_string(e.path())
                .ok()
                .and_then(|t| JobSpec::from_json(&t).ok())
            {
                Some(spec) => out.push((id, spec)),
                None => self.forget(id),
            }
        }
        out.sort_by_key(|(id, _)| *id);
        out
    }

    /// 복원 등록(같은 id · 작업 파일은 이미 있음): 저널이 있으면 `resume` = `journal` = 그 파일(이어 적는다).
    pub(crate) fn push_restored(&mut self, id: u64, mut spec: JobSpec) {
        if let Some(dir) = &self.dir {
            let j = journal_path(dir, id);
            spec.resume = j.exists().then(|| j.clone());
            spec.journal = Some(j);
        }
        self.last_id = self.last_id.max(id);
        self.pending.push_back(QueuedJob {
            id,
            spec,
            cut: false,
        });
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

fn read_owner(dir: &Path) -> Option<(u32, u64)> {
    let text = std::fs::read_to_string(dir.join(OWNER)).ok()?;
    let mut it = text.lines();
    let pid = it.next()?.trim().parse().ok()?;
    let start = it.next().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    Some((pid, start))
}

/// 적힌 소유자가 지금도 그 프로세스인가 — 살아 있고(`kill 0`/`OpenProcess`) 기동 시각이 같을 때(모르면 pid만).
fn owner_alive(pid: u32, start: u64) -> bool {
    process_alive(pid)
        && match (start, process_start_time(pid)) {
            (0, _) | (_, None) => true,
            (s, Some(now)) => s == now,
        }
}

impl crate::App {
    /// 기동 때(설정 폴더가 정해진 뒤 · T3는 직접 호출 · docs/26 §12.4): 소유 판정 → 남은 작업 → `transfer.queue_restore`.
    pub(crate) fn queue_restore_startup(&mut self) {
        if let Claim::OtherAlive(_) = self.transfer_queue.claim() {
            // 둘째 인스턴스(사용자 11-01): 저장하지 않고 메모리 큐만 · 상태줄 안내.
            self.transfer_queue.set_dir(None);
            // 상태줄 안내(유지 시간 · GAP-034) + 토스트(상태줄뿐이면 못 볼 수 있다 · 협업 실측 11-01).
            self.status_note(&tr("ops.queueOtherInstance"));
            let a = self.active;
            self.toast_panel(
                a,
                crate::toast::ToastKind::Info,
                tr("ops.progressTitle"),
                tr("ops.queueOtherInstance"),
            );
            return;
        }
        let left = self.transfer_queue.leftovers();
        if left.is_empty() {
            return;
        }
        match self.settings.get("transfer.queue_restore").unwrap_or("ask") {
            "discard" => self.queue_discard(left),
            "auto" => self.queue_resume(left),
            _ => {
                let n = left.len().to_string();
                let spec = DlgSpec {
                    title: tr("ops.queueRestoreTitle"),
                    text: trf("ops.queueRestore", &[&n]),
                    buttons: vec![(1, tr("ops.queueResume")), (2, tr("ops.queueDiscard"))],
                    default: 1,
                    cancel: 0, // Esc = 그대로 둔다(다음 기동에 다시 묻는다)
                    input: None,
                    delay: None,
                    arm: None,
                };
                let _ = self.ask(spec, DlgReply::QueueRestore(left));
            }
        }
    }

    /// 재개(대화상자 1 · `auto`): 같은 id로 다시 큐에 → 머리 실행 · 토스트 "재개 n건".
    pub(crate) fn queue_resume(&mut self, jobs: Vec<(u64, JobSpec)>) {
        let n = jobs.len().to_string();
        for (id, spec) in jobs {
            self.transfer_queue.push_restored(id, spec);
        }
        let a = self.active;
        self.toast_panel(
            a,
            crate::toast::ToastKind::Info,
            tr("ops.progressTitle"),
            trf("ops.queueResumed", &[&n]),
        );
        self.pump_queue();
        self.sync_queue_view();
    }

    /// 버림(대화상자 2 · `discard`): 작업 파일 · 저널 삭제 · 상태줄.
    pub(crate) fn queue_discard(&mut self, jobs: Vec<(u64, JobSpec)>) {
        for (id, _) in &jobs {
            self.transfer_queue.forget(*id);
        }
        self.status_note(&trf("ops.queueDiscarded", &[&jobs.len().to_string()]));
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

    /// 소유 표시 · 복원 자료(슬라이스 3): 없는 잠금/죽은 pid = 인수(우리 pid + 기동 시각 기록) · 우리 자신 = 인수 · 남은 작업 = id 순 ·
    /// 깨진 파일 = 삭제 · `push_restored` = 같은 id · 저널 있으면 resume/journal · `release_owner` = 우리 잠금만.
    #[test]
    fn claim_leftovers_and_restore() {
        let d = std::env::temp_dir().join(format!("ndir-queue-own-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let qd = d.join("queue");
        std::fs::create_dir_all(&qd).unwrap();
        std::fs::write(qd.join(OWNER), format!("{}\n0\n", u32::MAX - 7)).unwrap();
        let mut q = TransferQueue::new(Some(qd.clone()));
        assert_eq!(q.claim(), Claim::Owned, "죽은 pid = 인수");
        let me = std::process::id();
        assert_eq!(read_owner(&qd).map(|(p, _)| p), Some(me));
        assert_eq!(q.claim(), Claim::Owned, "우리 자신 = 인수");
        // 남은 작업: 9(깨짐 → 삭제) · 20 · 10 → id 순.
        std::fs::write(qd.join("20.job.json"), spec(1, &d).to_json()).unwrap();
        std::fs::write(qd.join("10.job.json"), spec(2, &d).to_json()).unwrap();
        std::fs::write(qd.join("9.job.json"), "not json").unwrap();
        std::fs::write(qd.join("10.journal"), "").unwrap();
        let left = q.leftovers();
        assert_eq!(
            left.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![10, 20]
        );
        assert!(!qd.join("9.job.json").exists(), "깨진 파일 삭제");
        for (id, s) in left {
            q.push_restored(id, s);
        }
        assert_eq!(q.ids(), vec![10, 20]);
        assert_eq!(
            q.pending[0].spec.resume.as_deref(),
            Some(qd.join("10.journal").as_path())
        );
        assert_eq!(
            q.pending[0].spec.journal.as_deref(),
            Some(qd.join("10.journal").as_path())
        );
        assert!(q.pending[1].spec.resume.is_none() && q.pending[1].spec.journal.is_some());
        let (next, _) = q.push(spec(1, &d), false);
        assert!(next > 20, "복원 id 뒤로 이어짐");
        q.release_owner();
        assert!(!qd.join(OWNER).exists());
        // 남의 잠금은 두고 간다.
        std::fs::write(qd.join(OWNER), "1\n0\n").unwrap();
        q.release_owner();
        assert!(qd.join(OWNER).exists());
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
