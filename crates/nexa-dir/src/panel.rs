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
/// 경로 제안 최대 개수(dir2 win.rs:7204 고정값 20).
const PATH_SUGGEST_MAX: usize = 20;
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
    /// 내용이 낡았을 수 있다(dir2 X-44 S1): 배경 탭은 폴더 감시 대상이 아니라 그동안의 바깥 변경을 모른다 —
    /// 전환·닫기로 드러날 때 세우고, 호스트(`update_status`)가 [`Panel::refresh_stale`]로 다시 읽어 내린다.
    pub stale: bool,
}

/// 포인터 캡처 대상(눌린 곳이 뗄 때까지 받는다).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Part {
    Tabs,
    Nav,
    Path,
    List,
    /// 탭 상태바(패널 맨 아래).
    Status,
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
    /// 탭 본체 더블클릭 동작(설정 `tabs.dblclick` · 기본 닫기).
    tab_dbl: TabDbl,
    /// 인라인 이름 바꾸기 확정(행, 새 이름) — 실행(fs)은 호스트(`App::apply_rename`).
    pending_rename: Option<(usize, String)>,
    /// 목록 우클릭(행 위 = true · 빈 영역 = false) — 호스트가 컨텍스트 메뉴를 연다.
    pending_ctx: Option<bool>,
    /// 열 머리글 우클릭(메뉴 표시는 호스트 · 1회성 — dir2 win.rs:6262 `show_bar_popup(false)`).
    pending_header_menu: bool,
    /// 타입어헤드 설정(`typeahead.*` — 호스트가 넣는다 · 탭이 생길 때마다 그 탭 그리드에도 적용).
    ta_opts: TaOpts,
    /// 이름 앞 아이콘 끔(설정 `list.row_icons` · 성능 향상 모드 — 탭이 생길 때마다 그 탭에도 적용).
    no_row_icons: bool,
    /// 이름 바꾸기 편집 필드 우클릭(글자 편집 메뉴 · 표시는 호스트 · 1회성 — dir2 `EditMenuTarget::Rename`).
    pending_rename_menu: bool,
    /// 경로 바에 넣은 `shell:` 별칭(해석은 호스트의 플랫폼 포트 · 1회성 — dir2 panel.rs:1530).
    pending_alias: Option<String>,
    /// 경로 편집 필드 우클릭 — 호스트가 편집 메뉴(실행 취소 · 잘라내기 · 복사 · 붙여넣기 · 삭제 · 전체 선택)를 연다(GAP-012).
    pending_path_menu: bool,
    session_dirty: bool,
    /// 네비 아이콘을 만든 배율(바뀌면 다시 만든다).
    m_icon_scale: f32,
    /// 활성 탭이 다른 폴더로 들어갔다(또는 새 탭이 열렸다) — 호스트가 폴더별 보기 옵션을 맞출 계기([`Self::take_navigated`]).
    navigated: bool,
    /// 사용자가 열 폭을 바꿨다(호스트가 수거해 반대 패널에 동기 · `list.col_width_sync`).
    col_changed: bool,
    /// 열 정의 전부(숨김 열 포함 · 다시 표시할 때의 기본 폭).
    pool_columns: Vec<Column>,
    /// 사용자가 컬럼 순서를 바꿨다(호스트가 수거해 전 탭 · 반대 패널에 전파).
    col_order_changed: bool,
    /// 탭 상태바(docs/22 NEW-004 · 패널 맨 아래 한 줄): 칸 = 폴더 상태 · Git 브랜치 · 뒤에 선택 요약. 꺼짐 = 높이 0.
    status: nexa_ctl::StatusBar,
    status_on: bool,
    /// Git 브랜치 캐시 `(본 폴더, (저장소 루트, 브랜치))` — 폴더가 바뀌거나 새로 고칠 때만 다시 읽는다.
    git: Option<(PathBuf, Option<(PathBuf, String)>)>,
    /// 탭 상태바 칸 클릭 `(칸 id, 우클릭인가)` — 호스트가 상세 메뉴를 연다.
    pending_status: Option<(String, bool)>,
    /// Git 칸에 덧붙이는 요약(`↑1 ●3` — 호스트가 `git status` 결과로 넣는다 · 빈 글 = 없음).
    git_extra: String,
}

/// 탭 상태바 높이(논리 px · 창 상태줄과 같다).
const TAB_STATUS_H: f32 = 22.0;
/// 탭 상태바 칸 id.
pub(crate) const SEG_FOLDER: &str = "folder";
pub(crate) const SEG_GIT: &str = "git";

/// 네비 버튼 1개 폭(고정 — 배치가 측정 없이 계산 · dir2 `nav_btn_w`).
fn nav_btn_w(m: &PanelMetrics) -> i32 {
    m.row_h + m.pad_x
}

/// 네비 글리프 4개([홈][←][→][↑]) — dir2 = **Segoe MDL2 Assets**(HomeSolid U+EA8A · Back U+E72B · Forward U+E72A · Up U+E74A ·
/// 사용자 확정 07-18/08-01). UI 글꼴 체인에 아이콘 글꼴이 있으면(Windows · nexa-font 115차) 그대로, 없으면(macOS/Linux) 유니코드.
pub(crate) fn nav_glyphs() -> [char; 4] {
    let set = if crate::app::fonts::icon_font_available() {
        crate::app::fonts::MDL2_GLYPHS
    } else {
        crate::app::fonts::FALLBACK_GLYPHS
    };
    [set[0], set[1], set[2], set[3]]
}

/// 탭 바 지표(논리 px · dir2: 줄 높이 22 · 좌우 여백 6 + 4).
const TAB_ROW_LOGICAL: i32 = 22;
const TAB_PAD_LOGICAL: i32 = 10;

/// 네비 버튼의 SVG 자산(아이콘 글꼴이 없는 OS — Windows의 MDL2 글리프와 같은 모양으로 그린다 · 사용자 10-03
/// "내 PC, 이전, 이후, 상위 버튼의 모양이 윈도우와 다르다").
const NAV_ASSETS: [(&str, &str); 4] = [
    (BTN_HOME, "nav-home"),
    (BTN_BACK, "nav-back"),
    (BTN_FORWARD, "nav-forward"),
    (BTN_UP, "nav-up"),
];

/// 네비 버튼 아이콘 크기(논리 px · 툴바 아이콘 칸과 같다 — MDL2 em 13의 잉크와 비슷한 크기).
const NAV_ICON: i32 = 14;

/// 네비 버튼 아이콘 4개(순서 = 홈 · 뒤로 · 앞으로 · 위로): 아이콘 글꼴이 있으면 MDL2 글리프(dir2 그대로) · 없으면 SVG 마스크
/// (배율 반영 · 마스크를 못 만들면 유니코드 글리프).
fn nav_icons(scale: f32) -> [ToolIcon; 4] {
    let g = nav_glyphs();
    let px = ((NAV_ICON as f32) * scale).round().max(8.0) as u32;
    let vector = !crate::app::fonts::icon_font_available();
    std::array::from_fn(|i| {
        vector
            .then(|| crate::icons::toolbar_mask(NAV_ASSETS[i].1, px))
            .flatten()
            .map_or_else(
                || ToolIcon::Glyph(g[i].to_string()),
                |(w, h, alpha)| ToolIcon::Mask { w, h, alpha },
            )
    })
}

/// [홈][←][→][↑] — 순서·폭(`nav_btn_w` × 4 · 틈 없음)은 dir2 그대로.
/// 네비 버튼 툴팁 키(순서 = 홈 · 뒤로 · 앞으로 · 위로).
const NAV_TIP_KEYS: [&str; 4] = ["nav.mypc", "cmd.navBack", "cmd.navForward", "cmd.navUp"];

fn nav_buttons() -> Toolbar {
    let tips = NAV_TIP_KEYS;
    let items = nav_icons(1.0)
        .into_iter()
        .enumerate()
        .map(|(i, icon)| ToolItem::new(NAV_ASSETS[i].0, icon).tip(ndir_i18n::tr(tips[i])))
        .collect();
    let mut t = Toolbar::new(items);
    // dir2 배치(panel.rs:392-399 · 1546-1548): 버튼 폭 26(= 14 + 6×2) · 4개가 틈 없이 왼쪽 끝부터 · 경로 바가 바로 붙는다.
    t.set_icon_size(NAV_ICON);
    t.set_padding(6, 0);
    t.set_side_margin(0);
    t.set_item_gap(0);
    // dir2 기준(dw.rs · chrome.rs:273-295): 아이콘 글꼴 em 13 · 칸 정중앙 · hover = 배경(sel_bg) · 글리프 색은 그대로.
    t.set_icon_glyphs(crate::app::fonts::nav_glyph_delta(
        crate::app::fonts::ui_font_px(),
    ));
    t.set_hover_background(true);
    t
}

/// 타입어헤드 설정 묶음(`typeahead.*` · nexa-sql `explorer.typeahead*` 기준 + dir2 계승 Backspace).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TaOpts {
    pub enabled: bool,
    pub reset_ms: u64,
    pub special: bool,
    pub space: bool,
    pub backspace: bool,
    /// 배지 위치(0~8 = 행 × 3 + 열 · 6 = 왼쪽 아래).
    pub hud_pos: u8,
}

impl Default for TaOpts {
    fn default() -> Self {
        Self {
            enabled: true,
            reset_ms: 2000,
            special: true,
            space: true,
            backspace: true,
            hud_pos: 6,
        }
    }
}

/// 배지 위치 설정 문자열(`top_left` … `bottom_right`) → 0~8(행 × 3 + 열) · 모르는 값 = 왼쪽 아래(6).
pub(crate) fn hud_pos_index(value: &str) -> u8 {
    match value.trim() {
        "top_left" => 0,
        "top_center" => 1,
        "top_right" => 2,
        "mid_left" => 3,
        "center" => 4,
        "mid_right" => 5,
        "bottom_center" => 7,
        "bottom_right" => 8,
        _ => 6,
    }
}

/// 패널당 폴더 감시 상한(dir2 `WATCH_CAP` · 현재 폴더 포함).
const WATCH_CAP: usize = 64;

/// 탭 본체 더블클릭 동작(설정 `tabs.dblclick` · dir2 `tab_dblclick`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TabDbl {
    Close,
    Pin,
    Lock,
}

impl TabDbl {
    /// 설정 값 → 동작(dir2와 같이 모르는 값 = 닫기).
    pub(crate) fn parse(value: &str) -> Self {
        match value {
            "pin" => Self::Pin,
            "lock" => Self::Lock,
            _ => Self::Close,
        }
    }
}

impl Panel {
    /// 첫 탭을 `path`로 시작(열기 실패여도 패널은 만들어진다 — 빈 목록 + 오류 문구).
    pub(crate) fn new(path: &Path, opts: ListOpts, m: PanelMetrics, columns: Vec<Column>) -> Panel {
        let mut inv = Invalidations::default();
        let mut rows = VirtualRows::new(TreeSource::open(path, opts), m.row_h, m.pad_x, m.indent_w);
        rows.set_columns(columns, &mut inv);
        rows.set_sort_mark_trailing(true, &mut inv); // 정렬 표시 = 칸 오른쪽 끝(nexa-sql 모양 · 사용자 10-03)
        rows.set_col_drag_marker(true); // 컬럼을 끄는 동안 놓일 자리 표식(T-123)
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
                stale: false,
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
            tab_dbl: TabDbl::Close,
            pending_rename: None,
            pending_ctx: None,
            pending_header_menu: false,
            ta_opts: TaOpts::default(),
            no_row_icons: false,
            pending_rename_menu: false,
            pending_alias: None,
            pending_path_menu: false,
            session_dirty: false,
            m_icon_scale: 1.0,
            navigated: false,
            col_changed: false,
            pool_columns: Vec::new(),
            col_order_changed: false,
            status: {
                let mut sb = nexa_ctl::StatusBar::new();
                sb.set_segments_leading(true, &mut inv);
                sb
            },
            status_on: false,
            git: None,
            pending_status: None,
            git_extra: String::new(),
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
        pool: Vec<Column>,
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
        panel.pool_columns = pool; // 저장된 레이아웃이 기본 숨김 열(확장자 · 종류)을 켜 두었을 때 그 열의 원형
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
        // 탭별 보기 옵션(dir2 `views` — 없는 자리는 설정 기본 그대로).
        for (slot, (orig, _)) in valid.iter().enumerate() {
            if let Some(&f) = ps.views.get(*orig) {
                let o = opts.with_view(ListOpts::view_of_flags(f));
                panel.tabs[slot].rows.source_mut().set_opts(o);
            }
        }
        for (slot, (orig, _)) in valid.iter().enumerate() {
            panel.tabs[slot].locked = ps.locked.get(*orig).copied().unwrap_or(false);
            panel.tabs[slot].pinned = ps.pinned.get(*orig).copied().unwrap_or(false);
        }
        // 열 순서/표시(`cols` · T-71) → 그 위에 열 폭(표시 순으로 저장돼 있다).
        if !ps.col_layout.is_empty() {
            let parsed =
                crate::order::parse_order_with(crate::order::COLUMN_BLOCKS, &ps.col_layout);
            if let Some((_, _, items)) = parsed.first() {
                let spec: Vec<(u32, bool)> = items
                    .iter()
                    .filter_map(|(k, v)| crate::order::col_key_id(k).map(|id| (id, *v)))
                    .collect();
                panel.apply_col_layout(&spec, &mut inv);
            }
        }
        // 열 폭은 저장 당시의 표시 순서대로 적혀 있다 → 그때의 열 종류에 맞춰 넣는다(기본 열 구성이 바뀌어도 · 저장값에
        // 없던 열이 끼어들어도 폭이 엉뚱한 열로 밀리지 않게 — 10-03 상태 열 추가).
        if !ps.col_widths.is_empty() {
            let keys = crate::order::saved_visible_keys(&ps.col_layout, ps.col_widths.len());
            let by_key: Vec<(u32, i32)> = keys
                .iter()
                .zip(&ps.col_widths)
                .filter_map(|(k, w)| crate::order::col_key_id(k).map(|id| (id, *w)))
                .collect();
            if !panel.apply_col_widths_by_key(&by_key, &mut inv) {
                panel.user_cols = true; // 폭이 기본과 같아도 저장된 폭이 있었다 = 사용자 폭
            }
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

    /// 현재 열 레이아웃 문자열(`cols:1[name:1,…]` — 표시 열 = 기본 열 순 · 숨김 열 = 정의 순으로 말미 `:0` · dir2 `panel_col_layout`).
    /// 기본 열(`base_columns`)이 기준이라 가상 최상위(드라이브 열)에서도 사용자 레이아웃이 나온다.
    pub(crate) fn col_layout_str(&self) -> String {
        let cols: &[Column] = if self.base_columns.is_empty() {
            self.rows().columns()
        } else {
            &self.base_columns
        };
        let mut items: Vec<(String, bool)> = cols
            .iter()
            .filter(|c| crate::order::col_key_id(crate::order::col_id_key(c.key)) == Some(c.key))
            .map(|c| (crate::order::col_id_key(c.key).to_string(), true))
            .collect();
        for (_, defs) in crate::order::COLUMN_BLOCKS {
            for d in *defs {
                if !items.iter().any(|(k, _)| k == d) {
                    items.push((d.to_string(), false));
                }
            }
        }
        crate::order::serialize_order_with(&[("cols".to_string(), true, items)], true)
    }

    /// 열 레이아웃(key · 표시) 적용 — 현재 폭 보존 · 재표시 열 = 기본 폭 · 전부 숨김이면 첫 정의 열(name) 강제(dir2 `apply_col_layout`).
    /// 기본 열을 바꾸고 가상 최상위가 아닌 탭에 넣는다(드라이브 열 탭은 `sync_columns_for_root`가 돌아올 때 기본 열을 쓴다).
    pub(crate) fn apply_col_layout(&mut self, spec: &[(u32, bool)], inv: &mut Invalidations) {
        let cur: Vec<Column> = self.rows().columns().to_vec();
        let base = if self.base_columns.is_empty() {
            cur.clone()
        } else {
            self.base_columns.clone()
        };
        let mut out: Vec<Column> = Vec::new();
        for (key, vis) in spec {
            if !vis {
                continue;
            }
            if let Some(c) = cur
                .iter()
                .chain(base.iter())
                .chain(self.pool_columns.iter())
                .find(|c| c.key == *key)
            {
                out.push(c.clone());
            }
        }
        if out.is_empty() {
            // 전부 숨김 = name 강제(정의상 첫 열 · 현재 순서가 아니라 key로 찾는다).
            if let Some(c) = cur
                .iter()
                .chain(base.iter())
                .find(|c| c.key == crate::filelist::COL_NAME)
                .or_else(|| base.first())
                .cloned()
            {
                out.push(c);
            }
        }
        if out.is_empty() {
            return;
        }
        self.base_columns = out.clone();
        self.user_cols = true;
        for tab in &mut self.tabs {
            if !tab.rows.source().is_virtual_root() {
                tab.rows.set_columns(out.clone(), inv);
            }
        }
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

    pub(crate) fn selected_paths_in_view_order(&self) -> Vec<PathBuf> {
        self.rows().source().selected_paths_in_view_order()
    }

    pub(crate) fn root_path(&self) -> PathBuf {
        self.rows().source().path().to_path_buf()
    }

    /// 폴더 감시 대상(dir2 WINB-014 `sync_watchers`): 현재 폴더 + 화면에 펼쳐진 하위 폴더(패널당 [`WATCH_CAP`]개까지 ·
    /// 넘는 것은 감시하지 않는다 = F5). 종전 dir3 = 현재 폴더만 → 펼친 폴더 안의 바깥 변경이 안 보였다.
    pub(crate) fn watch_dirs(&self) -> Vec<PathBuf> {
        let src = self.rows().source();
        let mut dirs = vec![src.path().to_path_buf()];
        dirs.extend(src.expanded_dirs(WATCH_CAP - 1));
        dirs
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

    pub(crate) fn take_alias(&mut self) -> Option<String> {
        self.pending_alias.take()
    }

    /// 끌어오다 머문 자리의 대상(dir2 win.rs `dnd_hover` 판정 · 탭이 폴더보다 우선): 활성이 아닌 탭 위 = 그 탭 ·
    /// 폴더 행 위 = 그 폴더 경로. 그 밖 = `None`.
    pub(crate) fn dnd_dwell_at(&self, x: i32, y: i32) -> Option<crate::app::dnd::Dwell> {
        use crate::app::dnd::Dwell;
        if let Some(t) = self.tabbar.tab_index_at(x, y) {
            return (t != self.active).then_some(Dwell::Tab(t));
        }
        let rows = self.rows();
        let row = rows.row_at(x, y)?;
        let path = rows.source().row_path(row)?;
        rows.source().row_is_dir(row).then_some(Dwell::Folder(path))
    }

    /// 머문 대상을 연다: 탭 = 전환 · 폴더 = 그 행을 펼친다(트리 보기에서 하위에 놓을 수 있게). 바뀌었으면 `true`.
    pub(crate) fn dnd_dwell_open(
        &mut self,
        target: &crate::app::dnd::Dwell,
        inv: &mut Invalidations,
    ) -> bool {
        use crate::app::dnd::Dwell;
        match target {
            Dwell::Tab(t) => {
                let before = self.active;
                self.switch_tab(*t, inv);
                self.active != before
            }
            Dwell::Folder(path) => {
                // 트리 보기 = 그 행을 펼친다(하위에 놓을 수 있게) · 그 밖의 보기(목록 · 타일) = 그 폴더 안으로 들어간다
                // (사용자 10-05 — 펼칠 수 없는 보기에서는 들어가야 하위에 놓을 수 있다 · Finder 스프링 폴더와 같은 뜻).
                if self.rows().view_mode() != ViewMode::Tree {
                    return self.navigate_to(path.clone(), inv).is_none();
                }
                let rows = &mut self.tabs[self.active].rows;
                let n = rows.source().len();
                let Some(row) =
                    (0..n).find(|&i| rows.source().row_path(i).as_deref() == Some(path.as_path()))
                else {
                    return false;
                };
                let opened = rows.source_mut().expand_row(row);
                if opened {
                    inv.push(self.bounds);
                }
                opened
            }
        }
    }

    /// 경로의 행이 화면에서 차지하는 사각형(목록 폭 전체 × 행 높이) — 보이지 않거나 타일 보기면 `None`(놓일 폴더 강조용).
    #[cfg(test)]
    pub(crate) fn row_rect_of(&self, path: &Path) -> Option<Rect> {
        let rows = self.rows();
        if rows.view_mode() == ViewMode::Tiles {
            return None;
        }
        let n = rows.source().len();
        let row = (0..n).find(|&i| rows.source().row_path(i).as_deref() == Some(path))?;
        let a = rows.row_anchor(row)?;
        let b = rows.bounds();
        let h = self.m.row_h;
        Some(Rect::new(b.x, a.y - h / 2, b.w, h))
    }

    /// 폴더 `dir`의 **화면에 보이는 묶음**(그 폴더 행 + 펼쳐진 하위 행들)이 차지하는 사각형 — 보이는 행이 하나도 없거나
    /// 타일 보기면 `None`. 보이는 범위의 행만 훑는다(큰 목록에서도 싸다).
    pub(crate) fn folder_block_rect(&self, dir: &Path) -> Option<Rect> {
        let rows = self.rows();
        if rows.view_mode() == ViewMode::Tiles {
            return None;
        }
        let b = rows.bounds();
        let h = self.m.row_h.max(1);
        let first = rows.scroll_row();
        let last = (first + (b.h / h) as usize + 2).min(rows.source().len());
        let (mut top, mut bottom) = (i32::MAX, i32::MIN);
        for i in first..last {
            let Some(path) = rows.source().row_path(i) else {
                continue;
            };
            if path != dir && !path.starts_with(dir) {
                continue;
            }
            if let Some(a) = rows.row_anchor(i) {
                top = top.min(a.y - h / 2);
                bottom = bottom.max(a.y - h / 2 + h);
            }
        }
        (top < bottom).then(|| Rect::new(b.x, top, b.w, bottom - top))
    }

    /// OS 드래그에서 돌아온 뒤 누름 상태 정리(그리드의 클릭 확정 보류 · 러버밴드 + 패널의 포인터 캡처) — 선택은 그대로.
    pub(crate) fn abort_press(&mut self) {
        self.tabs[self.active].rows.abort_press();
        self.pressed = None;
    }

    pub(crate) fn take_rename_menu(&mut self) -> bool {
        std::mem::take(&mut self.pending_rename_menu)
    }

    pub(crate) fn take_header_menu(&mut self) -> bool {
        std::mem::take(&mut self.pending_header_menu)
    }

    /// 경로 편집 필드 위인가 — 편집 중 + 경로 바 영역(그리기 캐시에 기대지 않는다: 편집 필드는 경로 바 전체를 차지한다).
    fn path_edit_at(&self, x: i32, y: i32) -> bool {
        self.pathbar.is_editing() && self.pathbar.bounds().contains(Point { x, y })
    }

    /// 경로 편집 메뉴 요청을 한 번 꺼낸다.
    pub(crate) fn take_path_menu(&mut self) -> bool {
        std::mem::take(&mut self.pending_path_menu)
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
        // 탭 상태바(켜져 있으면 목록 높이에서 뺀다).
        let status_h = if self.status_on {
            ((TAB_STATUS_H * self.m.scale).round() as i32).min((bounds.bottom() - list_y).max(0))
        } else {
            0
        };
        let list_h = (bounds.bottom() - list_y - status_h).max(0);
        self.status.set_scale(self.m.scale);
        self.status.set_bounds(
            Rect::new(bounds.x, list_y + list_h, bounds.w, status_h),
            inv,
        );
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
        // TabBar는 줄 높이·여백을 **논리 px**로 받아 배율을 스스로 곱한다(장치 px를 넘기면 배율 ≠ 1에서 여백이 두 배 ·
        // 여러 줄일 때 둘째 줄이 잘린다 — 10-03 적발). 줄 높이의 장치 px는 `m.tab_h`(= 22 × 배율)와 같아진다.
        self.tabbar
            .set_metrics(TAB_ROW_LOGICAL, TAB_PAD_LOGICAL, inv);
        // 배율이 바뀌면 SVG 네비 아이콘을 그 크기로 다시 만든다(글리프면 그대로).
        if (self.m_icon_scale - m.scale).abs() > f32::EPSILON {
            self.m_icon_scale = m.scale;
            for (i, icon) in nav_icons(m.scale).into_iter().enumerate() {
                self.navbtns.set_item_icon(NAV_ASSETS[i].0, icon, inv);
            }
        }
        self.navbtns.set_scale(m.scale);
        self.pathbar.set_metrics(m.bar_h, m.pad_x, inv);
        for tab in &mut self.tabs {
            tab.rows.set_metrics(m.row_h, m.pad_x, m.indent_w, inv);
        }
    }

    /// 기본 열(패널 폭에 맞춘 것) — 사용자가 열 폭을 바꾼 뒤에는 넣지 않는다.
    /// `pool` = 숨김 열까지 포함한 정의 전부(다시 표시할 때의 기본 폭 · [`Self::apply_col_layout`]).
    pub(crate) fn set_default_columns(
        &mut self,
        cols: Vec<Column>,
        pool: Vec<Column>,
        inv: &mut Invalidations,
    ) {
        self.pool_columns = pool;
        if self.user_cols {
            // 사용자 레이아웃(순서 · 표시 · 폭)은 배치가 덮지 않는다 — 단 ① 세션에서 복원한 패널은 기본 열이 비어 있어
            // (내 PC 드라이브 열 교체 · 새 탭 상속이 멈춘다) 지금 열로 채우고 ② 제목은 지금 언어로 다시 붙인다(언어 전환).
            if self.base_columns.is_empty() {
                self.base_columns = self
                    .tabs
                    .iter()
                    .find(|t| !t.rows.source().is_virtual_root())
                    .map_or(cols, |t| t.rows.columns().to_vec());
            }
            self.retitle_columns(inv);
            self.sync_columns_for_root(inv);
            return;
        }
        self.base_columns = cols.clone();
        for tab in &mut self.tabs {
            tab.rows.set_columns(cols.clone(), inv);
        }
        self.sync_columns_for_root(inv);
    }

    /// 열 제목을 지금 언어로(폭 · 순서 · 표시는 그대로) — 열 정의 원형(`pool_columns`)의 제목을 key로 찾아 넣는다.
    fn retitle_columns(&mut self, inv: &mut Invalidations) {
        let pool = self.pool_columns.clone();
        let title_of = |key: u32| -> Option<String> {
            if key == crate::filelist::COL_TOTAL {
                return Some(ndir_i18n::tr("col.total"));
            }
            if key == crate::filelist::COL_FREE {
                return Some(ndir_i18n::tr("col.free"));
            }
            pool.iter().find(|c| c.key == key).map(|c| c.title.clone())
        };
        let retitle = |cols: &mut Vec<Column>| -> bool {
            let mut changed = false;
            for c in cols.iter_mut() {
                if let Some(t) = title_of(c.key) {
                    if c.title != t {
                        c.title = t;
                        changed = true;
                    }
                }
            }
            changed
        };
        retitle(&mut self.base_columns);
        for tab in &mut self.tabs {
            let mut cols = tab.rows.columns().to_vec();
            if retitle(&mut cols) {
                tab.rows.set_columns(cols, inv);
            }
        }
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
        if self.status_on {
            self.status.paint(ctx, theme);
        }
    }

    /// 언어 전환 — 네비 버튼 툴팁 · 탭 제목/경로 바의 "내 PC"를 지금 언어로(T-134).
    pub(crate) fn relabel(&mut self, inv: &mut Invalidations) {
        for (i, key) in NAV_TIP_KEYS.iter().enumerate() {
            self.navbtns
                .set_item_tip(NAV_ASSETS[i].0, &ndir_i18n::tr(key));
        }
        self.sync_chrome(inv);
    }

    /// 탭 상태바 켜기/끄기(설정 `layout.tab_statusbar`) — 바뀌면 다시 배치한다.
    pub(crate) fn set_tab_status(&mut self, on: bool, inv: &mut Invalidations) {
        if self.status_on != on {
            self.status_on = on;
            let b = self.bounds;
            self.set_bounds(b, inv);
        }
    }

    /// 탭 상태바 내용을 활성 탭에 맞춘다(호스트의 `update_status` 길목): 칸 ① 폴더 상태(보이는 항목 수) ② Git 브랜치 ·
    /// 칸 뒤 = 선택 요약(선택이 있을 때).
    pub(crate) fn sync_status(&mut self, inv: &mut Invalidations) {
        if !self.status_on {
            return;
        }
        let root = self.root_path();
        if self.git.as_ref().is_none_or(|(p, _)| *p != root) {
            let found = if ndir_vfs::is_virtual_root(&root) || ndir_vfs::is_network_path(&root) {
                None
            } else {
                crate::dirinfo::git_branch(&root)
            };
            self.git = Some((root, found));
        }
        let src = self.rows().source();
        let mut segs = vec![nexa_ctl::StatusSeg::new(
            SEG_FOLDER,
            ndir_i18n::trf("status.itemCount", &[&src.len().to_string()]),
        )];
        if let Some((_, Some((_, branch)))) = &self.git {
            let mut text = ndir_i18n::trf("tabstatus.git", &[branch]);
            if !self.git_extra.is_empty() {
                text.push(' ');
                text.push_str(&self.git_extra);
            }
            segs.push(nexa_ctl::StatusSeg::new(SEG_GIT, text));
        }
        let sel = src.selection_count();
        let left = if sel > 0 {
            ndir_i18n::trf("status.selectedCount", &[&sel.to_string()])
        } else {
            String::new()
        };
        self.status.set_segments(segs, inv);
        self.status.set_left(&left, inv);
    }

    /// 탭 상태바의 Git 정보 `(저장소 루트, 브랜치)`(활성 탭 · 저장소 밖 = `None`).
    pub(crate) fn git_info(&self) -> Option<(PathBuf, String)> {
        self.git.as_ref().and_then(|(_, g)| g.clone())
    }

    /// Git 칸 요약(앞섬 · 뒤짐 · 변경 수)을 넣는다 — 다음 [`Self::sync_status`]가 칸에 반영한다.
    pub(crate) fn set_git_extra(&mut self, extra: String) {
        self.git_extra = extra;
    }

    /// Git 캐시를 버린다(새로 고침 · 수동 갱신) — 다음 [`Self::sync_status`]가 다시 읽는다.
    pub(crate) fn invalidate_dir_info(&mut self) {
        self.git = None;
    }

    /// 목록이 쓰는 메모리 추정(바이트 · 메모리 창) — 모든 탭의 행 수 × 행당 어림값(이름 · 메타 · 트리 노드).
    #[cfg(test)]
    pub(crate) fn mem_estimate(&self) -> u64 {
        let (active, background) = self.mem_estimate_split();
        active + background
    }

    /// 파일 목록 어림을 `(보이는 탭, 배경 탭)`으로 나눈 것(메모리 창 — 탭을 닫거나 다른 폴더로 가면 줄어드는 것이 보인다).
    pub(crate) fn mem_estimate_split(&self) -> (u64, u64) {
        const PER_ROW: u64 = 256;
        let (mut active, mut background) = (0u64, 0u64);
        for (i, t) in self.tabs.iter().enumerate() {
            let bytes = t.rows.source().len() as u64 * PER_ROW;
            if i == self.active {
                active += bytes;
            } else {
                background += bytes;
            }
        }
        (active, background)
    }

    /// 탭 상태바 칸 클릭(1회성 수거).
    pub(crate) fn take_status_click(&mut self) -> Option<(String, bool)> {
        self.pending_status.take()
    }

    /// 탭 상태바 자리(꺼짐 = 높이 0 · 덤프 · 시험).
    pub(crate) fn status_bounds(&self) -> Rect {
        self.status.bounds()
    }

    /// 탭 상태바 칸 글(시험 · 덤프): `id=글` 목록.
    pub(crate) fn status_summary(&self) -> Vec<String> {
        self.status
            .segments()
            .iter()
            .map(|s| format!("{}={}", s.id, s.text))
            .collect()
    }

    /// 탭 상태바 칸 자리(마지막으로 그린 자리 · 시험).
    #[cfg(test)]
    pub(crate) fn status_seg_rect(&self, id: &str) -> Option<Rect> {
        self.status.seg_rect(id)
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
    /// 경로 바에 친 글 → 실제 경로 글(`pathexpand` — `%VAR%` · `$env:VAR` · `$HOME` · `${VAR:-기본}` · `$(basename $PWD)` ·
    /// `~` · 상대 경로). `$PWD`와 상대 경로의 기준 = 이 패널의 현재 폴더(내 PC면 없음).
    pub(crate) fn expand_path_input(&self, text: &str) -> String {
        let root = self.root_path();
        let pwd = (!ndir_vfs::is_virtual_root(&root)).then_some(root.as_path());
        crate::pathexpand::expand_native(text, pwd)
    }

    /// 타입어헤드 설정 적용(`typeahead.*`) — 지금 있는 탭 전부에 넣고, 뒤에 생기는 탭은 [`Self::sync_chrome`]이 넣는다.
    pub(crate) fn set_typeahead(&mut self, opts: TaOpts, inv: &mut Invalidations) {
        self.ta_opts = opts;
        self.apply_typeahead(inv);
    }

    /// 이름 앞 아이콘 켬/끔(모든 탭 · 뒤에 생기는 탭은 [`Self::sync_chrome`]이 넣는다).
    pub(crate) fn set_row_icons(&mut self, on: bool, inv: &mut Invalidations) {
        self.no_row_icons = !on;
        self.apply_typeahead(inv);
    }

    fn apply_typeahead(&mut self, inv: &mut Invalidations) {
        let o = self.ta_opts;
        let icons = !self.no_row_icons;
        for tab in &mut self.tabs {
            if tab.rows.source_mut().set_icons(icons) {
                inv.push(tab.rows.bounds());
            }
            tab.rows.set_typeahead_opts(
                o.reset_ms,
                o.special,
                o.space,
                o.backspace,
                o.hud_pos,
                inv,
            );
            tab.rows.set_typeahead_enabled(o.enabled, inv);
        }
    }

    /// 활성 탭이 타입어헤드 입력 중인가(호스트가 유지 시간 만료를 보려고 깨어 있어야 하는 동안).
    pub(crate) fn typeahead_active(&self) -> bool {
        self.rows().typeahead_active()
    }

    fn sync_chrome(&mut self, inv: &mut Invalidations) {
        self.apply_typeahead(inv); // 새로 생긴 탭에도 같은 설정
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
            TreeSource::open(&path, self.opts), // 새 탭의 보기 옵션 = 설정 기본값(사용자 10-03)
            self.m.row_h,
            self.m.pad_x,
            self.m.indent_w,
        );
        rows.set_columns(self.rows().columns().to_vec(), inv);
        rows.set_sort_mark_trailing(true, inv);
        rows.set_col_drag_marker(true);
        rows.set_focused(self.focused, inv);
        rows.set_view_mode(self.rows().view_mode(), inv);
        self.tabs.push(Tab {
            rows,
            nav: History::new(path),
            locked: false,
            pinned: false,
            stale: false,
        });
        self.active = self.tabs.len() - 1;
        self.navigated = true;
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
        let was_active = self.active == i;
        self.tabs.remove(i);
        if self.active >= self.tabs.len() || self.active > i {
            self.active = self.active.saturating_sub(1).min(self.tabs.len() - 1);
        }
        if was_active {
            // 이웃 탭이 새로 드러난다 — 배경에 있던 동안의 낡음 해소(dir2 panel.rs:730-733).
            self.tabs[self.active].stale = true;
        }
        self.sync_chrome(inv);
        inv.push(self.bounds);
    }

    pub(crate) fn switch_tab(&mut self, i: usize, inv: &mut Invalidations) {
        if i < self.tabs.len() && i != self.active {
            self.active = i;
            // 활성화 = 갱신 계기(dir2 panel.rs:741-744): 배경 탭은 감시 대상이 아니다 → 호스트가 다시 읽는다.
            self.tabs[i].stale = true;
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
            TreeSource::open(&path, self.tabs[i].rows.source().opts()), // 원본 탭 보기 옵션 계승
            self.m.row_h,
            self.m.pad_x,
            self.m.indent_w,
        );
        rows.set_columns(self.rows().columns().to_vec(), inv);
        rows.set_sort_mark_trailing(true, inv);
        rows.set_col_drag_marker(true);
        rows.set_focused(self.focused, inv);
        rows.set_view_mode(self.tabs[i].rows.view_mode(), inv);
        self.tabs.insert(
            i + 1,
            Tab {
                rows,
                nav: History::new(path),
                locked: false,
                pinned: false,
                stale: false,
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
    /// 보기 옵션(숨김 · Dot · 폴더 우선): `adopt` = 대상 패널(활성 탭) 값 채택(범위 ≠ "활성 탭" — 패널 안 값 균일 유지) ·
    /// 아니면 탭이 지닌 값 그대로(dir2 08-02 `cross_move_tab`).
    pub(crate) fn attach_tab(
        &mut self,
        mut tab: Tab,
        at: Option<usize>,
        adopt: bool,
        inv: &mut Invalidations,
    ) {
        self.session_dirty = true;
        let at = at.unwrap_or(self.tabs.len()).min(self.tabs.len());
        tab.rows.set_focused(self.focused, inv);
        tab.rows.set_columns(self.rows().columns().to_vec(), inv);
        let view = if adopt {
            self.tab_opts().view()
        } else {
            tab.rows.source().opts().view()
        };
        tab.rows.source_mut().set_opts(self.opts.with_view(view));
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
    #[cfg(test)]
    pub(crate) fn col_widths_now(&self) -> Vec<i32> {
        self.rows().columns().iter().map(|c| c.width).collect()
    }

    /// 지금 열 폭을 **열 종류(key)별로**(활성 탭) — 패널 사이 동기는 자리(순서)가 아니라 key로 맞춘다(두 패널의 열 순서 ·
    /// 표시 열이 다르거나 한쪽이 내 PC(드라이브 열)를 보고 있어도 같은 열끼리만 폭이 옮겨 간다).
    pub(crate) fn col_widths_by_key(&self) -> Vec<(u32, i32)> {
        self.rows()
            .columns()
            .iter()
            .map(|c| (c.key, c.width))
            .collect()
    }

    /// key별 열 폭을 **이 패널의 모든 탭**에(없는 열은 건너뜀 · 기본 열에도 반영해 새 탭 · 내 PC 열이 그 폭을 잇는다).
    /// 바뀐 것이 있으면 `true`. 이후 배치가 기본 열로 덮지 않는다.
    pub(crate) fn apply_col_widths_by_key(
        &mut self,
        widths: &[(u32, i32)],
        inv: &mut Invalidations,
    ) -> bool {
        let of = |key: u32, cur: i32| {
            widths
                .iter()
                .find(|(k, _)| *k == key)
                .map_or(cur, |(_, w)| *w)
        };
        let mut changed = false;
        for tab in &mut self.tabs {
            let cur: Vec<i32> = tab.rows.columns().iter().map(|c| c.width).collect();
            let new: Vec<i32> = tab
                .rows
                .columns()
                .iter()
                .map(|c| of(c.key, c.width))
                .collect();
            if new != cur {
                tab.rows.set_col_widths(&new, inv);
                changed = true;
            }
        }
        for c in &mut self.base_columns {
            c.width = of(c.key, c.width).max(c.min_width);
        }
        if changed {
            self.user_cols = true;
            self.session_dirty = true;
        }
        changed
    }

    /// 활성 탭에 **지금 보이는 순서**의 열 레이아웃 `(key, 표시)`: 보이는 열 = 현재 순서 · 안 보이는 정의 열 = 뒤에 숨김으로.
    /// (`col_layout_str`은 기본 열 기준이라 방금 끌어 바꾼 순서를 아직 모른다.)
    pub(crate) fn col_order_spec(&self) -> Vec<(u32, bool)> {
        let mut spec: Vec<(u32, bool)> = self
            .rows()
            .columns()
            .iter()
            .filter(|c| crate::order::col_key_id(crate::order::col_id_key(c.key)) == Some(c.key))
            .map(|c| (c.key, true))
            .collect();
        for (_, defs) in crate::order::COLUMN_BLOCKS {
            for d in *defs {
                if let Some(id) = crate::order::col_key_id(d) {
                    if !spec.iter().any(|(k, _)| *k == id) {
                        spec.push((id, false));
                    }
                }
            }
        }
        spec
    }

    /// 사용자가 컬럼을 끌어 순서를 바꿨는가(1회성 · 호스트가 전파에 쓴다).
    pub(crate) fn take_col_order_changed(&mut self) -> bool {
        std::mem::take(&mut self.col_order_changed)
    }

    /// 컬럼 드래그 중이면 접는다(Esc — 원래 순서로 · dir2 WINC-031) · 접었으면 `true`.
    pub(crate) fn cancel_col_drag(&mut self, inv: &mut Invalidations) -> bool {
        let done = self.tabs[self.active].rows.cancel_col_drag(inv);
        if done && self.pressed == Some(Part::List) {
            self.pressed = None;
        }
        done
    }

    /// 지금 컬럼을 끌고 있는가.
    pub(crate) fn col_dragging(&self) -> bool {
        self.rows().col_dragging()
    }

    /// 사용자가 열 폭을 바꿨는가(1회성 · 호스트가 동기에 쓴다).
    /// 열 하나의 폭을 호스트가 정한다(자동 맞춤 · T-132) — 사용자 폭으로 남는다(배치가 기본 열로 덮지 않고 세션에 저장).
    /// 바뀌었으면 `true`.
    pub(crate) fn set_col_width_user(
        &mut self,
        col: usize,
        w: i32,
        inv: &mut Invalidations,
    ) -> bool {
        let i = self.active;
        self.tabs[i].rows.set_col_width(col, w, inv);
        if !self.tabs[i].rows.take_col_resized() {
            return false;
        }
        self.user_cols = true;
        self.session_dirty = true;
        true
    }

    /// 눌러서 끄는 중인가(열 폭 · 열 순서 · 탭 · 선택 드래그) — 포인터가 창을 벗어나도 hover 정리 사건을 보내지 않는다.
    pub(crate) fn is_pressed(&self) -> bool {
        self.pressed.is_some()
    }

    pub(crate) fn take_col_changed(&mut self) -> bool {
        std::mem::take(&mut self.col_changed)
    }

    // ── 네비게이션(활성 탭 — 탭별 독립) ──────────────

    /// 새 소스를 활성 탭에(스크롤·캐럿 리셋 · 열·정렬 유지).
    fn apply_source(&mut self, src: TreeSource, inv: &mut Invalidations) {
        self.tabs[self.active].rows.replace_source(src, inv);
        self.session_dirty = true;
        self.navigated = true;
        self.sync_columns_for_root(inv);
        self.sync_chrome(inv);
    }

    /// 새 경로 진입(히스토리 push — 앞으로 절단). 열기 실패 시 현 위치 유지(경로 바는 복귀) · 실패 사유를 돌려준다.
    pub(crate) fn navigate_to(&mut self, path: PathBuf, inv: &mut Invalidations) -> Option<String> {
        let src = TreeSource::open(&path, self.tab_opts());
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
        let src = TreeSource::open(&p, self.tab_opts());
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
        let src = TreeSource::open(&p, self.tab_opts());
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

    /// 여러 경로의 행을 선택(첫 경로 = 캐럿 + 단일 선택 · 나머지 = 선택에 더함) — 목록에 없는 경로는 건너뛴다.
    /// 삭제 실패분 강조용(dir2 win.rs `select_paths`).
    pub(crate) fn select_paths(&mut self, paths: &[PathBuf], inv: &mut Invalidations) {
        let align = self.nav_up_align;
        let rows = &mut self.tabs[self.active].rows;
        let n = rows.source().len();
        let mut first = true;
        for i in 0..n {
            let hit = rows
                .source()
                .row_path(i)
                .is_some_and(|p| paths.contains(&p));
            if hit {
                let op = if first {
                    SelectOp::Single
                } else {
                    SelectOp::Toggle
                };
                rows.select_program_aligned(i, op, align, inv);
                first = false;
            }
        }
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

    /// 설정(`list.*`) 반영(사용자 10-03 규칙):
    /// - **전역** = 보호된 운영 체제 항목 표시 → 모든 탭에 바로 적용(무간섭 재열람 — 캐럿·스크롤 유지).
    /// - **새 탭의 기본값** = 숨김 · Dot · 폴더 우선 · 대소문자 구분 정렬 → 이 패널의 기본값만 바뀐다. 이미 열린 탭은 자기
    ///   값을 지킨다(보기 범위대로 관리).
    pub(crate) fn set_opts(&mut self, opts: ListOpts, inv: &mut Invalidations) {
        self.opts = opts;
        for tab in &mut self.tabs {
            let view = tab.rows.source().opts().view();
            tab.rows.source_mut().set_opts(opts.with_view(view));
        }
        self.reopen(inv);
    }

    /// 활성 탭의 열람 옵션(새 탭 · 이동이 이 값을 계승한다 — 값의 원천은 탭).
    pub(crate) fn tab_opts(&self) -> ListOpts {
        self.rows().source().opts()
    }

    /// 활성 탭의 `(숨김, Dot, 폴더 우선)` — 툴바 · 메뉴 체크가 따라간다(dir2 `active_view_values`).
    #[cfg(test)]
    pub(crate) fn active_view_values(&self) -> (bool, bool, bool) {
        let (hidden, dot, folders, _) = self.tab_opts().view();
        (hidden, dot, folders)
    }

    /// 활성 탭의 보기 옵션 4종(숨김 · Dot · 폴더 우선 · 대소문자 구분 정렬).
    pub(crate) fn active_view(&self) -> crate::filelist::ViewOpts {
        self.tab_opts().view()
    }

    /// 탭 보기 옵션 기입(dir2 08-02 · `list.view_scope`가 전파 폭을 정한다): `all_tabs` = 이 패널 전 탭 · 아니면 활성 탭만.
    /// 새 탭의 기본값(설정)은 건드리지 않는다.
    /// 바뀐 탭은 무간섭 재열람(캐럿·스크롤 유지).
    pub(crate) fn set_view(
        &mut self,
        all_tabs: bool,
        view: crate::filelist::ViewOpts,
        inv: &mut Invalidations,
    ) {
        self.session_dirty = true;
        let active = self.active;
        for (i, tab) in self.tabs.iter_mut().enumerate() {
            if !all_tabs && i != active {
                continue;
            }
            let opts = tab.rows.source().opts().with_view(view);
            if tab.rows.source().opts() == opts {
                continue;
            }
            let (caret, sr, sx) = (tab.rows.caret(), tab.rows.scroll_row(), tab.rows.scroll_x());
            tab.rows.source_mut().set_opts(opts);
            tab.rows.restore_view(caret, sr, sx, inv);
        }
        self.sync_chrome(inv);
        inv.push(self.bounds);
    }

    /// 네비 버튼 항목(시험 · 덤프용).
    #[cfg(test)]
    pub(crate) fn nav_items(&self) -> &[ToolItem] {
        self.navbtns.items()
    }

    /// 지금 끌고 있는 탭(임계를 넘은 드래그만) — 호스트의 패널 간 이동 판정용(dir2 WINC-098/101).
    pub(crate) fn tab_dragging(&self) -> Option<usize> {
        self.tabbar.dragging()
    }

    /// 탭 드래그를 접는다(호스트가 다른 패널로 옮겼거나 Esc) — 눌림 캡처도 푼다.
    pub(crate) fn cancel_tab_drag(&mut self, inv: &mut Invalidations) {
        self.tabbar.cancel_drag();
        if self.pressed == Some(Part::Tabs) {
            self.pressed = None;
        }
        inv.push(self.tabbar.bounds());
    }

    /// 이 패널에 탭을 놓을 때의 자리: 탭 바의 탭 위 = 그 탭 앞 · 그 밖(탭 바 빈 곳 · 패널 본문) = 맨 끝(`None`).
    /// 함께 돌려주는 rect = 놓일 자리 표식(탭 왼쪽 가장자리 세로선 · 끝이면 마지막 탭 오른쪽).
    pub(crate) fn tab_drop_target(&self, x: i32, y: i32) -> (Option<usize>, Rect) {
        let tb = self.tabbar.bounds();
        let line = |px: i32, r: Rect| Rect::new(px - 1, r.y, 2, r.h);
        if let Some(i) = self.tabbar.tab_index_at(x, y) {
            if let Some(r) = self.tabbar.tab_rect(i) {
                return (Some(i), line(r.x.max(tb.x + 1), r));
            }
        }
        let last = self.tabs.len().saturating_sub(1);
        match self.tabbar.tab_rect(last) {
            Some(r) => (None, line(r.right().min(tb.right() - 1), r)),
            None => (None, Rect::new(tb.x, tb.y, 2, tb.h)),
        }
    }

    /// 탭 `i`의 자리(마지막 그리기 기준 · 시험용).
    #[cfg(test)]
    pub(crate) fn tab_rect(&self, i: usize) -> Option<Rect> {
        self.tabbar.tab_rect(i)
    }

    /// 탭 바 영역(표식 그리기용).
    pub(crate) fn tabbar_bounds(&self) -> Rect {
        self.tabbar.bounds()
    }

    /// 탭 바 모양(설정 `tabs.multiline` · `tabs.scroll_buttons`): 여러 줄(폭을 넘으면 다음 줄) 또는 한 줄 + ◀ ▶(자리 3가지).
    pub(crate) fn set_tab_style(
        &mut self,
        multiline: bool,
        buttons: nexa_ctl::controls::ScrollButtons,
        inv: &mut Invalidations,
    ) {
        self.tabbar.set_multiline(multiline);
        self.tabbar.set_scroll_buttons(buttons);
        inv.push(self.bounds);
    }

    /// 탭 본체 더블클릭 동작(설정 `tabs.dblclick` 값 — close/pin/lock · 모르는 값 = 닫기).
    pub(crate) fn set_tab_dblclick(&mut self, value: &str) {
        self.tab_dbl = TabDbl::parse(value);
    }

    /// 탭 바 줄 수가 바뀌었는가(그리기가 측정 · 1회성) — 참이면 호스트가 다시 배치한다(dir2 win.rs:4941).
    pub(crate) fn take_tab_lines_changed(&self) -> bool {
        self.tabbar.take_lines_changed()
    }

    /// 탭 바 줄 수(시험 · 덤프).
    #[cfg(test)]
    pub(crate) fn tab_lines(&self) -> usize {
        self.tabbar.lines()
    }

    /// 활성 탭의 폴더가 바뀌었는가(1회성).
    pub(crate) fn take_navigated(&mut self) -> bool {
        std::mem::take(&mut self.navigated)
    }

    /// 그 폴더를 보고 있는 **모든 탭**에 보기 옵션 기입(보기 범위 "폴더" — 같은 폴더는 공통) · 바뀐 탭은 무간섭 재열람.
    pub(crate) fn set_view_for_dir(
        &mut self,
        dir: &Path,
        view: crate::filelist::ViewOpts,
        inv: &mut Invalidations,
    ) {
        let mut changed = false;
        for tab in &mut self.tabs {
            if tab.rows.source().path() != dir {
                continue;
            }
            let opts = tab.rows.source().opts().with_view(view);
            if tab.rows.source().opts() == opts {
                continue;
            }
            let (caret, sr, sx) = (tab.rows.caret(), tab.rows.scroll_row(), tab.rows.scroll_x());
            tab.rows.source_mut().set_opts(opts);
            tab.rows.restore_view(caret, sr, sx, inv);
            changed = true;
        }
        if changed {
            self.session_dirty = true;
            self.sync_chrome(inv);
            inv.push(self.bounds);
        }
    }

    /// 세션 저장 — 탭별 보기 옵션 플래그(dir2 `panel{i}.views`).
    pub(crate) fn session_view_flags(&self) -> Vec<u8> {
        self.tabs
            .iter()
            .map(|t| t.rows.source().opts().view_flags())
            .collect()
    }

    /// 다시 읽기(각 탭 **자기 보기 옵션 그대로** · 캐럿·스크롤 유지).
    pub(crate) fn reopen(&mut self, inv: &mut Invalidations) {
        for tab in &mut self.tabs {
            let (caret, sr, sx) = (tab.rows.caret(), tab.rows.scroll_row(), tab.rows.scroll_x());
            tab.rows.source_mut().reload();
            tab.rows.restore_view(caret, sr, sx, inv);
            tab.stale = false;
        }
        self.sync_chrome(inv);
    }

    /// 활성 탭이 낡았는가(전환·닫기로 드러난 뒤 아직 다시 읽지 않음 · dir2 `active_tab_stale`).
    #[cfg(test)]
    pub(crate) fn active_tab_stale(&self) -> bool {
        self.tabs[self.active].stale
    }

    /// 낡은 활성 탭만 다시 읽는다(캐럿·스크롤 유지 · dir2 win.rs:4967-4976 "전환 수렴") — 읽었으면 `true`.
    /// 인라인 이름 편집 중이면 미룬다(편집 행이 바뀌면 안 된다 — 다음 길목에서 다시 본다).
    pub(crate) fn refresh_stale(&mut self, inv: &mut Invalidations) -> bool {
        if !self.tabs[self.active].stale || self.rows().is_renaming() {
            return false;
        }
        let tab = &mut self.tabs[self.active];
        let (caret, sr, sx) = (tab.rows.caret(), tab.rows.scroll_row(), tab.rows.scroll_x());
        tab.rows.source_mut().reload();
        tab.rows.restore_view(caret, sr, sx, inv);
        tab.stale = false;
        self.sync_chrome(inv);
        true
    }

    /// 잘라내기 표식(SHELL-044) 전 탭 적용 — 바뀐 탭의 목록만 무효화.
    pub(crate) fn set_cut_marks(
        &mut self,
        marks: &std::collections::HashSet<PathBuf>,
        inv: &mut Invalidations,
    ) {
        for tab in &mut self.tabs {
            if tab.rows.source_mut().set_cut_marks(marks.clone()) {
                inv.push(tab.rows.bounds());
            }
        }
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
        } else if self.status_on && self.status.bounds().contains(p) {
            Some(Part::Status)
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
            Part::Status => {
                use nexa_ctl::Widget as _;
                self.status.on_event(ev, inv);
                if let Some(c) = self.status.take_click() {
                    self.pending_status = Some(c);
                }
            }
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
                    if self.status_on {
                        use nexa_ctl::Widget as _;
                        self.status.on_event(ev, inv);
                    }
                }
            }
            InputEvent::MouseDown { x, y, .. } => {
                // 경로 제안 팝업(목록 위에 뜬다) 클릭 = 그 폴더로 이동(dir2 win.rs:8000-8007) — 아래 목록으로 흘리지 않는다.
                if self.pathbar.suggest_click(x, y, inv) {
                    return;
                }
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
                // 경로 편집 필드 더블클릭 = 전체 선택(dir2 win.rs:8638-8647).
                if self.path_edit_at(x, y) {
                    self.pathbar.edit_key(EditKey::SelectAll, false, inv);
                    return;
                }
                // 탭 본체 더블클릭 = 설정 동작(dir2 win.rs:8648-8657 · 기본 닫기) · 탭 바 빈 곳 더블클릭 = 새 탭(win.rs:8658-8665).
                if let Some(ti) = self.tabbar.tab_index_at(x, y) {
                    match self.tab_dbl {
                        TabDbl::Pin => self.toggle_tab_pin(ti, inv),
                        TabDbl::Lock => self.toggle_tab_lock(ti, inv),
                        TabDbl::Close => self.close_tab(ti, inv),
                    }
                    return;
                }
                if self.tabbar.empty_area_at(x, y) {
                    self.new_tab(inv);
                    return;
                }
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
                // 경로 편집 중 필드 우클릭 = 편집 메뉴(dir2 win.rs:7472-7532) — 처음 우클릭(편집 진입)과 구분.
                if matches!(ev, InputEvent::RightDown { .. }) && self.path_edit_at(x, y) {
                    self.pending_path_menu = true;
                    return;
                }
                // 이름 바꾸기 편집 필드 우클릭 = 글자 편집 메뉴(dir2 win.rs:7478 `EditMenuTarget::Rename`) — 종전에는 행 메뉴가 떴다.
                if matches!(ev, InputEvent::RightDown { .. })
                    && self.rows().is_renaming()
                    && self.rows().rename_hit(x, y)
                {
                    self.pending_rename_menu = true;
                    return;
                }
                // 열 머리글 우클릭 = 열 배치 메뉴(dir2 win.rs:6262-6304) — 종전에는 빈 곳 메뉴가 떴다.
                if matches!(ev, InputEvent::RightDown { .. }) && self.rows().header_area(x, y) {
                    self.pending_header_menu = true;
                    return;
                }
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
                InputEvent::Char { c, .. } => {
                    self.pathbar.edit_char(c, inv);
                    self.update_path_suggest(inv);
                }
                InputEvent::Key { key, shift, .. } => match key {
                    CtlKey::Enter => self.pathbar.submit_edit(inv),
                    // Esc: 제안 팝업이 떠 있으면 팝업만 닫는다(편집은 계속) · 없으면 편집 취소.
                    CtlKey::Escape if self.pathbar.suggest_open() => {
                        self.pathbar.close_suggest(inv)
                    }
                    CtlKey::Escape => self.pathbar.cancel_edit(inv),
                    CtlKey::Left => self.pathbar.edit_key(EditKey::Left, shift, inv),
                    CtlKey::Right => self.pathbar.edit_key(EditKey::Right, shift, inv),
                    CtlKey::Home => self.pathbar.edit_key(EditKey::Home, shift, inv),
                    CtlKey::End => self.pathbar.edit_key(EditKey::End, shift, inv),
                    CtlKey::Delete => {
                        self.pathbar.edit_key(EditKey::DeleteForward, shift, inv);
                        self.update_path_suggest(inv);
                    }
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

    /// 경로 제안 갱신(dir2 `update_path_suggest` win.rs:7199-7206 · GAP-013): 편집 글이 바뀔 때마다 — 마지막 구분자까지를 베이스로
    /// 하위 폴더를 열거해 접두사 일치(대소문자 무시)를 최대 [`PATH_SUGGEST_MAX`]개. 빈 목록 = 팝업 닫기.
    pub(crate) fn update_path_suggest(&mut self, inv: &mut Invalidations) {
        let Some(text) = self.pathbar.edit_text() else {
            return;
        };
        // 제안도 제출과 같은 확장을 거친다(변수 · `~` · 상대 경로 — 셸은 실행하지 않으므로 글자마다 불러도 안전하다).
        let expanded = self.expand_path_input(&text);
        let items = crate::pathinput::suggest_folders(
            &expanded,
            crate::pathinput::fs_dirs,
            PATH_SUGGEST_MAX,
        );
        self.pathbar.set_suggestions(items, inv);
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
            // 입력 해석(dir2 PathInterpreter): 감싼 따옴표 제거 · `%VAR%` · `$env:VAR` 확장 — 미정의 변수는 원문 그대로(열기 실패로 드러난다).
            let path = self.expand_path_input(&path);
            if crate::pathinput::is_shell_scheme(&path) {
                // `shell:` 별칭 = OS가 해석한다(패널은 OS를 모른다 — 호스트가 포트로 풀어 이동).
                self.pending_alias = Some(path);
            } else {
                let _ = self.navigate_to(PathBuf::from(path), inv);
            }
        }
        if self.tabs[self.active].rows.take_col_resized() {
            self.user_cols = true;
            self.col_changed = true;
        }
        // 컬럼을 끌어 순서를 바꿨다(GAP-016 — 종전에는 활성 탭에만 남았다): 호스트가 같은 패널의 모든 탭 · (동기화면)
        // 반대 패널 · 세션에 반영한다. 내 PC(드라이브 열)는 열 구성이 달라 전파하지 않는다.
        if self.tabs[self.active].rows.take_col_reordered()
            && !self.rows().source().is_virtual_root()
        {
            self.user_cols = true;
            self.session_dirty = true;
            self.col_order_changed = true;
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
            show_protected: true,
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
        // 자동완성(GAP-013): 구분자까지 치면 하위 폴더 제안 · 접두사로 좁혀짐 · Esc = 팝업만 닫기 · 제안 클릭 = 이동.
        p.pathbar.begin_edit(&mut inv);
        p.key_event(&InputEvent::SelectAll, &mut inv);
        let typed = format!("{}{}s", dir.to_string_lossy(), std::path::MAIN_SEPARATOR);
        for c in typed.chars() {
            p.key_event(&InputEvent::Char { c, now_ms: 0 }, &mut inv);
        }
        assert!(p.pathbar.suggest_open(), "하위 폴더 sub 제안");
        let esc = InputEvent::Key {
            key: CtlKey::Escape,
            shift: false,
            primary: false,
        };
        p.key_event(&esc, &mut inv);
        assert!(
            !p.pathbar.suggest_open() && p.pathbar.is_editing(),
            "Esc = 팝업만"
        );
        p.key_event(&InputEvent::Char { c: 'u', now_ms: 0 }, &mut inv);
        let r = p.pathbar.suggest_rect().expect("다시 제안");
        p.on_event(
            &InputEvent::MouseDown {
                x: r.x + 8,
                y: r.y + 4,
                shift: false,
                primary: false,
            },
            &mut inv,
        );
        p.drain_actions(&mut inv);
        assert_eq!(p.root_path(), dir.join("sub"), "제안 클릭 = 이동");
        assert!(!p.pathbar.is_editing());
        p.navigate_to(dir.clone(), &mut inv);
        p.pathbar.begin_edit(&mut inv);
        p.key_event(&InputEvent::SelectAll, &mut inv);
        for c in "zz-no-such".chars() {
            p.key_event(&InputEvent::Char { c, now_ms: 0 }, &mut inv);
        }
        assert!(!p.pathbar.suggest_open(), "구분자 없는 입력 = 제안 없음");
        p.key_event(&esc, &mut inv);
        assert!(!p.pathbar.is_editing(), "팝업 없으면 Esc = 편집 취소");
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
        q.attach_tab(tab, None, true, &mut inv);
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
        p.set_default_columns(cols(), cols(), &mut inv);
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
