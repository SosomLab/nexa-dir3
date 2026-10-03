//! App — 플러그인 매니저 1차(T-63 · EXT-415 축약 · dir2 EXT-125 드롭인 폴더 규약): 설정 창 플러그인 페이지의 [설치…](파일 창 `*.wasm` →
//! 검증 로드 → 사용자 플러그인 폴더에 복사) · 행 [삭제](사용자 설치분만 · 동봉분은 안내) · 적용 = 공급자 캐시 무효화 + 페이지 재구성(재시작 없음 · EXT-409).

use crate::*;
use std::path::Path;

impl App {
    /// 사용자 플러그인 폴더(설치 대상) — 시험 재지정 → `NDIR_PLUGINS_DIR` → `<설정 폴더>/plugins`.
    pub(crate) fn user_plugin_dir(&self) -> PathBuf {
        preview::user_plugin_dir()
    }

    /// 설정 창 플러그인 페이지 행: (id, `이름 (id) — ext, …`, 삭제 가능) — dir2 EXT-129 표시 문자열 유지 · 삭제 가능 = 사용자 폴더 안.
    pub(crate) fn plugin_rows(&self) -> Vec<(String, String, bool)> {
        let user = self.user_plugin_dir();
        preview::plugin_infos()
            .iter()
            .map(|p| {
                (
                    p.id.clone(),
                    format!("{} ({}) — {}", p.name, p.id, p.exts.join(", ")),
                    p.path.parent() == Some(user.as_path()),
                )
            })
            .collect()
    }

    /// 설치: 검증 로드(nx_meta) → 사용자 폴더에 같은 파일 이름으로 복사(덮어쓰기) → 캐시 무효화 → 페이지/도크 갱신 · 결과는 토스트.
    pub(crate) fn plugin_install(&mut self, path: &Path) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let result = preview::validate_plugin(path).and_then(|id| {
            let dir = self.user_plugin_dir();
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let dest = dir.join(&name);
            std::fs::copy(path, &dest).map_err(|e| e.to_string())?;
            Ok(id)
        });
        match result {
            Ok(id) => {
                preview::invalidate();
                self.toasts.push(
                    toast::ToastKind::Info,
                    tr("pref.cat.plugins"),
                    trf("plugins.installed", &[&id]),
                );
            }
            Err(e) => {
                self.toasts.push(
                    toast::ToastKind::Warn,
                    tr("pref.cat.plugins"),
                    trf("plugins.installFail", &[&name, &e]),
                );
            }
        }
        self.refresh_plugin_page();
    }

    /// 삭제: 사용자 폴더 안의 파일만(동봉분 = 안내) → 캐시 무효화 → 갱신.
    pub(crate) fn plugin_remove(&mut self, id: &str) {
        let user = self.user_plugin_dir();
        let path = preview::plugin_infos()
            .into_iter()
            .find(|p| p.id == id)
            .map(|p| p.path);
        match path {
            Some(p) if p.parent() == Some(user.as_path()) => match std::fs::remove_file(&p) {
                Ok(()) => {
                    preview::invalidate();
                    self.toasts.push(
                        toast::ToastKind::Info,
                        tr("pref.cat.plugins"),
                        trf("plugins.removed", &[id]),
                    );
                }
                Err(e) => self.toasts.push(
                    toast::ToastKind::Warn,
                    tr("pref.cat.plugins"),
                    trf("plugins.removeFail", &[id, &e.to_string()]),
                ),
            },
            Some(_) => self.toasts.push(
                toast::ToastKind::Info,
                tr("pref.cat.plugins"),
                tr("plugins.bundled"),
            ),
            None => {}
        }
        self.refresh_plugin_page();
    }

    /// 설정 창 플러그인 페이지 재구성 + 도크 재계산(공급자가 바뀌었을 수 있다).
    pub(crate) fn refresh_plugin_page(&mut self) {
        let rows = self.plugin_rows();
        self.prefs_win.set_plugins(rows, preview::load_notes());
        self.prefs_win.redraw();
        self.update_docks();
        self.redraw();
    }
}
