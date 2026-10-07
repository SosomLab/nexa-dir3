//! App — 폴더 비교 · 동기화(T-171/172 · NEW-042/043 · dir3 신규 · 사용자 10-05): 두 패널일 때 배경 우클릭 "반대 패널과 폴더
//! 비교…" → 작업 스레드(`ndir_ops::compare::compare`) → 비교 창 → [→]/[←]/[⇄] = 계획(`plan`) → 복사 워커(`apply_copies` · 전송
//! 진행 창 재사용 · 전송/풀기와 같은 슬롯) → 끝나면 다시 비교. 미러의 "대상에만 있는 것 삭제"는 1차 미포함(계획에는 있다).

use crate::*;
use ndir_ops::compare::{self as cmpx, CmpEntry, CmpOpts, CmpProgress, Direction};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

type CmpOutcome = std::io::Result<Vec<CmpEntry>>;

#[derive(Debug, Default)]
pub(crate) struct CmpShared {
    cancel: AtomicBool,
    progress: CmpProgress,
    finished: AtomicBool,
    result: Mutex<Option<CmpOutcome>>,
}

#[derive(Debug)]
pub(crate) struct CmpJob {
    shared: Arc<CmpShared>,
    by_content: bool,
}

type SyncOutcome = std::io::Result<Vec<(PathBuf, String)>>;

#[derive(Debug, Default)]
pub(crate) struct SyncShared {
    cancel: AtomicBool,
    done: AtomicU64,
    total: AtomicU64,
    finished: AtomicBool,
    result: Mutex<Option<SyncOutcome>>,
}

#[derive(Debug)]
pub(crate) struct SyncJob {
    shared: Arc<SyncShared>,
    count: usize,
}

impl App {
    /// 비교 시작(활성 패널 = 왼쪽 · 반대 패널 = 오른쪽). 진행 중이면 거부.
    pub(crate) fn start_compare(&mut self, left: &Path, right: &Path, by_content: bool) {
        if self.cmp_job.is_some() {
            return;
        }
        let shared = Arc::new(CmpShared::default());
        let sh = Arc::clone(&shared);
        let (l, r) = (left.to_path_buf(), right.to_path_buf());
        let opts = CmpOpts {
            by_content,
            ..CmpOpts::default()
        };
        std::thread::spawn(move || {
            let res = cmpx::compare(&l, &r, opts, &sh.progress, &sh.cancel);
            if let Ok(mut slot) = sh.result.lock() {
                *slot = Some(res);
            }
            sh.finished.store(true, Ordering::Relaxed);
        });
        self.cmp_job = Some(CmpJob { shared, by_content });
        self.compare_win
            .start(left, right, app::bulk::local_tz_min());
        self.open_compare = true;
    }

    /// 배경 메뉴 `ctx.compare_panels`.
    pub(crate) fn compare_panels(&mut self) {
        if !self.dual {
            return;
        }
        let left = self.panels[self.active].root_path();
        let right = self.panels[1 - self.active].root_path();
        if ndir_vfs::is_virtual_root(&left) || ndir_vfs::is_virtual_root(&right) {
            return;
        }
        self.start_compare(&left, &right, false);
    }

    /// 폴링 — 비교 진행/완료 + 동기화 진행/완료. 돌려주는 값 = 어느 쪽이든 진행 중.
    pub(crate) fn compare_tick(&mut self) -> bool {
        let mut live = false;
        if let Some(job) = &self.cmp_job {
            let p = &job.shared.progress;
            self.compare_win.set_progress(
                p.scanned.load(Ordering::Relaxed),
                p.hashed.load(Ordering::Relaxed),
            );
            if job.shared.finished.load(Ordering::Relaxed) {
                let job = self.cmp_job.take().expect("checked above");
                self.mem_after_job();
                let result = job.shared.result.lock().ok().and_then(|mut r| r.take());
                match result {
                    Some(Ok(entries)) => self.compare_win.set_result(entries),
                    Some(Err(e)) if e.kind() == std::io::ErrorKind::Interrupted => {
                        self.compare_win.set_error(&tr("ops.canceled"));
                    }
                    Some(Err(e)) => self
                        .compare_win
                        .set_error(&trf("cmp.failed", &[&e.to_string()])),
                    None => self.compare_win.set_error(&tr("cmp.failed")),
                }
                let _ = job.by_content;
            } else {
                live = true;
            }
        }
        if let Some(job) = &self.sync_job {
            let (done, total) = (
                job.shared.done.load(Ordering::Relaxed),
                job.shared.total.load(Ordering::Relaxed).max(1),
            );
            if self.progress_win.is_active() {
                self.progress_win
                    .update(done.min(total), total, Vec::new(), job.count, job.count);
                if self.progress_win.take_cancelled() {
                    job.shared.cancel.store(true, Ordering::Relaxed);
                }
            }
            if job.shared.finished.load(Ordering::Relaxed) {
                let job = self.sync_job.take().expect("checked above");
                self.mem_after_job();
                let result = job.shared.result.lock().ok().and_then(|mut r| r.take());
                let (text, warn) = match result {
                    Some(Ok(failed)) if failed.is_empty() => {
                        (trf("cmp.syncDone", &[&job.count.to_string()]), false)
                    }
                    Some(Ok(failed)) => (
                        trf(
                            "cmp.syncPartial",
                            &[
                                &(job.count - failed.len()).to_string(),
                                &failed.len().to_string(),
                            ],
                        ),
                        true,
                    ),
                    Some(Err(e)) if e.kind() == std::io::ErrorKind::Interrupted => {
                        (tr("ops.canceled"), true)
                    }
                    Some(Err(e)) => (trf("cmp.failed", &[&e.to_string()]), true),
                    None => (tr("cmp.failed"), true),
                };
                if self.progress_win.is_active() {
                    let ms = self.settings.int("transfer.close_ms").max(0) as u64;
                    let now = self.started.elapsed().as_millis() as u64;
                    self.progress_win.set_done(&text, ms, now);
                }
                self.dirsizes.clear();
                self.git_detail.clear(); // git 요약 — 작업으로 바뀌었다(10-06).
                let mut inv = Invalidations::default();
                for p in &mut self.panels {
                    p.reopen(&mut inv);
                }
                self.toasts.push(
                    if warn {
                        toast::ToastKind::Warn
                    } else {
                        toast::ToastKind::Info
                    },
                    tr("cmp.title"),
                    text.clone(),
                );
                self.statusbar.set_left(&text, &mut inv);
                self.update_status();
                // 끝났으면 다시 비교해 창을 최신으로.
                if self.compare_win.is_open() {
                    let (l, r) = (
                        self.compare_win.left.clone(),
                        self.compare_win.right.clone(),
                    );
                    self.start_compare(&l, &r, false);
                }
                self.redraw();
            } else {
                live = true;
            }
        }
        live
    }

    /// 창의 동기화 버튼 — 계획을 세워 복사 워커를 시작한다(전송 · 풀기 · 다른 동기화 중이면 거부).
    pub(crate) fn compare_sync(&mut self, dir: Direction, only: Vec<String>) {
        if self.transfer.is_some() || self.extract_job.is_some() || self.sync_job.is_some() {
            let mut inv = Invalidations::default();
            self.statusbar.set_left(&tr("ops.busy"), &mut inv);
            self.redraw();
            return;
        }
        let Some(entries) = self.compare_win_entries() else {
            return;
        };
        let (l, r) = (
            self.compare_win.left.clone(),
            self.compare_win.right.clone(),
        );
        let only_ref = (!only.is_empty()).then_some(only.as_slice());
        let plan = cmpx::plan(&entries, &l, &r, dir, false, only_ref);
        if plan.copies.is_empty() {
            let mut inv = Invalidations::default();
            self.statusbar.set_left(&tr("cmp.nothing"), &mut inv);
            self.redraw();
            return;
        }
        let shared = Arc::new(SyncShared::default());
        shared.total.store(plan.bytes, Ordering::Relaxed);
        let sh = Arc::clone(&shared);
        let count = plan.copies.len();
        std::thread::spawn(move || {
            let res = cmpx::apply_copies(
                &plan,
                &mut |n| {
                    sh.done.fetch_add(n, Ordering::Relaxed);
                },
                &sh.cancel,
            );
            if let Ok(mut slot) = sh.result.lock() {
                *slot = Some(res);
            }
            sh.finished.store(true, Ordering::Relaxed);
        });
        let label = trf("cmp.syncLabel", &[&count.to_string()]);
        if self.settings.int("transfer.close_ms") > 0 {
            self.progress_win.reset(&label);
            self.progress_win.update(
                0,
                plan_bytes_floor(shared.total.load(Ordering::Relaxed)),
                Vec::new(),
                0,
                count,
            );
            self.open_progress = true;
        }
        let mut inv = Invalidations::default();
        self.statusbar.set_left(&label, &mut inv);
        self.sync_job = Some(SyncJob { shared, count });
        self.redraw();
    }

    fn compare_win_entries(&self) -> Option<Vec<CmpEntry>> {
        self.compare_win.entries()
    }

    pub(crate) fn close_compare(&mut self) {
        if let Some(job) = self.cmp_job.take() {
            job.shared.cancel.store(true, Ordering::Relaxed);
        }
        self.compare_win.close();
        if let Some(w) = &self.window {
            w.focus_window();
        }
    }
}

fn plan_bytes_floor(b: u64) -> u64 {
    b.max(1)
}
