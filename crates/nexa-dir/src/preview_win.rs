//! 독립 미리보기 창(T-62 B · dir2 `previewwnd.rs` 이식 — F3 · 도크 ↗ · docs/port/20 §1.4 PLUG-050~062).
//!
//! 스타일드 라인 렌더(`\u{2}tag|` 계약 · GitHub 근사): 0 본문 · 1~3 h1~h3(굵게 · h1/h2 밑줄 괘선) · 4 코드(밴드 + 모노) · 5 인용(바 + 흐림) ·
//! 6 모노(표·아트) · 7 수평선. 이미지 마커(`\u{1}`) 줄은 빈 행(인라인 이미지 그리기는 T-31). 세로/가로 스크롤(휠·키) · Esc 닫기 ·
//! Ctrl+C = 전체 텍스트 복사(드래그 문자 선택은 후속). 모달 아님(소유 창). 창 없이도 내용을 들고 덤프할 수 있다(시험·시나리오).

use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::Rect;
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_gfx::{Font, Surface};
use std::collections::HashMap;
use std::rc::Rc;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// 창이 호스트에 요청하는 것.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PvAction {
    None,
    Paint,
    /// 전체 텍스트 복사.
    Copy(String),
}

const PAD_X: f32 = 8.0;
const PAD_TOP: f32 = 4.0;

/// 인라인 이미지 종류(dir2 `IMG_MARKER`/`IMG_PAD` · T-62 C-2): 8 = `\u{1}img|<경로>`(본문 = 경로) · 9 = `\u{1}pad`(예약 행).
pub(crate) const KIND_IMG: u8 = 8;
pub(crate) const KIND_PAD: u8 = 9;

/// 라인 종류(dir2 `parse_kind`): 태그 접두를 벗기고 종류 번호를 돌려준다.
pub(crate) fn parse_kind(l: &str) -> (u8, &str) {
    if let Some(p) = l.strip_prefix("\u{1}img|") {
        return (KIND_IMG, p);
    }
    if l == "\u{1}pad" {
        return (KIND_PAD, "");
    }
    for (tag, k) in [
        ("\u{2}h1|", 1u8),
        ("\u{2}h2|", 2),
        ("\u{2}h3|", 3),
        ("\u{2}code|", 4),
        ("\u{2}q|", 5),
        ("\u{2}mono|", 6),
        ("\u{2}hr|", 7),
    ] {
        if let Some(r) = l.strip_prefix(tag) {
            return (k, r);
        }
    }
    (0, l)
}

pub(crate) struct PreviewWin {
    window: Option<Rc<Window>>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    shift: bool,
    primary: bool,
    title: String,
    kinds: Vec<u8>,
    text: Vec<String>,
    /// 첫 가시 줄 · 가로 픽셀 오프셋.
    top: i32,
    /// 세로 휠 분수 누적(줄).
    wheel_acc: f32,
    left: i32,
    /// paint가 잰 행 높이 · 최장 줄 폭(가로 상한) · 가시 줄 수.
    line_h: i32,
    max_w: i32,
    vis: i32,
    /// 드래그 문자 선택(dir2 PLUG-057): 커서 · `(앵커, 현재)` = (줄, 문자 경계) · 드래그 중 · 줄별 문자 경계 x 오프셋(paint 캐시 · `[0, w1, w1+w2, …]`) ·
    /// 평균 글자 폭(캐시 없는 줄의 근사) · 마지막 자동 스크롤 틱.
    cursor: (i32, i32),
    sel: Option<((usize, usize), (usize, usize))>,
    drag: bool,
    offsets: HashMap<usize, Vec<i32>>,
    char_w: i32,
    last_auto_ms: u64,
}

/// 자동 스크롤 틱(ms · dir2 TIMER_DRAG 50).
const AUTO_SCROLL_MS: u64 = 50;

impl PreviewWin {
    pub(crate) fn new() -> Self {
        PreviewWin {
            window: None,
            surface: None,
            scale: 1.0,
            shift: false,
            primary: false,
            title: String::new(),
            kinds: Vec::new(),
            text: Vec::new(),
            top: 0,
            wheel_acc: 0.0,
            left: 0,
            line_h: 20,
            max_w: 0,
            vis: 1,
            cursor: (0, 0),
            sel: None,
            drag: false,
            offsets: HashMap::new(),
            char_w: 7,
            last_auto_ms: 0,
        }
    }

    fn pad_x(&self) -> i32 {
        (PAD_X * self.scale).round() as i32
    }

    fn pad_top(&self) -> i32 {
        (PAD_TOP * self.scale).round() as i32
    }

    /// 줄의 문자 경계 x 오프셋(paint 캐시 · 없으면 평균 글자 폭 근사 — 자동 스크롤로 보이면 다음 paint가 채운다).
    fn offsets_of(&self, line: usize) -> Vec<i32> {
        if let Some(o) = self.offsets.get(&line) {
            return o.clone();
        }
        let n = self.text.get(line).map_or(0, |t| t.chars().count());
        (0..=n).map(|i| i as i32 * self.char_w).collect()
    }

    /// 점 → (줄, 최근접 문자 경계)(dir2 `hit` · 마지막 줄 아래 = 마지막 줄).
    pub(crate) fn hit(&self, x: i32, y: i32) -> (usize, usize) {
        if self.text.is_empty() {
            return (0, 0);
        }
        let row = self.top + (y - self.pad_top()).div_euclid(self.line_h.max(1));
        let line = row.clamp(0, self.text.len() as i32 - 1) as usize;
        let rel = x - self.pad_x() + self.left;
        let offs = self.offsets_of(line);
        let mut best = 0usize;
        let mut bd = i32::MAX;
        for (i, o) in offs.iter().enumerate() {
            let d = (o - rel).abs();
            if d < bd {
                bd = d;
                best = i;
            }
        }
        (line, best)
    }

    /// 정렬된 선택 구간.
    fn sel_range(&self) -> Option<((usize, usize), (usize, usize))> {
        let (a, c) = self.sel?;
        Some(if a <= c { (a, c) } else { (c, a) })
    }

    /// 선택 텍스트(줄 구분 `\r\n` · 이미지/패드 줄 제외 · 빈 선택 = None).
    pub(crate) fn selected_text(&self) -> Option<String> {
        let (lo, hi) = self.sel_range()?;
        if lo == hi || lo.0 >= self.text.len() {
            return None;
        }
        let chars_of = |l: usize| -> Vec<char> {
            if matches!(self.kinds.get(l), Some(&KIND_IMG) | Some(&KIND_PAD)) {
                Vec::new()
            } else {
                self.text[l].chars().collect()
            }
        };
        let (ll, lc) = lo;
        let (hl, hc) = (hi.0.min(self.text.len() - 1), hi.1);
        let mut out: Vec<String> = Vec::new();
        for l in ll..=hl {
            let cs = chars_of(l);
            let s = if l == ll { lc.min(cs.len()) } else { 0 };
            let e = if l == hl { hc.min(cs.len()) } else { cs.len() };
            out.push(cs[s..e].iter().collect());
        }
        let text = out.join("\r\n");
        (!text.is_empty()).then_some(text)
    }

    /// 전체 선택(Ctrl+A · dir2 `select_all`).
    pub(crate) fn select_all(&mut self) {
        if self.text.is_empty() {
            return;
        }
        let last = self.text.len() - 1;
        let end = if matches!(self.kinds[last], KIND_IMG | KIND_PAD) {
            0
        } else {
            self.text[last].chars().count()
        };
        self.sel = Some(((0, 0), (last, end)));
        self.drag = false;
        self.redraw();
    }

    /// 누름 = 앵커(시험·입력 공용).
    pub(crate) fn begin_drag(&mut self, x: i32, y: i32) {
        let pos = self.hit(x, y);
        self.sel = Some((pos, pos));
        self.drag = true;
        self.redraw();
    }

    /// 이동 = 확장 + 경계 밖이면 스크롤(dir2 `drag_track`).
    pub(crate) fn drag_to(&mut self, x: i32, y: i32) {
        if !self.drag {
            return;
        }
        let (w, h) = self
            .window
            .as_ref()
            .map_or((i32::MAX / 2, i32::MAX / 2), |w| {
                let s = w.inner_size();
                (s.width as i32, s.height as i32)
            });
        if y < 0 {
            self.top -= 1;
        } else if y >= h {
            self.top += 1;
        }
        if x < 0 {
            self.left -= (24.0 * self.scale) as i32;
        } else if x >= w {
            self.left += (24.0 * self.scale) as i32;
        }
        self.clamp();
        let pos = self.hit(x, y);
        if let Some((a, cur)) = self.sel {
            if cur != pos {
                self.sel = Some((a, pos));
            }
        }
        self.redraw();
    }

    /// 뗌 = 확정(이동 없는 클릭 = 선택 없음 — 도크 규약).
    pub(crate) fn end_drag(&mut self) {
        if !self.drag {
            return;
        }
        self.drag = false;
        if let Some((a, c)) = self.sel {
            if a == c {
                self.sel = None;
            }
        }
        self.redraw();
    }

    /// 틱: 드래그 중 커서가 창 밖이면 50 ms마다 계속 스크롤(dir2 TIMER_DRAG). 다시 그려야 하면 true.
    pub(crate) fn tick(&mut self, now_ms: u64) -> bool {
        if !self.drag
            || self.window.is_none()
            || now_ms.saturating_sub(self.last_auto_ms) < AUTO_SCROLL_MS
        {
            return false;
        }
        self.last_auto_ms = now_ms;
        let (x, y) = self.cursor;
        let (w, h) = self.window.as_ref().map_or((0, 0), |w| {
            let s = w.inner_size();
            (s.width as i32, s.height as i32)
        });
        if x < 0 || y < 0 || x >= w || y >= h {
            self.drag_to(x, y);
            return true;
        }
        false
    }

    pub(crate) fn animating(&self) -> bool {
        self.drag && self.window.is_some()
    }

    /// 내용 교체(태그 → 종류 · 탭 4칸 · 마커 줄은 빈 텍스트로 보관).
    pub(crate) fn set_lines(&mut self, title: &str, lines: Vec<String>) {
        self.title = title.to_string();
        self.kinds.clear();
        self.text.clear();
        for l in lines {
            let (k, body) = parse_kind(&l);
            if k == KIND_IMG || k == KIND_PAD {
                self.kinds.push(k);
                self.text.push(body.to_string());
            } else if l.starts_with('\u{1}') {
                self.kinds.push(0);
                self.text.push(String::new());
            } else {
                self.kinds.push(k);
                self.text.push(body.replace('\t', "    "));
            }
        }
        self.top = 0;
        self.left = 0;
        self.max_w = 0;
        self.sel = None;
        self.drag = false;
        self.offsets.clear();
        if let Some(w) = &self.window {
            w.set_title(&format!(
                "{} — {}",
                self.title,
                ndir_i18n::tr("dock.preview")
            ));
        }
        self.redraw();
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn lines_len(&self) -> usize {
        self.text.len()
    }

    /// 전체 텍스트(복사 · 덤프) — 줄 구분 `\r\n` · 이미지/패드 행은 빈 줄.
    pub(crate) fn all_text(&self) -> String {
        self.text_lines().join("\r\n")
    }

    /// 텍스트 행(이미지 행 = 빈 줄).
    fn text_lines(&self) -> Vec<&str> {
        self.text
            .iter()
            .zip(&self.kinds)
            .map(|(t, k)| {
                if *k == KIND_IMG || *k == KIND_PAD {
                    ""
                } else {
                    t.as_str()
                }
            })
            .collect()
    }

    /// 인라인 이미지 수(덤프 · 시험).
    pub(crate) fn image_count(&self) -> usize {
        self.kinds.iter().filter(|k| **k == KIND_IMG).count()
    }

    /// 덤프: `open|closed <제목> lines N top T images I` + 줄들(이미지 행 = `[img <경로>]`).
    pub(crate) fn dump(&self) -> String {
        let lines: Vec<String> = self
            .text
            .iter()
            .zip(&self.kinds)
            .map(|(t, k)| match *k {
                KIND_IMG => format!("[img {t}]"),
                KIND_PAD => String::new(),
                _ => t.clone(),
            })
            .collect();
        format!(
            "{} {} lines {} top {} images {}\n{}\n",
            if self.window.is_some() {
                "open"
            } else {
                "closed"
            },
            self.title,
            self.text.len(),
            self.top,
            self.image_count(),
            lines.join("\n")
        )
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

    pub(crate) fn open(
        &mut self,
        el: &ActiveEventLoop,
        theme: Option<winit::window::Theme>,
        over: Option<(i32, i32, u32, u32)>,
        owner: Option<&Window>,
    ) {
        if let Some(w) = &self.window {
            w.set_title(&format!(
                "{} — {}",
                self.title,
                ndir_i18n::tr("dock.preview")
            ));
            w.focus_window();
            self.redraw();
            return;
        }
        // dir2: 소유자의 3/4(폭 480~1400 · 높이 360~1000) · 중앙 · 없으면 900×640.
        let (lw, lh) = over.map_or((900.0, 640.0), |(_, _, w, h)| {
            (
                (w as f32 * 0.75).clamp(480.0, 1400.0),
                (h as f32 * 0.75).clamp(360.0, 1000.0),
            )
        });
        let mut attrs = Window::default_attributes()
            .with_title(format!(
                "{} — {}",
                self.title,
                ndir_i18n::tr("dock.preview")
            ))
            .with_theme(theme)
            .with_resizable(true)
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
    }

    fn clamp(&mut self) {
        let max_top = (self.text.len() as i32 - self.vis).max(0);
        self.top = self.top.clamp(0, max_top);
        let w = self
            .window
            .as_ref()
            .map_or(0, |w| w.inner_size().width as i32);
        let max_left = (self.max_w + (PAD_X * 2.0 * self.scale) as i32 - w).max(0);
        self.left = self.left.clamp(0, max_left);
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn scroll_lines(&mut self, d: i32) {
        self.top += d;
        self.clamp();
        self.redraw();
    }

    pub(crate) fn handle(&mut self, ev: &WindowEvent) -> PvAction {
        match ev {
            WindowEvent::CloseRequested => {
                self.close();
                return PvAction::None;
            }
            WindowEvent::Resized(_) => {
                self.clamp();
                self.redraw();
                return PvAction::None;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.redraw();
                return PvAction::None;
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.primary = if cfg!(target_os = "macos") {
                    m.state().super_key()
                } else {
                    m.state().control_key()
                };
                return PvAction::None;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (*x, *y),
                    MouseScrollDelta::PixelDelta(p) => {
                        (p.x as f32 / 24.0, p.y as f32 / self.line_h.max(1) as f32)
                    }
                };
                if self.shift || dx != 0.0 {
                    let d = if self.shift { dy } else { dx };
                    self.left -= (d * 24.0 * 5.0 * self.scale).round() as i32;
                } else {
                    // 시스템 줄 수/노치 · 트랙패드의 작은 delta는 누적(종전 `(dy*3).round()`는 0.16줄 미만을 버렸다 — dir2 7d8b1e9).
                    self.wheel_acc += dy * nexa_ctl::wheel_lines() as f32;
                    let lines = self.wheel_acc.trunc();
                    self.wheel_acc -= lines;
                    self.top -= lines as i32;
                }
                self.clamp();
                self.redraw();
                return PvAction::None;
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as i32, position.y as i32);
                if self.drag {
                    self.drag_to(self.cursor.0, self.cursor.1);
                }
                return PvAction::None;
            }
            WindowEvent::MouseInput { state, button, .. } if *button == MouseButton::Left => {
                match state {
                    ElementState::Pressed => self.begin_drag(self.cursor.0, self.cursor.1),
                    ElementState::Released => self.end_drag(),
                }
                return PvAction::None;
            }
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                match kev.logical_key.as_ref() {
                    Key::Named(NamedKey::Escape) => self.close(),
                    Key::Named(NamedKey::ArrowDown) => self.top += 1,
                    Key::Named(NamedKey::ArrowUp) => self.top -= 1,
                    Key::Named(NamedKey::PageDown) => self.top += self.vis.max(1),
                    Key::Named(NamedKey::PageUp) => self.top -= self.vis.max(1),
                    Key::Named(NamedKey::Home) => {
                        self.top = 0;
                        self.left = 0;
                    }
                    Key::Named(NamedKey::End) => self.top = i32::MAX / 2,
                    Key::Named(NamedKey::ArrowLeft) => self.left -= (24.0 * self.scale) as i32,
                    Key::Named(NamedKey::ArrowRight) => self.left += (24.0 * self.scale) as i32,
                    Key::Character(t) if self.primary && t.eq_ignore_ascii_case("c") => {
                        // 선택이 있으면 선택만(dir2 PLUG-057) · 없으면 전체.
                        return PvAction::Copy(
                            self.selected_text().unwrap_or_else(|| self.all_text()),
                        );
                    }
                    Key::Character(t) if self.primary && t.eq_ignore_ascii_case("a") => {
                        self.select_all();
                    }
                    _ => {}
                }
                self.clamp();
                self.redraw();
                return PvAction::None;
            }
            WindowEvent::RedrawRequested => return PvAction::Paint,
            _ => {}
        }
        PvAction::None
    }

    pub(crate) fn paint(&mut self, ui: &Font, mono: Option<&Font>, th: &Theme, font_px: f32) {
        let sel_range = self.sel_range();
        let (Some(win), Some(surface)) = (self.window.clone(), self.surface.as_mut()) else {
            return;
        };
        let size = win.inner_size();
        let Some(mut buf) = surface.frame(size) else {
            return;
        };
        let s = self.scale;
        let (wi, hi) = (size.width as i32, size.height as i32);
        let pad_x = (PAD_X * s).round() as i32;
        let pad_top = (PAD_TOP * s).round() as i32;
        let mut max_w = 0;
        let mut new_offsets: Vec<(usize, Vec<i32>)> = Vec::new();
        {
            let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
            let prefs = FontPrefs::with_base(font_px);
            // 코드/모노 줄(kind 4·6)은 고정폭 글꼴(없으면 UI 글꼴).
            let fonts = nexa_ctl::raster::FontSet {
                base: ui,
                peerlist: None,
                message: None,
                status: None,
                mono,
            };
            let mut dc = RasterCtx::with_font_set(&mut gfx, fonts, s).with_fonts(prefs);
            dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
            dc.select_font(FontSlot::Base, false);
            self.char_w = dc.text_width("M").max(1);
            let lh = dc.text_height() + (3.0 * s).round() as i32;
            self.line_h = lh.max(12);
            self.vis = ((hi - pad_top) / self.line_h).max(1);
            let clip = Rect::new(0, 0, wi, hi);
            let x0 = pad_x - self.left;
            let mut y = pad_top;
            let first = self.top.max(0) as usize;
            for i in first..self.text.len() {
                if y >= hi {
                    break;
                }
                let row = Rect::new(0, y, wi, self.line_h);
                let kind = self.kinds[i];
                let text = &self.text[i];
                match kind {
                    7 => {
                        dc.fill_rect(
                            Rect::new(pad_x, y + self.line_h / 2, (wi - pad_x * 2).max(0), 1),
                            th.border,
                        );
                    }
                    // 인라인 이미지(dir2 §2.4): 마커 행 + 뒤따르는 패드 행 = 예약 영역 전체에 비율 유지 가운데.
                    KIND_IMG => {
                        let k = 1 + self.kinds[i + 1..]
                            .iter()
                            .take_while(|k| **k == KIND_PAD)
                            .count() as i32;
                        let area = Rect::new(
                            pad_x,
                            y,
                            (wi - pad_x * 2).max(0),
                            (self.line_h * k).min(hi - y),
                        );
                        dc.draw_image_hint(area, text);
                    }
                    KIND_PAD => {}
                    _ => {
                        let (slot, bold) = match kind {
                            1..=3 => (FontSlot::Base, true),
                            4 | 6 => (FontSlot::Mono, false),
                            _ => (FontSlot::Base, false),
                        };
                        if kind == 4 {
                            dc.fill_rect(row, th.panel_bg_alt);
                        }
                        dc.select_font(slot, bold);
                        let fg = if kind == 5 { th.text_dim } else { th.text };
                        // 문자 경계 오프셋 캐시(선택 히트·배경) — 가시 줄만 · 줄 수 상한 안에서는 비용 무시.
                        let offs: Vec<i32> = {
                            let mut v = Vec::with_capacity(text.chars().count() + 1);
                            let mut prefix = String::new();
                            v.push(0);
                            for c in text.chars() {
                                prefix.push(c);
                                v.push(dc.text_width(&prefix));
                            }
                            v
                        };
                        if let Some(((ll, lc), (hl, hc))) = sel_range {
                            if ll <= i && i <= hl {
                                let last = offs.len() - 1;
                                let s = if i == ll { lc.min(last) } else { 0 };
                                let e = if i == hl { hc.min(last) } else { last };
                                if e > s {
                                    dc.fill_rect(
                                        Rect::new(x0 + offs[s], y, offs[e] - offs[s], self.line_h),
                                        th.sel_bg,
                                    );
                                }
                            }
                        }
                        dc.text(x0, y + 1, clip, text, fg);
                        max_w = max_w.max(dc.text_width(text));
                        new_offsets.push((i, offs));
                        if kind == 5 {
                            dc.fill_rect(
                                Rect::new(2, y + 2, 3, (self.line_h - 4).max(1)),
                                th.text_dim,
                            );
                        }
                        if kind == 1 || kind == 2 {
                            dc.fill_rect(Rect::new(0, y + self.line_h - 1, wi, 1), th.border);
                        }
                    }
                }
                y += self.line_h;
            }
        }
        if max_w > self.max_w {
            self.max_w = max_w;
        }
        for (i, o) in new_offsets {
            self.offsets.insert(i, o);
        }
        let _ = buf.present();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_and_dump_without_window() {
        assert_eq!(parse_kind("\u{2}h1|Title"), (1, "Title"));
        assert_eq!(parse_kind("\u{2}code|x"), (4, "x"));
        assert_eq!(parse_kind("plain"), (0, "plain"));
        let mut w = PreviewWin::new();
        w.set_lines(
            "a.md",
            vec![
                "\u{2}h1|Title".into(),
                "\u{1}img|x.bmp".into(),
                "a\tb".into(),
                "\u{2}hr|".into(),
            ],
        );
        assert_eq!(w.lines_len(), 4);
        assert_eq!(w.title(), "a.md");
        assert_eq!(w.all_text(), "Title\r\n\r\na    b\r\n");
        assert_eq!(w.image_count(), 1);
        assert_eq!(parse_kind("\u{1}img|x.bmp"), (KIND_IMG, "x.bmp"));
        assert_eq!(parse_kind("\u{1}pad"), (KIND_PAD, ""));
        assert!(w
            .dump()
            .starts_with("closed a.md lines 4 top 0 images 1\nTitle\n[img x.bmp]\na    b\n"));
        w.scroll_lines(5);
        assert_eq!(w.top, 3, "가시 1줄 기준 상한 = 줄 수 - 1");
    }

    /// T-62 C-3 드래그 문자 선택(창 없이 · 평균 글자 폭 7 · 행 20 · PAD 8/4): 앵커 → 확장 → 확정 · 이동 없는 클릭 = 없음 · 여러 줄 · 이미지 줄 제외 · Ctrl+A.
    #[test]
    fn drag_selection_without_window() {
        let mut w = PreviewWin::new();
        w.set_lines(
            "t",
            vec![
                "hello world".into(),
                "\u{1}img|x.bmp".into(),
                "second".into(),
            ],
        );
        w.top = 0;
        assert_eq!(w.hit(8 + 7 * 3, 4), (0, 3));
        assert_eq!(
            w.hit(8 + 7 * 2 + 3, 4 + 20 * 2),
            (2, 2),
            "최근접 경계 · 셋째 줄"
        );
        assert_eq!(w.hit(0, 4 + 20 * 9), (2, 0), "마지막 줄 아래 = 마지막 줄");
        w.begin_drag(8 + 7 * 3, 4);
        w.end_drag();
        assert!(w.selected_text().is_none(), "이동 없는 클릭 = 선택 없음");
        w.begin_drag(8 + 7 * 3, 4);
        w.drag_to(8 + 7 * 8, 4);
        assert_eq!(w.selected_text().as_deref(), Some("lo wo"));
        w.drag_to(8 + 7 * 3, 4 + 20 * 2);
        w.end_drag();
        assert_eq!(
            w.selected_text().as_deref(),
            Some("lo world\r\n\r\nsec"),
            "여러 줄 · 이미지 줄은 빈 줄"
        );
        w.begin_drag(8 + 7 * 8, 4);
        w.drag_to(8 + 7 * 3, 4);
        w.end_drag();
        assert_eq!(w.selected_text().as_deref(), Some("lo wo"), "역방향 드래그");
        w.select_all();
        assert_eq!(
            w.selected_text().as_deref(),
            Some("hello world\r\n\r\nsecond")
        );
        w.set_lines("u", vec!["x".into()]);
        assert!(w.selected_text().is_none(), "내용 교체 = 선택 초기화");
    }
}
