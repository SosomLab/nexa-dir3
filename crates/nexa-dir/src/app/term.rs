//! 도크 터미널 호스트 배선(T-61 · dir2 `win.rs` 터미널 부분): 셸 선택 · 팔레트 · 지연 시작(paint) · 폴링 틱 · 키/마우스 라우팅 ·
//! → 폴더로 이동(cd) · 복사/붙여넣기/전체 선택 · 덤프. 뷰 자체는 `termview.rs`.

use crate::platform::ShellSpec;
use crate::termview::{TermStyle, TermView, POLL_MS};
use crate::*;

/// 설정 `term.copy_format` → `(HTML도 게시, RTF도 게시)`(순수 · dir2 win.rs:7413-7414): text = 평문만 · html · rtf · both.
pub(crate) fn copy_formats(value: &str) -> (bool, bool) {
    match value {
        "html" => (true, false),
        "rtf" => (false, true),
        "both" => (true, true),
        _ => (false, false),
    }
}

impl App {
    /// 설정 `term.shell`(빈 값 = 자동 탐지 · 아니면 후보 중 파일명/라벨 일치 · 없으면 그 경로 그대로).
    pub(crate) fn term_shell(&self) -> Option<ShellSpec> {
        let want = self
            .settings
            .get("term.shell")
            .unwrap_or("")
            .trim()
            .to_string();
        let cands = self.platform.shell.candidates();
        if want.is_empty() {
            return cands.into_iter().next();
        }
        let lw = want.to_ascii_lowercase();
        if let Some(c) = cands.iter().find(|c| {
            c.label.to_ascii_lowercase() == lw
                || c.program
                    .file_stem()
                    .is_some_and(|s| s.to_string_lossy().to_ascii_lowercase() == lw)
                || c.program.to_string_lossy().to_ascii_lowercase() == lw
        }) {
            return Some(c.clone());
        }
        let p = PathBuf::from(&want);
        if p.is_file() {
            return Some(ShellSpec {
                program: p,
                args: vec![],
                label: want,
            });
        }
        // 모르는 값(다른 OS의 설정 · 지워진 셸) = 기본 셸로 폴백 — 낡은 설정이 터미널을 죽이지 않는다.
        cands.into_iter().next()
    }

    /// 터미널 팔레트(설정 `term.theme`/`theme_dark`/`theme_light` + 앱 테마 — dir2 09-04 규칙).
    pub(crate) fn term_palette(&self) -> ndir_term::TermPalette {
        ndir_term::resolve_scheme(
            self.settings.get("term.theme").unwrap_or("system"),
            self.settings.get("term.theme_dark").unwrap_or(""),
            self.settings.get("term.theme_light").unwrap_or(""),
            self.theme.is_dark,
        )
        .palette
    }

    /// 표시 설정(dir2 X-3): `term.font_size`(Mono 슬롯 = 상태줄 크기 기준 증분) · `term.wrap` · `term.cols`.
    pub(crate) fn term_style(&self) -> TermStyle {
        let base = self.font_px("statusbar.font_size");
        // Windows Terminal 따라가기: 글꼴 크기 pt → DIP(×96/72) → **그 고정폭 글꼴의** px. nexa-ctl의 Mono 슬롯은 숫자 높이를
        // 본문 글꼴에 맞추는 광학 보정(0.75~1.15배)을 곱하므로 그만큼 나눠 실제 크기가 Windows Terminal과 같게 한다.
        // ★ 기본 터미널을 따르지 않을 때(`term.follow_windows_terminal` 꺼짐 · 프로필 없음)도 **같은 계산**을 쓴다(사용자 10-05
        //   "리눅스 기준으로 · 터미널 글꼴이 너무 크다"): 종전에는 그 경우만 본문 글꼴 지표로 환산한 크기에 Mono 슬롯의 광학 보정
        //   (최대 1.15배)이 그대로 곱해지고 줄 높이도 "글자 높이 + 3"이라, 같은 `term.font_size`가 Linux(프로필에 크기 없음 →
        //   이 계산)보다 크게 나왔다.
        let wt = self.mono_font.as_ref().map(|mono| {
            {
                // 크기: 따르는 프로필이 주면 그 pt → DIP · 그 밖(Linux · 따르지 않음)은 설정 `term.font_size`(em) — 어느 쪽이든
                // **그 고정폭 글꼴의** 지표로 px를 구한다(본문 글꼴 지표로 환산하면 글꼴마다 실제 크기가 달라진다).
                let em = self
                    .wt_profile
                    .as_ref()
                    .and_then(|wt| wt.size_pt)
                    .map_or_else(
                        || self.settings.font_px("term.font_size"),
                        |pt| pt * 96.0 / 72.0,
                    );
                let target = mono.em_to_px(em);
                let mult =
                    (self.ui_font.digit_height(100.0) / mono.digit_height(100.0)).clamp(0.75, 1.15);
                // 줄 높이 = 그 글꼴의 어센트+디센트+줄 간격(Windows Terminal의 칸 높이 · 배율 반영) — 종전 "글자 높이 + 3"은
                // 본문 글꼴 상자 높이 기준이라 줄 간격이 더 벌어졌다(10-03 캡처: 24.5 px ↔ WT 약 19~20 px).
                let cell_h = (mono.line_height(target) * self.scale).ceil() as i32;
                (target / mult, cell_h)
            }
        });
        let want = wt.map_or_else(|| self.font_px("term.font_size"), |w| w.0);
        TermStyle {
            cell_h: wt.map(|w| w.1),
            font_delta: if want > 0.0 && base > 0.0 {
                want - base
            } else {
                0.0
            },
            wrap: self.settings.flag("term.wrap"),
            cols: self.settings.int("term.cols").clamp(80, 1000) as usize,
        }
    }

    /// 도크 i의 내용 원천 패널(단일 정보 = 활성).
    pub(crate) fn dock_source(&self, i: usize) -> usize {
        let single =
            !self.dual || self.settings.get("layout.info_mode").unwrap_or("dual") != "dual";
        if single {
            self.active
        } else {
            i
        }
    }

    /// 터미널 cwd = 원천 패널 폴더(가상 최상위·없는 경로면 홈).
    pub(crate) fn term_cwd(&self, i: usize) -> PathBuf {
        let root = self.panels[self.dock_source(i)].root_path();
        if root.is_dir() {
            root
        } else {
            std::env::home_dir().unwrap_or_else(|| PathBuf::from("."))
        }
    }

    /// 기동 명령·메뉴가 다루는 도크(포커스 도크 · 없으면 활성 패널 쪽 · 단일 정보면 0).
    pub(crate) fn term_dock_index(&self) -> usize {
        if let Some(i) = self.term_focus {
            return i;
        }
        if self.docks[1].bounds().h > 0 && self.active == 1 {
            1
        } else {
            0
        }
    }

    /// 기동 명령(`term.send`/`term.focus`)이 paint보다 먼저 올 때 — 기본 80×24로 즉시 시작(다음 paint가 격자에 맞춰 리사이즈).
    pub(crate) fn term_ensure_started(&mut self, i: usize) {
        if self.terms[i].started() || self.terms[i].failed {
            return;
        }
        let shell = self.term_shell();
        let cwd = self.term_cwd(i);
        self.terms[i].start(&self.platform, shell, &cwd, 80, 24);
    }

    /// 도크 i가 보이고 터미널 종류인가.
    pub(crate) fn term_shown(&self, i: usize) -> bool {
        self.docks[i].bounds().h > 0 && self.docks[i].active_kind() == 2
    }

    pub(crate) fn term_focused(&self) -> Option<usize> {
        self.term_focus.filter(|&i| self.term_shown(i))
    }

    pub(crate) fn set_term_focus(&mut self, f: Option<usize>, inv: &mut Invalidations) {
        if self.term_focus != f {
            self.term_focus = f;
            for i in 0..2 {
                self.docks[i].set_focused(f == Some(i), inv);
            }
            if let Some(i) = f {
                let now = self.started.elapsed().as_millis() as u64;
                self.terms[i].caret_reset(now);
            }
        }
    }

    /// 폴링 틱: 출력 수거 · 종료 감지 · 캐럿 깜빡임. 반환 = 살아 있는(보이는) 터미널이 있어 계속 깨야 하는가.
    pub(crate) fn term_tick(&mut self, now_ms: u64) -> bool {
        let mut live = false;
        let mut changed = false;
        for i in 0..2 {
            if !self.terms[i].started() {
                continue;
            }
            if self.terms[i].pump() {
                changed = true;
            }
            let shown = self.term_shown(i);
            if self.terms[i].alive() && shown {
                live = true;
            }
            if self.term_focus == Some(i) && shown && self.terms[i].blink(now_ms) {
                changed = true;
            }
        }
        if changed {
            self.redraw();
        }
        live
    }

    /// 폴링 간격 — 터미널이 살아 있거나 포커스 깜빡임 중이면 `POLL_MS`.
    pub(crate) fn term_wake(&self, live: bool) -> Option<Duration> {
        // 시간 예산으로 끊긴 출력이 남아 있으면 쉬지 않고 이어서(사이사이 입력 사건이 처리된다).
        if self.terms.iter().any(|t| t.backlog) {
            return Some(Duration::from_millis(1));
        }
        (live || self.term_focused().is_some()).then(|| Duration::from_millis(POLL_MS))
    }

    /// 지연 시작·그리기(paint 길목 · dir2 `term_paint`): 도크 종류 2이고 세션이 없으면 cwd로 연다.
    pub(crate) fn paint_terms(&mut self, dc: &mut dyn DrawCtx, th: &Theme, row_h: i32) {
        let pal = self.term_palette();
        let style = self.term_style();
        for i in 0..2 {
            if !self.term_shown(i) {
                continue;
            }
            let rc = self.docks[i].content_rect();
            let (cols, rows, _, _) = TermView::grid_dims(dc, rc, row_h, &style);
            if cols >= 2 && rows >= 2 && !self.terms[i].started() && !self.terms[i].failed {
                let shell = self.term_shell();
                let cwd = self.term_cwd(i);
                self.terms[i].start(&self.platform, shell, &cwd, cols, rows);
            }
            let caret = self.term_focus == Some(i) && self.terms[i].caret_on;
            self.terms[i].paint(dc, rc, th, &pal, caret, row_h, &style);
        }
    }
    /// 좌표가 어느 도크의 터미널 격자 위인가.
    pub(crate) fn term_hit_at(&self, x: i32, y: i32) -> Option<usize> {
        (0..2)
            .find(|&i| self.term_shown(i) && self.docks[i].content_rect().contains(Point { x, y }))
    }

    /// 터미널 포커스 중 키 — 처리했으면 true(목록 단축키 차단 · dir2 KeyRoute::Term).
    pub(crate) fn term_key(&mut self, ev: &InputEvent, inv: &mut Invalidations) -> bool {
        let Some(i) = self.term_focused() else {
            if self.term_focus.is_some() {
                self.set_term_focus(None, inv); // 낡은 포커스(도크 숨김·종류 전환) 해제
            }
            return false;
        };
        let now = self.started.elapsed().as_millis() as u64;
        let t = &mut self.terms[i];
        if t.exited || t.failed {
            t.reset(); // 아무 키 = 재시작(다음 paint)
            inv.push(self.docks[i].bounds());
            return true;
        }
        match *ev {
            InputEvent::Key { key, .. } => {
                if let Some(s) = TermView::key_seq(key) {
                    t.write(s);
                }
            }
            InputEvent::Char { c, .. } => t.write(&TermView::char_seq(c)),
            InputEvent::SelectAll => {
                t.select_all();
            }
            _ => return false,
        }
        t.view_off = 0;
        t.caret_reset(now);
        inv.push(self.docks[i].bounds());
        true
    }

    /// 수식키 글자(event_loop에서 키맵보다 먼저): `cmd` = ⌘(mac 복사/붙여넣기) · 그 외 Ctrl+글자 = 제어 문자
    /// (Ctrl+Shift+C/V = 복사/붙여넣기 · Ctrl+C는 선택이 있으면 복사 — WT 규약). 처리했으면 true.
    pub(crate) fn term_ctrl(&mut self, letter: char, shift: bool, cmd: bool) -> bool {
        let Some(i) = self.term_focused() else {
            return false;
        };
        let l = letter.to_ascii_lowercase();
        if cmd || (shift && matches!(l, 'c' | 'v')) {
            return match l {
                'c' => self.term_copy(),
                'v' => self.term_paste(),
                'a' => self.term_select_all(),
                _ => cmd,
            };
        }
        if l == 'c' && self.terms[i].sel.is_some() {
            return self.term_copy();
        }
        if !l.is_ascii_lowercase() {
            return false;
        }
        let byte = (l as u8) & 0x1f;
        let t = &mut self.terms[i];
        if t.exited || t.failed {
            t.reset();
        } else {
            let mut b = [0u8; 1];
            b[0] = byte;
            t.write(std::str::from_utf8(&b).unwrap_or(""));
            t.view_off = 0;
        }
        self.redraw();
        true
    }

    /// → 버튼(dir2 QA 07-14 '폴더로 이동'): 살아 있으면 `cd` · 아니면 cwd로 재시작 · 포커스.
    pub(crate) fn term_goto(&mut self, i: usize, inv: &mut Invalidations) {
        let dir = self.term_cwd(i);
        let t = &mut self.terms[i];
        if t.alive() {
            let flag = if t.shell_label == "Command Prompt" {
                "/d "
            } else {
                ""
            };
            t.write(&format!("cd {flag}\"{}\"\r", dir.display()));
            t.view_off = 0;
        } else {
            t.reset();
        }
        self.set_term_focus(Some(i), inv);
        inv.push(self.docks[i].bounds());
    }

    pub(crate) fn term_copy(&mut self) -> bool {
        let Some(i) = self.term_focused() else {
            return false;
        };
        let Some(text) = self.terms[i].selected_text() else {
            return false;
        };
        // 복사 서식(dir2 X-50 `term.copy_format`): html = 평문 + HTML · rtf = 평문 + RTF · both = 셋 다(색·글꼴 = 현재 팔레트/설정).
        let (want_html, want_rtf) =
            copy_formats(self.settings.get("term.copy_format").unwrap_or("text"));
        let ok = if want_html || want_rtf {
            let pal = self.term_palette();
            let font = self
                .settings
                .get("term.font_face")
                .unwrap_or("Consolas")
                .split(',')
                .next()
                .map(str::trim)
                .filter(|f| !f.is_empty())
                .unwrap_or("Consolas")
                .to_string();
            let px = self.settings.font_px("term.font_size").round() as i32;
            match self.terms[i].selected_runs() {
                Some(runs) => {
                    let px = px.max(8);
                    let html =
                        want_html.then(|| ndir_term::export::to_html(&runs, &pal, &font, px));
                    let rtf = want_rtf.then(|| ndir_term::export::to_rtf(&runs, &pal, &font, px));
                    clipboard::write_rich(&text, html.as_deref(), rtf.as_deref())
                        || clipboard::write_text(&text)
                }
                None => clipboard::write_text(&text),
            }
        } else {
            clipboard::write_text(&text)
        };
        self.terms[i].sel = None;
        self.redraw();
        ok
    }
    pub(crate) fn term_paste(&mut self) -> bool {
        let Some(i) = self.term_focused() else {
            return false;
        };
        let Some(txt) = clipboard::read_text() else {
            return false;
        };
        let t = &mut self.terms[i];
        t.write(&txt.replace("\r\n", "\r").replace('\n', "\r"));
        t.view_off = 0;
        true
    }

    pub(crate) fn term_select_all(&mut self) -> bool {
        let Some(i) = self.term_focused() else {
            return false;
        };
        let ok = self.terms[i].select_all();
        self.redraw();
        ok
    }

    /// 덤프(`term.dump` · `assert.term:`): 상태 줄 + 화면 텍스트. 대상 = 포커스 도크 · 없으면 활성 패널 쪽.
    pub(crate) fn term_dump(&self) -> String {
        let i = self.term_focus.unwrap_or(self.active.min(1));
        let t = &self.terms[i];
        let st = self.term_style();
        format!(
            "dock{i} {} focus {:?} cwd {} wrap {} cols {}\n{}",
            t.state_line(),
            self.term_focus,
            t.cwd.display(),
            st.wrap,
            st.cols,
            t.screen_text()
        )
    }
}
