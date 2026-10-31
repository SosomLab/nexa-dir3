//! 미러(T-97 "미러/차등" · nexa-copy 10-25에서 엔진 부분을 옮김 10-31): 대상 폴더를 원본과 같게 — `compare::compare`(크기 + 수정 시각 ·
//! 허용 2초 · 재귀) → `plan(LeftToRight · mirror)` → 복사 + 대상에만 있는 것 **영구 삭제**(복사가 끝난 뒤 · 휴지통 없음).
//! 출력·종료 코드는 호출자(CLI · 앱) 몫 — 여기는 계획 · 안전 판정 · 적용만.

use ndir_ops::compare::{self, CmpOpts, CmpProgress, Direction, SyncPlan};
use ndir_ops::fastcopy::NameFilter;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

/// 계획 만들기(파일 시스템은 읽기만): 비교 → 필터 → 계획 · 경로 구분자는 이 OS 것으로.
pub fn mirror_plan(
    src: &Path,
    dst: &Path,
    filter: &NameFilter,
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

/// 계획 적용: 복사(`apply_copies`) → 대상에만 있는 것 삭제(복사 실패가 있어도 지운다 — 미러 의미 · 취소 깃발이 서면 멈춤).
pub fn apply_mirror(
    plan: &SyncPlan,
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
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
        match ndir_ops::delete_permanent(d) {
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
