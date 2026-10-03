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
    let settings = Settings::from_text(
        dir.join("settings.conf"),
        "launcher.visible=off\nlauncher.seed=2\n",
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
        "Name", "Size", "Ext", "sub", "a.txt", "b.md", "3 items", "Tab 1/1", "File", "\u{2190}",
    ] {
        assert!(
            rec.drew_text(needle),
            "missing text {needle:?}: {:?}",
            rec.strings().collect::<Vec<_>>()
        );
    }
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
    // 숨김 토글은 설정 + 메뉴 체크 + 툴바 토글이 함께 바뀐다.
    let before = app.settings.flag("list.show_hidden");
    app.command("view.hidden");
    assert_eq!(app.settings.flag("list.show_hidden"), !before);
    assert_eq!(app.menubar.is_checked("view.hidden"), Some(!before));
    assert_eq!(app.toolbar.item_checked("view.hidden"), !before);
    // 테마 순환: dark → system → light → dark.
    assert_eq!(app.settings.theme_mode(), ThemeMode::Dark);
    app.command("view.theme_cycle");
    assert_eq!(app.settings.theme_mode(), ThemeMode::System);
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
        !app.settings.flag("list.show_hidden"),
        "key:{first} = view.hidden 토글(기본 on → off)"
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
        list.contains("0 d0 Collapsed sub |") && list.contains("a.txt | txt | 5 B |"),
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
    assert!(app.dump_of("prefs").unwrap().contains("list.show_dotfiles"));
    app.startup_cmd("prefs.cat:pref.cat.keys");
    assert!(app.dump_of("prefs").unwrap().contains("key.file.new_tab"));
    // 설정 창이 값을 바꿨을 때의 길: 저장 → 적용 → 메뉴 체크 동기.
    let _ = app.settings.set("list.show_hidden", "off");
    app.after_setting_changed("list.show_hidden");
    assert_eq!(app.menubar.is_checked("view.hidden"), Some(false));
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
        .set_col_widths(&[200, 50, 60, 70, 80], &mut inv);
    let _ = app.settings.set("list.col_width_sync", "off");
    app.command("view.col_width_sync");
    assert!(app.settings.flag("list.col_width_sync"));
    assert_eq!(app.panels[1].col_widths_now(), vec![200, 50, 60, 70, 80]);
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
    assert_eq!(app.panels[0].rows().columns().len(), 5);
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
    assert_eq!((lb.y, lb.h), (app.toolbar.bounds().bottom(), 24), "{lb:?}");
    assert_eq!(app.panels[0].bounds().y, lb.bottom(), "패널은 런처 아래");
    assert!(app.dump_of("layout").unwrap().contains("launcher 0,"));
    let ld = app.dump_of("launcher").unwrap();
    assert_eq!(ld.lines().count(), 5, "{ld}");
    // T-30 B 아이콘: 없는 exe = 즉시 글리프 `Ba` · 자기 exe = Windows면 셸 아이콘(비동기 → 폴링 후 image) · 다른 OS = 글리프 `Se`.
    assert!(ld.contains("launch:2=glyph:Ba"), "{ld}");
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
        .items()
        .iter()
        .filter(|it| matches!(it.icon, nexa_ctl::ToolIcon::Mask { w: 20, h: 20, .. }))
        .count();
    assert_eq!(masks, 13, "13개 명령 = SVG 마스크(20px)");
    app.layout_for(1200, 800, 2.0);
    app.rebuild_toolbar();
    app.sync_menu_checks();
    assert!(
        app.toolbar
            .items()
            .iter()
            .all(|it| it.separator
                || matches!(it.icon, nexa_ctl::ToolIcon::Mask { w: 40, h: 40, .. }))
    );
    assert_eq!(App::toolbar_icon_px(1.5), 30);
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
    let ids =
        |app: &App| -> Vec<String> { app.toolbar.items().iter().map(|t| t.id.clone()).collect() };
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
    // 누락 foldersfirst는 가장 가까운 앞 형제(dot) 뒤로 보충된다(dir2 규칙).
    assert_eq!(after[0], "view.dot");
    assert_eq!(after[1], "view.folders_first");
    assert_eq!(after[2], "view.hidden");
    assert_eq!(
        app.settings.get("toolbar.layout").unwrap(),
        "show:1[dot:1,foldersfirst:1,hidden:1]|view:0[tree:1,flat:1,tiles:1]|refresh:1|panel:1[toggle:0,dock:1,info:1,colsync:1,ontop:1]|settings:1",
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
    // 컬럼: ext 앞 · size 숨김 → 활성 패널 + 동기(기본 on) 반대 패널 · 문자열 왕복 · 세션 스냅숏.
    assert_eq!(
        app.order_value_of("list.col_layout"),
        "cols:1[name:1,ext:1,size:1,modified:1,kind:1]"
    );
    app.order_changed("list.col_layout", "cols[ext,name,size:0]");
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
        "cols:1[ext:1,name:1,modified:1,kind:1,size:0]"
    );
    assert_eq!(
        app.session_snapshot().panels[0].col_layout,
        "cols:1[ext:1,name:1,modified:1,kind:1,size:0]"
    );
    // 전부 숨김 = name 강제.
    app.order_changed(
        "list.col_layout",
        "cols[name:0,ext:0,size:0,modified:0,kind:0]",
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

/// 우클릭 가속(사용자 10-03 · dir2 X-61 · SHELL-014/015): 셸 항목이 구축 중이면 메뉴는 **즉시**(자체 항목 + "불러오는 중") 열리고,
/// 통지가 오면 같은 자리에서 셸 항목으로 채워진다 · 선택이 300 ms 머물면 선행 구축을 건다 · 비동기 실행 결과는 틱에서 반영.
#[test]
fn context_menu_opens_immediately_and_fills_when_shell_items_arrive() {
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
    // ② 우클릭: 셸 항목이 아직 없어도 즉시 열린다(자체 항목 + 불러오는 중).
    app.cursor = (320, 240);
    app.open_row_menu(0);
    assert!(app.tab_menu.is_open());
    let d = app.dump_of("ctx").unwrap();
    assert!(
        d.starts_with("row ") && d.contains("cmd.activate") && d.contains("ctx.loading"),
        "{d}"
    );
    assert!(d.contains("edit.copy") && d.contains("edit.delete"), "{d}");
    assert_eq!(shared.borrow().asked, vec![target.clone()]);
    let at = (app.tab_menu.bounds().x, app.tab_menu.bounds().y);
    assert!(app.ctx_shell_tick(t1).is_some(), "기다리는 동안 틱 유지");
    // ③ 다른 대상의 통지는 무시 · 같은 대상의 통지 = 같은 자리에서 채움.
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
    assert!(app.dump_of("ctx").unwrap().contains("ctx.loading"));
    shared.borrow_mut().events.push_back(MenuEvent::Items {
        target,
        items: shell_items,
    });
    let _ = app.ctx_shell_tick(t1);
    let d = app.dump_of("ctx").unwrap();
    assert!(d.contains("shell:7") && !d.contains("ctx.loading"), "{d}");
    assert!(app.tab_menu.is_open());
    assert_eq!(
        (app.tab_menu.bounds().x, app.tab_menu.bounds().y),
        at,
        "같은 자리"
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
    let _ = std::fs::remove_dir_all(&dir);
}
