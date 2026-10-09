//! App — 기동 명령(`NDIR_STARTUP_CMD` · 자체 시험·하네스 T4 경로 · nexa-sql docs/61 §4 계승 · dir3 확장 = docs/18 §5 · docs/port/44 CI-106·107).
//!
//! `NDIR_STARTUP_CMD=<명령>,<명령>,…`(쉼표 구분) — 키 주입(SendKeys) 없이 특정 화면·상태를 만든다. 평소엔 변수 없음 = 비용 0.
//! - 명령 id(`view.hidden` · `nav.up` …) = [`App::command`] 한 길 · `ui.key:<조합>`/`key:<조합>` = 키맵 조회 뒤 명령
//! - `@ready:<명령>` = 첫 프레임 뒤 · `@idle:<명령>` = 작업 큐가 빈 뒤(지금은 ready와 같다 — 작업 큐는 M6) · `@after:<ms>:<명령>`
//! - `nav:<경로>` · `panel:<0|1>` · `ui.move|click|dclick|rclick|wheel:<x>/<y>[/<delta>]`(창 좌표 · 장치 px) · `ui.click:@<영역>`(영역 가운데 —
//!   `menubar` `toolbar` `panel0` `panel1` `panel0.tabs|nav|path|list` `splitter` `statusbar`)
//! - `<대상>.dump:<파일>`(`layout` `panel` `list` `tabs` `status` `menu` · `dump:<파일>` = 전부)
//! - `assert.<대상>:<식>` = 그 덤프에 `식`이 **들어 있어야**(앞에 `!` = 없어야) — 실패 = stderr 한 줄 + 덤프 + 종료 코드 3
//! - `quit[:코드]` = 정상 종료(코드 기본 0) · `app.exit` = `quit`

use crate::*;

/// 단언 실패 종료 코드.
pub(crate) const EXIT_ASSERT: u8 = 3;

/// 결정적 대기 명령인가(`<대상>.wait` — 뒤 `@ready` 명령을 보류한다 · [`App::startup_wait_busy`]가 해제 조건).
pub(crate) fn is_wait_cmd(id: &str) -> bool {
    matches!(
        id,
        "ctx.wait"
            | "xdnd.wait"
            | "ops.wait"
            | "dupes.wait"
            | "compare.wait"
            | "hash.wait"
            | "jobs.wait"
            | "dirsize.wait"
            | "git.wait"
    )
}

impl App {
    pub(crate) fn startup_cmd(&mut self, id: &str) {
        crash::note_command(id);
        // 예약된 도크 갱신(T-177 디바운스)은 기동 명령 앞에서 먼저 흘려보낸다 — 덤프 · 단언이 최신 도크를 본다(결정적).
        if self.docks_due.is_some() {
            self.update_docks();
        }
        // ★ 기동 명령 = 사용자 동작의 대역 → **유휴 시계를 되감는다**(10-08 메모리 2차: 종전엔 창 사건만 되감아 유휴 트림이 기동
        // 뒤 1회 돌고 잠겨 시나리오의 "동작 → 트림 뒤" 지점이 트림 없는 값으로 찍혔다 — 협업 세션 memscen 3회차 미재현). 덤프 ·
        // 단언 · 종료는 관찰이라 제외(덤프 직전에 트림이 미뤄지면 안 된다).
        let observe = id.contains(".dump:")
            || id.starts_with("dump:")
            || id.starts_with("assert.")
            || id == "quit"
            || id == "app.exit";
        if !observe {
            self.last_input_ms = self.started.elapsed().as_millis() as u64;
            self.idle_trimmed = false;
        }
        if let Some((target, path)) = id.split_once(".dump:") {
            if let Some(text) = self.dump_of(target) {
                let _ = std::fs::write(path, text);
            }
            return;
        }
        if let Some(path) = id.strip_prefix("dump:") {
            let _ = std::fs::write(path, self.dump_of("all").unwrap_or_default());
            return;
        }
        if let Some((target, expr)) = id.strip_prefix("assert.").and_then(|r| r.split_once(':')) {
            self.assert_dump(target, expr);
            return;
        }
        if let Some(code) = id.strip_prefix("quit") {
            // 단언 실패(3)는 뒤의 `quit:<코드>`가 덮지 못한다(10-03 실증: 실패 뒤 quit:5 = 5로 끝나 러너가 성공으로 봤다).
            if self.exit_code == 0 {
                self.exit_code = code
                    .strip_prefix(':')
                    .and_then(|c| c.trim().parse().ok())
                    .unwrap_or(0);
            }
            self.exit_requested = true;
            return;
        }
        if let Some(q) = id.strip_prefix("prefs.search:") {
            self.open_prefs = true;
            self.refresh_prefs();
            self.prefs_win.preset_query(q);
            return;
        }
        if let Some(cat) = id.strip_prefix("prefs.cat:") {
            self.open_prefs = true;
            self.refresh_prefs();
            self.prefs_win.select_category(cat);
            return;
        }
        if let Some(spec) = id.strip_prefix("xdnd.drop:") {
            self.xdnd_drop_cmd(spec);
            return;
        }
        if let Some(spec) = id.strip_prefix("xdnd.target:") {
            self.xdnd_target_cmd(spec);
            return;
        }
        // 끌기(T4 · 창 안 드래그 · 발신 세션): `ui.drag:@from>@to` = 누름(from) → 임계 넘는 이동 → to까지 이동(뗌은 `ui.up:@to`로 따로 —
        // 그 사이 틱이 돌아 외부 대상의 Status가 도착한다).
        if let Some(spec) = id.strip_prefix("ui.drag:") {
            let Some((from, to)) = spec.split_once('>') else {
                eprintln!("[startup] ui.drag: <from>><to>");
                return;
            };
            let (Some((fx, fy, _)), Some((tx, ty, _))) = (
                self.resolve_point(from.trim()),
                self.resolve_point(to.trim()),
            ) else {
                eprintln!("[startup] ui.drag: unknown area: {spec}");
                return;
            };
            let (shift, primary) = (self.shift, self.primary);
            self.cursor = (fx, fy);
            self.route(InputEvent::MouseMove { x: fx, y: fy });
            self.route(InputEvent::MouseDown {
                x: fx,
                y: fy,
                shift,
                primary,
            });
            self.cursor = (fx + 6, fy + 6);
            self.route(InputEvent::MouseMove {
                x: fx + 6,
                y: fy + 6,
            });
            self.cursor = (tx, ty);
            self.route(InputEvent::MouseMove { x: tx, y: ty });
            return;
        }
        if let Some(path) = id.strip_prefix("nav:") {
            let mut inv = Invalidations::default();
            let _ = self.panels[self.active].navigate_to(PathBuf::from(path), &mut inv);
            self.update_status();
            self.redraw();
            return;
        }
        if let Some(n) = id.strip_prefix("panel:") {
            let i = n.trim().parse::<usize>().unwrap_or(0).min(1);
            self.set_active(i);
            self.update_status();
            self.redraw();
            return;
        }
        if let Some(code) = id
            .strip_prefix("key:")
            .or_else(|| id.strip_prefix("ui.key:"))
        {
            if let Some(ch) = Chord::parse(code) {
                if let Some(cmd) = self.keymap.lookup(&ch) {
                    self.command(cmd);
                }
            }
            return;
        }
        if let Some((kind, where_)) = id
            .strip_prefix("ui.")
            .and_then(|r| r.split_once(':'))
            .filter(|(k, _)| {
                matches!(
                    *k,
                    "move" | "click" | "dclick" | "rclick" | "wheel" | "down" | "up"
                )
            })
        {
            let Some((x, y, delta)) = self.resolve_point(where_) else {
                eprintln!("[startup] unknown area: {where_}");
                return;
            };
            self.cursor = (x, y);
            self.route(InputEvent::MouseMove { x, y });
            let (shift, primary) = (self.shift, self.primary);
            match kind {
                "wheel" => self.route(InputEvent::Wheel { delta }),
                "rclick" => self.route(InputEvent::RightDown { x, y }),
                "dclick" => {
                    self.route(InputEvent::MouseDown {
                        x,
                        y,
                        shift,
                        primary,
                    });
                    self.route(InputEvent::MouseUp { x, y });
                    self.route(InputEvent::DoubleClick {
                        x,
                        y,
                        shift,
                        primary,
                    });
                    self.route(InputEvent::MouseUp { x, y });
                }
                "click" => {
                    self.route(InputEvent::MouseDown {
                        x,
                        y,
                        shift,
                        primary,
                    });
                    self.route(InputEvent::MouseUp { x, y });
                }
                "down" => self.route(InputEvent::MouseDown {
                    x,
                    y,
                    shift,
                    primary,
                }),
                "up" => self.route(InputEvent::MouseUp { x, y }),
                _ => {}
            }
            return;
        }
        if let Some(path) = id.strip_prefix("license.install:") {
            // 라이선스 파일 설치(LIC-164 ② · 파일 창 없이).
            self.license_install(std::path::Path::new(path.trim()));
            return;
        }
        if let Some(n) = id.strip_prefix("dock.kind:") {
            let k = n.trim().parse::<usize>().unwrap_or(0);
            let i = self.term_dock_index();
            let mut inv = Invalidations::default();
            self.docks[i].set_active_kind(k, &mut inv);
            self.update_docks();
            self.redraw();
            return;
        }
        if let Some(text) = id.strip_prefix("term.send:") {
            let i = self.term_dock_index();
            let mut inv = Invalidations::default();
            self.docks[i].set_active_kind(2, &mut inv);
            self.set_term_focus(Some(i), &mut inv);
            self.term_ensure_started(i);
            let s = text
                .replace("\\r", "\r")
                .replace("\\n", "\n")
                .replace("\\t", "\t")
                .replace("\\e", "\x1b");
            self.terms[i].write(&s);
            self.terms[i].view_off = 0;
            self.redraw();
            return;
        }
        if let Some(item) = id.strip_prefix("ctx.pick:") {
            self.ctx_pick(item.trim());
            return;
        }
        if let Some(key) = id.strip_prefix("order.open:") {
            // 순서 편집 창(T-71 · `toolbar.layout` / `list.col_layout` / `ctxmenu.layout`).
            self.open_order_editor(key.trim());
            return;
        }
        if let Some(n) = id.strip_prefix("dlg.pick:") {
            let id = n.trim().parse::<i32>().unwrap_or(0);
            self.dlg_pick(id);
            return;
        }
        if let Some(text) = id.strip_prefix("dlg.type:") {
            self.dlg_type(text);
            return;
        }
        if let Some(text) = id.strip_prefix("ui.type:") {
            // 글자 입력(이름 바꾸기·경로 바·타입어헤드) — `\b` = Backspace.
            let text = text.replace("\\b", "\u{8}");
            for c in text.chars() {
                self.route(InputEvent::Char { c, now_ms: 0 });
            }
            return;
        }
        if let Some(name) = id.strip_prefix("ui.press:") {
            use nexa_ctl::Key as K;
            let key = match name.trim() {
                "enter" => K::Enter,
                "escape" | "esc" => K::Escape,
                "up" => K::Up,
                "down" => K::Down,
                "left" => K::Left,
                "right" => K::Right,
                "home" => K::Home,
                "end" => K::End,
                "pageup" => K::PageUp,
                "pagedown" => K::PageDown,
                "delete" => K::Delete,
                "space" => K::Space,
                other => {
                    eprintln!("[startup] unknown key: {other}");
                    return;
                }
            };
            self.route(InputEvent::Key {
                key,
                shift: false,
                primary: false,
            });
            return;
        }
        if let Some(n) = id.strip_prefix("list.select:") {
            let row = n.trim().parse::<usize>().unwrap_or(0);
            let mut inv = Invalidations::default();
            self.panels[self.active].rows_mut().select_program(
                row,
                nexa_grid::SelectOp::Single,
                &mut inv,
            );
            self.update_status();
            self.redraw();
            return;
        }
        if id == "term.focus" {
            let i = self.term_dock_index();
            let mut inv = Invalidations::default();
            self.docks[i].set_active_kind(2, &mut inv);
            self.set_term_focus(Some(i), &mut inv);
            self.term_ensure_started(i);
            self.update_docks();
            self.redraw();
            return;
        }
        match id {
            "app.exit" => self.exit_requested = true,
            "ops.cancel" => self.cancel_transfer(),
            _ => self.command(id),
        }
    }

    /// `x/y[/delta]` 또는 `@<영역>[/delta]` → 창 좌표.
    fn resolve_point(&self, spec: &str) -> Option<(i32, i32, i32)> {
        if let Some(rest) = spec.strip_prefix('@') {
            let (area, delta) = rest
                .split_once('/')
                .map_or((rest, 120), |(a, d)| (a, d.trim().parse().unwrap_or(120)));
            let r = self.area_rect(area)?;
            return Some((r.x + r.w / 2, r.y + r.h / 2, delta));
        }
        let mut it = spec
            .split(['/', 'x'])
            .map(|v| v.trim().parse::<i32>().unwrap_or(0));
        let (x, y) = (it.next().unwrap_or(0), it.next().unwrap_or(0));
        Some((x, y, it.next().unwrap_or(120)))
    }

    /// 시험 자동화 `xdnd.drop:<경로>[;<경로>…]@<영역|x/y>`(Linux X11 · 10-10): 다른 X 연결이 XDND 소스가 되어 **우리 창의 그 자리**에
    /// 놓는다(포인터도 그 자리로 옮긴다 → `pointer_state`가 같은 자리를 읽는다). 수신은 winit → `dnd_hover/dnd_dropped` → 틱의
    /// 실제 경로 그대로. 뒤에 `xdnd.wait` · `ops.wait`.
    fn xdnd_drop_cmd(&mut self, spec: &str) {
        let Some((files, where_)) = spec.rsplit_once('@') else {
            eprintln!("[startup] xdnd.drop: <paths>@<area>");
            return;
        };
        let paths: Vec<PathBuf> = files
            .split(';')
            .filter(|s| !s.trim().is_empty())
            .map(|s| PathBuf::from(s.trim()))
            .collect();
        let point = if where_
            .chars()
            .all(|c| c.is_ascii_digit() || c == '/' || c == 'x')
        {
            self.resolve_point(where_)
        } else {
            self.resolve_point(&format!("@{where_}"))
        };
        let Some((x, y, _)) = point else {
            eprintln!("[startup] xdnd.drop: unknown area: {where_}");
            return;
        };
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            let Some(w) = self.window.as_ref() else {
                eprintln!("[startup] xdnd.drop: no window");
                return;
            };
            let Some(id) = winfocus::x11_window_id(w) else {
                eprintln!("[startup] xdnd.drop: not an X11 window");
                return;
            };
            let inner = w.inner_position().map_or((0, 0), |p| (p.x, p.y));
            let at = (
                i16::try_from(inner.0 + x).unwrap_or(i16::MAX),
                i16::try_from(inner.1 + y).unwrap_or(i16::MAX),
            );
            self.xdnd_started = Instant::now();
            platform::xdnd_set_inject_local(Some((x, y)));
            self.xdnd_job = Some(platform::xdnd_inject_drop(id, at, paths));
        }
        #[cfg(not(all(unix, not(target_os = "macos"))))]
        {
            let _ = (x, y, paths);
            eprintln!("[startup] xdnd.drop: Linux X11 only");
        }
    }

    /// 시험 자동화 `xdnd.target:<파일>@<영역>`(Linux X11): 그 영역(루트 좌표)에 가짜 XdndAware 창을 띄워 받은 uri-list를 파일에 쓴다 —
    /// 발신 세션(합성 모드)은 이 창만 대상으로 본다(`xdnd-send.scn`).
    fn xdnd_target_cmd(&mut self, spec: &str) {
        let Some((out, area)) = spec.rsplit_once('@') else {
            eprintln!("[startup] xdnd.target: <file>@<area>");
            return;
        };
        let Some(r) = self.area_rect(area.trim()) else {
            eprintln!("[startup] xdnd.target: unknown area: {area}");
            return;
        };
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            // 합성 모드(시나리오)는 창 기준 좌표로 등록한다(`App::root_point`와 같은 기준 · WM이 창을 옮겨도 안 어긋남).
            let inner = self
                .window
                .as_ref()
                .filter(|_| !platform::synthetic_pointer())
                .and_then(|w| w.inner_position().ok())
                .map_or((0, 0), |p| (p.x, p.y));
            let rect = (
                i16::try_from(inner.0 + r.x).unwrap_or(0),
                i16::try_from(inner.1 + r.y).unwrap_or(0),
                u16::try_from(r.w).unwrap_or(1),
                u16::try_from(r.h).unwrap_or(1),
            );
            if let Err(e) = platform::xdnd_fake_target(PathBuf::from(out.trim()), rect) {
                eprintln!("[startup] xdnd.target: {e}");
            }
        }
        #[cfg(not(all(unix, not(target_os = "macos"))))]
        {
            let _ = (out, r);
            eprintln!("[startup] xdnd.target: Linux X11 only");
        }
    }

    /// 영역 이름 → 사각형(덤프의 줄 이름과 같다). `panelN.row:<i>` = 목록의 i번째 가시 행(세로로 훑어 `row_at`이 그 행인 띠).
    pub(crate) fn area_rect(&self, name: &str) -> Option<Rect> {
        let panel = |i: usize, part: &str| -> Option<Rect> {
            let p = &self.panels[i];
            if let Some(n) = part.strip_prefix("row:") {
                let n: usize = n.trim().parse().ok()?;
                let b = p.rows().bounds();
                let x = b.x + 24;
                let (mut top, mut bottom) = (None, None);
                for y in b.y..b.bottom() {
                    if p.rows().row_at(x, y) == Some(n) {
                        top.get_or_insert(y);
                        bottom = Some(y);
                    } else if top.is_some() {
                        break;
                    }
                }
                let (t, bt) = (top?, bottom?);
                return Some(Rect::new(b.x, t, b.w, bt - t + 1));
            }
            Some(match part {
                "" => p.bounds(),
                "tabs" => p.tabbar.bounds(),
                "nav" => p.nav_rect(),
                "path" => p.pathbar.bounds(),
                "list" => p.rows().bounds(),
                _ => return None,
            })
        };
        match name {
            "menubar" => Some(self.menubar.bounds()),
            "toolbar" => Some(self.toolbar.bounds()),
            "splitter" => Some(self.splitter.rect()),
            "dsplit_h" => Some(self.dock_split_h.rect()),
            "dsplit_v" => Some(self.dock_split_v.rect()),
            "statusbar" => Some(self.statusbar.bounds()),
            "dock0" => Some(self.docks[0].bounds()),
            "dock1" => Some(self.docks[1].bounds()),
            _ => {
                let (p, part) = name.split_once('.').unwrap_or((name, ""));
                match p {
                    "panel0" => panel(0, part),
                    "panel1" => panel(1, part),
                    "panel" => panel(self.active, part),
                    _ => None,
                }
            }
        }
    }

    /// 단언 — 덤프 본문에 `expr`(앞 `!` = 부정)이 있는가. 실패 = stderr + 종료 코드 3 + 종료 요청(러너가 판정).
    pub(crate) fn assert_dump(&mut self, target: &str, expr: &str) -> bool {
        let (negate, needle) = expr.strip_prefix('!').map_or((false, expr), |e| (true, e));
        let Some(text) = self.dump_of(target) else {
            eprintln!("[assert] FAIL unknown target {target}");
            self.exit_code = EXIT_ASSERT;
            self.exit_requested = true;
            return false;
        };
        let hit = text.contains(needle);
        let ok = hit != negate;
        if !ok {
            eprintln!("[assert] FAIL {target}:{expr}\n{text}");
            self.exit_code = EXIT_ASSERT;
            self.exit_requested = true;
        }
        ok
    }

    /// 덤프 어휘(CI-107) — `layout` · `panel`(활성) · `list`(활성 패널의 보이는 행) · `tabs` · `status` · `menu` · `all`.
    pub(crate) fn dump_of(&mut self, target: &str) -> Option<String> {
        // 예약된 도크 갱신(T-177)을 먼저 흘려보낸다 — 덤프는 늘 최신 도크를 본다(시험 · T4 결정성).
        if self.docks_due.is_some() {
            self.update_docks();
        }
        Some(match target {
            "layout" => self.layout_dump(),
            "log" => self.log_dump(),
            "mem" => self.mem_dump(),
            "panel" => self.panel_dump(self.active),
            "panel0" => self.panel_dump(0),
            "panel1" => self.panel_dump(1),
            "list" => self.list_dump(self.active),
            "tabs" => {
                let p = &self.panels[self.active];
                let (paths, active) = p.session();
                let mut out = format!("tabs {} active {}\n", paths.len(), active);
                for (i, t) in paths.iter().enumerate() {
                    out.push_str(&format!("tab{i} {}\n", t.display()));
                }
                out
            }
            "status" => format!(
                "left {}\nright {}\n",
                self.statusbar.left(),
                self.statusbar.right()
            ),
            "prefs" => format!(
                "open {} keys {:?}\nplugins {:?}\n",
                self.prefs_win.is_open(),
                self.prefs_win.shown_keys(),
                self.prefs_win.plugin_states()
            ),
            "term" => self.term_dump(),
            "check" => self.check_win.table(),
            "license" => self.license_dump(),
            "archive" => self.archive_win.dump(),
            "progress" => self.progress_win.dump(),
            "bulk" => self.bulk_win.dump(),
            "order" => self.order_win.dump(),
            "ops" => self.ops_dump(),
            "dlg" => self.dlg_dump(),
            "pvwin" => self.preview_win.dump(),
            "ctx" => self.ctx_dump(),
            "launcher" => {
                format!(
                    "last {}\nicons {}\n{}\n",
                    self.launcher_last,
                    self.launcher_icon_summary().join(" "),
                    self.launcher_items
                        .iter()
                        .map(launcher::LauncherItem::encode)
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
            "preview" => {
                let (id, lines) = &self.dock_preview[self.term_dock_index()];
                format!("provider {id}\n{}\n", lines.join("\n"))
            }
            "dock" => {
                let d = &self.docks[0];
                format!(
                    "dock0 kind {} rect {},{} {}x{}\n",
                    d.active_kind(),
                    d.bounds().x,
                    d.bounds().y,
                    d.bounds().w,
                    d.bounds().h
                )
            }
            "menu" => match self.menubar.open_index() {
                Some(i) => format!("open {i}\n"),
                None => "closed\n".to_string(),
            },
            "all" => {
                let mut out = self.layout_dump();
                out.push_str(&self.panel_dump(0));
                out.push_str(&self.panel_dump(1));
                out.push_str(&self.list_dump(self.active));
                out
            }
            // ── 10-08 Linux 실기 자동화(T4 · docs/18 §4): 10-05~10-08 신규 기능 창의 상태 덤프 — 화면 판정 대신 시나리오가 본다.
            // 명령 팔레트: 열림 · 입력 · 결과(items 순서 id) · 선택 행.
            "palette" => {
                let items = self.palette_items();
                let ids: Vec<&str> = self
                    .palette
                    .matches()
                    .iter()
                    .filter_map(|&k| items.get(k).map(|it| it.id.as_str()))
                    .collect();
                format!(
                    "open {}\nquery {}\nmatches {}\nselected {}\nids {}\n",
                    self.palette.is_open(),
                    self.palette.query(),
                    ids.len(),
                    self.palette.selected(),
                    ids.join(" ")
                )
            }
            // 즐겨찾기: 목록 + 현재 폴더 포함 여부.
            "favs" => {
                let list = self.favorites();
                let mut out = format!(
                    "count {}\ncurrent {:?}\n",
                    list.len(),
                    self.fav_has_current()
                );
                for p in &list {
                    out.push_str(&format!("fav {}\n", p.display()));
                }
                out
            }
            // 중복 파일 찾기 창: 열림 · 진행 중 · 행 수 · 상태 줄 · 표시된 경로.
            "dupes" => {
                let mut out = format!(
                    "open {}\nrunning {}\nrows {}\nstatus {}\n",
                    self.dupes_win.is_open(),
                    self.dupes_win.is_running(),
                    self.dupes_win.rows_len(),
                    self.dupes_win.status()
                );
                for p in self.dupes_win.marked_paths() {
                    out.push_str(&format!("marked {}\n", p.display()));
                }
                out
            }
            // 폴더 비교 창: 열림 · 진행 중 · 행 수 · 상태 줄 · 항목(`rel verdict`).
            "compare" => {
                let mut out = format!(
                    "open {}\nrunning {}\nrows {}\nstatus {}\n",
                    self.compare_win.is_open(),
                    self.compare_win.is_running(),
                    self.compare_win.rows_len(),
                    self.compare_win.status()
                );
                for e in self.compare_win.entries().unwrap_or_default() {
                    out.push_str(&format!("entry {} {:?}\n", e.rel, e.verdict));
                }
                out
            }
            // 체크섬 창: 열림 · 진행/대기 · 파일 수 · 결과 글.
            "hash" => format!(
                "open {}\nrunning {}\nwaiting {}\nfiles {}\n{}\n",
                self.hash_win.is_open(),
                self.hash_win.is_running(),
                self.hash_win.is_waiting(),
                self.hash_win.files().len(),
                self.hash_win.results_text().replace("\r\n", "\n")
            ),
            // 압축 풀기 · 동기화 작업: 진행 중 여부.
            "jobs" => format!(
                "extract {}\nsync {}\ntransfer {}\n",
                self.extract_job.is_some(),
                self.sync_job.is_some(),
                self.transfer.is_some()
            ),
            // 탭 상태바의 Git 칸: 저장소 · 브랜치 · 요약 덧글(`git.wait` 뒤에 읽는다).
            "tabstatus" => {
                let p = &self.panels[self.active];
                match p.git_info() {
                    Some((repo, branch)) => format!(
                        "repo {}\nbranch {}\nextra {}\n",
                        repo.display(),
                        branch,
                        p.git_extra()
                    ),
                    None => "repo none\n".to_string(),
                }
            }
            // 정보 도크 글(폴더 크기 · 파일 정보 — 시험이 쓰는 `select_all_text` 경로와 같다).
            "docktext" => {
                // 예약 여부와 무관하게 도크를 지금 상태로 맞춘다(폴더 크기 결과가 표시 주기(250 ms)를 기다리는 중일 수 있다).
                self.update_docks();
                let mut inv = Invalidations::default();
                self.docks[0].select_all_text(&mut inv);
                self.docks[0].selected_text().unwrap_or_default()
            }
            _ => return None,
        })
    }

    /// 패널 덤프 — 경로 · 탭 · 선택 · 캐럿 · 보기 모드 · 정렬 · 뒤로/앞으로 가능.
    fn panel_dump(&self, i: usize) -> String {
        let p = &self.panels[i];
        let rows = p.rows();
        let sort: Vec<String> = rows
            .sort()
            .iter()
            .map(|(k, d)| format!("{k}{}", if *d { "-" } else { "+" }))
            .collect();
        format!(
            "panel{i} path {} tabs {} active {} rows {} selected {} caret {:?} mode {:?} sort [{}] back {} forward {} editing {}\n",
            p.root_path().display(),
            p.tab_count(),
            p.active_index(),
            rows.source().len(),
            rows.source().selection_count(),
            rows.caret(),
            rows.view_mode(),
            sort.join(","),
            p.can_back(),
            p.can_forward(),
            p.pathbar.is_editing()
        )
    }

    /// 목록 덤프 — 보이는 행(인덱스 · 깊이 · 마커 · 선택 · 이름 · 셀).
    fn list_dump(&self, i: usize) -> String {
        let rows = self.panels[i].rows();
        let src = rows.source();
        let (first, count) = rows.viewport();
        let mut out = format!(
            "list panel{i} rows {} viewport {first}+{count}\n",
            src.len()
        );
        // 열기 실패(0행의 원인 구분 — 10-03 copy-paste 흔들림): 오류 문구를 한 줄로.
        if let Some(e) = src.error() {
            out.push_str(&format!("error {e}\n"));
        }
        for r in first..(first + count).min(src.len()) {
            let it = src.row(r);
            let cells: Vec<String> = rows
                .columns()
                .iter()
                .skip(1)
                .map(|c| src.cell(r, c.key))
                .collect();
            out.push_str(&format!(
                "{r} d{} {:?} {}{} | {}\n",
                it.depth,
                it.marker,
                if src.is_selected(r) { "*" } else { "" },
                it.text,
                cells.join(" | ")
            ));
        }
        out
    }

    /// 배치·상태 덤프(한 줄 = 한 영역 · 골든 비교 · 하네스 T3). 패널은 dir2 수직 스택(탭 · 네비 · 경로 · 목록)을 그대로 적는다.
    pub(crate) fn layout_dump(&self) -> String {
        let r = |r: Rect| format!("{},{} {}x{}", r.x, r.y, r.w, r.h);
        let size = self.viewport;
        let mut out = String::new();
        out.push_str(&format!(
            "window {}x{} scale {:.2} theme {} dual {} active {}\n",
            size.0,
            size.1,
            self.scale,
            if self.theme.is_dark { "dark" } else { "light" },
            self.dual,
            self.active
        ));
        out.push_str(&format!("menubar {}\n", r(self.menubar.bounds())));
        out.push_str(&format!("toolbar {}\n", r(self.toolbar.bounds())));
        out.push_str(&format!(
            "launcher {} items {}\n",
            r(self.launcherbar.bounds()),
            self.launcher_items.len()
        ));
        for (i, p) in self.panels.iter().enumerate() {
            out.push_str(&format!(
                "panel{i} {} tabs {} active {} path {}\n",
                r(p.bounds()),
                p.tab_count(),
                p.active_index(),
                p.root_path().display()
            ));
            out.push_str(&format!("panel{i}.tabs {}\n", r(p.tabbar.bounds())));
            out.push_str(&format!("panel{i}.nav {}\n", r(p.nav_rect())));
            out.push_str(&format!(
                "panel{i}.path {} text {}\n",
                r(p.pathbar.bounds()),
                p.pathbar.path()
            ));
            out.push_str(&format!(
                "panel{i}.list {} rows {} caret {:?}\n",
                r(p.rows().bounds()),
                p.rows().source().len(),
                p.rows().caret()
            ));
            out.push_str(&format!(
                "panel{i}.status {} {}\n",
                r(p.status_bounds()),
                p.status_summary().join(" | ")
            ));
        }
        for (i, d) in self.docks.iter().enumerate() {
            out.push_str(&format!(
                "dock{i} {} kind {}\n",
                r(d.bounds()),
                d.active_kind()
            ));
        }
        // 보조 창 열림(10-08 Linux 실기 자동화 — 로그 창 · 메모리 창 토글 판정).
        out.push_str(&format!(
            "log {} mem {}\n",
            self.log_win.is_open() || self.open_log,
            self.mem_win.is_open()
        ));
        out.push_str(&format!("splitter {}\n", r(self.splitter.rect())));
        out.push_str(&format!("dsplit_h {}\n", r(self.dock_split_h.rect())));
        out.push_str(&format!("dsplit_v {}\n", r(self.dock_split_v.rect())));
        out.push_str(&format!(
            "statusbar {} left {} right {}\n",
            r(self.statusbar.bounds()),
            self.statusbar.left(),
            self.statusbar.right()
        ));
        out
    }

    /// `@ready` 큐 — 첫 프레임 뒤(시험은 직접 호출). 한 번만.
    pub(crate) fn fire_ready(&mut self) {
        if self.ready_fired {
            return;
        }
        self.ready_fired = true;
        // 기동 단계 로그(T-92 · NEW-001 — 종전 `status.firstRender`는 lang 키만 남고 코드가 없었다).
        let up = self.started.elapsed();
        self.log_with(
            ndir_log::LogKind::Startup,
            tr("log.msg.ready"),
            None,
            Some(up),
        );
        let cmds = std::mem::take(&mut self.startup_ready);
        self.run_ready(cmds);
    }

    /// `@ready` 명령 순차 실행 — `ctx.wait`를 만나면 열린 메뉴의 셸 항목이 채워질 때까지(비동기 · dir2 X-61) **나머지를 보류**한다
    /// (고정 ms 대기 없는 결정적 하네스 · [`Self::resume_blocked`]가 이어 돌린다). 기다릴 것이 없으면 그냥 지나간다.
    fn run_ready(&mut self, cmds: Vec<String>) {
        let mut it = cmds.into_iter();
        while let Some(id) = it.next() {
            // 결정적 대기: `ctx.wait` = 우클릭 메뉴의 셸 항목 도착까지 · `ops.wait` = 진행 중인 전송이 끝날 때까지(끝나면 목록도 갱신돼 있다).
            if is_wait_cmd(&id) {
                // 예약된 도크 갱신(T-177 디바운스)을 먼저 — 폴더 크기 요청이 도크 갱신에서 나오므로 `dirsize.wait`가 요청 전에 지나치지 않게.
                if self.docks_due.is_some() {
                    self.update_docks();
                }
                if self.startup_wait_busy(&id) {
                    self.startup_blocked = std::iter::once(id).chain(it).collect();
                    self.startup_blocked_since = Instant::now();
                    return;
                }
                continue;
            }
            self.startup_cmd(&id);
        }
    }

    /// 보류된 `@ready` 명령 재개 — 셸 항목이 도착했거나(대기 해제) 20 s가 지났으면(응답 없는 셸 확장 · 하네스가 멈추지 않게).
    pub(crate) fn resume_blocked(&mut self, now: Instant) {
        if self.startup_blocked.is_empty() {
            return;
        }
        let timed_out = now.duration_since(self.startup_blocked_since) > Duration::from_secs(20);
        let busy = self
            .startup_blocked
            .first()
            .cloned()
            .is_some_and(|id| self.startup_wait_busy(&id));
        if timed_out {
            // 응답 없는 대기는 풀고 진행(하네스가 멈추지 않게) — 대기 명령 자체는 건너뛴다.
            self.ctx_pending = None;
            self.ctx_wait = None;
            let mut cmds = std::mem::take(&mut self.startup_blocked);
            if !cmds.is_empty() {
                cmds.remove(0);
            }
            self.run_ready(cmds);
        } else if !busy {
            let cmds = std::mem::take(&mut self.startup_blocked);
            self.run_ready(cmds);
        }
    }

    /// 대기 명령이 아직 기다려야 하는가.
    fn startup_wait_busy(&self, id: &str) -> bool {
        match id {
            "ctx.wait" => self.ctx_pending.is_some() || self.ctx_wait.is_some(),
            "ops.wait" => self.transfer.is_some(),
            // `xdnd.drop` 뒤: 소스 스레드가 끝나고(XdndFinished) 모인 드롭이 틱에서 처리돼 전송이 시작될 때까지(1초 유예).
            // 수신(주입) · 발신(세션) 양쪽의 결정적 대기: 주입 스레드 끝 + 드롭 처리 + 1초 유예 / 세션 = Status(받음)까지 · 놓은 뒤 Finished까지.
            "xdnd.wait" => {
                let inject = match &self.xdnd_job {
                    None => false,
                    Some(j) if !j.is_finished() => true,
                    Some(_) => {
                        self.dnd_hovering()
                            || !self.dnd_drop.is_empty()
                            || (self.transfer.is_none()
                                && self.xdnd_started.elapsed() < Duration::from_millis(1000))
                    }
                };
                inject || self.xdnd.as_ref().is_some_and(|s| s.waiting())
            }
            // 10-08 Linux 실기 자동화: 신규 기능의 워커가 끝날 때까지(중복 찾기 · 폴더 비교 · 체크섬 · 압축 풀기/동기화 · 폴더 크기).
            "dupes.wait" => self.dup_job.is_some() || self.dupes_win.is_running(),
            "compare.wait" => {
                self.cmp_job.is_some() || self.compare_win.is_running() || self.sync_job.is_some()
            }
            "hash.wait" => {
                self.hash_job.is_some() || self.hash_win.is_running() || self.hash_win.is_waiting()
            }
            "jobs.wait" => {
                self.extract_job.is_some() || self.sync_job.is_some() || self.transfer.is_some()
            }
            "dirsize.wait" => self.dirsizes.running() || self.dirsizes.display_pending(),
            "git.wait" => !self.git_busy.is_empty(),
            _ => false,
        }
    }

    /// `NDIR_STARTUP_CMD` 해석 — 즉시/`@ready`/`@idle`/`@after` 분류(순수 판정은 [`classify_startup`]).
    pub(crate) fn queue_startup(&mut self, cmds: &str) {
        for id in cmds.split(',').map(str::trim).filter(|c| !c.is_empty()) {
            match classify_startup(id) {
                StartupWhen::Now(c) => self.startup_cmd(c),
                StartupWhen::Ready(c) => self.startup_ready.push(c.to_string()),
                StartupWhen::After(ms, c) => self
                    .startup_timed
                    .push((Instant::now() + Duration::from_millis(ms), c.to_string())),
            }
        }
    }
}

/// 기동 명령의 실행 시점.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StartupWhen<'a> {
    Now(&'a str),
    Ready(&'a str),
    After(u64, &'a str),
}

/// `@ready:` · `@idle:`(= ready · 작업 큐는 M6) · `@after:<ms>:` 접두 판정(순수).
pub(crate) fn classify_startup(id: &str) -> StartupWhen<'_> {
    if let Some(c) = id
        .strip_prefix("@ready:")
        .or_else(|| id.strip_prefix("@idle:"))
    {
        return StartupWhen::Ready(c);
    }
    if let Some(rest) = id.strip_prefix("@after:") {
        if let Some((ms, c)) = rest.split_once(':') {
            return StartupWhen::After(ms.trim().parse().unwrap_or(0), c);
        }
    }
    StartupWhen::Now(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_prefixes() {
        assert_eq!(classify_startup("nav.up"), StartupWhen::Now("nav.up"));
        assert_eq!(classify_startup("@ready:quit"), StartupWhen::Ready("quit"));
        assert_eq!(
            classify_startup("@idle:quit:2"),
            StartupWhen::Ready("quit:2")
        );
        assert_eq!(
            classify_startup("@after:250:layout.dump:x"),
            StartupWhen::After(250, "layout.dump:x")
        );
        assert_eq!(
            classify_startup("@after:bad"),
            StartupWhen::Now("@after:bad")
        );
    }
}
