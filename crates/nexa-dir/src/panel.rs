//! 패널 — 탭 바 + 네비 버튼([홈][←][→][↑]) + 경로 바 + 파일 목록 묶음(dir2 `panel.rs` 이식 · docs/port/13 PANEL-001·002 · 012~039).
//! 탭 = 독립 뷰 상태 + **탭별 독립 히스토리**. 플랫폼 중립 — 창·OS API를 모른다(전 플랫폼 테스트).
//! 창(App)은 패널 2개를 스플리터로 배치하고 **활성 패널**에 키보드를 라우팅한다.
//!
//! dir2 대비 이번 슬라이스에 없는 것(뒤 과제): 도크(M5) · 펼침 집합 영속·세션(T-45) · 탭 잠금/고정/복제/패널 간 이동(T-43 잔여) ·
//! 클라우드 경로(M6) · 무간섭 재열람의 선택 복원(지금은 캐럿·스크롤만).

use crate::filelist::{display_path, title_of, ListOpts, TreeSource};
use crate::nav::History;
use crate::session::PanelSession;
use nexa_ctl::controls::{Control, TabAction, TabBar, ToolIcon, ToolItem, Toolbar};
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::theme::Theme;
use nexa_ctl::{DrawCtx, InputEvent, Invalidations, Key as CtlKey, Widget};
use nexa_explorer::pathbar::PathBar;
use nexa_grid::{Column, EditKey, RowSource, ScrollAlign, SelectOp, ViewMode, VirtualRows};
use std::path::{Path, PathBuf};

/// 네비 버튼 id(= 명령 id · 호스트 `command`와 같은 어휘).
pub(crate) const BTN_HOME: &str = "nav.home";
pub(crate) const BTN_BACK: &str = "nav.back";
pub(crate) const BTN_FORWARD: &str = "nav.forward";
pub(crate) const BTN_UP: &str = "nav.up";

/// 패널 지표(배율 반영 · 호스트가 계산 — dir2 `panel_metrics`: row 20 · pad 6 · indent 16 · tab 22 · bar 24 @96dpi).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PanelMetrics {
    pub row_h: i32,
    pub pad_x: i32,
    pub indent_w: i32,
    pub tab_h: i32,
    pub bar_h: i32,
    pub scale: f32,
}

/// 탭 = 목록 뷰 상태 + 독립 back/forward 히스토리.
pub(crate) struct Tab {
    pub rows: VirtualRows<TreeSource>,
    pub nav: History,
    /// 탭 잠금(닫기 제외 · dir2 TAB-MENU) · 고정(📌 핀 그룹 앞 정렬) — 세션 영속.
    pub locked: bool,
    pub pinned: bool,
}

/// 포인터 캡처 대상(눌린 곳이 뗄 때까지 받는다).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Part {
    Tabs,
    Nav,
    Path,
    List,
}

pub(crate) struct Panel {
    pub tabbar: TabBar,
    navbtns: Toolbar,
    pub pathbar: PathBar,
    tabs: Vec<Tab>,
    active: usize,
    bounds: Rect,
    m: PanelMetrics,
    focused: bool,
    opts: ListOpts,
    nav_up_align: ScrollAlign,
    pressed: Option<Part>,
    /// 사용자가 열 폭을 바꿨다 — 배치가 기본 열을 다시 넣지 않는다(dir2 "사용자 폭 리셋 방지").
    user_cols: bool,
    /// 호스트가 준 기본 열(내 PC 드라이브 열 ↔ 일반 열 전환의 복귀 원본 · dir2 X-17).
    base_columns: Vec<Column>,
    /// 파일 활성화(실행은 호스트 몫 · 1회성 수거).
    pending_open: Option<PathBuf>,
    /// 탭 우클릭 메뉴 요청(표시는 호스트 · 1회성).
    pending_tab_menu: Option<usize>,
    /// 인라인 이름 바꾸기 확정(행, 새 이름) — 실행(fs)은 호스트(`App::apply_rename`).
    pending_rename: Option<(usize, String)>,
    /// 목록 우클릭(행 위 = true · 빈 영역 = false) — 호스트가 컨텍스트 메뉴를 연다.
    pending_ctx: Option<bool>,
    session_dirty: bool,
    /// 사용자가 열 폭을 바꿨다(호스트가 수거해 반대 패널에 동기 · `list.col_width_sync`).
    col_changed: bool,
}

/// 네비 버튼 1개 폭(고정 — 배치가 측정 없이 계산 · dir2 `nav_btn_w`).
fn nav_btn_w(m: &PanelMetrics) -> i32 {
    m.row_h + m.pad_x
}

/// [홈][←][→][↑] — dir2는 Segoe MDL2 PUA 글리프 · 3-OS는 유니코드 글리프(SVG 아이콘은 T-30).
fn nav_buttons() -> Toolbar {
    let items = [
        (BTN_HOME, "\u{2302}", "nav.mypc"),
        (BTN_BACK, "\u{2190}", "cmd.navBack"),
        (BTN_FORWARD, "\u{2192}", "cmd.navForward"),
        (BTN_UP, "\u{2191}", "cmd.navUp"),
    ]
    .into_iter()
    .map(|(id, g, tip)| ToolItem::new(id, ToolIcon::Glyph(g.to_string())).tip(ndir_i18n::tr(tip)))
    .collect();
    let mut t = Toolbar::new(items);
    t.set_icon_size(14);
    t.set_padding(4, 0);
    t
}

impl Panel {
    /// 첫 탭을 `path`로 시작(열기 실패여도 패널은 만들어진다 — 빈 목록 + 오류 문구).
    pub(crate) fn new(path: &Path, opts: ListOpts, m: PanelMetrics, columns: Vec<Column>) -> Panel {
        let mut inv = Invalidations::default();
        let mut rows = VirtualRows::new(TreeSource::open(path, opts), m.row_h, m.pad_x, m.indent_w);
        rows.set_columns(columns, &mut inv);
        let mut tabbar = TabBar::new();
        tabbar.set_show_new(true);
        let mut p = Panel {
            tabbar,
            navbtns: nav_buttons(),
            pathbar: PathBar::new(display_path(path), m.bar_h, m.pad_x),
            tabs: vec![Tab {
                rows,
                nav: History::new(path.to_path_buf()),
                locked: false,
                pinned: false,
            }],
            active: 0,
            bounds: Rect::default(),
            m,
            focused: false,
            opts,
            nav_up_align: ScrollAlign::Center,
            pressed: None,
            user_cols: false,
            base_columns: Vec::new(),
            pending_open: None,
            pending_tab_menu: None,
            pending_rename: None,
            pending_ctx: None,
            session_dirty: false,
            col_changed: false,
        };
        p.set_metrics(m, &mut inv);
        p.sync_chrome(&mut inv);
        p
    }

    /// 세션 복원(dir2 `Panel::restore` · PANEL-023) — 경로 목록으로 탭을 연다(열기 실패 탭은 건너뜀 · 전부 실패면 `fallback`) ·
    /// 탭별 보기 모드 · 활성 인덱스 클램프 · 열 폭(`colw`)은 기본 열 위에 덮는다.
    pub(crate) fn restore(
        ps: &PanelSession,
        fallback: &Path,
        opts: ListOpts,
        m: PanelMetrics,
        columns: Vec<Column>,
    ) -> Panel {
        let mut valid: Vec<(usize, PathBuf)> = ps
            .tabs
            .iter()
            .enumerate()
            .filter(|(_, p)| ndir_vfs::is_virtual_root(p) || p.is_dir())
            .map(|(i, p)| (i, p.clone()))
            .collect();
        if valid.is_empty() {
            valid.push((usize::MAX, fallback.to_path_buf()));
        }
        let (_, first) = &valid[0];
        let mut panel = Panel::new(first, opts, m, columns);
        let mut inv = Invalidations::default();
        for (_, p) in valid.iter().skip(1) {
            panel.new_tab(&mut inv);
            let _ = panel.navigate_to(p.clone(), &mut inv);
            // 복원한 탭의 히스토리는 새로(dir2 — 히스토리는 영속하지 않는다).
            let i = panel.active;
            panel.tabs[i].nav = History::new(p.clone());
        }
        for (slot, (orig, _)) in valid.iter().enumerate() {
            if let Some(mode) = ps.modes.get(*orig) {
                let mode = match mode.as_str() {
                    "flat" => ViewMode::Flat,
                    "tiles" => ViewMode::Tiles,
                    _ => ViewMode::Tree,
                };
                panel.tabs[slot].rows.set_view_mode(mode, &mut inv);
            }
        }
        // 활성 = 세션 인덱스가 살아남은 탭이면 그 자리 · 아니면 클램프.
        let active = valid
            .iter()
            .position(|(orig, _)| *orig == ps.active)
            .unwrap_or(0)
            .min(panel.tabs.len() - 1);
        panel.active = active;
        for (slot, (orig, _)) in valid.iter().enumerate() {
            panel.tabs[slot].locked = ps.locked.get(*orig).copied().unwrap_or(false);
            panel.tabs[slot].pinned = ps.pinned.get(*orig).copied().unwrap_or(false);
        }
        if !ps.col_widths.is_empty() {
            panel.apply_col_widths(&ps.col_widths, &mut inv);
        }
        panel.session_dirty = false;
        panel.sync_chrome(&mut inv);
        panel
    }

    /// 탭별 보기 모드(세션 `modes`).
    pub(crate) fn session_modes(&self) -> Vec<String> {
        self.tabs
            .iter()
            .map(|t| match t.rows.view_mode() {
                ViewMode::Flat => "flat".to_string(),
                ViewMode::Tiles => "tiles".to_string(),
                ViewMode::Tree => "tree".to_string(),
            })
            .collect()
    }

    /// 열 폭(활성 탭 · 표시 순) — 사용자가 바꾼 뒤에만 세션에 쓴다(기본 폭은 배치가 패널 폭에 맞춘다).
    pub(crate) fn col_widths(&self) -> Vec<i32> {
        if !self.user_cols {
            return Vec::new();
        }
        self.rows().columns().iter().map(|c| c.width).collect()
    }

    /// 세션의 열 폭을 모든 탭에(개수가 다르면 앞부분만) · 이후 배치가 기본 열을 덮지 않게.
    pub(crate) fn apply_col_widths(&mut self, widths: &[i32], inv: &mut Invalidations) {
        if widths.is_empty() {
            return;
        }
        for tab in &mut self.tabs {
            tab.rows.set_col_widths(widths, inv);
        }
        self.user_cols = true;
    }

    pub(crate) fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub(crate) fn active_index(&self) -> usize {
        self.active
    }

    pub(crate) fn rows(&self) -> &VirtualRows<TreeSource> {
        &self.tabs[self.active].rows
    }

    #[cfg_attr(not(test), allow(dead_code))] // 이름 바꾸기·선택 명령(M6)이 쓴다.
    pub(crate) fn rows_mut(&mut self) -> &mut VirtualRows<TreeSource> {
        &mut self.tabs[self.active].rows
    }

    /// 활성 탭의 선택 경로.
    pub(crate) fn selected_paths(&self) -> Vec<PathBuf> {
        self.rows().source().selected_paths()
    }

    pub(crate) fn root_path(&self) -> PathBuf {
        self.rows().source().path().to_path_buf()
    }

    pub(crate) fn bounds(&self) -> Rect {
        self.bounds
    }

    pub(crate) fn nav_rect(&self) -> Rect {
        self.navbtns.bounds()
    }

    pub(crate) fn can_back(&self) -> bool {
        self.tabs[self.active].nav.can_back()
    }

    pub(crate) fn can_forward(&self) -> bool {
        self.tabs[self.active].nav.can_forward()
    }

    /// 세션 스냅숏(탭 경로 · 활성) — T-45가 저장한다.
    #[allow(dead_code)]
    pub(crate) fn session(&self) -> (Vec<PathBuf>, usize) {
        (
            self.tabs
                .iter()
                .map(|t| t.rows.source().path().to_path_buf())
                .collect(),
            self.active,
        )
    }

    #[cfg_attr(not(test), allow(dead_code))] // T-45 디바운스 저장이 수거.
    pub(crate) fn take_session_dirty(&mut self) -> bool {
        std::mem::take(&mut self.session_dirty)
    }

    pub(crate) fn take_open(&mut self) -> Option<PathBuf> {
        self.pending_open.take()
    }

    pub(crate) fn take_rename(&mut self) -> Option<(usize, String)> {
        self.pending_rename.take()
    }

    pub(crate) fn take_ctx(&mut self) -> Option<bool> {
        self.pending_ctx.take()
    }

    pub(crate) fn take_tab_menu(&mut self) -> Option<usize> {
        self.pending_tab_menu.take()
    }

    /// 상태줄 요약(활성 탭).
    pub(crate) fn status_text(&self) -> String {
        self.rows().source().status_text()
    }

    // ── 배치·표시 ───────────────────────────────────────────

    /// 탭 바(상단) → 네비 바([홈][←][→][↑] + 경로 바) → 목록(잔여) 수직 배치(dir2 `set_bounds` · PANEL-002).
    /// 탭 바 높이 = tab_h × 필요 줄 수(멀티라인 · paint가 측정한 캐시).
    pub(crate) fn set_bounds(&mut self, bounds: Rect, inv: &mut Invalidations) {
        self.bounds = bounds;
        let tab_h = (self.m.tab_h * self.tabbar.lines().max(1) as i32).min(bounds.h);
        let bar_h = self.m.bar_h.min((bounds.h - tab_h).max(0));
        self.tabbar
            .set_bounds(Rect::new(bounds.x, bounds.y, bounds.w, tab_h), inv);
        let nav_w = (nav_btn_w(&self.m) * 4).min(bounds.w);
        self.navbtns
            .set_bounds(Rect::new(bounds.x, bounds.y + tab_h, nav_w, bar_h), inv);
        self.pathbar.set_bounds(
            Rect::new(
                bounds.x + nav_w,
                bounds.y + tab_h,
                (bounds.w - nav_w).max(0),
                bar_h,
            ),
            inv,
        );
        let list_y = bounds.y + tab_h + bar_h;
        let list_h = (bounds.bottom() - list_y).max(0);
        for tab in &mut self.tabs {
            tab.rows
                .set_bounds(Rect::new(bounds.x, list_y, bounds.w, list_h), inv);
        }
        // 자동완성 팝업 하한 = 목록 바닥.
        self.pathbar.set_overlay_bottom(list_y + list_h);
    }

    /// 지표 갱신(배율·글꼴 변경) — 모든 탭에.
    pub(crate) fn set_metrics(&mut self, m: PanelMetrics, inv: &mut Invalidations) {
        self.m = m;
        self.tabbar.set_scale(m.scale);
        self.tabbar.set_metrics(m.tab_h, m.pad_x + 4, inv);
        self.navbtns.set_scale(m.scale);
        self.pathbar.set_metrics(m.bar_h, m.pad_x, inv);
        for tab in &mut self.tabs {
            tab.rows.set_metrics(m.row_h, m.pad_x, m.indent_w, inv);
        }
    }

    /// 기본 열(패널 폭에 맞춘 것) — 사용자가 열 폭을 바꾼 뒤에는 넣지 않는다.
    pub(crate) fn set_default_columns(&mut self, cols: Vec<Column>, inv: &mut Invalidations) {
        self.base_columns = cols.clone();
        if self.user_cols {
            return;
        }
        for tab in &mut self.tabs {
            tab.rows.set_columns(cols.clone(), inv);
        }
        self.sync_columns_for_root(inv);
    }

    /// 내 PC 전용 열(이름 · 종류 · 전체 크기 · 여유 공간 — dir2 PANEL-044 · §2-5): 폭은 기본 열 것을 상속.
    fn drive_columns(&self) -> Vec<Column> {
        let w_of = |key: u32, fallback: f32| {
            self.base_columns
                .iter()
                .find(|c| c.key == key)
                .map_or((fallback * self.m.scale).round() as i32, |c| c.width)
        };
        let s = self.m.scale;
        vec![
            Column::new(
                crate::filelist::COL_NAME,
                ndir_i18n::tr("col.name"),
                w_of(crate::filelist::COL_NAME, 340.0),
            ),
            Column::new(
                crate::filelist::COL_KIND,
                ndir_i18n::tr("col.kind"),
                w_of(crate::filelist::COL_KIND, 110.0),
            ),
            Column::new(
                crate::filelist::COL_TOTAL,
                ndir_i18n::tr("col.total"),
                (110.0 * s).round() as i32,
            )
            .right_aligned(),
            Column::new(
                crate::filelist::COL_FREE,
                ndir_i18n::tr("col.free"),
                (130.0 * s).round() as i32,
            )
            .right_aligned(),
        ]
    }

    /// 가상 최상위 진입/이탈 **시점에만** 열을 교체한다(평시 재호출로 사용자 폭 리셋 방지 · dir2 X-17).
    pub(crate) fn sync_columns_for_root(&mut self, inv: &mut Invalidations) {
        if self.base_columns.is_empty() {
            return;
        }
        let virt = self.rows().source().is_virtual_root();
        let has_drive = self
            .rows()
            .columns()
            .iter()
            .any(|c| c.key == crate::filelist::COL_TOTAL);
        if virt != has_drive {
            let cols = if virt {
                self.drive_columns()
            } else {
                self.base_columns.clone()
            };
            self.tabs[self.active].rows.set_columns(cols, inv);
        }
    }

    /// 내 PC 드라이브 용량 채우기 — 호스트가 Disk 포트를 넘긴다(한 번만 · 이미 알면 건너뜀).
    pub(crate) fn fill_drive_space(
        &mut self,
        space_of: &dyn Fn(&Path) -> Option<(u64, u64)>,
        inv: &mut Invalidations,
    ) {
        let src = self.rows().source();
        if !src.is_virtual_root() || src.drive_space_known() {
            return;
        }
        let names = src.drive_names();
        let space: Vec<(String, (u64, u64))> = names
            .into_iter()
            .filter_map(|n| space_of(Path::new(&n)).map(|s| (n, s)))
            .collect();
        if !space.is_empty() {
            self.tabs[self.active]
                .rows
                .source_mut()
                .set_drive_space(space);
            inv.push(self.rows().bounds());
        }
    }

    pub(crate) fn set_focused(&mut self, focused: bool, inv: &mut Invalidations) {
        self.focused = focused;
        for tab in &mut self.tabs {
            tab.rows.set_focused(focused, inv);
        }
    }

    pub(crate) fn paint(&self, ctx: &mut dyn DrawCtx, theme: &Theme) {
        self.tabbar.paint(ctx, theme);
        ctx.fill_rect(self.navbtns.bounds(), theme.chrome_bg);
        self.navbtns.paint(ctx, theme);
        self.pathbar.paint(ctx, theme);
        self.rows().paint(ctx, theme);
    }

    /// 팝업 층(툴팁 · 경로 제안) — 호스트가 맨 뒤에 그린다.
    pub(crate) fn paint_popups(&self, ctx: &mut dyn DrawCtx, theme: &Theme) {
        self.navbtns.paint_tooltip(ctx, theme);
        let mut a = nexa_grid::Adapt(ctx);
        self.pathbar.paint_suggest(&mut a, theme);
    }

    /// 틱(목록 스크롤바 페이드 · 타입어헤드 · 툴팁) — 더 돌아야 하면 `inv.request_tick`.
    pub(crate) fn tick(&mut self, now_ms: u64, inv: &mut Invalidations) {
        self.tabs[self.active].rows.tick(now_ms, inv);
    }

    /// 탭 바·경로 바·네비 활성을 활성 탭 상태와 동기화(dir2 `sync_chrome` · PANEL-032).
    fn sync_chrome(&mut self, inv: &mut Invalidations) {
        let titles = self
            .tabs
            .iter()
            .map(|t| title_of(t.rows.source().path()))
            .collect();
        self.tabbar.set_tabs(titles, self.active, inv);
        self.tabbar
            .set_locked(self.tabs.iter().map(|t| t.locked).collect(), inv);
        self.tabbar
            .set_pinned(self.tabs.iter().map(|t| t.pinned).collect(), inv);
        let root = self.root_path();
        self.pathbar.set_path(display_path(&root), inv);
        let at_root = ndir_vfs::is_virtual_root(&root);
        let (cb, cf) = (self.can_back(), self.can_forward());
        self.navbtns.set_item_enabled(BTN_BACK, cb, inv);
        self.navbtns.set_item_enabled(BTN_FORWARD, cf, inv);
        self.navbtns.set_item_enabled(BTN_UP, !at_root, inv);
        self.navbtns.set_item_enabled(BTN_HOME, !at_root, inv);
    }

    // ── 탭 관리(dir2 F20: 패널별 탭) ───────────────────────────

    /// 현재 경로를 복제한 새 탭(Ctrl+T · [+]) — 열·포커스·보기 모드 계승 · 히스토리는 새로.
    pub(crate) fn new_tab(&mut self, inv: &mut Invalidations) {
        let path = self.root_path();
        let mut rows = VirtualRows::new(
            TreeSource::open(&path, self.opts),
            self.m.row_h,
            self.m.pad_x,
            self.m.indent_w,
        );
        rows.set_columns(self.rows().columns().to_vec(), inv);
        rows.set_focused(self.focused, inv);
        rows.set_view_mode(self.rows().view_mode(), inv);
        self.tabs.push(Tab {
            rows,
            nav: History::new(path),
            locked: false,
            pinned: false,
        });
        self.active = self.tabs.len() - 1;
        self.session_dirty = true;
        self.set_bounds(self.bounds, inv);
        self.sync_chrome(inv);
    }

    /// 탭 닫기(Ctrl+W · ×) — 패널은 항상 ≥1 탭.
    pub(crate) fn close_tab(&mut self, i: usize, inv: &mut Invalidations) {
        if self.tabs.len() <= 1 || i >= self.tabs.len() || self.tabs[i].locked {
            return;
        }
        self.session_dirty = true;
        self.tabs.remove(i);
        if self.active >= self.tabs.len() || self.active > i {
            self.active = self.active.saturating_sub(1).min(self.tabs.len() - 1);
        }
        self.sync_chrome(inv);
        inv.push(self.bounds);
    }

    pub(crate) fn switch_tab(&mut self, i: usize, inv: &mut Invalidations) {
        if i < self.tabs.len() && i != self.active {
            self.active = i;
            self.session_dirty = true;
            self.sync_columns_for_root(inv);
            self.sync_chrome(inv);
            inv.push(self.bounds);
        }
    }

    /// 다음/이전 탭으로 순환(Ctrl+Tab · Ctrl+Shift+Tab).
    pub(crate) fn next_tab(&mut self, inv: &mut Invalidations) {
        if self.tabs.len() > 1 {
            let next = (self.active + 1) % self.tabs.len();
            self.switch_tab(next, inv);
        }
    }

    pub(crate) fn prev_tab(&mut self, inv: &mut Invalidations) {
        if self.tabs.len() > 1 {
            let prev = (self.active + self.tabs.len() - 1) % self.tabs.len();
            self.switch_tab(prev, inv);
        }
    }

    /// 탭 재정렬(드래그) — 활성 인덱스 추종(dir2 `move_tab` · PANEL-017).
    pub(crate) fn move_tab(&mut self, from: usize, to: usize, inv: &mut Invalidations) {
        if from >= self.tabs.len() || to >= self.tabs.len() || from == to {
            return;
        }
        self.session_dirty = true;
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        self.active = if self.active == from {
            to
        } else if from < self.active && self.active <= to {
            self.active - 1
        } else if to <= self.active && self.active < from {
            self.active + 1
        } else {
            self.active
        };
        self.sync_chrome(inv);
        inv.push(self.bounds);
    }

    // ── 탭 잠금 · 고정 · 복제 · 패널 간 이동(dir2 PANEL-017~022) ──

    /// 탭 잠금 토글(우클릭 메뉴 — 닫기 제외).
    pub(crate) fn toggle_tab_lock(&mut self, i: usize, inv: &mut Invalidations) {
        if let Some(t) = self.tabs.get_mut(i) {
            t.locked = !t.locked;
            self.session_dirty = true;
            self.sync_chrome(inv);
        }
    }

    pub(crate) fn tab_locked(&self, i: usize) -> bool {
        self.tabs.get(i).is_some_and(|t| t.locked)
    }

    /// 탭 고정 토글 — 고정 시 핀 그룹 끝으로, 해제 시 그룹 밖으로 이동(dir2 07-15).
    pub(crate) fn toggle_tab_pin(&mut self, i: usize, inv: &mut Invalidations) {
        let Some(t) = self.tabs.get_mut(i) else {
            return;
        };
        t.pinned = !t.pinned;
        self.session_dirty = true;
        let target = self
            .tabs
            .iter()
            .enumerate()
            .filter(|(j, t)| *j != i && t.pinned)
            .count();
        if target != i {
            self.move_tab(i, target, inv);
        }
        self.sync_chrome(inv);
    }

    pub(crate) fn tab_pinned(&self, i: usize) -> bool {
        self.tabs.get(i).is_some_and(|t| t.pinned)
    }

    /// 탭 복제 — 같은 경로 · 바로 옆에 삽입 후 활성 · 복제본은 잠금·고정 해제.
    pub(crate) fn duplicate_tab(&mut self, i: usize, inv: &mut Invalidations) {
        if i >= self.tabs.len() {
            return;
        }
        let path = self.tabs[i].rows.source().path().to_path_buf();
        let mut rows = VirtualRows::new(
            TreeSource::open(&path, self.opts),
            self.m.row_h,
            self.m.pad_x,
            self.m.indent_w,
        );
        rows.set_columns(self.rows().columns().to_vec(), inv);
        rows.set_focused(self.focused, inv);
        rows.set_view_mode(self.tabs[i].rows.view_mode(), inv);
        self.tabs.insert(
            i + 1,
            Tab {
                rows,
                nav: History::new(path),
                locked: false,
                pinned: false,
            },
        );
        self.active = i + 1;
        self.session_dirty = true;
        self.set_bounds(self.bounds, inv);
        self.sync_chrome(inv);
    }

    /// 패널 간 탭 이동 — 분리(마지막 탭·잠긴 탭 거부).
    pub(crate) fn detach_tab(&mut self, i: usize, inv: &mut Invalidations) -> Option<Tab> {
        if self.tabs.len() <= 1 || i >= self.tabs.len() || self.tabs[i].locked {
            return None;
        }
        self.session_dirty = true;
        let tab = self.tabs.remove(i);
        if self.active >= self.tabs.len() || self.active > i {
            self.active = self.active.saturating_sub(1).min(self.tabs.len() - 1);
        }
        self.set_bounds(self.bounds, inv);
        self.sync_chrome(inv);
        inv.push(self.bounds);
        Some(tab)
    }

    /// 패널 간 탭 이동 — 결합: `at`(없음 = 끝)에 삽입·활성. 열 구성은 대상 패널 상속.
    pub(crate) fn attach_tab(&mut self, mut tab: Tab, at: Option<usize>, inv: &mut Invalidations) {
        self.session_dirty = true;
        let at = at.unwrap_or(self.tabs.len()).min(self.tabs.len());
        tab.rows.set_focused(self.focused, inv);
        tab.rows.set_columns(self.rows().columns().to_vec(), inv);
        tab.rows.source_mut().set_opts(self.opts);
        self.tabs.insert(at, tab);
        self.active = at;
        self.set_bounds(self.bounds, inv);
        self.sync_chrome(inv);
        inv.push(self.bounds);
    }

    pub(crate) fn session_locked(&self) -> Vec<bool> {
        self.tabs.iter().map(|t| t.locked).collect()
    }

    pub(crate) fn session_pinned(&self) -> Vec<bool> {
        self.tabs.iter().map(|t| t.pinned).collect()
    }

    /// 지금 열 폭(표시 순 · 사용자 변경 여부와 무관) — 반대 패널 동기용.
    pub(crate) fn col_widths_now(&self) -> Vec<i32> {
        self.rows().columns().iter().map(|c| c.width).collect()
    }

    /// 사용자가 열 폭을 바꿨는가(1회성 · 호스트가 동기에 쓴다).
    pub(crate) fn take_col_changed(&mut self) -> bool {
        std::mem::take(&mut self.col_changed)
    }

    // ── 네비게이션(활성 탭 — 탭별 독립) ──────────────

    /// 새 소스를 활성 탭에(스크롤·캐럿 리셋 · 열·정렬 유지).
    fn apply_source(&mut self, src: TreeSource, inv: &mut Invalidations) {
        self.tabs[self.active].rows.replace_source(src, inv);
        self.session_dirty = true;
        self.sync_columns_for_root(inv);
        self.sync_chrome(inv);
    }

    /// 새 경로 진입(히스토리 push — 앞으로 절단). 열기 실패 시 현 위치 유지(경로 바는 복귀) · 실패 사유를 돌려준다.
    pub(crate) fn navigate_to(&mut self, path: PathBuf, inv: &mut Invalidations) -> Option<String> {
        let src = TreeSource::open(&path, self.opts);
        if let Some(e) = src.error() {
            let why = e.to_string();
            self.sync_chrome(inv);
            return Some(why);
        }
        self.tabs[self.active].nav.push(path);
        self.apply_source(src, inv);
        None
    }

    pub(crate) fn nav_back(&mut self, inv: &mut Invalidations) {
        let Some(p) = self.tabs[self.active].nav.back().map(Path::to_path_buf) else {
            return;
        };
        let src = TreeSource::open(&p, self.opts);
        if src.error().is_some() {
            let _ = self.tabs[self.active].nav.forward(); // 실패 — 위치 복원
            return;
        }
        self.apply_source(src, inv);
    }

    pub(crate) fn nav_forward(&mut self, inv: &mut Invalidations) {
        let Some(p) = self.tabs[self.active].nav.forward().map(Path::to_path_buf) else {
            return;
        };
        let src = TreeSource::open(&p, self.opts);
        if src.error().is_some() {
            let _ = self.tabs[self.active].nav.back();
            return;
        }
        self.apply_source(src, inv);
    }

    /// 위로(부모 폴더) — **떠난 폴더를 자동 선택**(dir2 G-7 · `list.nav_up_align`). 드라이브 루트에서는 가상 최상위(내 PC)로.
    pub(crate) fn nav_up(&mut self, inv: &mut Invalidations) {
        let left = self.root_path();
        if ndir_vfs::is_virtual_root(&left) {
            return;
        }
        let parent = match left.parent().map(Path::to_path_buf) {
            Some(p) if !p.as_os_str().is_empty() => p,
            _ => PathBuf::from(ndir_vfs::MY_PC),
        };
        if self.navigate_to(parent, inv).is_none() {
            self.select_path(&left, inv);
        }
    }

    /// [홈] — 깊이와 무관하게 내 PC로 한 번에(dir2 08-01) · 떠난 위치를 선택.
    pub(crate) fn nav_home(&mut self, inv: &mut Invalidations) {
        let left = self.root_path();
        if ndir_vfs::is_virtual_root(&left) {
            return;
        }
        if self
            .navigate_to(PathBuf::from(ndir_vfs::MY_PC), inv)
            .is_none()
        {
            self.select_path(&left, inv);
        }
    }

    pub(crate) fn set_nav_up_align(&mut self, align: ScrollAlign) {
        self.nav_up_align = align;
    }

    /// 경로의 행을 캐럿 + 단일 선택(뷰 정렬 = `nav_up_align`).
    pub(crate) fn select_path(&mut self, path: &Path, inv: &mut Invalidations) {
        let align = self.nav_up_align;
        let rows = &mut self.tabs[self.active].rows;
        let n = rows.source().len();
        let Some(row) = (0..n).find(|&i| rows.source().row_path(i).as_deref() == Some(path)) else {
            return;
        };
        rows.select_program_aligned(row, SelectOp::Single, align, inv);
    }

    /// 행 활성화(더블클릭 · Enter): 폴더 = 진입 · 파일 = `pending_open`(실행은 호스트).
    pub(crate) fn activate_row(&mut self, row: usize, inv: &mut Invalidations) {
        let src = self.rows().source();
        let Some(path) = src.row_path(row) else {
            return;
        };
        if src.row_is_dir(row) {
            let _ = self.navigate_to(path, inv);
        } else {
            self.pending_open = Some(path);
        }
    }

    /// 보기 옵션 변경(숨김 · Dot · 폴더 우선) → 모든 탭 **무간섭 재열람**(캐럿·스크롤 유지 · 히스토리 무이동).
    pub(crate) fn set_opts(&mut self, opts: ListOpts, inv: &mut Invalidations) {
        self.opts = opts;
        self.reopen(inv);
    }

    pub(crate) fn reopen(&mut self, inv: &mut Invalidations) {
        let opts = self.opts;
        for tab in &mut self.tabs {
            let (caret, sr, sx) = (tab.rows.caret(), tab.rows.scroll_row(), tab.rows.scroll_x());
            tab.rows.source_mut().set_opts(opts);
            tab.rows.source_mut().reload();
            tab.rows.restore_view(caret, sr, sx, inv);
        }
        self.sync_chrome(inv);
    }

    pub(crate) fn set_view_mode(&mut self, mode: ViewMode, inv: &mut Invalidations) {
        self.tabs[self.active].rows.set_view_mode(mode, inv);
    }

    // ── 입력 ─────────────────────────────────────────────

    fn part_at(&self, p: Point) -> Option<Part> {
        if self.tabbar.bounds().contains(p) {
            Some(Part::Tabs)
        } else if self.navbtns.bounds().contains(p) {
            Some(Part::Nav)
        } else if self.pathbar.bounds().contains(p) {
            Some(Part::Path)
        } else if self.rows().bounds().contains(p) {
            Some(Part::List)
        } else {
            None
        }
    }

    fn send(&mut self, part: Part, ev: &InputEvent, inv: &mut Invalidations) {
        match part {
            Part::Tabs => self.tabbar.on_event(ev, inv),
            Part::Nav => self.navbtns.on_event(ev, inv),
            Part::Path => self.pathbar.on_event(ev, inv),
            Part::List => self.tabs[self.active].rows.on_event(ev, inv),
        }
    }

    /// 포인터 사건 — 눌린 부분이 캡처 · 이동은 전부(hover). 키는 [`Panel::key_event`].
    pub(crate) fn on_event(&mut self, ev: &InputEvent, inv: &mut Invalidations) {
        match *ev {
            InputEvent::MouseMove { .. } => {
                if let Some(p) = self.pressed {
                    self.send(p, ev, inv);
                } else {
                    self.tabbar.on_event(ev, inv);
                    self.navbtns.on_event(ev, inv);
                    self.pathbar.on_event(ev, inv);
                    self.tabs[self.active].rows.on_event(ev, inv);
                }
            }
            InputEvent::MouseDown { x, y, .. } => {
                let Some(part) = self.part_at(Point { x, y }) else {
                    return;
                };
                if part != Part::Path && self.pathbar.is_editing() {
                    self.pathbar.cancel_edit(inv);
                }
                self.pressed = Some(part);
                self.send(part, ev, inv);
            }
            InputEvent::MouseUp { .. } => {
                if let Some(p) = self.pressed.take() {
                    self.send(p, ev, inv);
                }
            }
            InputEvent::DoubleClick { x, y, .. } => {
                if let Some(part) = self.part_at(Point { x, y }) {
                    if part == Part::List {
                        if let Some(row) = self.rows().row_at(x, y) {
                            self.activate_row(row, inv);
                            return;
                        }
                    }
                    self.send(part, ev, inv);
                }
            }
            InputEvent::RightDown { x, y } | InputEvent::MiddleDown { x, y } => {
                if let Some(part) = self.part_at(Point { x, y }) {
                    self.send(part, ev, inv);
                    if part == Part::List && matches!(ev, InputEvent::RightDown { .. }) {
                        // 우클릭 = 행 단독 선택은 그리드가 했다 → 메뉴 종류만 보고(dir2 §2-1/§2-2).
                        self.pending_ctx = Some(self.rows().row_at(x, y).is_some());
                    }
                }
            }
            InputEvent::Wheel { .. } | InputEvent::HWheel { .. } => {
                // 휠은 호스트가 커서 아래 패널에 보낸다 — 목록으로.
                self.tabs[self.active].rows.on_event(ev, inv);
            }
            _ => {}
        }
    }

    /// 키 사건(호스트가 활성 패널에만) — 경로 바 편집 중이면 편집으로 · 아니면 목록(Enter = 활성화).
    pub(crate) fn key_event(&mut self, ev: &InputEvent, inv: &mut Invalidations) {
        if self.pathbar.is_editing() {
            match *ev {
                // Backspace = `\u{8}` 글자(dir2 EditState 규약 · `edit_char`가 처리).
                InputEvent::Char { c, .. } => self.pathbar.edit_char(c, inv),
                InputEvent::Key { key, shift, .. } => match key {
                    CtlKey::Enter => self.pathbar.submit_edit(inv),
                    CtlKey::Escape => self.pathbar.cancel_edit(inv),
                    CtlKey::Left => self.pathbar.edit_key(EditKey::Left, shift, inv),
                    CtlKey::Right => self.pathbar.edit_key(EditKey::Right, shift, inv),
                    CtlKey::Home => self.pathbar.edit_key(EditKey::Home, shift, inv),
                    CtlKey::End => self.pathbar.edit_key(EditKey::End, shift, inv),
                    CtlKey::Delete => self.pathbar.edit_key(EditKey::DeleteForward, shift, inv),
                    CtlKey::Up => {
                        self.pathbar.suggest_move(-1, inv);
                    }
                    CtlKey::Down => {
                        self.pathbar.suggest_move(1, inv);
                    }
                    _ => {}
                },
                InputEvent::SelectAll => self.pathbar.edit_key(EditKey::SelectAll, false, inv),
                _ => {}
            }
            return;
        }
        if self.rows().is_renaming() {
            // 인라인 이름 바꾸기(dir2 M3-2 · QA 07-13): 글자 = 편집 · Enter = 확정(호스트 실행) · Esc = 취소 · 편집 키.
            let rows = self.rows_mut();
            match *ev {
                InputEvent::Char { c, .. } => rows.rename_char(c, inv),
                InputEvent::Key { key, shift, .. } => match key {
                    CtlKey::Enter => {
                        if let Some(done) = rows.submit_rename(inv) {
                            self.pending_rename = Some(done);
                        }
                    }
                    CtlKey::Escape => rows.cancel_rename(inv),
                    CtlKey::Left => rows.rename_key(EditKey::Left, shift, inv),
                    CtlKey::Right => rows.rename_key(EditKey::Right, shift, inv),
                    CtlKey::Home => rows.rename_key(EditKey::Home, shift, inv),
                    CtlKey::End => rows.rename_key(EditKey::End, shift, inv),
                    CtlKey::Delete => rows.rename_key(EditKey::DeleteForward, shift, inv),
                    _ => {}
                },
                InputEvent::SelectAll => rows.rename_key(EditKey::SelectAll, false, inv),
                _ => {}
            }
            return;
        }
        if let InputEvent::Key {
            key: CtlKey::Enter, ..
        } = *ev
        {
            if let Some(row) = self.rows().caret() {
                self.activate_row(row, inv);
                return;
            }
        }
        self.tabs[self.active].rows.on_event(ev, inv);
    }

    /// 경로 바 편집 시작(우클릭 · 명령 — dir2에 단축키 없음 · 명령 표 등재는 T-44).
    #[allow(dead_code)]
    pub(crate) fn begin_path_edit(&mut self, inv: &mut Invalidations) {
        self.pathbar.begin_edit(inv);
    }

    /// 사건 뒤 컨트롤 보고 수거 — 탭 동작 · 네비 클릭 · 경로 제출 · 열 폭 변경.
    pub(crate) fn drain_actions(&mut self, inv: &mut Invalidations) {
        while let Some(act) = self.tabbar.take_action() {
            match act {
                TabAction::Switch(i) => self.switch_tab(i, inv),
                TabAction::Close(i) => self.close_tab(i, inv),
                TabAction::New => self.new_tab(inv),
                TabAction::Move { from, to } => self.move_tab(from, to, inv),
                TabAction::Context(i) => self.pending_tab_menu = Some(i),
                _ => {}
            }
        }
        while let Some(id) = self.navbtns.take_clicked() {
            match id.as_str() {
                BTN_HOME => self.nav_home(inv),
                BTN_BACK => self.nav_back(inv),
                BTN_FORWARD => self.nav_forward(inv),
                BTN_UP => self.nav_up(inv),
                _ => {}
            }
        }
        if let Some(path) = self.pathbar.take_navigation() {
            let _ = self.navigate_to(PathBuf::from(path), inv);
        }
        if self.tabs[self.active].rows.take_col_resized() {
            self.user_cols = true;
            self.col_changed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m() -> PanelMetrics {
        PanelMetrics {
            row_h: 20,
            pad_x: 6,
            indent_w: 16,
            tab_h: 22,
            bar_h: 24,
            scale: 1.0,
        }
    }

    fn opts() -> ListOpts {
        ListOpts {
            show_hidden: true,
            show_dotfiles: true,
            folders_first: true,
            case_sensitive: false,
        }
    }

    fn cols() -> Vec<Column> {
        vec![Column::new(0, "Name", 200), Column::new(2, "Size", 80)]
    }

    fn tree(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ndir-panel-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub/deep")).unwrap();
        std::fs::write(dir.join("a.txt"), b"x").unwrap();
        dir
    }

    /// dir2 `layout_stacks_tabbar_navbar_rows`: 패널 400×400 → 탭 (0,0,400,22) · 네비 (0,22,104,24) · 경로 (104,22,296,24) · 목록 (0,46,400,354).
    #[test]
    fn layout_stacks_tabbar_navbar_rows() {
        let dir = tree("layout");
        let mut p = Panel::new(&dir, opts(), m(), cols());
        let mut inv = Invalidations::default();
        p.set_bounds(Rect::new(0, 0, 400, 400), &mut inv);
        assert_eq!(p.tabbar.bounds(), Rect::new(0, 0, 400, 22));
        assert_eq!(p.nav_rect(), Rect::new(0, 22, 104, 24));
        assert_eq!(p.pathbar.bounds(), Rect::new(104, 22, 296, 24));
        assert_eq!(p.rows().bounds(), Rect::new(0, 46, 400, 354));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tabs_open_switch_close_keep_at_least_one() {
        let dir = tree("tabs");
        let mut p = Panel::new(&dir, opts(), m(), cols());
        let mut inv = Invalidations::default();
        p.set_bounds(Rect::new(0, 0, 400, 400), &mut inv);
        p.new_tab(&mut inv);
        assert_eq!(p.tab_count(), 2);
        assert_eq!(p.active_index(), 1);
        assert_eq!(p.tabbar.len(), 2);
        p.next_tab(&mut inv);
        assert_eq!(p.active_index(), 0);
        p.prev_tab(&mut inv);
        assert_eq!(p.active_index(), 1);
        p.move_tab(1, 0, &mut inv);
        assert_eq!(p.active_index(), 0);
        p.close_tab(0, &mut inv);
        assert_eq!(p.tab_count(), 1);
        p.close_tab(0, &mut inv);
        assert_eq!(p.tab_count(), 1, "마지막 탭은 안 닫힌다");
        assert!(p.take_session_dirty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn per_tab_history_and_nav_up_selects_left_folder() {
        let dir = tree("nav");
        let mut p = Panel::new(&dir, opts(), m(), cols());
        let mut inv = Invalidations::default();
        p.set_bounds(Rect::new(0, 0, 400, 400), &mut inv);
        assert!(p.navigate_to(dir.join("sub"), &mut inv).is_none());
        assert!(p.can_back() && !p.can_forward());
        assert!(p.pathbar.path().ends_with("sub"));
        assert_eq!(p.tabbar.active(), 0);
        p.new_tab(&mut inv); // 새 탭 = 같은 경로 · 히스토리 새로
        assert!(!p.can_back());
        p.nav_up(&mut inv);
        assert_eq!(p.root_path(), dir);
        let caret = p.rows().caret().expect("떠난 폴더가 캐럿");
        assert_eq!(p.rows().source().row_path(caret).unwrap(), dir.join("sub"));
        assert!(p.rows().source().is_selected(caret));
        p.switch_tab(0, &mut inv);
        assert!(p.root_path().ends_with("sub"));
        p.nav_back(&mut inv);
        assert_eq!(p.root_path(), dir);
        p.nav_forward(&mut inv);
        assert!(p.root_path().ends_with("sub"));
        // 실패 경로 = 위치 유지 + 사유.
        assert!(p.navigate_to(dir.join("nope"), &mut inv).is_some());
        assert!(p.root_path().ends_with("sub"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn activate_enters_dir_and_reports_file() {
        let dir = tree("activate");
        let mut p = Panel::new(&dir, opts(), m(), cols());
        let mut inv = Invalidations::default();
        p.set_bounds(Rect::new(0, 0, 400, 400), &mut inv);
        // 정렬: 폴더 먼저 → 0 = sub · 1 = a.txt
        p.activate_row(1, &mut inv);
        assert_eq!(p.take_open().unwrap(), dir.join("a.txt"));
        p.activate_row(0, &mut inv);
        assert!(p.root_path().ends_with("sub"));
        assert!(p.take_open().is_none());
        // 더블클릭 = 같은 길(목록 첫 행 = deep).
        let anchor = p.rows().row_anchor(0).unwrap();
        p.on_event(
            &InputEvent::DoubleClick {
                x: anchor.x + 30,
                y: anchor.y,
                shift: false,
                primary: false,
            },
            &mut inv,
        );
        assert!(p.root_path().ends_with("deep"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nav_buttons_and_path_edit() {
        let dir = tree("buttons");
        let mut p = Panel::new(&dir, opts(), m(), cols());
        let mut inv = Invalidations::default();
        p.set_bounds(Rect::new(0, 0, 400, 400), &mut inv);
        assert!(!p.navbtns.item_enabled(BTN_BACK));
        assert!(p.navbtns.item_enabled(BTN_UP));
        p.navigate_to(dir.join("sub"), &mut inv);
        assert!(p.navbtns.item_enabled(BTN_BACK));
        // 경로 바 편집: 우클릭 → 글자 → Enter = 이동(존재하는 경로).
        let pb = p.pathbar.bounds();
        p.on_event(
            &InputEvent::RightDown {
                x: pb.x + 10,
                y: pb.y + 5,
            },
            &mut inv,
        );
        assert!(p.pathbar.is_editing());
        p.key_event(&InputEvent::SelectAll, &mut inv);
        for c in dir.to_string_lossy().chars() {
            p.key_event(&InputEvent::Char { c, now_ms: 0 }, &mut inv);
        }
        p.key_event(
            &InputEvent::Key {
                key: CtlKey::Enter,
                shift: false,
                primary: false,
            },
            &mut inv,
        );
        p.drain_actions(&mut inv);
        assert_eq!(p.root_path(), dir);
        assert!(!p.pathbar.is_editing());
        // 재열람은 캐럿을 유지한다.
        p.rows_mut().select_program(1, SelectOp::Single, &mut inv);
        p.reopen(&mut inv);
        assert_eq!(p.rows().caret(), Some(1));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lock_pin_duplicate_detach_attach() {
        let dir = tree("lockpin");
        let mut p = Panel::new(&dir, opts(), m(), cols());
        let mut inv = Invalidations::default();
        p.set_bounds(Rect::new(0, 0, 400, 400), &mut inv);
        p.new_tab(&mut inv);
        p.new_tab(&mut inv); // 3탭 · 활성 2
        p.toggle_tab_lock(2, &mut inv);
        assert!(p.tab_locked(2));
        p.close_tab(2, &mut inv);
        assert_eq!(p.tab_count(), 3, "잠긴 탭은 안 닫힌다");
        assert!(
            p.detach_tab(2, &mut inv).is_none(),
            "잠긴 탭은 분리도 안 된다"
        );
        p.toggle_tab_lock(2, &mut inv);
        // 고정 = 핀 그룹 앞으로.
        p.toggle_tab_pin(2, &mut inv);
        assert!(p.tab_pinned(0) && p.active_index() == 0);
        p.toggle_tab_pin(0, &mut inv);
        assert!(!p.tab_pinned(0));
        // 복제 = 바로 옆 · 활성 · 잠금/고정 해제.
        p.navigate_to(dir.join("sub"), &mut inv);
        p.duplicate_tab(0, &mut inv);
        assert_eq!(p.tab_count(), 4);
        assert_eq!(p.active_index(), 1);
        assert!(p.root_path().ends_with("sub") && !p.tab_locked(1));
        // 분리 → 다른 패널에 부착(열 구성은 대상 상속).
        let mut q = Panel::new(&dir, opts(), m(), vec![Column::new(0, "Name", 123)]);
        q.set_bounds(Rect::new(0, 0, 300, 300), &mut inv);
        let tab = p.detach_tab(1, &mut inv).expect("detach");
        assert_eq!(p.tab_count(), 3);
        q.attach_tab(tab, None, &mut inv);
        assert_eq!(q.tab_count(), 2);
        assert_eq!(q.active_index(), 1);
        assert!(q.root_path().ends_with("sub"));
        assert_eq!(q.rows().columns()[0].width, 123);
        assert_eq!(q.session_locked(), vec![false, false]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 내 PC 진입 = 드라이브 열(이름·종류·전체·여유) · 이탈 = 기본 열 복귀 · 사용자 폭 보존.
    #[test]
    fn my_pc_switches_columns_and_back() {
        let dir = tree("mypc");
        let mut p = Panel::new(&dir, opts(), m(), cols());
        let mut inv = Invalidations::default();
        p.set_default_columns(cols(), &mut inv);
        p.set_bounds(Rect::new(0, 0, 400, 400), &mut inv);
        assert_eq!(p.rows().columns().len(), 2);
        p.nav_home(&mut inv);
        assert!(p.rows().source().is_virtual_root());
        let keys: Vec<u32> = p.rows().columns().iter().map(|c| c.key).collect();
        assert_eq!(keys, vec![0, 4, 5, 6]);
        assert_eq!(p.rows().columns()[0].width, 200, "이름 폭은 기본 열 상속");
        p.fill_drive_space(&|_| Some((10, 5)), &mut inv);
        p.nav_back(&mut inv);
        assert_eq!(p.rows().columns().len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
