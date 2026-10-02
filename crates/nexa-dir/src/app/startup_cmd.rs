//! App — 기동 명령(`NDIR_STARTUP_CMD` · 자체 시험·하네스 T5 경로 · nexa-sql docs/61 §4 계승).
//!
//! `NDIR_STARTUP_CMD=<명령>,<명령>,…`(쉼표 구분) — 키 주입(SendKeys) 없이 특정 화면·상태를 만든다. 평소엔 변수 없음 = 비용 0.
//! - 명령 id(`view.hidden` · `nav.up` …) = [`App::command`] 한 길
//! - `@after:<ms>:<명령>` = 기동 뒤 그 시간이 지나면
//! - `nav:<경로>` = 활성 패널 이동 · `panel:<0|1>` = 활성 패널
//! - `key:<조합>`(`ctrl+t`) = 키맵 조회 뒤 명령(단축키 표 시험)
//! - `ui.move|click|dclick|rclick|wheel:<x>/<y>[/<delta>]` = 창 좌표(장치 px) 포인터 사건 — OS 입력 주입이 아니다
//! - `layout.dump:<파일>` = 배치·상태 덤프([`App::layout_dump`] · 골든 비교용) · `app.exit` = 종료

use crate::*;

impl App {
    pub(crate) fn startup_cmd(&mut self, id: &str) {
        if let Some(path) = id.strip_prefix("layout.dump:") {
            let _ = std::fs::write(path, self.layout_dump());
            return;
        }
        if let Some(path) = id.strip_prefix("nav:") {
            self.navigate(self.active, std::path::Path::new(path));
            return;
        }
        if let Some(n) = id.strip_prefix("panel:") {
            let i = n.trim().parse::<usize>().unwrap_or(0).min(1);
            self.set_focus(Focus::Panel(i));
            self.redraw();
            return;
        }
        if let Some(code) = id.strip_prefix("key:") {
            if let Some(ch) = Chord::parse(code) {
                if let Some(cmd) = self.keymap.lookup(&ch) {
                    self.command(cmd);
                }
            }
            return;
        }
        if let Some((kind, xy)) = id
            .strip_prefix("ui.")
            .and_then(|r| r.split_once(':'))
            .filter(|(k, _)| matches!(*k, "move" | "click" | "dclick" | "rclick" | "wheel"))
        {
            let mut it = xy
                .split(['/', 'x'])
                .map(|v| v.trim().parse::<i32>().unwrap_or(0));
            let (x, y) = (it.next().unwrap_or(0), it.next().unwrap_or(0));
            let delta = it.next().unwrap_or(120);
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
        match id {
            "app.exit" => self.exit_requested = true,
            _ => self.command(id),
        }
    }

    /// 배치·상태 덤프(한 줄 = 한 영역 · 골든 비교 · 하네스 T3).
    pub(crate) fn layout_dump(&self) -> String {
        let r = |r: Rect| format!("{},{} {}x{}", r.x, r.y, r.w, r.h);
        let size = self
            .window
            .as_ref()
            .map(|w| w.inner_size())
            .map_or((0, 0), |s| (s.width, s.height));
        let mut out = String::new();
        out.push_str(&format!(
            "window {}x{} scale {:.2} theme {} dual {} focus {:?}\n",
            size.0,
            size.1,
            self.scale,
            if self.theme.is_dark { "dark" } else { "light" },
            self.dual,
            self.focus
        ));
        out.push_str(&format!("menubar {}\n", r(self.menubar.bounds())));
        out.push_str(&format!("toolbar {}\n", r(self.toolbar.bounds())));
        out.push_str(&format!(
            "tabs {} active {} n {}\n",
            r(self.tabs.bounds()),
            self.tabs.active(),
            self.tabs.len()
        ));
        out.push_str(&format!(
            "pathbar {} path {}\n",
            r(self.pathbar.bounds()),
            self.pathbar.path()
        ));
        for (i, p) in self.panels.iter().enumerate() {
            out.push_str(&format!(
                "panel{i} {} rows {} caret {:?} path {}\n",
                r(p.bounds()),
                p.source().len(),
                p.caret(),
                p.source().path().display()
            ));
        }
        out.push_str(&format!(
            "statusbar {} left {} right {}\n",
            r(self.statusbar.bounds()),
            self.statusbar.left(),
            self.statusbar.right()
        ));
        out
    }
}
