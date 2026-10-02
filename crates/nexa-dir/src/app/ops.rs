//! 파일 작업 배선(M6 A · dir2 `win.rs` `do_clip`/`paste_dest`/`start_transfer`/undo 축약 · docs/port/22 OPS-001~039).
//!
//! - **클립보드**: `edit.copy`/`edit.cut` = 선택 경로를 OS 파일 클립보드(FileClipboard 포트 · Windows CF_HDROP)에 쓰고 **앱 내 사본**도 둔다
//!   (포트 `Unsupported`인 OS는 앱 내 사본만 — T-52/53 뒤 OS 클립보드로). `edit.paste` = OS 클립보드 우선 · 없으면 앱 내 사본.
//! - **전송**: 대상 = 선택한 폴더 1개 또는 활성 패널 폴더(dir2 `paste_dest`) · `ndir_ops::transfer`를 **작업 스레드**에서(진행 = 공유 원자값 ·
//!   틱마다 상태줄 `ops.progress`) · 완료 = 두 패널 재열람 + 토스트(`ops.done/skipped/errors/canceled`) + 히스토리 기록(이동 = `MoveBatchOp` ·
//!   복사 = `CopyBatchOp` + 휴지통 삭제 주입). 동시 1건(`ops.busy`).
//! - **충돌**: 확인 창(T-29 nexa-dlg)이 오기 전까지 **건너뜀**(`Conflict::Skip` — 조용히 덮어쓰지 않는다 · 건너뛴 수는 토스트에) ⚠ OPS-004.
//! - **undo/redo**: `OperationHistory`(세션 한정 100) · 결과 문구 `undo.done/fail` · `history.*`.

use crate::platform::PlatformError;
use crate::*;
use ndir_ops::history::{CopyBatchOp, MoveBatchOp, OpError, OperationHistory};
use ndir_ops::{Conflict, Event, Op, Outcome};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// 전송 중 공유 상태(작업 스레드 ↔ UI 틱).
pub(crate) struct TransferShared {
    pub cancel: AtomicBool,
    pub done: AtomicU64,
    pub total: AtomicU64,
    pub outcome: Mutex<Option<Outcome>>,
}

/// 진행 중인 전송 1건.
pub(crate) struct TransferJob {
    pub shared: Arc<TransferShared>,
    pub op: Op,
    pub cut: bool,
    pub count: usize,
}

/// 전송 폴링 간격(ms).
pub(crate) const OPS_POLL_MS: u64 = 100;

fn op_error_text(e: &OpError) -> String {
    match e {
        OpError::Failed(n) => trf("history.failedItems", &[&n.to_string()]),
        OpError::MissingSource(s) => trf("history.missingSource", &[s]),
        OpError::NameExists(s) => trf("history.nameExists", &[s]),
    }
}

impl App {
    /// 붙여넣을 원본(OS 파일 클립보드 우선 · 없으면 앱 내 사본) → (경로, 잘라내기).
    pub(crate) fn clip_sources(&self) -> Option<(Vec<PathBuf>, bool)> {
        self.platform
            .clipboard
            .read_files()
            .filter(|(p, _)| !p.is_empty())
            .or_else(|| self.clip.clone())
    }

    /// `edit.copy`/`edit.cut` — 선택 없으면 무동작(클립보드 유지 · dir2 M3-5).
    pub(crate) fn clip_write(&mut self, cut: bool) {
        let paths = self.panels[self.active].selected_paths();
        if paths.is_empty() {
            return;
        }
        match self.platform.clipboard.write_files(&paths, cut) {
            Ok(()) | Err(PlatformError::Unsupported(_)) => {}
            Err(e) => {
                self.toasts
                    .push(toast::ToastKind::Warn, tr("menu.edit.copy"), e.to_string());
            }
        }
        let n = paths.len().to_string();
        self.clip = Some((paths, cut));
        let mut inv = Invalidations::default();
        self.statusbar.set_left(
            &trf(if cut { "clip.cut" } else { "clip.copied" }, &[&n]),
            &mut inv,
        );
        self.redraw();
    }

    /// 붙여넣기 대상(dir2 `paste_dest`): 선택 1개가 폴더면 그 폴더 · 파일이면 부모 · 그 외 활성 패널 폴더.
    pub(crate) fn paste_dest(&self) -> PathBuf {
        let panel = &self.panels[self.active];
        let sel = panel.selected_paths();
        if let [one] = &sel[..] {
            if one.is_dir() {
                return one.clone();
            }
            if let Some(parent) = one.parent() {
                if parent.is_dir() {
                    return parent.to_path_buf();
                }
            }
        }
        panel.root_path()
    }

    /// `edit.paste`.
    pub(crate) fn paste(&mut self) {
        if self.transfer.is_some() {
            self.status_note(&tr("ops.busy"));
            return;
        }
        let Some((sources, cut)) = self.clip_sources() else {
            self.status_note(&tr("clip.empty"));
            return;
        };
        let dest = self.paste_dest();
        if !dest.is_dir() {
            return;
        }
        let op = if cut { Op::Move } else { Op::Copy };
        self.start_transfer(sources, dest, op, cut);
    }

    fn status_note(&mut self, text: &str) {
        let mut inv = Invalidations::default();
        self.statusbar.set_left(text, &mut inv);
        self.redraw();
    }

    /// 전송 시작(작업 스레드) — 모든 복사/이동 진입점(붙여넣기 · DnD T-51 B)이 여기로.
    pub(crate) fn start_transfer(
        &mut self,
        sources: Vec<PathBuf>,
        dest: PathBuf,
        op: Op,
        cut: bool,
    ) {
        if sources.is_empty() {
            return;
        }
        if self.transfer.is_some() {
            self.status_note(&tr("ops.busy"));
            return;
        }
        let shared = Arc::new(TransferShared {
            cancel: AtomicBool::new(false),
            done: AtomicU64::new(0),
            total: AtomicU64::new(0),
            outcome: Mutex::new(None),
        });
        let sh = Arc::clone(&shared);
        let count = sources.len();
        std::thread::spawn(move || {
            let out = ndir_ops::transfer(
                &sources,
                &dest,
                op,
                // 충돌 확인 창(T-29) 전까지 = 건너뜀(조용한 덮어쓰기 금지).
                &mut |_| Conflict::Skip,
                &mut |ev| match ev {
                    Event::Plan { total_bytes, .. } => {
                        sh.total.store(total_bytes, Ordering::Relaxed)
                    }
                    Event::Bytes(p) => sh.done.store(p.done_bytes, Ordering::Relaxed),
                    _ => {}
                },
                &sh.cancel,
            );
            if let Ok(mut slot) = sh.outcome.lock() {
                *slot = Some(out);
            }
        });
        self.transfer = Some(TransferJob {
            shared,
            op,
            cut,
            count,
        });
        self.status_note(&trf("ops.progress", &["0"]));
    }

    /// 전송 취소 요청(진행 중일 때).
    pub(crate) fn cancel_transfer(&mut self) {
        if let Some(job) = &self.transfer {
            job.shared.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// 전송 폴링(틱): 진행률 상태줄 · 완료 수거. 반환 = 아직 진행 중.
    pub(crate) fn ops_tick(&mut self) -> bool {
        let Some(job) = &self.transfer else {
            return false;
        };
        let outcome = job.shared.outcome.lock().ok().and_then(|mut s| s.take());
        match outcome {
            Some(out) => {
                let job = self.transfer.take().expect("checked above");
                self.finish_transfer(job, out);
                false
            }
            None => {
                let (done, total) = (
                    job.shared.done.load(Ordering::Relaxed),
                    job.shared.total.load(Ordering::Relaxed),
                );
                let pct = (done.min(total) * 100)
                    .checked_div(total)
                    .unwrap_or(0)
                    .to_string();
                let mut inv = Invalidations::default();
                self.statusbar
                    .set_left(&trf("ops.progress", &[&pct]), &mut inv);
                self.redraw();
                true
            }
        }
    }

    /// 완료: 재열람 · 히스토리 · 토스트 · 잘라내기면 앱 내 클립보드 비움.
    fn finish_transfer(&mut self, job: TransferJob, out: Outcome) {
        let mut inv = Invalidations::default();
        for p in &mut self.panels {
            p.reopen(&mut inv);
        }
        if !out.transferred.is_empty() {
            let n = out.transferred.len().to_string();
            match job.op {
                Op::Move => self.history.push(Box::new(MoveBatchOp::new(
                    out.transferred.clone(),
                    trf("ops.done", &[&n]),
                ))),
                Op::Copy => {
                    let trash = Rc::clone(&self.platform.trash);
                    self.history.push(Box::new(CopyBatchOp::new(
                        out.transferred.clone(),
                        trf("ops.done", &[&n]),
                        Box::new(move |p: &std::path::Path| {
                            trash
                                .trash(std::slice::from_ref(&p.to_path_buf()))
                                .map(|_| ())
                                .map_err(|e| std::io::Error::other(e.to_string()))
                        }),
                    )));
                }
            }
        }
        if job.cut && job.op == Op::Move {
            self.clip = None;
        }
        let mut parts = vec![trf("ops.done", &[&out.transferred.len().to_string()])];
        if !out.skipped.is_empty() {
            parts.push(trf("ops.skipped", &[&out.skipped.len().to_string()]));
        }
        if !out.errors.is_empty() {
            parts.push(trf("ops.errors", &[&out.errors.len().to_string()]));
        }
        if out.canceled {
            parts.push(tr("ops.canceled"));
        }
        let kind = if out.errors.is_empty() {
            toast::ToastKind::Info
        } else {
            toast::ToastKind::Warn
        };
        let title = tr(if job.op == Op::Move {
            "menu.edit.cut"
        } else {
            "menu.edit.copy"
        });
        self.toasts.push(kind, title, parts.join(" · "));
        self.statusbar.set_left(&parts.join(" · "), &mut inv);
        self.update_status();
        self.redraw();
        let _ = job.count;
    }

    /// `edit.undo`/`edit.redo`.
    pub(crate) fn history_step(&mut self, redo: bool) {
        if self.transfer.is_some() {
            self.status_note(&tr("ops.busy"));
            return;
        }
        let desc = if redo {
            self.history.redo_description()
        } else {
            self.history.undo_description()
        }
        .map(str::to_string)
        .unwrap_or_default();
        let result = if redo {
            self.history.redo()
        } else {
            self.history.undo()
        };
        let text = match result {
            None => tr(if redo { "redo.none" } else { "undo.none" }),
            Some(Ok(())) => trf(if redo { "redo.done" } else { "undo.done" }, &[&desc]),
            Some(Err(e)) => trf(
                if redo { "redo.fail" } else { "undo.fail" },
                &[&desc, &op_error_text(&e)],
            ),
        };
        let mut inv = Invalidations::default();
        for p in &mut self.panels {
            p.reopen(&mut inv);
        }
        self.statusbar.set_left(&text, &mut inv);
        self.update_status();
        self.redraw();
    }

    /// 덤프 `ops`: 전송 상태 · undo/redo 가능 여부 · 클립보드.
    pub(crate) fn ops_dump(&self) -> String {
        let transfer = match &self.transfer {
            Some(j) => format!(
                "running {}/{} op {:?}",
                j.shared.done.load(Ordering::Relaxed),
                j.shared.total.load(Ordering::Relaxed),
                j.op
            ),
            None => "idle".into(),
        };
        let clip = match &self.clip {
            Some((p, cut)) => format!("{} {}", p.len(), if *cut { "cut" } else { "copy" }),
            None => "none".into(),
        };
        format!(
            "transfer {transfer}\nundo {} {}\nredo {}\nclip {clip}\n",
            self.history.can_undo(),
            self.history.undo_description().unwrap_or(""),
            self.history.can_redo()
        )
    }

    /// 히스토리 접근(시험).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn history(&self) -> &OperationHistory {
        &self.history
    }
}
