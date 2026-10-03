//! 전송 진행 창(T-70 · dir2 `dialog.rs` Progress 이식 — docs/port/16 DLG-059~062 · 22 OPS-216): **비모달** · 폭 400 · 1행 = 작업 라벨 ·
//! 2행 = `진행 / 전체 (pct%) · 파일 cur/count` · 세그먼트 바(nexa-ctl `SegProgress` · 높이 = 줄 높이) · 우하단 [취소].
//! 완료 = `set_done(label, ms)` → [취소]가 **[닫기 (N)]**로(N = 올림 초 · 1초마다 감소 · 0 = 자동 닫힘 · 버튼/X = 즉시) · 진행값은 그대로.
//! [취소]/X(진행 중) = `cancelled` 기록만(창은 닫지 않음) → 호스트가 폴링해 워커 취소 플래그에. 설정 `transfer.close_ms > 0`일 때만 열린다.

use ndir_i18n::{tr, trf};
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::{Button, Control, InputEvent, Invalidations, SegItem, SegProgress, Widget};
use nexa_gfx::{Font, Surface};
use std::rc::Rc;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// 창이 호스트에 요청하는 것.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProgAction {
    None,
    Paint,
}

const PAD: f32 = 12.0;
const BTN_W: f32 = 88.0;
const BTN_H: f32 = 28.0;

pub(crate) struct ProgressWin {
    window: Option<Rc<Window>>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    cursor: (i32, i32),
    label: String,
    bar: SegProgress,
    cur: usize,
    count: usize,
    btn: Button,
    /// 진행 중 [취소]/X를 눌렀다(1회성 · 호스트가 꺼낸다).
    cancelled: bool,
    /// 닫기 모드: (남은 ms · 마지막 틱 ms) — Some이면 [닫기 (N)].
    closing: Option<(u64, u64)>,
    /// 전송이 이 창을 쓰는 중(`reset` ~ 닫힘) — 창 생성 전/창 없는 시험에서도 `set_done`을 받는다.
    active: bool,
}

impl ProgressWin {
    pub(crate) fn new() -> Self {
        ProgressWin {
            window: None,
            surface: None,
            scale: 1.0,
            cursor: (0, 0),
            label: String::new(),
            bar: SegProgress::new(),
            cur: 0,
            count: 0,
            btn: Button::new(tr("ops.cancel")),
            cancelled: false,
            closing: None,
            active: false,
        }
    }

    /// 새 전송 시작 — 라벨 교체 · 진행 0 · 취소/닫기 상태 초기화.
    pub(crate) fn reset(&mut self, label: &str) {
        self.label = label.to_string();
        let mut inv = Invalidations::default();
        self.bar.set_items(Vec::new(), &mut inv);
        self.bar.set_totals(0, 0, &mut inv);
        self.cur = 0;
        self.count = 0;
        self.cancelled = false;
        self.closing = None;
        self.active = true;
        self.btn.set_label(tr("ops.cancel"));
        self.redraw();
    }

    /// 전송이 이 창을 쓰는 중인가(`reset` 뒤 · 닫히면 거짓).
    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    /// 값 갱신(틱마다 — 바뀐 것만 무효화).
    pub(crate) fn update(
        &mut self,
        done: u64,
        total: u64,
        items: Vec<SegItem>,
        cur: usize,
        count: usize,
    ) {
        let mut inv = Invalidations::default();
        self.bar.set_totals(done, total, &mut inv);
        self.bar.set_items(items, &mut inv);
        let changed = !inv.is_empty() || (self.cur, self.count) != (cur, count);
        self.cur = cur;
        self.count = count;
        if changed {
            self.redraw();
        }
    }

    /// 완료 — 라벨 교체 + [닫기 (N)] 카운트다운 시작(`ms` 뒤 자동 닫힘 · 0 = 바로).
    pub(crate) fn set_done(&mut self, label: &str, ms: u64, now_ms: u64) {
        self.label = label.to_string();
        self.closing = Some((ms, now_ms));
        self.btn.set_label(self.close_label(ms));
        self.redraw();
    }

    fn close_label(&self, remain_ms: u64) -> String {
        format!("{} ({})", tr("ops.close"), remain_ms.div_ceil(1000))
    }

    /// 진행 중 취소 요청을 꺼낸다(1회성).
    pub(crate) fn take_cancelled(&mut self) -> bool {
        std::mem::take(&mut self.cancelled)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn is_closing(&self) -> bool {
        self.closing.is_some()
    }

    /// 틱: 닫기 카운트다운(1초마다 라벨 · 0 = 닫힘) · 버튼 애니메이션. 다시 그려야 하면 true.
    pub(crate) fn tick(&mut self, now_ms: u64) -> bool {
        let mut any = self.btn.tick(now_ms);
        if let Some((remain, last)) = self.closing {
            let elapsed = now_ms.saturating_sub(last);
            if elapsed >= remain {
                self.close();
                return true;
            }
            // 1초 단위 라벨 갱신(남은 초가 바뀔 때만).
            let new_remain = remain - elapsed;
            if new_remain.div_ceil(1000) != remain.div_ceil(1000) {
                self.closing = Some((new_remain, now_ms));
                self.btn.set_label(self.close_label(new_remain));
                any = true;
            }
        }
        any
    }

    pub(crate) fn animating(&self) -> bool {
        self.window.is_some() && (self.closing.is_some() || self.btn.is_animating())
    }

    /// 자체 시험 덤프(`progress.dump`): `open|closed · label · done/total pct · file cur/count · closing · 항목 상태들`.
    pub(crate) fn dump(&self) -> String {
        let (done, total) = self.bar.totals();
        let items: Vec<String> = self
            .bar
            .items()
            .iter()
            .map(|i| format!("{:?}", i.status).to_ascii_lowercase())
            .collect();
        format!(
            "{} {} | {done}/{total} {}% | file {}/{} | closing {} | cancelled {} | {}\n",
            if self.window.is_some() {
                "open"
            } else {
                "closed"
            },
            self.label,
            self.bar.percent(),
            self.cur,
            self.count,
            self.closing.is_some(),
            self.cancelled,
            items.join(",")
        )
    }

    pub(crate) fn is(&self, id: WindowId) -> bool {
        self.window.as_ref().is_some_and(|w| w.id() == id)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn is_open(&self) -> bool {
        self.window.is_some()
    }

    pub(crate) fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn s(&self, v: f32) -> i32 {
        (v * self.scale).round() as i32
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
        // dir2 DLG-059: 폭 400 · 소유자 중앙에서 110px 위.
        let (lw, lh) = (400.0, 132.0);
        let mut attrs = Window::default_attributes()
            .with_title(format!("Nexa Dir — {}", tr("ops.progressTitle")))
            .with_theme(theme)
            .with_resizable(false)
            .with_inner_size(winit::dpi::LogicalSize::new(lw, lh));
        if let Some((x, y, w, h)) = over {
            let cx = x + (w as i32 - lw as i32) / 2;
            let cy = y + (h as i32 - lh as i32) / 2 - 110;
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
        self.redraw();
    }

    pub(crate) fn close(&mut self) {
        self.surface = None;
        self.window = None;
        self.closing = None;
        self.active = false;
        self.btn.clear_transient();
    }

    fn layout(&mut self) {
        let Some(win) = &self.window else { return };
        let size = win.inner_size();
        let (w, h) = (size.width as i32, size.height as i32);
        let pad = self.s(PAD);
        let mut inv = Invalidations::default();
        let (bw, bh) = (self.s(BTN_W), self.s(BTN_H));
        self.btn.set_scale(self.scale);
        self.btn
            .set_bounds(Rect::new(w - pad - bw, h - pad - bh, bw, bh), &mut inv);
    }

    pub(crate) fn handle(&mut self, ev: &WindowEvent) -> ProgAction {
        match ev {
            WindowEvent::CloseRequested => {
                // 진행 중 X = 취소 기록만(창 유지) · 닫기 모드 X = 즉시 닫힘.
                if self.closing.is_some() {
                    self.close();
                } else {
                    self.cancelled = true;
                }
                return ProgAction::None;
            }
            WindowEvent::RedrawRequested => return ProgAction::Paint,
            WindowEvent::Resized(_) => {
                self.layout();
                self.redraw();
                return ProgAction::None;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.layout();
                self.redraw();
                return ProgAction::None;
            }
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                if matches!(kev.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    self.press_button();
                }
                return ProgAction::None;
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as i32, position.y as i32);
                let (x, y) = self.cursor;
                let mut inv = Invalidations::default();
                self.btn.on_event(&InputEvent::MouseMove { x, y }, &mut inv);
                if !inv.is_empty() {
                    self.redraw();
                }
                return ProgAction::None;
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
                let mut inv = Invalidations::default();
                let p = Point { x, y };
                if matches!(e, InputEvent::MouseUp { .. }) || self.btn.bounds().contains(p) {
                    self.btn.on_event(&e, &mut inv);
                }
                if self.btn.take_clicked() {
                    self.press_button();
                }
                self.redraw();
                return ProgAction::None;
            }
            _ => {}
        }
        ProgAction::None
    }

    /// [취소](진행 중 = 취소 기록) / [닫기 (N)](닫기 모드 = 즉시 닫힘) — Esc도 같다.
    pub(crate) fn press_button(&mut self) {
        if self.closing.is_some() {
            self.close();
        } else {
            self.cancelled = true;
        }
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
        let pad = (PAD * s).round() as i32;
        {
            let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
            let prefs = FontPrefs::with_base(font_px);
            let mut dc = RasterCtx::new(&mut gfx, ui, s).with_fonts(prefs);
            dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
            dc.select_font(FontSlot::Base, false);
            let th_txt = dc.text_height();
            let clip = Rect::new(0, 0, wi, hi);
            let mut y = pad;
            dc.text(pad, y, clip, &self.label, th.text);
            y += th_txt + (4.0 * s).round() as i32;
            let (done, total) = self.bar.totals();
            let line = format!(
                "{} / {} ({}%) · {}",
                crate::filelist::format_size(done),
                crate::filelist::format_size(total),
                self.bar.percent(),
                trf(
                    "ops.fileCount",
                    &[&self.cur.to_string(), &self.count.to_string()]
                )
            );
            dc.text(pad, y, clip, &line, th.text_dim);
            y += th_txt + (6.0 * s).round() as i32;
            let mut inv = Invalidations::default();
            self.bar.set_scale(s);
            self.bar
                .set_bounds(Rect::new(pad, y, wi - pad * 2, th_txt.max(10)), &mut inv);
            self.bar.paint(&mut dc, th);
            self.btn.paint(&mut dc, th);
        }
        let _ = buf.present();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexa_ctl::SegStatus;

    /// 창 없이: 갱신 → 덤프 · 취소 1회성 · 완료 카운트다운(2000 ms → "닫기 (2)" → 1초 뒤 "(1)" → 만료 = 닫힘 상태).
    #[test]
    fn update_cancel_and_countdown_without_window() {
        ndir_i18n::activate(ndir_i18n::load("en", std::path::Path::new("nowhere")));
        let mut w = ProgressWin::new();
        w.reset("Transferring...");
        w.update(
            50,
            200,
            vec![
                SegItem {
                    size: 100,
                    done: 100,
                    status: SegStatus::Done,
                },
                SegItem {
                    size: 100,
                    done: 50,
                    status: SegStatus::Active,
                },
            ],
            2,
            2,
        );
        let d = w.dump();
        assert_eq!(
            d,
            "closed Transferring... | 50/200 25% | file 2/2 | closing false | cancelled false | done,active\n"
        );
        assert!(!w.take_cancelled());
        w.press_button();
        assert!(w.take_cancelled() && !w.take_cancelled(), "1회성");
        w.set_done("Done - closing shortly", 2000, 10_000);
        assert!(w.is_closing());
        assert_eq!(w.btn.label(), "Close (2)");
        assert!(!w.tick(10_500), "같은 초 = 변화 없음");
        assert!(w.tick(11_100), "남은 0.9 s → (1)");
        assert_eq!(w.btn.label(), "Close (1)");
        assert!(w.tick(12_100), "만료 = 닫힘");
        assert!(!w.is_closing() && !w.is_open());
        assert!(
            w.dump()
                .starts_with("closed Done - closing shortly | 50/200 25%"),
            "진행값 유지"
        );
    }
}
