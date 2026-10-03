//! App — 순서 편집기 호스트(T-71 DLG-069~073 · dir2 `prefs.rs:1011-1083` 스펙 + `win.rs:8997-9037` 실시간 적용): 설정 창 필드 3개의
//! [편집…] → [`OrderWin`] · 변경 통지 = 즉시 적용·저장(툴바 재구성 · 컬럼 레이아웃(활성 패널 + 동기) · 컨텍스트 메뉴는 다음 열 때 읽음).

use crate::order::{self, OrderDefs};
use crate::order_win::OrderSpec;
use crate::*;

/// 설정 키 → 순서 정의(설정 창·기동 명령이 같은 표를 본다).
pub(crate) fn order_defs_of(key: &str) -> Option<OrderDefs> {
    Some(match key {
        "toolbar.layout" => order::toolbar_blocks(),
        "list.col_layout" => order::COLUMN_BLOCKS,
        "ctxmenu.layout" => order::CTXMENU_BLOCKS,
        _ => return None,
    })
}

/// 도구 모음 라벨(dir2 `tbo_label` — 기존 메뉴 키 최대 재사용).
/// 툴바 그룹 제목(도크 그룹 · 순서 편집기와 같은 라벨).
pub(crate) fn toolbar_group_title(block: &str) -> String {
    tbo_label(block, None)
}

fn tbo_label(block: &str, item: Option<&str>) -> String {
    match (block, item) {
        ("panel", None) => tr("pref.tbo.grpPanel"),
        ("panel", Some("toggle")) => tr("pref.tbo.panelToggle"),
        ("panel", Some("dock")) => tr("menu.view.dock"),
        ("panel", Some("info")) => tr("pref.tbo.infoToggle"),
        ("panel", Some("colsync")) => tr("menu.view.colWidthSync"),
        ("panel", Some("ontop")) => tr("menu.view.alwaysOnTop"),
        ("view", None) => tr("pref.tbo.grpView"),
        ("view", Some("tree")) => tr("menu.view.modeTree"),
        ("view", Some("flat")) => tr("menu.view.modeFlat"),
        ("view", Some("tiles")) => tr("menu.view.modeTiles"),
        ("refresh", None) => tr("menu.view.refresh"),
        ("settings", None) => tr("menu.file.prefs")
            .trim_end_matches(['.', '…'])
            .to_string(),
        ("show", None) => tr("pref.tbo.grpShow"),
        ("show", Some("hidden")) => tr("menu.view.hidden"),
        ("show", Some("dot")) => tr("menu.view.dot"),
        ("show", Some("foldersfirst")) => tr("pref.sortFoldersFirst"),
        (b, i) => i.unwrap_or(b).to_string(),
    }
}

fn col_label(_block: &str, item: Option<&str>) -> String {
    match item {
        Some("name") => tr("col.name"),
        Some("ext") => tr("col.ext"),
        Some("size") => tr("col.size"),
        Some("modified") => tr("col.modified"),
        Some("kind") => tr("col.kind"),
        _ => tr("pref.colLayout"),
    }
}

fn ctxm_label(block: &str, item: Option<&str>) -> String {
    match (block, item) {
        ("row", None) => tr("pref.ctxm.grpRow"),
        ("bg", None) => tr("pref.ctxm.grpBg"),
        (_, Some("new")) => tr("menu.file.newFolder"),
        (_, Some("deletePermanent")) => tr("ctx.deletePermanent"),
        (_, Some("copyName")) => tr("ctx.copyName"),
        (_, Some("pasteInto")) => tr("ctx.pasteInto"),
        (_, Some("paste")) => tr("ctx.paste"),
        (_, Some("undo")) => tr("menu.edit.undo"),
        (_, Some("redo")) => tr("menu.edit.redo"),
        (b, i) => i.unwrap_or(b).to_string(),
    }
}

impl App {
    /// 키 → 편집기 스펙(DLG-069 어댑터 3종).
    pub(crate) fn order_spec_for(key: &str) -> Option<OrderSpec> {
        let defs = order_defs_of(key)?;
        Some(match key {
            "toolbar.layout" => OrderSpec {
                title: tr("pref.toolbarOrder"),
                key: key.into(),
                defs,
                with_vis: true,
                flat: false,
                locked: &[],
                label: tbo_label,
            },
            "list.col_layout" => OrderSpec {
                title: tr("pref.colLayout"),
                key: key.into(),
                defs,
                with_vis: true,
                flat: true,
                locked: &["name"],
                label: col_label,
            },
            _ => OrderSpec {
                title: tr("pref.ctxMenuOrder"),
                key: key.into(),
                defs,
                with_vis: true,
                flat: false,
                locked: &[],
                label: ctxm_label,
            },
        })
    }

    /// 현재 값 — 컬럼은 활성 패널의 실제 레이아웃(설정값보다 정확) · 나머지는 설정.
    pub(crate) fn order_value_of(&self, key: &str) -> String {
        match key {
            "list.col_layout" => self.panels[self.active].col_layout_str(),
            k => self.settings.get(k).unwrap_or("").to_string(),
        }
    }

    /// 편집 창 열기 요청(설정 창 [편집…] · 기동 명령 `order.open:<key>`). 모르는 키 = 무시.
    pub(crate) fn open_order_editor(&mut self, key: &str) {
        let Some(spec) = Self::order_spec_for(key) else {
            return;
        };
        let value = self.order_value_of(key);
        self.order_win.set(spec, &value);
        self.open_order = true;
    }

    /// 편집 창 변경 통지(DLG-073) — 정규화 → 저장 → 즉시 적용.
    pub(crate) fn order_changed(&mut self, key: &str, value: &str) {
        let Some(defs) = order_defs_of(key) else {
            return;
        };
        let norm = order::normalize(defs, value);
        let _ = self.settings.set(key, &norm);
        let _ = self.settings.save();
        self.after_setting_changed(key);
    }

    /// 컬럼 레이아웃 적용(DLG-069 "포커스 패널에 적용 — 컬럼 동기화 규약") — 활성 패널 + `list.col_width_sync`면 반대 패널도.
    pub(crate) fn apply_col_layout_str(&mut self, layout: &str) {
        let parsed = order::parse_order_with(order::COLUMN_BLOCKS, layout);
        let Some((_, _, items)) = parsed.first() else {
            return;
        };
        let spec: Vec<(u32, bool)> = items
            .iter()
            .filter_map(|(k, v)| order::col_key_id(k).map(|id| (id, *v)))
            .collect();
        let mut inv = Invalidations::default();
        let a = self.active;
        self.panels[a].apply_col_layout(&spec, &mut inv);
        if self.settings.flag("list.col_width_sync") && self.dual {
            self.panels[1 - a].apply_col_layout(&spec, &mut inv);
        }
    }
}
