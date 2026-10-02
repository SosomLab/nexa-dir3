//! **복사 버튼 부품**(nexa-sql `copybtn.rs` 복사 · 사용자 09-23 "복사 기능을 가진 버튼은 전부") — 누르면 ① 눌리는 효과(120 ms) → ② 체크 표시 ·
//! 녹색(`theme.ok`)으로 "됐다"를 알린 뒤 ③ 복귀 시간(기본 2 초) 지나면 복사 아이콘으로 돌아온다.
//!
//! 상태 = `hover` · `pressed_at`. 그리기는 `now`를 받아 순수하게 판정하고, 애니메이션 중에는 [`CopyBtn::next_tick`]이 다음 틱 시각을 돌려준다.
//! dir3 차이: 아이콘 마스크(nexa-sql `toolicons`) 대신 글리프(⧉ · ✓) — SVG 아이콘은 T-30.

use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::theme::Theme;
use std::time::{Duration, Instant};

/// 눌리는 효과 길이.
pub(crate) const PRESS_MS: u64 = 120;
/// 기본 복귀 시간.
pub(crate) const DEFAULT_FEEDBACK_MS: u64 = 2000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Look {
    Idle,
    Pressed,
    /// 체크 · 녹색(0 = 방금 · 1 = 복귀 직전).
    Done(f32),
}

#[derive(Clone, Debug)]
pub(crate) struct CopyBtn {
    pub rect: Rect,
    hover: bool,
    pressed_at: Option<Instant>,
    feedback: Duration,
}

impl Default for CopyBtn {
    fn default() -> Self {
        Self::new()
    }
}

impl CopyBtn {
    pub(crate) fn new() -> Self {
        CopyBtn {
            rect: Rect::default(),
            hover: false,
            pressed_at: None,
            feedback: Duration::from_millis(DEFAULT_FEEDBACK_MS),
        }
    }

    pub(crate) fn set_feedback_ms(&mut self, ms: i64) {
        self.feedback = Duration::from_millis(ms.clamp(300, 10_000) as u64);
    }

    pub(crate) fn set_rect(&mut self, r: Rect) {
        self.rect = r;
    }

    pub(crate) fn hit(&self, p: Point) -> bool {
        self.rect.contains(p)
    }

    /// hover 갱신 — 바뀌었으면 true(다시 그린다).
    pub(crate) fn set_hover(&mut self, on: bool) -> bool {
        let changed = on != self.hover;
        self.hover = on;
        changed
    }

    /// 눌렀다(복사가 실제로 됐을 때 호스트가 부른다).
    pub(crate) fn press(&mut self, now: Instant) {
        self.pressed_at = Some(now);
    }

    pub(crate) fn look(&self, now: Instant) -> Look {
        let Some(t0) = self.pressed_at else {
            return Look::Idle;
        };
        let el = now.saturating_duration_since(t0);
        if el < Duration::from_millis(PRESS_MS) {
            Look::Pressed
        } else if el < self.feedback {
            Look::Done(el.as_secs_f32() / self.feedback.as_secs_f32())
        } else {
            Look::Idle
        }
    }

    /// 애니메이션 중이면 다음에 다시 그릴 시각 · 평상시 None.
    pub(crate) fn next_tick(&self, now: Instant) -> Option<Instant> {
        let t0 = self.pressed_at?;
        match self.look(now) {
            Look::Idle => None,
            Look::Pressed => Some(now + Duration::from_millis(16)),
            Look::Done(_) => Some(t0 + self.feedback),
        }
    }

    /// 그리기 — `alpha` = 담는 쪽의 투명도 · `scale` = 배율. 상자(옅은 배경 + 테두리) + 글리프.
    pub(crate) fn paint(
        &self,
        dc: &mut dyn DrawCtx,
        th: &Theme,
        alpha: f32,
        scale: f32,
        now: Instant,
    ) {
        let r = self.rect;
        if r.w <= 0 || r.h <= 0 {
            return;
        }
        let px = |v: f32| (v * scale).round() as i32;
        let look = self.look(now);
        let (bg_color, bg_alpha, border, glyph_color, inset) = match look {
            Look::Idle => (
                th.text,
                if self.hover { 0.16 } else { 0.06 },
                th.border,
                if self.hover { th.text } else { th.text_dim },
                0,
            ),
            Look::Pressed => (th.text, 0.30, th.text_dim, th.text, px(1.0)),
            Look::Done(p) => (th.ok, 0.32 * (1.0 - 0.6 * p), th.ok, th.ok, 0),
        };
        dc.fill_round_rect_alpha(r, px(3.0), bg_color, bg_alpha * alpha);
        dc.stroke_round_rect_alpha(r, px(3.0), border, 1.0, alpha);
        let glyph = match look {
            Look::Done(_) => "\u{2713}",
            _ => "\u{29C9}",
        };
        dc.select_font(FontSlot::Status, false);
        let tw = dc.text_width(glyph);
        let ty = dc.text_center_y(r.y, r.h) + inset;
        dc.text(r.x + (r.w - tw) / 2, ty, r, glyph, glyph_color);
        dc.select_font(FontSlot::Base, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_then_check_then_back() {
        let mut b = CopyBtn::new();
        b.set_feedback_ms(2000);
        let t0 = Instant::now();
        assert_eq!(b.look(t0), Look::Idle);
        assert_eq!(b.next_tick(t0), None);
        b.press(t0);
        assert_eq!(b.look(t0), Look::Pressed);
        assert_eq!(b.look(t0 + Duration::from_millis(119)), Look::Pressed);
        assert!(matches!(b.look(t0 + Duration::from_millis(120)), Look::Done(p) if p < 0.1));
        assert!(matches!(b.look(t0 + Duration::from_millis(1900)), Look::Done(p) if p > 0.9));
        assert_eq!(
            b.next_tick(t0 + Duration::from_millis(500)),
            Some(t0 + Duration::from_millis(2000))
        );
        assert_eq!(b.look(t0 + Duration::from_millis(2000)), Look::Idle);
        b.set_feedback_ms(10);
        assert_eq!(b.feedback, Duration::from_millis(300));
    }

    #[test]
    fn hover_reports_change_only() {
        let mut b = CopyBtn::new();
        b.rect = Rect::new(10, 10, 20, 20);
        assert!(b.hit(Point { x: 15, y: 15 }) && !b.hit(Point { x: 5, y: 5 }));
        assert!(b.set_hover(true));
        assert!(!b.set_hover(true));
        assert!(b.set_hover(false));
    }
}
