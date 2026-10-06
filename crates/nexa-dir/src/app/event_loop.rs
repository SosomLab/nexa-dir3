//! App — winit 사건 처리기(`ApplicationHandler` · 창 생성 · 유휴 틱 · 창 사건 → 라우팅). nexa-sql `app/event_loop.rs` 축약.
//!
//! 재그리기 3원칙(SKEL-414): ① 그리기는 `RedrawRequested`에서만 ② 유휴는 `WaitUntil(next)` ③ 하위 기능은 `tick(now)`가 더 돌아야
//! 하면 `Invalidations::request_tick`으로 알린다(자체 타이머·스레드 금지).

use super::keywinit::chord_from_winit;
use crate::*;

impl ApplicationHandler<Wake> for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        // macOS Dock 아이콘 — 기동이 끝난 첫 resumed에서(nexa-sql 09-16 실기). 다른 OS no-op.
        icon::set_dock_icon();
        // 메인 창 규칙(dir2 계승): 마지막 위치·크기로(`window.main_size`/`main_pos`) · 숨긴 채 만들고 → 위치 → 보인다(맥 배율 함정 09-17).
        let attrs = icon::with_icon(
            Window::default_attributes()
                .with_title("Nexa Dir")
                .with_visible(false)
                .with_theme(theme::window_theme(self.settings.theme_mode()))
                .with_inner_size({
                    let (w, h) = self
                        .settings
                        .get("window.main_size")
                        .and_then(wingeom::parse_size)
                        .unwrap_or((1200.0, 800.0));
                    winit::dpi::LogicalSize::new(w, h)
                }),
        );
        // Windows: winit 기본 드롭 수신을 끄고 자체 수신부를 단다(포인터 자리 · 수식키 · 효과(커서)를 직접 다룬다 — T-147).
        #[cfg(windows)]
        let attrs = {
            use winit::platform::windows::WindowAttributesExtWindows as _;
            attrs.with_drag_and_drop(false)
        };
        let Ok(win) = el.create_window(attrs) else {
            eprintln!("nexa-dir: window creation failed");
            el.exit();
            return;
        };
        let win = Rc::new(win);
        self.drop_shared = platform::register_drop_target(&win);
        {
            let mons: Vec<_> = win.available_monitors().collect();
            let place = self
                .settings
                .get("window.main_pos")
                .and_then(wingeom::parse_pos)
                .filter(|p| wingeom::on_any_monitor(*p, mons.iter().cloned()));
            if let Some((x, y)) = place {
                win.set_outer_position(wingeom::logical(x, y));
            }
            wingeom::keep_on_screen(&win, None);
            if self.settings.flag("window.always_on_top") {
                win.set_window_level(winit::window::WindowLevel::AlwaysOnTop);
            }
            win.set_visible(true);
        }
        self.scale = win.scale_factor() as f32;
        if (self.scale - 1.0).abs() > 0.01 {
            // 창 배율에 맞는 크기로 툴바 SVG 아이콘 재렌더(T-30).
            self.rebuild_toolbar();
            self.sync_menu_checks();
        }
        // 창이 생기면 OS 판정(winit)이 정확해진다 — System 모드는 여기서 확정.
        self.theme = theme::resolve(self.settings.theme_mode(), win.theme());
        self.apply_icon_switches(); // 메뉴 바 아이콘 색 = 확정된 테마 글자색
        match present::Presenter::new(win.clone()) {
            Ok(p) => self.surface = Some(p),
            Err(e) => eprintln!("nexa-dir: {e}"),
        }
        self.window = Some(win);
        self.layout();
        // 복원된 활성 패널을 덮지 않는다(10-03 창 2회 실행 실증: 세션 active_panel=1이 0으로 돌아갔다).
        let a = self.active;
        self.set_active(a);
        self.update_status();
        // 자체 캡처·하네스용 기동 명령(`NDIR_STARTUP_CMD=…` · 쉼표 구분 · `@after:<ms>:<명령>`).
        if let Ok(cmds) = std::env::var("NDIR_STARTUP_CMD") {
            self.queue_startup(&cmds);
        }
        self.redraw();
    }

    fn user_event(&mut self, _el: &ActiveEventLoop, _ev: Wake) {
        // 배경 작업 수거(M4 전송·감시) — 지금은 깨우기만.
        self.redraw();
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if self.exit_requested {
            self.persist_window();
            el.exit();
            return;
        }
        let now = Instant::now();
        let now_ms = self.started.elapsed().as_millis() as u64;
        let mut inv = Invalidations::default();
        for p in &mut self.panels {
            p.tick(now_ms, &mut inv);
        }
        // 도크의 오버레이 스크롤 막대 자동 숨김 · 속도 배지 사라짐(dir2 e230f36 — 틱이 없으면 그대로 남는다).
        for d in &mut self.docks {
            if d.bounds().h > 0 {
                d.tick(&mut inv);
            }
        }
        let mut redraw = !inv.is_empty();
        for k in [
            SplitKind::Panel,
            SplitKind::DockHeight,
            SplitKind::DockSplit,
        ] {
            if self.split_of_mut(k).tick(now_ms) {
                redraw = true;
            }
        }
        if self.toasts.tick(now) {
            redraw = true;
        }
        let aux_live = self.aux_tick(now_ms);
        let term_live = self.term_tick(now_ms);
        let ops_live = self.ops_tick()
            | self.dirsize_tick()
            | self.hash_tick()
            | self.extract_tick()
            | self.dupes_tick()
            | self.compare_tick();
        // 다른 프로그램에서 끌어오는 동안 · 놓은 직후: OS에서 포인터 자리 · 수식키를 읽어 반영한다(winit은 드래그 중 그 사건을
        // 주지 않는다 — T-147 수신 보강). 수식키는 판정에만 쓰고 되돌린다(창이 포커스를 받으면 winit이 다시 알려 준다).
        // 자체 수신부(Windows)가 쌓아 둔 사건을 먼저 거둔다.
        if self.drop_pump(now) {
            redraw = true;
        }
        let dnd_active = self.dnd_hovering() || !self.dnd_drop.is_empty();
        let mods = (self.shift, self.primary);
        if dnd_active {
            let inner = self
                .window
                .as_ref()
                .and_then(|w| w.inner_position().ok())
                .map(|p| (p.x, p.y));
            if let (Some(ps), Some(inner)) = (platform::pointer_state(), inner) {
                let at = app::dnd::client_point((ps.x, ps.y), inner);
                // 멈춰 있는 포인터도 계속 본다(머물면 열기의 시간 · 수식키만 바꾼 경우 · 가장자리 자동 스크롤).
                if self.dnd_hovering() {
                    let _ = self.dnd_event(
                        platform::DropEvent::Over {
                            at,
                            ctrl: ps.ctrl,
                            shift: ps.shift,
                        },
                        now,
                    );
                    redraw = true;
                } else if self.dnd_track(at, ps.ctrl, ps.shift, now) {
                    redraw = true;
                }
            }
        }
        if self.dnd_flush() {
            redraw = true;
        }
        if dnd_active {
            (self.shift, self.primary) = mods;
        }
        self.open_requested_windows(el);
        if !self.startup_timed.is_empty() {
            let due: Vec<String> = self
                .startup_timed
                .iter()
                .filter(|(at, _)| *at <= now)
                .map(|(_, c)| c.clone())
                .collect();
            self.startup_timed.retain(|(at, _)| *at > now);
            for id in due {
                self.startup_cmd(&id);
            }
            // 지연 명령이 종료를 요청했으면 지금(다음 깨움이 없을 수 있다 — 10-03 실증: `@after:…:app.exit`가 잠든 채 남았다).
            if self.exit_requested {
                self.persist_window();
                el.exit();
                return;
            }
        }
        if redraw {
            self.redraw();
        }
        // 애니메이션 중에만 프레임 간격으로 깬다 · 아니면 다음 예약(기동 지연 명령)까지 잔다.
        let live = inv.tick_requested()
            || aux_live
            || self.toasts.animating()
            // 스플리터: 페이드가 움직이는 동안과 드래그 중에만 프레임 간격으로 깬다(종전 = hover 내내 16 ms 폴링 ·
            // 벗어난 뒤 페이드아웃은 깨우지 않았다).
            || [SplitKind::Panel, SplitKind::DockHeight, SplitKind::DockSplit]
                .into_iter()
                .any(|k| self.split_of(k).is_animating() || self.split_of(k).is_dragging());
        let mut next = if live {
            now + Duration::from_millis(16)
        } else {
            now + Duration::from_secs(3600)
        };
        if let Some(t) = self.startup_timed.iter().map(|(at, _)| *at).min() {
            next = next.min(t);
        }
        // 유휴 트림(T-179 A) — 애니메이션 · 작업 · 터미널 출력 · 셸 메뉴 구축 중에는 미룬다.
        let busy = live || ops_live || term_live || self.platform.ctxmenu.busy();
        if let Some(t) = self.idle_trim_tick(now, busy) {
            next = next.min(t);
        }
        next = next.min(self.watch_tick(now));
        if let Some(t) = self.docks_tick(now) {
            next = next.min(t);
        }
        if self.panels.iter().any(panel::Panel::typeahead_active) {
            // 타입어헤드 입력 중 = 유지 시간이 지나면 배지를 지워야 한다(사건이 없어도) → 짧게 깬다.
            next = next.min(now + Duration::from_millis(100));
        }
        if self.dnd_hovering() {
            // 끌어오는 동안은 사건이 오지 않는다 → 추적 간격으로 스스로 깬다(가장자리 자동 스크롤 · 놓는 자리 갱신).
            next = next.min(now + Duration::from_millis(app::dnd::DND_TRACK_MS));
        }
        if let Some(t) = self.session_tick(now) {
            next = next.min(t);
        }
        if let Some(t) = self.launcher_icons_tick(now) {
            next = next.min(t);
        }
        if let Some(t) = self.status_load_tick(now) {
            next = next.min(t);
        }
        if let Some(t) = self.git_tick(now) {
            next = next.min(t);
        }
        if let Some(t) = self.ctx_shell_tick(now) {
            next = next.min(t);
        }
        if let Some(t) = self.row_icons_tick(now) {
            next = next.min(t);
        }
        if let Some(t) = self.mem_tick(now) {
            next = next.min(t);
        }
        if let Some(t) = self.slow_click_tick(now) {
            next = next.min(t);
        }
        // 상태줄 표식(디스크 · 네트워크 ▲▼) 깜빡임 — 위상이 바뀔 때만 다시 그리고, 송수신이 있을 때만 다음 전환 시각에 깬다.
        if self.statusbar.tick(now_ms) {
            self.redraw();
        }
        if let Some(ms) = self.statusbar.next_blink_ms(now_ms) {
            next = next.min(now + Duration::from_millis(ms.max(1)));
        }
        // 우클릭 메뉴의 오버레이 스크롤 막대 — 감출 시각이 지났으면 감추고 다시 그린다.
        if self.tab_menu.tick(now) {
            self.redraw();
        }
        if let Some(t) = self.tab_menu.next_wake() {
            next = next.min(t);
        }
        if let Some(d) = self.term_wake(term_live) {
            next = next.min(now + d);
        }
        if ops_live {
            next = next.min(now + Duration::from_millis(app::ops::OPS_POLL_MS));
        }
        el.set_control_flow(ControlFlow::WaitUntil(next));
    }

    fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.trace_ime {
            match &event {
                WindowEvent::KeyboardInput {
                    event: k,
                    is_synthetic,
                    ..
                } => eprintln!(
                    "[ime] {id:?} key state={:?} logical={:?} physical={:?} text={:?} repeat={} synth={}",
                    k.state, k.logical_key, k.physical_key, k.text, k.repeat, is_synthetic
                ),
                WindowEvent::Ime(i) => eprintln!("[ime] {id:?} ime {i:?}"),
                WindowEvent::Focused(f) => eprintln!("[ime] {id:?} focused={f}"),
                WindowEvent::ModifiersChanged(m) => eprintln!("[ime] {id:?} mods={:?}", m.state()),
                _ => {}
            }
        }
        // 창이 포커스를 얻을 때 **이미 눌려 있는 키**의 합성 누름은 버린다(nexa-sql 09-21 — 닫힌 창의 키가 새는 길).
        if let WindowEvent::KeyboardInput {
            event: k,
            is_synthetic: true,
            ..
        } = &event
        {
            if k.state == ElementState::Pressed {
                return;
            }
        }
        // 사용자 입력 = 유휴 시계 되감기(어느 창이든 · T-179 A).
        if app::memory::is_user_input(&event) {
            self.last_input_ms = self.started.elapsed().as_millis() as u64;
            self.idle_trimmed = false;
        }
        // 보조 창(설정 · 단축키) 사건 = 창마다 자기 처리기로(처리했으면 끝) — 보조 창이 낸 "창 열기" 요청도 같은 펌프를 지난다.
        if self.aux_window_event(id, &event) {
            self.open_requested_windows(el);
            return;
        }
        if self.window.as_ref().is_some_and(|w| w.id() != id) {
            return;
        }
        match &event {
            WindowEvent::CloseRequested => {
                self.exit_requested = true;
                self.persist_window();
                el.exit();
            }
            WindowEvent::Focused(on) => {
                self.main_active = *on;
                if *on {
                    // 다른 앱(탐색기)이 잘라낸 것도 흐리게(SHELL-044) — 돌아올 때 한 번 동기.
                    self.sync_cut_marks();
                    // 시스템 마우스 설정(한 번에 스크롤할 줄 수)이 그 사이 바뀌었을 수 있다.
                    if let Some(n) = platform::wheel_lines() {
                        nexa_ctl::set_wheel_lines(n);
                    }
                } else {
                    self.pointer_gone();
                }
                self.redraw();
            }
            WindowEvent::CursorLeft { .. } => self.pointer_gone(),
            // 외부 끌어다 놓기 1차(SHELL-060 · 3-OS 공통): 파일마다 한 건 — 틱에서 모아 처리.
            WindowEvent::HoveredFile(p) => self.dnd_hover(p.clone()),
            WindowEvent::HoveredFileCancelled => self.dnd_cancel(),
            WindowEvent::DroppedFile(p) => self.dnd_dropped(p.clone()),
            WindowEvent::Moved(_) => {
                // 다른 배율의 모니터로 옮겨진 뒤 ScaleFactorChanged가 안 오는 경우(프로그램 이동) 배율을 다시 읽는다.
                if let Some(w) = &self.window {
                    let s = w.scale_factor() as f32;
                    if (s - self.scale).abs() > 0.01 {
                        self.scale = s;
                        self.rebuild_toolbar();
                        self.sync_menu_checks();
                        self.layout();
                    }
                }
                self.redraw();
            }
            WindowEvent::Resized(_) => {
                self.layout();
                self.redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.rebuild_toolbar();
                self.sync_menu_checks();
                self.layout();
                self.redraw();
            }
            WindowEvent::ThemeChanged(_) => {
                // OS 라이트/다크 전환 — System 모드일 때만 따라간다.
                if self.settings.theme_mode() == ThemeMode::System {
                    let wt = self.window.as_ref().and_then(|w| w.theme());
                    self.theme = theme::resolve(ThemeMode::System, wt);
                    self.apply_icon_switches();
                    self.redraw();
                }
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.primary = if cfg!(target_os = "macos") {
                    m.state().super_key()
                } else {
                    m.state().control_key()
                };
                self.ctrl_mac = cfg!(target_os = "macos") && m.state().control_key();
                self.alt = m.state().alt_key();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as i32, position.y as i32);
                if let Some(w) = &self.window {
                    let p = Point {
                        x: self.cursor.0,
                        y: self.cursor.1,
                    };
                    let over_edge = self.panels.iter().any(|g| g.rows().resize_hot(p.x, p.y));
                    w.set_cursor(match self.split_at(p) {
                        // 가로 경계(패널 ↔ 도크) = 위아래 화살표(dir2 IDC_SIZENS) · 세로 경계 = 좌우.
                        Some(SplitKind::DockHeight) => winit::window::CursorIcon::RowResize,
                        Some(_) => winit::window::CursorIcon::ColResize,
                        None if over_edge => winit::window::CursorIcon::ColResize,
                        None => winit::window::CursorIcon::Default,
                    });
                }
            }
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                if self.dlg.is_open() {
                    self.dlg.focus(); // 모달(T-29) — 단축키도 대화상자로
                    return;
                }
                // 한/영 키(Windows · 목록 입력 전용 — nexa-sql 탐색기와 같은 길): 메인 창은 IME를 붙이지 않아 OS 전환이 듣지
                // 않으므로 앱이 모드를 뒤집는다. VK_HANGUL은 키보드 드라이버 수준이라 IME 없이도 온다(논리 HangulMode · 물리 Lang1).
                if cfg!(windows)
                    && (kev.logical_key
                        == winit::keyboard::Key::Named(winit::keyboard::NamedKey::HangulMode)
                        || kev.physical_key
                            == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Lang1))
                    && self.toggle_hangul_mode()
                {
                    return;
                }
                // 단축키 = 키맵 표 조회(dir2 표 + macOS 대응안). 조합키 없는 글자는 타이핑(타입어헤드·경로바)이므로 가로채지 않는다.
                if let Some(ch) = chord_from_winit(
                    &kev.logical_key,
                    &kev.physical_key,
                    self.primary,
                    self.shift,
                    self.alt,
                    self.ctrl_mac,
                ) {
                    // 터미널 포커스(T-61): Ctrl/⌘+글자는 키맵보다 먼저 — 제어 문자·복사/붙여넣기 · Tab = 완성.
                    if self.term_focused().is_some() && self.pending_chord.is_none() {
                        let is_cmd = cfg!(target_os = "macos") && ch.primary;
                        let is_ctrl = if cfg!(target_os = "macos") {
                            ch.ctrl
                        } else {
                            ch.primary
                        };
                        let mut it = ch.key.chars();
                        if let (Some(c), None) = (it.next(), it.next()) {
                            if (is_cmd || is_ctrl)
                                && !ch.alt
                                && c.is_ascii_alphabetic()
                                && self.term_ctrl(c, ch.shift, is_cmd)
                            {
                                return;
                            }
                        }
                        if ch.key == "tab" && !ch.primary && !ch.alt && !ch.ctrl {
                            let mut inv = Invalidations::default();
                            self.term_key(&InputEvent::Char { c: '\t', now_ms: 0 }, &mut inv);
                            self.redraw();
                            return;
                        }
                    }
                    if self.key_chord(ch, kev.repeat) {
                        return;
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.paint();
                self.sync_ime_area();
                // 첫 프레임 뒤 = `@ready` 큐(초기 열거는 `App::new`에서 이미 끝났다).
                if !self.ready_fired {
                    self.fire_ready();
                    if self.exit_requested {
                        self.persist_window();
                        el.exit();
                    }
                }
                return;
            }
            _ => {}
        }
        // 입력기(IME) 글 — 조합 중인 글(Preedit)과 확정된 글(Commit)을 갈 곳(편집 필드 · 목록 타입어헤드 · 터미널)에 넣는다.
        // winit은 입력기를 붙이면 조합 중인 글을 OS가 그리지 않게 하고 앱에 넘긴다(`ime_input`이 직접 보여 준다).
        match &event {
            WindowEvent::Ime(winit::event::Ime::Preedit(text, _)) => self.ime_input("", text),
            WindowEvent::Ime(winit::event::Ime::Commit(text)) => self.ime_input(text, ""),
            WindowEvent::Ime(_) => {}
            _ => {
                if let Some(ev) = self.ctl_event(&event) {
                    // 글 영역의 더블/트리플 클릭 = 단어/줄 선택(처리했으면 여기서 끝).
                    if !self.text_click_select(&ev) {
                        self.route(ev);
                    }
                }
            }
        }
        self.ime_refresh(); // 포커스가 바뀌었으면(편집 시작/끝 · 터미널) 입력기를 붙이거나 뗀다
                            // 사건 처리 중에 쌓인 "창 열기" 요청을 한 번에.
        self.open_requested_windows(el);
    }
}

impl App {
    /// 조합 한 번의 처리(키맵 조회 → 명령) — 처리했으면 `true`(호출부는 그 키를 컨트롤로 흘리지 않는다). 창 사건 처리에서 떼어
    /// **시험이 실제 키 경로를 그대로** 밟게 했다(10-03: Enter · Alt+↓가 키맵에 걸린 뒤 처리 분기가 없어 아무 일도 안 했는데,
    /// 시험은 컨트롤 사건을 직접 넣어 통과하고 있었다).
    pub(crate) fn key_chord(&mut self, ch: ndir_settings::Chord, repeat: bool) -> bool {
        if let Some(first) = self.pending_chord.take() {
            if let Some(id) = self.keymap.lookup_seq(&first, &ch) {
                if !(repeat && !ndir_settings::repeatable(id)) {
                    self.command(id);
                }
            }
            return true;
        }
        let plain_char = !ch.primary && !ch.alt && !ch.ctrl && ch.key.chars().count() == 1;
        // 경로바 편집 중엔 조합키 없는 키 전부 편집으로(Tab = 패널 전환도 편집 중엔 입력이 아니다 → 그대로 명령).
        let editing = self.panels[self.active].pathbar.is_editing()
            || self.panels[self.active].rows().is_renaming();
        let term_typing = self.term_focused().is_some() && !ch.primary && !ch.alt && !ch.ctrl;
        let typing = plain_char || term_typing || (editing && !ch.primary && !ch.alt);
        if typing {
            return false;
        }
        if self.keymap.is_prefix(&ch) {
            self.pending_chord = Some(ch);
            return true;
        }
        let Some(id) = self.keymap.lookup(&ch) else {
            return false;
        };
        // 열린 메뉴(우클릭 메뉴 · 메뉴 바)의 Enter = 그 메뉴의 "고르기" — 목록 활성화 명령으로 가로채지 않는다.
        if id == "nav.activate" && (self.tab_menu.is_open() || self.menubar.is_open()) {
            return false;
        }
        if repeat && !ndir_settings::repeatable(id) {
            return true;
        }
        self.command(id);
        true
    }
}
