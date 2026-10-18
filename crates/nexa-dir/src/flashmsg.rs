//! 순간 메시지(Flash) 공용 호스트 — nexa-sql의 플래시(프로젝트 탐색기 "색인 다시 읽음" · 10-07)를 그대로 가져온 꼴(사용자 10-18 "nexa-sql의
//! flash 메시지를 가져와서 적용할 준비"). nexa-ctl [`Flash`] 위에 **설정 → 스타일**(`ui.flash_hold_ms` · `ui.flash_ms` · `ui.flash_shape` ·
//! `ui.flash_font_*` · 성능 향상 모드 = 즉시 모드) · **색 = 머티리얼 스낵바 역상 표면**(밝은 테마 = 진회색 상자 + 흰 글 · 어두운 테마 = 밝은 표면 +
//! 진한 글 · 테두리 없음) · **자리 = 앵커 옆**(nexa-ctl 규칙: 좌하단 = 앵커 우상단 → 왼쪽으로 → 아래 → 호스트 안) · **자체 글꼴 dc**(호스트가
//! [`FlashBox::font`]로 만든다) · **깨움**(페이드 중 = 프레임 · 즉시 모드 = 마감 때 한 번)을 얹는다.
//!
//! 쓰는 곳: 주 창 [`crate::App::flash`](활성 패널 경로 바 옆) · 라이선스 창(링크 옆 "복사됨") · 일괄 이름 변경 창(카드 열 위 오른쪽 · 프리셋 불러옴).
//! 호스트는 창의 **맨 마지막**에 그린다(다른 컨트롤·팝업이 덮지 않게).

use ndir_settings::Settings;
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::Rect;
use nexa_ctl::theme::Theme;
use nexa_ctl::{Color, Flash, FlashTone};
use nexa_gfx::Font;
use std::rc::Rc;
use std::time::Instant;

/// 플래시 스타일(설정 `ui.flash_*` 스냅숏) — 호스트가 그리기 직전에 [`FlashBox::set_style`]로 넘긴다(모양·글꼴·시간 변경이 바로 반영 · 비용 =
/// 조회 몇 번).
#[derive(Clone, Default)]
pub(crate) struct FlashStyle {
    /// `rect` · `rounded` · `none`(배경 없음 · 글만).
    pub shape: String,
    /// 전용 글꼴(`ui.flash_font_face` · None = 창의 UI 글꼴) · 글자 크기(논리 px · 0 = UI 크기).
    pub font: Option<Rc<Font>>,
    pub px: f32,
    /// 유지 · 페이드(ms).
    pub hold_ms: u64,
    pub fade_ms: u64,
    /// 즉시 모드(성능 향상 모드 · 페이드 없이 유지 뒤 바로 숨김 · 프레임 0).
    pub instant: bool,
}

impl FlashStyle {
    /// 설정에서(`flash_font` = 호스트가 미리 읽어 둔 전용 글꼴).
    pub(crate) fn from_settings(settings: &Settings, flash_font: Option<Rc<Font>>) -> Self {
        FlashStyle {
            shape: settings
                .get("ui.flash_shape")
                .unwrap_or("rounded")
                .to_string(),
            font: flash_font,
            px: settings.font_px("ui.flash_font_size"),
            hold_ms: settings.int("ui.flash_hold_ms").clamp(0, 30_000) as u64,
            fade_ms: settings.int("ui.flash_ms").clamp(200, 30_000) as u64,
            instant: settings.flag("perf.boost"),
        }
    }

    /// 모양 → nexa-ctl `Flash` 상태(배경 · 반지름(물리 px)). 순수 — `(배경 켬, 반지름)`.
    pub(crate) fn shape_of(shape: &str, scale: f32) -> (bool, i32) {
        match shape {
            "none" => (false, 0),
            "rect" => (true, 0),
            _ => (true, (5.0 * scale).round() as i32),
        }
    }
}

/// 스낵바 역상 표면 `(배경, 글)` — 테마 바탕과 대비가 확실한 쪽(nexa-sql 10-07 "촌스럽다 · 예쁘게").
pub(crate) fn snackbar_colors(th: &Theme) -> (Color, Color) {
    if th.is_dark {
        (Color(0x00E8_EAED), Color(0x0020_2124))
    } else {
        (Color(0x0032_3232), Color(0x00FF_FFFF))
    }
}

/// 플래시 한 줄 + 스타일 + 앵커(메시지 하나 · 새로 `show`하면 앞 것을 대체).
#[derive(Default)]
pub(crate) struct FlashBox {
    flash: Flash,
    style: FlashStyle,
    anchor: Rect,
    /// 마지막 paint가 "더 그려야 한다"고 했는가(페이드 진행 중).
    live: bool,
}

impl FlashBox {
    pub(crate) fn new() -> Self {
        let mut b = FlashBox {
            flash: Flash::new(),
            ..Default::default()
        };
        b.flash.set_border(false);
        b
    }

    pub(crate) fn set_style(&mut self, style: FlashStyle) {
        self.flash.set_instant(style.instant);
        self.style = style;
    }

    /// 보인다(유지 → 페이드 = 스타일의 시간) — `anchor` = 가리면 안 되는 사각형(메시지는 그 위·오른쪽에).
    pub(crate) fn show(&mut self, text: impl Into<String>, tone: FlashTone, anchor: Rect) {
        self.anchor = anchor;
        self.flash
            .show(text, tone, self.style.hold_ms, self.style.fade_ms);
        self.live = true;
    }

    pub(crate) fn clear(&mut self) {
        self.flash.clear();
        self.live = false;
    }

    pub(crate) fn active(&self) -> bool {
        self.flash.active()
    }

    /// 현재 글(없으면 빈 글) — 시험·덤프.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn text(&self) -> &str {
        self.flash.text()
    }

    /// 프레임마다 다시 그려야 하는가(페이드 진행 중 · 즉시 모드는 아니다).
    pub(crate) fn animating(&self) -> bool {
        self.flash.active() && self.live && !self.flash.instant()
    }

    /// 다음에 꼭 깨어야 할 시각(페이드 시작 · 마감) — 즉시 모드에서 마감 때 한 번 지우기 위해.
    pub(crate) fn next_wake(&self) -> Option<Instant> {
        if !self.flash.active() {
            return None;
        }
        let now = Instant::now();
        match (self.flash.fade_start(), self.flash.deadline()) {
            (Some(f), _) if f > now && !self.flash.instant() => Some(f),
            (_, Some(d)) => Some(d),
            _ => None,
        }
    }

    /// 틱: 마감이 지났으면 지우고 `true`(한 번 다시 그려 없앤다).
    pub(crate) fn tick(&mut self, now: Instant) -> bool {
        if !self.flash.active() {
            return false;
        }
        if self.flash.deadline().is_some_and(|d| now >= d) {
            self.clear();
            return true;
        }
        false
    }

    /// 전용 글꼴(없으면 호스트가 창의 UI 글꼴을 쓴다) · 글자 크기(0이면 `ui_px`).
    pub(crate) fn style_font(&self) -> Option<Rc<Font>> {
        self.style.font.clone()
    }

    pub(crate) fn px_or(&self, ui_px: f32) -> f32 {
        if self.style.px > 0.0 {
            self.style.px
        } else {
            ui_px
        }
    }

    /// 그린다(호스트가 [`Self::style_font`]·[`Self::px_or`]로 만든 dc · `host` = 메시지가 넘지 않을 영역). 더 그려야 하면 `true`.
    pub(crate) fn paint(
        &mut self,
        dc: &mut dyn DrawCtx,
        th: &Theme,
        scale: f32,
        host: Rect,
    ) -> bool {
        if !self.flash.active() {
            self.live = false;
            return false;
        }
        let (bg, radius) = FlashStyle::shape_of(&self.style.shape, scale);
        self.flash.set_background(bg);
        self.flash.set_radius(radius);
        self.flash.set_colors(Some(snackbar_colors(th)));
        dc.select_font(FontSlot::Base, false);
        self.live = self.flash.paint(dc, th, self.anchor, host);
        self.live
    }

    /// 덤프(`flash.dump` · 시험): `active=<bool> instant=<bool> text=<글>`.
    pub(crate) fn dump(&self) -> String {
        format!(
            "active={} instant={} text={}\n",
            self.flash.active(),
            self.flash.instant(),
            self.flash.text()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 설정 → 스타일(기본값 · 성능 모드 = 즉시) · show/active/text/tick(마감 뒤 지움) · 모양 표.
    #[test]
    fn style_from_settings_and_box_lifecycle() {
        let dir = std::env::temp_dir().join(format!("ndir-flash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let s = Settings::from_text(dir.join("settings.conf"), "");
        let st = FlashStyle::from_settings(&s, None);
        assert_eq!(
            (st.shape.as_str(), st.hold_ms, st.fade_ms, st.instant),
            ("rounded", 2000, 3000, false)
        );
        let s2 = Settings::from_text(dir.join("s2.conf"), "perf.boost=on\nui.flash_shape=none\n");
        let st2 = FlashStyle::from_settings(&s2, None);
        assert!(st2.instant && st2.shape == "none");
        assert_eq!(FlashStyle::shape_of("none", 2.0), (false, 0));
        assert_eq!(FlashStyle::shape_of("rect", 2.0), (true, 0));
        assert_eq!(FlashStyle::shape_of("rounded", 2.0), (true, 10));
        let mut b = FlashBox::new();
        b.set_style(FlashStyle {
            hold_ms: 0,
            fade_ms: 200,
            ..FlashStyle::default()
        });
        assert!(!b.active() && b.next_wake().is_none());
        b.show("hi", FlashTone::Info, Rect::new(10, 10, 100, 20));
        assert!(b.active() && b.text() == "hi" && b.animating());
        assert!(b.next_wake().is_some());
        assert!(!b.tick(Instant::now()), "마감 전 = 그대로");
        assert!(b.dump().starts_with("active=true instant=false text=hi"));
        b.clear();
        assert!(!b.active());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
