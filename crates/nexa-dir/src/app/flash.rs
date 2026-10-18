//! App — 순간 메시지(주 창 · nexa-sql 10-07 꼴 가져옴 · 10-18): [`App::flash`] = 활성 패널 **경로 바 옆**(가리지 않게 그 위·오른쪽) ·
//! 스타일 = 설정 `ui.flash_*`(그리기 직전에 읽는다) · 기동 명령 `flash:<글>` · 덤프 `flash` · 깨움 = [`App::aux_next_wake`].

use crate::flashmsg::FlashStyle;
use crate::*;
use nexa_ctl::FlashTone;

impl App {
    /// 설정 → 플래시 스타일(전용 글꼴은 `flash_font`에 미리 읽어 둔 것).
    pub(crate) fn flash_style(&self) -> FlashStyle {
        FlashStyle::from_settings(&self.settings, self.flash_font.clone())
    }

    /// 주 창 플래시 — 활성 패널 경로 바 옆.
    pub(crate) fn flash(&mut self, text: impl Into<String>, tone: FlashTone) {
        let anchor = self.panels[self.active].pathbar_bounds();
        self.flash_at(text, tone, anchor);
    }

    /// 주 창 플래시 — 앵커 지정(가리면 안 되는 사각형).
    pub(crate) fn flash_at(&mut self, text: impl Into<String>, tone: FlashTone, anchor: Rect) {
        let style = self.flash_style();
        self.flash.set_style(style);
        self.flash.show(text, tone, anchor);
        self.redraw();
    }

    /// 플래시 그리기(주 창의 맨 마지막 · 자체 글꼴 dc · 호스트 = 상태줄 위).
    pub(crate) fn paint_flash(&mut self, gfx: &mut Surface, wi: i32, hi: i32, s: f32) {
        if !self.flash.active() {
            return;
        }
        let style = self.flash_style();
        self.flash.set_style(style);
        let ui_px = self.font_px("ui.font_size");
        let ui_font = Rc::clone(&self.ui_font);
        let style_font = self.flash.style_font();
        let font: &Font = style_font.as_deref().unwrap_or(&ui_font);
        let prefs = FontPrefs::with_base(self.flash.px_or(ui_px));
        let mut dc = RasterCtx::new(gfx, font, s).with_fonts(prefs);
        let th = self.theme;
        let sb_y = self.statusbar.bounds().y;
        let host = Rect::new(0, 0, wi, if sb_y > 0 { sb_y } else { hi });
        if self.flash.paint(&mut dc, &th, s, host) {
            self.redraw();
        }
    }

    /// 보조 창·주 창 플래시의 다음 깨움 시각(페이드 시작 · 마감 — 즉시 모드는 마감 때 한 번 지운다).
    pub(crate) fn aux_next_wake(&self) -> Option<Instant> {
        [self.flash.next_wake(), self.bulk_win.flash_next_wake()]
            .into_iter()
            .flatten()
            .min()
    }
}
