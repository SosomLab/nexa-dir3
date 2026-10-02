//! F3 · 도크 ↗ — 독립 미리보기 창 열기(T-62 B · dir2 `open_preview_window` · docs/port/20 §5.4 메시지 흐름):
//! 단일 선택 파일 → 시임(`preview_for`) → Lines = 창 · Image = 안내 1줄 · Archive = 요약 텍스트(그리드 창은 T-62 C) ·
//! `NeedPassword` = 마스킹 입력 대화상자 → `read_via`(활성 암호 슬롯) → 성공 = 세션 기억(`pw::remember`) · 다시 실패 = 폐기 + 재시도 문구.

use crate::app::dialogs::DlgReply;
use crate::dlg_win::{DlgInput, DlgSpec};
use crate::preview::archive::{self, ArchiveStatus};
use crate::preview::{self, PreviewDoc};
use crate::*;
use ndir_core::secret::Secret;

impl App {
    /// 지정 패널의 단일 선택 파일을 독립 창으로. 폴더·다중 선택·선택 없음 = 무동작.
    pub(crate) fn open_preview_window(&mut self, panel: usize) {
        let sel = self.panels[panel].selected_paths();
        let [path] = &sel[..] else {
            return;
        };
        if path.is_dir() {
            return;
        }
        let path = path.clone();
        let map = self.settings.get("preview.map").unwrap_or("").to_string();
        let disabled = self
            .settings
            .get("plugins.disabled")
            .unwrap_or("")
            .to_string();
        preview::set_dark(self.theme.is_dark);
        let (doc, _) = preview::preview_for(&path, &map, &disabled);
        self.show_preview_doc(&path, doc, false);
    }

    /// 공급자 결과 → 창 또는 암호 대화상자.
    pub(crate) fn show_preview_doc(
        &mut self,
        path: &std::path::Path,
        doc: PreviewDoc,
        retry: bool,
    ) {
        let title = ndir_ops::leaf_name(path);
        let lines = match doc {
            PreviewDoc::Lines(l) => l,
            PreviewDoc::Image(_) => vec![tr("preview.window.image")],
            PreviewDoc::Archive(doc) => {
                if doc.status == ArchiveStatus::NeedPassword {
                    self.ask_archive_password(path, retry);
                    return;
                }
                archive::summary_lines(&doc, 0, 500)
            }
        };
        self.preview_win.set_lines(&title, lines);
        self.open_preview = true;
    }

    fn ask_archive_password(&mut self, path: &std::path::Path, retry: bool) {
        let name = ndir_ops::leaf_name(path);
        let mut text = trf("archive.pw.prompt", &[&name]);
        if retry {
            text.push('\n');
            text.push_str(&tr("archive.pw.wrong"));
        }
        text.push('\n');
        text.push_str(&tr("archive.pw.note"));
        let spec = DlgSpec {
            title: tr("archive.pw.title"),
            text,
            buttons: vec![(1, tr("archive.pw.ok")), (0, tr("archive.pw.cancel"))],
            default: 1,
            cancel: 0,
            input: Some(DlgInput {
                label: tr("archive.pw.label"),
                masked: true,
                initial: String::new(),
            }),
        };
        self.ask(spec, DlgReply::ArchivePassword(path.to_path_buf()));
    }

    /// 암호 대화상자 결과(dir2 `archivewnd::open` 루프): 취소 = 아무 창도 안 엶 · 확인 = 같은 공급자 경로로 재시도 →
    /// Ok = 세션 기억 · 다시 NeedPassword = 폐기 + 재시도 · 그 외 = 상태 그대로 표시.
    pub(crate) fn archive_password_result(&mut self, path: PathBuf, id: i32, text: Option<String>) {
        if id != 1 {
            return;
        }
        let mut raw = text.unwrap_or_default();
        let secret = Secret::take_from_string(&mut raw);
        if secret.is_empty() {
            self.ask_archive_password(&path, true);
            return;
        }
        let map = self.settings.get("preview.map").unwrap_or("").to_string();
        let disabled = self
            .settings
            .get("plugins.disabled")
            .unwrap_or("")
            .to_string();
        let doc = archive::read_via(&path, &map, &disabled, Some(secret.clone()));
        match doc.status {
            ArchiveStatus::Ok => {
                archive::pw::remember(&path, secret);
                self.show_preview_doc(&path, PreviewDoc::Archive(Box::new(doc)), false);
            }
            ArchiveStatus::NeedPassword => {
                archive::pw::forget(&path);
                self.ask_archive_password(&path, true);
            }
            _ => self.show_preview_doc(&path, PreviewDoc::Archive(Box::new(doc)), false),
        }
    }
}
