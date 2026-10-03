//! 창 없는 AppCore 시험(T-41 · CI-102·105 · 하네스 T3): `App::new` → `layout_for` → `layout_dump` 골든 · `paint_into(RecordCtx)` ·
//! `route(InputEvent)`/`command(id)` 시나리오. winit 창·present는 만들지 않는다 — 3-OS CI(헤드리스)에서 수 초에 돈다.
//!
//! 골든 = `tests/golden/layout-1200x800.txt`(배치 Rect 트리 · 픽셀 아님 · CI-105). 갱신 = `NDIR_UPDATE_GOLDEN=1 cargo test -p nexa-dir`.

use crate::*;

/// 고정 샘플 트리(sub/ · a.txt · b.md) + 기본 설정 + 영어 + OS 기본 UI 글꼴.
fn fixture(tag: &str) -> (App, PathBuf) {
    let dir = std::env::temp_dir().join(format!("ndir-core-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sub")).expect("mkdir");
    std::fs::write(dir.join("a.txt"), b"hello").expect("write");
    std::fs::write(dir.join("b.md"), b"# b").expect("write");
    std::fs::write(dir.join("sub/inner.rs"), b"fn x() {}").expect("write");
    // 상태줄 문구가 OS 언어에 따라 달라지지 않게 영어 고정.
    ndir_i18n::activate(ndir_i18n::load("en", &dir.join("nowhere")));
    // 런처 시드는 OS마다 달라 골든이 흔들린다 → 픽스처는 끔(런처 시험이 명시적으로 켠다).
    // 테마 기본값 = system이라 OS 테마(러너마다 다르다 — CI windows·macos = light · ubuntu = dark)에 따라 덤프가 달라진다
    // → 픽스처는 dark 고정(10-03 CI 빨강 273b462 · 기본값 자체는 ndir-settings 시험이 본다).
    let settings = Settings::from_text(
        dir.join("settings.conf"),
        "launcher.visible=off\nlauncher.seed=2\nui.theme=dark\n",
    );
    let font = nexa_font::ui_font(None).expect("OS UI font (CI installs fonts)");
    (
        App::new(
            settings,
            font.font,
            Some(dir.clone()),
            None,
            Platform::fake(),
        ),
        dir,
    )
}

fn normalize(dump: &str, root: &std::path::Path) -> String {
    dump.replace(&root.display().to_string(), "<root>")
        .replace('\\', "/")
}

fn down(x: i32, y: i32) -> InputEvent {
    InputEvent::MouseDown {
        x,
        y,
        shift: false,
        primary: false,
    }
}

/// 배치 골든(1200×800 · 배율 1): 영역 이름 + Rect 트리가 파일과 같다(dir2 수직 스택: 탭 22 · 네비/경로 24 · 목록).
#[test]
fn layout_golden_1200x800() {
    let (mut app, dir) = fixture("golden");
    app.layout_for(1200, 800, 1.0);
    let got = normalize(&app.layout_dump(), &dir);
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/golden/layout-1200x800.txt"
    );
    if std::env::var_os("NDIR_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(std::path::Path::new(path).parent().expect("dir")).expect("mkdir");
        std::fs::write(path, &got).expect("write golden");
    }
    let want = std::fs::read_to_string(path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert_eq!(got, want, "layout dump differs from golden:\n{got}");
    // dir2 실측 규약(PANEL-002): 네비 4버튼 = 4 × (row_h + pad_x) = 104 · 경로 바가 바로 이어 붙음.
    let p = &app.panels[0];
    assert_eq!(p.nav_rect().w, 104);
    assert_eq!(p.pathbar.bounds().x, 104);
    assert_eq!(p.tabbar.bounds().h, 22);
    assert_eq!(p.nav_rect().h, 24);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 배율 2: 모든 영역이 표면 안이고 메뉴바·상태줄·탭 높이가 2배 · 패널이 겹치지 않고 스플리터가 사이에 있다.
#[test]
fn layout_scales_and_stays_inside() {
    let (mut app, dir) = fixture("scale");
    app.layout_for(1200, 800, 1.0);
    let m1 = app.menubar.bounds().h;
    let s1 = app.statusbar.bounds().h;
    app.layout_for(1600, 1000, 2.0);
    let view = Rect::new(0, 0, 1600, 1000);
    for r in [
        app.menubar.bounds(),
        app.toolbar.bounds(),
        app.panels[0].bounds(),
        app.panels[1].bounds(),
        app.panels[0].rows().bounds(),
        app.panels[1].pathbar.bounds(),
        app.splitter.rect(),
        app.statusbar.bounds(),
    ] {
        assert!(view.contains(Point { x: r.x, y: r.y }), "{r:?} outside");
        assert!(
            r.right() <= view.right() && r.bottom() <= view.bottom(),
            "{r:?} outside"
        );
    }
    assert_eq!(app.menubar.bounds().h, m1 * 2);
    assert_eq!(app.statusbar.bounds().h, s1 * 2);
    assert_eq!(app.panels[0].tabbar.bounds().h, 44);
    let (l, r) = (app.panels[0].bounds(), app.panels[1].bounds());
    assert!(l.right() <= r.x, "panels overlap: {l:?} {r:?}");
    let sp = app.splitter.rect();
    assert!(
        sp.x >= l.right() - sp.w && sp.right() <= r.x + sp.w,
        "splitter between: {sp:?}"
    );
    // 패널 ▸ (틈) ▸ 도크 밴드 ▸ 상태줄 — 도크(기본 켜짐 · T-60)가 배율에도 상태줄 위에 꼭 맞는다.
    let d = app.docks[0].bounds();
    assert!(l.bottom() < d.y && d.y - l.bottom() <= 8, "{l:?} {d:?}");
    assert_eq!(d.bottom(), app.statusbar.bounds().y);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 그리기 기록(CI-105 ①~③): 패닉 0 · 표면 밖 사각형 0 · 컬럼 머리·행·상태줄·네비 글자가 그려진다.
#[test]
fn paint_records_inside_surface() {
    let (mut app, dir) = fixture("paint");
    app.layout_for(1200, 800, 1.0);
    let mut rec = nexa_ctl::RecordCtx::with_surface(1200, 800);
    app.paint_into(&mut rec, 1200, 800, 1.0);
    let view = Rect::new(0, 0, 1200, 800);
    let outside =
        |r: &Rect| r.x < 0 || r.y < 0 || r.right() > view.right() || r.bottom() > view.bottom();
    let bad_fills: Vec<_> = rec.fills.iter().filter(|(r, _)| outside(r)).collect();
    let bad_texts: Vec<_> = rec.texts.iter().filter(|(_, _, c, _)| outside(c)).collect();
    assert!(
        rec.all_inside(view),
        "outside surface — fills {bad_fills:?} texts {bad_texts:?}"
    );
    for needle in [
        "Name", "Size", "Status", "sub", "a.txt", "b.md", "3 items", "Tab 1/1", "File",
    ] {
        assert!(
            rec.drew_text(needle),
            "missing text {needle:?}: {:?}",
            rec.strings().collect::<Vec<_>>()
        );
    }
    // 네비 버튼: 아이콘 글꼴이 있으면 MDL2 글리프(글자) · 없으면 SVG 마스크(그림 — 글자로는 안 나온다).
    // (판정은 이 앱의 버튼으로 — 전역 깃발은 다른 시험 스레드가 바꿀 수 있다.)
    let glyph_nav = app.panels[0]
        .nav_items()
        .iter()
        .all(|it| matches!(it.icon, nexa_ctl::ToolIcon::Glyph(_)));
    assert_eq!(rec.drew_text("\u{E72B}"), glyph_nav, "뒤로 버튼");
    assert!(
        !rec.drew_text("\u{2190}"),
        "유니코드 화살표 대체는 더 이상 쓰지 않는다"
    );
    // 메뉴를 열면 드롭다운 항목(단축키 열 포함)이 최상위 층에 그려진다.
    let mut inv = Invalidations::default();
    app.menubar.open_menu_index(0, &mut inv);
    rec.clear();
    app.paint_into(&mut rec, 1200, 800, 1.0);
    assert!(rec.drew_text("New Tab") && rec.drew_text("Exit"));
    assert!(rec.all_inside(view));
    let _ = std::fs::remove_dir_all(&dir);
}

/// 입력 시나리오(창 없이): 오른쪽 패널 클릭 = 활성 · 더블클릭 = 폴더 진입 · 네비 버튼 ↑/←/→ · 탭 · 단일 패널 = 오른쪽 0폭.
#[test]
fn route_and_commands_without_window() {
    let (mut app, dir) = fixture("route");
    app.layout_for(1200, 800, 1.0);
    assert_eq!(app.active, 0);
    let r1 = app.panels[1].rows().bounds();
    let (cx, cy) = (r1.x + r1.w / 2, r1.bottom() - 10);
    app.route(InputEvent::MouseMove { x: cx, y: cy });
    app.route(down(cx, cy));
    app.route(InputEvent::MouseUp { x: cx, y: cy });
    assert_eq!(app.active, 1);
    // 첫 행(sub · 폴더) 더블클릭 → 오른쪽 패널만 sub로 · 경로 바·상태줄·네비 활성이 따라온다.
    let row0 = app.panels[1].rows().row_anchor(0).expect("row 0 anchor");
    app.route(InputEvent::DoubleClick {
        x: row0.x + 40,
        y: row0.y,
        shift: false,
        primary: false,
    });
    assert!(
        app.panels[1].root_path().ends_with("sub"),
        "{:?}",
        app.panels[1].root_path()
    );
    assert_eq!(app.panels[1].rows().source().len(), 1);
    assert_eq!(app.panels[0].root_path(), dir);
    assert!(app.panels[1].pathbar.path().ends_with("sub"));
    assert!(
        app.statusbar.left().starts_with("1 item"),
        "{}",
        app.statusbar.left()
    );
    assert!(app.panels[1].can_back());
    // 네비 버튼 [←] 클릭(두 번째 버튼 · 26px 폭) → 돌아옴 · [→] → 다시 sub · [↑] → 부모 + 떠난 폴더 선택.
    let nav = app.panels[1].nav_rect();
    let click = |app: &mut App, i: i32| {
        let (x, y) = (nav.x + 26 * i + 13, nav.y + nav.h / 2);
        app.route(InputEvent::MouseMove { x, y });
        app.route(down(x, y));
        app.route(InputEvent::MouseUp { x, y });
    };
    click(&mut app, 1);
    assert_eq!(app.panels[1].root_path(), dir);
    click(&mut app, 2);
    assert!(app.panels[1].root_path().ends_with("sub"));
    click(&mut app, 3);
    assert_eq!(app.panels[1].root_path(), dir);
    let caret = app.panels[1].rows().caret().expect("left folder selected");
    assert!(app.panels[1]
        .rows()
        .source()
        .row_path(caret)
        .unwrap()
        .ends_with("sub"));
    // 탭: 새 탭 · 다음/이전 · 닫기(마지막은 안 닫힘).
    app.command("file.new_tab");
    assert_eq!(app.panels[1].tab_count(), 2);
    assert!(
        app.statusbar.right().contains("2/2"),
        "{}",
        app.statusbar.right()
    );
    app.command("tab.next");
    assert_eq!(app.panels[1].active_index(), 0);
    app.command("tab.prev");
    assert_eq!(app.panels[1].active_index(), 1);
    app.command("file.close_tab");
    assert_eq!(app.panels[1].tab_count(), 1);
    app.command("file.close_tab");
    assert_eq!(app.panels[1].tab_count(), 1);
    // panel.switch = 왼쪽으로 · Enter(캐럿 없음)는 무해.
    app.command("panel.switch");
    assert_eq!(app.active, 0);
    app.route(InputEvent::Key {
        key: nexa_ctl::Key::Enter,
        shift: false,
        primary: false,
    });
    // 단일 패널 모드 → 오른쪽 0폭·스플리터 없음 · 다시 듀얼.
    app.command("view.panel_single");
    assert!(!app.dual && app.panels[1].bounds().w == 0);
    assert_eq!(app.panels[0].bounds().w, 1200);
    assert!(app.splitter.rect().is_empty());
    app.command("view.panel_toggle");
    assert!(app.dual && app.panels[1].bounds().w > 0);
    // 숨김 토글 = 활성 탭의 값 + 메뉴 체크 + 툴바 토글이 함께 바뀐다(설정값 = 새 탭 기본값은 그대로 · 탭별 보기 옵션).
    let before = app.panels[app.active].active_view_values().0;
    assert_eq!(before, app.settings.flag("list.show_hidden"));
    app.command("view.hidden");
    assert_eq!(app.panels[app.active].active_view_values().0, !before);
    assert_eq!(app.settings.flag("list.show_hidden"), before);
    assert_eq!(app.menubar.is_checked("view.hidden"), Some(!before));
    assert_eq!(app.toolbar.item_checked("view.hidden"), !before);
    // 테마 순환 dark(픽스처 고정) → system → light → dark.
    assert_eq!(app.settings.theme_mode(), ThemeMode::Dark);
    app.command("view.theme_cycle");
    assert_eq!(app.settings.theme_mode(), ThemeMode::System);
    app.command("view.theme_cycle");
    assert_eq!(app.settings.theme_mode(), ThemeMode::Light);
    app.command("view.theme_cycle");
    assert_eq!(app.settings.theme_mode(), ThemeMode::Dark);
    app.command("view.theme_light");
    assert!(!app.theme.is_dark);
    // 보기 모드는 활성 탭에 즉시.
    app.command("view.mode_flat");
    assert_eq!(app.panels[0].rows().view_mode(), ViewMode::Flat);
    // 모르는 명령 = 상태줄 안내 · 종료 요청.
    app.command("zz.unknown");
    assert!(app.statusbar.left().starts_with("zz.unknown:"));
    app.command("file.exit");
    assert!(app.exit_requested);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 스플리터 드래그: 띠를 잡고 +100 → 비율·패널 폭이 바뀐다 · 가운데 근처는 50%로 스냅 · Alt는 스냅 해제.
#[test]
fn splitter_drag_and_snap() {
    let (mut app, dir) = fixture("split");
    app.layout_for(1200, 800, 1.0);
    let sp = app.splitter.rect();
    let (x, y) = (sp.x + sp.w / 2, sp.y + 50);
    let w0 = app.panels[0].bounds().w;
    app.route(InputEvent::MouseMove { x, y });
    app.route(down(x, y));
    assert!(app.splitter.is_dragging());
    app.route(InputEvent::MouseMove { x: x + 100, y });
    assert!(
        app.panels[0].bounds().w > w0 + 60,
        "{} vs {w0}",
        app.panels[0].bounds().w
    );
    assert_eq!(app.settings.int("layout.panel_split_pct"), 58);
    app.route(InputEvent::MouseUp { x: x + 100, y });
    assert!(!app.splitter.is_dragging());
    // 다시 잡아 가운데 쪽으로 — 50% ±20 안이면 스냅.
    let sp = app.splitter.rect();
    let (x, y) = (sp.x + sp.w / 2, sp.y + 50);
    app.route(InputEvent::MouseMove { x, y });
    app.route(down(x, y));
    app.route(InputEvent::MouseMove { x: 612, y });
    assert_eq!(app.settings.int("layout.panel_split_pct"), 50);
    app.route(InputEvent::MouseUp { x: 612, y });
    // Alt = 스냅 없음.
    app.alt = true;
    let sp = app.splitter.rect();
    let (x, y) = (sp.x + sp.w / 2, sp.y + 50);
    app.route(InputEvent::MouseMove { x, y });
    app.route(down(x, y));
    app.route(InputEvent::MouseMove { x: 612, y });
    assert_eq!(app.settings.int("layout.panel_split_pct"), 51);
    app.route(InputEvent::MouseUp { x: 612, y });
    let _ = std::fs::remove_dir_all(&dir);
}

/// 스플리터 3종(T-115 · dir2 WINC-094~096 + 사용자 10-03 "두께 · 동작 방식 · 서서히 밝아지는"): 패널 ↔ 도크(가로) · 도크 좌우(세로)도
/// 패널 좌우와 같은 컨트롤 — 드래그 = 비율 설정 · 클램프 · 자석(서로의 선과 50 %) · 숨김 조건 · 우선순위 · hover 페이드 · 창 밖 = 해제.
#[test]
fn dock_splitters_drag_clamp_fade_and_hide() {
    let (mut app, dir) = fixture("docksplit");
    app.layout_for(1200, 800, 1.0);
    let gap = 3;
    // 세 띠 모두 같은 모양: 틈(3) + 양쪽 3 = 9 · 그리는 띠 = 틈 두께.
    let (a, b, c) = (
        app.splitter.rect(),
        app.dock_split_h.rect(),
        app.dock_split_v.rect(),
    );
    assert_eq!((a.w, b.h, c.w), (9, 9, 9));
    assert_eq!(b.w, 1200, "도크 위 경계 = 전폭");
    for k in [
        SplitKind::Panel,
        SplitKind::DockHeight,
        SplitKind::DockSplit,
    ] {
        assert!(app.split_shown(k));
        assert_eq!(app.split_of(k).band().map(|x| x.thickness), Some(gap));
    }
    assert!(app.dock_split_h.band().unwrap().dim_rest && !app.splitter.band().unwrap().dim_rest);
    // 틈에 꼭 맞는다: 도크 위 경계의 가운데 = 패널 바닥과 도크 꼭대기 사이.
    let (pb, dt) = (app.panels[0].bounds().bottom(), app.docks[0].bounds().y);
    assert_eq!((dt - pb, b.y + 3, b.y + 3 + gap), (gap, pb, dt));
    // (B) 도크 높이: 위로 끌면 도크가 커진다 · 15~50 % 클램프.
    let (x, y) = (300, b.y + b.h / 2);
    app.route(InputEvent::MouseMove { x, y });
    assert!(app.dock_split_h.is_hover() && !app.splitter.is_hover());
    let h0 = app.docks[0].bounds().h;
    app.route(down(x, y));
    assert!(app.dock_split_h.is_dragging());
    app.route(InputEvent::MouseMove { x, y: y - 100 });
    assert!(
        app.docks[0].bounds().h > h0 + 80,
        "{}",
        app.docks[0].bounds().h
    );
    let pct = app.settings.int("layout.dock_height_pct");
    assert!((40..=50).contains(&pct), "{pct}");
    app.route(InputEvent::MouseMove { x, y: 60 });
    assert_eq!(app.settings.int("layout.dock_height_pct"), 50, "상한");
    app.route(InputEvent::MouseMove { x, y: 795 });
    assert_eq!(app.settings.int("layout.dock_height_pct"), 15, "하한");
    app.route(InputEvent::MouseUp { x, y: 795 });
    assert!(!app.dock_split_h.is_dragging());
    // (C) 도크 좌우: 끌면 비율 · 15~85 % · 패널 스플리터 x와 50 %에 자석 · Alt = 해제.
    let _ = app.settings.set("layout.panel_split_pct", "30");
    app.layout_for(1200, 800, 1.0);
    let c = app.dock_split_v.rect();
    let (x, y) = (c.x + c.w / 2, c.y + c.h / 2);
    app.route(InputEvent::MouseMove { x, y });
    assert!(app.dock_split_v.is_hover());
    app.route(down(x, y));
    app.route(InputEvent::MouseMove { x: 800, y });
    assert_eq!(app.settings.int("layout.dock_split_pct"), 67);
    assert!(app.docks[0].bounds().w > 780);
    app.route(InputEvent::MouseMove { x: 370, y });
    assert_eq!(
        app.settings.int("layout.dock_split_pct"),
        30,
        "패널 스플리터(360)에 자석"
    );
    app.route(InputEvent::MouseMove { x: 590, y });
    assert_eq!(app.settings.int("layout.dock_split_pct"), 50, "50 % 자석");
    app.route(InputEvent::MouseMove { x: 5, y });
    assert_eq!(app.settings.int("layout.dock_split_pct"), 15, "하한");
    app.route(InputEvent::MouseMove { x: 1195, y });
    assert_eq!(app.settings.int("layout.dock_split_pct"), 85, "상한");
    app.alt = true;
    app.route(InputEvent::MouseMove { x: 590, y });
    assert_eq!(
        app.settings.int("layout.dock_split_pct"),
        49,
        "Alt = 자석 해제"
    );
    app.alt = false;
    app.route(InputEvent::MouseUp { x: 590, y });
    // (A) 패널 좌우는 도크 분할선에도 붙는다(dir2 snap_split_x).
    let _ = app.settings.set("layout.dock_split_pct", "70");
    app.layout_for(1200, 800, 1.0);
    let a = app.splitter.rect();
    let (x, y) = (a.x + a.w / 2, a.y + 50);
    app.route(InputEvent::MouseMove { x, y });
    app.route(down(x, y));
    app.route(InputEvent::MouseMove { x: 850, y });
    assert_eq!(
        app.settings.int("layout.panel_split_pct"),
        70,
        "도크 분할선(840)에 자석"
    );
    app.route(InputEvent::MouseUp { x: 850, y });
    // hover = 서서히: 틱마다 진행이 오르고 다 오르면 애니메이션이 멈춘다 · 창 밖 = 풀림.
    let b = app.dock_split_h.rect();
    app.route(InputEvent::MouseMove { x: 300, y: b.y + 4 });
    let mut last = 0.0;
    for t in (0..=3000).step_by(100) {
        app.dock_split_h.tick(t);
        let v = app.dock_split_h.hover_progress();
        assert!(v >= last);
        last = v;
    }
    assert!((last - 1.0).abs() < 1e-3 && !app.dock_split_h.is_animating());
    app.pointer_gone();
    assert!(!app.dock_split_h.is_hover());
    // 그리기: 세 띠가 틈 자리에(평상시 색).
    let mut rec = nexa_ctl::RecordCtx::with_surface(1200, 800);
    app.paint_into(&mut rec, 1200, 800, 1.0);
    let band = Rect::new(0, app.dock_split_h.rect().y + 3, 1200, gap);
    assert!(
        rec.fills.iter().any(|(r, _)| *r == band),
        "도크 위 띠 {band:?}"
    );
    // 숨김: 단일 정보 = 도크 좌우 없음 · 도크 숨김 = 둘 다 없음 · 단일 패널 = 패널 좌우 없음.
    let _ = app.settings.set("layout.info_mode", "single");
    app.layout_for(1200, 800, 1.0);
    assert!(app.split_shown(SplitKind::DockHeight) && !app.split_shown(SplitKind::DockSplit));
    let _ = app.settings.set("dock.visible", "off");
    app.layout_for(1200, 800, 1.0);
    assert!(!app.split_shown(SplitKind::DockHeight) && !app.split_shown(SplitKind::DockSplit));
    assert!(app.split_shown(SplitKind::Panel));
    assert_eq!(
        app.dump_of("layout")
            .map(|d| d.contains("dsplit_h 0,0 0x0")),
        Some(true)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 기동 명령 어휘(창 없이): `nav:` · `panel:` · `key:` · `ui.click` · `layout.dump:` · `app.exit`.
#[test]
fn startup_cmd_vocabulary() {
    let (mut app, dir) = fixture("startup");
    app.layout_for(1200, 800, 1.0);
    app.startup_cmd(&format!("nav:{}", dir.join("sub").display()));
    assert!(app.panels[0].root_path().ends_with("sub"));
    app.startup_cmd("panel:1");
    assert_eq!(app.active, 1);
    app.startup_cmd("key:f5");
    // 숨김 토글은 OS 프리셋마다 코드가 다르다(Windows/Linux ctrl+h · macOS ⇧⌘. — 10-03 CI mac 적발) → 키맵이 아는 첫 코드로.
    let hidden_code = app.keymap.code_of("view.hidden");
    let first = hidden_code.split('|').next().unwrap_or("");
    app.startup_cmd(&format!("key:{first}"));
    assert!(
        !app.panels[app.active].active_view_values().0,
        "key:{first} = view.hidden 토글(활성 탭 · 기본 on → off)"
    );
    let dump = dir.join("layout.txt");
    app.startup_cmd(&format!("layout.dump:{}", dump.display()));
    let text = std::fs::read_to_string(&dump).expect("dump written");
    assert!(text.starts_with("window 1200x800 scale 1.00"));
    assert!(text.contains("panel1.list"));
    // 메뉴바 첫 라벨 클릭 = 메뉴 열림 · 같은 자리 다시 = 닫힘.
    let m = app.menubar.bounds();
    app.startup_cmd(&format!("ui.click:{}/{}", m.x + 20, m.y + m.h / 2));
    assert!(app.menubar.is_open());
    app.startup_cmd(&format!("ui.click:{}/{}", m.x + 20, m.y + m.h / 2));
    assert!(!app.menubar.is_open());
    app.startup_cmd("app.exit");
    assert!(app.exit_requested);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 세션 왕복(T-45): 탭·경로·활성·보기 모드가 `Session`으로 나가고 새 App으로 돌아온다 · 실행 인자 경로가 있으면 세션 무시 ·
/// 사라진 경로 탭은 건너뛴다 · 더러움은 디바운스 저장기로 모인다.
#[test]
fn session_roundtrip_through_app() {
    let (mut app, dir) = fixture("session");
    app.layout_for(1200, 800, 1.0);
    app.command("file.new_tab");
    app.startup_cmd(&format!("nav:{}", dir.join("sub").display()));
    app.command("view.mode_flat");
    app.startup_cmd("panel:1");
    app.command("file.new_tab");
    app.startup_cmd(&format!("nav:{}", dir.join("nope").display())); // 실패 = 위치 유지
    assert!(app.session_save.dirty(), "탭·경로 변경 = 더러움");
    let s = app.session_snapshot();
    assert_eq!(s.active_panel, 1);
    assert_eq!(s.panels[0].tabs, vec![dir.clone(), dir.join("sub")]);
    assert_eq!(s.panels[0].active, 1);
    assert_eq!(s.panels[0].modes, vec!["tree", "flat"]);
    assert_eq!(s.panels[1].tabs.len(), 2);
    let text = s.serialize();
    let parsed = Session::parse(&text);
    // 전부 tree인 modes는 생략 직렬화(dir2 규약) → 파싱은 빈 목록 · 재직렬화는 안정.
    assert_eq!(parsed.panels[0], s.panels[0]);
    assert_eq!(parsed.panels[1].tabs, s.panels[1].tabs);
    assert_eq!(Session::parse(&parsed.serialize()), parsed);
    // 복원(실행 인자 없음) — 사라진 경로 하나를 끼워 넣어도 건너뛴다.
    let mut broken = parsed.clone();
    broken.panels[1].tabs.insert(0, dir.join("gone"));
    broken.panels[1].active = 2;
    let settings = Settings::from_text(dir.join("settings2.conf"), "");
    let font = nexa_font::ui_font(None).expect("font");
    let mut app2 = App::new(settings, font.font, None, Some(broken), Platform::fake());
    app2.layout_for(1200, 800, 1.0);
    assert_eq!(app2.active, 1);
    assert_eq!(app2.panels[0].tab_count(), 2);
    assert_eq!(app2.panels[0].active_index(), 1);
    assert!(app2.panels[0].root_path().ends_with("sub"));
    assert_eq!(app2.panels[0].rows().view_mode(), ViewMode::Flat);
    assert_eq!(app2.panels[1].tab_count(), 2, "사라진 탭은 건너뜀");
    assert_eq!(app2.panels[1].active_index(), 1);
    assert!(!app2.session_save.dirty(), "복원 직후는 깨끗");
    // 실행 인자가 있으면 세션 무시.
    let settings = Settings::from_text(dir.join("settings3.conf"), "");
    let font = nexa_font::ui_font(None).expect("font");
    let app3 = App::new(
        settings,
        font.font,
        Some(dir.join("sub")),
        Some(parsed),
        Platform::fake(),
    );
    assert_eq!(app3.panels[0].tab_count(), 1);
    assert!(app3.panels[0].root_path().ends_with("sub"));
    // 저장은 폴더가 있을 때만 · flush = 파일.
    app2.session_dir = Some(dir.join("home"));
    app2.session_flush();
    let saved = Session::load(&dir.join("home")).expect("saved");
    assert_eq!(saved.panels[0].tabs.len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-46 기동 명령 확장: `@ready` 큐 · `ui.click:@영역` · 덤프 어휘 · `assert.<대상>:<식>`(부정 · 실패 = 종료 코드 3) · `quit:<코드>`.
#[test]
fn startup_ready_assert_and_dumps() {
    let (mut app, dir) = fixture("t46");
    app.layout_for(1200, 800, 1.0);
    app.queue_startup("@ready:panel:1,@idle:file.new_tab, ,@after:10:nav.up");
    assert_eq!(app.active, 0, "ready 전에는 안 돈다");
    assert_eq!(app.startup_timed.len(), 1);
    app.fire_ready();
    app.fire_ready();
    assert_eq!(app.active, 1);
    assert_eq!(
        app.panels[1].tab_count(),
        2,
        "@idle = ready · 두 번 쏘지 않는다"
    );
    // 영역 클릭: 메뉴바 가운데 = 메뉴 열림(어느 라벨이든) · 덤프 menu = open.
    app.startup_cmd("ui.click:@panel0.list");
    assert_eq!(app.active, 0);
    // 메뉴바 가운데는 라벨이 없다(라벨은 왼쪽) → 첫 라벨 좌표로.
    let m = app.menubar.bounds();
    app.startup_cmd(&format!("ui.click:{}/{}", m.x + 20, m.y + m.h / 2));
    assert!(app.dump_of("menu").unwrap().starts_with("open "));
    app.startup_cmd("ui.click:@statusbar");
    assert_eq!(app.dump_of("menu").unwrap(), "closed\n");
    assert!(app.area_rect("panel1.nav").is_some() && app.area_rect("nope").is_none());
    // 덤프 어휘.
    let list = app.dump_of("list").unwrap();
    assert!(list.starts_with("list panel0 rows 3 viewport 0+"), "{list}");
    assert!(
        // 덤프 = 이름 뒤 표시 열(상태 · 크기 · 수정한 날짜) — 로컬 파일의 상태는 빈 칸.
        list.contains("0 d0 Collapsed sub |") && list.contains("a.txt |  | 5 B |"),
        "{list}"
    );
    let panel = app.dump_of("panel").unwrap();
    assert!(
        panel.contains("tabs 1 active 0 rows 3 selected 0 caret None mode Tree sort []"),
        "{panel}"
    );
    assert!(app
        .dump_of("tabs")
        .unwrap()
        .starts_with("tabs 1 active 0\ntab0 "));
    assert!(app
        .dump_of("status")
        .unwrap()
        .starts_with("left 3 items\nright Tab 1/1"));
    assert!(app.dump_of("all").unwrap().contains("panel1 path"));
    let f = dir.join("list.txt");
    app.startup_cmd(&format!("list.dump:{}", f.display()));
    assert_eq!(std::fs::read_to_string(&f).unwrap(), list);
    // 단언: 참 · 부정 · 실패 → 종료 코드 3 + 종료 요청.
    assert!(app.assert_dump("status", "3 items"));
    assert!(app.assert_dump("list", "!zzz"));
    assert!(!app.exit_requested);
    app.startup_cmd("assert.panel:selected 9");
    assert!(app.exit_requested && app.exit_code == super::startup_cmd::EXIT_ASSERT);
    app.exit_requested = false;
    app.exit_code = 0;
    app.startup_cmd("assert.nope:x");
    assert!(app.exit_requested && app.exit_code == super::startup_cmd::EXIT_ASSERT);
    app.startup_cmd("quit:0");
    assert_eq!(
        app.exit_code,
        super::startup_cmd::EXIT_ASSERT,
        "단언 실패 뒤 quit은 코드를 못 덮는다"
    );
    // quit 코드.
    app.exit_requested = false;
    app.exit_code = 0;
    app.startup_cmd("quit:7");
    assert!(app.exit_requested && app.exit_code == 7);
    app.startup_cmd("quit");
    assert_eq!(app.exit_code, 7, "코드 없는 quit은 기존 코드를 유지");
    assert_eq!(crash::last_command(), "quit");
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-44 호스트 배선(창 없이): `file.prefs` = 열기 깃발 · 기동 명령 `prefs.search:`/`prefs.cat:` · `after_setting_changed`가 적용·갱신한다.
#[test]
fn prefs_host_wiring_without_window() {
    let (mut app, dir) = fixture("prefs");
    app.layout_for(1200, 800, 1.0);
    app.command("file.prefs");
    assert!(app.open_prefs);
    app.open_prefs = false;
    app.startup_cmd("prefs.search:dotfiles");
    assert!(app.open_prefs);
    // "점 파일 표시"는 점 파일 토글이 있는 OS(Windows)의 설정 창에만 나온다(Linux · macOS = 점 파일이 곧 숨김 파일).
    assert_eq!(
        app.dump_of("prefs").unwrap().contains("list.show_dotfiles"),
        platform::has_dotfile_toggle()
    );
    app.startup_cmd("prefs.cat:pref.cat.keys");
    assert!(app.dump_of("prefs").unwrap().contains("key.file.new_tab"));
    // 설정 창이 값을 바꿨을 때의 길: 저장 → 적용. 숨김 = **새 탭의 기본값**이라 열린 탭(과 메뉴 체크)은 그대로이고
    // 새 탭이 그 값으로 열린다(사용자 10-03).
    let _ = app.settings.set("list.show_hidden", "off");
    app.after_setting_changed("list.show_hidden");
    assert_eq!(app.menubar.is_checked("view.hidden"), Some(true));
    assert!(app.panels[0].active_view_values().0);
    app.command("file.new_tab");
    assert!(
        !app.panels[app.active].active_view_values().0,
        "새 탭 = 설정 기본값"
    );
    assert_eq!(
        app.menubar.is_checked("view.hidden"),
        Some(false),
        "체크 = 활성 탭 값"
    );
    let _ = app.settings.reset("list.show_hidden");
    app.after_setting_changed("list.show_hidden");
    let _ = app.settings.set("ui.font_face", "Nope");
    app.after_setting_changed("ui.font_face");
    assert_eq!(app.statusbar.left(), "Takes effect after restart");
    assert_eq!(app.lang_choices()[0].0, "system");
    assert!(app.lang_choices().iter().any(|(c, _)| c == "ko"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-43 2차: 탭 우클릭 = 컨텍스트 메뉴 · 잠금/고정/복제/이동/닫기 동작 · 세션에 잠금/고정 · 열 폭 동기.
#[test]
fn tab_menu_and_column_sync() {
    let (mut app, dir) = fixture("tabmenu");
    app.layout_for(1200, 800, 1.0);
    app.command("file.new_tab");
    // 탭 사각형은 그릴 때 측정된다(TabBar 규약) → 기록기로 한 번 그린다.
    let mut rec = nexa_ctl::RecordCtx::with_surface(1200, 800);
    app.paint_into(&mut rec, 1200, 800, 1.0);
    let tr0 = app.panels[0].tabbar.tab_rect(0).expect("tab 0 rect");
    let (x, y) = (tr0.x + tr0.w / 2, tr0.y + tr0.h / 2);
    app.route(InputEvent::MouseMove { x, y });
    app.route(InputEvent::RightDown { x, y });
    assert!(app.tab_menu.is_open(), "탭 우클릭 = 메뉴");
    assert_eq!(app.tab_menu_at, Some((0, 0)));
    let ids: Vec<String> = app
        .tab_menu
        .item_ids()
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        ids,
        vec![
            "tab.lock",
            "tab.pin",
            "tab.duplicate",
            "tab.new",
            "tab.move_other",
            "tab.close"
        ]
    );
    // Esc = 닫힘.
    app.route(InputEvent::Key {
        key: nexa_ctl::Key::Escape,
        shift: false,
        primary: false,
    });
    assert!(!app.tab_menu.is_open());
    app.tab_menu_at = Some((0, 0));
    app.tab_menu_action("tab.lock");
    assert!(app.panels[0].tab_locked(0));
    app.tab_menu_at = Some((0, 0));
    app.tab_menu_action("tab.close");
    assert_eq!(app.panels[0].tab_count(), 2, "잠긴 탭은 안 닫힌다");
    app.tab_menu_at = Some((0, 1));
    app.tab_menu_action("tab.pin");
    assert!(app.panels[0].tab_pinned(0), "고정 = 앞으로");
    app.tab_menu_at = Some((0, 1));
    app.tab_menu_action("tab.duplicate");
    assert_eq!(app.panels[0].tab_count(), 3);
    app.tab_menu_at = Some((0, 2));
    app.tab_menu_action("tab.move_other");
    assert_eq!(app.panels[0].tab_count(), 2);
    assert_eq!(app.panels[1].tab_count(), 2);
    assert_eq!(app.active, 1);
    let snap = app.session_snapshot();
    assert_eq!(snap.panels[0].pinned, vec![true, false]);
    assert!(snap.panels[0].locked.iter().any(|l| *l));
    assert_eq!(Session::parse(&snap.serialize()).panels[0], snap.panels[0]);
    // 열 폭 동기: 켜면 활성 패널 폭이 반대 패널로.
    let mut inv = Invalidations::default();
    app.set_active(0);
    app.panels[0]
        .rows_mut()
        .set_col_widths(&[200, 50, 60, 70], &mut inv);
    let _ = app.settings.set("list.col_width_sync", "off");
    app.command("view.col_width_sync");
    assert!(app.settings.flag("list.col_width_sync"));
    assert_eq!(app.panels[1].col_widths_now(), vec![200, 50, 60, 70]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-50 포트 배선(가짜 플랫폼): 파일 활성화 = Opener · 실패 = 토스트 · 감시 변경 = 그 폴더 탭 재열람(캐럿 유지) · 감시 대상 = 두 패널 폴더.
#[test]
fn platform_ports_wire_open_and_watch() {
    let (mut app, dir) = fixture("ports");
    app.layout_for(1200, 800, 1.0);
    let log = app.platform.log.clone().expect("fake log");
    // 파일 활성화(Enter) → Opener.open.
    let mut inv = Invalidations::default();
    app.panels[0]
        .rows_mut()
        .select_program(1, nexa_grid::SelectOp::Single, &mut inv);
    app.route(InputEvent::Key {
        key: nexa_ctl::Key::Enter,
        shift: false,
        primary: false,
    });
    assert!(
        log.borrow()
            .calls
            .iter()
            .any(|c| c.starts_with("open:") && c.ends_with("a.txt")),
        "{:?}",
        log.borrow().calls
    );
    assert!(!app.toasts.animating(), "성공은 조용히");
    log.borrow_mut().open_fails = true;
    app.open_external(&dir.join("b.md"));
    assert!(app.toasts.animating(), "실패 = 토스트 한 번");
    // 감시: 1 s 틱 → 대상 = 두 패널 폴더 · 변경 주입 → 재열람(새 파일이 보인다 · 캐럿 유지).
    app.watch_next = Instant::now();
    app.watch_tick(Instant::now());
    assert_eq!(log.borrow().watched, vec![dir.clone()]);
    std::fs::write(dir.join("zz.txt"), b"new").expect("write");
    assert_eq!(app.panels[0].rows().source().len(), 3);
    log.borrow_mut().changed = vec![dir.clone()];
    app.watch_next = Instant::now();
    app.watch_tick(Instant::now());
    assert_eq!(app.panels[0].rows().source().len(), 4);
    assert_eq!(app.panels[0].rows().caret(), Some(1));
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-51 A: `edit.delete` = 선택 경로 → Trash 포트(가짜 기록) → 그 폴더 탭 재열람 · 선택 없음 = 무동작 · 실패 = 토스트.
#[test]
fn delete_goes_through_trash_port() {
    let (mut app, dir) = fixture("trash");
    app.layout_for(1200, 800, 1.0);
    let log = app.platform.log.clone().expect("fake log");
    app.command("edit.delete");
    assert!(
        log.borrow().calls.iter().all(|c| !c.starts_with("trash:")),
        "선택 없음 = 호출 없음"
    );
    let mut inv = Invalidations::default();
    app.panels[0]
        .rows_mut()
        .select_program(1, nexa_grid::SelectOp::Single, &mut inv);
    assert_eq!(app.panels[0].selected_paths(), vec![dir.join("a.txt")]);
    app.command("edit.delete");
    assert!(
        log.borrow().calls.iter().any(|c| c == "trash:1"),
        "{:?}",
        log.borrow().calls
    );
    assert!(app.toasts.animating());
    let _ = std::fs::remove_dir_all(&dir);
}

/// PANEL-044: `nav.home` = 내 PC(드라이브 열) · 가짜 Disk가 준 용량이 전체/여유 셀에 · 뒤로 = 기본 열.
#[test]
fn my_pc_drive_columns_from_disk_port() {
    let (mut app, dir) = fixture("mypc");
    app.layout_for(1200, 800, 1.0);
    let log = app.platform.log.clone().expect("fake log");
    log.borrow_mut().space = Some((4096, 1024));
    app.command("nav.home");
    let p = &app.panels[0];
    assert!(p.rows().source().is_virtual_root());
    let keys: Vec<u32> = p.rows().columns().iter().map(|c| c.key).collect();
    assert_eq!(keys, vec![0, 4, 5, 6]);
    if p.rows().source().len() > 0 {
        assert_eq!(p.rows().source().cell(0, filelist::COL_TOTAL), "4.0 KB");
        assert_eq!(p.rows().source().cell(0, filelist::COL_FREE), "1.0 KB");
        assert!(log.borrow().calls.iter().any(|c| c.starts_with("disk:")));
    }
    app.command("nav.back");
    assert_eq!(
        app.panels[0].rows().columns().len(),
        4,
        "기본 표시 열로 복귀"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-60 도크: 기본 켜짐 = 전폭 밴드(듀얼 = 좌/우 · 단일 정보 = 좌 전폭) · 패널 높이가 줄어든다 · 정보 줄 = 선택 파일 · 미리보기 종류 전환 = 텍스트 · 끄면 0.
#[test]
fn dock_layout_and_contents() {
    let (mut app, dir) = fixture("dock");
    app.layout_for(1200, 800, 1.0);
    let (d0, d1) = (app.docks[0].bounds(), app.docks[1].bounds());
    assert!(d0.h > 0 && d1.h > 0 && d0.right() <= d1.x, "{d0:?} {d1:?}");
    assert_eq!(d0.bottom(), app.statusbar.bounds().y);
    assert!(app.panels[0].bounds().bottom() < d0.y);
    // 높이 = 영역 × 30 %(기본) · 행 3줄 ~ 절반 클램프.
    let area_h = app.statusbar.bounds().y - app.panels[0].bounds().y;
    assert_eq!(
        d0.bottom() - app.panels[0].bounds().bottom(),
        area_h * 30 / 100
    );
    // 단일 정보 = 좌 전폭.
    let _ = app.settings.set("layout.info_mode", "single");
    app.apply_setting("layout.info_mode");
    assert_eq!(app.docks[0].bounds().w, 1200);
    assert_eq!(app.docks[1].bounds().h, 0);
    // 정보 줄 = 선택 파일(a.txt) · 선택 없음 = 현재 폴더.
    let mut rec = nexa_ctl::RecordCtx::with_surface(1200, 800);
    app.paint_into(&mut rec, 1200, 800, 1.0);
    assert!(
        rec.drew_text("Current folder:"),
        "{:?}",
        rec.strings().collect::<Vec<_>>()
    );
    let mut inv = Invalidations::default();
    app.panels[0]
        .rows_mut()
        .select_program(1, nexa_grid::SelectOp::Single, &mut inv);
    app.update_status();
    rec.clear();
    app.paint_into(&mut rec, 1200, 800, 1.0);
    assert!(rec.drew_text("Name: a.txt") && rec.drew_text("Kind: TXT"));
    // 종류 스트립 클릭(둘째 라벨 = 미리보기 · 클릭 범위는 paint 때 캐시 → 왼쪽부터 훑어 찾기) → 파일 내용.
    let strip_y = app.docks[0].bounds().y + 4;
    let mut x = app.docks[0].bounds().x + 2;
    while app.docks[0].active_kind() != 1 && x < 400 {
        app.route(down(x, strip_y));
        app.route(InputEvent::MouseUp { x, y: strip_y });
        x += 3;
    }
    assert_eq!(
        app.docks[0].active_kind(),
        1,
        "스트립 클릭으로 미리보기 전환"
    );
    app.update_status();
    rec.clear();
    app.paint_into(&mut rec, 1200, 800, 1.0);
    assert!(
        rec.drew_text("hello"),
        "{:?}",
        rec.strings().collect::<Vec<_>>()
    );
    assert!(app.dump_of("layout").unwrap().contains("dock0 "));
    assert!(app.dump_of("dock").unwrap().contains("kind 1"));
    let _ = app.settings.set("dock.visible", "off");
    app.apply_setting("dock.visible");
    assert_eq!(app.docks[0].bounds().h, 0);
    assert_eq!(app.panels[0].bounds().bottom(), app.statusbar.bounds().y);
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-61 도크 터미널(가짜 echo PTY): 종류 2 전환 → paint가 cwd로 지연 시작(로그) → term.send 되돌림 → 화면·덤프·그리기 →
/// 키 경로(Char/Enter) → 패널 클릭 = 포커스 해제 → 도크 숨김 = 낡은 포커스 무시.
#[test]
fn terminal_dock_with_fake_pty() {
    let (mut app, dir) = fixture("term");
    app.layout_for(1200, 800, 1.0);
    app.startup_cmd("dock.kind:2");
    assert_eq!(app.docks[0].active_kind(), 2);
    assert!(!app.terms[0].started(), "시작은 paint에서(지연)");
    let mut rec = nexa_ctl::RecordCtx::with_surface(1200, 800);
    app.paint_into(&mut rec, 1200, 800, 1.0);
    assert!(app.terms[0].alive(), "{}", app.term_dump());
    let log = app.platform.log.clone().expect("fake log");
    let spawn = log
        .borrow()
        .calls
        .iter()
        .find(|c| c.starts_with("pty:"))
        .cloned()
        .expect("pty spawn 기록");
    assert!(
        spawn.contains(&dir.to_string_lossy().to_string()),
        "cwd = 패널 폴더: {spawn}"
    );
    app.startup_cmd("term.send:ls\\r");
    assert_eq!(app.term_focus, Some(0));
    assert!(app.term_tick(100), "echo 수거 = 변화");
    assert!(app.term_dump().contains("\nls"), "{}", app.term_dump());
    rec.clear();
    app.paint_into(&mut rec, 1200, 800, 1.0);
    assert!(rec.drew_text("l") && rec.drew_text("s"), "셀 단위 그리기");
    // 키 경로: 포커스 중 Char/Enter는 셸로(목록 단축키 차단) — echo로 되돌아온다.
    app.route(InputEvent::Char { c: 'x', now_ms: 0 });
    app.route(InputEvent::Key {
        key: nexa_ctl::Key::Enter,
        shift: false,
        primary: false,
    });
    app.term_tick(200);
    assert!(app.term_dump().contains('x'), "{}", app.term_dump());
    // Ctrl+글자 = 제어 문자(선택 없음) · 전체 선택 → Ctrl+C = 복사 경로(클립보드 결과는 OS 종속 — 선택 해제만 확인).
    assert!(app.term_ctrl('l', false, false));
    assert!(app.term_select_all());
    assert!(app.terms[0].sel.is_some());
    app.term_ctrl('c', false, false);
    assert!(app.terms[0].sel.is_none(), "복사 뒤 선택 해제");
    // 패널 클릭 = 포커스 해제.
    let lp = app.panels[0].bounds();
    app.route(down(lp.x + 50, lp.y + 120));
    app.route(InputEvent::MouseUp {
        x: lp.x + 50,
        y: lp.y + 120,
    });
    assert_eq!(app.term_focus, None);
    // 터미널 격자 클릭 = 포커스 복귀.
    let cr = app.docks[0].content_rect();
    app.route(down(cr.x + 10, cr.y + 10));
    app.route(InputEvent::MouseUp {
        x: cr.x + 10,
        y: cr.y + 10,
    });
    assert_eq!(app.term_focus, Some(0));
    // 도크 탭으로 다른 종류 → 터미널 탭 클릭 = 포커스가 바로 온다(격자를 다시 누르지 않아도 입력 가능).
    app.startup_cmd("dock.kind:0");
    app.route(down(lp.x + 50, lp.y + 120));
    app.route(InputEvent::MouseUp {
        x: lp.x + 50,
        y: lp.y + 120,
    });
    assert_eq!((app.docks[0].active_kind(), app.term_focus), (0, None));
    let (db, cr) = (app.docks[0].bounds(), app.docks[0].content_rect());
    let ty = (db.y + cr.y) / 2;
    for tx in (db.x + 4..db.right()).step_by(6) {
        app.route(down(tx, ty));
        app.route(InputEvent::MouseUp { x: tx, y: ty });
        if app.docks[0].active_kind() == 2 {
            break;
        }
    }
    assert_eq!(
        (app.docks[0].active_kind(), app.term_focus),
        (2, Some(0)),
        "터미널 탭 클릭 = 포커스"
    );
    // 덤프 어휘 · 도크 숨김 = 낡은 포커스는 키를 삼키지 않는다.
    assert!(app.dump_of("term").unwrap().starts_with("dock0 alive"));
    let _ = app.settings.set("dock.visible", "off");
    app.apply_setting("dock.visible");
    assert!(app.term_focused().is_none());
    let mut inv = Invalidations::default();
    assert!(!app.term_key(&InputEvent::Char { c: 'q', now_ms: 0 }, &mut inv));
    assert_eq!(app.term_focus, None, "낡은 포커스 해제");
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-62 도크 미리보기 = 시임(동봉 markdown.wasm · `NDIR_PLUGINS_DIR` · 스레드 로컬 캐시): .md 선택 → 공급자 markdown ·
/// h1 태그가 벗겨진 줄이 그려진다 · `plugins.disabled` = 내장 폴백 · 덤프 어휘 `preview`.
#[test]
fn preview_plugin_renders_markdown_in_dock() {
    std::env::set_var(
        "NDIR_PLUGINS_DIR",
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins"),
    );
    let (mut app, dir) = fixture("plug");
    std::fs::write(dir.join("note.md"), "# Hello Plug\n\n- item\n").unwrap();
    app.command("view.refresh");
    app.layout_for(1200, 800, 1.0);
    app.startup_cmd("dock.kind:1");
    let row = (0..64)
        .find(|&r| {
            app.panels[0]
                .rows()
                .source()
                .row_path(r)
                .is_some_and(|p| p.ends_with("note.md"))
        })
        .expect("note.md 행");
    app.startup_cmd(&format!("list.select:{row}"));
    let dump = app.dump_of("preview").unwrap();
    assert!(dump.starts_with("provider markdown\n"), "{dump}");
    assert!(dump.contains("\nHello Plug"), "h1 태그 벗김: {dump}");
    let mut rec = nexa_ctl::RecordCtx::with_surface(1200, 800);
    app.paint_into(&mut rec, 1200, 800, 1.0);
    assert!(rec.drew_text("Hello Plug"));
    let _ = app.settings.set("plugins.disabled", "markdown");
    app.update_status();
    let dump = app.dump_of("preview").unwrap();
    assert!(
        dump.starts_with("provider builtin.text\n") && dump.contains("# Hello Plug"),
        "{dump}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-54: Help ▸ 자가 점검 명령은 보조 창 열기 요청(창 없는 시험 = 플래그) · 메뉴 표에 등재 · 덤프 `check`는 결과 전이면 안내 줄.
#[test]
fn help_selfcheck_requests_check_window() {
    let (mut app, dir) = fixture("check");
    assert!(super::menus::MENU_IDS.contains(&"help.selfcheck"));
    assert!(
        ndir_settings::command("help.selfcheck").is_some(),
        "명령 표 등재(키 설정 key.help.selfcheck)"
    );
    app.command("help.selfcheck");
    assert!(app.open_check);
    assert!(
        app.dump_of("check").unwrap().contains("selfcheck"),
        "{}",
        app.dump_of("check").unwrap()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// M6 A: edit.copy → 다른 폴더로 edit.paste(작업 스레드 전송 · 틱 수거) → 사본 존재 · undo = 휴지통 포트(가짜 로그) ·
/// edit.cut → paste = 이동 · undo = 되돌림(실제 fs) · redo · 덤프 `ops` · 선택 없으면 클립보드 유지 · 붙여넣을 것 없으면 안내.
#[test]
fn copy_cut_paste_undo_through_ops() {
    let (mut app, dir) = fixture("ops");
    app.layout_for(1200, 800, 1.0);
    let wait = |app: &mut App| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while app.ops_tick() {
            assert!(
                std::time::Instant::now() < deadline,
                "transfer did not finish"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    };
    let row_of = |app: &App, name: &str| {
        (0..64)
            .find(|&r| {
                app.panels[0]
                    .rows()
                    .source()
                    .row_path(r)
                    .is_some_and(|p| p.ends_with(name))
            })
            .expect("row")
    };
    // 붙여넣을 것 없음 = 안내만.
    app.command("edit.paste");
    assert!(app.transfer.is_none() && app.dump_of("ops").unwrap().contains("clip none"));
    // 복사: a.txt → sub/
    let r = row_of(&app, "a.txt");
    app.startup_cmd(&format!("list.select:{r}"));
    app.command("edit.copy");
    assert!(
        app.dump_of("ops").unwrap().contains("clip 1 copy"),
        "{}",
        app.dump_of("ops").unwrap()
    );
    app.startup_cmd(&format!("nav:{}", dir.join("sub").display()));
    app.command("edit.paste");
    assert!(app.transfer.is_some() || dir.join("sub/a.txt").is_file());
    wait(&mut app);
    assert!(
        dir.join("sub/a.txt").is_file() && dir.join("a.txt").is_file(),
        "복사 = 원본 유지"
    );
    assert!(app.history().can_undo());
    // undo(복사) = 사본을 휴지통 포트로(가짜 = 로그만).
    app.command("edit.undo");
    let log = app.platform.log.clone().expect("fake log");
    assert!(
        log.borrow().calls.iter().any(|c| c == "trash:1"),
        "{:?}",
        log.borrow().calls
    );
    assert!(app.history().can_redo());
    // 잘라내기: b.md → sub/ (이동 · 실제 fs) → undo = 되돌림 → redo = 다시 이동.
    app.startup_cmd(&format!("nav:{}", dir.display()));
    let r = row_of(&app, "b.md");
    app.startup_cmd(&format!("list.select:{r}"));
    app.command("edit.cut");
    assert!(app.dump_of("ops").unwrap().contains("clip 1 cut"));
    app.startup_cmd(&format!("nav:{}", dir.join("sub").display()));
    app.command("edit.paste");
    wait(&mut app);
    assert!(
        dir.join("sub/b.md").is_file() && !dir.join("b.md").exists(),
        "이동"
    );
    assert!(
        app.dump_of("ops").unwrap().contains("clip none"),
        "잘라내기 뒤 클립보드 비움"
    );
    app.command("edit.undo");
    assert!(
        dir.join("b.md").is_file() && !dir.join("sub/b.md").exists(),
        "undo = 되돌림"
    );
    app.command("edit.redo");
    assert!(dir.join("sub/b.md").is_file(), "redo = 다시 이동");
    // 바쁜 중 붙여넣기 금지는 start_transfer 가드(여기선 idle) · 덤프 어휘.
    assert!(app.dump_of("ops").unwrap().starts_with("transfer idle"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// M6 B: 새 폴더 → 생성 행이 선택되고 인라인 이름 바꾸기 중 → 타이핑 + Enter = 이름 확정(RenameOp) → undo 2회 = 이름 복귀 + 생성 취소(휴지통 포트) ·
/// F2 = 캐럿 행 이름 바꾸기(파일은 이름부만 선택 → 확장자 유지) · Esc = 취소 · 새 파일 `New File.txt`.
#[test]
fn new_folder_rename_and_undo() {
    let (mut app, dir) = fixture("newrn");
    app.layout_for(1200, 800, 1.0);
    app.command("file.new_folder");
    assert!(
        dir.join("New Folder").is_dir(),
        "{:?}",
        std::fs::read_dir(&dir).unwrap().count()
    );
    assert!(app.panels[0].rows().is_renaming(), "생성 직후 이름 바꾸기");
    app.startup_cmd("ui.type:Docs");
    app.startup_cmd("ui.press:enter");
    assert!(
        dir.join("Docs").is_dir() && !dir.join("New Folder").exists(),
        "이름 확정"
    );
    assert!(!app.panels[0].rows().is_renaming());
    assert!(
        app.history()
            .undo_description()
            .is_some_and(|d| d.contains("Docs")),
        "{:?}",
        app.history().undo_description()
    );
    app.command("edit.undo");
    assert!(
        dir.join("New Folder").is_dir() && !dir.join("Docs").exists(),
        "undo = 이름 복귀"
    );
    app.command("edit.undo");
    let log = app.platform.log.clone().expect("fake log");
    assert!(
        log.borrow().calls.iter().any(|c| c == "trash:1"),
        "undo(생성) = 휴지통 포트"
    );
    // F2: a.txt 이름부만 바꿔 확장자 유지 · Esc 취소.
    let row = (0..64)
        .find(|&r| {
            app.panels[0]
                .rows()
                .source()
                .row_path(r)
                .is_some_and(|p| p.ends_with("a.txt"))
        })
        .expect("a.txt");
    app.startup_cmd(&format!("list.select:{row}"));
    app.command("edit.rename");
    assert!(app.panels[0].rows().is_renaming());
    app.startup_cmd("ui.press:escape");
    assert!(
        !app.panels[0].rows().is_renaming() && dir.join("a.txt").is_file(),
        "Esc = 취소"
    );
    app.command("edit.rename");
    app.startup_cmd("ui.type:zz");
    app.startup_cmd("ui.press:enter");
    assert!(
        dir.join("zz.txt").is_file() && !dir.join("a.txt").exists(),
        "이름부만 교체 · 확장자 유지"
    );
    // 새 파일.
    app.command("file.new_file");
    assert!(dir.join("New File.txt").is_file());
    app.startup_cmd("ui.press:escape");
    assert!(app.dump_of("ops").unwrap().contains("undo true"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-29 A 대화상자(창 없이 = 대기 사양 + dlg.pick): 영구 삭제 확인(취소 = 유지 · 확인 = 삭제) · 붙여넣기 충돌 4버튼
/// (작업 스레드가 채널로 묻고 UI가 답한다 — 건너뛰기 = 원본 유지 · 덮어쓰기 = 교체 · 모두 덮어쓰기 = 이후 무확인 · 취소 = 중단).
#[test]
fn dialogs_delete_permanent_and_paste_conflict() {
    let (mut app, dir) = fixture("dlg");
    app.layout_for(1200, 800, 1.0);
    let wait = |app: &mut App, pick: Option<i32>| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        loop {
            let live = app.ops_tick();
            if app.dlg_pending.is_some() {
                if let Some(id) = pick {
                    app.startup_cmd(&format!("dlg.pick:{id}"));
                } else {
                    return true;
                }
            }
            if !live && app.transfer.is_none() {
                return false;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "transfer did not finish"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    };
    let row_of = |app: &App, name: &str| {
        (0..64)
            .find(|&r| {
                app.panels[0]
                    .rows()
                    .source()
                    .row_path(r)
                    .is_some_and(|p| p.ends_with(name))
            })
            .expect("row")
    };
    // 영구 삭제: 취소 → 유지 · 확인 → 삭제(휴지통 포트 호출 없음).
    let r = row_of(&app, "b.md");
    app.startup_cmd(&format!("list.select:{r}"));
    app.command("edit.delete_permanent");
    let d = app.dump_of("dlg").unwrap();
    assert!(
        d.starts_with("pending Delete | Permanently delete 1"),
        "{d}"
    );
    app.startup_cmd("dlg.pick:0");
    assert!(dir.join("b.md").is_file() && app.dump_of("dlg").unwrap() == "none\n");
    app.command("edit.delete_permanent");
    app.startup_cmd("dlg.pick:1");
    assert!(!dir.join("b.md").exists(), "확인 = 영구 삭제");
    // 충돌: sub/a.txt(old)가 있을 때 a.txt 복사 → 질문 → 건너뛰기.
    std::fs::write(dir.join("sub/a.txt"), b"old").unwrap();
    let r = row_of(&app, "a.txt");
    app.startup_cmd(&format!("list.select:{r}"));
    app.command("edit.copy");
    app.startup_cmd(&format!("nav:{}", dir.join("sub").display()));
    app.command("edit.paste");
    assert!(wait(&mut app, None), "충돌 질문이 온다");
    assert!(
        app.dump_of("dlg").unwrap().contains("Confirm Overwrite"),
        "{}",
        app.dump_of("dlg").unwrap()
    );
    app.startup_cmd("dlg.pick:3");
    wait(&mut app, Some(3));
    assert_eq!(
        std::fs::read(dir.join("sub/a.txt")).unwrap(),
        b"old",
        "건너뛰기 = 원본 유지"
    );
    // 덮어쓰기.
    app.command("edit.paste");
    wait(&mut app, Some(1));
    assert_eq!(
        std::fs::read(dir.join("sub/a.txt")).unwrap(),
        b"hello",
        "덮어쓰기 = 교체"
    );
    // 취소 = 중단(아무것도 안 바뀜) · 덤프 idle.
    std::fs::write(dir.join("sub/a.txt"), b"old2").unwrap();
    app.command("edit.paste");
    wait(&mut app, Some(4));
    assert_eq!(std::fs::read(dir.join("sub/a.txt")).unwrap(), b"old2");
    assert!(app.dump_of("ops").unwrap().starts_with("transfer idle"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-62 B: F3 = 단일 선택 파일을 독립 창 내용으로(창 없이 = 요청 플래그 + 덤프) · 폴더/없음 = 무동작 · 암호 필요 zip =
/// 마스킹 입력 대화상자 → 빈/틀린 암호 = 재시도 문구 → 취소 = 아무 창도 없음. 메뉴 등재(View) 확인.
#[test]
fn preview_window_and_archive_password_flow() {
    let (mut app, dir) = fixture("pvwin");
    app.layout_for(1200, 800, 1.0);
    assert!(super::menus::MENU_IDS.contains(&"view.preview_window"));
    app.command("view.preview_window");
    assert!(!app.open_preview, "선택 없음 = 무동작");
    let row_of = |app: &App, name: &str| {
        (0..64)
            .find(|&r| {
                app.panels[0]
                    .rows()
                    .source()
                    .row_path(r)
                    .is_some_and(|p| p.ends_with(name))
            })
            .expect("row")
    };
    let r = row_of(&app, "a.txt");
    app.startup_cmd(&format!("list.select:{r}"));
    app.command("view.preview_window");
    assert!(app.open_preview);
    let d = app.dump_of("pvwin").unwrap();
    assert!(
        d.starts_with("closed a.txt lines 1 top 0 images 0\nhello\n"),
        "{d}"
    );
    app.open_preview = false;
    // 암호 zip: 중앙 디렉터리 플래그 bit13(헤더 암호화) → NeedPassword → 마스킹 입력 창.
    let mut z: Vec<u8> = Vec::new();
    z.extend_from_slice(b"PK\x03\x04");
    z.extend_from_slice(&[0u8; 26]);
    let cd_off = z.len() as u32;
    let mut cd: Vec<u8> = Vec::new();
    cd.extend_from_slice(b"PK\x01\x02");
    cd.extend_from_slice(&[0u8; 4]);
    cd.extend_from_slice(&(0x800u16 | 0x2000).to_le_bytes());
    cd.extend_from_slice(&[0u8; 10]);
    cd.extend_from_slice(&7u32.to_le_bytes());
    cd.extend_from_slice(&10u32.to_le_bytes());
    cd.extend_from_slice(&(5u16).to_le_bytes());
    cd.extend_from_slice(&[0u8; 12]);
    cd.extend_from_slice(&0u32.to_le_bytes());
    cd.extend_from_slice(b"s.txt");
    let cd_size = cd.len() as u32;
    z.extend_from_slice(&cd);
    z.extend_from_slice(b"PK\x05\x06");
    z.extend_from_slice(&[0u8; 4]);
    z.extend_from_slice(&1u16.to_le_bytes());
    z.extend_from_slice(&1u16.to_le_bytes());
    z.extend_from_slice(&cd_size.to_le_bytes());
    z.extend_from_slice(&cd_off.to_le_bytes());
    z.extend_from_slice(&0u16.to_le_bytes());
    std::fs::write(dir.join("locked.zip"), &z).unwrap();
    app.command("view.refresh");
    let r = row_of(&app, "locked.zip");
    app.startup_cmd(&format!("list.select:{r}"));
    app.command("view.preview_window");
    assert!(!app.open_preview, "암호 필요 = 창 대신 대화상자");
    let d = app.dump_of("dlg").unwrap();
    assert!(
        d.starts_with("pending Archive password") && d.ends_with("input(masked)\n"),
        "{d}"
    );
    // 빈 암호 확인 → 재시도 문구.
    app.startup_cmd("dlg.pick:1");
    let d = app.dump_of("dlg").unwrap();
    assert!(d.contains("Wrong password"), "{d}");
    // 틀린 암호(내장 zip 리더는 CD 암호화를 풀지 않는다) → 다시 재시도.
    app.startup_cmd("dlg.type:nope");
    app.startup_cmd("dlg.pick:1");
    assert!(app.dump_of("dlg").unwrap().contains("Wrong password"));
    assert_eq!(
        crate::preview::archive::pw::len(),
        0,
        "틀린 암호는 기억하지 않는다"
    );
    app.startup_cmd("dlg.pick:0");
    assert_eq!(app.dump_of("dlg").unwrap(), "none\n");
    assert!(!app.open_preview);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 행/배경 컨텍스트 메뉴: 행 우클릭 = 행 메뉴(열기·편집·삭제·경로/이름 복사·새로 만들기) · 빈 영역 = 배경 메뉴(붙여넣기·undo·새 폴더·새로 고침) ·
/// ctx.pick = 항목 실행(새 폴더 생성) · Shift+F10 명령 = 캐럿 행 메뉴 · 덤프 `ctx`.
#[test]
fn row_and_background_context_menus() {
    let (mut app, dir) = fixture("ctx");
    app.layout_for(1200, 800, 1.0);
    let mut rec = nexa_ctl::RecordCtx::with_surface(1200, 800);
    app.paint_into(&mut rec, 1200, 800, 1.0);
    let list = app.panels[0].rows().bounds();
    // 행 1(a.txt) 위 우클릭.
    let (x, y) = (list.x + 40, list.y + 20 + 10);
    app.route(InputEvent::MouseMove { x, y });
    app.route(InputEvent::RightDown { x, y });
    let d = app.dump_of("ctx").unwrap();
    assert!(
        d.starts_with("row ")
            && d.contains("edit.copy")
            && d.contains("ctx.copy_path")
            && d.contains("ctx.new["),
        "{d}"
    );
    assert!(
        d.contains("fake.open"),
        "셸 항목(가짜 포트)이 상단에 합류: {d}"
    );
    app.startup_cmd("ctx.pick:fake.open");
    let log = app.platform.log.clone().expect("fake log");
    assert!(
        log.borrow()
            .calls
            .iter()
            .any(|c| c == "menu.invoke:fake.open:1"),
        "{:?}",
        log.borrow().calls
    );
    app.route(InputEvent::MouseMove { x, y });
    app.route(InputEvent::RightDown { x, y });
    app.startup_cmd("ctx.pick:ctx.copy_name");
    assert_eq!(app.dump_of("ctx").unwrap(), "none\n");
    // 빈 영역 우클릭 = 배경 메뉴 → 새 폴더.
    let (x, y) = (list.x + 40, list.bottom() - 10);
    app.route(InputEvent::MouseMove { x, y });
    app.route(InputEvent::RightDown { x, y });
    let d = app.dump_of("ctx").unwrap();
    assert!(
        d.starts_with("bg ")
            && d.contains("edit.paste")
            && d.contains("edit.undo")
            && d.contains("view.refresh"),
        "{d}"
    );
    app.startup_cmd("ctx.pick:file.new_folder");
    assert!(dir.join("New Folder").is_dir());
    app.startup_cmd("ui.press:escape");
    // 키보드 메뉴(캐럿 행).
    app.command("cmd.contextMenu");
    assert!(app.dump_of("ctx").unwrap().starts_with("row "));
    app.startup_cmd("ctx.pick:edit.copy");
    assert!(app.dump_of("ops").unwrap().contains("clip 1 copy"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-42 런처 바: 숨김/항목 0 = 높이 0 · 켜고 항목을 주면 도구 모음 아래 24 · 패널이 내려감 · 클릭 id `launch:<i>` 실행 = 상태줄 `launched` ·
/// 실패 = `launch failed` · 구분선은 버튼 아님 · 덤프 `launcher`.
#[test]
fn launcher_bar_layout_and_launch() {
    let (mut app, dir) = fixture("launch");
    app.layout_for(1200, 800, 1.0);
    assert_eq!(app.launcherbar.bounds().h, 0, "픽스처 = 숨김");
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let items = format!("Self|{exe}|--version;;-;;Bad|nope-xyz-program|");
    let _ = app.settings.set("launcher.items", &items);
    app.apply_setting("launcher.items");
    let _ = app.settings.set("launcher.visible", "on");
    app.apply_setting("launcher.visible");
    let lb = app.launcherbar.bounds();
    assert_eq!((lb.y, lb.h), (app.toolbar.bounds().bottom(), 28), "{lb:?}");
    assert_eq!(app.panels[0].bounds().y, lb.bottom(), "패널은 런처 아래");
    assert!(app.dump_of("layout").unwrap().contains("launcher 0,"));
    let ld = app.dump_of("launcher").unwrap();
    assert_eq!(ld.lines().count(), 5, "{ld}");
    // T-30 B 아이콘: 없는 exe = 즉시 글리프 `Ba` · 자기 exe = Windows면 셸 아이콘(비동기 → 폴링 후 image) · 다른 OS = 글리프 `Se`.
    // 아이콘 테마가 있는 Linux = 둘 다 그림(앱 아이콘 · 못 찾으면 일반 실행 파일 아이콘 — 사용자 10-03).
    let themed = nexa_fs::icontheme::theme_name().is_some();
    assert_eq!(ld.contains("launch:2=glyph:Ba"), !themed, "{ld}");
    assert_eq!(ld.contains("launch:2=image"), themed, "{ld}");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while app.launcher_icons_tick(std::time::Instant::now()).is_some()
        && std::time::Instant::now() < deadline
    {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let icons = app.launcher_icon_summary();
    if cfg!(windows) {
        assert!(
            icons.iter().any(|s| s.starts_with("launch:0=image 16x16")),
            "{icons:?}"
        );
    } else if themed {
        assert!(
            icons.iter().any(|s| s.starts_with("launch:0=image")),
            "{icons:?}"
        );
    } else {
        assert!(icons.iter().any(|s| s == "launch:0=glyph:Se"), "{icons:?}");
    }
    // 상태줄은 목록 갱신 틱이 덮어쓰므로 `launcher_last`(덤프 첫 줄)로 본다.
    app.command("launch:0");
    assert!(
        app.launcher_last.contains("launched"),
        "{}",
        app.launcher_last
    );
    assert!(app
        .dump_of("launcher")
        .unwrap()
        .starts_with("last Self launched"));
    app.command("launch:2");
    assert!(
        app.launcher_last.contains("launch failed"),
        "{}",
        app.launcher_last
    );
    app.command("launch:1");
    assert!(
        app.launcher_last.contains("launch failed"),
        "구분선은 실행 불가"
    );
    // 끄면 0 · 패널 복귀.
    app.command("view.launcher");
    assert_eq!(app.launcherbar.bounds().h, 0);
    assert_eq!(app.panels[0].bounds().y, app.toolbar.bounds().bottom());
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-80 라이선스: 픽스처 격리 폴더 = Free · 보기 표 · About = 대화상자([라이선스…] = 2 → 창 깃발) · Help ▸ 라이선스… 토글 ·
/// 손상 파일 설치 = 거부(파일 안 씀 · 상태줄/덤프 안내) · 제거 = 없음 안내 · 파일 창 사양(라이선스 = 열기 + .license 필터 · 설정 = 폴더).
#[test]
fn license_view_about_install_rejects_garbage() {
    let (mut app, dir) = fixture("license");
    let v = app.license_view();
    assert_eq!(v.state, "Free · non-commercial use only");
    assert!(v.warn && v.request.is_some());
    assert_eq!(v.rows[0], ("State".to_string(), "free".to_string()));
    assert_eq!(v.rows[1].1, "-", "파일 없음");
    assert!(v.rows.iter().any(|(k, val)| k == "Install location"
        && val.starts_with(&dir.join("license").display().to_string())));
    assert_eq!(v.contact, ndir_license::LICENSE_CONTACT);
    assert!(app.dump_of("license").unwrap().starts_with("badge=Free"));
    // About = 대화상자(제목 · 라이선스 줄 · [라이선스…][OK]) → 2 = 라이선스 창 요청.
    app.command("help.about");
    let d = app.dump_of("dlg").unwrap();
    assert!(
        d.contains("About Nexa Dir") && d.contains("2:License…") && d.contains("License: Free"),
        "{d}"
    );
    app.dlg_pick(2);
    assert!(app.open_license);
    app.open_license = false;
    app.command("help.license");
    assert!(app.open_license, "창이 없으면 열기 요청");
    // 손상 파일 → 거부 · 설치 자리에 아무것도 쓰지 않는다.
    let bad = dir.join("bad.license");
    std::fs::write(&bad, b"not a license").unwrap();
    app.license_install(&bad);
    let ld = app.dump_of("license").unwrap();
    assert!(
        ld.contains("state=free") && ld.contains("note=warn:Not installed: invalid"),
        "{ld}"
    );
    assert!(!dir.join("license").exists());
    assert_eq!(
        app.statusbar.left(),
        "Not installed: invalid (the file was not changed)"
    );
    app.license_remove();
    assert!(app
        .dump_of("license")
        .unwrap()
        .contains("note=warn:No license file to remove"));
    // 파일 창 사양.
    app.open_file_window(app::license::FilePurpose::License);
    let (mode, _, filters) = app.file_window_spec();
    assert!(app.open_file && mode == nexa_dlg::PickerMode::Open && filters.len() == 2);
    // 폴더형 설정 키는 아직 없다(prefs_win `is_folder_key` = false) — Text 키로 저장 경로만 검증.
    app.open_file_window(app::license::FilePurpose::Setting("launcher.items".into()));
    assert_eq!(app.file_window_spec().0, nexa_dlg::PickerMode::Folder);
    app.file_confirmed(dir.join("sub"));
    assert_eq!(
        app.settings.get("launcher.items"),
        Some(dir.join("sub").display().to_string().as_str())
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-62 C 압축 그리드 창: F3(view.preview_window)로 zip을 열면 텍스트 창이 아니라 그리드 창 요청 · 자료 = 읽어 둔 목록 ·
/// 덤프(행·상태·셀) · 선택/TSV · 정렬 통지 · 손상 파일 = 실패 사유 줄로도 연다.
#[test]
fn archive_grid_window_from_preview() {
    let (mut app, dir) = fixture("arcgrid");
    app.layout_for(1200, 800, 1.0);
    std::fs::write(
        dir.join("z.zip"),
        preview::archive::zip_bytes_for_tests("docs/readme.md"),
    )
    .unwrap();
    std::fs::write(dir.join("bad.zip"), b"not a zip at all").unwrap();
    let mut inv = Invalidations::default();
    let _ = app.panels[0].navigate_to(dir.clone(), &mut inv);
    let row = (0..app.panels[0].rows().source().len())
        .find(|&i| app.panels[0].rows().source().row(i).text == "z.zip")
        .expect("z.zip row");
    app.panels[0]
        .rows_mut()
        .select_program(row, nexa_grid::SelectOp::Single, &mut inv);
    app.command("view.preview_window");
    assert!(app.open_archive && !app.open_preview, "압축 = 그리드 창");
    let d = app.dump_of("archive").unwrap();
    assert!(
        d.contains("title=z.zip — Archive")
            && d.contains("rows=1")
            && d.contains("status=ZIP · 1 items")
            && d.contains("readme.md | docs | "),
        "{d}"
    );
    let src = app.archive_win.source_mut().unwrap();
    assert!(src.select(0, nexa_grid::SelectOp::Single));
    assert!(app.archive_win.tsv().starts_with("readme.md\tdocs\t"));
    assert!(app.archive_win.source_mut().unwrap().set_sort(&[(0, true)]));
    // 손상 파일 = 실패 사유를 상태 줄에 · 행 0.
    app.open_archive = false;
    let row = (0..app.panels[0].rows().source().len())
        .find(|&i| app.panels[0].rows().source().row(i).text == "bad.zip")
        .expect("bad.zip row");
    app.panels[0]
        .rows_mut()
        .select_program(row, nexa_grid::SelectOp::Single, &mut inv);
    app.command("view.preview_window");
    assert!(app.open_archive);
    let d = app.dump_of("archive").unwrap();
    assert!(
        d.contains("rows=0") && d.contains("status=could not read the archive"),
        "{d}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-30 툴바 SVG 아이콘: 13개 명령 전부 마스크 아이콘(글리프 폴백 아님) · 배율 2 = 40px 마스크 · 재구성 뒤 체크 상태 유지.
#[test]
fn toolbar_uses_svg_masks_and_rebuilds_on_scale() {
    let (mut app, dir) = fixture("tbicons");
    let masks = app
        .toolbar
        .all_items()
        .iter()
        .filter(|it| matches!(it.icon, nexa_ctl::ToolIcon::Mask { w: 20, h: 20, .. }))
        .count();
    // 14개 명령 = SVG 마스크(20px) · 점 파일 토글이 없는 OS(Linux · macOS)는 view.dot이 빠져 13개.
    let dot = usize::from(platform::has_dotfile_toggle());
    assert_eq!(masks, 13 + dot);
    // 대소문자 구분 정렬 토글 = 폴더 우선 바로 다음 · 누르면 전역 설정과 체크가 함께 바뀐다.
    let ids: Vec<String> = app
        .toolbar
        .all_items()
        .iter()
        .map(|t| t.id.clone())
        .collect();
    let ff = ids
        .iter()
        .position(|i| i == "view.folders_first")
        .expect("folders_first");
    assert_eq!(
        ids.get(ff + 1).map(String::as_str),
        Some("view.case_sensitive")
    );
    // 네 번째 탭 보기 옵션: 범위(기본 = 폴더)만큼 적용 · 설정값(새 탭 기본값)은 그대로 · 세션 플래그 bit3.
    assert!(!app.toolbar.item_checked("view.case_sensitive"));
    app.command("view.case_sensitive");
    assert!(
        !app.settings.flag("list.sort_case_sensitive"),
        "설정 = 새 탭 기본값은 불변"
    );
    assert!(app.toolbar.item_checked("view.case_sensitive"));
    assert_eq!(
        app.panels[app.active].active_view(),
        (true, true, true, true)
    );
    assert_eq!(app.session_snapshot().panels[app.active].views, vec![15]);
    let root = app.panels[app.active].root_path();
    assert_eq!(
        app.dir_view_of(&root),
        (true, true, true, true),
        "폴더에 기억"
    );
    app.command("view.case_sensitive");
    assert!(!app.toolbar.item_checked("view.case_sensitive"));
    assert!(
        app.session_snapshot().dir_views.is_empty(),
        "기본값 = 기억에서 빠짐"
    );
    // 설정을 켜면 새 탭의 기본값만 바뀐다(열린 탭은 그대로).
    let _ = app.settings.set("list.view_scope", "tab");
    let _ = app.settings.set("list.sort_case_sensitive", "on");
    app.after_setting_changed("list.sort_case_sensitive");
    assert!(!app.panels[app.active].active_view().3);
    app.command("file.new_tab");
    assert!(
        app.panels[app.active].active_view().3,
        "새 탭 = 설정 기본값"
    );
    let _ = app.settings.reset("list.sort_case_sensitive");
    let _ = app.settings.reset("list.view_scope");
    app.after_setting_changed("list.sort_case_sensitive");
    assert_eq!(
        app.toolbar.all_items().iter().any(|it| it.id == "view.dot"),
        dot == 1
    );
    app.layout_for(1200, 800, 2.0);
    app.rebuild_toolbar();
    app.sync_menu_checks();
    assert!(
        app.toolbar
            .all_items()
            .iter()
            .all(|it| it.separator
                || matches!(it.icon, nexa_ctl::ToolIcon::Mask { w: 40, h: 40, .. }))
    );
    assert_eq!(App::toolbar_icon_px(1.5), 30);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 숨김 파일 규칙(사용자 10-03 "리눅스는 . 으로 시작하면 숨김파일 · dot file 토글은 윈도우에서만"): Unix에서는 "숨김 파일 표시"가
/// 점 파일을 켜고 끄고 · "점 파일 표시"는 메뉴에 없고 명령도 아무 일을 하지 않는다. Windows는 종전대로 점 파일 토글이 따로 있다.
#[test]
fn hidden_toggle_covers_dot_files_on_unix() {
    let (mut app, dir) = fixture("dothidden");
    app.layout_for(1200, 800, 1.0);
    std::fs::write(dir.join(".secret"), b"x").unwrap();
    let mut inv = Invalidations::default();
    let _ = app.panels[0].navigate_to(dir.clone(), &mut inv);
    let shown = |app: &App| {
        let src = app.panels[0].rows().source();
        (0..src.len())
            .filter_map(|i| src.row_path(i))
            .any(|p| p.ends_with(".secret"))
    };
    assert!(shown(&app), "기본 = 숨김·점 파일 표시");
    let dot_toggle = platform::has_dotfile_toggle();
    assert_eq!(dot_toggle, !ndir_vfs::DOT_IS_HIDDEN);
    let menus = format!("{:?}", App::build_menus(&app.settings));
    assert!(menus.contains("view.hidden"));
    assert_eq!(
        menus.contains("view.dot"),
        dot_toggle,
        "점 파일 표시 = Windows 메뉴에만"
    );
    assert!(app::menus::menu_has("view.hidden", false) && !app::menus::menu_has("view.dot", false));
    assert!(app::menus::menu_has("view.dot", true));
    // 점 파일 토글 명령: Windows = 점 파일을 끈다 · Unix = 설정도 목록도 그대로.
    app.command("view.dot");
    assert_eq!(app.panels[0].active_view_values().1, !dot_toggle);
    assert_eq!(shown(&app), !dot_toggle);
    if dot_toggle {
        app.command("view.dot");
    }
    // 숨김 토글 명령: Unix = 점 파일이 사라진다 · Windows = 숨김 속성만 다루므로 점 파일은 남는다.
    app.command("view.hidden");
    assert!(!app.panels[0].active_view_values().0);
    assert_eq!(shown(&app), dot_toggle);
    app.command("view.hidden");
    assert!(shown(&app));
    let _ = std::fs::remove_dir_all(&dir);
}

/// 탭별 보기 옵션(dir2 08-02 이식 + 사용자 10-03 규칙): 숨김 · Dot · 폴더 우선의 주인은 **탭** · 토글은 범위(`list.view_scope` ·
/// 기본 = 활성 탭)만큼 · 설정값은 새 탭의 기본값 · 보호 항목 표시는 전역 · 체크는 활성 탭을 따라간다 · 세션에 탭별로 남는다.
#[test]
fn view_options_belong_to_tabs() {
    let (mut app, dir) = fixture("tabview");
    app.layout_for(1200, 800, 1.0);
    let mut inv = Invalidations::default();
    let _ = app.panels[0].navigate_to(dir.clone(), &mut inv);
    assert_eq!(app.view_scope(), "dir", "기본 = 폴더별");
    let _ = app.settings.set("list.view_scope", "tab");
    let view = |app: &App, p: usize| app.panels[p].active_view_values();
    assert_eq!(view(&app, 0), (true, true, true));
    // 탭 0에서 폴더 우선을 끈다 → 새 탭(설정 기본값)은 켜져 있고 · 탭을 오가면 체크가 따라간다.
    app.command("view.folders_first");
    assert_eq!(view(&app, 0), (true, true, false));
    assert!(
        app.settings.flag("list.folders_first"),
        "설정 = 새 탭 기본값은 불변"
    );
    assert!(!app.toolbar.item_checked("view.folders_first"));
    app.command("file.new_tab");
    assert_eq!(view(&app, 0), (true, true, true), "새 탭 = 설정 기본값");
    assert!(app.toolbar.item_checked("view.folders_first"));
    app.command("tab.prev");
    assert_eq!(view(&app, 0), (true, true, false), "탭 0은 자기 값");
    assert!(
        !app.toolbar.item_checked("view.folders_first"),
        "체크 = 활성 탭"
    );
    // 세션: 탭별 플래그(bit0 숨김 · bit1 Dot · bit2 폴더 우선).
    assert_eq!(app.session_snapshot().panels[0].views, vec![3, 7]);
    // 다른 패널은 영향 없음(범위 = 탭).
    assert_eq!(view(&app, 1), (true, true, true));
    // 범위 = 패널: 활성 패널의 전 탭 · 범위 = 전체: 두 패널 전 탭.
    let _ = app.settings.set("list.view_scope", "panel");
    app.command("view.hidden");
    assert_eq!(app.session_snapshot().panels[0].views, vec![2, 2]);
    assert!(view(&app, 1).0);
    let _ = app.settings.set("list.view_scope", "global");
    app.command("view.hidden");
    assert_eq!(app.session_snapshot().panels[0].views, vec![3, 3]);
    assert_eq!(
        view(&app, 1),
        (true, true, false),
        "전체 = 활성 탭 값이 두 패널로"
    );
    // 전역 설정(보호 항목 · 대소문자)을 바꿔도 탭 값은 그대로 · 숨김 기본값을 바꿔도 열린 탭은 그대로.
    app.command("view.folders_first");
    let before = app.session_snapshot().panels[0].views.clone();
    let _ = app.settings.set("list.show_protected", "on");
    app.after_setting_changed("list.show_protected");
    let _ = app.settings.set("list.show_hidden", "off");
    app.after_setting_changed("list.show_hidden");
    assert_eq!(app.session_snapshot().panels[0].views, before);
    assert!(
        app.panels[0].tab_opts().show_protected,
        "보호 항목 = 전역(전 탭)"
    );
    // 탭 복제 = 원본 탭 값 계승.
    let _ = app.settings.set("list.view_scope", "tab");
    app.command("view.hidden");
    let src = view(&app, 0);
    let mut inv = Invalidations::default();
    let i = app.panels[0].active_index();
    app.panels[0].duplicate_tab(i, &mut inv);
    assert_eq!(view(&app, 0), src);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 보기 옵션 관리 방법 "폴더"(기본 · 사용자 10-03 4택): 같은 폴더를 보는 탭은 좌우 어디든 함께 바뀌고 · 폴더마다 기억해
/// 다시 들어가면 그 값 · 기억이 없는 폴더 = 설정 기본값 · 세션에 남는다.
#[test]
fn view_options_follow_folders_by_default() {
    let (mut app, dir) = fixture("dirview");
    app.layout_for(1200, 800, 1.0);
    let sub = dir.join("sub");
    let mut inv = Invalidations::default();
    let _ = app.panels[0].navigate_to(dir.clone(), &mut inv);
    let _ = app.panels[1].navigate_to(dir.clone(), &mut inv);
    app.update_status();
    assert_eq!(app.view_scope(), "dir");
    let view = |app: &App, p: usize| app.panels[p].active_view_values();
    // 왼쪽에서 폴더 우선을 끄면 같은 폴더를 보는 오른쪽도 함께 꺼진다.
    app.command("view.folders_first");
    assert_eq!(view(&app, 0), (true, true, false));
    assert_eq!(view(&app, 1), (true, true, false), "같은 폴더 = 공통");
    assert_eq!(app.dir_view_of(&dir), (true, true, false, false));
    assert!(
        app.settings.flag("list.folders_first"),
        "설정 기본값은 불변"
    );
    // 다른 폴더로 들어가면 그 폴더의 값(기억 없음 = 설정 기본값) · 돌아오면 기억된 값.
    let _ = app.panels[0].navigate_to(sub.clone(), &mut inv);
    app.update_status();
    assert_eq!(view(&app, 0), (true, true, true), "기억 없는 폴더 = 기본값");
    assert!(app.toolbar.item_checked("view.folders_first"));
    assert_eq!(
        view(&app, 1),
        (true, true, false),
        "다른 폴더를 보는 탭은 그대로"
    );
    app.panels[0].nav_back(&mut inv);
    app.update_status();
    assert_eq!(view(&app, 0), (true, true, false), "돌아오면 그 폴더의 값");
    // 새 탭(같은 폴더) = 그 폴더의 값.
    app.command("file.new_tab");
    assert_eq!(view(&app, 0), (true, true, false));
    // sub에서 숨김을 끈다 → sub만 · 세션에 두 폴더가 기억된다.
    let _ = app.panels[0].navigate_to(sub.clone(), &mut inv);
    app.update_status();
    app.command("view.hidden");
    assert_eq!(view(&app, 0), (false, true, true));
    assert_eq!(view(&app, 1), (true, true, false));
    let snap = app.session_snapshot();
    assert_eq!(snap.dir_views, vec![(dir.clone(), 3), (sub, 6)]);
    assert_eq!(Session::parse(&snap.serialize()).dir_views, snap.dir_views);
    // 기본값으로 되돌리면 기억에서 빠진다.
    app.command("view.hidden");
    assert_eq!(app.session_snapshot().dir_views, vec![(dir.clone(), 3)]);
    // 범위를 탭으로 바꾸면 폴더를 옮겨도 탭 값이 따라간다(폴더 기억을 쓰지 않는다).
    let _ = app.settings.set("list.view_scope", "tab");
    app.command("view.hidden");
    let _ = app.panels[0].navigate_to(dir.clone(), &mut inv);
    app.update_status();
    assert_eq!(view(&app, 0), (false, true, true), "탭 범위 = 탭이 지닌 값");
    let _ = std::fs::remove_dir_all(&dir);
}

/// 네비 버튼 모양(사용자 10-03 "윈도우 기준으로 일치"): 아이콘 글꼴(MDL2)이 있으면 글리프 · 없으면 SVG 마스크(배율만큼 큰 그림) ·
/// 도크 머리의 종류 칸은 마우스를 올리면 hover 표시 · 창을 벗어나면 풀린다.
#[test]
fn nav_buttons_use_vector_icons_and_dock_tabs_hover() {
    let (mut app, dir) = fixture("navicons");
    app.layout_for(1200, 800, 1.0);
    // 판정은 이 앱의 버튼으로(전역 "아이콘 글꼴 있음" 깃발은 다른 시험 스레드가 바꿀 수 있다): 네 개가 전부 마스크이거나 전부 글리프.
    let masks = app.panels[0]
        .nav_items()
        .iter()
        .filter(|it| matches!(it.icon, nexa_ctl::ToolIcon::Mask { w: 14, h: 14, .. }))
        .count();
    assert!(masks == 4 || masks == 0, "{masks}");
    let vector = masks == 4;
    #[cfg(target_os = "linux")]
    assert!(vector, "Linux에는 Segoe MDL2가 없다 → SVG 마스크");
    app.layout_for(2400, 1600, 2.0);
    let big = app.panels[0]
        .nav_items()
        .iter()
        .filter(|it| matches!(it.icon, nexa_ctl::ToolIcon::Mask { w: 28, h: 28, .. }))
        .count();
    if vector {
        assert_eq!(big, 4, "배율 2 = 28 px 마스크");
    }
    // 도크 종류 칸 hover.
    app.layout_for(1200, 800, 1.0);
    let mut rec = nexa_ctl::RecordCtx::with_surface(1200, 800);
    app.paint_into(&mut rec, 1200, 800, 1.0);
    let (db, cr) = (app.docks[0].bounds(), app.docks[0].content_rect());
    let y = (db.y + cr.y) / 2;
    let mut seen = std::collections::BTreeSet::new();
    for x in (db.x + 4..db.x + 300).step_by(6) {
        app.route(InputEvent::MouseMove { x, y });
        if let Some(i) = app.docks[0].strip_hover() {
            seen.insert(i);
        }
    }
    assert!(seen.len() >= 3, "종류 칸마다 hover: {seen:?}");
    assert_eq!(app.docks[1].strip_hover(), None);
    app.pointer_gone();
    assert_eq!(app.docks[0].strip_hover(), None, "창 밖 = 해제");
    let _ = std::fs::remove_dir_all(&dir);
}

/// 열 폭 동기(사용자 10-03 점검): 켜는 순간(툴바 명령 · 설정 창 어느 길이든) 활성 패널 기준으로 맞춘다 · 폭은 **열 종류별로**
/// 옮긴다(열 순서가 달라도 같은 열끼리) · 반대 패널의 모든 탭과 같은 패널의 다른 탭도 따라온다 · 꺼져 있으면 독립.
#[test]
fn col_width_sync_matches_by_column_key() {
    let (mut app, dir) = fixture("colsync");
    app.layout_for(1200, 800, 1.0);
    let mut inv = Invalidations::default();
    let widths = |app: &App, p: usize| app.panels[p].col_widths_by_key();
    let w_of = |v: &[(u32, i32)], key: u32| v.iter().find(|(k, _)| *k == key).map(|(_, w)| *w);
    let (name, size) = (filelist::COL_NAME, filelist::COL_SIZE);
    // 꺼 둔 채 왼쪽만 바꾼다 → 오른쪽은 그대로.
    let _ = app.settings.set("list.col_width_sync", "off");
    app.after_setting_changed("list.col_width_sync");
    let right0 = widths(&app, 1);
    app.panels[0].apply_col_widths_by_key(&[(name, 333), (size, 111)], &mut inv);
    assert_eq!(widths(&app, 1), right0, "꺼짐 = 독립");
    // 오른쪽 열 순서를 바꿔 둔다(크기 열을 맨 앞쪽으로) — 자리로 복사하면 엉뚱한 열에 들어가는 조건.
    app.set_active(1);
    app.apply_col_layout_str("cols:1[name:1,size:1,ext:1,modified:1,kind:1]");
    app.set_active(0);
    // 설정 창 길로 켠다 → 즉시 활성(왼쪽) 기준으로 · 같은 열끼리.
    let _ = app.settings.set("list.col_width_sync", "on");
    app.after_setting_changed("list.col_width_sync");
    let r = widths(&app, 1);
    assert_eq!(
        (w_of(&r, name), w_of(&r, size)),
        (Some(333), Some(111)),
        "{r:?}"
    );
    assert_ne!(
        r.iter().map(|(k, _)| *k).collect::<Vec<_>>(),
        widths(&app, 0).iter().map(|(k, _)| *k).collect::<Vec<_>>(),
        "열 순서는 각자"
    );
    // 새 탭과 같은 패널의 다른 탭도 같은 폭.
    app.command("file.new_tab");
    app.panels[0].apply_col_widths_by_key(&[(name, 400)], &mut inv);
    app.sync_col_widths_from(0);
    app.command("tab.prev");
    assert_eq!(
        w_of(&widths(&app, 0), name),
        Some(400),
        "같은 패널의 다른 탭"
    );
    assert_eq!(w_of(&widths(&app, 1), name), Some(400));
    // 툴바 명령으로 껐다 켜도 같은 규칙(기준 = 활성 패널).
    app.command("view.col_width_sync");
    app.panels[1].apply_col_widths_by_key(&[(size, 77)], &mut inv);
    assert_eq!(w_of(&widths(&app, 0), size), Some(111));
    app.set_active(1);
    app.command("view.col_width_sync");
    assert_eq!(
        w_of(&widths(&app, 0), size),
        Some(77),
        "켜는 순간 활성(오른쪽) 기준"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 탭 여러 줄(T-121 · 사용자 10-03 "Multi-line 기본 · Single-line은 옵션 · 한 줄일 때 ◀ ▶ 자리 3가지"): 기본 = 폭을 넘으면
/// 다음 줄(그리기가 줄 수를 재고 다시 배치 → 목록이 내려간다) · 끄면 한 줄 · 배율 2에서도 줄 높이 = 22 × 2(논리 px 전달).
#[test]
fn tabs_wrap_into_lines_by_default() {
    let (mut app, dir) = fixture("tablines");
    app.layout_for(1200, 800, 1.0);
    app.apply_tab_style();
    assert!(app.settings.flag("tabs.multiline"), "기본 = 여러 줄");
    let paint = |app: &mut App, w: i32, h: i32, s: f32| {
        // 줄 수는 그리기가 잰다 → 바뀌면 다시 배치(실제 앱은 다음 프레임 · 시험은 한 번 더 그린다).
        for _ in 0..2 {
            let mut rec = nexa_ctl::RecordCtx::with_surface(w, h);
            app.paint_into(&mut rec, w, h, s);
        }
    };
    paint(&mut app, 1200, 800, 1.0);
    assert_eq!(app.panels[0].tab_lines(), 1);
    let list_y1 = app.panels[0].rows().bounds().y;
    for _ in 0..14 {
        app.command("file.new_tab");
    }
    paint(&mut app, 1200, 800, 1.0);
    let lines = app.panels[0].tab_lines();
    assert!(lines >= 2, "탭 15개 = 여러 줄: {lines}");
    let list_y2 = app.panels[0].rows().bounds().y;
    assert_eq!(
        list_y2 - list_y1,
        22 * (lines as i32 - 1),
        "목록이 줄 수만큼 내려간다"
    );
    assert_eq!(app.panels[1].tab_lines(), 1, "반대 패널은 그대로");
    // 배율 2: 줄 높이 = 44(장치 px) — 줄 수가 같으면 탭 바 높이 = 44 × 줄 수.
    app.layout_for(2400, 1600, 2.0);
    paint(&mut app, 2400, 1600, 2.0);
    let l2 = app.panels[0].tab_lines() as i32;
    let tab_h = app.panels[0].rows().bounds().y - app.panels[0].bounds().y - 48;
    assert_eq!(tab_h, 44 * l2, "배율 2 탭 바 높이(네비 줄 48 제외)");
    // 한 줄 옵션 + 버튼 자리.
    app.layout_for(1200, 800, 1.0);
    let _ = app.settings.set("tabs.multiline", "off");
    let _ = app.settings.set("tabs.scroll_buttons", "split");
    app.after_setting_changed("tabs.multiline");
    paint(&mut app, 1200, 800, 1.0);
    assert_eq!(app.panels[0].tab_lines(), 1);
    assert_eq!(
        app.panels[0].rows().bounds().y,
        list_y1,
        "한 줄 = 원래 자리"
    );
    assert_eq!(
        app.panels[0].tabbar.scroll_buttons(),
        nexa_ctl::controls::ScrollButtons::Split
    );
    // 버튼 자리 설정은 여러 줄을 끈 경우에만 쓸 수 있다(종속).
    assert_eq!(
        ndir_settings::dependency("tabs.scroll_buttons").map(|d| d.0),
        Some("tabs.multiline")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 다중 정렬 머리(T-127 · 사용자 10-03 · nexa-sql 모양): 정렬 표시는 칸 오른쪽 끝 · 다중 정렬일 때만 순번이 그 오른쪽 ·
/// Shift+클릭 = 오름 → 내림 → 없음 · 새 탭도 같은 모양.
#[test]
fn header_sort_marks_trail_and_shift_cycles() {
    let (mut app, dir) = fixture("sortmarks");
    app.layout_for(1200, 800, 1.0);
    let b = app.panels[0].rows().bounds();
    let cols: Vec<(u32, i32)> = app.panels[0].col_widths_by_key();
    // 열 가운데 x(머리 줄).
    let mid = |i: usize| b.x + cols[..i].iter().map(|c| c.1).sum::<i32>() + cols[i].1 / 2;
    let y = b.y + 5;
    let click = |app: &mut App, x: i32, shift: bool| {
        app.route(InputEvent::MouseDown {
            x,
            y,
            shift,
            primary: false,
        });
        app.route(InputEvent::MouseUp { x, y });
    };
    let mark = |app: &App, i: usize| app.panels[0].rows().sort_mark(cols[i].0);
    click(&mut app, mid(0), false);
    assert_eq!(mark(&app, 0).as_deref(), Some("▲"), "단일 = 순번 없음");
    click(&mut app, mid(2), true);
    assert_eq!(
        (mark(&app, 0).as_deref(), mark(&app, 2).as_deref()),
        (Some("▲1"), Some("▲2"))
    );
    click(&mut app, mid(2), true);
    assert_eq!(mark(&app, 2).as_deref(), Some("▼2"));
    click(&mut app, mid(2), true);
    assert_eq!((mark(&app, 0).as_deref(), mark(&app, 2)), (Some("▲"), None));
    let mut rec = nexa_ctl::RecordCtx::with_surface(1200, 800);
    app.paint_into(&mut rec, 1200, 800, 1.0);
    assert!(
        rec.drew_text("▲") && rec.drew_text("Name"),
        "표시와 제목을 따로 그린다"
    );
    app.command("file.new_tab");
    click(&mut app, mid(0), false);
    assert_eq!(mark(&app, 0).as_deref(), Some("▲"), "새 탭도 끝 정렬 모양");
    let _ = std::fs::remove_dir_all(&dir);
}

/// 열 경계 더블클릭 = 자동 맞춤(T-132 · dir2 07-19 `autofit_column`): 보이는 행 + 머리글 폭 · 머리글은 제목 + 정렬 표시 + 순번까지 ·
/// 상한 = `list.col_autofit_max` · 반대 패널 동기 · 세션에 남는다.
#[test]
fn header_edge_double_click_autofits_column() {
    let (mut app, dir) = fixture("autofit");
    app.layout_for(1200, 800, 1.0);
    let b = app.panels[0].rows().bounds();
    let y = b.y + 5;
    let width = |app: &App, p: usize, key: u32| {
        app.panels[p]
            .col_widths_by_key()
            .iter()
            .find(|c| c.0 == key)
            .map(|c| c.1)
            .unwrap()
    };
    // 열 i의 오른쪽 경계 x · 가운데 x(지금 폭 기준).
    let edge = |app: &App, i: usize| {
        b.x + app.panels[0].col_widths_by_key()[..=i]
            .iter()
            .map(|c| c.1)
            .sum::<i32>()
    };
    let dbl = |app: &mut App, x: i32| {
        app.route(InputEvent::DoubleClick {
            x,
            y,
            shift: false,
            primary: false,
        });
        app.route(InputEvent::MouseUp { x, y });
    };
    let size_px = app.ui_font.em_to_px(app.settings.font_px("list.font_size"));
    let pad2 = 2 * panel_metrics(&app.settings, 1.0).pad_x;
    // 상태 열(데이터 = 아이콘뿐): 머리글 폭으로 줄어든다.
    let x = edge(&app, 1);
    dbl(&mut app, x);
    let plain = width(&app, 0, filelist::COL_STATUS);
    let text_w = app.ui_font.measure(&tr("col.status"), size_px).ceil() as i32;
    assert_eq!(plain, (text_w + pad2 + 1).max(40));
    assert_eq!(
        width(&app, 1, filelist::COL_STATUS),
        plain,
        "반대 패널 동기"
    );
    // 넓힌 뒤 다시 더블클릭 = 같은 폭으로 돌아온다(사용자 폭으로 남아 배치가 덮지 않는다).
    let mut inv = Invalidations::default();
    app.panels[0].set_col_width_user(1, 100, &mut inv);
    app.layout_for(1200, 800, 1.0);
    assert_eq!(width(&app, 0, filelist::COL_STATUS), 100);
    let x = edge(&app, 1);
    dbl(&mut app, x);
    assert_eq!(width(&app, 0, filelist::COL_STATUS), plain);
    // 상한: 이름 열(긴 이름) = `list.col_autofit_max`를 넘지 않는다.
    let _ = app.settings.set("list.col_autofit_max", "50");
    let x = edge(&app, 0);
    dbl(&mut app, x);
    assert!(
        width(&app, 0, filelist::COL_NAME) <= 120,
        "상한 50 · 열 최소 폭 120"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 열 폭을 끄는 중 포인터가 창을 벗어나도(`pointer_gone`) 폭이 가짜 좌표를 따라가지 않는다(종전 = 최소 폭 40으로 줄었다).
#[test]
fn column_resize_survives_pointer_leaving_window() {
    let (mut app, dir) = fixture("resizegone");
    app.layout_for(1200, 800, 1.0);
    let b = app.panels[0].rows().bounds();
    let cols = app.panels[0].col_widths_by_key();
    let (x, y) = (b.x + cols[0].1 + cols[1].1 + cols[2].1 - 2, b.y + 5);
    app.route(InputEvent::MouseDown {
        x,
        y,
        shift: false,
        primary: false,
    });
    app.route(InputEvent::MouseMove { x: x + 20, y });
    let grown = app.panels[0].col_widths_by_key()[2].1;
    assert_eq!(grown, cols[2].1 + 20);
    app.pointer_gone();
    assert_eq!(
        app.panels[0].col_widths_by_key()[2].1,
        grown,
        "끄는 중 = 그대로"
    );
    app.route(InputEvent::MouseUp { x: x + 20, y });
    assert_eq!(app.panels[0].col_widths_by_key()[2].1, grown);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 언어 전환(사용자 10-03 "파일 그리드도 다국어 변경이 안 됨"): 사용자 열 레이아웃(폭을 바꾼 패널 · 세션 복원 패널)이어도
/// 열 제목은 새 열 정의의 제목으로 바뀐다 · 폭은 그대로 · 세션 복원 패널의 기본 열이 채워져 내 PC 열 교체가 된다.
#[test]
fn column_titles_follow_language_even_with_user_layout() {
    let (mut app, dir) = fixture("coltitles");
    app.layout_for(1200, 800, 1.0);
    let mut inv = Invalidations::default();
    assert!(app.panels[0].set_col_width_user(2, 123, &mut inv));
    let renamed = |mut cols: Vec<Column>| {
        for c in &mut cols {
            c.title = format!("X-{}", c.title);
        }
        cols
    };
    app.panels[0].set_default_columns(
        renamed(columns_for(600, 1.0)),
        renamed(all_columns_for(600, 1.0)),
        &mut inv,
    );
    let cols = app.panels[0].rows().columns().to_vec();
    assert!(cols.iter().all(|c| c.title.starts_with("X-")), "{cols:?}");
    assert_eq!(cols[2].width, 123, "폭은 그대로");
    // 세션에서 복원한 패널(열 폭 있음 = 사용자 레이아웃): 기본 열이 채워져 내 PC로 가면 드라이브 열로 바뀐다.
    let ps = crate::session::PanelSession {
        tabs: vec![dir.clone()],
        col_widths: vec![300, 60, 90, 150],
        ..Default::default()
    };
    let mut p = Panel::restore(
        &ps,
        &dir,
        list_opts(&app.settings),
        panel_metrics(&app.settings, 1.0),
        columns_for(600, 1.0),
        all_columns_for(600, 1.0),
    );
    p.set_default_columns(columns_for(600, 1.0), all_columns_for(600, 1.0), &mut inv);
    let _ = p.navigate_to(PathBuf::from(ndir_vfs::MY_PC), &mut inv);
    assert!(
        p.rows()
            .columns()
            .iter()
            .any(|c| c.key == filelist::COL_TOTAL),
        "내 PC = 드라이브 열"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 상태줄 구성(T-94 · NEW-003) + 탭 상태바(T-95 · NEW-004 1차): 오른쪽 칸 = `statusbar.items` 순서 · 라이선스 칸 클릭 =
/// 라이선스 창 · 패널마다 목록 아래 상태바(폴더 항목 수 · Git 브랜치) · 칸 클릭 = 상세 메뉴 · 끄면 목록이 그만큼 커진다.
#[test]
fn status_segments_and_tab_status_bar() {
    let (mut app, dir) = fixture("statusline");
    std::fs::create_dir_all(dir.join(".git")).unwrap();
    std::fs::write(
        dir.join(".git").join("HEAD"),
        "ref: refs/heads/feat/status\n",
    )
    .unwrap();
    let mut inv = Invalidations::default();
    app.panels[0].invalidate_dir_info();
    let _ = app.panels[0].navigate_to(dir.clone(), &mut inv);
    app.layout_for(1200, 800, 1.0);
    app.update_status();
    let ids = |app: &App| -> Vec<String> {
        app.statusbar
            .segments()
            .iter()
            .map(|s| s.id.clone())
            .collect()
    };
    assert_eq!(ids(&app), ["tab", "cpu", "mem", "io", "license"]);
    assert!(app.statusbar.segments()[0].text.contains("1/1"));
    // 부하: 첫 틱 = 메모리만 · 주기 전 재조회 없음.
    let t0 = Instant::now();
    let next = app.status_load_tick(t0).expect("wake");
    assert!(next > t0);
    assert!(app.load.is_some_and(|l| l.rss > 0));
    assert_eq!(app.status_load_tick(t0), Some(next), "주기 전 = 그대로");
    let _ = app.settings.set("statusbar.items", "license,tab");
    app.after_setting_changed("statusbar.items");
    assert_eq!(ids(&app), ["license", "tab"]);
    assert_eq!(
        app.status_load_tick(Instant::now()),
        None,
        "부하 칸 없음 = 깨우지 않음"
    );
    // 라이선스 칸 클릭 = 라이선스 창 요청.
    let mut rec = nexa_ctl::RecordCtx::with_surface(1200, 800);
    app.paint_into(&mut rec, 1200, 800, 1.0);
    let r = app.statusbar.seg_rect("license").expect("license seg");
    let (x, y) = (r.x + 4, r.y + 4);
    app.route(InputEvent::MouseDown {
        x,
        y,
        shift: false,
        primary: false,
    });
    app.route(InputEvent::MouseUp { x, y });
    assert!(app.open_license, "라이선스 창");
    // 탭 상태바: 목록 바로 아래 · 패널 바닥까지.
    let (pb, lb, sb) = (
        app.panels[0].bounds(),
        app.panels[0].rows().bounds(),
        app.panels[0].status_bounds(),
    );
    assert_eq!((sb.y, sb.bottom(), sb.h), (lb.bottom(), pb.bottom(), 22));
    let sum = app.panels[0].status_summary();
    assert!(sum[0].starts_with("folder="), "{sum:?}");
    assert_eq!(sum[1], "git=git: feat/status");
    // Git 칸 클릭 = 상세 메뉴(브랜치 복사 · 새로 고침).
    let r = app.panels[0].status_seg_rect("git").expect("git seg");
    let (x, y) = (r.x + 4, r.y + 4);
    app.route(InputEvent::MouseDown {
        x,
        y,
        shift: false,
        primary: false,
    });
    app.route(InputEvent::MouseUp { x, y });
    let menu = app.dump_of("ctx").unwrap_or_default();
    assert!(
        menu.contains("aux.git.copy") && menu.contains("aux.refresh"),
        "{menu}"
    );
    app.ctx_pick("aux.refresh");
    // 끄면 목록이 22만큼 커진다.
    let _ = app.settings.set("layout.tab_statusbar", "off");
    app.after_setting_changed("layout.tab_statusbar");
    assert_eq!(app.panels[0].rows().bounds().bottom(), pb.bottom());
    assert_eq!(app.panels[0].status_bounds().h, 0);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 자동 맞춤의 머리글 = 제목 + 정렬 삼각형 + 다중 정렬 순번(사용자 10-03): 정렬·다중 정렬을 걸면 머리글이 더 넓어지고
/// 자동 맞춤 폭도 그만큼 늘어난다(데이터가 더 길면 데이터가 이긴다).
#[test]
fn autofit_counts_sort_mark_and_order_in_header() {
    let (mut app, dir) = fixture("autofitsort");
    app.layout_for(1200, 800, 1.0);
    let fit = |app: &mut App| {
        let mut inv = Invalidations::default();
        app.autofit_column(0, 2, &mut inv);
        app.panels[0].col_widths_by_key()[2].1
    };
    // 크기 열: 시험 폴더의 값은 짧다 → 머리글이 폭을 정한다.
    let _ = app.settings.set("list.col_autofit_max", "2000");
    let plain = fit(&mut app);
    let b = app.panels[0].rows().bounds();
    let cols = app.panels[0].col_widths_by_key();
    let mid = |i: usize| b.x + cols[..i].iter().map(|c| c.1).sum::<i32>() + cols[i].1 / 2;
    let click = |app: &mut App, x: i32, shift: bool| {
        let y = b.y + 5;
        app.route(InputEvent::MouseDown {
            x,
            y,
            shift,
            primary: false,
        });
        app.route(InputEvent::MouseUp { x, y });
    };
    click(&mut app, mid(2), false);
    assert_eq!(
        app.panels[0].rows().sort_mark(cols[2].0).as_deref(),
        Some("▲")
    );
    let sorted = fit(&mut app);
    assert!(sorted > plain, "정렬 표시만큼 넓다: {sorted} vs {plain}");
    click(&mut app, mid(0), true);
    assert_eq!(
        app.panels[0].rows().sort_mark(cols[2].0).as_deref(),
        Some("▲1")
    );
    let multi = fit(&mut app);
    assert!(multi > sorted, "순번만큼 더 넓다: {multi} vs {sorted}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// 탭 패널 간 드래그(T-122 · dir2 71baf67 WINC-098/101/116): 탭을 끌어 반대 패널 위에서 놓으면 그 패널로 옮겨 가고(탭 위 = 그
/// 탭 앞 · 그 밖 = 끝) 활성 패널이 따라간다 · 끄는 동안 놓일 자리 표식 · Esc = 취소 · 마지막 탭은 옮기지 않는다.
#[test]
fn tab_drag_moves_between_panels() {
    let (mut app, dir) = fixture("tabdrag");
    app.layout_for(1200, 800, 1.0);
    let sub = dir.join("sub");
    let mut inv = Invalidations::default();
    let _ = app.panels[0].navigate_to(dir.clone(), &mut inv);
    app.command("file.new_tab");
    let _ = app.panels[0].navigate_to(sub.clone(), &mut inv);
    app.update_status();
    let paint = |app: &mut App| {
        let mut rec = nexa_ctl::RecordCtx::with_surface(1200, 800);
        app.paint_into(&mut rec, 1200, 800, 1.0);
        rec
    };
    paint(&mut app);
    assert_eq!(
        (app.panels[0].tab_count(), app.panels[1].tab_count()),
        (2, 1)
    );
    // 왼쪽 둘째 탭(sub)을 잡아 끈다.
    let r1 = app.panels[0].tab_rect(1).expect("tab 1");
    let (gx, gy) = (r1.x + 14, r1.y + r1.h / 2);
    app.route(down(gx, gy));
    app.route(InputEvent::MouseMove { x: gx + 30, y: gy });
    assert_eq!(
        app.panels[0].tab_dragging(),
        Some(1),
        "임계를 넘으면 드래그"
    );
    assert_eq!(app.tab_drop_hint, None, "같은 패널 위 = 표식 없음");
    // 오른쪽 패널 본문 위 = 표식(대상 = 오른쪽 · 끝).
    let rb = app.panels[1].bounds();
    let (dx, dy) = (rb.x + rb.w / 2, rb.y + rb.h / 2);
    app.route(InputEvent::MouseMove { x: dx, y: dy });
    assert_eq!(app.tab_drop_hint.map(|h| h.0), Some(1));
    let rec = paint(&mut app);
    let line = app.tab_drop_hint.unwrap().1;
    assert!(rec.fills.iter().any(|(r, _)| *r == line), "삽입선 {line:?}");
    // Esc = 취소(제자리).
    app.route(InputEvent::Key {
        key: nexa_ctl::Key::Escape,
        shift: false,
        primary: false,
    });
    assert_eq!(app.panels[0].tab_dragging(), None);
    assert_eq!(app.tab_drop_hint, None);
    app.route(InputEvent::MouseUp { x: dx, y: dy });
    assert_eq!(
        (app.panels[0].tab_count(), app.panels[1].tab_count()),
        (2, 1)
    );
    // 다시 끌어 오른쪽 본문에 놓는다 → 오른쪽 끝에 붙고 활성 패널 = 오른쪽.
    paint(&mut app);
    app.route(down(gx, gy));
    app.route(InputEvent::MouseMove { x: gx + 30, y: gy });
    app.route(InputEvent::MouseMove { x: dx, y: dy });
    app.route(InputEvent::MouseUp { x: dx, y: dy });
    assert_eq!(
        (app.panels[0].tab_count(), app.panels[1].tab_count()),
        (1, 2)
    );
    assert_eq!(app.active, 1);
    assert_eq!(app.panels[1].root_path(), sub, "옮겨 온 탭이 활성");
    assert_eq!(app.panels[1].active_index(), 1, "끝에 붙는다");
    assert_eq!(app.panels[0].tab_dragging(), None);
    assert_eq!(app.pressed, None);
    // 왼쪽에 하나 남은 탭은 옮기지 않는다(패널이 비지 않게).
    paint(&mut app);
    let tb = app.panels[0].tabbar_bounds();
    let (lx, ly) = (tb.x + 20, tb.y + tb.h / 2);
    app.route(down(lx, ly));
    app.route(InputEvent::MouseMove { x: lx + 30, y: ly });
    app.route(InputEvent::MouseMove { x: dx, y: dy });
    app.route(InputEvent::MouseUp { x: dx, y: dy });
    assert_eq!(
        (app.panels[0].tab_count(), app.panels[1].tab_count()),
        (1, 2)
    );
    // 오른쪽 탭을 왼쪽 탭 바의 첫 탭 위에 놓는다 = 그 탭 앞(자리 0).
    paint(&mut app);
    let rr = app.panels[1].tab_rect(1).expect("right tab 1");
    let (rx, ry) = (rr.x + 14, rr.y + rr.h / 2);
    app.route(down(rx, ry));
    app.route(InputEvent::MouseMove { x: rx - 30, y: ry });
    assert!(app.panels[1].tab_dragging().is_some());
    app.route(InputEvent::MouseMove { x: lx, y: ly });
    app.route(InputEvent::MouseUp { x: lx, y: ly });
    assert_eq!(
        (app.panels[0].tab_count(), app.panels[1].tab_count()),
        (2, 1)
    );
    assert_eq!(
        (app.active, app.panels[0].active_index()),
        (0, 0),
        "첫 탭 앞에 삽입"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 컬럼 이동(T-123 + GAP-016): 끄는 동안 놓일 자리 표식 · 놓으면 순서가 같은 패널의 모든 탭 · (동기화면) 반대 패널 ·
/// 세션에 반영 · Esc = 원래 순서.
#[test]
fn column_reorder_shows_marker_propagates_and_cancels() {
    let (mut app, dir) = fixture("coldrag");
    app.layout_for(1200, 800, 1.0);
    app.command("file.new_tab");
    app.command("tab.prev");
    let keys = |app: &App, p: usize| -> Vec<u32> {
        app.panels[p]
            .col_widths_by_key()
            .iter()
            .map(|c| c.0)
            .collect()
    };
    let orig = keys(&app, 0);
    let b = app.panels[0].rows().bounds();
    let w: Vec<i32> = app.panels[0]
        .col_widths_by_key()
        .iter()
        .map(|c| c.1)
        .collect();
    // 둘째 열(확장자) 머리를 잡아 넷째 열 너머로 끈다.
    let (x0, y) = (b.x + w[0] + w[1] / 2, b.y + 5);
    let x1 = b.x + w[0] + w[1] + w[2] + w[3] / 2 + 20;
    app.route(down(x0, y));
    app.route(InputEvent::MouseMove { x: x1, y });
    assert!(app.panels[0].col_dragging());
    let slot = app.panels[0]
        .rows()
        .col_drag_slot()
        .expect("놓일 자리 표식");
    assert_eq!(slot.h, b.h, "머리 + 본문 전체");
    // Esc = 취소(원래 순서).
    app.route(InputEvent::Key {
        key: nexa_ctl::Key::Escape,
        shift: false,
        primary: false,
    });
    assert!(!app.panels[0].col_dragging());
    app.route(InputEvent::MouseUp { x: x1, y });
    assert_eq!(keys(&app, 0), orig, "Esc = 원래 순서");
    // 다시 끌어 놓는다 → 순서 변경이 같은 패널의 다른 탭과(동기화 기본 on) 반대 패널에.
    app.route(down(x0, y));
    app.route(InputEvent::MouseMove { x: x1, y });
    app.route(InputEvent::MouseUp { x: x1, y });
    let moved = keys(&app, 0);
    assert_ne!(moved, orig);
    assert!(app.settings.flag("list.col_width_sync"));
    assert_eq!(keys(&app, 1), moved, "반대 패널도 같은 순서");
    app.command("tab.next");
    assert_eq!(keys(&app, 0), moved, "같은 패널의 다른 탭");
    // 세션: 기본과 다른 열 레이아웃이 저장된다.
    let snap = app.session_snapshot();
    assert!(
        snap.panels[0].col_layout.starts_with("cols:1["),
        "{}",
        snap.panels[0].col_layout
    );
    assert_eq!(snap.panels[0].col_layout, snap.panels[1].col_layout);
    // 동기화를 끄면 반대 패널은 그대로.
    let _ = app.settings.set("list.col_width_sync", "off");
    app.route(down(x0, y));
    app.route(InputEvent::MouseMove { x: x1, y });
    app.route(InputEvent::MouseUp { x: x1, y });
    assert_ne!(keys(&app, 0), moved);
    assert_eq!(keys(&app, 1), moved, "동기화 꺼짐 = 독립");
    let _ = std::fs::remove_dir_all(&dir);
}

/// 강제 값(T-120 · 사용자 10-03 "강제로 설정된 경우는 제약이 풀렸을 때 원래 값으로 돌아갈 수 있도록 · 사용자가 직접 설정한
/// 값은 보이지 않지만 유지"): "시스템 기본 터미널과 같게"가 켜져 있고 터미널 글꼴을 찾았으면 터미널 글꼴 카드는 그 글꼴을
/// 보여 주고 잠긴다(이유 덧줄 · [초기화] 무시) · 저장된 사용자 값은 그대로 · 끄면 사용자 값이 다시 보이고 풀린다.
#[test]
fn forced_settings_show_effective_value_and_keep_user_value() {
    let (mut app, dir) = fixture("forced");
    app.layout_for(1200, 800, 1.0);
    let _ = app.settings.set("term.font_face", "My Mono");
    // 시험이 PC 상태를 타지 않게 프로필을 비운다(Windows 러너에는 실제 Windows Terminal 프로필이 있다 — CI d49644c 적발).
    app.wt_profile = None;
    assert!(app.prefs_forced().is_empty(), "프로필 없음 = 강제 없음");
    app.wt_profile = Some(platform::WtProfile {
        faces: vec!["System Mono".into()],
        size_pt: None,
        scheme: None,
        commandline: None,
    });
    let forced = app.prefs_forced();
    assert_eq!(forced.len(), 1, "크기는 프로필이 줄 때만(Windows Terminal)");
    assert_eq!(
        (forced[0].0.as_str(), forced[0].1.as_str()),
        ("term.font_face", "System Mono")
    );
    app.startup_cmd("prefs.search:term.font_face");
    assert_eq!(
        app.prefs_win.card_value("term.font_face").as_deref(),
        Some("System Mono")
    );
    assert_eq!(app.prefs_win.is_locked("term.font_face"), Some(true));
    let why = app
        .prefs_win
        .lock_reason("term.font_face")
        .expect("이유 덧줄");
    assert!(why.contains("System Mono"), "{why}");
    assert_eq!(
        app.settings.get("term.font_face"),
        Some("My Mono"),
        "사용자 값은 보관"
    );
    // 크기까지 주는 프로필(Windows Terminal).
    app.wt_profile.as_mut().unwrap().size_pt = Some(12.0);
    assert_eq!(app.prefs_forced().len(), 2);
    // 끄면 강제가 풀리고 사용자 값이 다시 보인다.
    let _ = app.settings.set("term.follow_windows_terminal", "off");
    assert!(app.prefs_forced().is_empty());
    app.refresh_prefs();
    assert_eq!(
        app.prefs_win.card_value("term.font_face").as_deref(),
        Some("My Mono")
    );
    assert_eq!(app.prefs_win.is_locked("term.font_face"), Some(false));
    assert_eq!(app.prefs_win.lock_reason("term.font_face"), None);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 상태 열(T-126 · 사용자 10-03): 기본 열 = 이름 · 상태 · 크기 · 수정한 날짜 · 상태 칸은 아이콘 셀 · 아이콘은 색 입힌 그림 ·
/// 옛 세션의 열 폭(상태 열이 없던 때의 5개)은 열 종류에 맞춰 옮겨진다.
#[test]
fn status_column_defaults_icons_and_session_migration() {
    let (mut app, dir) = fixture("statuscol");
    app.layout_for(1200, 800, 1.0);
    let keys: Vec<u32> = app.panels[0]
        .rows()
        .columns()
        .iter()
        .map(|c| c.key)
        .collect();
    assert_eq!(
        keys,
        [
            filelist::COL_NAME,
            filelist::COL_STATUS,
            filelist::COL_SIZE,
            filelist::COL_MODIFIED
        ]
    );
    // 로컬 폴더의 평범한 파일 = 상태 없음(빈 칸 · 아이콘 없음).
    let src = app.panels[0].rows().source();
    assert_eq!(src.status_of(0), ndir_vfs::FileStatus::None);
    assert_eq!(
        nexa_grid::RowSource::cell_icon(src, 0, filelist::COL_STATUS),
        None
    );
    assert_eq!(
        nexa_grid::RowSource::cell_icon(src, 0, filelist::COL_SIZE),
        None
    );
    // 상태 → 아이콘 키 · 글(폴백) · 리졸버가 색 입힌 그림을 준다(온라인 전용 = 파랑 · 항상 유지 = 초록 원 + 흰 체크).
    use ndir_vfs::FileStatus as S;
    assert_eq!(filelist::status_icon_key(S::None), None);
    for (st, key, name) in [
        (S::CloudOnly, "status:cloud", "cloud"),
        (S::AvailableLocally, "status:local", "local"),
        (S::AlwaysKeep, "status:pinned", "pinned"),
        (S::Network, "status:network", "network"),
    ] {
        assert_eq!(filelist::status_icon_key(st), Some(key));
        assert!(!filelist::status_label(st).is_empty());
        let img = app::row_icons::resolve(key, "", 16).expect("status icon");
        assert_eq!((img.w, img.h), (16, 16));
        let (r, g, b) = app::row_icons::status_color(name).unwrap();
        let ink = img.rgba.chunks(4).filter(|p| p[3] > 128).count();
        assert!(ink >= 20, "{name}: 잉크 {ink}");
        if name != "pinned" {
            assert!(
                img.rgba.chunks(4).all(|p| (p[0], p[1], p[2]) == (r, g, b)),
                "{name}: 단색"
            );
        } else {
            assert!(
                img.rgba
                    .chunks(4)
                    .any(|p| p[3] > 128 && p[0] > 230 && p[1] > 230),
                "흰 체크"
            );
        }
    }
    assert!(app::row_icons::resolve("status:nope", "", 16).is_none());
    // 세션 이행: 옛 세션(레이아웃 없음 · 폭 5개 = 이름 · 확장자 · 크기 · 수정한 날짜 · 종류).
    let ps = crate::session::PanelSession {
        tabs: vec![dir.clone()],
        col_widths: vec![300, 55, 90, 150, 120],
        ..Default::default()
    };
    let mut p = Panel::restore(
        &ps,
        &dir,
        list_opts(&app.settings),
        panel_metrics(&app.settings, 1.0),
        columns_for(600, 1.0),
        all_columns_for(600, 1.0),
    );
    let w = p.col_widths_by_key();
    let of = |k: u32| w.iter().find(|c| c.0 == k).map(|c| c.1);
    assert_eq!(
        (
            of(filelist::COL_NAME),
            of(filelist::COL_SIZE),
            of(filelist::COL_MODIFIED)
        ),
        (Some(300), Some(90), Some(150)),
        "폭이 열 종류에 맞게: {w:?}"
    );
    // 새 열 = 기본 폭 = 머리글("Status")이 다 보이는 폭(글자 폭 + 여백) · 아이콘이 들어갈 최소 40.
    let sw = app::fonts::status_col_w();
    assert_eq!(of(filelist::COL_STATUS), Some(sw), "새 열 = 기본 폭");
    let px = app.ui_font.em_to_px(app.settings.font_px("list.font_size"));
    for label in ["Status", "상태", "状態", "Statut du fichier"] {
        let text_w = app.ui_font.measure(label, px);
        let w = app::fonts::status_col_w_for(text_w);
        assert!(
            w as f32 >= text_w + 12.0 && w >= 40,
            "{label}: {w} vs {text_w}"
        );
    }
    assert!(sw as f32 >= app.ui_font.measure(&tr("col.status"), px) + 12.0);
    // 기본 숨김 열을 다시 켜면 옛 세션의 폭이 아니라 정의의 기본 폭(숨긴 열의 폭은 보관하지 않는다).
    let mut inv = Invalidations::default();
    p.apply_col_layout(
        &[
            (filelist::COL_NAME, true),
            (filelist::COL_EXT, true),
            (filelist::COL_KIND, true),
        ],
        &mut inv,
    );
    let w2: Vec<(u32, i32)> = p.col_widths_by_key();
    assert_eq!(
        w2,
        vec![
            (filelist::COL_NAME, 300),
            (filelist::COL_EXT, 64),
            (filelist::COL_KIND, 110)
        ]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-51 B-2a 배경 셸 메뉴: 빈 영역 우클릭 = 셸 배경 항목(가짜 포트)이 상단에 · 앱 고유 항목 뒤따름 · 실행 = `invoke_bg(폴더)` ·
/// 생성 보고(`fake.bgnew`) = 재열람 + 선택 + 인라인 이름 바꾸기 · 가상 최상위에는 셸 항목 없음.
#[test]
fn background_menu_merges_shell_items_and_handles_created() {
    let (mut app, dir) = fixture("bgmenu");
    app.layout_for(1200, 800, 1.0);
    let mut inv = Invalidations::default();
    let _ = app.panels[0].navigate_to(dir.clone(), &mut inv);
    app.open_bg_menu(0);
    let d = app.dump_of("ctx").unwrap();
    assert!(
        d.starts_with("bg fake.bgopen "),
        "셸 배경 항목이 맨 앞: {d}"
    );
    assert!(
        d.contains("edit.paste") && d.contains("file.new_folder") && d.contains("view.refresh"),
        "{d}"
    );
    app.startup_cmd("ctx.pick:fake.bgopen");
    let log = app.platform.log.clone().expect("fake log");
    assert!(
        log.borrow()
            .calls
            .iter()
            .any(|c| c == "menu.invoke_bg:fake.bgopen"),
        "{:?}",
        log.borrow().calls
    );
    // 생성 보고 → 선택 + 이름 바꾸기 진입.
    app.open_bg_menu(0);
    app.startup_cmd("ctx.pick:fake.bgnew");
    assert!(dir.join("New Fake.txt").is_file());
    assert_eq!(
        app.panels[0].selected_paths(),
        vec![dir.join("New Fake.txt")]
    );
    assert!(
        app.panels[0].rows().is_renaming(),
        "생성 직후 인라인 이름 바꾸기"
    );
    // 가상 최상위 = 셸 항목 없이 자체 항목만.
    app.panels[0].rows_mut().cancel_rename(&mut inv);
    let _ = app.panels[0].navigate_to(PathBuf::from(ndir_vfs::MY_PC), &mut inv);
    app.open_bg_menu(0);
    let d = app.dump_of("ctx").unwrap();
    assert!(
        !d.contains("fake.bgopen") && d.contains("edit.paste"),
        "{d}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-51 B-2c 휴지통 undo: Delete(휴지통) → 히스토리 설명 `recycle 1 item(s)` → Ctrl+Z = 포트 `restore(원래 경로)` → Ctrl+Y = 다시 `trash`.
#[test]
fn trash_delete_is_undoable_via_restore() {
    let (mut app, dir) = fixture("trashundo");
    app.layout_for(1200, 800, 1.0);
    let mut inv = Invalidations::default();
    let _ = app.panels[0].navigate_to(dir.clone(), &mut inv);
    let row = (0..app.panels[0].rows().source().len())
        .find(|&i| app.panels[0].rows().source().row(i).text == "a.txt")
        .expect("a.txt");
    app.panels[0]
        .rows_mut()
        .select_program(row, nexa_grid::SelectOp::Single, &mut inv);
    app.delete_to_trash();
    assert_eq!(app.history.undo_description(), Some("recycle 1 item(s)"));
    // 가짜 휴지통은 파일을 실제로 옮기지 않는다 → 복원 호출을 받으려면 없는 상태로 만든다.
    std::fs::remove_file(dir.join("a.txt")).unwrap();
    app.command("edit.undo");
    let log = app.platform.log.clone().expect("fake log");
    assert!(
        log.borrow().calls.iter().any(|c| c == "trash.restore:1"),
        "{:?}",
        log.borrow().calls
    );
    assert_eq!(app.history.redo_description(), Some("recycle 1 item(s)"));
    std::fs::write(dir.join("a.txt"), b"back").unwrap();
    app.command("edit.redo");
    assert!(
        log.borrow()
            .calls
            .iter()
            .filter(|c| c.as_str() == "trash:1")
            .count()
            >= 2,
        "redo = 다시 휴지통: {:?}",
        log.borrow().calls
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 사용자 피드백 10-03: 설정 숫자(em · dir2 DIP)는 그대로, 글리프 크기는 em→높이 변환(≥ em · 맑은 고딕 12 → ≈16) · 영역별 prefs가 설정을 따른다 ·
/// 배치 지표(메뉴 높이 · 행 높이)는 em 숫자 그대로(골든 불변) · Windows/macOS/Linux 기본 고정폭 글꼴이 로드된다.
#[test]
fn font_sizes_use_em_convention_and_mono_font_loads() {
    let (mut app, dir) = fixture("fonts");
    let em = app.settings.font_px("ui.font_size");
    assert_eq!(em, 12.0, "dir2와 같은 기본 숫자");
    let px = app.font_px("ui.font_size");
    assert!(px >= em && px <= em * 1.6, "em→높이 변환: {px}");
    assert_eq!(px, app.ui_font.em_to_px(em));
    let p = app.font_prefs();
    assert_eq!(p.status.size, app.font_px("statusbar.font_size"));
    assert_eq!(p.peerlist.size, app.font_px("list.font_size"));
    let _ = app.settings.set("list.font_size", "20");
    assert_eq!(app.font_prefs().peerlist.size, app.ui_font.em_to_px(20.0));
    app.layout_for(1200, 800, 1.0);
    assert_eq!(
        app.menubar.bounds().h,
        23,
        "메뉴 높이 = em 12 + 11(배치는 변환 없음)"
    );
    assert!(app.mono_font.is_some(), "OS 기본 고정폭 글꼴");
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-70 진행 창: 복사 시작 = 창 열기 요청(`transfer.close_ms` 기본 2000) · 틱마다 항목 세그먼트/파일 n/m 갱신 · 완료 = [닫기 (N)] 카운트다운 ·
/// `transfer.close_ms=0` = 창 없음(제목줄 %만 — dir2).
#[test]
fn transfer_progress_window_updates_and_closes() {
    let (mut app, dir) = fixture("progress");
    app.layout_for(1200, 800, 1.0);
    std::fs::write(dir.join("big.bin"), vec![7u8; 300_000]).unwrap();
    app.start_transfer(
        vec![dir.join("a.txt"), dir.join("big.bin")],
        dir.join("sub"),
        ndir_ops::Op::Copy,
        false,
    );
    assert!(app.open_progress, "close_ms > 0 = 진행 창");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while app.ops_tick() {
        assert!(std::time::Instant::now() < deadline, "전송이 끝나지 않음");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let d = app.dump_of("progress").unwrap();
    assert!(
        d.contains("Done - closing shortly")
            && d.contains("file 2/2")
            && d.contains("done,done")
            && d.contains("100%"),
        "{d}"
    );
    assert!(dir.join("sub").join("big.bin").is_file());
    // 창이 없으니 닫기 모드도 창 없이 — 2000 ms 뒤 만료.
    assert!(app.progress_win.is_closing());
    assert!(app.progress_win.tick(10_000_000));
    assert!(!app.progress_win.is_closing());
    // close_ms = 0 → 진행 창 열지 않음.
    let _ = app.settings.set("transfer.close_ms", "0");
    app.open_progress = false;
    app.start_transfer(
        vec![dir.join("b.md")],
        dir.join("sub"),
        ndir_ops::Op::Copy,
        false,
    );
    assert!(!app.open_progress);
    while app.ops_tick() {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-71 일괄 이름 변경: 선택 2건 → `edit.bulk_rename` = 창 요청 + 항목 2 · 선택 없음 = 상태줄 안내 · 파이프라인 적용 → 순차 rename + undo 1건(설명 `renamed 2`) ·
/// 프리셋 저장(대화상자 → 파일) · 불러오기 · 이름 정리.
#[test]
fn bulk_rename_window_apply_undo_and_presets() {
    let (mut app, dir) = fixture("bulk");
    app.layout_for(1200, 800, 1.0);
    app.open_bulk_rename();
    assert_eq!(app.statusbar.left(), "no selection");
    assert!(!app.open_bulk);
    let mut inv = Invalidations::default();
    let row_of = |app: &App, name: &str| {
        (0..app.panels[0].rows().source().len())
            .find(|&i| app.panels[0].rows().source().row(i).text == name)
            .expect("row")
    };
    let a = row_of(&app, "a.txt");
    let b = row_of(&app, "b.md");
    app.panels[0]
        .rows_mut()
        .select_program(a, nexa_grid::SelectOp::Single, &mut inv);
    app.panels[0]
        .rows_mut()
        .select_program(b, nexa_grid::SelectOp::Toggle, &mut inv);
    app.command("edit.bulk_rename");
    assert!(app.open_bulk && app.bulk_win.items_len() == 2);
    app.bulk_win
        .load_ops(&[ndir_ops::batch_rename::RenameOp::Insert {
            scope: ndir_ops::batch_rename::Scope::Name,
            text: "x_".into(),
            at: ndir_ops::batch_rename::InsertAt {
                offset: 0,
                from_end: false,
            },
        }]);
    let d = app.dump_of("bulk").unwrap();
    assert!(
        d.contains("a.txt → x_a.txt apply") && d.contains("b.md → x_b.md apply"),
        "{d}"
    );
    let list = app.bulk_win.result();
    assert_eq!(list.len(), 2);
    app.bulk_action(crate::bulk_win::BulkAction::Rename(list));
    assert!(dir.join("x_a.txt").is_file() && dir.join("x_b.md").is_file());
    assert_eq!(app.history.undo_description(), Some("renamed 2"));
    app.command("edit.undo");
    assert!(
        dir.join("a.txt").is_file() && !dir.join("x_a.txt").exists(),
        "undo 1건"
    );
    // 프리셋 저장: 대화상자 → 이름 → 파일 · 불러오기 · 정리 규칙.
    let pdir = App::presets_dir();
    let _ = std::fs::remove_file(pdir.join("T71 test.cfg"));
    let preset_text = ndir_ops::batch_rename::serialize_ops(&app.bulk_win.ops());
    app.bulk_action(crate::bulk_win::BulkAction::SavePreset(preset_text));
    assert!(
        app.dump_of("dlg").unwrap().contains("pending"),
        "{}",
        app.dump_of("dlg").unwrap()
    );
    app.bulk_preset_saved(1, Some("T71 <test>".into()));
    assert!(pdir.join("T71 test.cfg").is_file());
    assert!(App::preset_names(&pdir).iter().any(|n| n == "T71 test"));
    assert_eq!(App::sanitize_preset_name("  a/b:c  "), Some("abc".into()));
    assert_eq!(App::sanitize_preset_name(" <> "), None);
    app.bulk_action(crate::bulk_win::BulkAction::LoadPreset("T71 test".into()));
    assert_eq!(app.bulk_win.ops().len(), 1);
    let _ = std::fs::remove_file(pdir.join("T71 test.cfg"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-71 순서 편집기: `order.open:` → 창 요청 + 덤프 · 통지 → 툴바 재구성(그룹 숨김·순서) · 컨텍스트 메뉴 항목 표시/순서 · 컬럼 레이아웃(활성 패널 + 동기 ·
/// 세션 스냅숏 · 정규화 저장).
#[test]
fn order_editor_applies_toolbar_ctxmenu_and_columns() {
    let (mut app, dir) = fixture("order");
    app.layout_for(1200, 800, 1.0);
    assert!(app.dump_of("order").unwrap().starts_with("none"));
    app.startup_cmd("order.open:toolbar.layout");
    assert!(app.open_order);
    let d = app.dump_of("order").unwrap();
    assert!(
        d.contains("key toolbar.layout")
            && d.contains("[x] Panel Controls")
            && d.contains("  [x] File Panel Toggle"),
        "{d}"
    );
    app.startup_cmd("order.open:bogus");
    // 툴바: 기본 = refresh 먼저 · view 그룹 숨김 + show 재배열 → 항목 집합/순서가 따라온다.
    let ids = |app: &App| -> Vec<String> {
        app.toolbar
            .all_items()
            .iter()
            .map(|t| t.id.clone())
            .collect()
    };
    let before = ids(&app);
    assert_eq!(before[0], "view.refresh");
    assert!(before.contains(&"view.mode_tree".to_string()));
    app.order_changed(
        "toolbar.layout",
        "show[dot,hidden]|view:0|refresh|panel[toggle:0]|settings",
    );
    let after = ids(&app);
    assert!(!after.contains(&"view.mode_tree".to_string()), "{after:?}");
    assert!(
        !after.contains(&"view.panel_toggle".to_string()),
        "{after:?}"
    );
    if platform::has_dotfile_toggle() {
        // 누락 foldersfirst는 가장 가까운 앞 형제(dot) 뒤로 보충된다(dir2 규칙).
        assert_eq!(after[0], "view.dot");
        assert_eq!(after[1], "view.folders_first");
        // 누락 casesensitive도 앞 형제(foldersfirst) 뒤로 보충된다 → hidden은 그 다음.
        assert_eq!(after[2], "view.case_sensitive");
        assert_eq!(after[3], "view.hidden");
    } else {
        // 점 파일 토글이 없는 OS: dot은 모르는 토큰으로 버려지고 foldersfirst는 앞 형제(hidden) 뒤로 보충된다.
        assert_eq!(after[0], "view.hidden");
        assert_eq!(after[1], "view.folders_first");
        assert_eq!(after[2], "view.case_sensitive");
        assert!(!after.contains(&"view.dot".to_string()));
    }
    assert_eq!(
        app.settings.get("toolbar.layout").unwrap(),
        if platform::has_dotfile_toggle() {
            "show:1[dot:1,foldersfirst:1,casesensitive:1,hidden:1]|view:0[tree:1,flat:1,tiles:1]|refresh:1[refresh:1,ontop:1]|panel:1[toggle:0,dock:1,info:1,colsync:1]|settings:1"
        } else {
            "show:1[hidden:1,foldersfirst:1,casesensitive:1]|view:0[tree:1,flat:1,tiles:1]|refresh:1[refresh:1,ontop:1]|panel:1[toggle:0,dock:1,info:1,colsync:1]|settings:1"
        },
        "정규화 저장"
    );
    // 컨텍스트 메뉴: copyName 숨김 · pasteInto 앞 · bg 그룹 숨김 = paste/undo/redo 전부 제외.
    let mut inv = Invalidations::default();
    app.panels[0]
        .rows_mut()
        .select_program(0, nexa_grid::SelectOp::Single, &mut inv);
    app.open_row_menu(0);
    let row = app.dump_of("ctx").unwrap();
    assert!(
        row.contains("ctx.copy_name")
            && row.contains("edit.delete_permanent")
            && row.contains("ctx.new["),
        "{row}"
    );
    app.tab_menu.close();
    app.order_changed("ctxmenu.layout", "row[pasteInto,copyName:0,new:0]|bg:0");
    app.open_row_menu(0);
    let row = app.dump_of("ctx").unwrap();
    assert!(
        !row.contains("ctx.copy_name")
            && !row.contains("ctx.new")
            && row.contains("ctx.paste_into"),
        "{row}"
    );
    app.tab_menu.close();
    app.open_bg_menu(0);
    let bg = app.dump_of("ctx").unwrap();
    assert!(
        !bg.contains("edit.undo") && !bg.contains("edit.paste") && bg.contains("view.refresh"),
        "{bg}"
    );
    app.tab_menu.close();
    // 컬럼: 기본 = 이름 · 상태 · 크기 · 수정한 날짜(확장자 · 종류 숨김 — 사용자 10-03) · ext 앞 · size 숨김 → 활성 패널 +
    // 동기(기본 on) 반대 패널 · 문자열 왕복 · 세션 스냅숏. 숨겨 둔 열을 켜면 정의의 기본 폭으로 나타난다.
    assert_eq!(
        app.order_value_of("list.col_layout"),
        "cols:1[name:1,status:1,size:1,modified:1,ext:0,kind:0]"
    );
    assert_eq!(
        app.session_snapshot().panels[0].col_layout,
        "",
        "기본 그대로 = 세션에 쓰지 않는다"
    );
    app.order_changed(
        "list.col_layout",
        "cols[ext,name,status:0,size:0,modified,kind]",
    );
    let keys = |app: &App, p: usize| -> Vec<u32> {
        app.panels[p]
            .rows()
            .columns()
            .iter()
            .map(|c| c.key)
            .collect()
    };
    assert_eq!(keys(&app, 0), vec![1, 0, 3, 4]);
    assert_eq!(
        keys(&app, 1),
        vec![1, 0, 3, 4],
        "col_width_sync on → 반대 패널도"
    );
    assert_eq!(
        app.panels[0].col_layout_str(),
        "cols:1[ext:1,name:1,modified:1,kind:1,status:0,size:0]"
    );
    assert_eq!(
        app.session_snapshot().panels[0].col_layout,
        "cols:1[ext:1,name:1,modified:1,kind:1,status:0,size:0]"
    );
    // 다시 켠 확장자 열 = 정의의 기본 폭(64).
    assert_eq!(app.panels[0].rows().columns()[0].width, 64);
    // 전부 숨김 = name 강제.
    app.order_changed(
        "list.col_layout",
        "cols[name:0,status:0,ext:0,size:0,modified:0,kind:0]",
    );
    assert_eq!(keys(&app, 0), vec![0]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-63 B 플러그인 페이지: 분류를 열면 플러그인당 체크박스(기본 켬) · 해제 → `plugins.disabled`(`|`) 통지 → 호스트 저장 → 창 재동기 ·
/// 밖에서 값을 비우면 다시 켬 · 다른 분류에서는 체크박스 없음 · 덤프 `prefs`에 상태.
#[test]
fn plugins_page_checkboxes_edit_disabled() {
    let (mut app, dir) = fixture("plugpage");
    app.layout_for(1200, 800, 1.0);
    app.prefs_win.set_plugins(
        vec![
            ("markdown".into(), "Markdown (markdown) — md".into(), false),
            ("archive".into(), "Archive (archive) — zip".into(), true),
        ],
        vec!["bad.wasm: oops".into()],
    );
    app.prefs_win.refresh(&app.settings);
    app.prefs_win.select_category("pref.cat.plugins");
    assert_eq!(
        app.prefs_win.plugin_states(),
        vec![
            ("markdown".to_string(), true),
            ("archive".to_string(), true)
        ]
    );
    app.prefs_win.toggle_plugin(0);
    let act = app.prefs_win.collect_changes();
    assert_eq!(
        act,
        crate::prefs_win::PrefsAction::Changed {
            key: "plugins.disabled".into(),
            value: "markdown".into()
        }
    );
    let _ = app.settings.set("plugins.disabled", "markdown");
    app.after_setting_changed("plugins.disabled");
    // 창이 없으면 after_setting_changed가 refresh를 건너뛴다 — 호스트가 열려 있을 때 하는 일을 직접.
    app.prefs_win.refresh(&app.settings);
    assert_eq!(
        app.prefs_win.plugin_states()[0],
        ("markdown".to_string(), false)
    );
    assert!(app
        .dump_of("prefs")
        .unwrap()
        .contains("plugins [(\"markdown\", false), (\"archive\", true)]"));
    // 둘 다 해제 → `markdown|archive` · 밖에서 비우면 둘 다 켬.
    app.prefs_win.toggle_plugin(1);
    assert_eq!(
        app.prefs_win.collect_changes(),
        crate::prefs_win::PrefsAction::Changed {
            key: "plugins.disabled".into(),
            value: "markdown|archive".into()
        }
    );
    let _ = app.settings.reset("plugins.disabled");
    app.after_setting_changed("plugins.disabled");
    app.prefs_win.refresh(&app.settings);
    assert!(app.prefs_win.plugin_states().iter().all(|(_, on)| *on));
    assert_eq!(
        app.prefs_win.collect_changes(),
        crate::prefs_win::PrefsAction::None
    );
    app.prefs_win.select_category("pref.cat.keys");
    assert!(
        app.prefs_win.plugin_states().is_empty(),
        "다른 분류 = 체크박스 없음"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// SHELL-008 새로 만들기 ▸: 단일 선택 행 메뉴에 서브메뉴(폴더 · 텍스트 문서 · 가짜 템플릿) · 파일 항목 = 부모 폴더 · 폴더 항목 = 자신 ·
/// 템플릿 선택 → `새 Fake Doc.fdoc`(바이트) 생성 + 이름 바꾸기 시작 · undo = 휴지통(가짜 기록) · 다중 선택 = 서브메뉴 없음.
#[test]
fn row_menu_new_submenu_creates_from_template() {
    let (mut app, dir) = fixture("shellnew");
    app.layout_for(1200, 800, 1.0);
    let mut inv = Invalidations::default();
    let row_of = |app: &App, name: &str| {
        (0..app.panels[0].rows().source().len())
            .find(|&i| app.panels[0].rows().source().row(i).text == name)
            .expect("row")
    };
    let a = row_of(&app, "a.txt");
    app.panels[0]
        .rows_mut()
        .select_program(a, nexa_grid::SelectOp::Single, &mut inv);
    app.open_row_menu(0);
    let d = app.dump_of("ctx").unwrap();
    assert!(d.contains("ctx.new[new.folder new.file new.tpl:0]"), "{d}");
    assert_eq!(
        app.ctx_new_dir.as_deref(),
        Some(dir.as_path()),
        "파일 항목 = 부모"
    );
    app.startup_cmd("ctx.pick:new.tpl:0");
    let made = dir.join("New Fake Doc.fdoc");
    assert_eq!(std::fs::read(&made).unwrap(), b"fake-template");
    assert!(app.panels[0].rows().is_renaming(), "생성 직후 이름 바꾸기");
    app.startup_cmd("ui.press:escape");
    assert_eq!(app.history.undo_description(), Some("new file"));
    app.command("edit.undo");
    assert!(app
        .platform
        .log
        .as_ref()
        .unwrap()
        .borrow()
        .calls
        .iter()
        .any(|c| c == "trash:1"));
    // 폴더 항목 = 자신 · 다중 선택 = 서브메뉴 없음.
    let s = row_of(&app, "sub");
    app.panels[0]
        .rows_mut()
        .select_program(s, nexa_grid::SelectOp::Single, &mut inv);
    app.open_row_menu(0);
    assert_eq!(app.ctx_new_dir.as_deref(), Some(dir.join("sub").as_path()));
    app.startup_cmd("ctx.pick:new.folder");
    assert!(dir.join("sub").join("New Folder").is_dir());
    app.startup_cmd("ui.press:escape");
    let b = row_of(&app, "b.md");
    let a = row_of(&app, "a.txt");
    app.panels[0]
        .rows_mut()
        .select_program(a, nexa_grid::SelectOp::Single, &mut inv);
    app.panels[0]
        .rows_mut()
        .select_program(b, nexa_grid::SelectOp::Toggle, &mut inv);
    app.open_row_menu(0);
    assert!(
        !app.dump_of("ctx").unwrap().contains("ctx.new"),
        "다중 선택 = 없음"
    );
    app.tab_menu.close();
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-63 매니저: 사용자 폴더 재지정(스레드 로컬) → 동봉 markdown.wasm 설치 = 사용자 폴더 복사 + 캐시 재구성(사용자분이 1순위 · 삭제 가능) ·
/// 잘못된 파일 = 실패 토스트 · 삭제 = 파일 제거 + 동봉분으로 복귀(삭제 불가) · 동봉분 삭제 요청 = 안내 · 설정 창 행 갱신.
#[test]
fn plugin_manager_install_and_remove() {
    let (mut app, dir) = fixture("plugmgr");
    app.layout_for(1200, 800, 1.0);
    let user = dir.join("uplugins");
    preview::set_user_plugin_dir(Some(user.clone()));
    assert_eq!(app.user_plugin_dir(), user);
    let bundled = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../plugins/markdown.wasm"
    ));
    assert!(bundled.is_file(), "{}", bundled.display());
    let before = app.plugin_rows();
    assert!(
        before
            .iter()
            .any(|(id, _, removable)| id == "markdown" && !removable),
        "{before:?}"
    );
    // 잘못된 파일.
    let bad = dir.join("bad.wasm");
    std::fs::write(&bad, b"not wasm").unwrap();
    app.plugin_install(&bad);
    assert!(!user.join("bad.wasm").exists(), "검증 실패 = 복사 안 함");
    // 설치.
    app.plugin_install(&bundled);
    assert!(user.join("markdown.wasm").is_file());
    let rows = app.plugin_rows();
    let md = rows
        .iter()
        .find(|(id, _, _)| id == "markdown")
        .expect("markdown");
    assert!(md.2, "사용자 설치분 = 삭제 가능: {rows:?}");
    assert_eq!(
        rows.iter().filter(|(id, _, _)| id == "markdown").count(),
        1,
        "같은 id는 1개"
    );
    app.prefs_win.refresh(&app.settings);
    app.prefs_win.select_category("pref.cat.plugins");
    assert!(app
        .prefs_win
        .plugin_states()
        .iter()
        .any(|(id, _)| id == "markdown"));
    // 삭제 → 동봉분 복귀.
    app.plugin_remove("markdown");
    assert!(!user.join("markdown.wasm").exists());
    let rows = app.plugin_rows();
    assert!(
        rows.iter()
            .any(|(id, _, removable)| id == "markdown" && !removable),
        "{rows:?}"
    );
    // 동봉분 삭제 요청 = 안내(파일 그대로).
    app.plugin_remove("markdown");
    assert!(bundled.is_file());
    preview::set_user_plugin_dir(None);
    let _ = std::fs::remove_dir_all(&dir);
}

/// SHELL-044 잘라내기 흐림: Ctrl+X = 그 행 is_ghosted · Ctrl+C = 해제 · 클립보드 비움 + 동기 = 해제 · 다른 탭에도 적용.
#[test]
fn cut_marks_ghost_rows_until_clipboard_changes() {
    let (mut app, dir) = fixture("cutmarks");
    app.layout_for(1200, 800, 1.0);
    let mut inv = Invalidations::default();
    let row_of = |app: &App, name: &str| {
        (0..app.panels[0].rows().source().len())
            .find(|&i| app.panels[0].rows().source().row(i).text == name)
            .expect("row")
    };
    let a = row_of(&app, "a.txt");
    app.panels[0]
        .rows_mut()
        .select_program(a, nexa_grid::SelectOp::Single, &mut inv);
    app.command("edit.cut");
    assert!(
        app.panels[0].rows().source().is_ghosted(a),
        "잘라내기 = 흐림"
    );
    let b = row_of(&app, "b.md");
    assert!(!app.panels[0].rows().source().is_ghosted(b));
    // 같은 폴더를 보는 다른 패널에도.
    assert!(app.panels[1]
        .rows()
        .source()
        .is_ghosted(row_of(&app, "a.txt")));
    app.command("edit.copy");
    assert!(
        !app.panels[0].rows().source().is_ghosted(a),
        "복사 = 흐림 해제"
    );
    app.command("edit.cut");
    assert!(app.panels[0].rows().source().is_ghosted(a));
    // OS 클립보드(가짜)가 1순위 — 둘 다 비워야 해제된다.
    app.clip = None;
    app.platform
        .log
        .as_ref()
        .expect("fake")
        .borrow_mut()
        .clipboard = None;
    app.sync_cut_marks();
    assert!(
        !app.panels[0].rows().source().is_ghosted(a),
        "클립보드 비움 + 동기 = 해제"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// DnD 1차(SHELL-060~062 · 068): 폴더 행 위에 놓기 = 그 폴더로 **이동**(같은 볼륨 기본) · Ctrl = 복사 · 빈 본문 = 패널 폴더 ·
/// 자기/하위 거부 · 전송 중 거부 · winit 파일별 DroppedFile 모아서 틱 처리.
#[test]
fn external_drop_moves_or_copies_into_folder_under_cursor() {
    let (mut app, dir) = fixture("dnd");
    app.layout_for(1200, 800, 1.0);
    let wait = |app: &mut App| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while app.ops_tick() {
            assert!(
                std::time::Instant::now() < deadline,
                "transfer did not finish"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    };
    let row_of = |app: &App, name: &str| {
        (0..app.panels[0].rows().source().len())
            .find(|&i| app.panels[0].rows().source().row(i).text == name)
            .expect("row")
    };
    // 행 "sub"의 화면 점(본문을 위에서 훑는다).
    let point_of = |app: &App, name: &str| {
        let r = row_of(app, name);
        let b = app.panels[0].rows().bounds();
        (b.y..b.bottom())
            .map(|y| Point { x: b.x + 40, y })
            .find(|p| app.panels[0].rows().row_at(p.x, p.y) == Some(r))
            .expect("point")
    };
    let p_sub = point_of(&app, "sub");
    assert_eq!(
        app.drop_dest_at(p_sub).map(|(_, d)| d),
        Some(dir.join("sub"))
    );
    let p_file = point_of(&app, "b.md");
    assert_eq!(
        app.drop_dest_at(p_file).map(|(_, d)| d),
        Some(dir.clone()),
        "파일 행 = 패널 폴더"
    );
    assert!(app.drop_dest_at(Point { x: -5, y: -5 }).is_none());
    // 거부: 자기 자신 · 전송 없음.
    assert!(app.external_drop(vec![dir.join("sub")], p_sub).is_none());
    assert!(app.transfer.is_none());
    // 기본 = 이동.
    let r = app.external_drop(vec![dir.join("a.txt")], p_sub);
    assert_eq!(r, Some((dir.join("sub"), ndir_ops::Op::Move)));
    wait(&mut app);
    assert!(dir.join("sub").join("a.txt").is_file() && !dir.join("a.txt").exists());
    // Ctrl = 복사 · winit 이벤트 경로(dnd_dropped → 틱 flush · 커서 = 폴더 행).
    app.primary = true;
    app.cursor = (p_sub.x, p_sub.y);
    app.dnd_hover(dir.join("b.md"));
    app.dnd_dropped(dir.join("b.md"));
    assert!(app.dnd_flush());
    assert!(!app.dnd_flush(), "두 번 처리하지 않는다");
    wait(&mut app);
    assert!(
        dir.join("sub").join("b.md").is_file() && dir.join("b.md").is_file(),
        "복사 = 원본 유지"
    );
    app.primary = false;
    let _ = std::fs::remove_dir_all(&dir);
}

/// dir2 X-12 글꼴 장식(KEY-061~063 · UIC-313): `list.header_italic/header_bold/folder_bold` → 패널 그리드 `set_font_decor` →
/// 헤더는 `select_font_styled(PeerList, hdr_bold, hdr_italic)` · 폴더 행(`sub`)은 굵게(nexa-ui 113차 `RecordCtx.fonts`).
#[test]
fn font_decor_settings_reach_grid_font_selection() {
    let (mut app, dir) = fixture("decor");
    app.layout_for(1200, 800, 1.0);
    let paint = |app: &mut App| {
        let mut rec = nexa_ctl::RecordCtx::with_surface(1200, 800);
        app.paint_into(&mut rec, 1200, 800, 1.0);
        rec.fonts
    };
    let fonts = paint(&mut app);
    assert!(
        fonts
            .iter()
            .all(|f| f.0 != nexa_ctl::FontSlot::PeerList || (!f.1 && !f.2)),
        "기본 = 장식 없음: {fonts:?}"
    );
    let _ = app.settings.set("list.header_italic", "on");
    app.after_setting_changed("list.header_italic");
    let fonts = paint(&mut app);
    assert!(
        fonts.contains(&(nexa_ctl::FontSlot::PeerList, false, true)),
        "헤더 이탤릭: {fonts:?}"
    );
    let _ = app.settings.set("list.header_bold", "on");
    let _ = app.settings.set("list.folder_bold", "on");
    app.after_setting_changed("list.folder_bold");
    let fonts = paint(&mut app);
    assert!(
        fonts.contains(&(nexa_ctl::FontSlot::PeerList, true, true)),
        "헤더 굵게+이탤릭: {fonts:?}"
    );
    assert!(
        fonts.contains(&(nexa_ctl::FontSlot::PeerList, true, false)),
        "폴더 행 굵게(sub): {fonts:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// UIC-310 클립 스택(nexa-ui 112차 · T-31): 패널 그리드는 자기 경계를 `push_clip`하고 짝 맞춰 `pop_clip`한다 —
/// 백엔드가 교차·복원 규칙대로 쌓으면 그리드 안 호출은 전부 패널 경계 안으로 잘린다(실제 픽셀은 nexa-ctl `clip_tests`).
#[test]
fn panel_grid_pushes_its_bounds_as_clip() {
    #[derive(Default)]
    struct ClipProbe {
        stack: Vec<Rect>,
        pushed: Vec<Rect>,
        pops: usize,
        max_depth: usize,
    }
    impl nexa_ctl::DrawCtx for ClipProbe {
        fn fill_rect(&mut self, _r: Rect, _c: nexa_ctl::Color) {}
        fn text_opaque(
            &mut self,
            _x: i32,
            _y: i32,
            _clip: Rect,
            _t: &str,
            _f: nexa_ctl::Color,
            _b: nexa_ctl::Color,
        ) {
        }
        fn text(&mut self, _x: i32, _y: i32, _clip: Rect, _t: &str, _f: nexa_ctl::Color) {}
        fn text_width(&mut self, text: &str) -> i32 {
            text.chars().count() as i32 * 7
        }
        fn push_clip(&mut self, rect: Rect) {
            let top = self.stack.last().map_or(rect, |t| rect.intersection(t));
            self.stack.push(top);
            self.pushed.push(rect);
            self.max_depth = self.max_depth.max(self.stack.len());
        }
        fn pop_clip(&mut self) {
            assert!(self.stack.pop().is_some(), "pop_clip without push");
            self.pops += 1;
        }
    }
    let (mut app, dir) = fixture("clip");
    app.layout_for(1200, 800, 1.0);
    let mut probe = ClipProbe::default();
    app.paint_into(&mut probe, 1200, 800, 1.0);
    assert_eq!(probe.pushed.len(), probe.pops, "push/pop 짝");
    assert!(probe.stack.is_empty() && probe.max_depth >= 1);
    for p in 0..2 {
        let b = app.panels[p].rows().bounds();
        assert!(
            probe.pushed.contains(&b),
            "panel {p} rows bounds {b:?} not pushed: {:?}",
            probe.pushed
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// 우클릭 메뉴 아이콘 칸(사용자 10-03 "아이콘 표시 공간이 없다" · SHELL-011): 셸 항목이 아이콘을 주면 메뉴 항목에 실리고
/// (nexa-ctl 메뉴는 하나라도 있으면 **전 행**에 아이콘 칸을 예약한다) 메뉴 폭이 그만큼 넓어진다.
#[test]
fn shell_item_icons_reach_context_menu() {
    let (mut app, dir) = fixture("ctxicon");
    app.layout_for(1200, 800, 1.0);
    let mut inv = Invalidations::default();
    app.panels[0].select_path(&dir.join("a.txt"), &mut inv);
    app.cursor = (300, 300);
    app.open_row_menu(0);
    assert!(app.tab_menu.is_open());
    let with_icon = app
        .ctx_items
        .iter()
        .any(|it| matches!(it, CtxItem::Item { id, icon: Some(_), .. } if id == "fake.open"));
    assert!(with_icon, "가짜 셸 항목의 아이콘이 메뉴 항목에 실린다");
    let w_icons = app.tab_menu.bounds().w;
    // 같은 항목에서 아이콘만 뺀 메뉴보다 넓다(아이콘 칸 예약).
    let plain: Vec<CtxItem> = app
        .ctx_items
        .iter()
        .cloned()
        .map(|it| it.with_icon(None))
        .collect();
    let host = Rect::new(0, 0, 1200, 800);
    app.tab_menu.open_at(300, 300, plain, host, 240);
    assert!(
        w_icons > app.tab_menu.bounds().w,
        "아이콘 칸만큼 넓다: {w_icons} vs {}",
        app.tab_menu.bounds().w
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 우클릭 가속(사용자 10-03 · dir2 X-61 · SHELL-014/015): 셸 항목이 준비돼 있으면 즉시 · 아니면 UI를 막지 않고 기다렸다가
/// **완성된 메뉴를 한 번** 연다(두 번 뜨지 않는다 · 상태줄에 진행 표시 · 3 s 초과 = 자체 항목만 · 다른 입력 = 취소) · 선택이 300 ms 머물면 선행 구축을 건다 · 비동기 실행 결과는 틱에서 반영.
#[test]
fn context_menu_opens_once_when_shell_items_are_ready() {
    use crate::platform::{
        ContextMenuProvider, MenuEvent, MenuTarget, PlatformError, ShellMenuItem,
    };
    use std::cell::RefCell;
    use std::collections::VecDeque;
    #[derive(Default)]
    struct Shared {
        asked: Vec<MenuTarget>,
        prepared: Vec<MenuTarget>,
        invoked: Vec<String>,
        events: VecDeque<MenuEvent>,
    }
    struct SlowMenu(Rc<RefCell<Shared>>);
    impl ContextMenuProvider for SlowMenu {
        fn items(&self, _p: &[PathBuf]) -> Result<Vec<ShellMenuItem>, PlatformError> {
            panic!("UI 경로는 동기 items를 부르지 않는다");
        }
        fn invoke(&self, _id: &str, _p: &[PathBuf]) -> Result<(), PlatformError> {
            panic!("UI 경로는 동기 invoke를 부르지 않는다");
        }
        fn prepare(&self, t: &MenuTarget) {
            self.0.borrow_mut().prepared.push(t.clone());
        }
        fn try_items(&self, t: &MenuTarget) -> Option<Vec<ShellMenuItem>> {
            self.0.borrow_mut().asked.push(t.clone());
            None
        }
        fn invoke_async(&self, id: &str, t: &MenuTarget) -> bool {
            let mut s = self.0.borrow_mut();
            s.invoked.push(id.to_string());
            s.events.push_back(MenuEvent::Invoked {
                target: t.clone(),
                result: Err("boom".into()),
            });
            true
        }
        fn poll(&self) -> Option<MenuEvent> {
            self.0.borrow_mut().events.pop_front()
        }
        fn busy(&self) -> bool {
            !self.0.borrow().events.is_empty()
        }
    }
    let (mut app, dir) = fixture("ctxfast");
    app.layout_for(1200, 800, 1.0);
    let shared = Rc::new(RefCell::new(Shared::default()));
    app.platform.ctxmenu = Box::new(SlowMenu(shared.clone()));
    let mut inv = Invalidations::default();
    let a = dir.join("a.txt");
    app.panels[0].select_path(&a, &mut inv);
    let target = MenuTarget::Rows(vec![a.clone()]);
    // ① 선행 구축: 머무름 전에는 안 걸고(다음 깨움 = 기한) 300 ms 뒤 한 번만 건다.
    let t0 = Instant::now();
    let wake = app.ctx_shell_tick(t0);
    assert!(wake.is_some_and(|w| w > t0), "머무름 기한에 깨어난다");
    assert!(shared.borrow().prepared.is_empty());
    let t1 = t0 + Duration::from_millis(301);
    assert_eq!(app.ctx_shell_tick(t1), None);
    assert_eq!(shared.borrow().prepared, vec![target.clone()]);
    let _ = app.ctx_shell_tick(t1 + Duration::from_millis(50));
    assert_eq!(
        shared.borrow().prepared.len(),
        1,
        "같은 선택은 다시 걸지 않는다"
    );
    // ② 우클릭: 셸 항목이 아직 없으면 메뉴를 **열지 않고** 기다린다(상태줄에 진행 · 메뉴가 두 번 뜨지 않게).
    app.cursor = (320, 240);
    app.open_row_menu(0);
    assert!(!app.tab_menu.is_open(), "준비 전에는 메뉴 없음");
    assert!(app.ctx_wait.is_some());
    assert_eq!(app.statusbar.left(), tr("ctx.loading"));
    assert_eq!(shared.borrow().asked, vec![target.clone()]);
    assert!(app.ctx_shell_tick(t1).is_some(), "기다리는 동안 틱 유지");
    // ③ 다른 대상의 통지는 무시 · 같은 대상의 통지 = 완성된 메뉴가 **한 번** 연 자리에 열린다.
    let shell_items = vec![ShellMenuItem {
        id: "shell:7".into(),
        label: "Open With Fake".into(),
        enabled: true,
        ..Default::default()
    }];
    shared.borrow_mut().events.push_back(MenuEvent::Items {
        target: MenuTarget::Rows(vec![dir.join("b.md")]),
        items: shell_items.clone(),
    });
    let _ = app.ctx_shell_tick(t1);
    assert!(!app.tab_menu.is_open() && app.ctx_wait.is_some());
    shared.borrow_mut().events.push_back(MenuEvent::Items {
        target,
        items: shell_items,
    });
    let _ = app.ctx_shell_tick(t1);
    assert!(app.tab_menu.is_open() && app.ctx_wait.is_none());
    let d = app.dump_of("ctx").unwrap();
    assert!(
        d.starts_with("row ") && d.contains("shell:7") && !d.contains("ctx.loading"),
        "{d}"
    );
    assert!(d.contains("edit.copy") && d.contains("edit.delete"), "{d}");
    assert_ne!(app.statusbar.left(), tr("ctx.loading"), "상태줄 복구");
    let b = app.tab_menu.bounds();
    assert!(
        (b.x - 320).abs() <= 2 && (b.y - 240).abs() <= 2,
        "연 자리: {b:?}"
    );
    // ④ 셸 항목 실행 = 비동기 접수 → 결과(오류)는 틱에서 토스트.
    app.startup_cmd("ctx.pick:shell:7");
    assert_eq!(shared.borrow().invoked, vec!["shell:7".to_string()]);
    assert!(!app.toasts.animating());
    let _ = app.ctx_shell_tick(t1);
    assert!(app.toasts.animating(), "실행 오류 = 토스트");
    // ⑤ 메뉴가 열려 있는 동안에는 선행 구축을 걸지 않고, 닫힌 뒤 선택이 바뀌면 다시 머무름부터.
    app.panels[0].select_path(&dir.join("b.md"), &mut inv);
    let t2 = t1 + Duration::from_secs(1);
    assert!(app.ctx_shell_tick(t2).is_some());
    let _ = app.ctx_shell_tick(t2 + Duration::from_millis(301));
    assert_eq!(shared.borrow().prepared.len(), 2);
    // ⑥ 기다리는 동안 다른 클릭 = 취소(메뉴는 뜨지 않는다) · 3 s 넘게 안 오면 자체 항목만으로 연다.
    app.tab_menu.close();
    app.set_active(0);
    app.open_bg_menu(0);
    assert!(app.ctx_wait.is_some() && !app.tab_menu.is_open());
    let r0 = app.panels[0].rows().bounds();
    app.route(down(r0.x + 20, r0.bottom() - 6));
    assert!(
        app.ctx_wait.is_none() && !app.tab_menu.is_open(),
        "클릭 = 취소"
    );
    app.route(InputEvent::MouseUp {
        x: r0.x + 20,
        y: r0.bottom() - 6,
    });
    app.open_bg_menu(0);
    let t4 = Instant::now();
    let _ = app.ctx_shell_tick(t4 + Duration::from_millis(2900));
    assert!(!app.tab_menu.is_open(), "3 s 전에는 계속 기다린다");
    let _ = app.ctx_shell_tick(t4 + Duration::from_millis(3100));
    assert!(
        app.tab_menu.is_open() && app.ctx_wait.is_none(),
        "시간 초과 = 자체 항목만"
    );
    let d = app.dump_of("ctx").unwrap();
    assert!(
        d.starts_with("bg ") && !d.contains("ctx.loading") && d.contains("view.refresh"),
        "{d}"
    );
    app.tab_menu.close();
    // ⑦ 하네스 `ctx.wait`: 기다릴 것이 없으면 지나가고, 대기 중이면 나머지 `@ready` 명령을 보류했다가 메뉴가 뜬 뒤 이어 돈다.
    let tabs0 = app.panels[0].tab_count();
    app.startup_ready = vec!["ctx.wait".into(), "file.new_tab".into()];
    app.ready_fired = false;
    app.fire_ready();
    assert_eq!(
        app.panels[0].tab_count(),
        tabs0 + 1,
        "대기 없음 = 바로 실행"
    );
    app.set_active(0);
    app.open_bg_menu(0);
    let (_, waiting, _) = app.ctx_wait.clone().expect("waiting");
    app.startup_ready = vec!["ctx.wait".into(), "file.new_tab".into()];
    app.ready_fired = false;
    app.fire_ready();
    assert_eq!(app.panels[0].tab_count(), tabs0 + 1, "대기 중 = 보류");
    assert_eq!(app.startup_blocked.len(), 2);
    let t3 = Instant::now();
    assert!(app.ctx_shell_tick(t3).is_some(), "보류 중에는 틱 유지");
    shared.borrow_mut().events.push_back(MenuEvent::Items {
        target: waiting,
        items: Vec::new(),
    });
    let _ = app.ctx_shell_tick(t3);
    assert!(app.startup_blocked.is_empty());
    assert_eq!(
        app.panels[0].tab_count(),
        tabs0 + 2,
        "메뉴가 뜬 뒤 이어서 실행"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 두부(□) 방지(사용자 10-03 "확장/축소 쉐브론이 깨짐"): 네비 4 + 쉐브론 2 글리프는 **이 OS의 UI 글꼴 체인이 가진 집합**에서 고른다 —
/// 아이콘 글꼴(Segoe MDL2/Fluent · nexa-font 115차)이 있으면 dir2와 같은 PUA 글리프, 없으면 유니코드 대체. 어느 쪽이든 전부 그려진다.
/// (전역 선택 상태는 건드리지 않는다 — 병렬 시험의 네비 글리프 기대와 독립.)
#[test]
fn icon_glyph_choice_never_yields_tofu() {
    let ui = nexa_font::ui_font(None).expect("OS UI font");
    let set = if app::fonts::icon_font_covers(&ui.font) {
        app::fonts::MDL2_GLYPHS
    } else {
        app::fonts::FALLBACK_GLYPHS
    };
    let missing: String = set.iter().filter(|&&c| !ui.font.covers(c)).collect();
    assert!(
        missing.is_empty(),
        "tofu: {missing:?} · chain {:?}",
        ui.chain
    );
    // Windows는 Segoe MDL2 Assets/Fluent Icons가 체인에 있어야 dir2와 같은 모양(없는 러너에서는 유니코드 대체 — 실패 아님).
    eprintln!(
        "icon font = {} · chain {:?}",
        app::fonts::icon_font_covers(&ui.font),
        ui.chain
    );
}

/// 무간섭 재열람(dir2 PANEL-036 · GAP-005): 폴더가 밖에서 바뀌어 다시 열려도 **선택 · 펼친 폴더 · 정렬 키**가 그대로다 —
/// 사라진 항목만 선택에서 빠진다(종전 = 선택이 전부 풀려 "메뉴가 채워진 뒤 복사가 무반응"이던 원인).
#[test]
fn reopen_keeps_selection_expansion_and_sort() {
    let (mut app, dir) = fixture("reopenkeep");
    app.layout_for(1200, 800, 1.0);
    std::fs::write(dir.join("sub").join("inner.txt"), b"x").unwrap();
    let mut inv = Invalidations::default();
    app.panels[0].reopen(&mut inv);
    // 정렬 = 이름 내림차순 · sub 펼침 · a.txt + sub/inner.txt 선택.
    assert!(app.panels[0].rows_mut().source_mut().set_sort(&[(0, true)]));
    let rows = app.panels[0].rows_mut();
    let sub = (0..rows.source().len())
        .find(|&i| {
            rows.source()
                .row_path(i)
                .is_some_and(|p| p.ends_with("sub"))
        })
        .expect("sub row");
    assert!(rows.source_mut().toggle(sub), "sub 펼침");
    let idx = |app: &App, name: &str| {
        let s = app.panels[0].rows().source();
        (0..s.len()).find(|&i| s.row_path(i).is_some_and(|p| p.ends_with(name)))
    };
    let (ia, ii) = (idx(&app, "a.txt").unwrap(), idx(&app, "inner.txt").unwrap());
    let src = app.panels[0].rows_mut().source_mut();
    src.select(ia, nexa_grid::SelectOp::Single);
    src.select(ii, nexa_grid::SelectOp::Toggle);
    let order_before: Vec<_> = {
        let s = app.panels[0].rows().source();
        (0..s.len()).filter_map(|i| s.row_path(i)).collect()
    };
    let mut sel_before = app.panels[0].selected_paths();
    sel_before.sort();
    assert_eq!(sel_before.len(), 2);
    // 밖에서 파일이 하나 생기고 → 재열람.
    std::fs::write(dir.join("zz-new.txt"), b"x").unwrap();
    app.panels[0].reopen(&mut inv);
    let mut sel_after = app.panels[0].selected_paths();
    sel_after.sort();
    assert_eq!(sel_after, sel_before, "선택 유지");
    assert!(
        idx(&app, "inner.txt").is_some(),
        "펼침 유지(sub 안 항목이 보인다)"
    );
    let order_after: Vec<_> = {
        let s = app.panels[0].rows().source();
        (0..s.len()).filter_map(|i| s.row_path(i)).collect()
    };
    let without_new: Vec<_> = order_after
        .iter()
        .filter(|p| !p.ends_with("zz-new.txt"))
        .cloned()
        .collect();
    assert_eq!(without_new, order_before, "정렬(이름 내림차순) 유지");
    // 선택된 파일이 사라지면 그 항목만 빠진다.
    std::fs::remove_file(dir.join("a.txt")).unwrap();
    app.panels[0].reopen(&mut inv);
    let sel = app.panels[0].selected_paths();
    assert_eq!(sel.len(), 1);
    assert!(sel[0].ends_with("inner.txt"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// 행 아이콘 키(dir2 PANEL-064 · GAP-003): 폴더 = `dir` · 확장자 = 그 확장자 · 확장자 없음 = `file` · 파일별(exe 등) = 소문자 전체 경로 —
/// 그리드는 이름 앞에 아이콘 칸을 두고 호스트 리졸버(`app::row_icons`)에 묻는다.
#[test]
fn rows_expose_dir2_icon_keys() {
    use crate::filelist::icon_key;
    use std::path::Path;
    assert_eq!(icon_key(true, Path::new("C:/x/sub"), false), "dir");
    assert_eq!(icon_key(false, Path::new("C:/x/a.TXT"), false), "txt");
    assert_eq!(icon_key(false, Path::new("C:/x/README"), false), "file");
    assert_eq!(
        icon_key(false, Path::new("C:/X/App.EXE"), false),
        "c:/x/app.exe"
    );
    assert_eq!(
        icon_key(true, Path::new("D:/"), true),
        "d:/",
        "가상 최상위 항목 = 경로별"
    );
    let (app, dir) = fixture("iconkeys");
    let s = app.panels[0].rows().source();
    let keys: Vec<String> = (0..s.len())
        .filter_map(|i| nexa_grid::RowSource::icon(s, i))
        .map(|(k, _)| k)
        .collect();
    assert!(
        keys.contains(&"dir".to_string()) && keys.contains(&"txt".to_string()),
        "{keys:?}"
    );
    assert_eq!(keys.len(), s.len(), "모든 행이 아이콘 키를 준다");
    let _ = std::fs::remove_dir_all(&dir);
}

/// 상단 툴바 = 그룹 도크(사용자 10-03): 그룹 손잡이를 끌면 순서가 바뀌고 배치가 설정에 저장돼 재구성 뒤에도 유지 · Esc = 취소 ·
/// 아이콘 크기/간격 설정은 즉시 반영(높이·아이콘 칸 · 아래 영역이 따라 내려간다) · 그룹 안 아이콘은 기본 간격 0(붙임).
#[test]
fn toolbar_groups_move_by_drag_and_size_gap_settings_apply_live() {
    let (mut app, dir) = fixture("tooldock");
    app.layout_for(1200, 800, 1.0);
    let ids = |app: &App| -> Vec<String> {
        app.toolbar
            .groups()
            .into_iter()
            .map(|(id, _, _)| id)
            .collect()
    };
    assert_eq!(ids(&app), ["refresh", "panel", "view", "show", "settings"]);
    // 그룹 안 아이콘 = 붙임(기본 toolbar.item_gap 0).
    let (a, b) = (
        app.toolbar.item_rect("view.mode_tree").unwrap(),
        app.toolbar.item_rect("view.mode_flat").unwrap(),
    );
    assert_eq!(a.right(), b.x, "칸 사이 추가 간격 0(item_gap)");
    assert_eq!(a.w, 22, "아이콘 20 + 둘레 여백 1×2 → 아이콘 사이 2px");
    assert_eq!(app.toolbar.bounds().h, 30, "툴바 높이 = 20 + (1 + 4)×2");
    // ① 그룹 이동: `view` 그룹의 손잡이(그룹 툴바 왼쪽 10px)를 잡아 맨 앞으로 끈다.
    let vb = app.toolbar.bar("view").unwrap().bounds();
    let (gx, gy) = (vb.x - 5, vb.y + vb.h / 2);
    let first = app.toolbar.bar("refresh").unwrap().bounds();
    app.route(InputEvent::MouseMove { x: gx, y: gy });
    app.route(down(gx, gy));
    assert!(!app.toolbar.is_dragging() || app.toolbar.is_dragging());
    for x in [gx - 10, gx - 60, first.x - 8] {
        app.route(InputEvent::MouseMove { x, y: gy });
    }
    assert!(app.toolbar.is_dragging(), "끄는 중");
    app.route(InputEvent::MouseUp {
        x: first.x - 8,
        y: gy,
    });
    assert_eq!(ids(&app)[0], "view", "맨 앞으로 이동: {:?}", ids(&app));
    let saved = app
        .settings
        .get("toolbar.dock_layout")
        .unwrap_or("")
        .to_string();
    assert!(saved.starts_with("view"), "배치 저장: {saved}");
    // 재구성(언어·배율 변경 경로)에도 유지.
    app.rebuild_toolbar();
    assert_eq!(ids(&app)[0], "view");
    // ② Esc = 취소: 다시 끌다가 Esc → 순서 그대로 · 설정 그대로.
    app.layout_for(1200, 800, 1.0);
    let sb = app.toolbar.bar("settings").unwrap().bounds();
    let (sx, sy) = (sb.x - 5, sb.y + sb.h / 2);
    let before = ids(&app);
    app.route(InputEvent::MouseMove { x: sx, y: sy });
    app.route(down(sx, sy));
    app.route(InputEvent::MouseMove { x: sx - 200, y: sy });
    assert!(app.toolbar.is_dragging());
    app.route(InputEvent::Key {
        key: nexa_ctl::Key::Escape,
        shift: false,
        primary: false,
    });
    assert!(!app.toolbar.is_dragging());
    assert_eq!(ids(&app), before, "Esc = 원래 순서");
    assert_eq!(app.settings.get("toolbar.dock_layout").unwrap_or(""), saved);
    app.route(InputEvent::MouseUp { x: sx - 200, y: sy });
    // ③ 크기 설정 즉시 반영: 32 → 아이콘 칸·툴바 높이가 커지고 아래(패널)가 따라 내려간다 · 20으로 되돌리면 원래대로.
    let (h20, panel_y20) = (app.toolbar.bounds().h, app.panels[0].bounds().y);
    let _ = app.settings.set("toolbar.icon_size", "32");
    app.after_setting_changed("toolbar.icon_size");
    assert_eq!(app.toolbar.item_rect("view.mode_tree").unwrap().w, 34);
    assert_eq!(app.toolbar.bounds().h, h20 + 12);
    assert_eq!(app.panels[0].bounds().y, panel_y20 + 12);
    assert_eq!(ids(&app)[0], "view", "크기 변경에도 배치 유지");
    let _ = app.settings.set("toolbar.icon_size", "20");
    app.after_setting_changed("toolbar.icon_size");
    assert_eq!(app.toolbar.bounds().h, h20);
    // ④ 간격 설정 즉시 반영: 아이콘 사이 6 · 그룹 사이 0.
    let _ = app.settings.set("toolbar.item_gap", "6");
    app.after_setting_changed("toolbar.item_gap");
    let (a, b) = (
        app.toolbar.item_rect("view.mode_tree").unwrap(),
        app.toolbar.item_rect("view.mode_flat").unwrap(),
    );
    assert_eq!(b.x - a.right(), 6);
    let gap_of = |app: &App| {
        let g = ids(app);
        let (l, r) = (
            app.toolbar.bar(&g[0]).unwrap().bounds(),
            app.toolbar.bar(&g[1]).unwrap().bounds(),
        );
        r.x - l.right()
    };
    let g4 = gap_of(&app);
    let _ = app.settings.set("toolbar.group_gap", "0");
    app.after_setting_changed("toolbar.group_gap");
    assert_eq!(gap_of(&app), g4 - 4, "그룹 간격 4 → 0");
    // 체크 상태는 재구성 뒤에도 맞는다.
    assert_eq!(
        app.toolbar.item_checked("view.hidden"),
        app.settings.flag("list.show_hidden")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// 도크 정보: 내 PC(가상 최상위)에서는 내부 표식 `::PC::`가 아니라 표시명을 보인다.
#[test]
fn dock_info_shows_display_name_for_virtual_root() {
    let lines = crate::dockinfo::info_lines(&[], std::path::Path::new(ndir_vfs::MY_PC));
    assert_eq!(lines.len(), 1);
    assert!(!lines[0].contains("::PC::"), "{lines:?}");
    assert!(lines[0].contains(&ndir_i18n::tr("nav.mypc")), "{lines:?}");
}

/// 퀵 런처 바 크기/간격 설정(사용자 10-03 "퀵 런처 바도 설정으로"): 기본 = 상단 툴바와 같은 20(바 28 · dir2는 16) · `launcher.icon_size` 32 = 바 40 +
/// 아래 영역이 따라 내려감 · `launcher.item_gap` 0 = 아이콘 칸이 맞닿음 — 전부 즉시 반영.
#[test]
fn launcher_bar_size_and_gap_settings_apply_live() {
    let (mut app, dir) = fixture("launchsize");
    let _ = app.settings.set("launcher.visible", "on");
    let _ = app
        .settings
        .set("launcher.items", "A|ndir-no-such-a|;;B|ndir-no-such-b|");
    app.after_setting_changed("launcher.items");
    app.layout_for(1200, 800, 1.0);
    assert_eq!(app.launcherbar.bounds().h, 28, "기본 = 아이콘 20 + 여백");
    let panel_y = app.panels[0].bounds().y;
    let gap = |app: &App| {
        let (a, b) = (
            app.launcherbar.item_rect("launch:0").unwrap(),
            app.launcherbar.item_rect("launch:1").unwrap(),
        );
        (a.w, b.x - a.right())
    };
    assert_eq!(gap(&app), (24, 4), "칸 = 20 + 여백 4 · 간격 4");
    let _ = app.settings.set("launcher.icon_size", "32");
    app.after_setting_changed("launcher.icon_size");
    assert_eq!(app.launcherbar.bounds().h, 40);
    assert_eq!(app.panels[0].bounds().y, panel_y + 12);
    assert_eq!(gap(&app).0, 36);
    let _ = app.settings.set("launcher.item_gap", "0");
    app.after_setting_changed("launcher.item_gap");
    assert_eq!(gap(&app).1, 0);
    let _ = app.settings.set("launcher.icon_size", "16");
    app.after_setting_changed("launcher.icon_size");
    assert_eq!(app.launcherbar.bounds().h, 24);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 터미널 두부(사용자 10-03): Nerd Font가 설치돼 있으면 고정폭 체인이 프롬프트 아이콘 글리프(Powerline · Font Awesome 대역)를 가진다 ·
/// 없으면 체인은 그래도 만들어진다(시험은 설치 여부에 따라 분기 — 실패 아님).
#[test]
fn terminal_font_chain_covers_nerd_glyphs_when_installed() {
    let font = app::fonts::mono_chain(None, "").expect("mono chain");
    let has_nerd = [
        "Symbols Nerd Font",
        "JetBrainsMonoNL Nerd Font",
        "JetBrainsMono Nerd Font",
        "CaskaydiaCove Nerd Font",
        "MesloLGS NF",
        "FiraCode Nerd Font",
        "Hack Nerd Font",
    ]
    .iter()
    .any(|f| nexa_font::find_font_by_family(f).is_some());
    let covered = app::fonts::NERD_PROBE.iter().all(|&c| font.covers(c));
    eprintln!("nerd font installed = {has_nerd} · probe covered = {covered}");
    if has_nerd {
        assert!(covered, "설치된 Nerd Font가 터미널 폴백에 들어가야 한다");
    }
    // 본문 글자·한글은 언제나.
    assert!(font.covers('A') && font.covers('한'));
}

/// dir2 스크롤 조치(X-63 · 7d8b1e9 · eb29089 · e230f36) 호스트 반영: `scroll.*` 설정이 nexa-grid/nexa-ctl 전역 고속 스크롤에 닿고
/// (파일 그리드 = 한 단계 더 빠르게) · 터미널 휠은 분수 delta를 누적해 줄 단위로 환산한다(종전 = 120 미만은 0으로 버림).
#[test]
fn scroll_settings_reach_controls_and_terminal_wheel_accumulates() {
    let (mut app, dir) = fixture("scrollcfg");
    // 값 확인은 순수 계산으로(전역 고속 스크롤은 프로세스 공유 — 병렬 시험이 덮어써 흔들렸다 · 전역에 쓰는 것은 apply 한 줄뿐).
    let _ = app.settings.set("scroll.fast_step", "5");
    let _ = app.settings.set("scroll.fast_max", "9");
    let _ = app.settings.set("scroll.fast_hud", "off");
    let (g, grid, c) = App::scroll_configs(&app.settings);
    assert_eq!((g.enabled, g.step, g.max, g.hud), (true, 5, 9, false));
    let grid = grid.expect("scroll.fast_grid_extra 기본 on");
    assert_eq!(
        (grid.step, grid.max),
        (4, 18),
        "그리드 = 한 단계 더 빠르게(step-1 · max×2)"
    );
    assert_eq!((c.enabled, c.step, c.max), (true, 5, 9));
    let _ = app.settings.set("scroll.fast", "off");
    let (g, _, c) = App::scroll_configs(&app.settings);
    assert!(!g.enabled && !c.enabled);
    let _ = app.settings.set("scroll.fast_grid_extra", "off");
    assert!(App::scroll_configs(&app.settings).1.is_none());
    for k in [
        "scroll.fast",
        "scroll.fast_step",
        "scroll.fast_max",
        "scroll.fast_hud",
        "scroll.fast_grid_extra",
    ] {
        let _ = app.settings.reset(k);
    }
    // 휠 누적기: 30씩 4번 = 노치 1개 = 시스템 줄 수만큼.
    let mut acc = nexa_ctl::WheelAccum::default();
    let lines: i32 = (0..4).map(|_| acc.add(30, 3)).sum();
    assert_eq!(lines, 3);
    assert_eq!(
        app.terms[0].wheel.add(60, 4),
        2,
        "터미널 누적기 = 분수 delta도 반영"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// ⚠ GAP-011: 경로 바 편집 중 Ctrl+C/X/V/Z · Delete는 **경로 글자**를 다룬다 — 파일 붙여넣기(전송)·삭제·되돌리기가 실행되면 안 된다.
#[test]
fn path_edit_shortcuts_edit_text_not_files() {
    let (mut app, dir) = fixture("pathedit");
    app.layout_for(1200, 800, 1.0);
    let mut inv = Invalidations::default();
    // 파일을 하나 선택해 두고(복사 대상 후보) 경로 편집에 들어간다.
    app.panels[0].select_path(&dir.join("a.txt"), &mut inv);
    app.panels[0].pathbar.begin_edit(&mut inv);
    assert!(app.panels[0].pathbar.is_editing());
    let log = app.platform.log.clone().expect("fake log");
    let ops_before = log.borrow().calls.len();
    let entries_before = std::fs::read_dir(&dir).unwrap().count();
    for id in [
        "edit.copy",
        "edit.cut",
        "edit.paste",
        "edit.undo",
        "edit.delete",
        "edit.redo",
    ] {
        app.command(id);
        assert!(
            app.panels[0].pathbar.is_editing(),
            "{id}: 편집 상태 유지(파일 명령으로 빠지지 않는다)"
        );
    }
    assert_eq!(
        log.borrow().calls.len(),
        ops_before,
        "휴지통·파일 클립보드 등 플랫폼 호출 없음: {:?}",
        log.borrow().calls
    );
    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        entries_before,
        "파일 변화 없음"
    );
    assert!(dir.join("a.txt").exists());
    assert!(!app.toasts.animating(), "전송/오류 토스트 없음");
    // 전체 선택 → 삭제 = 경로 글자가 비워진다.
    app.command("edit.select_all");
    app.command("edit.delete");
    assert_eq!(app.panels[0].pathbar.edit_text().as_deref(), Some(""));
    // 편집 중이 아니면 가로채지 않는다.
    app.panels[0].pathbar.cancel_edit(&mut inv);
    assert!(!app.path_edit("edit.copy"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// 키보드로 연 행 메뉴(SHELL-003 · 10-03 캡처 판정 "창 왼쪽 위 모서리에 열림"): 마우스 커서가 아니라 캐럿 행 자리에 열린다.
#[test]
fn keyboard_context_menu_opens_at_the_caret_row() {
    let (mut app, dir) = fixture("ctxkbd");
    app.layout_for(1000, 700, 1.0);
    app.cursor = (0, 0);
    app.set_active(0);
    app.route(InputEvent::Key {
        key: nexa_ctl::Key::Down,
        shift: false,
        primary: false,
    });
    let rows = app.panels[0].rows();
    let at = rows
        .row_anchor(rows.caret().expect("caret"))
        .expect("visible");
    let bounds = rows.bounds();
    app.command("cmd.contextMenu");
    assert_eq!(app.ctx_anchor.1, at.y, "캐럿 행 높이");
    assert!(
        app.ctx_anchor.0 > at.x
            && bounds.contains(nexa_ctl::Point {
                x: app.ctx_anchor.0,
                y: app.ctx_anchor.1,
            })
    );
    assert!(app.ctx_anchor_next.is_none(), "한 번 쓰고 비운다");
    // 마우스 우클릭 경로는 종전대로 커서 자리.
    app.tab_menu.close();
    app.ctx_wait = None;
    app.cursor = (300, 200);
    app.open_row_menu(0);
    assert_eq!(app.ctx_anchor, (300, 200));
    let _ = std::fs::remove_dir_all(&dir);
}

/// GAP-012: 경로 편집 필드 우클릭 = 글자 편집 메뉴(6항목 · 활성 규칙) · 항목 실행은 경로 글자에만 · 더블클릭 = 전체 선택.
#[test]
fn path_edit_right_click_opens_text_menu() {
    let (mut app, dir) = fixture("pathmenu");
    app.layout_for(1200, 800, 1.0);
    let pb = app.panels[0].pathbar.bounds();
    let (x, y) = (pb.x + 12, pb.y + pb.h / 2);
    // 첫 우클릭 = 편집 진입(메뉴 없음) · 편집 중 우클릭 = 메뉴.
    app.route(InputEvent::RightDown { x, y });
    assert!(app.panels[0].pathbar.is_editing() && !app.tab_menu.is_open());
    app.route(InputEvent::RightDown { x, y });
    assert!(app.tab_menu.is_open());
    let d = app.dump_of("ctx").unwrap();
    assert_eq!(
        d.trim(),
        "pathedit edit.undo edit.cut edit.copy edit.paste edit.delete edit.select_all"
    );
    assert!(app.panels[0].pathbar.is_editing(), "메뉴가 떠도 편집 유지");
    // 삭제 = 선택된 경로 글자만 지운다(편집 진입 = 전체 선택 상태) · 파일은 그대로.
    let before = std::fs::read_dir(&dir).unwrap().count();
    app.ctx_pick("edit.delete");
    assert_eq!(app.panels[0].pathbar.edit_text().as_deref(), Some(""));
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), before);
    // 실행 취소로 글자 복구 → 더블클릭 = 전체 선택(선택이 있어야 "복사"가 활성).
    app.route(InputEvent::RightDown { x, y });
    app.ctx_pick("edit.undo");
    let text = app.panels[0].pathbar.edit_text().unwrap();
    assert!(!text.is_empty(), "되돌림");
    app.route(InputEvent::Key {
        key: nexa_ctl::Key::Home,
        shift: false,
        primary: false,
    });
    assert_eq!(
        app.panels[0].pathbar.edit_menu_state().map(|s| s.1),
        Some(false),
        "Home = 선택 해제"
    );
    app.route(InputEvent::DoubleClick {
        x,
        y,
        shift: false,
        primary: false,
    });
    assert_eq!(
        app.panels[0].pathbar.edit_menu_state().map(|s| s.1),
        Some(true),
        "더블클릭 = 전체 선택"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// GAP-007: 폴더를 가리키는 `.lnk` 활성화 = 앱 안 이동(OS 열기 호출 없음) · 대상이 파일/해석 불가 = OS 열기 · `.lnk`가 아니면 해석하지 않는다.
#[test]
fn folder_shortcut_navigates_inside_the_app() {
    let (mut app, dir) = fixture("lnknav");
    app.layout_for(1200, 800, 1.0);
    let log = app.platform.log.clone().expect("fake log");
    let lnk = dir.join("To Sub.lnk");
    std::fs::write(&lnk, b"fake").unwrap();
    let opens = |log: &std::rc::Rc<std::cell::RefCell<crate::platform::fake::FakeLog>>| {
        log.borrow()
            .calls
            .iter()
            .filter(|c| c.starts_with("open:"))
            .count()
    };
    // ① 대상 = 폴더 → 이동.
    log.borrow_mut().link_target = Some(dir.join("sub"));
    app.open_external(&lnk);
    assert_eq!(app.panels[0].root_path(), dir.join("sub"));
    assert_eq!(opens(&log), 0, "OS 열기 없음");
    // ② 대상 = 파일 → OS 열기(바로 가기 자체를 넘긴다).
    log.borrow_mut().link_target = Some(dir.join("a.txt"));
    app.open_external(&lnk);
    assert_eq!(app.panels[0].root_path(), dir.join("sub"));
    assert_eq!(opens(&log), 1);
    // ③ 해석 불가 → OS 열기 · ④ `.lnk`가 아닌 파일은 대상이 주입돼 있어도 그냥 연다.
    log.borrow_mut().link_target = None;
    app.open_external(&lnk);
    assert_eq!(opens(&log), 2);
    // 명령 `cmd.activate` = 캐럿 행 활성화(파일 = OS 열기).
    let mut inv = Invalidations::default();
    app.panels[0].navigate_to(dir.clone(), &mut inv);
    app.panels[0].select_path(&dir.join("a.txt"), &mut inv);
    app.command("cmd.activate");
    assert_eq!(opens(&log), 3);
    app.panels[0].navigate_to(dir.join("sub"), &mut inv);
    log.borrow_mut().link_target = Some(dir.clone());
    app.open_external(&dir.join("a.txt"));
    assert_eq!(opens(&log), 4);
    assert_eq!(app.panels[0].root_path(), dir.join("sub"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// GAP-008: 링크 표식(재분석 지점)이 있는 행은 종류별 키("dir")가 아니라 경로 키로 아이콘을 묻는다(셸이 화살표 오버레이를 얹게) ·
/// 앱 시작 때 nexa-fs 링크 오버레이가 켜진다.
#[test]
fn link_rows_use_per_path_icons() {
    use nexa_grid::RowSource as _;
    let (app, dir) = fixture("lnkicon");
    let src = app.panels[0].rows().source();
    let (key, hint) = src.icon(0).expect("icon");
    assert_eq!(key, "dir", "일반 폴더 = 종류별 키: {hint}");
    crate::app::row_icons::install();
    assert!(nexa_fs::shell::link_overlay_enabled());
    let _ = std::fs::remove_dir_all(&dir);
}

/// 탐색 단축키를 **키맵 경로 그대로**(조합 → 키맵 → 명령) 확인(사용자 10-03 "Alt 이동 · 목록 Enter"): Enter · Alt+↓ = 활성화 ·
/// Alt+← → ↑ = 뒤로 · 앞으로 · 위로 · 메뉴가 열려 있으면 Enter는 가로채지 않는다.
#[test]
fn nav_shortcuts_work_through_the_keymap() {
    use ndir_settings::Chord;
    let (mut app, dir) = fixture("navkeys");
    app.layout_for(1200, 800, 1.0);
    let mut inv = Invalidations::default();
    let press = |app: &mut App, code: &str| app.key_chord(Chord::parse(code).expect(code), false);
    // Enter = 캐럿 폴더로 진입.
    app.panels[0].select_path(&dir.join("sub"), &mut inv);
    assert!(press(&mut app, "enter"));
    assert_eq!(app.panels[0].root_path(), dir.join("sub"), "Enter = 진입");
    // Alt+← = 뒤로 · Alt+→ = 앞으로 · Alt+↑ = 위로.
    let (back, fwd, up) = if cfg!(target_os = "macos") {
        ("cmd+[", "cmd+]", "cmd+up")
    } else {
        ("alt+left", "alt+right", "alt+up")
    };
    assert!(press(&mut app, back));
    assert_eq!(app.panels[0].root_path(), dir, "뒤로");
    assert!(press(&mut app, fwd));
    assert_eq!(app.panels[0].root_path(), dir.join("sub"), "앞으로");
    assert!(press(&mut app, up));
    assert_eq!(app.panels[0].root_path(), dir, "위로");
    // Alt+↓(macOS ⌘↓) = 활성화.
    app.panels[0].select_path(&dir.join("sub"), &mut inv);
    assert!(press(
        &mut app,
        if cfg!(target_os = "macos") {
            "cmd+down"
        } else {
            "alt+down"
        }
    ));
    assert_eq!(app.panels[0].root_path(), dir.join("sub"), "Alt+↓ = 진입");
    // 파일 위 Enter = OS 열기(가짜 포트 기록).
    assert!(press(&mut app, up));
    app.panels[0].select_path(&dir.join("a.txt"), &mut inv);
    let log = app.platform.log.clone().expect("fake log");
    assert!(press(&mut app, "enter"));
    assert!(log
        .borrow()
        .calls
        .iter()
        .any(|c| c.starts_with("open:") && c.ends_with("a.txt")));
    // 메뉴가 열려 있으면 Enter는 메뉴 몫(가로채지 않는다) · 글자 키는 타이핑(타입어헤드).
    app.cursor = (300, 200);
    app.open_bg_menu(0);
    app.ctx_wait = None;
    app.open_tab_menu(0, 0);
    assert!(app.tab_menu.is_open());
    assert!(!press(&mut app, "enter"), "메뉴 열림 = 통과");
    app.tab_menu.close();
    assert!(!press(&mut app, "a"), "글자 = 타이핑");
    let _ = std::fs::remove_dir_all(&dir);
}
