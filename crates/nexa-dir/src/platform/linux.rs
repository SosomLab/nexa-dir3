//! Linux 구현(T-50 몫: 셸 탐지 · 열기/보기). freedesktop Trash · uri-list 클립보드 · XDND · inotify · openpty · 용량(statvfs)은 T-53.

use super::*;

pub(super) struct NativeShell;

impl Shell for NativeShell {
    /// `$SHELL` → `/bin/bash` → `/bin/sh`(SKEL-425).
    fn candidates(&self) -> Vec<ShellSpec> {
        let mut names: Vec<(String, &[&str], &str)> = Vec::new();
        if let Some(sh) = std::env::var_os("SHELL") {
            names.push((sh.to_string_lossy().into_owned(), &["-l"], "$SHELL"));
        }
        names.push(("/bin/bash".into(), &["-l"], "bash"));
        names.push(("/bin/sh".into(), &[], "sh"));
        let owned: Vec<(&str, &[&str], &str)> =
            names.iter().map(|(n, a, l)| (n.as_str(), *a, *l)).collect();
        let mut v = shell_from_names(&owned);
        v.dedup_by(|a, b| a.program == b.program);
        v
    }
}

/// `xdg-open`(보기는 부모 폴더 열기 — 파일 관리자별 선택 지원은 T-53).
pub(super) fn opener() -> CommandOpener {
    CommandOpener {
        open: ["xdg-open", "{path}"]
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        reveal: ["xdg-open", "{path}"]
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    }
}
