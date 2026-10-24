//! 프리셋 편집 창(일괄 이름 변경 · 사용자 10-18 · Path Finder "Edit Renaming Sequences" 꼴): 저장된 프리셋 전체를 **순서대로** 보이고
//! 드래그(또는 Ctrl+↑↓)로 순서를 바꾸며(▲▼ 버튼 없음 · 사용자 10-18) 행 오른쪽 ⊖(또는 Delete)로 개별 삭제한다. OK = 호스트가 순서 파일 + 파일 삭제를 반영
//! ([`PresetAction::Apply`]) · 취소/Esc = 버린다. `order_win.rs`와 같은 보조 창 골격 — 창 없이도 모델을 조작할 수 있다(시험).

use ndir_i18n::tr;
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::{Button, Control, InputEvent, Invalidations, Widget};
use nexa_gfx::{Font, Surface};
use std::rc::Rc;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// 창이 호스트에 요청하는 것.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PresetAction {
    None,
    Paint,
    /// OK — 남은 프리셋을 이 순서로(빠진 이름 = 삭제).
    Apply(Vec<String>),
}

const PAD: f32 = 12.0;
const ROW_H: f32 = 26.0;
/// 버튼(사용자 10-18 "크기를 줄여줘" · 확인 = 기본 버튼 Accent).
const BTN_H: f32 = 24.0;
const BTN_W: f32 = 64.0;
const LIST_W: f32 = 320.0;
/// 행 오른쪽 삭제(⊖) 칸 폭.
const DEL_W: f32 = 30.0;
/// 보이는 행 = 6.5(사용자 10-18 "6개 + 0.5개" — 반 행은 다음에 더 있음을 알리는 창).
const VISIBLE_ROWS: f32 = 6.5;
/// 드래그 자동 스크롤: 목록 위/아래 가장자리 띠(행 높이)에 들어오면 틱마다 이만큼(논리 px).
const AUTO_SCROLL_STEP: f32 = 6.0;
const DRAG_THRESHOLD: i32 = 5;

struct Drag {
    press_y: i32,
    cur_y: i32,
    active: bool,
}

pub(crate) struct PresetWin {
    window: Option<Rc<Window>>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    cursor: (i32, i32),
    shift: bool,
    primary: bool,
    names: Vec<String>,
    sel: Option<usize>,
    scroll: i32,
    list: Rect,
    drag: Option<Drag>,
    ok_btn: Button,
    cancel_btn: Button,
}

impl PresetWin {
    pub(crate) fn new() -> Self {
        let mut w = PresetWin {
            window: None,
            surface: None,
            scale: 1.0,
            cursor: (0, 0),
            shift: false,
            primary: false,
            names: Vec::new(),
            sel: None,
            scroll: 0,
            list: Rect::default(),
            drag: None,
            ok_btn: Button::new(tr("bulk.preset.ok")),
            cancel_btn: Button::new(tr("bulk.preset.cancel")),
        };
        w.ok_btn.set_tone(nexa_ctl::controls::ButtonTone::Accent);
        w
    }

    /// 편집 대상(열 때 · 호스트가 읽은 순서 그대로).
    pub(crate) fn set(&mut self, names: Vec<String>) {
        self.names = names;
        self.sel = (!self.names.is_empty()).then_some(0);
        self.scroll = 0;
        self.drag = None;
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn names(&self) -> &[String] {
        &self.names
    }

    // ── 모델(창 없이도) ──

    pub(crate) fn select(&mut self, i: usize) {
        if i < self.names.len() {
            self.sel = Some(i);
            self.ensure_visible(i);
        }
    }

    /// 선택 행을 한 칸 위/아래로. 움직였으면 `true`.
    pub(crate) fn move_sel(&mut self, up: bool) -> bool {
        let Some(i) = self.sel else {
            return false;
        };
        let j = if up {
            i.checked_sub(1)
        } else {
            (i + 1 < self.names.len()).then_some(i + 1)
        };
        let Some(j) = j else {
            return false;
        };
        self.names.swap(i, j);
        self.sel = Some(j);
        self.ensure_visible(j);
        true
    }

    /// 선택 행을 `to` 자리로(드래그 놓기). 움직였으면 `true`.
    pub(crate) fn move_to(&mut self, to: usize) -> bool {
        let Some(i) = self.sel else {
            return false;
        };
        if to >= self.names.len() || to == i {
            return false;
        }
        let n = self.names.remove(i);
        self.names.insert(to, n);
        self.sel = Some(to);
        true
    }

    /// `i`번째 삭제(선택은 같은 자리의 다음 항목으로). 지웠으면 `true`.
    pub(crate) fn remove(&mut self, i: usize) -> bool {
        if i >= self.names.len() {
            return false;
        }
        self.names.remove(i);
        self.sel = if self.names.is_empty() {
            None
        } else {
            Some(i.min(self.names.len() - 1))
        };
        self.clamp_scroll();
        true
    }

    /// 자체 시험 덤프(`presets.dump`): `open|closed sel=<i|-> [a|b|c]`.
    pub(crate) fn dump(&self) -> String {
        format!(
            "{} sel={} [{}]\n",
            if self.window.is_some() {
                "open"
            } else {
                "closed"
            },
            self.sel.map_or("-".to_string(), |i| i.to_string()),
            self.names.join("|")
        )
    }

    // ── 창 ──

    fn s(&self, v: f32) -> i32 {
        (v * self.scale).round() as i32
    }

    pub(crate) fn is(&self, id: WindowId) -> bool {
        self.window.as_ref().is_some_and(|w| w.id() == id)
    }

    pub(crate) fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    pub(crate) fn tick(&mut self, now_ms: u64) -> bool {
        if self.window.is_none() {
            return false;
        }
        let mut any = false;
        for b in self.buttons() {
            any |= b.tick(now_ms);
        }
        // 드래그 자동 스크롤(사용자 10-18 "위로 끌면 자동 스크롤"): 가장자리 띠 안이면 한 걸음.
        if let Some(dir) = self.auto_scroll_dir() {
            let before = self.scroll;
            self.scroll += dir * self.s(AUTO_SCROLL_STEP);
            self.clamp_scroll();
            any |= self.scroll != before;
        }
        any
    }

    /// 드래그 중 커서가 목록 위/아래 가장자리 띠에 있으면 `-1`/`+1`(아직 더 갈 수 있을 때만).
    fn auto_scroll_dir(&self) -> Option<i32> {
        let d = self.drag.as_ref().filter(|d| d.active)?;
        let edge = self.row_h();
        let max = (self.names.len() as i32 * self.row_h() - self.list.h).max(0);
        if d.cur_y < self.list.y + edge && self.scroll > 0 {
            Some(-1)
        } else if d.cur_y > self.list.bottom() - edge && self.scroll < max {
            Some(1)
        } else {
            None
        }
    }

    pub(crate) fn animating(&self) -> bool {
        self.window.is_some()
            && ([&self.ok_btn, &self.cancel_btn]
                .iter()
                .any(|b| b.is_animating())
                || self.auto_scroll_dir().is_some())
    }

    fn buttons(&mut self) -> [&mut Button; 2] {
        [&mut self.ok_btn, &mut self.cancel_btn]
    }

    /// 창 논리 크기 — 행 수(4~12)에 맞춘다.
    fn logical_size(&self) -> (f64, f64) {
        let n = VISIBLE_ROWS;
        let w = PAD * 2.0 + LIST_W;
        let h = PAD * 3.0 + n * ROW_H + 2.0 + BTN_H;
        (w as f64, h as f64)
    }

    /// 언어 전환 — 버튼 글.
    pub(crate) fn relabel(&mut self) {
        self.ok_btn.set_label(tr("bulk.preset.ok"));
        self.cancel_btn.set_label(tr("bulk.preset.cancel"));
        if let Some(w) = &self.window {
            w.set_title(&format!(
                "{} — {}",
                crate::APP_TITLE,
                tr("bulk.preset.editTitle")
            ));
            w.request_redraw();
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
            self.layout();
            self.redraw();
            return;
        }
        let (lw, lh) = self.logical_size();
        let mut attrs = Window::default_attributes()
            .with_title(format!(
                "{} — {}",
                crate::APP_TITLE,
                tr("bulk.preset.editTitle")
            ))
            .with_theme(theme)
            .with_resizable(false)
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
        let _ = crate::platform::apply_window_opacity(&win); // 창 투명도(T-102)
        let win = Rc::new(win);
        self.scale = win.scale_factor() as f32;
        self.surface = crate::present::Presenter::new(win.clone()).ok();
        self.window = Some(win);
        self.layout();
        self.redraw();
    }

    pub(crate) fn close(&mut self) {
        self.surface = None;
        self.window = None;
        self.drag = None;
    }

    fn layout(&mut self) {
        let Some(win) = &self.window else { return };
        let size = win.inner_size();
        let (w, h) = (size.width as i32, size.height as i32);
        let s = self.scale;
        let pad = self.s(PAD);
        let by = h - pad - self.s(BTN_H);
        let list_w = w - pad * 2;
        self.list = Rect::new(pad, pad, list_w, by - pad * 2);
        let mut inv = Invalidations::default();
        let (bh, gap) = (self.s(BTN_H), self.s(6.0));
        let bw = self.s(BTN_W);
        // Path Finder 꼴: [취소] [OK] 오른쪽 정렬 · OK가 맨 오른쪽.
        self.ok_btn.set_scale(s);
        self.ok_btn
            .set_bounds(Rect::new(w - pad - bw, by, bw, bh), &mut inv);
        self.cancel_btn.set_scale(s);
        self.cancel_btn
            .set_bounds(Rect::new(w - pad - bw * 2 - gap, by, bw, bh), &mut inv);
        self.clamp_scroll();
    }

    fn row_h(&self) -> i32 {
        self.s(ROW_H)
    }

    fn clamp_scroll(&mut self) {
        let max = (self.names.len() as i32 * self.row_h() - self.list.h).max(0);
        self.scroll = self.scroll.clamp(0, max);
    }

    fn ensure_visible(&mut self, row: usize) {
        if self.list.h <= 0 {
            return;
        }
        let top = row as i32 * self.row_h();
        if top < self.scroll {
            self.scroll = top;
        } else if top + self.row_h() > self.scroll + self.list.h {
            self.scroll = top + self.row_h() - self.list.h;
        }
        self.clamp_scroll();
    }

    fn row_at(&self, p: Point) -> Option<usize> {
        if !self.list.contains(p) {
            return None;
        }
        let i = (p.y - self.list.y + self.scroll) / self.row_h();
        (i >= 0 && (i as usize) < self.names.len()).then_some(i as usize)
    }

    /// 드래그 중 놓일 자리(`move_to` 대상 행) — 커서 y(목록 밖 = 끝자리).
    fn drop_target(&self) -> Option<usize> {
        let d = self.drag.as_ref().filter(|d| d.active)?;
        if self.names.is_empty() {
            return None;
        }
        let yy = d.cur_y.clamp(self.list.y, self.list.bottom() - 1);
        Some(
            self.row_at(Point {
                x: self.list.x + 1,
                y: yy,
            })
            .unwrap_or(self.names.len() - 1),
        )
    }

    /// 인디케이터 y(강조색 가로줄): 아래로 옮기면 대상 행 **아래** · 위로 옮기면 대상 행 **위**(`move_to` 결과와 같은 자리).
    fn drop_indicator_y(&self) -> Option<i32> {
        let to = self.drop_target()?;
        let from = self.sel?;
        let top = self.list.y + to as i32 * self.row_h() - self.scroll;
        let y = if to > from { top + self.row_h() } else { top };
        (y >= self.list.y && y <= self.list.bottom())
            .then_some(y.clamp(self.list.y + 1, self.list.bottom() - 1))
    }

    /// 행 오른쪽 ⊖ 칸.
    fn del_rect(&self, row: usize) -> Rect {
        let y = self.list.y + row as i32 * self.row_h() - self.scroll;
        Rect::new(
            self.list.right() - self.s(DEL_W),
            y,
            self.s(DEL_W),
            self.row_h(),
        )
    }

    pub(crate) fn handle(&mut self, ev: &WindowEvent) -> PresetAction {
        match ev {
            WindowEvent::CloseRequested => {
                self.close();
                return PresetAction::None;
            }
            WindowEvent::Resized(_) => {
                self.layout();
                self.redraw();
                return PresetAction::None;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.layout();
                self.redraw();
                return PresetAction::None;
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.primary = if cfg!(target_os = "macos") {
                    m.state().super_key()
                } else {
                    m.state().control_key()
                };
                return PresetAction::None;
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as i32, position.y as i32);
                let (x, y) = self.cursor;
                if let Some(d) = self.drag.as_mut() {
                    d.cur_y = y;
                    if !d.active && (y - d.press_y).abs() > DRAG_THRESHOLD {
                        d.active = true;
                    }
                }
                let mv = InputEvent::MouseMove { x, y };
                let mut inv = Invalidations::default();
                for b in self.buttons() {
                    b.on_event(&mv, &mut inv);
                }
                self.redraw();
                return PresetAction::None;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y * ROW_H * 3.0 * self.scale,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };
                self.scroll -= dy.round() as i32;
                self.clamp_scroll();
                self.redraw();
                return PresetAction::None;
            }
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                let act = self.on_key(&kev.logical_key);
                self.redraw();
                return act;
            }
            WindowEvent::RedrawRequested => return PresetAction::Paint,
            _ => {}
        }
        let (x, y) = self.cursor;
        let ie = match ev {
            WindowEvent::MouseInput { state, button, .. } => match (state, button) {
                (ElementState::Pressed, MouseButton::Left) => InputEvent::MouseDown {
                    x,
                    y,
                    shift: self.shift,
                    primary: false,
                },
                (ElementState::Released, MouseButton::Left) => InputEvent::MouseUp { x, y },
                _ => return PresetAction::None,
            },
            _ => return PresetAction::None,
        };
        let p = Point { x, y };
        match ie {
            InputEvent::MouseDown { .. } => {
                if let Some(i) = self.row_at(p) {
                    for b in self.buttons() {
                        b.set_focused(false);
                    }
                    if self.del_rect(i).contains(p) {
                        self.remove(i);
                        self.redraw();
                        return PresetAction::None;
                    }
                    self.select(i);
                    self.drag = Some(Drag {
                        press_y: y,
                        cur_y: y,
                        active: false,
                    });
                    self.redraw();
                    return PresetAction::None;
                }
            }
            InputEvent::MouseUp { .. } => {
                let target = self.drop_target();
                if let Some(d) = self.drag.take() {
                    if d.active {
                        // 놓일 자리 = 인디케이터와 같은 판정(목록 밖 = 끝자리).
                        if let Some(r) = target {
                            self.move_to(r);
                        }
                        self.redraw();
                        return PresetAction::None;
                    }
                }
            }
            _ => {}
        }
        let mut inv = Invalidations::default();
        for b in self.buttons() {
            b.on_event(&ie, &mut inv);
        }
        if let InputEvent::MouseDown { .. } = ie {
            for b in self.buttons() {
                let hit = b.bounds().contains(p);
                b.set_focused(hit);
            }
        }
        if self.cancel_btn.take_clicked() {
            self.close();
            return PresetAction::None;
        }
        if self.ok_btn.take_clicked() {
            return self.apply();
        }
        self.redraw();
        PresetAction::None
    }

    fn apply(&mut self) -> PresetAction {
        let names = self.names.clone();
        self.close();
        PresetAction::Apply(names)
    }

    /// 키: ↑/↓ 선택 · Ctrl+↑/↓ 이동(드래그와 같다) · Delete = 선택 삭제 · Enter = 확인(기본 버튼) · Esc = 드래그 취소 → 아니면 닫기.
    fn on_key(&mut self, k: &Key) -> PresetAction {
        match k.as_ref() {
            Key::Named(NamedKey::Escape) => {
                if self.drag.take().is_some_and(|d| d.active) {
                    return PresetAction::None;
                }
                self.close();
                PresetAction::None
            }
            Key::Named(NamedKey::Enter) => self.apply(),
            Key::Named(NamedKey::Delete) => {
                if let Some(i) = self.sel {
                    self.remove(i);
                }
                PresetAction::None
            }
            Key::Named(NamedKey::ArrowUp | NamedKey::ArrowDown) => {
                let up = matches!(k.as_ref(), Key::Named(NamedKey::ArrowUp));
                if self.primary {
                    self.move_sel(up);
                } else if let Some(i) = self.sel {
                    let j = if up {
                        i.saturating_sub(1)
                    } else {
                        (i + 1).min(self.names.len().saturating_sub(1))
                    };
                    self.select(j);
                }
                PresetAction::None
            }
            _ => PresetAction::None,
        }
    }

    pub(crate) fn paint(&mut self, ui: &Font, th: &Theme, font_px: f32) {
        let Some(win) = self.window.clone() else {
            return;
        };
        let size = win.inner_size();
        let s = self.scale;
        let (wi, hi) = (size.width as i32, size.height as i32);
        let list = self.list;
        let row_h = self.row_h();
        let del_w = self.s(DEL_W);
        let ghost = self.drag.as_ref().filter(|d| d.active).map(|d| d.cur_y);
        let indicator = self.drop_indicator_y();
        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        let Some(mut buf) = surface.frame(size) else {
            return;
        };
        {
            let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
            let prefs = FontPrefs::with_base(font_px);
            let mut dc = RasterCtx::new(&mut gfx, ui, s).with_fonts(prefs);
            dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
            dc.select_font(FontSlot::Base, false);
            let th_txt = dc.text_height();
            dc.fill_rect(list, th.panel_bg);
            if self.names.is_empty() {
                let t = tr("bulk.preset.empty");
                let tw = dc.text_width(&t);
                dc.text(
                    list.x + (list.w - tw) / 2,
                    list.y + (list.h - th_txt) / 2,
                    list,
                    &t,
                    th.text_dim,
                );
            }
            let first = (self.scroll / row_h).max(0) as usize;
            for (i, name) in self.names.iter().enumerate().skip(first) {
                let y = list.y + i as i32 * row_h - self.scroll;
                if y >= list.bottom() {
                    break;
                }
                let rr = Rect::new(list.x, y, list.w, row_h);
                let clip = rr.intersection(&list);
                if clip.h <= 0 {
                    continue;
                }
                if self.sel == Some(i) {
                    dc.fill_rect(clip, th.sel_bg);
                }
                let x = list.x + (8.0 * s).round() as i32;
                let text_clip = Rect::new(clip.x, clip.y, (list.w - del_w).max(0), clip.h);
                dc.text(x, y + (row_h - th_txt) / 2, text_clip, name, th.text);
                // ⊖ — 빨간 원 + 흰 막대(Path Finder 꼴 · 글꼴에 기대지 않는다).
                let d = (14.0 * s).round() as i32;
                let cx = list.right() - del_w / 2;
                let cy = y + row_h / 2;
                let circle = Rect::new(cx - d / 2, cy - d / 2, d, d).intersection(&list);
                if circle.h == d {
                    dc.fill_round_rect(circle, d / 2, th.danger);
                    let bar_w = (8.0 * s).round() as i32;
                    let bar_h = (2.0 * s).round() as i32;
                    dc.fill_rect(
                        Rect::new(cx - bar_w / 2, cy - bar_h / 2, bar_w, bar_h),
                        th.window_bg,
                    );
                }
                // 행 사이 가는 줄.
                let sep_y = y + row_h - 1;
                if sep_y < list.bottom() {
                    dc.fill_rect(Rect::new(list.x, sep_y, list.w, 1), th.border);
                }
            }
            dc.stroke_round_rect(list, 0, th.border, 1.0);
            // 놓일 자리 인디케이터(강조색 2px · 사용자 10-18).
            if let Some(iy) = indicator {
                dc.fill_rect(Rect::new(list.x, iy - 1, list.w, 2), th.accent);
            }
            // 드래그 고스트(커서 y 추종).
            if let (Some(cy), Some(i)) = (ghost, self.sel) {
                let text = self.names.get(i).cloned().unwrap_or_default();
                let gy = (cy - row_h / 2).clamp(list.y, (list.bottom() - row_h).max(list.y));
                let grc = Rect::new(
                    list.x + (6.0 * s).round() as i32,
                    gy,
                    list.w - (12.0 * s).round() as i32,
                    row_h,
                );
                dc.fill_rect(grc, th.sel_bg);
                dc.stroke_round_rect(grc, 2, th.accent, 1.0);
                dc.text(
                    grc.x + (8.0 * s).round() as i32,
                    gy + (row_h - th_txt) / 2,
                    grc,
                    &text,
                    th.text,
                );
            }
            for b in [&mut self.ok_btn, &mut self.cancel_btn] {
                b.paint(&mut dc, th);
            }
        }
        let _ = buf.present();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 창 없이 모델(10-18): 순서 이동(▲▼ · 끝에서 멈춤 · 드래그 놓기) · 개별 삭제(선택 = 같은 자리 다음) · 비면 선택 없음 · 덤프.
    #[test]
    fn model_moves_and_removes_without_window() {
        let mut w = PresetWin::new();
        w.set(vec!["a".into(), "b".into(), "c".into()]);
        assert_eq!(w.dump(), "closed sel=0 [a|b|c]\n");
        assert!(!w.move_sel(true), "맨 위에서 Ctrl+↑ = 없음");
        assert!(w.move_sel(false));
        assert_eq!(w.dump(), "closed sel=1 [b|a|c]\n");
        w.select(0);
        assert!(w.move_to(2), "드래그 놓기 = 맨 끝으로");
        assert_eq!(w.names(), ["a", "c", "b"]);
        assert!(!w.move_to(2), "같은 자리 = 변화 없음");
        assert!(w.remove(1));
        assert_eq!(
            w.dump(),
            "closed sel=1 [a|b]\n",
            "삭제 뒤 선택 = 같은 자리의 다음"
        );
        assert!(w.remove(1) && w.remove(0));
        assert_eq!(w.dump(), "closed sel=- []\n");
        assert!(!w.remove(0) && !w.move_sel(false));
    }
}
