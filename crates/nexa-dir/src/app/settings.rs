//! App — 설정 적용(`apply_setting` · nexa-sql `app/settings.rs` 축약 · docs/port/41 SET-060~097). 설정 창·단축키 창·명령·JSON이
//! 값을 바꾼 뒤 **한 길**로 반영한다. 적용 못 하는 키(글꼴 얼굴 등)는 `false` = 상태줄 "다시 시작 후 적용".
//!
//! ★ 적용 누락 감시(T-44): [`apply_setting`]이 `false`를 돌려주는 키의 목록 = [`NEEDS_RESTART`]와 같아야 한다(시험) — 레지스트리에 키가
//! 늘었는데 적용 가지가 없으면 시험이 적발한다.

use crate::*;

/// 바꾸면 다시 시작해야 반영되는 키(의도된 목록 — 그 밖의 `false`는 누락).
pub(crate) const NEEDS_RESTART: &[&str] = &[
    "ui.font_face",
    "ui.menu_font_face",
    "list.font_face",
    "statusbar.font_face",
    "term.font_face",
    "ui.dialog_font_face",
    "ui.text_gdi",
    "ui.text_hint",
    "ui.text_snap",
    "ui.text_weight",
    "ui.text_contrast",
];

impl App {
    /// 바뀐 설정 하나를 지금 상태에 반영. 반영했으면 `true`.
    pub(crate) fn apply_setting(&mut self, key: &str) -> bool {
        let mut inv = Invalidations::default();
        match key {
            "ui.theme" => {
                let mode = self.settings.theme_mode();
                let wt = self.window.as_ref().and_then(|w| w.theme());
                self.theme = theme::resolve(mode, wt);
                if let Some(w) = &self.window {
                    w.set_theme(theme::window_theme(mode));
                }
                self.sync_menu_checks();
            }
            "ui.lang" => {
                init_i18n(&self.settings);
                self.relabel();
            }
            "ui.font_size"
            | "ui.menu_font_size"
            | "list.font_size"
            | "statusbar.font_size"
            | "layout.panel_split_pct"
            | "term.font_size"
            | "ui.dialog_font_size" => {
                self.layout();
            }
            "layout.panel_mode" => {
                self.dual = self.settings.get("layout.panel_mode").unwrap_or("dual") == "dual";
                self.set_active(if self.dual { self.active } else { 0 });
                self.sync_menu_checks();
                self.layout();
            }
            "list.show_hidden"
            | "list.show_dotfiles"
            | "list.folders_first"
            | "list.sort_case_sensitive" => {
                let opts = list_opts(&self.settings);
                for p in &mut self.panels {
                    p.set_opts(opts, &mut inv);
                }
                self.sync_menu_checks();
            }
            "list.view_mode" => {
                let mode = view_mode_of(self.settings.get("list.view_mode").unwrap_or("tree"));
                for p in &mut self.panels {
                    p.set_view_mode(mode, &mut inv);
                }
                self.sync_menu_checks();
            }
            "list.nav_up_align" => {
                let a = nav_up_align(&self.settings);
                for p in &mut self.panels {
                    p.set_nav_up_align(a);
                }
            }
            "window.always_on_top" => {
                if let Some(w) = &self.window {
                    w.set_window_level(if self.settings.flag("window.always_on_top") {
                        winit::window::WindowLevel::AlwaysOnTop
                    } else {
                        winit::window::WindowLevel::Normal
                    });
                }
                self.sync_menu_checks();
            }
            "input.scroll_natural" => {
                input::set_natural_scroll(self.settings.flag("input.scroll_natural"));
            }
            "dock.visible"
            | "layout.info_mode"
            | "layout.dock_height_pct"
            | "layout.dock_split_pct" => {
                self.sync_menu_checks();
                self.layout();
            }
            "launcher.visible" => {
                self.sync_menu_checks();
                self.layout();
            }
            "launcher.items" | "launcher.seed" => self.rebuild_launcher(),
            // 순서 편집기(T-71 DLG-073): 툴바 재구성 · 컬럼 = 활성 패널(+동기) · 컨텍스트 메뉴는 다음 열 때 읽는다.
            "toolbar.layout" => {
                self.rebuild_toolbar();
                self.sync_menu_checks();
                self.layout();
            }
            "list.col_layout" => {
                let v = self
                    .settings
                    .get("list.col_layout")
                    .unwrap_or("")
                    .to_string();
                self.apply_col_layout_str(&v);
            }
            "list.col_width_sync" => {
                self.sync_menu_checks();
            }
            "term.wrap" | "term.cols" | "term.theme" | "term.theme_dark" | "term.theme_light"
            | "term.copy_format" | "term.shell" => {
                self.redraw(); // 다음 paint가 설정을 읽는다(열·팔레트 · 셸은 다음 시작부터).
            }
            k if k.starts_with("key.") => {
                self.keymap = Keymap::from_settings(&self.settings);
                self.sync_menu_shortcuts();
            }
            k if NEEDS_RESTART.contains(&k) => return false,
            // 값은 저장됐고 쓰는 쪽이 읽을 때 반영되는 키(창 기하 · 타입어헤드 · 고속 스크롤 · 전송 · 탭 · 도크 비율 · 터미널 · 플러그인 · 미리보기 ·
            // 런처 · 클라우드 · 라이선스 · 컨텍스트 메뉴 · 고급 스위치 상태 · 더블클릭 간격 · 열 자동 맞춤).
            _ => {}
        }
        self.update_status();
        self.redraw();
        true
    }

    /// 툴바 아이콘 논리 크기 → 물리 px(배율 반영 · SVG 마스크를 그 크기로 렌더해 선명).
    pub(crate) fn toolbar_icon_px(scale: f32) -> u32 {
        (TOOLBAR_ICON_LOGICAL as f32 * scale).round().max(8.0) as u32
    }

    /// 툴바 재구성(언어 · 배율 변경 — 체크 상태는 `sync_menu_checks`가 다시 맞춘다).
    pub(crate) fn rebuild_toolbar(&mut self) {
        let mut toolbar = Toolbar::new(App::build_toolbar(
            &self.settings,
            App::toolbar_icon_px(self.scale),
        ));
        toolbar.set_icon_size(TOOLBAR_ICON_LOGICAL);
        toolbar.set_padding(2, 2);
        self.toolbar = toolbar;
    }

    /// 언어 전환 뒤 라벨 재구성(메뉴 · 툴바 · 열 · 상태줄).
    pub(crate) fn relabel(&mut self) {
        self.menubar.set_menus(App::build_menus(&self.settings));
        self.rebuild_toolbar();
        self.sync_menu_shortcuts();
        self.sync_menu_checks();
        self.layout();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 적용 누락 감시: 레지스트리의 모든 키에 대해 `apply_setting`이 `true`이거나 `NEEDS_RESTART`에 있어야 한다.
    #[test]
    fn every_registry_key_is_applied_or_declared_restart() {
        let dir = std::env::temp_dir().join(format!("ndir-apply-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        ndir_i18n::activate(ndir_i18n::load("en", &dir.join("nowhere")));
        // `ui.lang=en` 고정 — `apply_setting("ui.lang")`이 i18n 전역 표를 바꾸므로 병렬 시험이 OS 언어로 새지 않게(10-03 경합 적발).
        let settings = Settings::from_text(dir.join("settings.conf"), "ui.lang=en");
        let font = nexa_font::ui_font(None).expect("font");
        let mut app = App::new(
            settings,
            font.font,
            Some(dir.clone()),
            None,
            Platform::fake(),
        );
        app.layout_for(1200, 800, 1.0);
        let mut missing = Vec::new();
        for e in ndir_settings::REGISTRY {
            let ok = app.apply_setting(e.key);
            if ok == NEEDS_RESTART.contains(&e.key) {
                missing.push(e.key);
            }
        }
        assert!(missing.is_empty(), "apply/restart mismatch: {missing:?}");
        // 몇 가지는 실제로 상태를 바꾼다.
        let _ = app.settings.set("layout.panel_mode", "single");
        assert!(app.apply_setting("layout.panel_mode") && !app.dual);
        let _ = app.settings.set("ui.theme", "light");
        assert!(app.apply_setting("ui.theme") && !app.theme.is_dark);
        let _ = app.settings.set("key.view.refresh", "f9");
        assert!(app.apply_setting("key.view.refresh"));
        assert_eq!(app.keymap.code_of("view.refresh"), "f9");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
