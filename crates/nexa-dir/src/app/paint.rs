//! App — 그리기(프레임 합성 · nexa-sql `app/paint.rs` 축약). 층 순서 = 본문 → 크롬(툴바·메뉴바 배경) → 상태줄 → 토스트 →
//! 툴팁 → **메뉴바 드롭다운**(최상위 · nexa-sql 09-15 교훈: 풀다운이 뒤로 가림) → 경로바 제안 팝업.

use crate::*;

impl App {
    pub(crate) fn paint(&mut self) {
        let (Some(win), Some(surface)) = (self.window.clone(), self.surface.as_mut()) else {
            return;
        };
        let size = win.inner_size();
        let Some(mut buf) = surface.frame(size) else {
            return;
        };
        let s = self.scale;
        let (wi, hi) = (size.width as i32, size.height as i32);
        {
            let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
            let th = self.theme;
            let ui_px = self.settings.font_px("ui.font_size");
            let mut dc =
                RasterCtx::new(&mut gfx, &self.ui_font, s).with_fonts(FontPrefs::with_base(ui_px));
            dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
            // 본문
            self.tabs.paint(&mut dc, &th);
            self.pathbar.paint(&mut dc, &th);
            self.panels[0].paint(&mut dc, &th);
            if self.dual {
                let l = self.panels[0].bounds();
                let r = self.panels[1].bounds();
                dc.fill_rect(
                    Rect::new(l.right(), l.y, r.x - l.right(), l.h),
                    th.chrome_bg,
                );
                self.panels[1].paint(&mut dc, &th);
            }
            // 크롬(창 전폭)
            dc.fill_rect(self.toolbar.bounds(), th.chrome_bg);
            self.toolbar.paint(&mut dc, &th);
            dc.fill_rect(
                Rect::new(0, self.toolbar.bounds().bottom() - 1, wi, 1),
                th.border,
            );
            dc.fill_rect(self.menubar.bounds(), th.chrome_bg);
            // 상태줄
            let sb = self.statusbar.bounds();
            dc.fill_rect(sb, th.chrome_bg);
            dc.fill_rect(Rect::new(0, sb.y, wi, 1), th.border);
            self.statusbar.paint(&mut dc, &th);
            // 팝업 층
            self.toasts.paint(&mut dc, &th, wi, sb.y, s);
            self.toolbar.paint_tooltip(&mut dc, &th);
            self.menubar.paint(&mut dc, &th);
            // 경로바 제안 팝업은 dir2 세대 DrawCtx 어휘(nexa-grid `Adapt`로 감싼다 · 어휘 어댑터 원칙 journal §10).
            let mut adapt = nexa_grid::Adapt(&mut dc);
            self.pathbar.paint_suggest(&mut adapt, &th);
        }
        let _ = buf.present();
    }
}
