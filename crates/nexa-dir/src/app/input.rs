//! App — 입력 변환·라우팅(nexa-sql `app/input.rs` 축약 · docs/port/40 §1-5).
//!
//! 규칙: 포인터 = **눌린 곳**이 뗄 때까지 받는다(`pressed`) · 이동은 hover가 있는 컨트롤 전부에 · 키 = **활성 패널**(dir2 PANEL §2-4).
//! 열린 메뉴가 있으면 메뉴가 먼저(바깥 클릭 = 닫기). 사건 뒤에는 컨트롤의 "보고"(`take_*`)를 거둬 명령 한 길로.

use crate::*;

/// IME 조합 창을 둘 캐럿 자리(순수 · dir2 GAP-014 `edit_info` 계약): `(캐럿 앞 글, 편집 필드 rect, 안쪽 여백)` + 글 폭 재는 함수 →
/// `(x, y, 폭 1, 높이)` — 캐럿 = `rect.x + pad + 폭(캐럿 앞 글)` · 필드 오른쪽 끝을 넘지 않는다.
pub(crate) fn ime_caret(
    info: &(String, Rect, i32),
    measure: impl Fn(&str) -> i32,
) -> (i32, i32, i32, i32) {
    let (text, rect, pad) = info;
    let x = (rect.x + pad + measure(text))
        .min(rect.right() - 1)
        .max(rect.x);
    (x, rect.y, 1, rect.h.max(1))
}

/// 한글 모드면 자판 글자 → 두벌식 자모(순수): 자모가 없는 글자(숫자 · 기호)와 한글 모드가 아닐 때는 그대로.
pub(crate) fn hangul_key(c: char, hangul: bool) -> char {
    if hangul {
        nexa_ctl::hangul::jamo_from_qwerty(c, c.is_ascii_uppercase()).unwrap_or(c)
    } else {
        c
    }
}

/// 글자 키가 목록의 타입어헤드로 가는 상태인가(순수 · MC/DC): 글을 넣는 곳(대화상자 · 열린 메뉴 · 경로 편집 · 이름 바꾸기 ·
/// 터미널)이 하나도 없을 때만.
pub(crate) fn typeahead_target_of(
    dialog: bool,
    menu: bool,
    path_edit: bool,
    renaming: bool,
    terminal: bool,
) -> bool {
    !(dialog || menu || path_edit || renaming || terminal)
}

impl App {
    /// 지금 글자 키가 활성 패널 목록의 타입어헤드로 가는가([`typeahead_target_of`]).
    pub(crate) fn typeahead_target(&self) -> bool {
        let p = &self.panels[self.active];
        typeahead_target_of(
            self.dlg.is_open(),
            self.tab_menu.is_open() || self.menubar.is_open(),
            p.pathbar.is_editing(),
            p.rows().is_renaming(),
            self.term_focused().is_some(),
        )
    }

    /// 한/영 키(Windows): 목록 입력의 한글 모드를 뒤집고 상태줄에 알린다 — 목록이 글자를 받는 상태일 때만. 처리했으면 `true`.
    pub(crate) fn toggle_hangul_mode(&mut self) -> bool {
        if !self.typeahead_target() || !self.settings.flag("typeahead.enabled") {
            return false;
        }
        self.hangul_mode = !self.hangul_mode;
        let mut inv = Invalidations::default();
        self.statusbar.set_left(
            &tr(if self.hangul_mode {
                "status.hangulOn"
            } else {
                "status.hangulOff"
            }),
            &mut inv,
        );
        self.redraw();
        true
    }

    /// 지금 글자를 편집 중인 필드의 IME 조합 창 자리 — 경로 바 편집(dir2 A/win.rs:4792) 또는 목록의 인라인 이름 바꾸기 ·
    /// 편집 중이 아니면 `None`.
    pub(crate) fn ime_area(&self) -> Option<(i32, i32, i32, i32)> {
        let p = self.panels.get(self.active)?;
        let (info, key) = match p.pathbar.edit_info() {
            Some(i) => (i, "ui.font_size"),
            None => (p.rows().rename_edit_info()?, "list.font_size"),
        };
        let px = self.font_px(key) * self.scale;
        Some(ime_caret(&info, |t| {
            self.ui_font.measure(t, px).round() as i32
        }))
    }

    /// 조합 창 자리가 바뀌었으면 창에 알린다(그린 뒤마다 · 편집이 끝나면 다음 편집 때 다시 알린다).
    pub(crate) fn sync_ime_area(&mut self) {
        let area = self.ime_area();
        if area == self.ime_last {
            return;
        }
        self.ime_last = area;
        if let (Some(w), Some((x, y, cw, ch))) = (&self.window, area) {
            w.set_ime_cursor_area(
                winit::dpi::PhysicalPosition::new(x, y),
                winit::dpi::PhysicalSize::new(cw.max(1) as u32, ch.max(1) as u32),
            );
        }
    }

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
                        // 시각을 싣는다 — 0이면 타입어헤드의 유지 시간 기준이 0으로 되돌아가 다음 틱에 바로 지워진다.
                        now_ms: self.started.elapsed().as_millis() as u64,
                    },
                    Key::Character(t) => {
                        if self.primary || self.alt {
                            return None;
                        }
                        let c = t.chars().next()?;
                        if c.is_control() {
                            return None;
                        }
                        // Windows 목록 한글 모드: 메인 창은 IME가 없어 라틴 글자가 온다 → 두벌식 자모로(대문자 = Shift ·
                        // 숫자 · 기호는 그대로). 조합은 그리드의 타입어헤드가 한다(nexa-sql 탐색기와 같은 길).
                        let c = hangul_key(
                            c,
                            cfg!(windows) && self.hangul_mode && self.typeahead_target(),
                        );
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
    pub(crate) fn area_at(&self, p: Point) -> Option<Area> {
        if self.menubar.bounds().contains(p) {
            Some(Area::Menu)
        } else if self.toolbar.bounds().contains(p) {
            Some(Area::Tool)
        } else if self.launcherbar.bounds().h > 0 && self.launcherbar.bounds().contains(p) {
            Some(Area::Launcher)
        } else if let Some(k) = self.split_at(p) {
            Some(Area::Split(k))
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
            // 툴바 우클릭 = 순서 편집 · 설정 바로가기(dir2 CMD-097~099 `show_bar_popup` · T-133).
            Area::Tool if matches!(ev, InputEvent::RightDown { .. }) => self.open_toolbar_menu(),
            Area::Tool => self.toolbar.on_event(ev, inv),
            Area::Panel(i) => self.panels[i].on_event(ev, inv),
            Area::Split(k) => self.split_event(k, ev, inv),
            Area::Status => {
                // 상태줄 우클릭 = 툴바와 같은 메뉴(상태바 편집… · 설정…) — 바로 편집 창으로 가지 않는다(사용자 10-04).
                if matches!(ev, InputEvent::RightDown { .. }) {
                    self.open_statusbar_menu();
                    return;
                }
                self.statusbar.on_event(ev, inv);
                if let Some((id, right)) = self.statusbar.take_click() {
                    self.status_click(&id, right);
                }
            }
            // 도크 우클릭 = 글자 편집 메뉴(dir2 win.rs:7486-7491): 터미널 격자 = 복사 · 붙여넣기 · 모두 선택 /
            // 정보 · 미리보기 글 = 복사 · 모두 선택. 해당 없으면 아래 일반 처리로.
            Area::Dock(i)
                if matches!(ev, InputEvent::RightDown { .. })
                    && self.open_dock_edit_menu(i, inv) => {}
            Area::Dock(i) => {
                // 도크 탭으로 터미널을 고르면 바로 입력할 수 있게 포커스를 준다(사용자 10-03 Linux 실기 — 종전에는 격자를 한 번 더
                // 눌러야 했다). 셸은 다음 paint에서 시작한다.
                let was_term = self.docks[i].active_kind() == 2;
                self.docks[i].on_event(ev, inv);
                if !was_term && self.docks[i].active_kind() == 2 {
                    self.set_term_focus(Some(i), inv);
                }
            }
            // 빠른 실행 우클릭 = 항목 추가 · 편집 · 제거 · 숨기기 · 설정(dir3 신규 · T-133).
            Area::Launcher if matches!(ev, InputEvent::RightDown { .. }) => {
                self.open_launcher_menu()
            }
            Area::Launcher => self.launcherbar.on_event(ev, inv),
        }
    }

    /// 스플리터 사건 → 비율 갱신 · 드래그 끝 = 설정 저장(세 스플리터 공통).
    fn split_event(&mut self, kind: SplitKind, ev: &InputEvent, inv: &mut Invalidations) {
        match self.split_of_mut(kind).on_event(ev) {
            SplitEvent::Hover | SplitEvent::Start => inv.push(self.split_of(kind).rect()),
            SplitEvent::Drag(v) => {
                self.split_drag(kind, v);
                inv.push(Rect::new(0, 0, self.viewport.0, self.viewport.1));
            }
            SplitEvent::End => {
                let _ = self.settings.save();
                inv.push(self.split_of(kind).rect());
            }
            SplitEvent::None => {}
        }
    }

    /// 탭을 끄는 중 포인터가 **반대 패널** 위에 있으면 놓일 자리 표식을 갱신한다(탭 바 강조 + 삽입선 — `paint_into`가 그린다).
    fn tab_drag_hint(&mut self, ev: &InputEvent, inv: &mut Invalidations) {
        let InputEvent::MouseMove { x, y } = *ev else {
            return;
        };
        let hint = self
            .tab_cross_target(x, y)
            .map(|(_, dst, _, line)| (dst, line));
        if hint != self.tab_drop_hint {
            for h in [hint, self.tab_drop_hint].into_iter().flatten() {
                inv.push(self.panels[h.0].tabbar_bounds());
            }
            self.tab_drop_hint = hint;
        }
    }

    /// 패널 간 탭 놓기 대상(순수 판정): 듀얼 · 한 패널에서 탭을 끄는 중 · 포인터가 반대 패널 안 →
    /// `(원래 패널, 대상 패널, 삽입 자리(None = 끝), 표식 선)`.
    fn tab_cross_target(&self, x: i32, y: i32) -> Option<(usize, usize, Option<usize>, Rect)> {
        if !self.dual {
            return None;
        }
        let src = (0..2).find(|i| self.panels[*i].tab_dragging().is_some())?;
        let dst = 1 - src;
        if !self.panels[dst].bounds().contains(Point { x, y }) {
            return None;
        }
        let (at, line) = self.panels[dst].tab_drop_target(x, y);
        Some((src, dst, at, line))
    }

    /// 탭을 반대 패널에 놓기(사용자 10-03 "탭이 좌우 패널 간 이동도 가능하도록" · dir2 71baf67 `cross_move_tab`): 떼어 내
    /// 대상 패널의 그 자리에 붙이고 활성 패널을 옮긴다. 마지막 탭 · 잠긴 탭은 옮기지 않는다(제자리). 옮겼으면 `true`.
    fn tab_cross_drop(&mut self, x: i32, y: i32, inv: &mut Invalidations) -> bool {
        let Some((src, dst, at, _)) = self.tab_cross_target(x, y) else {
            if self.tab_drop_hint.take().is_some() {
                inv.push(Rect::new(0, 0, self.viewport.0, self.viewport.1));
            }
            return false;
        };
        self.tab_drop_hint = None;
        let Some(tab_i) = self.panels[src].tab_dragging() else {
            return false;
        };
        self.panels[src].cancel_tab_drag(inv);
        inv.push(Rect::new(0, 0, self.viewport.0, self.viewport.1));
        let Some(tab) = self.panels[src].detach_tab(tab_i, inv) else {
            return true; // 마지막/잠긴 탭 — 드래그만 접고 제자리
        };
        // 탭/폴더 범위 = 탭이 지닌 값 그대로 · 그 밖 = 대상 패널 값 채택(메뉴 "반대 패널로 이동"과 같은 규칙).
        let adopt = matches!(self.view_scope(), "global" | "panel");
        self.panels[dst].attach_tab(tab, at, adopt, inv);
        self.set_active(dst);
        self.update_status();
        true
    }

    /// 포인터 이동 = 세 스플리터의 hover 갱신. 한 자리에 둘이 겹치면(도크 높이 띠와 세로 띠의 끝) 우선순위가 높은 것만
    /// hover — 나머지는 "벗어남"으로 본다(둘이 함께 밝아지지 않게).
    fn split_hover(&mut self, ev: &InputEvent, inv: &mut Invalidations) {
        let InputEvent::MouseMove { x, y } = *ev else {
            return;
        };
        let top = self.split_at(Point { x, y });
        for k in [
            SplitKind::DockHeight,
            SplitKind::DockSplit,
            SplitKind::Panel,
        ] {
            if !self.split_shown(k) {
                continue;
            }
            if top == Some(k) || self.split_of(k).is_dragging() {
                self.split_event(k, ev, inv);
            } else if self.split_of_mut(k).pointer_gone() {
                inv.push(self.split_of(k).rect());
            }
        }
    }

    pub(crate) fn route(&mut self, ev: InputEvent) {
        let mut inv = Invalidations::default();
        self.slow_click_before(&ev); // 느린 재클릭 = 이름 바꾸기 예약/폐기(선택이 바뀌기 전 상태로 판정)
        self.route_inner(ev, &mut inv);
        self.drag_out_after(&ev, &mut inv); // 선택된 행을 끌면 OS 드래그 발신(T-147)
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
        // 상태줄 상세 팝업 토글: 누르기 시작할 때 떠 있던 팝업의 칸을 기억한다(바깥 클릭이 팝업을 닫고 아래로 흘러
        // 같은 칸을 다시 누르게 되는데, 그때 다시 열지 않는다 — 사용자 10-04 "다시 클릭하면 감춤").
        if matches!(
            ev,
            InputEvent::MouseDown { .. } | InputEvent::DoubleClick { .. }
        ) {
            self.status_popup_was = self
                .status_popup
                .clone()
                .filter(|_| self.tab_menu.is_open());
        }
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
        // 탭을 끄는 중 Esc = 취소(제자리 · dir2 WINC-116).
        if let Some(src) = (0..2).find(|i| self.panels[*i].tab_dragging().is_some()) {
            if let InputEvent::Key {
                key: nexa_ctl::Key::Escape,
                ..
            } = ev
            {
                self.panels[src].cancel_tab_drag(inv);
                self.pressed = None;
                self.tab_drop_hint = None;
                inv.push(Rect::new(0, 0, self.viewport.0, self.viewport.1));
                return;
            }
        }
        // 컬럼을 끄는 중 Esc = 취소(원래 순서 · dir2 WINC-031).
        if matches!(
            ev,
            InputEvent::Key {
                key: nexa_ctl::Key::Escape,
                ..
            }
        ) && self.panels.iter().any(panel::Panel::col_dragging)
        {
            for p in &mut self.panels {
                let _ = p.cancel_col_drag(inv);
            }
            self.pressed = None;
            inv.push(Rect::new(0, 0, self.viewport.0, self.viewport.1));
            return;
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
                    // 탭을 끄는 중이면 반대 패널 위의 놓일 자리 표식을 갱신한다(눌린 영역 = 원래 패널이 사건을 받는다).
                    self.tab_drag_hint(&ev, inv);
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
                self.split_hover(&ev, inv);
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
                // 열 경계 더블클릭 = 내용에 맞춰 폭 자동 맞춤(dir2 07-19 `win.rs:8667-8673` · T-132).
                if matches!(ev, InputEvent::DoubleClick { .. }) {
                    for i in 0..2 {
                        if !self.dual && i != self.active {
                            continue;
                        }
                        if let Some(col) = self.panels[i].rows().autofit_col_at(x, y) {
                            self.autofit_column(i, col, inv);
                            return;
                        }
                    }
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
                // 탭을 반대 패널 위에서 놓았다 = 패널 간 이동(위젯보다 먼저 판정 — dir2 WINC-101).
                if self.tab_cross_drop(x, y, inv) {
                    self.pressed = None;
                    return;
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
            if let Some(alias) = self.panels[i].take_alias() {
                // `shell:` 별칭(dir2 shellpath.rs): 풀리면 그 폴더로 · 못 풀면 원문 그대로 열어 본다(열기 실패 = 자리 유지).
                let target = self
                    .platform
                    .opener
                    .resolve_alias(&alias)
                    .unwrap_or_else(|| PathBuf::from(&alias));
                let _ = self.panels[i].navigate_to(target, inv);
            }
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
            if self.panels[i].take_rename_menu() {
                if i != self.active {
                    self.set_active(i);
                }
                self.open_rename_edit_menu(i);
            }
            if self.panels[i].take_header_menu() {
                if i != self.active {
                    self.set_active(i);
                }
                self.open_header_menu();
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
            if let Some((seg, _right)) = self.panels[i].take_status_click() {
                self.open_tab_status_menu(i, &seg);
                inv.push(Rect::new(0, 0, self.viewport.0, self.viewport.1));
            }
            if let Some(t) = self.panels[i].take_tab_menu() {
                self.open_tab_menu(i, t);
                inv.push(Rect::new(0, 0, self.viewport.0, self.viewport.1));
            }
            // 컬럼 순서(GAP-016): 끌어 바꾼 순서를 같은 패널의 모든 탭에 · 동기화가 켜져 있으면 반대 패널에도(세션에는 패널
            // 더러움으로 저장된다).
            if self.panels[i].take_col_order_changed() {
                self.sync_col_layout_from(i);
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
                    // 탭/폴더 범위 = 탭이 지닌 값 그대로(폴더 범위는 같은 폴더끼리 이미 같다) · 그 밖 = 대상 패널 값 채택.
                    let adopt = matches!(self.view_scope(), "global" | "panel");
                    self.panels[other].attach_tab(t, None, adopt, &mut inv);
                    self.set_active(other);
                }
            }
            _ => {}
        }
        self.update_status();
        self.redraw();
    }

    /// 컬럼 순서/표시 전파 — `from` 패널 활성 탭의 레이아웃을 같은 패널의 모든 탭에 · 동기화(`list.col_width_sync`)가 켜져
    /// 있으면 반대 패널에도(설정 창 순서 편집과 같은 규약 — app/order.rs `apply_col_layout_str`).
    pub(crate) fn sync_col_layout_from(&mut self, from: usize) {
        let spec = self.panels[from].col_order_spec();
        let mut inv = Invalidations::default();
        self.panels[from].apply_col_layout(&spec, &mut inv);
        if self.dual && self.settings.flag("list.col_width_sync") {
            self.panels[1 - from].apply_col_layout(&spec, &mut inv);
        }
        self.redraw();
    }

    /// 열 자동 맞춤(dir2 `win.rs::autofit_column` · 07-19): **보이는 행 + 머리글**의 폭을 목록 글꼴로 재어 그 최대값으로.
    /// 머리글은 제목 + 정렬 표시(▲/▼) + 다중 정렬 순번까지 한 덩어리로 잰다(머리 굵게 반영 — 데이터가 더 길면 데이터가 이긴다).
    /// 상한 = `list.col_autofit_max`(논리 px) · 하한 = 열 최소 폭. 반영 뒤 같은 패널의 탭 · (동기 켜짐) 반대 패널로 전파.
    pub(crate) fn autofit_column(&mut self, panel: usize, col: usize, inv: &mut Invalidations) {
        let samples = self.panels[panel].rows().autofit_texts(col);
        if samples.is_empty() {
            return;
        }
        let size = self
            .ui_font
            .em_to_px(self.settings.font_px("list.font_size"))
            * self.scale;
        let hdr_bold = self.settings.flag("list.header_bold");
        let folder_bold = self.settings.flag("list.folder_bold");
        let need = samples
            .iter()
            .enumerate()
            .map(|(i, (text, extra))| {
                // 0번 = 머리글(머리 굵게) · 그 밖 = 행(폴더 굵게가 켜져 있으면 넉넉하게 굵은 폭으로).
                let bold = if i == 0 { hdr_bold } else { folder_bold };
                self.ui_font
                    .measure_from_styled(text, size, 0.0, bold)
                    .ceil() as i32
                    + extra
                    + 1
            })
            .max()
            .unwrap_or(0);
        let max = px(
            self.settings.int("list.col_autofit_max").clamp(50, 2000) as f32,
            self.scale,
        );
        if self.panels[panel].set_col_width_user(col, need.min(max), inv) {
            if self.dual && self.settings.flag("list.col_width_sync") {
                self.sync_col_widths_from(panel);
            } else {
                // 같은 패널의 다른 탭 · 기본 열에도(새 탭이 그 폭을 잇는다).
                let widths = self.panels[panel].col_widths_by_key();
                self.panels[panel].apply_col_widths_by_key(&widths, inv);
            }
        }
        self.redraw();
    }

    /// 열 폭 동기 — `from` 패널 활성 탭의 폭을 **열 종류(key)별로** 반대 패널의 모든 탭과 같은 패널의 다른 탭에
    /// (사용자 10-03 점검: 종전 = 자리(순서)로 복사해 열 순서·표시가 다르면 엉뚱한 열에 들어갔고 · 같은 패널의 다른 탭은 그대로였다).
    pub(crate) fn sync_col_widths_from(&mut self, from: usize) {
        let widths = self.panels[from].col_widths_by_key();
        let mut inv = Invalidations::default();
        self.panels[from].apply_col_widths_by_key(&widths, &mut inv);
        self.panels[1 - from].apply_col_widths_by_key(&widths, &mut inv);
        self.redraw();
    }

    /// 포인터가 창을 떠남/비활성 = hover 정리.
    pub(crate) fn pointer_gone(&mut self) {
        let mut inv = Invalidations::default();
        let away = InputEvent::MouseMove { x: -1, y: -1 };
        self.toolbar.on_event(&away, &mut inv);
        // 창을 벗어나면 스플리터 hover도 푼다(종전 = 남아 있었다 · 드래그 중이면 유지).
        for k in [
            SplitKind::DockHeight,
            SplitKind::DockSplit,
            SplitKind::Panel,
        ] {
            if self.split_of_mut(k).pointer_gone() {
                inv.push(self.split_of(k).rect());
            }
        }
        // 끄는 중인 패널은 건너뛴다 — 열 폭을 끌다 포인터가 창을 벗어나면(또는 포커스를 잃으면) 가짜 좌표(-1)를 따라가
        // 그 열이 최소 폭 40으로 줄었다(10-03 세션 `colw=…,40,…` 관찰 · 스플리터의 "드래그 중이면 유지"와 같은 규칙).
        for p in &mut self.panels {
            if !p.is_pressed() {
                p.on_event(&away, &mut inv);
            }
        }
        // 도크 머리의 종류 칸 hover(nexa-ui 127차)도 푼다.
        for d in &mut self.docks {
            if d.bounds().h > 0 {
                d.on_event(&away, &mut inv);
            }
        }
        if !inv.is_empty() {
            self.redraw();
        }
    }
}
