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
use std::rc::Rc;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
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
    left: i32,
    /// paint가 잰 행 높이 · 최장 줄 폭(가로 상한) · 가시 줄 수.
    line_h: i32,
    max_w: i32,
    vis: i32,
}

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
            left: 0,
            line_h: 20,
            max_w: 0,
            vis: 1,
        }
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
                    self.top -= (dy * 3.0).round() as i32;
                }
                self.clamp();
                self.redraw();
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
                        return PvAction::Copy(self.all_text());
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

    pub(crate) fn paint(&mut self, ui: &Font, th: &Theme, font_px: f32) {
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
        {
            let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
            let prefs = FontPrefs::with_base(font_px);
            let mut dc = RasterCtx::new(&mut gfx, ui, s).with_fonts(prefs);
            dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
            dc.select_font(FontSlot::Base, false);
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
                        dc.text(x0, y + 1, clip, text, fg);
                        max_w = max_w.max(dc.text_width(text));
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
}
