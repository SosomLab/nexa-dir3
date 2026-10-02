//! 도크 터미널 뷰(T-61 · dir2 `win.rs` `TermState`/`term_paint`/`term_key_seq` 이식 — 창·GDI 대신 `DrawCtx` · ConPTY 대신 `PtySession` 포트).
//!
//! 수명: 도크 종류가 터미널일 때 호스트가 **지연 시작**(`start` · cwd = 원천 패널 폴더) → `pump`(폴링 틱 · 바이트 → UTF-8 → `VtScreen::feed`)
//! → `paint`(셀 격자 · 선택 반전 · 캐럿 · 종료 안내). 종료 뒤 아무 키 = `reset` → 다음 paint가 재시작(dir2 규약).
//! 줄 바꿈 모드만(가시 폭 = 열 수) — 고정 열 + 가로 스크롤(`term.wrap=off`)·트랙패드 픽셀 스크롤·고속 스크롤은 T-61 B.

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
    session: Option<Box<dyn PtySession>>,
    pub(crate) screen: VtScreen,
    chunker: Utf8Chunker,
    /// 셸이 끝났다(안내 표시 · 아무 키 = 재시작).
    pub(crate) exited: bool,
    /// 시작 실패(`term.fail` 표시 · 아무 키 = 재시도).
    pub(crate) failed: bool,
    /// 스크롤백 보기 오프셋(0 = 하단 라이브).
    pub(crate) view_off: usize,
    /// 마우스 선택(절대 라인, 열): (앵커, 끝).
    pub(crate) sel: Option<Sel>,
    drag: bool,
    /// paint가 캐시한 격자: (내용 rect, cell_w, cell_h).
    grid: (Rect, i32, i32),
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
            session: None,
            screen: VtScreen::new(80, 24),
            chunker: Utf8Chunker::default(),
            exited: false,
            failed: false,
            view_off: 0,
            sel: None,
            drag: false,
            grid: (Rect::default(), 8, 16),
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

    /// 격자 치수(열, 행) — 호스트가 지연 시작 전에 계산(폰트 = Mono 슬롯 · 행 = row_h).
    pub(crate) fn grid_dims(
        dc: &mut dyn DrawCtx,
        rc: Rect,
        row_h: i32,
    ) -> (usize, usize, i32, i32) {
        dc.select_font(FontSlot::Mono, false);
        let cw = dc.text_width("M").max(1);
        let ch = row_h.max(8);
        (
            ((rc.w - 4) / cw).max(0) as usize,
            ((rc.h - 2) / ch).max(0) as usize,
            cw,
            ch,
        )
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
                self.screen = VtScreen::new(c, r);
                self.chunker = Utf8Chunker::default();
                self.exited = false;
                self.failed = false;
                self.view_off = 0;
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
        self.sel = None;
    }

    /// 출력 비우기 → 화면에 반영. 변화가 있었으면 true. 세션 종료 감지도 여기서.
    pub(crate) fn pump(&mut self) -> bool {
        let Some(s) = self.session.as_mut() else {
            return false;
        };
        let mut changed = false;
        let mut buf = [0u8; 8192];
        loop {
            let n = s.read(&mut buf).unwrap_or(0);
            if n == 0 {
                break;
            }
            if let Some(text) = self.chunker.push(&buf[..n]) {
                self.screen.feed(&text);
                changed = true;
            }
        }
        if !self.exited && !s.alive() {
            self.exited = true;
            changed = true;
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
                let _ = s.resize(
                    u16::try_from(cols).unwrap_or(u16::MAX),
                    u16::try_from(rows).unwrap_or(u16::MAX),
                );
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

    /// 격자 좌표 → (절대 라인, 열) — 범위 클램프.
    pub(crate) fn cell_at(&self, x: i32, y: i32) -> (usize, usize) {
        let (rc, cw, ch) = self.grid;
        let col = ((x - rc.x - 2) / cw.max(1)).max(0) as usize;
        let row = ((y - rc.y - 1) / ch.max(1)).max(0) as usize;
        let sb = self.screen.scrollback_count();
        let top = sb - self.view_off.min(sb);
        let line = (top + row).min(self.screen.line_count().saturating_sub(1));
        (line, col.min(self.screen.cols().saturating_sub(1)))
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
        let (rc, _, _) = self.grid;
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
            "{st} {}x{} view {} sel {}",
            self.screen.cols(),
            self.screen.rows(),
            self.view_off,
            self.sel.is_some()
        )
    }

    /// 셀 격자 렌더(dir2 `term_paint` 축약 — 줄 바꿈 모드). `caret` = 포커스 중 깜빡임 켜짐 프레임.
    pub(crate) fn paint(
        &mut self,
        dc: &mut dyn DrawCtx,
        rc: Rect,
        theme: &Theme,
        pal: &TermPalette,
        caret: bool,
        row_h: i32,
    ) {
        let (cols, rows, cell_w, cell_h) = Self::grid_dims(dc, rc, row_h);
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
        self.grid = (rc, cell_w, cell_h);
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
        // 클립은 호출마다 rect로(nexa-ctl DrawCtx에는 클립 스택이 없다 — T-31).
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
            let eff = |c: usize| -> (u32, u32, bool) {
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
                (fg, bg, cell.faint)
            };
            let c_end = cols.min(line.len());
            let mut c = 0;
            while c < c_end {
                let (fg, bg, faint) = eff(c);
                let start = c;
                while c < c_end {
                    let (cf, cb, _) = eff(c);
                    if cf != fg || cb != bg {
                        break;
                    }
                    c += 1;
                }
                let x = rc.x + 2 + start as i32 * cell_w;
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
                    let wide = line.get(i + 1).is_some_and(|n| n.ch == '\0');
                    let cx = rc.x + 2 + i as i32 * cell_w;
                    let cclip = Rect::new(cx, y, if wide { 2 } else { 1 } * cell_w, row_h);
                    let mut buf = [0u8; 4];
                    dc.text(cx, y, cclip, ch.encode_utf8(&mut buf), fgc);
                }
            }
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
        } else if caret {
            let cr = sb + self.screen.cursor_row();
            let cc = self.screen.cursor_col();
            if cr >= top && cr < top + rows && cc < cols {
                let cx = rc.x + 2 + cc as i32 * cell_w;
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
        assert!(!t.scroll_view(3), "스크롤백 없음 = 불변");
        let mut rec = nexa_ctl::RecordCtx::with_surface(400, 120);
        let th = Theme::dark();
        t.paint(
            &mut rec,
            Rect::new(0, 0, 400, 120),
            &th,
            &TermPalette::dark(),
            true,
            20,
        );
        assert!(rec.drew_text("h") && rec.drew_text("w"));
        assert!(t.hit(5, 5) && !t.hit(500, 5));
        t.reset();
        assert!(!t.started() && t.state_line().starts_with("none"));
        // 셸 없음 = failed.
        assert!(!t.start(&p, None, Path::new("."), 40, 5));
        assert!(t.failed);
    }
}
