//! App — 일괄 이름 변경(T-71 · dir2 DLG-074~088 호출부 `win.rs:6424-6460`): 선택 → `BulkItem`(선택 순서 · 메타) → 창 · [Rename] = 순차 rename +
//! **undo 1건**(`MoveBatchOp` · 실패 개별 격리 · `bulk.done/fail`) → 재열람 · 프리셋 = `<설정 폴더>/renames/<이름>.cfg`(`serialize_ops`/`parse_ops` ·
//! 이름 금지 문자 제거 · 같은 이름 덮어쓰기 · 최대 64).

use crate::app::dialogs::DlgReply;
use crate::bulk_win::{BulkAction, BulkItem, BulkWin};
use crate::dlg_win::{DlgArm, DlgInput, DlgSpec};
use crate::*;
use ndir_ops::batch_rename as br;
use std::path::Path;

/// 지금의 현지 시간대 오프셋(분) — 날짜 토큰(`${YYYY}` …)이 현지 시각으로 찍히게(dir2 `tz_offset_min` · 종전 dir3 = 0 = UTC라
/// 자정 근처 파일의 날짜가 하루 어긋났다).
pub(crate) fn local_tz_min() -> i32 {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let lt = nexa_fs::local_time(now);
    tz_min_from(secs, (lt.year, lt.month, lt.day), (lt.hour, lt.min, lt.sec))
}

/// UTC 초와 그 순간의 현지 달력 → 오프셋(분 · 순수). 달력을 UTC로 읽은 초와의 차이다.
pub(crate) fn tz_min_from(utc_secs: i64, ymd: (i32, u32, u32), hms: (u32, u32, u32)) -> i32 {
    // 그레고리력 → 1970-01-01부터의 날 수(Howard Hinnant `days_from_civil`).
    let (y, m, d) = (i64::from(ymd.0), i64::from(ymd.1), i64::from(ymd.2));
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let local = days * 86_400 + i64::from(hms.0) * 3600 + i64::from(hms.1) * 60 + i64::from(hms.2);
    // 분 단위로 반올림(초 경계의 1초 어긋남 흡수).
    i32::try_from((local - utc_secs + 30).div_euclid(60)).unwrap_or(0)
}

/// 되돌릴 수 없는 프리셋 확정(덮어쓰기 · 모두 제거)의 버튼 잠금 시간.
const PRESET_CONFIRM_MS: u64 = 3000;

impl App {
    /// `edit.bulk_rename` — 선택 없음 = 상태줄 안내(창 열지 않음 · DLG-074).
    pub(crate) fn open_bulk_rename(&mut self) {
        let sel = self.panels[self.active].selected_paths();
        let items: Vec<BulkItem> = sel.iter().filter_map(|p| BulkItem::from_path(p)).collect();
        if items.is_empty() {
            let mut inv = Invalidations::default();
            self.statusbar.set_left(&tr("bulk.noSelection"), &mut inv);
            self.redraw();
            return;
        }
        // 이미 열린 창이 있으면 그 창은 그대로 두고 새 창을 연다(사용자 10-18 "중복 띄우기" · 종전 = 같은 창을 다시 채워 카드 0장으로 열렸다).
        if self.bulk_win.is_open() {
            let old = std::mem::replace(&mut self.bulk_win, BulkWin::new());
            self.bulk_others.push(old);
        }
        // 이전/이후 열의 정렬 순서 = 여는 탭의 자연 정렬 값(탭 보기 옵션 · 10-08).
        self.bulk_win
            .set_natural(self.panels[self.active].active_view().4);
        self.bulk_win.set_items(items, local_tz_min());
        let style = self.flash_style();
        self.bulk_win.set_flash_style(style);
        self.bulk_win
            .set_presets(Self::preset_names(&Self::presets_dir()));
        self.open_bulk = true;
    }

    /// 프리셋 폴더(`<설정 폴더>/renames` · dir2 `data\\renames`).
    pub(crate) fn presets_dir() -> PathBuf {
        ndir_settings::config_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("renames")
    }

    /// 프리셋 순서 파일(`renames/order.txt` · 한 줄 = 이름 · 10-18 편집 창의 순서).
    fn preset_order_path(dir: &Path) -> PathBuf {
        dir.join("order.txt")
    }

    /// 프리셋 이름 목록(`*.cfg` · 최대 64): `order.txt`의 순서를 먼저, 거기 없는 이름은 뒤에 이름순.
    pub(crate) fn preset_names(dir: &Path) -> Vec<String> {
        let mut found: Vec<String> = std::fs::read_dir(dir)
            .map(|rd| {
                rd.flatten()
                    .filter_map(|e| {
                        let p = e.path();
                        (p.extension().is_some_and(|x| x == "cfg"))
                            .then(|| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                            .flatten()
                    })
                    .collect()
            })
            .unwrap_or_default();
        found.sort_by_key(|n| n.to_lowercase());
        let mut v: Vec<String> = Vec::new();
        if let Ok(text) = std::fs::read_to_string(Self::preset_order_path(dir)) {
            for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
                if let Some(pos) = found.iter().position(|n| n == line) {
                    v.push(found.remove(pos));
                }
            }
        }
        v.extend(found);
        v.truncate(64);
        v
    }

    /// 편집 창 결과 반영(10-18): `names`에 없는 프리셋 파일은 지우고 순서를 `order.txt`에.
    pub(crate) fn apply_preset_edit(dir: &Path, names: &[String]) {
        for n in Self::preset_names(dir) {
            if !names.contains(&n) {
                let _ = std::fs::remove_file(dir.join(format!("{n}.cfg")));
            }
        }
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(Self::preset_order_path(dir), names.join("\n"));
    }

    /// 모든 프리셋 제거(`*.cfg` + `order.txt`).
    pub(crate) fn clear_presets(dir: &Path) {
        for n in Self::preset_names(dir) {
            let _ = std::fs::remove_file(dir.join(format!("{n}.cfg")));
        }
        let _ = std::fs::remove_file(Self::preset_order_path(dir));
    }

    /// 열린 모든 일괄 창의 프리셋 메뉴를 폴더 상태로 다시.
    fn refresh_presets(&mut self) {
        let names = Self::preset_names(&Self::presets_dir());
        for w in std::iter::once(&mut self.bulk_win).chain(self.bulk_others.iter_mut()) {
            w.set_presets(names.clone());
            w.redraw();
        }
    }

    /// `id` 창을 `bulk_win` 자리로(다른 창이었으면 바꿔 넣는다) · 닫힌 창은 버린다. 그 창이면 `true`.
    pub(crate) fn bulk_select(&mut self, id: winit::window::WindowId) -> bool {
        if !self.bulk_win.is(id) {
            let Some(i) = self.bulk_others.iter().position(|w| w.is(id)) else {
                return false;
            };
            std::mem::swap(&mut self.bulk_win, &mut self.bulk_others[i]);
        }
        self.bulk_others.retain(BulkWin::is_open);
        true
    }

    /// `bulk_win`이 닫혔으면 남은 창 하나를 그 자리로.
    pub(crate) fn bulk_settle(&mut self) {
        self.bulk_others.retain(BulkWin::is_open);
        if !self.bulk_win.is_open() {
            if let Some(w) = self.bulk_others.pop() {
                self.bulk_win = w;
            }
        }
    }

    /// 열린 일괄 창 수(시험).
    #[cfg(test)]
    pub(crate) fn bulk_open_count(&self) -> usize {
        usize::from(self.bulk_win.is_open())
            + self.bulk_others.iter().filter(|w| w.is_open()).count()
    }

    /// 프리셋 이름 정리(DLG-083: `<>:"/\|?*` 제거 + trim · 빈 값 = None).
    pub(crate) fn sanitize_preset_name(name: &str) -> Option<String> {
        let s: String = name
            .chars()
            .filter(|c| !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
            .collect();
        let s = s.trim().to_string();
        (!s.is_empty()).then_some(s)
    }

    /// 창 결과 처리.
    pub(crate) fn bulk_action(&mut self, a: BulkAction) {
        match a {
            BulkAction::Close => {
                self.bulk_win.close();
                if let Some(w) = &self.window {
                    w.focus_window();
                }
            }
            BulkAction::Rename(list) => {
                self.bulk_win.close();
                self.bulk_apply(list);
                if let Some(w) = &self.window {
                    w.focus_window();
                }
            }
            BulkAction::SavePreset(text) => {
                self.bulk_pending_preset = Some(text);
                let spec = DlgSpec {
                    title: tr("bulk.preset.saveSeq"),
                    // 본문 없음 — 입력칸 라벨 하나로(종전 = "프리셋 이름"이 두 번 · 사용자 10-18).
                    text: String::new(),
                    buttons: vec![(1, tr("bulk.preset.ok")), (0, tr("bulk.preset.cancel"))],
                    default: 1,
                    cancel: 0,
                    input: Some(DlgInput {
                        label: tr("bulk.preset.name"),
                        masked: false,
                        // 바로 칠 수 있게 비워 두고 기본 이름은 플레이스홀더로(사용자 10-18) · 비운 채 확인 = 기본 이름.
                        initial: String::new(),
                        placeholder: tr("bulk.preset.savedSeq"),
                    }),
                    delay: None,
                    // 같은 이름이면 [확인]이 그 자리에서 "덮어쓰기 (3)" 무장 버튼으로(사용자 10-18 · 별도 창 없음 · 시간 지나면 원복).
                    arm: Some(DlgArm {
                        id: 1,
                        label: tr("bulk.preset.overwrite"),
                        ms: PRESET_CONFIRM_MS,
                        names: Self::preset_names(&Self::presets_dir()),
                        empty_as: tr("bulk.preset.savedSeq"),
                    }),
                };
                self.ask(spec, DlgReply::BulkPreset);
            }
            BulkAction::LoadPreset(name) => {
                let path = Self::presets_dir().join(format!("{name}.cfg"));
                if let Ok(text) = std::fs::read_to_string(&path) {
                    self.bulk_win.load_ops(&br::parse_ops(&text));
                    self.bulk_win.notify_preset_loaded(&name);
                }
            }
            BulkAction::EditPresets => {
                // 편집 창(DLG-084 · 10-18): 순서 · 개별 삭제 → OK에서 반영.
                self.preset_win
                    .set(Self::preset_names(&Self::presets_dir()));
                self.open_presets = true;
            }
            BulkAction::ClearPresets => {
                // 창이 이미 "제한 시간 안에 두 번"으로 확인했다(사용자 10-18) → 바로.
                Self::clear_presets(&Self::presets_dir());
                self.refresh_presets();
                self.bulk_win.notify_presets_cleared();
            }
            BulkAction::Paint | BulkAction::None => {}
        }
        self.redraw();
    }

    /// 프리셋 저장 대화상자 회신(확인 = 1 · 이름 입력).
    pub(crate) fn bulk_preset_saved(&mut self, id: i32, name: Option<String>) {
        let Some(text) = self.bulk_pending_preset.take() else {
            return;
        };
        if id != 1 {
            return;
        }
        let name = name
            .as_deref()
            .and_then(Self::sanitize_preset_name)
            .unwrap_or_else(|| tr("bulk.preset.savedSeq"));
        let dir = Self::presets_dir();
        // 같은 이름은 저장 창 안에서 이미 무장 버튼으로 재확인했다(`DlgArm` · 10-18) → 그대로 덮어쓴다.
        Self::write_preset(&dir, &name, &text);
        self.refresh_presets();
        self.bulk_win.notify_preset_saved(&name);
    }

    fn write_preset(dir: &Path, name: &str, text: &str) {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(dir.join(format!("{name}.cfg")), text);
    }

    /// 편집 창 OK(10-18): 순서 + 삭제 반영 → 메뉴 갱신 → 일괄 창으로 포커스.
    pub(crate) fn presets_edited(&mut self, names: Vec<String>) {
        Self::apply_preset_edit(&Self::presets_dir(), &names);
        self.refresh_presets();
        self.bulk_win.focus();
    }

    /// 순차 rename(실패 개별 격리) + undo 1건 + 재열람 + 토스트.
    pub(crate) fn bulk_apply(&mut self, list: Vec<(PathBuf, String)>) {
        // 덮어쓰지 않는 적용(직전 재확인 · 연쇄/맞바꾸기 = 임시 이름 2단계 · 대소문자만 변경) — 종전 `std::fs::rename`은 미리보기와
        // 적용 사이에 생긴 같은 이름의 파일을 조용히 덮어썼다(dir2 = `nexa_ops::rename`이 거부).
        let targets: Vec<(PathBuf, PathBuf)> = list
            .iter()
            .map(|(from, new_name)| (from.clone(), from.with_file_name(new_name)))
            .collect();
        let out = br::apply_renames(&targets);
        let (pairs, failed) = (out.done, out.failed.len());
        let done = pairs.len();
        if done > 0 {
            self.history
                .push(Box::new(ndir_ops::history::RenameBatchOp::new(
                    pairs,
                    trf("bulk.done", &[&done.to_string()]),
                )));
        }
        let mut inv = Invalidations::default();
        self.dirsizes.clear(); // 폴더 크기 캐시(T-166) — 이름이 바뀌었다.
        self.git_detail.clear(); // git 요약 — 작업으로 바뀌었다(10-06).
        for p in &mut self.panels {
            p.reopen(&mut inv);
        }
        let mut msg = trf("bulk.done", &[&done.to_string()]);
        if failed > 0 {
            msg.push_str(" · ");
            msg.push_str(&trf("bulk.fail", &[&failed.to_string()]));
        }
        let a = self.active;
        self.toast_panel(
            a,
            if failed > 0 {
                toast::ToastKind::Warn
            } else {
                toast::ToastKind::Info
            },
            tr("bulk.title"),
            msg.clone(),
        );
        self.statusbar.set_left(&msg, &mut inv);
        self.update_status();
    }
}
