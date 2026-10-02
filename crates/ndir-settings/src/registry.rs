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
const VIEW_SCOPE_OPTS: &[(&str, &str)] = &[
    ("global", "pref.viewScope.global"),
    ("panel", "pref.viewScope.panel"),
    ("tab", "pref.viewScope.tab"),
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

/// ★ 설정 트리(dir2 `prefs.rs` TREE 순서) — (그룹 i18n 키, [카테고리 i18n 키]). 그룹 = 카테고리 1개면 설정 창이 최상위 잎으로 그린다
/// (dir2의 `tabs`·`plugins` 최상위 항목 재현).
pub const CATEGORY_TREE: &[(&str, &[&str])] = &[
    ("pref.grp.general", &[CAT_APPEARANCE, CAT_FONTS, CAT_LANG]),
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
        "dark"
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
        "panel"
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
        "on"
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
];

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
