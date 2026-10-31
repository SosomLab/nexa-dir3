//! 미러(`--mirror` · T-97 "미러/차등" CLI 1차 · 10-25): 대상 폴더를 원본 폴더와 같게 — 엔진 `compare::compare`(크기 + 수정 시각 · 허용 2초 ·
//! 재귀) → `compare::plan(LeftToRight · mirror)` → 복사(`apply_copies` = 새것/더 새것/다른 것) + 대상에만 있는 것 **영구 삭제**(복사가 끝난 뒤 ·
//! 휴지통 없음 — CLI). `--dry-run` = 계획만(탭 줄 · stdout) · `--include/--exclude` = 상대 경로의 이름으로 거른다 · `--verify` = 복사한 파일 대조.

use crate::cli::{human_bytes, human_secs, Options};
use ndir_ops::compare::{self, SyncPlan};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

// 계획 · 구분자 정규화 · 홈 · 위험 대상 판정은 ndir-xfer(`mirror` 모듈)로 옮겼다(10-31 T-97 — 앱·CLI·cdylib 공용).
pub(crate) use ndir_xfer::mirror::{dangerous_dest, home_dir, mirror_plan_with};

/// 계획을 탭 줄로(dry-run · `action · bytes · source · destination`): copy · overwrite · delete.
pub(crate) fn plan_rows(plan: &SyncPlan) -> Vec<String> {
    let mut rows = Vec::with_capacity(plan.copies.len() + plan.deletes.len());
    for (s, d, over) in &plan.copies {
        let bytes = std::fs::metadata(s).map_or(0, |m| if m.is_dir() { 0 } else { m.len() });
        rows.push(format!(
            "{}\t{bytes}\t{}\t{}",
            if *over { "overwrite" } else { "copy" },
            s.display(),
            d.display()
        ));
    }
    for d in &plan.deletes {
        rows.push(format!("delete\t0\t\t{}", d.display()));
    }
    rows
}

/// `--mirror` 실행 — 원본 폴더 1개 · 대상 폴더(둘 다 있어야) · dry-run이면 계획만.
pub(crate) fn run_mirror(
    o: &Options,
    cancel: &AtomicBool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> ExitCode {
    let [src] = o.sources.as_slice() else {
        let _ = writeln!(err, "nexa-copy: --mirror needs exactly one source folder");
        return ExitCode::from(2);
    };
    if !src.is_dir() {
        let _ = writeln!(
            err,
            "nexa-copy: --mirror source is not a folder: {}",
            src.display()
        );
        return ExitCode::from(2);
    }
    if ndir_ops::is_same_or_sub(src, &o.dest) || ndir_ops::is_same_or_sub(&o.dest, src) {
        let _ = writeln!(err, "nexa-copy: --mirror source and destination overlap");
        return ExitCode::from(2);
    }
    // 안전장치 ①(협업 10-25 · robocopy /MIR 사고 유형): 대상이 드라이브 루트·사용자 홈이면 거부 — 대상 경로 실수 한 번이 복구 불가 삭제가 된다.
    if let Some(why) = dangerous_dest(&o.dest, home_dir().as_deref()) {
        let _ = writeln!(err, "nexa-copy: --mirror refused: destination is {why}");
        return ExitCode::from(2);
    }
    let started = Instant::now();
    // `--by-content`(T-202 ⑦): 해시 진행은 사람용 출력에 안 보인다(기계용은 `--job`/`--progress` 경로의 plan 줄).
    let plan = match mirror_plan_with(src, &o.dest, &o.filter, o.by_content, &mut |_| {}, cancel) {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
            let _ = writeln!(err, "canceled");
            return ExitCode::from(130);
        }
        Err(e) => {
            let _ = writeln!(err, "nexa-copy: compare failed: {e}");
            return ExitCode::from(1);
        }
    };
    // 안전장치 ②: 지울 것이 있으면 `--yes`가 있어야 한다(비대화형 CLI — 프롬프트 대신 목록 + 종료 2 · `--dry-run`으로 검토).
    if !o.dry_run && !plan.deletes.is_empty() && !o.yes {
        for d in &plan.deletes {
            let _ = writeln!(err, "would delete: {}", d.display());
        }
        let _ = writeln!(
            err,
            "nexa-copy: --mirror would permanently delete {} item(s) in the destination; re-run with --yes to confirm (or --dry-run to review)",
            plan.deletes.len()
        );
        return ExitCode::from(2);
    }
    if o.dry_run {
        for row in plan_rows(&plan) {
            let _ = writeln!(out, "{row}");
        }
        let _ = out.flush();
        if !o.quiet {
            let _ = writeln!(
                err,
                "dry run (mirror): {} to copy · {} to delete · {} · nothing changed",
                plan.copies.len(),
                plan.deletes.len(),
                human_bytes(plan.bytes)
            );
        }
        return ExitCode::SUCCESS;
    }
    let total = plan.bytes;
    let mut done = 0u64;
    let mut last = Instant::now();
    let mut on_bytes = |n: u64| {
        done += n;
        if !o.quiet && last.elapsed() >= Duration::from_millis(250) {
            last = Instant::now();
            let _ = write!(
                err,
                "\rmirroring · {} of {}",
                human_bytes(done),
                human_bytes(total)
            );
            let _ = err.flush();
        }
    };
    // 속도 제한(`--max-bps` · T-202 ③): 미러 복사도 같은 제한기.
    let failed = match ndir_ops::fastcopy::with_pacer(o.max_bps, || {
        compare::apply_copies(&plan, &mut on_bytes, cancel)
    }) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
            let _ = writeln!(err, "\ncanceled");
            return ExitCode::from(130);
        }
        Err(e) => {
            let _ = writeln!(err, "\nnexa-copy: mirror failed: {e}");
            return ExitCode::from(1);
        }
    };
    if !o.quiet && done > 0 {
        let _ = writeln!(err);
    }
    for (p, why) in &failed {
        let _ = writeln!(err, "error: {}: {why}", p.display());
    }
    // 삭제는 복사가 끝난 뒤(복사 실패가 있어도 대상에만 있는 것은 지운다 — 미러 의미) · 취소 깃발이 섰으면 멈춘다.
    let mut deleted = 0usize;
    let mut del_failed = 0usize;
    for d in &plan.deletes {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = writeln!(err, "canceled");
            return ExitCode::from(130);
        }
        match ndir_ops::delete_permanent(d) {
            Ok(()) => {
                deleted += 1;
                if !o.quiet {
                    let _ = writeln!(err, "deleted: {}", d.display());
                }
            }
            Err(e) => {
                del_failed += 1;
                let _ = writeln!(err, "error: {}: {e}", d.display());
            }
        }
    }
    let mut mismatches = 0usize;
    if o.verify {
        let pairs: Vec<(PathBuf, PathBuf)> = plan
            .copies
            .iter()
            .filter(|(s, _, _)| s.is_file())
            .map(|(s, d, _)| (s.clone(), d.clone()))
            .collect();
        match ndir_xfer::verify::verify_pairs(&pairs, &mut |_| {}, cancel) {
            Ok(rep) => {
                for (p, why) in &rep.failures {
                    let _ = writeln!(err, "verify failed: {}: {why}", p.display());
                }
                mismatches = rep.failures.len();
                if !o.quiet {
                    let _ = writeln!(
                        err,
                        "verified {} file(s) · {} mismatch(es)",
                        rep.files, mismatches
                    );
                }
            }
            Err(e) => {
                let _ = writeln!(err, "verify error: {e}");
                mismatches += 1;
            }
        }
    }
    if !o.quiet {
        // "copied"는 파일만(계획의 폴더 항목은 만들기만 한다).
        let copied_files = plan
            .copies
            .iter()
            .filter(|(s, _, _)| !s.is_dir())
            .count()
            .saturating_sub(failed.len());
        let _ = writeln!(
            err,
            "mirror: {} copied · {} deleted · {} failed · {} in {}",
            copied_files,
            deleted,
            failed.len() + del_failed,
            human_bytes(total),
            human_secs(started.elapsed())
        );
    }
    if failed.is_empty() && del_failed == 0 && mismatches == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndir_ops::fastcopy::NameFilter;

    fn sandbox(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nexa-copy-mirror-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// 안전장치 ①(순수): 루트 · 홈 = 거부 · 보통 폴더 = 통과.
    #[test]
    fn dangerous_dest_rejects_root_and_home() {
        let home = std::env::temp_dir().join(format!("nexa-copy-home-{}", std::process::id()));
        std::fs::create_dir_all(home.join("sub")).unwrap();
        assert_eq!(
            dangerous_dest(&home, Some(&home)),
            Some("the user home folder")
        );
        assert_eq!(
            dangerous_dest(&home.join("sub"), Some(&home)),
            None,
            "홈 안의 폴더는 됨"
        );
        assert_eq!(dangerous_dest(&home.join("sub"), None), None);
        let root = if cfg!(windows) {
            PathBuf::from("C:\\")
        } else {
            PathBuf::from("/")
        };
        assert_eq!(
            dangerous_dest(&root, None),
            Some("a drive or file system root")
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    /// 계획 = 새것 copy · 더 새것/다른 것 overwrite · 대상에만 있는 것 delete · 같은 것 없음 · 필터(exclude) 반영 · 탭 줄.
    #[test]
    fn mirror_plan_and_rows() {
        let d = sandbox("plan");
        let (s, t) = (d.join("s"), d.join("t"));
        std::fs::create_dir_all(s.join("sub")).unwrap();
        std::fs::create_dir_all(t.join("sub")).unwrap();
        std::fs::write(s.join("same.txt"), b"same").unwrap();
        std::fs::write(t.join("same.txt"), b"same").unwrap();
        std::fs::write(s.join("new.txt"), b"new").unwrap();
        std::fs::write(s.join("sub").join("differ.txt"), b"longer content").unwrap();
        std::fs::write(t.join("sub").join("differ.txt"), b"short").unwrap();
        std::fs::write(t.join("extra.tmp"), b"extra").unwrap();
        std::fs::write(s.join("skip.log"), b"log").unwrap();
        let cancel = AtomicBool::new(false);
        let plan =
            mirror_plan_with(&s, &t, &NameFilter::default(), false, &mut |_| {}, &cancel).unwrap();
        let rows = plan_rows(&plan);
        let acts: Vec<&str> = rows.iter().map(|r| r.split('\t').next().unwrap()).collect();
        assert_eq!(acts.iter().filter(|a| **a == "copy").count(), 2, "{rows:?}"); // new.txt · skip.log
        assert_eq!(acts.iter().filter(|a| **a == "overwrite").count(), 1);
        assert_eq!(acts.iter().filter(|a| **a == "delete").count(), 1);
        assert!(rows.iter().any(|r| r.ends_with("extra.tmp")));
        let sep = std::path::MAIN_SEPARATOR;
        assert!(
            plan.copies.iter().all(|(a, b, _)| {
                let (a, b) = (a.to_string_lossy(), b.to_string_lossy());
                !a.contains(if sep == '/' { '\\' } else { '/' })
                    && !b.contains(if sep == '/' { '\\' } else { '/' })
            }),
            "구분자 섞임 없음: {:?}",
            plan.copies
        );
        let f = NameFilter {
            include: vec![],
            exclude: vec!["*.log".into(), "*.tmp".into()],
        };
        let plan = mirror_plan_with(&s, &t, &f, false, &mut |_| {}, &cancel).unwrap();
        assert_eq!(plan.copies.len(), 2, "{plan:?}"); // new.txt · differ.txt
        assert!(plan.deletes.is_empty(), "제외된 extra.tmp는 지우지 않는다");
        let _ = std::fs::remove_dir_all(&d);
    }
}
