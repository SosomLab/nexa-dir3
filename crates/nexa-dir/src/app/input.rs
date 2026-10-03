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
        } else if self.launcherbar.bounds().h > 0 && self.launcherbar.bounds().contains(p) {
            Some(Area::Launcher)
        } else if self.dual && self.splitter.rect().contains(p) {
            Some(Area::Split)
        } else if self.panels[0].bounds().contains(p) {
            Some(Area::Panel(0))
        } else if self.dual && self.panels[1].bounds().contains(p) {
            Some(Area::Panel(1))
        } else if self.statusbar.bounds().contains(p) {
            Some(Area::Status)
        } else if self.docks[0].bounds().contains(p) {
            Some(Area::Dock(0))
        } else if self.docks[1].bounds().contains(p) {
            Some(Area::Dock(1))
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
            Area::Dock(i) => self.docks[i].on_event(ev, inv),
            Area::Launcher => self.launcherbar.on_event(ev, inv),
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
        // 대화상자가 열려 있으면 모달(T-29): 메인 창 입력은 무시하고 대화상자로 포커스.
        if self.dlg.is_open() {
            if matches!(ev, InputEvent::MouseDown { .. }) {
                self.dlg.focus();
            }
            return;
        }
        // 열린 탭 메뉴 = 모달(안 = 고르기 · Esc/바깥 클릭 = 닫기 · 바깥 클릭은 아래로 흘린다 — 팝업 UX 규칙).
        if self.tab_menu.is_open() {
            let outside = self.tab_menu.is_outside_click(&ev);
            let _ = self.tab_menu.on_event(&ev);
            inv.push(Rect::new(0, 0, self.viewport.0, self.viewport.1));
            if !outside {
                return;
            }
        }
        // 셸 항목을 기다리는 우클릭 메뉴: 다른 클릭·키 입력이 오면 그 메뉴는 열지 않는다(늦게 떠서 엉뚱한 곳을 덮지 않게).
        if self.ctx_wait.is_some()
            && matches!(
                ev,
                InputEvent::MouseDown { .. }
                    | InputEvent::RightDown { .. }
                    | InputEvent::MiddleDown { .. }
                    | InputEvent::DoubleClick { .. }
                    | InputEvent::Key { .. }
                    | InputEvent::Char { .. }
            )
        {
            let esc = matches!(
                ev,
                InputEvent::Key {
                    key: nexa_ctl::Key::Escape,
                    ..
                }
            );
            self.ctx_cancel_wait();
            if esc {
                return;
            }
        }
        // 툴바 그룹을 끄는 중 Esc = 취소(원래 순서·행으로 · nexa-ctl `ToolDock::cancel_drag`).
        if self.toolbar.is_dragging() {
            if let InputEvent::Key {
                key: nexa_ctl::Key::Escape,
                ..
            } = ev
            {
                self.toolbar.cancel_drag(inv);
                self.pressed = None;
                return;
            }
        }
        match ev {
            InputEvent::MouseMove { x, y } => {
                self.cursor = (x, y);
                if let Some(i) = self.term_focus {
                    if self.terms[i].mouse_move(x, y) {
                        inv.push(self.docks[i].bounds());
                        return;
                    }
                }
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
                if self.launcherbar.bounds().h > 0 {
                    self.launcherbar.on_event(&ev, inv);
                }
                if self.dual {
                    self.split_event(&ev, inv);
                }
                self.panels[0].on_event(&ev, inv);
                if self.dual {
                    self.panels[1].on_event(&ev, inv);
                }
                for d in &mut self.docks {
                    if d.bounds().h > 0 {
                        d.on_event(&ev, inv);
                    }
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
                // 터미널 격자 클릭 = 터미널 포커스 + 선택 시작(dir2 QA 07-14) — 도크 위젯으로는 보내지 않는다.
                if let InputEvent::MouseDown { shift, .. } = ev {
                    if let Some(i) = self.term_hit_at(x, y) {
                        if self.terms[i].started() {
                            self.set_term_focus(Some(i), inv);
                            // TUI 마우스 모드(dir2 X-5): 좌표를 셸로 보내고 로컬 선택은 억제(Shift = 로컬 선택 강제).
                            if !shift {
                                if let Some(rep) = self.terms[i].mouse_report(x, y, 0, true) {
                                    self.terms[i].write(&rep);
                                    self.term_mouse_down = Some(i);
                                    return;
                                }
                            }
                            self.terms[i].mouse_down(x, y, shift);
                            inv.push(self.docks[i].bounds());
                            return;
                        }
                    }
                }
                let Some(area) = self.area_at(p) else { return };
                if matches!(area, Area::Panel(_)) && self.term_focus.is_some() {
                    self.set_term_focus(None, inv); // 목록 클릭 = 터미널 포커스 해제
                }
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
            InputEvent::MouseUp { x, y } => {
                if let Some(i) = self.term_mouse_down.take() {
                    if let Some(rep) = self.terms[i].mouse_report(x, y, 0, false) {
                        self.terms[i].write(&rep);
                    }
                }
                if let Some(i) = self.term_focus {
                    self.terms[i].mouse_up();
                }
                if let Some(a) = self.pressed.take() {
                    self.send(a, &ev, inv);
                } else if self.menubar.is_open() {
                    self.menubar.on_event(&ev, inv);
                }
            }
            InputEvent::Wheel { .. } | InputEvent::HWheel { .. } => {
                let (x, y) = self.cursor;
                if let (InputEvent::Wheel { delta }, Some(i)) = (ev, self.term_hit_at(x, y)) {
                    // 터미널 위 휠(dir2 eb29089 · 7d8b1e9): TUI 마우스 모드 = 노치당 휠 보고 1회(64/65 · 트랙패드의 잔 사건마다
                    // 보내면 너무 빠르다) · 아니면 스크롤백 — 시스템 줄 수/노치 · 작은 delta는 누적(종전 `delta*3/120`은 0으로 버려짐).
                    if self.terms[i].mouse_report(x, y, 64, true).is_some() {
                        let n = self.terms[i].tui_wheel.add(delta, 1);
                        let btn = if n > 0 { 64 } else { 65 };
                        for _ in 0..n.abs() {
                            if let Some(rep) = self.terms[i].mouse_report(x, y, btn, true) {
                                self.terms[i].write(&rep);
                            }
                        }
                        return;
                    }
                    let lines = self.terms[i].wheel.add(delta, nexa_ctl::wheel_lines());
                    if lines != 0 && self.terms[i].scroll_view(lines) {
                        inv.push(self.docks[i].bounds());
                    }
                    return;
                }
                if let (InputEvent::HWheel { delta }, Some(i)) = (ev, self.term_hit_at(x, y)) {
                    // 가로 휠/Shift+휠 = 고정 열 모드 가로 스크롤(4열/노치 · dir2 X-3 · 분수 누적).
                    let cols = self.terms[i].hwheel.add(delta, 4);
                    if cols != 0 && self.terms[i].scroll_x(cols) {
                        inv.push(self.docks[i].bounds());
                    }
                    return;
                }
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
                if self.term_key(&ev, inv) {
                    return; // 터미널 포커스 = 키를 셸로(목록 단축키 차단 · dir2 KeyRoute::Term)
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
        self.toolbar_actions();
        if let Some(id) = self.launcherbar.take_clicked() {
            self.command(&id);
        }
        for i in 0..2 {
            self.panels[i].drain_actions(inv);
            if let Some(path) = self.panels[i].take_open() {
                self.open_external(&path);
            }
            if let Some((row, name)) = self.panels[i].take_rename() {
                self.apply_rename(i, row, &name);
            }
            if self.panels[i].take_path_menu() {
                if i != self.active {
                    self.set_active(i);
                }
                self.open_path_edit_menu(i);
            }
            if let Some(on_row) = self.panels[i].take_ctx() {
                if i != self.active {
                    self.set_active(i);
                }
                if on_row {
                    self.open_row_menu(i);
                } else {
                    self.open_bg_menu(i);
                }
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
        for i in 0..2 {
            if self.docks[i].take_goto() {
                self.term_goto(i, inv); // → = 현재 폴더로 cd(살아 있으면) · 아니면 재시작 · 포커스
            }
            if self.docks[i].take_popout() {
                let src = self.dock_source(i);
                self.open_preview_window(src); // ↗ = 독립 미리보기 창(T-62 B)
            }
        }
        if let Some(id) = self.tab_menu.take_picked() {
            if self.tab_menu_at.is_some() {
                self.tab_menu_action(&id);
            } else {
                self.ctx_menu_action(&id);
            }
        }
        if let Some(id) = self.toasts.take_action() {
            self.command(&id);
        }
        // 선택 수·탭이 바뀌면 상태줄.
        self.update_status();
    }

    /// 파일 활성화 — 연결 프로그램으로 열기(platform 층 T-6x). 지금은 토스트.
    pub(crate) fn open_external(&mut self, path: &std::path::Path) {
        // 폴더를 가리키는 바로 가기(.lnk) = 앱 안에서 그 폴더로 이동(탐색기와 같음 · GAP-007) — 파일 대상·해석 실패는 OS 열기로.
        if crate::filelist::is_lnk(path) {
            if let Some(dir) = self
                .platform
                .opener
                .link_target(path)
                .filter(|t| t.is_dir())
            {
                let a = self.active;
                let mut inv = Invalidations::default();
                if let Some(why) = self.panels[a].navigate_to(dir, &mut inv) {
                    self.toasts
                        .push(toast::ToastKind::Warn, tr("cmd.activate"), why);
                }
                self.update_status();
                self.redraw();
                return;
            }
        }
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
