//! nexa-copy 프로세스 시험(T4급 · 사용자 10-24 "테스트 자동화"): 실제 실행 파일을 띄워 종료 코드 · stderr 요약 · 파일 결과로 판정한다.
//! 취소는 Ctrl+C 대신 시험용 환경 변수 `NEXA_COPY_CANCEL_AFTER_MS`(같은 취소 깃발 · 핸들러만 생략)로 — 콘솔 신호를 OS마다 흉내 내지 않는다.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nexa-copy"))
}

fn sandbox(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("nexa-copy-it-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).expect("mkdir");
    d
}

struct Run {
    code: i32,
    out: String,
    err: String,
}

fn run(args: &[&Path], extra: &[&str], env: &[(&str, &str)]) -> Run {
    let mut c = Command::new(exe());
    c.args(extra);
    c.args(args);
    for (k, v) in env {
        c.env(k, v);
    }
    let o = c.output().expect("spawn nexa-copy");
    Run {
        code: o.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&o.stdout).into_owned(),
        err: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

/// stdin에 글을 넣어 실행(`--ask` 프롬프트 답).
fn run_stdin(args: &[&Path], extra: &[&str], input: &str) -> Run {
    use std::io::Write as _;
    let mut c = Command::new(exe());
    c.args(extra);
    c.args(args);
    c.stdin(std::process::Stdio::piped());
    c.stdout(std::process::Stdio::piped());
    c.stderr(std::process::Stdio::piped());
    let mut child = c.spawn().expect("spawn nexa-copy");
    if let Some(mut si) = child.stdin.take() {
        let _ = si.write_all(input.as_bytes());
    }
    let o = child.wait_with_output().expect("wait");
    Run {
        code: o.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&o.stdout).into_owned(),
        err: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

fn make_tree(src: &Path) {
    fs::create_dir_all(src.join("tree").join("deep")).expect("mkdir");
    for i in 0..40 {
        fs::write(
            src.join("tree").join(format!("f{i}.txt")),
            format!("file {i}\n").repeat(50),
        )
        .expect("write");
    }
    fs::write(
        src.join("tree").join("deep").join("inner.bin"),
        vec![7u8; 300_000],
    )
    .expect("write");
    fs::write(src.join("one.txt"), b"one").expect("write");
}

fn same_tree(a: &Path, b: &Path) {
    for e in fs::read_dir(a).expect("read_dir") {
        let e = e.expect("entry");
        let other = b.join(e.file_name());
        if e.file_type().expect("ft").is_dir() {
            assert!(other.is_dir(), "{}", other.display());
            same_tree(&e.path(), &other);
        } else {
            assert_eq!(
                fs::read(e.path()).expect("read"),
                fs::read(&other).expect("read"),
                "{}",
                other.display()
            );
        }
    }
}

/// 도움말 · 버전 = stdout · 종료 0 · 인자 오류 = stderr 사용법 · 2.
#[test]
fn help_version_and_usage_errors() {
    let h = run(&[], &["--help"], &[]);
    assert_eq!(h.code, 0, "{}", h.err);
    assert!(h.out.contains("Usage: nexa-copy") && h.out.contains("--overwrite"));
    let v = run(&[], &["--version"], &[]);
    assert_eq!(v.code, 0);
    assert!(v.out.starts_with("nexa-copy "));
    let bad = run(&[], &["--bogus", "a", "b"], &[]);
    assert_eq!(bad.code, 2);
    assert!(bad.err.contains("unknown option") && bad.err.contains("Usage"));
    let one = run(&[], &["only-one"], &[]);
    assert_eq!(one.code, 2);
}

/// 복사(폴더 + 파일 · 내용 같음 · 요약) → 재실행 = 건너뜀(0) → `--overwrite` = 바뀜 → `--move` = 원본 사라짐 → 없는 원본/대상 = 2.
#[test]
fn copy_skip_overwrite_move_and_errors() {
    let d = sandbox("flow");
    let (src, dst) = (d.join("src"), d.join("dst"));
    make_tree(&src);
    fs::create_dir_all(&dst).expect("mkdir");
    let r = run(&[&src.join("tree"), &src.join("one.txt"), &dst], &[], &[]);
    assert_eq!(r.code, 0, "{}", r.err);
    same_tree(&src.join("tree"), &dst.join("tree"));
    assert_eq!(fs::read(dst.join("one.txt")).expect("read"), b"one");
    assert!(
        r.err.contains("2 item(s)") && r.err.contains("2 transferred"),
        "{}",
        r.err
    );
    assert!(r.err.contains("(100%)"), "마지막 진행 줄: {}", r.err);
    // 건너뜀(기본) · 옮긴 바이트 0 · `--verify`는 건너뛴 항목을 대조하지 않음을 알린다(협업 점검 10-25).
    fs::write(src.join("one.txt"), b"ONE").expect("write");
    let r = run(&[&src.join("one.txt"), &dst], &["--verify"], &[]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(fs::read(dst.join("one.txt")).expect("read"), b"one");
    assert!(
        r.err.contains("1 skipped") && r.err.contains("0 B of"),
        "{}",
        r.err
    );
    assert!(
        r.err.contains(
            "verified 0 file(s) · 0 B read · 0 mismatch(es) · 1 skipped item(s) not verified"
        ),
        "{}",
        r.err
    );
    // 덮어쓰기 · 조용히 = 출력 없음.
    let r = run(&[&src.join("one.txt"), &dst], &["--overwrite", "-q"], &[]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(fs::read(dst.join("one.txt")).expect("read"), b"ONE");
    assert!(r.err.is_empty(), "quiet: {}", r.err);
    // 이동 · `--verify --move` = 무시 안내 한 줄.
    let moved = d.join("moved");
    fs::create_dir_all(&moved).expect("mkdir");
    let r = run(
        &[&src.join("one.txt"), &moved],
        &["--move", "--verify"],
        &[],
    );
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(!src.join("one.txt").exists());
    assert_eq!(fs::read(moved.join("one.txt")).expect("read"), b"ONE");
    assert!(
        r.err.contains("note: --verify is ignored with --move"),
        "{}",
        r.err
    );
    // 전략 옵션(루프 · 순차) + 검증(`--verify` · 크기 + SHA-256)도 같은 결과.
    let seq = d.join("seq");
    fs::create_dir_all(&seq).expect("mkdir");
    let r = run(
        &[&src.join("tree"), &seq],
        &["--no-native", "--threads", "1", "--verify"],
        &[],
    );
    assert_eq!(r.code, 0, "{}", r.err);
    same_tree(&src.join("tree"), &seq.join("tree"));
    assert!(
        r.err.contains("verified 41 file(s)") && r.err.contains("0 mismatch(es)"),
        "{}",
        r.err
    );
    // 오류 = 2.
    let r = run(&[&src.join("nope"), &dst], &[], &[]);
    assert_eq!(r.code, 2);
    assert!(r.err.contains("source not found"));
    let r = run(&[&src.join("tree"), &d.join("no-such-dir")], &[], &[]);
    assert_eq!(r.code, 2);
    assert!(r.err.contains("destination is not an existing folder"));
    let _ = fs::remove_dir_all(&d);
}

/// `--ask`(10-25): 충돌마다 stderr 프롬프트 + stdin 한 줄 — y 덮어쓰기 · n 건너뜀 · a 전부 덮어쓰기 · c 취소(130) · EOF = 건너뜀.
#[test]
fn ask_prompts_on_conflicts_and_honors_answers() {
    let d = sandbox("ask");
    let (src, dst) = (d.join("src"), d.join("dst"));
    fs::create_dir_all(&src).expect("mkdir");
    fs::create_dir_all(&dst).expect("mkdir");
    for n in ["a.txt", "b.txt", "c.txt"] {
        fs::write(src.join(n), format!("new {n}")).expect("write");
        fs::write(dst.join(n), format!("old {n}")).expect("write");
    }
    let srcs = [src.join("a.txt"), src.join("b.txt"), src.join("c.txt")];
    let args: Vec<&Path> = srcs
        .iter()
        .map(PathBuf::as_path)
        .chain([dst.as_path()])
        .collect();
    // y · n · (EOF) → a 덮어씀 · b 그대로 · c 그대로.
    let r = run_stdin(&args, &["--ask"], "y\nn\n");
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(r.err.matches("overwrite? [y]es").count() == 3, "{}", r.err);
    assert_eq!(fs::read(dst.join("a.txt")).unwrap(), b"new a.txt");
    assert_eq!(fs::read(dst.join("b.txt")).unwrap(), b"old b.txt");
    assert_eq!(fs::read(dst.join("c.txt")).unwrap(), b"old c.txt");
    assert!(r.err.contains("1 transferred · 2 skipped"), "{}", r.err);
    // a = 남은 전부 덮어쓰기(프롬프트 1번만).
    let r = run_stdin(&args, &["--ask"], "a\n");
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(r.err.matches("overwrite? [y]es").count(), 1, "{}", r.err);
    assert_eq!(fs::read(dst.join("c.txt")).unwrap(), b"new c.txt");
    // c = 취소 → 130 · 이후 항목 손대지 않음.
    for n in ["a.txt", "b.txt", "c.txt"] {
        fs::write(dst.join(n), format!("old {n}")).expect("write");
    }
    let r = run_stdin(&args, &["--ask"], "c\n");
    assert_eq!(r.code, 130, "{}", r.err);
    assert!(
        fs::read(dst.join("b.txt")).unwrap() == b"old b.txt"
            && fs::read(dst.join("c.txt")).unwrap() == b"old c.txt"
    );
    let _ = fs::remove_dir_all(&d);
}

/// `--mirror`(10-25): dry-run 계획(copy · overwrite · delete 탭 줄) → 실행 = 새것/바뀐 것 복사 + 대상에만 있는 것 영구 삭제 · 재실행 = 할 일 없음 ·
/// 원본 2개/파일/겹침 = 2.
#[test]
fn mirror_makes_destination_identical() {
    let d = sandbox("mirror");
    let (src, dst) = (d.join("src"), d.join("dst"));
    make_tree(&src);
    fs::create_dir_all(dst.join("tree")).expect("mkdir");
    fs::write(dst.join("tree").join("stale.txt"), b"stale").expect("write");
    fs::write(dst.join("tree").join("f0.txt"), b"old content").expect("write");
    let r = run(
        &[&src.join("tree"), &dst.join("tree")],
        &["--mirror", "--dry-run"],
        &[],
    );
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(
        r.out
            .lines()
            .any(|l| l.starts_with("delete\t") && l.ends_with("stale.txt")),
        "{}",
        r.out
    );
    assert!(
        r.out
            .lines()
            .any(|l| l.starts_with("overwrite\t") && l.contains("f0.txt")),
        "{}",
        r.out
    );
    assert!(
        dst.join("tree").join("stale.txt").is_file(),
        "dry-run = 그대로"
    );
    assert!(r.err.contains("dry run (mirror)"), "{}", r.err);
    // 안전장치 ②: 지울 것이 있는데 `--yes`가 없으면 목록 + 종료 2 · 아무것도 안 바뀜.
    let r = run(&[&src.join("tree"), &dst.join("tree")], &["--mirror"], &[]);
    assert_eq!(r.code, 2, "{}", r.err);
    assert!(
        r.err.contains("would delete:") && r.err.contains("re-run with --yes"),
        "{}",
        r.err
    );
    assert!(
        dst.join("tree").join("stale.txt").is_file() && !dst.join("tree").join("f1.txt").exists(),
        "아무것도 안 바뀜"
    );
    let r = run(
        &[&src.join("tree"), &dst.join("tree")],
        &["--mirror", "--verify", "--yes"],
        &[],
    );
    assert_eq!(r.code, 0, "{}", r.err);
    same_tree(&src.join("tree"), &dst.join("tree"));
    assert!(
        !dst.join("tree").join("stale.txt").exists(),
        "대상에만 있던 것 삭제"
    );
    assert!(
        r.err.contains("deleted:") && r.err.contains("mirror: 41 copied · 1 deleted · 0 failed"),
        "{}",
        r.err
    );
    assert!(
        r.err.contains("verified 41 file(s) · 0 mismatch(es)"),
        "{}",
        r.err
    );
    // 다시 = 할 일 없음(지울 것이 없으면 --yes 불필요).
    let r = run(&[&src.join("tree"), &dst.join("tree")], &["--mirror"], &[]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(r.err.contains("mirror: 0 copied · 0 deleted"), "{}", r.err);
    // 인자 오류.
    let r = run(
        &[&src.join("tree"), &src.join("one.txt"), &dst],
        &["--mirror"],
        &[],
    );
    assert_eq!(r.code, 2);
    let r = run(&[&src.join("one.txt"), &dst], &["--mirror"], &[]);
    assert_eq!(r.code, 2);
    let r = run(&[&src, &src.join("tree")], &["--mirror"], &[]);
    assert_eq!(r.code, 2, "겹침");
    let _ = fs::remove_dir_all(&d);
}

/// `--include/--exclude`(10-25): 폴더 안은 엔진 계획 단계에서 거르고 · 최상위 원본은 CLI가 걸러 안내한다 · 대소문자 무시 · 제외가 우선.
#[test]
fn include_exclude_filters_files_and_top_level_sources() {
    let d = sandbox("filter");
    let (src, dst) = (d.join("src"), d.join("dst"));
    make_tree(&src); // tree/f0..f39.txt · tree/deep/inner.bin · one.txt
    fs::write(src.join("tree").join("note.md"), b"md").expect("write");
    fs::create_dir_all(&dst).expect("mkdir");
    let r = run(
        &[&src.join("tree"), &src.join("one.txt"), &dst],
        &[
            "--include",
            "*.TXT",
            "--exclude",
            "f1*",
            "--exclude",
            "deep",
        ],
        &[],
    );
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(dst.join("tree").join("f0.txt").is_file() && dst.join("one.txt").is_file());
    assert!(
        !dst.join("tree").join("f1.txt").exists() && !dst.join("tree").join("f10.txt").exists(),
        "exclude f1*"
    );
    assert!(
        !dst.join("tree").join("note.md").exists(),
        "include = *.txt만"
    );
    assert!(
        !dst.join("tree").join("deep").exists(),
        "제외 폴더는 통째로"
    );
    // include가 하나도 안 맞으면 경고(폴더 뼈대만 생김).
    let d3 = d.join("dst3");
    fs::create_dir_all(&d3).expect("mkdir");
    let r = run(&[&src.join("tree"), &d3], &["--include", "*.nomatch"], &[]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(
        r.err.contains("note: --include matched no files"),
        "{}",
        r.err
    );
    // 최상위 원본이 걸러지면 안내 + 나머지만.
    let d2 = d.join("dst2");
    fs::create_dir_all(&d2).expect("mkdir");
    let r = run(
        &[&src.join("one.txt"), &src.join("tree"), &d2],
        &["--exclude", "one.*"],
        &[],
    );
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(
        r.err.contains("filtered out:") && r.err.contains("one.txt"),
        "{}",
        r.err
    );
    assert!(!d2.join("one.txt").exists() && d2.join("tree").is_dir());
    let _ = fs::remove_dir_all(&d);
}

/// `--dry-run`: stdout = 탭 보고서(항목마다 한 줄) · stderr = 요약 · 종료 0 · 대상 폴더 비어 있음 그대로.
#[test]
fn dry_run_prints_plan_and_changes_nothing() {
    let d = sandbox("dry");
    let (src, dst) = (d.join("src"), d.join("dst"));
    make_tree(&src);
    fs::create_dir_all(&dst).expect("mkdir");
    let r = run(
        &[&src.join("tree"), &src.join("one.txt"), &dst],
        &["-n"],
        &[],
    );
    assert_eq!(r.code, 0, "{}", r.err);
    let rows: Vec<Vec<&str>> = r.out.lines().map(|l| l.split('\t').collect()).collect();
    assert_eq!(rows.len(), 2, "{}", r.out);
    assert_eq!(rows[0][0], "copy");
    assert_eq!(
        rows[0][1], "315500",
        "폴더 = 재귀 합(10 × 350 B + 30 × 400 B + 300 000 B)"
    );
    assert_eq!(rows[1][..2].to_vec(), vec!["copy", "3"]);
    assert!(
        r.err.contains("dry run: 2 to copy") && r.err.contains("nothing changed"),
        "{}",
        r.err
    );
    assert!(
        fs::read_dir(&dst).expect("read_dir").next().is_none(),
        "대상 비어 있음"
    );
    let _ = fs::remove_dir_all(&d);
}

/// 취소(Ctrl+C 대역 = `NEXA_COPY_CANCEL_AFTER_MS`): 큰 파일을 루프 경로로 복사하다 멈춤 → 종료 130 · 요약 `canceled` · 부분 파일 없음 · 원본 그대로.
#[test]
fn cancel_mid_copy_cleans_partial_and_exits_130() {
    let d = sandbox("cancel");
    let (src, dst) = (d.join("src"), d.join("dst"));
    fs::create_dir_all(&src).expect("mkdir");
    fs::create_dir_all(&dst).expect("mkdir");
    // 512 MiB — 희소 파일이면 읽기가 빨라 루프라도 수백 ms · 취소는 20 ms 뒤(청크 4 MiB 경계에서 멈춘다).
    let big = src.join("big.bin");
    let f = fs::File::create(&big).expect("create");
    f.set_len(512 * 1024 * 1024).expect("set_len");
    drop(f);
    let r = run(
        &[&big, &dst],
        &["--no-native", "--threads", "1"],
        &[("NEXA_COPY_CANCEL_AFTER_MS", "20")],
    );
    assert_eq!(r.code, 130, "stderr:\n{}", r.err);
    assert!(r.err.contains("canceled"), "{}", r.err);
    assert!(!dst.join("big.bin").exists(), "부분 파일 정리");
    assert_eq!(
        fs::metadata(&big).expect("meta").len(),
        512 * 1024 * 1024,
        "원본 그대로"
    );
    let _ = fs::remove_dir_all(&d);
}
