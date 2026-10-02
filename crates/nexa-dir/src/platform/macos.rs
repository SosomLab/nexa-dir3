//! macOS 구현(T-50 몫: 셸 탐지 · 열기/보기). trashItem · NSPasteboard · NSDragging · FSEvents · forkpty · 용량(statvfs)은 T-52.

use super::*;

pub(super) struct NativeShell;

impl Shell for NativeShell {
    /// `$SHELL` → `/bin/zsh` → `/bin/bash` → `/bin/sh`(SKEL-425).
    fn candidates(&self) -> Vec<ShellSpec> {
        let mut names: Vec<(String, &[&str], &str)> = Vec::new();
        if let Some(sh) = std::env::var_os("SHELL") {
            names.push((sh.to_string_lossy().into_owned(), &["-l"], "$SHELL"));
        }
        names.push(("/bin/zsh".into(), &["-l"], "zsh"));
        names.push(("/bin/bash".into(), &["-l"], "bash"));
        names.push(("/bin/sh".into(), &[], "sh"));
        let owned: Vec<(&str, &[&str], &str)> =
            names.iter().map(|(n, a, l)| (n.as_str(), *a, *l)).collect();
        let mut v = shell_from_names(&owned);
        v.dedup_by(|a, b| a.program == b.program);
        v
    }
}

/// `open` · `open -R`(Finder에서 보기).
pub(super) fn opener() -> CommandOpener {
    CommandOpener {
        open: ["open", "{path}"]
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        reveal: ["open", "-R", "{path}"]
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    }
}
