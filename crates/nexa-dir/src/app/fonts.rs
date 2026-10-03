//! App — 글꼴 크기·얼굴 조립(사용자 피드백 10-03 "글이 작게 보임" · "터미널/정보가 전각처럼").
//!
//! - 설정 숫자(`*.font_size`)는 dir2와 같은 **em DIP**(GDI `lfHeight`)다. nexa-gfx의 크기는 어센트−디센트 **높이**라 같은 숫자면 ~25 % 작게
//!   보였다 → 글리프 크기만 [`nexa_gfx::Font::em_to_px`]로 변환하고(맑은 고딕 12 → 16 px), **배치 지표**(행 높이 · 메뉴 높이 = `size + n`)는
//!   dir2 수식 그대로 em 숫자를 쓴다(골든 불변).
//! - 고정폭 글꼴(`term.font_face` · 없으면 OS 기본 Consolas/Menlo/DejaVu Sans Mono)을 `FontSet.mono`에 넣는다 — 종전엔 Mono 슬롯이 가변폭
//!   UI 글꼴로 떨어져 터미널 셀 폭(= "M" 폭)이 넓어 글자가 전각처럼 벌어졌다.

use crate::*;
use nexa_ctl::raster::FontSet;
use nexa_ctl::theme::SlotFont;

/// UI 글꼴 체인에 아이콘 글꼴(Segoe MDL2 Assets / Fluent Icons — PUA U+E700~)이 있는가. 기동 때 [`init_icon_glyphs`]가 정한다.
static ICON_FONT: AtomicBool = AtomicBool::new(false);

pub(crate) fn icon_font_available() -> bool {
    ICON_FONT.load(Ordering::Relaxed)
}

/// dir2가 쓰는 MDL2 글리프(네비 4 + 쉐브론 2).
pub(crate) const MDL2_GLYPHS: [char; 6] = [
    '\u{EA8A}', '\u{E72B}', '\u{E72A}', '\u{E74A}', '\u{E76C}', '\u{E70D}',
];

/// 아이콘 글꼴이 없을 때의 대체 글리프(네비 4 + 쉐브론 2 · nexa-font `UI_SYMBOLS` 보장 대역).
pub(crate) const FALLBACK_GLYPHS: [char; 6] = [
    '\u{2302}', '\u{2190}', '\u{2192}', '\u{2191}', '\u{203A}', '\u{2304}',
];

/// 순수 판정: 이 글꼴(체인)이 dir2 MDL2 글리프 6개를 전부 가졌는가.
pub(crate) fn icon_font_covers(ui: &Font) -> bool {
    MDL2_GLYPHS.iter().all(|&c| ui.covers(c))
}

/// 글꼴이 정해진 직후 1회(패널을 만들기 **전에**): 아이콘 글꼴이 dir2 글리프 6개를 전부 가지면 그대로 쓰고(dir2와 같은 모양),
/// 하나라도 없으면 두부(□) 대신 유니코드로 — 네비 = ⌂←→↑ · 트리 쉐브론 = › ⌄(nexa-font `UI_SYMBOLS` 보장 대역).
pub(crate) fn init_icon_glyphs(ui: &Font) {
    let ok = icon_font_covers(ui);
    ICON_FONT.store(ok, Ordering::Relaxed);
    if ok {
        nexa_grid::set_marker_glyphs("\u{E76C}", "\u{E70D}");
    } else {
        nexa_grid::set_marker_glyphs("\u{203A}", "\u{2304}");
    }
}

impl App {
    /// 설정 키의 글리프 크기(px · em → 높이 변환 · 배율 전 논리 px).
    pub(crate) fn font_px(&self, key: &str) -> f32 {
        self.ui_font.em_to_px(self.settings.font_px(key))
    }

    /// 영역별 글꼴 설정(기본 = ui · 목록 = list · 대화 = ui.dialog · 상태/터미널 기준 = statusbar) — 전부 em → px.
    pub(crate) fn font_prefs(&self) -> FontPrefs {
        FontPrefs {
            base: SlotFont::plain(self.font_px("ui.font_size")),
            peerlist: SlotFont::plain(self.font_px("list.font_size")),
            message: SlotFont::plain(self.font_px("ui.dialog_font_size")),
            status: SlotFont::plain(self.font_px("statusbar.font_size")),
        }
    }

    /// 얼굴 묶음 — 기본 = UI 글꼴 · Mono = 고정폭(없으면 기본으로 폴백).
    pub(crate) fn font_set<'f>(&self, ui: &'f Font, mono: Option<&'f Font>) -> FontSet<'f> {
        FontSet {
            base: ui,
            peerlist: None,
            message: None,
            status: None,
            mono,
        }
    }

    /// 고정폭 글꼴 로드(`term.font_face` → 없으면 OS 기본) — 실패 = None(Mono 슬롯은 기본 얼굴로).
    pub(crate) fn load_mono_font(settings: &Settings) -> Option<Rc<Font>> {
        let face = settings
            .get("term.font_face")
            .map(str::trim)
            .filter(|f| !f.is_empty());
        nexa_font::mono_font(face)
            .or_else(|| nexa_font::mono_font(None))
            .map(|l| Rc::new(l.font))
    }
}
