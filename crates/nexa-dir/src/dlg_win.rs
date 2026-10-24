//! 대화상자 창(T-29 A · "nexa-dlg" 앱 층 — docs/port/20 §4-6 "winit는 중첩 루프가 없다 → 보조 창 + 상태기계"):
//! 제목 · 본문(줄 바꿈) · 버튼 N개(기본/취소) · 선택적 입력란(마스킹 — 암호). 결과는 [`DlgAction::Done`]로 호스트에 돌아가고,
//! 열린 동안 메인 창 입력은 호스트가 막는다(모달). 소비자: 영구 삭제 확인 · 전송 충돌 4버튼 · 압축 암호(T-62 B).
//! 창 없이도 동작(시험·기동 명령 `dlg.pick`): 사양만 들고 결과를 바로 돌려준다.

use ndir_i18n::tr;
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::{Button, Control, InputEvent, Invalidations, Key as CtlKey, TextBox, Widget};
use nexa_gfx::{Font, Surface};
use std::rc::Rc;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// 입력란 사양.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DlgInput {
    pub label: String,
    pub masked: bool,
    pub initial: String,
    /// 빈 칸에 흐리게 보이는 안내(사용자 10-18 "설명은 플레이스홀더로").
    pub placeholder: String,
}

/// 무장 버튼(사용자 10-18 "중복이면 별도 창이 아니라 그 자리에서 버튼만 강조된 타임아웃 버튼으로 재확인 · 시간 지나면 원복"):
/// 입력란 글(정리한 이름 · 비면 `empty_as`)이 `names`에 있으면 `id` 버튼의 첫 누름은 확정하지 않고 **무장**한다(라벨 `label (N)` · 위험색 ·
/// `ms` 동안) — 그 안에 다시 누르면 확정 · 지나면 원래 라벨로 돌아간다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DlgArm {
    pub id: i32,
    pub label: String,
    pub ms: u64,
    pub names: Vec<String>,
    pub empty_as: String,
}

/// 대화상자 사양.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DlgSpec {
    pub title: String,
    pub text: String,
    /// (id, 라벨) — 왼쪽부터 오른쪽.
    pub buttons: Vec<(i32, String)>,
    /// Enter = 이 id.
    pub default: i32,
    /// Esc · 닫기 = 이 id.
    pub cancel: i32,
    pub input: Option<DlgInput>,
    /// (버튼 id, ms): 그 버튼은 ms 동안 눌리지 않고 라벨에 남은 초를 보인다 — 되돌릴 수 없는 확정의 2중 확인(사용자 10-18).
    pub delay: Option<(i32, u64)>,
    /// 무장 버튼(그 자리 재확인) — [`DlgArm`].
    pub arm: Option<DlgArm>,
}

impl DlgSpec {
    /// 확인/취소 2버튼(확인 = 1 · 취소 = 0).
    pub(crate) fn confirm(title: String, text: String, ok_label: String) -> Self {
        DlgSpec {
            title,
            text,
            buttons: vec![(1, ok_label), (0, tr("dlg.cancel"))],
            default: 1,
            cancel: 0,
            input: None,
            delay: None,
            arm: None,
        }
    }

    /// 덤프 한 줄(`title | text | 1:OK 0:Cancel | input`).
    pub(crate) fn dump(&self) -> String {
        let btns: Vec<String> = self
            .buttons
            .iter()
            .map(|(id, l)| format!("{id}:{l}"))
            .collect();
        format!(
            "{} | {} | {} | {}",
            self.title,
            self.text.replace('\n', " / "),
            btns.join(" "),
            self.input
                .as_ref()
                .map_or("-".to_string(), |i| if i.masked {
                    "input(masked)".into()
                } else {
                    "input".into()
                })
        ) + &self
            .delay
            .map(|(id, ms)| format!(" | wait {id}:{ms}"))
            .unwrap_or_default()
            + &self
                .arm
                .as_ref()
                .map(|a| format!(" | arm {}:{}", a.id, a.ms))
                .unwrap_or_default()
    }
}

/// 창이 호스트에 요청하는 것.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DlgAction {
    None,
    Paint,
    /// 버튼 id + 입력란 텍스트(입력 사양이 있을 때).
    Done {
        id: i32,
        text: Option<String>,
    },
}

const PAD: f32 = 16.0;
const BTN_H: f32 = 28.0;
const BTN_W: f32 = 104.0;
const INPUT_H: f32 = 30.0;
const WIDTH: f32 = 460.0;

pub(crate) struct DlgWin {
    window: Option<Rc<Window>>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    cursor: (i32, i32),
    shift: bool,
    primary: bool,
    spec: Option<DlgSpec>,
    buttons: Vec<Button>,
    input: Option<TextBox>,
    done: Option<(i32, Option<String>)>,
    /// 잠금 버튼(사양 `delay`): 첫 틱부터 ms 동안 끔 + 라벨 `(N)`.
    wait: Option<Wait>,
    /// 무장 상태(사양 `arm`): 무장한 뒤 첫 틱의 시각(ms) — 없으면 무장 안 됨.
    armed: Option<Option<u64>>,
}

/// 잠금 버튼 상태.
struct Wait {
    idx: usize,
    id: i32,
    label: String,
    total_ms: u64,
    started: Option<u64>,
    done: bool,
}

impl DlgWin {
    pub(crate) fn new() -> Self {
        DlgWin {
            window: None,
            surface: None,
            scale: 1.0,
            cursor: (0, 0),
            shift: false,
            primary: false,
            spec: None,
            buttons: Vec::new(),
            input: None,
            done: None,
            wait: None,
            armed: None,
        }
    }

    pub(crate) fn spec(&self) -> Option<&DlgSpec> {
        self.spec.as_ref()
    }

    pub(crate) fn is(&self, id: WindowId) -> bool {
        self.window.as_ref().is_some_and(|w| w.id() == id)
    }

    pub(crate) fn is_open(&self) -> bool {
        self.window.is_some()
    }

    pub(crate) fn focus(&self) {
        if let Some(w) = &self.window {
            w.focus_window();
        }
    }

    pub(crate) fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    pub(crate) fn tick(&mut self, now_ms: u64) -> bool {
        // 잠금·무장 카운트다운은 창 없이도 돈다(시험 · 창 생성 전 틱).
        let wait = self.wait_tick(now_ms) | self.arm_tick(now_ms);
        if self.window.is_none() {
            return wait;
        }
        let mut any = wait;
        for b in &mut self.buttons {
            any |= b.tick(now_ms);
        }
        if let Some(tb) = &mut self.input {
            any |= tb.tick(now_ms);
        }
        any
    }

    /// 잠금 버튼 카운트다운(시각은 호스트가 준다 = 결정적). 라벨이 바뀌거나 풀리면 `true`.
    fn wait_tick(&mut self, now_ms: u64) -> bool {
        let Some(w) = self.wait.as_mut() else {
            return false;
        };
        if w.done {
            return false;
        }
        let started = *w.started.get_or_insert(now_ms);
        let elapsed = now_ms.saturating_sub(started);
        let Some(b) = self.buttons.get_mut(w.idx) else {
            return false;
        };
        if elapsed >= w.total_ms {
            w.done = true;
            b.set_label(w.label.clone());
            b.set_enabled(true);
            return true;
        }
        let secs = (w.total_ms - elapsed).div_ceil(1000);
        b.set_label(format!("{} ({secs})", w.label));
        true
    }

    /// 그 id가 아직 잠겨 있는가(Enter · 클릭 · `pick` 모두 거부).
    fn locked(&self, id: i32) -> bool {
        self.wait.as_ref().is_some_and(|w| w.id == id && !w.done)
    }

    /// 지금 입력란 글이 무장을 요구하는가(`arm.names`에 있는 이름).
    fn arm_needed(&self, id: i32) -> bool {
        let Some(arm) = self.spec.as_ref().and_then(|s| s.arm.as_ref()) else {
            return false;
        };
        if arm.id != id {
            return false;
        }
        let text = self.input_text().unwrap_or_default();
        let name = crate::App::sanitize_preset_name(&text).unwrap_or_else(|| arm.empty_as.clone());
        arm.names.contains(&name)
    }

    fn arm_button(&mut self) -> Option<(usize, DlgArm)> {
        let arm = self.spec.as_ref()?.arm.clone()?;
        let idx = self
            .spec
            .as_ref()?
            .buttons
            .iter()
            .position(|(i, _)| *i == arm.id)?;
        Some((idx, arm))
    }

    /// 무장(라벨 `label (N)` · 위험색).
    fn arm(&mut self) {
        let Some((idx, arm)) = self.arm_button() else {
            return;
        };
        if let Some(b) = self.buttons.get_mut(idx) {
            b.set_label(format!("{} ({})", arm.label, arm.ms.div_ceil(1000)));
            b.set_tone(nexa_ctl::controls::ButtonTone::Danger);
        }
        self.armed = Some(None);
    }

    /// 원복(원래 라벨 · 기본 톤).
    fn disarm(&mut self) {
        self.armed = None;
        let Some((idx, _)) = self.arm_button() else {
            return;
        };
        let label = self
            .spec
            .as_ref()
            .map(|s| s.buttons[idx].1.clone())
            .unwrap_or_default();
        if let Some(b) = self.buttons.get_mut(idx) {
            b.set_label(label);
            b.set_tone(nexa_ctl::controls::ButtonTone::Default);
        }
    }

    /// 무장 중인가(시험).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn armed(&self) -> bool {
        self.armed.is_some()
    }

    /// 무장 카운트다운(시각은 호스트가 준다). 라벨이 바뀌거나 원복되면 `true`.
    fn arm_tick(&mut self, now_ms: u64) -> bool {
        let Some(started) = self.armed.as_mut() else {
            return false;
        };
        let started = *started.get_or_insert(now_ms);
        let Some((idx, arm)) = self.arm_button() else {
            return false;
        };
        let elapsed = now_ms.saturating_sub(started);
        if elapsed >= arm.ms {
            self.disarm();
            return true;
        }
        let secs = (arm.ms - elapsed).div_ceil(1000);
        if let Some(b) = self.buttons.get_mut(idx) {
            b.set_label(format!("{} ({secs})", arm.label));
        }
        true
    }

    /// 버튼 `id`의 결정: 잠김 = 없음 · 무장이 필요한데 아직이면 무장만(없음) · 그 밖 = 확정.
    fn decide(&mut self, id: i32) -> Option<DlgAction> {
        if self.locked(id) {
            return None;
        }
        if self.arm_needed(id) && self.armed.is_none() {
            self.arm();
            self.redraw();
            return None;
        }
        let text = self.input_text();
        Some(DlgAction::Done { id, text })
    }

    pub(crate) fn animating(&self) -> bool {
        self.window.is_some()
            && (self.buttons.iter().any(|b| b.is_animating())
                || self.wait.as_ref().is_some_and(|w| !w.done)
                || self.armed.is_some()
                || self.input.as_ref().is_some_and(|t| t.is_animating()))
    }

    /// 사양으로 컨트롤 구성(창 유무 무관 — 시험은 창 없이 `pick`).
    pub(crate) fn set_spec(&mut self, spec: DlgSpec) {
        self.buttons = spec
            .buttons
            .iter()
            .map(|(_, l)| Button::new(l.clone()))
            .collect();
        self.input = spec.input.as_ref().map(|i| {
            let mut tb = TextBox::new(i.placeholder.as_str()).with_text(&i.initial);
            tb.set_masked(i.masked);
            tb
        });
        self.done = None;
        self.armed = None;
        self.wait = spec.delay.and_then(|(id, ms)| {
            let idx = spec.buttons.iter().position(|(i, _)| *i == id)?;
            let label = spec.buttons[idx].1.clone();
            if let Some(b) = self.buttons.get_mut(idx) {
                b.set_enabled(false);
                b.set_label(format!("{label} ({})", ms.div_ceil(1000)));
            }
            Some(Wait {
                idx,
                id,
                label,
                total_ms: ms,
                started: None,
                done: false,
            })
        });
        self.spec = Some(spec);
    }

    /// 현재 입력란 텍스트(없으면 None).
    pub(crate) fn input_text(&self) -> Option<String> {
        self.input.as_ref().map(|t| t.text())
    }

    /// 입력란 텍스트 설정(기동 명령 `dlg.type`).
    pub(crate) fn set_input_text(&mut self, text: &str) {
        if let Some(tb) = &mut self.input {
            tb.set_text(text);
            self.redraw();
        }
    }

    /// 버튼 id로 결정(기동 명령 · 시험) — 입력란 텍스트 동봉.
    pub(crate) fn pick(&mut self, id: i32) -> Option<DlgAction> {
        self.spec.as_ref()?;
        self.decide(id)
    }

    pub(crate) fn open(
        &mut self,
        el: &ActiveEventLoop,
        spec: DlgSpec,
        theme: Option<winit::window::Theme>,
        over: Option<(i32, i32, u32, u32)>,
        owner: Option<&Window>,
    ) {
        self.set_spec(spec);
        self.open_kept(el, theme, over, owner);
    }

    /// 이미 실린 사양(창 없이 `set_spec`/`pick`으로 상태만 있던 것 · 기동 명령 뒤)으로 창을 만든다.
    pub(crate) fn open_kept(
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
        let Some(spec) = self.spec.as_ref() else {
            return;
        };
        // 높이 = 여백 + 본문 줄(대략 60자/줄) + 입력란 + 버튼.
        let est_lines: usize = if spec.text.is_empty() {
            0
        } else {
            spec.text
                .split('\n')
                .map(|l| 1 + l.chars().count() / 60)
                .sum()
        };
        let lh = (est_lines as f32) * 20.0;
        let input_h = if spec.input.is_some() {
            INPUT_H + 28.0
        } else {
            0.0
        };
        // 본문이 없으면(입력만) 위 여백 한 단 줄임.
        let pads = if spec.text.is_empty() { 2.0 } else { 3.0 };
        let (lw, lhgt) = (WIDTH, PAD * pads + lh + input_h + BTN_H + 8.0);
        let mut attrs = Window::default_attributes()
            .with_title(spec.title.clone())
            .with_theme(theme)
            .with_resizable(false)
            .with_inner_size(winit::dpi::LogicalSize::new(lw, lhgt));
        if let Some((x, y, w, h)) = over {
            let cx = x + (w as i32 - lw as i32) / 2;
            let cy = y + (h as i32 / 3).max(0);
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
        if let Some(tb) = &mut self.input {
            tb.set_focused(true);
        }
        self.redraw();
    }

    pub(crate) fn close(&mut self) {
        self.surface = None;
        self.window = None;
        self.spec = None;
        self.buttons.clear();
        self.input = None;
        self.wait = None;
        self.armed = None;
    }

    fn s(&self, v: f32) -> i32 {
        (v * self.scale).round() as i32
    }

    fn layout(&mut self) {
        let Some(win) = &self.window else { return };
        let size = win.inner_size();
        let (w, h) = (size.width as i32, size.height as i32);
        let s = self.scale;
        let pad = self.s(PAD);
        let mut inv = Invalidations::default();
        let bh = self.s(BTN_H);
        let bw = self.s(BTN_W);
        let gap = self.s(8.0);
        let by = h - pad - bh;
        // 버튼은 오른쪽 정렬(기본 버튼이 왼쪽에 오도록 사양 순서 유지).
        let total = self.buttons.len() as i32 * bw + (self.buttons.len() as i32 - 1).max(0) * gap;
        let mut x = w - pad - total;
        for b in &mut self.buttons {
            b.set_scale(s);
            b.set_bounds(Rect::new(x, by, bw, bh), &mut inv);
            x += bw + gap;
        }
        let ih = self.s(INPUT_H);
        if let Some(tb) = &mut self.input {
            tb.set_scale(s);
            tb.set_bounds(Rect::new(pad, by - gap - ih, w - pad * 2, ih), &mut inv);
        }
    }

    pub(crate) fn handle(&mut self, ev: &WindowEvent) -> DlgAction {
        let (cancel_id, default_id) = match &self.spec {
            Some(s) => (s.cancel, s.default),
            None => (0, 1),
        };
        match ev {
            WindowEvent::CloseRequested => {
                return DlgAction::Done {
                    id: cancel_id,
                    text: None,
                };
            }
            WindowEvent::Resized(_) => {
                self.layout();
                self.redraw();
                return DlgAction::None;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.layout();
                self.redraw();
                return DlgAction::None;
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.primary = if cfg!(target_os = "macos") {
                    m.state().super_key()
                } else {
                    m.state().control_key()
                };
                return DlgAction::None;
            }
            WindowEvent::Ime(ime) => {
                let mut inv = Invalidations::default();
                if let Some(tb) = &mut self.input {
                    match ime {
                        winit::event::Ime::Preedit(t, _) => tb.set_preedit(t, &mut inv),
                        winit::event::Ime::Commit(t) => {
                            tb.set_preedit("", &mut inv);
                            for c in t.chars().filter(|c| !c.is_control()) {
                                tb.on_event(&InputEvent::Char { c, now_ms: 0 }, &mut inv);
                            }
                        }
                        _ => {}
                    }
                    self.redraw();
                }
                return DlgAction::None;
            }
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                match kev.logical_key.as_ref() {
                    Key::Named(NamedKey::Escape) => {
                        return DlgAction::Done {
                            id: cancel_id,
                            text: None,
                        }
                    }
                    Key::Named(NamedKey::Enter) => {
                        if let Some(a) = self.decide(default_id) {
                            return a;
                        }
                    }
                    _ => {}
                }
                if let Some(tb) = &mut self.input {
                    let mut inv = Invalidations::default();
                    let ie = match kev.logical_key.as_ref() {
                        Key::Named(NamedKey::ArrowLeft) => Some(InputEvent::Key {
                            key: CtlKey::Left,
                            shift: self.shift,
                            primary: false,
                        }),
                        Key::Named(NamedKey::ArrowRight) => Some(InputEvent::Key {
                            key: CtlKey::Right,
                            shift: self.shift,
                            primary: false,
                        }),
                        Key::Named(NamedKey::Home) => Some(InputEvent::Key {
                            key: CtlKey::Home,
                            shift: self.shift,
                            primary: false,
                        }),
                        Key::Named(NamedKey::End) => Some(InputEvent::Key {
                            key: CtlKey::End,
                            shift: self.shift,
                            primary: false,
                        }),
                        Key::Named(NamedKey::Delete) => Some(InputEvent::Key {
                            key: CtlKey::Delete,
                            shift: self.shift,
                            primary: false,
                        }),
                        Key::Named(NamedKey::Backspace) => Some(InputEvent::Char {
                            c: '\u{8}',
                            now_ms: 0,
                        }),
                        Key::Named(NamedKey::Space) => Some(InputEvent::Char { c: ' ', now_ms: 0 }),
                        Key::Character(t) if self.primary => {
                            match t.to_ascii_lowercase().as_str() {
                                "a" => Some(InputEvent::SelectAll),
                                "v" => {
                                    if let Some(p) = crate::clipboard::read_text() {
                                        for c in p.chars().filter(|c| !c.is_control()) {
                                            tb.on_event(
                                                &InputEvent::Char { c, now_ms: 0 },
                                                &mut inv,
                                            );
                                        }
                                    }
                                    None
                                }
                                _ => None,
                            }
                        }
                        Key::Character(t) => t
                            .chars()
                            .next()
                            .filter(|c| !c.is_control())
                            .map(|c| InputEvent::Char { c, now_ms: 0 }),
                        _ => None,
                    };
                    if let Some(ie) = ie {
                        tb.on_event(&ie, &mut inv);
                    }
                    self.redraw();
                }
                return DlgAction::None;
            }
            WindowEvent::RedrawRequested => return DlgAction::Paint,
            _ => {}
        }
        if let WindowEvent::CursorMoved { position, .. } = ev {
            self.cursor = (position.x as i32, position.y as i32);
        }
        let (x, y) = self.cursor;
        let ie = match ev {
            WindowEvent::CursorMoved { .. } => InputEvent::MouseMove { x, y },
            WindowEvent::MouseInput { state, button, .. } => match (state, button) {
                (ElementState::Pressed, MouseButton::Left) => InputEvent::MouseDown {
                    x,
                    y,
                    shift: self.shift,
                    primary: false,
                },
                (ElementState::Released, MouseButton::Left) => InputEvent::MouseUp { x, y },
                _ => return DlgAction::None,
            },
            _ => return DlgAction::None,
        };
        let mut inv = Invalidations::default();
        for b in &mut self.buttons {
            b.on_event(&ie, &mut inv);
        }
        if let Some(tb) = &mut self.input {
            tb.on_event(&ie, &mut inv);
            if let InputEvent::MouseDown { x, y, .. } = ie {
                tb.set_focused(tb.bounds().contains(Point { x, y }));
            }
        }
        let ids: Vec<i32> = self
            .spec
            .as_ref()
            .map(|s| s.buttons.iter().map(|(id, _)| *id).collect())
            .unwrap_or_default();
        let mut clicked: Option<i32> = None;
        for (i, b) in self.buttons.iter_mut().enumerate() {
            if b.take_clicked() && clicked.is_none() {
                clicked = Some(ids.get(i).copied().unwrap_or(cancel_id));
            }
        }
        if let Some(id) = clicked {
            if let Some(a) = self.decide(id) {
                return a;
            }
        }
        self.redraw();
        DlgAction::None
    }

    /// 본문 줄 바꿈(단어 경계 · 창 폭) — 그릴 때 측정.
    fn wrap(dc: &mut dyn DrawCtx, text: &str, max_w: i32) -> Vec<String> {
        let mut out = Vec::new();
        for para in text.split('\n') {
            let mut line = String::new();
            for word in para.split(' ') {
                let cand = if line.is_empty() {
                    word.to_string()
                } else {
                    format!("{line} {word}")
                };
                if !line.is_empty() && dc.text_width(&cand) > max_w {
                    out.push(std::mem::take(&mut line));
                    line = word.to_string();
                } else {
                    line = cand;
                }
            }
            out.push(line);
        }
        out
    }

    pub(crate) fn paint(&mut self, ui: &Font, th: &Theme, font_px: f32) {
        let Some(spec) = self.spec.clone() else {
            return;
        };
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
            let lines = Self::wrap(&mut dc, &spec.text, wi - pad * 2);
            let mut y = pad;
            for l in &lines {
                dc.text(pad, y, Rect::new(0, 0, wi, hi), l, th.text);
                y += th_txt + 4;
            }
            if let (Some(tb), Some(inp)) = (&mut self.input, &spec.input) {
                let r = tb.bounds();
                dc.text(
                    pad,
                    r.y - th_txt - 4,
                    Rect::new(0, 0, wi, hi),
                    &inp.label,
                    th.text_dim,
                );
                tb.paint(&mut dc, th);
            }
            for b in &mut self.buttons {
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
    fn spec_dump_and_headless_pick() {
        ndir_i18n::activate(ndir_i18n::load("en", std::path::Path::new("nowhere")));
        let mut w = DlgWin::new();
        assert!(w.pick(1).is_none(), "사양 없음 = None");
        w.set_spec(DlgSpec::confirm("T".into(), "a\nb".into(), "OK".into()));
        assert_eq!(w.spec().unwrap().dump(), "T | a / b | 1:OK 0:Cancel | -");
        assert_eq!(w.pick(1), Some(DlgAction::Done { id: 1, text: None }));
        w.set_spec(DlgSpec {
            title: "P".into(),
            text: "pw".into(),
            buttons: vec![(1, "OK".into()), (0, "Cancel".into())],
            default: 1,
            cancel: 0,
            input: Some(DlgInput {
                label: "Password".into(),
                masked: true,
                initial: String::new(),
                placeholder: String::new(),
            }),
            delay: None,
            arm: None,
        });
        assert!(w.spec().unwrap().dump().ends_with("input(masked)"));
        w.set_input_text("s3cret");
        assert_eq!(
            w.pick(1),
            Some(DlgAction::Done {
                id: 1,
                text: Some("s3cret".into())
            })
        );
        // 무장 버튼(10-18): 이름이 `names`에 있으면 첫 pick = 무장(라벨 `덮어쓰기 (3)` · 확정 없음) · 둘째 pick = 확정 · 시간이 지나면 원복 ·
        // 다른 이름은 바로 확정 · 덤프 `arm`.
        w.set_spec(DlgSpec {
            title: "S".into(),
            text: String::new(),
            buttons: vec![(1, "OK".into()), (0, "Cancel".into())],
            default: 1,
            cancel: 0,
            input: Some(DlgInput {
                label: "Name".into(),
                masked: false,
                initial: "dup".into(),
                placeholder: String::new(),
            }),
            delay: None,
            arm: Some(DlgArm {
                id: 1,
                label: "Overwrite".into(),
                ms: 3000,
                names: vec!["dup".into()],
                empty_as: "Default".into(),
            }),
        });
        assert!(
            w.spec().unwrap().dump().ends_with("| input | arm 1:3000"),
            "{}",
            w.spec().unwrap().dump()
        );
        assert!(w.pick(1).is_none() && w.armed(), "첫 누름 = 무장");
        assert_eq!(w.buttons[0].label(), "Overwrite (3)");
        assert!(w.tick(5_000) && w.tick(6_200));
        assert_eq!(w.buttons[0].label(), "Overwrite (2)");
        assert_eq!(
            w.pick(1),
            Some(DlgAction::Done {
                id: 1,
                text: Some("dup".into())
            }),
            "둘째 누름 = 확정"
        );
        w.set_spec(DlgSpec::confirm("T".into(), "a".into(), "OK".into()));
        w.set_spec(DlgSpec {
            title: "S".into(),
            text: String::new(),
            buttons: vec![(1, "OK".into()), (0, "Cancel".into())],
            default: 1,
            cancel: 0,
            input: Some(DlgInput {
                label: "Name".into(),
                masked: false,
                initial: String::new(),
                placeholder: String::new(),
            }),
            delay: None,
            arm: Some(DlgArm {
                id: 1,
                label: "Overwrite".into(),
                ms: 3000,
                names: vec!["Default".into()],
                empty_as: "Default".into(),
            }),
        });
        assert!(w.pick(1).is_none(), "빈 이름 = 기본 이름이 겹침 → 무장");
        assert!(
            w.tick(10_000) && w.tick(13_000) && !w.armed(),
            "시간 지나면 원복"
        );
        assert_eq!(w.buttons[0].label(), "OK");
        w.set_input_text("other");
        assert!(
            matches!(w.pick(1), Some(DlgAction::Done { id: 1, .. })),
            "다른 이름 = 바로 확정"
        );
        // 잠금 버튼(10-18): 틱 전·중에는 pick 거부 + 라벨 `(N)` · 시간이 지나면 풀리고 라벨 복원 · 덤프에 `wait`.
        let mut spec = DlgSpec::confirm("W".into(), "sure?".into(), "Remove".into());
        spec.delay = Some((1, 3000));
        w.set_spec(spec);
        assert!(
            w.spec().unwrap().dump().ends_with("| - | wait 1:3000"),
            "{}",
            w.spec().unwrap().dump()
        );
        assert!(w.pick(1).is_none(), "잠김");
        assert_eq!(
            w.pick(0),
            Some(DlgAction::Done { id: 0, text: None }),
            "취소는 바로"
        );
        assert!(w.tick(10_000));
        assert_eq!(w.buttons[0].label(), "Remove (3)");
        assert!(w.tick(11_200));
        assert_eq!(w.buttons[0].label(), "Remove (2)");
        assert!(w.pick(1).is_none());
        assert!(w.tick(13_000));
        assert_eq!(w.buttons[0].label(), "Remove");
        assert!(!w.tick(14_000), "풀린 뒤에는 조용");
        assert_eq!(w.pick(1), Some(DlgAction::Done { id: 1, text: None }));
    }
}
