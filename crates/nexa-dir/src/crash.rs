//! 패닉 훅·크래시 기록(T-46 · docs/18 §5 · docs/port/44 CI-072·112 · dir2 `win.rs:1406` `crash.txt`).
//!
//! 릴리스는 `panic = "abort"`라 패닉 = 즉시 종료 → **훅이 유일한 기록 수단**. `<HOME>/crash/crash-<unix초>.txt`에 메시지·위치·버전·OS·
//! 마지막 명령 id를 남기고, 다음 기동 때 아직 안 알린 가장 새 기록을 한 번 안내한다(`crash/.seen` 표식). 순수 함수 = 시험.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// 마지막으로 실행한 명령 id(훅이 읽는다 — 재현 불가 보고의 단서).
static LAST_CMD: OnceLock<Mutex<String>> = OnceLock::new();

pub(crate) fn note_command(id: &str) {
    if let Ok(mut g) = LAST_CMD.get_or_init(|| Mutex::new(String::new())).lock() {
        g.clear();
        g.push_str(id);
    }
}

pub(crate) fn last_command() -> String {
    LAST_CMD
        .get()
        .and_then(|m| m.lock().ok().map(|g| g.clone()))
        .unwrap_or_default()
}

/// 기록 본문(순수 — 시험).
pub(crate) fn report(info: &str, last_cmd: &str, unix_secs: u64) -> String {
    format!(
        "nexa-dir {} crash\ntime: {} (unix {unix_secs})\nos: {} {}\nlast command: {}\n\n{info}\n",
        env!("CARGO_PKG_VERSION"),
        crate::filelist::format_time((unix_secs as i64) * 1000),
        std::env::consts::OS,
        std::env::consts::ARCH,
        if last_cmd.is_empty() { "-" } else { last_cmd },
    )
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// 패닉 훅 설치 — `home/crash/`에 기록 · stderr에도 한 줄. 훅 안에서는 패닉하지 않는다(모든 실패 무시).
pub(crate) fn install(home: Option<PathBuf>) {
    std::panic::set_hook(Box::new(move |info| {
        let text = report(&info.to_string(), &last_command(), now_secs());
        eprintln!("{text}");
        if let Some(home) = &home {
            let dir = home.join("crash");
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join(format!("crash-{}.txt", now_secs())), text);
        }
    }));
}

/// 아직 안 알린 가장 새 크래시 기록 — 돌려주면서 `.seen`에 적어 다음엔 `None`(기동 때 한 번 안내 · 안내는 호스트 몫).
pub(crate) fn take_unreported(home: &Path) -> Option<PathBuf> {
    let dir = home.join("crash");
    let seen = std::fs::read_to_string(dir.join(".seen")).unwrap_or_default();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("crash-") && n.ends_with(".txt"))
        })
        .collect();
    files.sort();
    let newest = files.pop()?;
    let name = newest.file_name()?.to_string_lossy().into_owned();
    if seen.trim() == name {
        return None;
    }
    let _ = std::fs::write(dir.join(".seen"), &name);
    Some(newest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_has_version_os_and_last_command() {
        note_command("view.hidden");
        let r = report(
            "panicked at src/x.rs:1:1:\nboom",
            &last_command(),
            1_791_030_896,
        );
        assert!(r.starts_with(&format!("nexa-dir {} crash\n", env!("CARGO_PKG_VERSION"))));
        assert!(r.contains("time: 2026-10-03 12:34 (unix 1791030896)"));
        assert!(r.contains(&format!("os: {} ", std::env::consts::OS)));
        assert!(r.contains("last command: view.hidden"));
        assert!(r.ends_with("boom\n"));
        assert!(report("x", "", 0).contains("last command: -"));
    }

    #[test]
    fn unreported_is_taken_once_and_newest_first() {
        let home = std::env::temp_dir().join(format!("ndir-crash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        assert!(take_unreported(&home).is_none(), "폴더 없음 = 없음");
        let dir = home.join("crash");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("crash-100.txt"), "old").unwrap();
        std::fs::write(dir.join("crash-200.txt"), "new").unwrap();
        std::fs::write(dir.join("notes.md"), "ignore").unwrap();
        let got = take_unreported(&home).expect("newest");
        assert!(got.ends_with("crash-200.txt"));
        assert!(take_unreported(&home).is_none(), "한 번 알리면 끝");
        std::fs::write(dir.join("crash-300.txt"), "newer").unwrap();
        assert!(take_unreported(&home).unwrap().ends_with("crash-300.txt"));
        let _ = std::fs::remove_dir_all(&home);
    }
}
