//! 파일 행 · 배경 컨텍스트 메뉴(dir2 docs/port/19 §2-1·§2-2 구성): **셸 항목**(ContextMenuProvider 포트 · Windows IContextMenu · T-51 B)이 상단에,
//! 그 아래 앱 고유 항목. 셸 verb cut/copy/paste/delete/rename/copyaspath는 **앱 경로로 가로채기**(SHELL-005/006 — undo 기록·인라인 이름 바꾸기·교차 폴더).
//! 행 = 열기 · 잘라내기/복사/붙여넣기 · 삭제/완전 삭제/이름 바꾸기 · 경로 복사/이름 복사 · 폴더에 붙여넣기(단일 폴더 + 클립보드) · 새로 만들기.
//! 배경 = **셸 배경 메뉴**(SHELL-009 · 보기·새로 만들기·속성 … · Windows) + 붙여넣기 · 실행 취소/다시 실행(설명 포함) · 새 폴더/새 파일 · 새로 고침.
//! 셸 배경 항목이 폴더에 항목을 1개 만들면(새로 만들기) 선택 + 인라인 이름 바꾸기(SHELL-008/009 `Created`). 탭 메뉴와 같은 `ContextMenu` 인스턴스를 쓴다.

use crate::platform::{MenuEvent, MenuTarget, ShellMenuItem};
use crate::*;
use std::path::Path;

/// 열린 메뉴의 주인(탭 메뉴는 `tab_menu_at`가 따로 든다).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CtxKind {
    /// 행(선택) 메뉴.
    Row(usize),
    /// 배경(빈 영역) 메뉴.
    Bg(usize),
    /// 경로 바 편집 필드의 글자 편집 메뉴(GAP-012).
    PathEdit(usize),
    /// 이름 바꾸기 편집 필드의 글자 편집 메뉴(dir2 `EditMenuTarget::Rename` · CMD-086~091).
    RenameEdit(usize),
    /// 도크 정보 · 미리보기 글 메뉴(복사 · 모두 선택 — CMD-092/093) · 값 = 도크 자리.
    DockText(usize),
    /// 도크 터미널 메뉴(복사 · 붙여넣기 · 모두 선택 — CMD-094~096) · 값 = 도크 자리.
    TermEdit(usize),
    /// 보조 메뉴(탭 상태바 칸 · 툴바 · 런처) — 항목 id가 스스로 뜻을 가진다(`aux.*` → `App::aux_menu_action`).
    Aux(usize),
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
    // 경로 복사 = 셸 항목 자리 그대로 · 라벨만 앱 언어로(dir2 제자리 대체 · win.rs:3024 — OS 라벨 "경로로 복사(A)" 대신).
    let label = if id == "ctx.copy_path" {
        tr("ctx.copyPath")
    } else {
        it.label.clone()
    };
    // 셸 확장 아이콘(SHELL-011) — 하나라도 있으면 nexa-ctl 메뉴가 전 행에 아이콘 칸을 예약한다.
    let icon = it
        .icon
        .as_ref()
        .filter(|i| i.rgba.len() == (i.w * i.h * 4) as usize && i.w > 0 && i.h > 0)
        .map(|i| nexa_ctl::controls::MenuIcon::from_rgba(i.w, i.h, &i.rgba));
    if it.children.is_empty() {
        CtxItem::maybe(id, label, it.enabled).with_icon(icon)
    } else {
        CtxItem::submenu(id, label, it.children.iter().map(shell_to_ctx).collect()).with_icon(icon)
    }
}

/// 선행 구축 머무름(dir2 `CTX_PREBUILD_MS`).
const CTX_PREBUILD_MS: u64 = 300;
/// 선행 구축 대상 선택 수 상한(그보다 많으면 우클릭 때 구축 — 매 틱 경로 목록을 만들지 않는다).
const CTX_PREBUILD_MAX: usize = 256;
/// 구축/실행을 기다리는 동안의 틱 간격.
const CTX_POLL_MS: u64 = 30;
/// 셸 항목을 이만큼 기다려도 안 오면 자체 항목만으로 연다(느린 셸 확장 · 네트워크 경로).
const CTX_WAIT_MAX_MS: u64 = 3000;

/// 메뉴 행 높이(논리 px · nexa-ctl 메뉴의 행 = 글자 16 + 여백 10) · 위아래 안쪽 여백 — 창 높이에 들어가는 행 수 계산용.
const CTX_ROW_H: f32 = 26.0;
const CTX_PAD_V: f32 = 5.0;

/// 구분선 정리(사용자 10-03 "속성 밑에 내용이 없는 구분자 2개"): 셸이 이미 준 동사를 빼고 나면 구분선만 연달아 남는다 →
/// 맨 앞·맨 뒤·연속 구분선을 걷는다(하위 메뉴도).
fn tidy_separators(items: Vec<CtxItem>) -> Vec<CtxItem> {
    let mut out: Vec<CtxItem> = Vec::with_capacity(items.len());
    for it in items {
        match it {
            CtxItem::Separator => {
                if !matches!(out.last(), None | Some(CtxItem::Separator)) {
                    out.push(CtxItem::Separator);
                }
            }
            mut it => {
                if let CtxItem::Item { children, .. } = &mut it {
                    *children = tidy_separators(std::mem::take(children));
                }
                out.push(it);
            }
        }
    }
    if matches!(out.last(), Some(CtxItem::Separator)) {
        out.pop();
    }
    out
}

/// 창 높이 `viewport_h`(물리 px)에 온전히 들어가는 메뉴 행 수(구분선은 행보다 낮으므로 행 높이로 세면 항상 안전 쪽).
fn ctx_rows_that_fit(viewport_h: i32, scale: f32) -> usize {
    let row = px(CTX_ROW_H, scale).max(1);
    let pad = px(CTX_PAD_V, scale) * 2;
    usize::try_from((viewport_h - pad) / row)
        .unwrap_or(0)
        .max(3)
}

fn has_id(items: &[CtxItem], id: &str) -> bool {
    items.iter().any(|c| match c {
        CtxItem::Item {
            id: cid, children, ..
        } => cid == id || has_id(children, id),
        _ => false,
    })
}

/// 우클릭 메뉴 대상(순수 · dir2 `context_targets`): 선택을 **캐럿 항목의 부모 폴더** 것으로 줄인다 — 셸 메뉴(`GetUIObjectOf`)는
/// 한 부모의 항목만 표현할 수 있다. 캐럿이 없거나 부모가 없으면(최상위) · 줄인 결과가 비면 그대로 둔다.
pub(crate) fn context_targets(sel: Vec<PathBuf>, caret: Option<&Path>) -> Vec<PathBuf> {
    let Some(parent) = caret.and_then(Path::parent) else {
        return sel;
    };
    let kept: Vec<PathBuf> = sel
        .iter()
        .filter(|p| p.parent() == Some(parent))
        .cloned()
        .collect();
    if kept.is_empty() {
        sel
    } else {
        kept
    }
}

impl App {
    pub(crate) fn open_ctx(&mut self, kind: CtxKind, items: Vec<CtxItem>) {
        self.ctx_anchor = self.ctx_anchor_next.take().unwrap_or(self.cursor);
        self.ctx_kind = Some(kind);
        self.tab_menu_at = None;
        self.reopen_ctx(items);
    }

    /// 메뉴를 (다시) 연다 — 연 자리(`ctx_anchor`) 그대로(셸 항목이 늦게 도착했을 때 같은 자리에서 채운다).
    pub(crate) fn reopen_ctx(&mut self, items: Vec<CtxItem>) {
        let host = Rect::new(0, 0, self.viewport.0, self.viewport.1);
        let text_w = px(240.0, self.scale);
        let (x, y) = self.ctx_anchor;
        // 앱 고유 항목 앞 아이콘(복사 · 잘라내기 · 붙여넣기 … — `menu_icons` · 셸 확장이 준 아이콘은 그대로).
        let items = app::menu_icons::decorate(tidy_separators(items));
        // 창보다 긴 메뉴(셸 확장이 많은 PC)는 아래가 잘렸다 → 창에 들어가는 행 수까지만 보이고 나머지는 스크롤(휠 · 키 · 오른쪽 표시).
        self.tab_menu
            .set_max_rows(Some(ctx_rows_that_fit(self.viewport.1, self.scale)));
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
        // 트리에서 여러 폴더에 걸쳐 고른 선택 = 캐럿 항목의 부모 폴더 것만(dir2 win.rs:2779-2801 — 셸 메뉴는 한 부모만 표현한다).
        let caret = {
            let rows = self.panels[panel].rows();
            rows.caret().and_then(|c| rows.source().row_path(c))
        };
        let sel = context_targets(sel, caret.as_deref());
        self.ctx_set_owner();
        // Shift를 누른 채 열면 확장 동사까지(dir2 SHELL-004) — 선행 구축분(평소 메뉴)과 대상이 달라 새로 구축한다.
        self.ctx_extended = self.shift;
        let target = self.rows_target(sel.clone());
        match self.platform.ctxmenu.try_items(&target) {
            Some(shell) => {
                let items = self.row_menu_items(&sel, Some(&shell));
                self.open_ctx(CtxKind::Row(panel), items);
            }
            None => self.ctx_begin_wait(CtxKind::Row(panel), target),
        }
    }

    /// 셸 항목이 아직 없다 → 메뉴를 열지 않고 기다린다(사용자 10-03 "우클릭하면 메뉴가 두 번 뜬다" — 종전은 자체 항목으로 먼저
    /// 열고 도착하면 다시 채웠다). UI는 멈추지 않고 상태줄에 진행을 알린다(DR-20) · 준비되면 [`Self::ctx_shell_tick`]이 한 번 연다.
    fn ctx_begin_wait(&mut self, kind: CtxKind, target: MenuTarget) {
        self.ctx_anchor = self.ctx_anchor_next.take().unwrap_or(self.cursor);
        self.ctx_pending = None;
        self.ctx_wait = Some((kind, target, Instant::now()));
        let mut inv = Invalidations::default();
        self.statusbar.set_left(&tr("ctx.loading"), &mut inv);
        self.redraw();
    }

    /// 키보드로 여는 행 메뉴(Shift+F10 · Apps 키 · `cmd.contextMenu` — SHELL-003): 마우스 커서가 아니라 **캐럿 행 자리**에 연다
    /// (캐럿이 화면 밖이면 목록 왼쪽 위).
    pub(crate) fn open_row_menu_at_caret(&mut self, panel: usize) {
        let rows = self.panels[panel].rows();
        let b = rows.bounds();
        let at = rows.caret().and_then(|c| rows.row_anchor(c)).map_or(
            (b.x + px(24.0, self.scale), b.y + px(24.0, self.scale)),
            |p| (p.x + px(24.0, self.scale), p.y),
        );
        self.ctx_anchor_next = Some(at);
        self.open_row_menu(panel);
        self.ctx_anchor_next = None;
    }

    /// 경로 바 편집 필드 메뉴(dir2 CMD-086~091 · win.rs:7472-7532): 실행 취소 · 잘라내기 · 복사 · 붙여넣기 · 삭제 · 전체 선택.
    /// 활성 = 네이티브 EDIT 메뉴와 같은 규칙(되돌릴 것 · 선택 · 클립보드 글자 · 비어 있지 않음). 실행은 [`Self::path_edit`].
    pub(crate) fn open_path_edit_menu(&mut self, panel: usize) {
        let Some((can_undo, has_sel, empty)) = self.panels[panel].pathbar.edit_menu_state() else {
            return;
        };
        let has_text = clipboard::read_text().is_some_and(|t| !t.is_empty());
        let items = vec![
            CtxItem::maybe("edit.undo", tr("menu.edit.undo"), can_undo),
            CtxItem::Separator,
            CtxItem::maybe("edit.cut", tr("menu.edit.cut"), has_sel),
            CtxItem::maybe("edit.copy", tr("menu.edit.copy"), has_sel),
            CtxItem::maybe("edit.paste", tr("menu.edit.paste"), has_text),
            CtxItem::maybe("edit.delete", tr("menu.edit.delete"), has_sel),
            CtxItem::Separator,
            CtxItem::maybe("edit.select_all", tr("menu.edit.selectAll"), !empty),
        ];
        self.ctx_wait = None;
        self.open_ctx(CtxKind::PathEdit(panel), items);
    }

    /// 이름 바꾸기 편집 필드 메뉴(dir2 win.rs:7513-7531 — 경로 바와 같은 6항목 · 실행 = [`Self::rename_edit`]).
    pub(crate) fn open_rename_edit_menu(&mut self, panel: usize) {
        let Some((can_undo, has_sel, empty)) = self.panels[panel].rows().rename_menu_state() else {
            return;
        };
        let has_text = clipboard::read_text().is_some_and(|t| !t.is_empty());
        let items = vec![
            CtxItem::maybe("edit.undo", tr("menu.edit.undo"), can_undo),
            CtxItem::Separator,
            CtxItem::maybe("edit.cut", tr("menu.edit.cut"), has_sel),
            CtxItem::maybe("edit.copy", tr("menu.edit.copy"), has_sel),
            CtxItem::maybe("edit.paste", tr("menu.edit.paste"), has_text),
            CtxItem::maybe("edit.delete", tr("menu.edit.delete"), has_sel),
            CtxItem::Separator,
            CtxItem::maybe("edit.select_all", tr("menu.edit.selectAll"), !empty),
        ];
        self.ctx_wait = None;
        self.open_ctx(CtxKind::RenameEdit(panel), items);
    }

    /// 도크 우클릭(dir2 win.rs:7486-7491 · 7533-7548): 터미널 격자 위 = 터미널 메뉴(포커스도 옮긴다 — 붙여넣기 대상 확정 ·
    /// TUI 마우스 모드면 셸 몫이라 열지 않는다) · 고를 수 있는 글 위 = 글 메뉴. 열었으면 `true`.
    pub(crate) fn open_dock_edit_menu(&mut self, dock: usize, inv: &mut Invalidations) -> bool {
        let (x, y) = self.cursor;
        if self.term_hit_at(x, y) == Some(dock) && self.terms[dock].started() {
            // TUI 마우스 모드면 우클릭은 셸 몫 — 단, **Shift+우클릭은 늘 로컬 메뉴**(dir2 win.rs:8185 · 좌클릭의 Shift = 로컬 선택과 같은 규칙).
            if !self.shift && self.terms[dock].mouse_report(x, y, 2, true).is_some() {
                return false;
            }
            self.set_term_focus(Some(dock), inv);
            self.open_term_edit_menu(dock);
            return true;
        }
        if self.docks[dock].text_selectable()
            && self.docks[dock].content_rect().contains(Point { x, y })
        {
            self.open_dock_text_menu(dock);
            return true;
        }
        false
    }

    /// 터미널 메뉴: 복사(선택이 있을 때) · 붙여넣기(클립보드에 글이 있을 때) · 모두 선택.
    pub(crate) fn open_term_edit_menu(&mut self, dock: usize) {
        let has_sel = self.terms[dock].selected_text().is_some();
        let has_text = clipboard::read_text().is_some_and(|t| !t.is_empty());
        let items = vec![
            CtxItem::maybe("edit.copy", tr("menu.edit.copy"), has_sel),
            CtxItem::maybe("edit.paste", tr("menu.edit.paste"), has_text),
            CtxItem::Separator,
            CtxItem::item("edit.select_all", tr("menu.edit.selectAll")),
        ];
        self.ctx_wait = None;
        self.open_ctx(CtxKind::TermEdit(dock), items);
    }

    /// 도크 글(정보 · 미리보기) 메뉴: 복사(선택이 있을 때) · 모두 선택.
    pub(crate) fn open_dock_text_menu(&mut self, dock: usize) {
        let has_sel = self.docks[dock].selected_text().is_some();
        let selectable = self.docks[dock].text_selectable();
        let items = vec![
            CtxItem::maybe("edit.copy", tr("menu.edit.copy"), has_sel),
            CtxItem::Separator,
            CtxItem::maybe("edit.select_all", tr("menu.edit.selectAll"), selectable),
        ];
        self.ctx_wait = None;
        self.open_ctx(CtxKind::DockText(dock), items);
    }

    /// 도크 글 · 터미널 메뉴 실행.
    fn dock_edit_action(&mut self, kind: CtxKind, id: &str) {
        match kind {
            CtxKind::TermEdit(i) => {
                let mut inv = Invalidations::default();
                self.set_term_focus(Some(i), &mut inv);
                match id {
                    "edit.copy" => {
                        self.term_copy();
                    }
                    "edit.paste" => {
                        self.term_paste();
                    }
                    "edit.select_all" => {
                        self.term_select_all();
                    }
                    _ => {}
                }
            }
            CtxKind::DockText(i) => match id {
                "edit.copy" => {
                    if let Some(t) = self.docks[i].selected_text() {
                        let _ = clipboard::write_text(&t);
                    }
                }
                "edit.select_all" => {
                    let mut inv = Invalidations::default();
                    self.docks[i].select_all_text(&mut inv);
                }
                _ => {}
            },
            _ => {}
        }
        self.redraw();
    }

    /// 기다리던 메뉴 취소(다른 곳 클릭 · Esc · 키 입력).
    pub(crate) fn ctx_cancel_wait(&mut self) -> bool {
        if self.ctx_wait.take().is_some() {
            self.update_status();
            self.redraw();
            return true;
        }
        false
    }

    /// 배경 메뉴의 셸 대상 — 이번 메뉴를 Shift로 열었으면 확장 동사 대상.
    fn bg_target(&self, dir: PathBuf) -> MenuTarget {
        if self.ctx_extended {
            MenuTarget::BgExtended(dir)
        } else {
            MenuTarget::Bg(dir)
        }
    }

    /// 행 메뉴의 셸 대상 — 이번 메뉴를 Shift로 열었으면([`Self::open_row_menu`]) 확장 동사 대상.
    fn rows_target(&self, sel: Vec<PathBuf>) -> MenuTarget {
        if self.ctx_extended {
            MenuTarget::RowsExtended(sel)
        } else {
            MenuTarget::Rows(sel)
        }
    }

    /// 기다림이 끝났다 — 연 자리(`ctx_anchor`)에 완성된 메뉴를 한 번 연다.
    fn ctx_open_waited(&mut self, kind: CtxKind, target: &MenuTarget, shell: &[ShellMenuItem]) {
        let items = match (kind, target) {
            (CtxKind::Row(_), MenuTarget::Rows(sel) | MenuTarget::RowsExtended(sel)) => {
                self.row_menu_items(sel, Some(shell))
            }
            _ => self.bg_menu_items(Some(shell)),
        };
        self.update_status();
        self.ctx_kind = Some(kind);
        self.tab_menu_at = None;
        self.reopen_ctx(items);
    }

    /// 행 메뉴 항목 조립 — `shell` = 셸 항목(`None` = 구축 중).
    pub(crate) fn row_menu_items(
        &mut self,
        sel: &[PathBuf],
        shell: Option<&[ShellMenuItem]>,
    ) -> Vec<CtxItem> {
        let loading = shell.is_none();
        let has_clip = self.clip_sources().is_some();
        let single_dir = matches!(sel, [one] if one.is_dir());
        let mut shell: Vec<CtxItem> = shell.unwrap_or_default().iter().map(shell_to_ctx).collect();
        // 앱 고유 항목(dir2 CTXMENU_BLOCKS `row`) — 순서/표시 = 설정 `ctxmenu.layout`(T-71 DLG-069 · 그룹 숨김 = 전부 제외 · `new`는 하단 고정 섹션).
        let own = self.ctx_layout("row");
        let vis = |k: &str| own.iter().any(|(x, v)| x == k && *v);
        // 이름 복사 = 셸의 경로 복사 **바로 아래** 고정(dir2 `after_id` · win.rs:2996 — 설정 순서보다 우선) · 그 항목이 없으면 아래 고유 구역.
        let copy_path_at = shell
            .iter()
            .position(|c| matches!(c, CtxItem::Item { id, .. } if id == "ctx.copy_path"));
        let name_anchored = vis("copyName") && copy_path_at.is_some();
        if let Some(at) = copy_path_at.filter(|_| name_anchored) {
            shell.insert(at + 1, CtxItem::item("ctx.copy_name", tr("ctx.copyName")));
        }
        let have = |id: &str| has_id(&shell, id);
        let mut items: Vec<CtxItem> = Vec::new();
        if !shell.is_empty() {
            // Linux 앱 연결 항목(`xdg.*`)에는 폴더의 "열기"(= 앱 안 이동)가 없다 → 폴더면 앱의 열기를 맨 위에 둔다
            // (Windows 셸 메뉴는 자기 "열기"를 갖고 온다).
            if single_dir && (have("xdg.openwith") || have("xdg.props")) {
                items.push(CtxItem::item("cmd.activate", tr("cmd.activate")).with_emphasis(true));
            }
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
                "copyName" if !name_anchored => {
                    items.push(CtxItem::item("ctx.copy_name", tr("ctx.copyName")));
                }
                "pasteInto" => items.push(CtxItem::maybe(
                    "ctx.paste_into",
                    tr("ctx.pasteInto"),
                    single_dir && has_clip,
                )),
                _ => {}
            }
        }
        // 체크섬(T-167 · dir3 신규): 파일 1개일 때 · 경로/이름 복사 묶음 뒤(이름 복사는 경로 복사 바로 아래여야 한다 — dir2 규칙).
        if !sel.is_empty() && sel.iter().all(|p| p.is_file()) {
            items.push(CtxItem::item("ctx.checksum", tr("ctx.checksum")));
        }
        // 중복 파일 찾기(T-170 · dir3 신규): 선택에 폴더가 있으면 그 폴더들 안에서.
        if sel.iter().any(|p| p.is_dir()) {
            items.push(CtxItem::item("ctx.find_dupes", tr("ctx.findDupes")));
        }
        // 압축 풀기 ▸(T-169 · dir3 신규): zip · tar · gz · tgz 파일 1개일 때 — 여기에 / "<이름>" 폴더에.
        if let [one] = sel {
            if one.is_file() && app::extract::extract_ext_ok(one) {
                let folder = ndir_ops::leaf_name(&app::extract::extract_dest(one, false));
                items.push(CtxItem::submenu(
                    "ctx.extract",
                    tr("ctx.extract"),
                    vec![
                        CtxItem::item("ctx.extract_here", tr("ctx.extractHere")),
                        CtxItem::item("ctx.extract_to", trf("ctx.extractTo", &[&folder])),
                    ],
                ));
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
        let shell = if ndir_vfs::is_virtual_root(&dir) {
            Vec::new()
        } else {
            self.ctx_set_owner();
            // Shift를 누른 채 열면 확장 동사까지(dir2 win.rs:2875) — 행 메뉴와 같은 규칙.
            self.ctx_extended = self.shift;
            let target = self.bg_target(dir);
            match self.platform.ctxmenu.try_items(&target) {
                Some(shell) => shell,
                None => return self.ctx_begin_wait(CtxKind::Bg(panel), target),
            }
        };
        let mut items = self.bg_menu_items(Some(&shell));
        // 중복 파일 찾기(T-170 · dir3 신규) — 현재 폴더 안에서 · 폴더 비교(T-171) — 두 패널일 때 반대 패널과.
        items.push(CtxItem::Separator);
        items.push(CtxItem::item("ctx.find_dupes", tr("ctx.findDupes")));
        if self.dual {
            items.push(CtxItem::item("ctx.compare_panels", tr("ctx.comparePanels")));
        }
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
                    // 기다리던 메뉴의 셸 항목 도착 → 완성된 메뉴를 한 번 연다.
                    if self.ctx_wait.as_ref().is_some_and(|w| w.1 == target) {
                        if let Some((kind, t, _)) = self.ctx_wait.take() {
                            self.ctx_open_waited(kind, &t, &items);
                        }
                        continue;
                    }
                    if self.ctx_pending.as_ref() != Some(&target) {
                        continue;
                    }
                    self.ctx_pending = None;
                    if !self.tab_menu.is_open() {
                        continue;
                    }
                    let filled = match (self.ctx_kind, &target) {
                        (
                            Some(CtxKind::Row(_)),
                            MenuTarget::Rows(sel) | MenuTarget::RowsExtended(sel),
                        ) => self.row_menu_items(sel, Some(&items)),
                        (Some(CtxKind::Bg(_)), MenuTarget::Bg(_) | MenuTarget::BgExtended(_)) => {
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
        // 셸이 너무 오래 걸리면 자체 항목만으로 연다(뒤늦게 다시 채우지 않는다 — 결과는 캐시에 남아 다음 우클릭에 쓰인다).
        if self
            .ctx_wait
            .as_ref()
            .is_some_and(|w| now.duration_since(w.2) >= Duration::from_millis(CTX_WAIT_MAX_MS))
        {
            if let Some((kind, t, _)) = self.ctx_wait.take() {
                self.ctx_open_waited(kind, &t, &[]);
            }
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
            || self.ctx_wait.is_some()
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
            CtxKind::Row(p) | CtxKind::Bg(p) | CtxKind::PathEdit(p) | CtxKind::RenameEdit(p) => p,
            CtxKind::Aux(p) => return self.aux_menu_action(p, id),
            CtxKind::DockText(_) | CtxKind::TermEdit(_) => return self.dock_edit_action(kind, id),
        };
        if panel != self.active {
            self.set_active(panel);
        }
        if matches!(kind, CtxKind::PathEdit(_)) {
            // 글자 편집 명령만(파일 명령으로 빠지지 않는다 — 편집이 이미 끝났으면 아무 일도 하지 않는다).
            let _ = self.path_edit(id);
            return;
        }
        if matches!(kind, CtxKind::RenameEdit(_)) {
            // 이름 글자 편집 명령만(편집이 이미 끝났으면 아무 일도 하지 않는다 — 파일 명령으로 빠지지 않는다).
            let _ = self.rename_edit(id);
            return;
        }
        match id {
            // 경로/이름 복사 = 선택 전체(교차 폴더 포함)를 **화면에 보이는 순서**로 · 한 줄에 하나(dir2 win.rs:3105-3124).
            "ctx.copy_path" => {
                let text = self.panels[panel]
                    .selected_paths_in_view_order()
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\r\n");
                let _ = clipboard::write_text(&text);
            }
            "ctx.checksum" => {
                let paths = self.panels[panel].selected_paths_in_view_order();
                self.open_checksum(&paths);
            }
            "ctx.compare_panels" => self.compare_panels(),
            "ctx.find_dupes" => {
                let dirs: Vec<PathBuf> = self.panels[panel]
                    .selected_paths()
                    .into_iter()
                    .filter(|p| p.is_dir())
                    .collect();
                let roots = if dirs.is_empty() || matches!(kind, CtxKind::Bg(_)) {
                    vec![self.panels[panel].root_path()]
                } else {
                    dirs
                };
                self.start_dupes(roots);
            }
            "ctx.extract_here" | "ctx.extract_to" => {
                if let Some(p) = self.panels[panel].selected_paths().first().cloned() {
                    self.start_extract(&p, id == "ctx.extract_here");
                }
            }
            "ctx.copy_name" => {
                let text = self.panels[panel]
                    .selected_paths_in_view_order()
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
                    self.bg_target(self.panels[panel].root_path())
                } else {
                    // 사용자가 본 메뉴와 같은 대상(확장 동사로 열었으면 그 메뉴)으로 실행한다 — id는 그 메뉴의 것이다.
                    self.rows_target(self.panels[panel].selected_paths())
                };
                if self.platform.ctxmenu.invoke_async(other, &target) {
                    self.ctx_invoke_panel = Some(panel);
                } else {
                    let result = match &target {
                        MenuTarget::Bg(dir) | MenuTarget::BgExtended(dir) => {
                            self.platform.ctxmenu.invoke_bg(other, dir)
                        }
                        MenuTarget::Rows(sel) | MenuTarget::RowsExtended(sel) => {
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
            (Some(CtxKind::PathEdit(_)), _) => "pathedit",
            (Some(CtxKind::RenameEdit(_)), _) => "renameedit",
            (Some(CtxKind::DockText(_)), _) => "docktext",
            (Some(CtxKind::TermEdit(_)), _) => "termedit",
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

#[cfg(test)]
mod tidy_tests {
    use super::*;

    /// 구분선 정리 · 창 높이에 맞춘 행 수(사용자 10-03 "속성 밑에 빈 구분자 2개 · 하단 메뉴 잘림").
    #[test]
    fn separators_collapse_and_rows_fit_the_window() {
        let it = |id: &str| CtxItem::item(id, id);
        let sub = CtxItem::submenu(
            "s",
            "s",
            vec![CtxItem::Separator, it("x"), CtxItem::Separator],
        );
        let out = tidy_separators(vec![
            CtxItem::Separator,
            it("a"),
            CtxItem::Separator,
            CtxItem::Separator,
            sub,
            CtxItem::Separator,
        ]);
        let shape: Vec<bool> = out
            .iter()
            .map(|c| matches!(c, CtxItem::Separator))
            .collect();
        assert_eq!(shape, [false, true, false]);
        let CtxItem::Item { children, .. } = &out[2] else {
            panic!("submenu");
        };
        assert_eq!(children.len(), 1, "하위 메뉴도 정리");
        // 높이 800 · 배율 1: (800 − 10) / 26 = 30행 · 배율 1.5: (800 − 16) / 39 = 20행 · 아주 낮은 창도 3행은 보인다.
        assert_eq!(ctx_rows_that_fit(800, 1.0), 30);
        assert_eq!(ctx_rows_that_fit(800, 1.5), 20);
        assert_eq!(ctx_rows_that_fit(40, 1.0), 3);
    }
}
