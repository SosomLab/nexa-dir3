//! **ndir-xfer** — 전송 엔진 겉모양(T-97 · 10-31 사용자 결정 ①): `ndir-ops`(복사·이동 `transfer_with` · 비교/미러 `compare` · 해시)를
//! **직렬화 가능한 명세·진행·결과**로 감싼다. 앱(진행 창) · `nexa-copy`(`--job` · `--progress`) · cdylib(C ABI `ndir_xfer_run(spec_json)
//! -> report_json` · `capi.rs` · T-202 ①)이 같은 [`JobSpec`] → [`run`] → [`Report`]를 쓴다. 설계 = docs/26-xfer-design.md.
//!
//! - 외부 crate 0(DR-8) · JSON은 `json.rs`(ndir-settings 사본) · 엔진 규칙(같은 폴더 · 충돌 · 스테이징 · 교차 볼륨 · OS 복사 경로)은 ndir-ops 그대로.
//! - 단계 = plan(계획 · dry-run은 여기서 끝) → transfer(복사/이동) → verify(크기 + SHA-256) / mirror(비교 → 복사 → 삭제).
//! - 취소 = `AtomicBool`(협력적) · 진행 = 콜백(`&mut dyn FnMut(&Progress)`) · 항목별 실패 격리(배치는 계속).

pub mod capi;
pub mod json;
pub mod mirror;
pub mod report;
pub mod spec;
pub mod verify;

pub use report::{ItemReport, Phase, Progress, Report, VerifySummary};
pub use spec::{ConflictPolicy, JobSpec};

use ndir_ops::fastcopy::{self, Tuning};
use ndir_ops::{plan_transfer, transfer_with, Event, ItemStatus, Op, PlanAction};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

fn verb(op: Op) -> &'static str {
    match op {
        Op::Copy => "copy",
        Op::Move => "move",
    }
}

/// 명세 사전 검증(파일 시스템 존재만) — 오류 글(없으면 None).
#[must_use]
pub fn validate(spec: &JobSpec) -> Option<String> {
    if !spec.dest.is_dir() {
        return Some(format!(
            "destination is not an existing folder: {}",
            spec.dest.display()
        ));
    }
    if let Some(p) = spec.sources.iter().find(|p| !p.exists()) {
        return Some(format!("source not found: {}", p.display()));
    }
    if spec.mirror {
        let [src] = spec.sources.as_slice() else {
            return Some("mirror needs exactly one source folder".into());
        };
        if !src.is_dir() {
            return Some(format!("mirror source is not a folder: {}", src.display()));
        }
        if ndir_ops::is_same_or_sub(src, &spec.dest) || ndir_ops::is_same_or_sub(&spec.dest, src) {
            return Some("mirror source and destination overlap".into());
        }
        if let Some(why) = mirror::dangerous_dest(&spec.dest, mirror::home_dir().as_deref()) {
            return Some(format!("mirror refused: destination is {why}"));
        }
    }
    None
}

/// 작업 실행 — 명세대로 계획/전송/검증/미러 · 진행은 `on_progress` · 취소는 `cancel`. 오류는 [`Report::errors`]로(패닉 없음).
pub fn run(spec: &JobSpec, on_progress: &mut dyn FnMut(&Progress), cancel: &AtomicBool) -> Report {
    let started = Instant::now();
    let mut rep = Report {
        ok: true,
        dry_run: spec.dry_run,
        ..Report::default()
    };
    if let Some(e) = validate(spec) {
        rep.ok = false;
        rep.errors.push(e);
        return finish(rep, started);
    }
    if spec.mirror {
        run_mirror(spec, &mut rep, on_progress, cancel);
        return finish(rep, started);
    }
    // 최상위 원본 필터(폴더 안은 엔진 계획 단계 `with_filter`).
    let sources: Vec<PathBuf> = spec
        .sources
        .iter()
        .filter(|p| spec.filter.allows(&ndir_ops::leaf_name(p), p.is_dir()))
        .cloned()
        .collect();
    let mut resolve = |_: &std::path::Path| spec.conflict.resolve();
    if spec.dry_run {
        let plan = plan_transfer(&sources, &spec.dest, spec.op, &mut resolve);
        for it in &plan.items {
            let (action, status, note) = match &it.action {
                PlanAction::Transfer => (verb(spec.op), "planned", String::new()),
                PlanAction::Duplicate => ("duplicate", "planned", String::new()),
                PlanAction::Overwrite => ("overwrite", "planned", "exists".into()),
                PlanAction::Skip => ("skip", "skipped", "exists".into()),
                PlanAction::NoOp => ("noop", "skipped", "same folder".into()),
                PlanAction::Error(why) => ("error", "failed", why.clone()),
            };
            match status {
                "failed" => rep.failed += 1,
                "skipped" => rep.skipped += 1,
                _ => {}
            }
            rep.items.push(ItemReport {
                action: action.into(),
                status: status.into(),
                bytes: it.bytes,
                source: it.src.clone(),
                dest: it.dest.clone(),
                note,
            });
        }
        rep.transferred_bytes = plan.active_bytes;
        rep.ok = rep.failed == 0;
        return finish(rep, started);
    }
    let tuning = Tuning {
        native: spec.native,
        unbuffered_min: spec.unbuffered_mb.saturating_mul(1024 * 1024),
        threads: spec.threads,
    };
    let mut sizes: Vec<u64> = Vec::new();
    let mut total = 0u64;
    let mut dests: Vec<PathBuf> = vec![PathBuf::new(); sources.len()];
    let mut statuses: Vec<Option<ItemStatus>> = vec![None; sources.len()];
    let mut on_event = |ev: Event| match ev {
        Event::Plan {
            sizes: s,
            total_bytes,
        } => {
            sizes = s.to_vec();
            total = total_bytes;
            on_progress(&Progress {
                phase: Phase::Plan,
                done_bytes: 0,
                total_bytes,
                item_index: 0,
                item_count: s.len(),
                current: None,
            });
        }
        Event::ItemStart { index, dest } => {
            if let Some(d) = dests.get_mut(index) {
                *d = dest.to_path_buf();
            }
        }
        Event::Bytes(p) => on_progress(&Progress {
            phase: Phase::Transfer,
            done_bytes: p.done_bytes,
            total_bytes: p.total_bytes,
            item_index: p.item_index,
            item_count: p.item_count,
            current: dests
                .get(p.item_index)
                .filter(|d| !d.as_os_str().is_empty())
                .cloned(),
        }),
        Event::ItemEnd { index, status } => {
            if let Some(s) = statuses.get_mut(index) {
                *s = Some(status);
            }
        }
    };
    let out = fastcopy::with_filter(spec.filter.clone(), || {
        transfer_with(
            &sources,
            &spec.dest,
            spec.op,
            &mut resolve,
            &mut on_event,
            cancel,
            &tuning,
        )
    });
    rep.canceled = out.canceled;
    for (i, src) in sources.iter().enumerate() {
        let bytes = sizes.get(i).copied().unwrap_or(0);
        let dest = dests.get(i).cloned().unwrap_or_default();
        let (status, note) = if let Some((_, why)) = out.errors.iter().find(|(p, _)| p == src) {
            rep.failed += 1;
            ("failed", why.clone())
        } else if out.skipped.iter().any(|p| p == src) {
            rep.skipped += 1;
            ("skipped", "exists".to_string())
        } else if out.transferred.iter().any(|(s, _)| s == src) {
            rep.transferred_bytes += bytes;
            ("done", String::new())
        } else if out.canceled {
            ("canceled", String::new())
        } else {
            ("skipped", "same folder".to_string())
        };
        let dest = if dest.as_os_str().is_empty() {
            out.transferred
                .iter()
                .find(|(s, _)| s == src)
                .map(|(_, d)| d.clone())
                .unwrap_or_default()
        } else {
            dest
        };
        rep.items.push(ItemReport {
            action: verb(spec.op).into(),
            status: status.into(),
            bytes,
            source: src.clone(),
            dest,
            note,
        });
    }
    if spec.verify && spec.op == Op::Copy && !out.canceled {
        let total: u64 = out
            .transferred
            .iter()
            .map(|(s, _)| ndir_ops::size_of(s))
            .sum::<u64>()
            .saturating_mul(2);
        let mut read = 0u64;
        let mut on_bytes = |n: u64| {
            read += n;
            on_progress(&Progress {
                phase: Phase::Verify,
                done_bytes: read,
                total_bytes: total,
                item_index: 0,
                item_count: 0,
                current: None,
            });
        };
        match verify::verify_pairs(&out.transferred, &mut on_bytes, cancel) {
            Ok(v) => {
                if !v.ok() {
                    rep.ok = false;
                }
                rep.verify = Some(VerifySummary {
                    files: v.files,
                    bytes: v.bytes,
                    mismatches: v.failures,
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => rep.canceled = true,
            Err(e) => {
                rep.ok = false;
                rep.errors.push(format!("verify error: {e}"));
            }
        }
    }
    if rep.failed > 0 || rep.canceled {
        rep.ok = false;
    }
    finish(rep, started)
}

fn run_mirror(
    spec: &JobSpec,
    rep: &mut Report,
    on_progress: &mut dyn FnMut(&Progress),
    cancel: &AtomicBool,
) {
    let src = &spec.sources[0];
    let plan = match mirror::mirror_plan(src, &spec.dest, &spec.filter, cancel) {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
            rep.canceled = true;
            rep.ok = false;
            return;
        }
        Err(e) => {
            rep.ok = false;
            rep.errors.push(format!("compare failed: {e}"));
            return;
        }
    };
    for (s, d, over) in &plan.copies {
        let bytes = std::fs::metadata(s).map_or(0, |m| if m.is_dir() { 0 } else { m.len() });
        rep.items.push(ItemReport {
            action: if *over { "overwrite" } else { "copy" }.into(),
            status: "planned".into(),
            bytes,
            source: s.clone(),
            dest: d.clone(),
            note: String::new(),
        });
    }
    for d in &plan.deletes {
        rep.items.push(ItemReport {
            action: "delete".into(),
            status: "planned".into(),
            bytes: 0,
            source: PathBuf::new(),
            dest: d.clone(),
            note: String::new(),
        });
    }
    rep.transferred_bytes = plan.bytes;
    if spec.dry_run {
        return;
    }
    if !plan.deletes.is_empty() && !spec.yes {
        rep.ok = false;
        rep.errors.push(format!(
            "mirror would permanently delete {} item(s) in the destination; set yes to confirm (or dry_run to review)",
            plan.deletes.len()
        ));
        return;
    }
    let total = plan.bytes;
    let mut done = 0u64;
    let mut on_bytes = |n: u64| {
        done += n;
        on_progress(&Progress {
            phase: Phase::Mirror,
            done_bytes: done,
            total_bytes: total,
            item_index: 0,
            item_count: 0,
            current: None,
        });
    };
    let out = match mirror::apply_mirror(&plan, &mut on_bytes, cancel) {
        Ok(o) => o,
        Err(e) => {
            rep.ok = false;
            rep.errors.push(format!("mirror failed: {e}"));
            return;
        }
    };
    rep.canceled = out.canceled;
    for it in &mut rep.items {
        if it.action == "delete" {
            if out.deleted.contains(&it.dest) {
                it.status = "done".into();
            } else if let Some((_, why)) = out.delete_failed.iter().find(|(d, _)| *d == it.dest) {
                it.status = "failed".into();
                it.note = why.clone();
            } else {
                it.status = "canceled".into();
            }
        } else if let Some((_, why)) = out.failed.iter().find(|(d, _)| *d == it.dest) {
            it.status = "failed".into();
            it.note = why.clone();
        } else if !out.canceled {
            it.status = "done".into();
        } else {
            it.status = "canceled".into();
        }
    }
    rep.deleted = out.deleted.len();
    rep.failed = out.failed.len() + out.delete_failed.len();
    if spec.verify && !out.canceled {
        let pairs: Vec<(PathBuf, PathBuf)> = plan
            .copies
            .iter()
            .filter(|(s, _, _)| s.is_file())
            .map(|(s, d, _)| (s.clone(), d.clone()))
            .collect();
        match verify::verify_pairs(&pairs, &mut |_| {}, cancel) {
            Ok(v) => {
                if !v.ok() {
                    rep.ok = false;
                }
                rep.verify = Some(VerifySummary {
                    files: v.files,
                    bytes: v.bytes,
                    mismatches: v.failures,
                });
            }
            Err(e) => {
                rep.ok = false;
                rep.errors.push(format!("verify error: {e}"));
            }
        }
    }
    if rep.failed > 0 || rep.canceled {
        rep.ok = false;
    }
}

fn finish(mut rep: Report, started: Instant) -> Report {
    rep.elapsed_ms = started.elapsed().as_millis() as u64;
    rep
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sandbox(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ndir-xfer-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("src/dir")).unwrap();
        fs::create_dir_all(d.join("dst")).unwrap();
        fs::write(d.join("src/a.txt"), b"hello").unwrap();
        fs::write(d.join("src/dir/b.bin"), vec![7u8; 3000]).unwrap();
        d
    }

    /// dry-run → 계획만(파일 불변) · 실행 → 복사 + 검증 · 진행 콜백이 단계를 지난다.
    #[test]
    fn dry_run_then_copy_with_verify() {
        let d = sandbox("copy");
        let mut spec = JobSpec {
            sources: vec![d.join("src/a.txt"), d.join("src/dir")],
            dest: d.join("dst"),
            dry_run: true,
            verify: true,
            ..JobSpec::default()
        };
        let cancel = AtomicBool::new(false);
        let r = run(&spec, &mut |_| {}, &cancel);
        assert!(r.ok && r.dry_run && r.items.len() == 2, "{r:?}");
        assert!(r
            .items
            .iter()
            .all(|i| i.status == "planned" && i.action == "copy"));
        assert!(!d.join("dst/a.txt").exists(), "dry-run = 불변");
        spec.dry_run = false;
        let mut phases = Vec::new();
        let r = run(&spec, &mut |p| phases.push(p.phase), &cancel);
        assert!(r.ok, "{r:?}");
        assert!(d.join("dst/a.txt").is_file() && d.join("dst/dir/b.bin").is_file());
        assert_eq!(r.transferred_bytes, 3005);
        assert!(r.items.iter().all(|i| i.status == "done"), "{:?}", r.items);
        assert!(
            phases.contains(&Phase::Plan) && phases.contains(&Phase::Verify),
            "{phases:?}"
        );
        let v = r.verify.as_ref().unwrap();
        assert_eq!((v.files, v.mismatches.len()), (2, 0));
        // 두 번째 = 충돌 skip 정책 → skipped.
        let r = run(&spec, &mut |_| {}, &cancel);
        assert!(r.ok && r.skipped == 2, "{r:?}");
        // JSON 왕복.
        assert!(json::parse(&r.to_json()).is_ok());
        let _ = fs::remove_dir_all(&d);
    }

    /// 미러: 삭제가 있으면 yes 없이는 거부(오류) · dry_run은 계획 · yes면 복사 + 삭제.
    #[test]
    fn mirror_needs_yes_for_deletes() {
        let d = sandbox("mirror");
        fs::write(d.join("dst/extra.txt"), b"x").unwrap();
        let mut spec = JobSpec {
            sources: vec![d.join("src")],
            dest: d.join("dst"),
            mirror: true,
            ..JobSpec::default()
        };
        let cancel = AtomicBool::new(false);
        let r = run(&spec, &mut |_| {}, &cancel);
        assert!(!r.ok && r.errors[0].contains("permanently delete"), "{r:?}");
        assert!(d.join("dst/extra.txt").exists(), "거부 = 불변");
        spec.dry_run = true;
        let r = run(&spec, &mut |_| {}, &cancel);
        assert!(
            r.ok && r.items.iter().any(|i| i.action == "delete"),
            "{r:?}"
        );
        spec.dry_run = false;
        spec.yes = true;
        let r = run(&spec, &mut |_| {}, &cancel);
        assert!(r.ok && r.deleted == 1, "{r:?}");
        assert!(!d.join("dst/extra.txt").exists() && d.join("dst/a.txt").is_file());
        let _ = fs::remove_dir_all(&d);
    }

    /// 검증 실패: 대상 없음 · 원본 없음 · 미러 원본이 파일.
    #[test]
    fn validation_errors() {
        let d = sandbox("validate");
        let cancel = AtomicBool::new(false);
        let bad = JobSpec {
            sources: vec![d.join("src/a.txt")],
            dest: d.join("nope"),
            ..JobSpec::default()
        };
        assert!(run(&bad, &mut |_| {}, &cancel).errors[0].contains("destination"));
        let bad = JobSpec {
            sources: vec![d.join("src/missing")],
            dest: d.join("dst"),
            ..JobSpec::default()
        };
        assert!(run(&bad, &mut |_| {}, &cancel).errors[0].contains("source not found"));
        let bad = JobSpec {
            sources: vec![d.join("src/a.txt")],
            dest: d.join("dst"),
            mirror: true,
            ..JobSpec::default()
        };
        assert!(run(&bad, &mut |_| {}, &cancel).errors[0].contains("not a folder"));
        let _ = fs::remove_dir_all(&d);
    }
}
