//! 메모리 창(T-93 1차 · docs/22 NEW-002 · nexa-sql `mem_win.rs` 개념 이식): 상태줄의 앱 메모리 칸 → **모덜리스** 창.
//! 위 = 이 프로그램이 쓰는 메모리(운영체제가 보는 값) · 표 = 영역별 추정 바이트 + 막대(전체 대비) · 차이는 "기타" ·
//! 아래 = 시스템 메모리(사용/전체) · [닫기]. 값은 호스트가 상태줄 조회 주기마다 다시 그려 갱신한다(창이 닫혀 있으면 비용 0).
//! 골격은 `license_win.rs`와 같다.

use ndir_i18n::{tr, trf};
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::{Button, Control, InputEvent, Invalidations, Widget};
use nexa_gfx::{Font, Surface};
use std::rc::Rc;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// 호스트가 그릴 때 넘기는 보기.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MemView {
    /// 이 프로그램의 메모리(운영체제 값 · 모르면 `None`).
    pub app: Option<u64>,
    /// 영역별 추정 `(이름, 바이트)` — 호스트가 i18n으로 만든다("기타" 포함).
    pub rows: Vec<(String, u64)>,
    /// 시스템 메모리 `(쓰는 양, 전체)`.
    pub system: Option<(u64, u64)>,
}

/// 영역별 추정 + 운영체제 값 → 표 행(순수): 추정 합이 운영체제 값보다 작으면 그 차이를 `other` 이름으로 덧붙인다
/// (추정이 더 크면 기타 = 0 — 음수가 되지 않는다).
pub(crate) fn rows_with_other(
    parts: Vec<(String, u64)>,
    app: Option<u64>,
    other: String,
) -> Vec<(String, u64)> {
    let sum: u64 = parts.iter().map(|p| p.1).sum();
    let mut rows = parts;
    if let Some(total) = app {
        rows.push((other, total.saturating_sub(sum)));
    }
    rows
}

/// 창이 호스트에 요청하는 것.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemAction {
    None,
    Paint,
    Close,
}

pub(crate) struct MemWin {
    window: Option<Rc<Window>>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    cursor: (i32, i32),
    btn_close: Button,
}

impl MemWin {
    pub(crate) fn new() -> Self {
        MemWin {
            window: None,
            surface: None,
            scale: 1.0,
            cursor: (0, 0),
            btn_close: Button::new(tr("license.btn.close")),
        }
    }

    pub(crate) fn open(
        &mut self,
        el: &ActiveEventLoop,
        theme: Option<winit::window::Theme>,
        over: Option<(i32, i32, u32, u32)>,
        owner: Option<&Window>,
    ) {
        if let Some(w) = &self.window {
            w.focus_window();
            self.redraw();
            return;
        }
        let (lw, lh) = (460.0, 340.0);
        let mut attrs = Window::default_attributes()
            .with_title(format!("Nexa Dir — {}", tr("mem.title")))
            .with_theme(theme)
            .with_resizable(true)
            .with_min_inner_size(winit::dpi::LogicalSize::new(360.0, 240.0))
            .with_inner_size(winit::dpi::LogicalSize::new(lw, lh));
        if let Some((x, y, w, h)) = over {
            let cx = x + (w as i32 - lw as i32) / 2;
            let cy = y + (h as i32 - lh as i32) / 2;
            attrs = attrs.with_position(winit::dpi::PhysicalPosition::new(cx.max(0), cy.max(0)));
        }
        let attrs = crate::winfocus::owned_by(crate::icon::with_icon(attrs), owner);
        let Ok(win) = el.create_window(attrs) else {
            return;
        };
        let win = Rc::new(win);
        self.scale = win.scale_factor() as f32;
        self.surface = crate::present::Presenter::new(win.clone()).ok();
        self.window = Some(win);
        self.redraw();
    }

    pub(crate) fn close(&mut self) {
        self.surface = None;
        self.window = None;
        self.btn_close.clear_transient();
    }

    pub(crate) fn is_open(&self) -> bool {
        self.window.is_some()
    }

    pub(crate) fn is(&self, id: WindowId) -> bool {
        self.window.as_ref().is_some_and(|w| w.id() == id)
    }

    pub(crate) fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    /// 언어 전환 — 버튼 글 · 창 제목.
    pub(crate) fn relabel(&mut self) {
        self.btn_close.set_label(tr("license.btn.close"));
        if let Some(w) = &self.window {
            w.set_title(&format!("Nexa Dir — {}", tr("mem.title")));
        }
        self.redraw();
    }

    pub(crate) fn tick(&mut self, now_ms: u64) -> bool {
        self.window.is_some() && self.btn_close.tick(now_ms)
    }

    pub(crate) fn animating(&self) -> bool {
        self.window.is_some() && self.btn_close.is_animating()
    }

    pub(crate) fn handle(&mut self, ev: &WindowEvent) -> MemAction {
        let mut inv = Invalidations::default();
        match ev {
            WindowEvent::CloseRequested => return MemAction::Close,
            WindowEvent::RedrawRequested => return MemAction::Paint,
            WindowEvent::Resized(_) => self.redraw(),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.redraw();
            }
            WindowEvent::Focused(false) => {
                self.btn_close.clear_transient();
                self.redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as i32, position.y as i32);
                let (x, y) = self.cursor;
                self.btn_close
                    .on_event(&InputEvent::MouseMove { x, y }, &mut inv);
                if !inv.is_empty() {
                    self.redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } if *button == MouseButton::Left => {
                let (x, y) = self.cursor;
                let e = match state {
                    ElementState::Pressed => InputEvent::MouseDown {
                        x,
                        y,
                        shift: false,
                        primary: false,
                    },
                    ElementState::Released => InputEvent::MouseUp { x, y },
                };
                let up = matches!(e, InputEvent::MouseUp { .. });
                if up || self.btn_close.bounds().contains(Point { x, y }) {
                    self.btn_close.on_event(&e, &mut inv);
                }
                self.redraw();
                if self.btn_close.take_clicked() {
                    return MemAction::Close;
                }
            }
            WindowEvent::KeyboardInput { event: kev, .. }
                if kev.state == ElementState::Pressed
                    && matches!(kev.logical_key.as_ref(), Key::Named(NamedKey::Escape)) =>
            {
                return MemAction::Close;
            }
            _ => {}
        }
        MemAction::None
    }

    pub(crate) fn paint(&mut self, view: MemView, font: &Font, th: &Theme, ui_px: f32) {
        let Some(win) = self.window.clone() else {
            return;
        };
        let Some(mut surface) = self.surface.take() else {
            return;
        };
        let size = win.inner_size();
        let Some(mut buf) = surface.frame(size) else {
            self.surface = Some(surface);
            return;
        };
        let s = self.scale;
        let (wi, hi) = (size.width as i32, size.height as i32);
        let px = |v: f32| (v * s).round() as i32;
        let inv = &mut Invalidations::default();
        let fmt = crate::filelist::format_size;
        {
            let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
            let prefs = FontPrefs::with_base(ui_px);
            let mut dc = RasterCtx::new(&mut gfx, font, s).with_fonts(prefs);
            dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
            dc.select_font(FontSlot::Base, false);
            let th_txt = dc.text_height();
            let pad = px(12.0);
            let clip = Rect::new(0, 0, wi, hi);
            // 머리: 이 프로그램의 메모리(운영체제 값).
            let head = trf("mem.app", &[&view.app.map_or_else(|| "–".to_string(), fmt)]);
            dc.fill_rect_alpha(
                Rect::new(pad, pad, wi - pad * 2, th_txt + px(10.0)),
                th.accent,
                0.12,
            );
            dc.text(pad + px(8.0), pad + px(5.0), clip, &head, th.text);
            let mut y = pad + th_txt + px(20.0);
            // 표: 이름 · 막대(전체 대비) · 바이트(오른쪽 정렬).
            let total = view
                .app
                .unwrap_or_else(|| view.rows.iter().map(|r| r.1).sum())
                .max(1);
            let label_w = view
                .rows
                .iter()
                .map(|(k, _)| dc.text_width(k))
                .max()
                .unwrap_or(0)
                + px(14.0);
            let val_w = dc.text_width("999.9 MB") + px(8.0);
            let row_h = th_txt + px(8.0);
            let bar_x = pad + label_w;
            let bar_w = (wi - pad * 2 - label_w - val_w).max(px(20.0));
            for (k, v) in &view.rows {
                dc.text(pad, y, clip, k, th.text_dim);
                let track = Rect::new(bar_x, y + px(3.0), bar_w, (th_txt - px(4.0)).max(2));
                dc.fill_rect(track, th.panel_bg_alt);
                let w = ((*v as f64 / total as f64).min(1.0) * f64::from(bar_w)).round() as i32;
                if w > 0 {
                    dc.fill_rect(Rect::new(track.x, track.y, w, track.h), th.accent);
                }
                let txt = fmt(*v);
                let tw = dc.text_width(&txt);
                dc.text(wi - pad - tw, y, clip, &txt, th.text);
                y += row_h;
            }
            y += px(6.0);
            dc.fill_rect(Rect::new(pad, y, wi - pad * 2, 1), th.border);
            y += px(10.0);
            if let Some((used, all)) = view.system {
                let pct = if all == 0 {
                    0.0
                } else {
                    used as f64 / all as f64 * 100.0
                };
                let line = trf("mem.system", &[&fmt(used), &fmt(all), &format!("{pct:.0}")]);
                dc.text(pad, y, clip, &line, th.text_dim);
            }
            // 닫기 버튼(오른쪽 아래).
            let btn_h = th_txt + px(12.0);
            self.btn_close.set_scale(s);
            let bw = dc.text_width(&tr("license.btn.close")) + px(28.0);
            self.btn_close
                .set_bounds(Rect::new(wi - pad - bw, hi - pad - btn_h, bw, btn_h), inv);
            self.btn_close.paint(&mut dc, th);
        }
        let _ = buf.present();
        self.surface = Some(surface);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 기타 = 운영체제 값 − 추정 합(음수 없음) · 운영체제 값을 모르면 기타 없음.
    #[test]
    fn other_row_is_remainder_and_never_negative() {
        let parts = || vec![("lists".to_string(), 30u64), ("surfaces".to_string(), 50)];
        let rows = rows_with_other(parts(), Some(100), "other".into());
        assert_eq!(rows.last(), Some(&("other".to_string(), 20)));
        let rows = rows_with_other(parts(), Some(60), "other".into());
        assert_eq!(rows.last().map(|r| r.1), Some(0), "추정이 더 커도 0");
        assert_eq!(rows_with_other(parts(), None, "other".into()).len(), 2);
    }
}
