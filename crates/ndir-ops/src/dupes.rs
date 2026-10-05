//! 중복 파일 찾기 엔진(T-170 · NEW-041 · dir3 신규 · 사용자 10-05 "대상 목록") — 3단 판정으로 읽기를 최소화한다:
//! ① 크기가 같은 파일만 후보(0바이트는 제외 · 설정 가능한 최소 크기) ② 앞 64 KiB의 CRC32가 같은 것만 ③ 전체 SHA-256이 같은 것 =
//! 중복 묶음. 링크(심볼릭 링크 · 정션)는 따라가지도 세지도 않는다. 취소 · 진행(훑은 파일 수 · 해시한 바이트) · 읽기 실패는 건너뛰고
//! 센다. UI는 묶음별로 "보존 1개"를 고르고 나머지를 휴지통으로 보낸다(앱 쪽).

use crate::hash::{digest, new_digest, Algo};
use std::collections::HashMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// 앞부분 지문 길이(② 단계).
pub const HEAD_BYTES: usize = 64 * 1024;

/// 진행 공유값(작업 스레드가 올리고 UI가 읽는다).
#[derive(Debug, Default)]
pub struct DupProgress {
    /// 훑은 파일 수.
    pub scanned: AtomicU64,
    /// ①을 통과해 해시 대상이 된 파일 수.
    pub candidates: AtomicU64,
    /// 해시한 바이트(② + ③).
    pub hashed: AtomicU64,
    /// 읽지 못한 파일/폴더 수.
    pub errors: AtomicU64,
}

/// 중복 묶음 — 같은 크기 · 같은 SHA-256.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DupGroup {
    pub size: u64,
    pub sha256: String,
    /// 발견 순서(훑은 순서) — UI가 보존 규칙으로 다시 정렬한다.
    pub paths: Vec<PathBuf>,
}

impl DupGroup {
    /// 이 묶음에서 1개만 남길 때 되찾는 바이트.
    #[must_use]
    pub fn reclaimable(&self) -> u64 {
        self.size * (self.paths.len() as u64 - 1)
    }
}

/// 검색 옵션.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DupOpts {
    /// 이 크기 미만은 무시(기본 1 — 0바이트 파일은 전부 "같아서" 뜻이 없다).
    pub min_size: u64,
    /// 하위 폴더 포함.
    pub recursive: bool,
}

impl Default for DupOpts {
    fn default() -> Self {
        DupOpts {
            min_size: 1,
            recursive: true,
        }
    }
}

fn canceled() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Interrupted, "canceled")
}

/// `roots`(폴더 또는 파일) 아래를 훑어 중복 묶음을 찾는다 — 묶음은 되찾는 바이트가 큰 것부터. 취소 = `Interrupted`.
pub fn find(
    roots: &[PathBuf],
    opts: DupOpts,
    progress: &DupProgress,
    cancel: &AtomicBool,
) -> std::io::Result<Vec<DupGroup>> {
    // ① 크기별로 모은다.
    let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    let mut stack: Vec<PathBuf> = roots.to_vec();
    while let Some(p) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            return Err(canceled());
        }
        let Ok(md) = std::fs::symlink_metadata(&p) else {
            progress.errors.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        if md.file_type().is_symlink() || crate::is_link(&p) {
            continue;
        }
        if md.is_dir() {
            if !opts.recursive && !roots.contains(&p) {
                continue;
            }
            match std::fs::read_dir(&p) {
                Ok(rd) => {
                    for e in rd {
                        match e {
                            Ok(e) => stack.push(e.path()),
                            Err(_) => {
                                progress.errors.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
                Err(_) => {
                    progress.errors.fetch_add(1, Ordering::Relaxed);
                }
            }
        } else if md.is_file() {
            progress.scanned.fetch_add(1, Ordering::Relaxed);
            if md.len() >= opts.min_size {
                by_size.entry(md.len()).or_default().push(p);
            }
        }
    }
    let mut groups: Vec<DupGroup> = Vec::new();
    let mut sizes: Vec<u64> = by_size
        .iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(k, _)| *k)
        .collect();
    sizes.sort_unstable_by(|a, b| b.cmp(a));
    for size in sizes {
        let paths = &by_size[&size];
        progress
            .candidates
            .fetch_add(paths.len() as u64, Ordering::Relaxed);
        // ② 앞부분 지문.
        let mut by_head: HashMap<String, Vec<&PathBuf>> = HashMap::new();
        for p in paths {
            if cancel.load(Ordering::Relaxed) {
                return Err(canceled());
            }
            match head_fingerprint(p, progress) {
                Some(fp) => by_head.entry(fp).or_default().push(p),
                None => {
                    progress.errors.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        // ③ 전체 해시(지문이 같은 둘 이상만 · 파일이 지문 길이 이하면 지문이 곧 전체라 다시 읽지 않는다).
        for (_, same_head) in by_head.into_iter().filter(|(_, v)| v.len() > 1) {
            let mut by_full: HashMap<String, Vec<PathBuf>> = HashMap::new();
            for p in same_head {
                if cancel.load(Ordering::Relaxed) {
                    return Err(canceled());
                }
                let full = if size <= HEAD_BYTES as u64 {
                    std::fs::read(p).ok().map(|d| digest(Algo::Sha256, &d))
                } else {
                    full_sha256(p, progress, cancel)?
                };
                match full {
                    Some(h) => by_full.entry(h).or_default().push(p.clone()),
                    None => {
                        progress.errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
            for (sha256, mut paths) in by_full.into_iter().filter(|(_, v)| v.len() > 1) {
                paths.sort();
                groups.push(DupGroup {
                    size,
                    sha256,
                    paths,
                });
            }
        }
    }
    groups.sort_by(|a, b| {
        b.reclaimable()
            .cmp(&a.reclaimable())
            .then_with(|| a.paths.cmp(&b.paths))
    });
    Ok(groups)
}

fn head_fingerprint(p: &Path, progress: &DupProgress) -> Option<String> {
    let mut f = std::fs::File::open(p).ok()?;
    let mut buf = vec![0u8; HEAD_BYTES];
    let mut got = 0usize;
    while got < buf.len() {
        let n = f.read(&mut buf[got..]).ok()?;
        if n == 0 {
            break;
        }
        got += n;
    }
    progress.hashed.fetch_add(got as u64, Ordering::Relaxed);
    Some(digest(Algo::Crc32, &buf[..got]))
}

/// 전체 SHA-256 — 읽기 실패 = `Ok(None)`(건너뜀) · 취소 = `Err`.
fn full_sha256(
    p: &Path,
    progress: &DupProgress,
    cancel: &AtomicBool,
) -> std::io::Result<Option<String>> {
    let Ok(mut f) = std::fs::File::open(p) else {
        return Ok(None);
    };
    let mut d = new_digest(Algo::Sha256);
    let mut buf = vec![0u8; 1 << 20];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(canceled());
        }
        let n = match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => return Ok(None),
        };
        d.update(&buf[..n]);
        progress.hashed.fetch_add(n as u64, Ordering::Relaxed);
    }
    Ok(Some(d.finish()))
}

/// 보존 규칙(순수) — 묶음에서 남길 하나의 인덱스.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    /// 가장 최근에 수정한 것.
    Newest,
    /// 가장 오래된 것.
    Oldest,
    /// 경로가 가장 짧은 것(얕은 폴더).
    ShortestPath,
    /// 첫 번째(이름순).
    First,
}

/// `keep` 규칙으로 남길 항목의 인덱스 — `mtimes` = 경로별 수정 시각(모르면 0).
#[must_use]
pub fn keep_index(paths: &[PathBuf], mtimes: &[i64], keep: Keep) -> usize {
    if paths.is_empty() {
        return 0;
    }
    let idx = 0..paths.len();
    match keep {
        Keep::First => 0,
        Keep::Newest => idx
            .max_by_key(|&i| (mtimes.get(i).copied().unwrap_or(0), std::cmp::Reverse(i)))
            .unwrap_or(0),
        Keep::Oldest => idx
            .min_by_key(|&i| (mtimes.get(i).copied().unwrap_or(i64::MAX), i))
            .unwrap_or(0),
        Keep::ShortestPath => idx
            .min_by_key(|&i| (paths[i].as_os_str().len(), i))
            .unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nexa_dupes_{}_{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// 같은 크기 · 다른 내용(앞부분 같고 뒤 다름 포함)은 묶이지 않고, 같은 내용은 폴더가 달라도 묶인다 · 0바이트 제외 ·
    /// 묶음은 되찾는 바이트 큰 것부터 · 진행값 · 취소.
    #[test]
    fn finds_groups_by_size_head_and_full_hash() {
        let d = fixture("find");
        std::fs::create_dir_all(d.join("a").join("deep")).unwrap();
        std::fs::create_dir_all(d.join("b")).unwrap();
        let big: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        let mut big_tail = big.clone();
        *big_tail.last_mut().unwrap() ^= 0xFF; // 앞 64 KiB 같음 · 끝만 다름
        std::fs::write(d.join("a").join("big1.bin"), &big).unwrap();
        std::fs::write(d.join("b").join("big2.bin"), &big).unwrap();
        std::fs::write(d.join("a").join("deep").join("big3.bin"), &big).unwrap();
        std::fs::write(d.join("b").join("big_tail.bin"), &big_tail).unwrap();
        std::fs::write(d.join("s1.txt"), b"same small").unwrap();
        std::fs::write(d.join("a").join("s2.txt"), b"same small").unwrap();
        std::fs::write(d.join("other.txt"), b"other same").unwrap(); // 같은 크기 · 다른 내용
        std::fs::write(d.join("e1"), b"").unwrap();
        std::fs::write(d.join("e2"), b"").unwrap();
        let prog = DupProgress::default();
        let groups = find(
            std::slice::from_ref(&d),
            DupOpts::default(),
            &prog,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(groups.len(), 2, "{groups:?}");
        assert_eq!(groups[0].size, big.len() as u64);
        assert_eq!(groups[0].paths.len(), 3);
        assert_eq!(groups[0].reclaimable(), 2 * big.len() as u64);
        assert_eq!(groups[1].size, 10);
        assert_eq!(groups[1].paths.len(), 2);
        assert_eq!(groups[1].sha256, digest(Algo::Sha256, b"same small"));
        assert_eq!(prog.scanned.load(Ordering::Relaxed), 9);
        assert!(prog.candidates.load(Ordering::Relaxed) >= 7);
        assert!(prog.hashed.load(Ordering::Relaxed) > 3 * big.len() as u64);
        // 비재귀 = 최상위만.
        let flat = find(
            std::slice::from_ref(&d),
            DupOpts {
                min_size: 1,
                recursive: false,
            },
            &DupProgress::default(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(flat.is_empty(), "{flat:?}");
        // 최소 크기로 작은 묶음 제외.
        let only_big = find(
            std::slice::from_ref(&d),
            DupOpts {
                min_size: 1000,
                recursive: true,
            },
            &DupProgress::default(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(only_big.len(), 1);
        // 취소.
        let e = find(
            std::slice::from_ref(&d),
            DupOpts::default(),
            &DupProgress::default(),
            &AtomicBool::new(true),
        )
        .unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::Interrupted);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn keep_rules() {
        let p: Vec<PathBuf> = ["D:/x/longer/path/f.txt", "D:/x/f.txt", "D:/y/f.txt"]
            .iter()
            .map(PathBuf::from)
            .collect();
        let mt = [100, 300, 200];
        assert_eq!(keep_index(&p, &mt, Keep::First), 0);
        assert_eq!(keep_index(&p, &mt, Keep::Newest), 1);
        assert_eq!(keep_index(&p, &mt, Keep::Oldest), 0);
        assert_eq!(
            keep_index(&p, &mt, Keep::ShortestPath),
            1,
            "같은 길이면 앞쪽"
        );
        assert_eq!(keep_index(&[], &[], Keep::Newest), 0);
    }
}
