//! 일괄 이름 변경 창(T-71 · dir2 `bulkrename.rs` 이식 — docs/port/16 DLG-074~088): 좌 = **카드 스택**(동작 1블록 = 카드 1장 · 위→아래 파이프라인 ·
//! 종류 콤보 6종 + [+]/[−] · 마지막 1장 삭제 불가) · 우 = **미리보기 그리드**(nexa-grid · # · 이전 · 이후(⚠ 사유) · 적용 ✓) · 하단 = 프리셋 콤보 `[… ⌄]` ·
//! 건수/검증 오류 · [취소][Rename]. 코어는 `ndir_ops::batch_rename`(수확 규칙 DLG-077 · 실시간 미리보기 DLG-079 · 충돌 · 프리셋 직렬화).
//! 창 880×620 고정 · 소유자 중앙. [Rename] 활성 = 동작 ≥ 1 · 변경 ≥ 1 · 검증 통과 · 충돌 0. 결과 = `(전체 경로, 새 이름)` 목록 → 호스트가 순차 rename + undo 1건.

use ndir_i18n::{tr, trf};
use ndir_ops::batch_rename::{
    self as br, CaseMode, Conflict, DateKind, DateSpec, InsertAt, NumberSpec, RenameInput,
    RenameOp, ReplaceMode, Scope,
};
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::{Point, Rect};
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::{
    Button, Checkbox, Combo, ComboControl, ComboItem, Control, InputEvent, Invalidations, TextBox,
    Widget,
};
use nexa_gfx::{Font, Surface};
use nexa_grid::{Column, Marker, RowItem, RowSource, SelectOp, VirtualRows};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent};
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
    /// 프리셋 관리 요청(호스트가 폴더/목록 처리).
    EditPresets,
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
                if find.is_empty() && mode != ReplaceMode::Entire {
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
                if find.is_empty() {
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
                if text.is_empty() {
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
                } else if r.changed {
                    r.after.clone()
                } else {
                    String::new()
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
    /// 정렬(DLG-081): 이전/이후 기준 대소문자 무시 · 안정 · 다중 키 · 빈 사양 = 선택 순서.
    fn set_sort(&mut self, keys: &[(u32, bool)]) -> bool {
        if keys.is_empty() {
            self.rows.sort_by_key(|r| r.idx);
            return true;
        }
        self.rows.sort_by(|a, b| {
            for &(k, desc) in keys {
                let (x, y) = match k {
                    COL_BEFORE => (a.before.to_lowercase(), b.before.to_lowercase()),
                    COL_AFTER => (a.after.to_lowercase(), b.after.to_lowercase()),
                    _ => (a.idx.to_string(), b.idx.to_string()),
                };
                let o = if k == COL_NO {
                    a.idx.cmp(&b.idx)
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
    presets: Combo,
    preset_names: Vec<String>,
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
            presets: Combo::new(vec![ComboItem::new("", "…")], 0),
            preset_names: Vec::new(),
            btn_cancel: Button::new(tr("bulk.cancel")),
            btn_rename: Button::new(tr("bulk.rename")),
            error: None,
            count: 0,
            conflicts: 0,
            can_rename: false,
            sig: String::new(),
            tz_min: 0,
        };
        w.set_presets(Vec::new());
        w
    }

    /// 대상 교체(열 때) — 카드는 기본 1장 · 선택 순서 보존.
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

    /// 프리셋 목록(이름순 · 최대 64) → `[… ⌄]` 메뉴 = 프리셋들 · 저장… · 관리….
    pub(crate) fn set_presets(&mut self, mut names: Vec<String>) {
        names.sort_by_key(|n| n.to_lowercase());
        names.truncate(64);
        let mut items = vec![ComboItem::new("", "…")];
        for n in &names {
            items.push(ComboItem::new(format!("preset:{n}"), n.clone()));
        }
        items.push(ComboItem::new("save", tr("bulk.preset.saveSeq")));
        items.push(ComboItem::new("edit", tr("bulk.preset.editSeq")));
        self.preset_names = names;
        self.presets = Combo::new(items, 0);
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
        self.recompute(true);
    }

    /// 현재 파이프라인(유효 카드만).
    pub(crate) fn ops(&self) -> Vec<RenameOp> {
        self.cards.iter().filter_map(Card::op).collect()
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
        let names = self.preset_names.clone();
        self.set_presets(names);
        if let Some(w) = &self.window {
            w.set_title(&format!("Nexa Dir — {}", tr("bulk.title")));
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
            .with_title(format!("Nexa Dir — {}", tr("bulk.title")))
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
        let mut any = self.btn_cancel.tick(now_ms) | self.btn_rename.tick(now_ms);
        any |= self.presets.tick_hover(now_ms);
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
                || self.presets.hover_animating())
    }

    /// 배치(DLG-074): 좌 카드 열 320 + 썸 10 · 우 그리드 · 하단 행.
    fn layout(&mut self) {
        let Some(win) = &self.window else { return };
        let size = win.inner_size();
        let (w, h) = (size.width as i32, size.height as i32);
        let pad = self.s(PAD);
        let bh = self.s(BTN_H);
        let bottom_y = h - pad - bh;
        let mut inv = Invalidations::default();
        self.cards_rect = Rect::new(pad, pad, self.s(CARD_W), bottom_y - pad * 2);
        let gx = pad + self.s(CARD_W + 10.0) + pad;
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
        self.presets.set_scale(self.scale);
        self.presets
            .set_bounds(Rect::new(pad, bottom_y, self.s(64.0), bh), &mut inv);
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

    /// 열린 콤보(팝업이 모든 입력을 먼저 받는다).
    fn open_combo(&mut self) -> Option<&mut Combo> {
        if self.presets.is_open() {
            return Some(&mut self.presets);
        }
        for c in &mut self.cards {
            for cb in c.combos() {
                if cb.is_open() {
                    return Some(cb);
                }
            }
        }
        None
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
        for c in &mut self.cards {
            for tb in c.boxes() {
                tb.set_focused(tb.bounds().h > 0 && tb.bounds().contains(p));
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
        if let Some(v) = self.presets.take_changed() {
            self.presets.select_value("");
            if let Some(name) = v.strip_prefix("preset:") {
                return Some(BulkAction::LoadPreset(name.to_string()));
            }
            if v == "save" {
                let ops = self.ops();
                if !ops.is_empty() {
                    return Some(BulkAction::SavePreset(br::serialize_ops(&ops)));
                }
            }
            if v == "edit" {
                return Some(BulkAction::EditPresets);
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
                if matches!(kev.logical_key.as_ref(), Key::Named(NamedKey::Escape)) {
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
                if self.primary && crate::input::shortcut_letter(kev) == Some('a') {
                    if let Some(tb) = self.focused_box() {
                        tb.on_event(&InputEvent::SelectAll, &mut inv);
                    }
                    self.redraw();
                    return BulkAction::None;
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
                if self.cards_rect.contains(p) {
                    let dy = match delta {
                        MouseScrollDelta::LineDelta(_, y) => *y * 48.0 * self.scale,
                        MouseScrollDelta::PixelDelta(p) => p.y as f32,
                    };
                    self.scroll -= dy.round() as i32;
                    self.clamp_scroll();
                    self.layout_cards();
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
                if let Some(cb) = self.open_combo() {
                    cb.on_event(&mv, &mut inv);
                } else {
                    for c in &mut self.cards {
                        if c.rect.h == 0 {
                            continue;
                        }
                        for b in c.buttons() {
                            b.on_event(&mv, &mut inv);
                        }
                        for cb in c.combos() {
                            cb.on_event(&mv, &mut inv);
                        }
                        for tb in c.boxes() {
                            tb.on_event(&mv, &mut inv);
                        }
                        c.match_case.on_event(&mv, &mut inv);
                    }
                    self.presets.on_event(&mv, &mut inv);
                    self.btn_cancel.on_event(&mv, &mut inv);
                    self.btn_rename.on_event(&mv, &mut inv);
                    self.rows.on_event(&mv, &mut inv);
                }
                if !inv.is_empty() {
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
                // 누름 = 커서 아래 컨트롤만 · 뗌 = 전부.
                for c in &mut self.cards {
                    if c.rect.h == 0 {
                        continue;
                    }
                    for b in c.buttons() {
                        if up || b.bounds().contains(p) {
                            b.on_event(&e, &mut inv);
                        }
                        b.set_focused(false);
                    }
                    for cb in c.combos() {
                        if up || cb.bounds().contains(p) {
                            cb.on_event(&e, &mut inv);
                        }
                    }
                    for tb in c.boxes() {
                        if up || tb.bounds().contains(p) {
                            tb.on_event(&e, &mut inv);
                        }
                    }
                    if up || c.match_case.bounds().contains(p) {
                        c.match_case.on_event(&e, &mut inv);
                    }
                    let _ = c.match_case.take_toggled();
                }
                if up || self.presets.bounds().contains(p) {
                    self.presets.on_event(&e, &mut inv);
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
                dc.stroke_round_rect(clip, self.s(6.0), th.border, 1.0);
                let lx = r.x + inner_pad;
                let ty = |ry: i32| ry + (row_h - 4 - th_txt) / 2;
                let kind = c.kind_value();
                let mut ry = r.y + inner_pad + row_h;
                dc.text(lx, ty(ry), clip, &tr("bulk.lbl.applyTo"), th.text_dim);
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
            // 그리드.
            self.rows.paint(&mut dc, th);
            // 하단: 프리셋 콤보 · 건수/오류 · 버튼.
            if !self.presets.is_open() {
                self.presets.paint(&mut dc, th);
            }
            let bottom_y = hi - pad - self.s(BTN_H);
            let info = self.error.clone().unwrap_or_else(|| self.count_text());
            let ix = self.presets.bounds().right() + pad;
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
            // 열린 콤보 드롭다운 = 맨 뒤.
            for c in &self.cards {
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
            if self.presets.is_open() {
                self.presets.paint(&mut dc, th);
                self.presets.paint_dropdown(&mut dc, th);
            }
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
        w.load_ops(&[]);
        assert_eq!(w.cards.len(), 1, "빈 프리셋 = 기본 카드 1장");
        w.set_presets(vec!["zeta".into(), "Alpha".into()]);
        assert_eq!(
            w.preset_names,
            vec!["Alpha".to_string(), "zeta".to_string()]
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
