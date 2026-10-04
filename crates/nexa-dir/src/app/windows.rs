//! App — 보조 창(설정 · 단축키) 열기 펌프 · 사건 분배 · 창 기하 기억(nexa-sql `app/windows.rs` + `event_loop.rs::aux_window_event` 축약).
//!
//! 규칙(docs/port/40 SKEL-419): 보조 창 하나 = 필드 + `open_<이름>` 깃발 + `open_requested_windows` 소비 줄 + `aux_window_event` 분배 줄 +
//! `all_aux_windows` + 틱(`aux_tick`) + 기하 기억(`persist_window_sizes`/`apply_window_sizes`).

use crate::archive_win::ArcAction;
use crate::bulk_win::BulkAction;
use crate::check_win::CheckAction;
use crate::dlg_win::DlgAction;
use crate::file_win::FileWinAction;
use crate::keys_win::KeysAction;
use crate::license_win::LicAction;
use crate::order_win::OrderAction;
use crate::prefs_win::PrefsAction;
use crate::preview_win::PvAction;
use crate::progress_win::ProgAction;
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
            // 플러그인 페이지(T-63 B · dir2 EXT-129): 로드된 플러그인당 체크박스 + 로드 오류 줄(해제 = `plugins.disabled`).
            let rows = self.plugin_rows();
            self.prefs_win.set_plugins(rows, preview::load_notes());
            self.refresh_prefs();
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
        if std::mem::take(&mut self.open_preview) && self.window.is_some() {
            let over = self.main_rect();
            let theme = theme::window_theme(self.settings.theme_mode());
            let owner = self.window.clone();
            self.preview_win.open(el, theme, over, owner.as_deref());
        }
        if std::mem::take(&mut self.open_bulk) && self.window.is_some() {
            let over = self.main_rect();
            let theme = theme::window_theme(self.settings.theme_mode());
            let owner = self.window.clone();
            self.bulk_win.open(el, theme, over, owner.as_deref());
        }
        if std::mem::take(&mut self.open_order) && self.window.is_some() {
            let over = self.main_rect();
            let theme = theme::window_theme(self.settings.theme_mode());
            let owner = self.window.clone();
            self.order_win.open(el, theme, over, owner.as_deref());
        }
        if std::mem::take(&mut self.open_progress) && self.window.is_some() {
            let over = self.main_rect();
            let theme = theme::window_theme(self.settings.theme_mode());
            let owner = self.window.clone();
            self.progress_win.open(el, theme, over, owner.as_deref());
        }
        if std::mem::take(&mut self.open_archive) && self.window.is_some() {
            let over = self.main_rect();
            let theme = theme::window_theme(self.settings.theme_mode());
            let owner = self.window.clone();
            self.archive_win.open(el, theme, over, owner.as_deref());
        }
        if std::mem::take(&mut self.open_memory) && self.window.is_some() {
            let over = self.main_rect();
            let theme = theme::window_theme(self.settings.theme_mode());
            let owner = self.window.clone();
            self.mem_win.open(el, theme, over, owner.as_deref());
            self.mem_next = Instant::now(); // 첫 표본은 다음 유휴 틱에
        }
        if std::mem::take(&mut self.open_license) && self.window.is_some() {
            self.licensing.refresh();
            let over = self.main_rect();
            let theme = theme::window_theme(self.settings.theme_mode());
            let owner = self.window.clone();
            self.license_win.open(el, theme, over, owner.as_deref());
        }
        if std::mem::take(&mut self.open_file) && self.window.is_some() {
            let (mode, start, filters) = self.file_window_spec();
            let over = self.main_rect();
            let theme = theme::window_theme(self.settings.theme_mode());
            // 라이선스 창이 열려 있으면 그 위에(닫히면 그 창으로 포커스가 돌아온다).
            let owner = self.license_win.window_rc().or_else(|| self.window.clone());
            self.file_win.open(
                el,
                theme,
                over,
                owner.as_deref(),
                mode,
                start.as_deref(),
                filters,
            );
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
            let ui_px = self.font_px("ui.font_size");
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
                PrefsAction::EditOrder(key) => self.open_order_editor(&key),
                PrefsAction::InstallPlugin => {
                    self.open_file_window(app::license::FilePurpose::Plugin);
                }
                PrefsAction::RemovePlugin(id) => self.plugin_remove(&id),
                PrefsAction::BrowseFolder { key, .. } => {
                    self.open_file_window(app::license::FilePurpose::Setting(key));
                }
                PrefsAction::OpenColors(_) | PrefsAction::EditJson => {
                    // 색 창(dir2에 없음) · JSON 편집(T-44 잔여) — 안내만.
                    self.toasts
                        .push(toast::ToastKind::Info, tr("pref.title"), tr("cmd.notYet"));
                    self.redraw();
                }
                PrefsAction::None => {}
            }
            return true;
        }
        if self.keys_win.is(id) {
            let ui_px = self.font_px("ui.font_size");
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
        if self.preview_win.is(id) {
            let ui_px = self.font_px("ui.font_size");
            match self.preview_win.handle(event) {
                PvAction::Paint => {
                    let font = Rc::clone(&self.ui_font);
                    let mono = self.mono_font.clone();
                    self.preview_win
                        .paint(&font, mono.as_deref(), &self.theme, ui_px);
                }
                PvAction::Copy(text) => {
                    let _ = clipboard::write_text(&text);
                }
                PvAction::None => {}
            }
            return true;
        }
        if self.dlg.is(id) {
            let ui_px = self.font_px("ui.font_size");
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
        if self.bulk_win.is(id) {
            let ui_px = self.font_px("ui.font_size");
            match self.bulk_win.handle(event) {
                BulkAction::Paint => {
                    let font = Rc::clone(&self.ui_font);
                    self.bulk_win.paint(&font, &self.theme, ui_px);
                }
                BulkAction::None => {}
                other => self.bulk_action(other),
            }
            return true;
        }
        if self.order_win.is(id) {
            let ui_px = self.font_px("ui.font_size");
            match self.order_win.handle(event) {
                OrderAction::Paint => {
                    let font = Rc::clone(&self.ui_font);
                    self.order_win.paint(&font, &self.theme, ui_px);
                }
                OrderAction::Changed { key, value } => {
                    self.order_changed(&key, &value);
                    self.order_win.redraw();
                }
                OrderAction::None => {}
            }
            return true;
        }
        if self.progress_win.is(id) {
            let ui_px = self.font_px("ui.font_size");
            if self.progress_win.handle(event) == ProgAction::Paint {
                let font = Rc::clone(&self.ui_font);
                self.progress_win.paint(&font, &self.theme, ui_px);
            }
            return true;
        }
        if self.archive_win.is(id) {
            let ui_px = self.font_px("ui.font_size");
            match self.archive_win.handle(event) {
                ArcAction::Paint => {
                    let font = Rc::clone(&self.ui_font);
                    self.archive_win.paint(&font, &self.theme, ui_px);
                }
                ArcAction::Copy(text) => {
                    let _ = clipboard::write_text(&text);
                }
                ArcAction::None => {}
            }
            return true;
        }
        if self.mem_win.is(id) {
            let ui_px = self.font_px("ui.font_size");
            match self.mem_win.handle(event) {
                crate::mem_win::MemAction::Paint => {
                    let font = Rc::clone(&self.ui_font);
                    self.mem_win.paint(&font, &self.theme, ui_px);
                }
                crate::mem_win::MemAction::Trim => self.mem_trim(),
                crate::mem_win::MemAction::Close => {
                    self.mem_win.close();
                    if let Some(w) = &self.window {
                        w.focus_window();
                    }
                }
                crate::mem_win::MemAction::None => {}
            }
            return true;
        }
        if self.license_win.is(id) {
            let ui_px = self.font_px("ui.font_size");
            match self.license_win.handle(event) {
                LicAction::Paint => {
                    let font = Rc::clone(&self.ui_font);
                    let view = self.license_view();
                    self.license_win.paint(view, &font, &self.theme, ui_px);
                }
                LicAction::Close => {
                    self.license_win.close();
                    if let Some(w) = &self.window {
                        w.focus_window();
                    }
                }
                LicAction::OpenFile => {
                    self.open_file_window(app::license::FilePurpose::License);
                }
                LicAction::Remove => self.license_remove(),
                LicAction::CopyRequest(name, email) => {
                    self.license_copy_request(&name, &email);
                }
                LicAction::CopyText(text) => {
                    let ok = clipboard::write_text(&text);
                    let msg = if ok {
                        tr("license.note.emailCopied")
                    } else {
                        tr("license.note.copyFailed")
                    };
                    self.license_win.set_flash(msg, !ok);
                }
                LicAction::None => {}
            }
            return true;
        }
        if self.file_win.is(id) {
            let ui_px = self.font_px("ui.font_size");
            match self.file_win.handle(event) {
                FileWinAction::Paint => {
                    let font = Rc::clone(&self.ui_font);
                    self.file_win.paint(&font, &self.theme, ui_px);
                }
                FileWinAction::Confirm(path) => {
                    self.file_confirmed(path);
                    self.license_win.redraw();
                }
                FileWinAction::Cancel => {
                    self.file_purpose = None;
                    self.license_win.redraw();
                }
                FileWinAction::CopyText(text) => {
                    let _ = clipboard::write_text(&text);
                }
                FileWinAction::None => {}
            }
            return true;
        }
        if self.check_win.is(id) {
            let ui_px = self.font_px("ui.font_size");
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
            self.refresh_prefs();
            self.prefs_win.redraw();
        }
        if self.keys_win.is_open() {
            self.keys_win.refresh(&self.keymap);
            self.keys_win.redraw();
        }
        if self.check_win.is_open() {
            self.check_win.redraw(); // 테마·글꼴 변경 반영
        }
        if self.preview_win.is_open() {
            self.preview_win.redraw();
        }
        if self.license_win.is_open() {
            self.license_win.redraw();
        }
        if self.mem_win.is_open() {
            self.mem_win.redraw();
        }
        if self.archive_win.is_open() {
            self.archive_win.redraw();
        }
        if self.bulk_win.is_open() {
            self.bulk_win.redraw();
        }
        // 편집 창이 다루는 키가 밖에서(설정 창 텍스트 · JSON) 바뀌면 모델을 다시 읽는다 — 창 자신의 통지는 값이 같아 무해.
        if self.order_win.is_open() && self.order_win.setting_key() == Some(key) {
            let value = self.order_value_of(key);
            if self.order_win.value() != value {
                if let Some(spec) = Self::order_spec_for(key) {
                    self.order_win.set(spec, &value);
                }
            }
            self.order_win.redraw();
        }
        if self.file_win.is_open() {
            self.file_win.redraw();
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
        // 밖에서(다른 설치 경로) 라이선스 파일이 바뀌면 재판정 → 창 다시 그림(LIC-109).
        if self.licensing.refresh() {
            self.license_win.redraw();
        }
        if self.license_win.tick(now_ms) {
            self.license_win.redraw();
        }
        if self.mem_win.tick(now_ms) {
            self.mem_win.redraw();
        }
        if self.archive_win.tick(now_ms) {
            self.archive_win.redraw();
        }
        if self.preview_win.tick(now_ms) {
            self.preview_win.redraw();
        }
        if self.progress_win.tick(now_ms) {
            self.progress_win.redraw();
        }
        if self.bulk_win.tick(now_ms) {
            self.bulk_win.redraw();
        }
        if self.order_win.tick(now_ms) {
            self.order_win.redraw();
        }
        if self.file_win.tick(now_ms) {
            self.file_win.redraw();
        }
        self.persist_window_sizes();
        self.prefs_win.animating()
            || self.keys_win.animating()
            || self.check_win.animating()
            || self.dlg.animating()
            || self.license_win.animating()
            || self.mem_win.animating()
            || self.archive_win.animating()
            || self.preview_win.animating()
            || self.progress_win.animating()
            || self.bulk_win.animating()
            || self.order_win.animating()
            || self.file_win.animating()
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
