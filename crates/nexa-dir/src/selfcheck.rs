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
    pub(crate) fn as_str(self) -> &'static str {
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
            "config" => check_config(&mut r),
            "resources" => check_resources(&mut r),
            "license" => check_license(&mut r),
            "shell" => check_shell(&mut r),
            "ctxmenu" => check_ctxmenu(&mut r),
            "open" => check_open(&mut r),
            "fs" => {
                check_fs(&mut r);
                check_watch(&mut r);
            }
            "trash" => check_trash(&mut r, opts.ci),
            "plugin" => check_plugin(&mut r),
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

/// 임시 폴더 꼬리표 — pid + 호출 순번(같은 프로세스에서 동시에 두 점검이 돌아도 충돌 없음 · 10-03 시험 적발: 창 점검 + CLI 점검 시험 동시 실행 시 같은 폴더를 두 쪽이 지웠다).
fn unique_tag() -> String {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!(
        "{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
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

/// config — 설정 레지스트리(기본값 전수 유효 · 곁 표 키 존재) · 설정 폴더 판정 · 격리 폴더에서 저장 → 재적재 왕복.
/// 실제 설정 폴더에는 쓰지 않는다(임시 폴더 안 PID 하위).
fn check_config(r: &mut Report) {
    timed(r, "config", "registry defaults valid", || {
        let bad: Vec<&str> = ndir_settings::REGISTRY
            .iter()
            .filter(|e| ndir_settings::normalize(e.kind, e.default).is_none())
            .map(|e| e.key)
            .collect();
        let side = ndir_settings::HIDDEN
            .iter()
            .chain(ndir_settings::ADVANCED)
            .chain(ndir_settings::INFO_KEYS)
            .filter(|k| ndir_settings::entry(k).is_none())
            .count();
        if bad.is_empty() && side == 0 {
            (
                Verdict::Pass,
                format!("{} keys", ndir_settings::REGISTRY.len()),
            )
        } else {
            (
                Verdict::Fail,
                format!("invalid defaults {bad:?} · dangling side-table keys {side}"),
            )
        }
    });
    timed(
        r,
        "config",
        "config dir",
        || match ndir_settings::config_dir() {
            Some(d) => {
                let src = if std::env::var_os(ndir_settings::ENV_HOME).is_some() {
                    "NDIR_HOME"
                } else if ndir_settings::portable_dir().is_some() {
                    "portable data/"
                } else {
                    "user config dir"
                };
                (Verdict::Pass, format!("{src}: {}", d.display()))
            }
            None => (Verdict::Fail, "none".into()),
        },
    );
    timed(r, "config", "save/reload round trip", || {
        let dir =
            std::env::temp_dir().join(format!("nexa-dir-selfcheck-cfg-{}", std::process::id()));
        let path = dir.join(ndir_settings::FILE_NAME);
        let mut s = ndir_settings::Settings::open(path.clone());
        let r1 = s
            .set("ui.theme", "light")
            .and_then(|_| s.set("scroll.fast_step", "7"));
        let saved = r1.is_ok() && s.save().is_ok();
        let back = ndir_settings::Settings::open(path);
        let ok = saved
            && back.get("ui.theme") == Some("light")
            && back.int("scroll.fast_step") == 7
            && !back.is_modified("ui.lang");
        let _ = std::fs::remove_dir_all(&dir);
        if ok {
            (Verdict::Pass, "temp dir · 2 keys".into())
        } else {
            (
                Verdict::Fail,
                format!(
                    "saved={saved} theme={:?} step={}",
                    back.get("ui.theme"),
                    back.int("scroll.fast_step")
                ),
            )
        }
    });
}

/// license — 루트 키 등재 · 기기 ID · 설치 상태(기본 자리 · 읽기만) · 요청 코드 왕복. 파일을 쓰지 않는다.
fn check_license(r: &mut Report) {
    timed(r, "license", "root keys", || {
        let n = ndir_license::ROOT_KEYS.len();
        if n == 0 {
            (
                Verdict::Fail,
                "ROOT_KEYS empty — every license file would be Invalid(NoRootKey)".into(),
            )
        } else {
            (
                Verdict::Pass,
                format!(
                    "{n} key(s): {}",
                    ndir_license::ROOT_KEYS
                        .iter()
                        .map(|k| k.id)
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            )
        }
    });
    timed(r, "license", "product", || {
        let p = &ndir_license::PRODUCT;
        if p.id == "nexa-dir" && p.version == env!("CARGO_PKG_VERSION") && p.build_date.len() == 10
        {
            (
                Verdict::Pass,
                format!("{}/{} built {}", p.id, p.version, p.build_date),
            )
        } else {
            (
                Verdict::Fail,
                format!("{p:?} vs app {}", env!("CARGO_PKG_VERSION")),
            )
        }
    });
    timed(
        r,
        "license",
        "machine id",
        || match ndir_license::Licensing::machine_code() {
            Some(c) => (Verdict::Pass, format!("{}…", &c[..c.len().min(8)])),
            None => (
                Verdict::Warn,
                "unavailable (container? request code disabled)".into(),
            ),
        },
    );
    timed(r, "license", "request code round trip", || {
        let meta = ndir_license::RequestMeta {
            name: "selfcheck".into(),
            email: String::new(),
        };
        match ndir_license::Licensing::request_code(&meta) {
            Some(code) => match ndir_license::request::decode(&code) {
                Some(r)
                    if r.meta.get("n") == Some("selfcheck")
                        && r.meta
                            .get("app")
                            .is_some_and(|a| a.starts_with("nexa-dir/")) =>
                {
                    (Verdict::Pass, format!("{} chars", code.len()))
                }
                Some(_) => (Verdict::Fail, "decoded meta mismatch".into()),
                None => (Verdict::Fail, "decode failed".into()),
            },
            None => (Verdict::Skip, "no machine id".into()),
        }
    });
    timed(r, "license", "installed state", || {
        let l = ndir_license::Licensing::open_default();
        let dirs = l
            .dirs()
            .iter()
            .map(|d| d.display().to_string())
            .collect::<Vec<_>>()
            .join(" → ");
        let state = l.state().name();
        let v = match l.state() {
            ndir_license::LicenseState::Invalid(_) => Verdict::Warn,
            _ => Verdict::Pass,
        };
        (v, format!("{state} · {dirs}"))
    });
}

/// resources — 내장 자원. M1: i18n 표(언어 3종 · 키 수 · 파리티는 빌드가 보장하므로 여기서는 적재·조회만) ·
/// 아이콘·샘플은 M2·M5에서 추가.
fn check_resources(r: &mut Report) {
    timed(r, "resources", "i18n tables", || {
        let n = ndir_i18n::KEY_COUNT;
        if n == 0 {
            return (Verdict::Fail, "key count 0".into());
        }
        let nowhere = std::path::Path::new("");
        let mut bad = Vec::new();
        for (code, _) in ndir_i18n::BUILTIN {
            let l = ndir_i18n::load(code, nowhere);
            if l.get("menu.file").is_none() || l.get("status.tab").is_none() {
                bad.push(*code);
            }
        }
        if bad.is_empty() {
            (
                Verdict::Pass,
                format!("{} langs · {n} keys", ndir_i18n::BUILTIN.len()),
            )
        } else {
            (
                Verdict::Fail,
                format!("missing core keys in {}", bad.join(",")),
            )
        }
    });
    timed(r, "resources", "os locale", || {
        let code = ndir_i18n::syslang::system_lang_code();
        let avail: Vec<(String, String)> = ndir_i18n::BUILTIN
            .iter()
            .map(|(c, n)| (c.to_string(), n.to_string()))
            .collect();
        let resolved = ndir_i18n::resolve_code(
            "system",
            ndir_i18n::syslang::system_locale().unwrap_or("?"),
            &avail,
        );
        (
            Verdict::Pass,
            format!("{} → {resolved}", if code.is_empty() { "?" } else { &code }),
        )
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

/// 휴지통 왕복(T-51 B-2c `Trash::restore`): 임시 파일 삭제 → 원래 경로로 복원(Windows 셸 undelete · Linux .trashinfo · macOS 미지원 = SKIP).
fn check_trash_round_trip(r: &mut Report, ci: bool) {
    timed(r, "trash", "trash → restore round trip", || {
        if ci {
            return (Verdict::Skip, "needs user trash (not in --ci)".into());
        }
        let p = crate::platform::Platform::native();
        let dir = std::env::temp_dir().join(format!("ndir-selfcheck-restore-{}", unique_tag()));
        let _ = std::fs::create_dir_all(&dir);
        let f = dir.join("restore-me.txt");
        if std::fs::write(&f, b"restore").is_err() {
            return (Verdict::Fail, "cannot write temp file".into());
        }
        let out = match p.trash.trash(std::slice::from_ref(&f)) {
            Ok(1) if !f.exists() => match p.trash.restore(std::slice::from_ref(&f)) {
                Ok(1) if f.exists() => (Verdict::Pass, "restored to original path".into()),
                Ok(n) => (
                    Verdict::Warn,
                    format!("restored {n} · exists {}", f.exists()),
                ),
                Err(crate::platform::PlatformError::Unsupported(_)) => {
                    (Verdict::Skip, "restore unsupported on this OS".into())
                }
                Err(e) => (Verdict::Fail, e.to_string()),
            },
            Ok(n) => (Verdict::Warn, format!("trash returned {n}")),
            Err(e) => (Verdict::Fail, e.to_string()),
        };
        let _ = std::fs::remove_dir_all(&dir);
        out
    });
}

/// 셸 탐지(T-50 `Shell` 포트 · SKEL-425): 기본 셸이 있고 실행 파일이 존재한다 · 후보 목록.
fn check_shell(r: &mut Report) {
    let p = crate::platform::Platform::native();
    timed(r, "shell", "default shell", || {
        match p.shell.default_shell() {
            Some(sh) if sh.program.is_file() => (
                Verdict::Pass,
                format!("{} ({})", sh.label, sh.program.display()),
            ),
            Some(sh) => (Verdict::Fail, format!("missing: {}", sh.program.display())),
            None => (Verdict::Fail, "no shell found".into()),
        }
    });
    timed(r, "shell", "candidates", || {
        let c = p.shell.candidates();
        (
            if c.is_empty() {
                Verdict::Warn
            } else {
                Verdict::Pass
            },
            c.iter()
                .map(|s| s.label.as_str())
                .collect::<Vec<_>>()
                .join(" · "),
        )
    });
}

/// 폴더 감시(T-51 B-2b `Watcher` 포트): 임시 폴더를 감시하고 파일 하나를 만들어 통지가 간격×4 안에 오는지(Windows 통지 · 그 외 폴링).
fn check_watch(r: &mut Report) {
    let mut p = crate::platform::Platform::native();
    let dir = std::env::temp_dir().join(format!("ndir-selfcheck-watch-{}", unique_tag()));
    let _ = std::fs::create_dir_all(&dir);
    timed(r, "fs", "folder watch notifies", || {
        p.watcher.watch(std::slice::from_ref(&dir));
        let interval = p.watcher.poll_interval_ms();
        std::thread::sleep(std::time::Duration::from_millis(30));
        let _ = p.watcher.poll();
        let _ = std::fs::write(dir.join("probe.txt"), b"x");
        let deadline = Instant::now() + std::time::Duration::from_millis(interval * 4);
        while Instant::now() < deadline {
            if p.watcher.poll().contains(&dir) {
                return (Verdict::Pass, format!("interval {interval} ms"));
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        (
            Verdict::Warn,
            format!("no notification within {} ms", interval * 4),
        )
    });
    p.watcher.watch(&[]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 셸 컨텍스트 메뉴(T-51 B `ContextMenuProvider` 포트): 임시 파일의 행 메뉴 · 임시 폴더의 배경 메뉴를 **실제로 구축**해 항목 수를 본다
/// (실행은 하지 않는다). Windows 외 OS는 자체 메뉴만이라 SKIP.
fn check_ctxmenu(r: &mut Report) {
    if !cfg!(windows) {
        r.items.push(Item {
            group: "ctxmenu",
            name: "shell menu".into(),
            verdict: Verdict::Skip,
            detail: "app menu only on this OS".into(),
            ms: 0,
        });
        return;
    }
    let p = crate::platform::Platform::native();
    let dir = std::env::temp_dir().join(format!("ndir-selfcheck-ctx-{}", unique_tag()));
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join("probe.txt");
    let _ = std::fs::write(&file, b"x");
    timed(r, "ctxmenu", "row menu items", || {
        match p.ctxmenu.items(std::slice::from_ref(&file)) {
            Ok(v) if !v.is_empty() => {
                let verbs = v
                    .iter()
                    .filter(|i| !i.verb.is_empty())
                    .map(|i| i.verb.to_ascii_lowercase())
                    .collect::<Vec<_>>();
                let icons = v.iter().filter(|i| i.icon.is_some()).count();
                (
                    Verdict::Pass,
                    format!(
                        "{} items · {icons} icons · verbs {}",
                        v.len(),
                        verbs.join(",")
                    ),
                )
            }
            Ok(_) => (Verdict::Warn, "no items".into()),
            Err(e) => (Verdict::Fail, e.to_string()),
        }
    });
    // 우클릭 가속(dir2 X-61): 선행 구축된 같은 대상의 재조회는 캐시에서 즉시(구축 시간 = 위 항목의 ms).
    timed(r, "ctxmenu", "prepared menu is instant", || {
        let target = crate::platform::MenuTarget::Rows(vec![file.clone()]);
        p.ctxmenu.prepare(&target);
        let t0 = std::time::Instant::now();
        let mut got = None;
        while t0.elapsed() < std::time::Duration::from_secs(30) {
            while p.ctxmenu.poll().is_some() {}
            if !p.ctxmenu.busy() {
                let t1 = std::time::Instant::now();
                got = p
                    .ctxmenu
                    .try_items(&target)
                    .map(|v| (v.len(), t1.elapsed()));
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        match got {
            Some((n, d)) if d < std::time::Duration::from_millis(50) => (
                Verdict::Pass,
                format!(
                    "{n} items in {} µs after {} ms prebuild",
                    d.as_micros(),
                    t0.elapsed().as_millis()
                ),
            ),
            Some((n, d)) => (
                Verdict::Warn,
                format!("{n} items but lookup took {} ms", d.as_millis()),
            ),
            None => (Verdict::Fail, "prebuild did not finish".into()),
        }
    });
    timed(r, "ctxmenu", "background menu items", || {
        match p.ctxmenu.bg_items(&dir) {
            Ok(v) if !v.is_empty() => (
                Verdict::Pass,
                format!(
                    "{} items · {} submenus",
                    v.len(),
                    v.iter().filter(|i| !i.children.is_empty()).count()
                ),
            ),
            Ok(_) => (Verdict::Warn, "no items".into()),
            Err(e) => (Verdict::Fail, e.to_string()),
        }
    });
    let _ = std::fs::remove_dir_all(&dir);
}

/// 열기 수단(T-50 `Opener` 포트): 실행은 하지 않고 명령 존재만(CI 안전).
fn check_open(r: &mut Report) {
    timed(r, "open", "opener command", || {
        let name = if cfg!(windows) {
            "cmd.exe"
        } else if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        let found = if cfg!(windows) {
            std::env::var_os("SystemRoot")
                .map(std::path::PathBuf::from)
                .map(|r| r.join("System32").join("cmd.exe"))
                .is_some_and(|p| p.is_file())
                || crate::platform::find_in_path(name).is_some()
        } else {
            crate::platform::find_in_path(name).is_some()
        };
        if found {
            (Verdict::Pass, name.into())
        } else {
            (Verdict::Warn, format!("{name} not in PATH"))
        }
    });
}

/// 파일 시스템(샌드박스 임시 폴더 안에서만 · docs/18 §6 fs): 만들기·복사·이동·이름 바꾸기·삭제·유니코드 · 드라이브 용량(`Disk` 포트).
/// 플러그인(T-62 · port/20 §7 T-27 "상시 점검"): 탐색 경로 · 동봉 2종 로드 + `nx_meta` id 일치 · 로드 오류 0건.
fn check_plugin(r: &mut Report) {
    timed(r, "plugin", "search paths", || {
        let dirs = crate::preview::plugin_dirs();
        let found: Vec<String> = dirs
            .iter()
            .filter(|d| d.is_dir())
            .map(|d| d.display().to_string())
            .collect();
        if found.is_empty() {
            (
                Verdict::Warn,
                format!("no plugins folder present ({} candidates)", dirs.len()),
            )
        } else {
            (Verdict::Pass, found.join(" · "))
        }
    });
    timed(r, "plugin", "bundled markdown · archive-sample", || {
        let infos = crate::preview::plugin_infos();
        let has = |id: &str| infos.iter().any(|i| i.id == id);
        let ids: Vec<&str> = infos.iter().map(|i| i.id.as_str()).collect();
        if has("markdown") && has("archive-sample") {
            (Verdict::Pass, ids.join(", "))
        } else if infos.is_empty() {
            (
                Verdict::Warn,
                "no plugins loaded (bundled set missing?)".into(),
            )
        } else {
            (
                Verdict::Warn,
                format!("bundled ids missing — loaded: {}", ids.join(", ")),
            )
        }
    });
    timed(r, "plugin", "load errors", || {
        let notes = crate::preview::load_notes();
        if notes.is_empty() {
            (Verdict::Pass, "0".into())
        } else {
            (Verdict::Fail, notes.join(" | "))
        }
    });
}

fn check_fs(r: &mut Report) {
    let base = std::env::temp_dir().join(format!("nexa-dir-selfcheck-fs-{}", unique_tag()));
    let _ = std::fs::remove_dir_all(&base);
    timed(r, "fs", "create · copy · rename · delete", || {
        let run = || -> std::io::Result<String> {
            std::fs::create_dir_all(base.join("한글 폴더"))?;
            std::fs::write(base.join("한글 폴더").join("a.txt"), b"nexa")?;
            std::fs::copy(base.join("한글 폴더").join("a.txt"), base.join("b.txt"))?;
            std::fs::rename(base.join("b.txt"), base.join("c.txt"))?;
            let n = std::fs::read(base.join("c.txt"))?.len();
            std::fs::remove_file(base.join("c.txt"))?;
            std::fs::remove_dir_all(base.join("한글 폴더"))?;
            Ok(format!("{n} bytes round trip"))
        };
        let out = run();
        let _ = std::fs::remove_dir_all(&base);
        match out {
            Ok(d) => (Verdict::Pass, d),
            Err(e) => (Verdict::Fail, e.to_string()),
        }
    });
    timed(r, "fs", "drive space (temp)", || {
        let p = crate::platform::Platform::native();
        match p.disk.space(&std::env::temp_dir()) {
            Some((total, free)) => (
                Verdict::Pass,
                format!(
                    "{} free / {}",
                    crate::filelist::format_size(free),
                    crate::filelist::format_size(total)
                ),
            ),
            None => (Verdict::Skip, "unsupported on this OS (T-52/53)".into()),
        }
    });
}

/// 휴지통(T-51 `Trash` 포트): 샌드박스 임시 파일 하나를 실제 휴지통으로(사용자 휴지통에 1개 남는다 → `--ci`는 SKIP).
fn check_trash(r: &mut Report, ci: bool) {
    check_trash_round_trip(r, ci);
    timed(r, "trash", "trash one temp file", || {
        if ci {
            return (Verdict::Skip, "needs user trash (not in --ci)".into());
        }
        let dir = std::env::temp_dir().join(format!("nexa-dir-selfcheck-trash-{}", unique_tag()));
        let _ = std::fs::create_dir_all(&dir);
        let f = dir.join("nexa-dir-selfcheck.txt");
        if let Err(e) = std::fs::write(&f, b"nexa-dir selfcheck") {
            return (Verdict::Fail, e.to_string());
        }
        let p = crate::platform::Platform::native();
        let out = match p.trash.trash(std::slice::from_ref(&f)) {
            Ok(1) if !f.exists() => (Verdict::Pass, "moved".into()),
            Ok(n) => (Verdict::Warn, format!("moved {n} but still exists")),
            Err(crate::platform::PlatformError::Unsupported(w)) => {
                (Verdict::Skip, format!("unsupported: {w}"))
            }
            Err(e) => (Verdict::Fail, e.to_string()),
        };
        let _ = std::fs::remove_dir_all(&dir);
        out
    });
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
    fn resources_group_loads_i18n() {
        let r = run(&Options {
            only: Some("resources".into()),
            ..Options::default()
        });
        assert_eq!(r.failed(), 0, "{}", r.to_table());
        assert!(r
            .items
            .iter()
            .any(|i| i.name == "i18n tables" && i.detail.contains("3 langs")));
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

    /// CI 부분집합으로 돈다 — 기본 옵션은 실제 휴지통 왕복·셸 메뉴 COM을 건드려 시험 규율 위반이고, 다른 시험(셸 메뉴·check_win)과 겹치면
    /// 멈췄다(10-03 로컬 실증 · 병렬 실행에서만 재현).
    #[test]
    fn all_groups_present_and_unimplemented_are_skip() {
        let _g = crate::platform::os_test_guard();
        let r = run(&Options {
            ci: true,
            ..Default::default()
        });
        for g in GROUPS {
            assert!(r.items.iter().any(|i| i.group == *g), "group {g} missing");
        }
        let fails: Vec<&Item> = r
            .items
            .iter()
            .filter(|i| i.verdict == Verdict::Fail)
            .collect();
        assert!(fails.is_empty(), "{fails:?}");
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
