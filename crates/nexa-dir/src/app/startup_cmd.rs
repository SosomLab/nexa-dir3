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

impl App {
    pub(crate) fn startup_cmd(&mut self, id: &str) {
        crash::note_command(id);
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
            self.prefs_win.refresh(&self.settings);
            self.prefs_win.preset_query(q);
            return;
        }
        if let Some(cat) = id.strip_prefix("prefs.cat:") {
            self.open_prefs = true;
            self.prefs_win.refresh(&self.settings);
            self.prefs_win.select_category(cat);
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
            .filter(|(k, _)| matches!(*k, "move" | "click" | "dclick" | "rclick" | "wheel"))
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

    /// 영역 이름 → 사각형(덤프의 줄 이름과 같다).
    pub(crate) fn area_rect(&self, name: &str) -> Option<Rect> {
        let panel = |i: usize, part: &str| -> Option<Rect> {
            let p = &self.panels[i];
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
    pub(crate) fn dump_of(&self, target: &str) -> Option<String> {
        Some(match target {
            "layout" => self.layout_dump(),
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
                "open {} keys {:?}\n",
                self.prefs_win.is_open(),
                self.prefs_win.shown_keys()
            ),
            "term" => self.term_dump(),
            "check" => self.check_win.table(),
            "license" => self.license_dump(),
            "archive" => self.archive_win.dump(),
            "ops" => self.ops_dump(),
            "dlg" => self.dlg_dump(),
            "pvwin" => self.preview_win.dump(),
            "ctx" => self.ctx_dump(),
            "launcher" => {
                format!(
                    "last {}\n{}\n",
                    self.launcher_last,
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
        }
        for (i, d) in self.docks.iter().enumerate() {
            out.push_str(&format!(
                "dock{i} {} kind {}\n",
                r(d.bounds()),
                d.active_kind()
            ));
        }
        out.push_str(&format!("splitter {}\n", r(self.splitter.rect())));
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
        for id in std::mem::take(&mut self.startup_ready) {
            self.startup_cmd(&id);
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
