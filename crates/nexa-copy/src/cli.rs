//! 인자 해석(순수) · 실행(엔진 호출 · 진행 출력 · 요약 · 종료 코드).
//!
//! ```text
//! nexa-copy [options] <source>... <destination-folder>
//!   --move               move instead of copy (same-folder move is a no-op, like the app)
//!   --overwrite          overwrite existing files/folders in the destination (default: skip them)
//!   --threads <n>        worker threads for files inside folders: 0 = auto (default), 1 = sequential, up to 16
//!   --no-native          do not use the OS copy path (plain read/write loop everywhere)
//!   --unbuffered-mb <n>  Windows only: copy files of at least n MiB without the file cache (0 = off)
//!   -q, --quiet          no progress or summary (errors still go to stderr)
//!   -h, --help / -V, --version
//! exit code: 0 = all items done · 1 = one or more items failed · 2 = usage/argument error · 130 = canceled
//! ```
//! 진행 · 요약은 stderr(stdout은 비워 둔다 — 파이프 친화) · 영어 고정(패키지/CLI 문구 규칙 · 사용자 10-10).

use ndir_ops::fastcopy::{self, Tuning, THREADS_MAX};
use ndir_ops::{transfer, Conflict, Event, ItemStatus, Op, Outcome};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) const USAGE: &str = "nexa-copy — fast file copy/move (Nexa Dir transfer engine)

Usage: nexa-copy [options] <source>... <destination-folder>

Options:
  --move               move instead of copy
  --overwrite          overwrite existing items in the destination (default: skip)
  --threads <n>        worker threads for files inside folders: 0 = auto (default), 1 = sequential, max 16
  --no-native          do not use the OS copy path (plain read/write loop)
  --unbuffered-mb <n>  Windows only: copy files of at least n MiB without the file cache (0 = off)
  -q, --quiet          no progress or summary (errors still go to stderr)
  -h, --help           show this help
  -V, --version        show version

Exit code: 0 done · 1 some items failed · 2 usage error · 130 canceled
";

/// 실행 옵션(순수 해석 결과).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Options {
    pub op: Op,
    pub overwrite: bool,
    pub threads: usize,
    pub native: bool,
    pub unbuffered_mb: u64,
    pub quiet: bool,
    pub sources: Vec<PathBuf>,
    pub dest: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Parsed {
    Run(Options),
    Help,
    Version,
}

/// 인자 해석(순수): 옵션은 어디에 와도 되고 `--` 뒤는 전부 경로 · 마지막 경로 = 대상 폴더 · 경로가 2개 미만이면 오류.
pub(crate) fn parse(args: &[String]) -> Result<Parsed, String> {
    let mut op = Op::Copy;
    let mut overwrite = false;
    let mut threads = 0usize;
    let mut native = true;
    let mut unbuffered_mb = 0u64;
    let mut quiet = false;
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut only_paths = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if only_paths || !a.starts_with('-') || a == "-" {
            paths.push(PathBuf::from(a));
            continue;
        }
        match a.as_str() {
            "--" => only_paths = true,
            "-h" | "--help" => return Ok(Parsed::Help),
            "-V" | "--version" => return Ok(Parsed::Version),
            "--move" => op = Op::Move,
            "--copy" => op = Op::Copy,
            "--overwrite" => overwrite = true,
            "--skip" => overwrite = false,
            "--no-native" => native = false,
            "-q" | "--quiet" => quiet = true,
            "--threads" => {
                let v = it.next().ok_or("--threads needs a number")?;
                threads = v
                    .parse::<usize>()
                    .ok()
                    .filter(|n| *n <= THREADS_MAX)
                    .ok_or_else(|| format!("--threads: expected 0..={THREADS_MAX}, got '{v}'"))?;
            }
            "--unbuffered-mb" => {
                let v = it.next().ok_or("--unbuffered-mb needs a number")?;
                unbuffered_mb = v
                    .parse::<u64>()
                    .map_err(|_| format!("--unbuffered-mb: expected a number, got '{v}'"))?;
            }
            other => return Err(format!("unknown option '{other}'")),
        }
    }
    if paths.len() < 2 {
        return Err("need at least one source and a destination folder".into());
    }
    let dest = paths.pop().unwrap_or_default();
    Ok(Parsed::Run(Options {
        op,
        overwrite,
        threads,
        native,
        unbuffered_mb,
        quiet,
        sources: paths,
        dest,
    }))
}

/// 진입점(시험 가능): 해석 → 도움말/버전은 stdout · 실행은 [`run`].
pub(crate) fn main_with(args: &[String], err: &mut dyn Write) -> ExitCode {
    match parse(args) {
        Err(msg) => {
            let _ = writeln!(err, "nexa-copy: {msg}\n\n{USAGE}");
            ExitCode::from(2)
        }
        Ok(Parsed::Help) => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Parsed::Version) => {
            println!("nexa-copy {VERSION}");
            ExitCode::SUCCESS
        }
        Ok(Parsed::Run(o)) => run(&o, err),
    }
}

/// 실행: 대상 폴더 · 원본 존재 확인 → 전략 조절값 → `transfer`(앱과 같은 길) → 진행(stderr · 100 ms 간격) → 요약 → 종료 코드.
pub(crate) fn run(o: &Options, err: &mut dyn Write) -> ExitCode {
    if !o.dest.is_dir() {
        let _ = writeln!(
            err,
            "nexa-copy: destination is not an existing folder: {}",
            o.dest.display()
        );
        return ExitCode::from(2);
    }
    let missing: Vec<&PathBuf> = o.sources.iter().filter(|p| !p.exists()).collect();
    if !missing.is_empty() {
        for p in &missing {
            let _ = writeln!(err, "nexa-copy: source not found: {}", p.display());
        }
        return ExitCode::from(2);
    }
    fastcopy::set_tuning(Tuning {
        native: o.native,
        unbuffered_min: o.unbuffered_mb.saturating_mul(1024 * 1024),
        threads: o.threads,
    });
    let cancel = AtomicBool::new(false);
    let mut resolve = |_: &Path| {
        if o.overwrite {
            Conflict::Overwrite
        } else {
            Conflict::Skip
        }
    };
    let started = Instant::now();
    let mut printer = Printer::new(o.quiet, started);
    let out = transfer(
        &o.sources,
        &o.dest,
        o.op,
        &mut resolve,
        &mut |ev| printer.event(&ev, err),
        &cancel,
    );
    printer.summary(&out, started.elapsed(), err);
    exit_code(&out)
}

/// 종료 코드(순수): 취소 130 · 실패 항목 있음 1 · 그 밖 0(건너뜀은 실패가 아니다 — 정책대로 한 것).
pub(crate) fn exit_code(out: &Outcome) -> ExitCode {
    if out.canceled {
        ExitCode::from(130)
    } else if out.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// 진행 출력(stderr · 한 줄 되감기 · 100 ms 간격) — 조용히(quiet)면 아무것도 쓰지 않는다.
struct Printer {
    quiet: bool,
    started: Instant,
    last: Option<Instant>,
    total: u64,
    items: usize,
    line_open: bool,
}

impl Printer {
    fn new(quiet: bool, started: Instant) -> Self {
        Self {
            quiet,
            started,
            last: None,
            total: 0,
            items: 0,
            line_open: false,
        }
    }

    fn event(&mut self, ev: &Event, err: &mut dyn Write) {
        if self.quiet {
            return;
        }
        match ev {
            Event::Plan { sizes, total_bytes } => {
                self.total = *total_bytes;
                self.items = sizes.len();
                let _ = writeln!(
                    err,
                    "{} item(s) · {}",
                    sizes.len(),
                    human_bytes(*total_bytes)
                );
            }
            Event::ItemStart { index, dest } => {
                self.end_line(err);
                let _ = writeln!(err, "[{}/{}] {}", index + 1, self.items, dest.display());
            }
            Event::Bytes(p) => {
                let now = Instant::now();
                if self
                    .last
                    .is_some_and(|t| now.duration_since(t) < Duration::from_millis(100))
                {
                    return;
                }
                self.last = Some(now);
                let _ = write!(
                    err,
                    "\r  {} / {} ({}%) · {}",
                    human_bytes(p.done_bytes),
                    human_bytes(p.total_bytes),
                    percent(p.done_bytes, p.total_bytes),
                    rate(p.done_bytes, self.started.elapsed())
                );
                let _ = err.flush();
                self.line_open = true;
            }
            Event::ItemEnd { index, status } => {
                self.end_line(err);
                let word = match status {
                    ItemStatus::Done => "done",
                    ItemStatus::Skipped => "skipped",
                    ItemStatus::Failed => "failed",
                };
                let _ = writeln!(err, "[{}/{}] {word}", index + 1, self.items);
            }
        }
    }

    fn end_line(&mut self, err: &mut dyn Write) {
        if self.line_open {
            let _ = writeln!(err);
            self.line_open = false;
        }
    }

    fn summary(&mut self, out: &Outcome, elapsed: Duration, err: &mut dyn Write) {
        for (p, why) in &out.errors {
            let _ = writeln!(err, "error: {}: {why}", p.display());
        }
        if self.quiet {
            return;
        }
        self.end_line(err);
        let _ = writeln!(
            err,
            "{} transferred · {} skipped · {} failed{} · {} in {}",
            out.transferred.len(),
            out.skipped.len(),
            out.errors.len(),
            if out.canceled { " · canceled" } else { "" },
            human_bytes(self.total),
            human_secs(elapsed)
        );
    }
}

/// 백분율(순수 · 총량 0 = 100).
pub(crate) fn percent(done: u64, total: u64) -> u64 {
    done.saturating_mul(100).checked_div(total).unwrap_or(100)
}

/// 속도 글(순수): 경과 0이면 "-".
pub(crate) fn rate(done: u64, elapsed: Duration) -> String {
    let secs = elapsed.as_secs_f64();
    if secs <= 0.0 {
        return "-".into();
    }
    format!("{}/s", human_bytes((done as f64 / secs) as u64))
}

/// 사람이 읽는 바이트(순수 · 1024 단위 · 소수 1자리 · B는 정수).
pub(crate) fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    if n < 1024 {
        return format!("{n} B");
    }
    let mut v = n as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    format!("{v:.1} {}", UNITS[u])
}

/// 경과 시간 글(순수): 1분 미만 = 초(소수 1자리) · 그 위 = m:ss.
pub(crate) fn human_secs(d: Duration) -> String {
    let s = d.as_secs_f64();
    if s < 60.0 {
        format!("{s:.1}s")
    } else {
        let m = (s / 60.0).floor() as u64;
        format!("{m}m{:02.0}s", s - m as f64 * 60.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn a(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    /// 인자 해석(순수): 기본값 · 옵션 어디서나 · `--` 뒤 경로 · 범위 · 오류 문구 · 도움말/버전.
    #[test]
    fn parse_options_and_errors() {
        let Ok(Parsed::Run(o)) = parse(&a(&["a", "b", "dest"])) else {
            panic!()
        };
        assert_eq!(o.op, Op::Copy);
        assert!(!o.overwrite && o.native && !o.quiet);
        assert_eq!((o.threads, o.unbuffered_mb), (0, 0));
        assert_eq!(o.sources, vec![PathBuf::from("a"), PathBuf::from("b")]);
        assert_eq!(o.dest, PathBuf::from("dest"));
        let Ok(Parsed::Run(o)) = parse(&a(&[
            "--move",
            "x",
            "--overwrite",
            "--threads",
            "4",
            "--no-native",
            "-q",
            "--unbuffered-mb",
            "64",
            "--",
            "-weird",
            "d",
        ])) else {
            panic!()
        };
        assert_eq!(o.op, Op::Move);
        assert!(o.overwrite && !o.native && o.quiet);
        assert_eq!((o.threads, o.unbuffered_mb), (4, 64));
        assert_eq!(o.sources, vec![PathBuf::from("x"), PathBuf::from("-weird")]);
        assert_eq!(o.dest, PathBuf::from("d"));
        assert_eq!(parse(&a(&["-h"])), Ok(Parsed::Help));
        assert_eq!(parse(&a(&["a", "--version"])), Ok(Parsed::Version));
        assert!(parse(&a(&["only"])).unwrap_err().contains("destination"));
        assert!(parse(&a(&["--threads", "99", "a", "b"]))
            .unwrap_err()
            .contains("--threads"));
        assert!(parse(&a(&["--threads"])).is_err());
        assert!(parse(&a(&["--bogus", "a", "b"]))
            .unwrap_err()
            .contains("--bogus"));
    }

    #[test]
    fn human_readable_helpers() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(1023), "1023 B");
        assert_eq!(human_bytes(1536), "1.5 KiB");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5.0 MiB");
        assert_eq!(percent(0, 0), 100);
        assert_eq!(percent(1, 4), 25);
        assert_eq!(rate(1024, Duration::ZERO), "-");
        assert_eq!(rate(2048, Duration::from_secs(2)), "1.0 KiB/s");
        assert_eq!(human_secs(Duration::from_millis(1500)), "1.5s");
        assert_eq!(human_secs(Duration::from_secs(125)), "2m05s");
    }

    fn sandbox(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nexa-copy-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).expect("mkdir");
        d
    }

    /// 실행(샌드박스): 폴더 + 파일 복사 → 내용 같음 · 두 번째는 건너뜀(기본) · `--overwrite`면 덮어씀 · `--move`는 원본이 사라짐 ·
    /// 없는 원본/대상 = 종료 2 · 조용히(quiet) = 요약 없음 · 진행/요약은 stderr 버퍼에.
    #[test]
    fn run_copies_skips_overwrites_and_moves() {
        let d = sandbox("run");
        let src = d.join("src");
        fs::create_dir_all(src.join("tree").join("deep")).expect("mkdir");
        fs::write(src.join("tree").join("deep").join("x.txt"), b"xx").expect("write");
        fs::write(src.join("tree").join("y.bin"), vec![3u8; 70_000]).expect("write");
        fs::write(src.join("one.txt"), b"one").expect("write");
        let dest = d.join("dest");
        fs::create_dir_all(&dest).expect("mkdir");
        let mut err: Vec<u8> = Vec::new();
        let o = match parse(&a(&[
            src.join("tree").to_string_lossy().as_ref(),
            src.join("one.txt").to_string_lossy().as_ref(),
            dest.to_string_lossy().as_ref(),
        ])) {
            Ok(Parsed::Run(o)) => o,
            other => panic!("{other:?}"),
        };
        assert_eq!(run(&o, &mut err), ExitCode::SUCCESS);
        assert_eq!(
            fs::read(dest.join("tree").join("deep").join("x.txt")).expect("read"),
            b"xx"
        );
        assert_eq!(
            fs::metadata(dest.join("tree").join("y.bin"))
                .expect("meta")
                .len(),
            70_000
        );
        assert_eq!(fs::read(dest.join("one.txt")).expect("read"), b"one");
        let text = String::from_utf8_lossy(&err).into_owned();
        assert!(
            text.contains("2 item(s)") && text.contains("2 transferred"),
            "{text}"
        );
        // 기본 = 건너뜀(종료 0 · 요약에 skipped) · 덮어쓰기.
        fs::write(src.join("one.txt"), b"ONE").expect("write");
        let mut err: Vec<u8> = Vec::new();
        assert_eq!(run(&o, &mut err), ExitCode::SUCCESS);
        assert_eq!(fs::read(dest.join("one.txt")).expect("read"), b"one");
        assert!(String::from_utf8_lossy(&err).contains("2 skipped"));
        let ov = Options {
            overwrite: true,
            quiet: true,
            ..o.clone()
        };
        let mut err: Vec<u8> = Vec::new();
        assert_eq!(run(&ov, &mut err), ExitCode::SUCCESS);
        assert_eq!(fs::read(dest.join("one.txt")).expect("read"), b"ONE");
        assert!(
            err.is_empty(),
            "quiet = 출력 없음: {}",
            String::from_utf8_lossy(&err)
        );
        // 이동 = 원본 사라짐.
        let moved_dest = d.join("moved");
        fs::create_dir_all(&moved_dest).expect("mkdir");
        let mv = Options {
            op: Op::Move,
            sources: vec![src.join("one.txt")],
            dest: moved_dest.clone(),
            ..o.clone()
        };
        let mut err: Vec<u8> = Vec::new();
        assert_eq!(run(&mv, &mut err), ExitCode::SUCCESS);
        assert!(!src.join("one.txt").exists());
        assert_eq!(fs::read(moved_dest.join("one.txt")).expect("read"), b"ONE");
        // 없는 원본 · 대상 폴더 아님 = 2.
        let bad = Options {
            sources: vec![d.join("nope")],
            ..o.clone()
        };
        let mut err: Vec<u8> = Vec::new();
        assert_eq!(run(&bad, &mut err), ExitCode::from(2));
        assert!(String::from_utf8_lossy(&err).contains("not found"));
        let bad2 = Options {
            dest: d.join("no-such-dir"),
            ..o
        };
        assert_eq!(run(&bad2, &mut Vec::new()), ExitCode::from(2));
        // 도움말 · 버전 · 사용 오류 종료 코드.
        assert_eq!(
            main_with(&a(&["--help"]), &mut Vec::new()),
            ExitCode::SUCCESS
        );
        let mut err: Vec<u8> = Vec::new();
        assert_eq!(main_with(&a(&["x"]), &mut err), ExitCode::from(2));
        assert!(String::from_utf8_lossy(&err).contains("Usage"));
        let _ = fs::remove_dir_all(&d);
    }
}
