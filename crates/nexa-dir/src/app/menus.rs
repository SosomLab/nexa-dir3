//! App — 메뉴바·툴바 구성(dir2 `win.rs` `build_menus`/`build_toolbar` 대응 · docs/port/30 CMD-120~167) + **명령 한 길**(`command`).
//!
//! 메뉴·툴바·단축키·기동 명령이 같은 문자열 id(`ndir_settings::commands::COMMANDS`)를 쓴다(SKEL-421). 라벨은 i18n 키(DR-14).
//! 체크/라디오 상태는 설정에서 매번 계산(`sync_menu_checks`) — 설정이 단일 원천.

use crate::*;
use ndir_settings::commands::COMMANDS;

/// 명령 id → 라벨 i18n 키(표에 없는 id = id 그대로).
fn label_key(id: &str) -> &str {
    COMMANDS.iter().find(|c| c.id == id).map_or(id, |c| c.label)
}

fn item(id: &str) -> MenuEntry {
    MenuEntry::Item(ComboItem::new(id, tr(label_key(id))))
}

fn items(ids: &[&str]) -> Vec<MenuEntry> {
    let mut out = Vec::new();
    for id in ids {
        if *id == "-" {
            out.push(MenuEntry::Separator);
        } else if menu_has(id, platform::has_dotfile_toggle()) {
            out.push(item(id));
        }
    }
    out
}

/// 이 OS의 메뉴에 그 명령이 있는가(순수): "점 파일 표시"는 점 파일 토글이 있는 OS(Windows)에만.
pub(crate) fn menu_has(id: &str, dotfile_toggle: bool) -> bool {
    id != "view.dot" || dotfile_toggle
}

/// 메뉴 항목 전체(단축키·체크 동기화 대상).
pub(crate) const MENU_IDS: &[&str] = &[
    "file.new_tab",
    "file.close_tab",
    "-",
    "file.new_folder",
    "file.new_file",
    "-",
    "file.prefs",
    "-",
    "file.exit",
    "edit.undo",
    "edit.redo",
    "-",
    "edit.cut",
    "edit.copy",
    "edit.paste",
    "-",
    "edit.select_all",
    "-",
    "edit.bulk_rename",
    "view.mode_tree",
    "view.mode_flat",
    "view.mode_tiles",
    "-",
    "view.panel_dual",
    "view.panel_single",
    "view.info_dual",
    "view.info_single",
    "view.col_width_sync",
    "-",
    "view.hidden",
    "view.dot",
    "view.dock",
    "view.launcher",
    "view.always_on_top",
    "-",
    "view.refresh",
    "view.preview_window",
    "-",
    "view.theme_system",
    "view.theme_light",
    "view.theme_dark",
    "-",
    "view.lang_system",
    "nav.back",
    "nav.forward",
    "nav.up",
    "-",
    "tab.next",
    "tab.prev",
    "-",
    "panel.switch",
    "help.about",
    "help.license",
    "help.selfcheck",
];

impl App {
    /// 메뉴바 정의(dir2 File · Edit · View · [Go] · Help — Cloud 메뉴는 M5 플러그인/클라우드에서).
    pub(crate) fn build_menus(settings: &Settings) -> Vec<MenuDef> {
        let home = ndir_settings::config_dir().unwrap_or_else(std::env::temp_dir);
        let mut view = items(&MENU_IDS[19..43]);
        // 언어 목록(동적 명령 `lang:<code>` — 단축키 재정의 대상 아님).
        for (code, name) in ndir_i18n::discover(&home) {
            view.push(MenuEntry::Item(ComboItem::new(
                format!("lang:{code}"),
                name,
            )));
        }
        let _ = settings;
        vec![
            MenuDef::new(tr("menu.file"), items(&MENU_IDS[..9])),
            MenuDef::new(tr("menu.edit"), items(&MENU_IDS[9..19])),
            MenuDef::new(tr("menu.view"), view),
            MenuDef::new(tr("menu.go"), items(&MENU_IDS[43..51])),
            MenuDef::new(tr("menu.help"), items(&MENU_IDS[51..])),
        ]
    }

    /// 런처 항목 실행 — 활성 패널 현재 폴더(`%path%`) · 결과 = 상태줄(`launcher.ran/failed` · dir2 무중단 규약).
    pub(crate) fn launch_item(&mut self, idx: usize) {
        let Some(item) = self.launcher_items.get(idx).cloned() else {
            return;
        };
        let folder = self.term_cwd(0);
        let mut inv = Invalidations::default();
        let text = match launcher::launch(&item, &folder) {
            Ok(()) => trf("launcher.ran", &[&item.label]),
            Err(e) => format!("{} — {e}", trf("launcher.failed", &[&item.label])),
        };
        self.statusbar.set_left(&text, &mut inv);
        self.toasts.push(
            toast::ToastKind::Info,
            tr("menu.view.launcher"),
            text.clone(),
        );
        self.launcher_last = text;
        self.redraw();
    }

    /// 설정 `launcher.items` 변경 → 바 재구성 + 재배치.
    pub(crate) fn rebuild_launcher(&mut self) {
        self.launcher_items =
            launcher::parse_items(self.settings.get("launcher.items").unwrap_or(""));
        let (bar, pending) = App::make_launcherbar(&self.launcher_items, &self.settings);
        self.launcherbar = bar;
        self.launcher_icons_pending = pending;
        self.launcher_icon_ver = nexa_fs::shell::IconService::global().version();
        self.layout();
    }

    /// 메뉴 단축키 열 = 키맵 표시(설정 재정의 반영).
    pub(crate) fn sync_menu_shortcuts(&mut self) {
        for id in MENU_IDS.iter().filter(|id| **id != "-") {
            let text = self.keymap.menu_display_of(id);
            self.menubar.set_shortcut(id, &text);
        }
    }

    /// 체크·라디오 = 설정값(dir2 `build_menus` 인자 14개와 같은 출처).
    pub(crate) fn sync_menu_checks(&mut self) {
        let s = &self.settings;
        let checks = [
            ("view.dock", s.flag("dock.visible")),
            ("view.launcher", s.flag("launcher.visible")),
            ("view.always_on_top", s.flag("window.always_on_top")),
            ("view.col_width_sync", s.flag("list.col_width_sync")),
        ];
        let mode = s.get("list.view_mode").unwrap_or("tree").to_string();
        let panel = s.get("layout.panel_mode").unwrap_or("dual").to_string();
        let info = s.get("layout.info_mode").unwrap_or("dual").to_string();
        let theme = s.theme_mode();
        let lang = s.lang_setting().to_string();
        let mut inv = Invalidations::default();
        for (id, on) in checks {
            self.menubar.set_checked(id, on, &mut inv);
        }
        self.menubar.set_radio(&format!("view.mode_{mode}"));
        self.menubar.set_radio(&format!("view.panel_{panel}"));
        self.menubar.set_radio(&format!("view.info_{info}"));
        self.menubar
            .set_radio(&format!("view.theme_{}", theme.as_str()));
        self.menubar.set_radio(&if lang == "system" {
            "view.lang_system".to_string()
        } else {
            format!("lang:{lang}")
        });
        // 툴바 토글도 같은 출처.
        let tool = [
            ("view.panel_toggle", panel == "dual"),
            ("view.dock", s.flag("dock.visible")),
            ("view.always_on_top", s.flag("window.always_on_top")),
            ("view.info_toggle", info == "dual"),
            ("view.col_width_sync", s.flag("list.col_width_sync")),
            ("view.mode_tree", mode == "tree"),
            ("view.mode_flat", mode == "flat"),
            ("view.mode_tiles", mode == "tiles"),
        ];
        for (id, on) in tool {
            self.toolbar.set_item_checked(id, on, &mut inv);
        }
        self.sync_view_checks();
    }

    /// 툴바 그룹(dir2 `build_toolbar` — **순서/표시 = 설정 `toolbar.layout`** · 블록 1개 = 도크 그룹 1개(T-71 `order::TOOLBAR_BLOCKS` 기본 refresh · panel · view · show · settings) ·
    /// 블록 사이 구분선 · 그룹 숨김 = 통째 · 글리프 폴백 · SVG 아이콘은 T-30).
    pub(crate) fn build_tool_groups(settings: &Settings, icon_px: u32) -> Vec<ToolGroup> {
        // dir2 SVG 아이콘(T-30 · `icons.rs` · nexa-gfx svg 마스크 → 테마 틴트) · 없으면 글리프(자산 미등록·파싱 실패 격리).
        let g = |id: &str, glyph: &str, tip_key: &str| {
            let icon = icons::asset_of(id)
                .and_then(|name| icons::toolbar_mask(name, icon_px))
                .map_or_else(
                    || ToolIcon::Glyph(glyph.to_string()),
                    |(w, h, alpha)| ToolIcon::Mask { w, h, alpha },
                );
            ToolItem::new(id, icon).tip(tr(tip_key))
        };
        let scope = settings
            .get("list.view_scope")
            .filter(|s| matches!(*s, "global" | "panel" | "tab"))
            .unwrap_or("dir");
        let scoped = |it: ToolItem, label_key: &str| {
            it.tip(format!(
                "{} — {}",
                tr(label_key),
                tr(&format!("pref.viewScope.{scope}"))
            ))
        };
        let item = |block: &str, key: &str| -> Option<ToolItem> {
            Some(match (block, key) {
                ("panel", "toggle") => g("view.panel_toggle", "▌▐", "cmd.panelToggle"),
                ("panel", "dock") => g("view.dock", "▂", "menu.view.dock"),
                ("panel", "info") => g("view.info_toggle", "ⓘ", "cmd.infoToggle"),
                ("panel", "colsync") => g("view.col_width_sync", "⇔", "menu.view.colWidthSync"),
                ("view", "tree") => g("view.mode_tree", "├─", "menu.view.modeTree"),
                ("view", "flat") => g("view.mode_flat", "☰", "menu.view.modeFlat"),
                ("view", "tiles") => g("view.mode_tiles", "▦", "menu.view.modeTiles"),
                // 새로 고침 그룹 = 새로 고침 · 항상 위(사용자 10-03 "항상 최상위 고정은 새로고침 그룹으로 · 새로고침 다음에").
                ("refresh", "refresh") => g("view.refresh", "⟳", "menu.view.refresh"),
                ("refresh", "ontop") => g("view.always_on_top", "📌", "menu.view.alwaysOnTop"),
                ("settings", _) => g("file.prefs", "⚙", "menu.file.prefs"),
                // 보기 옵션 3종 = 툴팁에 적용 범위를 덧붙인다(dir2 08-02 `scope_tip` "라벨 — 범위").
                ("show", "hidden") => scoped(
                    g("view.hidden", "👁", "menu.view.hidden"),
                    "menu.view.hidden",
                ),
                ("show", "dot") => scoped(g("view.dot", "…", "menu.view.dot"), "menu.view.dot"),
                ("show", "foldersfirst") => scoped(
                    g("view.folders_first", "▲", "pref.sortFoldersFirst"),
                    "pref.sortFoldersFirst",
                ),
                // 대소문자 구분 정렬(사용자 10-03 "폴더 우선정렬 옆에 · 보기 토글과 동일한 개념") — 네 번째 탭 보기 옵션.
                ("show", "casesensitive") => scoped(
                    g("view.case_sensitive", "Aa", "pref.sortCaseSensitive"),
                    "pref.sortCaseSensitive",
                ),
                _ => return None,
            })
        };
        let layout = settings.get("toolbar.layout").unwrap_or("");
        let mut out: Vec<ToolGroup> = Vec::new();
        for (block, bvis, items) in
            crate::order::parse_order_with(crate::order::toolbar_blocks(), layout)
        {
            if !bvis {
                continue;
            }
            let tools: Vec<ToolItem> = if items.is_empty() {
                item(&block, "").into_iter().collect()
            } else {
                items
                    .iter()
                    .filter(|(_, v)| *v)
                    .filter_map(|(k, _)| item(&block, k))
                    .collect()
            };
            if tools.is_empty() {
                continue; // 전부 숨긴 블록 = 그룹도 없음
            }
            let title = app::order::toolbar_group_title(&block);
            out.push(ToolGroup::new(block, title, tools));
        }
        out
    }

    /// 툴바 항목 평탄 목록(그룹 사이 = 구분자) — 시험·점검용(종전 단일 툴바 구성과 같은 순서).
    #[cfg(test)]
    pub(crate) fn build_toolbar(settings: &Settings, icon_px: u32) -> Vec<ToolItem> {
        let mut out: Vec<ToolItem> = Vec::new();
        for g in App::build_tool_groups(settings, icon_px) {
            if !out.is_empty() {
                out.push(ToolItem::separator());
            }
            out.extend(g.items);
        }
        out
    }

    /// 선택 항목을 휴지통으로(Trash 포트 · 실패/미지원 = 토스트 한 번) → 히스토리(undo = 복원 · T-51 B-2c) → 그 폴더를 보는 탭 전부 재열람.
    pub(crate) fn delete_to_trash(&mut self) {
        let paths = self.panels[self.active].selected_paths();
        if paths.is_empty() {
            return;
        }
        let mut inv = Invalidations::default();
        match self.platform.trash.trash(&paths) {
            Ok(n) => {
                self.history.push(Box::new(trashop::TrashOp::new(
                    paths.clone(),
                    trf("del.recycleOp", &[&n.to_string()]),
                    Rc::clone(&self.platform.trash),
                )));
                self.toasts.push(
                    toast::ToastKind::Info,
                    tr("menu.edit.delete"),
                    trf("status.deletedCount", &[&n.to_string()]),
                );
                let dir = self.panels[self.active].root_path();
                for p in &mut self.panels {
                    if p.root_path() == dir {
                        p.reopen(&mut inv);
                    }
                }
            }
            Err(e) => {
                self.toasts.push(
                    toast::ToastKind::Warn,
                    tr("menu.edit.delete"),
                    e.to_string(),
                );
            }
        }
    }

    /// 설정 토글(on/off) + 저장. 저장 실패는 조용히(다음 종료 때 다시).
    fn toggle_flag(&mut self, key: &str) -> bool {
        let on = !self.settings.flag(key);
        let _ = self.settings.set(key, if on { "on" } else { "off" });
        let _ = self.settings.save();
        on
    }

    fn set_setting(&mut self, key: &str, raw: &str) {
        let _ = self.settings.set(key, raw);
        let _ = self.settings.save();
    }

    /// 보기 옵션(숨김 · Dot · 폴더 우선) 변경 → 두 패널 전 탭 무간섭 재열람.
    /// 설정 창 갱신 — 강제 값 목록을 먼저 넣고 스냅샷을 다시 읽는다.
    pub(crate) fn refresh_prefs(&mut self) {
        self.prefs_win.set_forced(self.prefs_forced());
        self.prefs_win.refresh(&self.settings);
    }

    /// 지금 다른 설정이 대신 정하고 있는 값들(키 · 실제로 쓰이는 값 · 이유) — 설정 창이 그 값을 보여 주고 잠근다.
    /// 저장된 사용자 값은 건드리지 않는다(강제가 풀리면 그대로 다시 쓰인다 · 사용자 10-03 "성능향상모드처럼 … 원래 값으로").
    /// - "시스템 기본 터미널과 같게"(`term.follow_windows_terminal`)가 켜져 있고 기본 터미널 글꼴을 찾았으면:
    ///   `term.font_face` = 그 글꼴 · 프로필이 크기도 주면(Windows Terminal) `term.font_size`도.
    pub(crate) fn prefs_forced(&self) -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        if let Some(p) = self
            .wt_profile
            .as_ref()
            .filter(|_| self.settings.flag("term.follow_windows_terminal"))
        {
            let by = tr("pref.termFollowWt");
            if let Some(face) = p.faces.first() {
                out.push((
                    "term.font_face".to_string(),
                    face.clone(),
                    trf("pref.forcedBy", &[&by, face]),
                ));
            }
            if let Some(pt) = p.size_pt {
                let shown = format!("{pt} pt");
                out.push((
                    "term.font_size".to_string(),
                    shown.clone(),
                    trf("pref.forcedBy", &[&by, &shown]),
                ));
            }
        }
        out
    }

    /// 탭 바 모양 설정(`tabs.multiline` 기본 on · `tabs.scroll_buttons` end/start/split) → 두 패널.
    pub(crate) fn apply_tab_style(&mut self) {
        use nexa_ctl::controls::ScrollButtons;
        let multiline = self.settings.flag("tabs.multiline");
        let buttons = match self.settings.get("tabs.scroll_buttons") {
            Some("start") => ScrollButtons::Start,
            Some("split") => ScrollButtons::Split,
            _ => ScrollButtons::End,
        };
        let mut inv = Invalidations::default();
        for p in &mut self.panels {
            p.set_tab_style(multiline, buttons, &mut inv);
        }
        self.layout();
        self.redraw();
    }

    /// 글꼴 장식 설정(dir2 X-12 · `list.folder_bold`/`list.header_bold`/`list.header_italic`) → 두 패널 그리드.
    pub(crate) fn apply_font_decor(&mut self) {
        let (fb, hb, hi) = (
            self.settings.flag("list.folder_bold"),
            self.settings.flag("list.header_bold"),
            self.settings.flag("list.header_italic"),
        );
        let mut inv = Invalidations::default();
        for p in &mut self.panels {
            p.rows_mut().set_font_decor(fb, hb, hi, &mut inv);
        }
        if !inv.is_empty() {
            self.redraw();
        }
    }

    /// 숨김 · Dot · 폴더 우선 · 대소문자 구분 체크(메뉴 · 툴바) = **활성 패널의 활성 탭 값**(dir2 08-02 미러 — 탭·패널을 바꾸면 따라간다).
    /// `update_status` 길목이 부른다.
    pub(crate) fn sync_view_checks(&mut self) {
        let (hidden, dot, folders, case) = self.panels[self.active].active_view();
        let mut inv = Invalidations::default();
        self.menubar.set_checked("view.hidden", hidden, &mut inv);
        self.menubar.set_checked("view.dot", dot, &mut inv);
        for (id, on) in [
            ("view.hidden", hidden),
            ("view.dot", dot),
            ("view.folders_first", folders),
            ("view.case_sensitive", case),
        ] {
            self.toolbar.set_item_checked(id, on, &mut inv);
        }
        if !inv.is_empty() {
            self.redraw();
        }
    }

    /// 보기 옵션 관리 방법(설정 `list.view_scope` · 사용자 10-03 4택): `global` 전체 통일 · `panel` 좌/우 패널별(패널 안 전 탭) ·
    /// `tab` 탭별 · `dir` 폴더별(같은 폴더를 보는 탭은 좌우 어디든 공통 · 폴더마다 기억 — **기본**).
    pub(crate) fn view_scope(&self) -> &str {
        match self.settings.get("list.view_scope") {
            Some(s @ ("global" | "panel" | "tab")) => s,
            _ => "dir",
        }
    }

    /// 폴더의 보기 옵션(범위 "폴더"): 기억된 값 · 없으면 설정 기본값.
    pub(crate) fn dir_view_of(&self, dir: &std::path::Path) -> filelist::ViewOpts {
        self.session_keep
            .dir_views
            .iter()
            .find(|(p, _)| p == dir)
            .map_or_else(
                || list_opts(&self.settings).view(),
                |(_, f)| filelist::ListOpts::view_of_flags(*f),
            )
    }

    /// 폴더의 보기 옵션 기억(설정 기본값과 같으면 지운다 — 기본을 따르는 폴더는 적어 두지 않는다 · 상한 초과 = 오래된 것부터).
    fn remember_dir_view(&mut self, dir: &std::path::Path, view: filelist::ViewOpts) {
        let base = list_opts(&self.settings);
        let v = &mut self.session_keep.dir_views;
        v.retain(|(p, _)| p != dir);
        if view != base.view() {
            v.push((dir.to_path_buf(), base.with_view(view).view_flags()));
            if v.len() > crate::session::DIR_VIEWS_MAX {
                let cut = v.len() - crate::session::DIR_VIEWS_MAX;
                v.drain(..cut);
            }
        }
        self.session_save.mark(Instant::now());
    }

    /// 범위 "폴더": 방금 폴더를 옮긴(또는 새로 열린) 활성 탭을 그 폴더의 보기 옵션에 맞춘다 — 같은 폴더를 보는 다른 탭이
    /// 있으면 그 값(이미 공통) · 없으면 기억된 값 · 그것도 없으면 설정 기본값. `update_status` 길목이 부른다.
    pub(crate) fn sync_dir_views(&mut self) {
        let dir_scope = self.view_scope() == "dir";
        let mut inv = Invalidations::default();
        for pi in 0..self.panels.len() {
            if !self.panels[pi].take_navigated() || !dir_scope {
                continue;
            }
            let dir = self.panels[pi].root_path();
            let view = self.dir_view_of(&dir);
            self.panels[pi].set_view(false, view, &mut inv);
        }
        if !inv.is_empty() {
            self.redraw();
        }
    }

    /// 보기 옵션 토글 3종(dir2 08-02 `CMD_TOGGLE_HIDDEN/DOTFILES/FOLDERS_FIRST` · 값의 주인 = 탭): 활성 탭의 값을 뒤집어
    /// 범위(`list.view_scope`)만큼 적용한다. 설정값(`list.show_*` = 새 탭의 기본값)은 건드리지 않는다(사용자 10-03).
    fn toggle_view_option(&mut self, id: &str) {
        let (mut hidden, mut dot, mut folders, mut case) = self.panels[self.active].active_view();
        match id {
            "view.hidden" => hidden = !hidden,
            "view.dot" => dot = !dot,
            "view.case_sensitive" => case = !case,
            _ => folders = !folders,
        }
        let scope = self.view_scope().to_string();
        let view = (hidden, dot, folders, case);
        let mut inv = Invalidations::default();
        if scope == "dir" {
            // 폴더별: 그 폴더를 보는 탭 전부(좌우 모두) + 폴더에 기억.
            let dir = self.panels[self.active].root_path();
            for p in &mut self.panels {
                p.set_view_for_dir(&dir, view, &mut inv);
            }
            self.remember_dir_view(&dir, view);
        } else {
            let targets: &[usize] = if scope == "global" {
                &[0, 1]
            } else if self.active == 0 {
                &[0]
            } else {
                &[1]
            };
            for &pi in targets {
                self.panels[pi].set_view(scope != "tab", view, &mut inv);
            }
        }
        self.sync_menu_checks();
        self.update_status();
        self.redraw();
    }

    /// 테마 모드 적용(설정 저장 포함 · 창 장식도 따라간다).
    fn apply_theme_mode(&mut self, mode: ThemeMode) {
        self.set_setting("ui.theme", mode.as_str());
        let wt = self.window.as_ref().and_then(|w| w.theme());
        self.theme = theme::resolve(mode, wt);
        if let Some(w) = &self.window {
            w.set_theme(theme::window_theme(mode));
        }
        self.sync_menu_checks();
        self.redraw();
    }

    /// 명령 한 길 — 메뉴 · 툴바 · 단축키 · 기동 명령이 전부 여기로(SKEL-421). 모르는 id = 상태줄 안내(구현 단계 표시).
    pub(crate) fn command(&mut self, id: &str) {
        let mut inv = Invalidations::default();
        let a = self.active;
        // 터미널 포커스 중 편집 명령은 터미널로(dir2 Edit 메뉴 규약 · T-61).
        if self.term_focused().is_some() {
            let done = match id {
                "edit.copy" => {
                    self.term_copy();
                    true
                }
                "edit.paste" => {
                    self.term_paste();
                    true
                }
                "edit.select_all" => {
                    self.term_select_all();
                    true
                }
                _ => false,
            };
            if done {
                return;
            }
        }
        // 인라인 이름 바꾸기 중 편집 명령은 편집 필드로(dir2 do_clip ②).
        if self.rename_edit(id) {
            return;
        }
        // ⚠ 경로 바 편집 중 편집 명령(Ctrl+C/X/V/Z/A · Delete)은 **경로 글자**에(dir2 do_clip ① · win.rs:7246-7286) —
        // 이 분기가 없으면 Ctrl+V가 파일 붙여넣기(전송 시작) · Ctrl+Z가 마지막 파일 작업 되돌리기로 실행된다(GAP-011).
        if self.path_edit(id) {
            return;
        }
        match id {
            "file.exit" => self.exit_requested = true,
            "file.new_folder" => self.create_new(true),
            "file.new_file" => self.create_new(false),
            "edit.rename" => self.begin_rename(),
            "edit.bulk_rename" => self.open_bulk_rename(),
            "file.prefs" => self.open_prefs = true,
            "keys.window" => self.open_keys = true,
            "file.new_tab" => self.panels[a].new_tab(&mut inv),
            "file.close_tab" => {
                let i = self.panels[a].active_index();
                self.panels[a].close_tab(i, &mut inv);
            }
            "tab.next" => self.panels[a].next_tab(&mut inv),
            "tab.prev" => self.panels[a].prev_tab(&mut inv),
            "nav.back" => self.panels[a].nav_back(&mut inv),
            "nav.forward" => self.panels[a].nav_forward(&mut inv),
            "nav.up" => self.panels[a].nav_up(&mut inv),
            "nav.home" => self.panels[a].nav_home(&mut inv),
            "edit.select_all" => self.panels[a].key_event(&InputEvent::SelectAll, &mut inv),
            "edit.delete" => self.delete_to_trash(),
            "edit.delete_permanent" => self.delete_permanent_ask(),
            "edit.copy" => self.clip_write(false),
            "edit.cut" => self.clip_write(true),
            "edit.paste" => self.paste(),
            "edit.undo" => self.history_step(false),
            "edit.redo" => self.history_step(true),
            "view.refresh" => {
                for p in &mut self.panels {
                    p.reopen(&mut inv);
                }
            }
            // 점 파일 토글이 없는 OS(Linux · macOS)에서는 단축키로 불려도 아무 일도 하지 않는다(점 파일 = 숨김 파일 → view.hidden).
            "view.dot" if !platform::has_dotfile_toggle() => {}
            "view.hidden" | "view.dot" | "view.folders_first" | "view.case_sensitive" => {
                self.toggle_view_option(id);
            }
            "view.dock" | "view.launcher" | "view.col_width_sync" => {
                let key = match id {
                    "view.dock" => "dock.visible",
                    "view.launcher" => "launcher.visible",
                    _ => "list.col_width_sync",
                };
                self.toggle_flag(key);
                self.sync_menu_checks();
                // 동기화를 켜는 순간 활성 패널 폭으로 반대 패널 즉시 정렬(dir2 07-18).
                if id == "view.col_width_sync"
                    && self.dual
                    && self.settings.flag("list.col_width_sync")
                {
                    self.sync_col_widths_from(a);
                }
                if id == "view.dock" || id == "view.launcher" {
                    self.layout();
                }
            }
            "view.always_on_top" => {
                let on = self.toggle_flag("window.always_on_top");
                if let Some(w) = &self.window {
                    w.set_window_level(if on {
                        winit::window::WindowLevel::AlwaysOnTop
                    } else {
                        winit::window::WindowLevel::Normal
                    });
                }
                self.sync_menu_checks();
            }
            "view.theme_system" => self.apply_theme_mode(ThemeMode::System),
            "view.theme_light" => self.apply_theme_mode(ThemeMode::Light),
            "view.theme_dark" => self.apply_theme_mode(ThemeMode::Dark),
            "view.theme_cycle" => {
                let cur = self.settings.theme_mode();
                let i = ThemeMode::ALL.iter().position(|m| *m == cur).unwrap_or(0);
                self.apply_theme_mode(ThemeMode::ALL[(i + 1) % ThemeMode::ALL.len()]);
            }
            "view.panel_dual" | "view.panel_single" | "view.panel_toggle" => {
                let dual = match id {
                    "view.panel_dual" => true,
                    "view.panel_single" => false,
                    _ => !self.dual,
                };
                self.set_setting("layout.panel_mode", if dual { "dual" } else { "single" });
                self.dual = dual;
                self.set_active(if dual { self.active } else { 0 });
                self.sync_menu_checks();
                self.layout();
            }
            "view.info_dual" | "view.info_single" | "view.info_toggle" => {
                let cur = self.settings.get("layout.info_mode").unwrap_or("dual") == "dual";
                let dual = match id {
                    "view.info_dual" => true,
                    "view.info_single" => false,
                    _ => !cur,
                };
                self.set_setting("layout.info_mode", if dual { "dual" } else { "single" });
                self.sync_menu_checks();
                self.layout();
            }
            "view.mode_tree" | "view.mode_flat" | "view.mode_tiles" => {
                let mode = &id["view.mode_".len()..];
                self.set_setting("list.view_mode", mode);
                self.panels[a].set_view_mode(view_mode_of(mode), &mut inv);
                self.sync_menu_checks();
            }
            "view.lang_system" => self.switch_lang("system"),
            "panel.switch" => {
                if self.dual {
                    self.set_active(1 - self.active);
                }
            }
            "help.selfcheck" => self.open_check = true,
            // 라이선스 창(T-80 · LIC-105): 열려 있으면 닫기 토글.
            "help.license" => {
                if self.license_win.is_open() {
                    self.license_win.close();
                } else {
                    self.open_license = true;
                }
            }
            "view.preview_window" => self.open_preview_window(a),
            "cmd.contextMenu" => self.open_row_menu_at_caret(a),
            // 활성화(Enter와 같은 일 — 폴더 = 진입 · 파일 = 열기): 기동 명령·키맵에서 이름으로 부를 수 있게(행 메뉴의 "열기"와 같은 id).
            // `nav.activate` = 키맵 id(Enter · Alt+↓ · macOS ⌘↓/⌘O) · `cmd.activate` = 행 메뉴 "열기"의 id — 같은 일.
            "nav.activate" | "cmd.activate" => {
                if let Some(row) = self.panels[a].rows().caret() {
                    self.panels[a].activate_row(row, &mut inv);
                    if let Some(path) = self.panels[a].take_open() {
                        self.open_external(&path);
                    }
                }
            }
            "help.about" => self.about_ask(),
            _ if id.starts_with("launch:") => {
                if let Ok(i) = id["launch:".len()..].trim().parse::<usize>() {
                    self.launch_item(i);
                }
            }
            _ if id.starts_with("lang:") => self.switch_lang(&id["lang:".len()..]),
            _ => {
                // 아직 없는 명령 — 상태줄에 id(검증 매트릭스 ⚠ · 구현 단계가 오면 위 가지로).
                self.statusbar
                    .set_left(&format!("{id}: {}", tr("cmd.notYet")), &mut inv);
                self.redraw();
                return;
            }
        }
        self.update_status();
        self.redraw();
    }

    /// 언어 전환 — 설정 + i18n 재활성 + 메뉴·툴바·컬럼 라벨 재구성.
    fn switch_lang(&mut self, code: &str) {
        self.set_setting("ui.lang", code);
        self.after_setting_changed("ui.lang");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 메뉴 id는 전부 명령 표에 있다(동적 `lang:` 제외) · 구분선 경계가 메뉴 5개와 맞는다.
    #[test]
    fn menu_ids_are_commands() {
        for id in MENU_IDS.iter().filter(|id| **id != "-") {
            assert!(COMMANDS.iter().any(|c| c.id == *id), "{id} not in COMMANDS");
        }
        assert_eq!(MENU_IDS[0], "file.new_tab");
        assert_eq!(MENU_IDS[9], "edit.undo");
        assert_eq!(MENU_IDS[19], "view.mode_tree");
        assert_eq!(MENU_IDS[42], "view.lang_system");
        assert!(MENU_IDS[19..43].contains(&"view.preview_window"));
        assert_eq!(MENU_IDS[43], "nav.back");
        assert_eq!(MENU_IDS[51], "help.about");
        assert_eq!(MENU_IDS[52], "help.license");
        assert_eq!(MENU_IDS[53], "help.selfcheck");
        assert_eq!(MENU_IDS.len(), 54);
    }

    #[test]
    fn menus_build_with_labels() {
        let s = Settings::from_text(std::env::temp_dir().join("ndir-menus-test.conf"), "");
        let menus = App::build_menus(&s);
        assert_eq!(menus.len(), 5);
        assert!(menus[0].entries.len() >= 9);
        let tools = App::build_toolbar(&s, 20);
        assert!(tools.len() >= 13);
    }

    /// 원장 전수(T-90 · docs/port/30 §1 CMD-001~067): dir2 명령 상수/탭 메뉴 함수가 적힌 행마다 dir3 문자열 id 대응이 있고 그 id가
    /// 명령 표(COMMANDS) 또는 메뉴/탭 어휘에 있다. 동적(`+ i`)·클라우드(미이식 M8)·설명만 있는 행은 대상 밖(개수만 센다).
    #[test]
    fn dir2_catalog_menu_commands_map_to_dir3_ids() {
        const MAP: &[(&str, &str)] = &[
            ("CMD_NEW_TAB", "file.new_tab"),
            ("CMD_CLOSE_TAB", "file.close_tab"),
            ("CMD_NEW_FOLDER", "file.new_folder"),
            ("CMD_NEW_FILE", "file.new_file"),
            ("CMD_PREFS", "file.prefs"),
            ("CMD_EXIT", "file.exit"),
            ("CMD_UNDO", "edit.undo"),
            ("CMD_REDO", "edit.redo"),
            ("CMD_CUT", "edit.cut"),
            ("CMD_COPY", "edit.copy"),
            ("CMD_PASTE", "edit.paste"),
            ("CMD_SELECT_ALL", "edit.select_all"),
            ("CMD_BULK_RENAME", "edit.bulk_rename"),
            ("CMD_VIEW_TREE", "view.mode_tree"),
            ("CMD_VIEW_FLAT", "view.mode_flat"),
            ("CMD_VIEW_TILES", "view.mode_tiles"),
            ("CMD_PANEL_DUAL", "view.panel_dual"),
            ("CMD_PANEL_SINGLE", "view.panel_single"),
            ("CMD_PANEL_TOGGLE", "view.panel_toggle"),
            ("CMD_INFO_DUAL", "view.info_dual"),
            ("CMD_INFO_SINGLE", "view.info_single"),
            ("CMD_INFO_TOGGLE", "view.info_toggle"),
            ("CMD_COLW_SYNC", "view.col_width_sync"),
            ("CMD_TOGGLE_HIDDEN", "view.hidden"),
            ("CMD_TOGGLE_DOTFILES", "view.dot"),
            ("CMD_TOGGLE_FOLDERS_FIRST", "view.folders_first"),
            ("CMD_TOGGLE_DOCK", "view.dock"),
            ("CMD_TOGGLE_LAUNCHER", "view.launcher"),
            ("CMD_TOGGLE_TOPMOST", "view.always_on_top"),
            ("CMD_REFRESH", "view.refresh"),
            ("CMD_THEME_SYSTEM", "view.theme_system"),
            ("CMD_THEME_LIGHT", "view.theme_light"),
            ("CMD_THEME_DARK", "view.theme_dark"),
            ("CMD_LANG_SYSTEM", "view.lang_system"),
            ("CMD_ABOUT", "help.about"),
            ("toggle_tab_lock", "tab.lock"),
            ("toggle_tab_pin", "tab.pin"),
            ("duplicate_tab", "tab.duplicate"),
            ("new_tab(Ctrl+T · [+] 동일 경로)", "tab.new"),
            ("close_tab", "tab.close"),
        ];
        const TAB_IDS: &[&str] = &[
            "tab.lock",
            "tab.pin",
            "tab.duplicate",
            "tab.new",
            "tab.close",
        ];
        let doc = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/port/30-catalog-commands-shortcuts.md"
        ))
        .expect("docs/port/30");
        let known = |id: &str| {
            COMMANDS.iter().any(|c| c.id == id) || MENU_IDS.contains(&id) || TAB_IDS.contains(&id)
        };
        let (mut mapped, mut skipped, mut problems) = (Vec::new(), 0usize, Vec::new());
        for line in doc.lines() {
            if !line.starts_with("| CMD-0") && !line.starts_with("| CMD-1") {
                continue;
            }
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            if cells.len() < 7 {
                continue;
            }
            let id = cells[1];
            let num: u32 = id.trim_start_matches("CMD-").parse().unwrap_or(999);
            if num > 67 {
                break; // §1-5부터(컨텍스트 메뉴 이후)는 다른 묶음 행
            }
            let what = cells[5].trim_matches('`');
            let is_const =
                what.starts_with("CMD_") && !what.contains('+') && !what.contains("CLOUD");
            let is_tab_fn = MAP
                .iter()
                .any(|(k, _)| *k == what && !k.starts_with("CMD_"));
            if !is_const && !is_tab_fn {
                skipped += 1;
                continue;
            }
            match MAP.iter().find(|(k, _)| *k == what) {
                Some((_, dir3)) if known(dir3) => mapped.push(id.to_string()),
                Some((_, dir3)) => problems.push(format!("{id} {what} → {dir3}: dir3 어휘에 없음")),
                None => problems.push(format!("{id} {what}: 대응표에 없음")),
            }
        }
        assert!(problems.is_empty(), "{problems:?}");
        assert!(
            mapped.len() >= 45,
            "대응 {} · 건너뜀(동적/클라우드/설명) {}",
            mapped.len(),
            skipped
        );
    }
}
