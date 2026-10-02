//! App — 그리기(프레임 합성 · nexa-sql `app/paint.rs` 축약). 층 순서 = 본문(패널 · 스플리터) → 크롬(툴바·메뉴바 배경) → 상태줄 →
//! 토스트 → 툴팁·경로 제안 → **메뉴바 드롭다운**(최상위 · nexa-sql 09-15 교훈: 풀다운이 뒤로 가림).
//!
//! `paint`(Shell · 표면 빌림 + 래스터 컨텍스트) / [`App::paint_into`](AppCore · 어떤 `DrawCtx`든 — 시험은 `RecordCtx` · CI-105).

use crate::*;

impl App {
    pub(crate) fn paint(&mut self) {
        // 표면을 잠시 꺼내 둔다 — 버퍼가 표면을 빌리는 동안 `paint_into(&mut self)`를 부르기 위해(그리기 중 `self.surface`는 안 쓴다).
        let (Some(win), Some(mut surface)) = (self.window.clone(), self.surface.take()) else {
            return;
        };
        let size = win.inner_size();
        let s = self.scale;
        let (wi, hi) = (size.width as i32, size.height as i32);
        if let Some(mut buf) = surface.frame(size) {
            {
                let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
                let ui_px = self.settings.font_px("ui.font_size");
                let font = Rc::clone(&self.ui_font);
                let mut dc =
                    RasterCtx::new(&mut gfx, &font, s).with_fonts(FontPrefs::with_base(ui_px));
                self.paint_into(&mut dc, wi, hi, s);
            }
            let _ = buf.present();
        }
        self.surface = Some(surface);
    }

    /// 한 프레임을 `dc`에(창 무관). `wi`×`hi` = 표면 크기(장치 px) · `s` = 배율.
    pub(crate) fn paint_into(&mut self, dc: &mut dyn DrawCtx, wi: i32, hi: i32, s: f32) {
        let th = self.theme;
        dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
        // 본문
        self.panels[0].paint(dc, &th);
        if self.dual {
            self.panels[1].paint(dc, &th);
            self.splitter.paint(dc, &th);
        }
        for d in &self.docks {
            if d.bounds().h > 0 {
                d.paint(dc, &th);
            }
        }
        // 크롬(창 전폭)
        dc.fill_rect(self.toolbar.bounds(), th.chrome_bg);
        self.toolbar.paint(dc, &th);
        dc.fill_rect(
            Rect::new(0, self.toolbar.bounds().bottom() - 1, wi, 1),
            th.border,
        );
        dc.fill_rect(self.menubar.bounds(), th.chrome_bg);
        // 상태줄
        let sb = self.statusbar.bounds();
        dc.fill_rect(sb, th.chrome_bg);
        dc.fill_rect(Rect::new(0, sb.y, wi, 1), th.border);
        self.statusbar.paint(dc, &th);
        // 팝업 층
        self.toasts.paint(dc, &th, wi, sb.y, s);
        self.toolbar.paint_tooltip(dc, &th);
        self.panels[0].paint_popups(dc, &th);
        if self.dual {
            self.panels[1].paint_popups(dc, &th);
        }
        self.menubar.paint(dc, &th);
        self.tab_menu.paint(dc, &th);
    }
}
