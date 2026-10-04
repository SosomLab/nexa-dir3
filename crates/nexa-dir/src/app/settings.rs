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
            // 보호 항목 = 전역(전 탭 즉시) · 숨김 · Dot · 폴더 우선 · 대소문자 구분 = **새 탭의 기본값**(열린 탭은 자기 값 유지 ·
            // 사용자 10-03) — 나누는 일은 `Panel::set_opts`가 한다.
            "list.show_hidden"
            | "list.show_dotfiles"
            | "list.show_protected"
            | "list.folders_first"
            | "list.sort_case_sensitive" => {
                let opts = list_opts(&self.settings);
                for p in &mut self.panels {
                    p.set_opts(opts, &mut inv);
                }
                self.sync_menu_checks();
            }
            "list.view_scope" => self.rebuild_toolbar(),
            "tabs.multiline" | "tabs.scroll_buttons" | "tabs.dblclick" => self.apply_tab_style(),
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
            "perf.boost" | "statusbar.layout" | "statusbar.load_interval_ms" => {
                self.load_next = Instant::now();
            }
            "layout.tab_statusbar" => self.layout(),
            "launcher.items" | "launcher.seed" | "launcher.icon_size" | "launcher.item_gap" => {
                self.rebuild_launcher();
            }
            // 순서 편집기(T-71 DLG-073): 툴바 재구성 · 컬럼 = 활성 패널(+동기) · 컨텍스트 메뉴는 다음 열 때 읽는다.
            "toolbar.icon_size"
            | "toolbar.icon_pad"
            | "toolbar.hover_fill_pct"
            | "toolbar.icon_scale_pct"
            | "toolbar.on_color"
            | "toolbar.on_line_color"
            | "toolbar.on_fill_pct"
            | "toolbar.on_line_pct"
            | "toolbar.state_step_pct"
            | "toolbar.state_radius"
            | "toolbar.on_icon_accent"
            | "toolbar.item_gap"
            | "toolbar.group_gap"
            | "toolbar.row_gap"
            | "toolbar.dock_layout" => {
                // 즉시 반영: 그 크기로 아이콘을 다시 만들고(선명) 높이가 달라지므로 창 배치도 다시.
                self.rebuild_toolbar();
                self.sync_menu_checks();
                self.layout();
            }
            "toolbar.layout" => {
                self.rebuild_toolbar();
                self.sync_menu_checks();
                self.layout();
            }
            "list.folder_bold" | "list.header_bold" | "list.header_italic" => {
                self.apply_font_decor();
            }
            // 터미널 글꼴(따르기 · 글꼴 목록 · 대체 글꼴) = 즉시: 고정폭 글꼴 체인을 다시 만들어 다음 그리기부터 쓴다
            // (종전 = 글꼴 얼굴은 다시 시작해야 반영 — 사용자 10-05 "바꿨는데 반영이 안 된다").
            "term.follow_windows_terminal" | "term.font_face" | "term.fallback_fonts" => {
                self.wt_profile = App::load_wt_profile(&self.settings);
                self.mono_font = App::load_mono_font(&self.settings, self.wt_profile.as_ref());
                self.layout();
            }
            "list.icon_overrides" => self.apply_icon_overrides(),
            // 메뉴 키보드 순환 이동(wrap-around) — 우클릭 · 탭 · 상태줄 메뉴 공통(같은 메뉴 컨트롤).
            "menu.wrap_around" => self
                .tab_menu
                .set_wrap_around(self.settings.flag("menu.wrap_around")),
            "menu.char_jump" => self
                .tab_menu
                .set_char_jump(self.settings.flag("menu.char_jump")),
            k if k.starts_with("scroll.") => self.apply_scroll_settings(),
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
                // 설정 창에서 켜도 툴바·메뉴로 켤 때와 같이 즉시 맞춘다(기준 = 활성 패널 · 종전 = 체크만 바뀌었다).
                if self.dual && self.settings.flag("list.col_width_sync") {
                    self.sync_col_widths_from(self.active);
                }
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
    #[cfg(test)]
    pub(crate) fn toolbar_icon_px(scale: f32) -> u32 {
        (TOOLBAR_ICON_LOGICAL as f32 * scale).round().max(8.0) as u32
    }

    /// 툴바 재구성(언어 · 배율 변경 — 체크 상태는 `sync_menu_checks`가 다시 맞춘다).
    pub(crate) fn rebuild_toolbar(&mut self) {
        self.toolbar = App::make_tool_dock(&self.settings, self.scale);
    }

    /// 스크롤 설정 적용(dir2 X-63 f986415 · 7d8b1e9 — 호스트 누락분): `scroll.fast*` → nexa-grid(목록·도크·그리드 창) +
    /// nexa-ctl(설정 창 등 `ScrollBars`) 전역 고속 스크롤 · 파일 그리드 한 단계 더 빠르게 · 시스템 "한 번에 스크롤할 줄 수".
    pub(crate) fn apply_scroll_settings(&self) {
        let (grid, grid_extra, ctl) = Self::scroll_configs(&self.settings);
        nexa_grid::fastscroll::set_fast_scroll(grid);
        nexa_grid::fastscroll::set_fast_scroll_grid(grid_extra);
        // 키보드 이동(↑/↓)에도 적용할지 — 고속 스크롤이 꺼져 있으면 어차피 배수 1이라 이 값은 영향이 없다.
        nexa_grid::fastscroll::set_fast_scroll_keys(self.settings.flag("scroll.fast_keys"));
        nexa_ctl::set_fast_scroll(ctl);
        if let Some(n) = platform::wheel_lines() {
            nexa_ctl::set_wheel_lines(n);
        }
    }

    /// 설정 `scroll.*` → 고속 스크롤 구성(순수): (nexa-grid 공통 · 파일 그리드 전용(한 단계 더 빠르게 — 꺼져 있으면 `None`) · nexa-ctl).
    /// 전역에 쓰는 일은 [`Self::apply_scroll_settings`]만 한다 — 시험은 이 함수로 값을 확인한다(전역은 프로세스 공유라 병렬 시험이
    /// 서로 덮어쓴다 · 10-03 흔들림).
    pub(crate) fn scroll_configs(
        s: &Settings,
    ) -> (
        nexa_grid::fastscroll::FastScroll,
        Option<nexa_grid::fastscroll::FastScroll>,
        nexa_ctl::FastScroll,
    ) {
        let int = |k: &str, lo: i64, hi: i64| s.int(k).clamp(lo, hi);
        let pos = s.position_index("scroll.fast_hud_pos").min(8);
        let grid = nexa_grid::fastscroll::FastScroll {
            enabled: s.flag("scroll.fast"),
            step: int("scroll.fast_step", 1, 50) as u32,
            max: int("scroll.fast_max", 1, 32) as i32,
            window_ms: int("scroll.fast_window_ms", 20, 2000) as u64,
            hud: s.flag("scroll.fast_hud"),
            hud_pos: pos as u8,
            hud_hold_ms: int("scroll.fast_hud_hold_ms", 0, 10_000) as u64,
            hud_fade_ms: int("scroll.fast_hud_fade_ms", 0, 10_000) as u64,
        };
        let grid_extra = s
            .flag("scroll.fast_grid_extra")
            .then(|| nexa_grid::fastscroll::grid_extra_of(&grid));
        let ctl = nexa_ctl::FastScroll {
            enabled: grid.enabled,
            step: grid.step,
            max: grid.max,
            window_ms: grid.window_ms,
            hud: grid.hud,
            hud_pos: nexa_ctl::HudPos::parse(s.get("scroll.fast_hud_pos").unwrap_or("top_right")),
            hud_hold_ms: grid.hud_hold_ms,
            hud_fade_ms: grid.hud_fade_ms,
        };
        (grid, grid_extra, ctl)
    }

    /// 설정 `toolbar.icon_size`(16/20/24/32 · 그 밖 = 20).
    pub(crate) fn toolbar_icon_logical(settings: &Settings) -> i32 {
        match settings.get("toolbar.icon_size").unwrap_or("20") {
            "16" => 16,
            "24" => 24,
            "32" => 32,
            _ => TOOLBAR_ICON_LOGICAL,
        }
    }

    fn setting_px(settings: &Settings, key: &str, default: i32, max: i32) -> i32 {
        settings
            .get(key)
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(default)
            .clamp(0, max)
    }

    /// 툴바 그룹 도크 생성 — 그룹 = `toolbar.layout`의 블록(숨긴 블록·빈 블록 제외) · 아이콘 크기/간격 = 설정 · 배치 = `toolbar.dock_layout`.
    /// 아이콘 칸 여백은 0(아이콘 사이 간격은 `toolbar.item_gap` 하나로 정한다 · 기본 0 = 붙임).
    pub(crate) fn make_tool_dock(settings: &Settings, scale: f32) -> ToolDock {
        let logical = App::toolbar_icon_logical(settings);
        let icon_px = (logical as f32 * scale).round().max(8.0) as u32;
        // 그림만 줄인다(사용자 10-04 "버튼 크기는 그대로 · 이미지만 90 %"): 마스크를 줄인 크기로 만들어 확대·축소 없이 또렷하게.
        let icon_scale =
            App::setting_px(settings, "toolbar.icon_scale_pct", 90, 100).max(50) as f32 / 100.0;
        let draw_px = Toolbar::icon_draw_px(icon_px as i32, icon_scale).max(8) as u32;
        let mut dock = ToolDock::new(App::build_tool_groups(settings, draw_px));
        dock.set_icon_scale(icon_scale);
        dock.set_icon_size(logical);
        // 아이콘 둘레 여백(`toolbar.icon_pad` · 기본 1 = 상하좌우 1px → 칸 22 · 아이콘 사이 2 · 툴바 높이 30) + 양끝/위아래 4.
        dock.set_padding(App::setting_px(settings, "toolbar.icon_pad", 1, 8), 4);
        dock.set_item_gap(App::setting_px(settings, "toolbar.item_gap", 0, 16));
        // 상태 표시(사용자 10-03 디자인 개편): 옅은 채움 + 얇은 테두리 + 켜진 아이콘은 강조색 — 농도는 고급 설정(%).
        let pct = |k: &str, d: i32| App::setting_px(settings, k, d, 100) as f32 / 100.0;
        // 켜짐 표시(사용자 10-04): line = 배경은 종전(강조색 옅은 채움) 그대로 · **선만 `toolbar.on_line_color`**(기본 #0000FF —
        // 초록은 "너무 이상하다"로 정정 · 테두리 또렷하게 +
        // 아이콘 선) — 처음엔 초록으로 꽉 채웠다가 "배경은 원복하고 선만 초록"으로 정정 · accent = 종전(강조색 테두리 · 농도 설정).
        let green = settings.get("toolbar.on_color").unwrap_or("accent") == "line";
        let line_color = nexa_ctl::theme::color_from_hex(
            settings.get("toolbar.on_line_color").unwrap_or("").trim(),
        )
        .unwrap_or(nexa_ctl::theme::Color(0x0000_00FF));
        dock.set_soft_states(Some(nexa_ctl::controls::SoftStates {
            hover_fill: pct("toolbar.hover_fill_pct", 8),
            on_fill: pct("toolbar.on_fill_pct", 26),
            on_line: if green {
                1.0
            } else {
                pct("toolbar.on_line_pct", 12)
            },
            step: pct("toolbar.state_step_pct", 20),
            radius: App::setting_px(settings, "toolbar.state_radius", 4, 12),
            on_icon_accent: !green && settings.flag("toolbar.on_icon_accent"),
            // hover(사용자 10-04 · nexa-sql 툴바와 같게): 아이콘을 강조색(파란 계통)으로 · 토글 버튼은 옅은 회색 배경도.
            hover_icon_accent: true,
            hover_fill_toggle_only: true,
            on_color: None,
            on_icon: green.then_some(line_color),
            on_line_color: green.then_some(line_color),
        }));
        dock.set_gaps(
            App::setting_px(settings, "toolbar.group_gap", 4, 32),
            App::setting_px(settings, "toolbar.row_gap", 0, 16),
        );
        dock.apply_layout(&DockLayout::parse(
            settings.get("toolbar.dock_layout").unwrap_or(""),
        ));
        let _ = dock.take_actions();
        dock
    }

    /// 도크가 알린 일 처리: 배치 변경 = 설정 저장 · 행 수 변경 = 창 재배치 · 떼어 내기 = 도로 붙임(플로팅 창은 후속).
    pub(crate) fn toolbar_actions(&mut self) {
        for a in self.toolbar.take_actions() {
            match a {
                DockAction::Float { id, .. } => {
                    self.toolbar.dock(&id);
                    self.layout();
                }
                DockAction::LayoutChanged => {
                    let v = self.toolbar.layout().serialize();
                    if self.settings.get("toolbar.dock_layout").unwrap_or("") != v {
                        let _ = self.settings.set("toolbar.dock_layout", &v);
                        let _ = self.settings.save();
                    }
                }
                DockAction::Resized => self.layout(),
            }
        }
    }

    /// 언어 전환 뒤 라벨 재구성(메뉴 · 툴바 · 열 · 상태줄).
    pub(crate) fn relabel(&mut self) {
        self.menubar.set_menus(App::build_menus(&self.settings));
        self.rebuild_toolbar();
        self.sync_menu_shortcuts();
        self.sync_menu_checks();
        // 한 번만 만들던 글(T-134 · 언어 전환 때 안 바뀌던 곳): 입력칸 우클릭 메뉴 · 도크 종류 칸 · 패널 네비 툴팁/내 PC 제목 ·
        // 보조 창의 버튼/제목.
        crate::install_ctl_labels();
        let mut inv = Invalidations::default();
        for d in &mut self.docks {
            d.set_kinds(
                vec![tr("dock.info"), tr("dock.preview"), tr("dock.terminal")],
                &mut inv,
            );
        }
        for p in &mut self.panels {
            p.relabel(&mut inv);
        }
        self.layout();
        self.prefs_win.relabel();
        self.keys_win.relabel();
        self.check_win.relabel();
        self.license_win.relabel();
        self.order_win.relabel();
        self.bulk_win.relabel();
        self.mem_win.relabel();
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
