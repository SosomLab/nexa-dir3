//! App — 보조 창(설정 · 단축키) 열기 펌프 · 사건 분배 · 창 기하 기억(nexa-sql `app/windows.rs` + `event_loop.rs::aux_window_event` 축약).
//!
//! 규칙(docs/port/40 SKEL-419): 보조 창 하나 = 필드 + `open_<이름>` 깃발 + `open_requested_windows` 소비 줄 + `aux_window_event` 분배 줄 +
//! `all_aux_windows` + 틱(`aux_tick`) + 기하 기억(`persist_window_sizes`/`apply_window_sizes`).

use crate::check_win::CheckAction;
use crate::dlg_win::DlgAction;
use crate::keys_win::KeysAction;
use crate::prefs_win::PrefsAction;
use crate::*;

impl App {
    /// 메인 창 사각형(보조 창을 가운데 띄우기 위해).
    fn main_rect(&self) -> Option<(i32, i32, u32, u32)> {
        let w = self.window.as_ref()?;
        let p = w.outer_position().ok()?;
        let s = w.outer_size();
        Some((p.x, p.y, s.width, s.height))
    }

    /// 사건 처리 중에 쌓인 "창 열기" 요청을 한 번에(명령 · 기동 명령 · 설정 창의 바로가기 전부 같은 펌프).
    pub(crate) fn open_requested_windows(&mut self, el: &ActiveEventLoop) {
        if std::mem::take(&mut self.open_prefs) {
            self.prefs_win
                .set_advanced(self.settings.flag("ui.prefs_advanced"));
            self.prefs_win
                .set_dyn_choices("ui.lang", self.lang_choices());
            // 플러그인 페이지(T-63 A · dir2 PLUG-125 축약): 로드된 플러그인 목록·로드 오류를 `plugins.disabled` 설명 줄로 —
            // 체크박스 목록(동적 Bool 묶음)은 T-63 B(nexa-ctl Checkbox 페이지).
            self.prefs_win
                .set_note("plugins.disabled", Some(self.plugin_note()));
            self.prefs_win.refresh(&self.settings);
            let over = self.main_rect();
            let theme = theme::window_theme(self.settings.theme_mode());
            let owner = self.window.clone();
            self.prefs_win.open(el, theme, over, owner.as_deref());
        }
        if std::mem::take(&mut self.open_keys) {
            self.keys_win.refresh(&self.keymap);
            let over = self.main_rect();
            let theme = theme::window_theme(self.settings.theme_mode());
            let owner = self.window.clone();
            self.keys_win.open(el, theme, over, owner.as_deref());
        }
        if std::mem::take(&mut self.open_check) {
            let over = self.main_rect();
            let theme = theme::window_theme(self.settings.theme_mode());
            let owner = self.window.clone();
            self.check_win.open(el, theme, over, owner.as_deref());
        }
        if self.window.is_some() && !self.dlg.is_open() {
            if let Some((spec, reply)) = self.dlg_pending.take() {
                self.dlg_reply = Some(reply);
                let over = self.main_rect();
                let theme = theme::window_theme(self.settings.theme_mode());
                let owner = self.window.clone();
                self.dlg.open(el, spec, theme, over, owner.as_deref());
            }
        }
    }

    /// 설정 창 플러그인 설명 줄: `이름 (id) — ext, …` 목록(없으면 `pref.plugins.empty`) + 로드 오류.
    pub(crate) fn plugin_note(&self) -> String {
        let infos = preview::plugin_infos();
        let mut s = if infos.is_empty() {
            tr("pref.plugins.empty")
        } else {
            infos
                .iter()
                .map(|p| format!("{} ({}) — {}", p.name, p.id, p.exts.join(", ")))
                .collect::<Vec<_>>()
                .join(" · ")
        };
        let notes = preview::load_notes();
        if !notes.is_empty() {
            s.push_str(" · ");
            s.push_str(&notes.join(" · "));
        }
        s
    }

    /// 언어 콤보 후보(`system` + 내장/오버레이 발견분).
    pub(crate) fn lang_choices(&self) -> Vec<(String, String)> {
        let home = ndir_settings::config_dir().unwrap_or_else(std::env::temp_dir);
        let mut v = vec![("system".to_string(), tr("pref.lang.system"))];
        v.extend(ndir_i18n::discover(&home));
        v
    }

    /// 보조 창의 사건을 그 창의 처리기로. 처리했으면 `true`(메인 창 처리로 가지 않는다).
    pub(crate) fn aux_window_event(&mut self, id: WindowId, event: &WindowEvent) -> bool {
        if self.prefs_win.is(id) {
            let ui_px = self.settings.font_px("ui.font_size");
            match self.prefs_win.handle(event) {
                PrefsAction::Paint => {
                    let font = Rc::clone(&self.ui_font);
                    self.prefs_win.paint(&font, &self.theme, ui_px);
                }
                PrefsAction::Changed { key, value } => {
                    let ok = if value.is_empty()
                        && ndir_settings::entry(&key).is_some_and(|e| e.default.is_empty())
                    {
                        self.settings
                            .reset(&key)
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    } else {
                        self.settings
                            .set(&key, &value)
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    };
                    match ok {
                        Ok(()) => {
                            let _ = self.settings.save();
                            self.after_setting_changed(&key);
                        }
                        Err(e) => self.prefs_win.set_error(&key, e),
                    }
                    self.prefs_win.redraw();
                }
                PrefsAction::Reset(key) => {
                    let _ = self.settings.reset(&key);
                    let _ = self.settings.save();
                    self.after_setting_changed(&key);
                    self.prefs_win.redraw();
                }
                PrefsAction::OpenKeys => self.open_keys = true,
                PrefsAction::OpenColors(_)
                | PrefsAction::BrowseFolder { .. }
                | PrefsAction::EditJson => {
                    // 색 창(dir2에 없음) · 폴더 고르기(T-29) · JSON 편집(T-44 잔여) — 안내만.
                    self.toasts
                        .push(toast::ToastKind::Info, tr("pref.title"), tr("cmd.notYet"));
                    self.redraw();
                }
                PrefsAction::None => {}
            }
            return true;
        }
        if self.keys_win.is(id) {
            let ui_px = self.settings.font_px("ui.font_size");
            match self.keys_win.handle(event) {
                KeysAction::Paint => {
                    let font = Rc::clone(&self.ui_font);
                    self.keys_win.paint(&font, &self.theme, ui_px);
                }
                KeysAction::Changed { id, code } => {
                    let _ = self.settings.set(&ndir_settings::setting_key(&id), &code);
                    let _ = self.settings.save();
                    self.after_setting_changed("key.preset");
                    self.keys_win.redraw();
                }
                KeysAction::ResetAll => {
                    for c in ndir_settings::COMMANDS {
                        let _ = self.settings.reset(&ndir_settings::setting_key(c.id));
                    }
                    let _ = self.settings.save();
                    self.after_setting_changed("key.preset");
                    self.keys_win.redraw();
                }
                KeysAction::None => {}
            }
            return true;
        }
        if self.dlg.is(id) {
            let ui_px = self.settings.font_px("ui.font_size");
            match self.dlg.handle(event) {
                DlgAction::Paint => {
                    let font = Rc::clone(&self.ui_font);
                    self.dlg.paint(&font, &self.theme, ui_px);
                }
                DlgAction::Done { id, text } => {
                    self.dlg.close();
                    self.dlg_done(id, text);
                    if let Some(w) = &self.window {
                        w.focus_window();
                    }
                }
                DlgAction::None => {}
            }
            return true;
        }
        if self.check_win.is(id) {
            let ui_px = self.settings.font_px("ui.font_size");
            match self.check_win.handle(event) {
                CheckAction::Paint => {
                    let font = Rc::clone(&self.ui_font);
                    self.check_win.paint(&font, &self.theme, ui_px);
                }
                CheckAction::Copy(text) => {
                    let _ = clipboard::write_text(&text);
                }
                CheckAction::None => {}
            }
            return true;
        }
        false
    }

    /// 설정이 바뀐 뒤 — 적용(`apply_setting`) · 열린 창 갱신 · 적용 불가면 상태줄 안내.
    pub(crate) fn after_setting_changed(&mut self, key: &str) {
        if !self.apply_setting(key) {
            let mut inv = Invalidations::default();
            self.statusbar
                .set_left(&tr("status.needsRestart"), &mut inv);
        }
        if self.prefs_win.is_open() {
            self.prefs_win.refresh(&self.settings);
            self.prefs_win.redraw();
        }
        if self.keys_win.is_open() {
            self.keys_win.refresh(&self.keymap);
            self.keys_win.redraw();
        }
        if self.check_win.is_open() {
            self.check_win.redraw(); // 테마·글꼴 변경 반영
        }
        self.redraw();
    }

    /// 보조 창 틱 — 애니메이션 중이면 그 창을 다시 그린다. 더 돌아야 하면 `true`.
    pub(crate) fn aux_tick(&mut self, now_ms: u64) -> bool {
        if self.prefs_win.tick(now_ms) {
            self.prefs_win.redraw();
        }
        if self.keys_win.tick(now_ms) {
            self.keys_win.redraw();
        }
        if self.check_win.tick(now_ms) {
            self.check_win.redraw();
        }
        if self.dlg.tick(now_ms) {
            self.dlg.redraw();
        }
        self.persist_window_sizes();
        self.prefs_win.animating()
            || self.keys_win.animating()
            || self.check_win.animating()
            || self.dlg.animating()
    }

    /// 닫힌 보조 창의 마지막 (위치, 크기)를 설정에(`window.prefs_pos`/`_size` · dir2 계승).
    pub(crate) fn persist_window_sizes(&mut self) {
        let Some(((x, y), (w, h))) = self.prefs_win.take_last() else {
            return;
        };
        let mut changed = false;
        for (key, v) in [
            ("window.prefs_pos", wingeom::format_pos(x, y)),
            ("window.prefs_size", wingeom::format_size(w, h)),
        ] {
            if self.settings.get(key) != Some(v.as_str()) {
                let _ = self.settings.set(key, &v);
                changed = true;
            }
        }
        if changed {
            let _ = self.settings.save();
            self.apply_window_sizes();
        }
    }

    /// 설정의 기억된 기하를 보조 창의 메모로(열 때 같은 모니터면 그대로).
    pub(crate) fn apply_window_sizes(&mut self) {
        let memo = wingeom::Memo {
            pos: self
                .settings
                .get("window.prefs_pos")
                .and_then(wingeom::parse_pos),
            size: self
                .settings
                .get("window.prefs_size")
                .and_then(wingeom::parse_size),
        };
        self.prefs_win.set_memo(memo);
    }
}
