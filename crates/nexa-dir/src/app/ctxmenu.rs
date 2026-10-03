//! 파일 행 · 배경 컨텍스트 메뉴(dir2 docs/port/19 §2-1·§2-2 구성): **셸 항목**(ContextMenuProvider 포트 · Windows IContextMenu · T-51 B)이 상단에,
//! 그 아래 앱 고유 항목. 셸 verb cut/copy/paste/delete/rename/copyaspath는 **앱 경로로 가로채기**(SHELL-005/006 — undo 기록·인라인 이름 바꾸기·교차 폴더).
//! 행 = 열기 · 잘라내기/복사/붙여넣기 · 삭제/완전 삭제/이름 바꾸기 · 경로 복사/이름 복사 · 폴더에 붙여넣기(단일 폴더 + 클립보드) · 새로 만들기.
//! 배경 = **셸 배경 메뉴**(SHELL-009 · 보기·새로 만들기·속성 … · Windows) + 붙여넣기 · 실행 취소/다시 실행(설명 포함) · 새 폴더/새 파일 · 새로 고침.
//! 셸 배경 항목이 폴더에 항목을 1개 만들면(새로 만들기) 선택 + 인라인 이름 바꾸기(SHELL-008/009 `Created`). 탭 메뉴와 같은 `ContextMenu` 인스턴스를 쓴다.

use crate::platform::{MenuEvent, MenuTarget, ShellMenuItem};
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
    // 셸 확장 아이콘(SHELL-011) — 하나라도 있으면 nexa-ctl 메뉴가 전 행에 아이콘 칸을 예약한다.
    let icon = it
        .icon
        .as_ref()
        .filter(|i| i.rgba.len() == (i.w * i.h * 4) as usize && i.w > 0 && i.h > 0)
        .map(|i| nexa_ctl::controls::MenuIcon::from_rgba(i.w, i.h, &i.rgba));
    if it.children.is_empty() {
        CtxItem::maybe(id, it.label.clone(), it.enabled).with_icon(icon)
    } else {
        CtxItem::submenu(
            id,
            it.label.clone(),
            it.children.iter().map(shell_to_ctx).collect(),
        )
        .with_icon(icon)
    }
}

/// 선행 구축 머무름(dir2 `CTX_PREBUILD_MS`).
const CTX_PREBUILD_MS: u64 = 300;
/// 선행 구축 대상 선택 수 상한(그보다 많으면 우클릭 때 구축 — 매 틱 경로 목록을 만들지 않는다).
const CTX_PREBUILD_MAX: usize = 256;
/// 구축/실행을 기다리는 동안의 틱 간격.
const CTX_POLL_MS: u64 = 30;

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
        self.ctx_anchor = self.cursor;
        self.ctx_kind = Some(kind);
        self.tab_menu_at = None;
        self.reopen_ctx(items);
    }

    /// 메뉴를 (다시) 연다 — 연 자리(`ctx_anchor`) 그대로(셸 항목이 늦게 도착했을 때 같은 자리에서 채운다).
    fn reopen_ctx(&mut self, items: Vec<CtxItem>) {
        let host = Rect::new(0, 0, self.viewport.0, self.viewport.1);
        let text_w = px(240.0, self.scale);
        let (x, y) = self.ctx_anchor;
        self.ctx_items = items.clone();
        self.tab_menu.open_at(x, y, items, host, text_w);
        self.redraw();
    }

    fn ctx_set_owner(&self) {
        if let Some(w) = &self.window {
            if let Some(h) = winfocus::hwnd(w) {
                self.platform.ctxmenu.set_owner(h);
            }
        }
    }

    /// 행 메뉴(선택 항목 기준): 셸 항목 상단 합류 → 앱 고유 항목(셸이 이미 준 동사는 중복 금지).
    /// 셸 항목이 아직 구축 중이면(dir2 X-61 비차단) 자체 항목 + "불러오는 중" 줄로 **즉시** 열고, 도착하면 같은 자리에서 채운다.
    pub(crate) fn open_row_menu(&mut self, panel: usize) {
        let sel = self.panels[panel].selected_paths();
        if sel.is_empty() {
            return self.open_bg_menu(panel);
        }
        self.ctx_set_owner();
        let target = MenuTarget::Rows(sel.clone());
        let shell = self.platform.ctxmenu.try_items(&target);
        let items = self.row_menu_items(&sel, shell.as_deref());
        self.ctx_pending = shell.is_none().then_some(target);
        self.open_ctx(CtxKind::Row(panel), items);
    }

    /// 행 메뉴 항목 조립 — `shell` = 셸 항목(`None` = 구축 중).
    fn row_menu_items(&mut self, sel: &[PathBuf], shell: Option<&[ShellMenuItem]>) -> Vec<CtxItem> {
        let loading = shell.is_none();
        let has_clip = self.clip_sources().is_some();
        let single_dir = matches!(sel, [one] if one.is_dir());
        let shell: Vec<CtxItem> = shell.unwrap_or_default().iter().map(shell_to_ctx).collect();
        let have = |id: &str| has_id(&shell, id);
        let mut items: Vec<CtxItem> = Vec::new();
        if !shell.is_empty() {
            items.extend(shell.iter().cloned());
            items.push(CtxItem::Separator);
        } else {
            items.push(CtxItem::item("cmd.activate", tr("cmd.activate")).with_emphasis(true));
            if loading {
                items.push(CtxItem::maybe("ctx.loading", tr("ctx.loading"), false));
            }
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
        // 새로 만들기 ▸(SHELL-008 · dir2 CLSID_NewMenu 호스팅 → 자체 서브메뉴): 단일 선택일 때만 · 대상 = 폴더 항목 자신 / 파일 항목 부모 ·
        // 자식 = 폴더 · 텍스트 문서(템플릿에 txt가 없을 때) · OS/사용자 템플릿.
        self.ctx_new_dir = None;
        self.ctx_templates.clear();
        if vis("new") && sel.len() == 1 {
            let target = if sel[0].is_dir() {
                Some(sel[0].clone())
            } else {
                sel[0].parent().map(std::path::Path::to_path_buf)
            };
            if let Some(dir) = target.filter(|d| d.is_dir()) {
                let tpls = self.platform.templates.list();
                let mut kids = vec![CtxItem::item("new.folder", tr("new.menuFolder"))];
                if !tpls.iter().any(|t| t.ext == "txt") {
                    kids.push(CtxItem::item("new.file", tr("new.menuTextFile")));
                }
                if !tpls.is_empty() {
                    kids.push(CtxItem::Separator);
                }
                for (i, t) in tpls.iter().enumerate() {
                    kids.push(CtxItem::item(format!("new.tpl:{i}"), t.label.clone()));
                }
                items.push(CtxItem::Separator);
                items.push(CtxItem::submenu("ctx.new", tr("ctx.new"), kids));
                self.ctx_new_dir = Some(dir);
                self.ctx_templates = tpls;
            }
        }
        items
    }

    /// 배경 메뉴(빈 영역).
    pub(crate) fn open_bg_menu(&mut self, panel: usize) {
        // 셸 배경 메뉴(실경로 폴더만 · 가상 최상위는 자체 항목만).
        let dir = self.panels[panel].root_path();
        let (shell, pending) = if ndir_vfs::is_virtual_root(&dir) {
            (Some(Vec::new()), None)
        } else {
            self.ctx_set_owner();
            let target = MenuTarget::Bg(dir);
            let shell = self.platform.ctxmenu.try_items(&target);
            let pending = shell.is_none().then_some(target);
            (shell, pending)
        };
        let items = self.bg_menu_items(shell.as_deref());
        self.ctx_pending = pending;
        self.open_ctx(CtxKind::Bg(panel), items);
    }

    /// 배경 메뉴 항목 조립 — `shell` = 셸 배경 항목(`None` = 구축 중).
    fn bg_menu_items(&mut self, shell: Option<&[ShellMenuItem]>) -> Vec<CtxItem> {
        let loading = shell.is_none();
        let has_clip = self.clip_sources().is_some();
        let undo = self.history.undo_description().map(str::to_string);
        let redo = self.history.redo_description().map(str::to_string);
        let shell: Vec<CtxItem> = shell.unwrap_or_default().iter().map(shell_to_ctx).collect();
        let have = |id: &str| has_id(&shell, id);
        let mut items: Vec<CtxItem> = Vec::new();
        if !shell.is_empty() {
            items.extend(shell.iter().cloned());
            items.push(CtxItem::Separator);
        } else if loading {
            items.push(CtxItem::maybe("ctx.loading", tr("ctx.loading"), false));
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
        items
    }

    /// 셸 메뉴 틱(dir2 X-61 · SHELL-014/015): ① 비동기 통지 수거 — 기다리던 메뉴 채우기 · 실행 결과 반영 ② **선행 구축** —
    /// 선택(없으면 폴더 배경)이 300 ms 머물면 미리 구축해 우클릭이 즉시 뜨게 한다. 돌려주는 값 = 다음에 깨어날 시각.
    pub(crate) fn ctx_shell_tick(&mut self, now: Instant) -> Option<Instant> {
        while let Some(ev) = self.platform.ctxmenu.poll() {
            match ev {
                MenuEvent::Items { target, items } => {
                    if self.ctx_pending.as_ref() != Some(&target) {
                        continue;
                    }
                    self.ctx_pending = None;
                    if !self.tab_menu.is_open() {
                        continue;
                    }
                    let filled = match (self.ctx_kind, &target) {
                        (Some(CtxKind::Row(_)), MenuTarget::Rows(sel)) => {
                            self.row_menu_items(sel, Some(&items))
                        }
                        (Some(CtxKind::Bg(_)), MenuTarget::Bg(_)) => {
                            self.bg_menu_items(Some(&items))
                        }
                        _ => continue,
                    };
                    self.reopen_ctx(filled);
                }
                MenuEvent::Invoked { result, .. } => {
                    let panel = self
                        .ctx_invoke_panel
                        .take()
                        .unwrap_or(self.active)
                        .min(self.panels.len().saturating_sub(1));
                    self.ctx_invoked(panel, result);
                    self.redraw();
                }
            }
        }
        if self.ctx_pending.is_some() && !self.tab_menu.is_open() {
            self.ctx_pending = None; // 채우기 전에 닫힘 — 결과는 캐시에 남아 다음 우클릭이 즉시 뜬다.
        }
        let mut wake: Option<Instant> = None;
        if !self.tab_menu.is_open() {
            let a = self.active.min(self.panels.len().saturating_sub(1));
            let count = self.panels[a].rows().source().selection_count();
            let target = if count == 0 {
                let dir = self.panels[a].root_path();
                (!ndir_vfs::is_virtual_root(&dir)).then_some(MenuTarget::Bg(dir))
            } else if count <= CTX_PREBUILD_MAX {
                Some(MenuTarget::Rows(self.panels[a].selected_paths()))
            } else {
                None
            };
            if target != self.ctx_dwell_target {
                self.ctx_dwell_target = target;
                self.ctx_dwell_since = now;
                self.ctx_dwell_done = false;
            }
            if !self.ctx_dwell_done {
                if let Some(t) = &self.ctx_dwell_target {
                    let due = self.ctx_dwell_since + Duration::from_millis(CTX_PREBUILD_MS);
                    if now >= due {
                        self.ctx_set_owner();
                        self.platform.ctxmenu.prepare(t);
                        self.ctx_dwell_done = true;
                    } else {
                        wake = Some(due);
                    }
                }
            }
        }
        // 기동 명령 `ctx.wait`로 보류된 하네스 명령 재개(종료 요청이 나오면 곧바로 깨어나 처리).
        self.resume_blocked(now);
        if self.exit_requested {
            return Some(now);
        }
        if self.ctx_pending.is_some()
            || self.platform.ctxmenu.busy()
            || !self.startup_blocked.is_empty()
        {
            let t = now + Duration::from_millis(CTX_POLL_MS);
            wake = Some(wake.map_or(t, |w| w.min(t)));
        }
        wake
    }

    /// 셸 항목 실행 결과 반영 — FS가 바뀌었을 수 있어 재열람(SHELL-012) · 정확히 1개 생성(새로 만들기)이면 선택 + 인라인 이름 바꾸기.
    fn ctx_invoked(&mut self, panel: usize, result: Result<Option<PathBuf>, String>) {
        match result {
            Ok(created) => {
                let mut inv = Invalidations::default();
                for p in &mut self.panels {
                    p.reopen(&mut inv);
                }
                if let Some(path) = created {
                    if panel != self.active {
                        self.set_active(panel);
                    }
                    self.panels[panel].select_path(&path, &mut inv);
                    self.begin_rename();
                }
                self.update_status();
            }
            Err(e) => {
                self.toasts
                    .push(toast::ToastKind::Warn, tr("cmd.contextMenu"), e);
            }
        }
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
            "new.folder" | "new.file" => {
                if let Some(dir) = self.ctx_new_dir.take() {
                    let kind = if id == "new.folder" {
                        app::ops::NewKind::Folder
                    } else {
                        app::ops::NewKind::File
                    };
                    self.create_new_at(dir, kind);
                }
            }
            t if t.starts_with("new.tpl:") => {
                let idx: Option<usize> = t["new.tpl:".len()..].parse().ok();
                if let (Some(dir), Some(tpl)) = (
                    self.ctx_new_dir.take(),
                    idx.and_then(|i| self.ctx_templates.get(i).cloned()),
                ) {
                    self.create_new_at(dir, app::ops::NewKind::Template(tpl));
                }
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
            "ctx.loading" => {}
            other if ndir_settings::command(other).is_none() => {
                // 셸 항목(`shell:<id>` · 가짜 `fake.*`) = 플랫폼 포트 실행. 배경 메뉴면 폴더 기준.
                // 비동기 실행을 지원하면(Windows 메뉴 스레드 — 속성 창 같은 모달이 UI를 붙잡지 않는다) 결과는 틱에서, 아니면 동기.
                let target = if matches!(kind, CtxKind::Bg(_)) {
                    MenuTarget::Bg(self.panels[panel].root_path())
                } else {
                    MenuTarget::Rows(self.panels[panel].selected_paths())
                };
                if self.platform.ctxmenu.invoke_async(other, &target) {
                    self.ctx_invoke_panel = Some(panel);
                } else {
                    let result = match &target {
                        MenuTarget::Bg(dir) => self.platform.ctxmenu.invoke_bg(other, dir),
                        MenuTarget::Rows(sel) => {
                            self.platform.ctxmenu.invoke(other, sel).map(|()| None)
                        }
                    };
                    self.ctx_invoked(panel, result.map_err(|e| e.to_string()));
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
        // 서브메뉴 자식은 `부모[자식 자식]`로(새로 만들기 ▸ · 셸 서브메뉴) — 열 때 보관한 사본(`ctx_items`) · 탭 메뉴는 id 목록만.
        if self.ctx_kind.is_none() {
            return format!("{kind} {}\n", self.tab_menu.item_ids().join(" "));
        }
        let ids: Vec<String> = self
            .ctx_items
            .iter()
            .filter_map(|it| match it {
                CtxItem::Item { id, children, .. } if !children.is_empty() => Some(format!(
                    "{id}[{}]",
                    children
                        .iter()
                        .filter_map(|c| match c {
                            CtxItem::Item { id, .. } => Some(id.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                )),
                CtxItem::Item { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect();
        format!("{kind} {}\n", ids.join(" "))
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
