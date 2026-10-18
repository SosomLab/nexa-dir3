//! 일괄 이름 변경 창(T-71 · dir2 `bulkrename.rs` 이식 — docs/port/16 DLG-074~088): 좌 = **카드 스택**(동작 1블록 = 카드 1장 · 위→아래 파이프라인 ·
//! 종류 콤보 6종 + [+]/[−] · 마지막 1장 삭제 불가) · 우 = **미리보기 그리드**(nexa-grid · # · 이전 · 이후(⚠ 사유) · 적용 ✓) · 하단 = 프리셋 콤보 `[… ⌄]` ·
//! 건수/검증 오류 · [취소][Rename]. 코어는 `ndir_ops::batch_rename`(수확 규칙 DLG-077 · 실시간 미리보기 DLG-079 · 충돌 · 프리셋 직렬화).
//! 창 880×620 고정 · 소유자 중앙. [Rename] 활성 = 동작 ≥ 1 · 변경 ≥ 1 · 검증 통과 · 충돌 0. 결과 = `(전체 경로, 새 이름)` 목록 → 호스트가 순차 rename + undo 1건.

use ndir_i18n::{tr, trf};
use ndir_ops::batch_rename::{
    self as br, CaseMode, Conflict, DateKind, DateSpec, InsertAt, NumberSpec, RenameInput,
    RenameOp, ReplaceMode, Scope,
};
use nexa_ctl::controls::ctxmenu::{ContextMenu, CtxItem};
use nexa_ctl::controls::toast::{ToastCorner, ToastKind, Toasts};
use nexa_ctl::controls::EditCtxAction;
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::ScrollBars;
use nexa_ctl::{
    Button, Checkbox, Combo, ComboControl, ComboItem, Control, InputEvent, Invalidations,
    Key as CtlKey, TextBox, Widget,
};
use nexa_gfx::{Font, Surface};
use nexa_grid::{Column, Marker, RowItem, RowSource, SelectOp, VirtualRows};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use winit::event::{ElementState, Ime, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// 창이 호스트에 요청하는 것.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BulkAction {
    None,
    Paint,
    Close,
    /// [Rename] — (전체 경로, 새 이름) 목록(변경 + 적용 + 충돌 없음).
    Rename(Vec<(PathBuf, String)>),
    /// 프리셋 저장 요청(직렬화된 파이프라인) — 호스트가 이름을 묻고 파일로.
    SavePreset(String),
    /// 프리셋 편집 창 요청(순서 · 삭제 · 10-18).
    EditPresets,
    /// 모든 프리셋 제거 요청(호스트가 확인창 뒤 삭제 · 10-18).
    ClearPresets,
    /// 프리셋 불러오기(이름) — 호스트가 파일을 읽어 `load_ops`.
    LoadPreset(String),
}

/// 대상 항목(DLG-088: 선택 순서 보존 · 메타 실패 = 0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BulkItem {
    pub parent: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub modified_ms: i64,
    pub created_ms: i64,
}

impl BulkItem {
    /// 경로에서(메타 조회 실패 = 0).
    pub(crate) fn from_path(p: &Path) -> Option<BulkItem> {
        let name = p.file_name()?.to_string_lossy().into_owned();
        let parent = p.parent()?.to_path_buf();
        let meta = std::fs::metadata(p).ok();
        let ms = |t: Option<std::time::SystemTime>| {
            t.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_millis() as i64)
        };
        Some(BulkItem {
            is_dir: meta.as_ref().is_some_and(|m| m.is_dir()),
            modified_ms: ms(meta.as_ref().and_then(|m| m.modified().ok())),
            created_ms: ms(meta.as_ref().and_then(|m| m.created().ok())),
            parent,
            name,
        })
    }
}

const KINDS: [(&str, &str); 6] = [
    ("replace", "bulk.kind.replace"),
    ("replacerx", "bulk.kind.replaceRx"),
    ("insert", "bulk.kind.insert"),
    ("case", "bulk.kind.case"),
    ("number", "bulk.kind.number"),
    ("date", "bulk.kind.date"),
];
const SCOPES: [(&str, &str); 4] = [
    ("name", "bulk.scope.name"),
    ("nameext", "bulk.scope.nameext"),
    ("ext", "bulk.scope.ext"),
    ("extdot", "bulk.scope.extdot"),
];
const MODES: [(&str, &str); 4] = [
    ("all", "bulk.mode.all"),
    ("first", "bulk.mode.first"),
    ("last", "bulk.mode.last"),
    ("entire", "bulk.mode.entire"),
];
const CASES: [(&str, &str); 4] = [
    ("upper", "bulk.case.upper"),
    ("lower", "bulk.case.lower"),
    ("title", "bulk.case.title"),
    ("sentence", "bulk.case.sentence"),
];
const DIRS: [(&str, &str); 2] = [("end", "bulk.dirEnd"), ("start", "bulk.dirStart")];
const DATES: [(&str, &str); 2] = [
    ("modified", "bulk.date.modified"),
    ("created", "bulk.date.created"),
];
const PADS: [&str; 6] = ["1", "01", "001", "0001", "00001", "000001"];
const DEFAULT_DATE_FMT: &str = "${YYYY}-${MM}-${DD}";

fn combo_of(items: &[(&str, &str)], selected: usize) -> Combo {
    Combo::new(
        items
            .iter()
            .map(|(v, k)| ComboItem::new(*v, tr(k)))
            .collect(),
        selected,
    )
}

fn scope_of(v: &str) -> Scope {
    match v {
        "nameext" => Scope::NameExt,
        "ext" => Scope::Ext,
        "extdot" => Scope::ExtDot,
        _ => Scope::Name,
    }
}

fn scope_value(s: Scope) -> &'static str {
    match s {
        Scope::Name => "name",
        Scope::NameExt => "nameext",
        Scope::Ext => "ext",
        Scope::ExtDot => "extdot",
    }
}

/// 카드 1장 = 동작 1블록(폼 컨트롤 전부 보유 · 종류에 따라 보이는 행만 배치).
struct Card {
    kind: Combo,
    add: Button,
    del: Button,
    scope: Combo,
    mode: Combo,
    match_case: Checkbox,
    find: TextBox,
    with: TextBox,
    dir: Combo,
    offset: TextBox,
    text: TextBox,
    case_mode: Combo,
    pad: Combo,
    start: TextBox,
    step: TextBox,
    prefix: TextBox,
    suffix: TextBox,
    date_kind: Combo,
    fmt: TextBox,
    rect: Rect,
}

impl Card {
    fn new(kind_idx: usize) -> Card {
        let mut start = TextBox::new("");
        start.set_text("1");
        let mut step = TextBox::new("");
        step.set_text("1");
        let mut offset = TextBox::new("");
        offset.set_text("0");
        let mut fmt = TextBox::new("");
        fmt.set_text(DEFAULT_DATE_FMT);
        Card {
            kind: combo_of(&KINDS, kind_idx),
            add: Button::new("+"),
            del: Button::new("−"),
            scope: combo_of(&SCOPES, 0),
            mode: combo_of(&MODES, 0),
            match_case: Checkbox::new(tr("bulk.matchCase"), false),
            find: TextBox::new(""),
            with: TextBox::new(""),
            dir: combo_of(&DIRS, 0),
            offset,
            text: TextBox::new(""),
            case_mode: combo_of(&CASES, 0),
            pad: Combo::new(
                PADS.iter()
                    .enumerate()
                    .map(|(i, l)| ComboItem::new((i + 1).to_string(), *l))
                    .collect(),
                2,
            ),
            start,
            step,
            prefix: TextBox::new(""),
            suffix: TextBox::new(""),
            date_kind: combo_of(&DATES, 0),
            fmt,
            rect: Rect::default(),
        }
    }

    fn kind_value(&self) -> String {
        self.kind.selected_value()
    }

    /// 종류별 본문 행 수(공통 = 종류 행 + 대상 행).
    fn body_rows(&self) -> usize {
        match self.kind_value().as_str() {
            "replace" => 4,
            "replacerx" => 3,
            "insert" => 2,
            "case" => 1,
            "number" => 4,
            "date" => 4,
            _ => 0,
        }
    }

    fn at(&self) -> InsertAt {
        let off = self.offset.text().trim().parse::<usize>().unwrap_or(0);
        InsertAt {
            offset: off.min(999),
            from_end: self.dir.selected_value() == "end",
        }
    }

    /// 폼 → 동작(DLG-077 수확 규칙): 무효 카드 = None(파이프라인에서 건너뜀).
    fn op(&self) -> Option<RenameOp> {
        self.op_with(true)
    }

    /// 폼 → 동작 · `strict = false`면 빈 찾기/글도 그대로 담는다(프리셋 저장 = 카드 전부 · 사용자 10-18).
    fn op_with(&self, strict: bool) -> Option<RenameOp> {
        let scope = scope_of(&self.scope.selected_value());
        match self.kind_value().as_str() {
            "replace" => {
                let mode = match self.mode.selected_value().as_str() {
                    "first" => ReplaceMode::First,
                    "last" => ReplaceMode::Last,
                    "entire" => ReplaceMode::Entire,
                    _ => ReplaceMode::All,
                };
                let find = self.find.text();
                if strict && find.is_empty() && mode != ReplaceMode::Entire {
                    return None;
                }
                Some(RenameOp::Replace {
                    scope,
                    find,
                    with: self.with.text(),
                    match_case: self.match_case.is_checked(),
                    regex: false,
                    mode,
                })
            }
            "replacerx" => {
                let find = self.find.text();
                if strict && find.is_empty() {
                    return None;
                }
                Some(RenameOp::Replace {
                    scope,
                    find,
                    with: self.with.text(),
                    match_case: self.match_case.is_checked(),
                    regex: true,
                    mode: ReplaceMode::All,
                })
            }
            "insert" => {
                let text = self.text.text();
                if strict && text.is_empty() {
                    return None;
                }
                Some(RenameOp::Insert {
                    scope,
                    text,
                    at: self.at(),
                })
            }
            "case" => Some(RenameOp::Case {
                scope,
                mode: match self.case_mode.selected_value().as_str() {
                    "lower" => CaseMode::Lower,
                    "title" => CaseMode::Title,
                    "sentence" => CaseMode::Sentence,
                    _ => CaseMode::Upper,
                },
            }),
            "number" => Some(RenameOp::Number {
                scope,
                spec: NumberSpec {
                    start: self
                        .start
                        .text()
                        .trim()
                        .parse::<i64>()
                        .unwrap_or(1)
                        .clamp(-9999, 9999),
                    step: self.step.text().trim().parse::<i64>().unwrap_or(1),
                    pad: self.pad.selected_value().parse::<usize>().unwrap_or(3),
                    at: self.at(),
                    prefix: self.prefix.text(),
                    suffix: self.suffix.text(),
                },
            }),
            "date" => {
                let f = self.fmt.text();
                let format = if f.trim().is_empty() {
                    DEFAULT_DATE_FMT.to_string()
                } else {
                    br::migrate_date_format(&f)
                };
                Some(RenameOp::Date {
                    scope,
                    spec: DateSpec {
                        kind: if self.date_kind.selected_value() == "created" {
                            DateKind::Created
                        } else {
                            DateKind::Modified
                        },
                        format,
                        at: self.at(),
                        prefix: self.prefix.text(),
                        suffix: self.suffix.text(),
                    },
                })
            }
            _ => None,
        }
    }

    /// 동작 → 폼(프리셋 복원 · `set_form_from_op` · Move/ChangeExt = None).
    fn from_op(op: &RenameOp) -> Option<Card> {
        let set_at = |c: &mut Card, at: &InsertAt| {
            c.offset.set_text(&at.offset.to_string());
            c.dir
                .select_value(if at.from_end { "end" } else { "start" });
        };
        Some(match op {
            RenameOp::Replace {
                scope,
                find,
                with,
                match_case,
                regex,
                mode,
            } => {
                let mut c = Card::new(if *regex { 1 } else { 0 });
                c.scope.select_value(scope_value(*scope));
                c.find.set_text(find);
                c.with.set_text(with);
                c.match_case.set_checked(*match_case);
                c.mode.select_value(match mode {
                    ReplaceMode::All => "all",
                    ReplaceMode::First => "first",
                    ReplaceMode::Last => "last",
                    ReplaceMode::Entire => "entire",
                });
                c
            }
            RenameOp::Insert { scope, text, at } => {
                let mut c = Card::new(2);
                c.scope.select_value(scope_value(*scope));
                c.text.set_text(text);
                set_at(&mut c, at);
                c
            }
            RenameOp::Case { scope, mode } => {
                let mut c = Card::new(3);
                c.scope.select_value(scope_value(*scope));
                c.case_mode.select_value(match mode {
                    CaseMode::Upper => "upper",
                    CaseMode::Lower => "lower",
                    CaseMode::Title => "title",
                    CaseMode::Sentence => "sentence",
                });
                c
            }
            RenameOp::Number { scope, spec } => {
                let mut c = Card::new(4);
                c.scope.select_value(scope_value(*scope));
                c.start.set_text(&spec.start.to_string());
                c.step.set_text(&spec.step.to_string());
                c.pad.select_value(&spec.pad.clamp(1, 6).to_string());
                c.prefix.set_text(&spec.prefix);
                c.suffix.set_text(&spec.suffix);
                set_at(&mut c, &spec.at);
                c
            }
            RenameOp::Date { scope, spec } => {
                let mut c = Card::new(5);
                c.scope.select_value(scope_value(*scope));
                c.date_kind.select_value(match spec.kind {
                    DateKind::Modified => "modified",
                    DateKind::Created => "created",
                });
                c.fmt.set_text(&spec.format);
                c.prefix.set_text(&spec.prefix);
                c.suffix.set_text(&spec.suffix);
                set_at(&mut c, &spec.at);
                c
            }
            RenameOp::Move { .. } | RenameOp::ChangeExt { .. } => return None,
        })
    }

    fn combos(&mut self) -> Vec<&mut Combo> {
        vec![
            &mut self.kind,
            &mut self.scope,
            &mut self.mode,
            &mut self.dir,
            &mut self.case_mode,
            &mut self.pad,
            &mut self.date_kind,
        ]
    }

    fn boxes(&mut self) -> Vec<&mut TextBox> {
        vec![
            &mut self.find,
            &mut self.with,
            &mut self.offset,
            &mut self.text,
            &mut self.start,
            &mut self.step,
            &mut self.prefix,
            &mut self.suffix,
            &mut self.fmt,
        ]
    }

    fn buttons(&mut self) -> [&mut Button; 2] {
        [&mut self.add, &mut self.del]
    }
}

/// 미리보기 행(그리드 소스).
#[derive(Debug, Clone, PartialEq, Eq)]
struct PvRow {
    /// 항목 인덱스(선택 순서 · 연번).
    idx: usize,
    before: String,
    after: String,
    conflict: Conflict,
    changed: bool,
    apply: bool,
}

#[derive(Debug, Default)]
struct PvSource {
    rows: Vec<PvRow>,
    selected: Option<usize>,
    /// 자연 정렬(여는 탭의 보기 옵션 — 10-08 탭 소유 · 파일 목록과 같은 순서로 이전/이후 열을 정렬).
    natural: bool,
}

const COL_NO: u32 = 0;
const COL_BEFORE: u32 = 1;
const COL_AFTER: u32 = 2;
const COL_APPLY: u32 = 3;

fn conflict_text(c: Conflict) -> String {
    match c {
        Conflict::None => String::new(),
        Conflict::Empty => tr("bulk.conflict.empty"),
        Conflict::Invalid => tr("bulk.conflict.invalid"),
        Conflict::Duplicate => tr("bulk.conflict.dup"),
        Conflict::Exists => tr("bulk.conflict.exists"),
        _ => tr("bulk.conflict.nested"),
    }
}

impl RowSource for PvSource {
    fn len(&self) -> usize {
        self.rows.len()
    }
    fn row(&self, index: usize) -> RowItem {
        RowItem {
            text: (self.rows[index].idx + 1).to_string(),
            is_dir: false,
            depth: 0,
            marker: Marker::None,
        }
    }
    fn cell(&self, index: usize, key: u32) -> String {
        let r = &self.rows[index];
        match key {
            COL_BEFORE => r.before.clone(),
            COL_AFTER => {
                if r.conflict != Conflict::None {
                    format!("⚠ {} ({})", r.after, conflict_text(r.conflict))
                } else {
                    // 바뀌지 않아도 이름을 그대로 보인다(사용자 10-18 "변경 설정이 없으면 이후 이름도 그대로") — 바뀜 여부는 "적용" 열.
                    r.after.clone()
                }
            }
            COL_APPLY => {
                if !r.changed {
                    String::new()
                } else if !r.apply {
                    "✗".into()
                } else if r.conflict != Conflict::None {
                    "⚠".into()
                } else {
                    "✓".into()
                }
            }
            _ => String::new(),
        }
    }
    /// 정렬(DLG-081): 이전/이후 기준 대소문자 무시 · 안정 · 다중 키 · 빈 사양 = 선택 순서. 여는 탭의 자연 정렬이 켜져 있으면
    /// 파일 목록과 같은 순서(숫자를 값으로).
    fn set_sort(&mut self, keys: &[(u32, bool)]) -> bool {
        if keys.is_empty() {
            self.rows.sort_by_key(|r| r.idx);
            return true;
        }
        let natural = self.natural;
        self.rows.sort_by(|a, b| {
            for &(k, desc) in keys {
                let (x, y) = match k {
                    COL_BEFORE => (a.before.to_lowercase(), b.before.to_lowercase()),
                    COL_AFTER => (a.after.to_lowercase(), b.after.to_lowercase()),
                    _ => (a.idx.to_string(), b.idx.to_string()),
                };
                let o = if k == COL_NO {
                    a.idx.cmp(&b.idx)
                } else if natural {
                    ndir_tree::cmp_natural(&x, &y, true).then_with(|| x.cmp(&y))
                } else {
                    x.cmp(&y)
                };
                let o = if desc { o.reverse() } else { o };
                if o != std::cmp::Ordering::Equal {
                    return o;
                }
            }
            a.idx.cmp(&b.idx)
        });
        true
    }
    fn is_selected(&self, index: usize) -> bool {
        self.selected == Some(index)
    }
    fn select(&mut self, index: usize, _op: SelectOp) -> bool {
        self.selected = Some(index);
        true
    }
    fn clear_selection(&mut self) -> bool {
        self.selected.take().is_some()
    }
}

const W: f32 = 880.0;
const H: f32 = 620.0;
const PAD: f32 = 10.0;
const CARD_W: f32 = 320.0;
const ROW_H: f32 = 28.0;
const LABEL_W: f32 = 96.0;
const BTN_H: f32 = 28.0;
/// 카드 드래그가 시작되는 이동 거리(px).
const CARD_DRAG_THRESHOLD: i32 = 5;
/// 카드 열 밖 고스트 축소 비율.
const CARD_GHOST_SCALE: f32 = 0.4;

/// 진행 중인 카드 드래그.
struct CardDrag {
    from: usize,
    press: (i32, i32),
    cur: (i32, i32),
    active: bool,
}

pub(crate) struct BulkWin {
    window: Option<Rc<Window>>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    cursor: (i32, i32),
    shift: bool,
    primary: bool,
    items: Vec<BulkItem>,
    cards: Vec<Card>,
    rows: VirtualRows<PvSource>,
    /// 카드 열 세로 스크롤(px) · 카드 합 높이.
    scroll: i32,
    cards_h: i32,
    cards_rect: Rect,
    grid_rect: Rect,
    /// `[…]` 버튼 + 그 위로 펴는 프리셋 메뉴(사용자 10-18 · 종전 콤보 = ✓ 칸 · 헤더/구분자 불가).
    /// 카드 드래그 이동(사용자 10-18 · Path Finder 꼴): 카드 바탕(컨트롤 밖)을 잡고 끌면 삽입 위치 인디케이터 + 고스트(열 밖 = 축소).
    card_drag: Option<CardDrag>,
    /// 카드 열 스크롤 막대(오버레이 · 썸 드래그 · 전역 고속 스크롤 설정 공유 · 사용자 10-18).
    bars: ScrollBars,
    /// 창 안 토스트(프리셋 불러옴 안내 · 사용자 10-18) + 모서리(설정 `ui.toast_position` · 호스트가 준다).
    toasts: Toasts,
    toast_corner: ToastCorner,
    presets_btn: Button,
    preset_menu: ContextMenu,
    preset_names: Vec<String>,
    /// 메뉴 라벨 최대 폭(마지막 paint에서 측정 · `open_at`에 준다 · 첫 paint가 실측으로 보정한다).
    menu_text_w: i32,
    btn_cancel: Button,
    btn_rename: Button,
    /// 검증 오류(`⚠ #블록: 메시지`) · 건수 · 마지막 수확 서명(변경 감지).
    error: Option<String>,
    count: usize,
    /// 충돌로 빠지는 행 수(건수 줄에 함께 알린다).
    conflicts: usize,
    /// [Rename] 가능(버튼 활성과 같다 — Enter 키가 본다).
    can_rename: bool,
    sig: String,
    tz_min: i32,
    /// 자연 정렬(여는 탭의 보기 옵션 — [`Self::set_natural`] · 다음 대상부터 행 공급자에 실린다).
    natural: bool,
}

impl BulkWin {
    pub(crate) fn new() -> Self {
        let mut rows = VirtualRows::new(PvSource::default(), 20, 8, 16);
        let mut inv = Invalidations::default();
        rows.set_columns(
            vec![
                Column::new(COL_NO, "#", 44).right_aligned(),
                Column::new(COL_BEFORE, tr("bulk.grid.before"), 200),
                Column::new(COL_AFTER, tr("bulk.grid.after"), 220),
                Column::new(COL_APPLY, tr("bulk.grid.apply"), 56),
            ],
            &mut inv,
        );
        let mut w = BulkWin {
            window: None,
            surface: None,
            scale: 1.0,
            cursor: (0, 0),
            shift: false,
            primary: false,
            items: Vec::new(),
            cards: vec![Card::new(0)],
            rows,
            scroll: 0,
            cards_h: 0,
            cards_rect: Rect::default(),
            grid_rect: Rect::default(),
            card_drag: None,
            bars: ScrollBars::new(),
            toasts: Toasts::new(),
            toast_corner: ToastCorner::BottomRight,
            presets_btn: Button::new("…"),
            preset_menu: ContextMenu::new(),
            preset_names: Vec::new(),
            menu_text_w: 160,
            btn_cancel: Button::new(tr("bulk.cancel")),
            btn_rename: Button::new(tr("bulk.rename")),
            error: None,
            count: 0,
            conflicts: 0,
            can_rename: false,
            sig: String::new(),
            tz_min: 0,
            natural: true,
        };
        w.set_presets(Vec::new());
        // [이름 변경] = 기본 버튼(Accent · 사용자 10-18).
        w.btn_rename
            .set_tone(nexa_ctl::controls::ButtonTone::Accent);
        w
    }

    /// 대상 교체(열 때) — 카드는 기본 1장 · 선택 순서 보존.
    /// 여는 탭의 자연 정렬 값(탭 보기 옵션)을 넣는다 — 다음 정렬부터 이전/이후 열이 파일 목록과 같은 순서가 된다.
    pub(crate) fn set_natural(&mut self, on: bool) {
        self.natural = on;
    }

    pub(crate) fn set_items(&mut self, items: Vec<BulkItem>, tz_min: i32) {
        self.items = items;
        self.tz_min = tz_min;
        self.cards = vec![Card::new(0)];
        self.scroll = 0;
        self.sig.clear();
        // 이전 대상의 적용 체크를 버린다(종전에는 행 번호로 이어받아, 지난번에 뺀 행 번호가 새 대상에서도 빠진 채 열렸다).
        let mut inv = Invalidations::default();
        self.rows.replace_source(
            PvSource {
                rows: Vec::new(),
                selected: None,
                natural: self.natural,
            },
            &mut inv,
        );
        self.recompute(true);
        // 열자마자 첫 입력칸에서 바로 친다(종전에는 포커스가 없어 먼저 눌러야 했다).
        if let Some(tb) = self.cards[0].boxes().into_iter().next() {
            tb.set_focused(true);
        }
    }

    /// 건수 줄: "N개 항목이 변경됩니다" · 충돌이 있으면 건너뛰는 수를 함께.
    fn count_text(&self) -> String {
        if self.conflicts > 0 {
            trf(
                "bulk.countSkip",
                &[&self.count.to_string(), &self.conflicts.to_string()],
            )
        } else {
            trf("bulk.count", &[&self.count.to_string()])
        }
    }

    /// 선택한 미리보기 행의 적용 여부를 뒤집는다(Space · 적용 열 클릭과 같다) — 뒤집었으면 `true`.
    fn toggle_selected(&mut self) -> bool {
        let src = self.rows.source_mut();
        let Some(r) = src.selected.and_then(|i| src.rows.get_mut(i)) else {
            return false;
        };
        if !r.changed {
            return false;
        }
        r.apply = !r.apply;
        self.recompute(false);
        true
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn items_len(&self) -> usize {
        self.items.len()
    }

    /// 토스트 모서리(설정 `ui.toast_position` · 열 때 호스트가 준다).
    pub(crate) fn set_toast_corner(&mut self, corner: ToastCorner) {
        self.toast_corner = corner;
    }

    /// 프리셋을 불러왔음을 창 안 토스트로(사용자 10-18 "어떤 프리셋으로 몇 개 카드가 읽어졌는지").
    pub(crate) fn notify_preset_loaded(&mut self, name: &str) {
        let body = trf("bulk.preset.loaded", &[name, &self.cards.len().to_string()]);
        self.toasts
            .push(ToastKind::Info, tr("bulk.preset.header"), body);
        self.redraw();
    }

    #[cfg(test)]
    pub(crate) fn toasts_len(&self) -> usize {
        self.toasts.len()
    }

    /// 프리셋 목록(호스트가 정한 순서 · 최대 64) — `[…]` 버튼의 메뉴에 그대로 실린다(10-18 · 순서 = `renames/order.txt`).
    pub(crate) fn set_presets(&mut self, mut names: Vec<String>) {
        names.truncate(64);
        self.preset_names = names;
        if self.preset_menu.is_open() {
            self.preset_menu.close();
        }
    }

    /// 프리셋 메뉴(사용자 10-18 · Path Finder 꼴): `Presets` 헤더(클릭 없음) · ─ · 저장된 프리셋들(있을 때만 · 뒤에 ─) ·
    /// 저장 · 편집 · ─ · 모든 프리셋 제거(없으면 끔).
    pub(crate) fn preset_menu_items(&self) -> Vec<CtxItem> {
        let mut items = vec![
            CtxItem::maybe("hdr", tr("bulk.preset.header"), false),
            CtxItem::Separator,
        ];
        for n in &self.preset_names {
            items.push(CtxItem::item(format!("preset:{n}"), n.clone()));
        }
        if !self.preset_names.is_empty() {
            items.push(CtxItem::Separator);
        }
        items.push(CtxItem::item("save", tr("bulk.preset.saveSeq")));
        items.push(CtxItem::item("edit", tr("bulk.preset.editSeq")));
        items.push(CtxItem::Separator);
        items.push(
            CtxItem::maybe(
                "clear",
                tr("bulk.preset.removeAll"),
                !self.preset_names.is_empty(),
            )
            .with_danger(true),
        );
        items
    }

    /// `[…]` 위로 메뉴를 편다(창 안에서 자리 접기 = ContextMenu 공용 규칙).
    fn open_preset_menu(&mut self) {
        let b = self.presets_btn.bounds();
        let host = self
            .window
            .as_ref()
            .map_or(Rect::new(0, 0, 4096, 4096), |w| {
                let s = w.inner_size();
                Rect::new(0, 0, s.width as i32, s.height as i32)
            });
        let items = self.preset_menu_items();
        self.preset_menu.set_scale(self.scale);
        self.preset_menu
            .open_at(b.x, b.y, items, host, self.menu_text_w);
    }

    /// 메뉴 선택 → 호스트 요청. 저장은 카드가 1장(무효 포함)이어도 된다(사용자 10-18 "들어가서 바로 저장" — 빈 파이프라인 = 기본 카드).
    fn preset_pick(&mut self, id: &str) -> Option<BulkAction> {
        if let Some(name) = id.strip_prefix("preset:") {
            return Some(BulkAction::LoadPreset(name.to_string()));
        }
        match id {
            "save" => Some(BulkAction::SavePreset(br::serialize_ops(&self.ops_all()))),
            "edit" => Some(BulkAction::EditPresets),
            "clear" => Some(BulkAction::ClearPresets),
            _ => None,
        }
    }

    /// 파이프라인 교체(프리셋 불러오기 · 시험) — 빈/전부 스킵 = 기본 카드 1장.
    pub(crate) fn load_ops(&mut self, ops: &[RenameOp]) {
        let cards: Vec<Card> = ops.iter().filter_map(Card::from_op).collect();
        self.cards = if cards.is_empty() {
            vec![Card::new(0)]
        } else {
            cards
        };
        self.scroll = 0;
        self.sig.clear();
        // 새 카드에 bounds·축척을 준다(종전에는 없어서 프리셋을 불러오면 왼쪽 카드가 전부 사라졌다 · 사용자 10-18).
        self.layout_cards();
        self.clamp_scroll();
        self.recompute(true);
    }

    /// 현재 파이프라인(유효 카드만).
    pub(crate) fn ops(&self) -> Vec<RenameOp> {
        self.cards.iter().filter_map(Card::op).collect()
    }

    /// 카드 전부(무효 포함) — 프리셋 저장용(불러오면 같은 카드 수로 돌아온다).
    pub(crate) fn ops_all(&self) -> Vec<RenameOp> {
        self.cards.iter().filter_map(|c| c.op_with(false)).collect()
    }

    /// 시험용 카드 폼 접근(종류 · 찾기 · 바꾸기 · 텍스트).
    #[cfg(test)]
    fn card_mut(&mut self, i: usize) -> &mut Card {
        &mut self.cards[i]
    }

    /// 실시간 미리보기(DLG-079): 수확 → 검증 → 새 이름 → 충돌 → 그리드 행 · 건수 · [Rename] 활성.
    fn recompute(&mut self, force: bool) {
        let ops = self.ops();
        let applies: Vec<bool> = self.rows.source().rows.iter().map(|r| r.apply).collect();
        let sig = format!("{}|{:?}", br::serialize_ops(&ops), applies);
        if !force && sig == self.sig {
            return;
        }
        self.sig = sig;
        self.error = br::validate(&ops)
            .err()
            .map(|(i, m)| format!("⚠ #{}: {m}", i + 1));
        let inputs: Vec<RenameInput> = self
            .items
            .iter()
            .map(|it| RenameInput {
                name: it.name.clone(),
                is_dir: it.is_dir,
                modified_ms: it.modified_ms,
                created_ms: it.created_ms,
            })
            .collect();
        let news = if self.error.is_none() {
            br::preview(&inputs, &ops, self.tz_min)
        } else {
            inputs.iter().map(|i| i.name.clone()).collect()
        };
        // 적용 체크는 보존(인덱스 기준).
        let old_apply: std::collections::HashMap<usize, bool> = self
            .rows
            .source()
            .rows
            .iter()
            .map(|r| (r.idx, r.apply))
            .collect();
        // 적용에서 뺀 행은 **바뀌지 않는 항목**으로 넘긴다 — 중복 · 연쇄 판정에서 빠져, 겹치는 둘 중 하나를 빼면 나머지가 풀린다.
        let triples: Vec<(String, String, String)> = self
            .items
            .iter()
            .zip(&news)
            .enumerate()
            .map(|(i, (it, n))| {
                let new = if old_apply.get(&i).copied().unwrap_or(true) {
                    n.clone()
                } else {
                    it.name.clone()
                };
                (it.parent.display().to_string(), it.name.clone(), new)
            })
            .collect();
        let conflicts = br::conflicts(&triples, &|parent, name| {
            Path::new(parent).join(name).exists()
        });
        let rows: Vec<PvRow> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, it)| PvRow {
                idx: i,
                before: it.name.clone(),
                after: news[i].clone(),
                conflict: conflicts.get(i).copied().unwrap_or(Conflict::None),
                changed: news[i] != it.name,
                apply: old_apply.get(&i).copied().unwrap_or(true),
            })
            .collect();
        let sort: Vec<(u32, bool)> = self.rows.sort().to_vec();
        let mut inv = Invalidations::default();
        let mut src = PvSource {
            rows,
            selected: None,
            natural: self.natural,
        };
        src.set_sort(&sort);
        self.rows.replace_source(src, &mut inv);
        let src = self.rows.source();
        self.count = src
            .rows
            .iter()
            .filter(|r| r.changed && r.apply && r.conflict == Conflict::None)
            .count();
        // 충돌 행은 **그 행만** 빠진다(종전에는 하나라도 있으면 [Rename] 전체가 막혔다 — 사용자 10-05 UX 검토).
        self.conflicts = src
            .rows
            .iter()
            .filter(|r| r.conflict != Conflict::None)
            .count();
        self.can_rename = !ops.is_empty() && self.count > 0 && self.error.is_none();
        self.btn_rename.set_enabled(self.can_rename);
        self.redraw();
    }

    /// 결과(DLG-086): 변경 + 적용 + 충돌 없음 → (전체 경로, 새 이름).
    pub(crate) fn result(&self) -> Vec<(PathBuf, String)> {
        self.rows
            .source()
            .rows
            .iter()
            .filter(|r| r.changed && r.apply && r.conflict == Conflict::None)
            .map(|r| {
                let it = &self.items[r.idx];
                (it.parent.join(&it.name), r.after.clone())
            })
            .collect()
    }

    /// 자체 시험 덤프(`bulk.dump`): 상태 줄 + 행마다 `이전 → 이후 [⚠사유] [apply|skip]`.
    pub(crate) fn dump(&self) -> String {
        let mut s = format!(
            "{} items {} cards {} ops {} count {} error {}\n",
            if self.window.is_some() {
                "open"
            } else {
                "closed"
            },
            self.items.len(),
            self.cards.len(),
            self.ops().len(),
            self.count,
            self.error.as_deref().unwrap_or("-")
        );
        for r in &self.rows.source().rows {
            s.push_str(&format!(
                "{} → {}{}{}\n",
                r.before,
                if r.changed { r.after.as_str() } else { "=" },
                if r.conflict == Conflict::None {
                    String::new()
                } else {
                    format!(" ⚠{}", conflict_text(r.conflict))
                },
                if r.changed && r.conflict == Conflict::None {
                    if r.apply {
                        " apply"
                    } else {
                        " skip"
                    }
                } else {
                    ""
                }
            ));
        }
        s.push_str(&format!("presets [{}]\n", self.preset_names.join("|")));
        let kinds: Vec<String> = self.cards.iter().map(Card::kind_value).collect();
        s.push_str(&format!("kinds [{}]\n", kinds.join("|")));
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

    /// 보조 창(프리셋 편집)이 닫힌 뒤 이 창으로 포커스.
    pub(crate) fn focus(&self) {
        if let Some(w) = &self.window {
            w.focus_window();
        }
    }

    fn s(&self, v: f32) -> i32 {
        (v * self.scale).round() as i32
    }

    /// 언어 전환 — 그리드 열 제목(폭은 그대로) · 버튼 글 · 프리셋 콤보 · 창 제목(T-134).
    pub(crate) fn relabel(&mut self) {
        let mut inv = Invalidations::default();
        let mut cols = self.rows.columns().to_vec();
        for c in &mut cols {
            match c.key {
                COL_BEFORE => c.title = tr("bulk.grid.before"),
                COL_AFTER => c.title = tr("bulk.grid.after"),
                COL_APPLY => c.title = tr("bulk.grid.apply"),
                _ => {}
            }
        }
        self.rows.set_columns(cols, &mut inv);
        self.btn_cancel.set_label(tr("bulk.cancel"));
        self.btn_rename.set_label(tr("bulk.rename"));
        // 프리셋 메뉴는 열 때 만든다(라벨 = 그때의 언어).
        if let Some(w) = &self.window {
            w.set_title(&format!("{} — {}", crate::APP_TITLE, tr("bulk.title")));
            w.request_redraw();
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
        let mut attrs = Window::default_attributes()
            .with_title(format!("{} — {}", crate::APP_TITLE, tr("bulk.title")))
            .with_theme(theme)
            .with_resizable(false)
            .with_inner_size(winit::dpi::LogicalSize::new(W, H));
        if let Some((x, y, w, h)) = over {
            let cx = x + (w as i32 - W as i32) / 2;
            let cy = y + (h as i32 - H as i32) / 2;
            attrs = attrs.with_position(winit::dpi::PhysicalPosition::new(cx.max(0), cy.max(0)));
        }
        let attrs = crate::winfocus::owned_by(crate::icon::with_icon(attrs), owner);
        let Ok(win) = el.create_window(attrs) else {
            return;
        };
        let win = Rc::new(win);
        self.scale = win.scale_factor() as f32;
        self.surface = crate::present::Presenter::new(win.clone()).ok();
        win.set_ime_allowed(crate::input::system_ime());
        self.window = Some(win);
        self.layout();
        self.redraw();
    }

    pub(crate) fn close(&mut self) {
        self.surface = None;
        self.window = None;
        self.preset_menu.close();
        for c in &mut self.cards {
            for b in c.boxes() {
                b.set_focused(false);
            }
        }
    }

    /// 틱: 버튼·콤보·텍스트 애니메이션. 다시 그려야 하면 true.
    pub(crate) fn tick(&mut self, now_ms: u64) -> bool {
        if self.window.is_none() {
            return false;
        }
        let mut any =
            self.btn_cancel.tick(now_ms) | self.btn_rename.tick(now_ms) | self.bars.tick(now_ms);
        any |= self.presets_btn.tick(now_ms);
        let now = std::time::Instant::now();
        any |= self.preset_menu.tick(now);
        any |= self.toasts.tick(now);
        for c in &mut self.cards {
            for b in c.buttons() {
                any |= b.tick(now_ms);
            }
            for cb in c.combos() {
                any |= cb.tick_hover(now_ms);
            }
            for tb in c.boxes() {
                any |= tb.tick(now_ms);
            }
        }
        let mut inv = Invalidations::default();
        self.rows.tick(now_ms, &mut inv);
        any | !inv.is_empty() | inv.tick_requested()
    }

    pub(crate) fn animating(&self) -> bool {
        self.window.is_some()
            && (self.btn_cancel.is_animating()
                || self.btn_rename.is_animating()
                || self.presets_btn.is_animating()
                || self.toasts.animating()
                || self.preset_menu.next_wake().is_some())
    }

    /// 배치(DLG-074): 좌 카드 열 320(막대는 열 안 오버레이) + 틈 6 · 우 그리드 · 하단 행.
    fn layout(&mut self) {
        let Some(win) = &self.window else { return };
        let size = win.inner_size();
        let (w, h) = (size.width as i32, size.height as i32);
        let pad = self.s(PAD);
        let bh = self.s(BTN_H);
        let bottom_y = h - pad - bh;
        let mut inv = Invalidations::default();
        self.cards_rect = Rect::new(pad, pad, self.s(CARD_W), bottom_y - pad * 2);
        // 카드 열과 그리드 사이 = 6px(종전 썸 10 + 여백 10 — 막대는 이제 카드 열 안 오버레이 · 사용자 10-18 "여백이 너무 많다").
        let gx = pad + self.s(CARD_W) + self.s(6.0);
        self.grid_rect = Rect::new(gx, pad, w - gx - pad, bottom_y - pad * 2);
        self.rows.set_bounds(self.grid_rect, &mut inv);
        let bw = self.s(88.0);
        for b in [&mut self.btn_rename, &mut self.btn_cancel] {
            b.set_scale(self.scale);
        }
        self.btn_rename
            .set_bounds(Rect::new(w - pad - bw, bottom_y, bw, bh), &mut inv);
        self.btn_cancel.set_bounds(
            Rect::new(w - pad - bw * 2 - pad, bottom_y, bw, bh),
            &mut inv,
        );
        self.presets_btn.set_scale(self.scale);
        self.presets_btn
            .set_bounds(Rect::new(pad, bottom_y, self.s(32.0), bh), &mut inv); // 폭 절반(사용자 10-18)
        self.preset_menu.set_scale(self.scale);
        self.layout_cards();
    }

    /// 카드 스택 배치(스크롤 반영 · 종류별 행 수).
    fn layout_cards(&mut self) {
        let s = self.scale;
        let row_h = self.s(ROW_H);
        let gap = self.s(6.0);
        let inner_pad = self.s(8.0);
        let label_w = self.s(LABEL_W);
        let cr = self.cards_rect;
        let bw = self.s(26.0);
        let w56 = self.s(56.0);
        let mut y = cr.y - self.scroll;
        let mut inv = Invalidations::default();
        let cw = cr.w;
        let mut total = 0;
        for c in &mut self.cards {
            // 종류별로 쓰는 컨트롤만 배치 — 나머지는 빈 bounds(옛 자리 잔존 금지).
            for cb in c.combos() {
                cb.set_bounds(Rect::default(), &mut inv);
            }
            for tb in c.boxes() {
                tb.set_bounds(Rect::default(), &mut inv);
            }
            c.match_case.set_bounds(Rect::default(), &mut inv);
            let rows = 2 + c.body_rows();
            let ch = inner_pad * 2 + rows as i32 * row_h;
            c.rect = Rect::new(cr.x, y, cw, ch);
            let x0 = cr.x + inner_pad;
            let field_x = x0 + label_w;
            let field_w = cw - inner_pad * 2 - label_w;
            let mut ry = y + inner_pad;
            // 행 1: 종류 콤보 + [+][−].
            c.kind.set_scale(s);
            c.kind.set_bounds(
                Rect::new(x0, ry, cw - inner_pad * 2 - bw * 2 - gap * 2, row_h - 4),
                &mut inv,
            );
            c.add.set_scale(s);
            c.add.set_bounds(
                Rect::new(cr.right() - inner_pad - bw * 2 - gap, ry, bw, row_h - 4),
                &mut inv,
            );
            c.del.set_scale(s);
            c.del.set_bounds(
                Rect::new(cr.right() - inner_pad - bw, ry, bw, row_h - 4),
                &mut inv,
            );
            ry += row_h;
            // 행 2: 대상.
            c.scope.set_scale(s);
            c.scope
                .set_bounds(Rect::new(field_x, ry, field_w, row_h - 4), &mut inv);
            ry += row_h;
            let kind = c.kind_value();
            let half = (field_w - gap) / 2;
            let mut place = |ctl: &mut dyn Widget, x: i32, w: i32, ry: i32| {
                ctl.set_bounds(Rect::new(x, ry, w, row_h - 4), &mut inv);
            };
            match kind.as_str() {
                "replace" | "replacerx" => {
                    if kind == "replace" {
                        c.mode.set_scale(s);
                        place(&mut c.mode, field_x, field_w, ry);
                        ry += row_h;
                    }
                    c.match_case.set_scale(s);
                    place(&mut c.match_case, field_x, field_w, ry);
                    ry += row_h;
                    c.find.set_scale(s);
                    place(&mut c.find, field_x, field_w, ry);
                    ry += row_h;
                    c.with.set_scale(s);
                    place(&mut c.with, field_x, field_w, ry);
                }
                "insert" => {
                    c.offset.set_scale(s);
                    place(&mut c.offset, field_x, w56, ry);
                    c.dir.set_scale(s);
                    place(&mut c.dir, field_x + w56 + gap, field_w - w56 - gap, ry);
                    ry += row_h;
                    c.text.set_scale(s);
                    place(&mut c.text, field_x, field_w, ry);
                }
                "case" => {
                    c.case_mode.set_scale(s);
                    place(&mut c.case_mode, field_x, field_w, ry);
                }
                "number" => {
                    c.pad.set_scale(s);
                    place(&mut c.pad, field_x, field_w, ry);
                    ry += row_h;
                    c.offset.set_scale(s);
                    place(&mut c.offset, field_x, w56, ry);
                    c.dir.set_scale(s);
                    place(&mut c.dir, field_x + w56 + gap, field_w - w56 - gap, ry);
                    ry += row_h;
                    c.start.set_scale(s);
                    place(&mut c.start, field_x, half, ry);
                    c.prefix.set_scale(s);
                    place(&mut c.prefix, field_x + half + gap, half, ry);
                    ry += row_h;
                    c.step.set_scale(s);
                    place(&mut c.step, field_x, half, ry);
                    c.suffix.set_scale(s);
                    place(&mut c.suffix, field_x + half + gap, half, ry);
                }
                "date" => {
                    c.date_kind.set_scale(s);
                    place(&mut c.date_kind, field_x, field_w, ry);
                    ry += row_h;
                    c.offset.set_scale(s);
                    place(&mut c.offset, field_x, w56, ry);
                    c.dir.set_scale(s);
                    place(&mut c.dir, field_x + w56 + gap, field_w - w56 - gap, ry);
                    ry += row_h;
                    c.prefix.set_scale(s);
                    place(&mut c.prefix, field_x, half, ry);
                    c.suffix.set_scale(s);
                    place(&mut c.suffix, field_x + half + gap, half, ry);
                    ry += row_h;
                    c.fmt.set_scale(s);
                    place(&mut c.fmt, field_x, field_w, ry);
                }
                _ => {}
            }
            let _ = ry;
            y += ch + gap;
            total += ch + gap;
        }
        self.cards_h = total;
        let n = self.cards.len();
        for (i, c) in self.cards.iter_mut().enumerate() {
            c.del.set_enabled(n > 1 && i < n);
        }
    }

    fn clamp_scroll(&mut self) {
        let max = (self.cards_h - self.cards_rect.h).max(0);
        self.scroll = self.scroll.clamp(0, max);
    }

    /// `p`가 어느 카드의 **바탕**(컨트롤 밖 · 카드 열 안)인가 → 그 카드 번호.
    fn card_background_at(&mut self, p: Point) -> Option<usize> {
        if !self.cards_rect.contains(p) {
            return None;
        }
        let i = self
            .cards
            .iter()
            .position(|c| c.rect.h > 0 && c.rect.contains(p))?;
        let c = &mut self.cards[i];
        let mut hit = c.match_case.bounds().h > 0 && c.match_case.bounds().contains(p);
        hit |= c.buttons().iter().any(|b| b.bounds().contains(p));
        hit |= c
            .combos()
            .iter()
            .any(|cb| cb.bounds().h > 0 && cb.bounds().contains(p));
        hit |= c
            .boxes()
            .iter()
            .any(|tb| tb.bounds().h > 0 && tb.bounds().contains(p));
        (!hit).then_some(i)
    }

    /// 커서 y → 삽입 자리(0..=카드 수): 중심이 y보다 위에 있는 카드 수.
    fn insert_index(&self, y: i32) -> usize {
        self.cards
            .iter()
            .filter(|c| c.rect.y + c.rect.h / 2 < y)
            .count()
    }

    /// 드래그 중 고스트의 위 모서리 y — 고스트가 카드 열과 겹칠 때만 `Some`(= 놓으면 이동 · 아니면 취소).
    fn drag_ghost_top(&self, d: &CardDrag) -> Option<i32> {
        let card = self.cards.get(d.from)?;
        let top = d.cur.1 - (d.press.1 - card.rect.y);
        let ghost = Rect::new(
            d.cur.0 - (d.press.0 - card.rect.x),
            top,
            card.rect.w,
            card.rect.h,
        );
        let hit = ghost.intersection(&self.cards_rect);
        (hit.w > 0 && hit.h > 0).then_some(top)
    }

    /// 삽입 자리의 인디케이터 y(카드 사이 틈 가운데).
    fn insert_y(&self, idx: usize) -> i32 {
        let gap = self.s(6.0);
        match self.cards.get(idx) {
            Some(c) => c.rect.y - gap / 2,
            None => self
                .cards
                .last()
                .map_or(self.cards_rect.y, |c| c.rect.bottom() + gap / 2),
        }
    }

    /// 카드 `from`을 삽입 자리 `to`(0..=카드 수)로. 자리가 같으면 아무것도 안 한다. 움직였으면 `true`.
    pub(crate) fn move_card(&mut self, from: usize, to: usize) -> bool {
        let n = self.cards.len();
        if from >= n || to > n || to == from || to == from + 1 {
            return false;
        }
        let c = self.cards.remove(from);
        let at = if to > from { to - 1 } else { to };
        self.cards.insert(at, c);
        self.sig.clear();
        self.layout_cards();
        self.clamp_scroll();
        self.recompute(true);
        true
    }

    /// 드래그 중 그리기: 열 안 = 삽입 인디케이터(강조색 2px) + 원래 크기 반투명 고스트 · 열 밖 = 축소 고스트(DnD 식별만).
    fn paint_card_drag(&self, dc: &mut dyn DrawCtx, th: &Theme, wi: i32, hi: i32) {
        let Some(d) = self.card_drag.as_ref().filter(|d| d.active) else {
            return;
        };
        let Some(card) = self.cards.get(d.from) else {
            return;
        };
        let (cx, cy) = d.cur;
        let top = self.drag_ghost_top(d);
        let inside = top.is_some();
        let win = Rect::new(0, 0, wi, hi);
        if let Some(top) = top {
            let iy = self.insert_y(self.insert_index(top));
            let cr = self.cards_rect;
            if iy >= cr.y && iy <= cr.bottom() {
                dc.fill_rect(Rect::new(cr.x, iy - 1, cr.w, 2), th.accent);
            }
        }
        let k = if inside { 1.0 } else { CARD_GHOST_SCALE };
        let w = (card.rect.w as f32 * k).round() as i32;
        let h = (card.rect.h as f32 * k).round() as i32;
        // 잡은 점이 카드 안에서 차지하던 비율을 유지해 축소돼도 커서 아래에 머문다.
        let (dx, dy) = (d.press.0 - card.rect.x, d.press.1 - card.rect.y);
        let gx = cx - (dx as f32 * k).round() as i32;
        let gy = cy - (dy as f32 * k).round() as i32;
        let ghost = Rect::new(gx, gy, w, h);
        let r = self.s(6.0);
        dc.fill_round_rect_alpha(ghost, r, th.panel_bg_alt, 0.92);
        dc.stroke_round_rect(ghost, r, th.accent, 1.0);
        // 헤더 글(종류) + 행 틀(필드 자리 윤곽)로 카드 모양을 흉내 낸다.
        let kind = card.kind_value();
        let title = KINDS
            .iter()
            .find(|(v, _)| *v == kind)
            .map(|(_, key)| tr(key))
            .unwrap_or_default();
        dc.select_font(FontSlot::Base, false);
        let th_txt = dc.text_height();
        let inner = (self.s(8.0) as f32 * k).round() as i32;
        let row_h = (self.s(ROW_H) as f32 * k).round() as i32;
        let label_w = (self.s(LABEL_W) as f32 * k).round() as i32;
        let clip = ghost.intersection(&win);
        let head = Rect::new(
            ghost.x + inner,
            ghost.y + inner,
            (ghost.w - inner * 2).max(0),
            (row_h - 4).max(0),
        );
        dc.stroke_round_rect(head, self.s(4.0), th.border, 1.0);
        if head.h >= th_txt {
            dc.text(
                head.x + self.s(6.0),
                head.y + (head.h - th_txt) / 2,
                head.intersection(&clip),
                &title,
                th.text,
            );
        }
        let rows = 1 + card.body_rows() as i32;
        for i in 0..rows {
            let ry = ghost.y + inner + row_h * (i + 1);
            let fr = Rect::new(
                ghost.x + inner + label_w,
                ry,
                (ghost.w - inner * 2 - label_w).max(0),
                (row_h - 4).max(0),
            );
            if fr.bottom() <= ghost.bottom() {
                dc.stroke_round_rect(fr, self.s(4.0), th.border, 1.0);
            }
        }
    }

    /// 스크롤 값 적용(막대·휠·썸 드래그 공용): 카드 재배치 + 열린 카드 콤보는 닫는다(가려진 팝업이 입력을 먹지 않게 · 사용자 10-18).
    fn apply_scroll(&mut self, ny: i32) {
        if ny == self.scroll {
            return;
        }
        self.scroll = ny;
        self.clamp_scroll();
        self.layout_cards();
        let mut inv = Invalidations::default();
        for c in &mut self.cards {
            for cb in c.combos() {
                if cb.is_open() {
                    cb.close(&mut inv);
                }
            }
        }
    }

    /// 열린 콤보(팝업이 모든 입력을 먼저 받는다).
    fn open_combo(&mut self) -> Option<&mut Combo> {
        for c in &mut self.cards {
            for cb in c.combos() {
                if cb.is_open() {
                    return Some(cb);
                }
            }
        }
        None
    }

    /// 우클릭 편집 메뉴가 열린 입력칸(팝업이 모든 입력을 먼저 받는다).
    fn popup_textbox(&mut self) -> Option<&mut TextBox> {
        for c in &mut self.cards {
            for tb in c.boxes() {
                if tb.popup_open() {
                    return Some(tb);
                }
            }
        }
        None
    }

    /// 커서 아래 입력칸(보이는 것만).
    fn box_at(&mut self, p: Point) -> Option<&mut TextBox> {
        if !self.cards_rect.contains(p) {
            return None;
        }
        for c in &mut self.cards {
            if c.rect.h == 0 {
                continue;
            }
            for tb in c.boxes() {
                if tb.bounds().h > 0 && tb.bounds().contains(p) {
                    return Some(tb);
                }
            }
        }
        None
    }

    /// 편집 메뉴·단축키의 클립보드 동작(사용자 10-18 "입력 컨트롤 단축키·우클릭 정상 동작") — 대상 = 포커스 입력칸.
    fn clip(&mut self, act: EditCtxAction) {
        let mut inv = Invalidations::default();
        match act {
            EditCtxAction::Copy => {
                if let Some(text) = self.focused_box().and_then(|tb| tb.copy_selection()) {
                    let _ = crate::clipboard::write_text(&text);
                }
            }
            EditCtxAction::Cut => {
                if let Some(text) = self.focused_box().and_then(|tb| tb.cut_selection(&mut inv)) {
                    let _ = crate::clipboard::write_text(&text);
                }
            }
            EditCtxAction::Paste => {
                if let Some(text) = crate::clipboard::read_text() {
                    if let Some(tb) = self.focused_box() {
                        tb.paste(&text, &mut inv);
                    }
                }
            }
            EditCtxAction::Custom(_) => {}
        }
        self.recompute(false);
        self.redraw();
    }

    fn focused_box(&mut self) -> Option<&mut TextBox> {
        for c in &mut self.cards {
            for tb in c.boxes() {
                if tb.is_focused() {
                    return Some(tb);
                }
            }
        }
        None
    }

    fn own_focus(&mut self, p: Point) {
        let in_cards = self.cards_rect.contains(p);
        for c in &mut self.cards {
            for tb in c.boxes() {
                tb.set_focused(in_cards && tb.bounds().h > 0 && tb.bounds().contains(p));
            }
        }
    }

    /// 카드 [+]/[−] · 종류 변경 · 프리셋 선택 수거 → 구조 변화면 재배치.
    fn collect(&mut self) -> Option<BulkAction> {
        let mut restructure = false;
        let mut add_after: Option<usize> = None;
        let mut del_at: Option<usize> = None;
        for (i, c) in self.cards.iter_mut().enumerate() {
            if c.add.take_clicked() {
                add_after = Some(i);
            }
            if c.del.take_clicked() {
                del_at = Some(i);
            }
            if c.kind.take_changed().is_some() {
                restructure = true;
            }
            for cb in c.combos() {
                let _ = cb.take_changed();
            }
        }
        if let Some(i) = add_after {
            self.cards.insert(i + 1, Card::new(0));
            restructure = true;
        }
        if let Some(i) = del_at {
            if self.cards.len() > 1 {
                self.cards.remove(i);
                restructure = true;
            }
        }
        if restructure {
            self.layout_cards();
            self.clamp_scroll();
        }
        self.recompute(false);
        None
    }

    fn apply_column_x(&self) -> i32 {
        let cols = self.rows.columns();
        let w: i32 = cols.iter().take(3).map(|c| c.width).sum();
        self.grid_rect.x + w - self.rows.scroll_x()
    }

    pub(crate) fn handle(&mut self, ev: &WindowEvent) -> BulkAction {
        let mut inv = Invalidations::default();
        match ev {
            WindowEvent::CloseRequested => return BulkAction::Close,
            WindowEvent::RedrawRequested => return BulkAction::Paint,
            WindowEvent::Resized(_) => {
                self.layout();
                self.redraw();
                return BulkAction::None;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.layout();
                self.redraw();
                return BulkAction::None;
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.primary = if cfg!(target_os = "macos") {
                    m.state().super_key()
                } else {
                    m.state().control_key()
                };
                return BulkAction::None;
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                if let Some(tb) = self.focused_box() {
                    tb.set_preedit("", &mut inv);
                    for ch in text.chars().filter(|c| !c.is_control()) {
                        tb.on_event(&InputEvent::Char { c: ch, now_ms: 0 }, &mut inv);
                    }
                }
                self.recompute(false);
                self.redraw();
                return BulkAction::None;
            }
            WindowEvent::Ime(Ime::Preedit(text, _)) => {
                if let Some(tb) = self.focused_box() {
                    tb.set_preedit(text, &mut inv);
                }
                self.redraw();
                return BulkAction::None;
            }
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                if self.preset_menu.is_open() {
                    let k = match kev.logical_key.as_ref() {
                        Key::Named(NamedKey::Escape) => {
                            self.preset_menu.close();
                            self.redraw();
                            return BulkAction::None;
                        }
                        Key::Named(NamedKey::ArrowUp) => Some(CtlKey::Up),
                        Key::Named(NamedKey::ArrowDown) => Some(CtlKey::Down),
                        Key::Named(NamedKey::Enter) => Some(CtlKey::Enter),
                        _ => None,
                    };
                    if let Some(key) = k {
                        self.preset_menu.on_event(&InputEvent::Key {
                            key,
                            shift: false,
                            primary: false,
                        });
                        self.redraw();
                        if let Some(id) = self.preset_menu.take_picked() {
                            if let Some(a) = self.preset_pick(&id) {
                                return a;
                            }
                        }
                    }
                    return BulkAction::None;
                }
                if matches!(kev.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
                    if let Some(tb) = self.popup_textbox() {
                        tb.on_event(
                            &InputEvent::Key {
                                key: CtlKey::Escape,
                                shift: false,
                                primary: false,
                            },
                            &mut inv,
                        );
                        self.redraw();
                        return BulkAction::None;
                    }
                    if self.card_drag.take().is_some_and(|d| d.active) {
                        self.redraw();
                        return BulkAction::None;
                    }
                    if let Some(cb) = self.open_combo() {
                        cb.close(&mut inv);
                        self.redraw();
                        return BulkAction::None;
                    }
                    return BulkAction::Close;
                }
                // Enter = [Rename](버튼이 켜져 있을 때 — 입력칸은 한 줄이라 Enter에 다른 뜻이 없다).
                if matches!(kev.logical_key.as_ref(), Key::Named(NamedKey::Enter))
                    && self.open_combo().is_none()
                {
                    if self.can_rename {
                        return BulkAction::Rename(self.result());
                    }
                    return BulkAction::None;
                }
                // Space = 선택한 미리보기 행의 적용 토글(입력칸에 포커스가 없을 때).
                if matches!(kev.logical_key.as_ref(), Key::Named(NamedKey::Space))
                    && self.focused_box().is_none()
                {
                    if self.toggle_selected() {
                        self.redraw();
                    }
                    return BulkAction::None;
                }
                if self.primary {
                    // 편집 단축키(사용자 10-18): ⌘/Ctrl+C·X·V·Z·Y·A — 포커스 입력칸.
                    match crate::input::shortcut_letter(kev) {
                        Some('c') => {
                            self.clip(EditCtxAction::Copy);
                            return BulkAction::None;
                        }
                        Some('x') => {
                            self.clip(EditCtxAction::Cut);
                            return BulkAction::None;
                        }
                        Some('v') => {
                            self.clip(EditCtxAction::Paste);
                            return BulkAction::None;
                        }
                        Some(l @ ('z' | 'y' | 'a')) => {
                            let ev = match l {
                                'z' if self.shift => InputEvent::Redo,
                                'z' => InputEvent::Undo,
                                'y' => InputEvent::Redo,
                                _ => InputEvent::SelectAll,
                            };
                            if let Some(tb) = self.focused_box() {
                                tb.on_event(&ev, &mut inv);
                            }
                            self.recompute(false);
                            self.redraw();
                            return BulkAction::None;
                        }
                        _ => {}
                    }
                }
                if let Some(e) = crate::input::text_key_event(
                    kev,
                    self.shift,
                    self.primary,
                    crate::input::TextKeys::Line,
                ) {
                    if let Some(tb) = self.focused_box() {
                        tb.on_event(&e, &mut inv);
                        self.recompute(false);
                    } else {
                        self.rows.on_event(&e, &mut inv);
                    }
                    self.redraw();
                }
                return BulkAction::None;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (x, y) = self.cursor;
                let p = Point { x, y };
                let we = crate::input::wheel_event(delta, self.shift);
                if let Some(tb) = self.box_at(p) {
                    // 입력칸 위 휠 = 그 입력칸(긴 글 가로 스크롤 · 사용자 10-18).
                    tb.on_event(&we, &mut inv);
                    self.redraw();
                    return BulkAction::None;
                }
                if self.cards_rect.contains(p) {
                    // 카드 열 휠 = ScrollBars(고속 스크롤 · 속도 HUD · 막대 표시 — 설정 창과 같은 꼴 · 사용자 10-18).
                    let (_, ny, _) = self.bars.on_event(
                        &we,
                        self.cards_rect,
                        self.cards_rect.w,
                        self.cards_h.max(self.cards_rect.h),
                        0,
                        self.scroll,
                        self.scale,
                    );
                    self.apply_scroll(ny);
                } else if self.grid_rect.contains(p) {
                    let e = crate::input::wheel_event(delta, self.shift);
                    self.rows.on_event(&e, &mut inv);
                }
                self.redraw();
                return BulkAction::None;
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as i32, position.y as i32);
                let (x, y) = self.cursor;
                let mv = InputEvent::MouseMove { x, y };
                if self.preset_menu.is_open() {
                    if self.preset_menu.on_event(&mv) {
                        self.redraw();
                    }
                    return BulkAction::None;
                }
                if let Some(tb) = self.popup_textbox() {
                    tb.on_event(&mv, &mut inv);
                    self.redraw();
                    return BulkAction::None;
                }
                if let Some(d) = self.card_drag.as_mut() {
                    d.cur = (x, y);
                    if !d.active
                        && ((x - d.press.0).abs() > CARD_DRAG_THRESHOLD
                            || (y - d.press.1).abs() > CARD_DRAG_THRESHOLD)
                    {
                        d.active = true;
                    }
                    if d.active {
                        self.redraw();
                        return BulkAction::None;
                    }
                }
                if self.bars.is_dragging() || self.cards_rect.contains(Point { x, y }) {
                    let (_, ny, consumed) = self.bars.on_event(
                        &mv,
                        self.cards_rect,
                        self.cards_rect.w,
                        self.cards_h.max(self.cards_rect.h),
                        0,
                        self.scroll,
                        self.scale,
                    );
                    self.apply_scroll(ny);
                    if consumed {
                        self.redraw();
                        return BulkAction::None;
                    }
                }
                if let Some(cb) = self.open_combo() {
                    cb.on_event(&mv, &mut inv);
                } else {
                    // 카드 열 밖(스크롤로 가려진 자리 · 하단 줄)에서는 카드 컨트롤에 '벗어남' 좌표를 준다 — 가려진 컨트롤이 hover를
                    // 받지 않는다(사용자 10-18 "뒤에 숨은 객체가 이벤트를 먼저 받음").
                    let cmv = if self.cards_rect.contains(Point { x, y }) {
                        mv
                    } else {
                        InputEvent::MouseMove { x: -1, y: -1 }
                    };
                    for c in &mut self.cards {
                        if c.rect.h == 0 {
                            continue;
                        }
                        for b in c.buttons() {
                            b.on_event(&cmv, &mut inv);
                        }
                        for cb in c.combos() {
                            cb.on_event(&cmv, &mut inv);
                        }
                        for tb in c.boxes() {
                            tb.on_event(&cmv, &mut inv);
                        }
                        c.match_case.on_event(&cmv, &mut inv);
                    }
                    self.presets_btn.on_event(&mv, &mut inv);
                    self.btn_cancel.on_event(&mv, &mut inv);
                    self.btn_rename.on_event(&mv, &mut inv);
                    self.rows.on_event(&mv, &mut inv);
                }
                if !inv.is_empty() {
                    self.redraw();
                }
                return BulkAction::None;
            }
            WindowEvent::MouseInput { state, button, .. }
                if *button == MouseButton::Right && *state == ElementState::Pressed =>
            {
                let (x, y) = self.cursor;
                let p = Point { x, y };
                let e = InputEvent::RightDown { x, y };
                if self.preset_menu.is_open() {
                    self.preset_menu.close();
                }
                if let Some(tb) = self.popup_textbox() {
                    tb.on_event(&e, &mut inv);
                    self.redraw();
                    return BulkAction::None;
                }
                // 우클릭 = 입력칸 편집 메뉴(복사·잘라내기·붙여넣기·전체 선택 — TextBox 내장 · 사용자 10-18).
                let has = crate::clipboard::read_text().is_some_and(|s| !s.is_empty());
                for c in &mut self.cards {
                    for tb in c.boxes() {
                        tb.set_clipboard_has_text(has);
                    }
                }
                if self.box_at(p).is_some() {
                    self.own_focus(p);
                    if let Some(tb) = self.box_at(p) {
                        tb.on_event(&e, &mut inv);
                    }
                    self.redraw();
                }
                return BulkAction::None;
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
                if let Some(tb) = self.popup_textbox() {
                    tb.on_event(&e, &mut inv);
                    let act = tb.take_edit_ctx();
                    let still_open = tb.popup_open();
                    if let Some(act) = act {
                        self.clip(act);
                        return BulkAction::None;
                    }
                    if still_open || up {
                        self.redraw();
                        return BulkAction::None;
                    }
                }
                if !up && self.toasts.click(p) {
                    self.redraw();
                    return BulkAction::None;
                }
                if self.preset_menu.is_open() {
                    if self.preset_menu.is_outside_click(&e) {
                        self.preset_menu.close();
                        self.redraw();
                        return BulkAction::None;
                    }
                    self.preset_menu.on_event(&e);
                    self.redraw();
                    if let Some(id) = self.preset_menu.take_picked() {
                        if let Some(a) = self.preset_pick(&id) {
                            return a;
                        }
                    }
                    return BulkAction::None;
                }
                if up {
                    if let Some(d) = self.card_drag.take() {
                        if d.active {
                            // 판정 = 고스트(끌고 있는 카드) 위 모서리(커서가 아니다 — 카드 아래쪽을 잡으면 커서가 1번 카드 중심 위까지
                            // 못 올라가 맨 앞으로 옮길 수 없었다 · 사용자 10-18).
                            if let Some(top) = self.drag_ghost_top(&d) {
                                let to = self.insert_index(top);
                                self.move_card(d.from, to);
                            }
                            self.redraw();
                            return BulkAction::None;
                        }
                    }
                }
                if self.open_combo().is_none() && (up || self.cards_rect.contains(p)) {
                    let (_, ny, consumed) = self.bars.on_event(
                        &e,
                        self.cards_rect,
                        self.cards_rect.w,
                        self.cards_h.max(self.cards_rect.h),
                        0,
                        self.scroll,
                        self.scale,
                    );
                    self.apply_scroll(ny);
                    if consumed {
                        self.redraw();
                        return BulkAction::None;
                    }
                }
                if let Some(cb) = self.open_combo() {
                    cb.on_event(&e, &mut inv);
                    self.redraw();
                    if let Some(a) = self.collect() {
                        return a;
                    }
                    return BulkAction::None;
                }
                if !up {
                    self.own_focus(p);
                    // 카드 바탕(컨트롤 밖) 누름 = 드래그 이동 후보(사용자 10-18).
                    if let Some(i) = self.card_background_at(p) {
                        self.card_drag = Some(CardDrag {
                            from: i,
                            press: (x, y),
                            cur: (x, y),
                            active: false,
                        });
                    }
                    // 적용 열 클릭 = 그 행 토글(DLG-080).
                    if self.grid_rect.contains(p) && x >= self.apply_column_x() {
                        if let Some(row) = self.rows.row_at(x, y) {
                            let src = self.rows.source_mut();
                            if let Some(r) = src.rows.get_mut(row) {
                                // 충돌 행도 뺄 수 있다(빼면 겹치던 상대가 풀린다).
                                if r.changed {
                                    r.apply = !r.apply;
                                }
                            }
                            self.recompute(false);
                            self.redraw();
                            return BulkAction::None;
                        }
                    }
                }
                // 누름 = 커서 아래 컨트롤만(카드 열 안에서만 — 스크롤로 가려진 컨트롤은 받지 않는다 · 사용자 10-18) · 뗌 = 전부.
                let in_cards = self.cards_rect.contains(p);
                for c in &mut self.cards {
                    if c.rect.h == 0 {
                        continue;
                    }
                    for b in c.buttons() {
                        if up || (in_cards && b.bounds().contains(p)) {
                            b.on_event(&e, &mut inv);
                        }
                        b.set_focused(false);
                    }
                    for cb in c.combos() {
                        if up || (in_cards && cb.bounds().contains(p)) {
                            cb.on_event(&e, &mut inv);
                        }
                    }
                    for tb in c.boxes() {
                        if up || (in_cards && tb.bounds().contains(p)) {
                            tb.on_event(&e, &mut inv);
                        }
                    }
                    if up || (in_cards && c.match_case.bounds().contains(p)) {
                        c.match_case.on_event(&e, &mut inv);
                    }
                    let _ = c.match_case.take_toggled();
                }
                if up || self.presets_btn.bounds().contains(p) {
                    self.presets_btn.on_event(&e, &mut inv);
                    self.presets_btn.set_focused(false);
                }
                for b in [&mut self.btn_cancel, &mut self.btn_rename] {
                    if up || b.bounds().contains(p) {
                        b.on_event(&e, &mut inv);
                    }
                    b.set_focused(false);
                }
                if self.grid_rect.contains(p) || up {
                    self.rows.on_event(&e, &mut inv);
                }
                self.redraw();
                if self.presets_btn.take_clicked() {
                    self.open_preset_menu();
                    self.redraw();
                    return BulkAction::None;
                }
                if self.btn_cancel.take_clicked() {
                    return BulkAction::Close;
                }
                if self.btn_rename.take_clicked() && self.btn_rename.base().enabled {
                    return BulkAction::Rename(self.result());
                }
                if let Some(a) = self.collect() {
                    return a;
                }
                return BulkAction::None;
            }
            _ => {}
        }
        BulkAction::None
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
        let pad = self.s(PAD);
        {
            let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
            let mut prefs = FontPrefs::with_base(font_px);
            prefs.peerlist = prefs.base;
            let mut dc = RasterCtx::new(&mut gfx, ui, s).with_fonts(prefs);
            dc.fill_rect(Rect::new(0, 0, wi, hi), th.window_bg);
            dc.select_font(FontSlot::Base, false);
            let th_txt = dc.text_height();
            let row_h = self.s(ROW_H);
            // 그리드 지표(글꼴 + 4).
            let grid_row = th_txt + self.s(4.0);
            let mut inv = Invalidations::default();
            self.rows
                .set_metrics(grid_row, self.s(8.0), self.s(16.0), &mut inv);
            // 카드 열(클립 = 카드 영역).
            let cr = self.cards_rect;
            dc.fill_rect(cr, th.panel_bg);
            let label_w = self.s(LABEL_W);
            let inner_pad = self.s(8.0);
            for c in &self.cards {
                let r = c.rect;
                if r.bottom() < cr.y || r.y > cr.bottom() {
                    continue;
                }
                let clip = r.intersection(&cr);
                dc.fill_round_rect(clip, self.s(6.0), th.panel_bg_alt);
                // 헤더 띠(종류 콤보 행 · Path Finder 꼴 · 사용자 10-18): 패널과 대비되는 `bubble_peer` · 위 모서리만 둥글게 · 아래 경계선.
                let rad = self.s(6.0);
                let head =
                    Rect::new(r.x, r.y, r.w, inner_pad + row_h - self.s(2.0)).intersection(&cr);
                if head.h > 0 {
                    dc.fill_round_rect(head, rad, th.bubble_peer);
                    dc.fill_rect(
                        Rect::new(head.x, head.y + rad, head.w, (head.h - rad).max(0)),
                        th.bubble_peer,
                    );
                    dc.fill_rect(Rect::new(head.x, head.bottom() - 1, head.w, 1), th.border);
                }
                dc.stroke_round_rect(clip, self.s(6.0), th.border, 1.0);
                let lx = r.x + inner_pad;
                let ty = |ry: i32| ry + (row_h - 4 - th_txt) / 2;
                let kind = c.kind_value();
                let mut ry = r.y + inner_pad + row_h;
                // 다른 라벨과 같이 오른쪽 정렬(사용자 10-18 — 종전 왼쪽 정렬).
                let apply_to = tr("bulk.lbl.applyTo");
                let aw = dc.text_width(&apply_to);
                dc.text(
                    lx + label_w - self.s(6.0) - aw,
                    ty(ry),
                    clip,
                    &apply_to,
                    th.text_dim,
                );
                ry += row_h;
                let labels: Vec<&str> = match kind.as_str() {
                    "replace" => vec![
                        "bulk.lbl.mode",
                        "bulk.lbl.matchCase",
                        "bulk.lbl.find",
                        "bulk.lbl.with",
                    ],
                    "replacerx" => vec!["bulk.lbl.matchCase", "bulk.lbl.findRx", "bulk.lbl.with"],
                    "insert" => vec!["bulk.lbl.pos", "bulk.lbl.text"],
                    "case" => vec!["bulk.lbl.caseTo"],
                    "number" => vec![
                        "bulk.lbl.padding",
                        "bulk.lbl.pos",
                        "bulk.lbl.start",
                        "bulk.lbl.step",
                    ],
                    "date" => vec![
                        "bulk.lbl.type",
                        "bulk.lbl.pos",
                        "bulk.lbl.wrap",
                        "bulk.lbl.fmt",
                    ],
                    _ => vec![],
                };
                for k in labels {
                    let t = tr(k);
                    let tw = dc.text_width(&t);
                    dc.text(
                        lx + label_w - self.s(6.0) - tw,
                        ty(ry),
                        clip,
                        &t,
                        th.text_dim,
                    );
                    ry += row_h;
                }
            }
            // 컨트롤(콤보는 닫힌 것만 · 열린 것은 맨 뒤).
            for c in &self.cards {
                if c.rect.bottom() < cr.y || c.rect.y > cr.bottom() {
                    continue;
                }
                c.add.paint(&mut dc, th);
                c.del.paint(&mut dc, th);
                for cb in [
                    &c.kind,
                    &c.scope,
                    &c.mode,
                    &c.dir,
                    &c.case_mode,
                    &c.pad,
                    &c.date_kind,
                ] {
                    if cb.bounds().h > 0 && !cb.is_open() && cb.bounds().y >= cr.y - row_h {
                        cb.paint(&mut dc, th);
                    }
                }
                for tb in [
                    &c.find, &c.with, &c.offset, &c.text, &c.start, &c.step, &c.prefix, &c.suffix,
                    &c.fmt,
                ] {
                    if tb.bounds().h > 0 {
                        tb.paint(&mut dc, th);
                    }
                }
                if c.match_case.bounds().h > 0 {
                    c.match_case.paint(&mut dc, th);
                }
            }
            // 카드 영역 위/아래 바깥은 창 배경으로 덮는다(스크롤 잘림).
            dc.fill_rect(Rect::new(cr.x, 0, cr.w, cr.y), th.window_bg);
            dc.fill_rect(
                Rect::new(cr.x, cr.bottom(), cr.w, hi - cr.bottom()),
                th.window_bg,
            );
            self.bars.paint(
                &mut dc,
                th,
                cr,
                cr.w,
                self.cards_h.max(cr.h),
                0,
                self.scroll,
                s,
            );
            // 그리드.
            self.rows.paint(&mut dc, th);
            // 하단: 프리셋 콤보 · 건수/오류 · 버튼.
            self.presets_btn.paint(&mut dc, th);
            let items = self.preset_menu_items();
            self.menu_text_w = items
                .iter()
                .filter_map(|it| match it {
                    CtxItem::Item { label, .. } => Some(dc.text_width(label)),
                    CtxItem::Separator => None,
                })
                .max()
                .unwrap_or(0);
            let bottom_y = hi - pad - self.s(BTN_H);
            let info = self.error.clone().unwrap_or_else(|| self.count_text());
            let ix = self.presets_btn.bounds().right() + pad;
            let iw = (self.btn_cancel.bounds().x - pad - ix).max(0);
            let info = nexa_ctl::draw::ellipsize_middle(&mut dc, &info, iw);
            dc.text(
                ix,
                bottom_y + (self.s(BTN_H) - th_txt) / 2,
                Rect::new(ix, bottom_y, iw, self.s(BTN_H)),
                &info,
                if self.error.is_some() {
                    th.danger
                } else {
                    th.text_dim
                },
            );
            self.btn_cancel.paint(&mut dc, th);
            self.btn_rename.paint(&mut dc, th);
            // 열린 콤보 드롭다운 = 맨 뒤(카드가 열 안에 보일 때만 — 가려진 카드의 상자가 하단 줄 위에 그려졌다 · 10-18).
            for c in &self.cards {
                if c.rect.bottom() <= cr.y || c.rect.y >= cr.bottom() {
                    continue;
                }
                for cb in [
                    &c.kind,
                    &c.scope,
                    &c.mode,
                    &c.dir,
                    &c.case_mode,
                    &c.pad,
                    &c.date_kind,
                ] {
                    if cb.is_open() {
                        cb.paint(&mut dc, th);
                        cb.paint_dropdown(&mut dc, th);
                    }
                }
            }
            self.paint_card_drag(&mut dc, th, wi, hi);
            // 토스트 영역 = 하단 버튼 줄 제외(버튼을 가리지 않게 · 사용자 10-18).
            let toast_area = Rect::new(0, 0, wi, (hi - pad * 2 - self.s(BTN_H)).max(0));
            self.toasts
                .paint_in(&mut dc, th, toast_area, self.toast_corner, s);
            // 프리셋 메뉴 = 맨 뒤(모든 컨트롤 위).
            self.preset_menu.paint(&mut dc, th);
        }
        for c in &self.cards {
            for tb in [
                &c.find, &c.with, &c.offset, &c.text, &c.start, &c.step, &c.prefix, &c.suffix,
                &c.fmt,
            ] {
                if tb.popup_open() {
                    let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
                    let prefs = FontPrefs::with_base(font_px);
                    let mut dc = RasterCtx::new(&mut gfx, ui, s).with_fonts(prefs);
                    tb.paint_popup(&mut dc, th);
                }
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

    /// 이름 부분 정규식 치환 규칙.
    fn rx(find: &str, with: &str) -> RenameOp {
        RenameOp::Replace {
            scope: br::Scope::Name,
            find: find.into(),
            with: with.into(),
            match_case: true,
            regex: true,
            mode: br::ReplaceMode::All,
        }
    }

    fn item(dir: &Path, name: &str) -> BulkItem {
        BulkItem {
            parent: dir.to_path_buf(),
            name: name.into(),
            is_dir: false,
            modified_ms: 1_700_000_000_000,
            created_ms: 0,
        }
    }

    /// 창 없이: 기본 카드(찾기 빈 값 = 무효) → 변경 0 · Rename 비활성 → 찾기/바꾸기 입력 → 미리보기 · 건수 · 결과 · 적용 해제 · 충돌(중복) · 프리셋 왕복.
    #[test]
    fn preview_count_conflicts_and_presets_without_window() {
        en();
        let dir = std::env::temp_dir().join(format!("ndir-bulk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("taken.txt"), b"").unwrap();
        let mut w = BulkWin::new();
        w.set_items(
            vec![
                item(&dir, "photo_a.jpg"),
                item(&dir, "photo_b.jpg"),
                item(&dir, "note.txt"),
            ],
            0,
        );
        assert_eq!(w.items_len(), 3);
        assert!(w.ops().is_empty(), "찾기 빈 값 = 무효 카드");
        assert_eq!(w.count, 0);
        assert!(!w.btn_rename.base().enabled);
        assert!(w
            .dump()
            .starts_with("closed items 3 cards 1 ops 0 count 0 error -\n"));
        w.card_mut(0).find.set_text("photo");
        w.card_mut(0).with.set_text("img");
        w.recompute(false);
        assert_eq!(w.ops().len(), 1);
        assert_eq!(w.count, 2);
        assert!(w.btn_rename.base().enabled);
        let d = w.dump();
        assert!(
            d.contains("photo_a.jpg → img_a.jpg apply") && d.contains("note.txt → ="),
            "{d}"
        );
        assert_eq!(
            w.result(),
            vec![
                (dir.join("photo_a.jpg"), "img_a.jpg".to_string()),
                (dir.join("photo_b.jpg"), "img_b.jpg".to_string())
            ]
        );
        // 적용 해제(행 0) = 건수 1.
        w.rows.source_mut().rows[0].apply = false;
        w.recompute(false);
        assert_eq!(w.count, 1);
        assert!(w.dump().contains("photo_a.jpg → img_a.jpg skip"));
        // 중복 충돌: 전체 교체 → 둘 다 "same" → Duplicate · 기존 파일 존재 → Exists(뺀 행은 충돌 계산에서 빠지므로 다시 넣는다).
        w.rows.source_mut().rows[0].apply = true;
        w.card_mut(0).mode.select_value("entire");
        w.card_mut(0).with.set_text("same");
        w.recompute(false);
        assert!(w.dump().contains("⚠duplicate"), "{}", w.dump());
        assert!(
            !w.btn_rename.base().enabled,
            "바꿀 수 있는 행이 없음 = 비활성"
        );
        w.card_mut(0).scope.select_value("nameext");
        w.card_mut(0).with.set_text("taken.txt");
        w.recompute(false);
        assert!(
            w.dump().contains("⚠already exists") || w.dump().contains("⚠duplicate"),
            "{}",
            w.dump()
        );
        // 검증 오류: 잘못된 정규식.
        w.card_mut(0).kind.select_value("replacerx");
        w.card_mut(0).find.set_text("(");
        w.recompute(false);
        assert!(
            w.error.as_deref().is_some_and(|e| e.starts_with("⚠ #1:")),
            "{:?}",
            w.error
        );
        // 프리셋 왕복: 번호 + 대문자 → 직렬화 → 다시 로드 → 같은 ops.
        let ops = vec![
            RenameOp::Number {
                scope: Scope::Name,
                spec: NumberSpec {
                    start: 5,
                    step: 2,
                    pad: 4,
                    at: InsertAt {
                        offset: 0,
                        from_end: false,
                    },
                    prefix: "[".into(),
                    suffix: "]".into(),
                },
            },
            RenameOp::Case {
                scope: Scope::Ext,
                mode: CaseMode::Upper,
            },
        ];
        w.load_ops(&ops);
        assert_eq!(w.cards.len(), 2);
        assert_eq!(w.ops(), ops);
        let text = br::serialize_ops(&w.ops());
        w.load_ops(&br::parse_ops(&text));
        assert_eq!(w.ops(), ops, "프리셋 직렬화 왕복");
        assert!(
            w.dump().contains("photo_a.jpg → [0005]photo_a.JPG"),
            "{}",
            w.dump()
        );
        // 카드 드래그 이동(사용자 10-18): 삽입 자리 = 카드 중심 기준 · 같은 자리 = 무변화 · 이동 뒤 파이프라인 순서가 바뀐다.
        w.load_ops(&ops);
        assert!(w.dump().contains("kinds [number|case]"), "{}", w.dump());
        let mid0 = w.cards[0].rect.y + w.cards[0].rect.h / 2;
        assert_eq!(w.insert_index(mid0 - 1), 0);
        assert_eq!(w.insert_index(mid0 + 1), 1);
        assert_eq!(w.insert_index(i32::MAX), 2);
        assert!(
            !w.move_card(0, 0) && !w.move_card(0, 1),
            "같은 자리 = 무변화"
        );
        assert!(w.move_card(0, 2));
        assert!(w.dump().contains("kinds [case|number]"), "{}", w.dump());
        assert_eq!(w.ops(), vec![ops[1].clone(), ops[0].clone()]);
        assert!(w.move_card(1, 0));
        assert_eq!(w.ops(), ops);
        // 프리셋 저장 = 카드 전부(빈 찾기 카드 포함 · 사용자 10-18): 유효 2 + 빈 치환 1 → 3장으로 돌아온다.
        w.load_ops(&ops);
        w.cards.push(Card::new(0));
        assert_eq!((w.ops().len(), w.ops_all().len()), (2, 3));
        let saved = match w.preset_pick("save") {
            Some(BulkAction::SavePreset(t)) => t,
            other => panic!("{other:?}"),
        };
        w.load_ops(&br::parse_ops(&saved));
        assert_eq!(w.cards.len(), 3, "무효 카드도 저장·복원");
        assert_eq!(w.ops(), ops);
        w.load_ops(&[]);
        assert_eq!(w.cards.len(), 1, "빈 프리셋 = 기본 카드 1장");
        w.set_presets(vec!["zeta".into(), "Alpha".into()]);
        assert_eq!(
            w.preset_names,
            vec!["zeta".to_string(), "Alpha".to_string()],
            "순서는 호스트(order.txt)가 정한다"
        );
        let ids = |w: &BulkWin| -> Vec<String> {
            w.preset_menu_items()
                .iter()
                .map(|it| match it {
                    CtxItem::Item { id, enabled, .. } => {
                        format!("{id}{}", if *enabled { "" } else { "!" })
                    }
                    CtxItem::Separator => "-".into(),
                })
                .collect()
        };
        assert_eq!(
            ids(&w),
            [
                "hdr!",
                "-",
                "preset:zeta",
                "preset:Alpha",
                "-",
                "save",
                "edit",
                "-",
                "clear"
            ],
            "헤더(클릭 없음) · ─ · 프리셋들 · ─ · 저장 · 편집 · ─ · 모두 제거"
        );
        w.set_presets(Vec::new());
        assert_eq!(
            ids(&w),
            ["hdr!", "-", "save", "edit", "-", "clear!"],
            "없으면 목록 구간 없음 · 모두 제거 끔"
        );
        assert!(
            matches!(w.preset_pick("save"), Some(BulkAction::SavePreset(_))),
            "기본 카드 1장(무효)이어도 저장 요청은 나간다(빈 파이프라인)"
        );
        assert_eq!(w.preset_pick("clear"), Some(BulkAction::ClearPresets));
        assert_eq!(
            w.preset_pick("preset:zeta"),
            Some(BulkAction::LoadPreset("zeta".into()))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 충돌 행은 그 행만 빠진다(전체를 막지 않는다) · 건수 줄에 건너뛰는 수 · 겹치는 둘 중 하나를 빼면 나머지가 풀린다 ·
    /// 맞바꾸기는 충돌이 아니다 · Space = 선택 행 토글 · 열자마자 첫 입력칸 포커스.
    #[test]
    fn conflict_rows_skip_only_themselves() {
        en();
        let dir = std::env::temp_dir().join(format!("ndir-bulk-skip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for n in ["a1.txt", "a2.txt", "b.txt", "x.txt", "y.txt"] {
            std::fs::write(dir.join(n), b"").unwrap();
        }
        let mut w = BulkWin::new();
        w.set_items(
            vec![
                item(&dir, "a1.txt"),
                item(&dir, "a2.txt"),
                item(&dir, "b.txt"),
            ],
            0,
        );
        assert!(w.focused_box().is_some(), "열자마자 첫 입력칸");
        // a1 · a2 → 둘 다 "a.txt"(중복) · b → "c.txt"(정상).
        w.load_ops(&[rx("^a\\d$", "a"), rx("^b$", "c")]);
        assert_eq!((w.count, w.conflicts), (1, 2));
        assert!(w.can_rename, "충돌이 있어도 나머지는 바꿀 수 있다");
        assert!(w.count_text().contains('2'), "{}", w.count_text());
        assert_eq!(w.result().len(), 1);
        // a2를 뺀다(Space) → a1이 풀린다.
        w.rows.source_mut().selected = Some(1);
        assert!(w.toggle_selected());
        assert_eq!((w.count, w.conflicts), (2, 0), "{}", w.dump());
        let names: Vec<String> = w.result().into_iter().map(|(_, n)| n).collect();
        assert_eq!(names, vec!["a.txt".to_string(), "c.txt".to_string()]);
        // 맞바꾸기 x ↔ y = 충돌 아님.
        w.set_items(vec![item(&dir, "x.txt"), item(&dir, "y.txt")], 0);
        w.load_ops(&[rx("^x$", "tmp"), rx("^y$", "x"), rx("^tmp$", "y")]);
        assert_eq!((w.count, w.conflicts), (2, 0));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
