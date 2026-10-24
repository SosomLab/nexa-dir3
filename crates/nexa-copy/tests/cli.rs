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
