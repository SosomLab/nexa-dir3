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
                let prefs = self.font_prefs();
                let font = Rc::clone(&self.ui_font);
                let mono = self.mono_font.clone();
                let mut dc =
                    RasterCtx::with_font_set(&mut gfx, self.font_set(&font, mono.as_deref()), s)
                        .with_fonts(prefs);
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
        // 탭 바 줄 수가 바뀌었으면(여러 줄 — 그리기가 측정한다) 다시 배치하고 한 번 더 그린다(dir2 win.rs:4941-4947).
        let l0 = self.panels[0].take_tab_lines_changed();
        let l1 = self.panels[1].take_tab_lines_changed();
        if l0 || l1 {
            self.layout_core();
            self.redraw();
        }
        // 탭을 반대 패널로 끄는 중: 대상 탭 바를 강조색으로 옅게 덮고 놓일 자리에 세로선(탭 바 안 이동의 표식과 같은 색).
        if let Some((dst, line)) = self.tab_drop_hint {
            dc.fill_rect_alpha(self.panels[dst].tabbar_bounds(), th.accent, 0.12);
            dc.fill_rect(line, th.accent);
        }
        // 끌어오는 중: **놓일 자리만** 강조한다(폴더 행 = 그 행 · 파일 행/빈 곳 = 목록 전체 = 현재 폴더 — 탐색기와 같다 ·
        // 놓을 수 없는 곳은 표시 없음 + 금지 커서). 옅은 강조색 채움 + 2px 테두리.
        if let Some(m) = &self.dnd_mark {
            let r = m.rect;
            let t = (2.0 * s).round().max(1.0) as i32;
            dc.fill_rect_alpha(r, th.accent, 0.14);
            dc.fill_rect(Rect::new(r.x, r.y, r.w, t), th.accent);
            dc.fill_rect(Rect::new(r.x, r.bottom() - t, r.w, t), th.accent);
            dc.fill_rect(Rect::new(r.x, r.y, t, r.h), th.accent);
            dc.fill_rect(Rect::new(r.right() - t, r.y, t, r.h), th.accent);
        }
        // 도크 경계 2종(비어 있으면 안 그린다 — 도크 숨김 · 단일 정보).
        self.dock_split_h.paint(dc, &th);
        self.dock_split_v.paint(dc, &th);
        for d in &self.docks {
            if d.bounds().h > 0 {
                d.paint(dc, &th);
            }
        }
        self.paint_terms(dc, &th, panel_metrics(&self.settings, s).row_h);
        // 크롬(창 전폭)
        dc.fill_rect(self.toolbar.bounds(), th.chrome_bg);
        self.toolbar.paint(dc, &th);
        if self.launcherbar.bounds().h > 0 {
            dc.fill_rect(self.launcherbar.bounds(), th.chrome_bg);
            self.launcherbar.paint(dc, &th);
        }
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
        self.toolbar.paint_drag_overlay(dc, &th); // 끌려가는 그룹(툴바 밖까지 나간다)
        self.panels[0].paint_popups(dc, &th);
        if self.dual {
            self.panels[1].paint_popups(dc, &th);
        }
        self.menubar.paint(dc, &th);
        self.tab_menu.paint(dc, &th);
    }
}
