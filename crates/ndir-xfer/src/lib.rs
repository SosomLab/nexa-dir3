//! **ndir-xfer** — 전송 엔진 겉모양(T-97 · 10-31 사용자 결정 ①): `ndir-ops`(복사·이동 `transfer_with` · 비교/미러 `compare` · 해시)를
//! **직렬화 가능한 명세·진행·결과**로 감싼다. 앱(진행 창) · `nexa-copy`(`--job` · `--progress`) · cdylib(C ABI `ndir_xfer_run(spec_json)
//! -> report_json` · `capi.rs` · T-202 ①)이 같은 [`JobSpec`] → [`run`] → [`Report`]를 쓴다. 설계 = docs/26-xfer-design.md.
//!
//! - 외부 crate 0(DR-8) · JSON은 `json.rs`(ndir-settings 사본) · 엔진 규칙(같은 폴더 · 충돌 · 스테이징 · 교차 볼륨 · OS 복사 경로)은 ndir-ops 그대로.
//! - 단계 = plan(계획 · dry-run은 여기서 끝) → transfer(복사/이동) → verify(크기 + SHA-256) / mirror(비교 → 복사 → 삭제).
//! - 취소 = `AtomicBool`(협력적) · 진행 = 콜백(`&mut dyn FnMut(&Progress)`) · 항목별 실패 격리(배치는 계속).
//! - 재개(T-202 ② · `resume.rs`): `journal` = 최상위 항목 완료마다 한 줄 · `resume` = 이전 Report/저널의 `done` 쌍 건너뜀.
//! - 미러(T-202 ⑦): `mirror_by_content` = 크기·시각 같은 파일도 해시 대조 · [`run_with`]의 `delete` 콜백 = 휴지통 미러(앱이 포트 주입 · 기본 영구 삭제).

pub mod capi;
pub mod json;
pub mod mirror;
pub mod report;
pub mod resume;
pub mod spec;
/// 복사 검증 — ndir-ops로 옮김(T-202 ⑤ · 이동 검증이 엔진 안에서 쓴다) · 경로 호환을 위해 재수출.
pub use ndir_ops::verify;

pub use report::{ItemReport, Phase, Progress, Report, VerifySummary};
pub use spec::{ConflictPolicy, JobSpec};

use ndir_ops::fastcopy::{self, Tuning};
use ndir_ops::{plan_transfer, transfer_with, Event, ItemStatus, Op, PlanAction};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

fn verb(op: Op) -> &'static str {
    match op {
        Op::Copy => "copy",
        Op::Move => "move",
    }
}

/// 명세 사전 검증(파일 시스템 존재만) — 오류 글(없으면 None). `resume`이 있으면 그 파일도 읽어 이미 끝난 항목은
/// 존재 검사에서 뺀다(이동 재개 = 원본이 이미 없다 · 협업 지적 10-31 ①).
#[must_use]
pub fn validate(spec: &JobSpec) -> Option<String> {
    match (resumed(spec), interrupted(spec)) {
        (Ok(done), Ok(cut)) => validate_with(spec, &done, &cut),
        (Err(e), _) | (_, Err(e)) => Some(e),
    }
}

/// 재개 입력 읽기 — `resume` 없음 또는 미러(`validate_with`가 거른다) = 빈 목록.
fn resumed(spec: &JobSpec) -> Result<Vec<ItemReport>, String> {
    resume_read(spec, resume::done_items)
}

/// 재개 입력의 끊긴 항목(`started`만 · GAP-030) — 없으면 빈 목록.
fn interrupted(spec: &JobSpec) -> Result<Vec<ItemReport>, String> {
    resume_read(spec, resume::interrupted_items)
}

fn resume_read(
    spec: &JobSpec,
    pick: fn(&str) -> Result<Vec<ItemReport>, String>,
) -> Result<Vec<ItemReport>, String> {
    let Some(path) = &spec.resume else {
        return Ok(Vec::new());
    };
    if spec.mirror {
        return Ok(Vec::new());
    }
    std::fs::read_to_string(path)
        .map_err(|e| e.to_string())
        .and_then(|t| pick(&t))
        .map_err(|e| format!("resume: {}: {e}", path.display()))
}

/// `src`가 이 명세에서 이미 끝난 항목인가 — (source, dest의 부모 = `spec.dest`) 쌍으로 본다(대상을 바꿔 다시 돌리면 다시 복사 · 협업 ④).
fn resumed_item<'a>(done: &'a [ItemReport], spec: &JobSpec, src: &Path) -> Option<&'a ItemReport> {
    done.iter()
        .find(|i| i.source == src && i.dest.parent().is_some_and(|p| p == spec.dest))
}

fn validate_with(spec: &JobSpec, done: &[ItemReport], cut: &[ItemReport]) -> Option<String> {
    if !spec.dest.is_dir() {
        return Some(format!(
            "destination is not an existing folder: {}",
            spec.dest.display()
        ));
    }
    if spec.mirror && (spec.journal.is_some() || spec.resume.is_some()) {
        return Some(
            "journal/resume do not apply to mirror (a mirror re-compares; just run it again)"
                .into(),
        );
    }
    // 끝난 항목 · 끊긴 항목(GAP-030: 이동 중 죽어 원본이 이미 없을 수 있음)은 존재 검사에서 뺀다.
    if let Some(p) = spec.sources.iter().find(|p| {
        !p.exists() && resumed_item(done, spec, p).is_none() && resumed_item(cut, spec, p).is_none()
    }) {
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

/// 미러 삭제 콜백(T-202 ⑦): 대상에만 있는 항목의 경로를 받아 지운다(휴지통 포트 등) · `Err` = 그 항목 실패.
pub type DeleteFn<'a> = &'a mut dyn FnMut(&Path) -> std::io::Result<()>;

/// 작업 실행 — 명세대로 계획/전송/검증/미러 · 진행은 `on_progress` · 취소는 `cancel`. 오류는 [`Report::errors`]로(패닉 없음).
/// 미러 삭제는 영구 삭제([`run_with`]에 콜백을 주면 휴지통 등으로 바꿀 수 있다).
pub fn run(spec: &JobSpec, on_progress: &mut dyn FnMut(&Progress), cancel: &AtomicBool) -> Report {
    run_with(spec, on_progress, cancel, None)
}

/// [`run`] + 미러 삭제 콜백(`delete` · None = 영구 삭제). CLI · cdylib은 [`run`] · 앱은 휴지통 포트를 넣는다(T-202 ⑦).
pub fn run_with(
    spec: &JobSpec,
    on_progress: &mut dyn FnMut(&Progress),
    cancel: &AtomicBool,
    delete: Option<DeleteFn<'_>>,
) -> Report {
    let started = Instant::now();
    let mut rep = Report {
        ok: true,
        dry_run: spec.dry_run,
        ..Report::default()
    };
    let (done, interrupted) = match (resumed(spec), interrupted(spec)) {
        (Ok(d), Ok(c)) => (d, c),
        (Err(e), _) | (_, Err(e)) => {
            rep.ok = false;
            rep.errors.push(e);
            return finish(rep, started);
        }
    };
    if let Some(e) = validate_with(spec, &done, &interrupted) {
        rep.ok = false;
        rep.errors.push(e);
        return finish(rep, started);
    }
    if spec.mirror {
        run_mirror(spec, &mut rep, on_progress, cancel, delete);
        return finish(rep, started);
    }
    // 재개(T-202 ②): 이미 끝난 (source, dest) 쌍 = skipped/resumed(먼저 적힘) · 그 밖은 최상위 원본 필터(폴더 안은 엔진 계획 단계 `with_filter`).
    let mut sources: Vec<PathBuf> = Vec::new();
    for p in &spec.sources {
        if let Some(prev) = resumed_item(&done, spec, p) {
            rep.skipped += 1;
            rep.items.push(ItemReport {
                action: verb(spec.op).into(),
                status: "skipped".into(),
                bytes: prev.bytes,
                source: p.clone(),
                dest: prev.dest.clone(),
                note: "resumed".into(),
            });
        } else if let Some(prev) = resumed_item(&interrupted, spec, p).filter(|_| !p.exists()) {
            // 끊긴 항목인데 원본이 없다(GAP-030 · 협업 지적): 같은 볼륨 이동은 rename 직후 죽으면 대상이 유일한 사본 → **건드리지 않고** 알린다.
            rep.skipped += 1;
            rep.items.push(ItemReport {
                action: verb(spec.op).into(),
                status: "skipped".into(),
                bytes: prev.bytes,
                source: p.clone(),
                dest: prev.dest.clone(),
                note: "interrupted run: source is gone, destination kept — check it".into(),
            });
        } else if spec.filter.allows(&ndir_ops::leaf_name(p), p.is_dir()) {
            sources.push(p.clone());
        }
    }
    // 끊긴 항목(GAP-030 · 저널 `started`만 · 사용자 결정 11-01 "이전 전송이 만든 대상만 덮어쓰기"):
    // - note `new`(시작 전엔 대상이 없었음 · 같은 폴더 복제 " (2)" 포함) = 부분 대상은 전부 우리 것 → **미리 지운다**(자리 판정이 같은 이름을
    //   다시 내고 Duplicate도 " (3)"으로 밀리지 않는다 · 협업 지적 ①).
    // - note `overwrite`(원래 있던 대상 · 스테이징 교체 중 죽음) = 대상은 옛 내용 그대로 → 충돌 정책과 무관하게 덮어쓰기 강제.
    // - 둘 다 스테이징 잔여(`.<이름>.nexa-tmp-*` · 살아 있는 pid 것은 제외)를 치우고, 끝난 뒤 `old` 잔여도.
    // 선삭제·강제 덮어쓰기는 **복사 + 원본이 지금 있을 때만**(협업 지적 11-01): 이동은 교차 볼륨 원본 정리 중 죽으면 대상이 전체이고 원본은
    // 일부라, 대상을 지우거나 교체하면 이미 지워진 원본이 영구 손실된다 → 이동의 끊긴 항목은 평소 정책대로 두고 note로 확인을 요청한다.
    let interrupted: Vec<ItemReport> = interrupted
        .into_iter()
        .filter(|i| sources.contains(&i.source))
        .collect();
    let resumable = |i: &ItemReport| spec.op == Op::Copy && i.source.exists();
    let force_overwrite: Vec<PathBuf> = interrupted
        .iter()
        .filter(|i| i.note == "overwrite" && resumable(i))
        .map(|i| i.dest.clone())
        .collect();
    // 끊긴 **이동**은 대상 불가침을 실제로 보증한다(협업 재현 11-01 d3: `conflict: overwrite`로 재개하면 스테이징 교체가 전체 대상을 남은 원본
    // 일부로 바꿔 5,439개 소실) → 충돌 정책과 무관하게 **Skip** · 원본이 온전한지 가릴 방법이 없다 · 대상이 없으면 평소대로 이동한다.
    let force_skip: Vec<PathBuf> = if spec.op == Op::Move {
        interrupted.iter().map(|i| i.dest.clone()).collect()
    } else {
        Vec::new()
    };
    if !spec.dry_run {
        for i in interrupted.iter().filter(|i| resumable(i)) {
            ndir_ops::remove_staging_leftovers(&i.dest, &["tmp"]);
            if i.note == "new" {
                let _ = ndir_ops::delete_permanent(&i.dest);
            }
        }
    }
    let mut resolve = |dest: &std::path::Path| {
        if force_skip.iter().any(|d| d == dest) {
            ndir_ops::Conflict::Skip
        } else if force_overwrite.iter().any(|d| d == dest) {
            ndir_ops::Conflict::Overwrite
        } else {
            spec.conflict.resolve()
        }
    };
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
    // 저널(T-202 ②): dry-run은 여기 오지 않는다(파일 시스템 불변) · 못 열면 작업을 시작하지 않는다(재개를 믿을 수 없다).
    let mut journal = match &spec.journal {
        Some(path) => match resume::Journal::open(path) {
            Ok(j) => Some(j),
            Err(e) => {
                rep.ok = false;
                rep.errors.push(format!("journal: {}: {e}", path.display()));
                return finish(rep, started);
            }
        },
        None => None,
    };
    // verify가 켜지면 복사 때가 아니라 검증을 통과한 뒤에 done을 적는다(불일치 파일을 재개 때 건너뛰지 않게 · 협업 지적 ③).
    let journal_at_verify = spec.verify && spec.op == Op::Copy;
    let mut journal_errors: Vec<String> = Vec::new();
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
        Event::ItemStart {
            index,
            dest,
            overwrite,
        } => {
            if let Some(d) = dests.get_mut(index) {
                *d = dest.to_path_buf();
            }
            // 저널 시작 줄(GAP-030): 끝나지 않은 채 죽으면 재개가 "이전 전송이 만든 부분 대상"으로 알고 덮어쓴다 · note = 당시 결정.
            if let (Some(j), Some(src)) = (journal.as_mut(), sources.get(index)) {
                let item = ItemReport {
                    action: verb(spec.op).into(),
                    status: "started".into(),
                    bytes: sizes.get(index).copied().unwrap_or(0),
                    source: src.clone(),
                    dest: dest.to_path_buf(),
                    note: if overwrite { "overwrite" } else { "new" }.into(),
                };
                if let Err(e) = j.record(&item) {
                    journal_errors.push(e.to_string());
                }
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
            if !journal_at_verify && matches!(status, ItemStatus::Done) {
                if let (Some(j), Some(src)) = (journal.as_mut(), sources.get(index)) {
                    let item = ItemReport {
                        action: verb(spec.op).into(),
                        status: "done".into(),
                        bytes: sizes.get(index).copied().unwrap_or(0),
                        source: src.clone(),
                        dest: dests.get(index).cloned().unwrap_or_default(),
                        note: String::new(),
                    };
                    if let Err(e) = j.record(&item) {
                        journal_errors.push(e.to_string());
                    }
                }
            }
        }
    };
    // 속도 제한(T-202 ③): 전송 단계만(검증 읽기는 제한하지 않음) · 복제 경로는 세지 않음 · 병렬 작업 스레드 합계(`fastcopy::Pacer`).
    // 이동 검증(T-202 ⑤): `verify` + move = 교차 볼륨 이동이 원본을 지우기 전에 사본 대조(불일치 = 그 항목 failed · 원본·사본 유지).
    // 복사 검증(GAP-027 · 11-01): 전송이 실제로 옮긴 경로 목록을 모아(`with_verify_collect`) 그 목록만 대조 — 원본 트리를 다시 훑지 않는다.
    let out = fastcopy::with_verify_collect(spec.verify && spec.op == Op::Copy, || {
        fastcopy::with_move_verify(spec.verify && spec.op == Op::Move, || {
            fastcopy::with_pacer(spec.max_bps, || {
                fastcopy::with_filter(spec.filter.clone(), || {
                    transfer_with(
                        &sources,
                        &spec.dest,
                        spec.op,
                        &mut resolve,
                        &mut on_event,
                        cancel,
                        &tuning,
                    )
                })
            })
        })
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
            // 끊긴 이동이 충돌(대상 있음)로 건너뜀(GAP-030): 대상이 전체 · 원본이 일부일 수 있다 — 사용자가 판단하도록 알린다.
            if interrupted.iter().any(|i| i.source == *src) {
                (
                    "skipped",
                    "exists · interrupted run: destination kept — check source and destination"
                        .to_string(),
                )
            } else {
                ("skipped", "exists".to_string())
            }
        } else if out.transferred.iter().any(|(s, _)| s == src) {
            rep.transferred_bytes += bytes;
            // 이동 + verify(T-202 ⑤): 같은 볼륨은 rename이라 검증하지 않았음을 적는다(엔진과 같은 `same_volume` 판정 · 협업 지적).
            let mut note = if spec.verify && spec.op == Op::Move {
                if ndir_ops::same_volume(src, &spec.dest) {
                    "renamed on the same volume (not verified)"
                } else {
                    "verified"
                }
            } else {
                ""
            }
            .to_string();
            // 끊긴 항목을 다시 돌렸다(GAP-030): 복사 = 부분 대상을 덮어썼고 옛 대상 잔여(`old`)는 **done일 때만** 치운다 · 이동 = 평소 경로로 끝남.
            if let Some(prev) = interrupted.iter().find(|i| i.source == *src) {
                let tail = if spec.op == Op::Copy {
                    ndir_ops::remove_staging_leftovers(&prev.dest, &["old"]);
                    "resumed: overwrote the partial copy left by the interrupted run"
                } else {
                    "resumed: interrupted move completed"
                };
                note = if note.is_empty() {
                    tail.to_string()
                } else {
                    format!("{note} · {tail}")
                };
            }
            ("done", note)
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
        // 목록 검증(GAP-027): 복사가 옮긴 경로(필터 반영 · 신생 파일 제외 · 빈 폴더 · 링크 안쪽 포함)만 대조.
        match verify::verify_listed(&verify::pairs_for(&out), &mut on_bytes, cancel) {
            Ok(v) => {
                if !v.ok() {
                    rep.ok = false;
                }
                // 검증을 통과한 항목만 저널에(그 항목 아래 불일치가 없을 때).
                if let Some(j) = journal.as_mut() {
                    for it in rep.items.iter().filter(|i| {
                        i.status == "done"
                            && !v
                                .failures
                                .iter()
                                .any(|(p, _)| ndir_ops::is_same_or_sub(&i.dest, p))
                    }) {
                        if let Err(e) = j.record(it) {
                            journal_errors.push(e.to_string());
                        }
                    }
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
    if !journal_errors.is_empty() {
        rep.ok = false;
        rep.errors.push(format!(
            "journal: {} write(s) failed: {}",
            journal_errors.len(),
            journal_errors[0]
        ));
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
    delete: Option<DeleteFn<'_>>,
) {
    let src = &spec.sources[0];
    // 내용 비교의 해시 읽기 = Progress `plan`(done = 읽은 바이트 · total 0 = 미정 · T-202 ⑦).
    let mut hashed = 0u64;
    let mut on_hash = |n: u64| {
        hashed += n;
        on_progress(&Progress {
            phase: Phase::Plan,
            done_bytes: hashed,
            total_bytes: 0,
            item_index: 0,
            item_count: 0,
            current: None,
        });
    };
    let plan = match mirror::mirror_plan_with(
        src,
        &spec.dest,
        &spec.filter,
        spec.mirror_by_content,
        &mut on_hash,
        cancel,
    ) {
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
            "mirror would delete {} item(s) in the destination (permanently unless the caller supplies a trash delete); set yes to confirm (or dry_run to review)",
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
    // 삭제 = 주입된 콜백(앱 휴지통 포트) 또는 영구 삭제(CLI · cdylib) — T-202 ⑦.
    let mut permanent = |p: &Path| ndir_ops::delete_permanent(p);
    let delete: DeleteFn<'_> = match delete {
        Some(d) => d,
        None => &mut permanent,
    };
    let out = match fastcopy::with_pacer(spec.max_bps, || {
        mirror::apply_mirror_with(&plan, &mut on_bytes, cancel, delete)
    }) {
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
        assert!(!r.ok && r.errors[0].contains("would delete"), "{r:?}");
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

    /// 저널 + 재개(T-202 ②): 1차 복사 = 저널 done 2줄 → 대상 비움 → resume = 둘 다 skipped/resumed(복사 안 함) · 새 원본만 복사 · 저널 3줄 ·
    /// dest가 다르면 쌍이 달라 다시 복사 · 이동 재개 = 원본이 없어도 validate 통과 · 미러 + resume = 오류 · resume 파일 없음 = 오류(항목 0).
    #[test]
    fn journal_then_resume_skips_done_items() {
        let d = sandbox("resume");
        let journal = d.join("job.log");
        let cancel = AtomicBool::new(false);
        let mut spec = JobSpec {
            sources: vec![d.join("src/a.txt"), d.join("src/dir")],
            dest: d.join("dst"),
            journal: Some(journal.clone()),
            ..JobSpec::default()
        };
        let r = run(&spec, &mut |_| {}, &cancel);
        assert!(r.ok, "{r:?}");
        let text = fs::read_to_string(&journal).unwrap();
        assert_eq!(text.lines().count(), 4, "항목마다 started + done: {text}");
        assert_eq!(
            text.lines()
                .filter(|l| l.contains("\"status\":\"done\""))
                .count(),
            2,
            "{text}"
        );
        fs::remove_dir_all(d.join("dst")).unwrap();
        fs::create_dir_all(d.join("dst")).unwrap();
        fs::write(d.join("src/c.txt"), b"new").unwrap();
        spec.sources.push(d.join("src/c.txt"));
        spec.resume = Some(journal.clone());
        let r = run(&spec, &mut |_| {}, &cancel);
        assert!(r.ok && r.skipped == 2 && r.items.len() == 3, "{r:?}");
        assert_eq!(
            r.items
                .iter()
                .filter(|i| i.note == "resumed" && i.status == "skipped")
                .count(),
            2
        );
        assert!(!d.join("dst/a.txt").exists(), "재개 = 복사 안 함");
        assert!(d.join("dst/c.txt").is_file(), "새 항목만 복사");
        assert_eq!(fs::read_to_string(&journal).unwrap().lines().count(), 6);
        // 다른 dest = 다른 쌍 → 다시 복사.
        fs::create_dir_all(d.join("dst2")).unwrap();
        let other = JobSpec {
            dest: d.join("dst2"),
            journal: None,
            ..spec.clone()
        };
        let r = run(&other, &mut |_| {}, &cancel);
        assert!(
            r.ok && r.skipped == 0 && d.join("dst2/a.txt").is_file(),
            "{r:?}"
        );
        // 이동 재개: 원본이 이미 없어도 저널의 done 쌍이면 통과.
        let mvlog = d.join("mv.log");
        fs::create_dir_all(d.join("mv")).unwrap();
        let mv = JobSpec {
            op: Op::Move,
            sources: vec![d.join("src/c.txt")],
            dest: d.join("mv"),
            journal: Some(mvlog.clone()),
            ..JobSpec::default()
        };
        assert!(run(&mv, &mut |_| {}, &cancel).ok);
        assert!(!d.join("src/c.txt").exists() && d.join("mv/c.txt").is_file());
        let again = JobSpec {
            resume: Some(mvlog),
            ..mv
        };
        let r = run(&again, &mut |_| {}, &cancel);
        assert!(
            r.ok && r.skipped == 1 && r.items[0].note == "resumed",
            "{r:?}"
        );
        // 미러 + resume = 오류 · resume 파일 없음 = 오류(항목 0 → CLI 종료 2).
        let bad = JobSpec {
            sources: vec![d.join("src")],
            dest: d.join("dst"),
            mirror: true,
            resume: Some(journal),
            ..JobSpec::default()
        };
        assert!(run(&bad, &mut |_| {}, &cancel).errors[0].contains("mirror"));
        let bad = JobSpec {
            resume: Some(d.join("nope.log")),
            ..spec.clone()
        };
        let r = run(&bad, &mut |_| {}, &cancel);
        assert!(
            r.errors[0].starts_with("resume:") && r.items.is_empty(),
            "{r:?}"
        );
        let _ = fs::remove_dir_all(&d);
    }

    /// verify가 켜지면 저널은 검증 통과 뒤에 적힌다(협업 지적 ③) — 통과한 항목 수만큼.
    #[test]
    fn journal_waits_for_verify() {
        let d = sandbox("journal-verify");
        let journal = d.join("v.log");
        let spec = JobSpec {
            sources: vec![d.join("src/a.txt"), d.join("src/dir")],
            dest: d.join("dst"),
            verify: true,
            journal: Some(journal.clone()),
            ..JobSpec::default()
        };
        let r = run(&spec, &mut |_| {}, &AtomicBool::new(false));
        assert!(r.ok && r.verify.is_some(), "{r:?}");
        let text = fs::read_to_string(&journal).unwrap();
        assert_eq!(
            text.lines().count(),
            4,
            "started 2 + (검증 뒤) done 2: {text}"
        );
        let _ = fs::remove_dir_all(&d);
    }

    /// GAP-030(11-01): 저널에 `started`만 남은 항목(프로세스 사망) = 이전 전송이 만든 부분 대상 → 재개가 충돌 정책(skip)과 무관하게
    /// 덮어쓰고 스테이징 잔여(`.<이름>.nexa-tmp-*` · 끝난 뒤 `old`)를 치운다 · 그 항목 note에 표시 · 시작 전 항목은 평소대로.
    #[test]
    fn resume_overwrites_partial_copy_and_cleans_staging() {
        let d = sandbox("gap030");
        let journal = d.join("job.log");
        let started = ItemReport {
            action: "copy".into(),
            status: "started".into(),
            bytes: 5,
            source: d.join("src/a.txt"),
            dest: d.join("dst/a.txt"),
            note: "new".into(),
        };
        fs::write(
            &journal,
            format!("{}\n", json::dump(&started.to_json_value())),
        )
        .unwrap();
        fs::write(d.join("dst/a.txt"), b"he").unwrap(); // 부분 파일(대상 이름 그대로)
        fs::write(d.join("dst/.a.txt.nexa-tmp-4000000000-0"), b"stale").unwrap(); // 스테이징 잔여
        fs::write(d.join("dst/.a.txt.nexa-old-4000000001-1"), b"old").unwrap();
        fs::write(d.join("dst/b.bin"), b"keep").unwrap(); // 시작 전 항목의 기존 대상 = 평소대로 건너뜀
        fs::write(d.join("src/b.bin"), b"new b").unwrap();
        // 덮어쓰기 중 죽은 폴더 항목: 대상에 옛 내용이 그대로(스테이징 미커밋) → 재개 = 강제 덮어쓰기(스테이징 교체 · 옛 파일 사라짐).
        fs::create_dir_all(d.join("dst/dir")).unwrap();
        fs::write(d.join("dst/dir/stale.txt"), b"old content").unwrap();
        let started_dir = ItemReport {
            action: "copy".into(),
            status: "started".into(),
            bytes: 3000,
            source: d.join("src/dir"),
            dest: d.join("dst/dir"),
            note: "overwrite".into(),
        };
        {
            use std::io::Write as _;
            fs::OpenOptions::new()
                .append(true)
                .open(&journal)
                .unwrap()
                .write_all(format!("{}\n", json::dump(&started_dir.to_json_value())).as_bytes())
                .unwrap();
        }
        let spec = JobSpec {
            sources: vec![d.join("src/a.txt"), d.join("src/b.bin"), d.join("src/dir")],
            dest: d.join("dst"),
            resume: Some(journal),
            ..JobSpec::default()
        };
        let r = run(&spec, &mut |_| {}, &AtomicBool::new(false));
        assert!(r.ok, "{r:?}");
        assert_eq!(
            fs::read(d.join("dst/a.txt")).unwrap(),
            b"hello",
            "부분 파일을 덮어씀"
        );
        assert!(
            !d.join("dst/.a.txt.nexa-tmp-4000000000-0").exists(),
            "스테이징 사본 잔여 정리"
        );
        assert!(
            !d.join("dst/.a.txt.nexa-old-4000000001-1").exists(),
            "옛 대상 잔여 정리(끝난 뒤)"
        );
        assert_eq!(
            fs::read(d.join("dst/b.bin")).unwrap(),
            b"keep",
            "시작 전 항목 = skip 정책 그대로"
        );
        let a = r
            .items
            .iter()
            .find(|i| i.source.ends_with("a.txt"))
            .unwrap();
        assert!(a.status == "done" && a.note.contains("partial"), "{a:?}");
        assert_eq!(r.skipped, 1);
        assert!(
            d.join("dst/dir/b.bin").is_file() && !d.join("dst/dir/stale.txt").exists(),
            "overwrite 항목 = 스테이징 교체로 통째 다시"
        );
        // 같은 폴더 복제(Duplicate " (2)") 중 죽음: started(new · dest = " (2)") → 재개가 부분 " (2)"를 지우고 같은 이름으로 다시(" (3)" 없음).
        let dup_journal = d.join("dup.log");
        let dup_dest = d.join("src/a (2).txt");
        let started_dup = ItemReport {
            action: "duplicate".into(),
            status: "started".into(),
            bytes: 5,
            source: d.join("src/a.txt"),
            dest: dup_dest.clone(),
            note: "new".into(),
        };
        fs::write(
            &dup_journal,
            format!("{}\n", json::dump(&started_dup.to_json_value())),
        )
        .unwrap();
        fs::write(&dup_dest, b"he").unwrap();
        let spec = JobSpec {
            sources: vec![d.join("src/a.txt")],
            dest: d.join("src"),
            resume: Some(dup_journal),
            ..JobSpec::default()
        };
        let r = run(&spec, &mut |_| {}, &AtomicBool::new(false));
        assert!(r.ok, "{r:?}");
        assert_eq!(fs::read(&dup_dest).unwrap(), b"hello", "같은 이름으로 다시");
        assert!(!d.join("src/a (3).txt").exists(), "\" (3)\" 없음");
        let _ = fs::remove_dir_all(&d);
    }

    /// GAP-030 안전 규칙(협업 지적 11-01): 끊긴 항목이라도 **대상이 유일한 사본일 수 있으면 건드리지 않는다** —
    /// ① 이동 · 원본 없음 · 대상 있음(같은 볼륨 rename 직후 사망) = 대상 유지 · skipped + note ② 이동 · 원본 일부 · 대상 전체(교차 볼륨
    /// 원본 정리 중 사망) = 대상 유지(평소 skip 정책) + note ③ 복사 · 원본 없음 · 대상 있음 = 대상 유지(선삭제 안 함) · validate 통과.
    #[test]
    fn resume_never_touches_destination_when_source_may_be_gone() {
        let d = sandbox("gap030-safe");
        let line = |src: &Path, dest: &Path| {
            let it = ItemReport {
                action: "move".into(),
                status: "started".into(),
                bytes: 0,
                source: src.to_path_buf(),
                dest: dest.to_path_buf(),
                note: "new".into(),
            };
            format!("{}\n", json::dump(&it.to_json_value()))
        };
        let cancel = AtomicBool::new(false);
        // ① 이동 · 원본 없음 · 대상 있음.
        fs::write(d.join("dst/gone.txt"), b"only copy").unwrap();
        let j1 = d.join("j1.log");
        fs::write(&j1, line(&d.join("src/gone.txt"), &d.join("dst/gone.txt"))).unwrap();
        let r = run(
            &JobSpec {
                op: Op::Move,
                sources: vec![d.join("src/gone.txt")],
                dest: d.join("dst"),
                resume: Some(j1),
                ..JobSpec::default()
            },
            &mut |_| {},
            &cancel,
        );
        assert!(r.ok && r.skipped == 1, "{r:?}");
        assert_eq!(
            fs::read(d.join("dst/gone.txt")).unwrap(),
            b"only copy",
            "대상 유지"
        );
        assert!(r.items[0].note.contains("source is gone"), "{:?}", r.items);
        // ② 이동 · 원본 일부(폴더에 파일 1개 남음) · 대상 전체.
        fs::create_dir_all(d.join("src/mv")).unwrap();
        fs::write(d.join("src/mv/left.txt"), b"left").unwrap();
        fs::create_dir_all(d.join("dst/mv")).unwrap();
        fs::write(d.join("dst/mv/left.txt"), b"left").unwrap();
        fs::write(d.join("dst/mv/moved.txt"), b"already moved").unwrap();
        let j2 = d.join("j2.log");
        fs::write(&j2, line(&d.join("src/mv"), &d.join("dst/mv"))).unwrap();
        let r = run(
            &JobSpec {
                op: Op::Move,
                sources: vec![d.join("src/mv")],
                dest: d.join("dst"),
                resume: Some(j2),
                ..JobSpec::default()
            },
            &mut |_| {},
            &cancel,
        );
        assert!(r.ok && r.skipped == 1, "{r:?}");
        assert!(
            d.join("dst/mv/moved.txt").is_file() && d.join("src/mv/left.txt").is_file(),
            "대상 전체 · 원본 일부 모두 유지"
        );
        assert!(
            r.items[0].note.contains("destination kept"),
            "{:?}",
            r.items
        );
        // ②' 같은 상황인데 정책이 overwrite(협업 재현 d3 · 5,439개 소실) = 그래도 대상 유지 · skipped.
        let j2b = d.join("j2b.log");
        fs::write(&j2b, line(&d.join("src/mv"), &d.join("dst/mv"))).unwrap();
        let r = run(
            &JobSpec {
                op: Op::Move,
                sources: vec![d.join("src/mv")],
                dest: d.join("dst"),
                conflict: ConflictPolicy::Overwrite,
                resume: Some(j2b),
                ..JobSpec::default()
            },
            &mut |_| {},
            &cancel,
        );
        assert!(r.ok && r.skipped == 1, "{r:?}");
        assert!(
            d.join("dst/mv/moved.txt").is_file() && d.join("src/mv/left.txt").is_file(),
            "overwrite 정책이어도 끊긴 이동의 대상은 불가침"
        );
        assert!(
            r.items[0].note.contains("destination kept"),
            "{:?}",
            r.items
        );
        // ③ 복사 · 원본 없음 · 대상 있음 = 선삭제 없음 · validate 통과.
        fs::write(d.join("dst/c.txt"), b"keep me").unwrap();
        let j3 = d.join("j3.log");
        fs::write(&j3, line(&d.join("src/c.txt"), &d.join("dst/c.txt"))).unwrap();
        let r = run(
            &JobSpec {
                sources: vec![d.join("src/c.txt")],
                dest: d.join("dst"),
                resume: Some(j3),
                ..JobSpec::default()
            },
            &mut |_| {},
            &cancel,
        );
        assert!(r.ok && r.skipped == 1, "{r:?}");
        assert_eq!(fs::read(d.join("dst/c.txt")).unwrap(), b"keep me");
        let _ = fs::remove_dir_all(&d);
    }

    /// 속도 제한(T-202 ③): `max_bps`가 전송 단계에 걸린다 — 512 KiB를 2 MiB/s로 = ≥ 200 ms(하한만) · native를 꺼 복제 경로를 배제.
    #[test]
    fn max_bps_slows_transfer() {
        let d = sandbox("bps");
        fs::write(d.join("src/big.bin"), vec![3u8; 512 * 1024]).unwrap();
        let spec = JobSpec {
            sources: vec![d.join("src/big.bin")],
            dest: d.join("dst"),
            native: false,
            max_bps: 2 * 1024 * 1024,
            ..JobSpec::default()
        };
        let t0 = std::time::Instant::now();
        let r = run(&spec, &mut |_| {}, &AtomicBool::new(false));
        assert!(r.ok && r.transferred_bytes == 512 * 1024, "{r:?}");
        assert!(
            t0.elapsed() >= std::time::Duration::from_millis(200),
            "{:?}",
            t0.elapsed()
        );
        let _ = fs::remove_dir_all(&d);
    }

    /// 미러 내용 비교 + 삭제 콜백(T-202 ⑦): 크기·시각이 같은데 내용이 다른 파일 = 기본 미러는 그대로 · `mirror_by_content`면 덮어씀 ·
    /// `run_with(delete)` = 대상에만 있는 항목이 콜백으로(휴지통 흉내 = 옮김) · 콜백 Err = 그 항목 failed.
    #[test]
    fn mirror_by_content_and_delete_callback() {
        let d = sandbox("by-content");
        let t = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        for (side, body) in [("src", b"AAAA"), ("dst", b"BBBB")] {
            let p = d.join(side).join("same.txt");
            fs::write(&p, body).unwrap();
            fs::File::options()
                .write(true)
                .open(&p)
                .unwrap()
                .set_modified(t)
                .unwrap();
        }
        fs::write(d.join("dst/extra.txt"), b"x").unwrap();
        let cancel = AtomicBool::new(false);
        let base = JobSpec {
            sources: vec![d.join("src")],
            dest: d.join("dst"),
            mirror: true,
            yes: true,
            ..JobSpec::default()
        };
        // 기본: 크기·시각 같음 = 같다고 봄(덮어쓰지 않음).
        let r = run(&base, &mut |_| {}, &cancel);
        assert!(r.ok, "{r:?}");
        assert_eq!(fs::read(d.join("dst/same.txt")).unwrap(), b"BBBB");
        assert!(
            !r.items.iter().any(|i| i.dest.ends_with("same.txt")),
            "{:?}",
            r.items
        );
        // 내용 비교: 해시가 달라 덮어씀 · 삭제는 콜백으로(옮김) · Report delete = done · deleted 1.
        fs::write(d.join("dst/extra.txt"), b"x").unwrap();
        let trash = d.join("trash");
        fs::create_dir_all(&trash).unwrap();
        let spec = JobSpec {
            mirror_by_content: true,
            ..base.clone()
        };
        let mut moved = 0;
        let mut to_trash = |p: &Path| {
            moved += 1;
            fs::rename(p, trash.join(p.file_name().unwrap()))
        };
        let mut hashed = 0u64;
        let r = run_with(
            &spec,
            &mut |p| {
                if p.phase == Phase::Plan {
                    hashed = hashed.max(p.done_bytes);
                }
            },
            &cancel,
            Some(&mut to_trash),
        );
        assert!(r.ok && r.deleted == 1, "{r:?}");
        assert_eq!(moved, 1);
        assert_eq!(
            hashed, 6018,
            "같다고 본 쌍 전부 양쪽 해시: (5 + 3000 + 4) × 2 · 진행 보고"
        );
        assert_eq!(
            fs::read(d.join("dst/same.txt")).unwrap(),
            b"AAAA",
            "내용 비교로 덮어씀"
        );
        assert!(trash.join("extra.txt").is_file() && !d.join("dst/extra.txt").exists());
        assert!(r
            .items
            .iter()
            .any(|i| i.action == "delete" && i.status == "done"));
        // 콜백 Err = 그 항목 failed(ok false · 나머지는 계속).
        fs::write(d.join("dst/extra2.txt"), b"y").unwrap();
        let mut refuse = |_: &Path| Err(std::io::Error::other("trash unavailable"));
        let r = run_with(&base, &mut |_| {}, &cancel, Some(&mut refuse));
        assert!(!r.ok && r.failed == 1 && r.deleted == 0, "{r:?}");
        assert!(r
            .items
            .iter()
            .any(|i| i.action == "delete" && i.status == "failed" && i.note.contains("trash")));
        assert!(d.join("dst/extra2.txt").is_file(), "실패 = 그대로");
        // 제외된 파일은 해시하지 않는다(협업 지적 ①): same.txt를 exclude → plan 진행 0.
        fs::write(d.join("dst/same.txt"), b"BBBB").unwrap();
        fs::File::options()
            .write(true)
            .open(d.join("dst/same.txt"))
            .unwrap()
            .set_modified(t)
            .unwrap();
        let excl = JobSpec {
            mirror_by_content: true,
            filter: ndir_ops::fastcopy::NameFilter {
                include: vec![],
                exclude: vec!["same.txt".into()],
            },
            ..base
        };
        let mut hashed = 0u64;
        let r = run(
            &excl,
            &mut |p| {
                if p.phase == Phase::Plan {
                    hashed = hashed.max(p.done_bytes);
                }
            },
            &cancel,
        );
        assert!(r.ok, "{r:?}");
        assert_eq!(hashed, 6010, "제외한 same.txt(4 × 2)만 빠짐 = 읽지 않음");
        assert_eq!(fs::read(d.join("dst/same.txt")).unwrap(), b"BBBB");
        let _ = fs::remove_dir_all(&d);
    }

    /// 이동 + verify(T-202 ⑤): 같은 볼륨 이동은 rename이라 검증 대상이 아니고 그대로 성공(깃발이 경로를 깨지 않음) · Report verify 요약은 없음(move).
    #[test]
    fn move_with_verify_still_moves() {
        let d = sandbox("move-verify");
        let spec = JobSpec {
            op: Op::Move,
            sources: vec![d.join("src/a.txt")],
            dest: d.join("dst"),
            verify: true,
            ..JobSpec::default()
        };
        let r = run(&spec, &mut |_| {}, &AtomicBool::new(false));
        assert!(r.ok && r.verify.is_none(), "{r:?}");
        assert!(!d.join("src/a.txt").exists() && d.join("dst/a.txt").is_file());
        assert!(r.items[0].note.contains("not verified"), "{:?}", r.items);
        assert!(!fastcopy::move_verify(), "끝나면 복원");
        // GAP-027: 복사 + verify + exclude — 제외한 파일은 검증에서도 빠진다(ok · files 1).
        let spec = JobSpec {
            sources: vec![d.join("src/dir")],
            dest: d.join("dst"),
            verify: true,
            filter: ndir_ops::fastcopy::NameFilter {
                include: vec![],
                exclude: vec!["*.bin".into()],
            },
            ..JobSpec::default()
        };
        fs::write(d.join("src/dir/c.txt"), b"c").unwrap();
        let r = run(&spec, &mut |_| {}, &AtomicBool::new(false));
        assert!(r.ok, "{r:?}");
        let v = r.verify.as_ref().unwrap();
        assert_eq!((v.files, v.mismatches.len()), (1, 0), "{v:?}");
        assert!(!d.join("dst/dir/b.bin").exists());
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
