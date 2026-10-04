//! ndir-check — T4 시나리오 러너(docs/18 §4 T4 · docs/port/44 CI-108 · T-06). **의존 0**.
//!
//! `tests/scenarios/*.scn`(줄 단위 `키: 값` · `#` 주석)을 읽어 격리 홈 + 샘플 트리를 만들고 `nexa-dir`를 `NDIR_HOME` · `NDIR_NO_ACTIVATE=1` ·
//! `NDIR_STARTUP_CMD`로 띄운 뒤 **종료 코드 · stderr 패닉 · 검사식**으로 판정한다. 고정 sleep 없음 — 시나리오가 `@ready … assert … quit`로 스스로 끝난다.
//!
//! ```text
//! id: nav-up-select            # 필수 · 출력 폴더 이름
//! title: 위로 = 떠난 폴더 선택
//! tree: dir sub/deep           # 샘플 트리(여러 줄) — dir <경로> | file <경로> [<바이트>]
//! tree: file a.txt 5
//! tree: text d.md # 제목\n본문  # 내용 있는 텍스트 파일(두 글자 `\n` = 줄바꿈)
//! settings: ui.lang=en         # 격리 홈 settings.conf 한 줄(여러 줄)
//! cmd: @ready:nav:<root>/sub   # NDIR_STARTUP_CMD 항목(여러 줄 · 자리표 <root> <home> <out>)
//! cmd: @ready:quit
//! exit: 0                      # 기대 종료 코드(기본 0)
//! timeout: 20                  # 초(기본 20)
//! check: <out>/panel.txt: caret Some(     # <파일>: <부분 문자열> · 앞에 `!` = 없어야 · 파일 `stderr` = 앱 stderr
//! settings@windows: launcher.items=Shell|cmd.exe|/c exit   # 키 뒤 `@<os>` = 그 OS에서만 읽는 줄(T-117)
//! settings@unix: launcher.items=Shell|/bin/sh|-c exit      # os = windows | linux | macos | unix(linux + macos)
//! ```
//!
//! 사용: `ndir-check [--bin <nexa-dir 경로>] [--out <폴더>] [--filter <부분>] [--ci] [<.scn 파일|폴더>…]`
//! 결과 = 표 + `<out>/summary.txt` · 종료 코드 = 실패 수.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
enum TreeItem {
    Dir(String),
    File(String, usize),
    /// 내용이 있는 텍스트 파일(`\n` = 줄바꿈 · 미리보기/플러그인 시나리오).
    Text(String, String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Check {
    file: String,
    needle: String,
    negate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Scenario {
    id: String,
    title: String,
    tree: Vec<TreeItem>,
    settings: Vec<String>,
    cmds: Vec<String>,
    checks: Vec<Check>,
    exit: i32,
    timeout: Duration,
}

/// 이 실행기가 도는 OS 이름(`@<os>` 줄 판정용).
fn host_os() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

/// `@<os>` 꼬리표가 이 OS에 해당하는가(순수): windows · linux · macos · unix(= linux 또는 macos). 모르는 꼬리표 = `None`(오류).
fn os_matches(tag: &str, os: &str) -> Option<bool> {
    match tag {
        "windows" | "linux" | "macos" => Some(tag == os),
        "unix" => Some(os != "windows"),
        _ => None,
    }
}

/// `.scn` 본문 → 시나리오(이 OS 기준).
fn parse(text: &str) -> Result<Scenario, String> {
    parse_for(text, host_os())
}

/// `.scn` 본문 → 시나리오(순수 · 시험). `id`가 없으면 오류. 키 뒤 `@<os>`가 붙은 줄은 `os`가 맞을 때만 읽는다.
fn parse_for(text: &str, os: &str) -> Result<Scenario, String> {
    let mut s = Scenario {
        id: String::new(),
        title: String::new(),
        tree: Vec::new(),
        settings: Vec::new(),
        cmds: Vec::new(),
        checks: Vec::new(),
        exit: 0,
        timeout: Duration::from_secs(20),
    };
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        // 주석 = `#`로 시작하는 줄만(값 안의 `#`는 보존).
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let (k, v) = line
            .split_once(':')
            .ok_or_else(|| format!("line {}: expected `key: value`", n + 1))?;
        let v = v.trim();
        // `키@os` = 그 OS에서만(다른 OS = 줄을 건너뛴다 · T-117: Windows 전제 줄과 Unix 대응 줄을 한 파일에).
        let k = match k.trim().split_once('@') {
            Some((key, tag)) => match os_matches(tag.trim(), os) {
                Some(true) => key.trim(),
                Some(false) => continue,
                None => return Err(format!("line {}: unknown os tag `@{}`", n + 1, tag.trim())),
            },
            None => k.trim(),
        };
        match k {
            "id" => s.id = v.to_string(),
            "title" => s.title = v.to_string(),
            "tree" => {
                let mut it = v.split_whitespace();
                match (it.next(), it.next(), it.next()) {
                    (Some("text"), Some(p), _) => {
                        // `text <경로> <내용>` — 내용은 경로 뒤 전부(두 글자 `\n` = 줄바꿈).
                        let body = v.trim_start()["text".len()..].trim_start()[p.len()..]
                            .trim_start()
                            .replace("\\n", "\n");
                        s.tree.push(TreeItem::Text(p.to_string(), body));
                    }
                    (Some("dir"), Some(p), _) => s.tree.push(TreeItem::Dir(p.to_string())),
                    (Some("file"), Some(p), size) => s.tree.push(TreeItem::File(
                        p.to_string(),
                        size.and_then(|z| z.parse().ok()).unwrap_or(0),
                    )),
                    _ => {
                        return Err(format!(
                        "line {}: tree = dir <path> | file <path> [bytes] | text <path> <content>",
                        n + 1
                    ))
                    }
                }
            }
            "settings" => s.settings.push(v.to_string()),
            "cmd" => s.cmds.push(v.to_string()),
            "exit" => {
                s.exit = v
                    .parse()
                    .map_err(|_| format!("line {}: exit = int", n + 1))?
            }
            "timeout" => {
                s.timeout = Duration::from_secs(
                    v.parse()
                        .map_err(|_| format!("line {}: timeout = secs", n + 1))?,
                );
            }
            "check" => {
                let (file, rest) = v
                    .split_once(": ")
                    .ok_or_else(|| format!("line {}: check = <file>: <needle>", n + 1))?;
                let (negate, needle) = rest.strip_prefix('!').map_or((false, rest), |r| (true, r));
                s.checks.push(Check {
                    file: file.trim().to_string(),
                    needle: needle.to_string(),
                    negate,
                });
            }
            other => return Err(format!("line {}: unknown key `{other}`", n + 1)),
        }
    }
    if s.id.is_empty() {
        return Err("missing `id:`".into());
    }
    Ok(s)
}

/// 자리표 치환 — `<root>` 샘플 트리 · `<home>` 격리 홈 · `<out>` 출력 폴더(경로는 OS 표기 그대로).
fn substitute(s: &str, root: &Path, home: &Path, out: &Path) -> String {
    s.replace("<root>", &root.display().to_string())
        .replace("<home>", &home.display().to_string())
        .replace("<out>", &out.display().to_string())
}

/// 검사식 하나 — `file` 본문(`stderr` = 앱 stderr)에 `needle`이 있는가(`negate` = 없어야).
fn evaluate(
    check: &Check,
    stderr: &str,
    read: &dyn Fn(&str) -> Option<String>,
) -> Result<(), String> {
    let body = if check.file == "stderr" {
        stderr.to_string()
    } else {
        read(&check.file).ok_or_else(|| format!("{}: file missing", check.file))?
    };
    let hit = body.contains(&check.needle);
    if hit == check.negate {
        return Err(format!(
            "{}: {}`{}`\n{}",
            check.file,
            if check.negate { "found " } else { "missing " },
            check.needle,
            body.lines().take(12).collect::<Vec<_>>().join("\n")
        ));
    }
    Ok(())
}

/// 샘플 트리 생성(부모 폴더 자동).
fn build_tree(root: &Path, items: &[TreeItem]) -> std::io::Result<()> {
    std::fs::create_dir_all(root)?;
    for it in items {
        match it {
            TreeItem::Dir(p) => std::fs::create_dir_all(root.join(p))?,
            TreeItem::File(p, size) => {
                let path = root.join(p);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(path, "x".repeat(*size))?;
            }
            TreeItem::Text(p, body) => {
                let path = root.join(p);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(path, body)?;
            }
        }
    }
    Ok(())
}

struct Outcome {
    ok: bool,
    exit: Option<i32>,
    elapsed: Duration,
    failures: Vec<String>,
}

/// 시나리오 하나 실행 — 격리 홈·샘플 트리·출력 폴더는 `out_root/<id>/{home,root,out}`.
fn run(scn: &Scenario, bin: &Path, out_root: &Path) -> Outcome {
    let base = out_root.join(&scn.id);
    let _ = std::fs::remove_dir_all(&base);
    let (home, root, out) = (base.join("home"), base.join("root"), base.join("out"));
    let mut failures = Vec::new();
    let started = Instant::now();
    if let Err(e) = std::fs::create_dir_all(&home)
        .and_then(|()| std::fs::create_dir_all(&out))
        .and_then(|()| build_tree(&root, &scn.tree))
    {
        failures.push(format!("setup: {e}"));
        return Outcome {
            ok: false,
            exit: None,
            elapsed: started.elapsed(),
            failures,
        };
    }
    let mut conf = String::from("_schema=1\n");
    for line in &scn.settings {
        conf.push_str(&substitute(line, &root, &home, &out));
        conf.push('\n');
    }
    let _ = std::fs::write(home.join("settings.conf"), conf);
    let cmds: Vec<String> = scn
        .cmds
        .iter()
        .map(|c| substitute(c, &root, &home, &out))
        .collect();
    let child = Command::new(bin)
        .current_dir(&root)
        .env("NDIR_HOME", &home)
        .env("NDIR_NO_ACTIVATE", "1")
        // 시나리오의 복사/잘라내기가 사용자의 실제 OS 클립보드를 덮어쓰지 않게(프로세스 안 가짜 — CLAUDE.md §5).
        .env("NDIR_FAKE_CLIPBOARD", "1")
        .env("NDIR_PLUGINS_DIR", plugins_dir())
        .env("NDIR_STARTUP_CMD", cmds.join(","))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            failures.push(format!("spawn {}: {e}", bin.display()));
            return Outcome {
                ok: false,
                exit: None,
                elapsed: started.elapsed(),
                failures,
            };
        }
    };
    // stderr는 별 스레드로 비운다(파이프가 차면 앱이 멈춘다).
    let stderr_pipe = child.stderr.take();
    let reader = std::thread::spawn(move || {
        use std::io::Read as _;
        let mut buf = String::new();
        if let Some(mut p) = stderr_pipe {
            let _ = p.read_to_string(&mut buf);
        }
        buf
    });
    let mut exit = None;
    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(st)) => {
                exit = st.code();
                break;
            }
            Ok(None) => {
                if started.elapsed() > scn.timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    timed_out = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(e) => {
                failures.push(format!("wait: {e}"));
                break;
            }
        }
    }
    let stderr = reader.join().unwrap_or_default();
    let _ = std::fs::write(out.join("stderr.txt"), &stderr);
    if timed_out {
        failures.push(format!("timeout after {:?}", scn.timeout));
    }
    if stderr.contains("panicked at") {
        failures.push("stderr: panic".into());
    }
    if !timed_out && exit != Some(scn.exit) {
        failures.push(format!("exit {exit:?} (expected {})", scn.exit));
    }
    for c in &scn.checks {
        let resolved = Check {
            file: substitute(&c.file, &root, &home, &out),
            needle: substitute(&c.needle, &root, &home, &out),
            negate: c.negate,
        };
        let read = |f: &str| {
            // 상대 경로 = 출력 폴더 기준.
            let p = Path::new(f);
            let p = if p.is_absolute() {
                p.to_path_buf()
            } else {
                out.join(p)
            };
            std::fs::read_to_string(p).ok()
        };
        if let Err(e) = evaluate(&resolved, &stderr, &read) {
            failures.push(e);
        }
    }
    Outcome {
        ok: failures.is_empty(),
        exit,
        elapsed: started.elapsed(),
        failures,
    }
}

/// `nexa-dir` 실행 파일 — 같은 폴더(`target/<profile>/`)의 형제 · 없으면 PATH 이름.
/// 저장소 동봉 플러그인 폴더(`plugins/` — T-62 시나리오가 markdown.wasm을 쓴다).
fn plugins_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins")
}

fn default_bin() -> PathBuf {
    let name = format!("nexa-dir{}", std::env::consts::EXE_SUFFIX);
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(&name)))
        .filter(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from(name))
}

/// 인자 폴더/파일 → `.scn` 목록(정렬).
fn collect(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for p in paths {
        if p.is_dir() {
            if let Ok(rd) = std::fs::read_dir(p) {
                let mut v: Vec<PathBuf> = rd
                    .filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|f| f.extension().is_some_and(|x| x == "scn"))
                    .collect();
                v.sort();
                out.extend(v);
            }
        } else {
            out.push(p.clone());
        }
    }
    out
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut bin: Option<PathBuf> = None;
    let mut out_root = PathBuf::from("tests/out");
    let mut filter: Option<String> = None;
    let mut inputs: Vec<PathBuf> = Vec::new();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--bin" => bin = args.next().map(PathBuf::from),
            "--out" => {
                if let Some(o) = args.next() {
                    out_root = PathBuf::from(o);
                }
            }
            "--filter" => filter = args.next(),
            "--ci" => {}
            "--help" | "-h" => {
                println!("ndir-check [--bin <nexa-dir>] [--out <dir>] [--filter <substr>] [--ci] [<.scn|dir>…]");
                return ExitCode::SUCCESS;
            }
            other => inputs.push(PathBuf::from(other)),
        }
    }
    if inputs.is_empty() {
        inputs.push(PathBuf::from("tests/scenarios"));
    }
    let bin = bin.unwrap_or_else(default_bin);
    // 앱의 cwd는 샘플 트리다 → 홈·출력·자리표는 절대 경로여야 한다(10-03 1차 실행: 상대 `tests/out`가 트리 밑으로 갔다).
    let out_root = std::env::current_dir().map_or(out_root.clone(), |d| d.join(&out_root));
    // 구분자를 OS 표기로 통일(Windows `tests/out\x` 혼용 → 앱이 돌려주는 경로 `tests\out\x`와 검사식이 어긋났다 · 10-03).
    let out_root = PathBuf::from(
        out_root
            .to_string_lossy()
            .replace(['/', '\\'], std::path::MAIN_SEPARATOR_STR),
    );
    let files = collect(&inputs);
    let mut rows = Vec::new();
    let mut fails = 0u32;
    for f in files {
        let text = match std::fs::read_to_string(&f) {
            Ok(t) => t,
            Err(e) => {
                rows.push(format!("FAIL {} — read: {e}", f.display()));
                fails += 1;
                continue;
            }
        };
        let scn = match parse(&text) {
            Ok(s) => s,
            Err(e) => {
                rows.push(format!("FAIL {} — parse: {e}", f.display()));
                fails += 1;
                continue;
            }
        };
        if filter
            .as_ref()
            .is_some_and(|q| !scn.id.contains(q.as_str()))
        {
            continue;
        }
        let o = run(&scn, &bin, &out_root);
        let mark = if o.ok { "PASS" } else { "FAIL" };
        rows.push(format!(
            "{mark} {:<24} {:>5} ms  exit {:<4} {}",
            scn.id,
            o.elapsed.as_millis(),
            o.exit.map_or("-".to_string(), |c| c.to_string()),
            scn.title
        ));
        for e in &o.failures {
            for (i, line) in e.lines().enumerate() {
                rows.push(format!("      {}{line}", if i == 0 { "· " } else { "  " }));
            }
        }
        if !o.ok {
            fails += 1;
        }
    }
    let summary = format!(
        "{}\n— {} scenario(s) · {} failed\n",
        rows.join("\n"),
        rows.iter()
            .filter(|r| r.starts_with("PASS") || r.starts_with("FAIL"))
            .count(),
        fails
    );
    print!("{summary}");
    let _ = std::fs::create_dir_all(&out_root);
    let _ = std::fs::write(out_root.join("summary.txt"), &summary);
    ExitCode::from(fails.min(255) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCN: &str = "# 주석\nid: demo\ntitle: 제목 # 값 안 샵\ntree: dir sub/deep\ntree: file a.txt 5\ntree: file b.md\nsettings: ui.lang=en\ncmd: @ready:nav:<root>/sub\ncmd: @ready:quit:2\nexit: 2\ntimeout: 7\ncheck: <out>/p.txt: path <root>\ncheck: stderr: !panicked\n";

    #[test]
    fn parse_all_keys() {
        let s = parse(SCN).unwrap();
        assert_eq!(s.id, "demo");
        assert_eq!(s.title, "제목 # 값 안 샵");
        assert_eq!(
            s.tree,
            vec![
                TreeItem::Dir("sub/deep".into()),
                TreeItem::File("a.txt".into(), 5),
                TreeItem::File("b.md".into(), 0)
            ]
        );
        assert_eq!(s.settings, vec!["ui.lang=en"]);
        assert_eq!(s.cmds.len(), 2);
        assert_eq!(s.exit, 2);
        assert_eq!(s.timeout, Duration::from_secs(7));
        assert_eq!(
            s.checks[1],
            Check {
                file: "stderr".into(),
                needle: "panicked".into(),
                negate: true
            }
        );
        assert!(parse("title: x\n").unwrap_err().contains("id"));
        assert!(parse("id: a\nbogus: 1\n").unwrap_err().contains("bogus"));
        // OS 꼬리표(T-117): 맞는 OS의 줄만 읽는다 · unix = linux + macos · 모르는 꼬리표 = 오류.
        assert_eq!(os_matches("windows", "windows"), Some(true));
        assert_eq!(os_matches("windows", "linux"), Some(false));
        assert_eq!(os_matches("unix", "linux"), Some(true));
        assert_eq!(os_matches("unix", "macos"), Some(true));
        assert_eq!(os_matches("unix", "windows"), Some(false));
        assert_eq!(os_matches("macos", "linux"), Some(false));
        assert_eq!(os_matches("bsd", "linux"), None);
        let src = "id: a\nsettings@windows: k=w\nsettings@unix: k=u\nsettings: z=1\ncheck@linux: f.txt: only-linux\n";
        let w = parse_for(src, "windows").unwrap();
        assert_eq!(w.settings, ["k=w", "z=1"]);
        assert!(w.checks.is_empty());
        let l = parse_for(src, "linux").unwrap();
        assert_eq!(l.settings, ["k=u", "z=1"]);
        assert_eq!(l.checks.len(), 1);
        let m = parse_for(src, "macos").unwrap();
        assert_eq!(m.settings, ["k=u", "z=1"]);
        assert!(m.checks.is_empty());
        assert!(parse_for("id: a\ncmd@bsd: quit\n", "linux")
            .unwrap_err()
            .contains("@bsd"));
        assert!(parse("id: a\ntree: link x\n").is_err());
    }

    #[test]
    fn substitute_and_evaluate() {
        let (r, h, o) = (Path::new("R"), Path::new("H"), Path::new("O"));
        assert_eq!(substitute("<root>/a <home> <out>", r, h, o), "R/a H O");
        let read = |f: &str| (f == "p.txt").then(|| "path R\ncaret Some(0)".to_string());
        let ok = Check {
            file: "p.txt".into(),
            needle: "caret Some(".into(),
            negate: false,
        };
        assert!(evaluate(&ok, "", &read).is_ok());
        let neg = Check {
            file: "p.txt".into(),
            needle: "caret None".into(),
            negate: true,
        };
        assert!(evaluate(&neg, "", &read).is_ok());
        let miss = Check {
            file: "p.txt".into(),
            needle: "zzz".into(),
            negate: false,
        };
        assert!(evaluate(&miss, "", &read)
            .unwrap_err()
            .contains("missing `zzz`"));
        let nofile = Check {
            file: "q.txt".into(),
            needle: "x".into(),
            negate: false,
        };
        assert!(evaluate(&nofile, "", &read)
            .unwrap_err()
            .contains("file missing"));
        let err = Check {
            file: "stderr".into(),
            needle: "[assert] FAIL".into(),
            negate: false,
        };
        assert!(evaluate(&err, "[assert] FAIL status:x", &read).is_ok());
    }

    #[test]
    fn build_tree_creates_parents_and_sizes() {
        let root = std::env::temp_dir().join(format!("ndir-check-tree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        build_tree(&root, &parse(SCN).unwrap().tree).unwrap();
        assert!(root.join("sub/deep").is_dir());
        assert_eq!(std::fs::read(root.join("a.txt")).unwrap().len(), 5);
        assert!(root.join("b.md").is_file());
        let _ = std::fs::remove_dir_all(&root);
    }
}
