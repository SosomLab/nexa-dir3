//! 퀵 런처(T-42 · dir2 `launcher.rs` 3-OS 이식 · docs/port/10 WINA-029·054 · KEY-066): 외부 프로그램 버튼 바.
//!
//! 항목 = `라벨|exe|인자`(인자 안의 `|` 보존 · `-` = 그룹 구분선 · `%path%` = 활성 패널 현재 폴더) — 설정 `launcher.items`는 항목을 **`;;`**로 잇는다
//! (파일 값은 한 줄 · 실제 줄바꿈도 허용. 두 글자 `\n`은 쓰지 않는다 — Windows 경로 `\nexa…`가 줄바꿈으로 오해됐다 · 10-03 시험 적발) · `launcher.seed` = 적용된 시드 버전. 첫 실행(둘 다 비어 있음) = OS별 시드 ·
//! 시드 버전이 낮으면 누락분만 1회 추가(사용자 편집 보존 — dir2 `seed_missing`).
//! 실행 = `std::process::Command`(작업 디렉터리 = 그 폴더 · macOS `.app`은 `open -a`). 실패는 상태줄 안내(앱 무중단).

use std::path::{Path, PathBuf};

/// 시드 버전(dir2 v2 = VS Code + pwsh/PowerShell + cmd · 3-OS 동형).
pub(crate) const SEED_VERSION: u32 = 2;
/// 항목 상한(dir2 32).
pub(crate) const MAX_ITEMS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LauncherItem {
    pub label: String,
    pub exe: String,
    pub args: String,
}

impl LauncherItem {
    pub(crate) fn separator() -> Self {
        LauncherItem {
            label: "-".into(),
            exe: String::new(),
            args: String::new(),
        }
    }

    pub(crate) fn is_separator(&self) -> bool {
        self.label == "-" && self.exe.is_empty()
    }

    /// `라벨|exe|인자` 한 줄(구분선 = `-`).
    pub(crate) fn encode(&self) -> String {
        if self.is_separator() {
            "-".into()
        } else {
            format!("{}|{}|{}", self.label, self.exe, self.args)
        }
    }

    /// 한 줄 파싱 — `-` = 구분선 · 필드 2개 이상 · 인자 안의 `|`는 보존.
    pub(crate) fn parse(line: &str) -> Option<Self> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }
        if line == "-" {
            return Some(Self::separator());
        }
        let mut it = line.splitn(3, '|');
        let label = it.next()?.trim().to_string();
        let exe = it.next()?.trim().to_string();
        let args = it.next().unwrap_or("").trim().to_string();
        if label.is_empty() || exe.is_empty() {
            return None;
        }
        Some(LauncherItem { label, exe, args })
    }
}

/// 항목 구분자(설정 한 줄 규약 — 경로·인자에 사실상 나오지 않는 두 글자).
pub(crate) const ITEM_SEP: &str = ";;";

/// 설정 값(`launcher.items`) → 항목 목록(`;;` 또는 실제 줄바꿈 구분 · 상한 32 · 손상 항목 무시).
pub(crate) fn parse_items(text: &str) -> Vec<LauncherItem> {
    text.split(['\n', '\r'])
        .flat_map(|line| line.split(ITEM_SEP))
        .filter_map(LauncherItem::parse)
        .take(MAX_ITEMS)
        .collect()
}

/// 항목 목록 → 설정 값(`;;` 구분).
pub(crate) fn encode_items(items: &[LauncherItem]) -> String {
    items
        .iter()
        .map(LauncherItem::encode)
        .collect::<Vec<_>>()
        .join(ITEM_SEP)
}

/// 실행 파일의 실제 경로(아이콘 조회용 · T-30 B): 절대 경로면 존재할 때 그대로 · 아니면 PATH 검색(Windows는 `.exe` 보충) · 없으면 None.
pub(crate) fn exe_path(exe: &str) -> Option<PathBuf> {
    let p = Path::new(exe);
    if p.is_absolute() {
        return p.is_file().then(|| p.to_path_buf());
    }
    if exe.contains(['/', '\\']) {
        return None;
    }
    on_path(exe)
        .or_else(|| {
            (cfg!(windows) && !exe.to_ascii_lowercase().ends_with(".exe"))
                .then(|| on_path(&format!("{exe}.exe")))
                .flatten()
        })
        .map(PathBuf::from)
}

fn on_path(name: &str) -> Option<String> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .filter(|d| !d.as_os_str().is_empty() && d.is_absolute())
        .map(|d| d.join(name))
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().into_owned())
}

#[cfg(windows)]
fn resolve_vscode() -> Option<String> {
    [
        std::env::var("LOCALAPPDATA")
            .map(|p| format!("{p}\\Programs\\Microsoft VS Code\\Code.exe")),
        std::env::var("ProgramFiles").map(|p| format!("{p}\\Microsoft VS Code\\Code.exe")),
        std::env::var("ProgramFiles(x86)").map(|p| format!("{p}\\Microsoft VS Code\\Code.exe")),
    ]
    .into_iter()
    .flatten()
    .find(|p| Path::new(p).is_file())
}

#[cfg(target_os = "macos")]
fn resolve_vscode() -> Option<String> {
    ["/Applications/Visual Studio Code.app"]
        .into_iter()
        .find(|p| Path::new(p).is_dir())
        .map(str::to_string)
        .or_else(|| on_path("code"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn resolve_vscode() -> Option<String> {
    on_path("code").or_else(|| on_path("codium"))
}

/// OS별 셸/터미널 시드(발견분만).
fn shell_items() -> Vec<LauncherItem> {
    let mut out = Vec::new();
    #[cfg(windows)]
    {
        if let Some(p) = on_path("pwsh.exe") {
            out.push(LauncherItem {
                label: "pwsh".into(),
                exe: p,
                args: String::new(),
            });
        } else if let Some(p) = std::env::var("SystemRoot")
            .ok()
            .map(|r| format!("{r}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"))
            .filter(|p| Path::new(p).is_file())
        {
            out.push(LauncherItem {
                label: "PowerShell".into(),
                exe: p,
                args: String::new(),
            });
        }
        if let Some(p) = std::env::var("ComSpec")
            .ok()
            .filter(|p| Path::new(p).is_file())
            .or_else(|| on_path("cmd.exe"))
        {
            out.push(LauncherItem {
                label: "cmd".into(),
                exe: p,
                args: String::new(),
            });
        }
    }
    #[cfg(target_os = "macos")]
    {
        for app in [
            "/System/Applications/Utilities/Terminal.app",
            "/Applications/Utilities/Terminal.app",
        ] {
            if Path::new(app).is_dir() {
                out.push(LauncherItem {
                    label: "Terminal".into(),
                    exe: app.into(),
                    args: "\"%path%\"".into(),
                });
                break;
            }
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        for name in [
            "x-terminal-emulator",
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
            "alacritty",
            "kitty",
            "xterm",
        ] {
            if let Some(p) = on_path(name) {
                out.push(LauncherItem {
                    label: "Terminal".into(),
                    exe: p,
                    args: String::new(),
                });
                break;
            }
        }
    }
    out
}

/// 첫 실행 시드(v2): [VS Code] │ [셸/터미널](발견분만 · 구분선으로 그룹).
pub(crate) fn seed() -> Vec<LauncherItem> {
    let mut out: Vec<LauncherItem> = resolve_vscode()
        .map(|exe| {
            vec![LauncherItem {
                label: "VS Code".into(),
                exe,
                args: "\"%path%\"".into(),
            }]
        })
        .unwrap_or_default();
    let shells = shell_items();
    if !shells.is_empty() {
        if !out.is_empty() {
            out.push(LauncherItem::separator());
        }
        out.extend(shells);
    }
    out
}

/// 구버전 시드 마이그레이션(1회 — `launcher.seed` < [`SEED_VERSION`]): 셸/터미널 항목이 없으면 뒤에 추가. 사용자 항목·비움 편집은 보존.
pub(crate) fn seed_missing(items: &mut Vec<LauncherItem>) {
    let has_shell = items.iter().any(|i| {
        !i.is_separator()
            && [
                "pwsh",
                "powershell",
                "cmd.exe",
                "terminal",
                "term",
                "konsole",
                "xterm",
            ]
            .iter()
            .any(|n| i.exe.to_ascii_lowercase().contains(n))
    });
    if has_shell {
        return;
    }
    let add = shell_items();
    if add.is_empty() {
        return;
    }
    if !items.is_empty() && !items.last().is_some_and(LauncherItem::is_separator) {
        items.push(LauncherItem::separator());
    }
    items.extend(add);
}

/// 인자 문자열 → 토큰(공백 구분 · 큰따옴표 묶음 · `%path%` 치환).
pub(crate) fn split_args(args: &str, folder: &Path) -> Vec<String> {
    let text = args.replace("%path%", &folder.to_string_lossy());
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut any = false;
    for c in text.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            ' ' | '\t' if !quoted => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            _ => {
                cur.push(c);
                any = true;
            }
        }
    }
    if any {
        out.push(cur);
    }
    out
}

/// 실행(작업 디렉터리 = 폴더 · macOS `.app` = `open -a`). 실패 사유 반환.
pub(crate) fn launch(item: &LauncherItem, folder: &Path) -> Result<(), String> {
    if item.is_separator() {
        return Err("separator".into());
    }
    let args = split_args(&item.args, folder);
    let mut cmd = if cfg!(target_os = "macos") && item.exe.ends_with(".app") {
        let mut c = std::process::Command::new("open");
        c.arg("-a").arg(&item.exe);
        if !args.is_empty() {
            c.arg("--args").args(&args);
        }
        c
    } else {
        let mut c = std::process::Command::new(&item.exe);
        c.args(&args);
        c
    };
    cmd.current_dir(folder);
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// 시드/마이그레이션 적용(App::new): (항목, 설정에 쓸 값이 바뀌었는가).
pub(crate) fn load_or_seed(items_text: &str, seed_version: u32) -> (Vec<LauncherItem>, bool) {
    let mut items = parse_items(items_text);
    let mut changed = false;
    if items_text.trim().is_empty() && seed_version == 0 {
        items = seed();
        changed = true;
    } else if seed_version < SEED_VERSION {
        let before = items.len();
        seed_missing(&mut items);
        changed = items.len() != before;
    }
    (items, changed || seed_version < SEED_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_encode_round_trip_and_separator() {
        let text = "VS Code|C:\\nexa\\Code.exe|\"%path%\";;-;;cmd|cmd.exe|/k echo a|b";
        let items = parse_items(text);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].label, "VS Code");
        assert_eq!(
            items[0].exe, "C:\\nexa\\Code.exe",
            "경로의 \\n은 줄바꿈이 아니다"
        );
        assert!(items[1].is_separator());
        assert_eq!(items[2].args, "/k echo a|b", "인자 안의 | 보존");
        assert_eq!(parse_items(&encode_items(&items)), items, "왕복");
        assert!(LauncherItem::parse("broken").is_none());
        assert!(LauncherItem::parse("|exe|").is_none());
        let many = (0..40)
            .map(|i| format!("l{i}|e{i}|"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            parse_items(&many).len(),
            MAX_ITEMS,
            "줄바꿈 구분도 허용 · 상한 32"
        );
        let many = (0..40)
            .map(|i| format!("l{i}|e{i}|"))
            .collect::<Vec<_>>()
            .join(";;");
        assert_eq!(parse_items(&many).len(), MAX_ITEMS, "상한 32");
    }

    #[test]
    fn args_split_and_path_substitution() {
        let f = Path::new("C:/My Dir");
        assert_eq!(split_args("\"%path%\" -n", f), vec!["C:/My Dir", "-n"]);
        assert_eq!(split_args("", f), Vec::<String>::new());
        assert_eq!(split_args("a   b", f), vec!["a", "b"]);
    }

    #[test]
    fn seed_and_migration_rules() {
        // 첫 실행 = 시드. 발견분만 넣으므로 GUI 터미널·VS Code가 없는 Linux CI 러너(ubuntu-latest · 10-03 CI 적발)에서는 비어
        // 있을 수 있다 — 시드 함수와 일치하는지만 본다.
        let (items, changed) = load_or_seed("", 0);
        assert!(changed);
        assert_eq!(items.is_empty(), seed().is_empty(), "시드 = seed()");
        if cfg!(windows) {
            assert!(!items.is_empty(), "Windows는 cmd.exe가 늘 있다");
        }
        // 사용자 편집(비움)은 보존: 값은 비었지만 시드 버전이 현재면 시드하지 않는다.
        let (items, changed) = load_or_seed("", SEED_VERSION);
        assert!(items.is_empty() && !changed);
        // 구버전 시드 + 사용자 항목만 = 셸 누락분 추가(구분선 동반).
        let (items, changed) = load_or_seed("Mine|notepad.exe|", 1);
        assert!(changed && items[0].label == "Mine");
        if items.len() > 1 {
            assert!(items[1].is_separator());
        }
    }

    #[test]
    fn launch_runs_our_own_binary_and_reports_failure() {
        let exe = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let ok = LauncherItem {
            label: "self".into(),
            exe,
            args: "--version".into(),
        };
        assert_eq!(launch(&ok, &std::env::temp_dir()), Ok(()));
        let bad = LauncherItem {
            label: "nope".into(),
            exe: "definitely-not-a-program-xyz".into(),
            args: String::new(),
        };
        assert!(launch(&bad, &std::env::temp_dir()).is_err());
        assert!(launch(&LauncherItem::separator(), &std::env::temp_dir()).is_err());
    }
}
