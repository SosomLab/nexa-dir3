//! 폴더 비교 · 동기화 계획(T-171/T-172 1차 · NEW-042/043 · dir3 신규 · 사용자 10-05 "대상 목록") — 두 폴더를 상대 경로로 맞춰
//! 항목마다 **왼쪽만 · 오른쪽만 · 같음 · 다름(왼쪽이 새로움 / 오른쪽이 새로움 / 크기만 다름) · 종류 불일치**를 판정한다.
//! 기본 기준 = 크기 + 수정 시각(허용 오차 2초 · FAT/NTFS 해상도 차 · 선택적으로 DST 1시간 어긋남 허용) · 선택적으로 내용(SHA-256 ·
//! 크기가 같을 때만 읽는다). 링크는 따라가지 않는다. 동기화는 비교 결과에서 **복사 계획**(원본 → 대상 쌍 · 덮어쓰기 여부)을 뽑아
//! 전송 엔진(`copy_onto_with_progress`)으로 실행한다 — 미러의 "대상에만 있는 것 삭제"는 계획에 `delete` 목록으로 따로 낸다(앱이 휴지통).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// 한쪽 항목의 메타데이터.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Meta {
    pub is_dir: bool,
    pub size: u64,
    /// 수정 시각(Unix 초 · 모르면 0).
    pub mtime: i64,
}

/// 판정.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    OnlyLeft,
    OnlyRight,
    Same,
    /// 왼쪽이 더 새롭다.
    LeftNewer,
    /// 오른쪽이 더 새롭다.
    RightNewer,
    /// 시각은 같은데 크기(또는 내용)가 다르다.
    Differ,
    /// 한쪽은 폴더 · 다른 쪽은 파일.
    TypeMismatch,
}

impl Verdict {
    /// 표시 기호(=, ≠, ←, →, ◁, ▷, ✕).
    #[must_use]
    pub fn glyph(self) -> &'static str {
        match self {
            Verdict::OnlyLeft => "▷",
            Verdict::OnlyRight => "◁",
            Verdict::Same => "=",
            Verdict::LeftNewer => "→",
            Verdict::RightNewer => "←",
            Verdict::Differ => "≠",
            Verdict::TypeMismatch => "✕",
        }
    }

    /// 같지 않은가.
    #[must_use]
    pub fn differs(self) -> bool {
        self != Verdict::Same
    }
}

/// 비교 결과 1건.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmpEntry {
    /// `/` 구분 상대 경로.
    pub rel: String,
    pub left: Option<Meta>,
    pub right: Option<Meta>,
    pub verdict: Verdict,
}

/// 비교 옵션.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CmpOpts {
    /// 시각 허용 오차(초).
    pub time_tol_secs: i64,
    /// 정확히 1시간(±허용 오차) 어긋난 것은 같은 시각으로 본다(DST · FAT 시간대).
    pub ignore_dst_hour: bool,
    /// 크기 · 시각이 같아도 내용(SHA-256)을 비교한다.
    pub by_content: bool,
    /// 하위 폴더 포함.
    pub recursive: bool,
}

impl Default for CmpOpts {
    fn default() -> Self {
        CmpOpts {
            time_tol_secs: 2,
            ignore_dst_hour: false,
            by_content: false,
            recursive: true,
        }
    }
}

/// 진행(훑은 항목 · 내용 비교로 읽은 바이트).
#[derive(Debug, Default)]
pub struct CmpProgress {
    pub scanned: AtomicU64,
    pub hashed: AtomicU64,
    pub errors: AtomicU64,
}

fn canceled() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Interrupted, "canceled")
}

fn mtime_secs(md: &std::fs::Metadata) -> i64 {
    md.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs() as i64)
}

/// 한쪽을 훑어 상대 경로 → 메타 표를 만든다(링크는 건너뛴다 · 오류는 세고 계속).
fn scan(
    root: &Path,
    opts: CmpOpts,
    progress: &CmpProgress,
    cancel: &AtomicBool,
) -> std::io::Result<BTreeMap<String, Meta>> {
    let mut out = BTreeMap::new();
    let mut stack: Vec<(PathBuf, String)> = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            return Err(canceled());
        }
        let rd = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => {
                progress.errors.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };
        for e in rd {
            let Ok(e) = e else {
                progress.errors.fetch_add(1, Ordering::Relaxed);
                continue;
            };
            let p = e.path();
            let Ok(md) = std::fs::symlink_metadata(&p) else {
                progress.errors.fetch_add(1, Ordering::Relaxed);
                continue;
            };
            if md.file_type().is_symlink() || crate::is_link(&p) {
                continue;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            let rel = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            progress.scanned.fetch_add(1, Ordering::Relaxed);
            let meta = Meta {
                is_dir: md.is_dir(),
                size: if md.is_dir() { 0 } else { md.len() },
                mtime: mtime_secs(&md),
            };
            if md.is_dir() && opts.recursive {
                stack.push((p, rel.clone()));
            }
            out.insert(rel, meta);
        }
    }
    Ok(out)
}

/// 두 메타의 판정(순수 · 내용 비교 제외).
#[must_use]
pub fn judge(l: Meta, r: Meta, opts: CmpOpts) -> Verdict {
    if l.is_dir != r.is_dir {
        return Verdict::TypeMismatch;
    }
    if l.is_dir {
        return Verdict::Same;
    }
    let dt = l.mtime - r.mtime;
    let within = |d: i64| d.abs() <= opts.time_tol_secs;
    let same_time =
        within(dt) || (opts.ignore_dst_hour && (within(dt - 3600) || within(dt + 3600)));
    if same_time {
        if l.size == r.size {
            Verdict::Same
        } else {
            Verdict::Differ
        }
    } else if dt > 0 {
        Verdict::LeftNewer
    } else {
        Verdict::RightNewer
    }
}

/// 두 폴더를 비교한다 — 결과는 상대 경로순 · 취소 = `Interrupted`.
pub fn compare(
    left: &Path,
    right: &Path,
    opts: CmpOpts,
    progress: &CmpProgress,
    cancel: &AtomicBool,
) -> std::io::Result<Vec<CmpEntry>> {
    let l = scan(left, opts, progress, cancel)?;
    let r = scan(right, opts, progress, cancel)?;
    let mut keys: Vec<&String> = l.keys().chain(r.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut out = Vec::with_capacity(keys.len());
    for k in keys {
        if cancel.load(Ordering::Relaxed) {
            return Err(canceled());
        }
        let (lm, rm) = (l.get(k).copied(), r.get(k).copied());
        let verdict = match (lm, rm) {
            (Some(a), Some(b)) => {
                let mut v = judge(a, b, opts);
                if v == Verdict::Same && opts.by_content && !a.is_dir && a.size > 0 {
                    let la = hash_of(&left.join(k), progress, cancel)?;
                    let rb = hash_of(&right.join(k), progress, cancel)?;
                    if la != rb {
                        v = Verdict::Differ;
                    }
                }
                v
            }
            (Some(_), None) => Verdict::OnlyLeft,
            (None, Some(_)) => Verdict::OnlyRight,
            (None, None) => continue,
        };
        out.push(CmpEntry {
            rel: k.clone(),
            left: lm,
            right: rm,
            verdict,
        });
    }
    Ok(out)
}

fn hash_of(
    p: &Path,
    progress: &CmpProgress,
    cancel: &AtomicBool,
) -> std::io::Result<Option<String>> {
    match crate::hash::hash_file(
        p,
        &[crate::hash::Algo::Sha256],
        &mut |n| {
            progress.hashed.fetch_add(n, Ordering::Relaxed);
        },
        cancel,
    ) {
        Ok(v) => Ok(v.into_iter().next().map(|(_, h)| h)),
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Err(e),
        Err(_) => {
            progress.errors.fetch_add(1, Ordering::Relaxed);
            Ok(None)
        }
    }
}

/// 동기화 방향.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// 왼쪽 → 오른쪽.
    LeftToRight,
    /// 오른쪽 → 왼쇽.
    RightToLeft,
    /// 양쪽 모두 새 것으로(새로운 쪽이 이긴다 · 한쪽에만 있는 것은 복사).
    Both,
}

/// 동기화 계획 — 복사 쌍(원본 · 대상 · 덮어쓰기)과 미러일 때 지울 것.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SyncPlan {
    pub copies: Vec<(PathBuf, PathBuf, bool)>,
    pub deletes: Vec<PathBuf>,
    pub bytes: u64,
}

/// 비교 결과 → 계획(순수). `mirror` = 대상에만 있는 항목을 지운다(미러 · `Both`에서는 무시) · `only` = 이 상대 경로들만(None = 전부).
#[must_use]
pub fn plan(
    entries: &[CmpEntry],
    left: &Path,
    right: &Path,
    dir: Direction,
    mirror: bool,
    only: Option<&[String]>,
) -> SyncPlan {
    let mut out = SyncPlan::default();
    for e in entries {
        if only.is_some_and(|o| !o.iter().any(|x| x == &e.rel)) {
            continue;
        }
        let (lp, rp) = (left.join(&e.rel), right.join(&e.rel));
        let size = |m: Option<Meta>| m.map_or(0, |m| m.size);
        match (e.verdict, dir) {
            (Verdict::Same, _) => {}
            (Verdict::TypeMismatch, _) => {} // 사람이 봐야 한다
            (Verdict::OnlyLeft, Direction::LeftToRight | Direction::Both) => {
                if !e.left.is_some_and(|m| m.is_dir) {
                    out.bytes += size(e.left);
                }
                out.copies.push((lp, rp, false));
            }
            (Verdict::OnlyLeft, Direction::RightToLeft) => {
                if mirror {
                    out.deletes.push(lp);
                }
            }
            (Verdict::OnlyRight, Direction::RightToLeft | Direction::Both) => {
                if !e.right.is_some_and(|m| m.is_dir) {
                    out.bytes += size(e.right);
                }
                out.copies.push((rp, lp, false));
            }
            (Verdict::OnlyRight, Direction::LeftToRight) => {
                if mirror {
                    out.deletes.push(rp);
                }
            }
            // 한 방향 = 원본 쪽이 늘 이긴다(미러 · TC "왼쪽 → 오른쪽") · 양쪽 = 새로운 쪽이 이긴다.
            (
                Verdict::LeftNewer | Verdict::RightNewer | Verdict::Differ,
                Direction::LeftToRight,
            )
            | (Verdict::LeftNewer, Direction::Both) => {
                out.bytes += size(e.left);
                out.copies.push((lp, rp, true));
            }
            (
                Verdict::LeftNewer | Verdict::RightNewer | Verdict::Differ,
                Direction::RightToLeft,
            )
            | (Verdict::RightNewer, Direction::Both) => {
                out.bytes += size(e.right);
                out.copies.push((rp, lp, true));
            }
            (Verdict::Differ, Direction::Both) => {} // 같은 시각 · 다른 내용 = 사람이 고른다
        }
    }
    out
}

/// 계획의 복사를 실행한다(폴더는 만들기만 · 파일은 전송 엔진으로 · 대상 상위 폴더 자동 생성) — 실패는 항목별 격리.
pub fn apply_copies(
    plan: &SyncPlan,
    on_bytes: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> std::io::Result<Vec<(PathBuf, String)>> {
    let mut failed = Vec::new();
    for (src, dest, overwrite) in &plan.copies {
        if cancel.load(Ordering::Relaxed) {
            return Err(canceled());
        }
        if src.is_dir() {
            if let Err(e) = std::fs::create_dir_all(dest) {
                failed.push((src.clone(), e.to_string()));
            }
            continue;
        }
        if let Some(parent) = dest.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                failed.push((src.clone(), e.to_string()));
                continue;
            }
        }
        match crate::copy_onto_with_progress(src, dest, *overwrite, on_bytes, cancel) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => return Err(e),
            Err(e) => failed.push((src.clone(), e.to_string())),
        }
    }
    Ok(failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nexa_cmp_{}_{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn touch(p: &Path, body: &[u8], secs: u64) {
        std::fs::write(p, body).unwrap();
        let f = std::fs::OpenOptions::new().write(true).open(p).unwrap();
        f.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs))
            .unwrap();
    }

    /// 판정(MC/DC): 허용 오차 · DST 1시간 · 크기 · 종류 불일치 · 폴더는 늘 같음.
    #[test]
    fn judge_rules() {
        let m = |size, mtime| Meta {
            is_dir: false,
            size,
            mtime,
        };
        let o = CmpOpts::default();
        assert_eq!(judge(m(5, 100), m(5, 101), o), Verdict::Same, "2초 안");
        assert_eq!(judge(m(5, 100), m(5, 103), o), Verdict::RightNewer);
        assert_eq!(judge(m(5, 110), m(5, 100), o), Verdict::LeftNewer);
        assert_eq!(judge(m(5, 100), m(6, 100), o), Verdict::Differ);
        assert_eq!(judge(m(5, 3700), m(5, 100), o), Verdict::LeftNewer);
        let dst = CmpOpts {
            ignore_dst_hour: true,
            ..o
        };
        assert_eq!(
            judge(m(5, 3700), m(5, 100), dst),
            Verdict::Same,
            "DST 1시간"
        );
        assert_eq!(judge(m(5, 3700), m(7, 100), dst), Verdict::Differ);
        let d = Meta {
            is_dir: true,
            size: 0,
            mtime: 0,
        };
        assert_eq!(judge(d, m(1, 1), o), Verdict::TypeMismatch);
        assert_eq!(judge(d, d, o), Verdict::Same);
    }

    /// 두 폴더: 왼쪽만 · 오른쪽만 · 같음 · 왼쪽 새로움 · 크기 다름 · 하위 폴더 · 내용 비교 → 계획(→ · ← · 양쪽 · 미러 삭제 · 일부만) ·
    /// 적용 → 오른쪽에 생김 · 취소.
    #[test]
    fn compare_and_plan_and_apply() {
        let d = fixture("cmp");
        let (l, r) = (d.join("L"), d.join("R"));
        std::fs::create_dir_all(l.join("sub")).unwrap();
        std::fs::create_dir_all(r.join("sub")).unwrap();
        touch(&l.join("same.txt"), b"same", 1000);
        touch(&r.join("same.txt"), b"same", 1001);
        touch(&l.join("lnew.txt"), b"left new", 2000);
        touch(&r.join("lnew.txt"), b"left old", 1000);
        touch(&l.join("size.txt"), b"abc", 1000);
        touch(&r.join("size.txt"), b"abcd", 1000);
        touch(&l.join("onlyl.txt"), b"L", 1000);
        touch(&r.join("onlyr.txt"), b"R", 1000);
        touch(&l.join("sub").join("deep.txt"), b"deep", 1000);
        touch(&l.join("hid.txt"), b"AAAA", 1000);
        touch(&r.join("hid.txt"), b"BBBB", 1000); // 크기 · 시각 같고 내용만 다름
        let prog = CmpProgress::default();
        let res = compare(&l, &r, CmpOpts::default(), &prog, &AtomicBool::new(false)).unwrap();
        let v = |rel: &str| res.iter().find(|e| e.rel == rel).map(|e| e.verdict);
        assert_eq!(v("same.txt"), Some(Verdict::Same));
        assert_eq!(v("lnew.txt"), Some(Verdict::LeftNewer));
        assert_eq!(v("size.txt"), Some(Verdict::Differ));
        assert_eq!(v("onlyl.txt"), Some(Verdict::OnlyLeft));
        assert_eq!(v("onlyr.txt"), Some(Verdict::OnlyRight));
        assert_eq!(v("sub"), Some(Verdict::Same));
        assert_eq!(v("sub/deep.txt"), Some(Verdict::OnlyLeft));
        assert_eq!(v("hid.txt"), Some(Verdict::Same), "메타만 보면 같다");
        assert!(prog.scanned.load(Ordering::Relaxed) >= 11);
        let by_content = compare(
            &l,
            &r,
            CmpOpts {
                by_content: true,
                ..CmpOpts::default()
            },
            &CmpProgress::default(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(
            by_content
                .iter()
                .find(|e| e.rel == "hid.txt")
                .unwrap()
                .verdict,
            Verdict::Differ
        );
        // 계획 →(미러): 복사 = lnew(덮어씀) · size(덮어씀) · onlyl · sub/deep · 삭제 = R/onlyr.
        let p = plan(&res, &l, &r, Direction::LeftToRight, true, None);
        let dests: Vec<String> = p
            .copies
            .iter()
            .map(|(_, d, _)| {
                d.strip_prefix(&r)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert_eq!(
            dests,
            ["lnew.txt", "onlyl.txt", "size.txt", "sub/deep.txt"],
            "{dests:?}"
        );
        assert_eq!(p.deletes, vec![r.join("onlyr.txt")]);
        assert!(p
            .copies
            .iter()
            .any(|(_, d, ow)| d.ends_with("lnew.txt") && *ow));
        assert_eq!(p.bytes, 8 + 1 + 3 + 4);
        // ←: onlyr 복사 · 다른 것은 전부 오른쪽이 이긴다(한 방향 = 원본 쪽 기준 · lnew도 오른쪽 것으로 덮인다).
        let p2 = plan(&res, &l, &r, Direction::RightToLeft, false, None);
        assert!(p2.copies.iter().any(|(_, d, _)| d.ends_with("onlyr.txt")));
        assert!(p2.copies.iter().any(|(_, d, _)| d.ends_with("size.txt")));
        assert!(p2
            .copies
            .iter()
            .any(|(_, d, ow)| d.ends_with("lnew.txt") && *ow));
        assert!(p2.deletes.is_empty());
        // 양쪽: 한쪽에만 있는 것 양방향 · 새로운 쪽 이김 · Differ는 안 건드림.
        let p3 = plan(&res, &l, &r, Direction::Both, true, None);
        assert_eq!(p3.copies.len(), 4, "{:?}", p3.copies);
        assert!(p3.deletes.is_empty());
        // 일부만.
        let p4 = plan(
            &res,
            &l,
            &r,
            Direction::LeftToRight,
            false,
            Some(&["onlyl.txt".to_string()]),
        );
        assert_eq!(p4.copies.len(), 1);
        // 적용.
        let mut sum = 0u64;
        let failed = apply_copies(&p, &mut |n| sum += n, &AtomicBool::new(false)).unwrap();
        assert!(failed.is_empty(), "{failed:?}");
        assert_eq!(sum, p.bytes);
        assert_eq!(std::fs::read(r.join("lnew.txt")).unwrap(), b"left new");
        assert_eq!(
            std::fs::read(r.join("sub").join("deep.txt")).unwrap(),
            b"deep"
        );
        assert!(r.join("onlyr.txt").exists(), "삭제는 앱이 휴지통으로");
        let again = compare(
            &l,
            &r,
            CmpOpts::default(),
            &CmpProgress::default(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(
            again
                .iter()
                .all(|e| e.verdict == Verdict::Same || e.rel == "onlyr.txt"),
            "{again:?}"
        );
        let e = apply_copies(&p, &mut |_| {}, &AtomicBool::new(true)).unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::Interrupted);
        let _ = std::fs::remove_dir_all(&d);
    }
}
