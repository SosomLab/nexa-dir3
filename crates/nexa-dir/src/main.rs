//! Nexa Dir — 앱 진입점(M3 · winit 호스트 + nexa-ui 위에 그리는 수직 슬라이스).
//!
//! 구조 = nexa-sql 3층(docs/port/40 SKEL-401): 이 파일(모듈 선언 · `struct App` · `layout_core()` · `main()`) /
//! `app/*.rs`(`impl App` 조각 — 이벤트 루프 · 입력 · 그리기 · 메뉴/명령 · 기동 명령) / 호스트 껍질은 파일 하나씩
//! (`present` · `winhost` · `wingeom` · `winfocus` · `theme` · `icon` · `input` · `clipboard` · `toast` — nexa-sql 복사 · SKEL-403).
//! 화면 = dir2 `win.rs::layout`(docs/port/13 PANEL-001·002): 메뉴 / 도구 모음 / [좌 패널 ║ 우 패널] / 상태바 · 패널 = `panel.rs`.
//! 규칙: 인자 해석·판정은 순수 함수(`cli.rs` · `selfcheck.rs`) · 그리기는 `RedrawRequested`에서만 · 유휴는 `WaitUntil`(SKEL-414).

mod app;
mod check_win;
mod cli;
#[allow(dead_code)] // M6 파일 작업·M5 터미널 복사에서 소비(지금은 호스트 껍질만 들여놓음).
mod clipboard;
#[cfg(all(unix, not(target_os = "macos")))]
#[allow(dead_code)]
mod clipboard_x11;
mod copybtn;
mod crash;
mod dlg_win;
mod dockinfo;
mod filelist;
mod icon;
#[allow(dead_code)]
mod input;
mod keys_win;
mod nav;
mod panel;
mod platform;
mod prefs_win;
#[allow(dead_code)]
// `set_mode`(macOS IOSurface 설정 · dir2에 키 없음 → Q-8) · `backend`(프레임 계측 T-46).
mod present;
mod preview;
mod preview_win;
mod selfcheck;
mod session;
mod termview;
mod theme;
#[allow(dead_code)]
mod toast;
#[allow(dead_code)]
mod winfocus;
#[allow(dead_code)]
mod wingeom;
#[allow(dead_code)] // 보조 창(설정 · 단축키 · About)이 T-44에서 쓴다.
mod winhost;

use check_win::CheckWin;
use dlg_win::DlgWin;
use filelist::ListOpts;
use keys_win::KeysWin;
use ndir_i18n::{tr, trf};
use ndir_settings::keymap::{Chord, Keymap};
use ndir_settings::{Settings, ThemeMode};
use nexa_ctl::controls::{
    ComboItem, ContextMenu, Control, CtxItem, MenuBar, MenuDef, MenuEntry, SplitAxis, SplitEvent,
    Splitter, StatusBar, ToolIcon, ToolItem, Toolbar,
};
use nexa_ctl::draw::DrawCtx;
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::{InputEvent, Invalidations, Widget};
use nexa_explorer::InfoDock;
use nexa_gfx::{Font, Surface};
use nexa_grid::{Column, RowSource, ScrollAlign, ViewMode};
use panel::{Panel, PanelMetrics};
use platform::Platform;
use prefs_win::PrefsWin;
use preview_win::PreviewWin;
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

/// UI 스레드를 깨우는 사용자 이벤트(배경 작업이 보낸다 — M4 전송·감시 스레드 · SKEL-415).
#[derive(Debug)]
struct Wake;

/// 포인터 캡처 대상(눌린 곳이 뗄 때까지 받는다).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Area {
    Menu,
    Tool,
    Panel(usize),
    Split,
    Status,
    Dock(usize),
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
    toolbar: Toolbar,
    /// 파일 패널 2(듀얼 · dir2 좌/우) — 단일 모드면 `panels[1]`은 빈 사각형(상태는 보존).
    panels: [Panel; 2],
    /// 좌/우 스플리터(`layout.panel_split_pct` · 드래그 · 50% 스냅).
    splitter: Splitter,
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
    /// 파일 작업 undo/redo(세션 한정 100).
    history: ndir_ops::history::OperationHistory,
    /// 플랫폼 포트 묶음(DR-5 · ADR-0001) — 운영 `Platform::native()` · 시험 `Platform::fake()`.
    platform: Platform,
    /// 폴더 감시 폴링 시각(1 s 간격 · 자동 재열람 PANEL-036).
    watch_next: Instant,
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
/// (nexa-grid는 셀을 패널 경계로 자르지 않는다 · 클립 스택 T-31 · 10-03 RecordCtx 시험 적발).
fn columns_for(panel_w: i32, s: f32) -> Vec<Column> {
    let (ext_w, size_w, mod_w, kind_w) = (px(64.0, s), px(96.0, s), px(140.0, s), px(110.0, s));
    let mut name_w = px(340.0, s);
    let total = name_w + ext_w + size_w + mod_w + kind_w;
    if total > panel_w {
        name_w = (panel_w - ext_w - size_w - mod_w - kind_w - px(8.0, s)).max(px(120.0, s));
    }
    vec![
        Column::new(filelist::COL_NAME, tr("col.name"), name_w),
        Column::new(filelist::COL_EXT, tr("col.ext"), ext_w),
        Column::new(filelist::COL_SIZE, tr("col.size"), size_w).right_aligned(),
        Column::new(filelist::COL_MODIFIED, tr("col.modified"), mod_w),
        Column::new(filelist::COL_KIND, tr("col.kind"), kind_w),
    ]
}

impl App {
    /// `start` = 실행 인자 경로(있으면 세션 무시 · dir2 PREFS-054) · `session` = 복원할 세션(탭이 없으면 `start`/현재 폴더).
    fn new(
        settings: Settings,
        ui_font: Font,
        start: Option<PathBuf>,
        session: Option<Session>,
        platform: Platform,
    ) -> App {
        let session = session.filter(|s| start.is_none() && !s.is_empty());
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
        let mut toolbar = Toolbar::new(App::build_toolbar(&settings));
        toolbar.set_icon_size(20);
        toolbar.set_padding(2, 2);
        let session_keep = session.clone().unwrap_or_default();
        let mut panels = match &session {
            Some(s) => [0usize, 1].map(|i| {
                let ps = &s.panels[i];
                Panel::restore(ps, &start_dir, opts, m, columns_for(600, 1.0))
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
        let mut app = App {
            window: None,
            surface: None,
            menubar: MenuBar::new(App::build_menus(&settings)),
            toolbar,
            panels,
            splitter: Splitter::new(SplitAxis::Vertical),
            active: 0,
            dual,
            statusbar: StatusBar::new(),
            toasts,
            settings,
            keymap,
            ui_font: Rc::new(ui_font),
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
            ready_fired: false,
            prefs_win: PrefsWin::new(),
            keys_win: KeysWin::new(),
            check_win: CheckWin::new(),
            open_check: false,
            preview_win: PreviewWin::new(),
            open_preview: false,
            dlg: DlgWin::new(),
            dlg_pending: None,
            dlg_reply: None,
            open_prefs: false,
            open_keys: false,
            tab_menu: ContextMenu::new(),
            tab_menu_at: None,
            ctx_kind: None,
            platform,
            watch_next: Instant::now(),
            clip: None,
            transfer: None,
            history: ndir_ops::history::OperationHistory::default(),
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
        app.apply_window_sizes();
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
        let top = menu_h + tool_h;
        let status_h = px(22.0, s);
        let bottom = (h - status_h).max(top);
        self.statusbar
            .set_bounds(Rect::new(0, bottom, w, h - bottom), &mut inv);
        let area_h = (bottom - top).max(0);
        let gap = px(SPLIT_TH, s).max(2);
        let g2 = gap / 2;
        let half = px(3.0, s);
        let m = panel_metrics(&self.settings, s);
        // 하단 도크 = 전폭 밴드(dir2 X-6): 높이 = 영역 × `layout.dock_height_pct`(행 3줄 ~ 절반) · 숨김 = 0.
        let band_h = if self.settings.flag("dock.visible") {
            let pct = self.settings.int("layout.dock_height_pct").clamp(5, 50) as i32;
            (area_h * pct / 100).clamp((m.row_h * 3).min(area_h / 2), area_h / 2)
        } else {
            0
        };
        let ph = (area_h - band_h).max(0);
        let rects = if self.dual {
            let sx = self.splitter_x(w);
            self.splitter
                .set_rect(Rect::new(sx - half, top, half * 2 + 1, ph));
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
            if single_info {
                [Rect::new(0, dock_y, w, dock_h), Rect::default()]
            } else {
                let dpct = self.settings.int("layout.dock_split_pct").clamp(10, 90) as i32;
                let dsx = (w * dpct / 100).clamp(w / 8, w * 7 / 8);
                [
                    Rect::new(0, dock_y, (dsx - g2).max(0), dock_h),
                    Rect::new(dsx - g2 + gap, dock_y, (w - dsx + g2 - gap).max(0), dock_h),
                ]
            }
        } else {
            [Rect::default(), Rect::default()]
        };
        for (d, r) in self.docks.iter_mut().zip(dock_rects) {
            d.set_metrics(m.row_h, m.pad_x, &mut inv);
            d.set_bounds(r, &mut inv);
        }
        for (p, r) in self.panels.iter_mut().zip(rects) {
            p.set_metrics(m, &mut inv);
            p.set_default_columns(columns_for(r.w, s), &mut inv);
            p.set_bounds(r, &mut inv);
        }
    }

    /// 스플리터 드래그(`SplitEvent::Drag(v)` · v = 띠의 새 x) → 비율 설정(저장은 `End`에서) · 50% 자석 스냅(Alt = 해제).
    fn split_drag(&mut self, v: i32) {
        let (w, _) = self.viewport;
        if w <= 0 {
            return;
        }
        let half = px(3.0, self.scale);
        let mut sx = v + half;
        let snap = px(SNAP_PX, self.scale);
        if !self.alt && (sx - w / 2).abs() <= snap {
            sx = w / 2;
        }
        let min = px(MIN_PANEL, self.scale);
        let sx = sx.clamp(min.min(w / 2), (w - min).max(w / 2));
        let pct = (sx * 100 / w).clamp(10, 90);
        let _ = self
            .settings
            .set("layout.panel_split_pct", &pct.to_string());
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
    let code = ndir_i18n::resolve_code(
        settings.lang_setting(),
        &ndir_i18n::syslang::system_lang_code(),
        &avail,
    );
    ndir_i18n::activate(ndir_i18n::load(&code, &home));
}

/// nexa-ctl 내장 메뉴(우클릭 편집) 라벨을 앱 i18n에 잇는다 — `fn` 포인터 계약이라 한 번 누수해 `'static`으로.
fn install_ctl_labels() {
    use nexa_ctl::controls::CtlMsg as C;
    static LABELS: std::sync::OnceLock<[&'static str; 4]> = std::sync::OnceLock::new();
    let leak = |k: &str| -> &'static str { Box::leak(tr(k).into_boxed_str()) };
    let _ = LABELS.set([
        leak("menu.edit.selectAll"),
        leak("menu.edit.copy"),
        leak("menu.edit.cut"),
        leak("menu.edit.paste"),
    ]);
    nexa_ctl::controls::set_ctl_labels(|m| {
        let l = LABELS
            .get()
            .copied()
            .unwrap_or(["Select All", "Copy", "Cut", "Paste"]);
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
        Ok(cli::Mode::Gui) => run_gui(),
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
fn run_gui() -> ExitCode {
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
    // 시작 폴더 = 현재 폴더(없으면 홈) — 세션 복원(T-45)이 마지막 탭으로 바꾼다.
    let start = std::env::current_dir()
        .ok()
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));
    // 세션 복원(dir2 PREFS-054 — 창 생성 전) · 저장 폴더 = 설정 폴더.
    let session_dir = ndir_settings::config_dir();
    let session = session_dir.as_deref().and_then(Session::load);
    let mut app = App::new(settings, ui.font, None, session, Platform::native());
    app.session_dir = session_dir;
    // 지난 실행의 크래시 기록을 한 번 안내(토스트 · 자세한 것은 파일).
    if let Some(p) = app.session_dir.as_deref().and_then(crash::take_unreported) {
        app.toasts.push(
            toast::ToastKind::Warn,
            tr("menu.help.about"),
            format!("crash report: {}", p.display()),
        );
    }
    let _ = start;
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
        assert_eq!(narrow.len(), 5);
        assert_eq!(view_mode_of("tiles"), ViewMode::Tiles);
        assert_eq!(view_mode_of("x"), ViewMode::Tree);
    }
}
