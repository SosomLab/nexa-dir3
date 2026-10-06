//! 체크섬 창(T-167 · NEW-038 · dir3 신규 · 사용자 10-05 · 10-06 개편): 우클릭 메뉴 "체크섬…" → **모덜리스** 창.
//!
//! - **파일 1개** = 알고리즘별 값 행 · 진행 막대(읽은 양 · 속도) · 행 클릭 = 그 값 복사 · 비교 입력(길이로 알고리즘을 골라 일치/불일치).
//! - **여러 파일** = 표(파일 · 크기 · "같음" · 알고리즘 열) — 내용이 같은 파일끼리 같은 글자(A · B …)로 묶어 한눈에 비교 ·
//!   비교 입력에 값을 넣으면 일치하는 행에 ✓ · Ctrl+C = 선택 행(없으면 전부) · [모두 복사] = 탭 구분 표.
//! - **자동 계산 상한**(사용자 10-06 "대용량/다수 파일은 프로세스를 잡아먹는다"): 합계가 설정 `hash.auto_limit_mb`를 넘으면 자동으로
//!   시작하지 않고 [계산] 버튼을 보인다(언제나 수동 = 상한 0). 계산은 호스트의 작업 스레드(`app/checksum.rs` · 낮은 우선순위 ·
//!   한 번에 하나)가 하고 이 창은 값만 받는다. 창을 닫으면 진행 중인 계산도 취소된다.
//! - 속도는 계산이 끝난 시점에 **고정**한다(종전 = 그릴 때마다 경과 시간으로 다시 나눠 마우스만 움직여도 값이 줄어 보였다 · 사용자 10-06).

use ndir_i18n::{tr, trf};
use ndir_ops::hash::{self, Algo};
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::{
    Button, Control, InputEvent, Invalidations, Key as CtlKey, SegProgress, TextBox, Widget,
};
use nexa_gfx::{Font, Surface};
use nexa_grid::{Column, Marker, RowItem, RowSource, SelectOp, VirtualRows};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;
use winit::event::{ElementState, Ime, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// 창이 호스트에 요청하는 것.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HashAction {
    None,
    Paint,
    /// 닫기(진행 중이면 호스트가 계산도 취소한다).
    Close,
    /// 클립보드에 글을 넣는다(행 클릭 · [모두 복사] · Ctrl+C).
    CopyText(String),
    /// [계산] — 상한을 넘어 자동으로 시작하지 않은 계산을 시작한다.
    Start,
}

const PAD: f32 = 12.0;
const ROW_H: f32 = 26.0;
const BTN_H: f32 = 28.0;
const WIN_W: f32 = 640.0;
const WIN_W_MULTI: f32 = 960.0;
const GRID_H_MULTI: f32 = 300.0;

/// 파일 1건의 상태.
#[derive(Debug, Clone)]
pub(crate) struct FileRow {
    pub(crate) path: PathBuf,
    pub(crate) size: u64,
    /// 알고리즘 순서대로 값(`None` = 아직).
    pub(crate) values: Vec<Option<String>>,
    pub(crate) error: Option<String>,
}

/// 여러 파일 표의 행 공급자.
#[derive(Debug)]
pub(crate) struct HashSource {
    rows: Vec<FileRow>,
    algos: Vec<Algo>,
    /// 내용 묶음 글자(마지막 알고리즘 값이 같은 파일끼리 A · B … · 혼자면 없음).
    groups: Vec<Option<char>>,
    /// 비교 입력과 일치하는 행.
    matched: Vec<bool>,
    selected: Vec<bool>,
    anchor: Option<usize>,
}

impl HashSource {
    /// 묶음 글자 다시 매기기(순수): 가장 강한(마지막) 알고리즘 값이 같으면 같은 글자 · 둘 이상일 때만.
    fn regroup(&mut self) {
        let last = self.algos.len().saturating_sub(1);
        let mut seen: Vec<(String, char)> = Vec::new();
        let mut next = b'A';
        let groups: Vec<Option<char>> = self
            .rows
            .iter()
            .map(|r| {
                let v = r.values.get(last).and_then(|v| v.clone())?;
                if let Some((_, c)) = seen.iter().find(|(h, _)| *h == v) {
                    return Some(*c);
                }
                let c = char::from(next);
                next = next.saturating_add(1).min(b'Z');
                seen.push((v, c));
                Some(c)
            })
            .collect();
        self.groups = groups
            .iter()
            .map(|g| {
                let n = groups.iter().filter(|x| *x == g).count();
                if g.is_some() && n >= 2 {
                    *g
                } else {
                    None
                }
            })
            .collect();
    }
}

const COL_SIZE: u32 = 1;
const COL_SAME: u32 = 2;
const COL_ALGO0: u32 = 3;

impl RowSource for HashSource {
    fn len(&self) -> usize {
        self.rows.len()
    }
    fn row(&self, index: usize) -> RowItem {
        RowItem {
            text: ndir_ops::leaf_name(&self.rows[index].path),
            is_dir: false,
            depth: 0,
            marker: Marker::None,
        }
    }
    fn cell(&self, index: usize, key: u32) -> String {
        let r = &self.rows[index];
        match key {
            COL_SIZE => crate::filelist::format_size(r.size),
            COL_SAME => {
                let mut s = self.groups[index].map(String::from).unwrap_or_default();
                if self.matched[index] {
                    s.push_str(" \u{2713}");
                }
                s
            }
            k if k >= COL_ALGO0 => {
                let i = (k - COL_ALGO0) as usize;
                if let Some(e) = &r.error {
                    return if i == 0 {
                        format!("\u{26A0} {e}")
                    } else {
                        String::new()
                    };
                }
                r.values
                    .get(i)
                    .and_then(|v| v.clone())
                    .unwrap_or_else(|| tr("hash.calculating"))
            }
            _ => String::new(),
        }
    }
    fn is_selected(&self, index: usize) -> bool {
        self.selected.get(index).copied().unwrap_or(false)
    }
    fn select(&mut self, index: usize, op: SelectOp) -> bool {
        if index >= self.rows.len() {
            return false;
        }
        match op {
            SelectOp::Single => {
                for (i, s) in self.selected.iter_mut().enumerate() {
                    *s = i == index;
                }
                self.anchor = Some(index);
            }
            SelectOp::Toggle => {
                self.selected[index] = !self.selected[index];
                self.anchor = Some(index);
            }
            SelectOp::RangeTo => {
                let a = self.anchor.unwrap_or(index);
                let (lo, hi) = (a.min(index), a.max(index));
                for (i, s) in self.selected.iter_mut().enumerate() {
                    *s = (lo..=hi).contains(&i);
                }
            }
        }
        true
    }
    fn select_all(&mut self) -> bool {
        self.selected.iter_mut().for_each(|s| *s = true);
        !self.rows.is_empty()
    }
    fn clear_selection(&mut self) -> bool {
        let had = self.selected.iter().any(|s| *s);
        self.selected.iter_mut().for_each(|s| *s = false);
        had
    }
}

pub(crate) struct HashWin {
    window: Option<Rc<Window>>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    cursor: (i32, i32),
    shift: bool,
    primary: bool,
    algos: Vec<Algo>,
    /// 파일들(1개면 단일 보기 · 여럿이면 표).
    files: Vec<FileRow>,
    /// 여러 파일 표(단일 보기에서는 `None`).
    grid: Option<VirtualRows<HashSource>>,
    row_h: i32,
    /// 단일 보기의 알고리즘 행 자리(클릭 = 복사).
    row_rects: Vec<Rect>,
    bar: SegProgress,
    done: u64,
    total: u64,
    /// 진행 중인 파일 번호(표시용).
    current: usize,
    started: Instant,
    /// 끝났을 때의 경과(초) — 속도 고정(사용자 10-06).
    elapsed_fixed: Option<f64>,
    running: bool,
    /// 상한을 넘어 자동 시작하지 않음 — [계산]을 기다린다.
    waiting: bool,
    error: Option<String>,
    tb_cmp: TextBox,
    btn_calc: Button,
    btn_copy: Button,
    btn_close: Button,
    /// 바닥 안내(복사됨 등 · 다음 클릭까지).
    note: Option<String>,
}

impl HashWin {
    pub(crate) fn new() -> Self {
        HashWin {
            window: None,
            surface: None,
            scale: 1.0,
            cursor: (0, 0),
            shift: false,
            primary: false,
            algos: Vec::new(),
            files: Vec::new(),
            grid: None,
            row_h: 0,
            row_rects: Vec::new(),
            bar: SegProgress::new(),
            done: 0,
            total: 0,
            current: 0,
            started: Instant::now(),
            elapsed_fixed: None,
            running: false,
            waiting: false,
            error: None,
            tb_cmp: TextBox::new(""),
            btn_calc: Button::new(tr("hash.calc")),
            btn_copy: Button::new(tr("hash.copyAll")),
            btn_close: Button::new(tr("ops.cancel")),
            note: None,
        }
    }

    fn multi(&self) -> bool {
        self.files.len() > 1
    }

    /// 대상 지정 — `auto` = 바로 계산(호스트가 작업을 띄웠다) · 아니면 [계산] 대기(상한 초과).
    pub(crate) fn start(&mut self, files: &[(PathBuf, u64)], algos: &[Algo], auto: bool) {
        self.algos = algos.to_vec();
        self.files = files
            .iter()
            .map(|(p, s)| FileRow {
                path: p.clone(),
                size: *s,
                values: vec![None; algos.len()],
                error: None,
            })
            .collect();
        self.total = files.iter().map(|(_, s)| *s).sum();
        self.done = 0;
        self.current = 0;
        self.started = Instant::now();
        self.elapsed_fixed = None;
        self.running = auto;
        self.waiting = !auto;
        self.error = None;
        self.note = None;
        let mut inv = Invalidations::default();
        self.bar.set_items(Vec::new(), &mut inv);
        self.bar.set_totals(0, self.total.max(1), &mut inv);
        self.btn_copy.set_enabled(false);
        self.btn_calc.set_enabled(!auto);
        self.btn_close.set_label(tr(if auto {
            "ops.cancel"
        } else {
            "license.btn.close"
        }));
        self.rebuild_grid();
        self.redraw();
    }

    /// [계산]을 눌러 호스트가 작업을 띄웠다.
    pub(crate) fn begin(&mut self) {
        self.waiting = false;
        self.running = true;
        self.started = Instant::now();
        self.elapsed_fixed = None;
        self.btn_calc.set_enabled(false);
        self.btn_close.set_label(tr("ops.cancel"));
        self.redraw();
    }

    fn rebuild_grid(&mut self) {
        if !self.multi() {
            self.grid = None;
            return;
        }
        let n = self.files.len();
        let mut src = HashSource {
            rows: self.files.clone(),
            algos: self.algos.clone(),
            groups: vec![None; n],
            matched: vec![false; n],
            selected: vec![false; n],
            anchor: None,
        };
        src.regroup();
        let mut inv = Invalidations::default();
        let mut rows = VirtualRows::new(src, self.row_h.max(20), 8, 16);
        let mut cols = vec![
            Column::new(0, tr("col.name"), 220),
            Column::new(COL_SIZE, tr("col.size"), 80).right_aligned(),
            Column::new(COL_SAME, tr("hash.col.same"), 60),
        ];
        for (i, a) in self.algos.iter().enumerate() {
            cols.push(Column::new(
                COL_ALGO0 + i as u32,
                a.name(),
                (a.hex_len() as i32 * 7).clamp(90, 460),
            ));
        }
        rows.set_columns(cols, &mut inv);
        rows.set_font_decor(false, true, false, &mut inv);
        self.grid = Some(rows);
        self.refresh_compare();
    }

    /// 진행(읽은 바이트 누계 · 지금 파일 번호).
    pub(crate) fn update(&mut self, done: u64, current: usize) {
        if done == self.done && current == self.current {
            return;
        }
        self.done = done;
        self.current = current;
        let mut inv = Invalidations::default();
        self.bar.set_totals(done, self.total.max(1), &mut inv);
        self.redraw();
    }

    /// 파일 1건의 결과(또는 오류 글).
    pub(crate) fn set_file_result(
        &mut self,
        index: usize,
        result: Result<Vec<(Algo, String)>, String>,
    ) {
        let algos = self.algos.clone();
        let Some(f) = self.files.get_mut(index) else {
            return;
        };
        match result {
            Ok(list) => {
                for (i, a) in algos.iter().enumerate() {
                    if let Some((_, hex)) = list.iter().find(|(x, _)| x == a) {
                        f.values[i] = Some(hex.clone());
                    }
                }
            }
            Err(e) => f.error = Some(e),
        }
        if let Some(g) = &mut self.grid {
            let src = g.source_mut();
            src.rows[index] = self.files[index].clone();
            src.regroup();
        }
        self.refresh_compare();
        self.redraw();
    }

    /// 전부 끝남(`error` = 취소 등 안내).
    pub(crate) fn finish(&mut self, error: Option<String>) {
        self.running = false;
        self.waiting = false;
        self.error = error;
        self.elapsed_fixed = Some(self.started.elapsed().as_secs_f64().max(0.001));
        if self.error.is_none() {
            self.done = self.total;
            let mut inv = Invalidations::default();
            self.bar
                .set_totals(self.total.max(1), self.total.max(1), &mut inv);
        }
        self.btn_copy.set_enabled(
            self.files
                .iter()
                .any(|f| f.values.iter().any(Option::is_some)),
        );
        self.btn_close.set_label(tr("license.btn.close"));
        self.redraw();
    }

    /// 대상 파일들(호스트의 [계산] 처리용).
    pub(crate) fn files(&self) -> &[FileRow] {
        &self.files
    }

    #[cfg(test)]
    pub(crate) fn is_running(&self) -> bool {
        self.running
    }

    #[cfg(test)]
    pub(crate) fn is_waiting(&self) -> bool {
        self.waiting
    }

    /// 결과 글(복사 · 시험): 파일 1개 = `ALGO  값  파일` 줄들 · 여럿 = 탭 구분 표(머리 줄 포함).
    pub(crate) fn results_text(&self) -> String {
        if !self.multi() {
            let Some(f) = self.files.first() else {
                return String::new();
            };
            let name = ndir_ops::leaf_name(&f.path);
            return self
                .algos
                .iter()
                .zip(&f.values)
                .filter_map(|(a, v)| v.as_ref().map(|h| format!("{:<8} {h}  {name}", a.name())))
                .collect::<Vec<_>>()
                .join("\r\n");
        }
        let mut lines = vec![format!(
            "name\tsize\t{}",
            self.algos
                .iter()
                .map(|a| a.name())
                .collect::<Vec<_>>()
                .join("\t")
        )];
        lines.extend(self.files.iter().map(Self::tsv_line));
        lines.join("\r\n")
    }

    fn tsv_line(f: &FileRow) -> String {
        format!(
            "{}\t{}\t{}",
            f.path.display(),
            f.size,
            f.values
                .iter()
                .map(|v| v.clone().unwrap_or_default())
                .collect::<Vec<_>>()
                .join("\t")
        )
    }

    #[cfg(test)]
    pub(crate) fn results(&self) -> Vec<FileRow> {
        self.files.clone()
    }

    #[cfg(test)]
    pub(crate) fn set_compare_text(&mut self, text: &str) {
        self.tb_cmp.set_text(text);
        self.refresh_compare();
    }

    /// 묶음 글자(시험).
    #[cfg(test)]
    pub(crate) fn group_of(&self, index: usize) -> Option<char> {
        self.grid
            .as_ref()
            .and_then(|g| g.source().groups.get(index).copied().flatten())
    }

    /// 비교 입력의 판정(순수): `None` = 입력 없음/맞는 길이 없음 · `Some(Err(()))` = 16진이 아님 ·
    /// `Some(Ok((알고리즘, 일치한 파일 수)))`.
    pub(crate) fn compare_state(&self) -> Option<Result<(Algo, usize), ()>> {
        let text = self.tb_cmp.text();
        if text.trim().is_empty() {
            return None;
        }
        let Some(norm) = hash::normalize_hex(&text) else {
            return Some(Err(()));
        };
        let (ai, algo) = self
            .algos
            .iter()
            .enumerate()
            .find(|(_, a)| a.hex_len() == norm.len())?;
        let hits = self
            .files
            .iter()
            .filter(|f| f.values.get(ai).and_then(|v| v.as_deref()) == Some(norm.as_str()))
            .count();
        Some(Ok((*algo, hits)))
    }

    /// 표의 ✓ 열을 비교 입력에 맞춘다.
    fn refresh_compare(&mut self) {
        let Some(g) = &mut self.grid else {
            return;
        };
        let norm = hash::normalize_hex(&self.tb_cmp.text());
        let ai = norm
            .as_ref()
            .and_then(|n| self.algos.iter().position(|a| a.hex_len() == n.len()));
        let src = g.source_mut();
        for (i, f) in self.files.iter().enumerate() {
            src.matched[i] = match (&norm, ai) {
                (Some(n), Some(ai)) => {
                    f.values.get(ai).and_then(|v| v.as_deref()) == Some(n.as_str())
                }
                _ => false,
            };
        }
    }

    pub(crate) fn open(
        &mut self,
        el: &ActiveEventLoop,
        theme: Option<winit::window::Theme>,
        over: Option<(i32, i32, u32, u32)>,
        owner: Option<&Window>,
    ) {
        if let Some(w) = &self.window {
            w.focus_window();
            self.redraw();
            return;
        }
        let tail = 8.0 + 44.0 + 8.0 + 56.0 + 8.0 + BTN_H;
        let (lw, lh) = if self.multi() {
            (WIN_W_MULTI, PAD * 2.0 + 30.0 + GRID_H_MULTI + tail)
        } else {
            let rows = self.algos.len().max(1) as f32;
            (WIN_W, PAD * 2.0 + 30.0 + rows * ROW_H + tail)
        };
        let mut attrs = Window::default_attributes()
            .with_title(format!("{} — {}", crate::APP_TITLE, tr("hash.title")))
            .with_theme(theme)
            .with_resizable(true)
            .with_min_inner_size(winit::dpi::LogicalSize::new(420.0, lh.min(360.0)))
            .with_inner_size(winit::dpi::LogicalSize::new(lw, lh));
        if let Some((x, y, w, h)) = over {
            let cx = x + (w as i32 - lw as i32) / 2;
            let cy = y + (h as i32 - lh as i32) / 2;
            attrs = attrs.with_position(winit::dpi::PhysicalPosition::new(cx.max(0), cy.max(0)));
        }
        let attrs = crate::winfocus::owned_by(crate::icon::with_icon(attrs), owner);
        let Ok(win) = el.create_window(attrs) else {
            return;
        };
        let win = Rc::new(win);
        win.set_ime_allowed(crate::input::system_ime());
        self.scale = win.scale_factor() as f32;
        self.surface = crate::present::Presenter::new(win.clone()).ok();
        self.window = Some(win);
        self.tb_cmp.set_focused(true);
        self.redraw();
    }

    pub(crate) fn close(&mut self) {
        self.surface = None;
        self.window = None;
        for b in [&mut self.btn_close, &mut self.btn_copy, &mut self.btn_calc] {
            b.clear_transient();
        }
        self.tb_cmp.set_text("");
        self.tb_cmp.set_focused(false);
        self.files.clear();
        self.grid = None;
        self.running = false;
        self.waiting = false;
        self.error = None;
        self.note = None;
    }

    pub(crate) fn is_open(&self) -> bool {
        self.window.is_some()
    }

    pub(crate) fn is(&self, id: WindowId) -> bool {
        self.window.as_ref().is_some_and(|w| w.id() == id)
    }

    pub(crate) fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    pub(crate) fn relabel(&mut self) {
        self.btn_calc.set_label(tr("hash.calc"));
        self.btn_copy.set_label(tr("hash.copyAll"));
        self.btn_close.set_label(tr(if self.running {
            "ops.cancel"
        } else {
            "license.btn.close"
        }));
        if let Some(w) = &self.window {
            w.set_title(&format!("{} — {}", crate::APP_TITLE, tr("hash.title")));
        }
        self.redraw();
    }

    pub(crate) fn tick(&mut self, now_ms: u64) -> bool {
        self.window.is_some()
            && (self.btn_close.tick(now_ms)
                | self.btn_copy.tick(now_ms)
                | self.btn_calc.tick(now_ms)
                | self.tb_cmp.tick(now_ms))
    }

    pub(crate) fn animating(&self) -> bool {
        self.window.is_some()
            && (self.btn_close.is_animating()
                || self.btn_copy.is_animating()
                || self.btn_calc.is_animating()
                || self.tb_cmp.is_animating())
    }

    /// 표로 보낼 입력(마우스 · 휠 · 방향키 — 비교 입력이 포커스면 글쇠는 보내지 않음).
    fn grid_input(&self, ev: &WindowEvent) -> Option<InputEvent> {
        let (x, y) = self.cursor;
        let key = |k: CtlKey| InputEvent::Key {
            key: k,
            shift: self.shift,
            primary: self.primary,
        };
        Some(match ev {
            WindowEvent::MouseWheel { delta, .. } => crate::input::wheel_event(delta, self.shift),
            WindowEvent::MouseInput { state, button, .. } => match (state, button) {
                (ElementState::Pressed, MouseButton::Left) => InputEvent::MouseDown {
                    x,
                    y,
                    shift: self.shift,
                    primary: self.primary,
                },
                (ElementState::Released, MouseButton::Left) => InputEvent::MouseUp { x, y },
                _ => return None,
            },
            WindowEvent::KeyboardInput { event: kev, .. }
                if kev.state == ElementState::Pressed && !self.tb_cmp.is_focused() =>
            {
                match kev.logical_key.as_ref() {
                    Key::Named(NamedKey::ArrowUp) => key(CtlKey::Up),
                    Key::Named(NamedKey::ArrowDown) => key(CtlKey::Down),
                    Key::Named(NamedKey::Home) => key(CtlKey::Home),
                    Key::Named(NamedKey::End) => key(CtlKey::End),
                    Key::Named(NamedKey::PageUp) => key(CtlKey::PageUp),
                    Key::Named(NamedKey::PageDown) => key(CtlKey::PageDown),
                    _ => return None,
                }
            }
            _ => return None,
        })
    }

    /// 표의 선택 행(없으면 전부)을 탭 구분으로.
    fn selected_tsv(&self) -> String {
        let Some(g) = &self.grid else {
            return self.results_text();
        };
        let src = g.source();
        let any = src.selected.iter().any(|s| *s);
        self.files
            .iter()
            .enumerate()
            .filter(|(i, _)| !any || src.selected[*i])
            .map(|(_, f)| Self::tsv_line(f))
            .collect::<Vec<_>>()
            .join("\r\n")
    }

    pub(crate) fn handle(&mut self, ev: &WindowEvent) -> HashAction {
        let mut inv = Invalidations::default();
        match ev {
            WindowEvent::CloseRequested => return HashAction::Close,
            WindowEvent::RedrawRequested => return HashAction::Paint,
            WindowEvent::Resized(_) => self.redraw(),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.redraw();
            }
            WindowEvent::Focused(false) => {
                for b in [&mut self.btn_close, &mut self.btn_copy, &mut self.btn_calc] {
                    b.clear_transient();
                }
                self.redraw();
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.primary = if cfg!(target_os = "macos") {
                    m.state().super_key()
                } else {
                    m.state().control_key()
                };
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as i32, position.y as i32);
                let (x, y) = self.cursor;
                let mv = InputEvent::MouseMove { x, y };
                for b in [&mut self.btn_close, &mut self.btn_copy, &mut self.btn_calc] {
                    b.on_event(&mv, &mut inv);
                }
                self.tb_cmp.on_event(&mv, &mut inv);
                if let Some(g) = &mut self.grid {
                    g.on_event(&mv, &mut inv);
                }
                // 단일 보기의 행 강조는 커서를 따라간다(속도 글은 끝나면 고정이라 다시 그려도 흔들리지 않는다).
                if !inv.is_empty() || (!self.multi() && !self.row_rects.is_empty()) {
                    self.redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } if *button == MouseButton::Left => {
                let (x, y) = self.cursor;
                let p = Point { x, y };
                let e = match state {
                    ElementState::Pressed => InputEvent::MouseDown {
                        x,
                        y,
                        shift: self.shift,
                        primary: self.primary,
                    },
                    ElementState::Released => InputEvent::MouseUp { x, y },
                };
                let up = matches!(e, InputEvent::MouseUp { .. });
                if !up {
                    self.tb_cmp.set_focused(self.tb_cmp.bounds().contains(p));
                    // 단일 보기: 행 클릭 = 그 값 복사.
                    if let Some(i) = self.row_rects.iter().position(|r| r.contains(p)) {
                        if let Some(v) = self
                            .files
                            .first()
                            .and_then(|f| f.values.get(i))
                            .and_then(|v| v.clone())
                        {
                            self.note = Some(trf("hash.copied", &[self.algos[i].name()]));
                            self.redraw();
                            return HashAction::CopyText(v);
                        }
                    }
                }
                if up || self.tb_cmp.bounds().contains(p) {
                    self.tb_cmp.on_event(&e, &mut inv);
                }
                for b in [&mut self.btn_close, &mut self.btn_copy, &mut self.btn_calc] {
                    if up || b.bounds().contains(p) {
                        b.on_event(&e, &mut inv);
                    }
                }
                let ie = self.grid_input(ev);
                if let (Some(g), Some(ie)) = (&mut self.grid, ie) {
                    if up || g.bounds().contains(p) {
                        g.on_event(&ie, &mut inv);
                    }
                }
                self.redraw();
                if self.btn_close.take_clicked() {
                    return HashAction::Close;
                }
                if self.btn_calc.take_clicked() {
                    return HashAction::Start;
                }
                if self.btn_copy.take_clicked() {
                    self.note = Some(trf("hash.copied", &[&tr("hash.copyAll")]));
                    return HashAction::CopyText(self.results_text());
                }
            }
            WindowEvent::MouseWheel { .. } => {
                let ie = self.grid_input(ev);
                if let (Some(g), Some(ie)) = (&mut self.grid, ie) {
                    g.on_event(&ie, &mut inv);
                    self.redraw();
                }
            }
            WindowEvent::Ime(Ime::Preedit(text, _)) => {
                self.tb_cmp.set_preedit(text, &mut inv);
                self.redraw();
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                self.tb_cmp.set_preedit("", &mut inv);
                for ch in text.chars().filter(|c| !c.is_control()) {
                    self.tb_cmp
                        .on_event(&InputEvent::Char { c: ch, now_ms: 0 }, &mut inv);
                }
                self.refresh_compare();
                self.redraw();
            }
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                if matches!(kev.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    return HashAction::Close;
                }
                let letter = if self.primary {
                    crate::input::shortcut_letter(kev)
                } else {
                    None
                };
                if letter == Some('c') && !self.tb_cmp.is_focused() && self.grid.is_some() {
                    self.note = Some(trf("hash.copied", &[&tr("hash.copyAll")]));
                    self.redraw();
                    return HashAction::CopyText(self.selected_tsv());
                }
                if letter == Some('a') {
                    if self.tb_cmp.is_focused() {
                        self.tb_cmp.on_event(&InputEvent::SelectAll, &mut inv);
                    } else if let Some(g) = &mut self.grid {
                        g.on_event(&InputEvent::SelectAll, &mut inv);
                    }
                    self.redraw();
                    return HashAction::None;
                }
                if self.tb_cmp.is_focused() {
                    if let Some(e) = crate::input::text_key_event(
                        kev,
                        self.shift,
                        self.primary,
                        crate::input::TextKeys::Line,
                    ) {
                        self.tb_cmp.on_event(&e, &mut inv);
                        self.refresh_compare();
                        self.redraw();
                    }
                } else {
                    let ie = self.grid_input(ev);
                    if let (Some(g), Some(ie)) = (&mut self.grid, ie) {
                        g.on_event(&ie, &mut inv);
                        self.redraw();
                    }
                }
            }
            _ => {}
        }
        HashAction::None
    }

    pub(crate) fn paint(&mut self, font: &Font, th: &Theme, ui_px: f32) {
        let Some(win) = self.window.clone() else {
            return;
        };
        let Some(mut surface) = self.surface.take() else {
            return;
        };
        let size = win.inner_size();
        let Some(mut buf) = surface.frame(size) else {
            self.surface = Some(surface);
            return;
        };
        let s = self.scale;
        let (wi, hi) = (size.width as i32, size.height as i32);
        let px = |v: f32| (v * s).round() as i32;
        {
            let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
            let mut prefs = FontPrefs::with_base(ui_px);
            prefs.peerlist = prefs.base;
            let mut dc = RasterCtx::new(&mut gfx, font, s).with_fonts(prefs);
            dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
            let inv = &mut Invalidations::default();
            let pad = px(PAD);
            let clip = Rect::new(0, 0, wi, hi);
            let mut y = pad;

            // 머리: 파일 이름(굵게) · 크기 — 여럿이면 "파일 N개 · 합계".
            dc.select_font(FontSlot::Base, true);
            let th_txt = dc.text_height();
            let head = if self.multi() {
                trf("hash.files", &[&self.files.len().to_string()])
            } else {
                self.files
                    .first()
                    .map(|f| ndir_ops::leaf_name(&f.path))
                    .unwrap_or_default()
            };
            dc.text(pad, y, clip, &head, th.text);
            let nw = dc.text_width(&head);
            dc.select_font(FontSlot::Base, false);
            dc.text(
                pad + nw + px(10.0),
                y,
                clip,
                &crate::dockinfo::fmt_size_long(self.total),
                th.text_dim,
            );
            y += th_txt + px(10.0);

            // 본문: 단일 = 알고리즘 행 · 여럿 = 표(남는 높이를 다 쓴다).
            let row_h = px(ROW_H);
            let tb_h = th_txt + px(10.0);
            let bottom_fixed = px(14.0 + 4.0)
                + th_txt
                + px(10.0)
                + tb_h
                + px(4.0)
                + th_txt
                + px(8.0)
                + px(BTN_H)
                + pad;
            if self.multi() {
                let grid_h = (hi - y - bottom_fixed).max(px(80.0));
                let rh = th_txt + px(4.0);
                if rh != self.row_h {
                    self.row_h = rh;
                    if let Some(g) = &mut self.grid {
                        g.set_metrics(rh, px(8.0), px(16.0), inv);
                    }
                }
                if let Some(g) = &mut self.grid {
                    let r = Rect::new(pad, y, wi - pad * 2, grid_h);
                    if g.bounds() != r {
                        g.set_bounds(r, inv);
                    }
                    g.paint(&mut dc, th);
                }
                self.row_rects.clear();
                y += grid_h + px(8.0);
            } else {
                let label_w = dc.text_width("SHA-256") + px(16.0);
                self.row_rects.clear();
                let hover = Point {
                    x: self.cursor.0,
                    y: self.cursor.1,
                };
                let f = self.files.first();
                for (i, a) in self.algos.iter().enumerate() {
                    let v = f.and_then(|f| f.values.get(i)).and_then(|v| v.clone());
                    let r = Rect::new(pad, y, wi - pad * 2, row_h);
                    if v.is_some() && r.contains(hover) {
                        dc.fill_round_rect(r, px(3.0), th.panel_bg_alt);
                    }
                    let ty = dc.text_center_y(y, row_h);
                    dc.text(pad + px(4.0), ty, clip, a.name(), th.text_dim);
                    let value = v.clone().unwrap_or_else(|| {
                        if self.error.is_some() || f.is_some_and(|f| f.error.is_some()) {
                            "\u{2013}".to_string()
                        } else if self.waiting {
                            tr("hash.waiting")
                        } else {
                            tr("hash.calculating")
                        }
                    });
                    let vx = pad + label_w;
                    let value = nexa_ctl::draw::ellipsize_middle(&mut dc, &value, wi - pad - vx);
                    dc.text(
                        vx,
                        ty,
                        clip,
                        &value,
                        if v.is_some() { th.text } else { th.text_dim },
                    );
                    self.row_rects.push(r);
                    y += row_h;
                }
                y += px(8.0);
            }

            // 진행 막대 + 글(읽은 양 · 비율 · 속도 — 끝나면 고정) 또는 오류/대기 안내.
            let bar_h = px(14.0);
            self.bar.set_scale(s);
            self.bar
                .set_bounds(Rect::new(pad, y, wi - pad * 2, bar_h), inv);
            self.bar.paint(&mut dc, th);
            y += bar_h + px(4.0);
            let file_err = self.files.iter().find_map(|f| f.error.clone());
            let line = if let Some(e) = self.error.clone().or(file_err.clone()) {
                if self.error.is_some() {
                    e
                } else {
                    trf("hash.failed", &[&e])
                }
            } else if self.waiting {
                trf(
                    "hash.overLimit",
                    &[&crate::filelist::format_size(self.total)],
                )
            } else {
                let secs = self
                    .elapsed_fixed
                    .unwrap_or_else(|| self.started.elapsed().as_secs_f64().max(0.001));
                let speed = (self.done as f64 / secs) as u64;
                let pct = (self.done * 100).checked_div(self.total).unwrap_or(100);
                let mut t = trf(
                    "hash.progress",
                    &[
                        &crate::filelist::format_size(self.done),
                        &crate::filelist::format_size(self.total),
                        &pct.to_string(),
                        &crate::filelist::format_size(speed),
                    ],
                );
                if self.multi() && self.running {
                    t.push_str(&format!(
                        " \u{00B7} {}/{}",
                        (self.current + 1).min(self.files.len()),
                        self.files.len()
                    ));
                }
                t
            };
            let line_color = if self.error.is_some() || file_err.is_some() {
                th.danger
            } else {
                th.text_dim
            };
            dc.text(pad, y, clip, &line, line_color);
            y += th_txt + px(10.0);

            // 비교 입력.
            let label = tr("hash.compare");
            let lw = dc.text_width(&label) + px(8.0);
            let ly = dc.text_center_y(y, tb_h);
            dc.text(pad, ly, clip, &label, th.text);
            self.tb_cmp.set_scale(s);
            self.tb_cmp
                .set_bounds(Rect::new(pad + lw, y, wi - pad * 2 - lw, tb_h), inv);
            self.tb_cmp.paint(&mut dc, th);
            y += tb_h + px(4.0);
            let (verdict, color) = match self.compare_state() {
                None => (String::new(), th.text_dim),
                Some(Err(())) => (tr("hash.unknownLen"), th.text_dim),
                Some(Ok((a, 0))) => (trf("hash.mismatch", &[a.name()]), th.danger),
                Some(Ok((a, n))) if self.multi() => {
                    (trf("hash.matchFiles", &[&n.to_string(), a.name()]), th.ok)
                }
                Some(Ok((a, _))) => (trf("hash.match", &[a.name()]), th.ok),
            };
            dc.text(pad + lw, y, clip, &verdict, color);

            // 바닥: 안내 · [계산](대기 중일 때만) [모두 복사] [취소/닫기].
            let btn_h = px(BTN_H);
            let by = hi - pad - btn_h;
            if let Some(n) = &self.note {
                let ny = dc.text_center_y(by, btn_h);
                dc.text(pad, ny, clip, n, th.text_dim);
            }
            let mut x = wi - pad;
            let show_calc = self.waiting;
            for (b, show) in [
                (&mut self.btn_close, true),
                (&mut self.btn_copy, true),
                (&mut self.btn_calc, show_calc),
            ] {
                b.set_scale(s);
                if !show {
                    b.set_bounds(Rect::new(0, 0, 0, 0), inv);
                    continue;
                }
                let bw = dc.text_width(b.label()) + px(28.0);
                x -= bw;
                b.set_bounds(Rect::new(x, by, bw, btn_h), inv);
                b.paint(&mut dc, th);
                x -= px(8.0);
            }
        }
        let _ = buf.present();
        self.surface = Some(surface);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn en() {
        ndir_i18n::activate(ndir_i18n::load("en", std::path::Path::new("nowhere")));
    }

    /// 파일 1개: 시작 → 계산 중 · 결과 → 값 · 속도는 끝난 시점에 고정 · [모두 복사] 글 · 비교 입력 판정 · 오류.
    #[test]
    fn single_results_and_compare_without_window() {
        en();
        let mut w = HashWin::new();
        w.start(
            &[(PathBuf::from("D:/x/report.txt"), 3)],
            &[Algo::Crc32, Algo::Sha256],
            true,
        );
        assert!(w.is_running() && !w.is_waiting() && w.results_text().is_empty());
        w.update(2, 0);
        w.set_file_result(
            0,
            Ok(vec![
                (Algo::Crc32, "352441c2".into()),
                (
                    Algo::Sha256,
                    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
                ),
            ]),
        );
        w.finish(None);
        assert!(!w.is_running());
        let fixed = w.elapsed_fixed.expect("속도 고정");
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert_eq!(
            w.elapsed_fixed,
            Some(fixed),
            "끝난 뒤에는 경과가 늘지 않는다"
        );
        let text = w.results_text();
        assert!(text.starts_with("CRC32    352441c2  report.txt"), "{text}");
        assert_eq!(w.compare_state(), None);
        w.set_compare_text(" 3524-41C2 ");
        assert_eq!(w.compare_state(), Some(Ok((Algo::Crc32, 1))));
        w.set_compare_text("00000000");
        assert_eq!(w.compare_state(), Some(Ok((Algo::Crc32, 0))));
        w.set_compare_text("abc");
        assert_eq!(w.compare_state(), None);
        w.set_compare_text("zz");
        assert_eq!(w.compare_state(), Some(Err(())));
        w.start(&[(PathBuf::from("D:/x/b"), 10)], &[Algo::Md5], true);
        w.set_file_result(0, Err("boom".into()));
        w.finish(None);
        assert!(w.results_text().is_empty());
        w.close();
        assert!(w.results().is_empty());
    }

    /// 여러 파일: 표 모드 · 내용이 같은 파일은 같은 묶음 글자 · 혼자면 빈 값 · 비교 입력 = 일치 파일 수 + ✓ · 탭 구분 복사 ·
    /// 상한 초과 = [계산] 대기 → begin.
    #[test]
    fn multi_groups_and_compare() {
        en();
        let mut w = HashWin::new();
        let files = [
            (PathBuf::from("D:/a/1.txt"), 5u64),
            (PathBuf::from("D:/b/2.txt"), 5),
            (PathBuf::from("D:/c/3.txt"), 7),
        ];
        w.start(&files, &[Algo::Crc32, Algo::Md5], false);
        assert!(w.is_waiting() && !w.is_running());
        w.begin();
        assert!(w.is_running() && !w.is_waiting());
        let same = "d41d8cd98f00b204e9800998ecf8427e";
        let crc = |s: &str| (Algo::Crc32, s.to_string());
        w.set_file_result(0, Ok(vec![crc("00000001"), (Algo::Md5, same.into())]));
        w.set_file_result(1, Ok(vec![crc("00000001"), (Algo::Md5, same.into())]));
        w.set_file_result(
            2,
            Ok(vec![
                crc("00000002"),
                (Algo::Md5, "ffffffffffffffffffffffffffffffff".into()),
            ]),
        );
        w.finish(None);
        assert_eq!(w.group_of(0), Some('A'));
        assert_eq!(w.group_of(1), Some('A'));
        assert_eq!(w.group_of(2), None, "혼자인 묶음은 글자 없음");
        w.set_compare_text(same);
        assert_eq!(w.compare_state(), Some(Ok((Algo::Md5, 2))));
        let g = w.grid.as_ref().unwrap().source();
        assert_eq!(g.matched, vec![true, true, false]);
        let same_cell = g.cell(0, COL_SAME);
        assert!(
            same_cell.contains('A') && same_cell.contains('\u{2713}'),
            "{same_cell}"
        );
        let text = w.results_text();
        assert!(text.starts_with("name\tsize\tCRC32\tMD5"), "{text}");
        assert_eq!(text.lines().count(), 4);
        assert_eq!(w.selected_tsv().lines().count(), 3, "선택 없음 = 전부");
        w.close();
    }
}
