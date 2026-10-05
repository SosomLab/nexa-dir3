//! 폴더 비교 창(T-171/172 · NEW-042/043 · dir3 신규 · 사용자 10-05): 왼쪽(활성 패널) · 오른쪽(반대 패널) 폴더의 항목을 상대 경로로
//! 맞춰 한 줄에 — 상태 기호(▷ 왼쪽만 · ◁ 오른쪽만 · = 같음 · → 왼쪽이 새로움 · ← 오른쪽이 새로움 · ≠ 다름 · ✕ 종류 불일치).
//! 필터(다른 것만 / 전부) · 내용 비교 다시 실행 · 동기화 버튼 3개(→ 왼쪽 기준 · ← 오른쪽 기준 · ⇄ 새로운 쪽) = 선택 행이 있으면
//! 그 행만, 없으면 전부. 실행은 호스트(`app/compare.rs`)가 전송 엔진으로.

use ndir_i18n::{tr, trf};
use ndir_ops::compare::{CmpEntry, Direction, Meta, Verdict};
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
pub(crate) enum CmpAction {
    None,
    Paint,
    Close,
    /// 내용 비교로 다시 실행.
    Rescan(bool),
    /// 동기화(방향 · 상대 경로들 — 비어 있으면 전부).
    Sync(Direction, Vec<String>),
}

const PAD: f32 = 10.0;
const BTN_H: f32 = 28.0;

#[derive(Debug)]
pub(crate) struct CmpSource {
    all: Vec<CmpEntry>,
    /// 보이는 행 → `all` 인덱스.
    view: Vec<usize>,
    selected: Vec<bool>,
    anchor: Option<usize>,
    tz: i32,
}

impl CmpSource {
    fn new(all: Vec<CmpEntry>, only_diff: bool, tz: i32) -> Self {
        let mut s = CmpSource {
            all,
            view: Vec::new(),
            selected: Vec::new(),
            anchor: None,
            tz,
        };
        s.filter(only_diff);
        s
    }

    fn filter(&mut self, only_diff: bool) {
        self.view = (0..self.all.len())
            .filter(|&i| !only_diff || self.all[i].verdict.differs())
            .collect();
        self.selected = vec![false; self.view.len()];
        self.anchor = None;
    }

    fn set_range(&mut self, lo: usize, hi: usize) {
        for (i, s) in self.selected.iter_mut().enumerate() {
            *s = (lo..=hi).contains(&i);
        }
    }

    /// 선택 행의 상대 경로.
    fn selected_rels(&self) -> Vec<String> {
        self.view
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected[*i])
            .map(|(_, &a)| self.all[a].rel.clone())
            .collect()
    }

    fn fmt_meta(&self, m: Option<Meta>, key: u32) -> String {
        let Some(m) = m else {
            return String::new();
        };
        if key % 2 == 1 {
            if m.is_dir {
                tr("kind.folder")
            } else {
                crate::filelist::format_size(m.size)
            }
        } else if m.mtime > 0 {
            ndir_ops::batch_rename::format_date(
                "${YYYY}-${MM}-${DD} ${hh}:${mm}",
                m.mtime * 1000,
                self.tz,
            )
        } else {
            String::new()
        }
    }
}

impl RowSource for CmpSource {
    fn len(&self) -> usize {
        self.view.len()
    }
    fn row(&self, index: usize) -> RowItem {
        let e = &self.all[self.view[index]];
        RowItem {
            text: e.rel.clone(),
            is_dir: e.left.or(e.right).is_some_and(|m| m.is_dir),
            depth: 0,
            marker: Marker::None,
        }
    }
    fn cell(&self, index: usize, key: u32) -> String {
        let e = &self.all[self.view[index]];
        match key {
            1 | 2 => self.fmt_meta(e.left, key),
            3 => e.verdict.glyph().to_string(),
            4 | 5 => self.fmt_meta(e.right, key),
            _ => String::new(),
        }
    }
    fn is_selected(&self, index: usize) -> bool {
        self.selected.get(index).copied().unwrap_or(false)
    }
    fn select(&mut self, index: usize, op: SelectOp) -> bool {
        if index >= self.view.len() {
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
        if self.view.is_empty() {
            return false;
        }
        let hi = hi.min(self.view.len() - 1);
        self.set_range(lo.min(hi), hi);
        true
    }
    fn select_all(&mut self) -> bool {
        self.selected.iter_mut().for_each(|s| *s = true);
        !self.view.is_empty()
    }
    fn clear_selection(&mut self) -> bool {
        let had = self.selected.iter().any(|s| *s);
        self.selected.iter_mut().for_each(|s| *s = false);
        had
    }
}

pub(crate) struct CompareWin {
    window: Option<Rc<Window>>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    cursor: (i32, i32),
    shift: bool,
    primary: bool,
    rows: Option<VirtualRows<CmpSource>>,
    pub(crate) left: PathBuf,
    pub(crate) right: PathBuf,
    status: String,
    running: bool,
    row_h: i32,
    filter: Combo,
    only_diff: bool,
    btn_content: Button,
    btn_l2r: Button,
    btn_r2l: Button,
    btn_both: Button,
    btn_close: Button,
    tz: i32,
}

impl CompareWin {
    pub(crate) fn new() -> Self {
        CompareWin {
            window: None,
            surface: None,
            scale: 1.0,
            cursor: (0, 0),
            shift: false,
            primary: false,
            rows: None,
            left: PathBuf::new(),
            right: PathBuf::new(),
            status: String::new(),
            running: false,
            row_h: 0,
            filter: Self::filter_combo(),
            only_diff: true,
            btn_content: Button::new(tr("cmp.byContent")),
            btn_l2r: Button::new(tr("cmp.syncRight")),
            btn_r2l: Button::new(tr("cmp.syncLeft")),
            btn_both: Button::new(tr("cmp.syncBoth")),
            btn_close: Button::new(tr("license.btn.close")),
            tz: 0,
        }
    }

    fn filter_combo() -> Combo {
        Combo::new(
            vec![
                ComboItem::new("diff", tr("cmp.onlyDiff")),
                ComboItem::new("all", tr("cmp.showAll")),
            ],
            0,
        )
    }

    fn buttons(&mut self) -> [&mut Button; 5] {
        [
            &mut self.btn_content,
            &mut self.btn_l2r,
            &mut self.btn_r2l,
            &mut self.btn_both,
            &mut self.btn_close,
        ]
    }

    pub(crate) fn start(&mut self, left: &std::path::Path, right: &std::path::Path, tz: i32) {
        self.left = left.to_path_buf();
        self.right = right.to_path_buf();
        self.tz = tz;
        self.rows = None;
        self.running = true;
        self.status = trf("cmp.scanning", &["0"]);
        for b in [
            &mut self.btn_content,
            &mut self.btn_l2r,
            &mut self.btn_r2l,
            &mut self.btn_both,
        ] {
            b.set_enabled(false);
        }
        self.redraw();
    }

    pub(crate) fn set_progress(&mut self, scanned: u64, hashed: u64) {
        let s = if hashed > 0 {
            trf(
                "cmp.hashing",
                &[&scanned.to_string(), &crate::filelist::format_size(hashed)],
            )
        } else {
            trf("cmp.scanning", &[&scanned.to_string()])
        };
        if s != self.status {
            self.status = s;
            self.redraw();
        }
    }

    pub(crate) fn set_result(&mut self, entries: Vec<CmpEntry>) {
        self.running = false;
        let mut inv = Invalidations::default();
        let src = CmpSource::new(entries, self.only_diff, self.tz);
        let mut rows = VirtualRows::new(src, self.row_h.max(20), 8, 16);
        let cols = [
            ("cmp.col.path", 300),
            ("col.size", 90),
            ("col.modified", 130),
            ("cmp.col.state", 44),
            ("col.size", 90),
            ("col.modified", 130),
        ];
        rows.set_columns(
            cols.iter()
                .enumerate()
                .map(|(i, (k, w))| {
                    let c = Column::new(i as u32, tr(k), *w);
                    if i != 0 {
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
        for b in [
            &mut self.btn_content,
            &mut self.btn_l2r,
            &mut self.btn_r2l,
            &mut self.btn_both,
        ] {
            b.set_enabled(true);
        }
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
        let all = &r.source().all;
        let count = |v: Verdict| all.iter().filter(|e| e.verdict == v).count();
        let differ = all.iter().filter(|e| e.verdict.differs()).count();
        self.status = trf(
            "cmp.summary",
            &[
                &all.len().to_string(),
                &differ.to_string(),
                &count(Verdict::OnlyLeft).to_string(),
                &count(Verdict::OnlyRight).to_string(),
                &(count(Verdict::LeftNewer) + count(Verdict::RightNewer) + count(Verdict::Differ))
                    .to_string(),
            ],
        );
    }

    pub(crate) fn set_filter(&mut self, only_diff: bool) {
        self.only_diff = only_diff;
        self.filter
            .select_value(if only_diff { "diff" } else { "all" });
        if let Some(r) = &mut self.rows {
            r.source_mut().filter(only_diff);
            let mut inv = Invalidations::default();
            r.on_event(
                &InputEvent::Key {
                    key: CtlKey::Home,
                    shift: false,
                    primary: false,
                },
                &mut inv,
            );
            // Home이 첫 행을 고른다 → 필터를 바꾼 직후에는 선택 없음(동기화 버튼 = 전부).
            r.source_mut().clear_selection();
        }
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

    #[cfg(test)]
    pub(crate) fn status(&self) -> &str {
        &self.status
    }

    /// 비교 결과 전체(호스트가 계획을 세울 때).
    pub(crate) fn entries(&self) -> Option<Vec<CmpEntry>> {
        self.rows.as_ref().map(|r| r.source().all.clone())
    }

    /// 동기화 대상 상대 경로(선택 행 · 없으면 빈 = 전부).
    pub(crate) fn sync_targets(&self) -> Vec<String> {
        self.rows
            .as_ref()
            .map_or_else(Vec::new, |r| r.source().selected_rels())
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
        let (lw, lh) = over.map_or((1000.0, 620.0), |(_, _, w, h)| {
            (
                (w as f32 * 0.8).clamp(640.0, 1500.0),
                (h as f32 * 0.7).clamp(360.0, 1000.0),
            )
        });
        let mut attrs = Window::default_attributes()
            .with_title(format!("Nexa Dir — {}", tr("cmp.title")))
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
        for b in self.buttons() {
            b.clear_transient();
        }
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
        self.btn_content.set_label(tr("cmp.byContent"));
        self.btn_l2r.set_label(tr("cmp.syncRight"));
        self.btn_r2l.set_label(tr("cmp.syncLeft"));
        self.btn_both.set_label(tr("cmp.syncBoth"));
        self.btn_close.set_label(tr("license.btn.close"));
        self.filter = Self::filter_combo();
        self.filter
            .select_value(if self.only_diff { "diff" } else { "all" });
        self.refresh_summary();
        if let Some(w) = &self.window {
            w.set_title(&format!("Nexa Dir — {}", tr("cmp.title")));
        }
        self.redraw();
    }

    pub(crate) fn tick(&mut self, now_ms: u64) -> bool {
        if self.window.is_none() {
            return false;
        }
        let mut any = false;
        for b in self.buttons() {
            any |= b.tick(now_ms);
        }
        any
    }

    pub(crate) fn animating(&self) -> bool {
        self.window.is_some()
            && [
                &self.btn_content,
                &self.btn_l2r,
                &self.btn_r2l,
                &self.btn_both,
                &self.btn_close,
            ]
            .iter()
            .any(|b| b.is_animating())
    }

    fn head_h(&self) -> i32 {
        self.row_h.max(16) + (8.0 * self.scale).round() as i32
    }

    fn bottom_h(&self) -> i32 {
        ((BTN_H + PAD * 2.0) * self.scale).round() as i32 + self.row_h.max(16)
    }

    fn layout(&mut self) {
        let Some(win) = &self.window else { return };
        let size = win.inner_size();
        let (w, h) = (size.width as i32, size.height as i32);
        let top = self.head_h();
        let gh = (h - top - self.bottom_h()).max(10);
        if let Some(r) = &mut self.rows {
            let mut inv = Invalidations::default();
            r.set_bounds(Rect::new(0, top, w, gh), &mut inv);
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

    pub(crate) fn handle(&mut self, ev: &WindowEvent) -> CmpAction {
        let mut inv = Invalidations::default();
        match ev {
            WindowEvent::CloseRequested => return CmpAction::Close,
            WindowEvent::RedrawRequested => return CmpAction::Paint,
            WindowEvent::Resized(_) => {
                self.layout();
                self.redraw();
                return CmpAction::None;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.layout();
                self.redraw();
                return CmpAction::None;
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.primary = if cfg!(target_os = "macos") {
                    m.state().super_key()
                } else {
                    m.state().control_key()
                };
                return CmpAction::None;
            }
            WindowEvent::Focused(false) => {
                for b in self.buttons() {
                    b.clear_transient();
                }
                self.redraw();
                return CmpAction::None;
            }
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                if matches!(kev.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    if self.filter.is_open() {
                        self.filter.close(&mut inv);
                        self.redraw();
                        return CmpAction::None;
                    }
                    return CmpAction::Close;
                }
                if self.primary && crate::input::shortcut_letter(kev) == Some('a') {
                    if let Some(r) = &mut self.rows {
                        r.on_event(&InputEvent::SelectAll, &mut inv);
                    }
                    self.redraw();
                    return CmpAction::None;
                }
            }
            _ => {}
        }
        if let WindowEvent::CursorMoved { position, .. } = ev {
            self.cursor = (position.x as i32, position.y as i32);
            let (x, y) = self.cursor;
            let mv = InputEvent::MouseMove { x, y };
            for b in self.buttons() {
                b.on_event(&mv, &mut inv);
            }
            self.filter.on_event(&mv, &mut inv);
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
                if self.filter.is_open() || self.filter.bounds().contains(p) || up {
                    self.filter.on_event(&e, &mut inv);
                    let want = self.filter.selected_value() == "diff";
                    if want != self.only_diff {
                        self.set_filter(want);
                    }
                    if self.filter.is_open() {
                        self.redraw();
                        return CmpAction::None;
                    }
                }
                for b in self.buttons() {
                    if up || b.bounds().contains(p) {
                        b.on_event(&e, &mut inv);
                    }
                }
                if self.btn_close.take_clicked() {
                    return CmpAction::Close;
                }
                if self.btn_content.take_clicked() {
                    return CmpAction::Rescan(true);
                }
                let targets = self.sync_targets();
                if self.btn_l2r.take_clicked() {
                    return CmpAction::Sync(Direction::LeftToRight, targets);
                }
                if self.btn_r2l.take_clicked() {
                    return CmpAction::Sync(Direction::RightToLeft, targets);
                }
                if self.btn_both.take_clicked() {
                    return CmpAction::Sync(Direction::Both, targets);
                }
            }
        }
        let Some(ie) = self.to_input(ev) else {
            self.redraw();
            return CmpAction::None;
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
        self.redraw();
        CmpAction::None
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
            let clip = Rect::new(0, 0, wi, hi);
            // 머리: 왼쪽 · 오른쪽 폴더.
            let head = format!(
                "{}  ⟷  {}",
                crate::filelist::display_path(&self.left),
                crate::filelist::display_path(&self.right)
            );
            let head = nexa_ctl::draw::ellipsize_middle(&mut dc, &head, wi - pad * 2);
            dc.text(pad, (4.0 * s).round() as i32, clip, &head, th.text);
            let top = self.head_h();
            let bottom_h = self.bottom_h();
            let gh = (hi - top - bottom_h).max(10);
            let inv = &mut Invalidations::default();
            if let Some(r) = &mut self.rows {
                if r.bounds().h != gh || r.bounds().w != wi || r.bounds().y != top {
                    r.set_bounds(Rect::new(0, top, wi, gh), inv);
                }
                r.paint(&mut dc, th);
            }
            dc.select_font(FontSlot::Base, false);
            let sy = top + gh + (4.0 * s).round() as i32;
            let status = nexa_ctl::draw::ellipsize_middle(&mut dc, &self.status, wi - pad * 2);
            dc.text(pad, sy, clip, &status, th.text_dim);
            // 바닥: 필터 콤보 · [내용 비교] · [→] [←] [⇄] · [닫기].
            let btn_h = (BTN_H * s).round() as i32;
            let by = hi - pad - btn_h;
            self.filter.set_scale(s);
            self.filter
                .set_bounds(Rect::new(pad, by, (170.0 * s).round() as i32, btn_h), inv);
            let mut x = pad + (170.0 * s).round() as i32 + pad;
            self.btn_content.set_scale(s);
            let cw = dc.text_width(self.btn_content.label()) + (24.0 * s).round() as i32;
            self.btn_content
                .set_bounds(Rect::new(x, by, cw, btn_h), inv);
            self.btn_content.paint(&mut dc, th);
            x += cw + pad;
            for b in [&mut self.btn_l2r, &mut self.btn_r2l, &mut self.btn_both] {
                b.set_scale(s);
                let bw = dc.text_width(b.label()) + (24.0 * s).round() as i32;
                b.set_bounds(Rect::new(x, by, bw, btn_h), inv);
                b.paint(&mut dc, th);
                x += bw + (6.0 * s).round() as i32;
            }
            self.btn_close.set_scale(s);
            let bw = dc.text_width(self.btn_close.label()) + (28.0 * s).round() as i32;
            self.btn_close
                .set_bounds(Rect::new(wi - pad - bw, by, bw, btn_h), inv);
            self.btn_close.paint(&mut dc, th);
            self.filter.paint(&mut dc, th);
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

    fn e(rel: &str, v: Verdict) -> CmpEntry {
        let m = Meta {
            is_dir: false,
            size: 3,
            mtime: 1000,
        };
        CmpEntry {
            rel: rel.into(),
            left: (v != Verdict::OnlyRight).then_some(m),
            right: (v != Verdict::OnlyLeft).then_some(m),
            verdict: v,
        }
    }

    /// 창 없이: 결과 → 다른 것만 보임 · 전부 보기 · 요약 · 선택 행 = 동기화 대상 · 기호.
    #[test]
    fn filter_summary_and_targets() {
        en();
        let mut w = CompareWin::new();
        w.start(
            std::path::Path::new("D:/L"),
            std::path::Path::new("D:/R"),
            0,
        );
        assert!(w.is_running());
        w.set_result(vec![
            e("a", Verdict::Same),
            e("b", Verdict::OnlyLeft),
            e("c", Verdict::RightNewer),
            e("d", Verdict::OnlyRight),
        ]);
        assert!(!w.is_running());
        assert_eq!(w.rows_len(), 3, "다른 것만");
        assert!(w.status().contains("3"), "{}", w.status());
        w.set_filter(false);
        assert_eq!(w.rows_len(), 4);
        assert!(w.sync_targets().is_empty(), "선택 없음 = 전부");
        w.rows
            .as_mut()
            .unwrap()
            .source_mut()
            .select(1, SelectOp::Single);
        assert_eq!(w.sync_targets(), vec!["b".to_string()]);
        assert_eq!(Verdict::OnlyLeft.glyph(), "▷");
        w.close();
        assert_eq!(w.rows_len(), 0);
    }
}
