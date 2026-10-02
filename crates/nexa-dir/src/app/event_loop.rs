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
        let Ok(win) = el.create_window(attrs) else {
            eprintln!("nexa-dir: window creation failed");
            el.exit();
            return;
        };
        let win = Rc::new(win);
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
        // 창이 생기면 OS 판정(winit)이 정확해진다 — System 모드는 여기서 확정.
        self.theme = theme::resolve(self.settings.theme_mode(), win.theme());
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
        let mut redraw = !inv.is_empty();
        if self.splitter.tick(now_ms) {
            redraw = true;
        }
        if self.toasts.tick(now) {
            redraw = true;
        }
        let aux_live = self.aux_tick(now_ms);
        let term_live = self.term_tick(now_ms);
        let ops_live = self.ops_tick();
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
            || self.splitter.is_hover()
            || self.splitter.is_dragging();
        let mut next = if live {
            now + Duration::from_millis(16)
        } else {
            now + Duration::from_secs(3600)
        };
        if let Some(t) = self.startup_timed.iter().map(|(at, _)| *at).min() {
            next = next.min(t);
        }
        next = next.min(self.watch_tick(now));
        if let Some(t) = self.session_tick(now) {
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
                if !on {
                    self.pointer_gone();
                }
                self.redraw();
            }
            WindowEvent::CursorLeft { .. } => self.pointer_gone(),
            WindowEvent::Moved(_) => {
                // 다른 배율의 모니터로 옮겨진 뒤 ScaleFactorChanged가 안 오는 경우(프로그램 이동) 배율을 다시 읽는다.
                if let Some(w) = &self.window {
                    let s = w.scale_factor() as f32;
                    if (s - self.scale).abs() > 0.01 {
                        self.scale = s;
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
                self.layout();
                self.redraw();
            }
            WindowEvent::ThemeChanged(_) => {
                // OS 라이트/다크 전환 — System 모드일 때만 따라간다.
                if self.settings.theme_mode() == ThemeMode::System {
                    let wt = self.window.as_ref().and_then(|w| w.theme());
                    self.theme = theme::resolve(ThemeMode::System, wt);
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
                    let over_split = self.dual
                        && (self.splitter.is_dragging() || self.splitter.rect().contains(p));
                    let over_edge = self.panels.iter().any(|g| g.rows().resize_hot(p.x, p.y));
                    w.set_cursor(if over_split || over_edge {
                        winit::window::CursorIcon::ColResize
                    } else {
                        winit::window::CursorIcon::Default
                    });
                }
            }
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
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
                    if let Some(first) = self.pending_chord.take() {
                        if let Some(id) = self.keymap.lookup_seq(&first, &ch) {
                            if !(kev.repeat && !ndir_settings::repeatable(id)) {
                                self.command(id);
                            }
                        }
                        return;
                    }
                    let plain_char =
                        !ch.primary && !ch.alt && !ch.ctrl && ch.key.chars().count() == 1;
                    // 경로바 편집 중엔 조합키 없는 키 전부 편집으로(Tab = 패널 전환도 편집 중엔 입력이 아니다 → 그대로 명령).
                    let editing = self.panels[self.active].pathbar.is_editing();
                    let term_typing =
                        self.term_focused().is_some() && !ch.primary && !ch.alt && !ch.ctrl;
                    let typing = plain_char || term_typing || (editing && !ch.primary && !ch.alt);
                    if !typing {
                        if self.keymap.is_prefix(&ch) {
                            self.pending_chord = Some(ch);
                            return;
                        }
                        if let Some(id) = self.keymap.lookup(&ch) {
                            if kev.repeat && !ndir_settings::repeatable(id) {
                                return;
                            }
                            self.command(id);
                            return;
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.paint();
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
        if let Some(ev) = self.ctl_event(&event) {
            self.route(ev);
        }
        // 사건 처리 중에 쌓인 "창 열기" 요청을 한 번에.
        self.open_requested_windows(el);
    }
}
