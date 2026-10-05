//! 체크섬 창(T-167 · NEW-038 · dir3 신규 · 사용자 10-05): 우클릭 메뉴 "체크섬…" → **모덜리스** 창 하나에 알고리즘별 결과 ·
//! 진행 막대(읽은 양 · 속도) · 취소 · 행 클릭 = 그 값 복사 · [모두 복사] · 비교 입력(붙여 넣은 값과 자동 대조 — 길이로 알고리즘 추정).
//!
//! 계산은 호스트의 작업 스레드(`app/checksum.rs` · 한 번에 하나 · 진행 중이면 두 번째 요청 거부)가 하고, 이 창은 값만 받는다.
//! 창을 닫으면(X · Esc · [취소]) 진행 중인 계산도 취소된다.

use ndir_i18n::{tr, trf};
use ndir_ops::hash::{self, Algo};
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::{Button, Control, InputEvent, Invalidations, SegProgress, TextBox, Widget};
use nexa_gfx::{Font, Surface};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;
use winit::event::{ElementState, Ime, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// 창이 호스트에 요청하는 것.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HashAction {
    None,
    Paint,
    /// 닫기(진행 중이면 호스트가 계산도 취소한다).
    Close,
    /// 클립보드에 글을 넣는다(행 클릭 · [모두 복사]).
    CopyText(String),
}

const PAD: f32 = 12.0;
const ROW_H: f32 = 26.0;
const BTN_H: f32 = 28.0;
const WIN_W: f32 = 640.0;

pub(crate) struct HashWin {
    window: Option<Rc<Window>>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    cursor: (i32, i32),
    shift: bool,
    primary: bool,
    file: PathBuf,
    size: u64,
    /// 알고리즘 · 값(`None` = 아직).
    rows: Vec<(Algo, Option<String>)>,
    row_rects: Vec<Rect>,
    bar: SegProgress,
    done: u64,
    started: Instant,
    running: bool,
    error: Option<String>,
    tb_cmp: TextBox,
    btn_copy: Button,
    btn_close: Button,
    /// 바닥 안내(복사됨 등 · 다음 클릭까지).
    note: Option<String>,
}

impl HashWin {
    pub(crate) fn new() -> Self {
        HashWin {
            window: None,
            surface: None,
            scale: 1.0,
            cursor: (0, 0),
            shift: false,
            primary: false,
            file: PathBuf::new(),
            size: 0,
            rows: Vec::new(),
            row_rects: Vec::new(),
            bar: SegProgress::new(),
            done: 0,
            started: Instant::now(),
            running: false,
            error: None,
            tb_cmp: TextBox::new(""),
            btn_copy: Button::new(tr("hash.copyAll")),
            btn_close: Button::new(tr("ops.cancel")),
            note: None,
        }
    }

    /// 새 계산 시작 — 대상 · 알고리즘 목록 · 진행 0.
    pub(crate) fn start(&mut self, file: &Path, size: u64, algos: &[Algo]) {
        self.file = file.to_path_buf();
        self.size = size;
        self.rows = algos.iter().map(|&a| (a, None)).collect();
        self.done = 0;
        self.started = Instant::now();
        self.running = true;
        self.error = None;
        self.note = None;
        let mut inv = Invalidations::default();
        self.bar.set_items(Vec::new(), &mut inv);
        self.bar.set_totals(0, size.max(1), &mut inv);
        self.btn_copy.set_enabled(false);
        self.btn_close.set_label(tr("ops.cancel"));
        self.redraw();
    }

    /// 진행(읽은 바이트).
    pub(crate) fn update(&mut self, done: u64) {
        if done == self.done {
            return;
        }
        self.done = done;
        let mut inv = Invalidations::default();
        self.bar.set_totals(done, self.size.max(1), &mut inv);
        self.redraw();
    }

    /// 결과 도착.
    pub(crate) fn set_results(&mut self, results: &[(Algo, String)]) {
        for (a, v) in &mut self.rows {
            if let Some((_, hex)) = results.iter().find(|(x, _)| x == a) {
                *v = Some(hex.clone());
            }
        }
        self.done = self.size;
        let mut inv = Invalidations::default();
        self.bar
            .set_totals(self.size.max(1), self.size.max(1), &mut inv);
        self.finish();
    }

    /// 실패(읽기 오류 · 취소).
    pub(crate) fn set_error(&mut self, text: &str) {
        self.error = Some(text.to_string());
        self.finish();
    }

    fn finish(&mut self) {
        self.running = false;
        self.btn_copy
            .set_enabled(self.rows.iter().any(|(_, v)| v.is_some()));
        self.btn_close.set_label(tr("license.btn.close"));
        self.redraw();
    }

    #[cfg(test)]
    pub(crate) fn is_running(&self) -> bool {
        self.running
    }

    /// 결과(시험 · 복사용) — 알고리즘 이름 · 값 · 파일 이름을 한 줄에.
    pub(crate) fn results_text(&self) -> String {
        let name = ndir_ops::leaf_name(&self.file);
        self.rows
            .iter()
            .filter_map(|(a, v)| v.as_ref().map(|h| format!("{:<8} {h}  {name}", a.name())))
            .collect::<Vec<_>>()
            .join("\r\n")
    }

    #[cfg(test)]
    pub(crate) fn results(&self) -> Vec<(Algo, Option<String>)> {
        self.rows.clone()
    }

    #[cfg(test)]
    pub(crate) fn set_compare_text(&mut self, text: &str) {
        self.tb_cmp.set_text(text);
    }

    /// 비교 입력의 판정(순수): `None` = 입력 없음 · `Some(Err(()))` = 알 수 없는 길이 · `Some(Ok((알고리즘, 일치)))`.
    pub(crate) fn compare_state(&self) -> Option<Result<(Algo, bool), ()>> {
        let text = self.tb_cmp.text();
        if text.trim().is_empty() {
            return None;
        }
        let Some(norm) = hash::normalize_hex(&text) else {
            return Some(Err(()));
        };
        let algo = self
            .rows
            .iter()
            .map(|(a, _)| *a)
            .find(|a| a.hex_len() == norm.len())?;
        let hit = self
            .rows
            .iter()
            .find(|(a, _)| *a == algo)
            .and_then(|(_, v)| v.as_deref())
            .is_some_and(|v| v == norm);
        Some(Ok((algo, hit)))
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
        let rows = self.rows.len().max(1) as f32;
        let lh = PAD * 2.0 + 30.0 + rows * ROW_H + 8.0 + 44.0 + 8.0 + 56.0 + 8.0 + BTN_H;
        let mut attrs = Window::default_attributes()
            .with_title(format!("Nexa Dir — {}", tr("hash.title")))
            .with_theme(theme)
            .with_resizable(true)
            .with_min_inner_size(winit::dpi::LogicalSize::new(420.0, lh))
            .with_inner_size(winit::dpi::LogicalSize::new(WIN_W, lh));
        if let Some((x, y, w, h)) = over {
            let cx = x + (w as i32 - WIN_W as i32) / 2;
            let cy = y + (h as i32 - lh as i32) / 2;
            attrs = attrs.with_position(winit::dpi::PhysicalPosition::new(cx.max(0), cy.max(0)));
        }
        let attrs = crate::winfocus::owned_by(crate::icon::with_icon(attrs), owner);
        let Ok(win) = el.create_window(attrs) else {
            return;
        };
        let win = Rc::new(win);
        win.set_ime_allowed(crate::input::system_ime());
        self.scale = win.scale_factor() as f32;
        self.surface = crate::present::Presenter::new(win.clone()).ok();
        self.window = Some(win);
        self.tb_cmp.set_focused(true);
        self.redraw();
    }

    pub(crate) fn close(&mut self) {
        self.surface = None;
        self.window = None;
        self.btn_close.clear_transient();
        self.btn_copy.clear_transient();
        self.tb_cmp.set_text("");
        self.tb_cmp.set_focused(false);
        self.rows.clear();
        self.running = false;
        self.error = None;
        self.note = None;
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

    pub(crate) fn relabel(&mut self) {
        self.btn_copy.set_label(tr("hash.copyAll"));
        self.btn_close.set_label(tr(if self.running {
            "ops.cancel"
        } else {
            "license.btn.close"
        }));
        if let Some(w) = &self.window {
            w.set_title(&format!("Nexa Dir — {}", tr("hash.title")));
        }
        self.redraw();
    }

    pub(crate) fn tick(&mut self, now_ms: u64) -> bool {
        self.window.is_some()
            && (self.btn_close.tick(now_ms) | self.btn_copy.tick(now_ms) | self.tb_cmp.tick(now_ms))
    }

    pub(crate) fn animating(&self) -> bool {
        self.window.is_some()
            && (self.btn_close.is_animating()
                || self.btn_copy.is_animating()
                || self.tb_cmp.is_animating())
    }

    pub(crate) fn handle(&mut self, ev: &WindowEvent) -> HashAction {
        let mut inv = Invalidations::default();
        match ev {
            WindowEvent::CloseRequested => return HashAction::Close,
            WindowEvent::RedrawRequested => return HashAction::Paint,
            WindowEvent::Resized(_) => self.redraw(),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.redraw();
            }
            WindowEvent::Focused(false) => {
                self.btn_close.clear_transient();
                self.btn_copy.clear_transient();
                self.redraw();
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.primary = if cfg!(target_os = "macos") {
                    m.state().super_key()
                } else {
                    m.state().control_key()
                };
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as i32, position.y as i32);
                let (x, y) = self.cursor;
                let mv = InputEvent::MouseMove { x, y };
                self.btn_close.on_event(&mv, &mut inv);
                self.btn_copy.on_event(&mv, &mut inv);
                self.tb_cmp.on_event(&mv, &mut inv);
                if !inv.is_empty() {
                    self.redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } if *button == MouseButton::Left => {
                let (x, y) = self.cursor;
                let p = Point { x, y };
                let e = match state {
                    ElementState::Pressed => InputEvent::MouseDown {
                        x,
                        y,
                        shift: self.shift,
                        primary: self.primary,
                    },
                    ElementState::Released => InputEvent::MouseUp { x, y },
                };
                let up = matches!(e, InputEvent::MouseUp { .. });
                if !up {
                    self.tb_cmp.set_focused(self.tb_cmp.bounds().contains(p));
                    // 행 클릭 = 그 값 복사.
                    if let Some(i) = self.row_rects.iter().position(|r| r.contains(p)) {
                        if let Some((a, Some(v))) = self.rows.get(i) {
                            self.note = Some(trf("hash.copied", &[a.name()]));
                            let v = v.clone();
                            self.redraw();
                            return HashAction::CopyText(v);
                        }
                    }
                }
                if up || self.tb_cmp.bounds().contains(p) {
                    self.tb_cmp.on_event(&e, &mut inv);
                }
                for b in [&mut self.btn_close, &mut self.btn_copy] {
                    if up || b.bounds().contains(p) {
                        b.on_event(&e, &mut inv);
                    }
                }
                self.redraw();
                if self.btn_close.take_clicked() {
                    return HashAction::Close;
                }
                if self.btn_copy.take_clicked() {
                    self.note = Some(trf("hash.copied", &[&tr("hash.copyAll")]));
                    return HashAction::CopyText(self.results_text());
                }
            }
            WindowEvent::Ime(Ime::Preedit(text, _)) => {
                self.tb_cmp.set_preedit(text, &mut inv);
                self.redraw();
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                self.tb_cmp.set_preedit("", &mut inv);
                for ch in text.chars().filter(|c| !c.is_control()) {
                    self.tb_cmp
                        .on_event(&InputEvent::Char { c: ch, now_ms: 0 }, &mut inv);
                }
                self.redraw();
            }
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                if matches!(kev.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    return HashAction::Close;
                }
                if self.primary && crate::input::shortcut_letter(kev) == Some('a') {
                    self.tb_cmp.on_event(&InputEvent::SelectAll, &mut inv);
                    self.redraw();
                    return HashAction::None;
                }
                if let Some(e) = crate::input::text_key_event(
                    kev,
                    self.shift,
                    self.primary,
                    crate::input::TextKeys::Line,
                ) {
                    if self.tb_cmp.is_focused() {
                        self.tb_cmp.on_event(&e, &mut inv);
                        self.redraw();
                    }
                }
            }
            _ => {}
        }
        HashAction::None
    }

    pub(crate) fn paint(&mut self, font: &Font, th: &Theme, ui_px: f32) {
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
        {
            let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
            let prefs = FontPrefs::with_base(ui_px);
            let mut dc = RasterCtx::new(&mut gfx, font, s).with_fonts(prefs);
            dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
            let inv = &mut Invalidations::default();
            let pad = px(PAD);
            let clip = Rect::new(0, 0, wi, hi);
            let row_h = px(ROW_H);
            let mut y = pad;

            // 머리: 파일 이름(굵게) · 크기.
            dc.select_font(FontSlot::Base, true);
            let name = ndir_ops::leaf_name(&self.file);
            let th_txt = dc.text_height();
            dc.text(pad, y, clip, &name, th.text);
            let nw = dc.text_width(&name);
            dc.select_font(FontSlot::Base, false);
            dc.text(
                pad + nw + px(10.0),
                y,
                clip,
                &crate::dockinfo::fmt_size_long(self.size),
                th.text_dim,
            );
            y += th_txt + px(10.0);

            // 알고리즘 행: 이름 · 값(클릭 = 복사).
            let label_w = dc.text_width("SHA-256") + px(16.0);
            self.row_rects.clear();
            let hover = Point {
                x: self.cursor.0,
                y: self.cursor.1,
            };
            for (a, v) in &self.rows {
                let r = Rect::new(pad, y, wi - pad * 2, row_h);
                if v.is_some() && r.contains(hover) {
                    dc.fill_round_rect(r, px(3.0), th.panel_bg_alt);
                }
                let ty = dc.text_center_y(y, row_h);
                dc.text(pad + px(4.0), ty, clip, a.name(), th.text_dim);
                let value = v.clone().unwrap_or_else(|| {
                    if self.error.is_some() {
                        "–".to_string()
                    } else {
                        tr("hash.calculating")
                    }
                });
                let vx = pad + label_w;
                let value = nexa_ctl::draw::ellipsize_middle(&mut dc, &value, wi - pad - vx);
                dc.text(
                    vx,
                    ty,
                    clip,
                    &value,
                    if v.is_some() { th.text } else { th.text_dim },
                );
                self.row_rects.push(r);
                y += row_h;
            }
            y += px(8.0);

            // 진행 막대 + 글(읽은 양 · 비율 · 속도) 또는 오류.
            let bar_h = px(14.0);
            self.bar.set_scale(s);
            self.bar
                .set_bounds(Rect::new(pad, y, wi - pad * 2, bar_h), inv);
            self.bar.paint(&mut dc, th);
            y += bar_h + px(4.0);
            let line = if let Some(e) = &self.error {
                e.clone()
            } else {
                let secs = self.started.elapsed().as_secs_f64().max(0.001);
                let speed = (self.done as f64 / secs) as u64;
                let pct = (self.done * 100).checked_div(self.size).unwrap_or(100);
                trf(
                    "hash.progress",
                    &[
                        &crate::filelist::format_size(self.done),
                        &crate::filelist::format_size(self.size),
                        &pct.to_string(),
                        &crate::filelist::format_size(speed),
                    ],
                )
            };
            dc.text(
                pad,
                y,
                clip,
                &line,
                if self.error.is_some() {
                    th.danger
                } else {
                    th.text_dim
                },
            );
            y += th_txt + px(10.0);

            // 비교 입력.
            let label = tr("hash.compare");
            let lw = dc.text_width(&label) + px(8.0);
            let tb_h = th_txt + px(10.0);
            let ly = dc.text_center_y(y, tb_h);
            dc.text(pad, ly, clip, &label, th.text);
            self.tb_cmp.set_scale(s);
            self.tb_cmp
                .set_bounds(Rect::new(pad + lw, y, wi - pad * 2 - lw, tb_h), inv);
            self.tb_cmp.paint(&mut dc, th);
            y += tb_h + px(4.0);
            let (verdict, color) = match self.compare_state() {
                None => (String::new(), th.text_dim),
                Some(Err(())) => (tr("hash.unknownLen"), th.text_dim),
                Some(Ok((a, true))) => (trf("hash.match", &[a.name()]), th.ok),
                Some(Ok((a, false))) => (trf("hash.mismatch", &[a.name()]), th.danger),
            };
            dc.text(pad + lw, y, clip, &verdict, color);

            // 바닥: 안내 · [모두 복사] [취소/닫기].
            let btn_h = px(BTN_H);
            let by = hi - pad - btn_h;
            if let Some(n) = &self.note {
                let ny = dc.text_center_y(by, btn_h);
                dc.text(pad, ny, clip, n, th.text_dim);
            }
            let mut x = wi - pad;
            for b in [&mut self.btn_close, &mut self.btn_copy] {
                b.set_scale(s);
                let bw = dc.text_width(b.label()) + px(28.0);
                x -= bw;
                b.set_bounds(Rect::new(x, by, bw, btn_h), inv);
                b.paint(&mut dc, th);
                x -= px(8.0);
            }
        }
        let _ = buf.present();
        self.surface = Some(surface);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn en() {
        ndir_i18n::activate(ndir_i18n::load("en", std::path::Path::new("nowhere")));
    }

    /// 창 없이: 시작 → 계산 중 · 결과 → 값 · [모두 복사] 글 · 비교 입력 판정(일치 · 불일치 · 모르는 길이 · 비어 있음 · 구분 문자 무시).
    #[test]
    fn results_and_compare_without_window() {
        en();
        let mut w = HashWin::new();
        w.start(
            Path::new("D:/x/report.txt"),
            3,
            &[Algo::Crc32, Algo::Sha256],
        );
        assert!(w.is_running() && w.results_text().is_empty());
        w.update(2);
        w.set_results(&[
            (Algo::Crc32, "352441c2".into()),
            (
                Algo::Sha256,
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
            ),
        ]);
        assert!(!w.is_running());
        let text = w.results_text();
        assert!(text.starts_with("CRC32    352441c2  report.txt"), "{text}");
        assert!(text.contains("SHA-256  ba7816bf"), "{text}");
        assert_eq!(w.compare_state(), None);
        w.set_compare_text(" 3524-41C2 ");
        assert_eq!(w.compare_state(), Some(Ok((Algo::Crc32, true))));
        w.set_compare_text("00000000");
        assert_eq!(w.compare_state(), Some(Ok((Algo::Crc32, false))));
        w.set_compare_text("abc");
        assert_eq!(w.compare_state(), None, "맞는 길이의 알고리즘이 없음");
        w.set_compare_text("zz");
        assert_eq!(w.compare_state(), Some(Err(())));
        // 오류.
        w.start(Path::new("D:/x/b"), 10, &[Algo::Md5]);
        w.set_error("boom");
        assert!(!w.is_running() && w.results_text().is_empty());
        w.close();
        assert!(w.results().is_empty());
    }
}
