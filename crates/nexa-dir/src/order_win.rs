//! 순서/표시 편집 창(dir2 `ordereditor.rs` + `ctl/ordertree.rs` 이식 · T-71 DLG-069~073) — 창 1개가 [`OrderSpec`] 어댑터로 세 용도
//! (도구 모음 · 파일 컬럼(`flat` · `name` 잠금) · 컨텍스트 메뉴)를 구성한다. `keys_win.rs`와 같은 보조 창 골격.
//!
//! 규약: 그룹(레벨 0) 선택 = 블록 통째 이동 · 자식 = 그룹 안에서만 · Shift = 같은 부모 형제 범위(혼합 차단) · ▲▼/Ctrl+↑↓/드래그 = 이동 ·
//! Space/체크 클릭 = 표시 토글(잠금 key 거부 · 그룹 체크 = 통째 숨김 + 자식 상태 보존 · 부모 숨김이면 자식 체크 비활성) ·
//! Esc = 드래그 취소 → 아니면 닫기 · 변경할 때마다 [`OrderAction::Changed`]로 소유자에 **실시간 통지**(DLG-073 · OK/취소 없음).
//! dir2의 그룹 접기(셰브론)는 두지 않았다(행이 최대 12 — 스크롤로 충분).

use crate::order::{self, OrderBlock, OrderDefs};
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

/// 편집기 어댑터(DLG-069) — 세 설정 창의 차이를 데이터로.
#[derive(Clone)]
pub(crate) struct OrderSpec {
    pub title: String,
    /// 설정 키(통지에 같이 돌려준다 · `toolbar.layout` / `list.col_layout` / `ctxmenu.layout`).
    pub key: String,
    pub defs: OrderDefs,
    /// 표시 열(체크) 사용 — 직렬화에 `:0/1` 포함.
    pub with_vis: bool,
    /// 단일 블록 평면(그룹 헤더 생략 — 컬럼).
    pub flat: bool,
    /// 체크 잠금 key(해제 불가 — 컬럼 `name`).
    pub locked: &'static [&'static str],
    /// (블록, 자식) → 라벨(i18n).
    pub label: fn(&str, Option<&str>) -> String,
}

/// 창이 호스트에 요청하는 것.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OrderAction {
    None,
    Paint,
    /// 값이 바뀜(정규화한 직렬화 문자열 · 즉시 적용·저장).
    Changed {
        key: String,
        value: String,
    },
}

const PAD: f32 = 12.0;
const ROW_H: f32 = 24.0;
const BTN_H: f32 = 28.0;
const LIST_W: f32 = 300.0;
const SIDE_W: f32 = 36.0;
const CHECK_W: f32 = 20.0;
const INDENT: f32 = 18.0;
const MAX_ROWS: usize = 12;
const DRAG_THRESHOLD: i32 = 5;

/// 표시 행 — (라벨, 레벨 0/1, 체크 Option).
type Row = (String, u8, Option<bool>);

struct Drag {
    press_y: i32,
    cur_y: i32,
    active: bool,
}

pub(crate) struct OrderWin {
    window: Option<Rc<Window>>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    cursor: (i32, i32),
    shift: bool,
    primary: bool,
    spec: Option<OrderSpec>,
    model: Vec<OrderBlock>,
    /// 선택(index 오름차순 — 항상 같은 부모의 형제).
    sel: Vec<usize>,
    anchor: Option<usize>,
    scroll: i32,
    list: Rect,
    drag: Option<Drag>,
    up_btn: Button,
    down_btn: Button,
    close_btn: Button,
}

impl OrderWin {
    pub(crate) fn new() -> Self {
        OrderWin {
            window: None,
            surface: None,
            scale: 1.0,
            cursor: (0, 0),
            shift: false,
            primary: false,
            spec: None,
            model: Vec::new(),
            sel: Vec::new(),
            anchor: None,
            scroll: 0,
            list: Rect::default(),
            drag: None,
            up_btn: Button::new("▲"),
            down_btn: Button::new("▼"),
            close_btn: Button::new(tr("pref.btn.close")),
        }
    }

    /// 용도 + 현재 값(열 때 · 밖에서 값이 바뀌었을 때).
    pub(crate) fn set(&mut self, spec: OrderSpec, value: &str) {
        self.model = order::parse_order_with(spec.defs, value);
        self.spec = Some(spec);
        self.sel.clear();
        self.anchor = None;
        self.scroll = 0;
        self.drag = None;
        self.sync_buttons();
    }

    pub(crate) fn setting_key(&self) -> Option<&str> {
        self.spec.as_ref().map(|s| s.key.as_str())
    }

    /// 현재 값(정규화 직렬화).
    pub(crate) fn value(&self) -> String {
        let with_vis = self.spec.as_ref().is_some_and(|s| s.with_vis);
        order::serialize_order_with(&self.model, with_vis)
    }

    // ── 모델 ──

    fn rows(&self) -> Vec<Row> {
        let Some(sp) = &self.spec else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        for (block, bvis, items) in &self.model {
            if sp.flat {
                for (k, vis) in items {
                    rows.push(((sp.label)(block, Some(k)), 0, sp.with_vis.then_some(*vis)));
                }
            } else {
                rows.push(((sp.label)(block, None), 0, sp.with_vis.then_some(*bvis)));
                for (k, vis) in items {
                    rows.push(((sp.label)(block, Some(k)), 1, sp.with_vis.then_some(*vis)));
                }
            }
        }
        rows
    }

    /// 행 index → (블록 index, 자식 index — 그룹 헤더 = None).
    fn row_map(&self) -> Vec<(usize, Option<usize>)> {
        let flat = self.spec.as_ref().is_some_and(|s| s.flat);
        let mut map = Vec::new();
        for (bi, (_, _, items)) in self.model.iter().enumerate() {
            if !flat {
                map.push((bi, None));
            }
            for ii in 0..items.len() {
                map.push((bi, Some(ii)));
            }
        }
        map
    }

    /// 같은 부모 형제인가(그룹끼리 · 같은 블록의 자식끼리).
    fn siblings(map: &[(usize, Option<usize>)], a: usize, b: usize) -> bool {
        match (map.get(a), map.get(b)) {
            (Some((_, None)), Some((_, None))) => true,
            (Some((ba, Some(_))), Some((bb, Some(_)))) => ba == bb,
            _ => false,
        }
    }

    /// 부모 그룹이 숨김이면 자식 체크 비활성(값 유지).
    fn check_disabled(&self, row: usize) -> bool {
        let map = self.row_map();
        match map.get(row) {
            Some(&(bi, Some(_))) => {
                !self.spec.as_ref().is_some_and(|s| s.flat) && !self.model[bi].1
            }
            _ => false,
        }
    }

    /// 선택 이동(DLG-070) — 그룹 = 블록 통째 · 자식 = 그룹 안. 이동 후 같은 항목 재선택. 바뀌었으면 true.
    pub(crate) fn move_sel(&mut self, up: bool) -> bool {
        if self.sel.is_empty() {
            return false;
        }
        let map = self.row_map();
        let moved: Vec<(usize, Option<usize>)> = self
            .sel
            .iter()
            .filter_map(|&r| map.get(r).copied())
            .collect();
        let Some(&(bi0, ii0)) = moved.first() else {
            return false;
        };
        let ok = match ii0 {
            None => {
                let bis: Vec<usize> = moved.iter().map(|(b, _)| *b).collect();
                order::shift_range(&mut self.model, &bis, up)
            }
            Some(_) => {
                let iis: Vec<usize> = moved.iter().filter_map(|(_, i)| *i).collect();
                order::shift_range(&mut self.model[bi0].2, &iis, up)
            }
        };
        if !ok {
            return false;
        }
        let new_map = self.row_map();
        let delta = |v: usize| if up { v - 1 } else { v + 1 };
        self.sel = match ii0 {
            None => {
                let bis: Vec<usize> = moved.iter().map(|(b, _)| delta(*b)).collect();
                new_map
                    .iter()
                    .enumerate()
                    .filter(|(_, (b, i))| i.is_none() && bis.contains(b))
                    .map(|(r, _)| r)
                    .collect()
            }
            Some(_) => {
                let iis: Vec<usize> = moved.iter().filter_map(|(_, i)| *i).map(delta).collect();
                new_map
                    .iter()
                    .enumerate()
                    .filter(|(_, (b, i))| *b == bi0 && i.is_some_and(|x| iis.contains(&x)))
                    .map(|(r, _)| r)
                    .collect()
            }
        };
        self.anchor = self.sel.first().copied();
        if let Some(&f) = self.sel.first() {
            self.ensure_visible(f);
        }
        true
    }

    /// 체크 토글(DLG-071) — 잠금 key 거부 · 부모 숨김 거부 · 그룹 = 통째(자식 보존). 바뀌었으면 true.
    pub(crate) fn toggle_row(&mut self, row: usize) -> bool {
        let Some(sp) = self.spec.clone() else {
            return false;
        };
        if !sp.with_vis || self.check_disabled(row) {
            return false;
        }
        let map = self.row_map();
        match map.get(row) {
            Some(&(bi, Some(ii))) => {
                if sp.locked.contains(&self.model[bi].2[ii].0.as_str()) {
                    return false;
                }
                self.model[bi].2[ii].1 = !self.model[bi].2[ii].1;
                true
            }
            Some(&(bi, None)) => {
                self.model[bi].1 = !self.model[bi].1;
                true
            }
            None => false,
        }
    }

    fn select_single(&mut self, row: usize) {
        self.sel = vec![row];
        self.anchor = Some(row);
        self.ensure_visible(row);
        self.sync_buttons();
    }

    /// Shift 범위 — 앵커와 같은 부모 형제만(혼합 차단).
    fn select_range_to(&mut self, row: usize) {
        let map = self.row_map();
        let Some(a) = self.anchor else {
            self.select_single(row);
            return;
        };
        if !Self::siblings(&map, a, row) {
            return;
        }
        let (lo, hi) = (a.min(row), a.max(row));
        self.sel = (lo..=hi).filter(|&j| Self::siblings(&map, a, j)).collect();
        self.ensure_visible(row);
        self.sync_buttons();
    }

    /// ↑/↓ = 단일 선택 이동(없으면 첫/마지막).
    fn key_move(&mut self, up: bool) {
        let n = self.row_map().len();
        if n == 0 {
            return;
        }
        let next = match self.anchor.or_else(|| self.sel.first().copied()) {
            None => {
                if up {
                    n - 1
                } else {
                    0
                }
            }
            Some(c) => {
                if up {
                    c.saturating_sub(1)
                } else {
                    (c + 1).min(n - 1)
                }
            }
        };
        self.select_single(next);
    }

    /// Shift+↑/↓ = 형제 범위 확장/축소.
    fn key_extend(&mut self, up: bool) {
        let map = self.row_map();
        let Some(a) = self.anchor else {
            self.key_move(up);
            return;
        };
        let end = if up {
            self.sel.first().copied().unwrap_or(a)
        } else {
            self.sel.last().copied().unwrap_or(a)
        };
        let target = if up {
            (0..end).rev().find(|&j| Self::siblings(&map, a, j))
        } else {
            (end + 1..map.len()).find(|&j| Self::siblings(&map, a, j))
        };
        if let Some(t) = target {
            self.select_range_to(t);
        }
    }

    fn sync_buttons(&mut self) {
        let on = !self.sel.is_empty();
        self.up_btn.set_enabled(on);
        self.down_btn.set_enabled(on);
    }

    /// 변경 통지(DLG-073).
    fn changed(&self) -> OrderAction {
        match &self.spec {
            Some(sp) => OrderAction::Changed {
                key: sp.key.clone(),
                value: self.value(),
            },
            None => OrderAction::None,
        }
    }

    /// 덤프(`order.dump:`) — 제목 · 행(`[x]`/`[ ]`/`[-]` 비활성 · 들여쓰기 `  ` · 선택 `*`) · 값.
    pub(crate) fn dump(&self) -> String {
        let Some(sp) = &self.spec else {
            return "none\n".into();
        };
        let mut out = format!("{} key {}\n", sp.title, sp.key);
        for (i, (label, level, check)) in self.rows().iter().enumerate() {
            let c = match check {
                Some(_) if self.check_disabled(i) => "[-] ",
                Some(true) => "[x] ",
                Some(false) => "[ ] ",
                None => "",
            };
            let s = if self.sel.contains(&i) { "*" } else { " " };
            out.push_str(&format!("{s}{}{c}{label}\n", "  ".repeat(*level as usize)));
        }
        out.push_str(&format!("value {}\n", self.value()));
        out
    }

    // ── 창 ──

    fn s(&self, v: f32) -> i32 {
        (v * self.scale).round() as i32
    }

    pub(crate) fn is(&self, id: WindowId) -> bool {
        self.window.as_ref().is_some_and(|w| w.id() == id)
    }

    pub(crate) fn is_open(&self) -> bool {
        self.window.is_some()
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
        any
    }

    pub(crate) fn animating(&self) -> bool {
        self.window.is_some()
            && [&self.up_btn, &self.down_btn, &self.close_btn]
                .iter()
                .any(|b| b.is_animating())
    }

    fn buttons(&mut self) -> [&mut Button; 3] {
        [&mut self.up_btn, &mut self.down_btn, &mut self.close_btn]
    }

    /// 창 논리 크기 — 행 수(≤ 12)에 맞춘다.
    fn logical_size(&self) -> (f64, f64) {
        let n = self.row_map().len().clamp(1, MAX_ROWS) as f32;
        let w = PAD * 3.0 + LIST_W + SIDE_W;
        let h = PAD * 3.0 + n * ROW_H + 2.0 + BTN_H;
        (w as f64, h as f64)
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
            return;
        }
        let title = self
            .spec
            .as_ref()
            .map(|s| s.title.clone())
            .unwrap_or_default();
        let (lw, lh) = self.logical_size();
        let mut attrs = Window::default_attributes()
            .with_title(format!("Nexa Dir — {title}"))
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
        let win = Rc::new(win);
        self.scale = win.scale_factor() as f32;
        self.surface = crate::present::Presenter::new(win.clone()).ok();
        self.window = Some(win);
        self.sync_buttons();
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
        let list_w = w - pad * 3 - self.s(SIDE_W);
        self.list = Rect::new(pad, pad, list_w, by - pad * 2);
        let mut inv = Invalidations::default();
        let side_x = pad * 2 + list_w;
        let sw = self.s(SIDE_W);
        let (bh, gap) = (self.s(BTN_H), self.s(6.0));
        for (i, b) in [&mut self.up_btn, &mut self.down_btn]
            .into_iter()
            .enumerate()
        {
            b.set_scale(s);
            b.set_bounds(
                Rect::new(side_x, pad + i as i32 * (bh + gap), sw, bh),
                &mut inv,
            );
        }
        let bw = self.s(96.0);
        self.close_btn.set_scale(s);
        self.close_btn
            .set_bounds(Rect::new(w - pad - bw, by, bw, self.s(BTN_H)), &mut inv);
        self.clamp_scroll();
    }

    fn row_h(&self) -> i32 {
        self.s(ROW_H)
    }

    fn content_h(&self) -> i32 {
        self.row_map().len() as i32 * self.row_h()
    }

    fn clamp_scroll(&mut self) {
        let max = (self.content_h() - self.list.h).max(0);
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
        (i >= 0 && (i as usize) < self.row_map().len()).then_some(i as usize)
    }

    /// 드래그 확정 — 커서 행과 블록 첫 행의 형제 칸 차이만큼 한 칸 이동 반복(DLG-070).
    fn apply_drag_to(&mut self, row: usize) -> bool {
        let map = self.row_map();
        let Some(&first) = self.sel.first() else {
            return false;
        };
        let Some(&last) = self.sel.last() else {
            return false;
        };
        if (first..=last).contains(&row) {
            return false;
        }
        // 목표 행이 형제가 아니면(자식 위에 떨어진 그룹 드래그 등) 그 행의 부모/형제 자리로 환산.
        let target = if Self::siblings(&map, first, row) {
            Some(row)
        } else if matches!(map.get(first), Some((_, None))) {
            map.get(row)
                .and_then(|&(bi, _)| map.iter().position(|&(b, i)| b == bi && i.is_none()))
        } else {
            None
        };
        let Some(target) = target else {
            return false;
        };
        let pos = |r: usize| (0..r).filter(|&j| Self::siblings(&map, first, j)).count() as i32;
        let delta = pos(target) - pos(first);
        let mut changed = false;
        for _ in 0..delta.abs() {
            if !self.move_sel(delta < 0) {
                break;
            }
            changed = true;
        }
        changed
    }

    pub(crate) fn handle(&mut self, ev: &WindowEvent) -> OrderAction {
        match ev {
            WindowEvent::CloseRequested => {
                self.close();
                return OrderAction::None;
            }
            WindowEvent::Resized(_) => {
                self.layout();
                self.redraw();
                return OrderAction::None;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.layout();
                self.redraw();
                return OrderAction::None;
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.primary = if cfg!(target_os = "macos") {
                    m.state().super_key()
                } else {
                    m.state().control_key()
                };
                return OrderAction::None;
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
                return OrderAction::None;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y * ROW_H * 3.0 * self.scale,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };
                self.scroll -= dy.round() as i32;
                self.clamp_scroll();
                self.redraw();
                return OrderAction::None;
            }
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                let act = self.on_key(&kev.logical_key);
                self.redraw();
                return act;
            }
            WindowEvent::RedrawRequested => return OrderAction::Paint,
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
                _ => return OrderAction::None,
            },
            _ => return OrderAction::None,
        };
        let p = Point { x, y };
        match ie {
            InputEvent::MouseDown { .. } => {
                if let Some(i) = self.row_at(p) {
                    for b in self.buttons() {
                        b.set_focused(false);
                    }
                    let check_x = self.list.x + self.s(4.0);
                    let in_check = self.rows().get(i).is_some_and(|r| r.2.is_some())
                        && x >= check_x
                        && x < check_x + self.s(CHECK_W);
                    if in_check {
                        let act = if self.toggle_row(i) {
                            self.changed()
                        } else {
                            OrderAction::None
                        };
                        self.redraw();
                        return act;
                    }
                    if self.shift {
                        self.select_range_to(i);
                    } else {
                        if !self.sel.contains(&i) {
                            self.select_single(i);
                        }
                        self.drag = Some(Drag {
                            press_y: y,
                            cur_y: y,
                            active: false,
                        });
                    }
                    self.redraw();
                    return OrderAction::None;
                }
            }
            InputEvent::MouseUp { .. } => {
                if let Some(d) = self.drag.take() {
                    if d.active {
                        let act = match self.row_at(Point {
                            x: self.list.x + 1,
                            y,
                        }) {
                            Some(r) if self.apply_drag_to(r) => self.changed(),
                            _ => OrderAction::None,
                        };
                        self.redraw();
                        return act;
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
        let mut act = OrderAction::None;
        if self.up_btn.take_clicked() && self.move_sel(true) {
            act = self.changed();
        }
        if self.down_btn.take_clicked() && self.move_sel(false) {
            act = self.changed();
        }
        if self.close_btn.take_clicked() {
            self.close();
            return OrderAction::None;
        }
        self.redraw();
        act
    }

    /// 키보드(DLG-072): ↑/↓ 선택 · Shift 범위 · Ctrl 이동 · Space 토글 · Esc = 드래그 취소 → 닫기.
    fn on_key(&mut self, k: &Key) -> OrderAction {
        match k.as_ref() {
            Key::Named(NamedKey::Escape) => {
                if self.drag.take().is_some_and(|d| d.active) {
                    return OrderAction::None;
                }
                self.close();
                OrderAction::None
            }
            Key::Named(NamedKey::ArrowUp | NamedKey::ArrowDown) => {
                let up = matches!(k.as_ref(), Key::Named(NamedKey::ArrowUp));
                if self.primary {
                    if self.move_sel(up) {
                        return self.changed();
                    }
                } else if self.shift {
                    self.key_extend(up);
                } else {
                    self.key_move(up);
                }
                OrderAction::None
            }
            Key::Named(NamedKey::Space) => {
                if let [one] = self.sel[..] {
                    if self.toggle_row(one) {
                        return self.changed();
                    }
                }
                OrderAction::None
            }
            _ => OrderAction::None,
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
        let row_h = (ROW_H * s).round() as i32;
        let rows = self.rows();
        let disabled: Vec<bool> = (0..rows.len()).map(|i| self.check_disabled(i)).collect();
        let check_w = (CHECK_W * s).round() as i32;
        let indent = (INDENT * s).round() as i32;
        let ghost = self.drag.as_ref().filter(|d| d.active).map(|d| d.cur_y);
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
            let first = (self.scroll / row_h).max(0) as usize;
            for (i, (label, level, check)) in rows.iter().enumerate().skip(first) {
                let y = list.y + i as i32 * row_h - self.scroll;
                if y >= list.bottom() {
                    break;
                }
                let rr = Rect::new(list.x, y, list.w, row_h);
                let clip = rr.intersection(&list);
                if clip.h <= 0 {
                    continue;
                }
                if self.sel.contains(&i) {
                    dc.fill_rect(clip, th.sel_bg);
                }
                let dim = disabled[i];
                let mut x = list.x + (4.0 * s).round() as i32;
                if let Some(on) = check {
                    let bs = (12.0 * s).round() as i32;
                    let brc = Rect::new(x + (check_w - bs) / 2, y + (row_h - bs) / 2, bs, bs);
                    dc.stroke_round_rect(brc, 2, if dim { th.text_dim } else { th.border }, 1.0);
                    if *on {
                        let k = (3.0 * s).round() as i32;
                        dc.fill_rect(
                            Rect::new(brc.x + k, brc.y + k, bs - 2 * k, bs - 2 * k),
                            if dim { th.text_dim } else { th.accent },
                        );
                    }
                    x += check_w;
                }
                x += (4.0 * s).round() as i32 + *level as i32 * indent;
                let color = if dim || *level == 1 {
                    th.text_dim
                } else {
                    th.text
                };
                dc.text(x, y + (row_h - th_txt) / 2, clip, label, color);
            }
            dc.stroke_round_rect(list, 0, th.border, 1.0);
            // 드래그 고스트(커서 y 추종).
            if let Some(cy) = ghost {
                let n = self.sel.len();
                let label0 = self
                    .sel
                    .first()
                    .and_then(|&i| rows.get(i))
                    .map(|r| r.0.clone())
                    .unwrap_or_default();
                let text = if n > 1 {
                    format!("{label0} (+{})", n - 1)
                } else {
                    label0
                };
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
            for b in [&mut self.up_btn, &mut self.down_btn, &mut self.close_btn] {
                b.paint(&mut dc, th);
            }
        }
        let _ = buf.present();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(block: &str, item: Option<&str>) -> String {
        item.map_or_else(|| block.to_uppercase(), str::to_string)
    }

    fn toolbar_win() -> OrderWin {
        let mut w = OrderWin::new();
        w.set(
            OrderSpec {
                title: "tb".into(),
                key: "toolbar.layout".into(),
                // 컨트롤 동작 시험용 고정 정의(실제 도구 모음 구성이 바뀌어도 행 번호가 흔들리지 않게).
                defs: &[
                    ("refresh", &[]),
                    ("panel", &["toggle", "dock", "info", "colsync", "ontop"]),
                    ("view", &["tree", "flat", "tiles"]),
                    ("show", &["hidden", "dot", "foldersfirst"]),
                    ("settings", &[]),
                ],
                with_vis: true,
                flat: false,
                locked: &[],
                label,
            },
            "",
        );
        w
    }

    #[test]
    fn rows_move_toggle_and_dump() {
        let mut w = toolbar_win();
        // 행 = 블록 5 + 자식 11.
        assert_eq!(w.rows().len(), 5 + 5 + 3 + 3);
        // 그룹 refresh(행 0)는 맨 위 — 위로 이동 무동작 · 아래로 = panel 블록(자식 5) 뒤로.
        w.select_single(0);
        assert!(!w.move_sel(true));
        assert!(w.move_sel(false));
        assert_eq!(w.sel, vec![6], "블록 통째 이동 뒤 같은 항목 재선택");
        assert!(w
            .value()
            .starts_with("panel:1[toggle:1,dock:1,info:1,colsync:1,ontop:1]|refresh:1|"));
        // 자식 toggle(행 1)은 그룹 안에서만: 맨 위라 위로 무동작 · 아래로 한 칸.
        w.select_single(1);
        assert!(!w.move_sel(true));
        assert!(w.move_sel(false));
        assert_eq!(w.sel, vec![2]);
        assert!(w.value().starts_with("panel:1[dock:1,toggle:1,"));
        // Shift 범위 = 같은 부모 형제만(그룹 행 섞이면 거부).
        w.select_single(2);
        w.select_range_to(4);
        assert_eq!(w.sel, vec![2, 3, 4]);
        w.select_range_to(0);
        assert_eq!(w.sel, vec![2, 3, 4], "그룹 행과 혼합 차단");
        assert!(w.move_sel(false));
        assert_eq!(w.sel, vec![3, 4, 5], "연속 다중 일괄 한 칸");
        assert!(!w.move_sel(false), "경계면 무동작");
        // 그룹 체크 해제 = 통째 숨김 · 자식 상태 보존 · 자식 체크 비활성.
        assert!(w.toggle_row(0));
        assert!(w.value().starts_with("panel:0[dock:1,"));
        assert!(w.check_disabled(1));
        assert!(!w.toggle_row(1), "부모 숨김 = 자식 토글 거부");
        let d = w.dump();
        assert!(d.contains("[ ] PANEL") && d.contains("  [-] dock"), "{d}");
        assert!(w.toggle_row(0));
        assert!(w.toggle_row(1));
        assert!(w.value().starts_with("panel:1[dock:0,"));
        // 키보드 선택 이동 + Shift 확장.
        w.sel.clear();
        w.anchor = None;
        w.key_move(false);
        assert_eq!(w.sel, vec![0]);
        w.key_move(true);
        assert_eq!(w.sel, vec![0]);
        w.select_single(7);
        w.key_extend(false);
        assert_eq!(
            w.sel,
            vec![7, 11],
            "그룹 행의 형제 = 다음 그룹(자식 건너뜀)"
        );
        w.select_single(8);
        w.key_extend(false);
        assert_eq!(w.sel, vec![8, 9]);
    }

    #[test]
    fn flat_columns_lock_name_and_drag_delta() {
        let mut w = OrderWin::new();
        w.set(
            OrderSpec {
                title: "cols".into(),
                key: "list.col_layout".into(),
                defs: order::COLUMN_BLOCKS,
                with_vis: true,
                flat: true,
                locked: &["name"],
                label,
            },
            "cols:1[ext:1,name:1,size:0]",
        );
        assert_eq!(w.rows().len(), 6, "평면 = 그룹 헤더 없음");
        // 저장값에 없던 항목은 정의상 앞 형제 뒤에 **기본 표시 여부**로 보충된다(상태 = 표시 · 종류 = 숨김).
        assert_eq!(
            w.value(),
            "cols:1[ext:1,kind:0,name:1,status:1,size:0,modified:1]"
        );
        assert!(!w.toggle_row(2), "name 잠금");
        assert!(w.toggle_row(4));
        assert!(w.value().contains("size:1"));
        // 드래그: 행 0(ext)을 행 3 자리로 = 아래로 3칸.
        w.select_single(0);
        assert!(w.apply_drag_to(3));
        assert_eq!(
            w.value(),
            "cols:1[kind:0,name:1,status:1,ext:1,size:1,modified:1]"
        );
        assert_eq!(w.sel, vec![3]);
        assert!(!w.apply_drag_to(3), "자기 자리 = 무동작");
        assert!(!w
            .handle(&WindowEvent::CloseRequested)
            .eq(&OrderAction::Paint));
    }
}
