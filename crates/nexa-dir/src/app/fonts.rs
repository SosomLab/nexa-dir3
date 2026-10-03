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

/// 기본 UI 글꼴의 글리프 크기(px ×100 · `ui.font_size` em → px) — 패널이 네비 글리프 증분을 구할 때 쓴다(레이아웃 때 갱신).
static UI_FONT_PX100: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1600);

pub(crate) fn ui_font_px() -> f32 {
    UI_FONT_PX100.load(Ordering::Relaxed) as f32 / 100.0
}

pub(crate) fn set_ui_font_px(px: f32) {
    UI_FONT_PX100.store((px * 100.0).round().max(0.0) as u32, Ordering::Relaxed);
}

/// 상태 열의 기본 폭(논리 px) — 머리글("상태" · "Status" · "状態" …)이 잘리지 않는 폭. 언어·목록 글꼴이 정해질 때 호스트가
/// 잰다([`measure_status_col_w`]) · 열 정의(`all_columns_for`)가 읽는다.
static STATUS_COL_W: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(56);

pub(crate) fn status_col_w() -> i32 {
    STATUS_COL_W.load(Ordering::Relaxed)
}

/// 머리글 글자 폭(논리 px) → 상태 열 기본 폭(순수): 글자 + 좌우 여백 6×2 + 여유 6 · 아이콘(16)이 들어갈 최소 40.
pub(crate) fn status_col_w_for(text_w: f32) -> i32 {
    ((text_w.ceil() as i32) + 12 + 6).max(40)
}

/// 상태 열 기본 폭을 지금 언어의 머리글과 목록 글꼴(머리 굵게 포함)로 다시 잰다.
pub(crate) fn measure_status_col_w(font: &Font, settings: &Settings) -> i32 {
    let px = font.em_to_px(settings.font_px("list.font_size"));
    let w = font.measure_from_styled(
        &tr("col.status"),
        px,
        0.0,
        settings.flag("list.header_bold"),
    );
    let w = status_col_w_for(w);
    STATUS_COL_W.store(w, Ordering::Relaxed);
    w
}

/// dir2 아이콘 글꼴 크기(DirectWrite em · DIP): 네비 대형 13(사용자 확정 08-01 — 11은 식별 어려움 · 15는 과함) · 쉐브론 9
/// (nexa-dir2 `dw.rs:331-350`). 아이콘 글꼴은 em = 높이라 nexa-gfx 크기(px)로 그대로 쓴다.
pub(crate) const NAV_GLYPH_EM: f32 = 13.0;
pub(crate) const CHEVRON_EM: f32 = 9.0;

/// 네비 글리프 크기 증분(기본 UI 글꼴 px 대비 · 아이콘 글꼴이 없으면 `None` = 본문 크기 유니코드 글리프).
pub(crate) fn nav_glyph_delta(ui_px: f32) -> Option<f32> {
    icon_font_available().then_some(NAV_GLYPH_EM - ui_px)
}

/// dir2가 쓰는 MDL2 글리프(네비 4 + 쉐브론 2).
pub(crate) const MDL2_GLYPHS: [char; 6] = [
    '\u{EA8A}', '\u{E72B}', '\u{E72A}', '\u{E74A}', '\u{E76C}', '\u{E70D}',
];

/// 아이콘 글꼴이 없을 때의 대체 글리프(네비 4 + 쉐브론 2 · nexa-font `UI_SYMBOLS` 보장 대역).
pub(crate) const FALLBACK_GLYPHS: [char; 6] = [
    '\u{2302}', '\u{2190}', '\u{2192}', '\u{2191}', '\u{203A}', '\u{2304}',
];

/// 아이콘 글꼴이 없을 때(macOS · Linux)의 쉐브론 크기 증분(목록 글꼴 px 대비): 유니코드 쉐브론(› ⌄)은 글자 상자에 비해 그림이
/// 작아 본문보다 **키워야** 보인다(종전 −4 = Linux 실기 10-03 "쉐브론이 깨진다" — 점처럼 작게 그려졌다).
pub(crate) const FALLBACK_CHEVRON_DELTA: f32 = 3.0;

/// 대체 쉐브론 후보(접힘, 펼침) — 앞에서부터 **둘 다 그릴 수 있는** 첫 쌍(글꼴마다 가진 글자가 다르다 · 마지막 = ASCII라 항상 있다).
const FALLBACK_CHEVRONS: [(&str, &str); 4] = [
    ("\u{203A}", "\u{2304}"), // › ⌄
    ("\u{203A}", "\u{02C5}"), // › ˅
    ("\u{25B8}", "\u{25BE}"), // ▸ ▾
    (">", "v"),
];

/// 대체 쉐브론 고르기(순수 · `covers` = 그 글자를 그릴 수 있는가).
pub(crate) fn fallback_chevrons(covers: impl Fn(char) -> bool) -> (&'static str, &'static str) {
    FALLBACK_CHEVRONS
        .iter()
        .copied()
        .find(|(c, e)| c.chars().chain(e.chars()).all(&covers))
        .unwrap_or((">", "v"))
}

/// 순수 판정: 이 글꼴(체인)이 dir2 MDL2 글리프 6개를 전부 가졌는가.
pub(crate) fn icon_font_covers(ui: &Font) -> bool {
    MDL2_GLYPHS.iter().all(|&c| ui.covers(c))
}

/// 글꼴이 정해진 직후 1회(패널을 만들기 **전에**): 아이콘 글꼴이 dir2 글리프 6개를 전부 가지면 그대로 쓰고(dir2와 같은 모양),
/// 하나라도 없으면 두부(□) 대신 유니코드로 — 네비 = ⌂←→↑ · 트리 쉐브론 = › ⌄(nexa-font `UI_SYMBOLS` 보장 대역).
pub(crate) fn init_icon_glyphs(ui: &Font) {
    let ok = icon_font_covers(ui);
    ICON_FONT.store(ok, Ordering::Relaxed);
    // 쉐브론 크기(dir2 `mk_mdl2(9.0)` = 아이콘 글꼴 em 9): 목록 글꼴 px(기본 12 em → 16) 기준 증분. 아이콘 글꼴이 없으면 종전(−4).
    nexa_grid::draw::set_glyph_delta(if ok {
        CHEVRON_EM - ui.em_to_px(12.0)
    } else {
        FALLBACK_CHEVRON_DELTA
    });
    // 아이콘 글꼴이 없으면 쉐브론을 선으로 그린다(nexa-ui 123차 · Linux 실기 10-03 "윈도우와 다르게 작다") — 글꼴과 무관하게
    // MDL2 em 9와 같은 크기·모양. 아래 대체 글리프는 선 그리기를 끈 경우의 예비로 남긴다.
    nexa_grid::set_marker_vector(!ok);
    if ok {
        nexa_grid::set_marker_glyphs("\u{E76C}", "\u{E70D}");
    } else {
        let (c, e) = fallback_chevrons(|ch| ui.covers(ch));
        nexa_grid::set_marker_glyphs(c, e);
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
    pub(crate) fn load_mono_font(
        settings: &Settings,
        wt: Option<&platform::WtProfile>,
    ) -> Option<Rc<Font>> {
        let extra = settings.get("term.fallback_fonts").unwrap_or("");
        // Windows Terminal 따라가기(DR-21): 그 글꼴 목록의 첫 글꼴 = 주 글꼴 · 나머지 = 대체 글꼴(순서 그대로).
        if let Some(wt) = wt.filter(|w| !w.faces.is_empty()) {
            let rest = wt.faces[1..].join(",");
            let extra = if extra.trim().is_empty() {
                rest
            } else {
                format!("{rest},{extra}")
            };
            if let Some(f) = mono_chain(Some(&wt.faces[0]), &extra) {
                return Some(Rc::new(f));
            }
        }
        let face = settings
            .get("term.font_face")
            .map(str::trim)
            .filter(|f| !f.is_empty());
        mono_chain(face, extra).map(Rc::new)
    }

    /// 설정 `term.follow_windows_terminal`이 켜져 있으면 이 PC의 기본 터미널 글꼴(Windows = Windows Terminal 기본 프로필 ·
    /// Linux = 데스크톱 고정폭 글꼴 · 없으면 `None` = 설정 `term.font_face`/`term.font_size`).
    pub(crate) fn load_wt_profile(settings: &Settings) -> Option<platform::WtProfile> {
        settings
            .flag("term.follow_windows_terminal")
            .then(platform::system_terminal_profile)
            .flatten()
    }
}

/// 프롬프트 테마(oh-my-posh · starship · powerlevel)가 쓰는 아이콘 글리프(Nerd Fonts PUA)를 가진 글꼴 — 설치돼 있으면 자동으로
/// 터미널 폴백에 넣는다(사용자 10-03 "터미널 결과에 두부"). 앞에 있는 것이 우선.
const NERD_FAMILIES: [&str; 12] = [
    "Symbols Nerd Font Mono",
    "Symbols Nerd Font",
    "JetBrainsMonoNL Nerd Font",
    "JetBrainsMono Nerd Font",
    "CaskaydiaCove Nerd Font",
    "CaskaydiaMono Nerd Font",
    "MesloLGS NF",
    "MesloLGM Nerd Font",
    "FiraCode Nerd Font",
    "Hack Nerd Font",
    "D2CodingLigature Nerd Font",
    "DejaVuSansM Nerd Font",
];

/// Nerd Fonts 대표 글리프(Powerline 분기  · Font Awesome 폴더  · Devicons  · Material 󰊢 는 BMP 밖이라 제외).
pub(crate) const NERD_PROBE: [char; 3] = ['\u{E0A0}', '\u{F07B}', '\u{E0B0}'];

/// 터미널 고정폭 폴백(데스크톱 Linux에서 fontconfig가 터미널에 골라 주는 글꼴 — 앞이 우선): 기호(➜ ✗)를 한 칸 폭으로 가진
/// DejaVu Sans Mono(Linux 실기 10-03 — 넓은 기호 글꼴에서 오면 칸에서 잘린다).
const MONO_FALLBACK_FAMILIES: [&str; 1] = ["DejaVu Sans Mono"];
/// (파일 패밀리, 그 컬렉션 안의 얼굴) — fontconfig가 터미널 한글·한자에 골라 주는 고정폭판(`NotoSansCJK-*.ttc` 안에 일반판과
/// 함께 들어 있다). 한글 폭은 이 얼굴도 0.92 em이라 두 칸 안에서 조금 남는다(우분투 터미널과 같은 모양).
const MONO_CJK_FACE: (&str, &str) = ("Noto Sans CJK", "Noto Sans Mono CJK KR");

/// 터미널용 고정폭 글꼴 체인: **주 글꼴**(`term.font_face` → OS 고정폭) → **사용자 지정 폴백**(`term.fallback_fonts` · 쉼표) →
/// **설치된 Nerd Font**(주 글꼴이 그 글리프를 못 가질 때만) → 한글 UI 글꼴 → 기호 폴백. nexa-font `mono_font`와 같은 구성에 폴백 두 단계를
/// **기호 폴백 앞에** 끼운 것 — Nerd 아이콘 대역(U+E000~F8FF)은 Segoe MDL2/Fluent와 겹치므로 순서가 중요하다(뒤에 두면 엉뚱한 아이콘).
pub(crate) fn mono_chain(face: Option<&str>, extra: &str) -> Option<Font> {
    let primary = face
        .and_then(nexa_font::find_font_by_family)
        .or_else(|| nexa_font::system_mono_font().map(|f| (f.data, f.index)))
        .or_else(|| nexa_font::system_ui_font().map(|f| (f.data, f.index)))?;
    let mut font = Font::from_static(primary.0, primary.1).ok()?;
    for fam in extra.split(',').map(str::trim).filter(|f| !f.is_empty()) {
        if let Some((d, i)) = nexa_font::find_font_by_family(fam) {
            let _ = font.push_fallback(d, i);
        }
    }
    if !NERD_PROBE.iter().all(|&c| font.covers(c)) {
        if let Some((d, i)) = NERD_FAMILIES
            .iter()
            .find_map(|fam| nexa_font::find_font_by_family(fam))
        {
            let _ = font.push_fallback(d, i);
        }
    }
    // 고정폭 폴백(Linux 실기 10-03): 기호와 한글을 칸 폭에 맞게 가진 고정폭 글꼴을 가변폭 한글·기호 폴백보다 먼저 둔다
    // (설치돼 있지 않은 OS에서는 아무 일도 없다).
    for fam in MONO_FALLBACK_FAMILIES {
        if let Some((d, i)) = nexa_font::find_font_by_family(fam) {
            let _ = font.push_fallback(d, i);
        }
    }
    if let Some((d, i)) = nexa_font::find_collection_face(MONO_CJK_FACE.0, MONO_CJK_FACE.1) {
        let _ = font.push_fallback(d, i);
    }
    if let Some(s) = nexa_font::system_ui_font() {
        let _ = font.push_fallback(s.data, s.index);
    }
    for f in nexa_font::symbol_fallback_fonts() {
        let _ = font.push_fallback(f.data, f.index);
    }
    // 폴백 글꼴(한글 등)을 주 글꼴과 같은 em으로(nexa-ui 125차) — 전각 글자가 두 칸을 채운다(종전 = 같은 높이라 작고 벌어졌다).
    font.set_fallback_em_match(platform::term_fallback_em_match());
    Some(font)
}

#[cfg(test)]
mod chevron_tests {
    use super::*;

    /// 대체 쉐브론: 둘 다 그릴 수 있는 첫 쌍 · 아무것도 없으면 ASCII.
    #[test]
    fn fallback_chevrons_pick_first_drawable_pair() {
        assert_eq!(fallback_chevrons(|_| true), ("\u{203A}", "\u{2304}"));
        assert_eq!(
            fallback_chevrons(|c| c != '\u{2304}'),
            ("\u{203A}", "\u{02C5}")
        );
        assert_eq!(
            fallback_chevrons(|c| c == '\u{25B8}' || c == '\u{25BE}'),
            ("\u{25B8}", "\u{25BE}")
        );
        assert_eq!(fallback_chevrons(|c| c.is_ascii()), (">", "v"));
        assert_eq!(fallback_chevrons(|_| false), (">", "v"));
    }

    /// 터미널 글꼴 체인: 폴백 em 맞춤은 플랫폼 판정대로(Linux·macOS 켬 · Windows 종전).
    #[test]
    fn mono_chain_matches_fallback_em_per_platform() {
        let Some(f) = mono_chain(None, "") else {
            return; // 글꼴이 하나도 없는 환경
        };
        assert_eq!(f.fallback_em_match(), platform::term_fallback_em_match());
        assert_eq!(platform::term_fallback_em_match(), !cfg!(windows));
    }

    /// 아이콘 글꼴(MDL2)이 없는 OS에서는 쉐브론을 선으로 그린다 · 있으면 글리프(dir2와 같음).
    #[test]
    fn chevrons_are_drawn_as_lines_without_icon_font() {
        let ui = nexa_font::ui_font(None).expect("OS UI font");
        init_icon_glyphs(&ui.font);
        assert_eq!(nexa_grid::marker_vector(), !icon_font_covers(&ui.font));
        #[cfg(target_os = "linux")]
        assert!(nexa_grid::marker_vector(), "Linux에는 Segoe MDL2가 없다");
    }
}
