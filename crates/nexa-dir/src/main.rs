//! Nexa Dir — 앱 진입점(M3 T-40 · winit 호스트 + nexa-ui 위에 그리는 수직 슬라이스).
//!
//! 구조 = nexa-sql 3층(docs/port/40 SKEL-401): 이 파일(모듈 선언 · `struct App` · `enum Focus` · `layout()` · `main()`) /
//! `app/*.rs`(`impl App` 조각 — 이벤트 루프 · 입력 · 그리기 · 메뉴/명령 · 기동 명령) / 호스트 껍질은 파일 하나씩
//! (`present` · `winhost` · `wingeom` · `winfocus` · `theme` · `icon` · `input` · `clipboard` · `toast` — nexa-sql 복사 · SKEL-403).
//! 규칙: 인자 해석·판정은 순수 함수(`cli.rs` · `selfcheck.rs`) · 그리기는 `RedrawRequested`에서만 · 유휴는 `WaitUntil`(SKEL-414).

mod app;
mod cli;
#[allow(dead_code)] // M6 파일 작업·M5 터미널 복사에서 소비(지금은 호스트 껍질만 들여놓음).
mod clipboard;
#[cfg(all(unix, not(target_os = "macos")))]
#[allow(dead_code)]
mod clipboard_x11;
mod filelist;
mod icon;
#[allow(dead_code)]
mod input;
#[allow(dead_code)]
// `set_mode`(macOS IOSurface 설정 · dir2에 키 없음 → Q-8) · `backend`(프레임 계측 T-46).
mod present;
mod selfcheck;
mod theme;
#[allow(dead_code)]
mod toast;
#[allow(dead_code)]
mod winfocus;
#[allow(dead_code)]
mod wingeom;
#[allow(dead_code)] // 보조 창(설정 · 단축키 · About)이 T-44에서 쓴다.
mod winhost;

use filelist::{ListOpts, TreeSource};
use ndir_i18n::{tr, trf};
use ndir_settings::keymap::{Chord, Keymap};
use ndir_settings::{Settings, ThemeMode};
use nexa_ctl::controls::{
    ComboItem, Control, MenuBar, MenuDef, MenuEntry, StatusBar, TabAction, TabBar, ToolIcon,
    ToolItem, Toolbar,
};
use nexa_ctl::draw::DrawCtx;
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::{InputEvent, Invalidations, Widget};
use nexa_explorer::pathbar::PathBar;
use nexa_gfx::{Font, Surface};
use nexa_grid::{Column, RowSource, VirtualRows};
use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
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

/// UI 스레드를 깨우는 사용자 이벤트(배경 작업이 보낸다 — M4 전송·감시 스레드 · SKEL-415).
#[derive(Debug)]
struct Wake;

/// 키 입력이 가는 영역(docs/port/40 SKEL-417 — dir2 화면 구성의 첫 부분집합 · 도크·트리·필터는 M5).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Focus {
    /// 파일 패널(0 왼쪽 · 1 오른쪽).
    Panel(usize),
    /// 경로바 편집.
    PathBar,
}

/// 포인터 캡처 대상(눌린 곳이 뗄 때까지 받는다).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Area {
    Menu,
    Tool,
    Tabs,
    Path,
    Panel(usize),
    Status,
}

struct App {
    window: Option<Rc<Window>>,
    surface: Option<present::Presenter>,
    settings: Settings,
    keymap: Keymap,
    ui_font: Font,
    theme: Theme,
    scale: f32,
    /// 포인터(장치 px) · 수식키.
    cursor: (i32, i32),
    shift: bool,
    primary: bool,
    alt: bool,
    ctrl_mac: bool,
    focus: Focus,
    menubar: MenuBar,
    toolbar: Toolbar,
    tabs: TabBar,
    pathbar: PathBar,
    /// 파일 패널 2(듀얼) — 단일 모드면 `panels[1]`은 빈 사각형.
    panels: [VirtualRows<TreeSource>; 2],
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

/// 탭 제목 = 폴더 이름(루트는 경로 그대로).
fn tab_title(p: &std::path::Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| p.to_string_lossy().into_owned())
}

impl App {
    fn new(settings: Settings, ui_font: Font, start_dir: PathBuf) -> App {
        let keymap = Keymap::from_settings(&settings);
        let theme = theme::resolve(settings.theme_mode(), None);
        let opts = list_opts(&settings);
        let dual = settings.get("layout.panel_mode").unwrap_or("dual") == "dual";
        let mut inv = Invalidations::default();
        let mut tabs = TabBar::new();
        tabs.set_show_new(true);
        tabs.set_tabs(vec![tab_title(&start_dir)], 0, &mut inv);
        let mut toasts = toast::Toasts::new();
        toasts.configure(3000, 85);
        let mut app = App {
            window: None,
            surface: None,
            menubar: MenuBar::new(App::build_menus(&settings)),
            toolbar: Toolbar::new(App::build_toolbar(&settings)),
            tabs,
            pathbar: PathBar::new(start_dir.to_string_lossy().into_owned(), 26, 6),
            panels: [
                VirtualRows::new(TreeSource::open(&start_dir, opts), 24, 6, 16),
                VirtualRows::new(TreeSource::open(&start_dir, opts), 24, 6, 16),
            ],
            active: 0,
            dual,
            statusbar: StatusBar::new(),
            toasts,
            settings,
            keymap,
            ui_font,
            theme,
            scale: 1.0,
            cursor: (0, 0),
            shift: false,
            primary: false,
            alt: false,
            ctrl_mac: false,
            focus: Focus::Panel(0),
            started: Instant::now(),
            main_active: true,
            pending_chord: None,
            pressed: None,
            last_click: None,
            exit_requested: false,
            startup_timed: Vec::new(),
            trace_ime: input::trace_ime(),
        };
        app.sync_menu_shortcuts();
        app.sync_menu_checks();
        app.set_focus(Focus::Panel(0));
        app.update_status();
        app
    }

    fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn set_focus(&mut self, f: Focus) {
        self.focus = f;
        let mut inv = Invalidations::default();
        for (i, p) in self.panels.iter_mut().enumerate() {
            p.set_focused(f == Focus::Panel(i), &mut inv);
        }
        if let Focus::Panel(i) = f {
            if i == 0 || self.dual {
                self.active = i;
            }
        }
    }

    /// 상태줄 = 활성 패널 요약(왼쪽) · 탭 n/N(오른쪽).
    fn update_status(&mut self) {
        let left = self.panels[self.active].source().status_text();
        let right = trf(
            "status.tab",
            &[
                &(self.tabs.active() + 1).to_string(),
                &self.tabs.len().max(1).to_string(),
            ],
        );
        let mut inv = Invalidations::default();
        self.statusbar.set_text(&left, &right, &mut inv);
    }

    /// 배치 — 위에서부터 메뉴바 · 툴바 · 탭 · 경로바 · 본문(패널 1~2) · 상태줄(dir2 화면 구성 · docs/port/13).
    fn layout(&mut self) {
        let Some(win) = &self.window else { return };
        let size = win.inner_size();
        let (w, h) = (size.width as i32, size.height as i32);
        let s = self.scale;
        let mut inv = Invalidations::default();
        self.menubar.set_scale(s);
        self.toolbar.set_scale(s);
        self.tabs.set_scale(s);
        self.statusbar.set_scale(s);
        let menu_h = px(self.settings.font_px("ui.menu_font_size") + 11.0, s);
        self.menubar
            .set_bounds(Rect::new(0, 0, w, menu_h), &mut inv);
        // `preferred_height`는 논리 px(nexa-ctl 규약) → 장치 px로(nexa-sql 09-16 맥 2x 교훈).
        let tool_h = px(self.toolbar.preferred_height() as f32, s);
        self.toolbar
            .set_bounds(Rect::new(0, menu_h, w, tool_h), &mut inv);
        let mut y = menu_h + tool_h;
        let tab_h = px(28.0, s);
        self.tabs.set_metrics(tab_h, px(10.0, s), &mut inv);
        self.tabs.set_bounds(Rect::new(0, y, w, tab_h), &mut inv);
        y += tab_h;
        let path_h = px(26.0, s);
        self.pathbar.set_metrics(path_h, px(6.0, s), &mut inv);
        self.pathbar
            .set_bounds(Rect::new(0, y, w, path_h), &mut inv);
        y += path_h + px(2.0, s);
        let status_h = px(24.0, s);
        let body_h = (h - y - status_h).max(0);
        let row_h = px(self.settings.font_px("list.font_size") + 8.0, s);
        let cols = vec![
            Column::new(filelist::COL_NAME, tr("col.name"), px(300.0, s)),
            Column::new(filelist::COL_SIZE, tr("col.size"), px(90.0, s)).right_aligned(),
            Column::new(filelist::COL_MODIFIED, tr("col.modified"), px(140.0, s)),
            Column::new(filelist::COL_KIND, tr("col.kind"), px(90.0, s)),
        ];
        let gap = px(4.0, s);
        let pct = self.settings.int("layout.panel_split_pct").clamp(10, 90) as i32;
        let rects = if self.dual {
            let lw = (w - gap) * pct / 100;
            [
                Rect::new(0, y, lw, body_h),
                Rect::new(lw + gap, y, w - lw - gap, body_h),
            ]
        } else {
            [Rect::new(0, y, w, body_h), Rect::new(0, y, 0, 0)]
        };
        for (p, r) in self.panels.iter_mut().zip(rects) {
            p.set_metrics(row_h, px(6.0, s), px(16.0, s), &mut inv);
            p.set_columns(cols.clone(), &mut inv);
            p.set_bounds(r, &mut inv);
        }
        self.statusbar
            .set_bounds(Rect::new(0, h - status_h, w, status_h), &mut inv);
        self.pathbar.set_overlay_bottom(h - status_h);
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
    let mut app = App::new(settings, ui.font, start);
    if let Err(e) = el.run_app(&mut app) {
        eprintln!("nexa-dir: event loop error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn px_rounds_and_tab_title() {
        assert_eq!(px(24.0, 1.0), 24);
        assert_eq!(px(24.0, 1.5), 36);
        assert_eq!(px(26.0, 1.25), 33);
        assert_eq!(tab_title(std::path::Path::new("C:/Users/kiros")), "kiros");
        assert_eq!(tab_title(std::path::Path::new("C:/")), "C:/");
    }
}
