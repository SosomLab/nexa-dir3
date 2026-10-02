//! 파일 행 · 배경 컨텍스트 메뉴(dir2 docs/port/19 §2-1·§2-2 구성 — 셸 항목(IContextMenu · T-51 B) 제외한 **앱 고유 항목**):
//! 행 = 열기 · 잘라내기/복사/붙여넣기 · 삭제/완전 삭제/이름 바꾸기 · 경로 복사/이름 복사 · 폴더에 붙여넣기(단일 폴더 + 클립보드) · 새로 만들기.
//! 배경 = 붙여넣기 · 실행 취소/다시 실행(설명 포함) · 새 폴더/새 파일 · 새로 고침. 탭 메뉴와 같은 `ContextMenu` 인스턴스를 쓴다.

use crate::*;

/// 열린 메뉴의 주인(탭 메뉴는 `tab_menu_at`가 따로 든다).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CtxKind {
    /// 행(선택) 메뉴.
    Row(usize),
    /// 배경(빈 영역) 메뉴.
    Bg(usize),
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

    /// 행 메뉴(선택 항목 기준).
    pub(crate) fn open_row_menu(&mut self, panel: usize) {
        let sel = self.panels[panel].selected_paths();
        if sel.is_empty() {
            return self.open_bg_menu(panel);
        }
        let has_clip = self.clip_sources().is_some();
        let single_dir = matches!(&sel[..], [one] if one.is_dir());
        let items = vec![
            CtxItem::item("cmd.activate", tr("cmd.activate")).with_emphasis(true),
            CtxItem::Separator,
            CtxItem::item("edit.cut", tr("menu.edit.cut")),
            CtxItem::item("edit.copy", tr("menu.edit.copy")),
            CtxItem::maybe("edit.paste", tr("menu.edit.paste"), has_clip),
            CtxItem::Separator,
            CtxItem::item("edit.delete", tr("menu.edit.delete")),
            CtxItem::item("edit.delete_permanent", tr("ctx.deletePermanent")),
            CtxItem::maybe("edit.rename", tr("cmd.rename"), sel.len() == 1),
            CtxItem::Separator,
            CtxItem::item("ctx.copy_path", tr("ctx.copyPath")),
            CtxItem::item("ctx.copy_name", tr("ctx.copyName")),
            CtxItem::maybe(
                "ctx.paste_into",
                tr("ctx.pasteInto"),
                single_dir && has_clip,
            ),
            CtxItem::Separator,
            CtxItem::item("file.new_folder", tr("menu.file.newFolder")),
            CtxItem::item("file.new_file", tr("menu.file.newFile")),
        ];
        self.open_ctx(CtxKind::Row(panel), items);
    }

    /// 배경 메뉴(빈 영역).
    pub(crate) fn open_bg_menu(&mut self, panel: usize) {
        let has_clip = self.clip_sources().is_some();
        let undo = self.history.undo_description().map(str::to_string);
        let redo = self.history.redo_description().map(str::to_string);
        let items = vec![
            CtxItem::maybe("edit.paste", tr("ctx.paste"), has_clip),
            CtxItem::Separator,
            CtxItem::maybe(
                "edit.undo",
                undo.as_ref()
                    .map_or_else(|| tr("menu.edit.undo"), |d| trf("ctx.undoOf", &[d])),
                undo.is_some(),
            ),
            CtxItem::maybe(
                "edit.redo",
                redo.as_ref()
                    .map_or_else(|| tr("menu.edit.redo"), |d| trf("ctx.redoOf", &[d])),
                redo.is_some(),
            ),
            CtxItem::Separator,
            CtxItem::item("file.new_folder", tr("menu.file.newFolder")),
            CtxItem::item("file.new_file", tr("menu.file.newFile")),
            CtxItem::Separator,
            CtxItem::item("view.refresh", tr("menu.view.refresh")),
        ];
        self.open_ctx(CtxKind::Bg(panel), items);
    }

    /// 메뉴 선택 실행(명령 id는 `command` 한 길 · 고유 항목만 여기서).
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
