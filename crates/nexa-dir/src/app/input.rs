//! App — 입력 변환·라우팅(nexa-sql `app/input.rs` 축약 · docs/port/40 §1-5).
//!
//! 규칙: 포인터 = **눌린 곳**이 뗄 때까지 받는다(`pressed`) · 이동은 hover가 있는 컨트롤 전부에 · 키 = `focus`.
//! 열린 메뉴가 있으면 메뉴가 먼저(바깥 클릭 = 닫기). 사건 뒤에는 컨트롤의 "보고"(`take_*`)를 거둬 명령 한 길로.

use crate::*;

impl App {
    /// winit 창 사건 → 컨트롤 입력(장치 px · 수식키 반영). 더블클릭은 합성(`ui.dblclick_ms` · 같은 자리 ±4px).
    pub(crate) fn ctl_event(&mut self, event: &WindowEvent) -> Option<InputEvent> {
        let (x, y) = self.cursor;
        Some(match event {
            WindowEvent::CursorMoved { position, .. } => InputEvent::MouseMove {
                x: position.x as i32,
                y: position.y as i32,
            },
            WindowEvent::MouseInput { state, button, .. } => match (state, button) {
                (ElementState::Pressed, winit::event::MouseButton::Left) => {
                    let now = Instant::now();
                    let dbl_ms = self.settings.int("ui.dblclick_ms").clamp(100, 2000) as u64;
                    let double = self.last_click.is_some_and(|(at, lx, ly)| {
                        now.duration_since(at) <= Duration::from_millis(dbl_ms)
                            && (lx - x).abs() <= 4
                            && (ly - y).abs() <= 4
                    });
                    if double {
                        self.last_click = None;
                        InputEvent::DoubleClick {
                            x,
                            y,
                            shift: self.shift,
                            primary: self.primary,
                        }
                    } else {
                        self.last_click = Some((now, x, y));
                        InputEvent::MouseDown {
                            x,
                            y,
                            shift: self.shift,
                            primary: self.primary,
                        }
                    }
                }
                (ElementState::Released, winit::event::MouseButton::Left) => {
                    InputEvent::MouseUp { x, y }
                }
                (ElementState::Pressed, winit::event::MouseButton::Right) => {
                    InputEvent::RightDown { x, y }
                }
                (ElementState::Pressed, winit::event::MouseButton::Middle) => {
                    InputEvent::MiddleDown { x, y }
                }
                (ElementState::Pressed, winit::event::MouseButton::Back) => InputEvent::XButton {
                    x,
                    y,
                    forward: false,
                },
                (ElementState::Pressed, winit::event::MouseButton::Forward) => {
                    InputEvent::XButton {
                        x,
                        y,
                        forward: true,
                    }
                }
                _ => return None,
            },
            WindowEvent::MouseWheel { delta, .. } => crate::input::wheel_event(delta, self.shift),
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                use nexa_ctl::Key as CtlKey;
                let key = |k: CtlKey| InputEvent::Key {
                    key: k,
                    shift: self.shift,
                    primary: self.primary,
                };
                match kev.logical_key.as_ref() {
                    Key::Named(NamedKey::Enter) => key(CtlKey::Enter),
                    Key::Named(NamedKey::Escape) => key(CtlKey::Escape),
                    Key::Named(NamedKey::ArrowUp) => key(CtlKey::Up),
                    Key::Named(NamedKey::ArrowDown) => key(CtlKey::Down),
                    Key::Named(NamedKey::ArrowLeft) => key(CtlKey::Left),
                    Key::Named(NamedKey::ArrowRight) => key(CtlKey::Right),
                    Key::Named(NamedKey::Home) => key(CtlKey::Home),
                    Key::Named(NamedKey::End) => key(CtlKey::End),
                    Key::Named(NamedKey::PageUp) => key(CtlKey::PageUp),
                    Key::Named(NamedKey::PageDown) => key(CtlKey::PageDown),
                    Key::Named(NamedKey::Delete) => key(CtlKey::Delete),
                    Key::Named(NamedKey::Space) => key(CtlKey::Space),
                    Key::Named(NamedKey::Backspace) => InputEvent::Char {
                        c: '\u{8}',
                        now_ms: 0,
                    },
                    Key::Character(t) => {
                        if self.primary || self.alt {
                            return None;
                        }
                        let c = t.chars().next()?;
                        if c.is_control() {
                            return None;
                        }
                        InputEvent::Char {
                            c,
                            now_ms: self.started.elapsed().as_millis() as u64,
                        }
                    }
                    _ => return None,
                }
            }
            _ => return None,
        })
    }

    /// 자리 → 영역.
    fn area_at(&self, p: Point) -> Option<Area> {
        if self.menubar.bounds().contains(p) {
            Some(Area::Menu)
        } else if self.toolbar.bounds().contains(p) {
            Some(Area::Tool)
        } else if self.tabs.bounds().contains(p) {
            Some(Area::Tabs)
        } else if self.pathbar.bounds().contains(p) {
            Some(Area::Path)
        } else if self.panels[0].bounds().contains(p) {
            Some(Area::Panel(0))
        } else if self.dual && self.panels[1].bounds().contains(p) {
            Some(Area::Panel(1))
        } else if self.statusbar.bounds().contains(p) {
            Some(Area::Status)
        } else {
            None
        }
    }

    fn send(&mut self, area: Area, ev: &InputEvent, inv: &mut Invalidations) {
        match area {
            Area::Menu => self.menubar.on_event(ev, inv),
            Area::Tool => self.toolbar.on_event(ev, inv),
            Area::Tabs => self.tabs.on_event(ev, inv),
            Area::Path => self.pathbar.on_event(ev, inv),
            Area::Panel(i) => self.panels[i].on_event(ev, inv),
            Area::Status => self.statusbar.on_event(ev, inv),
        }
    }

    pub(crate) fn route(&mut self, ev: InputEvent) {
        let mut inv = Invalidations::default();
        self.route_inner(ev, &mut inv);
        self.after_event(&mut inv);
        if !inv.is_empty() || inv.tick_requested() {
            self.redraw();
        }
    }

    fn route_inner(&mut self, ev: InputEvent, inv: &mut Invalidations) {
        match ev {
            InputEvent::MouseMove { x, y } => {
                self.cursor = (x, y);
                if let Some(a) = self.pressed {
                    self.send(a, &ev, inv);
                    return;
                }
                if self.menubar.is_open() {
                    self.menubar.on_event(&ev, inv);
                    return;
                }
                // hover는 전부(들어오고 나갈 때 스스로 무효화한다).
                self.menubar.on_event(&ev, inv);
                self.toolbar.on_event(&ev, inv);
                self.tabs.on_event(&ev, inv);
                self.pathbar.on_event(&ev, inv);
                self.panels[0].on_event(&ev, inv);
                if self.dual {
                    self.panels[1].on_event(&ev, inv);
                }
            }
            InputEvent::MouseDown { x, y, .. }
            | InputEvent::RightDown { x, y }
            | InputEvent::MiddleDown { x, y }
            | InputEvent::DoubleClick { x, y, .. } => {
                let p = Point { x, y };
                if self.toasts.click(p) {
                    inv.push(Rect::new(0, 0, 1, 1));
                    return;
                }
                if self.menubar.is_open() {
                    // 열린 메뉴 = 메뉴가 먼저(안 = 고르기 · 밖 = 닫기 → 그 클릭은 아래로 흘리지 않는다 · 표준).
                    self.menubar.on_event(&ev, inv);
                    return;
                }
                let Some(area) = self.area_at(p) else { return };
                if let Area::Panel(i) = area {
                    if self.pathbar.is_editing() {
                        self.pathbar.cancel_edit(inv);
                    }
                    self.set_focus(Focus::Panel(i));
                    if i != self.active || self.focus != Focus::Panel(i) {
                        inv.push(self.pathbar.bounds());
                    }
                    let path = self.panels[i]
                        .source()
                        .path()
                        .to_string_lossy()
                        .into_owned();
                    self.pathbar.set_path(path, inv);
                }
                if let InputEvent::DoubleClick { .. } = ev {
                    if let Area::Panel(i) = area {
                        if let Some(row) = self.panels[i].row_at(x, y) {
                            if self.panels[i].source().row_is_dir(row) {
                                if let Some(p) = self.panels[i].source().row_path(row) {
                                    self.navigate(i, &p);
                                    return;
                                }
                            }
                        }
                    }
                }
                if matches!(ev, InputEvent::MouseDown { .. }) {
                    self.pressed = Some(area);
                }
                self.send(area, &ev, inv);
            }
            InputEvent::MouseUp { .. } => {
                if let Some(a) = self.pressed.take() {
                    self.send(a, &ev, inv);
                } else if self.menubar.is_open() {
                    self.menubar.on_event(&ev, inv);
                }
            }
            InputEvent::Wheel { .. } | InputEvent::HWheel { .. } => {
                let (x, y) = self.cursor;
                if let Some(a) = self.area_at(Point { x, y }) {
                    self.send(a, &ev, inv);
                }
            }
            InputEvent::XButton { forward, .. } => {
                self.command(if forward { "nav.forward" } else { "nav.back" });
            }
            InputEvent::Key { .. } | InputEvent::Char { .. } => {
                if self.menubar.is_open() {
                    self.menubar.on_event(&ev, inv);
                    return;
                }
                match self.focus {
                    Focus::PathBar => {
                        match ev {
                            InputEvent::Key {
                                key: nexa_ctl::Key::Enter,
                                ..
                            } => self.pathbar.submit_edit(inv),
                            InputEvent::Key {
                                key: nexa_ctl::Key::Escape,
                                ..
                            } => {
                                self.pathbar.cancel_edit(inv);
                                self.set_focus(Focus::Panel(self.active));
                            }
                            _ => self.pathbar.on_event(&ev, inv),
                        }
                        if !self.pathbar.is_editing() && self.focus == Focus::PathBar {
                            self.set_focus(Focus::Panel(self.active));
                        }
                    }
                    Focus::Panel(i) => {
                        // Enter = 활성(폴더 = 진입 · 파일 = 외부 열기 T-6x).
                        if let InputEvent::Key {
                            key: nexa_ctl::Key::Enter,
                            ..
                        } = ev
                        {
                            if let Some(row) = self.panels[i].caret() {
                                if self.panels[i].source().row_is_dir(row) {
                                    if let Some(p) = self.panels[i].source().row_path(row) {
                                        self.navigate(i, &p);
                                        return;
                                    }
                                }
                            }
                        }
                        self.panels[i].on_event(&ev, inv);
                    }
                }
            }
            _ => {}
        }
    }

    /// 사건 뒤 컨트롤 보고 수거 — 메뉴 선택 · 툴바 클릭 · 탭 동작 · 경로바 이동 · 토스트 동작.
    fn after_event(&mut self, inv: &mut Invalidations) {
        if let Some(id) = self.menubar.take_picked() {
            self.command(&id);
        }
        if let Some(id) = self.toolbar.take_clicked() {
            self.command(&id);
        }
        if let Some(act) = self.tabs.take_action() {
            match act {
                TabAction::Switch(i) => {
                    self.tabs.set_active(i, inv);
                    self.update_status();
                }
                TabAction::New => self.command("file.new_tab"),
                TabAction::Close(_) => self.command("file.close_tab"),
                _ => {}
            }
        }
        if let Some(path) = self.pathbar.take_navigation() {
            self.navigate(self.active, std::path::Path::new(&path));
            self.set_focus(Focus::Panel(self.active));
        }
        if self.pathbar.is_editing() && self.focus != Focus::PathBar {
            self.focus = Focus::PathBar;
        }
        if let Some(id) = self.toasts.take_action() {
            self.command(&id);
        }
        // 선택 수가 바뀌면 상태줄.
        self.update_status();
    }

    /// 포인터가 창을 떠남/비활성 = hover 정리.
    pub(crate) fn pointer_gone(&mut self) {
        let mut inv = Invalidations::default();
        let away = InputEvent::MouseMove { x: -1, y: -1 };
        self.toolbar.on_event(&away, &mut inv);
        self.tabs.on_event(&away, &mut inv);
        self.pathbar.on_event(&away, &mut inv);
        for p in &mut self.panels {
            p.on_event(&away, &mut inv);
        }
        if !inv.is_empty() {
            self.redraw();
        }
    }
}
