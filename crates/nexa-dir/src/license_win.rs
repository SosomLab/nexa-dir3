//! 라이선스 창(T-80 · docs/port/42 LIC-158~161 · nexa-sql `license_win.rs` 이식): Help ▸ 라이선스… · About [라이선스…] → 모덜리스 창.
//! 위 = 상태 띠 + 표(상태 · 파일 · [id · 사용자 · 종류/등급 · 기능 · 발급일 · 기한 · 만료 · 메이저] · 앱 버전 · 빌드일 · 기기 코드 · 설치 자리) ·
//! 가운데 = 요청 코드(이름 · 이메일 · 복사 · 이메일 링크 클릭 = 복사 + Flash) · 아래 = [라이선스 파일 열기…] [제거] [닫기] + 결과 안내.
//! 판정·설치는 호스트의 `ndir_license::Licensing`(단일 원천) — 창은 보기·요청만. 골격은 `check_win.rs`와 같다.

use ndir_i18n::{tr, trf};
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::{Button, Control, Flash, FlashTone, InputEvent, Invalidations, TextBox, Widget};
use nexa_gfx::{Font, Surface};
use std::rc::Rc;
use winit::event::{ElementState, Ime, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// 호스트가 그릴 때 넘기는 보기(라이선스 상태의 단일 원천은 `Licensing`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LicView {
    /// 상태 한 줄(About 배지와 같은 글) · 경고색 여부.
    pub state: String,
    pub warn: bool,
    /// (라벨, 값) 표 — 호스트가 i18n으로 만든다.
    pub rows: Vec<(String, String)>,
    /// 요청 코드(기기 ID 없으면 None).
    pub request: Option<String>,
    /// 요청 코드를 보내는 이메일(링크 · 클릭 = 복사).
    pub contact: String,
}

/// 창이 호스트에 요청하는 것.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LicAction {
    None,
    Paint,
    Close,
    /// 파일 창을 열어 라이선스 파일을 고른다.
    OpenFile,
    Remove,
    /// 요청 코드를 클립보드로(이름 · 이메일 = 메타).
    CopyRequest(String, String),
    /// 링크 글자(이메일)를 클립보드로.
    CopyText(String),
}

/// 순간 메시지 유지/페이드(ms) — dir3에는 `ui.flash_*` 설정이 없다(nexa-sql LIC-100 수치 그대로).
const FLASH_HOLD_MS: u64 = 1200;
const FLASH_FADE_MS: u64 = 700;

pub(crate) struct LicenseWin {
    window: Option<Rc<Window>>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    cursor: (i32, i32),
    shift: bool,
    primary: bool,
    tb_name: TextBox,
    tb_email: TextBox,
    btn_copy: Button,
    btn_open: Button,
    btn_remove: Button,
    btn_close: Button,
    /// 안내 줄(설치됨 · 거부 · 복사됨) · 경고색.
    note: Option<(String, bool)>,
    /// 마지막 페인트의 보기.
    last: LicView,
    /// 다음 페인트에서 창 높이를 내용에 맞춘다(열 때 · 상태가 바뀔 때 — 무료 6행/정식 14행).
    fit: bool,
    /// 이메일 링크 자리(마지막 페인트) · hover · 순간 메시지(nexa-ctl `Flash` · 링크 옆).
    link_rect: Rect,
    link_hover: bool,
    flash: Flash,
}

impl LicenseWin {
    pub(crate) fn new() -> Self {
        LicenseWin {
            window: None,
            surface: None,
            scale: 1.0,
            cursor: (0, 0),
            shift: false,
            primary: false,
            tb_name: TextBox::new(""),
            tb_email: TextBox::new(""),
            btn_copy: Button::new(tr("license.btn.copyReq")),
            btn_open: Button::new(tr("license.btn.open")),
            btn_remove: Button::new(tr("license.btn.remove")),
            btn_close: Button::new(tr("license.btn.close")),
            note: None,
            last: LicView::default(),
            fit: true,
            link_rect: Rect::default(),
            link_hover: false,
            flash: Flash::new(),
        }
    }

    /// 언어 전환 — 버튼 글 · 창 제목(T-134 · 표의 라벨은 그릴 때마다 호스트가 만든다).
    pub(crate) fn relabel(&mut self) {
        self.btn_copy.set_label(tr("license.btn.copyReq"));
        self.btn_open.set_label(tr("license.btn.open"));
        self.btn_remove.set_label(tr("license.btn.remove"));
        self.btn_close.set_label(tr("license.btn.close"));
        if let Some(w) = &self.window {
            w.set_title(&format!("Nexa Dir — {}", tr("license.title")));
        }
        self.fit = true;
        self.redraw();
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
        // 높이 = 정식 상태의 표(14행) + 요청 코드 구역 + 버튼 행 — 첫 페인트가 내용에 맞춘다(`fit`).
        let (lw, lh) = (640.0, 540.0);
        let mut attrs = Window::default_attributes()
            .with_title(format!("Nexa Dir — {}", tr("license.title")))
            .with_theme(theme)
            .with_resizable(true)
            .with_min_inner_size(winit::dpi::LogicalSize::new(520.0, 380.0))
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
        win.set_ime_allowed(crate::input::system_ime());
        self.window = Some(win);
        self.note = None;
        self.fit = true;
        self.redraw();
    }

    /// 순간 메시지(링크 옆 · 서서히 사라짐 · 링크를 가리지 않는다).
    pub(crate) fn set_flash(&mut self, text: String, warn: bool) {
        self.flash.show(
            text,
            if warn { FlashTone::Warn } else { FlashTone::Ok },
            FLASH_HOLD_MS,
            FLASH_FADE_MS,
        );
        self.redraw();
    }

    /// 호스트: 설치/제거/복사 결과 한 줄.
    pub(crate) fn set_note(&mut self, text: String, warn: bool) {
        self.note = Some((text, warn));
        self.fit = true;
        self.redraw();
    }

    /// 마지막 안내(덤프 `license` · `note=ok|warn:<글>`).
    pub(crate) fn note(&self) -> Option<&(String, bool)> {
        self.note.as_ref()
    }

    pub(crate) fn close(&mut self) {
        self.surface = None;
        self.window = None;
        for b in self.buttons() {
            b.clear_transient();
        }
        self.tb_name.set_focused(false);
        self.tb_email.set_focused(false);
    }

    /// 파일 창의 주인으로 넘길 때(그 위에 뜨고 · 닫히면 이 창으로 포커스가 돌아온다).
    pub(crate) fn window_rc(&self) -> Option<Rc<Window>> {
        self.window.clone()
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

    /// 틱: 버튼 애니메이션. 다시 그려야 하면 true.
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
            && [
                &self.btn_copy,
                &self.btn_open,
                &self.btn_remove,
                &self.btn_close,
            ]
            .iter()
            .any(|b| b.is_animating())
    }

    fn buttons(&mut self) -> [&mut Button; 4] {
        [
            &mut self.btn_copy,
            &mut self.btn_open,
            &mut self.btn_remove,
            &mut self.btn_close,
        ]
    }

    fn focused_box(&mut self) -> Option<&mut TextBox> {
        if self.tb_name.is_focused() {
            Some(&mut self.tb_name)
        } else if self.tb_email.is_focused() {
            Some(&mut self.tb_email)
        } else {
            None
        }
    }

    /// 포커스 링은 하나.
    fn own_focus(&mut self, p: Point) {
        let hit = [self.tb_name.bounds(), self.tb_email.bounds()]
            .iter()
            .position(|b| b.contains(p));
        self.tb_name.set_focused(hit == Some(0));
        self.tb_email.set_focused(hit == Some(1));
    }

    pub(crate) fn handle(&mut self, ev: &WindowEvent) -> LicAction {
        let mut inv = Invalidations::default();
        match ev {
            WindowEvent::CloseRequested => return LicAction::Close,
            WindowEvent::RedrawRequested => return LicAction::Paint,
            WindowEvent::Resized(_) => self.redraw(),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.redraw();
            }
            WindowEvent::Focused(false) => {
                for b in self.buttons() {
                    b.clear_transient();
                }
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
                for b in self.buttons() {
                    b.on_event(&mv, &mut inv);
                }
                for tb in [&mut self.tb_name, &mut self.tb_email] {
                    tb.on_event(&mv, &mut inv);
                }
                let over = self.link_rect.contains(Point { x, y });
                if over != self.link_hover {
                    self.link_hover = over;
                    if let Some(w) = &self.window {
                        w.set_cursor(if over {
                            winit::window::CursorIcon::Pointer
                        } else {
                            winit::window::CursorIcon::Default
                        });
                    }
                    self.redraw();
                }
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
                    self.own_focus(p);
                    // 이메일 링크 클릭 = 주소 복사.
                    if self.link_rect.contains(p) && !self.last.contact.is_empty() {
                        return LicAction::CopyText(self.last.contact.clone());
                    }
                }
                // 마우스 라우팅 규칙: 누름은 커서 아래 컨트롤에만 · 뗌은 전부.
                for tb in [&mut self.tb_name, &mut self.tb_email] {
                    if up || tb.bounds().contains(p) {
                        tb.on_event(&e, &mut inv);
                    }
                }
                for b in self.buttons() {
                    if up || b.bounds().contains(p) {
                        b.on_event(&e, &mut inv);
                    }
                    b.set_focused(false);
                }
                self.redraw();
                if self.btn_copy.take_clicked() {
                    return LicAction::CopyRequest(
                        self.tb_name.text().trim().to_string(),
                        self.tb_email.text().trim().to_string(),
                    );
                }
                if self.btn_open.take_clicked() {
                    return LicAction::OpenFile;
                }
                if self.btn_remove.take_clicked() {
                    return LicAction::Remove;
                }
                if self.btn_close.take_clicked() {
                    return LicAction::Close;
                }
            }
            WindowEvent::MouseInput { state, button, .. }
                if *button == MouseButton::Right && *state == ElementState::Pressed =>
            {
                let (x, y) = self.cursor;
                let p = Point { x, y };
                for tb in [&mut self.tb_name, &mut self.tb_email] {
                    if tb.bounds().contains(p) {
                        tb.on_event(&InputEvent::RightDown { x, y }, &mut inv);
                    }
                }
                self.redraw();
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                if let Some(tb) = self.focused_box() {
                    tb.set_preedit("", &mut inv);
                    for c in text.chars().filter(|c| !c.is_control()) {
                        tb.on_event(&InputEvent::Char { c, now_ms: 0 }, &mut inv);
                    }
                }
                self.redraw();
            }
            WindowEvent::Ime(Ime::Preedit(text, _)) => {
                if let Some(tb) = self.focused_box() {
                    tb.set_preedit(text, &mut inv);
                }
                self.redraw();
            }
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                if matches!(kev.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    return LicAction::Close;
                }
                if let Key::Character(_) = kev.logical_key.as_ref() {
                    if self.primary && crate::input::shortcut_letter(kev) == Some('a') {
                        if let Some(tb) = self.focused_box() {
                            tb.on_event(&InputEvent::SelectAll, &mut inv);
                        }
                        self.redraw();
                        return LicAction::None;
                    }
                }
                if let Some(e) = crate::input::text_key_event(
                    kev,
                    self.shift,
                    self.primary,
                    crate::input::TextKeys::Line,
                ) {
                    if let Some(tb) = self.focused_box() {
                        tb.on_event(&e, &mut inv);
                    }
                    self.redraw();
                }
            }
            _ => {}
        }
        LicAction::None
    }

    pub(crate) fn paint(&mut self, view: LicView, font: &Font, th: &Theme, ui_px: f32) {
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
        {
            let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
            let prefs = FontPrefs::with_base(ui_px);
            let mut dc = RasterCtx::new(&mut gfx, font, s).with_fonts(prefs);
            dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
            dc.select_font(FontSlot::Base, false);
            let th_txt = dc.text_height();
            let pad = px(12.0);
            let clip = Rect::new(0, 0, wi, hi);
            // 상태 띠(배지와 같은 글 · 경고색 · 알파 배경).
            let color = if view.warn { th.warn } else { th.ok };
            dc.fill_rect_alpha(
                Rect::new(pad, pad, wi - pad * 2, th_txt + px(10.0)),
                color,
                0.12,
            );
            dc.text(pad + px(8.0), pad + px(5.0), clip, &view.state, color);
            let mut y = pad + th_txt + px(18.0);
            // 표(라벨 dim · 값은 가운데 생략).
            let label_w = view
                .rows
                .iter()
                .map(|(k, _)| dc.text_width(k))
                .max()
                .unwrap_or(0)
                + px(16.0);
            let row_h = th_txt + px(6.0);
            for (k, v) in &view.rows {
                dc.text(pad, y, clip, k, th.text_dim);
                let vr = Rect::new(pad + label_w, y, wi - pad * 2 - label_w, row_h);
                let v = nexa_ctl::draw::ellipsize_middle(&mut dc, v, vr.w);
                dc.text(vr.x, y, vr, &v, th.text);
                y += row_h;
            }
            y += px(10.0);
            dc.fill_rect(Rect::new(pad, y, wi - pad * 2, 1), th.border);
            y += px(10.0);
            // 요청 코드 구역 — 제목 줄의 이메일은 링크(강조색 + 밑줄 · hover = 손 · 클릭 = 복사).
            let tpl = trf("license.req.title", &["\u{1}"]);
            let (pre, post) = tpl.split_once('\u{1}').unwrap_or((tpl.as_str(), ""));
            let mut tx = pad;
            dc.text(tx, y, clip, pre, th.text);
            tx += dc.text_width(pre);
            let lw = dc.text_width(&view.contact);
            let link_c = if self.link_hover {
                th.accent
            } else {
                th.accent.lerp(th.text, 0.25)
            };
            dc.text(tx, y, clip, &view.contact, link_c);
            dc.fill_rect(Rect::new(tx, y + th_txt - px(2.0), lw, 1), link_c);
            self.link_rect = Rect::new(tx, y, lw, th_txt + px(2.0));
            tx += lw;
            dc.text(tx, y, clip, post, th.text);
            y += th_txt + px(8.0);
            let fh = th_txt + px(12.0);
            let mut x = pad;
            for (label, tb, w) in [
                (tr("license.req.name"), &mut self.tb_name, px(150.0)),
                (tr("license.req.email"), &mut self.tb_email, px(200.0)),
            ] {
                dc.text(x, y + (fh - th_txt) / 2, clip, &label, th.text_dim);
                x += dc.text_width(&label) + px(6.0);
                tb.set_scale(s);
                tb.set_bounds(Rect::new(x, y, w, fh), inv);
                tb.paint(&mut dc, th);
                x += w + px(12.0);
            }
            self.btn_copy.set_scale(s);
            let cw = dc.text_width(&tr("license.btn.copyReq")) + px(28.0);
            self.btn_copy.set_enabled(view.request.is_some());
            self.btn_copy.set_bounds(Rect::new(x, y, cw, fh), inv);
            self.btn_copy.paint(&mut dc, th);
            y += fh + px(6.0);
            let code_line = view
                .request
                .clone()
                .unwrap_or_else(|| tr("license.noMachine"));
            let code_line = nexa_ctl::draw::ellipsize_middle(&mut dc, &code_line, wi - pad * 2);
            // 버튼 행은 창 맨 아래 고정 · 위 내용은 온전한 줄만 그린다.
            let btn_h = th_txt + px(14.0);
            let btn_y = hi - pad - btn_h;
            let floor = btn_y - px(10.0);
            if y + th_txt <= floor {
                dc.text(pad, y, clip, &code_line, th.text_dim);
            }
            y += th_txt + px(6.0);
            if y + th_txt <= floor {
                dc.text(pad, y, clip, &tr("license.hint"), th.text_dim);
            }
            y += th_txt;
            // 창 높이 = 내용 끝 + 버튼 행(열 때·상태 변화 때 한 번 · 사용자가 늘린 뒤엔 건드리지 않는다).
            if self.fit {
                self.fit = false;
                let need = y + px(12.0) + btn_h + pad;
                if need != hi {
                    let _ = win.request_inner_size(winit::dpi::PhysicalSize::new(
                        size.width,
                        need.max(px(200.0)) as u32,
                    ));
                }
            }
            let mut bx = pad;
            for (b, key) in [
                (&mut self.btn_open, "license.btn.open"),
                (&mut self.btn_remove, "license.btn.remove"),
                (&mut self.btn_close, "license.btn.close"),
            ] {
                b.set_scale(s);
                let bw = (dc.text_width(&tr(key)) + px(28.0)).max(px(84.0));
                b.set_bounds(Rect::new(bx, btn_y, bw, btn_h), inv);
                b.paint(&mut dc, th);
                bx += bw + px(8.0);
            }
            // 결과 안내(설치됨 · 거부 · 복사됨)는 버튼 오른쪽 같은 줄.
            if let Some((n, warn)) = &self.note {
                let nx = bx + px(4.0);
                let n = nexa_ctl::draw::ellipsize_middle(&mut dc, n, (wi - pad - nx).max(0));
                let r = Rect::new(nx, btn_y, (wi - pad - nx).max(0), btn_h);
                dc.text(
                    nx,
                    btn_y + (btn_h - th_txt) / 2,
                    r,
                    &n,
                    if *warn { th.danger } else { th.ok },
                );
            }
        }
        for tb in [&self.tb_name, &self.tb_email] {
            if tb.popup_open() {
                let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
                let prefs = FontPrefs::with_base(ui_px);
                let mut dc = RasterCtx::new(&mut gfx, font, s).with_fonts(prefs);
                tb.paint_popup(&mut dc, th);
            }
        }
        // 플래시 메시지 = 창의 맨 마지막(다른 컨트롤·팝업이 덮지 않음) · 진행 중이면 다시 그린다.
        {
            let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
            let prefs = FontPrefs::with_base(ui_px);
            let mut dc = RasterCtx::new(&mut gfx, font, s).with_fonts(prefs);
            dc.select_font(FontSlot::Base, false);
            if self
                .flash
                .paint(&mut dc, th, self.link_rect, Rect::new(0, 0, wi, hi))
            {
                win.request_redraw();
            }
        }
        let _ = buf.present();
        self.surface = Some(surface);
        self.last = view;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 창 없이: 안내 줄 기록 · 닫기 = 포커스/버튼 상태 정리(패닉 없음).
    #[test]
    fn note_and_close_without_window() {
        ndir_i18n::activate(ndir_i18n::load("en", std::path::Path::new("nowhere")));
        let mut w = LicenseWin::new();
        assert!(!w.is_open() && w.note().is_none());
        w.set_note("License installed: X".into(), false);
        assert_eq!(w.note(), Some(&("License installed: X".to_string(), false)));
        assert!(!w.tick(10) && !w.animating());
        w.close();
        assert!(!w.is_open());
    }
}
