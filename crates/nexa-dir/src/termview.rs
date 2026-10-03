//! 도크 터미널 뷰(T-61 · dir2 `win.rs` `TermState`/`term_paint`/`term_key_seq` 이식 — 창·GDI 대신 `DrawCtx` · ConPTY 대신 `PtySession` 포트).
//!
//! 수명: 도크 종류가 터미널일 때 호스트가 **지연 시작**(`start` · cwd = 원천 패널 폴더) → `pump`(폴링 틱 · 바이트 → UTF-8 → `VtScreen::feed`)
//! → `paint`(셀 격자 · 선택 반전 · 캐럿 · 종료 안내). 종료 뒤 아무 키 = `reset` → 다음 paint가 재시작(dir2 규약).
//! T-61 B: 글꼴 크기(`term.font_size`) · 줄 바꿈 끄기 = 고정 열(`term.cols`) + 가로 스크롤(`view_x` · Shift+휠) · TUI 마우스 모드(DECSET 1000/1002/1003 + SGR 1006)
//! 좌표 보고(`mouse_report`). 트랙패드 픽셀 스크롤·고속 스크롤은 후속.

use crate::platform::{Platform, PtySession, ShellSpec};
use ndir_i18n::tr;
use ndir_term::{TermPalette, VtScreen};
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::Rect;
use nexa_ctl::theme::{Color, Theme};
use nexa_ctl::Key;
use std::path::{Path, PathBuf};

/// 캐럿 깜빡임 주기(ms · Windows 기본 530).
pub(crate) const CARET_BLINK_MS: u64 = 530;
/// 출력 폴링 간격(ms) — 세션이 살아 있는 동안만 깬다.
pub(crate) const POLL_MS: u64 = 30;
/// 진단: 환경 변수 `NDIR_TERM_TRACE=<파일>`이 있으면 PTY에서 읽은 **원시 바이트**를 그 파일에 덧붙인다(이스케이프 시퀀스 분석용 ·
/// 설정되지 않으면 원자 읽기 한 번뿐 — 파일 I/O 없음).
fn trace_bytes(bytes: &[u8]) {
    use std::io::Write as _;
    static TRACE: std::sync::OnceLock<Option<std::sync::Mutex<std::fs::File>>> =
        std::sync::OnceLock::new();
    let t = TRACE.get_or_init(|| {
        let path = std::env::var_os("NDIR_TERM_TRACE")?;
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
            .map(std::sync::Mutex::new)
    });
    if let Some(f) = t {
        if let Ok(mut f) = f.lock() {
            let _ = f.write_all(bytes);
        }
    }
}

/// 아이콘 글꼴 글리프인가(사용자 영역 U+E000~F8FF · 보충 사용자 영역 U+F0000~) — Nerd Fonts · Terminal-Icons · 프롬프트 테마.
pub(crate) fn is_icon_glyph(c: char) -> bool {
    matches!(c, '\u{E000}'..='\u{F8FF}' | '\u{F0000}'..='\u{10FFFD}')
}

/// 한 칸보다 넓게 그려질 수 있는 기호인가(화살표 · 기술 기호 · 도형 · 기타 기호 · 딩벳 — 프롬프트의 ➜ ✗ 등). 대체 글꼴에서 오면
/// 칸보다 넓어 잘리므로, 뒤가 빈 칸이면 아이콘 글리프처럼 다음 칸까지 넘쳐 그린다(Linux 실기 10-03 "➜가 잘려 ⊣처럼 보임").
pub(crate) fn is_wide_symbol(c: char) -> bool {
    matches!(c, '\u{2190}'..='\u{21FF}' | '\u{2300}'..='\u{23FF}' | '\u{25A0}'..='\u{27BF}')
}

/// 한 번의 펌프가 UI 스레드를 잡는 상한(ms) — 출력이 폭주해도 입력(Ctrl+C)·그리기가 끼어들 수 있게 나눠 읽는다.
const PUMP_BUDGET_MS: u64 = 6;

/// 표시 설정(호스트가 설정에서 만든다 — dir2 X-3 · `term.font_size` · `term.wrap` · `term.cols`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TermStyle {
    /// Mono 슬롯 크기에 더하는 px(설정 `term.font_size` − 슬롯 기본).
    pub font_delta: f32,
    /// 줄 바꿈 = 가시 폭이 열 수 · 아니면 `cols` 고정 + 가로 스크롤.
    pub wrap: bool,
    pub cols: usize,
    /// 줄(칸) 높이 지정(px) — Windows Terminal 따라가기: 그 글꼴의 줄 높이(어센트+디센트+줄 간격). `None` = 종전 규칙.
    pub cell_h: Option<i32>,
}

impl Default for TermStyle {
    fn default() -> Self {
        TermStyle {
            cell_h: None,
            font_delta: 0.0,
            wrap: true,
            cols: 240,
        }
    }
}

/// 읽기 바이트의 UTF-8 디코더 — **순수 타입**(dir2 `Utf8Chunker` · 점검 2차 G9-01).
/// 멀티바이트가 읽기 경계에서 잘린 꼬리(≤3바이트)는 보존해 다음 호출과 합치고, 확정 불량 바이트는 U+FFFD로 치환한다.
#[derive(Debug, Default)]
pub(crate) struct Utf8Chunker {
    pending: Vec<u8>,
}

impl Utf8Chunker {
    pub(crate) fn push(&mut self, bytes: &[u8]) -> Option<String> {
        self.pending.extend_from_slice(bytes);
        let mut out = String::new();
        let mut consumed = 0usize;
        loop {
            match std::str::from_utf8(&self.pending[consumed..]) {
                Ok(s) => {
                    out.push_str(s);
                    consumed = self.pending.len();
                    break;
                }
                Err(e) => {
                    let valid = e.valid_up_to();
                    out.push_str(
                        std::str::from_utf8(&self.pending[consumed..consumed + valid])
                            .unwrap_or(""),
                    );
                    consumed += valid;
                    match e.error_len() {
                        None => break,
                        Some(n) => {
                            out.push('\u{FFFD}');
                            consumed += n;
                        }
                    }
                }
            }
        }
        self.pending.drain(..consumed);
        (!out.is_empty()).then_some(out)
    }
}

type Sel = ((usize, usize), (usize, usize));

pub(crate) struct TermView {
    /// 휠 누적기(dir2 eb29089 · 7d8b1e9): 세로 스크롤백 · 가로 열 · TUI 마우스 보고 — 트랙패드의 작은 delta를 잃지 않고
    /// 노치(120) 단위로 환산한다.
    pub(crate) wheel: nexa_ctl::WheelAccum,
    pub(crate) hwheel: nexa_ctl::WheelAccum,
    pub(crate) tui_wheel: nexa_ctl::WheelAccum,
    /// PTY에 마지막으로 알린 크기(열, 행) — 화면 버퍼 크기와 다르면 줄바꿈이 어긋난다(진단용 · `term` 덤프에).
    pub(crate) pty_size: (usize, usize),
    /// 읽을 출력이 남았다(시간 예산으로 끊음) — 호스트가 곧바로 다시 펌프한다.
    pub(crate) backlog: bool,
    /// 셸이 첫 출력을 냈다(그 전에는 "시작 중" 문구 — 프로필이 무거운 셸은 프롬프트까지 몇 초 걸린다).
    pub(crate) got_output: bool,
    session: Option<Box<dyn PtySession>>,
    pub(crate) screen: VtScreen,
    chunker: Utf8Chunker,
    /// 셸이 끝났다(안내 표시 · 아무 키 = 재시작).
    pub(crate) exited: bool,
    /// 시작 실패(`term.fail` 표시 · 아무 키 = 재시도).
    pub(crate) failed: bool,
    /// 스크롤백 보기 오프셋(0 = 하단 라이브).
    pub(crate) view_off: usize,
    /// 가로 보기 오프셋(열 — 고정 열 모드).
    pub(crate) view_x: usize,
    /// 마우스 선택(절대 라인, 열): (앵커, 끝).
    pub(crate) sel: Option<Sel>,
    drag: bool,
    /// paint가 캐시한 격자: (내용 rect, cell_w, cell_h, 가시 열 수).
    grid: (Rect, i32, i32, usize),
    pub(crate) caret_on: bool,
    blink_at: u64,
    /// 시작한 셸 라벨(cd 구문 선택 — cmd는 `/d`).
    pub(crate) shell_label: String,
    pub(crate) cwd: PathBuf,
}

impl Default for TermView {
    fn default() -> Self {
        Self::new()
    }
}

impl TermView {
    pub(crate) fn new() -> Self {
        TermView {
            wheel: nexa_ctl::WheelAccum::default(),
            hwheel: nexa_ctl::WheelAccum::default(),
            tui_wheel: nexa_ctl::WheelAccum::default(),
            pty_size: (0, 0),
            backlog: false,
            got_output: false,
            session: None,
            screen: VtScreen::new(80, 24),
            chunker: Utf8Chunker::default(),
            exited: false,
            failed: false,
            view_off: 0,
            view_x: 0,
            sel: None,
            drag: false,
            grid: (Rect::default(), 8, 16, 80),
            caret_on: true,
            blink_at: 0,
            shell_label: String::new(),
            cwd: PathBuf::new(),
        }
    }

    pub(crate) fn started(&self) -> bool {
        self.session.is_some()
    }

    pub(crate) fn alive(&self) -> bool {
        self.session.is_some() && !self.exited
    }

    /// 격자 치수(열, 행, cell_w, cell_h) — 호스트가 지연 시작 전에 계산(폰트 = Mono 슬롯 + 설정 증분 · 행 = 글꼴 높이 + 3).
    /// 줄 바꿈 끄기면 열 = `style.cols`(가시 열보다 많을 수 있다 — 가로 스크롤).
    pub(crate) fn grid_dims(
        dc: &mut dyn DrawCtx,
        rc: Rect,
        row_h: i32,
        style: &TermStyle,
    ) -> (usize, usize, i32, i32) {
        dc.select_font_sized(FontSlot::Mono, false, style.font_delta);
        // 칸 폭 = 글자 전진 폭의 **반올림**(터미널 관례 · Linux 실기 10-03): `text_width`는 올림이라 한 글자만 재면 8.2 px가
        // 9 px이 되어 우분투 터미널(8 px)보다 글자 사이가 벌어졌다 → 16자를 재어 평균을 반올림한다.
        let cw = ((dc.text_width("MMMMMMMMMMMMMMMM") as f32 / 16.0).round() as i32).max(1);
        let ch = match style.cell_h {
            Some(h) => h.max(8),
            None if style.font_delta == 0.0 => row_h.max(8),
            None => (dc.text_height() + 3).max(8),
        };
        let vis_cols = ((rc.w - 4) / cw).max(0) as usize;
        let cols = if style.wrap {
            vis_cols
        } else {
            style.cols.clamp(80, 1000)
        };
        (cols, ((rc.h - 2) / ch).max(0) as usize, cw, ch)
    }

    /// 세션 시작(cwd · 크기). 셸 없음/스폰 실패 = `failed`.
    pub(crate) fn start(
        &mut self,
        platform: &Platform,
        shell: Option<ShellSpec>,
        cwd: &Path,
        cols: usize,
        rows: usize,
    ) -> bool {
        let Some(shell) = shell else {
            self.failed = true;
            return false;
        };
        let (c, r) = (cols.max(2), rows.max(2));
        match platform.pty.spawn(
            &shell,
            cwd,
            u16::try_from(c).unwrap_or(u16::MAX),
            u16::try_from(r).unwrap_or(u16::MAX),
        ) {
            Ok(s) => {
                self.session = Some(s);
                self.pty_size = (c, r);
                self.got_output = false;
                self.backlog = false;
                self.screen = VtScreen::new(c, r);
                self.chunker = Utf8Chunker::default();
                self.exited = false;
                self.failed = false;
                self.view_off = 0;
                self.view_x = 0;
                self.sel = None;
                self.shell_label = shell.label;
                self.cwd = cwd.to_path_buf();
                true
            }
            Err(_) => {
                self.failed = true;
                false
            }
        }
    }

    /// 재시작 대기(종료·실패 뒤 아무 키 · → 버튼) — 다음 paint가 현재 폴더로 다시 연다.
    pub(crate) fn reset(&mut self) {
        self.session = None;
        self.exited = false;
        self.failed = false;
        self.view_off = 0;
        self.view_x = 0;
        self.sel = None;
    }

    /// 출력 비우기 → 화면에 반영. 변화가 있었으면 true. 세션 종료 감지도 여기서.
    pub(crate) fn pump(&mut self) -> bool {
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let mut changed = false;
        let mut buf = [0u8; 8192];
        let t0 = std::time::Instant::now();
        self.backlog = false;
        loop {
            let n = s.read(&mut buf).unwrap_or(0);
            if n == 0 {
                break;
            }
            trace_bytes(&buf[..n]);
            if let Some(text) = self.chunker.push(&buf[..n]) {
                self.screen.feed(&text);
                changed = true;
            }
            // 시간 예산을 넘기면 남은 것은 다음 틱에(그 사이 키 입력·그리기가 처리된다 · 종전 = 다 읽을 때까지 UI 스레드 점유).
            if t0.elapsed().as_millis() as u64 >= PUMP_BUDGET_MS {
                self.backlog = true;
                break;
            }
        }
        if !self.exited && !s.alive() {
            self.exited = true;
            changed = true;
        }
        // "시작 중" 해제 = **보이는 글자가 찍혔을 때**(ConPTY는 셸 프로필이 끝나기 전에 커서 숨김·화면 지우기 같은 제어 시퀀스를
        // 먼저 보낸다 — 바이트가 왔다는 것만으로 끄면 문구가 곧바로 사라져 빈 화면으로 몇 초를 기다리게 된다 · 10-03 캡처 검토).
        if !self.got_output && changed && !self.screen_text().trim().is_empty() {
            self.got_output = true;
        }
        changed
    }

    pub(crate) fn write(&mut self, text: &str) {
        if let Some(s) = self.session.as_mut() {
            let _ = s.write(text.as_bytes());
        }
    }

    fn sync_size(&mut self, cols: usize, rows: usize) {
        if cols < 2 || rows < 2 {
            return;
        }
        if self.screen.cols() != cols || self.screen.rows() != rows {
            self.screen.resize(cols, rows);
            if let Some(s) = self.session.as_mut() {
                if s.resize(
                    u16::try_from(cols).unwrap_or(u16::MAX),
                    u16::try_from(rows).unwrap_or(u16::MAX),
                )
                .is_ok()
                {
                    self.pty_size = (cols, rows);
                }
            }
        }
    }

    /// 캐럿 깜빡임 — 위상이 바뀌면 true.
    pub(crate) fn blink(&mut self, now_ms: u64) -> bool {
        if now_ms.saturating_sub(self.blink_at) >= CARET_BLINK_MS {
            self.blink_at = now_ms;
            self.caret_on = !self.caret_on;
            return true;
        }
        false
    }

    /// 입력 뒤 캐럿 켜기(dir2 QA 07-14 — 타이핑 중 보이게).
    pub(crate) fn caret_reset(&mut self, now_ms: u64) {
        self.caret_on = true;
        self.blink_at = now_ms;
    }

    /// 비문자 키 → VT 시퀀스(dir2 `term_key_seq` + Enter/Esc/Space).
    pub(crate) fn key_seq(key: Key) -> Option<&'static str> {
        Some(match key {
            Key::Up => "\x1b[A",
            Key::Down => "\x1b[B",
            Key::Right => "\x1b[C",
            Key::Left => "\x1b[D",
            Key::Home => "\x1b[H",
            Key::End => "\x1b[F",
            Key::Delete => "\x1b[3~",
            Key::PageUp => "\x1b[5~",
            Key::PageDown => "\x1b[6~",
            Key::Enter => "\r",
            Key::Escape => "\x1b",
            Key::Space => " ",
            _ => return None,
        })
    }

    /// 문자 → 바이트(Backspace `\u{8}` = DEL 0x7f — pwsh/bash 공통).
    pub(crate) fn char_seq(c: char) -> String {
        if c == '\u{8}' {
            "\x7f".into()
        } else {
            c.to_string()
        }
    }

    pub(crate) fn sel_norm(&self) -> Option<Sel> {
        let (a, b) = self.sel?;
        Some(if a <= b { (a, b) } else { (b, a) })
    }

    /// 격자 좌표 → (절대 라인, 열) — 범위 클램프 · 가로 오프셋 반영.
    pub(crate) fn cell_at(&self, x: i32, y: i32) -> (usize, usize) {
        let (rc, cw, ch, _) = self.grid;
        let col = ((x - rc.x - 2) / cw.max(1)).max(0) as usize + self.view_x;
        let row = ((y - rc.y - 1) / ch.max(1)).max(0) as usize;
        let sb = self.screen.scrollback_count();
        let top = sb - self.view_off.min(sb);
        let line = (top + row).min(self.screen.line_count().saturating_sub(1));
        (line, col.min(self.screen.cols().saturating_sub(1)))
    }

    /// TUI 마우스 모드(dir2 X-5 · DECSET 1000/1002/1003 + SGR 1006)면 `ESC[<b;col;row M|m`(1-기준 · 가시 화면 좌표). 아니면 None.
    pub(crate) fn mouse_report(&self, x: i32, y: i32, button: u8, press: bool) -> Option<String> {
        let (_, sgr) = self.screen.mouse_mode()?;
        if !sgr {
            return None; // 레거시(비SGR) 인코딩은 >127 바이트 — 현대 TUI는 전부 1006(dir2 동일 범위).
        }
        let (rc, cw, ch, _) = self.grid;
        if !rc.contains(nexa_ctl::geom::Point { x, y }) {
            return None;
        }
        let col = ((x - rc.x - 2) / cw.max(1)).max(0) as usize + self.view_x + 1;
        let row = ((y - rc.y - 1) / ch.max(1)).max(0) as usize + 1;
        Some(format!(
            "\x1b[<{button};{col};{row}{}",
            if press { 'M' } else { 'm' }
        ))
    }

    #[cfg(test)]
    pub(crate) fn hit(&self, x: i32, y: i32) -> bool {
        self.started() && self.grid.0.contains(nexa_ctl::geom::Point { x, y })
    }

    pub(crate) fn mouse_down(&mut self, x: i32, y: i32, shift: bool) {
        let cell = self.cell_at(x, y);
        self.sel = match (shift, self.sel) {
            (true, Some((a, _))) => Some((a, cell)),
            _ => Some((cell, cell)),
        };
        self.drag = true;
    }

    pub(crate) fn mouse_move(&mut self, x: i32, y: i32) -> bool {
        if !self.drag {
            return false;
        }
        let (rc, _, _, _) = self.grid;
        if y < rc.y {
            self.scroll_view(1);
        } else if y >= rc.bottom() {
            self.scroll_view(-1);
        }
        let cell = self.cell_at(
            x.clamp(rc.x, (rc.right() - 1).max(rc.x)),
            y.clamp(rc.y, (rc.bottom() - 1).max(rc.y)),
        );
        if let Some((a, _)) = self.sel {
            self.sel = Some((a, cell));
        }
        true
    }

    pub(crate) fn mouse_up(&mut self) {
        self.drag = false;
        // 클릭만(앵커 == 끝) = 선택 없음.
        if let Some((a, b)) = self.sel {
            if a == b {
                self.sel = None;
            }
        }
    }

    /// 스크롤백 보기 이동(양수 = 위로). 바뀌면 true.
    pub(crate) fn scroll_view(&mut self, lines: i32) -> bool {
        let max = self.screen.scrollback_count();
        let next = (self.view_off as i64 + lines as i64).clamp(0, max as i64) as usize;
        if next != self.view_off {
            self.view_off = next;
            true
        } else {
            false
        }
    }

    /// 가로 보기 이동(열 · 양수 = 오른쪽 · 고정 열 모드에서만 의미). 바뀌면 true.
    pub(crate) fn scroll_x(&mut self, cols: i32) -> bool {
        let vis = self.grid.3.max(1);
        let max = self.screen.cols().saturating_sub(vis);
        let next = (self.view_x as i64 + cols as i64).clamp(0, max as i64) as usize;
        if next != self.view_x {
            self.view_x = next;
            true
        } else {
            false
        }
    }

    pub(crate) fn select_all(&mut self) -> bool {
        let n = self.screen.line_count();
        if n == 0 {
            return false;
        }
        self.sel = Some(((0, 0), (n - 1, self.screen.cols().saturating_sub(1))));
        true
    }

    pub(crate) fn selected_text(&self) -> Option<String> {
        let ((sl, sc), (el, ec)) = self.sel_norm()?;
        Some(self.screen.get_text(sl, sc, el, ec))
    }

    /// 선택 범위의 런 목록(HTML/RTF 복사 — `ndir_term::export`).
    pub(crate) fn selected_runs(&self) -> Option<Vec<Vec<ndir_term::TextRun>>> {
        let ((sl, sc), (el, ec)) = self.sel_norm()?;
        Some(self.screen.get_runs(sl, sc, el, ec))
    }

    /// 화면 전체 텍스트(스크롤백 + 가시 · 줄 끝 공백 트림 · 빈 꼬리 줄 제거) — 덤프·시험.
    pub(crate) fn screen_text(&self) -> String {
        let n = self.screen.line_count();
        if n == 0 {
            return String::new();
        }
        let s = self
            .screen
            .get_text(0, 0, n - 1, self.screen.cols().saturating_sub(1));
        s.trim_end_matches(['\r', '\n']).to_string()
    }

    /// 상태 한 줄(덤프): `none|alive|exited|failed cols x rows view_off`.
    pub(crate) fn state_line(&self) -> String {
        let st = if self.failed {
            "failed"
        } else if self.session.is_none() {
            "none"
        } else if self.exited {
            "exited"
        } else {
            "alive"
        };
        format!(
            "{st} {}x{} view {} x {} sel {} pty {}x{}",
            self.screen.cols(),
            self.screen.rows(),
            self.view_off,
            self.view_x,
            self.sel.is_some(),
            self.pty_size.0,
            self.pty_size.1
        )
    }

    /// 셀 격자 렌더(dir2 `term_paint` 축약). `caret` = 포커스 중 깜빡임 켜짐 프레임.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn paint(
        &mut self,
        dc: &mut dyn DrawCtx,
        rc: Rect,
        theme: &Theme,
        pal: &TermPalette,
        caret: bool,
        row_h: i32,
        style: &TermStyle,
    ) {
        let (cols, rows, cell_w, cell_h) = Self::grid_dims(dc, rc, row_h, style);
        let argb = |c: u32| Color::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8);
        dc.fill_rect(rc, argb(pal.bg));
        if self.failed {
            dc.text(
                rc.x + 2,
                rc.y + 1,
                Rect::new(rc.x, rc.y, rc.w, cell_h),
                &tr("term.fail"),
                theme.text_dim,
            );
            return;
        }
        if !self.started() || cols < 2 || rows < 2 {
            return;
        }
        self.sync_size(cols, rows);
        let vis_cols = (((rc.w - 4) / cell_w.max(1)).max(1) as usize).min(cols);
        self.grid = (rc, cell_w, cell_h, vis_cols);
        self.view_x = self.view_x.min(cols.saturating_sub(vis_cols)); // 래핑 전환·리사이즈 방어
        let c0 = self.view_x;
        let sb = self.screen.scrollback_count();
        let view = self.view_off.min(sb);
        let top = sb - view;
        let sel = self.sel_norm();
        let in_sel = |line: usize, col: usize| match sel {
            Some(((sl, sc), (el, ec))) => {
                (line > sl || (line == sl && col >= sc)) && (line < el || (line == el && col <= ec))
            }
            None => false,
        };
        let pal_light = {
            let b = pal.bg;
            (299 * ((b >> 16) & 0xFF) + 587 * ((b >> 8) & 0xFF) + 114 * (b & 0xFF)) / 1000 >= 128
        };
        let (ar, ag, ab) = theme.accent.rgb();
        let accent = 0xFF00_0000 | (u32::from(ar) << 16) | (u32::from(ag) << 8) | u32::from(ab);
        let mut cur_bold = (false, false); // (굵게, 기울임) — grid_dims가 보통 Mono를 골라 뒀다
        for r in 0..rows {
            let y = rc.y + 1 + r as i32 * cell_h;
            let row_h = cell_h.min(rc.bottom() - y);
            if row_h <= 0 {
                break;
            }
            let abs = top + r;
            if abs >= self.screen.line_count() {
                break;
            }
            let line = self.screen.line_at(abs);
            let eff = |c: usize| -> (u32, u32, bool, (bool, bool)) {
                let cell = &line[c];
                let (mut fg, mut bg) = (pal.resolve(cell.fg), pal.resolve(cell.bg));
                if cell.reverse {
                    std::mem::swap(&mut fg, &mut bg);
                }
                if in_sel(abs, c) {
                    if pal_light && fg == pal.fg && bg == pal.bg {
                        (fg, bg) = (pal.bg, accent);
                    } else {
                        std::mem::swap(&mut fg, &mut bg);
                    }
                }
                (fg, bg, cell.faint, (cell.bold, cell.italic))
            };
            let c_end = cols.min(line.len()).min(c0 + vis_cols);
            let mut c = c0.min(c_end);
            while c < c_end {
                let (fg, bg, faint, bold) = eff(c);
                let start = c;
                while c < c_end {
                    let (cf, cb, _, cbold) = eff(c);
                    if cf != fg || cb != bg || cbold != bold {
                        break;
                    }
                    c += 1;
                }
                // SGR 1 굵게(UIC-311 · dir2 셀 속성) — 슬롯 재선택은 비싸므로(고정폭 광학 보정 실측) 바뀔 때만.
                if cur_bold != bold {
                    dc.select_font_sized_styled(FontSlot::Mono, bold.0, bold.1, style.font_delta);
                    cur_bold = bold;
                }
                let x = rc.x + 2 + (start - c0) as i32 * cell_w;
                let clip = Rect::new(
                    x,
                    y,
                    ((c - start) as i32 * cell_w).min(rc.right() - x),
                    row_h,
                );
                if bg != pal.bg {
                    dc.fill_rect(clip, argb(bg));
                }
                let mut fgc = argb(fg);
                if faint {
                    // faint = 배경 쪽으로 절반 블렌드(PSReadLine 예측 표시).
                    let (fr, fg_, fb) = fgc.rgb();
                    let (br, bg_, bb) = argb(bg).rgb();
                    let mid = |a: u8, b: u8| ((u16::from(a) + u16::from(b)) / 2) as u8;
                    fgc = Color::from_rgb(mid(fr, br), mid(fg_, bg_), mid(fb, bb));
                }
                for i in start..c {
                    let ch = line[i].ch;
                    if ch == '\0' || ch == ' ' {
                        continue;
                    }
                    // 폭 2칸: 전각 글자(다음 칸 = 연속 표식) · **아이콘 글리프 뒤가 빈 칸**(Windows Terminal처럼 한 칸보다 넓은
                    // Nerd Font 아이콘을 다음 칸까지 넘쳐 그린다 — 칸 안에서 자르면 아이콘이 잘리고 뒤 공백이 넓어 보인다).
                    let wide = line.get(i + 1).is_some_and(|n| {
                        n.ch == '\0' || ((is_icon_glyph(ch) || is_wide_symbol(ch)) && n.ch == ' ')
                    });
                    let cx = rc.x + 2 + (i - c0) as i32 * cell_w;
                    let cclip = Rect::new(cx, y, if wide { 2 } else { 1 } * cell_w, row_h);
                    let mut buf = [0u8; 4];
                    dc.text(cx, y, cclip, ch.encode_utf8(&mut buf), fgc);
                }
            }
        }
        if cur_bold != (false, false) {
            dc.select_font_sized(FontSlot::Mono, false, style.font_delta); // 종료 문구·뒤 그리기는 보통 굵기
        }
        // 시작 중 표시(사용자 10-03 "터미널을 누르면 반응 없이 오래 기다린다" · 1초 이상 걸리는 일은 진행 상태를 보인다):
        // 세션은 떴지만 셸이 아직 아무것도 내지 않았다 = 프로필 로딩 중.
        if self.session.is_some() && !self.exited && !self.got_output {
            dc.text(
                rc.x + 2,
                rc.y + 1,
                Rect::new(rc.x + 2, rc.y + 1, rc.w - 4, cell_h),
                &ndir_i18n::trf("term.starting", &[&self.shell_label]),
                theme.text_dim,
            );
        }
        if self.exited {
            let y = rc.bottom() - cell_h - 1;
            dc.text_opaque(
                rc.x + 2,
                y,
                Rect::new(rc.x + 2, y, rc.w - 4, cell_h),
                &tr("term.exited"),
                theme.accent,
                argb(pal.bg),
            );
        } else if caret && self.got_output {
            // 시작 중 문구 위에 캐럿이 겹치지 않게(10-03 캡처 검토) — 셸이 화면을 쓰기 시작한 뒤에만.
            let cr = sb + self.screen.cursor_row();
            let cc = self.screen.cursor_col();
            if cr >= top && cr < top + rows && cc >= c0 && cc < c0 + vis_cols {
                let cx = rc.x + 2 + (cc - c0) as i32 * cell_w;
                let cy = rc.y + 1 + (cr - top) as i32 * cell_h;
                if cy + cell_h <= rc.bottom() {
                    dc.fill_rect(Rect::new(cx, cy, 2, cell_h), argb(pal.fg));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunker_keeps_tail_and_replaces_bad_bytes() {
        let mut c = Utf8Chunker::default();
        assert_eq!(c.push("abc 한글".as_bytes()).as_deref(), Some("abc 한글"));
        assert_eq!(c.push(b""), None);
        assert_eq!(c.push(&[0xFF, b'a']).as_deref(), Some("\u{FFFD}a"));
        let bytes = "한".as_bytes();
        assert_eq!(c.push(&bytes[0..1]), None);
        assert_eq!(c.push(&bytes[1..2]), None);
        assert_eq!(c.push(&bytes[2..3]).as_deref(), Some("한"));
        assert_eq!(c.push(&[b'a', 0xC3]).as_deref(), Some("a"));
        assert_eq!(c.push(&[0x95]).as_deref(), Some("Õ"));
    }

    #[test]
    fn chunker_split_stream_equals_whole() {
        let text = "줄1 한글 ✓ \x1b[1;32mé\x1b[0m 🙂 끝\r\n".repeat(11);
        for step in [1usize, 2, 3, 5, 7, 4096] {
            let mut c = Utf8Chunker::default();
            let mut joined = String::new();
            for chunk in text.as_bytes().chunks(step) {
                if let Some(s) = c.push(chunk) {
                    joined.push_str(&s);
                }
            }
            assert_eq!(joined, text, "step {step}");
        }
    }

    #[test]
    fn key_and_char_sequences() {
        assert_eq!(TermView::key_seq(Key::Up), Some("\x1b[A"));
        assert_eq!(TermView::key_seq(Key::Enter), Some("\r"));
        assert_eq!(TermView::key_seq(Key::WordLeft), None);
        assert_eq!(TermView::char_seq('\u{8}'), "\x7f");
        assert_eq!(TermView::char_seq('a'), "a");
    }

    /// 가짜(echo) 세션으로 수명 왕복: 시작 → 쓰기 → pump → 화면 → 선택/텍스트 → 스크롤 상한.
    #[test]
    fn lifecycle_with_fake_pty() {
        let p = Platform::fake();
        let mut t = TermView::new();
        assert!(!t.started());
        let shell = p.shell.default_shell();
        assert!(t.start(&p, shell, Path::new("."), 40, 5));
        assert!(t.alive() && t.state_line().starts_with("alive 40x5"));
        t.write("hello\r\nworld");
        assert!(t.pump());
        assert!(!t.pump(), "두 번째는 변화 없음");
        assert_eq!(t.screen_text(), "hello\r\nworld");
        assert!(t.select_all());
        assert_eq!(
            t.selected_text()
                .map(|s| s.trim_end().to_string())
                .as_deref(),
            Some("hello\r\nworld")
        );
        assert!(t.selected_runs().is_some_and(|r| r.len() >= 2));
        assert!(!t.scroll_view(3), "스크롤백 없음 = 불변");
        let mut rec = nexa_ctl::RecordCtx::with_surface(400, 120);
        let th = Theme::dark();
        let style = TermStyle::default();
        t.paint(
            &mut rec,
            Rect::new(0, 0, 400, 120),
            &th,
            &TermPalette::dark(),
            true,
            20,
            &style,
        );
        assert!(rec.drew_text("h") && rec.drew_text("w"));
        assert!(
            rec.fonts.iter().all(|f| !f.1),
            "굵은 셀이 없으면 Mono 굵게 선택 없음: {:?}",
            rec.fonts
        );
        // SGR 1 굵게(UIC-311): 굵은 런 앞에서 Mono 굵게를 고르고 끝나면 보통으로 돌아온다.
        t.screen.feed("\r\n\x1b[1mBOLD\x1b[0m thin");
        rec.clear();
        t.paint(
            &mut rec,
            Rect::new(0, 0, 400, 120),
            &th,
            &TermPalette::dark(),
            true,
            20,
            &style,
        );
        let bolds: Vec<_> = rec.fonts.iter().map(|f| f.1).collect();
        assert!(bolds.contains(&true), "굵은 런 선택: {bolds:?}");
        assert_eq!(
            bolds.last(),
            Some(&false),
            "마지막은 보통 굵기로 복귀: {bolds:?}"
        );
        assert!(t.hit(5, 5) && !t.hit(500, 5));
        assert!(
            t.mouse_report(5, 5, 0, true).is_none(),
            "마우스 모드 꺼짐 = 보고 없음"
        );
        t.reset();
        assert!(!t.started() && t.state_line().starts_with("none"));
        // 셸 없음 = failed.
        assert!(!t.start(&p, None, Path::new("."), 40, 5));
        assert!(t.failed);
    }

    /// 고정 열(줄 바꿈 끄기): 화면 열 = 설정 열 · 가로 스크롤 상한 = 열 − 가시 열 · 셀 좌표는 오프셋 반영 · TUI 마우스 모드 보고(SGR).
    #[test]
    fn fixed_columns_horizontal_scroll_and_mouse_report() {
        let p = Platform::fake();
        let mut t = TermView::new();
        let shell = p.shell.default_shell();
        assert!(t.start(&p, shell, Path::new("."), 80, 5));
        let style = TermStyle {
            font_delta: 0.0,
            wrap: false,
            cols: 120,
            cell_h: None,
        };
        let mut rec = nexa_ctl::RecordCtx::with_surface(300, 100);
        let th = Theme::dark();
        // RecordCtx text_width = 글자 수 × 7 → 가시 열 = (300-4)/7 = 42.
        t.paint(
            &mut rec,
            Rect::new(0, 0, 300, 100),
            &th,
            &TermPalette::dark(),
            false,
            20,
            &style,
        );
        assert_eq!(t.screen.cols(), 120, "고정 열");
        let vis = (300 - 4) / 7;
        assert!(t.scroll_x(10) && t.view_x == 10);
        assert!(
            t.scroll_x(1000) && t.view_x == 120 - vis,
            "상한 = 열 − 가시 열"
        );
        assert!(!t.scroll_x(1));
        assert_eq!(
            t.cell_at(2 + 7 * 5, 1).1,
            120 - vis + 5,
            "열 = 가시 열 + 오프셋"
        );
        // DECSET 1000 + 1006 → 보고 · 뗌 = m.
        t.screen.feed("\x1b[?1000h\x1b[?1006h");
        assert!(t.screen.mouse_mode().is_some());
        let first = 120 - vis + 1;
        assert_eq!(
            t.mouse_report(2, 1, 0, true).as_deref(),
            Some(format!("\x1b[<0;{first};1M").as_str())
        );
        assert_eq!(
            t.mouse_report(2, 1, 0, false).as_deref(),
            Some(format!("\x1b[<0;{first};1m").as_str())
        );
        assert!(t.mouse_report(999, 1, 0, true).is_none(), "격자 밖 = 없음");
        // 줄 바꿈 켜기로 되돌리면 열 = 가시 열 · 오프셋 0.
        let wrap = TermStyle::default();
        t.paint(
            &mut rec,
            Rect::new(0, 0, 300, 100),
            &th,
            &TermPalette::dark(),
            false,
            20,
            &wrap,
        );
        assert_eq!((t.screen.cols(), t.view_x), (vis, 0));
    }

    /// 출력 폭주(사용자 10-03 "ls 출력 중 Ctrl+C가 안 먹는다"): 펌프는 시간 예산 안에서만 읽고 남은 것은 `backlog`로 알린다 —
    /// 호스트가 틱 사이에 입력을 처리할 수 있다. 몇 번 더 펌프하면 다 읽는다. 첫 출력 전에는 `got_output` = false("시작 중" 표시).
    #[test]
    fn pump_is_time_boxed_and_reports_backlog() {
        let p = Platform::fake();
        let mut t = TermView::new();
        let shell = p.shell.default_shell();
        assert!(t.start(&p, shell, Path::new("."), 80, 24));
        assert!(!t.got_output && !t.backlog);
        // 제어 시퀀스만 온 동안은 아직 "시작 중"(보이는 글자가 없다).
        t.write("[?25l[2J[H");
        t.pump();
        assert!(!t.got_output, "ESC 시퀀스뿐 = 시작 중 유지");
        // 가짜 PTY는 쓴 것을 그대로 돌려준다 — 수 MB를 흘린다.
        let line = "0123456789abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz\r\n";
        let big = line.repeat(60_000);
        t.write(&big);
        let t0 = std::time::Instant::now();
        assert!(t.pump());
        let first = t0.elapsed();
        assert!(t.got_output);
        assert!(t.backlog, "한 번에 다 읽지 않는다");
        assert!(
            first < std::time::Duration::from_millis(250),
            "펌프 1회가 UI를 오래 잡지 않는다: {first:?}"
        );
        let mut rounds = 1;
        while t.backlog && rounds < 100_000 {
            t.pump();
            rounds += 1;
        }
        assert!(!t.backlog && rounds > 1, "나눠 읽어 끝난다: {rounds}");
    }

    /// Windows Terminal과 같게: 아이콘 글리프(사용자 영역) 뒤가 빈 칸이면 2칸 폭으로 그린다(넘쳐 그리기) · 글자가 이어지면 1칸.
    #[test]
    fn icon_glyph_overflows_into_following_blank_cell() {
        assert!(is_icon_glyph('\u{F07B}') && is_icon_glyph('\u{E0A0}') && !is_icon_glyph('A'));
        // 넓은 기호(➜ ✗ ← ▶)는 뒤가 빈 칸이면 넘쳐 그린다 · 상자 그리기(─ │)와 글자는 아니다.
        assert!(['\u{279C}', '\u{2717}', '\u{2190}', '\u{25B6}']
            .into_iter()
            .all(is_wide_symbol));
        assert!(!['\u{2500}', '\u{2502}', 'A', '가', '\u{E0A0}']
            .into_iter()
            .any(is_wide_symbol));
        let p = Platform::fake();
        let mut t = TermView::new();
        assert!(t.start(&p, p.shell.default_shell(), Path::new("."), 40, 5));
        t.screen.feed("\u{F07B}  docs\r\n\u{F07B}x");
        let mut rec = nexa_ctl::RecordCtx::with_surface(400, 120);
        t.paint(
            &mut rec,
            Rect::new(0, 0, 400, 120),
            &Theme::dark(),
            &TermPalette::dark(),
            false,
            20,
            &TermStyle::default(),
        );
        let widths: Vec<i32> = rec
            .texts
            .iter()
            .filter(|t| t.3 == "\u{F07B}")
            .map(|t| t.2.w)
            .collect();
        assert_eq!(widths.len(), 2);
        assert_eq!(
            widths[0],
            widths[1] * 2,
            "뒤가 공백 = 2칸 · 뒤가 글자 = 1칸: {widths:?}"
        );
    }

    /// SGR 3 기울임 셀은 기울임 글꼴로 그린다(Windows Terminal의 `Length` 머리글처럼) · 끝나면 보통으로 복귀.
    #[test]
    fn italic_cells_select_italic_font() {
        let p = Platform::fake();
        let mut t = TermView::new();
        assert!(t.start(&p, p.shell.default_shell(), Path::new("."), 40, 5));
        t.screen.feed("plain \x1b[3mLength\x1b[23m Name");
        let mut rec = nexa_ctl::RecordCtx::with_surface(400, 120);
        t.paint(
            &mut rec,
            Rect::new(0, 0, 400, 120),
            &Theme::dark(),
            &TermPalette::dark(),
            false,
            20,
            &TermStyle::default(),
        );
        assert!(
            rec.fonts.contains(&(nexa_ctl::FontSlot::Mono, false, true)),
            "기울임 선택: {:?}",
            rec.fonts
        );
        assert_eq!(
            rec.fonts.last().map(|f| (f.1, f.2)),
            Some((false, false)),
            "보통으로 복귀"
        );
    }
}
