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
use ndir_ops::{Conflict, Event, ItemStatus, Op, Outcome};
use nexa_ctl::{SegItem, SegStatus};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};

/// 전송 중 공유 상태(작업 스레드 ↔ UI 틱).
pub(crate) struct TransferShared {
    pub cancel: AtomicBool,
    pub done: AtomicU64,
    pub total: AtomicU64,
    pub outcome: Mutex<Option<Outcome>>,
    /// 항목별 세그먼트(계획 뒤 채워짐 · 진행 창 바 · dir2 OPS-216).
    pub items: Mutex<Vec<SegItem>>,
    /// 현재 항목 번호(1부터 · 0 = 아직).
    pub current: AtomicUsize,
}

/// 충돌 결정(UI → 작업 스레드).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConflictChoice {
    Overwrite,
    OverwriteAll,
    Skip,
    Cancel,
}

/// 작업 스레드의 충돌 질문: (대상 경로, 회신 채널).
pub(crate) type ConflictReq = (PathBuf, mpsc::Sender<ConflictChoice>);

/// 진행 중인 전송 1건.
pub(crate) struct TransferJob {
    pub shared: Arc<TransferShared>,
    pub conflict_rx: mpsc::Receiver<ConflictReq>,
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

/// 새 항목 종류(SHELL-008).
#[derive(Debug, Clone)]
pub(crate) enum NewKind {
    Folder,
    File,
    Template(platform::NewTemplate),
}

/// 템플릿 원천을 `dest`에 만든다(빈 파일 · 복사 · 바이트) — 이미 있으면 실패(`create_new`).
fn materialize(dest: &std::path::Path, src: &platform::TemplateSource) -> std::io::Result<()> {
    match src {
        platform::TemplateSource::Empty => std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dest)
            .map(|_| ()),
        platform::TemplateSource::Copy(from) => {
            if dest.exists() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "exists",
                ));
            }
            std::fs::copy(from, dest).map(|_| ())
        }
        platform::TemplateSource::Data(bytes) => {
            use std::io::Write as _;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(dest)?;
            f.write_all(bytes)
        }
    }
}

/// 결과 안내의 "건너뜀" 수(순수): 엔진이 건너뛴 항목 + **취소로 손대지 못한 항목**(전체 − 전송 − 건너뜀 − 실패). 취소가 아니면
/// 엔진 값 그대로(사용자 10-04 "취소하면 건너뛴 파일은 전체가 되어야 한다").
pub(crate) fn skipped_total(count: usize, out: &Outcome) -> usize {
    let handled = out.transferred.len() + out.skipped.len() + out.errors.len();
    if out.canceled {
        out.skipped.len() + count.saturating_sub(handled)
    } else {
        out.skipped.len()
    }
}

/// 결과 안내 조각(토스트 · 상태줄 — ` · `로 잇는다): **전체 n** · 전송 a · 건너뜀 b · 실패 c · 취소됨. 전체 대상 수를 맨 앞에 두어
/// 숫자의 합이 맞는지 한눈에 보인다(사용자 10-04 "전체 대상이 포함되도록") · 건너뜀/실패는 있을 때만 · 취소는 끝에.
pub(crate) fn result_parts(count: usize, out: &Outcome) -> Vec<String> {
    let mut parts = vec![
        trf("ops.total", &[&count.to_string()]),
        trf("ops.done", &[&out.transferred.len().to_string()]),
    ];
    let skipped = skipped_total(count, out);
    if skipped > 0 {
        parts.push(trf("ops.skipped", &[&skipped.to_string()]));
    }
    if !out.errors.is_empty() {
        parts.push(trf("ops.errors", &[&out.errors.len().to_string()]));
    }
    if out.canceled {
        parts.push(tr("ops.canceled"));
    }
    parts
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
        self.sync_cut_marks();
        let mut inv = Invalidations::default();
        self.statusbar.set_left(
            &trf(if cut { "clip.cut" } else { "clip.copied" }, &[&n]),
            &mut inv,
        );
        self.redraw();
    }

    /// 잘라내기 흐림 동기(dir2 SHELL-044 `sync_cut_marks` — WM_CLIPBOARDUPDATE 대신 잘라내기/복사/붙여넣기 완료/창 포커스 때):
    /// 클립보드(OS 우선 · 앱 사본)가 **잘라내기** 파일 목록이면 그 경로 집합, 아니면 빈 집합 → 양 패널 전 탭 · 바뀐 목록만 다시 그린다.
    pub(crate) fn sync_cut_marks(&mut self) {
        let marks: std::collections::HashSet<PathBuf> = match self.clip_sources() {
            Some((paths, true)) => paths.into_iter().collect(),
            _ => std::collections::HashSet::new(),
        };
        let mut inv = Invalidations::default();
        for p in &mut self.panels {
            p.set_cut_marks(&marks, &mut inv);
        }
        if !inv.is_empty() {
            self.redraw();
        }
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
        if self.transfer.is_some() || self.extract_job.is_some() || self.sync_job.is_some() {
            self.status_note(&tr("ops.busy"));
            return;
        }
        let shared = Arc::new(TransferShared {
            cancel: AtomicBool::new(false),
            done: AtomicU64::new(0),
            total: AtomicU64::new(0),
            outcome: Mutex::new(None),
            items: Mutex::new(Vec::new()),
            current: AtomicUsize::new(0),
        });
        // 진행 창(dir2 DLG-059: `transfer.close_ms > 0`일 때만).
        if self.settings.int("transfer.close_ms") > 0 {
            self.progress_win.reset(&tr("ops.progressLabel"));
            self.open_progress = true;
        }
        let sh = Arc::clone(&shared);
        let count = sources.len();
        let (req_tx, conflict_rx) = mpsc::channel::<ConflictReq>();
        std::thread::spawn(move || {
            // 충돌 = UI 대화상자 4버튼(dir2 QA 07-14 개정): "모두 덮어쓰기"만 이후 무확인 · 취소 = 전체 중단 + 그 항목 건너뜀.
            let mut decided: Option<Conflict> = None;
            let sh2 = Arc::clone(&sh);
            let out = ndir_ops::transfer(
                &sources,
                &dest,
                op,
                &mut |p: &std::path::Path| {
                    if let Some(c) = decided {
                        return c;
                    }
                    let (tx, rx) = mpsc::channel();
                    if req_tx.send((p.to_path_buf(), tx)).is_err() {
                        return Conflict::Skip;
                    }
                    match rx.recv() {
                        Ok(ConflictChoice::Overwrite) => Conflict::Overwrite,
                        Ok(ConflictChoice::OverwriteAll) => {
                            decided = Some(Conflict::Overwrite);
                            Conflict::Overwrite
                        }
                        Ok(ConflictChoice::Skip) => Conflict::Skip,
                        Ok(ConflictChoice::Cancel) | Err(_) => {
                            sh2.cancel.store(true, Ordering::Relaxed);
                            Conflict::Skip
                        }
                    }
                },
                &mut |ev| match ev {
                    Event::Plan { sizes, total_bytes } => {
                        sh.total.store(total_bytes, Ordering::Relaxed);
                        if let Ok(mut it) = sh.items.lock() {
                            *it = sizes
                                .iter()
                                .map(|&size| SegItem {
                                    size,
                                    done: 0,
                                    status: SegStatus::Pending,
                                })
                                .collect();
                        }
                    }
                    Event::ItemStart { index, .. } => {
                        sh.current.store(index + 1, Ordering::Relaxed);
                        if let Ok(mut it) = sh.items.lock() {
                            if let Some(i) = it.get_mut(index) {
                                i.status = SegStatus::Active;
                            }
                        }
                    }
                    Event::Bytes(p) => {
                        sh.done.store(p.done_bytes, Ordering::Relaxed);
                        // 항목 내 진행 = 전체 누적 − 앞 항목 크기 합(세그먼트 부분 채움).
                        if let Ok(mut it) = sh.items.lock() {
                            let before: u64 = it.iter().take(p.item_index).map(|i| i.size).sum();
                            if let Some(i) = it.get_mut(p.item_index) {
                                i.done = p.done_bytes.saturating_sub(before).min(i.size);
                            }
                        }
                    }
                    Event::ItemEnd { index, status } => {
                        if let Ok(mut it) = sh.items.lock() {
                            if let Some(i) = it.get_mut(index) {
                                i.status = match status {
                                    ItemStatus::Done => SegStatus::Done,
                                    ItemStatus::Skipped => SegStatus::Skipped,
                                    ItemStatus::Failed => SegStatus::Failed,
                                };
                                if i.status == SegStatus::Done {
                                    i.done = i.size;
                                }
                            }
                        }
                    }
                },
                &sh.cancel,
            );
            if let Ok(mut slot) = sh.outcome.lock() {
                *slot = Some(out);
            }
        });
        self.transfer = Some(TransferJob {
            shared,
            conflict_rx,
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

    /// 진행 창에 지금 진행 상태(바이트 · 항목별 상태 · 몇 번째 파일)를 넣는다 — 바뀐 것이 있을 때만 다시 그린다.
    fn sync_progress_win(&mut self) {
        let Some(job) = &self.transfer else {
            return;
        };
        let items = job
            .shared
            .items
            .lock()
            .map(|v| v.clone())
            .unwrap_or_default();
        self.progress_win.update(
            job.shared.done.load(Ordering::Relaxed),
            job.shared.total.load(Ordering::Relaxed),
            items,
            job.shared.current.load(Ordering::Relaxed),
            job.count,
        );
    }

    /// 전송 폴링(틱): 진행률 상태줄 · 완료 수거. 반환 = 아직 진행 중.
    pub(crate) fn ops_tick(&mut self) -> bool {
        // 진행 창 스냅숏은 **질문을 띄우기 전에도 · 답을 기다리는 동안에도** 맞춘다 — 종전에는 덮어쓰기 질문이 떠 있으면 건너뛰어,
        // 앞 항목의 결과(덮어씀 = 채움 · 건너뜀 = 진한 회색)와 "파일 n/m"이 다음 질문 뒤에야 반영됐다(사용자 10-04).
        self.sync_progress_win();
        let Some(job) = &self.transfer else {
            return false;
        };
        // 진행 창 안에서 묻던 덮어쓰기 질문의 답 → 워커에 회신(창이 닫혔으면 취소로 본다).
        if self.conflict_inline.is_some() {
            let choice = match self.progress_win.take_conflict_choice() {
                Some(1) => Some(ConflictChoice::Overwrite),
                Some(2) => Some(ConflictChoice::OverwriteAll),
                Some(3) => Some(ConflictChoice::Skip),
                Some(_) => Some(ConflictChoice::Cancel),
                None if !self.progress_win.conflict_pending() => Some(ConflictChoice::Cancel),
                None => None,
            };
            let Some(choice) = choice else {
                return true; // 답을 기다린다
            };
            if let Some(tx) = self.conflict_inline.take() {
                let _ = tx.send(choice);
            }
            self.progress_win.set_conflict(None);
            return true;
        }
        // 충돌 질문 수거 → 진행 창 안 질문 또는 대화상자(동시 1건 · 작업 스레드는 회신까지 대기).
        if let Ok((path, tx)) = job.conflict_rx.try_recv() {
            self.conflict_ask(&path, tx);
            return true;
        }
        let outcome = job.shared.outcome.lock().ok().and_then(|mut s| s.take());
        match outcome {
            Some(out) => {
                let job = self.transfer.take().expect("checked above");
                self.finish_transfer(job, out);
                self.mem_after_job();
                false
            }
            None => {
                let (done, total) = (
                    job.shared.done.load(Ordering::Relaxed),
                    job.shared.total.load(Ordering::Relaxed),
                );
                // 진행 창(dir2 DLG-059/062): [취소]/X 폴링 → 워커 취소 플래그(스냅숏은 틱 맨 앞에서 맞췄다).
                if self.progress_win.take_cancelled() {
                    job.shared.cancel.store(true, Ordering::Relaxed);
                }
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
        // 진행 창 마감(DLG-061): 결과 한 줄 + [닫기 (N)] 카운트다운(`transfer.close_ms` · 진행값은 그대로).
        if self.progress_win.is_active() {
            let ms = self.settings.int("transfer.close_ms").max(0) as u64;
            let now = self.started.elapsed().as_millis() as u64;
            let mut items = job
                .shared
                .items
                .lock()
                .map(|v| v.clone())
                .unwrap_or_default();
            // 취소 = 손대지 못한 항목(아직 미처리 · 하던 중)도 "건너뜀"으로 결정된 것이다 — 밝은 회색(미처리)으로 남기지 않는다.
            if out.canceled {
                for i in &mut items {
                    if matches!(i.status, SegStatus::Pending | SegStatus::Active) {
                        i.status = SegStatus::Skipped;
                    }
                }
            }
            self.progress_win.update(
                job.shared.done.load(Ordering::Relaxed),
                job.shared.total.load(Ordering::Relaxed),
                items,
                job.count,
                job.count,
            );
            self.progress_win.set_done(&tr("ops.doneClosing"), ms, now);
        }
        let mut inv = Invalidations::default();
        self.dirsizes.clear(); // 폴더 크기 캐시(T-166) — 내용이 바뀌었다.
        self.git_detail.clear(); // git 요약 — 작업으로 바뀌었다(10-06).
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
        self.sync_cut_marks();
        let parts = result_parts(job.count, &out);
        self.log_with(
            if out.errors.is_empty() {
                ndir_log::LogKind::Ops
            } else {
                ndir_log::LogKind::Error
            },
            parts.join(" \u{00B7} "),
            Some(out.transferred.len() as u64),
            None,
        );
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
        self.dirsizes.clear(); // 폴더 크기 캐시(T-166) — 내용이 바뀌었다.
        self.git_detail.clear(); // git 요약 — 작업으로 바뀌었다(10-06).
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

    /// 새 폴더/새 파일(dir2 `create_new` · OPS-018 · CreateOp): 활성 폴더에 `unique_dest` 이름으로 만들고 → 재열람 → 그 행 선택 + 인라인 이름 바꾸기.
    pub(crate) fn create_new(&mut self, folder: bool) {
        let dir = self.panels[self.active].root_path();
        self.create_new_at(
            dir,
            if folder {
                NewKind::Folder
            } else {
                NewKind::File
            },
        );
    }

    /// `dir`에 새 항목(폴더 · 빈 txt · 템플릿 — SHELL-008 "새로 만들기 ▸") → 히스토리(undo = 휴지통 · redo = 재생성) → 재열람 → 선택 + 이름 바꾸기.
    pub(crate) fn create_new_at(&mut self, dir: PathBuf, kind: NewKind) {
        let a = self.active;
        if !dir.is_dir() {
            return;
        }
        let folder = matches!(kind, NewKind::Folder);
        let created = match &kind {
            NewKind::Folder => ndir_ops::create_new_dir(&dir, &tr("new.folderBase")),
            NewKind::File => {
                ndir_ops::create_new_file(&dir, &format!("{}.txt", tr("new.fileBase")))
            }
            NewKind::Template(t) => {
                let name = if t.ext.is_empty() {
                    trf("new.named", &[&t.label])
                } else {
                    format!("{}.{}", trf("new.named", &[&t.label]), t.ext)
                };
                let dest = ndir_ops::unique_dest(&dir, &name, false);
                materialize(&dest, &t.source).map(|()| dest)
            }
        };
        let path = match created {
            Ok(p) => p,
            Err(e) => {
                self.status_note(&trf("new.fail", &[&e.to_string()]));
                return;
            }
        };
        let desc = tr(if folder { "new.folderOp" } else { "new.fileOp" });
        let trash = Rc::clone(&self.platform.trash);
        let delete: ndir_ops::history::DeleteFn = Box::new(move |p: &std::path::Path| {
            trash
                .trash(std::slice::from_ref(&p.to_path_buf()))
                .map(|_| ())
                .map_err(|e| std::io::Error::other(e.to_string()))
        });
        let recreate: ndir_ops::history::RecreateFn = {
            let p = path.clone();
            match kind {
                NewKind::Folder => Box::new(move || std::fs::create_dir(&p)),
                NewKind::File => {
                    Box::new(move || materialize(&p, &platform::TemplateSource::Empty))
                }
                NewKind::Template(t) => Box::new(move || materialize(&p, &t.source)),
            }
        };
        self.history.push(Box::new(ndir_ops::history::CreateOp::new(
            path.clone(),
            desc,
            delete,
            recreate,
        )));
        let mut inv = Invalidations::default();
        self.dirsizes.clear(); // 폴더 크기 캐시(T-166) — 내용이 바뀌었다.
        self.git_detail.clear(); // git 요약 — 작업으로 바뀌었다(10-06).
        for p in &mut self.panels {
            p.reopen(&mut inv);
        }
        // 생성 행 선택 + 이름 바꾸기(dir2 RevealAndRename).
        self.panels[a].select_path(&path, &mut inv);
        self.begin_rename();
        self.update_status();
        self.redraw();
    }

    /// F2 — 캐럿 행 인라인 이름 바꾸기 시작(가상 최상위·캐럿 없음 = 무동작).
    pub(crate) fn begin_rename(&mut self) {
        let a = self.active;
        let Some(row) = self.panels[a].rows().caret() else {
            return;
        };
        let Some(path) = self.panels[a].rows().source().row_path(row) else {
            return;
        };
        let name = ndir_ops::leaf_name(&path);
        if name.is_empty() {
            return;
        }
        // 편집기에는 목록에 보이는 이름 그대로(바로 가기 확장자 숨김 — 확정 때 다시 붙인다 · GAP-006).
        let name = crate::filelist::display_name(
            &name,
            path.is_dir(),
            crate::platform::hides_shortcut_ext(),
        )
        .to_string();
        let mut inv = Invalidations::default();
        self.panels[a].rows_mut().begin_rename(row, &name, &mut inv);
        self.redraw();
    }

    /// 인라인 이름 바꾸기 확정(dir2 `apply_rename` · OPS-017 · RenameOp): 같은 이름 = 무동작 · 실패 = 상태줄 `rename.fail`.
    pub(crate) fn apply_rename(&mut self, panel: usize, row: usize, new_name: &str) {
        let Some(path) = self.panels[panel].rows().source().row_path(row) else {
            return;
        };
        let new_name = &crate::filelist::restore_shortcut_ext(
            &ndir_ops::leaf_name(&path),
            new_name,
            path.is_dir(),
            crate::platform::hides_shortcut_ext(),
        );
        let mut inv = Invalidations::default();
        match ndir_ops::rename(&path, new_name) {
            Ok(new_path) => {
                if new_path != path {
                    let desc = trf("rename.done", &[&ndir_ops::leaf_name(&path), new_name]);
                    self.history.push(Box::new(ndir_ops::history::RenameOp::new(
                        path.clone(),
                        new_path.clone(),
                        desc.clone(),
                    )));
                    self.statusbar.set_left(&desc, &mut inv);
                }
                self.dirsizes.clear(); // 폴더 크기 캐시(T-166) — 내용이 바뀌었다.
                self.git_detail.clear(); // git 요약 — 작업으로 바뀌었다(10-06).
                for p in &mut self.panels {
                    p.reopen(&mut inv);
                }
                self.panels[panel].select_path(&new_path, &mut inv);
            }
            Err(e) => {
                self.statusbar
                    .set_left(&trf("rename.fail", &[&e.to_string()]), &mut inv);
            }
        }
        self.update_status();
        self.redraw();
    }

    /// 이름 바꾸기 편집 필드 안의 편집 명령(dir2 `do_clip` ② — undo/cut/copy/paste/select_all/delete). 처리했으면 true.
    /// 경로 바 편집 중의 편집 명령 → 경로 글자(텍스트 클립보드). 처리했으면 true. 편집 중이면 **모르는 편집 명령도 삼킨다** —
    /// 경로를 고치는 동안 파일 명령(붙여넣기·삭제·되돌리기)이 실행되면 안 된다.
    pub(crate) fn path_edit(&mut self, id: &str) -> bool {
        let a = self.active;
        if !self.panels[a].pathbar.is_editing() {
            return false;
        }
        let mut inv = Invalidations::default();
        let bar = &mut self.panels[a].pathbar;
        match id {
            "edit.undo" => {
                bar.edit_undo(&mut inv);
            }
            "edit.cut" => {
                if let Some(t) = bar.edit_cut(&mut inv) {
                    let _ = clipboard::write_text(&t);
                }
            }
            "edit.copy" => {
                if let Some(t) = bar.edit_selected_text() {
                    let _ = clipboard::write_text(&t);
                }
            }
            "edit.paste" => {
                if let Some(t) = clipboard::read_text() {
                    // 여러 줄/따옴표로 감싼 경로(탐색기 "경로로 복사")도 한 줄 경로로.
                    let line: String = t
                        .lines()
                        .next()
                        .unwrap_or("")
                        .trim()
                        .trim_matches('"')
                        .chars()
                        .filter(|c| !c.is_control())
                        .collect();
                    bar.edit_paste(&line, &mut inv);
                }
            }
            "edit.select_all" => {
                bar.edit_key(nexa_grid::EditKey::SelectAll, false, &mut inv);
            }
            "edit.delete" | "edit.delete_permanent" => {
                bar.edit_delete(&mut inv);
            }
            // 그 밖의 편집/파일 변경 명령은 경로 편집 중에는 실행하지 않는다.
            "edit.redo" | "edit.rename" | "edit.bulk_rename" => {}
            _ => return false,
        }
        if matches!(
            id,
            "edit.undo" | "edit.cut" | "edit.paste" | "edit.delete" | "edit.delete_permanent"
        ) {
            self.panels[a].update_path_suggest(&mut inv);
        }
        self.redraw();
        true
    }

    pub(crate) fn rename_edit(&mut self, id: &str) -> bool {
        let a = self.active;
        if !self.panels[a].rows().is_renaming() {
            return false;
        }
        let mut inv = Invalidations::default();
        let rows = self.panels[a].rows_mut();
        let done = match id {
            "edit.undo" => {
                rows.rename_undo(&mut inv);
                true
            }
            "edit.cut" => {
                if let Some(t) = rows.rename_cut(&mut inv) {
                    let _ = clipboard::write_text(&t);
                }
                true
            }
            "edit.copy" => {
                if let Some(t) = rows.rename_selected_text() {
                    let _ = clipboard::write_text(&t);
                }
                true
            }
            "edit.paste" => {
                if let Some(t) = clipboard::read_text() {
                    let line: String = t.chars().filter(|c| !c.is_control()).collect();
                    rows.rename_paste(&line, &mut inv);
                }
                true
            }
            "edit.select_all" => {
                rows.rename_key(nexa_grid::EditKey::SelectAll, false, &mut inv);
                true
            }
            "edit.delete" => {
                rows.rename_delete(&mut inv);
                true
            }
            _ => false,
        };
        if done {
            self.redraw();
        }
        done
    }

    /// 히스토리 접근(시험).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn history(&self) -> &OperationHistory {
        &self.history
    }
}
