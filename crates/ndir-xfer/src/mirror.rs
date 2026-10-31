//! 미러(T-97 "미러/차등" · nexa-copy 10-25에서 엔진 부분을 옮김 10-31): 대상 폴더를 원본과 같게 — `compare::compare`(크기 + 수정 시각 ·
//! 허용 2초 · 재귀) → `plan(LeftToRight · mirror)` → 복사 + 대상에만 있는 것 **영구 삭제**(복사가 끝난 뒤 · 휴지통 없음).
//! 출력·종료 코드는 호출자(CLI · 앱) 몫 — 여기는 계획 · 안전 판정 · 적용만.
//! T-202 ⑦: `mirror_plan_with(by_content)` = 크기·시각이 같은 파일도 SHA-256 대조(같을 때만 해시 · 2단계) · `apply_mirror_with(delete)` =
//! 삭제 콜백 주입(앱이 휴지통 포트를 넣는다 · 기본 = 영구 삭제 · ndir-xfer는 `platform/`을 모른다 · DR-5).

use ndir_ops::compare::{self, CmpOpts, CmpProgress, Direction, SyncPlan, Verdict};
use ndir_ops::fastcopy::NameFilter;
use ndir_ops::hash::{hash_file, Algo};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

/// 계획 만들기(파일 시스템은 읽기만): 비교(크기 + 수정 시각) → 필터 → 계획 · 경로 구분자는 이 OS 것으로.
pub fn mirror_plan(
    src: &Path,
    dst: &Path,
    filter: &NameFilter,
    cancel: &AtomicBool,
) -> std::io::Result<SyncPlan> {
    mirror_plan_with(src, dst, filter, false, &mut |_| {}, cancel)
}

/// [`mirror_plan`] + `by_content`(T-202 ⑦): 크기·시각이 같은 파일도 SHA-256으로 대조해 다르면 덮어쓴다 — **필터 뒤**에 같다고 판정된
/// 파일 쌍만 여기서 해시한다(엔진 `CmpOpts::by_content`는 제외 파일까지 양쪽 읽고 읽기 실패를 "같음"으로 봐 쓰지 않는다 · 협업 지적 10-31).
/// 비용 = 그 쌍들의 양쪽 전량 읽기(`max_bps`는 걸리지 않음) · `on_hash` = 읽은 바이트(진행) · 읽기 실패 = 다름(복사를 시도해 오류가 Report에 남게).
pub fn mirror_plan_with(
    src: &Path,
    dst: &Path,
    filter: &NameFilter,
    by_content: bool,
    on_hash: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> std::io::Result<SyncPlan> {
    let progress = CmpProgress::default();
    let mut entries = compare::compare(src, dst, CmpOpts::default(), &progress, cancel)?;
    if !filter.is_empty() {
        entries.retain(|e| {
            let name = e.rel.rsplit('/').next().unwrap_or(&e.rel);
            let is_dir = e.left.or(e.right).is_some_and(|m| m.is_dir);
            filter.allows(name, is_dir)
        });
    }
    if by_content {
        for e in &mut entries {
            let (Some(a), Some(_)) = (e.left, e.right) else {
                continue;
            };
            if e.verdict != Verdict::Same || a.is_dir || a.size == 0 {
                continue;
            }
            if content_differs(&src.join(&e.rel), &dst.join(&e.rel), on_hash, cancel)? {
                e.verdict = Verdict::Differ;
            }
        }
    }
    let mut plan = compare::plan(&entries, src, dst, Direction::LeftToRight, true, None);
    for (a, b, _) in &mut plan.copies {
        *a = normalize_sep(a);
        *b = normalize_sep(b);
    }
    for d in &mut plan.deletes {
        *d = normalize_sep(d);
    }
    Ok(plan)
}

/// 두 파일의 SHA-256이 다른가 — 읽기 실패는 **다름**(복사를 시도해 그 오류가 Report에 남는다 · 조용한 "같음" 금지) · 취소 = `Err(Interrupted)`.
fn content_differs(
    a: &Path,
    b: &Path,
    on_hash: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> std::io::Result<bool> {
    let mut digest = |p: &Path| -> std::io::Result<Option<String>> {
        match hash_file(p, &[Algo::Sha256], on_hash, cancel) {
            Ok(v) => Ok(v.into_iter().next().map(|(_, h)| h)),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Err(e),
            Err(_) => Ok(None),
        }
    };
    let (ha, hb) = (digest(a)?, digest(b)?);
    Ok(match (ha, hb) {
        (Some(x), Some(y)) => x != y,
        _ => true,
    })
}

/// 경로의 구분자를 이 OS 것으로(`components()` 재조립 · 순수).
#[must_use]
pub fn normalize_sep(p: &Path) -> PathBuf {
    p.components().collect()
}

/// 사용자 홈(환경 변수 · 없으면 None).
#[must_use]
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

/// 미러 대상으로 위험한 자리(순수): 드라이브/파일 시스템 루트(부모 없음) · 사용자 홈 그 자체 — 사유 글(없으면 None).
#[must_use]
pub fn dangerous_dest(dest: &Path, home: Option<&Path>) -> Option<&'static str> {
    let d = dest.canonicalize().unwrap_or_else(|_| dest.to_path_buf());
    let trimmed = d
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_string();
    if d.parent().is_none() || trimmed.is_empty() || trimmed.ends_with(':') {
        return Some("a drive or file system root");
    }
    if let Some(h) = home {
        let h = h.canonicalize().unwrap_or_else(|_| h.to_path_buf());
        if ndir_ops::is_same_or_sub(&h, &d) && ndir_ops::is_same_or_sub(&d, &h) {
            return Some("the user home folder");
        }
    }
    None
}

/// 미러 적용 결과.
#[derive(Debug, Default)]
pub struct MirrorOutcome {
    /// 복사 실패(경로 · 사유).
    pub failed: Vec<(PathBuf, String)>,
    pub deleted: Vec<PathBuf>,
    pub delete_failed: Vec<(PathBuf, String)>,
    pub canceled: bool,
}

/// 계획 적용: 복사(`apply_copies`) → 대상에만 있는 것 **영구 삭제**(복사 실패가 있어도 지운다 — 미러 의미 · 취소 깃발이 서면 멈춤).
pub fn apply_mirror(
    plan: &SyncPlan,
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> std::io::Result<MirrorOutcome> {
    apply_mirror_with(plan, on_bytes, cancel, &mut |p: &Path| {
        ndir_ops::delete_permanent(p)
    })
}

/// [`apply_mirror`] + 삭제 콜백(T-202 ⑦ 휴지통 미러): 대상에만 있는 항목마다 `delete(path)` — 앱은 휴지통 포트를, CLI는 영구 삭제를 넣는다.
/// 콜백의 `Err`는 그 항목의 `delete_failed`로(나머지는 계속).
pub fn apply_mirror_with(
    plan: &SyncPlan,
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
    delete: &mut dyn FnMut(&Path) -> std::io::Result<()>,
) -> std::io::Result<MirrorOutcome> {
    let mut out = MirrorOutcome::default();
    match compare::apply_copies(plan, on_bytes, cancel) {
        Ok(f) => out.failed = f,
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
            out.canceled = true;
            return Ok(out);
        }
        Err(e) => return Err(e),
    }
    for d in &plan.deletes {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            out.canceled = true;
            return Ok(out);
        }
        match delete(d) {
            Ok(()) => out.deleted.push(d.clone()),
            Err(e) => out.delete_failed.push((d.clone(), e.to_string())),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dangerous_dest_root_and_home() {
        let root = if cfg!(windows) {
            PathBuf::from("C:\\")
        } else {
            PathBuf::from("/")
        };
        assert!(dangerous_dest(&root, None).is_some());
        let home = std::env::temp_dir().join(format!("xfer-home-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&home);
        assert_eq!(
            dangerous_dest(&home, Some(&home)),
            Some("the user home folder")
        );
        let sub = home.join("sub");
        let _ = std::fs::create_dir_all(&sub);
        assert_eq!(dangerous_dest(&sub, Some(&home)), None, "홈 아래는 허용");
        let _ = std::fs::remove_dir_all(&home);
    }
}
