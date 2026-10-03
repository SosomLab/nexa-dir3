//! App — 일괄 이름 변경(T-71 · dir2 DLG-074~088 호출부 `win.rs:6424-6460`): 선택 → `BulkItem`(선택 순서 · 메타) → 창 · [Rename] = 순차 rename +
//! **undo 1건**(`MoveBatchOp` · 실패 개별 격리 · `bulk.done/fail`) → 재열람 · 프리셋 = `<설정 폴더>/renames/<이름>.cfg`(`serialize_ops`/`parse_ops` ·
//! 이름 금지 문자 제거 · 같은 이름 덮어쓰기 · 최대 64).

use crate::app::dialogs::DlgReply;
use crate::bulk_win::{BulkAction, BulkItem};
use crate::dlg_win::{DlgInput, DlgSpec};
use crate::*;
use ndir_ops::batch_rename as br;
use std::path::Path;

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
        self.bulk_win.set_items(items, 0);
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

    /// 프리셋 이름 목록(`*.cfg` · 이름순 · 최대 64).
    pub(crate) fn preset_names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir)
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
        v.sort_by_key(|n| n.to_lowercase());
        v.truncate(64);
        v
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
                    text: tr("bulk.preset.savePrompt"),
                    buttons: vec![(1, tr("bulk.preset.ok")), (0, tr("bulk.preset.cancel"))],
                    default: 1,
                    cancel: 0,
                    input: Some(DlgInput {
                        label: tr("bulk.preset.name"),
                        masked: false,
                        initial: tr("bulk.preset.savedSeq"),
                    }),
                };
                self.ask(spec, DlgReply::BulkPreset);
            }
            BulkAction::LoadPreset(name) => {
                let path = Self::presets_dir().join(format!("{name}.cfg"));
                if let Ok(text) = std::fs::read_to_string(&path) {
                    self.bulk_win.load_ops(&br::parse_ops(&text));
                }
            }
            BulkAction::EditPresets => {
                // 관리 팝업(DLG-084)은 후속 — 폴더를 연다.
                let dir = Self::presets_dir();
                let _ = std::fs::create_dir_all(&dir);
                let _ = self.platform.opener.open(&dir);
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
        let Some(name) = name.as_deref().and_then(Self::sanitize_preset_name) else {
            return;
        };
        let dir = Self::presets_dir();
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join(format!("{name}.cfg")), text);
        self.bulk_win.set_presets(Self::preset_names(&dir));
        self.bulk_win.redraw();
    }

    /// 순차 rename(실패 개별 격리) + undo 1건 + 재열람 + 토스트.
    pub(crate) fn bulk_apply(&mut self, list: Vec<(PathBuf, String)>) {
        let mut pairs: Vec<(PathBuf, PathBuf)> = Vec::new();
        let mut failed = 0usize;
        for (from, new_name) in &list {
            let to = from.with_file_name(new_name);
            match std::fs::rename(from, &to) {
                Ok(()) => pairs.push((from.clone(), to)),
                Err(_) => failed += 1,
            }
        }
        let done = pairs.len();
        if done > 0 {
            self.history
                .push(Box::new(ndir_ops::history::MoveBatchOp::new(
                    pairs,
                    trf("bulk.done", &[&done.to_string()]),
                )));
        }
        let mut inv = Invalidations::default();
        for p in &mut self.panels {
            p.reopen(&mut inv);
        }
        let mut msg = trf("bulk.done", &[&done.to_string()]);
        if failed > 0 {
            msg.push_str(" · ");
            msg.push_str(&trf("bulk.fail", &[&failed.to_string()]));
        }
        self.toasts.push(
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
