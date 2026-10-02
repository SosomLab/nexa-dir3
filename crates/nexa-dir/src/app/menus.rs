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
        } else {
            out.push(item(id));
        }
    }
    out
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

    /// 런처 바 항목(dir2 WINA-029: 항목 순서대로 버튼 · `-` = 구분선 · 아이콘 추출(T-30)은 후속 — 라벨 버튼).
    pub(crate) fn build_launcherbar(items: &[launcher::LauncherItem]) -> Vec<ToolItem> {
        items
            .iter()
            .enumerate()
            .map(|(i, it)| {
                if it.is_separator() {
                    ToolItem::separator()
                } else {
                    ToolItem::text(format!("launch:{i}"), it.label.clone())
                }
            })
            .collect()
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
        self.launcherbar = Toolbar::new(App::build_launcherbar(&self.launcher_items));
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
            ("view.hidden", s.flag("list.show_hidden")),
            ("view.dot", s.flag("list.show_dotfiles")),
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
            ("view.hidden", s.flag("list.show_hidden")),
            ("view.dot", s.flag("list.show_dotfiles")),
            ("view.folders_first", s.flag("list.folders_first")),
        ];
        for (id, on) in tool {
            self.toolbar.set_item_checked(id, on, &mut inv);
        }
    }

    /// 툴바(dir2 `build_toolbar` 블록 순서 panel · view · refresh · settings · show — 글리프 폴백 · SVG 아이콘은 T-30/T-43).
    pub(crate) fn build_toolbar(settings: &Settings, icon_px: u32) -> Vec<ToolItem> {
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
        let _ = settings;
        vec![
            g("view.panel_toggle", "▌▐", "cmd.panelToggle"),
            g("view.dock", "▂", "menu.view.dock"),
            g("view.always_on_top", "📌", "menu.view.alwaysOnTop"),
            g("view.info_toggle", "ⓘ", "cmd.infoToggle"),
            g("view.col_width_sync", "⇔", "menu.view.colWidthSync"),
            ToolItem::separator(),
            g("view.mode_tree", "├─", "menu.view.modeTree"),
            g("view.mode_flat", "☰", "menu.view.modeFlat"),
            g("view.mode_tiles", "▦", "menu.view.modeTiles"),
            ToolItem::separator(),
            g("view.refresh", "⟳", "menu.view.refresh"),
            ToolItem::separator(),
            g("file.prefs", "⚙", "menu.file.prefs"),
            ToolItem::separator(),
            g("view.hidden", "👁", "menu.view.hidden"),
            g("view.dot", "…", "menu.view.dot"),
            g("view.folders_first", "▲", "pref.sortFoldersFirst"),
        ]
    }

    /// 선택 항목을 휴지통으로(Trash 포트 · 확인창은 T-29 뒤 · 실패/미지원 = 토스트 한 번) → 그 폴더를 보는 탭 전부 재열람.
    pub(crate) fn delete_to_trash(&mut self) {
        let paths = self.panels[self.active].selected_paths();
        if paths.is_empty() {
            return;
        }
        let mut inv = Invalidations::default();
        match self.platform.trash.trash(&paths) {
            Ok(n) => {
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
    fn apply_list_opts(&mut self) {
        let opts = list_opts(&self.settings);
        let mut inv = Invalidations::default();
        for p in &mut self.panels {
            p.set_opts(opts, &mut inv);
        }
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
        match id {
            "file.exit" => self.exit_requested = true,
            "file.new_folder" => self.create_new(true),
            "file.new_file" => self.create_new(false),
            "edit.rename" => self.begin_rename(),
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
            "view.hidden" | "view.dot" | "view.folders_first" => {
                let key = match id {
                    "view.hidden" => "list.show_hidden",
                    "view.dot" => "list.show_dotfiles",
                    _ => "list.folders_first",
                };
                self.toggle_flag(key);
                self.sync_menu_checks();
                self.apply_list_opts();
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
            "cmd.contextMenu" => self.open_row_menu(a),
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
}
