//! App — 입력 변환·라우팅(nexa-sql `app/input.rs` 축약 · docs/port/40 §1-5).
//!
//! 규칙: 포인터 = **눌린 곳**이 뗄 때까지 받는다(`pressed`) · 이동은 hover가 있는 컨트롤 전부에 · 키 = **활성 패널**(dir2 PANEL §2-4).
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
        } else if self.dual && self.splitter.rect().contains(p) {
            Some(Area::Split)
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
            Area::Panel(i) => self.panels[i].on_event(ev, inv),
            Area::Split => self.split_event(ev, inv),
            Area::Status => self.statusbar.on_event(ev, inv),
        }
    }

    /// 스플리터 사건 → 비율 갱신 · 드래그 끝 = 설정 저장.
    fn split_event(&mut self, ev: &InputEvent, inv: &mut Invalidations) {
        match self.splitter.on_event(ev) {
            SplitEvent::Hover => inv.push(self.splitter.rect()),
            SplitEvent::Start => inv.push(self.splitter.rect()),
            SplitEvent::Drag(v) => {
                self.split_drag(v);
                inv.push(Rect::new(0, 0, self.viewport.0, self.viewport.1));
            }
            SplitEvent::End => {
                let _ = self.settings.save();
                inv.push(self.splitter.rect());
            }
            SplitEvent::None => {}
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
        // 열린 탭 메뉴 = 모달(안 = 고르기 · Esc/바깥 클릭 = 닫기 · 바깥 클릭은 아래로 흘린다 — 팝업 UX 규칙).
        if self.tab_menu.is_open() {
            let outside = self.tab_menu.is_outside_click(&ev);
            let _ = self.tab_menu.on_event(&ev);
            inv.push(Rect::new(0, 0, self.viewport.0, self.viewport.1));
            if !outside {
                return;
            }
        }
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
                if self.dual {
                    self.split_event(&ev, inv);
                }
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
                    if i != self.active {
                        // 다른 패널 클릭 = 활성 전환(dir2 PANEL-001 "활성 패널에 키보드를 라우팅").
                        let other = 1 - i;
                        if self.panels[other].pathbar.is_editing() {
                            self.panels[other].pathbar.cancel_edit(inv);
                        }
                        self.set_active(i);
                        inv.push(self.panels[0].bounds());
                        inv.push(self.panels[1].bounds());
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
            InputEvent::Key { .. }
            | InputEvent::Char { .. }
            | InputEvent::SelectAll
            | InputEvent::Undo
            | InputEvent::Redo => {
                if self.menubar.is_open() {
                    self.menubar.on_event(&ev, inv);
                    return;
                }
                self.panels[self.active].key_event(&ev, inv);
            }
        }
    }

    /// 사건 뒤 컨트롤 보고 수거 — 메뉴 선택 · 툴바 클릭 · 패널 보고(탭·네비·경로·파일 열기) · 토스트 동작.
    fn after_event(&mut self, inv: &mut Invalidations) {
        if let Some(id) = self.menubar.take_picked() {
            self.command(&id);
        }
        if let Some(id) = self.toolbar.take_clicked() {
            self.command(&id);
        }
        for i in 0..2 {
            self.panels[i].drain_actions(inv);
            if let Some(path) = self.panels[i].take_open() {
                self.open_external(&path);
            }
            if let Some(t) = self.panels[i].take_tab_menu() {
                self.open_tab_menu(i, t);
                inv.push(Rect::new(0, 0, self.viewport.0, self.viewport.1));
            }
            // 열 폭 동기(`list.col_width_sync` · dir2 07-18): 사용자가 한쪽 열 폭을 바꾸면 반대 패널도.
            if self.panels[i].take_col_changed()
                && self.dual
                && self.settings.flag("list.col_width_sync")
            {
                self.sync_col_widths_from(i);
            }
        }
        if let Some(id) = self.tab_menu.take_picked() {
            self.tab_menu_action(&id);
        }
        if let Some(id) = self.toasts.take_action() {
            self.command(&id);
        }
        // 선택 수·탭이 바뀌면 상태줄.
        self.update_status();
    }

    /// 파일 활성화 — 연결 프로그램으로 열기(platform 층 T-6x). 지금은 토스트.
    pub(crate) fn open_external(&mut self, path: &std::path::Path) {
        if let Err(e) = self.platform.opener.open(path) {
            self.toasts
                .push(toast::ToastKind::Warn, tr("cmd.activate"), e.to_string());
        }
        self.redraw();
    }

    /// 탭 우클릭 메뉴(dir2 TAB-MENU 07-20 순서: 잠금 · 고정 · 복제 · 새 탭 · 닫기 + 패널 간 이동).
    pub(crate) fn open_tab_menu(&mut self, panel: usize, tab: usize) {
        let p = &self.panels[panel];
        let locked = p.tab_locked(tab);
        let pinned = p.tab_pinned(tab);
        let count = p.tab_count();
        let items = vec![
            CtxItem::item(
                "tab.lock",
                tr(if locked { "tab.unlock" } else { "tab.lock" }),
            ),
            CtxItem::item("tab.pin", tr(if pinned { "tab.unpin" } else { "tab.pin" })),
            CtxItem::item("tab.duplicate", tr("tab.duplicate")),
            CtxItem::item("tab.new", tr("tab.new")),
            CtxItem::maybe(
                "tab.move_other",
                tr("tab.moveOther"),
                self.dual && count > 1 && !locked,
            ),
            CtxItem::maybe("tab.close", tr("tab.close"), !locked && count > 1),
        ];
        let host = Rect::new(0, 0, self.viewport.0, self.viewport.1);
        let text_w = px(220.0, self.scale);
        self.tab_menu_at = Some((panel, tab));
        let (x, y) = self.cursor;
        self.tab_menu.open_at(x, y, items, host, text_w);
    }

    /// 탭 메뉴 선택 실행.
    pub(crate) fn tab_menu_action(&mut self, id: &str) {
        let Some((panel, tab)) = self.tab_menu_at.take() else {
            return;
        };
        let mut inv = Invalidations::default();
        match id {
            "tab.lock" => self.panels[panel].toggle_tab_lock(tab, &mut inv),
            "tab.pin" => self.panels[panel].toggle_tab_pin(tab, &mut inv),
            "tab.duplicate" => self.panels[panel].duplicate_tab(tab, &mut inv),
            "tab.new" => self.panels[panel].new_tab(&mut inv),
            "tab.close" => self.panels[panel].close_tab(tab, &mut inv),
            "tab.move_other" if self.dual => {
                if let Some(t) = self.panels[panel].detach_tab(tab, &mut inv) {
                    let other = 1 - panel;
                    self.panels[other].attach_tab(t, None, &mut inv);
                    self.set_active(other);
                }
            }
            _ => {}
        }
        self.update_status();
        self.redraw();
    }

    /// 열 폭 동기 — `from` 패널의 폭을 반대 패널에.
    pub(crate) fn sync_col_widths_from(&mut self, from: usize) {
        let widths = self.panels[from].col_widths_now();
        let mut inv = Invalidations::default();
        self.panels[1 - from].apply_col_widths(&widths, &mut inv);
        self.redraw();
    }

    /// 포인터가 창을 떠남/비활성 = hover 정리.
    pub(crate) fn pointer_gone(&mut self) {
        let mut inv = Invalidations::default();
        let away = InputEvent::MouseMove { x: -1, y: -1 };
        self.toolbar.on_event(&away, &mut inv);
        for p in &mut self.panels {
            p.on_event(&away, &mut inv);
        }
        if !inv.is_empty() {
            self.redraw();
        }
    }
}
