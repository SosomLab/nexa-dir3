//! 인자 해석(순수) · 실행(엔진 호출 · 진행 출력 · 요약 · 종료 코드).
//!
//! ```text
//! nexa-copy [options] <source>... <destination-folder>
//!   --move               move instead of copy (same-folder move is a no-op, like the app)
//!   --overwrite          overwrite existing files/folders in the destination (default: skip them)
//!   --ask                ask on each conflict (stderr prompt · one line from stdin: y/n/a(ll)/s(kip all)/c(ancel))
//!   --threads <n>        worker threads for files inside folders: 0 = auto (default), 1 = sequential, up to 16
//!   --no-native          do not use the OS copy path (plain read/write loop everywhere)
//!   --unbuffered-mb <n>  Windows only: copy files of at least n MiB without the file cache (0 = off)
//!   -n, --dry-run        print the plan (one tab-separated line per item on stdout) and change nothing
//!   --verify             after copying, compare every copied file with its source (size + SHA-256); mismatch = exit 1
//!   --mirror             make the destination folder identical to the one source folder (copy new/changed · delete extras permanently)
//!   --yes                confirm permanent deletions for --mirror (without it, deletions are listed and the exit code is 2)
//!   --include <glob>     copy only files whose name matches (repeatable · folders are always entered)
//!   --exclude <glob>     skip files and folders whose name matches (repeatable)
//!   --job <file.json>    run a job spec (ndir-xfer JobSpec JSON · all options above as keys · no --ask) instead of arguments
//!   --progress           machine-readable: one JSON line per progress step on stdout, then the final report JSON (T-97 · 10-31)
//!   -q, --quiet          no progress or summary (errors still go to stderr)
//!   -h, --help / -V, --version
//! exit code: 0 = all items done · 1 = one or more items failed · 2 = usage/argument error · 130 = canceled (Ctrl+C)
//! ```
//! 진행 · 요약은 stderr(stdout은 비워 둔다 — 파이프 친화 · dry-run 보고서만 stdout) · 영어 고정(패키지/CLI 문구 규칙 · 사용자 10-10).
//!
//! dry-run 보고서(stdout · 탭 구분 · 항목마다 한 줄): `<action>\t<bytes>\t<source>\t<destination>\t<note>` —
//! action = copy | move | duplicate | overwrite | skip | noop | error · note = `exists`(overwrite/skip) 또는 오류 사유 · 그 밖은 빈칸.

use ndir_ops::fastcopy::{NameFilter, Tuning, THREADS_MAX};
use ndir_ops::{
    plan_transfer, transfer_with, Conflict, Event, ItemStatus, Op, Outcome, PlanAction,
};
use ndir_xfer::{ConflictPolicy, JobSpec};
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
  --ask                ask on each conflict instead: prompt on stderr, answer on stdin —
                       y = overwrite · n = skip · a = overwrite all · s = skip all · c = cancel (exit 130)
  --threads <n>        worker threads for files inside folders: 0 = auto (default), 1 = sequential, max 16
  --no-native          do not use the OS copy path (plain read/write loop)
  --unbuffered-mb <n>  Windows only: copy files of at least n MiB without the file cache (0 = off)
  -n, --dry-run        print the plan and change nothing: one line per item on stdout,
                       tab-separated: action, bytes, source, destination, note
                       (action = copy | move | duplicate | overwrite | skip | noop | error)
  --mirror             make <destination-folder> identical to the single <source> folder: copy new and changed
                       files (size + modified time, 2 s tolerance), then permanently delete items that exist
                       only in the destination (no recycle bin) · works with --dry-run, --include/--exclude, --verify
                       · refuses a drive root or the user home as destination · deletions need --yes
  --yes                confirm permanent deletions (--mirror); without it they are only listed and the exit code is 2
  --include <glob>     copy only files whose name matches the glob (* ?, case-insensitive);
                       repeatable, or several globs separated by ; · folders are always entered
  --exclude <glob>     skip files and folders whose name matches (repeatable or ;-separated · wins over --include)
  --verify             after copying, compare each copied file with its source (size + SHA-256);
                       folders are checked recursively · a mismatch is reported and the exit code is 1
                       (ignored with --move: the source is gone)
  --job <file.json>    run an ndir-xfer JobSpec JSON instead of arguments (no --ask)
  --progress           JSON progress lines + final report on stdout (machine-readable)
  -q, --quiet          no progress or summary (errors still go to stderr)
  -h, --help           show this help
  -V, --version        show version

Exit code: 0 done · 1 some items failed (dry run: would fail) · 2 usage error · 130 canceled (Ctrl+C)
";

/// 실행 옵션(순수 해석 결과).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Options {
    pub op: Op,
    pub overwrite: bool,
    /// 충돌마다 묻기(`--ask` · stdin 한 줄 · 정책보다 우선 · dry-run에서는 건너뜀으로 본다).
    pub ask: bool,
    pub threads: usize,
    pub native: bool,
    pub unbuffered_mb: u64,
    pub quiet: bool,
    /// 계획만 보고하고 바꾸지 않는다(`--dry-run`).
    pub dry_run: bool,
    /// 복사 뒤 원본↔사본 대조(`--verify` · 크기 + SHA-256 · 이동은 무시).
    pub verify: bool,
    /// 이름 필터(`--include/--exclude` · 글롭 · 폴더 안과 최상위 원본 모두).
    pub filter: NameFilter,
    /// 미러(`--mirror` · 원본 폴더 1개 → 대상 폴더를 같게 · 대상에만 있는 것은 영구 삭제).
    pub mirror: bool,
    /// 영구 삭제 확인(`--yes` · 미러에서 지울 것이 있을 때 필요).
    pub yes: bool,
    pub sources: Vec<PathBuf>,
    pub dest: PathBuf,
    /// `--progress`: JSON 줄 출력(ndir-xfer 경로 · `--ask`와 함께 못 씀).
    pub progress: bool,
}

impl Options {
    /// 인자 → ndir-xfer 명세(`--ask`는 명세에 없다 · 호출자가 거른다).
    pub(crate) fn to_spec(&self) -> JobSpec {
        JobSpec {
            op: self.op,
            sources: self.sources.clone(),
            dest: self.dest.clone(),
            conflict: if self.overwrite {
                ConflictPolicy::Overwrite
            } else {
                ConflictPolicy::Skip
            },
            threads: self.threads,
            native: self.native,
            unbuffered_mb: self.unbuffered_mb,
            filter: self.filter.clone(),
            verify: self.verify,
            mirror: self.mirror,
            dry_run: self.dry_run,
            yes: self.yes,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Parsed {
    Run(Options),
    /// `--job <file>`(JobSpec JSON · 10-31 T-97) — `progress` = JSON 줄 출력 · `quiet` = 요약 생략.
    Job {
        file: PathBuf,
        progress: bool,
        quiet: bool,
    },
    Help,
    Version,
}

/// 인자 해석(순수): 옵션은 어디에 와도 되고 `--` 뒤는 전부 경로 · 마지막 경로 = 대상 폴더 · 경로가 2개 미만이면 오류.
pub(crate) fn parse(args: &[String]) -> Result<Parsed, String> {
    let mut op = Op::Copy;
    let mut overwrite = false;
    let mut ask = false;
    let mut threads = 0usize;
    let mut native = true;
    let mut unbuffered_mb = 0u64;
    let mut quiet = false;
    let mut dry_run = false;
    let mut verify = false;
    let mut filter = NameFilter::default();
    let mut mirror = false;
    let mut yes = false;
    let mut progress = false;
    let mut job: Option<PathBuf> = None;
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
            "--ask" => ask = true,
            "--skip" => overwrite = false,
            "--no-native" => native = false,
            "-q" | "--quiet" => quiet = true,
            "-n" | "--dry-run" => dry_run = true,
            "--verify" => verify = true,
            "--mirror" => mirror = true,
            "-y" | "--yes" => yes = true,
            "--progress" => progress = true,
            "--job" => {
                let v = it.next().ok_or("--job needs a file")?;
                job = Some(PathBuf::from(v));
            }
            "--include" | "--exclude" => {
                let v = it.next().ok_or_else(|| format!("{a} needs a glob"))?;
                // `;`로 여러 글롭(앱의 패턴 선택 · 빠른 필터와 같은 습관 · 협업 점검 10-25 — 종전엔 `;`를 글자로 봐 0개 복사).
                let globs: Vec<String> = v
                    .split(';')
                    .map(str::trim)
                    .filter(|g| !g.is_empty())
                    .map(str::to_string)
                    .collect();
                if globs.is_empty() {
                    return Err(format!("{a}: empty glob"));
                }
                if a == "--include" {
                    filter.include.extend(globs);
                } else {
                    filter.exclude.extend(globs);
                }
            }
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
    if let Some(file) = job {
        if !paths.is_empty() || ask || mirror || dry_run || verify || overwrite {
            return Err(
                "--job takes no sources/destination or operation flags (put them in the JSON)"
                    .into(),
            );
        }
        return Ok(Parsed::Job {
            file,
            progress,
            quiet,
        });
    }
    if paths.len() < 2 {
        return Err("need at least one source and a destination folder".into());
    }
    let dest = paths.pop().unwrap_or_default();
    Ok(Parsed::Run(Options {
        op,
        overwrite,
        ask,
        threads,
        native,
        unbuffered_mb,
        quiet,
        dry_run,
        verify,
        filter,
        mirror,
        yes,
        sources: paths,
        dest,
        progress,
    }))
}

/// `--job`/`--progress` 실행(ndir-xfer `run` · 10-31 T-97): 진행 = stdout JSON 줄(단계가 바뀌거나 100 ms마다) · 끝 = Report JSON 한 줄 ·
/// `--progress`가 아니면 사람용 요약(stderr) · 종료 코드 = `Report::exit_code`(명세 오류 = 2).
pub(crate) fn run_job_with(
    spec: &JobSpec,
    progress: bool,
    quiet: bool,
    cancel: &AtomicBool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> ExitCode {
    let mut last: Option<(Instant, ndir_xfer::Phase)> = None;
    let mut on_progress = |p: &ndir_xfer::Progress| {
        if !progress {
            return;
        }
        let due = match last {
            None => true,
            Some((t, ph)) => ph != p.phase || t.elapsed() >= Duration::from_millis(100),
        };
        if due {
            last = Some((Instant::now(), p.phase));
            let _ = writeln!(out, "{}", p.to_json_line());
            let _ = out.flush();
        }
    };
    let rep = ndir_xfer::run(spec, &mut on_progress, cancel);
    if progress {
        let _ = writeln!(out, "{}", rep.to_json());
        let _ = out.flush();
    } else if !quiet {
        for e in &rep.errors {
            let _ = writeln!(err, "nexa-copy: {e}");
        }
        for it in rep.items.iter().filter(|i| i.status == "failed") {
            let _ = writeln!(err, "error: {}: {}", it.dest.display(), it.note);
        }
        if let Some(v) = &rep.verify {
            for (p, why) in &v.mismatches {
                let _ = writeln!(err, "verify failed: {}: {why}", p.display());
            }
        }
        let _ = writeln!(
            err,
            "{}: {} item(s) · {} done · {} skipped · {} failed · {} deleted · {} in {}",
            if rep.dry_run { "dry run" } else { "job" },
            rep.items.len(),
            rep.items.iter().filter(|i| i.status == "done").count(),
            rep.skipped,
            rep.failed,
            rep.deleted,
            human_bytes(rep.transferred_bytes),
            human_secs(Duration::from_millis(rep.elapsed_ms))
        );
    } else {
        for e in &rep.errors {
            let _ = writeln!(err, "nexa-copy: {e}");
        }
    }
    if !rep.errors.is_empty() && rep.items.is_empty() && !rep.canceled {
        return ExitCode::from(2); // 명세/인자 수준 오류
    }
    ExitCode::from(rep.exit_code())
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
        Ok(Parsed::Job {
            file,
            progress,
            quiet,
        }) => {
            let text = match std::fs::read_to_string(&file) {
                Ok(t) => t,
                Err(e) => {
                    let _ = writeln!(err, "nexa-copy: --job {}: {e}", file.display());
                    return ExitCode::from(2);
                }
            };
            let spec = match JobSpec::from_json(&text) {
                Ok(s) => s,
                Err(e) => {
                    let _ = writeln!(err, "nexa-copy: --job {}: {e}", file.display());
                    return ExitCode::from(2);
                }
            };
            crate::ctrlc::install();
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            run_job_with(&spec, progress, quiet, crate::ctrlc::flag(), &mut out, err)
        }
        Ok(Parsed::Run(o)) if o.progress => {
            if o.ask {
                let _ = writeln!(err, "nexa-copy: --progress cannot be combined with --ask");
                return ExitCode::from(2);
            }
            crate::ctrlc::install();
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            run_job_with(
                &o.to_spec(),
                true,
                o.quiet,
                crate::ctrlc::flag(),
                &mut out,
                err,
            )
        }
        Ok(Parsed::Run(o)) => {
            crate::ctrlc::install(); // Ctrl+C → 취소 깃발(항목/청크 경계에서 멈추고 요약 · 130)
                                     // 시험 훅(통합 시험 `tests/cli.rs` · 10-24): `NEXA_COPY_CANCEL_AFTER_MS=<ms>` = 그 뒤 같은 취소 깃발을 세운다(Ctrl+C 대역 ·
                                     // 콘솔 신호를 OS마다 흉내 내지 않는다). 평소엔 변수 없음 = 비용 0.
            if let Some(ms) = std::env::var("NEXA_COPY_CANCEL_AFTER_MS")
                .ok()
                .and_then(|v| v.trim().parse::<u64>().ok())
            {
                let flag = crate::ctrlc::flag();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(ms));
                    flag.store(true, std::sync::atomic::Ordering::Relaxed);
                });
            }
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            run_with(&o, crate::ctrlc::flag(), &mut out, err)
        }
    }
}

/// [`run_with`] + 프로세스 취소 깃발 없이 · stdout 버림(시험 · 내장 호출용 — 취소 없음).
#[cfg(test)]
pub(crate) fn run(o: &Options, err: &mut dyn Write) -> ExitCode {
    run_with(o, &AtomicBool::new(false), &mut std::io::sink(), err)
}

/// 실행: 대상 폴더 · 원본 존재 확인 → 전략 조절값 → `transfer`(앱과 같은 길) → 진행(stderr · 100 ms 간격) → 요약 → 종료 코드.
/// `cancel`이 참이 되면(Ctrl+C) 엔진이 항목/청크 경계에서 멈추고 부분 파일을 정리한다 → 130. `--dry-run`이면 [`dry_run`](보고서 = `out`).
pub(crate) fn run_with(
    o: &Options,
    cancel: &AtomicBool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> ExitCode {
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
    if o.mirror {
        return crate::mirror::run_mirror(o, cancel, out, err);
    }
    if o.dry_run {
        return dry_run(o, out, err);
    }
    // 조절값은 인자로(`transfer_with` · 프로세스 전역 `set_tuning`을 쓰지 않는다).
    let tuning = Tuning {
        native: o.native,
        unbuffered_min: o.unbuffered_mb.saturating_mul(1024 * 1024),
        threads: o.threads,
    };
    // 충돌 결정: `--ask` = 항목마다 stderr 프롬프트 + stdin 한 줄(a/s = 남은 전부 · c = 취소 깃발) · 아니면 정책(`--overwrite`/건너뜀).
    // 프롬프트는 진행 출력(`err`)과 다른 핸들(실제 stderr)로 쓴다 — 같은 전송 호출에 두 클로저가 함께 빌리기 때문.
    let mut ask_all: Option<Conflict> = None;
    let mut resolve = |dest: &Path| {
        if !o.ask {
            return if o.overwrite {
                Conflict::Overwrite
            } else {
                Conflict::Skip
            };
        }
        if let Some(all) = ask_all {
            return all;
        }
        let (choice, all) =
            ask_conflict(dest, &mut std::io::stderr(), &mut std::io::stdin().lock());
        if all {
            ask_all = Some(choice);
        }
        if choice == Conflict::Skip
            && matches!(ask_all, Some(Conflict::Skip))
            && CANCEL_ASKED.with(|c| c.get())
        {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        choice
    };
    let started = Instant::now();
    let mut printer = Printer::new(o.quiet, started);
    // 이름 필터(`--include/--exclude`): 최상위 원본은 여기서(거른 것은 안내) · 폴더 안은 엔진 계획 단계(`with_filter`).
    let (sources, dropped): (Vec<PathBuf>, Vec<PathBuf>) = o
        .sources
        .iter()
        .cloned()
        .partition(|p| o.filter.allows(&ndir_ops::leaf_name(p), p.is_dir()));
    if !o.quiet {
        for p in &dropped {
            let _ = writeln!(err, "filtered out: {}", p.display());
        }
    }
    let out = ndir_ops::fastcopy::with_filter(o.filter.clone(), || {
        transfer_with(
            &sources,
            &o.dest,
            o.op,
            &mut resolve,
            &mut |ev| printer.event(&ev, err),
            cancel,
            &tuning,
        )
    });
    printer.summary(&out, started.elapsed(), err);
    // `--include`가 파일을 하나도 안 골랐다(옮긴 바이트 0 · 폴더 뼈대만) — 조용히 0으로 끝나면 "복사됐다"로 오해(협업 점검 10-25).
    if !o.filter.include.is_empty()
        && !out.transferred.is_empty()
        && printer.seen.is_none_or(|(d, _)| d == 0)
    {
        let _ = writeln!(
            err,
            "note: --include matched no files (only folders were created)"
        );
    }
    // 검증(`--verify` · T-97): 복사한 쌍마다 크기 + SHA-256 · 불일치는 오류로 세어 종료 1. 이동은 원본이 없어 못 하고(안내 한 줄) ·
    // 취소된 전송은 하지 않는다 · 건너뛴 항목(대상에 이미 있음)은 대조 대상이 아님을 요약에 적는다(협업 점검 10-25 — 조용히 통과로 오해 방지).
    if o.verify && o.op == Op::Move {
        let _ = writeln!(
            err,
            "note: --verify is ignored with --move (the source is gone after a move)"
        );
    } else if o.verify && out.canceled {
        let _ = writeln!(err, "note: verification skipped (canceled)");
    } else if o.verify {
        let v_started = Instant::now();
        // 읽을 총량 = 복사한 항목 크기 × 2(원본 + 사본) — "n of m" 진행.
        let total: u64 = out
            .transferred
            .iter()
            .map(|(s, _)| ndir_ops::size_of(s))
            .sum::<u64>()
            .saturating_mul(2);
        let mut read = 0u64;
        let mut last = Instant::now();
        let mut on_bytes = |n: u64| {
            read += n;
            if !o.quiet && last.elapsed() >= Duration::from_millis(250) {
                last = Instant::now();
                let _ = write!(
                    err,
                    "\rverifying · {} of {}",
                    human_bytes(read),
                    human_bytes(total)
                );
                let _ = err.flush();
            }
        };
        match ndir_xfer::verify::verify_pairs(&out.transferred, &mut on_bytes, cancel) {
            Ok(rep) => {
                if !o.quiet && read > 0 {
                    let _ = writeln!(err);
                }
                for (p, why) in &rep.failures {
                    let _ = writeln!(err, "verify failed: {}: {why}", p.display());
                }
                if !o.quiet {
                    let skipped = if out.skipped.is_empty() {
                        String::new()
                    } else {
                        format!(" · {} skipped item(s) not verified", out.skipped.len())
                    };
                    let _ = writeln!(
                        err,
                        "verified {} file(s) · {} read · {} mismatch(es){skipped} in {}",
                        rep.files,
                        human_bytes(rep.bytes),
                        rep.failures.len(),
                        human_secs(v_started.elapsed())
                    );
                }
                if !rep.ok() {
                    return ExitCode::from(1);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
                let _ = writeln!(err, "\nverify canceled");
                return ExitCode::from(130);
            }
            Err(e) => {
                let _ = writeln!(err, "\nverify error: {e}");
                return ExitCode::from(1);
            }
        }
    }
    exit_code(&out)
}

/// dry-run(10-24 · T-97 "dry-run 보고서"): 엔진의 `plan_transfer`(실행과 같은 판정)로 항목마다 한 줄을 stdout에 — 탭 구분
/// `action · bytes · source · destination · note` · 요약은 stderr(quiet면 생략) · 파일 시스템 불변 · 종료 0(실행하면 실패할 항목이 있으면 1).
fn dry_run(o: &Options, out: &mut dyn Write, err: &mut dyn Write) -> ExitCode {
    let mut resolve = |_: &Path| {
        if o.overwrite {
            Conflict::Overwrite
        } else {
            Conflict::Skip
        }
    };
    let sources: Vec<PathBuf> = o
        .sources
        .iter()
        .filter(|p| o.filter.allows(&ndir_ops::leaf_name(p), p.is_dir()))
        .cloned()
        .collect();
    let plan = plan_transfer(&sources, &o.dest, o.op, &mut resolve);
    let verb = match o.op {
        Op::Copy => "copy",
        Op::Move => "move",
    };
    let (mut active, mut skipped, mut failing) = (0usize, 0usize, 0usize);
    for it in &plan.items {
        let (action, note): (&str, &str) = match &it.action {
            PlanAction::Transfer => (verb, ""),
            PlanAction::Duplicate => ("duplicate", ""),
            PlanAction::Overwrite => ("overwrite", "exists"),
            PlanAction::Skip => ("skip", "exists"),
            PlanAction::NoOp => ("noop", "same folder"),
            PlanAction::Error(why) => ("error", why.as_str()),
        };
        match &it.action {
            PlanAction::Error(_) => failing += 1,
            PlanAction::Skip | PlanAction::NoOp => skipped += 1,
            _ => active += 1,
        }
        let _ = writeln!(
            out,
            "{action}\t{}\t{}\t{}\t{note}",
            it.bytes,
            it.src.display(),
            it.dest.display()
        );
    }
    let _ = out.flush();
    if !o.quiet {
        let _ = writeln!(
            err,
            "dry run: {active} to {verb} · {skipped} skipped · {failing} would fail · {} of {} planned · nothing changed",
            human_bytes(plan.active_bytes),
            human_bytes(plan.total_bytes)
        );
    }
    if failing == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
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
    /// 마지막으로 본 진행(누적 옮긴 바이트 · 총량) — 스로틀에 걸려 안 찍힌 줄을 항목 끝에 강제로 · 요약의 옮긴 양.
    seen: Option<(u64, u64)>,
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
            seen: None,
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
                self.seen = Some((p.done_bytes, p.total_bytes));
                let now = Instant::now();
                if self
                    .last
                    .is_some_and(|t| now.duration_since(t) < Duration::from_millis(100))
                {
                    return;
                }
                self.last = Some(now);
                self.progress_line(p.done_bytes, p.total_bytes, err);
            }
            Event::ItemEnd { index, status } => {
                // 스로틀에 걸려 마지막 진행(100 %)이 안 찍힌 채 끝나지 않게 한 번 강제로(협업 점검 10-24 ①).
                if let Some((done, total)) = self.seen.filter(|_| self.line_open) {
                    self.progress_line(done, total, err);
                }
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

    fn progress_line(&mut self, done: u64, total: u64, err: &mut dyn Write) {
        let _ = write!(
            err,
            "\r  {} / {} ({}%) · {}",
            human_bytes(done),
            human_bytes(total),
            percent(done, total),
            rate(done, self.started.elapsed())
        );
        let _ = err.flush();
        self.line_open = true;
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
        // 옮긴 바이트(마지막 진행 누적)와 계획 합계를 구분한다(전부 건너뛰면 0 of N · 협업 점검 10-24 ②).
        let moved = self.seen.map_or(0, |(d, _)| d);
        let _ = writeln!(
            err,
            "{} transferred · {} skipped · {} failed{} · {} of {} planned in {}",
            out.transferred.len(),
            out.skipped.len(),
            out.errors.len(),
            if out.canceled { " · canceled" } else { "" },
            human_bytes(moved),
            human_bytes(self.total),
            human_secs(elapsed)
        );
    }
}

thread_local! {
    /// `--ask`에서 `c`(취소)를 골랐다 — 결정 클로저가 취소 깃발을 세우게(답 읽기와 깃발 세우기를 한 자리에 두지 않은 이유 = 순수 함수 시험).
    static CANCEL_ASKED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 충돌 프롬프트(순수에 가까움 · 입출력은 인자로): `dest`가 이미 있다 → 한 줄 답. 돌려주는 값 = (결정, 남은 전부에 적용?).
/// `y` 덮어쓰기 · `n`/빈 줄/EOF/모르는 답 = 건너뜀 · `a` 전부 덮어쓰기 · `s` 전부 건너뜀 · `c` 취소(= 전부 건너뜀 + 취소 표시).
pub(crate) fn ask_conflict(
    dest: &Path,
    prompt: &mut dyn Write,
    answer: &mut dyn std::io::BufRead,
) -> (Conflict, bool) {
    let _ = write!(
        prompt,
        "exists: {} — overwrite? [y]es / [n]o / [a]ll / [s]kip all / [c]ancel: ",
        dest.display()
    );
    let _ = prompt.flush();
    let mut line = String::new();
    let _ = answer.read_line(&mut line);
    match line.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => (Conflict::Overwrite, false),
        "a" | "all" => (Conflict::Overwrite, true),
        "s" | "skip" => (Conflict::Skip, true),
        "c" | "cancel" => {
            CANCEL_ASKED.with(|c| c.set(true));
            (Conflict::Skip, true)
        }
        _ => (Conflict::Skip, false),
    }
}

/// 백분율(순수 · 총량 0 = 100).
pub(crate) fn percent(done: u64, total: u64) -> u64 {
    // 반올림(계획 합계에 폴더 자체 크기 등이 섞여 끝에서 99 %로 보이지 않게 · 총량 0 = 100).
    done.saturating_mul(100)
        .saturating_add(total / 2)
        .checked_div(total)
        .unwrap_or(100)
        .min(100)
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
        assert!(
            !o.overwrite && o.native && !o.quiet && !o.dry_run && !o.verify && !o.mirror && !o.yes
        );
        assert!(!o.ask);
        assert!(o.filter.is_empty());
        assert_eq!((o.threads, o.unbuffered_mb), (0, 0));
        assert_eq!(o.sources, vec![PathBuf::from("a"), PathBuf::from("b")]);
        assert_eq!(o.dest, PathBuf::from("dest"));
        let Ok(Parsed::Run(o)) = parse(&a(&[
            "--ask",
            "--yes",
            "--mirror",
            "--include",
            "*.txt",
            "--exclude",
            "*.tmp",
            "--verify",
            "-n",
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
        assert!(
            o.overwrite
                && !o.native
                && o.quiet
                && o.dry_run
                && o.verify
                && o.mirror
                && o.yes
                && o.ask
        );
        assert_eq!(o.filter.include, vec!["*.txt".to_string()]);
        assert_eq!(o.filter.exclude, vec!["*.tmp".to_string()]);
        if let Ok(Parsed::Run(o)) = parse(&a(&["--include", "*.txt; *.md;", "a", "b"])) {
            assert_eq!(
                o.filter.include,
                vec!["*.txt".to_string(), "*.md".to_string()],
                "`;` 분리"
            );
        } else {
            panic!("parse");
        }
        assert!(parse(&a(&["--include", " ; ", "a", "b"])).is_err());
        assert!(parse(&a(&["--include", "", "a", "b"])).is_err());
        assert!(parse(&a(&["--exclude"])).is_err());
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

    /// `--ask` 프롬프트(순수): 답 → 결정/전부 · 빈 줄·EOF·모르는 답 = 건너뜀 · 대소문자 무시 · 프롬프트에 경로.
    #[test]
    fn ask_conflict_maps_answers() {
        let p = Path::new("x/y.txt");
        let ask = |ans: &str| {
            let mut out: Vec<u8> = Vec::new();
            let mut inp = std::io::Cursor::new(ans.as_bytes().to_vec());
            let r = ask_conflict(p, &mut out, &mut inp);
            assert!(String::from_utf8_lossy(&out).contains("exists: x/y.txt"));
            r
        };
        assert_eq!(ask("y\n"), (Conflict::Overwrite, false));
        assert_eq!(ask("YES\n"), (Conflict::Overwrite, false));
        assert_eq!(ask("n\n"), (Conflict::Skip, false));
        assert_eq!(ask("\n"), (Conflict::Skip, false));
        assert_eq!(ask(""), (Conflict::Skip, false), "EOF = 건너뜀");
        assert_eq!(ask("a\n"), (Conflict::Overwrite, true));
        assert_eq!(ask("s\n"), (Conflict::Skip, true));
        CANCEL_ASKED.with(|c| c.set(false));
        assert_eq!(ask("c\n"), (Conflict::Skip, true));
        assert!(CANCEL_ASKED.with(|c| c.get()), "취소 표시");
        CANCEL_ASKED.with(|c| c.set(false));
    }

    #[test]
    fn human_readable_helpers() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(1023), "1023 B");
        assert_eq!(human_bytes(1536), "1.5 KiB");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5.0 MiB");
        assert_eq!(percent(0, 0), 100);
        assert_eq!(percent(1, 4), 25);
        assert_eq!(percent(199, 200), 100);
        assert_eq!(percent(1, 200), 1);
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

    /// `--dry-run`(10-24): stdout 탭 줄(action · bytes · src · dest · note) · 충돌 = skip/overwrite · 같은 폴더 = duplicate/noop ·
    /// 자기/하위 이동 = error → 종료 1 · 요약은 stderr · quiet면 없음 · **파일 시스템 불변**.
    #[test]
    fn dry_run_reports_plan_and_changes_nothing() {
        let d = sandbox("dry");
        let src = d.join("src");
        fs::create_dir_all(src.join("tree")).expect("mkdir");
        fs::write(src.join("tree").join("x.txt"), b"xx").expect("write");
        fs::write(src.join("one.txt"), b"one").expect("write");
        let dest = d.join("dest");
        fs::create_dir_all(&dest).expect("mkdir");
        fs::write(dest.join("one.txt"), b"OLD").expect("write");
        let base = |extra: &[&str]| {
            let mut v: Vec<&str> = extra.to_vec();
            v.push("--dry-run");
            let (t, o, dd) = (
                src.join("tree").to_string_lossy().into_owned(),
                src.join("one.txt").to_string_lossy().into_owned(),
                dest.to_string_lossy().into_owned(),
            );
            let mut args = a(&v);
            args.extend([t, o, dd]);
            match parse(&args) {
                Ok(Parsed::Run(o)) => o,
                other => panic!("{other:?}"),
            }
        };
        let (mut out, mut err): (Vec<u8>, Vec<u8>) = (Vec::new(), Vec::new());
        let o = base(&[]);
        assert_eq!(
            run_with(&o, &AtomicBool::new(false), &mut out, &mut err),
            ExitCode::SUCCESS
        );
        let text = String::from_utf8(out).expect("utf8");
        let lines: Vec<Vec<&str>> = text.lines().map(|l| l.split('\t').collect()).collect();
        assert_eq!(lines.len(), 2, "{text}");
        assert_eq!(&lines[0][..2], &["copy", "2"], "폴더 = 재귀 합");
        assert_eq!(lines[0][3], dest.join("tree").to_string_lossy());
        assert_eq!(&lines[1][..2], &["skip", "3"]);
        assert_eq!(lines[1][4], "exists");
        let summary = String::from_utf8(err).expect("utf8");
        assert!(
            summary.contains("1 to copy · 1 skipped · 0 would fail · 2 B of 5 B planned"),
            "{summary}"
        );
        assert!(!dest.join("tree").exists(), "dry-run은 복사하지 않는다");
        assert_eq!(fs::read(dest.join("one.txt")).expect("read"), b"OLD");
        // 덮어쓰기 + 이동 + quiet.
        let (mut out, mut err): (Vec<u8>, Vec<u8>) = (Vec::new(), Vec::new());
        let o = base(&["--overwrite", "--move", "-q"]);
        assert_eq!(
            run_with(&o, &AtomicBool::new(false), &mut out, &mut err),
            ExitCode::SUCCESS
        );
        let text = String::from_utf8(out).expect("utf8");
        assert!(
            text.starts_with("move\t2\t") && text.contains("\noverwrite\t3\t"),
            "{text}"
        );
        assert!(err.is_empty(), "quiet = 요약 없음");
        assert!(
            src.join("tree").exists() && src.join("one.txt").exists(),
            "이동 안 함"
        );
        // 자기 하위로 이동 = error → 1.
        let o = Options {
            op: Op::Move,
            sources: vec![src.clone()],
            dest: src.join("tree"),
            dry_run: true,
            ..o.clone()
        };
        let (mut out, mut err): (Vec<u8>, Vec<u8>) = (Vec::new(), Vec::new());
        assert_eq!(
            run_with(&o, &AtomicBool::new(false), &mut out, &mut err),
            ExitCode::from(1)
        );
        assert!(String::from_utf8_lossy(&out).starts_with("error\t"));
        let _ = fs::remove_dir_all(&d);
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
        assert!(text.contains("(100%)"), "마지막 진행 줄 강제 출력: {text}");
        assert!(text.contains("68.4 KiB of 68.4 KiB planned"), "{text}");
        // 기본 = 건너뜀(종료 0 · 요약에 skipped) · 덮어쓰기.
        fs::write(src.join("one.txt"), b"ONE").expect("write");
        let mut err: Vec<u8> = Vec::new();
        assert_eq!(run(&o, &mut err), ExitCode::SUCCESS);
        assert_eq!(fs::read(dest.join("one.txt")).expect("read"), b"one");
        let text = String::from_utf8_lossy(&err).into_owned();
        assert!(
            text.contains("2 skipped") && text.contains("0 B of 68.4 KiB planned"),
            "{text}"
        );
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
            ..o.clone()
        };
        assert_eq!(run(&bad2, &mut Vec::new()), ExitCode::from(2));
        // 취소 깃발이 이미 서 있으면 아무것도 옮기지 않고 130(Ctrl+C 경로의 종료 코드 · 요약에 canceled).
        let c = AtomicBool::new(true);
        let mut err: Vec<u8> = Vec::new();
        let src2 = d.join("src2");
        fs::create_dir_all(&src2).expect("mkdir");
        fs::write(src2.join("z.txt"), b"z").expect("write");
        let dest2 = d.join("dest2");
        fs::create_dir_all(&dest2).expect("mkdir");
        let oc = Options {
            sources: vec![src2.join("z.txt")],
            dest: dest2.clone(),
            quiet: false,
            ..o.clone()
        };
        assert_eq!(
            run_with(&oc, &c, &mut std::io::sink(), &mut err),
            ExitCode::from(130)
        );
        assert!(!dest2.join("z.txt").exists());
        assert!(String::from_utf8_lossy(&err).contains("canceled"));
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

    /// `--job`/`--progress`(10-31 T-97): 해석 · 간단 인자 → 명세 동등 · 실행 = JSON 진행 줄 + 마지막 Report 줄 · 명세 오류 = 2.
    #[test]
    fn job_and_progress_paths() {
        let p = parse(&["--job".into(), "j.json".into(), "--progress".into()]).unwrap();
        assert_eq!(
            p,
            Parsed::Job {
                file: PathBuf::from("j.json"),
                progress: true,
                quiet: false
            }
        );
        assert!(parse(&["--job".into(), "j.json".into(), "a".into(), "b".into()]).is_err());
        let Parsed::Run(o) = parse(&[
            "--move".into(),
            "--overwrite".into(),
            "--threads".into(),
            "2".into(),
            "--include".into(),
            "*.txt".into(),
            "--progress".into(),
            "/s".into(),
            "/d".into(),
        ])
        .unwrap() else {
            panic!("run")
        };
        assert!(o.progress);
        let spec = o.to_spec();
        assert_eq!(spec.op, Op::Move);
        assert_eq!(spec.conflict, ConflictPolicy::Overwrite);
        assert_eq!((spec.threads, spec.filter.include.len()), (2, 1));
        assert_eq!(spec.unbuffered_mb, 0, "간단 인자와 명세 기본 = 같다");
        // 실행: 임시 폴더 dry-run → stdout에 진행 JSON 줄 + Report 줄 · 종료 0.
        let d = std::env::temp_dir().join(format!("nexa-copy-job-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("dst")).unwrap();
        std::fs::write(d.join("a.txt"), b"hi").unwrap();
        let spec = JobSpec {
            sources: vec![d.join("a.txt")],
            dest: d.join("dst"),
            dry_run: true,
            ..JobSpec::default()
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_job_with(
            &spec,
            true,
            false,
            &AtomicBool::new(false),
            &mut out,
            &mut err,
        );
        assert_eq!(code, ExitCode::SUCCESS);
        let text = String::from_utf8(out).unwrap();
        let last = text.lines().last().unwrap();
        assert!(
            last.starts_with("{\"ok\":true") && last.contains("\"dry_run\":true"),
            "{text}"
        );
        // 명세 오류(대상 없음) = 2 · 사람용 요약은 stderr.
        let bad = JobSpec {
            dest: d.join("nope"),
            ..spec.clone()
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run_job_with(
            &bad,
            false,
            false,
            &AtomicBool::new(false),
            &mut out,
            &mut err,
        );
        assert_eq!(code, ExitCode::from(2));
        assert!(String::from_utf8(err).unwrap().contains("destination"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
