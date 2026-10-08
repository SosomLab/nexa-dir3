//! 메모리 창(T-93 · docs/22 NEW-002 · nexa-sql `mem_win.rs` 이식): 상태줄의 앱 메모리 칸 → **모덜리스** 창.
//!
//! - 맨 위 = **누적 정보**: 이 프로그램의 메모리(운영체제 값) · 상주 · 힙 여유 · 최근 60 표본 막대 그래프 · 영역 색으로 쌓은 총량 막대.
//! - 표 = **기능별 묶음**([`crate::memstat::Group`]) 안의 영역(색 칩 · 막대 · 바이트 · 비율) + 묶음 소계. 총량과의 차이(런타임 ·
//!   라이브러리 · 미집계)는 "프로그램" 묶음 끝에.
//! - **늘고 주는 과정**: 영역 값이 바뀌면 그 행에 ▲/▼ 변화량이 몇 표본 동안 남는다([`crate::memstat::Trend`]).
//! - 아래 = 운영체제가 보는 값(전용 · 상주 · 파일 매핑 · 힙) + 시스템 전체 · [힙 정리] · [닫기].
//!
//! 표본은 호스트가 주기마다 넣는다([`MemWin::set_sample`]) — 창이 닫혀 있으면 비용 0 · 닫을 때 표본 · 이력을 버린다.

use crate::memstat::{fmt, fmt_delta, Cat, Group, Sample, Trend};
use ndir_i18n::{tr, trf};
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{Color, FontPrefs, Theme};
use nexa_ctl::{Button, Control, InputEvent, Invalidations, Widget};
use nexa_gfx::{Font, Surface};
use std::collections::VecDeque;
use std::rc::Rc;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// 총량 이력 표본 수(맨 위 막대 그래프).
const HIST: usize = 60;
/// 창 기본 크기(논리 px · 사용자 10-08 "700×800") · 최소 폭 — 높이는 내용에 맞춘다.
const WIN_W: f32 = 700.0;
const WIN_H: f32 = 800.0;
const MIN_W: f32 = 460.0;

/// 창이 호스트에 요청하는 것.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemAction {
    None,
    Paint,
    /// [힙 정리] — 호스트가 `platform::procmem::trim` 뒤 곧바로 새 표본을 넣는다.
    Trim,
    Close,
}

pub(crate) struct MemWin {
    window: Option<Rc<Window>>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    cursor: (i32, i32),
    btn_close: Button,
    btn_trim: Button,
    /// 마지막 표본 · 총량 이력 · 영역별 변화.
    sample: Option<Sample>,
    hist: VecDeque<u64>,
    trend: Trend,
    /// 갱신 주기(ms · 바닥 안내 글).
    every_ms: u64,
    /// [힙 정리] 진행 표시(사용자 10-04 "눌러도 무엇이 진행 중인지 모르겠다"): 누른 뒤 **결과가 반영된 표본이 한 번 더 올 때까지**
    /// 버튼을 잠그고 글을 "정리 중…"으로 · 남은 표본 수(0 = 평소).
    trim_hold: u8,
    /// 마지막 정리 결과 안내(바닥 줄 · 남은 표시 표본 수).
    trim_note: Option<(String, u8)>,
    /// ★ 다시 그리기 생략(nexa-sql T-310 개념 · 사용자 10-08): 마지막으로 **그린** 표본의 표시 서명 · 그때의 현지 시각(바닥 "갱신 hh:mm:ss").
    last_sig: Option<u64>,
    updated_at: String,
    /// ★ 내용에 맞춘 창 높이(물리 px · 사용자 10-08 "내용을 꽉 채우고 바닥 공간 없이 바로 [닫기]"): 그린 뒤 표 끝 + 바닥 줄 + 여백을
    /// 재서 창에 요청한 값. 행 수(OS별 선택 행 · 시스템 줄) · 배율 · 글꼴이 바뀌어 값이 달라질 때만 다시 요청한다.
    fit_h: Option<u32>,
}

/// [힙 정리] 뒤 버튼을 잠가 두는 표본 수(즉시 표본 1 + 다음 주기 1) · 결과 안내가 남는 표본 수.
const TRIM_HOLD: u8 = 2;
const TRIM_NOTE_HOLD: u8 = 8;

impl MemWin {
    pub(crate) fn new() -> Self {
        MemWin {
            window: None,
            surface: None,
            scale: 1.0,
            cursor: (0, 0),
            btn_close: Button::new(tr("license.btn.close")),
            btn_trim: Button::new(tr("mem.trim")),
            sample: None,
            hist: VecDeque::new(),
            trend: Trend::default(),
            every_ms: 1000,
            trim_hold: 0,
            trim_note: None,
            last_sig: None,
            updated_at: String::new(),
            fit_h: None,
        }
    }

    /// 마지막으로 그린 표본의 갱신 시각(hh:mm:ss · 시험).
    #[cfg(test)]
    pub(crate) fn updated_at(&self) -> &str {
        &self.updated_at
    }

    /// 마지막으로 그린 표본의 표시 서명(시험).
    #[cfg(test)]
    pub(crate) fn last_sig(&self) -> Option<u64> {
        self.last_sig
    }

    /// [힙 정리]를 누른 직후 — 버튼을 잠그고 글을 "정리 중…"으로(호스트가 정리를 마치고 [`Self::set_trim_result`]를 부른다).
    fn begin_trim(&mut self) {
        self.trim_hold = TRIM_HOLD;
        self.btn_trim.set_enabled(false);
        self.btn_trim.set_label(tr("mem.trimming"));
        self.redraw();
    }

    /// 정리 결과(전 · 후 전용 메모리 · 걸린 µs) → 바닥 안내 글(줄었으면 반환량 · 아니면 "반환할 것이 없음").
    pub(crate) fn set_trim_result(&mut self, before: u64, after: u64, us: u128) {
        let ms = format!("{:.1}", us as f64 / 1000.0);
        let text = if before > after {
            trf("mem.trimmed", &[&fmt(before - after), &ms])
        } else {
            trf("mem.trimmedNone", &[&ms])
        };
        self.trim_note = Some((text, TRIM_NOTE_HOLD));
        self.redraw();
    }

    /// 정리 중인가(버튼이 잠겨 있다).
    #[cfg(test)]
    pub(crate) fn trimming(&self) -> bool {
        self.trim_hold > 0
    }

    #[cfg(test)]
    pub(crate) fn trim_note(&self) -> Option<&str> {
        self.trim_note.as_ref().map(|n| n.0.as_str())
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
        // 700×800(사용자 10-08 · nexa-sql 메모리 창 크기) — 높이는 첫 그리기 뒤 내용에 맞춰 다시 요청한다(`paint` · `fit_h`).
        let (lw, lh) = (WIN_W, WIN_H);
        let mut attrs = Window::default_attributes()
            .with_title(format!("{} — {}", crate::APP_TITLE, tr("mem.title")))
            .with_theme(theme)
            .with_resizable(true)
            .with_min_inner_size(winit::dpi::LogicalSize::new(MIN_W, 320.0))
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
        self.btn_trim.clear_transient();
        // 표본 · 이력은 창과 함께 버린다(닫힌 뒤 상주 0).
        self.sample = None;
        self.hist = VecDeque::new();
        self.trend = Trend::default();
        self.trim_hold = 0;
        self.trim_note = None;
        self.last_sig = None;
        self.updated_at.clear();
        self.fit_h = None;
        self.btn_trim.set_enabled(true);
        self.btn_trim.set_label(tr("mem.trim"));
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

    /// 새 표본(호스트가 주기마다) — 이력 · 변화 갱신 + 그리기 요청.
    pub(crate) fn set_sample(&mut self, s: Sample, every_ms: u64) {
        if self.hist.len() >= HIST {
            self.hist.pop_front();
        }
        self.hist.push_back(s.sys.footprint);
        self.trend.update(&s);
        // 정리 뒤 표본이 오면 잠금을 하나씩 푼다(0이 되면 버튼 복귀) · 결과 안내는 몇 표본 뒤 사라진다.
        if self.trim_hold > 0 {
            self.trim_hold -= 1;
            if self.trim_hold == 0 {
                self.btn_trim.set_enabled(true);
                self.btn_trim.set_label(tr("mem.trim"));
            }
        }
        if let Some((_, left)) = &mut self.trim_note {
            *left = left.saturating_sub(1);
            if *left == 0 {
                self.trim_note = None;
            }
        }
        // ★ 다시 그리기 생략(nexa-sql T-310 개념 · 사용자 10-08): 화면에 찍히는 글의 서명이 같으면 그리지 않는다(바닥 "갱신 hh:mm:ss"도
        //   그대로 — 값이 안 바뀌면 시각이 매초 바뀌지 않는다). 예외 = ▲/▼ 표시 중 · 정리 잠금 · 결과 안내 표시 중(사라지는 과정을 그려야 함).
        let sig = s.display_sig();
        let changed = self.last_sig != Some(sig);
        if changed {
            self.last_sig = Some(sig);
            self.updated_at = ndir_log::now_local().time_only();
            self.updated_at.truncate(8);
        }
        self.sample = Some(s);
        self.every_ms = every_ms;
        if changed || self.trend.any_shown() || self.trim_hold > 0 || self.trim_note.is_some() {
            self.redraw();
        }
    }

    /// 이 창의 표면(프레임 버퍼) 바이트 — 호스트가 [`Cat::SurfaceAux`]에 더한다.
    pub(crate) fn surface_bytes(&self) -> u64 {
        self.window.as_ref().map_or(0, |w| {
            let s = w.inner_size();
            u64::from(s.width) * u64::from(s.height) * 4
        })
    }

    #[cfg(test)]
    pub(crate) fn trend(&self) -> &Trend {
        &self.trend
    }

    /// 언어 전환 — 버튼 글 · 창 제목.
    pub(crate) fn relabel(&mut self) {
        self.btn_close.set_label(tr("license.btn.close"));
        self.btn_trim.set_label(tr(if self.trim_hold > 0 {
            "mem.trimming"
        } else {
            "mem.trim"
        }));
        if let Some(w) = &self.window {
            w.set_title(&format!("{} — {}", crate::APP_TITLE, tr("mem.title")));
        }
        self.redraw();
    }

    pub(crate) fn tick(&mut self, now_ms: u64) -> bool {
        self.window.is_some() && (self.btn_close.tick(now_ms) | self.btn_trim.tick(now_ms))
    }

    pub(crate) fn animating(&self) -> bool {
        self.window.is_some() && (self.btn_close.is_animating() || self.btn_trim.is_animating())
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
                self.btn_trim.clear_transient();
                self.redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as i32, position.y as i32);
                let (x, y) = self.cursor;
                let mv = InputEvent::MouseMove { x, y };
                self.btn_close.on_event(&mv, &mut inv);
                self.btn_trim.on_event(&mv, &mut inv);
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
                // 마우스 라우팅 규칙: 눌림은 커서 아래 컨트롤에만 · 뗌은 늘(눌린 상태를 풀게).
                let up = matches!(e, InputEvent::MouseUp { .. });
                let p = Point { x, y };
                if up || self.btn_close.bounds().contains(p) {
                    self.btn_close.on_event(&e, &mut inv);
                }
                if up || self.btn_trim.bounds().contains(p) {
                    self.btn_trim.on_event(&e, &mut inv);
                }
                self.redraw();
                if self.btn_close.take_clicked() {
                    return MemAction::Close;
                }
                if self.btn_trim.take_clicked() && self.trim_hold == 0 {
                    self.begin_trim();
                    return MemAction::Trim;
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
        let rgb = |(r, g, b): (u8, u8, u8)| Color::from_rgb(r, g, b);
        {
            let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
            let prefs = FontPrefs::with_base(ui_px);
            let mut dc = RasterCtx::new(&mut gfx, font, s).with_fonts(prefs);
            dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
            let inv = &mut Invalidations::default();
            let pad = px(12.0);
            let clip = Rect::new(0, 0, wi, hi);
            let sample = self.sample;
            let sys = sample.map(|sm| sm.sys).unwrap_or_default();
            let foot = sys.footprint;
            let denom = foot.max(1);

            // ── 맨 위(누적 정보): 제목 · 최근 총량 막대 그래프 · [힙 정리] ───────────────────────
            dc.select_font(FontSlot::Base, true);
            let th_txt = dc.text_height();
            let mut y = pad;
            let head_h = th_txt + px(8.0);
            let ty = dc.text_center_y(y, head_h);
            dc.text(pad, ty, clip, &tr("mem.title"), th.text);
            let title_w = dc.text_width(&tr("mem.title"));
            dc.select_font(FontSlot::Base, false);
            self.btn_trim.set_scale(s);
            // 버튼 폭 = 두 글("힙 정리" · "정리 중…") 중 넓은 쪽 — 눌러도 자리가 흔들리지 않는다.
            let trim_w = dc
                .text_width(&tr("mem.trim"))
                .max(dc.text_width(&tr("mem.trimming")))
                + px(28.0);
            self.btn_trim
                .set_bounds(Rect::new(wi - pad - trim_w, y, trim_w, head_h), inv);
            self.btn_trim.paint(&mut dc, th);
            // 막대 그래프(제목과 버튼 사이 · 최근 60 표본 · 가장 큰 값 = 꽉 찬 높이).
            let sp_x0 = pad + title_w + px(16.0);
            let sp_x1 = wi - pad - trim_w - px(12.0);
            if self.hist.len() >= 2 && sp_x1 - sp_x0 > px(60.0) {
                let sp_w = (sp_x1 - sp_x0).min(px(260.0));
                let area = Rect::new(sp_x1 - sp_w, y, sp_w, head_h);
                dc.fill_rect(area, th.panel_bg_alt);
                let max = self.hist.iter().copied().max().unwrap_or(1).max(1);
                let step = (area.w as f32 / HIST as f32).max(1.0);
                for (i, v) in self.hist.iter().enumerate() {
                    let h = ((*v as f64 / max as f64) * f64::from(area.h - 2)).round() as i32;
                    let x0 = area.x + (i as f32 * step).round() as i32;
                    let w = (step - 1.0).max(1.0) as i32;
                    dc.fill_rect(Rect::new(x0, area.bottom() - 1 - h, w, h.max(1)), th.accent);
                }
            }
            y += head_h + px(8.0);

            // 총량 + 상주 · 힙 여유.
            dc.select_font(FontSlot::Base, true);
            let total_txt = if sample.is_some() {
                fmt(foot)
            } else {
                "–".to_string()
            };
            dc.text(pad, y, clip, &total_txt, th.text);
            let tw = dc.text_width(&total_txt);
            dc.select_font(FontSlot::Base, false);
            let sub = trf("mem.subtitle", &[&fmt(sys.resident), &fmt(sys.heap_held)]);
            dc.text(pad + tw + px(10.0), y, clip, &sub, th.text_dim);
            y += th_txt + px(8.0);

            // 총량 막대(영역 색을 차례로 쌓고 나머지 = 기타 회색).
            let bar = Rect::new(pad, y, wi - pad * 2, px(18.0));
            dc.fill_rect(bar, th.panel_bg_alt);
            if let Some(sm) = sample {
                let mut x = bar.x;
                for c in Cat::ALL.into_iter().filter(|c| !c.file_backed()) {
                    let b = sm.data.get(c);
                    let w = (((b as f64 / denom as f64) * f64::from(bar.w)).round() as i32)
                        .min(bar.right() - x)
                        .max(0);
                    if w > 0 {
                        dc.fill_rect(Rect::new(x, bar.y, w, bar.h), rgb(c.color()));
                        x += w;
                    }
                }
                if bar.right() > x {
                    dc.fill_rect(
                        Rect::new(x, bar.y, bar.right() - x, bar.h),
                        rgb(Cat::OTHER_COLOR),
                    );
                }
            }
            dc.stroke_round_rect(bar, 0, th.border, 1.0);
            y += bar.h + px(10.0);

            // ── 표 ─────────────────────────────────────────────────────────────────
            let row_h = th_txt + px(5.0);
            let label_x = pad + px(18.0);
            let pct_w = dc.text_width("100.0%") + px(6.0);
            let bytes_w = dc.text_width("999.9 MB") + px(8.0);
            let delta_w = dc.text_width("▲ 999.9 MB") + px(8.0);
            let label_w = px(230.0).min((wi - pad * 2) / 2);
            let bar_x0 = label_x + label_w;
            let bar_x1 = wi - pad - pct_w - bytes_w - delta_w - px(6.0);
            let trend = self.trend;
            let row = |dc: &mut RasterCtx,
                       y: i32,
                       chip: Option<(u8, u8, u8)>,
                       label: &str,
                       bytes: u64,
                       denom: u64,
                       delta: Option<i64>| {
                if let Some(c) = chip {
                    let cw = px(10.0);
                    dc.fill_round_rect(
                        Rect::new(pad, y + (row_h - cw) / 2, cw, cw),
                        px(2.0),
                        rgb(c),
                    );
                }
                let ty = dc.text_center_y(y, row_h);
                dc.text(
                    label_x,
                    ty,
                    Rect::new(label_x, y, label_w - px(6.0), row_h),
                    label,
                    th.text,
                );
                if bar_x1 > bar_x0 + px(20.0) {
                    let full = bar_x1 - bar_x0;
                    let track = Rect::new(bar_x0, y + row_h / 2 - px(3.0), full, px(6.0));
                    dc.fill_rect(track, th.panel_bg_alt);
                    let w = ((bytes as f64 / denom.max(1) as f64).min(1.0) * f64::from(full))
                        .round() as i32;
                    if w > 0 {
                        dc.fill_rect(
                            Rect::new(track.x, track.y, w, track.h),
                            rgb(chip.unwrap_or(Cat::OTHER_COLOR)),
                        );
                    }
                }
                // 변화(▲ 늘었다 = 위험색 · ▼ 줄었다 = 강조색) — 바뀐 뒤 몇 표본 동안만.
                if let Some(d) = delta {
                    let dt = fmt_delta(d);
                    let dw = dc.text_width(&dt);
                    let dx = wi - pad - pct_w - bytes_w - dw - px(4.0);
                    let col = if d >= 0 { th.danger } else { th.accent };
                    dc.text(dx, ty, clip, &dt, col);
                }
                let bt = fmt(bytes);
                let bw = dc.text_width(&bt);
                dc.text(wi - pad - pct_w - bw, ty, clip, &bt, th.text);
                let pct = format!("{:.1}%", bytes as f64 * 100.0 / denom.max(1) as f64);
                let pw = dc.text_width(&pct);
                dc.text(wi - pad - pw, ty, clip, &pct, th.text_dim);
            };
            // 구획 머리: 묶음 이름(굵게) + 소계(오른쪽) + 밑줄.
            let section = |dc: &mut RasterCtx, y: i32, label: &str, sum: Option<u64>| {
                dc.select_font(FontSlot::Base, true);
                let ty = dc.text_center_y(y, row_h);
                dc.text(pad, ty, clip, label, th.text_dim);
                dc.select_font(FontSlot::Base, false);
                if let Some(v) = sum {
                    let t = fmt(v);
                    let w = dc.text_width(&t);
                    dc.text(wi - pad - pct_w - w, ty, clip, &t, th.text_dim);
                }
                dc.fill_rect(Rect::new(pad, y + row_h - 1, wi - pad * 2, 1), th.border);
            };

            if let Some(sm) = sample {
                for g in Group::ALL {
                    let other = if g == Group::Program { sm.other() } else { 0 };
                    section(
                        &mut dc,
                        y,
                        &tr(g.label_key()),
                        Some(sm.data.group_sum(g) + other),
                    );
                    y += row_h + px(1.0);
                    for c in Cat::ALL.into_iter().filter(|c| c.group() == g) {
                        // 파일 매핑(글꼴) = Private 밖 → 비율은 상주 기준 · 라벨에 매핑 전체 크기.
                        let (label, d) = if c.file_backed() {
                            let mapped = if c == Cat::Fonts {
                                sm.mapped.0
                            } else {
                                sm.mapped.1
                            };
                            (
                                format!(
                                    "{} \u{00B7} {}",
                                    tr(c.label_key()),
                                    trf("mem.mapped", &[&fmt(mapped)])
                                ),
                                sys.resident.max(1),
                            )
                        } else {
                            (tr(c.label_key()), denom)
                        };
                        row(
                            &mut dc,
                            y,
                            Some(c.color()),
                            &label,
                            sm.data.get(c),
                            d,
                            trend.shown(c.idx()),
                        );
                        y += row_h;
                    }
                    if g == Group::Program {
                        row(
                            &mut dc,
                            y,
                            Some(Cat::OTHER_COLOR),
                            &tr("mem.cat.runtime"),
                            other,
                            denom,
                            trend.shown(Trend::OTHER),
                        );
                        y += row_h;
                        // "기타"의 분해 행(들여쓰기 · 사용자 10-08 "미집계 49.8 MB가 무엇인지") + 모듈 · 스레드 수 한 줄(우클릭 뒤
                        // DLL 54 → 152개 같은 변화가 바로 보이게 · 라벨 열(230 px)에 맞춰 따로 둔다).
                        if let Some(parts) = sm.other_breakdown() {
                            for (key, v) in crate::memstat::RT_KEYS.iter().zip(parts) {
                                let label = format!("    \u{2514} {}", tr(key));
                                row(&mut dc, y, None, &label, v, denom, None);
                                y += row_h;
                            }
                            let info = format!(
                                "    \u{2514} {}",
                                trf(
                                    "mem.modules",
                                    &[&sm.rt.modules.to_string(), &sm.rt.threads.to_string()]
                                )
                            );
                            let ty = dc.text_center_y(y, row_h);
                            dc.text(label_x, ty, clip, &info, th.text_dim);
                            y += row_h;
                        }
                    }
                    y += px(4.0);
                }
            }
            // 운영체제가 보는 값(상주 대비) + 시스템 전체.
            section(&mut dc, y, &tr("mem.grp.system"), None);
            y += row_h + px(1.0);
            let scale_max = foot.max(sys.resident).max(1);
            // 전용 워킹 셋 = 작업 관리자 "메모리"와 같은 축(전용 = 커밋과 다름 · 사용자 10-07) · 못 세는 OS(mac)는 0 = 행 숨김.
            for (key, v, always) in [
                ("mem.sys.footprint", sys.footprint, true),
                ("mem.sys.privateWs", sys.private_ws, false),
                ("mem.sys.resident", sys.resident, true),
                ("mem.sys.file", sys.file_backed, true),
                ("mem.sys.compressed", sys.compressed, false),
                ("mem.sys.heapUsed", sys.heap_used, true),
                ("mem.sys.heapHeld", sys.heap_held, true),
            ] {
                if always || v > 0 {
                    row(&mut dc, y, None, &tr(key), v, scale_max, None);
                    y += row_h;
                }
            }
            if let Some((used, all)) = sample.and_then(|sm| sm.machine) {
                let pct = if all == 0 {
                    0.0
                } else {
                    used as f64 / all as f64 * 100.0
                };
                let line = trf(
                    "mem.system",
                    &[
                        &crate::filelist::format_size(used),
                        &crate::filelist::format_size(all),
                        &format!("{pct:.0}"),
                    ],
                );
                let ty = dc.text_center_y(y, row_h);
                dc.text(label_x, ty, clip, &line, th.text_dim);
                y += row_h;
            }

            // ── 바닥: 갱신 안내 · [닫기] — 표 **바로 아래**(바닥 고정이 아님 · 사용자 10-08) ─────────
            let btn_h = th_txt + px(12.0);
            let by = y + px(8.0);
            // 창 높이를 내용에 맞춘다: 표본이 있어 행이 다 그려진 뒤에만(첫 빈 그림으로 줄였다 다시 늘리지 않게) · 값이 바뀔 때만.
            let need = (by + btn_h + pad).max(1) as u32;
            if sample.is_some() && self.fit_h != Some(need) {
                self.fit_h = Some(need);
                win.set_min_inner_size(Some(winit::dpi::PhysicalSize::new(px(MIN_W) as u32, need)));
                let _ = win.request_inner_size(winit::dpi::PhysicalSize::new(size.width, need));
            }
            let note = match sample {
                // 방금 정리했으면 그 결과를 먼저 보여 준다(몇 초 뒤 평소 안내로 돌아간다).
                Some(_) if self.trim_note.is_some() => self
                    .trim_note
                    .as_ref()
                    .map(|n| n.0.clone())
                    .unwrap_or_default(),
                // 마지막으로 **그린**(값이 바뀐) 시각 — 경과 초 대신(매초 다시 그릴 이유가 사라진다).
                Some(_) => trf(
                    "mem.updated",
                    &[
                        &self.updated_at,
                        &format!("{:.1}", self.every_ms as f32 / 1000.0),
                    ],
                ),
                None => String::new(),
            };
            let ty = dc.text_center_y(by, btn_h);
            dc.text(pad, ty, clip, &note, th.text_dim);
            self.btn_close.set_scale(s);
            let bw = dc.text_width(&tr("license.btn.close")) + px(28.0);
            self.btn_close
                .set_bounds(Rect::new(wi - pad - bw, by, bw, btn_h), inv);
            self.btn_close.paint(&mut dc, th);
        }
        let _ = buf.present();
        self.surface = Some(surface);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memstat::Acc;

    fn sample() -> Sample {
        Sample {
            sys: Default::default(),
            data: Acc::default(),
            rt: Default::default(),
            machine: None,
            mapped: (0, 0),
        }
    }

    /// 다시 그리기 생략(T-310 개념): 표시 글이 같은 표본(수 KB 흔들림)은 서명·갱신 시각이 그대로 · 글이 바뀌는 표본(+5 MB)은 서명이 바뀐다 ·
    /// 닫으면 초기화.
    #[test]
    fn same_display_text_keeps_signature_and_updated_time() {
        let mut w = MemWin::new();
        let mb = 1024 * 1024;
        let mut a = sample();
        a.sys.footprint = 100 * mb;
        a.data.add(crate::memstat::Cat::ListsActive, 10 * mb);
        w.set_sample(a, 1000);
        let s1 = w.last_sig().expect("첫 표본 = 그림");
        assert_eq!(w.updated_at().len(), 8, "hh:mm:ss");
        let t1 = w.updated_at().to_string();
        let mut jitter = sample();
        jitter.sys.footprint = 100 * mb + 3_000;
        jitter
            .data
            .add(crate::memstat::Cat::ListsActive, 10 * mb + 700);
        w.set_sample(jitter, 1000);
        assert_eq!(w.last_sig(), Some(s1), "같은 글 = 같은 서명");
        assert_eq!(w.updated_at(), t1, "갱신 시각 그대로");
        let mut grown = sample();
        grown.sys.footprint = 100 * mb;
        grown.data.add(crate::memstat::Cat::ListsActive, 15 * mb);
        w.set_sample(grown, 1000);
        assert_ne!(w.last_sig(), Some(s1), "+5 MB = 다른 서명");
        w.close();
        assert!(w.last_sig().is_none() && w.updated_at().is_empty());
    }

    /// [힙 정리] 진행 표시: 누르면 잠기고("정리 중…") · 결과가 바닥 안내에 뜨고 · 표본이 두 번 온 뒤 버튼이 풀리며 ·
    /// 안내는 몇 표본 뒤 사라진다 · 줄지 않았으면 "반환할 것이 없음" 쪽 문구.
    #[test]
    fn trim_locks_the_button_until_samples_arrive_and_reports_the_result() {
        let mut w = MemWin::new();
        assert!(!w.trimming() && w.trim_note().is_none());
        w.begin_trim();
        assert!(w.trimming());
        w.set_trim_result(10 << 20, 4 << 20, 1500);
        let note = w.trim_note().expect("결과 안내").to_string();
        assert!(note.contains("6.00 MB") && note.contains("1.5"), "{note}");
        w.set_sample(sample(), 1000);
        assert!(w.trimming(), "즉시 표본 = 아직 잠김");
        w.set_sample(sample(), 1000);
        assert!(!w.trimming(), "다음 주기 표본 = 풀림");
        for _ in 0..TRIM_NOTE_HOLD {
            w.set_sample(sample(), 1000);
        }
        assert!(w.trim_note().is_none(), "안내는 사라진다");
        w.set_trim_result(4 << 20, 4 << 20, 300);
        assert_eq!(
            w.trim_note(),
            Some(trf("mem.trimmedNone", &["0.3"]).as_str())
        );
        // 닫으면 잠금 · 안내가 초기화된다.
        w.begin_trim();
        w.close();
        assert!(!w.trimming() && w.trim_note().is_none());
    }
}
