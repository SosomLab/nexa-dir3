//! 파일 행 · 배경 컨텍스트 메뉴(dir2 docs/port/19 §2-1·§2-2 구성): **셸 항목**(ContextMenuProvider 포트 · Windows IContextMenu · T-51 B)이 상단에,
//! 그 아래 앱 고유 항목. 셸 verb cut/copy/paste/delete/rename/copyaspath는 **앱 경로로 가로채기**(SHELL-005/006 — undo 기록·인라인 이름 바꾸기·교차 폴더).
//! 행 = 열기 · 잘라내기/복사/붙여넣기 · 삭제/완전 삭제/이름 바꾸기 · 경로 복사/이름 복사 · 폴더에 붙여넣기(단일 폴더 + 클립보드) · 새로 만들기.
//! 배경 = **셸 배경 메뉴**(SHELL-009 · 보기·새로 만들기·속성 … · Windows) + 붙여넣기 · 실행 취소/다시 실행(설명 포함) · 새 폴더/새 파일 · 새로 고침.
//! 셸 배경 항목이 폴더에 항목을 1개 만들면(새로 만들기) 선택 + 인라인 이름 바꾸기(SHELL-008/009 `Created`). 탭 메뉴와 같은 `ContextMenu` 인스턴스를 쓴다.

use crate::platform::ShellMenuItem;
use crate::*;

/// 열린 메뉴의 주인(탭 메뉴는 `tab_menu_at`가 따로 든다).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CtxKind {
    /// 행(선택) 메뉴.
    Row(usize),
    /// 배경(빈 영역) 메뉴.
    Bg(usize),
}

/// 셸 verb → 앱 명령(dir2 SHELL-005/006 가로채기).
fn intercept(verb: &str) -> Option<&'static str> {
    Some(match verb.to_ascii_lowercase().as_str() {
        "cut" => "edit.cut",
        "copy" => "edit.copy",
        "paste" => "edit.paste",
        "delete" => "edit.delete",
        "rename" => "edit.rename",
        "copyaspath" => "ctx.copy_path",
        _ => return None,
    })
}

/// 셸 항목 → 메뉴 항목(가로채기 id 치환 · 서브메뉴 재귀).
fn shell_to_ctx(it: &ShellMenuItem) -> CtxItem {
    if it.separator {
        return CtxItem::Separator;
    }
    let id = intercept(&it.verb).map_or_else(|| it.id.clone(), str::to_string);
    if it.children.is_empty() {
        CtxItem::maybe(id, it.label.clone(), it.enabled)
    } else {
        CtxItem::submenu(
            id,
            it.label.clone(),
            it.children.iter().map(shell_to_ctx).collect(),
        )
    }
}

fn has_id(items: &[CtxItem], id: &str) -> bool {
    items.iter().any(|c| match c {
        CtxItem::Item {
            id: cid, children, ..
        } => cid == id || has_id(children, id),
        _ => false,
    })
}

impl App {
    fn open_ctx(&mut self, kind: CtxKind, items: Vec<CtxItem>) {
        let host = Rect::new(0, 0, self.viewport.0, self.viewport.1);
        let text_w = px(240.0, self.scale);
        let (x, y) = self.cursor;
        self.ctx_kind = Some(kind);
        self.tab_menu_at = None;
        self.tab_menu.open_at(x, y, items, host, text_w);
        self.redraw();
    }

    /// 행 메뉴(선택 항목 기준): 셸 항목 상단 합류 → 앱 고유 항목(셸이 이미 준 동사는 중복 금지).
    pub(crate) fn open_row_menu(&mut self, panel: usize) {
        let sel = self.panels[panel].selected_paths();
        if sel.is_empty() {
            return self.open_bg_menu(panel);
        }
        let has_clip = self.clip_sources().is_some();
        let single_dir = matches!(&sel[..], [one] if one.is_dir());
        if let Some(w) = &self.window {
            if let Some(h) = winfocus::hwnd(w) {
                self.platform.ctxmenu.set_owner(h);
            }
        }
        let shell: Vec<CtxItem> = self
            .platform
            .ctxmenu
            .items(&sel)
            .map(|v| v.iter().map(shell_to_ctx).collect())
            .unwrap_or_default();
        let have = |id: &str| has_id(&shell, id);
        let mut items: Vec<CtxItem> = Vec::new();
        if !shell.is_empty() {
            items.extend(shell.iter().cloned());
            items.push(CtxItem::Separator);
        } else {
            items.push(CtxItem::item("cmd.activate", tr("cmd.activate")).with_emphasis(true));
            items.push(CtxItem::Separator);
        }
        if !have("edit.cut") {
            items.push(CtxItem::item("edit.cut", tr("menu.edit.cut")));
        }
        if !have("edit.copy") {
            items.push(CtxItem::item("edit.copy", tr("menu.edit.copy")));
        }
        if !have("edit.paste") {
            items.push(CtxItem::maybe(
                "edit.paste",
                tr("menu.edit.paste"),
                has_clip,
            ));
        }
        items.push(CtxItem::Separator);
        if !have("edit.delete") {
            items.push(CtxItem::item("edit.delete", tr("menu.edit.delete")));
        }
        // 앱 고유 항목(dir2 CTXMENU_BLOCKS `row`) — 순서/표시 = 설정 `ctxmenu.layout`(T-71 DLG-069 · 그룹 숨김 = 전부 제외 · `new`는 하단 고정 섹션).
        let own = self.ctx_layout("row");
        let vis = |k: &str| own.iter().any(|(x, v)| x == k && *v);
        if vis("deletePermanent") {
            items.push(CtxItem::item(
                "edit.delete_permanent",
                tr("ctx.deletePermanent"),
            ));
        }
        if !have("edit.rename") {
            items.push(CtxItem::maybe(
                "edit.rename",
                tr("cmd.rename"),
                sel.len() == 1,
            ));
        }
        items.push(CtxItem::Separator);
        if !have("ctx.copy_path") {
            items.push(CtxItem::item("ctx.copy_path", tr("ctx.copyPath")));
        }
        for (k, v) in &own {
            if !v {
                continue;
            }
            match k.as_str() {
                "copyName" => items.push(CtxItem::item("ctx.copy_name", tr("ctx.copyName"))),
                "pasteInto" => items.push(CtxItem::maybe(
                    "ctx.paste_into",
                    tr("ctx.pasteInto"),
                    single_dir && has_clip,
                )),
                _ => {}
            }
        }
        if vis("new") {
            items.push(CtxItem::Separator);
            items.push(CtxItem::item("file.new_folder", tr("menu.file.newFolder")));
            items.push(CtxItem::item("file.new_file", tr("menu.file.newFile")));
        }
        self.open_ctx(CtxKind::Row(panel), items);
    }

    /// 배경 메뉴(빈 영역).
    pub(crate) fn open_bg_menu(&mut self, panel: usize) {
        let has_clip = self.clip_sources().is_some();
        let undo = self.history.undo_description().map(str::to_string);
        let redo = self.history.redo_description().map(str::to_string);
        // 셸 배경 메뉴(실경로 폴더만 · 가상 최상위는 자체 항목만).
        let dir = self.panels[panel].root_path();
        let shell: Vec<CtxItem> = if ndir_vfs::is_virtual_root(&dir) {
            Vec::new()
        } else {
            if let Some(w) = &self.window {
                if let Some(h) = winfocus::hwnd(w) {
                    self.platform.ctxmenu.set_owner(h);
                }
            }
            self.platform
                .ctxmenu
                .bg_items(&dir)
                .map(|v| v.iter().map(shell_to_ctx).collect())
                .unwrap_or_default()
        };
        let have = |id: &str| has_id(&shell, id);
        let mut items: Vec<CtxItem> = Vec::new();
        if !shell.is_empty() {
            items.extend(shell.iter().cloned());
            items.push(CtxItem::Separator);
        }
        // 앱 고유 항목(dir2 CTXMENU_BLOCKS `bg`: paste · undo · redo) — 순서/표시 = 설정 `ctxmenu.layout`(T-71).
        let own = self.ctx_layout("bg");
        let mut edit_section = false;
        for (k, v) in &own {
            if !v {
                continue;
            }
            match k.as_str() {
                "paste" if !have("edit.paste") => {
                    items.push(CtxItem::maybe("edit.paste", tr("ctx.paste"), has_clip));
                }
                "undo" => {
                    if !edit_section {
                        items.push(CtxItem::Separator);
                        edit_section = true;
                    }
                    items.push(CtxItem::maybe(
                        "edit.undo",
                        undo.as_ref()
                            .map_or_else(|| tr("menu.edit.undo"), |d| trf("ctx.undoOf", &[d])),
                        undo.is_some(),
                    ));
                }
                "redo" => {
                    if !edit_section {
                        items.push(CtxItem::Separator);
                        edit_section = true;
                    }
                    items.push(CtxItem::maybe(
                        "edit.redo",
                        redo.as_ref()
                            .map_or_else(|| tr("menu.edit.redo"), |d| trf("ctx.redoOf", &[d])),
                        redo.is_some(),
                    ));
                }
                _ => {}
            }
        }
        items.extend(vec![
            CtxItem::Separator,
            CtxItem::item("file.new_folder", tr("menu.file.newFolder")),
            CtxItem::item("file.new_file", tr("menu.file.newFile")),
            CtxItem::Separator,
            CtxItem::item("view.refresh", tr("menu.view.refresh")),
        ]);
        self.open_ctx(CtxKind::Bg(panel), items);
    }

    /// 설정 `ctxmenu.layout`의 블록 자식(key, 표시) — 블록 숨김이면 빈 목록(dir2 07-19 "그룹 숨김 = 고유 항목 전부 제외").
    fn ctx_layout(&self, block: &str) -> Vec<(String, bool)> {
        let layout = self.settings.get("ctxmenu.layout").unwrap_or("");
        crate::order::parse_order_with(crate::order::CTXMENU_BLOCKS, layout)
            .into_iter()
            .find(|(b, _, _)| b == block)
            .filter(|(_, bv, _)| *bv)
            .map(|(_, _, items)| items)
            .unwrap_or_default()
    }

    /// 메뉴 선택 실행(명령 id는 `command` 한 길 · 고유 항목 · 셸 항목 = 포트 실행 뒤 재열람).
    pub(crate) fn ctx_menu_action(&mut self, id: &str) {
        let Some(kind) = self.ctx_kind.take() else {
            return;
        };
        let panel = match kind {
            CtxKind::Row(p) | CtxKind::Bg(p) => p,
        };
        if panel != self.active {
            self.set_active(panel);
        }
        match id {
            "ctx.copy_path" => {
                let text = self.panels[panel]
                    .selected_paths()
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\r\n");
                let _ = clipboard::write_text(&text);
            }
            "ctx.copy_name" => {
                let text = self.panels[panel]
                    .selected_paths()
                    .iter()
                    .map(|p| ndir_ops::leaf_name(p))
                    .collect::<Vec<_>>()
                    .join("\r\n");
                let _ = clipboard::write_text(&text);
            }
            "ctx.paste_into" => {
                let sel = self.panels[panel].selected_paths();
                if let ([dest], Some((sources, cut))) = (&sel[..], self.clip_sources()) {
                    if dest.is_dir() {
                        let op = if cut {
                            ndir_ops::Op::Move
                        } else {
                            ndir_ops::Op::Copy
                        };
                        let dest = dest.clone();
                        self.start_transfer(sources, dest, op, cut);
                    }
                }
            }
            "cmd.activate" => {
                let mut inv = Invalidations::default();
                if let Some(row) = self.panels[panel].rows().caret() {
                    self.panels[panel].activate_row(row, &mut inv);
                }
            }
            other if ndir_settings::command(other).is_none() => {
                // 셸 항목(`shell:<id>` · 가짜 `fake.*`) = 플랫폼 포트 실행 → FS가 바뀌었을 수 있어 재열람(SHELL-012).
                // 배경 메뉴면 폴더 기준 실행 · 정확히 1개 생성(새로 만들기)이면 선택 + 인라인 이름 바꾸기(SHELL-009 Created).
                let result = if matches!(kind, CtxKind::Bg(_)) {
                    let dir = self.panels[panel].root_path();
                    self.platform.ctxmenu.invoke_bg(other, &dir)
                } else {
                    let sel = self.panels[panel].selected_paths();
                    self.platform.ctxmenu.invoke(other, &sel).map(|()| None)
                };
                match result {
                    Ok(created) => {
                        let mut inv = Invalidations::default();
                        for p in &mut self.panels {
                            p.reopen(&mut inv);
                        }
                        if let Some(path) = created {
                            self.panels[panel].select_path(&path, &mut inv);
                            self.begin_rename();
                        }
                        self.update_status();
                    }
                    Err(e) => {
                        self.toasts.push(
                            toast::ToastKind::Warn,
                            tr("cmd.contextMenu"),
                            e.to_string(),
                        );
                    }
                }
            }
            other => self.command(other),
        }
        self.redraw();
    }

    /// 덤프 `ctx`: `row|bg|tab|none` + 항목 id 목록.
    pub(crate) fn ctx_dump(&self) -> String {
        if !self.tab_menu.is_open() {
            return "none\n".into();
        }
        let kind = match (self.ctx_kind, self.tab_menu_at) {
            (Some(CtxKind::Row(_)), _) => "row",
            (Some(CtxKind::Bg(_)), _) => "bg",
            (None, Some(_)) => "tab",
            _ => "?",
        };
        format!("{kind} {}\n", self.tab_menu.item_ids().join(" "))
    }

    /// 기동 명령 `ctx.pick:<id>` — 열린 메뉴를 닫고 그 항목을 실행.
    pub(crate) fn ctx_pick(&mut self, id: &str) {
        if !self.tab_menu.is_open() {
            return;
        }
        self.tab_menu.close();
        if self.tab_menu_at.is_some() {
            self.tab_menu_action(id);
        } else {
            self.ctx_menu_action(id);
        }
        self.update_status();
        self.redraw();
    }
}
