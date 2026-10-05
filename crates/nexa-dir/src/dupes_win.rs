//! 중복 파일 창(T-170 · NEW-041 · dir3 신규 · 사용자 10-05): 묶음(크기 · 개수 · 되찾을 용량) 헤더 아래 파일 행 · **삭제 표시(✓)**
//! · 보존 규칙(최신 · 가장 오래된 · 가장 얕은 경로 · 첫 번째 — 묶음마다 하나는 남긴다) · [휴지통으로 보내기 (N)] · [닫기].
//! 표시 토글 = ✓ 열 클릭 · Space(선택 행) · 묶음 헤더의 ▾ = 접기. 계산은 호스트 작업 스레드(`app/dupes.rs`)가 하고 이 창은
//! 진행 글과 결과만 받는다. 묶음의 모든 파일에 표시를 하면 그 묶음은 하나도 지우지 않는다(헤더에 경고).

use ndir_i18n::{tr, trf};
use ndir_ops::dupes::{keep_index, DupGroup, Keep};
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::{
    Button, Combo, ComboControl, ComboItem, Control, InputEvent, Invalidations, Key as CtlKey,
    Widget,
};
use nexa_gfx::{Font, Surface};
use nexa_grid::{Column, Marker, RowItem, RowSource, SelectOp, VirtualRows};
use std::path::PathBuf;
use std::rc::Rc;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DupAction {
    None,
    Paint,
    Close,
    /// 표시한 파일들을 휴지통으로.
    Trash(Vec<PathBuf>),
}

const PAD: f32 = 10.0;
const BTN_H: f32 = 28.0;
const COL_MARK: u32 = 1;
const COL_SIZE: u32 = 2;
const COL_MTIME: u32 = 3;

/// 한 행 — 묶음 헤더(`path = None`) 또는 파일.
#[derive(Debug, Clone)]
struct DupRow {
    group: usize,
    path: Option<PathBuf>,
    mtime: i64,
    marked: bool,
}

#[derive(Debug)]
pub(crate) struct DupSource {
    groups: Vec<DupGroup>,
    /// 묶음별 파일 수정 시각(초 · 모르면 0).
    mtimes: Vec<Vec<i64>>,
    /// 묶음별 표시 상태(파일 순서).
    marks: Vec<Vec<bool>>,
    collapsed: Vec<bool>,
    /// 보이는 행(헤더 + 펼친 묶음의 파일).
    rows: Vec<DupRow>,
    selected: Vec<bool>,
    anchor: Option<usize>,
    tz: i32,
}

impl DupSource {
    fn new(groups: Vec<DupGroup>, mtimes: Vec<Vec<i64>>, tz: i32) -> Self {
        let marks = groups.iter().map(|g| vec![false; g.paths.len()]).collect();
        let collapsed = vec![false; groups.len()];
        let mut s = DupSource {
            groups,
            mtimes,
            marks,
            collapsed,
            rows: Vec::new(),
            selected: Vec::new(),
            anchor: None,
            tz,
        };
        s.rebuild();
        s
    }

    fn rebuild(&mut self) {
        self.rows.clear();
        for (gi, g) in self.groups.iter().enumerate() {
            self.rows.push(DupRow {
                group: gi,
                path: None,
                mtime: 0,
                marked: false,
            });
            if self.collapsed[gi] {
                continue;
            }
            for (pi, p) in g.paths.iter().enumerate() {
                self.rows.push(DupRow {
                    group: gi,
                    path: Some(p.clone()),
                    mtime: self.mtimes[gi].get(pi).copied().unwrap_or(0),
                    marked: self.marks[gi][pi],
                });
            }
        }
        self.selected = vec![false; self.rows.len()];
        self.anchor = None;
    }

    /// 보존 규칙 적용 — 묶음마다 하나만 남기고 전부 표시.
    fn apply_keep(&mut self, keep: Keep) {
        for (gi, g) in self.groups.iter().enumerate() {
            let k = keep_index(&g.paths, &self.mtimes[gi], keep);
            for (pi, m) in self.marks[gi].iter_mut().enumerate() {
                *m = pi != k;
            }
        }
        self.rebuild();
    }

    fn toggle_mark_row(&mut self, row: usize) -> bool {
        let Some(r) = self.rows.get(row) else {
            return false;
        };
        let Some(p) = &r.path else {
            return false;
        };
        let gi = r.group;
        if let Some(pi) = self.groups[gi].paths.iter().position(|x| x == p) {
            self.marks[gi][pi] = !self.marks[gi][pi];
            self.rows[row].marked = self.marks[gi][pi];
            return true;
        }
        false
    }

    /// 묶음의 파일 전부에 표시가 됐는가(= 하나도 지우지 않는다 · 경고).
    fn all_marked(&self, gi: usize) -> bool {
        !self.marks[gi].is_empty() && self.marks[gi].iter().all(|&m| m)
    }

    /// 지울 파일(묶음 전체 표시는 제외).
    pub(crate) fn marked_paths(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for (gi, g) in self.groups.iter().enumerate() {
            if self.all_marked(gi) {
                continue;
            }
            for (pi, p) in g.paths.iter().enumerate() {
                if self.marks[gi][pi] {
                    out.push(p.clone());
                }
            }
        }
        out
    }

    /// 지운 경로를 목록에서 뺀다(1개만 남은 묶음은 사라진다).
    fn drop_paths(&mut self, gone: &[PathBuf]) {
        let mut groups = Vec::new();
        let mut mtimes = Vec::new();
        let mut marks = Vec::new();
        let mut collapsed = Vec::new();
        for (gi, g) in self.groups.iter().enumerate() {
            let keep: Vec<usize> = (0..g.paths.len())
                .filter(|&pi| !gone.contains(&g.paths[pi]))
                .collect();
            if keep.len() < 2 {
                continue;
            }
            groups.push(DupGroup {
                size: g.size,
                sha256: g.sha256.clone(),
                paths: keep.iter().map(|&pi| g.paths[pi].clone()).collect(),
            });
            mtimes.push(keep.iter().map(|&pi| self.mtimes[gi][pi]).collect());
            marks.push(keep.iter().map(|&pi| self.marks[gi][pi]).collect());
            collapsed.push(self.collapsed[gi]);
        }
        self.groups = groups;
        self.mtimes = mtimes;
        self.marks = marks;
        self.collapsed = collapsed;
        self.rebuild();
    }

    fn set_range(&mut self, lo: usize, hi: usize) {
        for (i, s) in self.selected.iter_mut().enumerate() {
            *s = (lo..=hi).contains(&i);
        }
    }

    fn header_text(&self, gi: usize) -> String {
        let g = &self.groups[gi];
        let mut s = trf(
            "dupes.group",
            &[
                &(gi + 1).to_string(),
                &crate::filelist::format_size(g.size),
                &g.paths.len().to_string(),
                &crate::filelist::format_size(g.reclaimable()),
            ],
        );
        if self.all_marked(gi) {
            s.push_str(" — ");
            s.push_str(&tr("dupes.allMarked"));
        }
        s
    }
}

impl RowSource for DupSource {
    fn len(&self) -> usize {
        self.rows.len()
    }
    fn row(&self, index: usize) -> RowItem {
        let r = &self.rows[index];
        match &r.path {
            None => RowItem {
                text: self.header_text(r.group),
                is_dir: true,
                depth: 0,
                marker: if self.collapsed[r.group] {
                    Marker::Collapsed
                } else {
                    Marker::Expanded
                },
            },
            Some(p) => RowItem {
                text: crate::filelist::display_path(p),
                is_dir: false,
                depth: 1,
                marker: Marker::None,
            },
        }
    }
    fn cell(&self, index: usize, key: u32) -> String {
        let r = &self.rows[index];
        if r.path.is_none() {
            return String::new();
        }
        match key {
            COL_MARK => {
                if r.marked {
                    "✓".into()
                } else {
                    String::new()
                }
            }
            COL_SIZE => crate::filelist::format_size(self.groups[r.group].size),
            COL_MTIME => {
                if r.mtime > 0 {
                    ndir_ops::batch_rename::format_date(
                        "${YYYY}-${MM}-${DD} ${hh}:${mm}",
                        r.mtime * 1000,
                        self.tz,
                    )
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        }
    }
    /// 헤더의 ▾/▸ = 묶음 접기 · 파일 행은 표시 토글(그리드가 마커 자리 클릭에 부른다).
    fn toggle(&mut self, index: usize) -> bool {
        let Some(r) = self.rows.get(index) else {
            return false;
        };
        if r.path.is_none() {
            let gi = r.group;
            self.collapsed[gi] = !self.collapsed[gi];
            self.rebuild();
            return true;
        }
        self.toggle_mark_row(index)
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
                self.set_range(index, index);
                self.anchor = Some(index);
            }
            SelectOp::Toggle => {
                self.selected[index] = !self.selected[index];
                self.anchor = Some(index);
            }
            SelectOp::RangeTo => {
                let a = self.anchor.unwrap_or(index);
                self.set_range(a.min(index), a.max(index));
            }
        }
        true
    }
    fn select_span(&mut self, lo: usize, hi: usize) -> bool {
        if self.rows.is_empty() {
            return false;
        }
        let hi = hi.min(self.rows.len() - 1);
        self.set_range(lo.min(hi), hi);
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

const KEEPS: [(&str, &str, Keep); 4] = [
    ("newest", "dupes.keepNewest", Keep::Newest),
    ("oldest", "dupes.keepOldest", Keep::Oldest),
    ("shortest", "dupes.keepShortest", Keep::ShortestPath),
    ("first", "dupes.keepFirst", Keep::First),
];

pub(crate) struct DupesWin {
    window: Option<Rc<Window>>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    cursor: (i32, i32),
    shift: bool,
    primary: bool,
    rows: Option<VirtualRows<DupSource>>,
    status: String,
    running: bool,
    row_h: i32,
    keep: Combo,
    keep_value: String,
    btn_trash: Button,
    btn_close: Button,
    tz: i32,
}

impl DupesWin {
    pub(crate) fn new() -> Self {
        DupesWin {
            window: None,
            surface: None,
            scale: 1.0,
            cursor: (0, 0),
            shift: false,
            primary: false,
            rows: None,
            status: String::new(),
            running: false,
            row_h: 0,
            keep: Self::keep_combo(),
            keep_value: "newest".into(),
            btn_trash: Button::new(trf("dupes.trash", &["0"])),
            btn_close: Button::new(tr("license.btn.close")),
            tz: 0,
        }
    }

    fn keep_combo() -> Combo {
        Combo::new(
            KEEPS
                .iter()
                .map(|(v, k, _)| ComboItem::new(*v, tr(k)))
                .collect(),
            0,
        )
    }

    fn keep_rule(&self) -> Keep {
        KEEPS
            .iter()
            .find(|(v, _, _)| *v == self.keep_value)
            .map_or(Keep::Newest, |(_, _, k)| *k)
    }

    /// 새 검색 시작(진행 글만 · 결과는 비움).
    pub(crate) fn start(&mut self, tz: i32) {
        self.tz = tz;
        self.rows = None;
        self.running = true;
        self.status = trf("dupes.scanning", &["0", "0", "0 B"]);
        self.btn_trash.set_enabled(false);
        self.redraw();
    }

    pub(crate) fn set_progress(&mut self, scanned: u64, candidates: u64, hashed: u64) {
        let s = trf(
            "dupes.scanning",
            &[
                &scanned.to_string(),
                &candidates.to_string(),
                &crate::filelist::format_size(hashed),
            ],
        );
        if s != self.status {
            self.status = s;
            self.redraw();
        }
    }

    /// 결과 — 묶음 + 묶음별 수정 시각 · 보존 규칙을 바로 적용한다.
    pub(crate) fn set_result(&mut self, groups: Vec<DupGroup>, mtimes: Vec<Vec<i64>>) {
        self.running = false;
        let mut src = DupSource::new(groups, mtimes, self.tz);
        src.apply_keep(self.keep_rule());
        let mut inv = Invalidations::default();
        let mut rows = VirtualRows::new(src, self.row_h.max(20), 8, 16);
        let cols = [
            ("col.name", 420),
            ("dupes.col.mark", 48),
            ("col.size", 96),
            ("col.modified", 140),
        ];
        rows.set_columns(
            cols.iter()
                .enumerate()
                .map(|(i, (k, w))| {
                    let c = Column::new(i as u32, tr(k), *w);
                    if i >= 2 {
                        c.right_aligned()
                    } else {
                        c
                    }
                })
                .collect(),
            &mut inv,
        );
        rows.set_font_decor(false, true, false, &mut inv);
        self.rows = Some(rows);
        self.refresh_summary();
        self.layout();
        self.redraw();
    }

    pub(crate) fn set_error(&mut self, text: &str) {
        self.running = false;
        self.status = text.to_string();
        self.redraw();
    }

    fn refresh_summary(&mut self) {
        let Some(r) = &self.rows else {
            return;
        };
        let src = r.source();
        let dup_count: usize = src.groups.iter().map(|g| g.paths.len() - 1).sum();
        let reclaim: u64 = src.groups.iter().map(DupGroup::reclaimable).sum();
        self.status = if src.groups.is_empty() {
            tr("dupes.none")
        } else {
            trf(
                "dupes.summary",
                &[
                    &src.groups.len().to_string(),
                    &dup_count.to_string(),
                    &crate::filelist::format_size(reclaim),
                ],
            )
        };
        let n = src.marked_paths().len();
        self.btn_trash
            .set_label(trf("dupes.trash", &[&n.to_string()]));
        self.btn_trash.set_enabled(n > 0);
    }

    /// 지운 경로를 목록에서 뺀다(호스트가 휴지통 뒤에 부른다).
    pub(crate) fn drop_paths(&mut self, gone: &[PathBuf]) {
        let mut inv = Invalidations::default();
        if let Some(r) = &mut self.rows {
            r.source_mut().drop_paths(gone);
            r.on_event(
                &InputEvent::Key {
                    key: CtlKey::Home,
                    shift: false,
                    primary: false,
                },
                &mut inv,
            );
        }
        self.refresh_summary();
        self.redraw();
    }

    #[cfg(test)]
    pub(crate) fn is_running(&self) -> bool {
        self.running
    }

    #[cfg(test)]
    pub(crate) fn rows_len(&self) -> usize {
        self.rows.as_ref().map_or(0, |r| r.source().len())
    }

    pub(crate) fn marked_paths(&self) -> Vec<PathBuf> {
        self.rows
            .as_ref()
            .map_or_else(Vec::new, |r| r.source().marked_paths())
    }

    #[cfg(test)]
    pub(crate) fn status(&self) -> &str {
        &self.status
    }

    /// 표시 토글(시험 · Space): 선택 행이 없으면 캐럿 행.
    pub(crate) fn toggle_selected_marks(&mut self) {
        let Some(r) = &mut self.rows else {
            return;
        };
        let caret = r.caret();
        let src = r.source_mut();
        let mut rows: Vec<usize> = (0..src.rows.len()).filter(|&i| src.selected[i]).collect();
        if rows.is_empty() {
            rows.extend(caret);
        }
        for i in rows {
            src.toggle_mark_row(i);
        }
        self.refresh_summary();
        self.redraw();
    }

    pub(crate) fn set_keep(&mut self, value: &str) {
        self.keep_value = value.to_string();
        self.keep.select_value(value);
        let rule = self.keep_rule();
        if let Some(r) = &mut self.rows {
            r.source_mut().apply_keep(rule);
        }
        self.refresh_summary();
        self.redraw();
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
        let (lw, lh) = over.map_or((900.0, 600.0), |(_, _, w, h)| {
            (
                (w as f32 * 0.7).clamp(560.0, 1400.0),
                (h as f32 * 0.7).clamp(360.0, 1000.0),
            )
        });
        let mut attrs = Window::default_attributes()
            .with_title(format!("Nexa Dir — {}", tr("dupes.title")))
            .with_theme(theme)
            .with_resizable(true)
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
        self.scale = win.scale_factor() as f32;
        self.surface = crate::present::Presenter::new(win.clone()).ok();
        self.window = Some(win);
        self.layout();
        self.redraw();
    }

    pub(crate) fn close(&mut self) {
        self.surface = None;
        self.window = None;
        self.rows = None;
        self.running = false;
        self.btn_trash.clear_transient();
        self.btn_close.clear_transient();
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
        self.btn_close.set_label(tr("license.btn.close"));
        self.keep = Self::keep_combo();
        self.keep.select_value(&self.keep_value.clone());
        self.refresh_summary();
        if let Some(w) = &self.window {
            w.set_title(&format!("Nexa Dir — {}", tr("dupes.title")));
        }
        self.redraw();
    }

    pub(crate) fn tick(&mut self, now_ms: u64) -> bool {
        self.window.is_some() && (self.btn_trash.tick(now_ms) | self.btn_close.tick(now_ms))
    }

    pub(crate) fn animating(&self) -> bool {
        self.window.is_some() && (self.btn_trash.is_animating() || self.btn_close.is_animating())
    }

    fn bottom_h(&self) -> i32 {
        ((BTN_H + PAD * 2.0) * self.scale).round() as i32 + self.row_h.max(16)
    }

    fn layout(&mut self) {
        let Some(win) = &self.window else { return };
        let size = win.inner_size();
        let (w, h) = (size.width as i32, size.height as i32);
        let gh = (h - self.bottom_h()).max(10);
        if let Some(r) = &mut self.rows {
            let mut inv = Invalidations::default();
            r.set_bounds(Rect::new(0, 0, w, gh), &mut inv);
        }
    }

    fn to_input(&self, ev: &WindowEvent) -> Option<InputEvent> {
        let (x, y) = self.cursor;
        let key = |k: CtlKey| InputEvent::Key {
            key: k,
            shift: self.shift,
            primary: self.primary,
        };
        Some(match ev {
            WindowEvent::CursorMoved { position, .. } => InputEvent::MouseMove {
                x: position.x as i32,
                y: position.y as i32,
            },
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
            WindowEvent::MouseWheel { delta, .. } => crate::input::wheel_event(delta, self.shift),
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                match kev.logical_key.as_ref() {
                    Key::Named(NamedKey::ArrowUp) => key(CtlKey::Up),
                    Key::Named(NamedKey::ArrowDown) => key(CtlKey::Down),
                    Key::Named(NamedKey::ArrowLeft) => key(CtlKey::Left),
                    Key::Named(NamedKey::ArrowRight) => key(CtlKey::Right),
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

    /// ✓ 열의 x 범위(가로 스크롤 반영).
    fn mark_col_range(&self) -> Option<(i32, i32)> {
        let r = self.rows.as_ref()?;
        let cols = r.columns();
        let x0 = r.bounds().x - r.scroll_x() + cols.first()?.width;
        Some((x0, x0 + cols.get(1)?.width))
    }

    pub(crate) fn handle(&mut self, ev: &WindowEvent) -> DupAction {
        let mut inv = Invalidations::default();
        match ev {
            WindowEvent::CloseRequested => return DupAction::Close,
            WindowEvent::RedrawRequested => return DupAction::Paint,
            WindowEvent::Resized(_) => {
                self.layout();
                self.redraw();
                return DupAction::None;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.layout();
                self.redraw();
                return DupAction::None;
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.primary = if cfg!(target_os = "macos") {
                    m.state().super_key()
                } else {
                    m.state().control_key()
                };
                return DupAction::None;
            }
            WindowEvent::Focused(false) => {
                self.btn_trash.clear_transient();
                self.btn_close.clear_transient();
                self.redraw();
                return DupAction::None;
            }
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                if matches!(kev.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    if self.keep.is_open() {
                        self.keep.close(&mut inv);
                        self.redraw();
                        return DupAction::None;
                    }
                    return DupAction::Close;
                }
                if matches!(kev.logical_key.as_ref(), Key::Named(NamedKey::Space)) {
                    self.toggle_selected_marks();
                    return DupAction::None;
                }
                if self.primary && crate::input::shortcut_letter(kev) == Some('a') {
                    if let Some(r) = &mut self.rows {
                        r.on_event(&InputEvent::SelectAll, &mut inv);
                    }
                    self.redraw();
                    return DupAction::None;
                }
            }
            _ => {}
        }
        if let WindowEvent::CursorMoved { position, .. } = ev {
            self.cursor = (position.x as i32, position.y as i32);
            let (x, y) = self.cursor;
            let mv = InputEvent::MouseMove { x, y };
            self.btn_trash.on_event(&mv, &mut inv);
            self.btn_close.on_event(&mv, &mut inv);
            self.keep.on_event(&mv, &mut inv);
        }
        if let WindowEvent::MouseInput { state, button, .. } = ev {
            if *button == MouseButton::Left {
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
                // 콤보가 열려 있으면 콤보가 전부 받는다.
                if self.keep.is_open() || self.keep.bounds().contains(p) || up {
                    self.keep.on_event(&e, &mut inv);
                    let v = self.keep.selected_value();
                    if v != self.keep_value {
                        self.set_keep(&v);
                    }
                    if self.keep.is_open() {
                        self.redraw();
                        return DupAction::None;
                    }
                }
                for b in [&mut self.btn_trash, &mut self.btn_close] {
                    if up || b.bounds().contains(p) {
                        b.on_event(&e, &mut inv);
                    }
                }
                if self.btn_close.take_clicked() {
                    return DupAction::Close;
                }
                if self.btn_trash.take_clicked() {
                    return DupAction::Trash(self.marked_paths());
                }
                // ✓ 열 클릭 = 표시 토글(선택은 바꾸지 않는다).
                if !up {
                    if let (Some((x0, x1)), Some(r)) = (self.mark_col_range(), &mut self.rows) {
                        if x >= x0 && x < x1 && r.bounds().contains(p) {
                            if let Some(row) = r.row_at(x, y) {
                                r.source_mut().toggle_mark_row(row);
                                self.refresh_summary();
                                self.redraw();
                                return DupAction::None;
                            }
                        }
                    }
                }
            }
        }
        let Some(ie) = self.to_input(ev) else {
            self.redraw();
            return DupAction::None;
        };
        if let Some(r) = &mut self.rows {
            if r.bounds().contains(Point {
                x: self.cursor.0,
                y: self.cursor.1,
            }) || matches!(ie, InputEvent::Key { .. } | InputEvent::MouseUp { .. })
            {
                r.on_event(&ie, &mut inv);
            }
        }
        self.refresh_summary();
        self.redraw();
        DupAction::None
    }

    pub(crate) fn paint(&mut self, ui: &Font, th: &Theme, font_px: f32) {
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
        let pad = (PAD * s).round() as i32;
        {
            let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
            let mut prefs = FontPrefs::with_base(font_px);
            prefs.peerlist = prefs.base;
            let mut dc = RasterCtx::new(&mut gfx, ui, s).with_fonts(prefs);
            dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
            dc.select_font(FontSlot::Base, false);
            let th_txt = dc.text_height();
            let row_h = th_txt + (4.0 * s).round() as i32;
            if row_h != self.row_h {
                self.row_h = row_h;
                let mut inv = Invalidations::default();
                if let Some(r) = &mut self.rows {
                    r.set_metrics(
                        row_h,
                        (8.0 * s).round() as i32,
                        (16.0 * s).round() as i32,
                        &mut inv,
                    );
                }
            }
            let bottom_h = self.bottom_h();
            let gh = (hi - bottom_h).max(10);
            let inv = &mut Invalidations::default();
            if let Some(r) = &mut self.rows {
                if r.bounds().h != gh || r.bounds().w != wi {
                    r.set_bounds(Rect::new(0, 0, wi, gh), inv);
                }
                r.paint(&mut dc, th);
            }
            // 상태 줄.
            dc.select_font(FontSlot::Base, false);
            let sy = gh + (4.0 * s).round() as i32;
            let status = nexa_ctl::draw::ellipsize_middle(&mut dc, &self.status, wi - pad * 2);
            dc.text(pad, sy, Rect::new(0, 0, wi, hi), &status, th.text_dim);
            // 바닥: 보존 규칙 콤보 · [휴지통으로 보내기 (N)] · [닫기].
            let btn_h = (BTN_H * s).round() as i32;
            let by = hi - pad - btn_h;
            let label = tr("dupes.keep");
            let lw = dc.text_width(&label) + (8.0 * s).round() as i32;
            let ly = dc.text_center_y(by, btn_h);
            dc.text(pad, ly, Rect::new(0, 0, wi, hi), &label, th.text);
            self.keep.set_scale(s);
            self.keep.set_bounds(
                Rect::new(pad + lw, by, (220.0 * s).round() as i32, btn_h),
                inv,
            );
            let mut x = wi - pad;
            for b in [&mut self.btn_close, &mut self.btn_trash] {
                b.set_scale(s);
                let bw = dc.text_width(b.label()) + (28.0 * s).round() as i32;
                x -= bw;
                b.set_bounds(Rect::new(x, by, bw, btn_h), inv);
                b.paint(&mut dc, th);
                x -= (8.0 * s).round() as i32;
            }
            self.keep.paint(&mut dc, th);
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

    fn g(size: u64, paths: &[&str]) -> DupGroup {
        DupGroup {
            size,
            sha256: "x".into(),
            paths: paths.iter().map(PathBuf::from).collect(),
        }
    }

    /// 창 없이: 결과 → 묶음마다 하나만 남기고 표시(최신 보존) · 규칙 바꾸기 · 묶음 전부 표시 = 지우지 않음 · 접기 · 지운 경로 제거.
    #[test]
    fn marks_follow_keep_rule_and_guard_whole_groups() {
        en();
        let mut w = DupesWin::new();
        w.start(0);
        assert!(w.is_running());
        w.set_result(
            vec![
                g(100, &["D:/a/1.txt", "D:/b/1.txt", "D:/c/1.txt"]),
                g(10, &["D:/x", "D:/y"]),
            ],
            vec![vec![1, 3, 2], vec![5, 5]],
        );
        assert!(!w.is_running());
        assert_eq!(w.rows_len(), 2 + 3 + 2);
        let marked = w.marked_paths();
        assert_eq!(marked.len(), 3, "{marked:?}");
        assert!(
            !marked.contains(&PathBuf::from("D:/b/1.txt")),
            "최신(3) 보존"
        );
        assert!(
            w.status().contains('2') && w.status().contains("210"),
            "{}",
            w.status()
        );
        w.set_keep("oldest");
        let marked = w.marked_paths();
        assert!(!marked.contains(&PathBuf::from("D:/a/1.txt")) && marked.len() == 3);
        w.set_keep("shortest");
        assert!(!w.marked_paths().contains(&PathBuf::from("D:/x")));
        // 묶음 2의 남은 하나(보존 중인 D:/x)까지 표시 → 그 묶음은 제외.
        let rows = w.rows.as_mut().unwrap();
        let idx = rows
            .source()
            .rows
            .iter()
            .position(|r| r.path.as_deref() == Some(std::path::Path::new("D:/x")))
            .unwrap();
        rows.source_mut().toggle_mark_row(idx);
        assert_eq!(w.marked_paths().len(), 2);
        assert!(w
            .rows
            .as_ref()
            .unwrap()
            .source()
            .row(4)
            .text
            .contains("every file"));
        // 접기 = 파일 행이 사라진다.
        w.rows.as_mut().unwrap().source_mut().toggle(0);
        assert_eq!(w.rows_len(), 1 + 1 + 2);
        // 지운 경로 제거 → 묶음 1은 1개 남아 사라진다.
        w.drop_paths(&[PathBuf::from("D:/a/1.txt"), PathBuf::from("D:/c/1.txt")]);
        assert_eq!(w.rows_len(), 1 + 2);
        w.close();
        assert_eq!(w.rows_len(), 0);
    }
}
