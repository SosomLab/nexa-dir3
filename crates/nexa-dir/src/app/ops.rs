//! 파일 작업 배선(M6 A · dir2 `win.rs` `do_clip`/`paste_dest`/`start_transfer`/undo 축약 · docs/port/22 OPS-001~039).
//!
//! - **클립보드**: `edit.copy`/`edit.cut` = 선택 경로를 OS 파일 클립보드(FileClipboard 포트 · Windows CF_HDROP)에 쓰고 **앱 내 사본**도 둔다
//!   (포트 `Unsupported`인 OS는 앱 내 사본만 — T-52/53 뒤 OS 클립보드로). `edit.paste` = OS 클립보드 우선 · 없으면 앱 내 사본.
//! - **전송**: 대상 = 선택한 폴더 1개 또는 활성 패널 폴더(dir2 `paste_dest`) · `ndir_xfer::run_full`(T-202 ④ 슬라이스 1 · 충돌 콜백 · 엔진
//!   이벤트 통과 · `lenient_sources` = 없는 원본은 그 항목만 실패 · 조절값은 설정 → `JobSpec`)을 **작업 스레드**에서(진행 = 공유 원자값 ·
//!   틱마다 상태줄 `ops.progress`) · 완료 = 두 패널 재열람 + 토스트(`ops.done/skipped/errors/canceled`) + 히스토리 기록(이동 = `MoveBatchOp` ·
//!   복사 = `CopyBatchOp` + 휴지통 삭제 주입). 실행 1건 + **대기 큐**(`transfer.queue` on · T-202 ④ 슬라이스 2 · `app/queue.rs` · 머리는
//!   `pump_queue`가 틱에서 자동 시작 · off = 종전 거부 `ops.busy`).
//! - **충돌**: 확인 창(T-29 nexa-dlg)이 오기 전까지 **건너뜀**(`Conflict::Skip` — 조용히 덮어쓰지 않는다 · 건너뛴 수는 토스트에) ⚠ OPS-004.
//! - **undo/redo**: `OperationHistory`(세션 한정 100) · 결과 문구 `undo.done/fail` · `history.*`.

use crate::app::dialogs::DlgReply;
use crate::platform::PlatformError;
use crate::*;
use ndir_ops::history::{CopyBatchOp, MoveBatchOp, OpError, OperationHistory};
use ndir_ops::{Conflict, Event, ItemStatus, Op, OpsError, Outcome};
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
    /// 항목별 앞 항목 크기 누적 합(계획 때 한 번 · `Bytes`마다 O(n) 합산하던 것을 O(1)로 · 10-18 체감 점검).
    pub offsets: Mutex<Vec<u64>>,
    /// 워커가 무엇이든 바꿨다 — UI 틱이 진행 창 스냅숏(항목 전체 복제)을 **바뀐 때만** 만든다(10-18).
    pub dirty: AtomicBool,
    /// 지금 옮기는 항목의 대상 경로(진행 창 "현재 파일" 줄 · T-96 · 10-25) — `ItemStart`마다.
    pub current_name: Mutex<String>,
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
    /// 큐 항목 id(끝나면 `queue/<id>.*` 파일 정리 · T-202 ④).
    pub queue_id: u64,
    /// 재개(슬라이스 3 · §12.7): 저널의 끊긴 항목 **대상** 집합 — 충돌 질문에 "중단된 사본" 줄을 덧붙인다(끊긴 이동은 엔진이 Skip).
    pub interrupted: std::collections::HashSet<PathBuf>,
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

/// 엔진 오류 → 사용자 문구(순수): 엔진 어휘([`OpsError`] · 영어 접두)면 i18n 키(`ops.err.*`) · 그 밖(OS 오류)은 문구 그대로.
pub(crate) fn ops_err_text(e: &std::io::Error) -> String {
    let text = e.to_string();
    match OpsError::of(&text) {
        Some(k) => {
            // 꼬리(`: 세부`)는 그대로 덧붙인다.
            let tail = &text[k.message().len()..];
            format!("{}{tail}", tr(k.i18n_key()))
        }
        None => text,
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
                let a = self.active;
                self.toast_panel(
                    a,
                    toast::ToastKind::Warn,
                    tr("menu.edit.copy"),
                    e.to_string(),
                );
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
        let Some((sources, cut)) = self.clip_sources() else {
            // 실제 파일이 없으면 가상 파일 폴백(T-147 ② · Outlook 첨부 · zip 안 · MTP → 워커 붙여넣기) — 그것도 없으면 안내.
            let dest = self.paste_dest();
            if !self.vpaste_start(&dest) {
                self.status_note(&tr("clip.empty"));
            }
            return;
        };
        let mut dest = self.paste_dest();
        // GAP-038(사용자 Windows 실기 11-02): 선택한 폴더가 붙여넣을 원본 **자신**이면(폴더 Ctrl+C → Ctrl+V) 그 안이 아니라
        // 부모에 — 같은 폴더 복사 = 순번 복제 "(2)"(파일과 같은 동작) · 잘라내기 = 제자리 이동 = 무동작. 원본의 하위 폴더를 골랐으면
        // 그대로 두고 엔진이 `IntoSelf`로 거절한다(dir2 `paste_dest`도 같은 빈틈 · 복사는 자기 안에 자기를 끝없이 복사했다).
        if sources
            .iter()
            .any(|s| ndir_ops::is_same_or_sub(s, &dest) && ndir_ops::is_same_or_sub(&dest, s))
        {
            if let Some(parent) = dest.parent() {
                dest = parent.to_path_buf();
            }
        }
        if !dest.is_dir() {
            return;
        }
        let op = if cut { Op::Move } else { Op::Copy };
        self.start_transfer(sources, dest, op, cut);
    }

    /// 상태줄 왼쪽 안내 — `ui.status_note_ms` 동안 `update_status`(항목 수)가 덮지 않는다(GAP-034 · 협업 실측 11-01: 미러 삭제 결과 ·
    /// 둘째 인스턴스 안내가 기동 직후 "n items"로 바로 덮였다). 0 = 종전대로 바로 덮임. 마감은 [`Self::status_tick`]이 본다.
    pub(crate) fn status_note(&mut self, text: &str) {
        let mut inv = Invalidations::default();
        self.statusbar.set_left(text, &mut inv);
        let ms = self.settings.int("ui.status_note_ms").clamp(0, 30_000) as u64;
        self.status_hold = (ms > 0).then(|| Instant::now() + Duration::from_millis(ms));
        self.redraw();
    }

    /// 안내 유지 마감 틱(GAP-034): 지났으면 항목 수로 되돌리고 `None` · 아직이면 깨울 시각.
    pub(crate) fn status_tick(&mut self, now: Instant) -> Option<Instant> {
        let d = self.status_hold?;
        if now < d {
            return Some(d);
        }
        self.status_hold = None;
        self.update_status();
        self.redraw();
        None
    }

    /// 전송 시작 — 모든 복사/이동 진입점(붙여넣기 · DnD T-51 B · 컨텍스트 메뉴 · 라이선스 창)이 여기로. 명세를 만들어 **큐에 넣고**
    /// 머리를 돌린다(T-202 ④ 슬라이스 2 · docs/26 §12.3): 비어 있으면 바로 시작 · 실행 중/풀기/동기화 중이면 대기(`transfer.queue` on) ·
    /// off = 종전 거부(`ops.busy`). 머리도 큐를 거친다(`queue/<id>.job.json` + 저널 → 재시작 복원 슬라이스 3).
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
        let waiting = self.transfer_busy() || self.extract_job.is_some() || self.sync_job.is_some();
        if waiting && !self.settings.flag("transfer.queue") {
            self.status_note(&tr("ops.busy"));
            return;
        }
        // 명세(T-202 ④): 충돌은 콜백이 결정하므로 정책은 자리만 · 조절값은 전역 `set_tuning`과 같은 설정 키(시작 시점 스냅숏 — 종전과 같다).
        let t = Self::transfer_tuning(&self.settings);
        let spec = ndir_xfer::JobSpec {
            op,
            sources,
            dest,
            conflict: ndir_xfer::ConflictPolicy::Skip,
            dir_conflict: Self::dir_conflict_mode(&self.settings),
            threads: t.threads,
            native: t.native,
            unbuffered_mb: t.unbuffered_min / (1024 * 1024),
            ..ndir_xfer::JobSpec::default()
        };
        let (_id, saved) = self.transfer_queue.push(spec, cut);
        if saved == app::queue::Saved::NotUtf8 {
            let a = self.active;
            self.toast_panel(
                a,
                toast::ToastKind::Warn,
                tr("ops.progressTitle"),
                tr("ops.queueNotSaved"),
            );
        }
        // 상태줄 안내는 `sync_queue_view`(update_status가 왼쪽 글을 다시 쓴다) **뒤에**.
        let started = self.pump_queue();
        self.sync_queue_view();
        if !started {
            let n = self.transfer_queue.len().to_string();
            self.status_note(&trf("ops.queued", &[&n]));
        }
    }

    /// 폴더 이름 충돌 처리(DR-37 · 설정 `transfer.dir_conflict` · 모르는 값 = 병합).
    pub(crate) fn dir_conflict_mode(settings: &ndir_settings::Settings) -> ndir_ops::DirConflict {
        settings
            .get("transfer.dir_conflict")
            .and_then(ndir_ops::DirConflict::parse)
            .unwrap_or_default()
    }

    /// 전송이 돌고 있다(머리 1건).
    pub(crate) fn transfer_running(&self) -> bool {
        self.transfer.is_some()
    }

    /// 대기 중인 작업이 있다.
    pub(crate) fn queue_pending(&self) -> bool {
        !self.transfer_queue.is_empty()
    }

    /// 실행 중 또는 대기 있음 — 다른 작업(풀기 · 동기화 · 압축 · undo)의 문지기(대기 작업의 원본을 건드리지 않게 · 협업 분석 11-01).
    pub(crate) fn transfer_busy(&self) -> bool {
        self.transfer_running() || self.queue_pending()
    }

    /// 새 드래그/드롭을 막아야 하는가 — 큐가 **꺼져** 있을 때만(켜져 있으면 드롭도 큐로 · 협업 분석 11-01: 종전 §12.3 "드롭은 자연히
    /// 큐로"는 DnD 문지기 셋 때문에 사실이 아니었다).
    pub(crate) fn transfer_blocks_new(&self) -> bool {
        self.transfer_running() && !self.settings.flag("transfer.queue")
    }

    /// 큐 머리를 돌린다(실행 중 · 풀기 · 동기화가 없을 때) — 반환 = 시작했다. 실행 차례의 재검사는 ndir-xfer가 한다(대상 없음 =
    /// `Outcome.errors` → 실패 토스트 · 없는 원본 = 그 항목만).
    pub(crate) fn pump_queue(&mut self) -> bool {
        if self.transfer.is_some() || self.extract_job.is_some() || self.sync_job.is_some() {
            return false;
        }
        let Some(job) = self.transfer_queue.pop_front() else {
            return false;
        };
        self.launch_transfer(job);
        self.sync_queue_view();
        true
    }

    /// 진행 창 대기 목록 · 상태바 xfer 칸("대기 n") 갱신.
    pub(crate) fn sync_queue_view(&mut self) {
        self.progress_win.set_queue(self.transfer_queue.labels());
        self.update_status();
        self.redraw();
    }

    /// 진행 창의 대기 행 버튼([위로] [아래로] [삭제]) 수거 → 큐에 반영(행 번호 → id).
    pub(crate) fn queue_actions_tick(&mut self) {
        let Some((row, act)) = self.progress_win.take_queue_action() else {
            return;
        };
        let Some(&id) = self.transfer_queue.ids().get(row) else {
            return;
        };
        let changed = match act {
            crate::progress_win::QueueAct::Up => self.transfer_queue.move_up(id),
            crate::progress_win::QueueAct::Down => self.transfer_queue.move_down(id),
            crate::progress_win::QueueAct::Remove => self.transfer_queue.remove(id),
        };
        if changed {
            self.sync_queue_view();
        }
    }

    /// 작업 스레드 시작(큐 머리 1건) — 진행 = 공유 원자값 · 충돌 = 질문 채널 · 결과 = `Outcome`(ndir-xfer `run_full`).
    fn launch_transfer(&mut self, job: app::queue::QueuedJob) {
        let app::queue::QueuedJob {
            id: queue_id,
            spec,
            cut,
        } = job;
        let op = spec.op;
        let count = spec.sources.len();
        // 재개(슬라이스 3): 저널의 끊긴 항목 대상 = 질문 글 표시용(강제 덮어쓰기 대신 묻는다 = `ask_interrupted`).
        let interrupted: std::collections::HashSet<PathBuf> = spec
            .resume
            .as_deref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| ndir_xfer::resume::interrupted_items(&t).ok())
            .map(|v| v.into_iter().map(|i| i.dest).collect())
            .unwrap_or_default();
        let shared = Arc::new(TransferShared {
            cancel: AtomicBool::new(false),
            done: AtomicU64::new(0),
            total: AtomicU64::new(0),
            outcome: Mutex::new(None),
            items: Mutex::new(Vec::new()),
            current: AtomicUsize::new(0),
            offsets: Mutex::new(Vec::new()),
            dirty: AtomicBool::new(false),
            current_name: Mutex::new(String::new()),
        });
        // 진행 창(dir2 DLG-059: `transfer.close_ms > 0`일 때만).
        if self.settings.int("transfer.close_ms") > 0 {
            self.progress_win.reset(&tr("ops.progressLabel"));
            self.open_progress = true;
        }
        let sh = Arc::clone(&shared);
        let (req_tx, conflict_rx) = mpsc::channel::<ConflictReq>();
        std::thread::spawn(move || {
            // 충돌 = UI 대화상자 4버튼(dir2 QA 07-14 개정): "모두 덮어쓰기"만 이후 무확인 · 취소 = 전체 중단 + 그 항목 건너뜀.
            let mut decided: Option<Conflict> = None;
            let sh2 = Arc::clone(&sh);
            let mut resolve = |p: &std::path::Path| {
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
            };
            let mut on_event = |ev: &Event<'_>| match ev {
                Event::Plan { sizes, total_bytes } => {
                    sh.total.store(*total_bytes, Ordering::Relaxed);
                    if let Ok(mut off) = sh.offsets.lock() {
                        let mut acc = 0u64;
                        *off = sizes
                            .iter()
                            .map(|&s| {
                                let o = acc;
                                acc = acc.saturating_add(s);
                                o
                            })
                            .collect();
                    }
                    sh.dirty.store(true, Ordering::Relaxed);
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
                Event::ItemStart { index, dest, .. } => {
                    sh.current.store(*index + 1, Ordering::Relaxed);
                    if let Ok(mut n) = sh.current_name.lock() {
                        *n = dest.display().to_string();
                    }
                    sh.dirty.store(true, Ordering::Relaxed);
                    if let Ok(mut it) = sh.items.lock() {
                        if let Some(i) = it.get_mut(*index) {
                            i.status = SegStatus::Active;
                        }
                    }
                }
                Event::Bytes(p) => {
                    sh.done.store(p.done_bytes, Ordering::Relaxed);
                    sh.dirty.store(true, Ordering::Relaxed);
                    // 항목 내 진행 = 전체 누적 − 앞 항목 크기 합(세그먼트 부분 채움) — 누적 합은 계획 때 만든 표에서 O(1).
                    let before: u64 = sh
                        .offsets
                        .lock()
                        .ok()
                        .and_then(|o| o.get(p.item_index).copied())
                        .unwrap_or(0);
                    if let Ok(mut it) = sh.items.lock() {
                        if let Some(i) = it.get_mut(p.item_index) {
                            i.done = p.done_bytes.saturating_sub(before).min(i.size);
                        }
                    }
                }
                Event::ItemEnd { index, status } => {
                    sh.dirty.store(true, Ordering::Relaxed);
                    if let Ok(mut it) = sh.items.lock() {
                        if let Some(i) = it.get_mut(*index) {
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
            };
            // 결과는 엔진 `Outcome`(실제 (원본, 최종 대상) · undo) — Report는 앱이 쓰지 않는다(§12.3 ②). 취소 = 콜백이 세운 `sh.cancel`.
            let (_report, out) = ndir_xfer::run_full(
                &spec,
                &mut |_| {},
                &sh.cancel,
                ndir_xfer::Hooks {
                    resolve: Some(&mut resolve),
                    on_event: Some(&mut on_event),
                    lenient_sources: true,
                    ask_interrupted: true,
                    ..ndir_xfer::Hooks::default()
                },
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
            queue_id,
            interrupted,
        });
        self.status_note(&trf("ops.progress", &["0"]));
    }

    /// 종료 때(협업 확인 11-01 · ⓑ): 돌고 있는 전송을 **취소 경로로** 멈춘다 — 그냥 끊으면 매 정상 종료가 GAP-030의 끊긴 항목(부분 파일 ·
    /// `.nexa-tmp-<pid>` 잔여)을 만든다. 취소 = 엔진이 부분 파일·스테이징을 치우고 저널은 started 그대로 → 다음 기동 복원이 이어 받는다.
    /// 충돌 질문에 멈춰 있으면(진행 창 · 대화상자 · 아직 수거 전) 취소 답을 보내 깨운다 · 최대 3초 기다린다 · **큐 파일은 지우지 않는다**.
    pub(crate) fn transfer_abort_for_exit(&mut self) {
        let Some(job) = &self.transfer else {
            return;
        };
        job.shared.cancel.store(true, Ordering::Relaxed);
        if let Some(tx) = self.conflict_inline.take() {
            let _ = tx.send(ConflictChoice::Cancel);
        }
        if matches!(self.dlg_reply, Some(DlgReply::Conflict(_))) {
            if let Some(DlgReply::Conflict(tx)) = self.dlg_reply.take() {
                let _ = tx.send(ConflictChoice::Cancel);
            }
        }
        if matches!(self.dlg_pending, Some((_, DlgReply::Conflict(_)))) {
            if let Some((_, DlgReply::Conflict(tx))) = self.dlg_pending.take() {
                let _ = tx.send(ConflictChoice::Cancel);
            }
        }
        while let Ok((_, tx)) = job.conflict_rx.try_recv() {
            let _ = tx.send(ConflictChoice::Cancel);
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while job
            .shared
            .outcome
            .lock()
            .map(|s| s.is_none())
            .unwrap_or(false)
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
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
        // 바뀐 것이 없으면 항목 전체 복제(수만 개)도 없다(10-18).
        if !job.shared.dirty.swap(false, Ordering::Relaxed) {
            return;
        }
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
        let name = job
            .shared
            .current_name
            .lock()
            .map(|n| n.clone())
            .unwrap_or_default();
        self.progress_win.set_current_name(&name);
    }

    /// 전송 폴링(틱): 진행률 상태줄 · 완료 수거. 반환 = 아직 진행 중.
    pub(crate) fn ops_tick(&mut self) -> bool {
        // 진행 창 스냅숏은 **질문을 띄우기 전에도 · 답을 기다리는 동안에도** 맞춘다 — 종전에는 덮어쓰기 질문이 떠 있으면 건너뛰어,
        // 앞 항목의 결과(덮어씀 = 채움 · 건너뜀 = 진한 회색)와 "파일 n/m"이 다음 질문 뒤에야 반영됐다(사용자 10-04).
        self.sync_progress_win();
        self.queue_actions_tick();
        let Some(job) = &self.transfer else {
            // 실행 중이 없으면 머리를 돌린다(풀기/동기화에 막혀 있으면 대기 유지 = 계속 폴링 · T4 `ops.wait`는 큐도 본다).
            return self.pump_queue() || self.queue_pending();
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
                let qid = job.queue_id;
                self.finish_transfer(job, out);
                self.transfer_queue.forget(qid);
                self.mem_after_job();
                // 다음 머리(T-202 ④): 같은 틱에 바로 시작해 폴링(`WaitUntil` · T3 `while ops_tick()`)이 끊기지 않는다.
                self.pump_queue() || self.queue_pending()
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
                // 퍼센트 글이 바뀐 틱만 창을 다시 그린다(종전 = 100 ms마다 무조건 · 10-18) — 그때 진행 칸(T-96)도.
                if !inv.is_empty() {
                    let segs = self.status_segments();
                    self.statusbar.set_segments(segs, &mut inv);
                    self.redraw();
                }
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
        let a = self.active;
        // 완료 토스트 → 실행 취소(T-96 · NEW-006): 되돌릴 것이 있으면(히스토리에 넣었음) 카드를 누르면 `edit.undo`.
        if out.transferred.is_empty() {
            self.toast_panel(a, kind, title, parts.join(" · "));
        } else {
            self.toast_panel_action(
                a,
                kind,
                format!("{title} \u{2014} {}", tr("ops.undoHint")),
                parts.join(" · "),
                "edit.undo",
            );
        }
        // 결과 요약은 안내 경로(GAP-034 · 유지 시간 동안 항목 수로 안 덮임).
        self.status_note(&parts.join(" · "));
        self.update_status();
        self.redraw();
        let _ = job.count;
    }

    /// `edit.undo`/`edit.redo`.
    pub(crate) fn history_step(&mut self, redo: bool) {
        if self.transfer_busy() {
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
            "transfer {transfer}\nqueue {}{}\nundo {} {}\nredo {}\nclip {clip}\n",
            self.transfer_queue.len(),
            if self.transfer_queue.memory_only() {
                " memory"
            } else {
                ""
            },
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
        let mut note: Option<String> = None;
        match ndir_ops::rename(&path, new_name) {
            Ok(new_path) => {
                if new_path != path {
                    let desc = trf("rename.done", &[&ndir_ops::leaf_name(&path), new_name]);
                    self.history.push(Box::new(ndir_ops::history::RenameOp::new(
                        path.clone(),
                        new_path.clone(),
                        desc.clone(),
                    )));
                    note = Some(desc);
                }
                self.dirsizes.clear(); // 폴더 크기 캐시(T-166) — 내용이 바뀌었다.
                self.git_detail.clear(); // git 요약 — 작업으로 바뀌었다(10-06).
                                         // 펼침·선택을 새 이름으로 이어서 복원(WINB-033 `rename_expanded` · 10-25 — 종전엔 이름 바꾼 폴더 안이 접혔다).
                for p in &mut self.panels {
                    p.reopen_renamed(&path, &new_path, &mut inv);
                }
                self.panels[panel].select_path(&new_path, &mut inv);
            }
            Err(e) => {
                note = Some(trf("rename.fail", &[&ops_err_text(&e)]));
            }
        }
        // 상태줄 안내는 갱신 **뒤에**(갱신이 왼쪽 글을 덮는다 — 종전엔 성공/실패 문구가 보이지 않았다 · 10-25).
        self.update_status();
        if let Some(n) = note {
            self.statusbar.set_left(&n, &mut inv);
        }
        self.redraw();
    }

    /// 이름 바꾸기 편집 필드 안의 편집 명령(dir2 `do_clip` ② — undo/cut/copy/paste/select_all/delete). 처리했으면 true.
    /// 검색창(빠른 필터 상자) 포커스 중의 편집 명령 → 상자 글자(텍스트 클립보드 · 사용자 10-26 — 종전엔 Ctrl+C/X/V가 파일
    /// 복사/붙여넣기로 갔다). 처리했으면 true. 경로 바와 같은 규칙: 파일을 바꾸는 편집 명령은 삼킨다.
    pub(crate) fn filter_edit(&mut self, id: &str) -> bool {
        use nexa_ctl::EditCtxAction as A;
        let a = self.active;
        if !self.panels[a].filter_focused() {
            return false;
        }
        let mut inv = Invalidations::default();
        match id {
            "edit.copy" => self.panels[a].filter_clip(A::Copy, &mut inv),
            "edit.cut" => self.panels[a].filter_clip(A::Cut, &mut inv),
            "edit.paste" => self.panels[a].filter_clip(A::Paste, &mut inv),
            "edit.select_all" => self.panels[a].key_event(&InputEvent::SelectAll, &mut inv),
            "edit.delete" | "edit.delete_permanent" => {
                self.panels[a].key_event(
                    &InputEvent::Key {
                        key: nexa_ctl::Key::Delete,
                        shift: false,
                        primary: false,
                    },
                    &mut inv,
                );
            }
            // 되돌리기/다시 실행은 상자 글자에(TextBox 단계 되돌리기 · 협업 검토 10-26) — 파일 작업 되돌리기로 가지 않는다.
            "edit.undo" => self.panels[a].key_event(&InputEvent::Undo, &mut inv),
            "edit.redo" => self.panels[a].key_event(&InputEvent::Redo, &mut inv),
            "edit.rename" | "edit.bulk_rename" => {}
            _ => return false,
        }
        self.redraw();
        true
    }

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

/// 클립보드 가상 파일 붙여넣기 워커(T-147 ② · SHELL-046~049): 워커가 항목마다 `(index, ok)`를 보내고 끝나면 채널을 닫는다 — `vpaste_tick`이 수거.
pub(crate) struct VPasteJob {
    pub(crate) rx: std::sync::mpsc::Receiver<(usize, bool)>,
    pub(crate) cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub(crate) roots: Vec<PathBuf>,
    pub(crate) total: usize,
    pub(crate) done: usize,
    pub(crate) failed: usize,
}

impl App {
    /// 클립보드에 실제 파일(CF_HDROP)이 없을 때의 폴백(dir2 X-42): 가상 파일(Outlook 첨부 · zip 안 · MTP)이 있으면 `dest`로 **워커** 붙여넣기 시작.
    /// 돌려주는 값 = 시작했는가. 동시 1잡 · 가짜 클립보드 모드(시험)에서는 실 클립보드를 읽지 않는다.
    pub(crate) fn vpaste_start(&mut self, dest: &std::path::Path) -> bool {
        if platform::fake_clipboard() || self.vpaste_job.is_some() || !dest.is_dir() {
            return false;
        }
        let Some(plan) = platform::vpaste_plan(dest) else {
            return false;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (roots, total) = (plan.roots.clone(), plan.items);
        platform::vpaste_run(plan, tx, std::sync::Arc::clone(&cancel));
        self.vpaste_job = Some(VPasteJob {
            rx,
            cancel,
            roots,
            total,
            done: 0,
            failed: 0,
        });
        self.status_note(&trf("clip.vpasteProgress", &["0", &total.to_string()]));
        self.redraw();
        true
    }

    pub(crate) fn vpaste_busy(&self) -> bool {
        self.vpaste_job.is_some()
    }

    /// 워커 수거(틱) — 진행이 바뀌었거나 끝났으면 `true`.
    pub(crate) fn vpaste_tick(&mut self) -> bool {
        let (changed, finished, done, total) = {
            let Some(job) = &mut self.vpaste_job else {
                return false;
            };
            let mut changed = false;
            let mut finished = false;
            loop {
                match job.rx.try_recv() {
                    Ok((_, ok)) => {
                        job.done += 1;
                        if !ok {
                            job.failed += 1;
                        }
                        changed = true;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        finished = true;
                        break;
                    }
                }
            }
            (changed, finished, job.done, job.total)
        };
        if finished {
            let job = self.vpaste_job.take().expect("checked above");
            self.vpaste_finish(job);
            return true;
        }
        if changed {
            self.status_note(&trf(
                "clip.vpasteProgress",
                &[&done.to_string(), &total.to_string()],
            ));
        }
        changed
    }

    /// 결과 반영: 토스트(붙여넣은 수 · 실패 수) · 생성된 최상위 = 실행 취소(`VPasteOp` · 휴지통으로) · 재열람.
    fn vpaste_finish(&mut self, job: VPasteJob) {
        let created: Vec<PathBuf> = job.roots.into_iter().filter(|p| p.exists()).collect();
        let ok = job.done.saturating_sub(job.failed);
        let mut text = trf("clip.vpasteDone", &[&ok.to_string()]);
        if job.failed > 0 {
            text.push_str(" · ");
            text.push_str(&trf("clip.vpasteFail", &[&job.failed.to_string()]));
        }
        if !created.is_empty() {
            let n = created.len().to_string();
            self.history.push(Box::new(trashop::VPasteOp::new(
                created,
                trf("clip.vpasteOp", &[&n]),
                Rc::clone(&self.platform.trash),
            )));
        }
        let a = self.active;
        self.toast_panel(
            a,
            if job.failed > 0 {
                toast::ToastKind::Warn
            } else {
                toast::ToastKind::Info
            },
            tr("menu.edit.paste"),
            text,
        );
        let mut inv = Invalidations::default();
        for p in &mut self.panels {
            p.reopen(&mut inv);
        }
        self.status_hold = None;
        self.update_status();
    }
}

impl App {
    /// 붙여넣기 워커에 취소 신호(남은 항목 건너뜀 · 쓰다 만 파일 삭제) — 창 닫기 · 기동 명령 `vpaste.cancel`.
    pub(crate) fn vpaste_cancel(&self) {
        if let Some(job) = &self.vpaste_job {
            job.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
}
