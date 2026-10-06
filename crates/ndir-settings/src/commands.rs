//! ★ 명령 표 — 메뉴바·도구 모음·컨텍스트 메뉴·단축키·기동 명령·하네스가 **같은 어휘**(문자열 id)를 쓴다(docs/01 §4 · docs/port/40 SKEL-421).
//!
//! 출처 = dir2 명령 상수(`win.rs:208-278` · docs/port/30 CMD-120~167)와 단축키 표(CMD-287~329) + macOS 대응안(CMD-480~497).
//! 구조 = nexa-sql `keymap.rs` `COMMANDS`(id · 라벨 · OS별 기본 코드 · 설정 `key.<id>` 재정의 · `key.preset`).
//! 코드 문법 = `ctrl+shift+n` · `cmd+,` · `f5` · 대안은 `|` · 2단은 `,`(`ctrl+k,ctrl+u`) · `none` = 없음. `ctrl`/`cmd`/`primary` = 주 조합키
//! (Windows/Linux Ctrl · macOS ⌘) · macOS Control 단독 = `control+…`.
//!
//! dir2와 다른 점(의도된 차이 · 검증 매트릭스에 등재): `tab.prev`(⌃⇧Tab · CMD-487 권장) · `edit.redo`의 둘째 코드 `ctrl+shift+z`(dir2 CMD-304에
//! 있으나 메뉴 표기 없음) · macOS 코드는 CMD-480~497 대응안. 동적 명령(언어 i · 런처 i · 클라우드 i)은 표에 없다 — 호스트가 `lang:<code>`·
//! `launcher:<n>`·`cloud.<동작>:<n>` 꼴로 만든다(단축키 재정의 대상 아님).

/// 명령 하나 — 기본 코드가 없는 명령은 `""`(메뉴·도구 모음 전용이지만 `key.<id>`로 지정할 수 있다).
#[derive(Clone, Copy, Debug)]
pub struct Command {
    /// `<영역>.<동작>` — 메뉴 항목 id · 기동 명령 id와 같다.
    pub id: &'static str,
    /// 라벨 i18n 키(dir2 `menu.*`·`ctx.*` 재사용 · 없던 것은 `cmd.*`).
    pub label: &'static str,
    pub win: &'static str,
    pub mac: &'static str,
    pub linux: &'static str,
    /// 키를 누르고 있을 때의 자동 반복을 그대로 실행해도 되는가(nexa-sql 09-17 Ctrl+T 80개 교훈) — 이동·편집만 참.
    pub repeatable: bool,
}

macro_rules! c {
    ($id:expr, $label:expr, $win:expr, $mac:expr, $linux:expr) => {
        Command {
            id: $id,
            label: $label,
            win: $win,
            mac: $mac,
            linux: $linux,
            repeatable: false,
        }
    };
    ($id:expr, $label:expr, $win:expr, $mac:expr, $linux:expr, repeat) => {
        Command {
            id: $id,
            label: $label,
            win: $win,
            mac: $mac,
            linux: $linux,
            repeatable: true,
        }
    };
}

/// 명령 표(순서 = 단축키 창 목록 순서 = dir2 메뉴 순서).
pub const COMMANDS: &[Command] = &[
    // ── 파일(CMD-001~006)
    c!(
        "file.new_tab",
        "menu.file.newTab",
        "ctrl+t",
        "cmd+t",
        "ctrl+t"
    ),
    c!(
        "file.close_tab",
        "menu.file.closeTab",
        "ctrl+w",
        "cmd+w",
        "ctrl+w"
    ),
    c!(
        "file.new_folder",
        "menu.file.newFolder",
        "ctrl+shift+n",
        "cmd+shift+n",
        "ctrl+shift+n"
    ),
    c!("file.new_file", "menu.file.newFile", "", "", ""),
    c!("file.prefs", "menu.file.prefs", "ctrl+,", "cmd+,", "ctrl+,"),
    c!("file.exit", "menu.file.exit", "", "cmd+q", ""),
    // ── 편집(CMD-007~013 · 목록 키 CMD-303~309 · F2)
    c!(
        "edit.undo",
        "menu.edit.undo",
        "ctrl+z",
        "cmd+z",
        "ctrl+z",
        repeat
    ),
    c!(
        "edit.redo",
        "menu.edit.redo",
        "ctrl+y|ctrl+shift+z",
        "cmd+shift+z|cmd+y",
        "ctrl+y|ctrl+shift+z",
        repeat
    ),
    c!("edit.cut", "menu.edit.cut", "ctrl+x", "cmd+x", "ctrl+x"),
    c!("edit.copy", "menu.edit.copy", "ctrl+c", "cmd+c", "ctrl+c"),
    c!("edit.paste", "menu.edit.paste", "ctrl+v", "cmd+v", "ctrl+v"),
    c!(
        "edit.select_all",
        "menu.edit.selectAll",
        "ctrl+a",
        "cmd+a",
        "ctrl+a"
    ),
    // 선택 반전(dir3 신규 · 탐색기 "선택 영역 반전").
    c!(
        "edit.select_invert",
        "menu.edit.selectInvert",
        "ctrl+shift+a",
        "cmd+shift+a",
        "ctrl+shift+a"
    ),
    c!(
        "edit.bulk_rename",
        "menu.edit.bulkRename",
        "ctrl+shift+r",
        "cmd+shift+r",
        "ctrl+shift+r"
    ),
    c!("edit.rename", "cmd.rename", "f2", "f2", "f2"),
    // macOS 노트북에는 Delete 키가 없다(CMD-483) — ⌘⌫ = 휴지통 · ⌥⌘⌫ = 완전 삭제.
    c!(
        "edit.delete",
        "menu.edit.delete",
        "delete",
        "cmd+backspace|delete",
        "delete"
    ),
    c!(
        "edit.delete_permanent",
        "ctx.deletePermanent",
        "shift+delete",
        "cmd+alt+backspace|shift+delete",
        "shift+delete"
    ),
    // ── 보기(CMD-014~031 · 키 CMD-291~293 · 313~316)
    c!("view.mode_tree", "menu.view.modeTree", "", "", ""),
    c!("view.mode_flat", "menu.view.modeFlat", "", "", ""),
    c!("view.mode_tiles", "menu.view.modeTiles", "", "", ""),
    c!("view.panel_dual", "menu.view.panelDual", "", "", ""),
    c!("view.panel_single", "menu.view.panelSingle", "", "", ""),
    c!("view.panel_toggle", "cmd.panelToggle", "", "", ""),
    c!("view.info_dual", "menu.view.infoDual", "", "", ""),
    c!("view.info_single", "menu.view.infoSingle", "", "", ""),
    c!("view.info_toggle", "cmd.infoToggle", "", "", ""),
    c!("view.col_width_sync", "menu.view.colWidthSync", "", "", ""),
    // ⌘H = 앱 숨기기 · ⌘. = 취소라 macOS는 ⇧⌘.(Finder 관례 · CMD-486) — 점 파일은 macOS에서 숨김과 같은 개념이라 기본 코드 없음.
    c!(
        "view.hidden",
        "menu.view.hidden",
        "ctrl+h",
        "cmd+shift+.",
        "ctrl+h"
    ),
    c!("view.dot", "menu.view.dot", "ctrl+.", "", "ctrl+."),
    c!("view.folders_first", "pref.sortFoldersFirst", "", "", ""),
    c!("view.case_sensitive", "pref.sortCaseSensitive", "", "", ""),
    c!("view.natural_sort", "pref.sortNatural", "", "", ""),
    // ⌘` 는 OS의 창 순환(CMD-489) → macOS는 Control 단독.
    c!(
        "view.dock",
        "menu.view.dock",
        "ctrl+`",
        "control+`",
        "ctrl+`"
    ),
    c!("view.launcher", "menu.view.launcher", "", "", ""),
    c!("view.always_on_top", "menu.view.alwaysOnTop", "", "", ""),
    c!("view.refresh", "menu.view.refresh", "f5", "cmd+r|f5", "f5"),
    c!("view.theme_system", "menu.view.theme.system", "", "", ""),
    c!("view.theme_light", "menu.view.theme.light", "", "", ""),
    c!("view.theme_dark", "menu.view.theme.dark", "", "", ""),
    // dir2는 테마 3항목 모두 "F6"으로 표기(CMD-491) — 순환 명령 하나에만 붙인다.
    c!("view.theme_cycle", "cmd.themeCycle", "f6", "f6", "f6"),
    c!("view.lang_system", "menu.view.lang.system", "", "", ""),
    // F3 독립 미리보기 창(CMD-293) — macOS는 ⇧⌘Y 보조(⌘Y는 edit.redo 둘째 코드와 충돌 · 시험이 잡음 10-03).
    c!(
        "view.preview_window",
        "cmd.previewWindow",
        "f3",
        "cmd+shift+y|f3",
        "f3"
    ),
    // 명령 팔레트(dir3 신규 · T-138 · 사용자 10-04 · Sublime/VS Code 관례 Ctrl+⇧P · macOS ⌘⇧P).
    c!(
        "view.palette",
        "cmd.palette",
        "ctrl+shift+p",
        "cmd+shift+p",
        "ctrl+shift+p"
    ),
    // ── 탐색(CMD-326~329 Alt 조합 · macOS는 ⌘[ ⌘] ⌘↑ ⌘↓ — ⌥←→는 편집 필드 단어 이동)
    c!(
        "nav.back",
        "cmd.navBack",
        "alt+left",
        "cmd+[",
        "alt+left",
        repeat
    ),
    c!(
        "nav.forward",
        "cmd.navForward",
        "alt+right",
        "cmd+]",
        "alt+right",
        repeat
    ),
    c!("nav.up", "cmd.navUp", "alt+up", "cmd+up", "alt+up", repeat),
    // 폴더 즐겨찾기(dir3 신규 · T-168 · TC 핫리스트 Ctrl+D 관례).
    c!(
        "nav.favorites",
        "cmd.favorites",
        "ctrl+b",
        "cmd+b",
        "ctrl+b"
    ),
    c!(
        "nav.fav_toggle",
        "cmd.favToggle",
        "ctrl+d",
        "cmd+d",
        "ctrl+d"
    ),
    c!(
        "nav.activate",
        "cmd.activate",
        "enter|alt+down",
        "enter|cmd+down|cmd+o",
        "enter|alt+down"
    ),
    // ── 패널·탭(CMD-295~296 · tab.prev는 dir2에 없던 명령 — CMD-487 권장 · 의도된 차이)
    c!("panel.switch", "cmd.panelSwitch", "tab", "tab", "tab"),
    c!(
        "tab.next",
        "cmd.tabNext",
        "ctrl+tab",
        "control+tab|cmd+shift+]",
        "ctrl+tab",
        repeat
    ),
    c!(
        "tab.prev",
        "cmd.tabPrev",
        "ctrl+shift+tab",
        "control+shift+tab|cmd+shift+[",
        "ctrl+shift+tab",
        repeat
    ),
    // ── 컨텍스트 메뉴(CMD-310 · 325 · macOS는 Apps 키가 없다 → ⌃Return · CMD-492)
    c!(
        "list.context_menu",
        "cmd.contextMenu",
        "shift+f10|contextmenu",
        "control+enter|shift+f10",
        "shift+f10|contextmenu"
    ),
    // ── 도움말
    c!("help.about", "menu.help.about", "", "", ""),
    c!("help.license", "menu.help.license", "", "", ""),
    c!("help.selfcheck", "menu.help.selfcheck", "", "", ""),
];

/// id로 찾기.
#[must_use]
pub fn command(id: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.id == id)
}

/// 설정 키(`key.<id>`).
#[must_use]
pub fn setting_key(id: &str) -> String {
    format!("key.{id}")
}

/// 기본 세트 프리셋(설정 `key.preset` = auto|windows|macos|linux).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    Windows,
    Macos,
    Linux,
}

impl Preset {
    /// 실행 OS의 프리셋.
    #[must_use]
    pub fn os() -> Preset {
        if cfg!(target_os = "macos") {
            Preset::Macos
        } else if cfg!(target_os = "linux") {
            Preset::Linux
        } else {
            Preset::Windows
        }
    }

    /// 설정 값(`auto` = OS).
    #[must_use]
    pub fn parse(v: &str) -> Preset {
        match v {
            "windows" => Preset::Windows,
            "macos" => Preset::Macos,
            "linux" => Preset::Linux,
            _ => Preset::os(),
        }
    }
}

/// 프리셋별 기본 코드.
#[must_use]
pub fn preset_default(c: &Command, p: Preset) -> &'static str {
    match p {
        Preset::Windows => c.win,
        Preset::Macos => c.mac,
        Preset::Linux => c.linux,
    }
}

/// 자동 반복 허용 여부(모르는 id = false).
#[must_use]
pub fn repeatable(id: &str) -> bool {
    command(id).is_some_and(|c| c.repeatable)
}
