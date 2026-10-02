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
    app.command("edit.bulk_rename");
    assert!(app.statusbar.left().starts_with("edit.bulk_rename:"));
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
    assert!(d.starts_with("closed a.txt lines 1 top 0\nhello\n"), "{d}");
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
            && d.contains("file.new_folder"),
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
    assert_eq!(ld.lines().count(), 4, "{ld}");
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
