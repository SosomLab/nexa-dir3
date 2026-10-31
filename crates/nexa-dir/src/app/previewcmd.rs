//! F3 · 도크 ↗ — 독립 미리보기 창 열기(T-62 B · dir2 `open_preview_window` · docs/port/20 §5.4 메시지 흐름):
//! 단일 선택 파일 → 시임(`preview_for`) → Lines = 창 · Image = 안내 1줄 · Archive = **그리드 창**(`archive_win` · T-62 C · dir2 PLUG-070) ·
//! `NeedPassword` = 마스킹 입력 대화상자 → `read_via`(활성 암호 슬롯) → 성공 = 세션 기억(`pw::remember`) · 다시 실패 = 폐기 + 재시도 문구.

use crate::app::dialogs::DlgReply;
use crate::dlg_win::{DlgInput, DlgSpec};
use crate::preview::archive::{self, ArchiveStatus};
use crate::preview::{self, PreviewDoc};
use crate::*;
use ndir_core::secret::Secret;

impl App {
    /// 지정 패널의 단일 선택 파일을 독립 창으로. 폴더·다중 선택·선택 없음 = 무동작.
    /// Quick Look 토글(10-18 · Finder Space/⌘Y · 노틸러스 Space): 열려 있으면 닫고(프로세스 종료) · 아니면 선택으로 연다 · 이 OS/환경에
    /// 없으면(Windows · Sushi 없음) 앱 미리보기 창(F3)으로 대신.
    pub(crate) fn quick_look_toggle(&mut self) {
        // macOS 인-프로세스 QLPreviewPanel(T-187 · 10-31): 선택을 따라가는 패널 · 실패/가짜 플랫폼 = 종전 `qlmanage` 경로.
        #[cfg(target_os = "macos")]
        if self.platform.log.is_none() {
            if let Some(view) = self
                .window
                .as_ref()
                .and_then(|w| crate::winfocus::ns_view_ptr(w))
            {
                platform::macql::install(view);
            }
            if platform::macql::available() {
                if platform::macql::is_visible() {
                    platform::macql::hide();
                    return;
                }
                let paths = self.panels[self.active].selected_paths_in_view_order();
                if paths.is_empty() || platform::macql::show(&paths) {
                    return;
                }
            }
        }
        if let Some(mut child) = self.quicklook.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
        }
        let paths = self.panels[self.active].selected_paths_in_view_order();
        if paths.is_empty() {
            return;
        }
        match self.platform.opener.quick_look(&paths) {
            Ok(child) => self.quicklook = Some(child),
            Err(_) => self.open_preview_window(self.active),
        }
    }

    /// 훑어보기 덤프(`ql.dump:<파일>` · T-187): macOS = 설치(패널 안 띄움) 뒤 `available <bool> visible <bool>` · 다른 OS = `available false`.
    pub(crate) fn quick_look_dump(&mut self) -> String {
        #[cfg(target_os = "macos")]
        if self.platform.log.is_none() {
            if let Some(view) = self
                .window
                .as_ref()
                .and_then(|w| crate::winfocus::ns_view_ptr(w))
            {
                platform::macql::install(view);
            }
            return format!(
                "available {} visible {}\n",
                platform::macql::available(),
                platform::macql::is_visible()
            );
        }
        "available false visible false\n".to_string()
    }

    /// 훑어보기 선택 추종(macOS 패널이 열려 있을 때 · 도크 갱신 길목에서) — 다른 OS/가짜 = 무동작.
    pub(crate) fn quick_look_follow(&mut self) {
        #[cfg(target_os = "macos")]
        if self.platform.log.is_none() && platform::macql::available() {
            let paths = self.panels[self.active].selected_paths_in_view_order();
            platform::macql::follow(&paths);
        }
    }

    /// 패널 안 키(↑/↓ = 커서 이동 · Space = 닫기)를 거둬 적용한다(유휴 틱 · macOS).
    pub(crate) fn quick_look_keys_tick(&mut self) -> bool {
        #[cfg(target_os = "macos")]
        {
            let keys = platform::macql::take_keys();
            if keys.is_empty() {
                return false;
            }
            for k in keys {
                match k {
                    platform::macql::QlKey::Up | platform::macql::QlKey::Down => {
                        let key = if k == platform::macql::QlKey::Up {
                            nexa_ctl::Key::Up
                        } else {
                            nexa_ctl::Key::Down
                        };
                        self.route(InputEvent::Key {
                            key,
                            shift: false,
                            primary: false,
                        });
                    }
                    platform::macql::QlKey::Toggle => platform::macql::hide(),
                }
            }
            self.quick_look_follow();
            return true;
        }
        #[allow(unreachable_code)]
        false
    }

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
                // 읽어 둔 목록을 그대로 그리드 창에(재조회 없음 · 실패 상태도 사유 줄과 함께 연다).
                self.archive_win
                    .set_doc(&title, *doc, app::bulk::local_tz_min());
                self.open_archive = true;
                return;
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
                placeholder: String::new(),
            }),
            delay: None,
            arm: None,
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
