//! App — 폴더 크기 계산(T-166 · NEW-037 · dir3 신규 — dir2와 탐색기 목록에는 없고 탐색기 속성 창에만 있다 · 사용자 10-05).
//!
//! 정보 도크에 **폴더 1개**가 보이면 작업 스레드가 그 트리를 훑어 바이트 · 파일 수 · 폴더 수를 재고, 재는 동안은 진행값을
//! 보여 준다. 규약(체크섬 T-167과 같은 틀): **한 번에 하나**(새 대상이 오면 이전 작업은 취소 · 세대 번호 가드) · 결과는 경로별
//! 캐시 · 그 트리에 변경이 닿으면(폴더 감시 · 전송 · 삭제 · 이름 변경) 캐시를 비운다 · 설정 `dock.folder_size`로 끌 수 있고
//! 성능 향상 모드에서는 끈다. 링크(심볼릭 링크 · 정션)는 따라가지 않는다(대상이 다른 곳에 있다 — 전송 엔진과 같은 규칙).

use crate::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// 작업 스레드 ↔ UI 공유 상태(진행값은 훑는 동안 계속 커진다).
#[derive(Debug, Default)]
pub(crate) struct DirSizeShared {
    cancel: AtomicBool,
    bytes: AtomicU64,
    files: AtomicU64,
    dirs: AtomicU64,
    /// 읽지 못한 폴더 수(권한 · 사라짐) — 0이 아니면 결과는 "일부".
    errors: AtomicU64,
    done: AtomicBool,
}

/// 진행 중인 계산 1건.
#[derive(Debug)]
pub(crate) struct DirSizeJob {
    pub(crate) path: PathBuf,
    shared: Arc<DirSizeShared>,
}

/// 끝난 계산.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DirSize {
    pub(crate) bytes: u64,
    pub(crate) files: u64,
    pub(crate) dirs: u64,
    /// 읽지 못한 폴더가 있었다.
    pub(crate) partial: bool,
}

/// 폴더 크기 상태(App 필드 하나).
#[derive(Debug, Default)]
pub(crate) struct DirSizes {
    job: Option<DirSizeJob>,
    cache: HashMap<PathBuf, DirSize>,
}

/// 캐시 항목 `key`가 `changed` 폴더의 변경에 영향을 받는가(순수): 바뀐 폴더가 항목 자신이거나 그 안(조상 항목의 합계가 변함) ·
/// 또는 항목이 바뀐 폴더 안(항목 자체가 지워지거나 바뀌었을 수 있다).
pub(crate) fn stale(key: &Path, changed: &Path) -> bool {
    changed.starts_with(key) || key.starts_with(changed)
}

/// 트리를 훑는다(작업 스레드 · 반복문 — 깊은 트리에서 스택을 쓰지 않는다). 링크는 세지도 따라가지도 않는다 ·
/// 열거 실패는 `errors`에 세고 계속 · `cancel`이면 바로 그친다. `tick`은 항목마다 불린다(진행 통지용).
pub(crate) fn walk(root: &Path, sh: &DirSizeShared) {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if sh.cancel.load(Ordering::Relaxed) {
            return;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else {
            sh.errors.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        for e in rd {
            if sh.cancel.load(Ordering::Relaxed) {
                return;
            }
            let Ok(e) = e else {
                sh.errors.fetch_add(1, Ordering::Relaxed);
                continue;
            };
            let Ok(ft) = e.file_type() else {
                sh.errors.fetch_add(1, Ordering::Relaxed);
                continue;
            };
            let p = e.path();
            if ft.is_symlink() || ndir_ops::is_link(&p) {
                continue;
            }
            if ft.is_dir() {
                sh.dirs.fetch_add(1, Ordering::Relaxed);
                stack.push(p);
            } else {
                sh.files.fetch_add(1, Ordering::Relaxed);
                if let Ok(m) = e.metadata() {
                    sh.bytes.fetch_add(m.len(), Ordering::Relaxed);
                }
            }
        }
    }
}

impl DirSizeShared {
    fn snapshot(&self) -> DirSize {
        DirSize {
            bytes: self.bytes.load(Ordering::Relaxed),
            files: self.files.load(Ordering::Relaxed),
            dirs: self.dirs.load(Ordering::Relaxed),
            partial: self.errors.load(Ordering::Relaxed) > 0,
        }
    }
}

impl DirSizes {
    /// `path`의 크기를 요청한다 — 캐시에 있거나 같은 경로를 재는 중이면 무동작 · 다른 경로를 재는 중이면 그것을 취소하고 시작.
    pub(crate) fn request(&mut self, path: &Path) {
        if self.cache.contains_key(path) {
            return;
        }
        if let Some(j) = &self.job {
            if j.path == path {
                return;
            }
        }
        self.cancel();
        let shared = Arc::new(DirSizeShared::default());
        let sh = Arc::clone(&shared);
        let root = path.to_path_buf();
        std::thread::spawn(move || {
            walk(&root, &sh);
            sh.done.store(true, Ordering::Relaxed);
        });
        self.job = Some(DirSizeJob {
            path: path.to_path_buf(),
            shared,
        });
    }

    /// 진행 중인 계산을 취소한다(스레드는 다음 항목에서 멈춘다).
    pub(crate) fn cancel(&mut self) {
        if let Some(j) = self.job.take() {
            j.shared.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// 폴링: 끝난 작업을 캐시로 옮긴다. 돌려주는 값 = 아직 재는 중(깨움이 필요하다).
    pub(crate) fn tick(&mut self) -> bool {
        let Some(j) = &self.job else {
            return false;
        };
        if !j.shared.done.load(Ordering::Relaxed) {
            return true;
        }
        let j = self.job.take().expect("checked above");
        self.cache.insert(j.path, j.shared.snapshot());
        false
    }

    /// 재는 중인가(설정 창 · 시험용).
    pub(crate) fn running(&self) -> bool {
        self.job.is_some()
    }

    /// `path`의 상태: 끝남 = `Ok(크기)` · 재는 중 = `Err(Some(진행값))` · 모름 = `Err(None)`.
    pub(crate) fn get(&self, path: &Path) -> Result<DirSize, Option<DirSize>> {
        if let Some(d) = self.cache.get(path) {
            return Ok(*d);
        }
        match &self.job {
            Some(j) if j.path == path => Err(Some(j.shared.snapshot())),
            _ => Err(None),
        }
    }

    /// 바뀐 폴더들에 닿는 캐시 항목을 버리고, 닿는 작업은 취소한다(다음 도크 갱신이 다시 요청한다).
    pub(crate) fn invalidate(&mut self, changed: &[PathBuf]) {
        self.cache
            .retain(|k, _| !changed.iter().any(|c| stale(k, c)));
        if self
            .job
            .as_ref()
            .is_some_and(|j| changed.iter().any(|c| stale(&j.path, c)))
        {
            self.cancel();
        }
    }

    /// 전부 버린다(전송 · 삭제 · 이름 변경 뒤 — 어디가 바뀌었는지 일일이 따지지 않는다).
    pub(crate) fn clear(&mut self) {
        self.cache.clear();
        self.cancel();
    }

    #[cfg(test)]
    pub(crate) fn cached(&self, path: &Path) -> Option<DirSize> {
        self.cache.get(path).copied()
    }
}

/// 정보 도크의 크기 줄(순수): 끝남 = "크기: …" + "포함: 파일 N개, 폴더 M개"(일부면 표시) · 재는 중 = "크기: 계산 중… 지금까지" ·
/// 모름 = 없음.
pub(crate) fn size_lines(state: Result<DirSize, Option<DirSize>>) -> Vec<String> {
    let long = crate::dockinfo::fmt_size_long;
    match state {
        Ok(d) => {
            let size = if d.partial {
                trf("info.sizePartial", &[&long(d.bytes)])
            } else {
                trf("info.size", &[&long(d.bytes)])
            };
            vec![
                size,
                trf(
                    "info.contains",
                    &[
                        &crate::dockinfo::group_thousands(d.files),
                        &crate::dockinfo::group_thousands(d.dirs),
                    ],
                ),
            ]
        }
        Err(Some(p)) => vec![trf(
            "info.sizeCalc",
            &[
                &filelist::format_size(p.bytes),
                &crate::dockinfo::group_thousands(p.files),
            ],
        )],
        Err(None) => Vec::new(),
    }
}

impl App {
    /// 폴더 크기 계산을 쓰는가 — 설정 `dock.folder_size` · 성능 향상 모드면 끔.
    pub(crate) fn dirsize_enabled(&self) -> bool {
        self.settings.flag("dock.folder_size") && !self.settings.flag("perf.boost")
    }

    /// 도크 갱신 길목: 정보 도크가 보여 줄 폴더 1개가 있으면 그 크기를 요청하고, 없으면 재던 것을 멈춘다.
    pub(crate) fn dirsize_sync(&mut self, folder: Option<&Path>) {
        match folder {
            Some(p) if self.dirsize_enabled() && !ndir_vfs::is_virtual_root(p) => {
                self.dirsizes.request(p);
            }
            _ => self.dirsizes.cancel(),
        }
    }

    /// 폴링(`OPS_POLL_MS`) — 재는 중이면 도크의 진행값을 갱신하고 `true`.
    pub(crate) fn dirsize_tick(&mut self) -> bool {
        let was = self.dirsizes.running();
        let live = self.dirsizes.tick();
        if was {
            self.update_docks();
            self.redraw();
        }
        live
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ndir-dirsize-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// 캐시 무효화 판정(MC/DC): 자신 · 안쪽 변경 · 바깥 조상 변경 · 무관.
    #[test]
    fn stale_rules() {
        let k = Path::new("D:/a/b");
        assert!(stale(k, Path::new("D:/a/b")));
        assert!(stale(k, Path::new("D:/a/b/c/d")));
        assert!(stale(k, Path::new("D:/a")));
        assert!(!stale(k, Path::new("D:/a/bb")));
        assert!(!stale(k, Path::new("E:/x")));
    }

    /// 훑기: 바이트 · 파일 · 폴더 합계 · 빈 폴더 · 없는 폴더 = 오류 1 · 취소 = 즉시 그침.
    #[test]
    fn walk_counts_and_cancels() {
        let d = fixture("walk");
        std::fs::create_dir_all(d.join("x").join("y")).unwrap();
        std::fs::create_dir_all(d.join("empty")).unwrap();
        std::fs::write(d.join("a.txt"), vec![1u8; 100]).unwrap();
        std::fs::write(d.join("x").join("b.txt"), vec![1u8; 250]).unwrap();
        std::fs::write(d.join("x").join("y").join("c.txt"), vec![1u8; 7]).unwrap();
        let sh = DirSizeShared::default();
        walk(&d, &sh);
        assert_eq!(
            sh.snapshot(),
            DirSize {
                bytes: 357,
                files: 3,
                dirs: 3,
                partial: false
            }
        );
        let sh = DirSizeShared::default();
        walk(&d.join("nope"), &sh);
        assert!(sh.snapshot().partial);
        let sh = DirSizeShared::default();
        sh.cancel.store(true, Ordering::Relaxed);
        walk(&d, &sh);
        assert_eq!(sh.snapshot().files, 0);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// 상태 기계: 요청 → 재는 중(진행값) → 끝 = 캐시 · 같은 경로 재요청 무동작 · 다른 경로 = 교체 · 무효화 = 캐시 제거.
    #[test]
    fn request_tick_cache_and_invalidate() {
        let d = fixture("state");
        std::fs::write(d.join("a"), b"12345").unwrap();
        let mut s = DirSizes::default();
        assert_eq!(s.get(&d), Err(None));
        s.request(&d);
        assert!(s.running());
        let at = std::time::Instant::now();
        while s.tick() {
            assert!(at.elapsed().as_secs() < 10, "계산이 끝나지 않음");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(s.get(&d).map(|x| (x.bytes, x.files)), Ok((5, 1)));
        s.request(&d);
        assert!(!s.running(), "캐시 = 다시 재지 않음");
        s.invalidate(&[d.join("sub")]);
        assert_eq!(s.get(&d), Err(None), "안쪽 변경 = 버림");
        s.request(&d);
        s.request(&d.join("other"));
        assert_eq!(
            s.job.as_ref().map(|j| j.path.clone()),
            Some(d.join("other"))
        );
        s.clear();
        assert!(!s.running());
        let _ = std::fs::remove_dir_all(&d);
    }
}
