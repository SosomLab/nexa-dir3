//! 압축 미리보기 **그리드 창**(T-62 C · dir2 `archivewnd.rs` 이식 · docs/port/20 PLUG-070~077).
//!
//! 하단 도크가 요약 텍스트를 보여 주는 것과 같은 자료([`ArchiveDoc`])를 **파일 목록과 같은 규약의 그리드**
//! (nexa-grid `VirtualRows` — 헤더 리사이즈·정렬·선택·오버레이 스크롤바)로 크게 보여 준다. 컬럼 8(PLUG-072) · 지브라 ·
//! 초기 정렬 = 경로순 · 폴더 우선 없음(PLUG-074) · 상태 줄(PLUG-075 · 실패 상태는 사유 그대로) ·
//! Esc = 닫기 · Ctrl+C = 선택 행(없으면 전체) TSV 복사(PLUG-076) · Ctrl+A · 그 외 = 그리드 규약(PLUG-077).
//! 암호 흐름(PLUG-070)은 호스트(`app/previewcmd.rs`)가 대화상자로 돈다 — 이 창은 읽어 둔 목록만 받는다.

use crate::preview::archive::{self as arc, ArchiveDoc, ArchiveStatus};
use ndir_i18n::{tr, trf};
use ndir_vfs::archive::ArchiveEntry;
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::Rect;
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::{InputEvent, Invalidations, Key as CtlKey, Widget};
use nexa_gfx::{Font, Surface};
use nexa_grid::{Column, Marker, RowItem, RowSource, SelectOp, VirtualRows};
use std::rc::Rc;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// 창이 호스트에 요청하는 것.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArcAction {
    None,
    Paint,
    /// TSV 텍스트를 클립보드로.
    Copy(String),
}

const PAD: f32 = 10.0;

/// 컬럼(제목 키, 기본 폭) — 순서 = [`row_cells`]의 셀 순서 = 컬럼 key(PLUG-072).
pub(crate) const COLS: [(&str, i32); 8] = [
    ("archive.col.name", 240),
    ("archive.col.path", 200),
    ("archive.col.size", 90),
    ("archive.col.packed", 90),
    ("archive.col.ratio", 60),
    ("archive.col.method", 90),
    ("archive.col.modified", 130),
    ("archive.col.flags", 90),
];

/// 항목 1개 → 그리드 셀들(컬럼 순서 = [`COLS`] · PLUG-073).
pub(crate) fn row_cells(e: &ArchiveEntry, tz: i32) -> Vec<String> {
    let mut flags: Vec<String> = Vec::new();
    if e.is_dir {
        flags.push(tr("archive.flag.dir"));
    }
    if e.encrypted {
        flags.push(tr("archive.flag.locked"));
    }
    if e.suspicious {
        flags.push(tr("archive.flag.unsafe"));
    }
    let size_of = |dir: bool, v: Option<u64>| match (dir, v) {
        (true, _) => String::new(),
        (_, Some(s)) => crate::filelist::format_size(s),
        _ => "-".into(),
    };
    vec![
        e.name().to_string(),
        e.parent().to_string(),
        size_of(e.is_dir, e.size),
        size_of(e.is_dir, e.packed),
        e.ratio().map(|r| format!("{r}%")).unwrap_or_default(),
        e.method.clone(),
        e.modified
            .map(|t| arc::fmt_entry_time(t, e.time_is_local, tz))
            .unwrap_or_default(),
        flags.join(" · "),
    ]
}

/// 정렬(PLUG-074) — 크기·압축 크기·압축률·시각 = 수치 · 이름·경로·방식 = 문자열 · 표시 = 튜플 · 동률/빈 사양 = 경로순.
/// 폴더 우선은 두지 않는다(압축 목록은 "무엇이 들어 있나"가 관심사).
pub(crate) fn sort_entries(entries: &mut [ArchiveEntry], spec: &[(u32, bool)]) {
    if spec.is_empty() {
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        return;
    }
    entries.sort_by(|a, b| {
        for &(col, desc) in spec {
            let ord = match col {
                0 => a.name().cmp(b.name()),
                1 => a.parent().cmp(b.parent()),
                2 => a.size.unwrap_or(0).cmp(&b.size.unwrap_or(0)),
                3 => a.packed.unwrap_or(0).cmp(&b.packed.unwrap_or(0)),
                4 => a.ratio().unwrap_or(0).cmp(&b.ratio().unwrap_or(0)),
                5 => a.method.cmp(&b.method),
                6 => a.modified.unwrap_or(0).cmp(&b.modified.unwrap_or(0)),
                7 => (a.is_dir, a.encrypted, a.suspicious).cmp(&(
                    b.is_dir,
                    b.encrypted,
                    b.suspicious,
                )),
                _ => std::cmp::Ordering::Equal,
            };
            let ord = if desc { ord.reverse() } else { ord };
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        a.path.cmp(&b.path)
    });
}

/// 상태 줄 문구(PLUG-075) — 포맷·항목 수·총 크기 + 암호/솔리드/분할/절단 · 실패 상태면 사유 그대로. 주석은 창에 표시 안 함.
pub(crate) fn status_text(doc: &ArchiveDoc) -> String {
    match &doc.status {
        ArchiveStatus::Ok => {
            let l = &doc.listing;
            let (size, packed) = l.totals();
            let mut s = trf(
                "archive.status",
                &[
                    &l.label,
                    &l.entries.len().to_string(),
                    &crate::filelist::format_size(size),
                    &crate::filelist::format_size(packed),
                ],
            );
            for (flag, key) in [
                (l.has_encrypted, "archive.encrypted"),
                (l.solid, "archive.solid"),
                (l.multivolume, "archive.multivolume"),
                (l.truncated, "archive.truncated"),
            ] {
                if flag {
                    s.push_str(" · ");
                    s.push_str(&tr(key));
                }
            }
            s
        }
        ArchiveStatus::NeedPassword => tr("archive.needPassword"),
        ArchiveStatus::NeedPlugin(fmt, codec) => trf("archive.needPlugin", &[fmt, codec]),
        ArchiveStatus::Failed(why) => trf("archive.failed", &[why]),
    }
}

/// 그리드 행 공급자 — 항목 벡터 + 선택 집합(위젯은 가시 행만 묻는다 · 정렬은 `set_sort`로 통지).
#[derive(Debug)]
pub(crate) struct ArchiveSource {
    entries: Vec<ArchiveEntry>,
    selected: Vec<bool>,
    anchor: Option<usize>,
    tz: i32,
}

impl ArchiveSource {
    pub(crate) fn new(mut entries: Vec<ArchiveEntry>, tz: i32) -> Self {
        sort_entries(&mut entries, &[]);
        let n = entries.len();
        ArchiveSource {
            entries,
            selected: vec![false; n],
            anchor: None,
            tz,
        }
    }

    pub(crate) fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }

    /// 선택 행 인덱스(오름차순).
    pub(crate) fn selected_rows(&self) -> Vec<usize> {
        (0..self.entries.len())
            .filter(|&i| self.selected[i])
            .collect()
    }

    /// 선택 행(없으면 전체)을 탭 구분 + `\r\n` 줄로(PLUG-076).
    pub(crate) fn tsv(&self) -> String {
        let sel = self.selected_rows();
        let idx: Vec<usize> = if sel.is_empty() {
            (0..self.entries.len()).collect()
        } else {
            sel
        };
        let mut text = String::new();
        for i in idx {
            if let Some(e) = self.entries.get(i) {
                text.push_str(&row_cells(e, self.tz).join("\t"));
                text.push_str("\r\n");
            }
        }
        text
    }

    fn set_range(&mut self, lo: usize, hi: usize) {
        for (i, s) in self.selected.iter_mut().enumerate() {
            *s = (lo..=hi).contains(&i);
        }
    }
}

impl RowSource for ArchiveSource {
    fn len(&self) -> usize {
        self.entries.len()
    }

    fn row(&self, index: usize) -> RowItem {
        let e = &self.entries[index];
        RowItem {
            text: e.name().to_string(),
            is_dir: e.is_dir,
            depth: 0,
            marker: Marker::None,
        }
    }

    fn cell(&self, index: usize, key: u32) -> String {
        let e = &self.entries[index];
        row_cells(e, self.tz)
            .into_iter()
            .nth(key as usize)
            .unwrap_or_default()
    }

    fn set_sort(&mut self, keys: &[(u32, bool)]) -> bool {
        sort_entries(&mut self.entries, keys);
        self.selected.iter_mut().for_each(|s| *s = false);
        self.anchor = None;
        true
    }

    fn is_selected(&self, index: usize) -> bool {
        self.selected.get(index).copied().unwrap_or(false)
    }

    fn select(&mut self, index: usize, op: SelectOp) -> bool {
        if index >= self.entries.len() {
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
        if self.entries.is_empty() {
            return false;
        }
        let hi = hi.min(self.entries.len() - 1);
        self.set_range(lo.min(hi), hi);
        true
    }

    fn select_all(&mut self) -> bool {
        self.selected.iter_mut().for_each(|s| *s = true);
        !self.entries.is_empty()
    }

    fn clear_selection(&mut self) -> bool {
        let had = self.selected.iter().any(|s| *s);
        self.selected.iter_mut().for_each(|s| *s = false);
        had
    }
}

pub(crate) struct ArchiveWin {
    window: Option<Rc<Window>>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    cursor: (i32, i32),
    shift: bool,
    primary: bool,
    rows: Option<VirtualRows<ArchiveSource>>,
    title: String,
    status: String,
    /// 마지막 페인트의 행 높이(글꼴·배율이 바뀌면 `set_metrics`).
    row_h: i32,
}

impl ArchiveWin {
    pub(crate) fn new() -> Self {
        ArchiveWin {
            window: None,
            surface: None,
            scale: 1.0,
            cursor: (0, 0),
            shift: false,
            primary: false,
            rows: None,
            title: String::new(),
            status: String::new(),
            row_h: 0,
        }
    }

    /// 읽어 둔 목록을 창 자료로(열기 전/열린 뒤 모두 — 열린 창은 다시 그린다).
    pub(crate) fn set_doc(&mut self, name: &str, doc: ArchiveDoc, tz: i32) {
        self.title = trf("archive.window.title", &[name]);
        self.status = status_text(&doc);
        let mut inv = Invalidations::default();
        let src = ArchiveSource::new(doc.listing.entries, tz);
        let mut rows = VirtualRows::new(src, self.row_h.max(20), 8, 16);
        rows.set_columns(
            COLS.iter()
                .enumerate()
                .map(|(i, (k, w))| {
                    let c = Column::new(i as u32, tr(k), *w);
                    if (2..=4).contains(&i) {
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
        if let Some(w) = &self.window {
            w.set_title(&self.title);
        }
        self.layout();
        self.redraw();
    }

    pub(crate) fn rows_len(&self) -> usize {
        self.rows.as_ref().map_or(0, |r| r.source().len())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn source_mut(&mut self) -> Option<&mut ArchiveSource> {
        self.rows.as_mut().map(VirtualRows::source_mut)
    }

    /// 선택 행(없으면 전체) TSV — Ctrl+C · 기동 명령.
    pub(crate) fn tsv(&self) -> String {
        self.rows
            .as_ref()
            .map_or_else(String::new, |r| r.source().tsv())
    }

    /// 자체 시험 덤프(`archive.dump`): 제목 · 열림 · 행 수 · 선택 수 · 정렬 · 상태 · 앞 20행 셀.
    pub(crate) fn dump(&self) -> String {
        let mut s = format!(
            "title={}\nopen={}\nrows={}\nselected={}\nsort={:?}\nstatus={}\n",
            self.title,
            self.window.is_some(),
            self.rows_len(),
            self.rows
                .as_ref()
                .map_or(0, |r| r.source().selected_rows().len()),
            self.rows.as_ref().map_or(&[][..], |r| r.sort()),
            self.status
        );
        if let Some(r) = &self.rows {
            for e in r.source().entries().iter().take(20) {
                s.push_str(&row_cells(e, 0).join(" | "));
                s.push('\n');
            }
        }
        s
    }

    pub(crate) fn is(&self, id: WindowId) -> bool {
        self.window.as_ref().is_some_and(|w| w.id() == id)
    }

    pub(crate) fn is_open(&self) -> bool {
        self.window.is_some()
    }

    pub(crate) fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    /// 틱(스크롤바 페이드 · 타입어헤드) — 다시 그려야 하면 true.
    pub(crate) fn tick(&mut self, now_ms: u64) -> bool {
        if self.window.is_none() {
            return false;
        }
        let mut inv = Invalidations::default();
        if let Some(r) = &mut self.rows {
            r.tick(now_ms, &mut inv);
        }
        !inv.is_empty() || inv.tick_requested()
    }

    pub(crate) fn animating(&self) -> bool {
        false
    }

    pub(crate) fn open(
        &mut self,
        el: &ActiveEventLoop,
        theme: Option<winit::window::Theme>,
        over: Option<(i32, i32, u32, u32)>,
        owner: Option<&Window>,
    ) {
        if let Some(w) = &self.window {
            w.set_title(&self.title);
            w.focus_window();
            self.redraw();
            return;
        }
        // dir2 PLUG-071: 소유자의 3/4(폭 560~1500 · 높이 360~1000) · 중앙 · 없으면 960×640.
        let (lw, lh) = over.map_or((960.0, 640.0), |(_, _, w, h)| {
            (
                (w as f32 * 0.75).clamp(560.0, 1500.0),
                (h as f32 * 0.75).clamp(360.0, 1000.0),
            )
        });
        let mut attrs = Window::default_attributes()
            .with_title(self.title.clone())
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
    }

    fn status_h(&self) -> i32 {
        ((self.row_h.max(16)) as f32 + 8.0 * self.scale).round() as i32
    }

    /// 그리드 = 상단 전체 · 상태 줄 = 하단 1행(dir2 §2.2).
    fn layout(&mut self) {
        let Some(win) = &self.window else { return };
        let size = win.inner_size();
        let (w, h) = (size.width as i32, size.height as i32);
        let pad = (PAD * self.scale).round() as i32;
        let gh = (h - self.status_h() - pad).max(10);
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
                (ElementState::Pressed, MouseButton::Right) => InputEvent::RightDown { x, y },
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
                    Key::Named(NamedKey::Space) => key(CtlKey::Space),
                    Key::Character(t) => {
                        let c = t.chars().next()?;
                        if c.is_control() || self.primary {
                            return None;
                        }
                        InputEvent::Char { c, now_ms: 0 }
                    }
                    _ => return None,
                }
            }
            _ => return None,
        })
    }

    pub(crate) fn handle(&mut self, ev: &WindowEvent) -> ArcAction {
        match ev {
            WindowEvent::CloseRequested => {
                self.close();
                return ArcAction::None;
            }
            WindowEvent::RedrawRequested => return ArcAction::Paint,
            WindowEvent::Resized(_) => {
                self.layout();
                self.redraw();
                return ArcAction::None;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.layout();
                self.redraw();
                return ArcAction::None;
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.primary = if cfg!(target_os = "macos") {
                    m.state().super_key()
                } else {
                    m.state().control_key()
                };
                return ArcAction::None;
            }
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                if matches!(kev.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    self.close();
                    return ArcAction::None;
                }
                if self.primary {
                    match crate::input::shortcut_letter(kev) {
                        Some('c') => return ArcAction::Copy(self.tsv()),
                        Some('a') => {
                            let mut inv = Invalidations::default();
                            if let Some(r) = &mut self.rows {
                                r.on_event(&InputEvent::SelectAll, &mut inv);
                            }
                            self.redraw();
                            return ArcAction::None;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        if let WindowEvent::CursorMoved { position, .. } = ev {
            self.cursor = (position.x as i32, position.y as i32);
            let over = self
                .rows
                .as_ref()
                .is_some_and(|r| r.resize_hot(self.cursor.0, self.cursor.1));
            if let Some(w) = &self.window {
                w.set_cursor(if over {
                    winit::window::CursorIcon::ColResize
                } else {
                    winit::window::CursorIcon::Default
                });
            }
        }
        let Some(ie) = self.to_input(ev) else {
            return ArcAction::None;
        };
        let mut inv = Invalidations::default();
        if let Some(r) = &mut self.rows {
            r.on_event(&ie, &mut inv);
        }
        self.redraw();
        ArcAction::None
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
            // 목록 슬롯(nexa-grid `List` = nexa-ctl `PeerList`)도 같은 크기로.
            let mut prefs = FontPrefs::with_base(font_px);
            prefs.peerlist = prefs.base;
            let mut dc = RasterCtx::new(&mut gfx, ui, s).with_fonts(prefs);
            dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
            dc.select_font(FontSlot::Base, false);
            let th_txt = dc.text_height();
            // 행 높이 = 글꼴 + 상하 4px(dir2 `ctl/grid.rs`) — 바뀌면 지표 갱신 + 재배치.
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
            let status_h = self.status_h();
            let gh = (hi - status_h - pad).max(10);
            if let Some(r) = &mut self.rows {
                if r.bounds().h != gh || r.bounds().w != wi {
                    let mut inv = Invalidations::default();
                    r.set_bounds(Rect::new(0, 0, wi, gh), &mut inv);
                }
                r.paint(&mut dc, th);
            }
            dc.select_font(FontSlot::Base, false);
            let sr = Rect::new(
                pad,
                gh + (4.0 * s).round() as i32,
                (wi - pad * 2).max(10),
                status_h,
            );
            let status = nexa_ctl::draw::ellipsize_middle(&mut dc, &self.status, sr.w);
            dc.text(
                sr.x,
                sr.y + (status_h - th_txt) / 2,
                sr,
                &status,
                th.text_dim,
            );
        }
        let _ = buf.present();
        self.surface = Some(surface);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, size: u64, packed: u64, dir: bool) -> ArchiveEntry {
        ArchiveEntry {
            path: path.into(),
            is_dir: dir,
            size: (!dir).then_some(size),
            packed: (!dir).then_some(packed),
            modified: Some(1_700_000_000),
            method: "Deflate".into(),
            ..Default::default()
        }
    }

    fn en() {
        ndir_i18n::activate(ndir_i18n::load("en", std::path::Path::new("nowhere")));
    }

    /// dir2 `cells_follow_column_order_and_blank_dirs`.
    #[test]
    fn cells_follow_column_order_and_blank_dirs() {
        en();
        let cells = row_cells(&entry("docs/readme.md", 1000, 250, false), 0);
        assert_eq!(cells.len(), COLS.len());
        assert_eq!(cells[0], "readme.md");
        assert_eq!(cells[1], "docs");
        assert_eq!(cells[4], "75%");
        assert_eq!(cells[5], "Deflate");
        let dir = row_cells(&entry("docs", 0, 0, true), 0);
        assert!(
            dir[2].is_empty() && dir[3].is_empty(),
            "폴더 행은 크기 비움"
        );
        assert_eq!(dir[7], "folder");
    }

    /// dir2 `sort_uses_numeric_order_for_size_and_reverses` + `empty_sort_spec_falls_back_to_path_order`.
    #[test]
    fn sort_numeric_and_path_fallback() {
        let mut v = vec![
            entry("a.bin", 100, 50, false),
            entry("b.bin", 3000, 100, false),
            entry("c.bin", 20, 10, false),
        ];
        sort_entries(&mut v, &[(2, false)]);
        assert_eq!(
            v.iter().map(|e| e.path.as_str()).collect::<Vec<_>>(),
            ["c.bin", "a.bin", "b.bin"],
            "크기는 문자열이 아니라 수치 순"
        );
        sort_entries(&mut v, &[(2, true)]);
        assert_eq!(v[0].path, "b.bin");
        let mut v = vec![entry("z.txt", 1, 1, false), entry("a/b.txt", 1, 1, false)];
        sort_entries(&mut v, &[]);
        assert_eq!(v[0].path, "a/b.txt");
    }

    /// 행 공급자: 선택 규약(단일/토글/범위/전체/해제) · 정렬 통지 = 재정렬 + 선택 해제 · TSV = 선택(없으면 전체).
    #[test]
    fn source_selection_sort_and_tsv() {
        en();
        let mut s = ArchiveSource::new(
            vec![
                entry("b.bin", 3000, 100, false),
                entry("a.bin", 100, 50, false),
                entry("c.bin", 20, 10, false),
            ],
            0,
        );
        assert_eq!(s.row(0).text, "a.bin", "초기 = 경로순");
        assert_eq!(s.cell(0, 4), "50%");
        assert!(s.select(0, SelectOp::Single) && s.select(2, SelectOp::RangeTo));
        assert_eq!(s.selected_rows(), vec![0, 1, 2]);
        assert!(s.select(1, SelectOp::Toggle));
        assert_eq!(s.selected_rows(), vec![0, 2]);
        assert_eq!(s.tsv().lines().count(), 2);
        assert!(s.clear_selection() && !s.clear_selection());
        assert_eq!(s.tsv().lines().count(), 3, "선택 없음 = 전체");
        assert!(s.select_all() && s.selected_rows().len() == 3);
        assert!(s.set_sort(&[(2, true)]));
        assert_eq!(s.row(0).text, "b.bin");
        assert!(s.selected_rows().is_empty(), "정렬 뒤 선택 해제");
        let mut w = ArchiveWin::new();
        assert_eq!(w.rows_len(), 0);
        w.set_doc(
            "x.zip",
            ArchiveDoc {
                path: "x.zip".into(),
                listing: ndir_vfs::archive::Listing {
                    label: "ZIP".into(),
                    entries: vec![entry("q/w.txt", 10, 7, false)],
                    ..Default::default()
                },
                status: ArchiveStatus::Ok,
                provider: "builtin.archive".into(),
            },
            0,
        );
        let d = w.dump();
        assert!(
            d.contains("title=x.zip — Archive")
                && d.contains("rows=1")
                && d.contains("status=ZIP · 1 items")
                && d.contains("w.txt | q | "),
            "{d}"
        );
        assert!(!w.tick(5) && !w.animating());
    }
}
