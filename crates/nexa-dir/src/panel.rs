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

/// 워커 결과를 문턱(ms) 안에서만 기다린다 — **문턱 0 = 기다리지 않는다**(설명문 "0 = 늘 옛 목록 먼저" 그대로 · 종전엔
/// `recv_timeout(0)`이라 빠른 러너(macOS CI 10-24 · `async_navigation_applies_on_tick` 397/1)에서 워커가 그 순간 이미 끝나 있으면
/// 즉석 적용돼 `pending`이 안 생겼다 — 결과는 다음 틱이 끼운다).
fn recv_within<T>(
    rx: &std::sync::mpsc::Receiver<T>,
    wait_ms: u64,
) -> Result<T, std::sync::mpsc::RecvTimeoutError> {
    if wait_ms == 0 {
        return Err(std::sync::mpsc::RecvTimeoutError::Timeout);
    }
    rx.recv_timeout(std::time::Duration::from_millis(wait_ms))
}

/// 내 PC(드라이브 열) 열 key — 세션 `drivew` 순서(이름 · 종류 · 전체 크기 · 여유 공간 · T-193).
pub(crate) const DRIVE_KEYS: [u32; 4] = [
    crate::filelist::COL_NAME,
    crate::filelist::COL_KIND,
    crate::filelist::COL_TOTAL,
    crate::filelist::COL_FREE,
];
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
    /// 워커에서 읽는 중인 진입(10-18 결정 ② · `list.async_read`) — 결과는 [`Panel::nav_poll`](틱)이 적용한다.
    pub pending: Option<PendingNav>,
    /// 워커에서 다시 읽는 중(② 2단계 · 재열람 = F5 · 작업 뒤 · 감시 · 전환 수렴) — 결과는 [`Panel::nav_poll`]이 끼운다.
    pub reloading: Option<PendingReload>,
}

/// 워커 재열람 1건: 결과 채널 · 만들 때의 경로·옵션(그 사이 진입·옵션이 바뀌었으면 버린다) · 감시가 시킨 것인가(신선 스탬프 지움) ·
/// 진행 중에 또 요청이 오면 `again`(끝난 뒤 한 번 더 — 통지 폭주에도 워커는 하나).
pub(crate) struct PendingReload {
    rx: std::sync::mpsc::Receiver<crate::filelist::ReloadBuilt>,
    path: PathBuf,
    opts: ListOpts,
    from_watch: bool,
    again: bool,
}

/// 워커 열기 1건: 결과 채널 · 대상 · 적용 뒤 선택할 경로(위로/홈 = 떠난 폴더) · 히스토리 동작(적용 때 움직인다).
pub(crate) struct PendingNav {
    rx: std::sync::mpsc::Receiver<TreeSource>,
    path: PathBuf,
    select: Option<PathBuf>,
    kind: NavKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NavKind {
    Push,
    Back,
    Forward,
}

/// 정렬 키 목록 → `name+,size-`(T-128 세션 표기 · 빈 목록 = `""`).
pub(crate) fn sort_to_str(keys: &[(u32, bool)]) -> String {
    keys.iter()
        .map(|(k, desc)| {
            format!(
                "{}{}",
                crate::order::col_id_key(*k),
                if *desc { '-' } else { '+' }
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// `name+,size-` → 정렬 키 목록(모르는 이름 · 방향 없는 항목은 버린다 · 손상 = 빈 목록).
pub(crate) fn sort_from_str(spec: &str) -> Vec<(u32, bool)> {
    spec.split(',')
        .filter_map(|item| {
            let item = item.trim();
            let (name, desc) = match item.chars().last()? {
                '+' => (&item[..item.len() - 1], false),
                '-' => (&item[..item.len() - 1], true),
                _ => return None,
            };
            crate::order::col_key_id(name).map(|id| (id, desc))
        })
        .collect()
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
    /// 빠른 필터 바(경로 바 아래 · 켜져 있을 때만 · NEW-057).
    Filter,
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
    /// 내 PC 드라이브 용량 워커 결과 채널(10-18 · `fill_drive_space` → 틱 수거).
    drive_rx: Option<std::sync::mpsc::Receiver<crate::platform::DriveSpaces>>,
    opts: ListOpts,
    nav_up_align: ScrollAlign,
    /// 비동기 폴더 읽기(10-18 결정 ② · `list.async_read` · 문턱 `list.async_wait_ms`) — 창이 있을 때만(`probe::is_async`).
    async_read: bool,
    async_wait_ms: u64,
    #[cfg(test)]
    async_force: bool,
    pressed: Option<Part>,
    /// 사용자가 열 폭을 바꿨다 — 배치가 기본 열을 다시 넣지 않는다(dir2 "사용자 폭 리셋 방지").
    user_cols: bool,
    /// 호스트가 준 기본 열(내 PC 드라이브 열 ↔ 일반 열 전환의 복귀 원본 · dir2 X-17).
    base_columns: Vec<Column>,
    /// 내 PC(드라이브 열)에서 사용자가 바꾼 열 폭(key별 · 없는 key는 기본 열 폭 상속) — 실제 폴더 폭과 **따로** 산다(사용자 10-24:
    /// 가상 폴더와 실제 폴더는 유형이 달라 서로 동기하지 않는다 · T-193) · 세션 `drivew`.
    drive_widths: Vec<(u32, i32)>,
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
    /// 종류 열 OS 이름(NEW-059 · 설정 `list.os_icons` · 10-31) — 탭 원천마다 전파.
    os_kind: bool,
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
    /// 빠른 필터 바(NEW-057 · 10-25): 경로 바 아래 한 줄 입력 상자 · 글 = 활성 탭 원천의 필터(탭별) · 꺼짐 = 높이 0.
    /// 10-26: nexa-ui 235 `SearchBox` = 텍스트박스 + 틀 안쪽 토글 Aa·ab·.* 한 컨트롤(nexa-sql filterbar 꼴 · 혜성 = 틀 둘레 · 오류 = 틀 danger).
    filter: nexa_ctl::SearchBox,
    filter_on: bool,
    /// 토글이 바뀌었다(호스트가 설정 `list.filter_*`에 적는다).
    filter_opts_changed: bool,
    /// 검색창 편집 메뉴(TextBox 팝업)에서 고른 행동(복사 · 잘라내기 · 붙여넣기) — 호스트가 [`Panel::take_filter_clip`]로 거둬
    /// [`Panel::filter_clip`]으로 잇는다(클립보드 접근은 호스트 몫 · 사용자 10-26).
    filter_clip: Option<nexa_ctl::EditCtxAction>,
    /// 새 탭·새 원천이 이어받는 토글(설정 기본값 · App이 `set_filter_opts`로 준다 · 종전 프로세스 전역 `filelist::FILTER_OPTS_DEFAULT`는 10-26 제거 · docs/15 §2-8).
    filter_defaults: ndir_ops::namefilter::FilterOpts,
    /// 새 그리드의 Shift 없는 헤더 클릭 = 정렬 추가(설정 `list.sort_click_adds` · App이 `set_sort_click_adds`로 준다 · 종전 전역 제거).
    sort_click_adds: bool,
}

/// 탭 상태바 높이(논리 px · 창 상태줄과 같다).
const TAB_STATUS_H: f32 = 22.0;
/// 탭 상태바 칸 id.
pub(crate) const SEG_FOLDER: &str = "folder";
pub(crate) const SEG_GIT: &str = "git";
/// 분해형(NFD) 이름 집계 칸 — 클릭 = 그 행만 보기 토글(10-31 사용자 결정 ②).
pub(crate) const SEG_NFD: &str = "nfd";

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
    /// 찾기 범위(설정 `typeahead.scope` · GAP-015 · DR-31 ⑤ · 10-25): 가시 스트림(기본) · 같은 수준 · 전체 처음부터.
    pub scope: ndir_tree::FindScope,
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
            scope: ndir_tree::FindScope::VisibleStream,
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
        rows.set_sort_mark_vector(true, &mut inv); // ▲/▼ = 삼각형 도형(T-128 · DR-31 ⑥ · nexa-ui 231)
        rows.set_col_drag_marker(true); // 컬럼을 끄는 동안 놓일 자리 표식(T-123)
        let mut tabbar = TabBar::new();
        tabbar.set_show_new(true);
        // 넷째 토글 = 하위 폴더 검색(사용자 10-26 · nexa-sql "경로까지 검색" 자리 · nexa-ui 237 `with_scope_toggle`).
        let mut filter =
            nexa_ctl::SearchBox::new(ndir_i18n::tr("filter.placeholder")).with_scope_toggle();
        filter.set_tips(
            ndir_i18n::tr("filter.tip.case"),
            ndir_i18n::tr("filter.tip.word"),
            ndir_i18n::tr("filter.tip.regex"),
        );
        filter.set_tip(
            nexa_ctl::SearchToggle::Scope,
            ndir_i18n::tr("filter.tip.subdirs"),
        );
        nexa_ctl::Control::set_scale(&mut filter, m.scale);
        let mut p = Panel {
            tabbar,
            navbtns: nav_buttons(),
            pathbar: PathBar::new(display_path(path), m.bar_h, m.pad_x),
            filter,
            filter_on: false,
            tabs: vec![Tab {
                rows,
                nav: History::new(path.to_path_buf()),
                locked: false,
                pinned: false,
                stale: false,
                pending: None,
                reloading: None,
            }],
            active: 0,
            bounds: Rect::default(),
            m,
            focused: false,
            drive_rx: None,
            opts,
            nav_up_align: ScrollAlign::Center,
            async_read: true,
            async_wait_ms: 150,
            #[cfg(test)]
            async_force: false,
            pressed: None,
            user_cols: false,
            base_columns: Vec::new(),
            drive_widths: Vec::new(),
            pending_open: None,
            pending_tab_menu: None,
            tab_dbl: TabDbl::Close,
            pending_rename: None,
            pending_ctx: None,
            pending_header_menu: false,
            ta_opts: TaOpts::default(),
            no_row_icons: false,
            os_kind: false,
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
            filter_opts_changed: false,
            filter_clip: None,
            filter_defaults: ndir_ops::namefilter::FilterOpts::default(),
            sort_click_adds: false,
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
        // 탭별 보기 옵션(dir2 `views` — 없는 자리는 설정 기본 그대로) — 열 때부터 적용해 탭당 열거 1회(10-18).
        let tab_opts = |orig: usize| {
            ps.views
                .get(orig)
                .map_or(opts, |&f| opts.with_view(ListOpts::view_of_flags(f)))
        };
        let (orig0, first) = &valid[0];
        let mut panel = Panel::new(first, tab_opts(*orig0), m, columns);
        panel.opts = opts; // 패널 기본(새 탭용)은 설정값
        panel.pool_columns = pool; // 저장된 레이아웃이 기본 숨김 열(확장자 · 종류)을 켜 두었을 때 그 열의 원형
        let mut inv = Invalidations::default();
        for (orig, p) in valid.iter().skip(1) {
            // 복원한 탭의 히스토리는 새로(dir2 — 히스토리는 영속하지 않는다) = `push_tab`의 `History::new`.
            panel.push_tab(p.clone(), tab_opts(*orig), &mut inv);
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
            // 탭별 정렬(T-128 · 없는 자리는 기본 그대로).
            if let Some(spec) = ps.sort.get(*orig) {
                let keys = sort_from_str(spec);
                panel.tabs[slot].rows.set_sort(&keys, &mut inv);
            }
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
            if !panel.apply_col_widths_by_key(&by_key, false, &mut inv) {
                panel.user_cols = true; // 폭이 기본과 같아도 저장된 폭이 있었다 = 사용자 폭
            }
        }
        // 내 PC 열 폭(`drivew` · T-193) — 드라이브 열 순서(이름 · 종류 · 전체 · 여유)로 적혀 있다 · 탭이 아직 없어도 기억만 남긴다.
        if ps.drive_widths.len() == DRIVE_KEYS.len() {
            let by_key: Vec<(u32, i32)> = DRIVE_KEYS
                .iter()
                .copied()
                .zip(ps.drive_widths.iter().copied())
                .collect();
            panel.apply_col_widths_by_key(&by_key, true, &mut inv);
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
        // 활성 탭이 내 PC(드라이브 열)면 실제 폴더 폭은 기본 열에서(드라이브 폭은 `drivew`로 따로 — 종전엔 드라이브 폭 4개가
        // `colw`에 실려 복원 때 실제 폴더 열에 잘못 들어갔다 · 사용자 10-24 "좌우 폭이 다르게 보임"의 원인 하나 · T-193).
        if self.active_is_virtual_root() && !self.base_columns.is_empty() {
            return self.base_columns.iter().map(|c| c.width).collect();
        }
        self.rows().columns().iter().map(|c| c.width).collect()
    }

    /// 내 PC 열 폭(세션 `drivew` · 이름 · 종류 · 전체 크기 · 여유 공간 순) — 사용자가 내 PC에서 폭을 바꾼 뒤에만(T-193).
    pub(crate) fn drive_widths_session(&self) -> Vec<i32> {
        if self.drive_widths.is_empty() {
            return Vec::new();
        }
        self.drive_columns().iter().map(|c| c.width).collect()
    }

    /// 활성 탭이 가상 최상위(내 PC)인가 — 열 폭 동기의 "폴더 유형"(T-193).
    pub(crate) fn active_is_virtual_root(&self) -> bool {
        self.rows().source().is_virtual_root()
    }

    /// 실제 폴더 열 폭(key별) = 이 패널의 실제 폴더 기억(기본 열) — 활성 탭이 내 PC여도 실제 폴더 폭을 준다(T-197 저장 1벌 · 동기).
    /// 기본 열이 아직 없으면(복원 직후 · 배치 전) 활성 탭이 실제 폴더일 때 그 열 · 아니면 빈 목록.
    pub(crate) fn real_col_widths_by_key(&self) -> Vec<(u32, i32)> {
        let cols: &[Column] = if !self.base_columns.is_empty() {
            &self.base_columns
        } else if !self.active_is_virtual_root() {
            self.rows().columns()
        } else {
            return Vec::new();
        };
        cols.iter().map(|c| (c.key, c.width)).collect()
    }

    /// 내 PC 열 폭 기억(key별 · 비어 있으면 아직 안 바꿈 · T-197).
    pub(crate) fn drive_widths_by_key(&self) -> Vec<(u32, i32)> {
        self.drive_widths.clone()
    }

    /// key별 실제 폴더 폭을 **이 패널의 열 순서**(세션 `colw` 꼴 · 표시 순)로 — 없는 key는 지금 폭(T-197 ① 저장 1벌).
    pub(crate) fn col_widths_in_layout(&self, widths: &[(u32, i32)]) -> Vec<i32> {
        let cols: &[Column] = if !self.base_columns.is_empty() {
            &self.base_columns
        } else {
            self.rows().columns()
        };
        cols.iter()
            .map(|c| {
                widths
                    .iter()
                    .find(|(k, _)| *k == c.key)
                    .map_or(c.width, |(_, w)| *w)
            })
            .collect()
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

    /// 선택 수와 첫 경로(T-189 ② · 도크 갱신은 이것만 쓴다).
    pub(crate) fn selection_head(&self) -> (usize, Option<PathBuf>) {
        self.rows().source().selection_head()
    }

    /// 선택 반전(`edit.select_invert`) — 활성 탭의 보이는 행.
    pub(crate) fn invert_selection(&mut self, inv: &mut Invalidations) {
        let rows = self.rows_mut();
        if rows.source_mut().invert_selection() {
            inv.push(rows.bounds());
        }
    }

    pub(crate) fn selected_paths_in_view_order(&self) -> Vec<PathBuf> {
        self.rows().source().selected_paths_in_view_order()
    }

    /// 패턴 선택/해제(`edit.select_pattern` · NEW-054) — 활성 탭의 보이는 행 · 바뀐 행 수.
    pub(crate) fn select_pattern(
        &mut self,
        spec: &str,
        select: bool,
        inv: &mut Invalidations,
    ) -> usize {
        let rows = self.rows_mut();
        let n = rows.source_mut().select_pattern(spec, select);
        if n > 0 {
            inv.push(rows.bounds());
        }
        n
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
        // 저장소의 `.git`(HEAD · index · refs 변경 = 커밋 · 체크아웃 · stash) — git 요약 갱신용(10-06 · 목록 재열람 대상은 아니다).
        if let Some((_, Some((repo, _)))) = &self.git {
            if let Some(g) = crate::dirinfo::git_dir(repo) {
                if !dirs.contains(&g) {
                    dirs.push(g);
                }
            }
        }
        dirs
    }

    pub(crate) fn bounds(&self) -> Rect {
        self.bounds
    }

    /// 경로 바 사각형(플래시 앵커 · 10-18).
    pub(crate) fn pathbar_bounds(&self) -> Rect {
        self.pathbar.bounds()
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
        // 빠른 필터 바(NEW-057): 켜져 있으면 경로 바 아래 한 줄(같은 높이) · 목록이 그만큼 내려간다.
        let filter_h = if self.filter_on {
            bar_h.min((bounds.bottom() - bounds.y - tab_h - bar_h).max(0))
        } else {
            0
        };
        {
            use nexa_ctl::Widget as _;
            let fy = bounds.y + tab_h + bar_h;
            // 검색창 한 컨트롤(토글은 틀 안쪽 — nexa-ui 235 SearchBox가 배치 · 10-26).
            self.filter
                .set_bounds(Rect::new(bounds.x, fy, bounds.w, filter_h), inv);
        }
        let list_y = bounds.y + tab_h + bar_h + filter_h;
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
        self.filter.set_scale(m.scale);
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
        // 폭 = 내 PC에서 바꾼 폭(`drive_widths`) → 기본 열(이름 · 종류 상속) → 기본값 순.
        let w_of = |key: u32, fallback: f32| {
            self.drive_widths
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, w)| *w)
                .or_else(|| {
                    self.base_columns
                        .iter()
                        .find(|c| c.key == key)
                        .map(|c| c.width)
                })
                .unwrap_or((fallback * self.m.scale).round() as i32)
        };
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
            // 전체/여유 공간은 정렬 키가 없다(PANEL-065 `TOTAL`/`FREE` 무시) → 머리 클릭도 받지 않는다(T-128 · 종전엔 ▲만 떴다).
            Column::new(
                crate::filelist::COL_TOTAL,
                ndir_i18n::tr("col.total"),
                w_of(crate::filelist::COL_TOTAL, 110.0),
            )
            .right_aligned()
            .not_sortable(),
            Column::new(
                crate::filelist::COL_FREE,
                ndir_i18n::tr("col.free"),
                w_of(crate::filelist::COL_FREE, 130.0),
            )
            .right_aligned()
            .not_sortable(),
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

    /// 내 PC 드라이브 용량 채우기 — Disk 포트의 워커 경로에 한 번만 묻고(`drive_space_asked`) 결과는 [`Self::tick`]이 수거한다
    /// (10-18: 종전엔 UI 스레드에서 드라이브마다 동기 조회 · 결과가 비면 사건마다 재시도).
    pub(crate) fn fill_drive_space(
        &mut self,
        disk: &dyn crate::platform::Disk,
        inv: &mut Invalidations,
    ) {
        let src = self.rows().source();
        if !src.is_virtual_root() || src.drive_space_known() || src.drive_space_asked() {
            return;
        }
        let names = src.drive_names();
        self.tabs[self.active]
            .rows
            .source_mut()
            .mark_drive_space_asked();
        self.drive_rx = Some(disk.space_async(names));
        self.poll_drive_space(inv); // 동기 포트(가짜 · 미지원)는 이미 보냈다 → 바로 채운다
    }

    /// 워커 결과 수거(틱) — 채워졌으면 목록 무효화.
    fn poll_drive_space(&mut self, inv: &mut Invalidations) {
        let Some(rx) = &self.drive_rx else {
            return;
        };
        let Ok(space) = rx.try_recv() else {
            return;
        };
        self.drive_rx = None;
        if !space.is_empty() {
            self.tabs[self.active]
                .rows
                .source_mut()
                .set_drive_space(space);
            inv.push(self.rows().bounds());
        }
    }

    /// 용량 워커 결과를 기다리는가(이벤트 루프 깨움).
    pub(crate) fn drive_space_pending(&self) -> bool {
        self.drive_rx.is_some()
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
        if self.filter_on {
            use nexa_ctl::Widget as _;
            ctx.fill_rect(self.filter_bar_rect(), theme.chrome_bg);
            // 정규식 오류 = 틀 테두리 danger(`SearchBox::set_error` · 거를 때마다 `sync_filter_error`가 맞춘다).
            self.filter.paint(ctx, theme);
        }
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
        self.filter
            .set_placeholder(ndir_i18n::tr("filter.placeholder"));
        self.filter.set_tips(
            ndir_i18n::tr("filter.tip.case"),
            ndir_i18n::tr("filter.tip.word"),
            ndir_i18n::tr("filter.tip.regex"),
        );
        self.filter.set_tip(
            nexa_ctl::SearchToggle::Scope,
            ndir_i18n::tr("filter.tip.subdirs"),
        );
        self.sync_chrome(inv);
    }

    // ── 빠른 필터(NEW-057 · 10-25) ──────────────────────────────

    /// 필터 바가 보이는가(시험 · 덤프).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn filter_on(&self) -> bool {
        self.filter_on
    }

    /// 필터 입력 상자가 글자 키를 받는 중인가(호스트 = 경로 바 편집과 같은 규칙으로 타입어헤드 · 입력기 경로를 돌린다).
    pub(crate) fn filter_focused(&self) -> bool {
        use nexa_ctl::Control as _;
        self.filter_on && self.filter.is_focused()
    }

    /// 탭 상태바 폴더 칸 글(시험 · "n / m 항목(필터)").
    #[cfg(test)]
    pub(crate) fn status_folder_text(&self) -> String {
        self.status
            .segments()
            .iter()
            .find(|s| s.id == SEG_FOLDER)
            .map(|s| s.text.clone())
            .unwrap_or_default()
    }

    /// 활성 탭의 필터 글(없으면 빈 글).
    pub(crate) fn filter_text(&self) -> String {
        self.rows().source().name_filter().to_string()
    }

    /// `list.filter`: 꺼져 있으면 켜고 입력 상자에 포커스 · 켜져 있고 포커스가 상자에 있으면 끈다(필터 풀림) · 켜져 있는데 포커스가
    /// 목록이면 상자로 포커스만.
    pub(crate) fn toggle_filter(&mut self, inv: &mut Invalidations) {
        use nexa_ctl::Control as _;
        if !self.filter_on {
            self.filter_on = true;
            self.filter.set_focused(true);
            let b = self.bounds;
            self.set_bounds(b, inv);
            inv.push(self.bounds);
        } else if self.filter.is_focused() {
            self.close_filter(inv);
        } else {
            self.filter.set_focused(true);
            inv.push(self.filter_bounds());
        }
    }

    /// 필터 바 닫기 = 글 지움 · 필터 풀림 · 목록이 다시 커진다.
    pub(crate) fn close_filter(&mut self, inv: &mut Invalidations) {
        use nexa_ctl::Control as _;
        if !self.filter_on {
            return;
        }
        self.filter.set_text("");
        self.filter.set_focused(false);
        self.filter_on = false;
        self.apply_filter_text(inv);
        let b = self.bounds;
        self.set_bounds(b, inv);
        inv.push(self.bounds);
    }

    fn filter_bounds(&self) -> Rect {
        use nexa_ctl::Widget as _;
        self.filter.bounds()
    }

    /// 입력 상자의 글 → 활성 탭 원천의 필터(바뀌면 목록 무효화 · 캐럿/스크롤은 그리드가 길이에 맞춘다).
    /// 필터 바 전체(상자 + 토글) — 영역 판정 · 바탕.
    fn filter_bar_rect(&self) -> Rect {
        use nexa_ctl::Widget as _;
        self.filter.bounds()
    }

    /// 토글 셋(대소문자 · 단어 · 정규식 · 하위 폴더) — 버튼 색(켜짐 = 강조)으로 들고 있다.
    pub(crate) fn filter_opts(&self) -> ndir_ops::namefilter::FilterOpts {
        use nexa_ctl::SearchToggle as T;
        ndir_ops::namefilter::FilterOpts {
            case: self.filter.is_on(T::Case),
            word: self.filter.is_on(T::Word),
            regex: self.filter.is_on(T::Regex),
            subdirs: self.filter.is_on(T::Scope),
        }
    }

    /// 토글 셋을 밖에서(설정 · 시험) 넣는다 — 모든 탭 원천에 적용 · 목록 다시 걸러짐.
    pub(crate) fn set_filter_opts(
        &mut self,
        opts: ndir_ops::namefilter::FilterOpts,
        inv: &mut Invalidations,
    ) {
        use nexa_ctl::SearchToggle as T;
        self.filter_defaults = opts;
        self.filter.set_checked(T::Case, opts.case);
        self.filter.set_checked(T::Word, opts.word);
        self.filter.set_checked(T::Regex, opts.regex);
        self.filter.set_checked(T::Scope, opts.subdirs);
        for tab in &mut self.tabs {
            if tab.rows.source_mut().set_filter_opts(opts) {
                inv.push(tab.rows.bounds());
            }
        }
        self.sync_filter_error();
        inv.push(self.filter_bar_rect());
    }

    /// 토글이 바뀌었는가(호스트가 가져가 설정에 적는다 · 한 번만 참).
    pub(crate) fn take_filter_opts_changed(&mut self) -> bool {
        std::mem::take(&mut self.filter_opts_changed)
    }

    /// 검색창 편집 메뉴에서 고른 행동(한 번만) — 호스트가 [`Self::filter_clip`]으로 잇는다.
    pub(crate) fn take_filter_clip(&mut self) -> Option<nexa_ctl::EditCtxAction> {
        self.filter_clip.take()
    }

    /// 검색창 글의 클립보드 동작(Ctrl+C/X/V · 편집 메뉴 · 사용자 10-26 — 종전엔 Ctrl+C/X/V가 파일 복사/붙여넣기로 갔고 팝업 항목은
    /// 아무 일도 안 했다): 복사 = 선택 글 → 클립보드 · 잘라내기 · 붙여넣기(단일 행 상자라 개행은 공백으로) · 글이 바뀌면 다시 거른다.
    pub(crate) fn filter_clip(&mut self, act: nexa_ctl::EditCtxAction, inv: &mut Invalidations) {
        use nexa_ctl::EditCtxAction as A;
        let tb = self.filter.textbox_mut();
        match act {
            A::Copy => {
                if let Some(t) = tb.copy_selection() {
                    let _ = crate::clipboard::write_text(&t);
                }
                return;
            }
            A::Cut => {
                if let Some(t) = tb.cut_selection(inv) {
                    let _ = crate::clipboard::write_text(&t);
                }
            }
            A::Paste => {
                if let Some(t) = crate::clipboard::read_text() {
                    tb.paste(&t, inv);
                }
            }
            A::Custom(_) => return,
        }
        inv.push(self.filter_bounds());
        self.apply_filter_text(inv);
    }

    /// 검색 상자(시험 · 덤프).
    #[cfg(test)]
    pub(crate) fn filter_box(&self) -> &nexa_ctl::SearchBox {
        &self.filter
    }

    /// 혜성꼬리(nexa-ctl `BusyRing` · TextBox 속성): 필터가 적용되는 동안 Running → 바로 Done(동기 · 최소 표시 뒤 완료 깜빡임) · 빈 글 = Idle.
    fn flash_filter_busy(&mut self) {
        use nexa_ctl::controls::busy::BusyState;
        if self.filter.text().trim().is_empty() {
            self.filter.set_busy(BusyState::Idle);
        } else {
            self.filter.set_busy(BusyState::Running);
            // 하위 폴더 검색(10-26)이 워커에 가 있으면 결과가 다 올 때까지 Running — `tick`이 Done으로 바꾼다.
            if !self.rows().source().deep_pending() {
                self.filter.set_busy(BusyState::Done);
            }
        }
    }

    fn apply_filter_text(&mut self, inv: &mut Invalidations) {
        let text = self.filter.text();
        let rows = self.rows_mut();
        if rows.source_mut().set_name_filter(&text) {
            inv.push(rows.bounds());
            self.sync_status(inv);
            self.flash_filter_busy(); // 혜성꼬리(10-25 검색창)
        }
        self.sync_filter_error();
    }

    /// 활성 탭 원천의 정규식 오류 → 검색창 틀 danger(nexa-sql 규약 · 10-26).
    fn sync_filter_error(&mut self) {
        let err = self.rows().source().filter_err();
        self.filter.set_error(err);
    }

    /// 필터 상자 진행 표시 상태(시험).
    #[cfg(test)]
    pub(crate) fn filter_busy_state(&self) -> nexa_ctl::controls::busy::BusyState {
        self.filter.busy_state()
    }

    /// 필터 바로 가는 키(글자 · 편집 키 · Enter = 목록으로 포커스 · Esc = 글이 있으면 지움 · 없으면 닫기). 처리했으면 `true`.
    fn filter_key(&mut self, ev: &InputEvent, inv: &mut Invalidations) -> bool {
        use nexa_ctl::Control as _;
        if !self.filter_focused() {
            return false;
        }
        match *ev {
            InputEvent::Key {
                key: CtlKey::Escape,
                ..
            } => {
                if self.filter.text().is_empty() {
                    self.close_filter(inv);
                } else {
                    self.filter.set_text("");
                    inv.push(self.filter_bounds());
                    self.apply_filter_text(inv);
                }
            }
            InputEvent::Key {
                key: CtlKey::Enter, ..
            } => {
                self.filter.set_focused(false);
                inv.push(self.filter_bounds());
            }
            _ => {
                let _ = self.filter.handle(ev, inv);
                self.apply_filter_text(inv);
            }
        }
        true
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
        self.ensure_git_info();
        let src = self.rows().source();
        // 빠른 필터 중이면 "n / m 항목"(협업 제안 10-25 — 빈 목록이 필터 때문임을 알린다) · "NFD만"도 필터다.
        let folder_text = if src.name_filter().is_empty() && !src.nfd_only() {
            ndir_i18n::trf("status.itemCount", &[&src.len().to_string()])
        } else {
            ndir_i18n::trf(
                "status.itemCountFiltered",
                &[&src.len().to_string(), &src.unfiltered_len().to_string()],
            )
        };
        // 폴더 칸 폭 견본(T-145 선례 · 10-31 협업 판정): 필터 켬/끔에 글이 "n개 항목" ↔ "m개 중 n개 항목(필터)"로 바뀌어도 칸 너비가
        // 그대로라 오른쪽 "NFD n" 칸이 밀리지 않는다(종전엔 토글 뒤 같은 자리를 다시 누르면 폴더 칸이 눌렸다).
        let folder_hint = ndir_i18n::trf(
            "status.itemCountFiltered",
            &[
                &src.unfiltered_len().to_string(),
                &src.unfiltered_len().to_string(),
            ],
        );
        let mut segs = vec![nexa_ctl::StatusSeg::with_parts(
            SEG_FOLDER,
            // 글은 왼쪽(nexa-ui 252 `left()` — 견본 폭만 빌리고 "n개 항목" 글머리는 제자리 · 협업 관찰: 오른쪽 정렬이면 왼쪽 빈칸).
            vec![nexa_ctl::StatusPart::new(folder_text)
                .hints(vec![folder_hint])
                .left()],
        )];
        if let Some((_, Some((_, branch)))) = &self.git {
            let mut text = ndir_i18n::trf("tabstatus.git", &[branch]);
            if !self.git_extra.is_empty() {
                text.push(' ');
                text.push_str(&self.git_extra);
            }
            segs.push(nexa_ctl::StatusSeg::new(SEG_GIT, text));
        }
        // 분해형 이름 집계(10-31 ②): 있을 때만 · "NFD만" 보기 중이면 켜진 글.
        let nfd = src.nfd_count();
        if nfd > 0 || src.nfd_only() {
            let key = if src.nfd_only() {
                "status.nfdOnly"
            } else {
                "status.nfdCount"
            };
            segs.push(nexa_ctl::StatusSeg::new(
                SEG_NFD,
                ndir_i18n::trf(key, &[&nfd.to_string()]),
            ));
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

    /// Git 브랜치 캐시를 활성 탭의 폴더에 맞춘다 — 폴더가 바뀌었거나 캐시를 버렸을 때만 `.git/HEAD`를 읽는다(가상 최상위 · 네트워크
    /// 경로 = 없음). [`Self::sync_status`]와 호스트 `git_sync`(캐시를 버린 뒤 `.git/HEAD` 재독 — 10-08) 공용.
    fn ensure_git_info(&mut self) {
        let root = self.root_path();
        if self.git.as_ref().is_none_or(|(p, _)| *p != root) {
            let found = if ndir_vfs::is_virtual_root(&root) || ndir_vfs::is_network_path(&root) {
                None
            } else {
                crate::dirinfo::git_branch(&root)
            };
            self.git = Some((root, found));
        }
    }

    /// 탭 상태바의 Git 정보 `(저장소 루트, 브랜치)`(활성 탭 · 저장소 밖 = `None`) — 캐시 그대로(읽지 않는다).
    pub(crate) fn git_info(&self) -> Option<(PathBuf, String)> {
        self.git.as_ref().and_then(|(_, g)| g.clone())
    }

    /// [`Self::git_info`]와 같되 캐시가 비었으면(폴더 이동 · [`Self::invalidate_dir_info`] 뒤) 먼저 `.git/HEAD`를 읽는다 —
    /// 호스트 `git_sync`가 쓴다(캐시를 버린 직후에도 저장소를 놓치지 않게). 탭 상태바가 꺼져 있으면 읽지 않는다(칸이 없다).
    pub(crate) fn git_info_fresh(&mut self) -> Option<(PathBuf, String)> {
        if self.status_on {
            self.ensure_git_info();
        }
        self.git_info()
    }

    /// Git 칸 요약(앞섬 · 뒤짐 · 변경 수)을 넣는다 — 다음 [`Self::sync_status`]가 칸에 반영한다.
    /// Git 요약 덧글(`↑n ↓n ●n` …) — 기동 명령 `tabstatus` 덤프.
    pub(crate) fn git_extra(&self) -> &str {
        &self.git_extra
    }

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

    /// "NFD n" 칸 클릭 = 분해형 이름 행만 보기 토글(10-31 ②) — 목록·상태 줄 무효화.
    pub(crate) fn toggle_nfd_only(&mut self, inv: &mut Invalidations) {
        let on = !self.rows().source().nfd_only();
        let rows = &mut self.tabs[self.active].rows;
        if rows.source_mut().set_nfd_only(on) {
            inv.push(self.bounds);
        }
        self.sync_status(inv);
    }

    /// 분해형 이름 행만 보는 중인가(시험).
    #[cfg(test)]
    pub(crate) fn nfd_only(&self) -> bool {
        self.rows().source().nfd_only()
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
    /// 자기 사각형 밖에 그리는 팝업이 보이는가(네비 버튼 툴팁 · 경로 제안) — 호스트의 부분 다시 그리기가 전체로 바꾸는 판정(10-18 결정 ①).
    pub(crate) fn popups_open(&self) -> bool {
        self.navbtns.tooltip_visible()
            || self.pathbar.suggest_open()
            // 검색창 편집 메뉴 · 토글 툴팁(10-31 사용자 macOS "툴팁이 안 보이거나 깜빡임": 빠져 있어 부분 프레임 클립에 잘려
            // IOSurface 표면마다 다르게 그려졌다 · nexa-ui 249 `tooltip_visible`).
            || (self.filter_on && (self.filter.popup_open() || self.filter.tooltip_visible()))
    }

    /// 검색창(시험 · 툴팁 지연·토글 자리).
    #[cfg(test)]
    pub(crate) fn filter_box_mut(&mut self) -> &mut nexa_ctl::SearchBox {
        &mut self.filter
    }

    pub(crate) fn paint_popups(&self, ctx: &mut dyn DrawCtx, theme: &Theme) {
        self.navbtns.paint_tooltip(ctx, theme);
        if self.filter_on {
            // 검색창 편집 메뉴 + 토글 툴팁(10-26).
            self.filter.paint_popup(ctx, theme);
        }
        let mut a = nexa_grid::Adapt(ctx);
        self.pathbar.paint_suggest(&mut a, theme);
    }

    /// 틱(목록 스크롤바 페이드 · 타입어헤드 · 툴팁) — 더 돌아야 하면 `inv.request_tick`.
    pub(crate) fn tick(&mut self, now_ms: u64, inv: &mut Invalidations) {
        if self.nav_poll(inv) {
            inv.push(self.bounds);
        }
        self.tabs[self.active].rows.tick(now_ms, inv);
        // 필터 상자(혜성꼬리 · 호버) + 토글 버튼(호버 페이드) — 움직이는 동안 프레임을 이어 간다(10-25 검색창).
        if self.filter_on {
            if self.filter.tick(now_ms) {
                inv.push(self.filter_bar_rect());
            }
            if self.filter.wants_tick() {
                inv.request_tick();
            }
        }
        // 빈 폴더 프로브 결과(워커 · 10-18) — 바뀐 마커만큼 목록을 다시 그린다.
        if self.tabs[self.active].rows.source_mut().drain_probes() {
            inv.push(self.tabs[self.active].rows.bounds());
        }
        // 하위 폴더 검색 결과(워커 · 사용자 10-26 검색창 범위 토글) — 목록이 바뀌면 다시 그리고 "n/m" 갱신 · 다 왔으면 혜성 Done.
        if self.tabs[self.active].rows.source_mut().deep_tick() {
            inv.push(self.tabs[self.active].rows.bounds());
            self.sync_status(inv);
        }
        {
            use nexa_ctl::controls::busy::BusyState;
            if self.filter_on
                && self.filter.busy_state() == BusyState::Running
                && !self.rows().source().deep_pending()
            {
                self.filter.set_busy(BusyState::Done);
                inv.push(self.filter_bar_rect());
            }
        }
        self.poll_drive_space(inv);
    }

    /// 활성 탭의 보이는 폴더 프로브를 지금 제출(덤프 전).
    pub(crate) fn probe_prefetch(&self) {
        self.tabs[self.active].rows.source().probe_prefetch();
    }

    /// 활성 탭이 프로브 결과를 기다리는가(이벤트 루프 깨움 · 덤프 대기).
    pub(crate) fn probes_pending(&self) -> bool {
        self.tabs[self.active].rows.source().probes_pending()
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

    /// 종류 열 OS 이름 켬/끔(NEW-059 · 10-31) — 모든 탭 원천에.
    pub(crate) fn set_os_kind(&mut self, on: bool, inv: &mut Invalidations) {
        self.os_kind = on;
        self.apply_typeahead(inv);
    }

    fn apply_typeahead(&mut self, inv: &mut Invalidations) {
        let o = self.ta_opts;
        let icons = !self.no_row_icons;
        let os_kind = self.os_kind;
        for tab in &mut self.tabs {
            if tab.rows.source_mut().set_icons(icons) {
                inv.push(tab.rows.bounds());
            }
            if tab.rows.source_mut().set_os_kind(os_kind) {
                inv.push(tab.rows.bounds());
            }
            tab.rows.source_mut().set_find_scope(o.scope);
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
        // 빠른 필터 바 = 활성 탭 원천의 글(탭 전환 · 이동(새 원천 = 빈 글)에 맞춘다).
        let ft = self.filter_text();
        if self.filter.text() != ft {
            self.filter.set_text(&ft);
            inv.push(self.filter_bounds());
        }
        let root = self.root_path();
        // 워커가 읽는 중이면 경로 바는 이미 새 위치(결과가 올 때까지 목록만 옛것 · 10-18 결정 ②).
        let shown = self
            .nav_reading()
            .map_or_else(|| root.clone(), Path::to_path_buf);
        self.pathbar.set_path(display_path(&shown), inv);
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
        self.push_tab(path, self.opts, inv); // 새 탭의 보기 옵션 = 설정 기본값(사용자 10-03)
    }

    /// 탭 하나를 **열거 1회**로 더한다(새 탭 · 세션 복원 공용 · 10-18 체감 점검 — 종전 복원은 `new_tab`(현재 폴더 열거) →
    /// `navigate_to`(대상 열거) → `set_opts`(옵션이 다르면 재열거)로 탭마다 2~3회 열거했다). 열 · 보기 모드 · 정렬 · 포커스는
    /// 활성 탭에서 물려받고 루트 종류(내 PC = 드라이브 열)에 맞춰 열을 맞춘다.
    fn push_tab(&mut self, path: PathBuf, opts: ListOpts, inv: &mut Invalidations) {
        let mut rows = VirtualRows::new(
            TreeSource::open(&path, opts),
            self.m.row_h,
            self.m.pad_x,
            self.m.indent_w,
        );
        rows.set_columns(self.rows().columns().to_vec(), inv);
        rows.set_sort_mark_trailing(true, inv);
        rows.set_sort_mark_vector(true, inv);
        rows.set_sort_click_adds(self.sort_click_adds);
        rows.source_mut().set_filter_opts(self.filter_defaults); // 토글 이어받기(10-26 · 전역 없음)
        rows.set_col_drag_marker(true);
        rows.set_focused(self.focused, inv);
        rows.set_view_mode(self.rows().view_mode(), inv);
        rows.set_sort(self.rows().sort(), inv); // 정렬 상태 상속(T-128 · nexa-ui 191)
        self.tabs.push(Tab {
            rows,
            nav: History::new(path),
            locked: false,
            pinned: false,
            stale: false,
            pending: None,
            reloading: None,
        });
        self.active = self.tabs.len() - 1;
        self.navigated = true;
        self.session_dirty = true;
        self.sync_columns_for_root(inv);
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
        rows.set_sort_mark_vector(true, inv);
        rows.set_sort_click_adds(self.sort_click_adds);
        rows.source_mut().set_filter_opts(self.filter_defaults); // 토글 이어받기(10-26 · 전역 없음)
        rows.set_col_drag_marker(true);
        rows.set_focused(self.focused, inv);
        rows.set_view_mode(self.tabs[i].rows.view_mode(), inv);
        rows.set_sort(self.tabs[i].rows.sort(), inv); // 원본 탭 정렬 상속(T-128)
        self.tabs.insert(
            i + 1,
            Tab {
                rows,
                nav: History::new(path),
                locked: false,
                pinned: false,
                stale: false,
                pending: None,
                reloading: None,
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
    /// 표시 열이 달라도 같은 열끼리만 폭이 옮겨 간다 · 폴더 유형은 [`Self::apply_col_widths_by_key`]의 `virt`가 가른다).
    pub(crate) fn col_widths_by_key(&self) -> Vec<(u32, i32)> {
        self.rows()
            .columns()
            .iter()
            .map(|c| (c.key, c.width))
            .collect()
    }

    /// key별 열 폭을 **이 패널의 같은 유형 탭 전부**에(없는 열은 건너뜀) — `virt` = 내 PC(드라이브 열) 탭 · 아니면 실제 폴더 탭.
    /// 실제 폴더면 기본 열에도 반영해 새 탭이 그 폭을 잇고 · 내 PC면 `drive_widths`에 기억해 다음 진입이 잇는다(탭이 없어도).
    /// 유형이 다른 탭은 건드리지 않는다(사용자 10-24 · T-193). 바뀐 것이 있으면 `true`. 이후 배치가 기본 열로 덮지 않는다.
    pub(crate) fn apply_col_widths_by_key(
        &mut self,
        widths: &[(u32, i32)],
        virt: bool,
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
            if tab.rows.source().is_virtual_root() != virt {
                continue;
            }
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
        if virt {
            for key in DRIVE_KEYS {
                let Some(w) = widths.iter().find(|(k, _)| *k == key).map(|(_, w)| *w) else {
                    continue;
                };
                match self.drive_widths.iter_mut().find(|(k, _)| *k == key) {
                    Some(slot) if slot.1 == w => {}
                    Some(slot) => {
                        slot.1 = w;
                        changed = true;
                    }
                    None => {
                        self.drive_widths.push((key, w));
                        changed = true;
                    }
                }
            }
            if changed {
                self.session_dirty = true;
            }
            return changed;
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
        self.open_nav(path, NavKind::Push, None, inv)
    }

    pub(crate) fn nav_back(&mut self, inv: &mut Invalidations) {
        let Some(p) = self.tabs[self.active]
            .nav
            .peek_back()
            .map(Path::to_path_buf)
        else {
            return;
        };
        let _ = self.open_nav(p, NavKind::Back, None, inv); // 실패 = 위치 그대로
    }

    pub(crate) fn nav_forward(&mut self, inv: &mut Invalidations) {
        let Some(p) = self.tabs[self.active]
            .nav
            .peek_forward()
            .map(Path::to_path_buf)
        else {
            return;
        };
        let _ = self.open_nav(p, NavKind::Forward, None, inv);
    }

    /// 비동기 폴더 읽기 설정(`list.async_read` · `list.async_wait_ms`).
    pub(crate) fn set_async_read(&mut self, on: bool, wait_ms: i64) {
        self.async_read = on;
        self.async_wait_ms = wait_ms.max(0) as u64;
    }

    /// 시험: 창 없이도 워커 경로를 쓴다.
    #[cfg(test)]
    pub(crate) fn force_async(&mut self) {
        self.async_force = true;
    }

    fn async_enabled(&self) -> bool {
        #[cfg(test)]
        if self.async_force {
            return self.async_read;
        }
        self.async_read && crate::filelist::probe::is_async()
    }

    /// 폴더 열기 한 길(10-18 결정 ②): 사전 점검(없음 · 권한) → 동기(창 없음 · 설정 끔)면 그 자리에서 · 아니면 워커에 맡기고
    /// 문턱(`async_wait_ms`) 안에 끝나면 그 자리에서 적용 · 넘으면 `pending`(경로 바만 새 위치 · 목록은 결과가 올 때까지 옛것 ·
    /// 결과는 [`Self::nav_poll`]). 히스토리는 적용 때 움직인다(연속 진입이면 앞 결과는 버려진다).
    fn open_nav(
        &mut self,
        path: PathBuf,
        kind: NavKind,
        select: Option<PathBuf>,
        inv: &mut Invalidations,
    ) -> Option<String> {
        if let Err(why) = TreeSource::can_open(&path) {
            self.sync_chrome(inv);
            return Some(why);
        }
        let opts = self.tab_opts();
        if !self.async_enabled() {
            let src = TreeSource::open(&path, opts);
            if let Some(e) = src.error() {
                let why = e.to_string();
                self.sync_chrome(inv);
                return Some(why);
            }
            self.tabs[self.active].pending = None;
            let i = self.active;
            self.apply_nav(i, src, kind, select, inv);
            return None;
        }
        let rx = TreeSource::open_async(path.clone(), opts);
        match recv_within(&rx, self.async_wait_ms) {
            Ok(src) => {
                if let Some(e) = src.error() {
                    let why = e.to_string();
                    self.sync_chrome(inv);
                    return Some(why);
                }
                self.tabs[self.active].pending = None;
                let i = self.active;
                self.apply_nav(i, src, kind, select, inv);
                None
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                self.tabs[self.active].pending = Some(PendingNav {
                    rx,
                    path,
                    select,
                    kind,
                });
                self.sync_chrome(inv); // 경로 바 = 새 위치 · 상태줄 "읽는 중…"은 호스트(`update_status`)
                None
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                self.sync_chrome(inv);
                Some("enumeration worker stopped".into())
            }
        }
    }

    /// 열기 결과 적용(동기 · 워커 공용): 히스토리 이동 → 소스 교체(활성 탭 = 열 맞춤 · 크롬 · 선택 / 배경 탭 = 교체만).
    fn apply_nav(
        &mut self,
        i: usize,
        src: TreeSource,
        kind: NavKind,
        select: Option<PathBuf>,
        inv: &mut Invalidations,
    ) {
        let mut src = src;
        src.set_filter_opts(self.filter_defaults); // 새 원천도 이 패널의 토글을 이어받는다(10-26 · 전역 없음)
        let tab = &mut self.tabs[i];
        tab.reloading = None; // 진입 결과가 최신 — 걸려 있던 재열람은 버린다
        match kind {
            NavKind::Push => tab.nav.push(src.path().to_path_buf()),
            NavKind::Back => {
                let _ = tab.nav.back();
            }
            NavKind::Forward => {
                let _ = tab.nav.forward();
            }
        }
        if i == self.active {
            self.apply_source(src, inv);
            if let Some(p) = select {
                self.select_path(&p, inv);
            }
        } else {
            tab.rows.replace_source(src, inv);
            tab.stale = false;
            self.session_dirty = true;
            self.navigated = true;
        }
    }

    /// 워커 열기 결과 수거(틱) — 적용했거나 버렸으면 `true`. 오류 결과(그 사이 사라짐)는 자리 유지 · 경로 바 복귀.
    pub(crate) fn nav_poll(&mut self, inv: &mut Invalidations) -> bool {
        let mut done = self.reload_poll(inv);
        for i in 0..self.tabs.len() {
            let Some(p) = &self.tabs[i].pending else {
                continue;
            };
            match p.rx.try_recv() {
                Ok(src) => {
                    let p = self.tabs[i].pending.take().expect("pending");
                    if src.error().is_none() {
                        self.apply_nav(i, src, p.kind, p.select, inv);
                    } else if i == self.active {
                        self.sync_chrome(inv);
                    }
                    done = true;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.tabs[i].pending = None;
                    if i == self.active {
                        self.sync_chrome(inv);
                    }
                    done = true;
                }
            }
        }
        done
    }

    /// 워커 재열람 결과 수거(틱 · `nav_poll`이 부른다) — 끼웠거나 버렸으면 `true`. 만들 때와 경로·옵션이 다르거나 진입이 걸려
    /// 있으면 버린다(낡음). 진행 중 또 요청이 있었으면(`again`) 한 번 더.
    fn reload_poll(&mut self, inv: &mut Invalidations) -> bool {
        let mut done = false;
        for i in 0..self.tabs.len() {
            let Some(r) = &self.tabs[i].reloading else {
                continue;
            };
            match r.rx.try_recv() {
                Ok(built) => {
                    let r = self.tabs[i].reloading.take().expect("reloading");
                    let src = self.tabs[i].rows.source();
                    let same = self.tabs[i].pending.is_none()
                        && src.path() == r.path
                        && src.opts() == r.opts;
                    if same {
                        self.apply_reload(i, built, r.from_watch, inv);
                    }
                    if r.again {
                        self.reload_tab(i, r.from_watch, inv);
                    }
                    done = true;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.tabs[i].reloading = None;
                    done = true;
                }
            }
        }
        done
    }

    /// 어느 탭이든 워커가 읽는 중인가(진입 · 재열람 — 이벤트 루프 깨움 · 덤프 대기).
    pub(crate) fn nav_pending(&self) -> bool {
        self.tabs
            .iter()
            .any(|t| t.pending.is_some() || t.reloading.is_some())
    }

    /// 활성 탭이 읽는 중인 대상(경로 바 · 상태줄 "읽는 중…").
    pub(crate) fn nav_reading(&self) -> Option<&Path> {
        self.tabs[self.active]
            .pending
            .as_ref()
            .map(|p| p.path.as_path())
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
        let _ = self.open_nav(parent, NavKind::Push, Some(left), inv); // 떠난 폴더 선택은 결과 적용 때
    }

    /// [홈] — 깊이와 무관하게 내 PC로 한 번에(dir2 08-01) · 떠난 위치를 선택.
    pub(crate) fn nav_home(&mut self, inv: &mut Invalidations) {
        let left = self.root_path();
        if ndir_vfs::is_virtual_root(&left) {
            return;
        }
        let _ = self.open_nav(
            PathBuf::from(ndir_vfs::MY_PC),
            NavKind::Push,
            Some(left),
            inv,
        );
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

    /// Shift 없는 헤더 클릭 = 정렬 추가(설정 `list.sort_click_adds` · T-128 · DR-31 ⑦) — 열린 탭 전부에 즉시.
    pub(crate) fn set_sort_click_adds(&mut self, on: bool) {
        self.sort_click_adds = on;
        for tab in &mut self.tabs {
            tab.rows.set_sort_click_adds(on);
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
        let (hidden, dot, folders, _, _) = self.tab_opts().view();
        (hidden, dot, folders)
    }

    /// 활성 탭의 보기 옵션 5종(숨김 · Dot · 폴더 우선 · 대소문자 구분 정렬 · 자연 정렬).
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

    /// 탭 최소 폭 · 닫기 상자 뒤 여백(설정 `tabs.min_width` · `tabs.close_pad` · 논리 px).
    pub(crate) fn set_tab_metrics(&mut self, min_w: i32, close_pad: i32, inv: &mut Invalidations) {
        self.tabbar.set_min_width(min_w, inv);
        self.tabbar.set_close_pad(Some(close_pad), inv);
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

    /// 세션 저장 — 탭별 정렬(`panel{i}.sort` · dir3 신규 T-128): 탭마다 `name+,size-`(키 이름 + 방향 · 우선순위 순) ·
    /// 빈 문자열 = 소스 기본(이름 오름차순). 전부 기본이면 빈 목록(직렬화 생략 왕복).
    pub(crate) fn session_sort(&self) -> Vec<String> {
        let out: Vec<String> = self
            .tabs
            .iter()
            .map(|t| sort_to_str(t.rows.sort()))
            .collect();
        if out.iter().all(String::is_empty) {
            Vec::new() // 전부 소스 기본(이름 오름차순 · 새 그리드의 빈 정렬 목록)
        } else {
            out
        }
    }

    /// 세션 저장 — 탭별 보기 옵션 플래그(dir2 `panel{i}.views`).
    pub(crate) fn session_view_flags(&self) -> Vec<u8> {
        self.tabs
            .iter()
            .map(|t| t.rows.source().opts().view_flags())
            .collect()
    }

    /// 다시 읽기(활성 탭 **자기 보기 옵션 그대로** · 캐럿·스크롤 유지). 배경 탭은 다시 읽지 않고 **낡음**만 세운다(10-18 체감 점검:
    /// 감시 통지·파일 작업 뒤마다 모든 탭을 동기 재열거하던 낭비 — 배경 탭은 어차피 전환 때 `stale`로 다시 읽는다 · dir2 X-44 S1).
    pub(crate) fn reopen(&mut self, inv: &mut Invalidations) {
        self.reopen_with(false, inv);
    }

    /// 이름을 바꾼 뒤 다시 읽기(WINB-033 · 10-25): 활성 탭은 펼침·선택의 접두를 새 이름으로 바꿔 복원(동기) · 다른 탭은 낡음.
    pub(crate) fn reopen_renamed(&mut self, old: &Path, new: &Path, inv: &mut Invalidations) {
        let active = self.active;
        for (i, tab) in self.tabs.iter_mut().enumerate() {
            if i != active {
                tab.stale = true;
            }
        }
        let tab = &mut self.tabs[active];
        if tab.pending.is_some() || tab.reloading.is_some() {
            tab.stale = true;
        } else {
            let (caret, sr, sx) = (tab.rows.caret(), tab.rows.scroll_row(), tab.rows.scroll_x());
            tab.rows.source_mut().reload_renamed(old, new);
            tab.rows.restore_view(caret, sr, sx, inv);
            tab.stale = false;
        }
        self.sync_chrome(inv);
    }

    /// 감시 통지로 다시 읽기(10-18 결정 ③): `reopen` + 신선 스탬프 지움 — 감시가 읽은 것은 억제 창을 열지 않는다
    /// (바깥에서 변경이 이어지면 통지마다 종전대로 다시 읽는다).
    pub(crate) fn reopen_from_watch(&mut self, inv: &mut Invalidations) {
        self.reopen_with(true, inv);
    }

    fn reopen_with(&mut self, from_watch: bool, inv: &mut Invalidations) {
        let active = self.active;
        for (i, tab) in self.tabs.iter_mut().enumerate() {
            if i != active {
                tab.stale = true;
            }
        }
        self.reload_tab(active, from_watch, inv);
        self.sync_chrome(inv);
    }

    /// 탭 하나 다시 읽기(동기 · 워커 공용 · ② 2단계): 진입이 걸려 있으면 그 결과가 최신(낡음만 세움) · 재열람이 진행 중이면 끝난 뒤
    /// 한 번 더(`again`) · 동기(창 없음 · 설정 끔)면 그 자리에서 · 아니면 워커 + 문턱 안에 끝나면 그 자리 · 넘으면 `reloading`.
    fn reload_tab(&mut self, i: usize, from_watch: bool, inv: &mut Invalidations) {
        let (is_async, wait_ms) = (self.async_enabled(), self.async_wait_ms);
        let tab = &mut self.tabs[i];
        if tab.pending.is_some() {
            tab.stale = true;
            return;
        }
        if let Some(r) = &mut tab.reloading {
            r.again = true;
            r.from_watch |= from_watch;
            return;
        }
        if !is_async {
            let src = tab.rows.source();
            let built = TreeSource::reload_build(&src.reload_plan());
            self.apply_reload(i, built, from_watch, inv);
            return;
        }
        let rx = tab.rows.source().reload_async();
        match recv_within(&rx, wait_ms) {
            Ok(built) => self.apply_reload(i, built, from_watch, inv),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                let src = tab.rows.source();
                tab.reloading = Some(PendingReload {
                    rx,
                    path: src.path().to_path_buf(),
                    opts: src.opts(),
                    from_watch,
                    again: false,
                });
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {}
        }
    }

    /// 만든 트리를 끼운다(캐럿 · 스크롤 유지 · 낡음 내림 · 감시가 시킨 것이면 신선 스탬프 지움 · 활성 탭이면 크롬).
    fn apply_reload(
        &mut self,
        i: usize,
        built: crate::filelist::ReloadBuilt,
        from_watch: bool,
        inv: &mut Invalidations,
    ) {
        let tab = &mut self.tabs[i];
        let (caret, sr, sx) = (tab.rows.caret(), tab.rows.scroll_row(), tab.rows.scroll_x());
        tab.rows.source_mut().reload_apply(built);
        tab.rows.restore_view(caret, sr, sx, inv);
        tab.stale = false;
        if from_watch {
            tab.rows.source_mut().clear_fresh();
        }
        if i == self.active {
            self.sync_chrome(inv);
        }
    }

    /// 활성 탭이 `dir`을 방금(`within` 안) 스스로 읽었고 폴더 수정 시각이 그때와 같은가 — 감시 통지 = 우리 변경의 메아리.
    pub(crate) fn self_fresh(&self, dir: &Path, within: std::time::Duration) -> bool {
        let src = self.rows().source();
        src.path() == dir && src.self_fresh(within)
    }

    /// 활성 탭이 낡았는가(전환·닫기로 드러난 뒤 아직 다시 읽지 않음 · dir2 `active_tab_stale`).
    #[cfg(test)]
    pub(crate) fn active_tab_stale(&self) -> bool {
        self.tabs[self.active].stale
    }

    /// 낡은 활성 탭만 다시 읽는다(캐럿·스크롤 유지 · dir2 win.rs:4967-4976 "전환 수렴") — 읽었으면 `true`.
    /// 인라인 이름 편집 중이면 미룬다(편집 행이 바뀌면 안 된다 — 다음 길목에서 다시 본다).
    pub(crate) fn refresh_stale(&mut self, inv: &mut Invalidations) -> bool {
        let tab = &self.tabs[self.active];
        if !tab.stale
            || tab.reloading.is_some()
            || tab.pending.is_some()
            || self.rows().is_renaming()
        {
            return false;
        }
        let active = self.active;
        self.reload_tab(active, false, inv);
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

    /// 활성 탭의 **보이는 폴더 행** 경로(T-166 3차 · 크기 열) — 가상 루트(내 PC)는 없음(드라이브 전체를 재지 않는다).
    pub(crate) fn visible_dir_paths(&self) -> Vec<PathBuf> {
        let rows = self.rows();
        let src = rows.source();
        if ndir_vfs::is_virtual_root(src.path()) {
            return Vec::new();
        }
        let (s, e) = rows.viewport();
        // 번들(macOS `.app` · 파일로 다룸)도 폴더 크기 합계 대상(GAP-025 후속 · 10-31 — 종전 크기 열 = 디렉터리 메타 길이).
        (s..e)
            .filter(|&i| src.row_is_dir(i) || src.row_is_bundle(i))
            .filter_map(|i| src.row_path(i))
            .collect()
    }

    /// 크기 열의 폴더 크기 지도(T-166 3차): 활성 탭에 넘기고(바뀌면 목록 무효화) · 빈 지도면 모든 탭을 비운다(설정 끔).
    pub(crate) fn set_dir_sizes(
        &mut self,
        sizes: std::collections::HashMap<PathBuf, (u64, bool)>,
        inv: &mut Invalidations,
    ) {
        if sizes.is_empty() {
            for tab in &mut self.tabs {
                if tab.rows.source_mut().set_dir_sizes(Default::default()) {
                    inv.push(tab.rows.bounds());
                }
            }
            return;
        }
        let tab = &mut self.tabs[self.active];
        if tab.rows.source_mut().set_dir_sizes(sizes) {
            inv.push(tab.rows.bounds());
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
        } else if self.filter_on && self.filter_bar_rect().contains(p) {
            Some(Part::Filter)
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
            Part::Filter => {
                use nexa_ctl::SearchEvent;
                if matches!(*ev, InputEvent::RightDown { .. }) {
                    // 편집 메뉴의 "붙여넣기" 활성 = 클립보드에 글이 있을 때(prefs_win과 같은 규칙 · 10-26).
                    let has = crate::clipboard::read_text().is_some_and(|s| !s.is_empty());
                    self.filter.textbox_mut().set_clipboard_has_text(has);
                }
                // 토글(뗄 때 · DR-27) · 상자 포커스 · 글자는 SearchBox가 가른다(nexa-ui 235 · 10-26) — 토글 = 다시 걸러 혜성 + 설정에 적기 ·
                // 글 = 다시 거름.
                match self.filter.handle(ev, inv) {
                    SearchEvent::Toggled(_) => {
                        let o = self.filter_opts();
                        self.set_filter_opts(o, inv);
                        self.filter_opts_changed = true;
                        self.flash_filter_busy();
                    }
                    SearchEvent::Changed => self.apply_filter_text(inv),
                    SearchEvent::None | SearchEvent::Consumed => {}
                }
                // 편집 메뉴에서 고른 행동(복사 · 잘라내기 · 붙여넣기) → 호스트가 클립보드를 잇는다(`take_filter_clip`).
                if let Some(act) = self.filter.textbox_mut().take_edit_ctx() {
                    self.filter_clip = Some(act);
                }
            }
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
        // 검색창 편집 메뉴(TextBox 팝업)가 열려 있으면 포인터 사건은 전부 검색창으로(사용자 10-26 — 종전엔 팝업이 목록 위에 떠 있어
        // 항목 클릭을 `part_at`이 목록으로 보냈고 복사/붙여넣기가 듣지 않았다). 팝업 밖 클릭 = 닫기만(아래로 흘리지 않는다 · 팝업 UX 규칙).
        if self.filter_on
            && self.filter.popup_open()
            && matches!(
                *ev,
                InputEvent::MouseMove { .. }
                    | InputEvent::MouseDown { .. }
                    | InputEvent::MouseUp { .. }
                    | InputEvent::RightDown { .. }
            )
        {
            self.send(Part::Filter, ev, inv);
            return;
        }
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
                    // 검색창도 hover를 받는다(토글 hover 페이드 · 토글 툴팁 — 10-31 사용자 macOS "툴팁이 안 보임": 종전엔 누른 채
                    // 움직일 때와 편집 메뉴가 열렸을 때만 가서 호버로는 툴팁이 뜰 수 없었다).
                    if self.filter_on {
                        self.send(Part::Filter, ev, inv);
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
                // 필터 바 밖 클릭 = 상자 포커스 해제(글과 필터는 그대로 · 바는 열린 채).
                if part != Part::Filter && self.filter_focused() {
                    use nexa_ctl::Control as _;
                    self.filter.set_focused(false);
                    inv.push(self.filter_bounds());
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
        if self.filter_key(ev, inv) {
            return;
        }
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
        // 머리를 눌러 정렬을 바꿨다 → 세션에 남긴다(T-128 영속 · `panel{i}.sort`).
        if self.tabs[self.active].rows.take_sort_changed() {
            self.session_dirty = true;
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
            natural: true,
            hide_empty_glyph: false,
            nfc_display: true,
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
        struct FixedDisk;
        impl crate::platform::Disk for FixedDisk {
            fn space(&self, _root: &Path) -> Option<(u64, u64)> {
                Some((10, 5))
            }
        }
        p.fill_drive_space(&FixedDisk, &mut inv);
        assert!(
            p.rows().source().drive_space_known(),
            "동기 포트 = 바로 채움"
        );
        p.nav_back(&mut inv);
        assert_eq!(p.rows().columns().len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
