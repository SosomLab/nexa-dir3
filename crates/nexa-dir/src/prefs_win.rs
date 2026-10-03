//! 환경 설정 창(nexa-sql `prefs_win.rs` 복사 · dir3 M3 T-44 · docs/port/41 SET · port/15 PREFS-301~313 참고) — VS Code/DBeaver식:
//! **검색** · 왼쪽 **트리**(그룹 ▸ 카테고리 = `CATEGORY_TREE` · dir2 사이드바 순) · 오른쪽 **설정 카드 목록**(스크롤) —
//! 종류별 컨트롤: Bool = 스위치 · Choice = 콤보 · Int/Size/Text = 입력란(즉시 검증) · Position = 3×3 위치 드롭다운 ·
//! 단축키(`key.*`) = 입력란 + [지정…](단축키 창). 기본값과 다른 카드는 [초기화]. 값은 바꾸는 즉시 호스트가 저장·반영(`PrefsAction::Changed`).
//! 이 창은 레지스트리 스냅샷(`refresh`)만 가진다 — 설정 파일·적용은 호스트 몫.
//!
//! nexa-sql 대비 뺀 것: SQL 포맷 미리보기 · 성능 향상 강제값 · 확장 분류 · 검색어 이력 · 색 창(dir2에 색 설정 없음 — 코드 경로만 남김).
//! 라벨은 전부 i18n 키 문자열(DR-14 · `Msg` 열거 없음).

use crate::copybtn::CopyBtn;
use ndir_i18n::{tr, trf};
use ndir_settings::{Entry, SettingKind, Settings, CATEGORY_TREE};
use nexa_ctl::controls::{PositionDropdown, Switch};
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::tokens::{hover_alpha, FadeSpeed, IntentFade};
use nexa_ctl::HudPos;
use nexa_ctl::{
    Button, Checkbox, Combo, ComboControl, ComboItem, Control, EditCtxAction, InputEvent,
    Invalidations, Key as CtlKey, LabelSide, ScrollBars, TextBox, TreeControl, TreeModel, TreeNode,
    TreeView, Widget,
};
use nexa_gfx::{Font, Surface};
use std::rc::Rc;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// 창이 호스트에 요청하는 것.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PrefsAction {
    None,
    Paint,
    /// 값 변경(호스트가 정규화·저장·반영).
    Changed {
        key: String,
        value: String,
    },
    /// 기본값으로.
    Reset(String),
    /// 색 창 열기(dir2에 색 설정 없음 — 자리만).
    OpenColors(String),
    OpenKeys,
    /// 폴더 고르기 대화상자(T-29 nexa-dlg 뒤).
    BrowseFolder {
        key: String,
        current: String,
    },
    /// settings.json으로 편집(호스트가 내보내고 열고 감시한다).
    EditJson,
    /// 순서/표시 편집 창(T-71 DLG-069 · `toolbar.layout` / `list.col_layout` / `ctxmenu.layout`).
    EditOrder(String),
}

const PAD: f32 = 12.0;
const TREE_W: f32 = 230.0;
const SPLIT_W: f32 = 6.0;
const LEFT_MIN: f32 = 160.0;
const RIGHT_MIN: f32 = 320.0;
const SEARCH_H: f32 = 30.0;
const CTL_H: f32 = 28.0;
const CARD_GAP: f32 = 10.0;

enum CardCtl {
    Bool(Switch),
    Choice(Box<Combo>),
    Text(Box<TextBox>),
    Pos(Box<PositionDropdown>),
}

struct Card {
    entry: &'static Entry,
    value: String,
    modified: bool,
    ctl: CardCtl,
    reset: Button,
    /// 보조 버튼(지정…/선택…/찾아보기…).
    aux: Option<Button>,
    locked: bool,
    rect: Rect,
    /// 검색 모드에서 카테고리 표시.
    show_cat: bool,
    error: Option<String>,
    default_below: bool,
    copy: CopyBtn,
}

/// 입력란을 카드 폭만큼 넓힐 글자 설정 — 읽기 전용 정보와 경로(셸 명령).
fn wide_text(key: &str) -> bool {
    ndir_settings::is_info(key) || key == "term.shell"
}

#[derive(Clone)]
struct Snap {
    entry: &'static Entry,
    value: String,
    modified: bool,
}

pub(crate) struct PrefsWin {
    window: Option<Rc<Window>>,
    memo: crate::wingeom::Memo,
    last: Option<((i32, i32), (f64, f64))>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    cursor: (i32, i32),
    shift: bool,
    primary: bool,
    search: TextBox,
    tree: TreeView,
    hidden: Vec<&'static str>,
    dyn_choices: std::collections::HashMap<&'static str, Vec<(String, String)>>,
    info: std::collections::HashMap<String, String>,
    notes: std::collections::HashMap<String, String>,
    vtree: Vec<(&'static str, Vec<&'static str>)>,
    advanced: Switch,
    json_btn: Button,
    close_btn: Button,
    cards: Vec<Card>,
    list: Rect,
    scroll: i32,
    content_h: i32,
    bars: ScrollBars,
    snap: Vec<Snap>,
    adv_hidden: usize,
    copy_feedback_ms: i64,
    sel: (usize, Option<usize>),
    last_tree_row: usize,
    query: String,
    left_w: f32,
    split_drag: Option<(i32, f32)>,
    split_fade: IntentFade,
    split_rect: Rect,
    /// 플러그인 페이지(T-63 B · dir2 EXT-129): 호스트가 준 (id, 라벨) 목록 + 로드 오류 줄 · 페이지가 보일 때만 체크박스를 만든다 ·
    /// 해제 = `plugins.disabled`(`|` 구분) — 카드 밖의 **동적 묶음**이라 레지스트리 카드와 따로 둔다.
    plugins: Vec<(String, String)>,
    plugin_notes: Vec<String>,
    plugin_boxes: Vec<Checkbox>,
    plugin_page: bool,
    plugin_rect: Rect,
    plugin_dirty: bool,
}

fn is_color_key(k: &str) -> bool {
    k.ends_with("_color")
}

fn is_key_key(k: &str) -> bool {
    k.starts_with("key.") && k != "key.preset"
}

fn is_folder_key(_k: &str) -> bool {
    false
}

/// 별도 편집 창으로 고치는 순서/표시 값(DLG-069 — 설정 창 필드 3개).
fn is_order_key(k: &str) -> bool {
    matches!(k, "toolbar.layout" | "list.col_layout" | "ctxmenu.layout")
}

impl PrefsWin {
    fn visible_tree(hidden: &[&'static str]) -> Vec<(&'static str, Vec<&'static str>)> {
        CATEGORY_TREE
            .iter()
            .map(|(g, cats)| {
                let kept: Vec<&'static str> = cats
                    .iter()
                    .copied()
                    .filter(|c| !hidden.contains(c))
                    .collect();
                (*g, kept)
            })
            .filter(|(_, cats)| !cats.is_empty())
            .collect()
    }

    fn build_model(tree: &[(&'static str, Vec<&'static str>)]) -> TreeModel {
        let mut roots: Vec<TreeNode> = Vec::new();
        for (g, cats) in tree {
            let kids: Vec<TreeNode> = cats.iter().map(|c| TreeNode::leaf(tr(c))).collect();
            roots.push(TreeNode::branch(tr(g), kids));
        }
        let mut model = TreeModel::new(roots);
        for gi in 0..tree.len() {
            model.set_expanded(&[gi], true);
        }
        model
    }

    #[allow(dead_code)] // 호스트 상황 덧말(셸 탐지 결과 등 · M5)에서 쓴다.
    pub(crate) fn set_note(&mut self, key: &str, note: Option<String>) {
        let changed = match &note {
            Some(n) => self.notes.get(key) != Some(n),
            None => self.notes.contains_key(key),
        };
        match note {
            Some(n) => {
                self.notes.insert(key.to_string(), n);
            }
            None => {
                self.notes.remove(key);
            }
        }
        if changed {
            self.redraw();
        }
    }

    /// 동적 후보 갱신(값, 라벨) — 바뀌었으면 카드를 다시 만든다(언어 목록 · 셸 목록).
    pub(crate) fn set_dyn_choices(&mut self, key: &'static str, items: Vec<(String, String)>) {
        if self.dyn_choices.get(key) == Some(&items) {
            return;
        }
        self.dyn_choices.insert(key, items);
        if !self.cards.is_empty() {
            self.rebuild_cards();
        }
    }

    /// 분류 숨김 갱신(플러그인·클라우드 분류를 기능이 꺼졌을 때) — 트리를 다시 만들고 선택을 보정한다.
    #[allow(dead_code)] // 플러그인·클라우드 분류 숨김(M5/M6)에서 쓴다.
    pub(crate) fn set_hidden_categories(&mut self, hidden: Vec<&'static str>) {
        if self.hidden == hidden {
            return;
        }
        self.hidden = hidden;
        self.vtree = Self::visible_tree(&self.hidden);
        self.tree = TreeView::new(Self::build_model(&self.vtree));
        if self.sel.0 >= self.vtree.len() {
            self.sel = (0, None);
        } else if let Some(ci) = self.sel.1 {
            if ci >= self.vtree[self.sel.0].1.len() {
                self.sel = (self.sel.0, None);
            }
        }
        if self.window.is_some() {
            self.rebuild_cards();
            self.layout();
            self.redraw();
        }
    }

    pub(crate) fn new() -> Self {
        let vtree = Self::visible_tree(&[]);
        let model = Self::build_model(&vtree);
        PrefsWin {
            hidden: Vec::new(),
            dyn_choices: std::collections::HashMap::new(),
            info: std::collections::HashMap::new(),
            notes: std::collections::HashMap::new(),
            vtree,
            window: None,
            memo: crate::wingeom::Memo::default(),
            last: None,
            surface: None,
            scale: 1.0,
            cursor: (0, 0),
            shift: false,
            primary: false,
            search: TextBox::new(tr("pref.search.placeholder")).with_clearable(),
            tree: TreeView::new(model),
            advanced: Switch::new(tr("pref.advanced"), false).with_label_side(LabelSide::Right),
            json_btn: Button::new(tr("pref.btn.json")),
            close_btn: Button::new(tr("pref.btn.close")),
            cards: Vec::new(),
            list: Rect::default(),
            scroll: 0,
            content_h: 0,
            bars: ScrollBars::new(),
            snap: Vec::new(),
            adv_hidden: 0,
            copy_feedback_ms: crate::copybtn::DEFAULT_FEEDBACK_MS as i64,
            sel: (0, Some(0)),
            last_tree_row: 1,
            query: String::new(),
            left_w: TREE_W,
            split_drag: None,
            split_fade: IntentFade::with_speed(FadeSpeed::Fast),
            split_rect: Rect::default(),
            plugins: Vec::new(),
            plugin_notes: Vec::new(),
            plugin_boxes: Vec::new(),
            plugin_page: false,
            plugin_rect: Rect::default(),
            plugin_dirty: false,
        }
    }

    /// 플러그인 목록(id, `이름 (id) — ext…`) + 로드 오류 줄(열 때 · 바뀌면 페이지 재구성).
    pub(crate) fn set_plugins(&mut self, rows: Vec<(String, String)>, notes: Vec<String>) {
        if self.plugins == rows && self.plugin_notes == notes {
            return;
        }
        self.plugins = rows;
        self.plugin_notes = notes;
        if !self.cards.is_empty() || self.plugin_page {
            self.rebuild_cards();
        }
    }

    /// 페이지의 체크 상태(id, 켜짐) — 시험·덤프.
    pub(crate) fn plugin_states(&self) -> Vec<(String, bool)> {
        self.plugins
            .iter()
            .zip(&self.plugin_boxes)
            .map(|((id, _), b)| (id.clone(), b.is_checked()))
            .collect()
    }

    /// 해제된 id를 `|`로(설정 `plugins.disabled` 값).
    fn disabled_value(&self) -> String {
        self.plugins
            .iter()
            .zip(&self.plugin_boxes)
            .filter(|(_, b)| !b.is_checked())
            .map(|((id, _), _)| id.as_str())
            .collect::<Vec<_>>()
            .join("|")
    }

    fn sync_plugin_boxes(&mut self) {
        let disabled = self
            .snap
            .iter()
            .find(|s| s.entry.key == "plugins.disabled")
            .map(|s| s.value.clone())
            .unwrap_or_default();
        for ((id, _), b) in self.plugins.iter().zip(self.plugin_boxes.iter_mut()) {
            b.set_checked(!disabled.split('|').any(|d| d.trim() == id));
        }
    }

    #[cfg(test)]
    pub(crate) fn toggle_plugin(&mut self, i: usize) {
        if let Some(b) = self.plugin_boxes.get_mut(i) {
            let on = b.is_checked();
            b.set_checked(!on);
            self.plugin_dirty = true;
        }
    }

    /// 읽기 전용 정보 값 — [`Self::refresh`] 앞에 부른다.
    #[allow(dead_code)] // 읽기 전용 정보 값(셸·PTY 탐지 · M5)에서 쓴다.
    pub(crate) fn set_info(&mut self, values: Vec<(String, String)>) {
        self.info = values.into_iter().collect();
    }

    /// 레지스트리 스냅샷 갱신(열 때 · 값이 바뀔 때). 카드는 값만 갱신(입력 중인 상자는 건드리지 않음).
    pub(crate) fn refresh(&mut self, s: &Settings) {
        self.snap = s
            .list()
            .into_iter()
            .map(|(e, v, m)| Snap {
                entry: e,
                value: if ndir_settings::is_info(e.key) {
                    self.info.get(e.key).cloned().unwrap_or_default()
                } else if let Some(shown) = self.info.get(e.key).filter(|_| {
                    ndir_settings::dependency(e.key)
                        .is_some_and(|(parent, dep)| !dep.satisfied(s.get(parent).unwrap_or("")))
                }) {
                    shown.clone()
                } else {
                    v.to_string()
                },
                modified: m,
            })
            .collect();
        if self.cards.is_empty() {
            self.rebuild_cards();
            return;
        }
        for c in &mut self.cards {
            let Some(sn) = self.snap.iter().find(|x| x.entry.key == c.entry.key) else {
                continue;
            };
            c.modified = sn.modified;
            if c.value != sn.value {
                c.value = sn.value.clone();
                c.error = None;
                match &mut c.ctl {
                    CardCtl::Bool(sw) => sw.set_on(sn.value == "on"),
                    CardCtl::Choice(cb) => cb.select_value(&sn.value),
                    CardCtl::Pos(pd) => pd.select_value(HudPos::parse(&sn.value).code()),
                    CardCtl::Text(tb) => {
                        if !tb.is_focused() || tb.is_read_only() {
                            tb.set_text(&sn.value);
                            let mut inv = Invalidations::default();
                            tb.select_range(0, 0, &mut inv);
                        }
                    }
                }
            }
        }
        self.sync_plugin_boxes();
        self.apply_deps();
    }

    pub(crate) fn set_error(&mut self, key: &str, e: String) {
        if let Some(c) = self.cards.iter_mut().find(|c| c.entry.key == key) {
            c.error = Some(e);
        }
    }

    /// 지금 보이는 카드의 키 목록(시험 · 덤프).
    pub(crate) fn shown_keys(&self) -> Vec<&'static str> {
        self.cards.iter().map(|c| c.entry.key).collect()
    }

    /// 카드 잠김(종속 조건 불충족) 여부(시험).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn is_locked(&self, key: &str) -> Option<bool> {
        self.cards
            .iter()
            .find(|c| c.entry.key == key)
            .map(|c| c.locked)
    }

    /// 지금 선택/검색에 맞는 카드 목록을 만든다.
    fn rebuild_cards(&mut self) {
        let adv = self.advanced.is_on();
        let q = self.query.trim().to_lowercase();
        let mut chosen: Vec<Snap> = Vec::new();
        let mut adv_hidden = 0usize;
        let mut plugin_page = false;
        if !q.is_empty() {
            for sn in &self.snap {
                if self.hidden.contains(&sn.entry.cat) {
                    continue;
                }
                let hay = format!(
                    "{} {} {} {}",
                    sn.entry.key,
                    tr(sn.entry.label),
                    tr(sn.entry.desc),
                    tr(sn.entry.cat)
                )
                .to_lowercase();
                if hay.contains(&q) {
                    if !adv && ndir_settings::is_advanced(sn.entry.key) {
                        adv_hidden += 1;
                        continue;
                    }
                    chosen.push(sn.clone());
                }
            }
        } else {
            let (gi, ci) = self.sel;
            let cats: Vec<&'static str> = match self.vtree.get(gi) {
                Some((_, cats)) => match ci {
                    Some(c) => cats.get(c).copied().into_iter().collect(),
                    None => cats.clone(),
                },
                None => Vec::new(),
            };
            plugin_page = cats.contains(&"pref.cat.plugins");
            for sn in &self.snap {
                if !cats.contains(&sn.entry.cat) {
                    continue;
                }
                if !adv && ndir_settings::is_advanced(sn.entry.key) {
                    adv_hidden += 1;
                    continue;
                }
                chosen.push(sn.clone());
            }
        }
        chosen.sort_by_key(|s| ndir_settings::display_order(s.entry.key));
        self.adv_hidden = adv_hidden;
        let show_cat = !q.is_empty() || self.sel.1.is_none();
        let dyn_choices = &self.dyn_choices;
        self.cards = chosen
            .into_iter()
            .map(|sn| {
                let dyn_opts = dyn_choices.get(sn.entry.key);
                let ctl = match sn.entry.kind {
                    SettingKind::Text if dyn_opts.is_some_and(|o| !o.is_empty()) => {
                        let opts = dyn_opts.map(Vec::as_slice).unwrap_or_default();
                        let items: Vec<ComboItem> = opts
                            .iter()
                            .map(|(v, l)| ComboItem::new(v.clone(), l.clone()))
                            .collect();
                        let idx = opts.iter().position(|(v, _)| *v == sn.value).unwrap_or(0);
                        CardCtl::Choice(Box::new(Combo::new(items, idx)))
                    }
                    SettingKind::Bool => CardCtl::Bool(
                        Switch::new("", sn.value == "on").with_label_side(LabelSide::None),
                    ),
                    SettingKind::Choice(opts) => {
                        let items: Vec<ComboItem> = opts
                            .iter()
                            .map(|(v, m)| ComboItem::new(*v, tr(m)))
                            .collect();
                        let idx = opts.iter().position(|(v, _)| *v == sn.value).unwrap_or(0);
                        CardCtl::Choice(Box::new(Combo::new(items, idx)))
                    }
                    SettingKind::Position => CardCtl::Pos(Box::new(PositionDropdown::new(
                        HudPos::parse(&sn.value).code(),
                    ))),
                    SettingKind::Int { .. } | SettingKind::Size { .. } | SettingKind::Text => {
                        let mut tb = TextBox::new("").with_text(&sn.value);
                        let mut inv = Invalidations::default();
                        tb.select_range(0, 0, &mut inv);
                        CardCtl::Text(Box::new(tb))
                    }
                };
                let aux = if is_color_key(sn.entry.key) {
                    Some(Button::new(tr("pref.btn.pick")))
                } else if is_key_key(sn.entry.key) {
                    Some(Button::new(tr("pref.btn.capture")))
                } else if is_folder_key(sn.entry.key) {
                    Some(Button::new(tr("pref.btn.browse")))
                } else if is_order_key(sn.entry.key) {
                    Some(Button::new(tr("pref.btn.edit")))
                } else {
                    None
                };
                Card {
                    entry: sn.entry,
                    value: sn.value,
                    modified: sn.modified,
                    ctl,
                    reset: Button::new(tr("pref.btn.reset")),
                    aux,
                    rect: Rect::default(),
                    show_cat,
                    error: None,
                    locked: false,
                    default_below: false,
                    copy: CopyBtn::new(),
                }
            })
            .collect();
        self.plugin_page = plugin_page;
        self.plugin_boxes = if plugin_page {
            self.plugins
                .iter()
                .map(|(_, label)| Checkbox::new(label.clone(), true))
                .collect()
        } else {
            Vec::new()
        };
        self.plugin_rect = Rect::default();
        self.sync_plugin_boxes();
        self.scroll = 0;
        self.content_h = 0;
        self.apply_deps();
    }

    /// 종속 잠금 계산 — 부모 값은 스냅샷(현재 설정)에서.
    fn apply_deps(&mut self) {
        let snap = self.snap.clone();
        let parent_val = |key: &str| -> String {
            snap.iter()
                .find(|s| s.entry.key == key)
                .map(|s| s.value.clone())
                .unwrap_or_default()
        };
        for c in &mut self.cards {
            c.locked = ndir_settings::dependency(c.entry.key)
                .is_some_and(|(parent, dep)| !dep.satisfied(&parent_val(parent)))
                || ndir_settings::is_info(c.entry.key);
            match &mut c.ctl {
                CardCtl::Text(tb) => tb.set_read_only(c.locked),
                CardCtl::Choice(cb) if c.locked => cb.set_focused(false),
                CardCtl::Pos(pd) if c.locked => pd.set_focused(false),
                _ => {}
            }
        }
    }

    #[allow(dead_code)] // 이 창 위의 대화상자 주인(T-29).
    pub(crate) fn window_rc(&self) -> Option<Rc<Window>> {
        self.window.clone()
    }

    #[allow(dead_code)] // 모달 소유 창 목록(T-29).
    pub(crate) fn window(&self) -> Option<&Window> {
        self.window.as_deref()
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

    pub(crate) fn tick(&mut self, now_ms: u64) -> bool {
        if self.window.is_none() {
            return false;
        }
        let mut any = self.bars.tick(now_ms)
            | self.split_fade.tick(now_ms)
            | self.tree.tick(now_ms)
            | self.search.tick(now_ms)
            | self.close_btn.tick(now_ms)
            | self.json_btn.tick(now_ms);
        let now = std::time::Instant::now();
        for c in &mut self.cards {
            any |= c.reset.tick(now_ms);
            any |= c.copy.next_tick(now).is_some();
            if let Some(b) = &mut c.aux {
                any |= b.tick(now_ms);
            }
            match &mut c.ctl {
                CardCtl::Bool(_) | CardCtl::Pos(_) => {}
                CardCtl::Choice(cb) => any |= cb.tick_hover(now_ms),
                CardCtl::Text(tb) => any |= tb.tick(now_ms),
            }
        }
        any
    }

    pub(crate) fn animating(&self) -> bool {
        self.window.is_some()
            && (self.bars.is_visible()
                || self.split_fade.is_animating()
                || self.search.is_animating()
                || self.close_btn.is_animating()
                || self.json_btn.is_animating()
                || self.cards.iter().any(|c| {
                    c.reset.is_animating()
                        || c.copy.next_tick(std::time::Instant::now()).is_some()
                        || c.aux.as_ref().is_some_and(|b| b.is_animating())
                        || match &c.ctl {
                            CardCtl::Bool(_) | CardCtl::Pos(_) => false,
                            CardCtl::Choice(cb) => cb.hover_animating(),
                            CardCtl::Text(tb) => tb.is_animating(),
                        }
                }))
    }

    /// 검색어를 미리 넣는다(기동 명령 `prefs.search:<검색어>` — 자체 캡처용).
    pub(crate) fn preset_query(&mut self, q: &str) {
        self.search.set_text(q);
        let _ = self.search.take_changed();
        self.query = q.to_string();
        self.last_tree_row = self.tree.selected_row();
        self.rebuild_cards();
    }

    /// 분류 하나를 골라 연다(기동 명령 `prefs.cat:<i18n 키>`): 그룹을 펼치고 트리 행을 선택 · 검색어는 비운다.
    pub(crate) fn select_category(&mut self, cat: &str) {
        let Some((gi, ci)) = self
            .vtree
            .iter()
            .enumerate()
            .find_map(|(gi, (_, cats))| cats.iter().position(|c| *c == cat).map(|ci| (gi, ci)))
        else {
            return;
        };
        self.tree.model_mut().set_expanded(&[gi], true);
        let rows = self.tree.model().flatten();
        if let Some(row) = rows
            .iter()
            .position(|r| r.path.first() == Some(&gi) && r.path.get(1) == Some(&ci))
        {
            self.tree.set_selected_row(row);
            self.last_tree_row = row;
        }
        self.sel = (gi, Some(ci));
        self.query.clear();
        self.search.set_text("");
        let _ = self.search.take_changed();
        self.rebuild_cards();
        self.layout();
    }

    /// "고급" 스위치 상태(설정 `ui.prefs_advanced`) — 열기 전에 호스트가 넣는다.
    pub(crate) fn set_advanced(&mut self, on: bool) {
        if self.advanced.is_on() != on {
            self.advanced.set_on(on);
            if !self.cards.is_empty() {
                self.rebuild_cards();
            }
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
            return;
        }
        let same = self.memo.on_same_monitor(owner);
        let (lw, lh) = same.and_then(|(_, s)| s).unwrap_or((920.0, 640.0));
        let mut attrs = Window::default_attributes()
            .with_title(format!("Nexa Dir — {}", tr("pref.title")))
            .with_theme(theme)
            .with_resizable(true)
            .with_inner_size(winit::dpi::LogicalSize::new(lw, lh));
        if let Some(((x, y), _)) = same {
            attrs = attrs.with_position(crate::wingeom::logical(x, y));
        } else if let Some((x, y, w, h)) = over {
            let cx = x + (w as i32 - lw as i32) / 2;
            let cy = y + (h as i32 - lh as i32) / 2;
            attrs = attrs.with_position(winit::dpi::PhysicalPosition::new(cx.max(0), cy.max(0)));
        }
        let attrs = crate::winfocus::owned_by(crate::icon::with_icon(attrs), owner);
        let Ok(win) = el.create_window(attrs) else {
            return;
        };
        if let Some(((x, y), _)) = same {
            crate::wingeom::place_outer(&win, Some((x, y)));
        }
        let win = Rc::new(win);
        self.scale = win.scale_factor() as f32;
        self.surface = crate::present::Presenter::new(win.clone()).ok();
        win.set_ime_allowed(crate::input::system_ime());
        self.window = Some(win);
        self.rebuild_cards();
        self.layout();
        self.redraw();
    }

    pub(crate) fn close(&mut self) {
        if let Some(w) = &self.window {
            if let Some(p) = crate::wingeom::outer_pos(w) {
                self.last = Some((p, crate::wingeom::logical_size(w)));
            }
        }
        self.surface = None;
        self.window = None;
        self.cards.clear();
    }

    pub(crate) fn set_memo(&mut self, m: crate::wingeom::Memo) {
        self.memo = m;
    }

    pub(crate) fn take_last(&mut self) -> Option<((i32, i32), (f64, f64))> {
        self.last.take()
    }

    fn s(&self, v: f32) -> i32 {
        (v * self.scale).round() as i32
    }

    fn layout(&mut self) {
        let Some(win) = &self.window else { return };
        let size = win.inner_size();
        let (w, h) = (size.width as i32, size.height as i32);
        let s = self.scale;
        let pad = self.s(PAD);
        let mut inv = Invalidations::default();
        let bottom_h = self.s(CTL_H);
        let by = h - pad - bottom_h;
        let max_left = ((w - pad * 2 - self.s(SPLIT_W)) as f32 / s - RIGHT_MIN).max(LEFT_MIN);
        self.left_w = self.left_w.clamp(LEFT_MIN, max_left.max(LEFT_MIN));
        let lw = self.s(self.left_w);
        let top = pad;
        self.search.set_scale(s);
        self.search
            .set_bounds(Rect::new(pad, top, lw, self.s(SEARCH_H)), &mut inv);
        let tree_top = top + self.s(SEARCH_H) + self.s(8.0);
        self.tree.set_scale(s);
        self.tree
            .set_bounds(Rect::new(pad, tree_top, lw, by - pad - tree_top), &mut inv);
        self.split_rect = Rect::new(pad + lw, top, self.s(SPLIT_W), by - pad - top);
        let lx = self.split_rect.right() + pad / 2;
        self.list = Rect::new(lx, top, w - lx - pad, by - pad - top);
        self.advanced.set_scale(s);
        self.advanced
            .set_bounds(Rect::new(pad, by, self.s(160.0), bottom_h), &mut inv);
        self.json_btn.set_scale(s);
        self.json_btn.set_bounds(
            Rect::new(
                w - pad - self.s(96.0) * 2 - self.s(8.0),
                by,
                self.s(96.0),
                bottom_h,
            ),
            &mut inv,
        );
        self.close_btn.set_scale(s);
        self.close_btn.set_bounds(
            Rect::new(w - pad - self.s(96.0), by, self.s(96.0), bottom_h),
            &mut inv,
        );
        self.clamp_scroll();
    }

    fn clamp_scroll(&mut self) {
        let max = (self.content_h - self.list.h).max(0);
        self.scroll = self.scroll.clamp(0, max);
    }

    fn to_input(&self, ev: &WindowEvent) -> Option<InputEvent> {
        let (x, y) = self.cursor;
        let key = |k: CtlKey| InputEvent::Key {
            key: k,
            shift: self.shift,
            primary: false,
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
                    primary: false,
                },
                (ElementState::Released, MouseButton::Left) => InputEvent::MouseUp { x, y },
                (ElementState::Pressed, MouseButton::Right) => InputEvent::RightDown { x, y },
                _ => return None,
            },
            WindowEvent::MouseWheel { delta, .. } => crate::input::wheel_event(delta, self.shift),
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                match kev.logical_key.as_ref() {
                    Key::Named(NamedKey::Enter) => key(CtlKey::Enter),
                    Key::Named(NamedKey::ArrowLeft) => key(CtlKey::Left),
                    Key::Named(NamedKey::ArrowRight) => key(CtlKey::Right),
                    Key::Named(NamedKey::ArrowUp) => key(CtlKey::Up),
                    Key::Named(NamedKey::ArrowDown) => key(CtlKey::Down),
                    Key::Named(NamedKey::Home) => key(CtlKey::Home),
                    Key::Named(NamedKey::End) => key(CtlKey::End),
                    Key::Named(NamedKey::Delete) => key(CtlKey::Delete),
                    Key::Named(NamedKey::Backspace) => InputEvent::Char {
                        c: '\u{8}',
                        now_ms: 0,
                    },
                    Key::Named(NamedKey::Space) => InputEvent::Char { c: ' ', now_ms: 0 },
                    Key::Character(t) => {
                        if self.primary {
                            return None;
                        }
                        let c = t.chars().next()?;
                        if c.is_control() {
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

    /// 포커스 텍스트박스(IME).
    fn focused_textbox(&mut self) -> Option<&mut TextBox> {
        if self.search.is_focused() {
            return Some(&mut self.search);
        }
        for c in &mut self.cards {
            if let CardCtl::Text(tb) = &mut c.ctl {
                if tb.is_focused() {
                    return Some(&mut **tb);
                }
            }
        }
        None
    }

    fn any_combo_open(&self) -> bool {
        self.cards.iter().any(|c| match &c.ctl {
            CardCtl::Choice(cb) => cb.is_open(),
            CardCtl::Pos(pd) => pd.is_open(),
            _ => false,
        })
    }

    pub(crate) fn handle(&mut self, ev: &WindowEvent) -> PrefsAction {
        match ev {
            WindowEvent::CloseRequested => {
                self.close();
                return PrefsAction::None;
            }
            WindowEvent::Resized(_) => {
                self.layout();
                self.redraw();
                return PrefsAction::None;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.layout();
                self.redraw();
                return PrefsAction::None;
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.primary = if cfg!(target_os = "macos") {
                    m.state().super_key()
                } else {
                    m.state().control_key()
                };
                return PrefsAction::None;
            }
            WindowEvent::KeyboardInput { event: kev, .. }
                if kev.state == ElementState::Pressed
                    && self.primary
                    && matches!(kev.logical_key.as_ref(), Key::Character(_)) =>
            {
                let c = crate::input::shortcut_letter(kev)
                    .map(|c| c.to_string())
                    .unwrap_or_default();
                let mut inv = Invalidations::default();
                match c.as_str() {
                    "c" => return self.clip(EditCtxAction::Copy),
                    "x" => return self.clip(EditCtxAction::Cut),
                    "v" => return self.clip(EditCtxAction::Paste),
                    "a" => {
                        if let Some(tb) = self.focused_textbox() {
                            tb.on_event(&InputEvent::SelectAll, &mut inv);
                        }
                        self.redraw();
                        return PrefsAction::None;
                    }
                    "z" if !self.shift => {
                        if let Some(tb) = self.focused_textbox() {
                            tb.on_event(&InputEvent::Undo, &mut inv);
                        }
                        return self.after_edit();
                    }
                    "z" | "y" => {
                        if let Some(tb) = self.focused_textbox() {
                            tb.on_event(&InputEvent::Redo, &mut inv);
                        }
                        return self.after_edit();
                    }
                    _ => return PrefsAction::None,
                }
            }
            WindowEvent::Ime(ime) => {
                let mut inv = Invalidations::default();
                if let Some(tb) = self.focused_textbox() {
                    match ime {
                        winit::event::Ime::Preedit(t, _) => tb.set_preedit(t, &mut inv),
                        winit::event::Ime::Commit(t) => {
                            tb.set_preedit("", &mut inv);
                            for c in t.chars().filter(|c| !c.is_control()) {
                                tb.on_event(&InputEvent::Char { c, now_ms: 0 }, &mut inv);
                            }
                        }
                        _ => {}
                    }
                    self.redraw();
                }
                let _ = self.search.take_changed();
                if self.search.is_focused() {
                    let q = self.search.display_text();
                    if q != self.query {
                        self.query = q;
                        self.rebuild_cards();
                        self.redraw();
                    }
                }
                return self.after_edit();
            }
            WindowEvent::KeyboardInput { event: kev, .. }
                if kev.state == ElementState::Pressed
                    && matches!(kev.logical_key.as_ref(), Key::Named(NamedKey::Escape)) =>
            {
                if !self.any_combo_open() {
                    self.close();
                    return PrefsAction::None;
                }
            }
            WindowEvent::RedrawRequested => return PrefsAction::Paint,
            _ => {}
        }
        if let WindowEvent::CursorMoved { position, .. } = ev {
            self.cursor = (position.x as i32, position.y as i32);
        }
        let Some(ie) = self.to_input(ev) else {
            return PrefsAction::None;
        };
        let mut inv = Invalidations::default();
        if matches!(ie, InputEvent::RightDown { .. }) {
            let has = crate::clipboard::read_text().is_some_and(|s| !s.is_empty());
            for c in &mut self.cards {
                if let CardCtl::Text(tb) = &mut c.ctl {
                    tb.set_clipboard_has_text(has);
                }
            }
            self.search.set_clipboard_has_text(has);
        }
        let outside_click = matches!(
            ie,
            InputEvent::MouseDown { .. } | InputEvent::RightDown { .. }
        );
        if let Some(tb) = self.popup_textbox() {
            tb.on_event(&ie, &mut inv);
            let act = tb.take_edit_ctx();
            let still_open = tb.popup_open();
            if let Some(act) = act {
                return self.clip(act);
            }
            if still_open || !outside_click {
                self.redraw();
                return PrefsAction::None;
            }
        }
        match ie {
            InputEvent::MouseMove { x, y } => {
                if let Some((x0, w0)) = self.split_drag {
                    self.left_w = w0 + (x - x0) as f32 / self.scale;
                    self.layout();
                    self.redraw();
                    return PrefsAction::None;
                }
                let over = self.split_rect.contains(Point { x, y }) && !self.any_combo_open();
                self.split_fade.set(over.then_some(0));
                if let Some(w) = &self.window {
                    w.set_cursor(if over {
                        winit::window::CursorIcon::ColResize
                    } else {
                        winit::window::CursorIcon::Default
                    });
                }
            }
            InputEvent::MouseDown { x, y, .. }
                if self.split_rect.contains(Point { x, y }) && !self.any_combo_open() =>
            {
                self.split_drag = Some((x, self.left_w));
                self.split_fade.jump(Some(0));
                return PrefsAction::None;
            }
            InputEvent::MouseUp { .. } if self.split_drag.is_some() => {
                self.split_drag = None;
                self.redraw();
                return PrefsAction::None;
            }
            _ => {}
        }
        if self.any_combo_open() {
            let mut on_head = false;
            for c in &mut self.cards {
                match &mut c.ctl {
                    CardCtl::Choice(cb) if cb.is_open() => {
                        if let InputEvent::MouseDown { x, y, .. } = ie {
                            on_head |= cb.bounds().contains(Point { x, y });
                        }
                        cb.on_event(&ie, &mut inv);
                    }
                    CardCtl::Pos(pd) if pd.is_open() => {
                        if let InputEvent::MouseDown { x, y, .. } = ie {
                            on_head |= pd.bounds().contains(Point { x, y });
                        }
                        pd.on_event(&ie, &mut inv);
                    }
                    _ => {}
                }
            }
            let a = self.collect_changes();
            self.redraw();
            if !matches!(a, PrefsAction::None)
                || !matches!(ie, InputEvent::MouseDown { .. })
                || self.any_combo_open()
                || on_head
            {
                return a;
            }
        }
        let p = Point {
            x: self.cursor.0,
            y: self.cursor.1,
        };
        let is_mouse = matches!(
            ie,
            InputEvent::MouseDown { .. }
                | InputEvent::MouseUp { .. }
                | InputEvent::MouseMove { .. }
        );
        if let InputEvent::MouseDown { .. } = ie {
            let in_search = self.search.bounds().contains(p);
            self.search.set_focused(in_search);
            self.tree.set_focused(self.tree.bounds().contains(p));
            self.close_btn
                .set_focused(self.close_btn.bounds().contains(p));
            self.json_btn
                .set_focused(self.json_btn.bounds().contains(p));
            for c in &mut self.cards {
                let open = !c.locked;
                match &mut c.ctl {
                    CardCtl::Text(tb) => tb.set_focused(tb.bounds().contains(p)),
                    CardCtl::Choice(cb) => cb.set_focused(open && cb.bounds().contains(p)),
                    CardCtl::Pos(pd) => pd.set_focused(open && pd.bounds().contains(p)),
                    CardCtl::Bool(sw) => sw.set_focused(false),
                }
                c.reset.set_focused(c.reset.bounds().contains(p));
                if let Some(b) = &mut c.aux {
                    b.set_focused(b.bounds().contains(p));
                }
            }
        }
        let is_wheel = matches!(ie, InputEvent::Wheel { .. } | InputEvent::HWheel { .. });
        if self.list.contains(p) || (!is_mouse && !is_wheel) {
            let (_, ny, consumed) = self.bars.on_event(
                &ie,
                self.list,
                self.list.w,
                self.content_h.max(self.list.h),
                0,
                self.scroll,
                self.scale,
            );
            if ny != self.scroll {
                self.scroll = ny;
                self.clamp_scroll();
            }
            if consumed || is_wheel {
                self.redraw();
                return PrefsAction::None;
            }
        }
        let route = |r: Rect, focused: bool| -> bool {
            if is_mouse || is_wheel {
                r.contains(p)
            } else {
                focused
            }
        };
        if route(self.search.bounds(), self.search.is_focused()) {
            self.search.on_event(&ie, &mut inv);
            let _ = self.search.take_committed();
        }
        if route(self.tree.bounds(), self.tree.is_focused()) {
            self.tree.on_event(&ie, &mut inv);
        }
        if route(self.advanced.bounds(), false) {
            self.advanced.on_event(&ie, &mut inv);
        }
        if route(self.close_btn.bounds(), self.close_btn.is_focused())
            || matches!(ie, InputEvent::MouseMove { .. })
        {
            self.close_btn.on_event(&ie, &mut inv);
        }
        if route(self.json_btn.bounds(), self.json_btn.is_focused())
            || matches!(ie, InputEvent::MouseMove { .. })
        {
            self.json_btn.on_event(&ie, &mut inv);
        }
        if let Some(q) = self.search.take_changed() {
            self.query = q;
            self.rebuild_cards();
            self.redraw();
            return PrefsAction::None;
        }
        if self.advanced.take_toggled().is_some() {
            self.rebuild_cards();
            self.redraw();
            return PrefsAction::Changed {
                key: "ui.prefs_advanced".into(),
                value: if self.advanced.is_on() { "on" } else { "off" }.into(),
            };
        }
        if self.close_btn.take_clicked() {
            self.close();
            return PrefsAction::None;
        }
        if self.json_btn.take_clicked() {
            return PrefsAction::EditJson;
        }
        let row = self.tree.selected_row();
        if row != self.last_tree_row {
            self.last_tree_row = row;
            let rows = self.tree.model().flatten();
            if let Some(r) = rows.get(row) {
                let gi = r.path.first().copied().unwrap_or(0);
                let ci = r.path.get(1).copied();
                self.sel = (gi, ci);
                if !self.query.is_empty() {
                    self.query.clear();
                    self.search.set_text("");
                }
                self.rebuild_cards();
                self.layout();
                self.redraw();
                return PrefsAction::None;
            }
        }
        let drag_follow = matches!(
            ie,
            InputEvent::MouseMove { .. } | InputEvent::MouseUp { .. }
        ) && self
            .cards
            .iter()
            .any(|c| matches!(&c.ctl, CardCtl::Text(tb) if tb.is_focused()));
        if self.list.contains(p) || !is_mouse || drag_follow {
            for c in &mut self.cards {
                if c.rect.h == 0 {
                    continue;
                }
                if !c.locked {
                    match &mut c.ctl {
                        CardCtl::Bool(sw) => sw.on_event(&ie, &mut inv),
                        CardCtl::Choice(cb) => cb.on_event(&ie, &mut inv),
                        CardCtl::Pos(pd) => pd.on_event(&ie, &mut inv),
                        CardCtl::Text(tb) => tb.on_event(&ie, &mut inv),
                    }
                } else if let CardCtl::Text(tb) = &mut c.ctl {
                    tb.on_event(&ie, &mut inv);
                }
                c.reset.on_event(&ie, &mut inv);
                if let Some(b) = &mut c.aux {
                    b.on_event(&ie, &mut inv);
                }
            }
            for b in &mut self.plugin_boxes {
                b.on_event(&ie, &mut inv);
            }
        }
        match ie {
            InputEvent::MouseMove { .. } => {
                for c in &mut self.cards {
                    c.copy.set_hover(c.rect.h > 0 && c.copy.hit(p));
                }
            }
            InputEvent::MouseDown { shift, .. } if self.list.contains(p) => {
                let all = shift || self.primary;
                if let Some(i) = self
                    .cards
                    .iter()
                    .position(|c| c.rect.h > 0 && c.copy.hit(p))
                {
                    let text = if all {
                        self.shown_as_text()
                    } else {
                        self.cards[i].entry.key.to_string()
                    };
                    if crate::clipboard::write_text(&text) {
                        let ms = self.copy_feedback_ms;
                        let b = &mut self.cards[i].copy;
                        b.set_feedback_ms(ms);
                        b.press(std::time::Instant::now());
                    }
                    self.redraw();
                    return PrefsAction::None;
                }
            }
            _ => {}
        }
        let a = self.collect_changes();
        self.redraw();
        a
    }

    /// 지금 보이는 설정 전부를 글로(`# 카테고리 › 라벨` 줄 + `키=값` 줄 · settings.conf와 같은 형식).
    fn shown_as_text(&self) -> String {
        let mut out = String::new();
        for c in &self.cards {
            out.push_str(&format!(
                "# {} › {}\n{}={}\n",
                tr(c.entry.cat),
                tr(c.entry.label),
                c.entry.key,
                c.value
            ));
        }
        out
    }

    fn after_edit(&mut self) -> PrefsAction {
        self.collect_changes()
    }

    /// 클립보드 행동(Ctrl+C/X/V · 입력란 우클릭 편집 메뉴).
    fn clip(&mut self, act: EditCtxAction) -> PrefsAction {
        let mut inv = Invalidations::default();
        match act {
            EditCtxAction::Copy => {
                if let Some(text) = self.focused_textbox().and_then(|tb| tb.copy_selection()) {
                    let _ = crate::clipboard::write_text(&text);
                }
                self.redraw();
                PrefsAction::None
            }
            EditCtxAction::Cut => {
                if let Some(text) = self
                    .focused_textbox()
                    .and_then(|tb| tb.cut_selection(&mut inv))
                {
                    let _ = crate::clipboard::write_text(&text);
                }
                self.redraw();
                self.after_edit()
            }
            EditCtxAction::Paste => {
                if let Some(text) = crate::clipboard::read_text() {
                    if let Some(tb) = self.focused_textbox() {
                        tb.paste(&text, &mut inv);
                    }
                }
                self.redraw();
                self.after_edit()
            }
            EditCtxAction::Custom(_) => PrefsAction::None,
        }
    }

    fn popup_textbox(&mut self) -> Option<&mut TextBox> {
        if self.search.popup_open() {
            return Some(&mut self.search);
        }
        for c in &mut self.cards {
            if let CardCtl::Text(tb) = &mut c.ctl {
                if tb.popup_open() {
                    return Some(&mut **tb);
                }
            }
        }
        None
    }

    /// 컨트롤 변화 수거 → 첫 변경만 보고(한 이벤트에 하나).
    pub(crate) fn collect_changes(&mut self) -> PrefsAction {
        let mut toggled = std::mem::take(&mut self.plugin_dirty);
        for b in &mut self.plugin_boxes {
            if b.take_toggled().is_some() {
                toggled = true;
            }
        }
        if toggled {
            return PrefsAction::Changed {
                key: "plugins.disabled".into(),
                value: self.disabled_value(),
            };
        }
        for c in &mut self.cards {
            let key = c.entry.key.to_string();
            if c.reset.take_clicked() {
                return PrefsAction::Reset(key);
            }
            if let Some(b) = &mut c.aux {
                if b.take_clicked() {
                    return if is_color_key(&key) {
                        PrefsAction::OpenColors(key)
                    } else if is_folder_key(&key) {
                        if c.locked {
                            continue;
                        }
                        PrefsAction::BrowseFolder {
                            current: c.value.clone(),
                            key,
                        }
                    } else if is_order_key(&key) {
                        PrefsAction::EditOrder(key)
                    } else {
                        PrefsAction::OpenKeys
                    };
                }
            }
            if c.locked {
                match &mut c.ctl {
                    CardCtl::Bool(sw) => {
                        let _ = sw.take_toggled();
                    }
                    CardCtl::Choice(cb) => {
                        let _ = cb.take_changed();
                    }
                    CardCtl::Pos(pd) => {
                        let _ = pd.take_changed();
                    }
                    CardCtl::Text(tb) => {
                        let _ = tb.take_changed();
                    }
                }
                continue;
            }
            match &mut c.ctl {
                CardCtl::Bool(sw) => {
                    if let Some(on) = sw.take_toggled() {
                        return PrefsAction::Changed {
                            key,
                            value: if on { "on".into() } else { "off".into() },
                        };
                    }
                }
                CardCtl::Choice(cb) => {
                    if let Some(v) = cb.take_changed() {
                        return PrefsAction::Changed { key, value: v };
                    }
                }
                CardCtl::Pos(pd) => {
                    if let Some(code) = pd.take_changed() {
                        return PrefsAction::Changed {
                            key,
                            value: HudPos::from_code(&code).as_str().to_string(),
                        };
                    }
                }
                CardCtl::Text(tb) => {
                    if let Some(v) = tb.take_changed() {
                        if ndir_settings::normalize(c.entry.kind, &v).is_some()
                            || (v.trim().is_empty() && c.entry.default.is_empty())
                        {
                            c.error = None;
                            return PrefsAction::Changed { key, value: v };
                        }
                        c.error = Some(ndir_settings::allowed(c.entry.kind));
                    }
                }
            }
        }
        PrefsAction::None
    }

    /// 단어 단위 줄바꿈(설명).
    fn wrap(dc: &mut dyn DrawCtx, text: &str, w: i32) -> Vec<String> {
        let mut lines = Vec::new();
        for para in text.split('\n') {
            let mut cur = String::new();
            for word in para.split_whitespace() {
                let cand = if cur.is_empty() {
                    word.to_string()
                } else {
                    format!("{cur} {word}")
                };
                if dc.text_width(&cand) <= w || cur.is_empty() {
                    cur = cand;
                } else {
                    lines.push(std::mem::replace(&mut cur, word.to_string()));
                }
            }
            lines.push(cur);
        }
        lines
    }

    pub(crate) fn paint(&mut self, ui: &Font, th: &Theme, font_px: f32) {
        let (Some(win), Some(surface)) = (self.window.clone(), self.surface.as_mut()) else {
            return;
        };
        let size = win.inner_size();
        let Some(mut buf) = surface.frame(size) else {
            return;
        };
        let s = self.scale;
        let (wi, hi) = (size.width as i32, size.height as i32);
        let list = self.list;
        {
            let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
            let prefs = FontPrefs::with_base(font_px);
            let mut dc = RasterCtx::new(&mut gfx, ui, s).with_fonts(prefs);
            dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
            dc.select_font(FontSlot::Base, false);
            let th_txt = dc.text_height();
            let card_w = list.w - (18.0 * s).round() as i32;
            let inner_pad = (10.0 * s).round() as i32;
            let ctl_h = (CTL_H * s).round() as i32;
            let gap = (CARD_GAP * s).round() as i32;
            let text_w = card_w - inner_pad * 2 - (120.0 * s).round() as i32;
            let mut y = list.y + gap - self.scroll;
            let note_y = y;
            if self.adv_hidden > 0 {
                y += th_txt + gap;
            }
            let mut inv = Invalidations::default();
            let mut desc_lines: Vec<(Vec<String>, usize)> = Vec::with_capacity(self.cards.len());
            // 플러그인 페이지 블록(T-63 B): 제목 · 설명(없으면 안내) · 체크박스 n줄 · 로드 오류 줄 — 카드와 같은 모양으로 카드 앞에.
            let mut plugin_lines: Vec<(String, bool)> = Vec::new();
            if self.plugin_page {
                for l in Self::wrap(&mut dc, &tr("pref.plugins.desc"), text_w) {
                    plugin_lines.push((l, false));
                }
                if self.plugins.is_empty() {
                    for l in Self::wrap(&mut dc, &tr("pref.plugins.empty"), text_w) {
                        plugin_lines.push((l, false));
                    }
                }
                for n in &self.plugin_notes {
                    for l in Self::wrap(&mut dc, n, text_w) {
                        plugin_lines.push((l, true));
                    }
                }
                let rows = self.plugin_boxes.len() as i32;
                let rows_h = if rows > 0 {
                    (8.0 * s).round() as i32 + rows * ctl_h
                } else {
                    0
                };
                let ph = inner_pad * 2
                    + th_txt
                    + (4.0 * s).round() as i32
                    + th_txt * plugin_lines.len() as i32
                    + rows_h;
                let visible = y + ph > list.y && y < list.bottom();
                self.plugin_rect = Rect::new(list.x, y, card_w, if visible { ph } else { 0 });
                let desc_n = plugin_lines.iter().filter(|(_, warn)| !warn).count() as i32;
                let mut by = y
                    + inner_pad
                    + th_txt
                    + (4.0 * s).round() as i32
                    + th_txt * desc_n
                    + (8.0 * s).round() as i32;
                for b in &mut self.plugin_boxes {
                    b.set_scale(s);
                    let r = Rect::new(list.x + inner_pad, by, card_w - inner_pad * 2, ctl_h);
                    let shown = visible && by >= list.y && by + ctl_h <= list.bottom();
                    b.set_bounds(if shown { r } else { Rect::default() }, &mut inv);
                    by += ctl_h;
                }
                y += ph + gap;
            }
            for c in &mut self.cards {
                let note = self
                    .notes
                    .get(c.entry.key)
                    .or_else(|| self.info.get(&format!("{}#note", c.entry.key)));
                let desc = if c.entry.desc.is_empty() {
                    String::new()
                } else {
                    tr(c.entry.desc)
                };
                let mut lines = Self::wrap(&mut dc, &desc, text_w);
                let note_at = lines.len();
                if let Some(n) = note.filter(|n| !n.is_empty()) {
                    lines.extend(Self::wrap(&mut dc, n, text_w));
                }
                let mut extra = if c.error.is_some() { th_txt } else { 0 };
                let dw = dc.text_width(&trf("pref.defaultValue", &[c.entry.default]));
                let ctl_w_of = |c: &Card| match &c.ctl {
                    CardCtl::Bool(_) => (56.0 * s).round() as i32,
                    CardCtl::Choice(_) => (260.0 * s).round() as i32,
                    CardCtl::Pos(_) => (64.0 * s).round() as i32,
                    CardCtl::Text(_) if wide_text(c.entry.key) => {
                        let side = (98.0 * s).round() as i32
                            + if c.aux.is_some() {
                                (98.0 * s).round() as i32
                            } else {
                                0
                            };
                        (card_w - inner_pad * 2 - side).max((320.0 * s).round() as i32)
                    }
                    CardCtl::Text(_) => (320.0 * s).round() as i32,
                };
                let row_used = {
                    let aux_w = if c.aux.is_some() {
                        (90.0 * s).round() as i32 + (8.0 * s).round() as i32
                    } else {
                        0
                    };
                    inner_pad * 2
                        + ctl_w_of(c)
                        + (8.0 * s).round() as i32
                        + aux_w
                        + (90.0 * s).round() as i32
                };
                c.default_below = !c.entry.default.is_empty()
                    && dw + (16.0 * s).round() as i32 > card_w - row_used;
                if c.default_below {
                    extra += th_txt;
                }
                let ch = inner_pad * 2
                    + th_txt
                    + (4.0 * s).round() as i32
                    + th_txt * lines.len() as i32
                    + (8.0 * s).round() as i32
                    + ctl_h
                    + extra;
                c.rect = Rect::new(list.x, y, card_w, ch);
                let cb = Rect::new(
                    list.x + card_w - inner_pad - th_txt,
                    y + inner_pad,
                    th_txt,
                    th_txt,
                );
                c.copy
                    .set_rect(if y + inner_pad >= list.y && cb.bottom() <= list.bottom() {
                        cb
                    } else {
                        Rect::default()
                    });
                let cy = y + ch - inner_pad - ctl_h;
                let mut cx = list.x + inner_pad;
                let ctl_w = ctl_w_of(c);
                let visible = y + ch > list.y && y < list.bottom();
                let ctl_visible = visible && cy >= list.y && cy + ctl_h <= list.bottom();
                let hidden = Rect::default();
                let r = if ctl_visible {
                    Rect::new(cx, cy, ctl_w, ctl_h)
                } else {
                    hidden
                };
                match &mut c.ctl {
                    CardCtl::Bool(sw) => {
                        sw.set_scale(s);
                        sw.set_bounds(r, &mut inv);
                    }
                    CardCtl::Choice(cb) => {
                        cb.set_scale(s);
                        cb.set_bounds(r, &mut inv);
                    }
                    CardCtl::Pos(pd) => {
                        pd.set_scale(s);
                        pd.set_max_bottom(list.bottom());
                        pd.set_bounds(r, &mut inv);
                    }
                    CardCtl::Text(tb) => {
                        tb.set_scale(s);
                        tb.set_bounds(r, &mut inv);
                    }
                }
                cx += ctl_w + (8.0 * s).round() as i32;
                if let Some(b) = &mut c.aux {
                    b.set_scale(s);
                    let bw = (90.0 * s).round() as i32;
                    b.set_bounds(
                        if ctl_visible {
                            Rect::new(cx, cy, bw, ctl_h)
                        } else {
                            hidden
                        },
                        &mut inv,
                    );
                    cx += bw + (8.0 * s).round() as i32;
                    if is_color_key(c.entry.key) {
                        cx += ctl_h + (8.0 * s).round() as i32;
                    }
                }
                c.reset.set_scale(s);
                let rw = (90.0 * s).round() as i32;
                c.reset.set_bounds(
                    if ctl_visible && c.modified {
                        Rect::new(cx, cy, rw, ctl_h)
                    } else {
                        hidden
                    },
                    &mut inv,
                );
                if !visible {
                    c.rect.h = 0;
                }
                desc_lines.push((lines, note_at));
                y += ch + gap;
            }
            self.content_h = y + self.scroll - list.y;
            self.search.paint(&mut dc, th);
            self.tree.paint(&mut dc, th);
            let sr = self.split_rect;
            let cx = sr.x + sr.w / 2;
            dc.fill_rect(Rect::new(cx, sr.y, 1, sr.h), th.border);
            let a = hover_alpha(self.split_drag.is_some(), self.split_fade.value(0));
            if a > 0.0 {
                dc.fill_rect_alpha(Rect::new(cx - 1, sr.y, 3, sr.h), th.accent, a.max(0.15));
            }
            dc.fill_rect(list, th.window_bg);
            if self.adv_hidden > 0 {
                let nr = Rect::new(list.x, note_y, card_w, th_txt).intersection(&list);
                if nr.h > 0 {
                    dc.text(
                        list.x + inner_pad,
                        note_y,
                        nr,
                        &trf("pref.advancedHidden", &[&self.adv_hidden.to_string()]),
                        th.text_dim,
                    );
                }
            }
            let now = std::time::Instant::now();
            if self.plugin_page && self.plugin_rect.h > 0 {
                let r = self.plugin_rect;
                let clip = r.intersection(&list);
                if clip.h > 0 {
                    dc.fill_round_rect(clip, (6.0 * s).round() as i32, th.panel_bg);
                    let tx = r.x + inner_pad;
                    let mut ty = r.y + inner_pad;
                    dc.select_font(FontSlot::Base, true);
                    dc.text(tx, ty, clip, &tr("pref.cat.plugins"), th.text);
                    dc.select_font(FontSlot::Base, false);
                    ty += th_txt + (4.0 * s).round() as i32;
                    let desc_n = plugin_lines.iter().filter(|(_, warn)| !warn).count() as i32;
                    for (l, warn) in plugin_lines.iter().filter(|(_, w)| !w) {
                        let _ = warn;
                        dc.text(tx, ty, clip, l, th.text_dim);
                        ty += th_txt;
                    }
                    for b in &self.plugin_boxes {
                        b.paint(&mut dc, th);
                    }
                    ty = r.y
                        + inner_pad
                        + th_txt
                        + (4.0 * s).round() as i32
                        + th_txt * desc_n
                        + if self.plugin_boxes.is_empty() {
                            0
                        } else {
                            (8.0 * s).round() as i32 + self.plugin_boxes.len() as i32 * ctl_h
                        };
                    for (l, _) in plugin_lines.iter().filter(|(_, w)| *w) {
                        dc.text(tx, ty, clip, l, th.warn);
                        ty += th_txt;
                    }
                }
            }
            for (c, (lines, note_at)) in self.cards.iter().zip(desc_lines.iter()) {
                if c.rect.h == 0 {
                    continue;
                }
                let r = c.rect;
                let clip = r.intersection(&list);
                if clip.h <= 0 {
                    continue;
                }
                dc.fill_round_rect(clip, (6.0 * s).round() as i32, th.panel_bg);
                let tx = r.x + inner_pad;
                let mut ty = r.y + inner_pad;
                dc.select_font(FontSlot::Base, true);
                let label = if c.show_cat {
                    format!("{} › {}", tr(c.entry.cat), tr(c.entry.label))
                } else {
                    tr(c.entry.label)
                };
                dc.text(tx, ty, clip, &label, th.text);
                dc.select_font(FontSlot::Base, false);
                let key_w = dc.text_width(c.entry.key);
                let key_color = if ndir_settings::is_advanced(c.entry.key) {
                    th.accent
                } else {
                    th.text_dim
                };
                let kx = r.right() - inner_pad - th_txt - (6.0 * s).round() as i32 - key_w;
                dc.text(kx, ty, clip, c.entry.key, key_color);
                c.copy.paint(&mut dc, th, 1.0, s, now);
                ty += th_txt + (4.0 * s).round() as i32;
                for (li, l) in lines.iter().enumerate() {
                    let col = if li >= *note_at { th.ok } else { th.text_dim };
                    dc.text(tx, ty, clip, l, col);
                    ty += th_txt;
                }
                if let Some(e) = &c.error {
                    dc.text(tx, ty, clip, e, th.danger);
                }
                let dv = trf("pref.defaultValue", &[c.entry.default]);
                let dw = dc.text_width(&dv);
                let cy = r.bottom() - inner_pad - ctl_h;
                if c.entry.default.is_empty() {
                } else if c.default_below {
                    let ty2 = ty + if c.error.is_some() { th_txt } else { 0 };
                    dc.text(tx, ty2, clip, &dv, th.text_dim);
                } else {
                    dc.text(
                        r.right() - inner_pad - dw,
                        cy + (ctl_h - th_txt) / 2,
                        clip,
                        &dv,
                        th.text_dim,
                    );
                }
                match &c.ctl {
                    CardCtl::Bool(sw) => sw.paint(&mut dc, th),
                    CardCtl::Choice(_) => {}
                    CardCtl::Pos(pd) => pd.paint(&mut dc, th),
                    CardCtl::Text(tb) => tb.paint(&mut dc, th),
                }
                if c.locked {
                    let cr = match &c.ctl {
                        CardCtl::Bool(sw) => sw.bounds(),
                        CardCtl::Choice(cb) => cb.bounds(),
                        CardCtl::Pos(pd) => pd.bounds(),
                        CardCtl::Text(tb) => tb.bounds(),
                    };
                    dc.fill_rect_alpha(cr, th.panel_bg, 0.6);
                }
                if let Some(b) = &c.aux {
                    b.paint(&mut dc, th);
                }
                if c.modified {
                    c.reset.paint(&mut dc, th);
                }
            }
            for c in &self.cards {
                if c.rect.h == 0 {
                    continue;
                }
                match &c.ctl {
                    CardCtl::Choice(cb) if !cb.is_open() => {
                        cb.paint(&mut dc, th);
                        if c.locked {
                            dc.fill_rect_alpha(cb.bounds(), th.panel_bg, 0.6);
                        }
                    }
                    CardCtl::Pos(pd) => pd.paint_popup(&mut dc, th),
                    _ => {}
                }
            }
            self.bars.paint(
                &mut dc,
                th,
                list,
                list.w,
                self.content_h.max(list.h),
                0,
                self.scroll,
                s,
            );
            self.advanced.paint(&mut dc, th);
            self.close_btn.paint(&mut dc, th);
            self.json_btn.paint(&mut dc, th);
            for c in &self.cards {
                if let CardCtl::Choice(cb) = &c.ctl {
                    if c.rect.h != 0 && cb.is_open() {
                        cb.paint(&mut dc, th);
                    }
                }
            }
            self.search.paint_popup(&mut dc, th);
            for c in &self.cards {
                if let CardCtl::Text(tb) = &c.ctl {
                    tb.paint_popup(&mut dc, th);
                }
            }
        }
        let _ = buf.present();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(tag: &str, text: &str) -> Settings {
        Settings::from_text(
            std::env::temp_dir().join(format!("ndir-prefs-{tag}-{}.conf", std::process::id())),
            text,
        )
    }

    /// 분류 선택 = 그 분류 카드만 · 검색 = 키/라벨/설명 전부 · 고급은 스위치가 꺼지면 숨기고 수만 센다.
    #[test]
    fn category_search_and_advanced() {
        let s = settings("cat", "");
        let mut w = PrefsWin::new();
        w.refresh(&s);
        w.select_category("pref.cat.keys");
        let keys = w.shown_keys();
        assert!(
            !keys.is_empty() && keys.iter().all(|k| k.starts_with("key.")),
            "{keys:?}"
        );
        w.preset_query("show_hidden");
        assert_eq!(w.shown_keys(), vec!["list.show_hidden"]);
        w.preset_query("");
        w.select_category("pref.cat.appearance");
        let before = w.shown_keys().len();
        let hidden = w.adv_hidden;
        w.set_advanced(true);
        assert_eq!(w.shown_keys().len(), before + hidden);
        assert_eq!(w.adv_hidden, 0);
    }

    /// 종속 잠금(DEPENDS): 부모가 꺼지면 자식 카드가 잠기고 · 켜면 바로 풀린다(카드를 다시 만들지 않고 `refresh`만).
    #[test]
    fn dependent_cards_lock_and_unlock_with_parent() {
        let mut s = settings("dep", "scroll.fast=off\n");
        let mut w = PrefsWin::new();
        w.refresh(&s);
        w.select_category("pref.cat.scroll");
        assert_eq!(w.is_locked("scroll.fast_step"), Some(true));
        assert_eq!(w.is_locked("scroll.fast"), Some(false));
        s.set("scroll.fast", "on").expect("set");
        w.refresh(&s);
        assert_eq!(w.is_locked("scroll.fast_step"), Some(false));
        // 동적 후보(언어 목록)는 Text 카드를 콤보로 바꾼다.
        w.set_dyn_choices(
            "ui.lang",
            vec![
                ("system".into(), "System".into()),
                ("en".into(), "English".into()),
            ],
        );
        w.select_category("pref.cat.lang");
        assert!(w
            .cards
            .iter()
            .any(|c| c.entry.key == "ui.lang" && matches!(c.ctl, CardCtl::Choice(_))));
    }
}
