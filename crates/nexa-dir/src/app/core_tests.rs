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
    let settings = Settings::from_text(dir.join("settings.conf"), "");
    let font = nexa_font::ui_font(None).expect("OS UI font (CI installs fonts)");
    (App::new(settings, font.font, Some(dir.clone()), None), dir)
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
    assert_eq!(l.bottom(), app.statusbar.bounds().y);
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
    let mut app2 = App::new(settings, font.font, None, Some(broken));
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
    let app3 = App::new(settings, font.font, Some(dir.join("sub")), Some(parsed));
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
