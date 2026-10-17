//! OS가 "이 경로를 열어 달라"고 넘기는 길(nexa-ui 212차 nexa-sys `open_files` 소비 · 10-18 · nexa-sql T-352와 같은 꼴):
//! macOS Finder 더블클릭 · Dock 아이콘에 떨어뜨리기 · `open -a "Nexa Dir" 경로`는 argv가 아니라 Apple Event(`odoc`)로 온다 → nexa-sys가
//! 큐에 쌓고 호스트가 `about_to_wait`마다 비워 연다. Windows · Linux는 파일 연결이 argv로 오므로(`cli::start_dir`) 큐가 늘 비어 있다.
//!
//! 규칙: 폴더 = 활성 패널에 **새 탭**으로 · 파일 = 그 부모 폴더를 새 탭으로 열고 **그 파일을 선택**(Finder "다음으로 열기"는 파일을 넘긴다) ·
//! 없는 경로 = 토스트 안내. 시험 훅 = 기동 명령 `os.open:<경로>`(모든 OS · `open_files::simulate`).

use crate::*;
use std::path::{Path, PathBuf};

impl App {
    /// OS 파일 열기 큐를 비워 연다(비어 있으면 비용 = 잠금 한 번). 돌려주는 값 = 연 개수.
    pub(crate) fn pump_os_open_files(&mut self, inv: &mut Invalidations) -> usize {
        let paths = nexa_sys::open_files::take();
        if paths.is_empty() {
            return 0;
        }
        paths.iter().filter(|p| self.open_os_path(p, inv)).count()
    }

    /// 경로 하나를 연다(위 규칙). `true` = 탭을 열었다.
    pub(crate) fn open_os_path(&mut self, path: &Path, inv: &mut Invalidations) -> bool {
        let (dir, select): (PathBuf, Option<PathBuf>) = if path.is_dir() {
            (path.to_path_buf(), None)
        } else if path.is_file() {
            match path.parent() {
                Some(d) if !d.as_os_str().is_empty() => (d.to_path_buf(), Some(path.to_path_buf())),
                _ => return false,
            }
        } else {
            self.toasts.push(
                toast::ToastKind::Warn,
                ndir_i18n::tr("osopen.title"),
                ndir_i18n::trf("osopen.missing", &[&path.display().to_string()]),
            );
            return false;
        };
        let a = self.active;
        let panel = &mut self.panels[a];
        panel.new_tab(inv);
        if let Some(why) = panel.navigate_to(dir, inv) {
            self.toasts
                .push(toast::ToastKind::Warn, ndir_i18n::tr("osopen.title"), why);
            return false;
        }
        if let Some(f) = select {
            panel.select_path(&f, inv);
        }
        self.update_status();
        true
    }
}
