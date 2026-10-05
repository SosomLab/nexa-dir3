//! App — 중복 파일 찾기(T-170 · NEW-041 · dir3 신규 · 사용자 10-05): 우클릭 "중복 파일 찾기…"(배경 = 현재 폴더 · 행 = 선택한
//! 폴더들) → 작업 스레드(`ndir_ops::dupes::find` · 크기 → 앞 64 KiB CRC32 → SHA-256) → 결과 창(`dupes_win.rs`) → [휴지통으로
//! 보내기] = `trash_checked`(잠금 확인 · 실행 취소 · 토스트 그대로). 작업 슬롯 1개(진행 중이면 거부) · 폴링 = `OPS_POLL_MS` 틱.

use crate::*;
use ndir_ops::dupes::{self as dp, DupGroup, DupOpts, DupProgress};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

type DupOutcome = std::io::Result<(Vec<DupGroup>, Vec<Vec<i64>>)>;

#[derive(Debug, Default)]
pub(crate) struct DupShared {
    cancel: AtomicBool,
    progress: DupProgress,
    finished: AtomicBool,
    result: Mutex<Option<DupOutcome>>,
}

#[derive(Debug)]
pub(crate) struct DupJob {
    shared: Arc<DupShared>,
}

/// 경로의 수정 시각(초 · 모르면 0).
fn mtime_secs(p: &Path) -> i64 {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs() as i64)
}

impl App {
    /// 검색 시작 — `roots` = 폴더(또는 파일)들. 진행 중이면 거부.
    pub(crate) fn start_dupes(&mut self, roots: Vec<PathBuf>) {
        if roots.is_empty() {
            return;
        }
        if self.dup_job.is_some() {
            let mut inv = Invalidations::default();
            self.statusbar.set_left(&tr("dupes.busy"), &mut inv);
            self.redraw();
            return;
        }
        let shared = Arc::new(DupShared::default());
        let sh = Arc::clone(&shared);
        std::thread::spawn(move || {
            let r = dp::find(&roots, DupOpts::default(), &sh.progress, &sh.cancel).map(|groups| {
                let mtimes = groups
                    .iter()
                    .map(|g| g.paths.iter().map(|p| mtime_secs(p)).collect())
                    .collect();
                (groups, mtimes)
            });
            if let Ok(mut slot) = sh.result.lock() {
                *slot = Some(r);
            }
            sh.finished.store(true, Ordering::Relaxed);
        });
        self.dup_job = Some(DupJob { shared });
        self.dupes_win.start(app::bulk::local_tz_min());
        self.open_dupes = true;
    }

    /// 폴링 — 진행 글 · 완료 수거. 돌려주는 값 = 아직 진행 중.
    pub(crate) fn dupes_tick(&mut self) -> bool {
        let Some(job) = &self.dup_job else {
            return false;
        };
        let p = &job.shared.progress;
        self.dupes_win.set_progress(
            p.scanned.load(Ordering::Relaxed),
            p.candidates.load(Ordering::Relaxed),
            p.hashed.load(Ordering::Relaxed),
        );
        if !job.shared.finished.load(Ordering::Relaxed) {
            return true;
        }
        let job = self.dup_job.take().expect("checked above");
        let result = job.shared.result.lock().ok().and_then(|mut r| r.take());
        match result {
            Some(Ok((groups, mtimes))) => self.dupes_win.set_result(groups, mtimes),
            Some(Err(e)) if e.kind() == std::io::ErrorKind::Interrupted => {
                self.dupes_win.set_error(&tr("ops.canceled"));
            }
            Some(Err(e)) => self
                .dupes_win
                .set_error(&trf("dupes.failed", &[&e.to_string()])),
            None => self.dupes_win.set_error(&tr("dupes.failed")),
        }
        false
    }

    /// 창 닫기 — 진행 중이면 취소.
    pub(crate) fn close_dupes(&mut self) {
        if let Some(job) = self.dup_job.take() {
            job.shared.cancel.store(true, Ordering::Relaxed);
        }
        self.dupes_win.close();
        if let Some(w) = &self.window {
            w.focus_window();
        }
    }

    /// [휴지통으로 보내기] — 표시한 파일들을 휴지통으로(잠금 확인 · 실행 취소는 `trash_checked`) · 목록에서 뺀다.
    pub(crate) fn dupes_trash(&mut self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }
        self.trash_checked(paths.clone());
        let gone: Vec<PathBuf> = paths.into_iter().filter(|p| !p.exists()).collect();
        self.dupes_win.drop_paths(&gone);
        self.dirsizes.clear();
    }
}
