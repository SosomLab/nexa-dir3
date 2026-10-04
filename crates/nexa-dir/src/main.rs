//! Nexa Dir — 앱 진입점(M3 · winit 호스트 + nexa-ui 위에 그리는 수직 슬라이스).
//!
//! 구조 = nexa-sql 3층(docs/port/40 SKEL-401): 이 파일(모듈 선언 · `struct App` · `layout_core()` · `main()`) /
//! `app/*.rs`(`impl App` 조각 — 이벤트 루프 · 입력 · 그리기 · 메뉴/명령 · 기동 명령) / 호스트 껍질은 파일 하나씩
//! (`present` · `winhost` · `wingeom` · `winfocus` · `theme` · `icon` · `input` · `clipboard` · `toast` — nexa-sql 복사 · SKEL-403).
//! 화면 = dir2 `win.rs::layout`(docs/port/13 PANEL-001·002): 메뉴 / 도구 모음 / [좌 패널 ║ 우 패널] / 상태바 · 패널 = `panel.rs`.
//! 규칙: 인자 해석·판정은 순수 함수(`cli.rs` · `selfcheck.rs`) · 그리기는 `RedrawRequested`에서만 · 유휴는 `WaitUntil`(SKEL-414).

mod app;
mod archive_win;
mod bulk_win;
mod check_win;
mod cli;
#[allow(dead_code)] // M6 파일 작업·M5 터미널 복사에서 소비(지금은 호스트 껍질만 들여놓음).
mod clipboard;
#[cfg(all(unix, not(target_os = "macos")))]
#[allow(dead_code)]
mod clipboard_x11;
mod copybtn;
mod crash;
mod dirinfo;
mod dlg_win;
mod dockinfo;
mod file_win;
mod filelist;
mod icon;
mod icons;
#[allow(dead_code)]
mod input;
mod keys_win;
mod launcher;
mod license_win;
mod mem_win;
mod memstat;
mod nav;
mod order;
mod order_win;
mod panel;
mod pathinput;
mod platform;
mod prefs_win;
#[allow(dead_code)]
// `set_mode`(macOS IOSurface 설정 · dir2에 키 없음 → Q-8) · `backend`(프레임 계측 T-46).
mod present;
mod preview;
mod preview_win;
mod progress_win;
mod selfcheck;
mod session;
mod termview;
mod theme;
pub(crate) use nexa_ctl::controls::toast; // nexa-ui 114차 승격(T-32 · UIK-213) — 앱 사본 삭제
mod trashop;
#[allow(dead_code)]
mod winfocus;
#[allow(dead_code)]
mod wingeom;
#[allow(dead_code)] // 보조 창(설정 · 단축키 · About)이 T-44에서 쓴다.
mod winhost;

use archive_win::ArchiveWin;
use bulk_win::BulkWin;
use check_win::CheckWin;
use dlg_win::DlgWin;
use file_win::FileWin;
use filelist::ListOpts;
use keys_win::KeysWin;
use license_win::LicenseWin;
use ndir_i18n::{tr, trf};
use ndir_settings::keymap::{Chord, Keymap};
use ndir_settings::{Settings, ThemeMode};
use nexa_ctl::controls::{
    ComboItem, ContextMenu, Control, CtxItem, DockAction, DockLayout, MenuBar, MenuDef, MenuEntry,
    SplitAxis, SplitBand, SplitEvent, Splitter, StatusBar, ToolDock, ToolGroup, ToolIcon, ToolItem,
    Toolbar,
};
use nexa_ctl::draw::DrawCtx;
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::{InputEvent, Invalidations, Widget};
use nexa_explorer::InfoDock;
use nexa_gfx::{Font, Surface};
use nexa_grid::{Column, RowSource, ScrollAlign, ViewMode};
use order_win::OrderWin;
use panel::{Panel, PanelMetrics};
use platform::Platform;
use prefs_win::PrefsWin;
use preview_win::PreviewWin;
use progress_win::ProgressWin;
use session::Session;
use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use termview::TermView;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// Linux X11 클립보드 직접 경로(`clipboard.rs`가 본다). dir2에 대응 설정이 없어 늘 켬(CLI 폴백은 그대로).
static CLIP_NATIVE: AtomicBool = AtomicBool::new(true);
#[cfg_attr(not(all(unix, not(target_os = "macos"))), allow(dead_code))]
pub(crate) fn settings_clip_native() -> bool {
    CLIP_NATIVE.load(Ordering::Relaxed)
}

/// 스플리터 두께(논리 px · dir2 `SPLIT_TH`) · 자석 스냅 임계(dir2 `SNAP_PX` — 창 50% · Alt = 해제) · 패널 최소 폭(dir2 `MIN_PANEL`).
const SPLIT_TH: f32 = 3.0;
const SNAP_PX: f32 = 20.0;
const MIN_PANEL: f32 = 200.0;
/// 스플리터를 잡는 띠가 틈 양쪽으로 넓어지는 폭(논리 px · dir2 `SPLIT_HALF` — 배율은 dir3가 곱한다).
const SPLIT_HALF: f32 = 3.0;
/// 스플리터 hover가 다 올라왔을 때 accent 알파(사용자 10-03 "서서히 밝아지는" — dir2는 평상시 색 ↔ 드래그 accent 두 상태뿐이라
/// 그 사이를 nexa-ctl 페이드(전역 hover 진입 시간)로 잇는다 · 드래그 중 = accent 그대로).
const SPLIT_HOVER_ALPHA: f32 = 0.7;

/// 스플리터 3종(dir2 WINC-094~096 · 클릭 우선순위 = 도크 높이 → 도크 좌우 → 패널 좌우).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SplitKind {
    /// 좌우 패널 사이(세로선 · `layout.panel_split_pct`).
    Panel,
    /// 패널과 하단 도크 사이(가로선 · `layout.dock_height_pct`).
    DockHeight,
    /// 하단 도크 좌우 사이(세로선 · `layout.dock_split_pct`).
    DockSplit,
}
/// 툴바 아이콘 논리 크기의 기본값(dir2 20) — 실제 값 = 설정 `toolbar.icon_size`(16/20/24/32 · 마스크는 배율 곱한 px로 렌더).
const TOOLBAR_ICON_LOGICAL: i32 = 20;

/// 비율(%) = 반올림(`part / whole` — 드래그한 자리와 저장되는 정수 %가 반 칸 이상 어긋나지 않게).
fn pct_of(part: i32, whole: i32) -> i64 {
    if whole <= 0 {
        return 0;
    }
    ((i64::from(part) * 200 + i64::from(whole)) / (i64::from(whole) * 2)).max(0)
}

/// UI 스레드를 깨우는 사용자 이벤트(배경 작업이 보낸다 — M4 전송·감시 스레드 · SKEL-415).
#[derive(Debug)]
struct Wake;

/// 포인터 캡처 대상(눌린 곳이 뗄 때까지 받는다).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Area {
    Menu,
    Tool,
    Panel(usize),
    Split(SplitKind),
    Status,
    Dock(usize),
    Launcher,
}

/// 앱 — 상태 한 곳. **창 없이도 동작**(DR-10 · CI-102 최소 분리): `viewport`·`scale`은 Shell(이벤트 루프)이 창에서 읽어 넣고,
/// `layout_core`·`route`·`paint_into`·`command`는 winit을 모른다 → `cargo test`가 `App::new` + `layout_for` + `RecordCtx`로 시나리오를 돈다.
struct App {
    window: Option<Rc<Window>>,
    surface: Option<present::Presenter>,
    settings: Settings,
    keymap: Keymap,
    /// `Rc` = 그리기 때 래스터 컨텍스트가 글꼴을 빌리는 동안 `&mut self`(paint_into)를 쓰기 위해.
    ui_font: Rc<Font>,
    /// 고정폭 글꼴(터미널 · 코드 줄 — `FontSet.mono` · 없으면 Mono 슬롯은 UI 글꼴).
    mono_font: Option<Rc<Font>>,
    /// Windows Terminal 기본 프로필(설정 `term.follow_windows_terminal` · Windows만 · 없으면 None) — 터미널 글꼴·크기를 따라간다.
    wt_profile: Option<platform::WtProfile>,
    theme: Theme,
    scale: f32,
    /// 창 안쪽 크기(장치 px) — 창이 있으면 `layout()`이 창에서 읽고, 시험은 `layout_for`로 넣는다.
    viewport: (i32, i32),
    /// 포인터(장치 px) · 수식키.
    cursor: (i32, i32),
    shift: bool,
    primary: bool,
    alt: bool,
    ctrl_mac: bool,
    menubar: MenuBar,
    /// 상단 툴바 = 그룹 도크(nexa-sql `ToolDock` · 그룹 손잡이를 끌어 순서·행 이동 · 배치 = 설정 `toolbar.dock_layout`).
    toolbar: ToolDock,
    /// 파일 패널 2(듀얼 · dir2 좌/우) — 단일 모드면 `panels[1]`은 빈 사각형(상태는 보존).
    panels: [Panel; 2],
    /// 좌/우 스플리터(`layout.panel_split_pct` · 드래그 · 50% 스냅).
    splitter: Splitter,
    /// 패널 ↔ 도크 경계(가로선) · 도크 좌우 경계(세로선) — 좌우 패널 스플리터와 같은 컨트롤 · 같은 모양(T-115).
    dock_split_h: Splitter,
    dock_split_v: Splitter,
    /// 탭을 끌어 반대 패널 위에 있을 때 놓일 자리(대상 패널, 삽입선) — 그리기용(T-122).
    tab_drop_hint: Option<(usize, Rect)>,
    active: usize,
    dual: bool,
    statusbar: StatusBar,
    toasts: toast::Toasts,
    started: Instant,
    main_active: bool,
    pending_chord: Option<Chord>,
    pressed: Option<Area>,
    /// 더블클릭 합성(winit은 더블클릭 사건이 없다 · dir2 Win32 `WM_LBUTTONDBLCLK` 대응): 마지막 좌클릭 시각·자리.
    last_click: Option<(Instant, i32, i32)>,
    exit_requested: bool,
    startup_timed: Vec<(Instant, String)>,
    trace_ime: bool,
    /// 세션 파일 폴더(설정 폴더 · 시험은 `None` = 저장 안 함) · 디바운스 저장기 · 복원 때 받은 세션(dir3가 안 쓰는 키 보존).
    session_dir: Option<PathBuf>,
    session_save: nexa_conf::SaveScheduler,
    session_keep: Session,
    /// 종료 코드(`quit:<코드>` · 단언 실패 3) · `@ready` 큐 · 첫 프레임 뒤 한 번.
    exit_code: u8,
    startup_ready: Vec<String>,
    /// `ctx.wait`로 보류된 `@ready` 명령(셸 메뉴 항목 도착 대기) + 보류 시작 시각.
    startup_blocked: Vec<String>,
    startup_blocked_since: Instant,
    ready_fired: bool,
    /// 보조 창(설정 · 단축키) + 열기 깃발(펌프 `open_requested_windows`가 소비).
    prefs_win: PrefsWin,
    keys_win: KeysWin,
    /// 자가 점검 창(T-54 · Help ▸ 자가 점검).
    check_win: CheckWin,
    open_check: bool,
    /// 대화상자 창(T-29 A · 모달 · 동시 1건) + 대기 요청 + 결과 수신자.
    /// 독립 미리보기 창(F3 · ↗ · T-62 B) + 열기 요청.
    preview_win: PreviewWin,
    open_preview: bool,
    /// 전송 진행 창(T-70 · `transfer.close_ms > 0`일 때만) + 열기 깃발.
    progress_win: ProgressWin,
    open_progress: bool,
    /// 일괄 이름 변경 창(T-71) + 열기 깃발 + 저장 대기 프리셋 본문.
    bulk_win: BulkWin,
    open_bulk: bool,
    bulk_pending_preset: Option<String>,
    /// 순서/표시 편집 창(T-71 DLG-069 · 툴바/컬럼/컨텍스트 메뉴 공통) + 열기 깃발.
    order_win: OrderWin,
    open_order: bool,
    /// 압축 미리보기 그리드 창(T-62 C · F3/↗ 결과가 Archive면 텍스트 창 대신) + 열기 깃발.
    archive_win: ArchiveWin,
    open_archive: bool,
    /// 라이선스(T-80): 판정 문맥(단일 원천) · 창 · 열기 깃발 · 파일 창(용도 = 라이선스 파일/설정 폴더) · 열기 깃발.
    licensing: ndir_license::Licensing,
    license_win: LicenseWin,
    /// 메모리 창(상태줄 앱 메모리 칸 → 모덜리스 · mem_win.rs) · 열기 요청.
    mem_win: mem_win::MemWin,
    /// 메모리 창의 다음 표본 시각(창이 열려 있을 때만 쓰인다 · `app/memory.rs`).
    mem_next: Instant,
    open_memory: bool,
    open_license: bool,
    file_win: FileWin,
    file_purpose: Option<app::license::FilePurpose>,
    open_file: bool,
    dlg: DlgWin,
    dlg_pending: Option<(dlg_win::DlgSpec, app::dialogs::DlgReply)>,
    dlg_reply: Option<app::dialogs::DlgReply>,
    open_prefs: bool,
    open_keys: bool,
    /// 탭 우클릭 메뉴(nexa-ctl ContextMenu · 팝업 층 맨 뒤) + 어느 패널·탭의 것인가.
    tab_menu: ContextMenu,
    tab_menu_at: Option<(usize, usize)>,
    /// 행/배경 컨텍스트 메뉴 주인(탭 메뉴와 같은 `ContextMenu` 공유).
    ctx_kind: Option<app::ctxmenu::CtxKind>,
    /// 행 메뉴 "새로 만들기 ▸"의 대상 폴더 + 열 때의 템플릿 목록(SHELL-008 · 메뉴가 열려 있는 동안만).
    ctx_new_dir: Option<PathBuf>,
    ctx_templates: Vec<platform::NewTemplate>,
    /// 행/배경 메뉴를 열 때의 항목 트리 사본(덤프 `ctx`가 서브메뉴 자식을 보여주기 위해 — nexa-ctl 메뉴는 id 목록만 준다).
    ctx_items: Vec<CtxItem>,
    /// 셸 항목을 기다리는 열린 메뉴의 대상(비차단 조회 · dir2 X-61) — `Items`가 오면 같은 자리에서 메뉴를 다시 채운다.
    ctx_pending: Option<platform::MenuTarget>,
    /// 셸 항목을 기다리는 중(메뉴는 아직 안 열림 — 준비되면 **완성된 메뉴를 한 번** 연다 · dir2 방식): (종류, 대상, 시작 시각).
    ctx_wait: Option<(app::ctxmenu::CtxKind, platform::MenuTarget, Instant)>,
    /// 메뉴를 연 자리(다시 채울 때 같은 자리).
    ctx_anchor: (i32, i32),
    /// 다음에 여는 메뉴의 자리(키보드로 열 때 = 캐럿 행 · 한 번 쓰고 비운다 · 없으면 마우스 커서).
    ctx_anchor_next: Option<(i32, i32)>,
    /// 비동기 셸 실행을 건 패널(`Invoked` 통지 처리용).
    ctx_invoke_panel: Option<usize>,
    /// 지금 행 메뉴를 Shift+우클릭(확장 동사)으로 열었는가 — 실행 때 같은 대상을 쓴다.
    ctx_extended: bool,
    /// 창에 마지막으로 알린 IME 조합 창 자리(편집 중이 아니면 `None`) — 바뀔 때만 OS에 알린다.
    ime_last: Option<(i32, i32, i32, i32)>,
    /// 선행 구축(선택 머무름 300 ms · dir2 `CTX_PREBUILD_MS`): 지금 대상 · 머문 시작 · 구축 요청함.
    ctx_dwell_target: Option<platform::MenuTarget>,
    ctx_dwell_since: Instant,
    ctx_dwell_done: bool,
    /// 외부 끌어다 놓기 1차(winit HoveredFile/DroppedFile · 틱에서 처리).
    dnd_hover: Vec<PathBuf>,
    dnd_drop: Vec<PathBuf>,
    /// 퀵 런처 바(T-42 · dir2 WINA-029: 도구 모음 아래 24 · 숨김/항목 0 = 0) + 항목.
    launcherbar: Toolbar,
    launcher_items: Vec<launcher::LauncherItem>,
    /// 마지막 런처 실행 결과(덤프 `launcher` 첫 줄 — 상태줄은 틱마다 갱신돼 시나리오가 못 본다).
    launcher_last: String,
    /// 런처 exe 아이콘 비동기 로딩(T-30 B): 조회 중 깃발 + 마지막으로 본 IconService 버전.
    launcher_icons_pending: bool,
    /// 행 아이콘 서비스 버전(조회 중이던 아이콘 도착 감지 · GAP-003).
    row_icon_ver: u64,
    launcher_icon_ver: u64,
    /// 상태줄 부하 칸(app/statusline.rs): 직전 표본 · 지금 부하 · 다음 조회 시각.
    load_prev: Option<(Instant, platform::sysload::SysSample)>,
    load: Option<platform::sysload::SysLoad>,
    load_next: Instant,
    /// 떠 있는 상태줄 상세 팝업의 칸 id(조회 주기마다 내용을 갱신한다).
    status_popup: Option<String>,
    /// 누르기 시작할 때 떠 있던 상세 팝업의 칸 id — 같은 칸을 다시 누르면 닫기만 한다(토글).
    status_popup_was: Option<String>,
    /// Git 상태(탭 상태바 · NEW-005 2차): 저장소 루트 → 요약 · 조회 중인 루트 · 워커 결과 통로.
    git_detail: std::collections::HashMap<PathBuf, dirinfo::GitDetail>,
    git_busy: std::collections::HashSet<PathBuf>,
    /// `git` 프로세스를 돌려 상태를 조회하는가(시험에서는 끈다 — 실제 프로세스를 띄우지 않는다).
    git_enabled: bool,
    git_tx: std::sync::mpsc::Sender<(PathBuf, Option<dirinfo::GitDetail>)>,
    git_rx: std::sync::mpsc::Receiver<(PathBuf, Option<dirinfo::GitDetail>)>,
    /// 하단 도크 2(dir2 X-6: 패널 밖 **전폭 밴드** · 듀얼 = 좌/우 · 단일 정보 = 좌 하나 전폭 · 내용 = 정보/미리보기/터미널).
    docks: [InfoDock; 2],
    /// 도크 미리보기의 마지막 산출(공급자 id · 줄) — `preview.dump`/`assert.preview:`(T-62).
    dock_preview: [(String, Vec<String>); 2],
    /// 도크 터미널 2(T-61 · 도크 종류 2일 때 지연 시작 · 폴링 틱).
    terms: [TermView; 2],
    /// 터미널 키 포커스(dir2 `term_focus`) — 클릭/→ 버튼으로 얻고 패널 클릭으로 잃는다.
    term_focus: Option<usize>,
    /// TUI 마우스 모드로 누름을 보낸 터미널(뗌도 같은 쪽으로).
    term_mouse_down: Option<usize>,
    /// 앱 내 파일 클립보드 사본(경로, 잘라내기) — OS 클립보드 미지원 OS의 폴백 · 붙여넣기 2순위(M6).
    clip: Option<(Vec<PathBuf>, bool)>,
    /// 진행 중 전송(동시 1건).
    transfer: Option<app::ops::TransferJob>,
    /// 진행 창 안에서 묻고 있는 덮어쓰기 질문의 회신 통로(워커가 기다린다 · app/dialogs.rs `conflict_ask`).
    conflict_inline: Option<std::sync::mpsc::Sender<app::ops::ConflictChoice>>,
    /// 파일 작업 undo/redo(세션 한정 100).
    history: ndir_ops::history::OperationHistory,
    /// 플랫폼 포트 묶음(DR-5 · ADR-0001) — 운영 `Platform::native()` · 시험 `Platform::fake()`.
    platform: Platform,
    /// 폴더 감시 폴링 시각(1 s 간격 · 자동 재열람 PANEL-036).
    watch_next: Instant,
    /// 편집·전송 중이라 미뤄 둔 감시 변경 폴더(`app/watch.rs::watch_tick`이 다음 틱에 다시 본다).
    watch_deferred: Vec<PathBuf>,
    /// "내 PC" 볼륨 구성 감시(`app/watch.rs::drives_tick`): 다음 확인 시각 · 마지막으로 본 지문.
    drives_next: Instant,
    drives_seen: Option<u64>,
}

/// 논리 px → 장치 px(반올림).
fn px(v: f32, s: f32) -> i32 {
    (v * s).round() as i32
}

/// 설정 `list.*` → 목록 옵션.
fn list_opts(s: &Settings) -> ListOpts {
    ListOpts {
        show_hidden: s.flag("list.show_hidden"),
        show_dotfiles: s.flag("list.show_dotfiles"),
        show_protected: s.flag("list.show_protected"),
        folders_first: s.flag("list.folders_first"),
        case_sensitive: s.flag("list.sort_case_sensitive"),
    }
}

/// 설정 `list.nav_up_align`(top/center/bottom · 기본 center).
fn nav_up_align(s: &Settings) -> ScrollAlign {
    match s.get("list.nav_up_align").unwrap_or("center") {
        "top" => ScrollAlign::Top,
        "bottom" => ScrollAlign::Bottom,
        _ => ScrollAlign::Center,
    }
}

/// 설정 `list.view_mode`(tree/flat/tiles).
fn view_mode_of(v: &str) -> ViewMode {
    match v {
        "flat" => ViewMode::Flat,
        "tiles" => ViewMode::Tiles,
        _ => ViewMode::Tree,
    }
}

/// 패널 지표(dir2 `panel_metrics`: row 20 · pad 6 · indent 16 · tab 22 · bar 24 @96dpi — 행은 목록 글꼴보다 작아지지 않게).
fn panel_metrics(settings: &Settings, s: f32) -> PanelMetrics {
    let list_px = settings.font_px("list.font_size");
    PanelMetrics {
        row_h: px(20.0, s).max(px(list_px + 6.0, s)).max(14),
        pad_x: px(6.0, s),
        indent_w: px(16.0, s),
        tab_h: px(22.0, s),
        bar_h: px(24.0, s),
        scale: s,
    }
}

/// 기본 5열(dir2 docs/port/13 §2-5: 340 · 64 · 96 · 140 · 110) — 패널보다 넓으면 이름 열이 줄어든다
/// (10-03 RecordCtx 시험 적발 — 넘친 셀은 nexa-ui 112차 클립 스택이 패널 경계로 자르지만, 보이지 않는 열은 쓸모가 없으니
///  폭 맞춤은 유지 · 열 폭 기억/동기는 T-43).
fn columns_for(panel_w: i32, s: f32) -> Vec<Column> {
    all_columns_for(panel_w, s)
        .into_iter()
        .filter(|c| order::default_visible("cols", order::col_id_key(c.key)))
        .collect()
}

/// 열 정의 전부(기본 숨김 열 포함 · 정의 순 = 이름 · 상태 · 크기 · 수정한 날짜 · 확장자 · 종류) — 숨긴 열을 순서 편집에서 다시
/// 켤 때 이 폭으로 나타난다. 이름 폭은 **기본 표시 열**의 합이 패널을 넘지 않게 줄인다.
fn all_columns_for(panel_w: i32, s: f32) -> Vec<Column> {
    // 상태 열 = 머리글이 다 보이는 폭(언어마다 다르다 — `app::fonts::measure_status_col_w`가 잰 값).
    let (status_w, ext_w, size_w, mod_w, kind_w) = (
        px(app::fonts::status_col_w() as f32, s),
        px(64.0, s),
        px(96.0, s),
        px(140.0, s),
        px(110.0, s),
    );
    let mut name_w = px(340.0, s);
    let shown = status_w + size_w + mod_w;
    if name_w + shown > panel_w {
        name_w = (panel_w - shown - px(8.0, s)).max(px(120.0, s));
    }
    let status = Column::new(filelist::COL_STATUS, tr("col.status"), status_w);
    vec![
        Column::new(filelist::COL_NAME, tr("col.name"), name_w),
        status,
        Column::new(filelist::COL_SIZE, tr("col.size"), size_w).right_aligned(),
        Column::new(filelist::COL_MODIFIED, tr("col.modified"), mod_w),
        Column::new(filelist::COL_EXT, tr("col.ext"), ext_w),
        Column::new(filelist::COL_KIND, tr("col.kind"), kind_w),
    ]
}

impl App {
    /// `start` = 실행 인자 경로(있으면 세션 무시 · dir2 PREFS-054) · `session` = 복원할 세션(탭이 없으면 `start`/현재 폴더).
    fn new(
        mut settings: Settings,
        ui_font: Font,
        start: Option<PathBuf>,
        session: Option<Session>,
        platform: Platform,
    ) -> App {
        let session = session.filter(|s| start.is_none() && !s.is_empty());
        app::fonts::measure_status_col_w(&ui_font, &settings); // 패널(열)을 만들기 전에
        let licensing = Self::licensing_for(start.as_deref());
        let start_dir = start
            .or_else(|| std::env::current_dir().ok())
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));
        let keymap = Keymap::from_settings(&settings);
        let theme = theme::resolve(settings.theme_mode(), None);
        let opts = list_opts(&settings);
        let dual = settings.get("layout.panel_mode").unwrap_or("dual") == "dual";
        let m = panel_metrics(&settings, 1.0);
        let mut inv = Invalidations::default();
        let mut toasts = toast::Toasts::new();
        toasts.configure(3000, 85);
        // 도구 모음 = dir2 28px 셀 / 아이콘 20(08-11 사용자 확정).
        // 런처 시드/마이그레이션(dir2 WINA-054): 첫 실행 = OS별 시드 · 구버전 = 누락분 추가 · 결과는 설정에 저장.
        let (launcher_items, seeded) = launcher::load_or_seed(
            settings.get("launcher.items").unwrap_or(""),
            settings.int("launcher.seed").max(0) as u32,
        );
        if seeded {
            let _ = settings.set("launcher.items", &launcher::encode_items(&launcher_items));
            let _ = settings.set("launcher.seed", &launcher::SEED_VERSION.to_string());
            let _ = settings.save();
        }
        let (launcherbar, launcher_icons_pending) =
            App::make_launcherbar(&launcher_items, &settings);
        let toolbar = App::make_tool_dock(&settings, 1.0);
        let session_keep = session.clone().unwrap_or_default();
        let mut panels = match &session {
            Some(s) => [0usize, 1].map(|i| {
                let ps = &s.panels[i];
                Panel::restore(
                    ps,
                    &start_dir,
                    opts,
                    m,
                    columns_for(600, 1.0),
                    all_columns_for(600, 1.0),
                )
            }),
            None => [
                Panel::new(&start_dir, opts, m, columns_for(600, 1.0)),
                Panel::new(&start_dir, opts, m, columns_for(600, 1.0)),
            ],
        };
        let active0 = session.as_ref().map_or(0, |s| s.active_panel.min(1));
        let mode = view_mode_of(settings.get("list.view_mode").unwrap_or("tree"));
        for p in &mut panels {
            p.set_nav_up_align(nav_up_align(&settings));
            if session.is_none() {
                p.set_view_mode(mode, &mut inv);
            }
        }
        let wt_profile = App::load_wt_profile(&settings);
        let mono_font = App::load_mono_font(&settings, wt_profile.as_ref());
        let (git_tx, git_rx) = std::sync::mpsc::channel();
        let mut app = App {
            window: None,
            surface: None,
            menubar: MenuBar::new(App::build_menus(&settings)),
            toolbar,
            panels,
            splitter: Splitter::new(SplitAxis::Vertical),
            dock_split_h: Splitter::new(SplitAxis::Horizontal),
            dock_split_v: Splitter::new(SplitAxis::Vertical),
            tab_drop_hint: None,
            active: 0,
            dual,
            statusbar: StatusBar::new(),
            toasts,
            settings,
            keymap,
            mono_font,
            wt_profile,
            ui_font: {
                let f = Rc::new(ui_font);
                // 플러그인 `render_svg`(Mermaid 다이어그램)의 텍스트 글꼴(T-62 C-2).
                preview::set_svg_font(Rc::clone(&f));
                f
            },
            theme,
            scale: 1.0,
            viewport: (0, 0),
            cursor: (0, 0),
            shift: false,
            primary: false,
            alt: false,
            ctrl_mac: false,
            started: Instant::now(),
            main_active: true,
            pending_chord: None,
            pressed: None,
            last_click: None,
            exit_requested: false,
            startup_timed: Vec::new(),
            trace_ime: input::trace_ime(),
            session_dir: None,
            session_save: nexa_conf::SaveScheduler::new(1000, 5000),
            session_keep,
            exit_code: 0,
            startup_ready: Vec::new(),
            startup_blocked: Vec::new(),
            startup_blocked_since: Instant::now(),
            ready_fired: false,
            prefs_win: PrefsWin::new(),
            keys_win: KeysWin::new(),
            check_win: CheckWin::new(),
            open_check: false,
            preview_win: PreviewWin::new(),
            open_preview: false,
            progress_win: ProgressWin::new(),
            open_progress: false,
            bulk_win: BulkWin::new(),
            open_bulk: false,
            bulk_pending_preset: None,
            order_win: OrderWin::new(),
            open_order: false,
            archive_win: ArchiveWin::new(),
            open_archive: false,
            licensing,
            license_win: LicenseWin::new(),
            mem_win: mem_win::MemWin::new(),
            mem_next: Instant::now(),
            open_memory: false,
            open_license: false,
            file_win: FileWin::new(),
            file_purpose: None,
            open_file: false,
            dlg: DlgWin::new(),
            dlg_pending: None,
            dlg_reply: None,
            open_prefs: false,
            open_keys: false,
            tab_menu: ContextMenu::new(),
            tab_menu_at: None,
            ctx_kind: None,
            ctx_new_dir: None,
            ctx_templates: Vec::new(),
            ctx_items: Vec::new(),
            ctx_pending: None,
            ctx_wait: None,
            ctx_anchor: (0, 0),
            ctx_anchor_next: None,
            ctx_invoke_panel: None,
            ctx_extended: false,
            ime_last: None,
            ctx_dwell_target: None,
            ctx_dwell_since: Instant::now(),
            ctx_dwell_done: false,
            dnd_hover: Vec::new(),
            dnd_drop: Vec::new(),
            platform,
            watch_next: Instant::now(),
            watch_deferred: Vec::new(),
            drives_next: Instant::now(),
            drives_seen: None,
            clip: None,
            transfer: None,
            conflict_inline: None,
            history: ndir_ops::history::OperationHistory::default(),
            launcherbar,
            launcher_items,
            launcher_last: String::new(),
            launcher_icons_pending,
            row_icon_ver: 0,
            launcher_icon_ver: nexa_fs::shell::IconService::global().version(),
            load_prev: None,
            load: None,
            load_next: Instant::now(),
            status_popup: None,
            status_popup_was: None,
            git_detail: std::collections::HashMap::new(),
            git_busy: std::collections::HashSet::new(),
            git_enabled: !cfg!(test),
            git_tx,
            git_rx,
            docks: [
                InfoDock::new(tr("dock.info"), 20, 6),
                InfoDock::new(tr("dock.info"), 20, 6),
            ],
            dock_preview: [(String::new(), Vec::new()), (String::new(), Vec::new())],
            terms: [TermView::new(), TermView::new()],
            term_focus: None,
            term_mouse_down: None,
        };
        for d in &mut app.docks {
            d.set_kinds(
                vec![tr("dock.info"), tr("dock.preview"), tr("dock.terminal")],
                &mut inv,
            );
        }
        // 창보다 긴 우클릭 메뉴의 스크롤 막대 = 스크롤할 때만 나타나는 오버레이(사용자 10-04).
        app.tab_menu.set_overlay_scrollbar(true);
        app.tab_menu
            .set_wrap_around(app.settings.flag("menu.wrap_around"));
        app.apply_window_sizes();
        app.apply_scroll_settings(); // 고속 스크롤 · 시스템 휠 줄 수(dir2 X-63)
        app.apply_icon_overrides(); // 행 아이콘 계층 1(사용자 지정)
        app.apply_tab_style(); // 탭 여러 줄(기본) · 한 줄일 때 ◀ ▶ 자리
        app.apply_font_decor(); // dir2 X-12 폴더 굵게 · 헤더 굵게/이탤릭(KEY-061~063)
        app.sync_menu_shortcuts();
        app.sync_menu_checks();
        app.set_active(if dual { active0 } else { 0 });
        app.update_status();
        app
    }

    fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    /// 활성 패널(키보드가 가는 곳 · 단일 모드는 늘 0).
    fn set_active(&mut self, i: usize) {
        let i = if self.dual { i.min(1) } else { 0 };
        self.active = i;
        let mut inv = Invalidations::default();
        for (k, p) in self.panels.iter_mut().enumerate() {
            p.set_focused(k == i, &mut inv);
        }
    }

    /// 상태줄 = 활성 패널 요약(왼쪽) · 탭 n/N(오른쪽).
    fn update_status(&mut self) {
        // 내 PC(가상 최상위)를 보는 탭 = 드라이브 용량을 Disk 포트로 한 번 채운다(PANEL-044).
        {
            let mut inv = Invalidations::default();
            let disk = &*self.platform.disk;
            for p in &mut self.panels {
                // 전환·닫기로 드러난 배경 탭은 그동안의 바깥 변경을 모른다 → 여기서 다시 읽는다(dir2 X-44 S1 · win.rs:4967-4976).
                p.refresh_stale(&mut inv);
                p.fill_drive_space(&|root| disk.space(root), &mut inv);
            }
        }
        // 세션 더러움 수거 길목(dir2 PREFS-051: update_status가 양 패널 플래그를 거둔다).
        self.session_collect_dirty(Instant::now());
        let p = &self.panels[self.active];
        let left = p.status_text();
        let right = trf(
            "status.tab",
            &[
                &(p.active_index() + 1).to_string(),
                &p.tab_count().to_string(),
            ],
        );
        let mut inv = Invalidations::default();
        self.statusbar.set_text(&left, &right, &mut inv);
        // 오른쪽 칸(탭 · CPU · 메모리 · 디스크 I/O · 라이선스 — `statusbar.layout`) + 패널마다 탭 상태바.
        let segs = self.status_segments();
        self.statusbar.set_segments(segs, &mut inv);
        for p in &mut self.panels {
            p.sync_status(&mut inv);
        }
        self.git_sync(&mut inv);
        self.sync_dir_views();
        self.sync_view_checks();
        self.update_docks();
    }

    /// 도크 내용(dir2 `update_dock_info`): 단일 정보 = 좌 도크의 원천은 **활성 패널** · 종류 0 정보 · 1 미리보기 · 2 터미널(T-61).
    /// 키 = 종류 + 대상(선택 경로·없으면 현재 폴더) → 같은 대상의 갱신은 스크롤·선택 유지.
    pub(crate) fn update_docks(&mut self) {
        let single_info =
            !self.dual || self.settings.get("layout.info_mode").unwrap_or("dual") != "dual";
        let mut inv = Invalidations::default();
        preview::set_dark(self.theme.is_dark); // 플러그인 `is_dark()` 신호(dir2 PLUG-009)
        let map = self.settings.get("preview.map").unwrap_or("").to_string();
        let disabled = self
            .settings
            .get("plugins.disabled")
            .unwrap_or("")
            .to_string();
        for i in 0..2 {
            if self.docks[i].bounds().h <= 0 {
                continue;
            }
            let src = if single_info { self.active } else { i };
            let selected = self.panels[src].selected_paths();
            let current = self.panels[src].root_path();
            let kind = self.docks[i].active_kind();
            let (lines, image) = match kind {
                1 => {
                    let out = dockinfo::preview_content(&selected, &map, &disabled);
                    self.dock_preview[i] = (out.provider, out.lines.clone());
                    (out.lines, out.image)
                }
                2 => (Vec::new(), None), // 터미널 = 호스트가 내용 영역을 직접 그린다(paint_terms)
                _ => (dockinfo::info_lines(&selected, &current), None),
            };
            let subject = selected.first().map_or_else(
                || current.display().to_string(),
                |p| p.display().to_string(),
            );
            let key = format!("{kind}|{}|{subject}", selected.len());
            self.docks[i].set_content(&key, lines, &mut inv);
            self.docks[i].set_image(image, &mut inv);
            self.docks[i].set_popout(kind == 1, &mut inv);
        }
    }

    /// 배치(Shell 쪽) — 창 크기를 읽어 [`App::layout_core`].
    fn layout(&mut self) {
        // 창이 없으면(시험) 마지막 `viewport`로 — 명령이 부르는 `layout()`도 창 없이 돌아야 한다(10-03 `view.panel_single` 시험 적발).
        if let Some(win) = &self.window {
            let size = win.inner_size();
            self.viewport = (size.width as i32, size.height as i32);
        }
        self.layout_core();
    }

    /// 배치(시험 쪽) — 창 없이 크기·배율을 넣는다(CI-105 골든 · T-41).
    #[cfg_attr(not(test), allow(dead_code))]
    fn layout_for(&mut self, w: i32, h: i32, scale: f32) {
        self.viewport = (w, h);
        self.scale = scale;
        self.layout_core();
    }

    /// 스플리터 x(창 기준 · dir2 `splitter_x`) — 비율에서 계산 · 패널 최소 폭 클램프.
    fn splitter_x(&self, w: i32) -> i32 {
        let pct = self.settings.int("layout.panel_split_pct").clamp(10, 90) as i32;
        let min = px(MIN_PANEL, self.scale);
        (w * pct / 100).clamp(min.min(w / 2), (w - min).max(w / 2))
    }

    /// 전체 배치(dir2 `win.rs::layout`): 메뉴 / 도구 모음 / [좌 ║ 우 패널] / 상태바. winit 무관.
    fn layout_core(&mut self) {
        // 상태 열 기본 폭 = 지금 언어의 머리글 폭(언어 · 목록 글꼴 · 머리 굵게가 바뀌면 달라진다).
        app::fonts::measure_status_col_w(&self.ui_font, &self.settings);
        let (w, h) = self.viewport;
        let s = self.scale;
        let mut inv = Invalidations::default();
        self.menubar.set_scale(s);
        self.toolbar.set_scale(s);
        self.statusbar.set_scale(s);
        self.tab_menu.set_scale(s);
        let menu_h = px(self.settings.font_px("ui.menu_font_size") + 11.0, s).min(h.max(0));
        self.menubar
            .set_bounds(Rect::new(0, 0, w, menu_h), &mut inv);
        // `preferred_height`는 논리 px(nexa-ctl 규약) → 장치 px로(nexa-sql 09-16 맥 2x 교훈).
        let tool_h = px(self.toolbar.preferred_height() as f32, s);
        self.toolbar
            .set_bounds(Rect::new(0, menu_h, w, tool_h), &mut inv);
        // 퀵 런처 바(dir2 WINA-065: 24 · 숨김이거나 실행 항목 0이면 0).
        let has_items = self.launcher_items.iter().any(|i| !i.is_separator());
        let launch_h = if self.settings.flag("launcher.visible") && has_items {
            // 높이 = 아이콘 크기 + 여백 8(기본 16 → dir2의 24) · `preferred_height`는 논리 px.
            px(self.launcherbar.preferred_height() as f32, s)
        } else {
            0
        };
        self.launcherbar.set_scale(s);
        self.launcherbar
            .set_bounds(Rect::new(0, menu_h + tool_h, w, launch_h), &mut inv);
        let top = menu_h + tool_h + launch_h;
        let status_h = px(22.0, s);
        let bottom = (h - status_h).max(top);
        self.statusbar
            .set_bounds(Rect::new(0, bottom, w, h - bottom), &mut inv);
        let area_h = (bottom - top).max(0);
        let gap = px(SPLIT_TH, s).max(2);
        let g2 = gap / 2;
        let half = px(SPLIT_HALF, s);
        // 세 스플리터 같은 모양: 틈(gap)을 채우는 띠 · 평상시 border(도크 위 경계만 text_dim — dir2 W:4917 "다크에서 보이게") ·
        // hover = accent가 서서히 · 드래그 = accent.
        for (sp, dim_rest) in [
            (&mut self.splitter, false),
            (&mut self.dock_split_h, true),
            (&mut self.dock_split_v, false),
        ] {
            sp.set_band(Some(SplitBand {
                thickness: gap,
                dim_rest,
                hover_alpha: SPLIT_HOVER_ALPHA,
            }));
        }
        let m = panel_metrics(&self.settings, s);
        // 하단 도크 = 전폭 밴드(dir2 X-6): 높이 = 영역 × `layout.dock_height_pct`(행 3줄 ~ 절반) · 숨김 = 0.
        let band_h = if self.settings.flag("dock.visible") {
            let pct = self.settings.int("layout.dock_height_pct").clamp(15, 50) as i32;
            (area_h * pct / 100).clamp((m.row_h * 3).min(area_h / 2), area_h / 2)
        } else {
            0
        };
        let ph = (area_h - band_h).max(0);
        let rects = if self.dual {
            let sx = self.splitter_x(w);
            // 잡는 띠 = 틈 + 양쪽 half(틈의 가운데 = 띠의 가운데 → 그리는 띠가 틈에 꼭 맞는다).
            self.splitter
                .set_rect(Rect::new(sx - g2 - half, top, gap + half * 2, ph));
            [
                Rect::new(0, top, (sx - g2).max(0), ph),
                Rect::new(sx - g2 + gap, top, (w - sx + g2 - gap).max(0), ph),
            ]
        } else {
            self.splitter.set_rect(Rect::default());
            [Rect::new(0, top, w, ph), Rect::default()]
        };
        // 도크 좌/우 = 파일 좌/우와 **독립** 비율(`layout.dock_split_pct`) · 단일 정보 = 좌 하나 전폭 · 우 = 0.
        let single_info =
            !self.dual || self.settings.get("layout.info_mode").unwrap_or("dual") != "dual";
        let dock_rects = if band_h > 0 {
            let band_y = top + ph;
            let dock_y = band_y + gap;
            let dock_h = (band_h - gap).max(0);
            // 패널 ↔ 도크 경계(전폭 · 틈 = band_y..band_y+gap).
            self.dock_split_h
                .set_rect(Rect::new(0, band_y - half, w, gap + half * 2));
            if single_info {
                self.dock_split_v.set_rect(Rect::default());
                [Rect::new(0, dock_y, w, dock_h), Rect::default()]
            } else {
                let dsx = self.dock_split_x(w);
                self.dock_split_v.set_rect(Rect::new(
                    dsx - g2 - half,
                    dock_y,
                    gap + half * 2,
                    dock_h,
                ));
                [
                    Rect::new(0, dock_y, (dsx - g2).max(0), dock_h),
                    Rect::new(dsx - g2 + gap, dock_y, (w - dsx + g2 - gap).max(0), dock_h),
                ]
            }
        } else {
            self.dock_split_h.set_rect(Rect::default());
            self.dock_split_v.set_rect(Rect::default());
            [Rect::default(), Rect::default()]
        };
        for (d, r) in self.docks.iter_mut().zip(dock_rects) {
            d.set_metrics(m.row_h, m.pad_x, &mut inv);
            d.set_bounds(r, &mut inv);
        }
        let tab_status = self.settings.flag("layout.tab_statusbar");
        for (p, r) in self.panels.iter_mut().zip(rects) {
            p.set_tab_status(tab_status, &mut inv);
            p.set_metrics(m, &mut inv);
            p.set_default_columns(columns_for(r.w, s), all_columns_for(r.w, s), &mut inv);
            p.set_bounds(r, &mut inv);
        }
    }

    /// 도크 분할선 x(dir2 `dock_split_x` W:1905) — `layout.dock_split_pct`(15~85) · 창 폭 1/8 ~ 7/8 클램프.
    fn dock_split_x(&self, w: i32) -> i32 {
        let pct = self.settings.int("layout.dock_split_pct").clamp(15, 85) as i32;
        (w * pct / 100).clamp(w / 8, w * 7 / 8)
    }

    /// 지금 그 스플리터가 화면에 있는가(숨김 조건: 단일 패널 · 도크 숨김 · 단일 정보).
    pub(crate) fn split_shown(&self, kind: SplitKind) -> bool {
        match kind {
            SplitKind::Panel => self.dual && !self.splitter.rect().is_empty(),
            SplitKind::DockHeight => !self.dock_split_h.rect().is_empty(),
            SplitKind::DockSplit => !self.dock_split_v.rect().is_empty(),
        }
    }

    pub(crate) fn split_of(&self, kind: SplitKind) -> &Splitter {
        match kind {
            SplitKind::Panel => &self.splitter,
            SplitKind::DockHeight => &self.dock_split_h,
            SplitKind::DockSplit => &self.dock_split_v,
        }
    }

    pub(crate) fn split_of_mut(&mut self, kind: SplitKind) -> &mut Splitter {
        match kind {
            SplitKind::Panel => &mut self.splitter,
            SplitKind::DockHeight => &mut self.dock_split_h,
            SplitKind::DockSplit => &mut self.dock_split_v,
        }
    }

    /// 자리의 스플리터(dir2 W:8020-8042 우선순위 = 도크 높이 → 도크 좌우 → 패널 좌우 · 드래그 중인 것이 있으면 그것).
    pub(crate) fn split_at(&self, p: Point) -> Option<SplitKind> {
        const ORDER: [SplitKind; 3] = [
            SplitKind::DockHeight,
            SplitKind::DockSplit,
            SplitKind::Panel,
        ];
        ORDER
            .into_iter()
            .find(|k| self.split_of(*k).is_dragging())
            .or_else(|| {
                ORDER
                    .into_iter()
                    .find(|k| self.split_shown(*k) && self.split_of(*k).rect().contains(p))
            })
    }

    /// 자석 스냅(dir2 `snap_split_x` W:6148): `x`가 후보 중 하나에 `SNAP_PX` 안이면 그 자리로 · Alt = 해제.
    fn snap_x(&self, x: i32, candidates: &[i32]) -> i32 {
        if self.alt {
            return x;
        }
        let snap = px(SNAP_PX, self.scale);
        candidates
            .iter()
            .copied()
            .filter(|c| (x - c).abs() <= snap)
            .min_by_key(|c| (x - c).abs())
            .unwrap_or(x)
    }

    /// 스플리터 드래그(`SplitEvent::Drag(v)` · v = 잡는 띠의 새 시작 좌표) → 비율 설정(저장은 `End`에서).
    /// - 패널 좌우: 50 % 와 **도크 분할선**(보일 때)에 자석 · 패널 최소 폭 · 10~90 %.
    /// - 도크 좌우: 50 % 와 **패널 스플리터**(듀얼일 때)에 자석 · 15~85 %.
    /// - 도크 높이: (영역 바닥 − y) / 영역 높이 · 15~50 %(행 3줄 하한은 배치가 건다) · 자석 없음.
    fn split_drag(&mut self, kind: SplitKind, v: i32) {
        let (w, _) = self.viewport;
        if w <= 0 {
            return;
        }
        let gap = px(SPLIT_TH, self.scale).max(2);
        let center = v + px(SPLIT_HALF, self.scale) + gap / 2;
        match kind {
            SplitKind::Panel => {
                let mut cands = vec![w / 2];
                if self.split_shown(SplitKind::DockSplit) {
                    cands.push(self.dock_split_x(w));
                }
                let sx = self.snap_x(center, &cands);
                let min = px(MIN_PANEL, self.scale);
                let sx = sx.clamp(min.min(w / 2), (w - min).max(w / 2));
                let pct = pct_of(sx, w).clamp(10, 90);
                let _ = self
                    .settings
                    .set("layout.panel_split_pct", &pct.to_string());
            }
            SplitKind::DockSplit => {
                let mut cands = vec![w / 2];
                if self.dual {
                    cands.push(self.splitter_x(w));
                }
                let sx = self.snap_x(center, &cands);
                let pct = pct_of(sx, w).clamp(15, 85);
                let _ = self.settings.set("layout.dock_split_pct", &pct.to_string());
            }
            SplitKind::DockHeight => {
                let top = self.panels[0].bounds().y;
                let bottom = self.statusbar.bounds().y;
                let area = bottom - top;
                if area <= 0 {
                    return;
                }
                // 틈의 윗변 = 도크 밴드의 시작.
                let band_y = v + px(SPLIT_HALF, self.scale);
                let pct = pct_of(bottom - band_y, area).clamp(15, 50);
                let _ = self
                    .settings
                    .set("layout.dock_height_pct", &pct.to_string());
            }
        }
        self.layout_core();
    }

    /// 창을 닫을 때 자리·크기 기억(`window.main_size`/`main_pos` · dir2 계승).
    fn persist_window(&mut self) {
        if let Some(w) = &self.window {
            let (lw, lh) = wingeom::logical_size(w);
            let _ = self
                .settings
                .set("window.main_size", &wingeom::format_size(lw, lh));
            if let Some((x, y)) = wingeom::outer_pos(w) {
                let _ = self
                    .settings
                    .set("window.main_pos", &wingeom::format_pos(x, y));
            }
            let _ = self.settings.save();
        }
        self.session_flush();
    }
}

/// i18n 활성화 — 설정 `ui.lang`(system = OS 언어) + 설정 폴더 `lang/` 오버레이(dir2 규약).
fn init_i18n(settings: &Settings) {
    let home = ndir_settings::config_dir().unwrap_or_else(std::env::temp_dir);
    let avail = ndir_i18n::discover(&home);
    let system = ndir_i18n::syslang::system_lang_code();
    let code = ndir_i18n::resolve_code(settings.lang_setting(), &system, &avail);
    // 대체 언어 = 시스템 기본 언어(쓸 수 있을 때) → 영어(사용자 10-04 결정).
    let sys_code = ndir_i18n::resolve_code("system", &system, &avail);
    ndir_i18n::activate(ndir_i18n::load_with_system(&code, &sys_code, &home));
}

/// nexa-ctl 내장 메뉴(우클릭 편집) 라벨을 앱 i18n에 잇는다 — `fn` 포인터 계약이라 `'static` 글이 필요해 누수한다.
/// **언어를 바꿀 때마다 다시 부른다**(종전 = `OnceLock`이라 재시작 전까지 기동 언어 · T-134) — 같은 글이면 다시 누수하지 않는다.
fn install_ctl_labels() {
    use nexa_ctl::controls::CtlMsg as C;
    static LABELS: std::sync::Mutex<[&'static str; 4]> =
        std::sync::Mutex::new(["Select All", "Copy", "Cut", "Paste"]);
    let keys = [
        "menu.edit.selectAll",
        "menu.edit.copy",
        "menu.edit.cut",
        "menu.edit.paste",
    ];
    {
        let mut cur = LABELS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for (slot, key) in cur.iter_mut().zip(keys) {
            let text = tr(key);
            if *slot != text {
                *slot = Box::leak(text.into_boxed_str());
            }
        }
    }
    nexa_ctl::controls::set_ctl_labels(|m| {
        let l = *LABELS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match m {
            C::CtxSelectAll => l[0],
            C::CtxCopy => l[1],
            C::CtxCut => l[2],
            C::CtxPaste => l[3],
        }
    });
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::parse(&args) {
        Err(msg) => {
            eprintln!("nexa-dir: {msg}\n{}", cli::USAGE);
            ExitCode::from(2)
        }
        Ok(cli::Mode::Version) => {
            println!("nexa-dir {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(cli::Mode::Help) => {
            println!("{}", cli::USAGE);
            ExitCode::SUCCESS
        }
        Ok(cli::Mode::Smoke) => run_smoke(),
        Ok(cli::Mode::SelfCheck(opts)) => {
            let report = selfcheck::run(&opts);
            if opts.json {
                println!("{}", report.to_json());
            } else {
                print!("{}", report.to_table());
            }
            ExitCode::from(report.exit_code())
        }
        Ok(cli::Mode::Gui(path)) => run_gui(path.as_deref().and_then(cli::start_dir)),
    }
}

/// `--smoke`: 창 없이 "기동에 필요한 것"을 순서대로 확인하고 0으로 끝난다(CI 3-OS 공통 게이트 · docs/18 §3).
/// = 자가 점검의 `--ci` 부분집합 전체(미구현 그룹은 SKIP) — 항목이 늘면 스모크도 같이 넓어진다.
fn run_smoke() -> ExitCode {
    let report = selfcheck::run(&selfcheck::Options {
        ci: true,
        only: None,
        json: false,
        with_clipboard: false,
    });
    if report.failed() == 0 {
        println!("smoke ok (nexa-dir {})", env!("CARGO_PKG_VERSION"));
        ExitCode::SUCCESS
    } else {
        print!("{}", report.to_table());
        ExitCode::FAILURE
    }
}

/// GUI 기동(SKEL-002 순서): 설정 → i18n → 글꼴 → 이벤트 루프 → `App` → `run_app`.
fn run_gui(start: Option<PathBuf>) -> ExitCode {
    // 설정 — 폴더를 모르면 임시 경로의 기본값(저장은 실패해도 앱은 뜬다).
    let settings = Settings::open_default().unwrap_or_else(|_| {
        Settings::open(
            std::env::temp_dir()
                .join("nexa-dir")
                .join(ndir_settings::FILE_NAME),
        )
    });
    init_i18n(&settings);
    // 패닉 훅(CI-112 · 릴리스 panic=abort라 유일한 기록 수단) — `<HOME>/crash/crash-<unix>.txt` + stderr.
    crash::install(ndir_settings::config_dir());
    install_ctl_labels();
    input::set_natural_scroll(settings.flag("input.scroll_natural"));
    // UI 글꼴 = 설정 `ui.font_face`(비면 OS 사슬 · 못 찾으면 사슬로 fail-over).
    let ui_pref = settings.get("ui.font_face").map(str::to_string);
    let Some(ui) = nexa_font::ui_font(ui_pref.as_deref()).or_else(|| nexa_font::ui_font(None))
    else {
        eprintln!("nexa-dir: no usable UI font");
        return ExitCode::FAILURE;
    };
    app::row_icons::install(); // 패널 행 셸 아이콘(GAP-003 · dir2 M1-7)
    app::fonts::set_ui_font_px(ui.font.em_to_px(settings.font_px("ui.font_size")));
    app::fonts::init_icon_glyphs(&ui.font); // 네비·쉐브론 글리프(MDL2 있으면 dir2 모양 · 없으면 유니코드 — 두부 방지)
                                            // Linux: 창 백엔드 = X11 우선(모달 창을 메인의 transient로 붙이려면 · Wayland 경로는 winit 0.30이 부모 창을 지원하지 않는다).
    let built = {
        let mut b = EventLoop::<Wake>::with_user_event();
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            use winit::platform::x11::EventLoopBuilderExtX11 as _;
            if std::env::var_os("DISPLAY").is_some() {
                b.with_x11();
            }
        }
        b.build()
            .or_else(|_| EventLoop::<Wake>::with_user_event().build())
    };
    let Ok(el) = built else {
        eprintln!("nexa-dir: event loop creation failed");
        return ExitCode::FAILURE;
    };
    // 시작 폴더: 명령행 경로(dir2 KEY-301 · 세션보다 우선) → 세션 복원(T-45) → 현재 폴더/홈(`App::new`가 고른다).
    // 세션 복원(dir2 PREFS-054 — 창 생성 전) · 저장 폴더 = 설정 폴더.
    let session_dir = ndir_settings::config_dir();
    let session = session_dir.as_deref().and_then(Session::load);
    let mut app = App::new(settings, ui.font, start, session, Platform::native());
    app.session_dir = session_dir;
    // 지난 실행의 크래시 기록을 한 번 안내(토스트 · 자세한 것은 파일).
    if let Some(p) = app.session_dir.as_deref().and_then(crash::take_unreported) {
        app.toasts.push(
            toast::ToastKind::Warn,
            tr("crash.title"),
            trf("crash.reported", &[&p.display().to_string()]),
        );
    }
    if let Err(e) = el.run_app(&mut app) {
        eprintln!("nexa-dir: event loop error: {e}");
        return ExitCode::FAILURE;
    }
    // `quit:<코드>` · 단언 실패(3) = 러너가 종료 코드로 판정(docs/18 §5).
    ExitCode::from(app.exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn px_rounds_and_columns_fit() {
        assert_eq!(px(24.0, 1.0), 24);
        assert_eq!(px(24.0, 1.5), 36);
        assert_eq!(px(26.0, 1.25), 33);
        // 넓은 패널 = dir2 기본 폭 그대로 · 좁은 패널 = 이름 열이 줄되 120 이상.
        let wide = columns_for(1000, 1.0);
        assert_eq!(wide[0].width, 340);
        let narrow = columns_for(500, 1.0);
        assert!(narrow[0].width >= 120 && narrow[0].width < 340);
        // 기본 표시 = 이름 · 상태 · 크기 · 수정한 날짜 · 정의 전부 = + 확장자 · 종류(숨김 · 다시 켤 때의 폭).
        let keys: Vec<u32> = narrow.iter().map(|c| c.key).collect();
        assert_eq!(
            keys,
            [
                filelist::COL_NAME,
                filelist::COL_STATUS,
                filelist::COL_SIZE,
                filelist::COL_MODIFIED
            ]
        );
        let all = all_columns_for(1000, 1.0);
        assert_eq!(all.len(), 6);
        assert!(all[1].sortable, "상태 열도 정렬한다(SortKey::Status)");
        assert_eq!((all[4].width, all[5].width), (64, 110));
        assert_eq!(view_mode_of("tiles"), ViewMode::Tiles);
        assert_eq!(view_mode_of("x"), ViewMode::Tree);
    }
}
