//! 자가 점검 창(T-54 · docs/18 §6 · "핵심 기능 오류 즉시 확인" 요구) — Help ▸ 자가 점검. `keys_win.rs`와 같은 보조 창 골격.
//!
//! 열면 [`crate::selfcheck::run`]을 **작업 스레드**에서 돌리고(`mpsc` · UI 스레드 차단 금지 — nexa-sql EXT-090 규약) 틱마다 수거해
//! 표(판정 · 그룹 · 항목 · ms · 상세)로 보인다. [다시 점검] · [복사](CLI `--selfcheck`와 같은 표 텍스트) · [닫기].
//! CI 환경(`CI` 변수)이면 `--ci`와 같은 부분집합(사용자 자원이 필요한 항목은 SKIP).

use crate::selfcheck::{self, Item, Options, Report, Verdict};
use ndir_i18n::{tr, trf};
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::{Button, Control, InputEvent, Invalidations, Widget};
use nexa_gfx::{Font, Surface};
use std::rc::Rc;
use std::sync::mpsc;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// 창이 호스트에 요청하는 것.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CheckAction {
    None,
    Paint,
    /// 표 텍스트를 클립보드로.
    Copy(String),
}

const PAD: f32 = 12.0;
const ROW_H: f32 = 24.0;
const HEAD_H: f32 = 24.0;
const BTN_H: f32 = 28.0;
const COL_VERDICT: f32 = 56.0;
const COL_GROUP: f32 = 84.0;
const COL_NAME: f32 = 230.0;
const COL_MS: f32 = 64.0;

pub(crate) struct CheckWin {
    window: Option<Rc<Window>>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    cursor: (i32, i32),
    shift: bool,
    rows: Vec<Item>,
    scroll: i32,
    list: Rect,
    running: bool,
    rx: Option<mpsc::Receiver<Report>>,
    run_btn: Button,
    copy_btn: Button,
    close_btn: Button,
}

impl CheckWin {
    pub(crate) fn new() -> Self {
        CheckWin {
            window: None,
            surface: None,
            scale: 1.0,
            cursor: (0, 0),
            shift: false,
            rows: Vec::new(),
            scroll: 0,
            list: Rect::default(),
            running: false,
            rx: None,
            run_btn: Button::new(tr("check.btn.run")),
            copy_btn: Button::new(tr("check.btn.copy")),
            close_btn: Button::new(tr("pref.btn.close")),
        }
    }

    /// 점검 시작(작업 스레드). 이미 도는 중이면 무시. CI 환경이면 사용자 자원 항목은 SKIP(`--ci`와 같은 부분집합).
    pub(crate) fn start(&mut self) {
        self.start_with(Options {
            ci: std::env::var_os("CI").is_some(),
            ..Default::default()
        });
    }

    /// 옵션을 지정해 시작(시험 = `ci: true` — 실제 휴지통·셸 메뉴 COM을 건드리지 않는다 · 10-03 로컬 시험 멈춤 적발).
    pub(crate) fn start_with(&mut self, opts: Options) {
        if self.running {
            return;
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(selfcheck::run(&opts));
        });
        self.rx = Some(rx);
        self.running = true;
        self.sync_enabled();
    }

    /// 작업 스레드 결과 수거 — 받았으면 true.
    pub(crate) fn poll(&mut self) -> bool {
        let Some(rx) = &self.rx else { return false };
        match rx.try_recv() {
            Ok(report) => {
                self.set_report(report);
                true
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                // 스레드가 죽었다(패닉) — 실패 1줄로 보인다.
                self.set_report(Report {
                    items: vec![Item {
                        group: "env",
                        name: "selfcheck thread".into(),
                        verdict: Verdict::Fail,
                        detail: "worker exited without a report".into(),
                        ms: 0,
                    }],
                });
                true
            }
            Err(mpsc::TryRecvError::Empty) => false,
        }
    }

    /// 결과 반영(시험·수거 공용).
    pub(crate) fn set_report(&mut self, report: Report) {
        self.rows = report.items;
        self.running = false;
        self.rx = None;
        self.scroll = 0;
        self.sync_enabled();
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn is_running(&self) -> bool {
        self.running
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn rows_len(&self) -> usize {
        self.rows.len()
    }

    /// (pass, fail, warn, skip).
    pub(crate) fn counts(&self) -> (usize, usize, usize, usize) {
        let n = |v: Verdict| self.rows.iter().filter(|i| i.verdict == v).count();
        (
            n(Verdict::Pass),
            n(Verdict::Fail),
            n(Verdict::Warn),
            n(Verdict::Skip),
        )
    }

    pub(crate) fn summary_text(&self) -> String {
        if self.running {
            return tr("check.running");
        }
        let (p, f, w, s) = self.counts();
        trf(
            "check.summary",
            &[
                &p.to_string(),
                &f.to_string(),
                &w.to_string(),
                &s.to_string(),
            ],
        )
    }

    /// CLI `--selfcheck`와 같은 표 텍스트(복사 · `check.dump`).
    pub(crate) fn table(&self) -> String {
        if self.running {
            return format!("{}\n", tr("check.running"));
        }
        Report {
            items: self.rows.clone(),
        }
        .to_table()
    }

    fn sync_enabled(&mut self) {
        self.run_btn.set_enabled(!self.running);
        self.copy_btn
            .set_enabled(!self.running && !self.rows.is_empty());
    }

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

    /// 틱: 버튼 애니메이션 + 결과 수거. 다시 그려야 하면 true.
    pub(crate) fn tick(&mut self, now_ms: u64) -> bool {
        let mut any = self.poll();
        if self.window.is_none() {
            return any;
        }
        for b in self.buttons() {
            any |= b.tick(now_ms);
        }
        any
    }

    /// 계속 깨야 하는가(점검 중 = 결과 폴링).
    pub(crate) fn animating(&self) -> bool {
        self.window.is_some()
            && (self.running
                || [&self.run_btn, &self.copy_btn, &self.close_btn]
                    .iter()
                    .any(|b| b.is_animating()))
    }

    fn buttons(&mut self) -> [&mut Button; 3] {
        [&mut self.run_btn, &mut self.copy_btn, &mut self.close_btn]
    }

    /// 언어 전환 — 버튼 글 · 창 제목(T-134).
    pub(crate) fn relabel(&mut self) {
        self.run_btn.set_label(tr("check.btn.run"));
        self.copy_btn.set_label(tr("check.btn.copy"));
        self.close_btn.set_label(tr("pref.btn.close"));
        if let Some(w) = &self.window {
            w.set_title(&format!("{} — {}", crate::APP_TITLE, tr("check.title")));
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
            return;
        }
        let (lw, lh) = (760.0, 520.0);
        let mut attrs = Window::default_attributes()
            .with_title(format!("{} — {}", crate::APP_TITLE, tr("check.title")))
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
        self.layout();
        self.start();
        self.redraw();
    }

    pub(crate) fn close(&mut self) {
        self.surface = None;
        self.window = None;
    }

    fn layout(&mut self) {
        let Some(win) = &self.window else { return };
        let size = win.inner_size();
        let (w, h) = (size.width as i32, size.height as i32);
        let s = self.scale;
        let pad = self.s(PAD);
        let mut inv = Invalidations::default();
        let hint_h = self.s(44.0);
        let top = pad + hint_h + self.s(HEAD_H);
        let by = h - pad - self.s(BTN_H);
        self.list = Rect::new(pad, top, w - pad * 2, (by - pad) - top);
        let bw = self.s(110.0);
        let gap = self.s(8.0);
        let bh = self.s(BTN_H);
        let mut x = pad;
        for b in [&mut self.run_btn, &mut self.copy_btn] {
            b.set_scale(s);
            b.set_bounds(Rect::new(x, by, bw, bh), &mut inv);
            x += bw + gap;
        }
        self.close_btn.set_scale(s);
        self.close_btn
            .set_bounds(Rect::new(w - pad - bw, by, bw, bh), &mut inv);
        self.clamp_scroll();
    }

    fn row_h(&self) -> i32 {
        self.s(ROW_H)
    }

    fn clamp_scroll(&mut self) {
        let max = (self.rows.len() as i32 * self.row_h() - self.list.h).max(0);
        self.scroll = self.scroll.clamp(0, max);
    }

    pub(crate) fn handle(&mut self, ev: &WindowEvent) -> CheckAction {
        match ev {
            WindowEvent::CloseRequested => {
                self.close();
                return CheckAction::None;
            }
            WindowEvent::Resized(_) => {
                self.layout();
                self.redraw();
                return CheckAction::None;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.layout();
                self.redraw();
                return CheckAction::None;
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                return CheckAction::None;
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as i32, position.y as i32);
                let (x, y) = self.cursor;
                let mv = InputEvent::MouseMove { x, y };
                let mut inv = Invalidations::default();
                for b in self.buttons() {
                    b.on_event(&mv, &mut inv);
                }
                self.redraw();
                return CheckAction::None;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y * ROW_H * 3.0 * self.scale,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };
                self.scroll -= dy.round() as i32;
                self.clamp_scroll();
                self.redraw();
                return CheckAction::None;
            }
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                match kev.logical_key.as_ref() {
                    Key::Named(NamedKey::Escape) => self.close(),
                    Key::Named(NamedKey::F5) if !self.running => {
                        self.start();
                        self.redraw();
                    }
                    _ => {}
                }
                return CheckAction::None;
            }
            WindowEvent::RedrawRequested => return CheckAction::Paint,
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
                _ => return CheckAction::None,
            },
            _ => return CheckAction::None,
        };
        let mut inv = Invalidations::default();
        for b in self.buttons() {
            b.on_event(&ie, &mut inv);
        }
        if let InputEvent::MouseDown { x, y, .. } = ie {
            let p = Point { x, y };
            for b in self.buttons() {
                let hit = b.bounds().contains(p);
                b.set_focused(hit);
            }
        }
        if self.run_btn.take_clicked() {
            self.start();
            self.redraw();
            return CheckAction::None;
        }
        if self.copy_btn.take_clicked() {
            self.redraw();
            return CheckAction::Copy(self.table());
        }
        if self.close_btn.take_clicked() {
            self.close();
            return CheckAction::None;
        }
        self.redraw();
        CheckAction::None
    }

    pub(crate) fn paint(&mut self, ui: &Font, th: &Theme, font_px: f32) {
        let summary = self.summary_text();
        let summary_ok = self.running || self.counts().1 == 0;
        let (Some(win), Some(surface)) = (self.window.clone(), self.surface.as_mut()) else {
            return;
        };
        let size = win.inner_size();
        let Some(mut buf) = surface.frame(size) else {
            return;
        };
        let s = self.scale;
        let (wi, hi) = (size.width as i32, size.height as i32);
        let pad = (PAD * s).round() as i32;
        let list = self.list;
        let row_h = (ROW_H * s).round() as i32;
        let cols = [
            (COL_VERDICT * s).round() as i32,
            (COL_GROUP * s).round() as i32,
            (COL_NAME * s).round() as i32,
            (COL_MS * s).round() as i32,
        ];
        {
            let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
            let prefs = FontPrefs::with_base(font_px);
            let mut dc = RasterCtx::new(&mut gfx, ui, s).with_fonts(prefs);
            dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
            dc.select_font(FontSlot::Base, false);
            let th_txt = dc.text_height();
            dc.text(
                pad,
                pad,
                Rect::new(0, 0, wi, hi),
                &tr("check.hint"),
                th.text_dim,
            );
            let sum_color = if summary_ok { th.text } else { th.danger };
            dc.text(
                pad,
                pad + th_txt + 2,
                Rect::new(0, 0, wi, hi),
                &summary,
                sum_color,
            );
            let hy = list.y - (HEAD_H * s).round() as i32;
            let head = Rect::new(list.x, hy, list.w, (HEAD_H * s).round() as i32);
            dc.fill_rect(head, th.panel_bg);
            let ty = hy + (head.h - th_txt) / 2;
            let x0 = list.x + pad / 2;
            dc.text(x0, ty, head, "", th.text_dim);
            dc.text(x0 + cols[0], ty, head, &tr("check.col.group"), th.text_dim);
            dc.text(
                x0 + cols[0] + cols[1],
                ty,
                head,
                &tr("check.col.item"),
                th.text_dim,
            );
            dc.text(
                x0 + cols[0] + cols[1] + cols[2],
                ty,
                head,
                "ms",
                th.text_dim,
            );
            dc.text(
                x0 + cols[0] + cols[1] + cols[2] + cols[3],
                ty,
                head,
                &tr("check.col.detail"),
                th.text_dim,
            );
            dc.stroke_round_rect(
                Rect::new(list.x, hy, list.w, list.h + head.h),
                0,
                th.border,
                1.0,
            );
            let first = (self.scroll / row_h).max(0) as usize;
            for (i, r) in self.rows.iter().enumerate().skip(first) {
                let y = list.y + i as i32 * row_h - self.scroll;
                if y >= list.bottom() {
                    break;
                }
                let rr = Rect::new(list.x, y, list.w, row_h);
                let clip = rr.intersection(&list);
                if clip.h <= 0 {
                    continue;
                }
                if i % 2 == 1 {
                    dc.fill_rect(clip, th.panel_bg);
                }
                let ty = y + (row_h - th_txt) / 2;
                let vc = match r.verdict {
                    Verdict::Pass => th.ok,
                    Verdict::Fail => th.danger,
                    Verdict::Warn => th.warn,
                    Verdict::Skip => th.text_dim,
                };
                dc.text(x0, ty, clip, r.verdict.as_str(), vc);
                dc.text(x0 + cols[0], ty, clip, r.group, th.text_dim);
                dc.text(x0 + cols[0] + cols[1], ty, clip, &r.name, th.text);
                dc.text(
                    x0 + cols[0] + cols[1] + cols[2],
                    ty,
                    clip,
                    &r.ms.to_string(),
                    th.text_dim,
                );
                dc.text(
                    x0 + cols[0] + cols[1] + cols[2] + cols[3],
                    ty,
                    clip,
                    &r.detail,
                    th.text,
                );
            }
            for b in [&mut self.run_btn, &mut self.copy_btn, &mut self.close_btn] {
                b.paint(&mut dc, th);
            }
        }
        let _ = buf.present();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_rows_counts_summary_and_table() {
        ndir_i18n::activate(ndir_i18n::load("en", std::path::Path::new("nowhere")));
        let mut w = CheckWin::new();
        assert_eq!(w.rows_len(), 0);
        w.set_report(Report {
            items: vec![
                Item {
                    group: "env",
                    name: "os".into(),
                    verdict: Verdict::Pass,
                    detail: "windows".into(),
                    ms: 1,
                },
                Item {
                    group: "plugin",
                    name: "load errors".into(),
                    verdict: Verdict::Fail,
                    detail: "x.wasm: junk".into(),
                    ms: 2,
                },
                Item {
                    group: "trash",
                    name: "trash one temp file".into(),
                    verdict: Verdict::Skip,
                    detail: "ci".into(),
                    ms: 0,
                },
            ],
        });
        assert_eq!(w.counts(), (1, 1, 0, 1));
        assert_eq!(w.summary_text(), "1 pass · 1 fail · 0 warn · 1 skip");
        let t = w.table();
        assert!(
            t.contains("PASS env") && t.contains("FAIL plugin") && t.contains("x.wasm: junk"),
            "{t}"
        );
    }

    /// 실제 점검을 작업 스레드에서 — UI 스레드는 폴링만(60 s 상한 · CI 변수면 --ci 부분집합).
    #[test]
    fn start_runs_in_background_and_reports() {
        ndir_i18n::activate(ndir_i18n::load("en", std::path::Path::new("nowhere")));
        let _g = crate::platform::os_test_guard();
        let mut w = CheckWin::new();
        // env 그룹만 — 셸 메뉴 COM·폴더 감시·PTY 같은 실자원 점검은 다른 시험(selfcheck · winshell)과 겹치면 멈춘다
        // (10-03 로컬 실증 · 병렬 실행). 창의 스레드/표/요약 동작이 시험 대상이지 점검 내용이 아니다.
        w.start_with(Options {
            ci: true,
            only: Some("env".into()),
            ..Default::default()
        });
        assert!(w.is_running() && w.summary_text() == "Running…");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !w.poll() {
            assert!(
                std::time::Instant::now() < deadline,
                "selfcheck did not finish"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(!w.is_running() && w.rows_len() >= 1, "{}", w.table());
        assert!(w.table().contains("PASS env"), "{}", w.table());
    }
}
