//! ★ 설정 레지스트리 — 단일 원천(DR-3). 새 설정 = 여기 한 줄 + i18n 라벨/설명(3언어) + 호스트 `apply_setting` 분기.
//!
//! 키 = nexa-sql 규칙(`<접두>.<이름>` · 접두 ↔ 카테고리) · 값·기본값·페이지 구성 = dir2 계승(docs/port/15 PREFS-101~170 ·
//! docs/port/31 §5-3 KEY-501~599 대응표). 등재 순서 = 설정 창 표시 순서(dir2 PREFS-310 카테고리별 항목 순서 그대로).
//! 라벨은 dir2 `.lang`의 `pref.*` 키 — 없던 것은 10-03에 추가(`ndir-i18n/lang`).

use crate::{Dep, Entry, SettingKind};

// ── 선택지(값, 라벨 i18n 키)
const THEME_OPTS: &[(&str, &str)] = &[
    ("system", "pref.theme.system"),
    ("light", "pref.theme.light"),
    ("dark", "pref.theme.dark"),
];
const TOOLBAR_SIZE_OPTS: &[(&str, &str)] = &[
    ("16", "pref.toolbarSize.16"),
    ("20", "pref.toolbarSize.20"),
    ("24", "pref.toolbarSize.24"),
    ("32", "pref.toolbarSize.32"),
];
const TOOLBAR_ON_COLOR_OPTS: &[(&str, &str)] = &[
    ("line", "pref.toolbarOnColor.line"),
    ("accent", "pref.toolbarOnColor.accent"),
];
const VIEW_SCOPE_OPTS: &[(&str, &str)] = &[
    ("global", "pref.viewScope.global"),
    ("panel", "pref.viewScope.panel"),
    ("tab", "pref.viewScope.tab"),
    ("dir", "pref.viewScope.dir"),
];
const ALIGN_OPTS: &[(&str, &str)] = &[
    ("top", "pref.align.top"),
    ("center", "pref.align.center"),
    ("bottom", "pref.align.bottom"),
];
const TA_SCOPE_OPTS: &[(&str, &str)] = &[
    ("global", "pref.taScope.global"),
    ("level", "pref.taScope.level"),
    ("visible", "pref.taScope.visible"),
];
const TAB_DBL_OPTS: &[(&str, &str)] = &[
    ("close", "pref.tabDbl.close"),
    ("pin", "pref.tabDbl.pin"),
    ("lock", "pref.tabDbl.lock"),
];
const TAB_SCROLL_BTN_OPTS: &[(&str, &str)] = &[
    ("end", "pref.tabScrollButtons.end"),
    ("start", "pref.tabScrollButtons.start"),
    ("split", "pref.tabScrollButtons.split"),
];
const TERM_COPY_OPTS: &[(&str, &str)] = &[
    ("text", "pref.termCopy.text"),
    ("html", "pref.termCopy.html"),
    ("rtf", "pref.termCopy.rtf"),
    ("both", "pref.termCopy.both"),
];
const VIEW_MODE_OPTS: &[(&str, &str)] = &[
    ("tree", "pref.viewMode.tree"),
    ("flat", "pref.viewMode.flat"),
    ("tiles", "pref.viewMode.tiles"),
];
const PANEL_MODE_OPTS: &[(&str, &str)] = &[
    ("single", "pref.panelMode.single"),
    ("dual", "pref.panelMode.dual"),
];

// ── 카테고리 i18n 키(dir2 PREFS-307 사이드바)
const CAT_APPEARANCE: &str = "pref.cat.appearance";
const CAT_FONTS: &str = "pref.cat.fonts";
const CAT_LANG: &str = "pref.cat.lang";
const CAT_LIST: &str = "pref.cat.listGeneral";
const CAT_TYPEAHEAD: &str = "pref.cat.typeahead";
const CAT_SCROLL: &str = "pref.cat.scroll";
const CAT_CTXMENU: &str = "pref.cat.ctxmenu";
const CAT_TRANSFER: &str = "pref.cat.transfer";
const CAT_TABS: &str = "pref.cat.tabs";
const CAT_DOCK: &str = "pref.cat.dock";
const CAT_TERMINAL: &str = "pref.cat.terminal";
const CAT_PLUGINS: &str = "pref.cat.plugins";
const CAT_CLOUD: &str = "pref.cat.cloud";
const CAT_KEYS: &str = "pref.cat.keys";
const KEY_PRESET_OPTS: &[(&str, &str)] = &[
    ("auto", "pref.keyPreset.auto"),
    ("windows", "pref.keyPreset.windows"),
    ("macos", "pref.keyPreset.macos"),
    ("linux", "pref.keyPreset.linux"),
];

/// ★ 설정 트리(dir2 `prefs.rs` TREE 순서) — (그룹 i18n 키, [카테고리 i18n 키]). 그룹 = 카테고리 1개면 설정 창이 최상위 잎으로 그린다
/// (dir2의 `tabs`·`plugins` 최상위 항목 재현).
pub const CATEGORY_TREE: &[(&str, &[&str])] = &[
    (
        "pref.grp.general",
        &[CAT_APPEARANCE, CAT_FONTS, CAT_LANG, CAT_KEYS],
    ),
    (
        "pref.cat.list",
        &[
            CAT_LIST,
            CAT_TYPEAHEAD,
            CAT_SCROLL,
            CAT_CTXMENU,
            CAT_TRANSFER,
        ],
    ),
    (CAT_TABS, &[CAT_TABS]),
    ("pref.grp.panel", &[CAT_DOCK, CAT_TERMINAL]),
    (CAT_PLUGINS, &[CAT_PLUGINS]),
    // dir2에는 페이지가 없던 항목(파일 직접 편집) — Advanced를 켜야 보인다.
    ("pref.grp.cloud", &[CAT_CLOUD]),
];

macro_rules! e {
    ($key:expr, $cat:expr, $label:expr, $desc:expr, $kind:expr, $default:expr) => {
        Entry {
            key: $key,
            cat: $cat,
            label: $label,
            desc: $desc,
            kind: $kind,
            default: $default,
        }
    };
}
use SettingKind::{Bool, Choice, Int, Position, Size, Text};

/// ★ 레지스트리. 종속 자식은 부모 **뒤**에 둔다(시험이 검사).
pub const REGISTRY: &[Entry] = &[
    // ── 일반 › 모양(PREFS-310 ①②) + 메뉴/툴바로만 바뀌는 값(HIDDEN)
    e!(
        "toolbar.layout",
        CAT_APPEARANCE,
        "pref.toolbarOrder",
        "pref.toolbarOrder.desc",
        Text,
        ""
    ),
    e!(
        "ui.theme",
        CAT_APPEARANCE,
        "pref.theme",
        "pref.theme.desc",
        Choice(THEME_OPTS),
        "system"
    ),
    e!(
        "window.always_on_top",
        CAT_APPEARANCE,
        "pref.alwaysOnTop",
        "pref.alwaysOnTop.desc",
        Bool,
        "off"
    ),
    e!(
        "layout.panel_mode",
        CAT_APPEARANCE,
        "pref.panelMode",
        "pref.panelMode.desc",
        Choice(PANEL_MODE_OPTS),
        "dual"
    ),
    e!(
        "layout.info_mode",
        CAT_APPEARANCE,
        "pref.infoMode",
        "pref.infoMode.desc",
        Choice(PANEL_MODE_OPTS),
        "dual"
    ),
    e!(
        "layout.panel_split_pct",
        CAT_APPEARANCE,
        "pref.panelSplit",
        "pref.panelSplit.desc",
        Int { min: 10, max: 90 },
        "50"
    ),
    e!(
        "launcher.visible",
        CAT_APPEARANCE,
        "pref.launcherVisible",
        "pref.launcherVisible.desc",
        Bool,
        "on"
    ),
    // 퀵 런처 바 크기/간격(dir3 신규 · 사용자 10-03 "퀵 런처 바도 설정으로") — 크기 = 일반 · 간격 = 고급.
    e!(
        "launcher.icon_size",
        CAT_APPEARANCE,
        "pref.launcherSize",
        "pref.launcherSize.desc",
        Choice(TOOLBAR_SIZE_OPTS),
        "20"
    ),
    e!(
        "launcher.item_gap",
        CAT_APPEARANCE,
        "pref.launcherItemGap",
        "pref.launcherItemGap.desc",
        Int { min: 0, max: 16 },
        "4"
    ),
    e!(
        "launcher.seed",
        CAT_APPEARANCE,
        "pref.launcherSeed",
        "pref.launcherSeed.desc",
        Int { min: 0, max: 9999 },
        "0"
    ),
    e!(
        "launcher.items",
        CAT_APPEARANCE,
        "pref.launcherItems",
        "pref.launcherItems.desc",
        Text,
        ""
    ),
    e!(
        "ui.prefs_advanced",
        CAT_APPEARANCE,
        "pref.prefsAdvanced",
        "pref.prefsAdvanced.desc",
        Bool,
        "off"
    ),
    // 상단 툴바(dir3 신규 · 사용자 10-03): 아이콘 크기 = 일반 설정 · 간격 3종 = 고급 · 그룹 배치(끌어서 옮긴 순서·행) = HIDDEN.
    e!(
        "toolbar.icon_size",
        CAT_APPEARANCE,
        "pref.toolbarSize",
        "pref.toolbarSize.desc",
        Choice(TOOLBAR_SIZE_OPTS),
        "20"
    ),
    e!(
        "toolbar.icon_pad",
        CAT_APPEARANCE,
        "pref.toolbarIconPad",
        "pref.toolbarIconPad.desc",
        Int { min: 0, max: 8 },
        "1"
    ),
    e!(
        "toolbar.hover_fill_pct",
        CAT_APPEARANCE,
        "pref.toolbarHoverFill",
        "pref.toolbarHoverFill.desc",
        Int { min: 0, max: 100 },
        "8"
    ),
    // 켜짐 표시(사용자 10-04 "배경은 원복하고 선만" → 색 "#0000FF"): line = 강조색 옅은 채움 + **선(테두리 · 아이콘) =
    // `toolbar.on_line_color`**(기본) · accent = 종전(테두리 · 아이콘도 강조색 규칙 — 테두리 농도 · 아이콘 강조색 설정이 이때만).
    e!(
        "toolbar.on_color",
        CAT_APPEARANCE,
        "pref.toolbarOnColor",
        "pref.toolbarOnColor.desc",
        Choice(TOOLBAR_ON_COLOR_OPTS),
        "line"
    ),
    e!(
        "toolbar.on_line_color",
        CAT_APPEARANCE,
        "pref.toolbarOnLineColor",
        "pref.toolbarOnLineColor.desc",
        Text,
        "#0000FF"
    ),
    e!(
        "toolbar.on_fill_pct",
        CAT_APPEARANCE,
        "pref.toolbarOnFill",
        "pref.toolbarOnFill.desc",
        Int { min: 0, max: 100 },
        "26"
    ),
    e!(
        "toolbar.on_line_pct",
        CAT_APPEARANCE,
        "pref.toolbarOnLine",
        "pref.toolbarOnLine.desc",
        Int { min: 0, max: 100 },
        "12"
    ),
    e!(
        "toolbar.state_step_pct",
        CAT_APPEARANCE,
        "pref.toolbarStateStep",
        "pref.toolbarStateStep.desc",
        Int { min: 0, max: 100 },
        "20"
    ),
    e!(
        "toolbar.state_radius",
        CAT_APPEARANCE,
        "pref.toolbarStateRadius",
        "pref.toolbarStateRadius.desc",
        Int { min: 0, max: 12 },
        "4"
    ),
    e!(
        "toolbar.on_icon_accent",
        CAT_APPEARANCE,
        "pref.toolbarOnIconAccent",
        "pref.toolbarOnIconAccent.desc",
        Bool,
        "off"
    ),
    e!(
        "toolbar.item_gap",
        CAT_APPEARANCE,
        "pref.toolbarItemGap",
        "pref.toolbarItemGap.desc",
        Int { min: 0, max: 16 },
        "0"
    ),
    e!(
        "toolbar.group_gap",
        CAT_APPEARANCE,
        "pref.toolbarGroupGap",
        "pref.toolbarGroupGap.desc",
        Int { min: 0, max: 32 },
        "4"
    ),
    e!(
        "toolbar.row_gap",
        CAT_APPEARANCE,
        "pref.toolbarRowGap",
        "pref.toolbarRowGap.desc",
        Int { min: 0, max: 16 },
        "0"
    ),
    e!(
        "toolbar.dock_layout",
        CAT_APPEARANCE,
        "pref.toolbarDockLayout",
        "pref.toolbarDockLayout.desc",
        Text,
        ""
    ),
    e!(
        "ui.dblclick_ms",
        CAT_APPEARANCE,
        "pref.dblclickMs",
        "pref.dblclickMs.desc",
        Int {
            min: 100,
            max: 2000
        },
        "400"
    ),
    e!(
        "input.scroll_natural",
        CAT_APPEARANCE,
        "pref.scrollNatural",
        "pref.scrollNatural.desc",
        Bool,
        "off"
    ),
    // 직접 그리는 글자 래스터(nexa-gfx · nexa-sql 42~44차) — Windows는 GDI ClearType 흉내 · macOS는 CoreText · Linux는 내장(OS_DEFAULTS).
    e!(
        "ui.text_gdi",
        CAT_APPEARANCE,
        "pref.textGdi",
        "pref.textGdi.desc",
        Bool,
        "on"
    ),
    e!(
        "ui.text_hint",
        CAT_APPEARANCE,
        "pref.textHint",
        "pref.textHint.desc",
        Bool,
        "on"
    ),
    e!(
        "ui.text_snap",
        CAT_APPEARANCE,
        "pref.textSnap",
        "pref.textSnap.desc",
        Bool,
        "on"
    ),
    e!(
        "ui.text_weight",
        CAT_APPEARANCE,
        "pref.textWeight",
        "pref.textWeight.desc",
        Int { min: 0, max: 100 },
        "25"
    ),
    e!(
        "ui.text_contrast",
        CAT_APPEARANCE,
        "pref.textContrast",
        "pref.textContrast.desc",
        Int { min: 50, max: 300 },
        "140"
    ),
    e!(
        "window.main_size",
        CAT_APPEARANCE,
        "pref.windowMainSize",
        "",
        Text,
        ""
    ),
    e!(
        "window.main_pos",
        CAT_APPEARANCE,
        "pref.windowMainPos",
        "",
        Text,
        ""
    ),
    e!(
        "window.prefs_size",
        CAT_APPEARANCE,
        "pref.windowPrefsSize",
        "",
        Text,
        ""
    ),
    e!(
        "window.prefs_pos",
        CAT_APPEARANCE,
        "pref.windowPrefsPos",
        "",
        Text,
        ""
    ),
    e!(
        "license.gates",
        CAT_APPEARANCE,
        "pref.licenseGates",
        "pref.licenseGates.desc",
        Bool,
        "off"
    ),
    // ── 일반 › 글꼴(PREFS-310 ①~⑨) — dir2 기본 `Segoe UI`/`Consolas`는 Windows 전용 → 빈 값 = OS 기본 UI 글꼴(OS_DEFAULTS로 터미널만 지정)
    e!(
        "ui.font_face",
        CAT_FONTS,
        "pref.baseFont",
        "pref.baseFont.desc",
        Text,
        ""
    ),
    e!(
        "ui.font_size",
        CAT_FONTS,
        "pref.baseFontSize",
        "pref.baseFontSize.desc",
        Size { min: 8, max: 32 },
        "12"
    ),
    e!(
        "term.font_face",
        CAT_FONTS,
        "pref.termFont",
        "pref.consoleFont.desc",
        Text,
        "Consolas"
    ),
    e!(
        "term.font_size",
        CAT_FONTS,
        "pref.termFontSize",
        "pref.termFontSize.desc",
        Size { min: 8, max: 32 },
        "12"
    ),
    e!(
        "ui.menu_font_face",
        CAT_FONTS,
        "pref.ctxFont",
        "pref.ctxFont.desc",
        Text,
        ""
    ),
    e!(
        "ui.menu_font_size",
        CAT_FONTS,
        "pref.ctxFontSize",
        "pref.ctxFontSize.desc",
        Size { min: 8, max: 32 },
        "12"
    ),
    e!(
        "statusbar.font_face",
        CAT_FONTS,
        "pref.statusFont",
        "pref.statusFont.desc",
        Text,
        ""
    ),
    e!(
        "statusbar.font_size",
        CAT_FONTS,
        "pref.statusFontSize",
        "pref.statusFontSize.desc",
        Size { min: 8, max: 32 },
        "12"
    ),
    // 상태줄 구성(dir3 신규 · docs/22 NEW-003 · NEW-004): 오른쪽 칸 순서 · 부하 조회 주기(고급) · 패널마다 탭 상태바.
    e!(
        "statusbar.items",
        CAT_APPEARANCE,
        "pref.statusItems",
        "pref.statusItems.desc",
        Text,
        "tab,cpu,mem,io,license"
    ),
    e!(
        "statusbar.load_interval_ms",
        CAT_APPEARANCE,
        "pref.statusInterval",
        "pref.statusInterval.desc",
        Int {
            min: 500,
            max: 60000
        },
        "2000"
    ),
    e!(
        "layout.tab_statusbar",
        CAT_APPEARANCE,
        "pref.tabStatusbar",
        "pref.tabStatusbar.desc",
        Bool,
        "on"
    ),
    e!(
        "list.font_face",
        CAT_FONTS,
        "pref.listFont",
        "pref.listFont.desc",
        Text,
        ""
    ),
    e!(
        "list.font_size",
        CAT_FONTS,
        "pref.listFontSize",
        "pref.listFontSize.desc",
        Size { min: 8, max: 32 },
        "12"
    ),
    e!(
        "list.folder_bold",
        CAT_FONTS,
        "pref.folderBold",
        "pref.folderBold.desc",
        Bool,
        "off"
    ),
    e!(
        "list.header_bold",
        CAT_FONTS,
        "pref.hdrBold",
        "pref.hdrBold.desc",
        Bool,
        "off"
    ),
    e!(
        "list.header_italic",
        CAT_FONTS,
        "pref.hdrItalic",
        "pref.hdrItalic.desc",
        Bool,
        "off"
    ),
    e!(
        "ui.dialog_font_face",
        CAT_FONTS,
        "pref.dlgFont",
        "pref.dlgFont.desc",
        Text,
        ""
    ),
    e!(
        "ui.dialog_font_size",
        CAT_FONTS,
        "pref.dlgFontSize",
        "pref.dlgFontSize.desc",
        Size { min: 9, max: 32 },
        "9pt"
    ),
    // ── 일반 › 언어 — `system` 또는 코드(발견 목록 = 설정 창 동적 후보 · 해석 = ndir_i18n::resolve_code)
    e!(
        "ui.lang",
        CAT_LANG,
        "pref.lang",
        "pref.lang.desc",
        Text,
        "system"
    ),
    // ── 파일 목록 › 보기·정렬(PREFS-310 ①~⑨)
    e!(
        "list.show_hidden",
        CAT_LIST,
        "pref.showHidden",
        "pref.showHidden.desc",
        Bool,
        "on"
    ),
    e!(
        "list.show_dotfiles",
        CAT_LIST,
        "pref.showDotfiles",
        "pref.showDotfiles.desc",
        Bool,
        "on"
    ),
    // 보호된 운영 체제 항목(Windows 숨김+시스템 · macOS 숨김+SIP 보호 · Linux 해당 없음) — 기본 = 숨김(탐색기 권장값 · 사용자 10-03).
    e!(
        "list.show_protected",
        CAT_LIST,
        "pref.showProtected",
        "pref.showProtected.desc",
        Bool,
        "off"
    ),
    e!(
        "list.col_autofit_max",
        CAT_LIST,
        "pref.colAutofitMax",
        "pref.colAutofitMax.desc",
        Int { min: 50, max: 2000 },
        "400"
    ),
    e!(
        "list.folders_first",
        CAT_LIST,
        "pref.sortFoldersFirst",
        "pref.sortFoldersFirst.desc",
        Bool,
        "on"
    ),
    e!(
        "list.hide_empty_glyph",
        CAT_LIST,
        "pref.hideEmptyGlyph",
        "pref.hideEmptyGlyph.desc",
        Bool,
        "on"
    ),
    e!(
        "list.sort_case_sensitive",
        CAT_LIST,
        "pref.sortCaseSensitive",
        "pref.sortCaseSensitive.desc",
        Bool,
        "off"
    ),
    e!(
        "list.view_scope",
        CAT_LIST,
        "pref.viewScope",
        "pref.viewScope.desc",
        Choice(VIEW_SCOPE_OPTS),
        "dir"
    ),
    e!(
        "list.nav_up_align",
        CAT_LIST,
        "pref.navUpAlign",
        "pref.navUpAlign.desc",
        Choice(ALIGN_OPTS),
        "center"
    ),
    e!(
        "list.view_mode",
        CAT_LIST,
        "pref.viewMode",
        "pref.viewMode.desc",
        Choice(VIEW_MODE_OPTS),
        "tree"
    ),
    e!(
        "list.col_width_sync",
        CAT_LIST,
        "pref.colWidthSync",
        "pref.colWidthSync.desc",
        Bool,
        "on"
    ),
    // 파일 컬럼 순서/표시(T-71 DLG-069 — 별도 편집 창 · 값 문법 = `cols:1[name:1,…]` · 세션이 패널별 실제 값을 따로 둔다).
    e!(
        "list.col_layout",
        CAT_LIST,
        "pref.colLayout",
        "pref.colLayout.desc",
        Text,
        ""
    ),
    // 행 아이콘 계층 1 — 직접 설정한 아이콘(dir3 신규 · 사용자 10-03): `종류:패턴=이미지` · `;` 구분 · 종류 = path|name|dir|ext.
    e!(
        "list.icon_overrides",
        CAT_LIST,
        "pref.iconOverrides",
        "pref.iconOverrides.desc",
        Text,
        ""
    ),
    // ── 파일 목록 › 타입어헤드(①~⑥)
    e!(
        "typeahead.scope",
        CAT_TYPEAHEAD,
        "pref.taScope",
        "pref.taScope.desc",
        Choice(TA_SCOPE_OPTS),
        "visible"
    ),
    e!(
        "typeahead.reset_ms",
        CAT_TYPEAHEAD,
        "pref.taReset",
        "pref.taReset.desc",
        Int {
            min: 200,
            max: 10000
        },
        "1000"
    ),
    e!(
        "typeahead.special",
        CAT_TYPEAHEAD,
        "pref.taSpecial",
        "pref.taSpecial.desc",
        Bool,
        "on"
    ),
    e!(
        "typeahead.space",
        CAT_TYPEAHEAD,
        "pref.taSpace",
        "pref.taSpace.desc",
        Bool,
        "on"
    ),
    e!(
        "typeahead.backspace",
        CAT_TYPEAHEAD,
        "pref.taBackspace",
        "pref.taBackspace.desc",
        Bool,
        "on"
    ),
    e!(
        "typeahead.hud_pos",
        CAT_TYPEAHEAD,
        "pref.taPos",
        "pref.taPos.desc",
        Position,
        "bottom_left"
    ),
    // ── 파일 목록 › 고속 스크롤(①~⑨ · 부모 = scroll.fast · HUD 3항목 부모 = scroll.fast_hud)
    e!(
        "scroll.fast",
        CAT_SCROLL,
        "pref.fsEnabled",
        "pref.fsEnabled.desc",
        Bool,
        "on"
    ),
    e!(
        "scroll.fast_grid_extra",
        CAT_SCROLL,
        "pref.fsGridExtra",
        "pref.fsGridExtra.desc",
        Bool,
        "on"
    ),
    e!(
        "scroll.fast_step",
        CAT_SCROLL,
        "pref.fsStep",
        "pref.fsStep.desc",
        Int { min: 1, max: 50 },
        "3"
    ),
    e!(
        "scroll.fast_max",
        CAT_SCROLL,
        "pref.fsMax",
        "pref.fsMax.desc",
        Int { min: 1, max: 32 },
        "16"
    ),
    e!(
        "scroll.fast_window_ms",
        CAT_SCROLL,
        "pref.fsWindow",
        "pref.fsWindow.desc",
        Int { min: 20, max: 2000 },
        "160"
    ),
    e!(
        "scroll.fast_hud",
        CAT_SCROLL,
        "pref.fsHud",
        "pref.fsHud.desc",
        Bool,
        "on"
    ),
    e!(
        "scroll.fast_hud_pos",
        CAT_SCROLL,
        "pref.fsHudPos",
        "pref.fsHudPos.desc",
        Position,
        "top_right"
    ),
    e!(
        "scroll.fast_hud_hold_ms",
        CAT_SCROLL,
        "pref.fsHudHold",
        "pref.fsHudHold.desc",
        Int { min: 0, max: 10000 },
        "250"
    ),
    e!(
        "scroll.fast_hud_fade_ms",
        CAT_SCROLL,
        "pref.fsHudFade",
        "pref.fsHudFade.desc",
        Int { min: 0, max: 10000 },
        "600"
    ),
    // ── 파일 목록 › 컨텍스트 메뉴
    e!(
        "ctxmenu.layout",
        CAT_CTXMENU,
        "pref.ctxMenuOrder",
        "pref.ctxMenuOrder.desc",
        Text,
        ""
    ),
    // ── 파일 목록 › 파일 전송
    e!(
        "transfer.close_ms",
        CAT_TRANSFER,
        "pref.transferClose",
        "pref.transferClose.desc",
        Int { min: 0, max: 10000 },
        "2000"
    ),
    e!(
        "transfer.dnd_hover_ms",
        CAT_TRANSFER,
        "pref.dndHover",
        "pref.dndHover.desc",
        Int {
            min: 200,
            max: 10000
        },
        "3000"
    ),
    // ── 탭
    e!(
        "tabs.dblclick",
        CAT_TABS,
        "pref.tabDblclick",
        "pref.tabDblclick.desc",
        Choice(TAB_DBL_OPTS),
        "close"
    ),
    // 탭 여러 줄(dir3 · 사용자 10-03 "탭 구성은 Multi-line을 기본 · Single-line은 옵션"): 켜면 폭을 넘는 탭이 다음 줄로(dir2는 늘
    // 여러 줄) · 끄면 한 줄 + ◀ ▶ 스크롤.
    e!(
        "tabs.multiline",
        CAT_TABS,
        "pref.tabMultiline",
        "pref.tabMultiline.desc",
        Bool,
        "on"
    ),
    // 한 줄일 때 ◀ ▶ 버튼 자리: 오른쪽 끝(기본) · 왼쪽 끝 · 양 끝(◀ 왼쪽 · ▶ 오른쪽).
    e!(
        "tabs.scroll_buttons",
        CAT_TABS,
        "pref.tabScrollButtons",
        "pref.tabScrollButtons.desc",
        Choice(TAB_SCROLL_BTN_OPTS),
        "end"
    ),
    // ── 하단 도크 › 하단 도크
    e!(
        "dock.visible",
        CAT_DOCK,
        "pref.dock",
        "pref.dock.desc",
        Bool,
        "on"
    ),
    e!(
        "layout.dock_height_pct",
        CAT_DOCK,
        "pref.dockHeight",
        "pref.dockHeight.desc",
        Int { min: 15, max: 50 },
        "30"
    ),
    e!(
        "layout.dock_split_pct",
        CAT_DOCK,
        "pref.dockSplit",
        "pref.dockSplit.desc",
        Int { min: 15, max: 85 },
        "50"
    ),
    // ── 하단 도크 › 터미널(①~⑥ + 셸 신설 KEY-591)
    e!(
        "term.wrap",
        CAT_TERMINAL,
        "pref.termWrap",
        "pref.termWrap.desc",
        Bool,
        "off"
    ),
    e!(
        "term.cols",
        CAT_TERMINAL,
        "pref.termCols",
        "pref.termCols.desc",
        Int { min: 80, max: 1000 },
        "240"
    ),
    e!(
        "term.theme",
        CAT_TERMINAL,
        "pref.termTheme",
        "pref.termTheme.desc",
        Text,
        "system"
    ),
    e!(
        "term.theme_dark",
        CAT_TERMINAL,
        "pref.termThemeDark",
        "pref.termThemeDark.desc",
        Text,
        "campbell"
    ),
    e!(
        "term.theme_light",
        CAT_TERMINAL,
        "pref.termThemeLight",
        "pref.termThemeLight.desc",
        Text,
        "github-light"
    ),
    e!(
        "term.copy_format",
        CAT_TERMINAL,
        "pref.termCopy",
        "pref.termCopy.desc",
        Choice(TERM_COPY_OPTS),
        "text"
    ),
    // Windows Terminal 따라가기(dir3 신규 · DR-21): 켜면 그 기본 프로필의 글꼴 목록·크기를 터미널 도크에 쓴다(Windows · 없으면 무시).
    e!(
        "term.follow_windows_terminal",
        CAT_TERMINAL,
        "pref.termFollowWt",
        "pref.termFollowWt.desc",
        Bool,
        "on"
    ),
    // 터미널 폴백 글꼴(dir3 신규 · 고급): 주 글꼴에 없는 글자(프롬프트 아이콘 등)를 그릴 글꼴 · 쉼표 구분 · 비우면 설치된 Nerd Font 자동.
    e!(
        "term.fallback_fonts",
        CAT_TERMINAL,
        "pref.termFallbackFonts",
        "pref.termFallbackFonts.desc",
        Text,
        ""
    ),
    e!(
        "term.shell",
        CAT_TERMINAL,
        "pref.termShell",
        "pref.termShell.desc",
        Text,
        ""
    ),
    // ── 플러그인(페이지는 동적 체크 목록 — 값은 이 두 키)
    e!(
        "plugins.disabled",
        CAT_PLUGINS,
        "pref.pluginsDisabled",
        "pref.pluginsDisabled.desc",
        Text,
        ""
    ),
    e!(
        "preview.map",
        CAT_PLUGINS,
        "pref.previewMap",
        "pref.previewMap.desc",
        Text,
        ""
    ),
    // ── 단축키(nexa-sql 구조 · SET-130 교훈 = 전 명령 `key.<id>` 등재 — 시험이 명령 표와 1:1 대조)
    e!(
        "key.preset",
        CAT_KEYS,
        "pref.keyPreset",
        "pref.keyPreset.desc",
        Choice(KEY_PRESET_OPTS),
        "auto"
    ),
    e!(
        "key.file.new_tab",
        CAT_KEYS,
        "menu.file.newTab",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.file.close_tab",
        CAT_KEYS,
        "menu.file.closeTab",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.file.new_folder",
        CAT_KEYS,
        "menu.file.newFolder",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.file.new_file",
        CAT_KEYS,
        "menu.file.newFile",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.file.prefs",
        CAT_KEYS,
        "menu.file.prefs",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.file.exit",
        CAT_KEYS,
        "menu.file.exit",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.edit.undo",
        CAT_KEYS,
        "menu.edit.undo",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.edit.redo",
        CAT_KEYS,
        "menu.edit.redo",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.edit.cut",
        CAT_KEYS,
        "menu.edit.cut",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.edit.copy",
        CAT_KEYS,
        "menu.edit.copy",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.edit.paste",
        CAT_KEYS,
        "menu.edit.paste",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.edit.select_all",
        CAT_KEYS,
        "menu.edit.selectAll",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.edit.bulk_rename",
        CAT_KEYS,
        "menu.edit.bulkRename",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.edit.rename",
        CAT_KEYS,
        "cmd.rename",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.edit.delete",
        CAT_KEYS,
        "menu.edit.delete",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.edit.delete_permanent",
        CAT_KEYS,
        "ctx.deletePermanent",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.mode_tree",
        CAT_KEYS,
        "menu.view.modeTree",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.mode_flat",
        CAT_KEYS,
        "menu.view.modeFlat",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.mode_tiles",
        CAT_KEYS,
        "menu.view.modeTiles",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.panel_dual",
        CAT_KEYS,
        "menu.view.panelDual",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.panel_single",
        CAT_KEYS,
        "menu.view.panelSingle",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.panel_toggle",
        CAT_KEYS,
        "cmd.panelToggle",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.info_dual",
        CAT_KEYS,
        "menu.view.infoDual",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.info_single",
        CAT_KEYS,
        "menu.view.infoSingle",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.info_toggle",
        CAT_KEYS,
        "cmd.infoToggle",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.col_width_sync",
        CAT_KEYS,
        "menu.view.colWidthSync",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.hidden",
        CAT_KEYS,
        "menu.view.hidden",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.dot",
        CAT_KEYS,
        "menu.view.dot",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.folders_first",
        CAT_KEYS,
        "pref.sortFoldersFirst",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.case_sensitive",
        CAT_KEYS,
        "pref.sortCaseSensitive",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.dock",
        CAT_KEYS,
        "menu.view.dock",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.launcher",
        CAT_KEYS,
        "menu.view.launcher",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.always_on_top",
        CAT_KEYS,
        "menu.view.alwaysOnTop",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.refresh",
        CAT_KEYS,
        "menu.view.refresh",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.theme_system",
        CAT_KEYS,
        "menu.view.theme.system",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.theme_light",
        CAT_KEYS,
        "menu.view.theme.light",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.theme_dark",
        CAT_KEYS,
        "menu.view.theme.dark",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.theme_cycle",
        CAT_KEYS,
        "cmd.themeCycle",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.lang_system",
        CAT_KEYS,
        "menu.view.lang.system",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.view.preview_window",
        CAT_KEYS,
        "cmd.previewWindow",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.nav.back",
        CAT_KEYS,
        "cmd.navBack",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.nav.forward",
        CAT_KEYS,
        "cmd.navForward",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.nav.up",
        CAT_KEYS,
        "cmd.navUp",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.nav.activate",
        CAT_KEYS,
        "cmd.activate",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.panel.switch",
        CAT_KEYS,
        "cmd.panelSwitch",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.tab.next",
        CAT_KEYS,
        "cmd.tabNext",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.tab.prev",
        CAT_KEYS,
        "cmd.tabPrev",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.list.context_menu",
        CAT_KEYS,
        "cmd.contextMenu",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.help.about",
        CAT_KEYS,
        "menu.help.about",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.help.license",
        CAT_KEYS,
        "menu.help.license",
        "pref.key.desc",
        Text,
        ""
    ),
    e!(
        "key.help.selfcheck",
        CAT_KEYS,
        "menu.help.selfcheck",
        "pref.key.desc",
        Text,
        ""
    ),
    // ── 클라우드(dir2 동적 키 군 → 고정 키 · 여러 줄 값)
    e!(
        "cloud.conns",
        CAT_CLOUD,
        "pref.cloudConns",
        "pref.cloudConns.desc",
        Text,
        ""
    ),
    e!(
        "cloud.client_id_onedrive",
        CAT_CLOUD,
        "pref.cloudIdOnedrive",
        "pref.cloudId.desc",
        Text,
        ""
    ),
    e!(
        "cloud.client_id_googledrive",
        CAT_CLOUD,
        "pref.cloudIdGoogle",
        "pref.cloudId.desc",
        Text,
        ""
    ),
    e!(
        "cloud.client_id_dropbox",
        CAT_CLOUD,
        "pref.cloudIdDropbox",
        "pref.cloudId.desc",
        Text,
        ""
    ),
    e!(
        "cloud.client_secret_onedrive",
        CAT_CLOUD,
        "pref.cloudSecretOnedrive",
        "pref.cloudSecret.desc",
        Text,
        ""
    ),
    e!(
        "cloud.client_secret_googledrive",
        CAT_CLOUD,
        "pref.cloudSecretGoogle",
        "pref.cloudSecret.desc",
        Text,
        ""
    ),
    e!(
        "cloud.client_secret_dropbox",
        CAT_CLOUD,
        "pref.cloudSecretDropbox",
        "pref.cloudSecret.desc",
        Text,
        ""
    ),
];

/// ★ OS별 기본값 — (키, macOS, Linux). Windows = 레지스트리 `default`.
/// 텍스트 래스터 5키 = nexa-sql 실측(macOS는 CoreText · 힌트 없음 · Linux는 FreeType 관례) · 터미널 글꼴 = OS 고정폭 기본.
pub const OS_DEFAULTS: &[(&str, &str, &str)] = &[
    ("ui.text_hint", "off", "on"),
    ("ui.text_snap", "off", "on"),
    ("ui.text_weight", "0", "25"),
    ("ui.text_contrast", "100", "140"),
    ("ui.text_gdi", "on", "off"),
    ("term.font_face", "Menlo", "DejaVu Sans Mono"),
];

/// (자식, 부모, 조건) — 부모가 조건을 만족하지 않으면 자식은 설정 화면에서 잠긴다(값은 유지). dir2 PREFS-144~152 · KEY-515.
pub const DEPENDS: &[(&str, &str, Dep)] = &[
    ("scroll.fast_grid_extra", "scroll.fast", Dep::On),
    ("scroll.fast_step", "scroll.fast", Dep::On),
    ("scroll.fast_max", "scroll.fast", Dep::On),
    ("scroll.fast_window_ms", "scroll.fast", Dep::On),
    ("scroll.fast_hud", "scroll.fast", Dep::On),
    ("scroll.fast_hud_pos", "scroll.fast_hud", Dep::On),
    ("scroll.fast_hud_hold_ms", "scroll.fast_hud", Dep::On),
    ("scroll.fast_hud_fade_ms", "scroll.fast_hud", Dep::On),
    ("term.cols", "term.wrap", Dep::Eq("off")),
    ("toolbar.on_line_color", "toolbar.on_color", Dep::Eq("line")),
    ("toolbar.on_line_pct", "toolbar.on_color", Dep::Eq("accent")),
    (
        "toolbar.on_icon_accent",
        "toolbar.on_color",
        Dep::Eq("accent"),
    ),
    // 배치(사용자 10-03 "상위를 끄면 하위는 설정 불가"): 단일 패널이면 좌우 분할·정보 배치가 무의미 · 도크를 숨기면 도크 크기가
    // 무의미 · 도크 좌우 분할은 도크 표시 ∧ 듀얼 패널 ∧ 듀얼 정보일 때만(여러 줄 = AND).
    ("layout.info_mode", "layout.panel_mode", Dep::Eq("dual")),
    (
        "layout.panel_split_pct",
        "layout.panel_mode",
        Dep::Eq("dual"),
    ),
    ("layout.dock_height_pct", "dock.visible", Dep::On),
    ("layout.dock_split_pct", "dock.visible", Dep::On),
    ("layout.dock_split_pct", "layout.info_mode", Dep::Eq("dual")),
    // 런처 바를 숨기면 크기·간격이 무의미.
    ("launcher.icon_size", "launcher.visible", Dep::On),
    ("launcher.item_gap", "launcher.visible", Dep::On),
    // 터미널 테마: system = 다크/라이트 스킴 둘 다 · dark/light = 그쪽만 · 스킴 id를 직접 적으면 둘 다 쓰이지 않는다.
    (
        "term.theme_dark",
        "term.theme",
        Dep::OneOf(&["system", "dark"]),
    ),
    (
        "term.theme_light",
        "term.theme",
        Dep::OneOf(&["system", "light"]),
    ),
    ("tabs.scroll_buttons", "tabs.multiline", Dep::Eq("off")),
];

/// **내부 전용**(사용자 10-03 "라이선스 게이트처럼 라이선스로 기능을 켜고 끄는 것은 보이면 안 된다") — 고급 토글을 켜도 설정 창에
/// 나오지 않고 · 검색에 안 걸리고 · JSON 내보내기/가져오기에도 없다. 코드(`get/flag`)만 읽는다.
pub const INTERNAL: &[&str] = &["license.gates"];

/// **Windows에만 뜻이 있는 설정**(사용자 10-03 "dot file은 윈도우에서만 표시") — Linux · macOS에서는 점으로 시작하는 이름이 곧
/// 숨김 파일이라 "점 파일 표시"가 따로 없다(`list.show_hidden`이 다룬다). 그 OS에서는 [`INTERNAL`]처럼 설정 창·검색·JSON에서 빠진다
/// (값은 남아 있어 같은 설정 파일을 Windows로 가져가면 그대로 쓴다).
pub const WINDOWS_ONLY: &[&str] = &["list.show_dotfiles", "key.view.dot"];

/// 비노출(자동 기억 값 · 메뉴/툴바로만 바뀌는 값 · 구현 상수) — `set/get/reset`은 되지만 설정 창에는 안 보인다.
pub const HIDDEN: &[&str] = &[
    "window.always_on_top",
    "layout.panel_mode",
    "layout.info_mode",
    "layout.panel_split_pct",
    "layout.dock_height_pct",
    "layout.dock_split_pct",
    "launcher.visible",
    "launcher.seed",
    "launcher.items",
    "ui.prefs_advanced",
    "toolbar.dock_layout",
    "ui.dblclick_ms",
    "window.main_size",
    "window.main_pos",
    "window.prefs_size",
    "window.prefs_pos",
    "license.gates",
    "list.view_mode",
    "list.col_width_sync",
    "plugins.disabled",
    "cloud.conns",
];

/// 고급(Advanced 토글을 켜야 보임) — HIDDEN은 자동 포함.
pub const ADVANCED: &[&str] = &[
    "statusbar.load_interval_ms",
    "toolbar.icon_pad",
    "toolbar.hover_fill_pct",
    "toolbar.on_fill_pct",
    "toolbar.on_line_pct",
    "toolbar.state_step_pct",
    "toolbar.state_radius",
    "toolbar.on_icon_accent",
    "toolbar.item_gap",
    "term.fallback_fonts",
    "launcher.item_gap",
    "toolbar.group_gap",
    "toolbar.row_gap",
    "input.scroll_natural",
    "ui.text_gdi",
    "ui.text_hint",
    "ui.text_snap",
    "ui.text_weight",
    "ui.text_contrast",
    "preview.map",
    "cloud.client_id_onedrive",
    "cloud.client_id_googledrive",
    "cloud.client_id_dropbox",
    "cloud.client_secret_onedrive",
    "cloud.client_secret_googledrive",
    "cloud.client_secret_dropbox",
];

/// 읽기 전용 정보 키(호스트가 채움 · 저장 안 함) — 지금은 없음. 후보: 설정 폴더 · 탐지된 셸 · 플러그인 폴더.
pub const INFO_KEYS: &[&str] = &[];

/// 플러그인이 소유한 설정 분류 ↔ 플러그인 id — 꺼진/미설치 플러그인의 분류를 설정 창이 숨긴다(지금은 없음).
pub const EXTENSION_CATEGORIES: &[(&str, &str)] = &[];

/// 기본값이 바뀐 키의 옛 기본값(파일에 옛 기본값이 그대로 있으면 새 기본값을 따른다) — 지금은 없음.
pub const OLD_DEFAULTS: &[(&str, &str)] = &[];

/// 키 이름 바꿈 표 `(옛 키, 새 키)` — dir3 안에서 바꾼 키만(dir2 → dir3 변환은 `migrate::import_dir2`).
pub const RENAMED: &[(&str, &str)] = &[];

/// 단위 변환 표 `(옛 키, 새 키, 배수)` — 지금은 없음.
pub const RESCALED: &[(&str, &str, i64)] = &[];
