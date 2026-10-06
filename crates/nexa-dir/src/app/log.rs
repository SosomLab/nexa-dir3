//! App — 로그(T-92 · docs/22 NEW-001 · DR-15 · nexa-sql `nsql-log`/`log_win` 이식 · 사용자 10-04 "로그 확인 방법"): 로그 창
//! 토글(`view.log` · F10) · 기본 로그 helper(`log` · `log_with`) · 설정 `log.*` 적용 · 개발자 상세 마스크 · 덤프(`log.dump:<파일>`).
//!
//! 상세 로그는 호스트 매크로 `dlog!(self, layer, level, make)`(main.rs) — 게이트가 꺼져 있으면 인자를 평가하지 않는다.

use crate::*;
use ndir_log::{LogEntry, LogKind};

/// 로그 설정 키 전부(기동 때 한 번에 적용 · 설정 창/스위치 변경은 키 하나씩).
pub(crate) const LOG_KEYS: &[&str] = &[
    "log.format",
    "log.template",
    "log.columns",
    "log.kinds",
    "log.wrap",
    "log.newest_first",
    "log.autoscroll",
    "log.always_on_top",
    "log.switch_scale",
    "log.max_lines",
    "log.dev_mode",
];

impl App {
    /// `view.log`(F10) — 열려 있으면 닫고 아니면 연다(모덜리스 · 메모리 창과 같은 규칙).
    pub(crate) fn toggle_log_window(&mut self) {
        if self.log_win.is_open() {
            self.log_win.close();
            self.persist_window_sizes();
        } else {
            self.open_log = true;
        }
        self.sync_log_toggle();
    }

    /// 도구 모음 ≡ 토글 = 로그 창 열림 상태(열기 · 닫기 · X로 닫힘 뒤에 맞춘다).
    pub(crate) fn sync_log_toggle(&mut self) {
        let mut inv = Invalidations::default();
        let on = self.log_win.is_open() || self.open_log;
        self.toolbar.set_item_checked("view.log", on, &mut inv);
        self.redraw();
    }

    /// 기본 로그 한 줄(늘 보인다 · 층 = App).
    pub(crate) fn log(&mut self, kind: LogKind, message: impl Into<String>) {
        self.log_win.push(LogEntry::new(kind, message));
    }

    /// 항목 수 · 소요가 있는 기본 로그.
    pub(crate) fn log_with(
        &mut self,
        kind: LogKind,
        message: impl Into<String>,
        items: Option<u64>,
        elapsed: Option<Duration>,
    ) {
        self.log_win
            .push(LogEntry::new(kind, message).items(items).elapsed(elapsed));
    }

    /// 로그 설정 전부 적용(기동 · 설정 가져오기 뒤).
    pub(crate) fn apply_log_settings(&mut self) {
        for key in LOG_KEYS {
            self.apply_log_setting(key);
        }
        self.apply_window_sizes();
    }

    /// `log.*` 키 하나 적용 — [`App::apply_setting`]에서 온다. 처리했으면 `true`.
    pub(crate) fn apply_log_setting(&mut self, key: &str) -> bool {
        let s = &self.settings;
        match key {
            "log.format" => self.log_win.set_format(s.get(key).unwrap_or("raw")),
            "log.template" => self.log_win.set_template(s.get(key).unwrap_or("")),
            "log.columns" => self.log_win.set_columns(s.get(key).unwrap_or("")),
            "log.kinds" => self.log_win.set_kinds(s.get(key).unwrap_or("")),
            "log.wrap" => self.log_win.set_wrap(s.flag(key)),
            "log.newest_first" => self.log_win.set_newest_first(s.flag(key)),
            "log.autoscroll" => self.log_win.set_autoscroll(s.flag(key)),
            "log.always_on_top" => self.log_win.set_on_top(s.flag(key)),
            "log.switch_scale" => self.log_win.set_switch_scale(s.int(key)),
            "log.max_lines" => self.log_win.set_max_lines(s.int(key).max(1) as usize),
            "log.dev_mode" | "log.dev_layers" => self.apply_detail_mask(),
            "log.open_at_start" => {}
            _ => return false,
        }
        true
    }

    /// 개발자 상세 마스크(설정 `log.dev_mode` × `log.dev_layers`) → `ndir_log` 게이트 + 창의 개발자 스위치/층 체크.
    pub(crate) fn apply_detail_mask(&mut self) {
        let on = self.settings.flag("log.dev_mode");
        let layers =
            ndir_log::parse_detail_layers(self.settings.get("log.dev_layers").unwrap_or(""));
        ndir_log::set_detail_mask(if on { layers } else { 0 });
        self.log_win.set_dev(on);
        self.log_win.set_dev_mask(layers);
    }

    /// 기동 때 쌓인 플러그인 적재 노트(오류 · 건너뜀)를 로그로(EXT-045 — dir2는 버렸다).
    pub(crate) fn log_plugin_notes(&mut self) {
        for n in preview::load_notes() {
            self.log(LogKind::Plugin, n);
        }
    }

    /// 덤프(`log.dump:<파일>` · 시험) — 보이는 줄을 지금 포맷으로.
    pub(crate) fn log_dump(&self) -> String {
        self.log_win.dump_text()
    }
}
