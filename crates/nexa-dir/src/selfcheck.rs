//! 자가 점검(doctor) — `--selfcheck` 와 Help ▸ 자가 점검 창이 **같은 함수**를 쓴다(docs/18 §6 · DR-10).
//!
//! 원칙: 격리 홈·임시 폴더 안에서만 쓴다 · 네트워크 0 · 항목 하나의 실패가 다음을 막지 않는다 ·
//! 각 항목은 포트(platform/) 구현의 공개 메서드를 그대로 호출한다(점검 전용 우회 경로 금지).
//! M0: 그룹 표와 보고 형식만 세우고 `env`만 실제 점검. 나머지는 SKIP(미구현)으로 자리를 비워 둔다 —
//! 각 마일스톤이 자기 그룹을 채운다(config·resources·license = M1 · fonts = M2 · fs·trash·shell·pty·ctxmenu·clipboard·dnd·open = M4 ·
//! preview·plugin·archive = M5 · window = M3).

use std::fmt::Write as _;
use std::time::Instant;

/// 점검 그룹 — 순서가 보고 순서(docs/18 §6 표와 같다).
pub(crate) const GROUPS: &[&str] = &[
    "env",
    "config",
    "resources",
    "fonts",
    "fs",
    "trash",
    "shell",
    "pty",
    "ctxmenu",
    "clipboard",
    "dnd",
    "open",
    "preview",
    "plugin",
    "archive",
    "license",
    "cloud",
    "window",
];

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Options {
    /// CI 모드: 표시·사용자 자원이 필요한 항목은 자동 SKIP.
    pub ci: bool,
    pub json: bool,
    /// 한 그룹만.
    pub only: Option<String>,
    /// 사용자 클립보드를 덮어쓰는 왕복 시험 허용(opt-in).
    pub with_clipboard: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    Pass,
    Fail,
    Warn,
    Skip,
}

impl Verdict {
    fn as_str(self) -> &'static str {
        match self {
            Verdict::Pass => "PASS",
            Verdict::Fail => "FAIL",
            Verdict::Warn => "WARN",
            Verdict::Skip => "SKIP",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Item {
    pub group: &'static str,
    pub name: String,
    pub verdict: Verdict,
    pub detail: String,
    pub ms: u128,
}

#[derive(Debug, Default)]
pub(crate) struct Report {
    pub items: Vec<Item>,
}

impl Report {
    pub(crate) fn failed(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.verdict == Verdict::Fail)
            .count()
    }

    /// 종료 코드 = FAIL 수(최대 255).
    pub(crate) fn exit_code(&self) -> u8 {
        self.failed().min(255) as u8
    }

    pub(crate) fn to_table(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "nexa-dir {} selfcheck", env!("CARGO_PKG_VERSION"));
        for i in &self.items {
            let _ = writeln!(
                s,
                "{:<4} {:<10} {:<28} {:>6} ms  {}",
                i.verdict.as_str(),
                i.group,
                i.name,
                i.ms,
                i.detail
            );
        }
        let pass = self
            .items
            .iter()
            .filter(|i| i.verdict == Verdict::Pass)
            .count();
        let warn = self
            .items
            .iter()
            .filter(|i| i.verdict == Verdict::Warn)
            .count();
        let skip = self
            .items
            .iter()
            .filter(|i| i.verdict == Verdict::Skip)
            .count();
        let _ = writeln!(
            s,
            "— pass {pass} · fail {} · warn {warn} · skip {skip}",
            self.failed()
        );
        s
    }

    /// 외부 crate 없이 최소 JSON(값은 이스케이프).
    pub(crate) fn to_json(&self) -> String {
        fn esc(s: &str) -> String {
            let mut o = String::with_capacity(s.len() + 2);
            o.push('"');
            for c in s.chars() {
                match c {
                    '"' => o.push_str("\\\""),
                    '\\' => o.push_str("\\\\"),
                    '\n' => o.push_str("\\n"),
                    '\r' => o.push_str("\\r"),
                    '\t' => o.push_str("\\t"),
                    c if (c as u32) < 0x20 => {
                        let _ = write!(o, "\\u{:04x}", c as u32);
                    }
                    c => o.push(c),
                }
            }
            o.push('"');
            o
        }
        let mut s = String::from("{\"version\":");
        s.push_str(&esc(env!("CARGO_PKG_VERSION")));
        s.push_str(",\"items\":[");
        for (n, i) in self.items.iter().enumerate() {
            if n > 0 {
                s.push(',');
            }
            let _ = write!(
                s,
                "{{\"group\":{},\"name\":{},\"verdict\":{},\"detail\":{},\"ms\":{}}}",
                esc(i.group),
                esc(&i.name),
                esc(i.verdict.as_str()),
                esc(&i.detail),
                i.ms
            );
        }
        let _ = write!(s, "],\"failed\":{}}}", self.failed());
        s
    }
}

/// 점검 실행. `only`가 알 수 없는 그룹이면 항목 0개(호출자가 표로 알 수 있다).
pub(crate) fn run(opts: &Options) -> Report {
    let mut r = Report::default();
    for g in GROUPS {
        if let Some(only) = &opts.only {
            if only != g {
                continue;
            }
        }
        match *g {
            "env" => check_env(&mut r),
            other => r.items.push(Item {
                group: other,
                name: "(not implemented)".into(),
                verdict: Verdict::Skip,
                detail: "M1~M5에서 채움".into(),
                ms: 0,
            }),
        }
    }
    r
}

fn timed(r: &mut Report, group: &'static str, name: &str, f: impl FnOnce() -> (Verdict, String)) {
    let t = Instant::now();
    let (verdict, detail) = f();
    r.items.push(Item {
        group,
        name: name.to_string(),
        verdict,
        detail,
        ms: t.elapsed().as_millis(),
    });
}

fn check_env(r: &mut Report) {
    timed(r, "env", "os", || {
        (
            Verdict::Pass,
            format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        )
    });
    timed(r, "env", "version", || {
        (Verdict::Pass, env!("CARGO_PKG_VERSION").to_string())
    });
    timed(r, "env", "temp dir writable", || {
        match probe_writable(&std::env::temp_dir()) {
            Ok(p) => (Verdict::Pass, p),
            Err(e) => (Verdict::Fail, e),
        }
    });
    // 홈(설정 폴더)은 M1 `ndir-settings::config_dir()`로 교체 — 지금은 NDIR_HOME이 있으면 그것만 본다.
    timed(r, "env", "NDIR_HOME writable", || {
        match std::env::var_os("NDIR_HOME") {
            Some(h) => match probe_writable(std::path::Path::new(&h)) {
                Ok(p) => (Verdict::Pass, p),
                Err(e) => (Verdict::Fail, e),
            },
            None => (Verdict::Skip, "unset (M1: config_dir)".into()),
        }
    });
}

/// 폴더에 프로브 파일을 만들었다 지운다(PID + 시퀀스 — 병렬 실행 충돌 없음).
fn probe_writable(dir: &std::path::Path) -> Result<String, String> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEQ: AtomicU32 = AtomicU32::new(0);
    if !dir.is_dir() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("{}: mkdir failed: {e}", dir.display()))?;
    }
    let name = format!(
        ".nexa-dir-probe.{}.{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    let p = dir.join(name);
    std::fs::write(&p, b"probe").map_err(|e| format!("{}: write failed: {e}", dir.display()))?;
    let _ = std::fs::remove_file(&p);
    Ok(dir.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_group_passes_in_temp() {
        let r = run(&Options {
            only: Some("env".into()),
            ..Options::default()
        });
        assert!(r.items.iter().all(|i| i.group == "env"));
        assert_eq!(r.failed(), 0, "{}", r.to_table());
        assert!(r
            .items
            .iter()
            .any(|i| i.name == "temp dir writable" && i.verdict == Verdict::Pass));
    }

    #[test]
    fn unknown_only_yields_no_items() {
        let r = run(&Options {
            only: Some("nope".into()),
            ..Options::default()
        });
        assert!(r.items.is_empty());
        assert_eq!(r.exit_code(), 0);
    }

    #[test]
    fn all_groups_present_and_unimplemented_are_skip() {
        let r = run(&Options::default());
        for g in GROUPS {
            assert!(r.items.iter().any(|i| i.group == *g), "group {g} missing");
        }
        assert_eq!(r.failed(), 0);
    }

    #[test]
    fn json_escapes_and_counts() {
        let mut r = Report::default();
        r.items.push(Item {
            group: "env",
            name: "q\"uote".into(),
            verdict: Verdict::Fail,
            detail: "a\\b\n".into(),
            ms: 3,
        });
        let j = r.to_json();
        assert!(j.contains("\"name\":\"q\\\"uote\""));
        assert!(j.contains("\"detail\":\"a\\\\b\\n\""));
        assert!(j.ends_with("\"failed\":1}"));
        assert_eq!(r.exit_code(), 1);
    }

    #[test]
    fn probe_cleans_up() {
        let dir = std::env::temp_dir().join(format!("nexa-dir-selfcheck-{}", std::process::id()));
        probe_writable(&dir).expect("writable");
        assert!(std::fs::read_dir(&dir)
            .map(|d| d.count() == 0)
            .unwrap_or(false));
        let _ = std::fs::remove_dir(&dir);
    }
}
