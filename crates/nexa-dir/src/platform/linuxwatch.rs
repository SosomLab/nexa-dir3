//! Linux 폴더 감시(T-53 · docs/port/19 §4-5 L08·L11 · SHELL-030~033의 inotify 대응): **inotify 비재귀** · fd 비차단(`IN_NONBLOCK`) ·
//! 스레드 없음 — 호스트 틱 `poll`이 큐를 비운다(디바운스 = 틱 간격 250 ms) · `IN_Q_OVERFLOW` = 전체 폴더 통지 후 **계속**(L11) ·
//! `IN_DELETE_SELF`/`IN_MOVE_SELF`/`IN_IGNORED` = 그 폴더 감시 해제 + 통지(다음 `watch`가 재구독 — SHELL-031 자가 치유) ·
//! 등록 실패(`ENOSPC` max_user_watches · 권한 · 소실)는 그 폴더만 공용 `PollWatcher` 폴백(SHELL-035) · 중지 = `inotify_rm_watch`(논블로킹 · L08).
//! libc 수동 extern(외부 crate 0).

use super::*;
use std::collections::HashSet;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

extern "C" {
    fn inotify_init1(flags: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, pathname: *const c_char, mask: u32) -> c_int;
    fn inotify_rm_watch(fd: c_int, wd: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn close(fd: c_int) -> c_int;
}

const IN_CLOEXEC: c_int = 0o2000000;
const IN_NONBLOCK: c_int = 0o4000;
const IN_MODIFY: u32 = 0x2;
const IN_ATTRIB: u32 = 0x4;
const IN_CLOSE_WRITE: u32 = 0x8;
const IN_MOVED_FROM: u32 = 0x40;
const IN_MOVED_TO: u32 = 0x80;
const IN_CREATE: u32 = 0x100;
const IN_DELETE: u32 = 0x200;
const IN_DELETE_SELF: u32 = 0x400;
const IN_MOVE_SELF: u32 = 0x800;
const IN_Q_OVERFLOW: u32 = 0x4000;
const IN_IGNORED: u32 = 0x8000;
/// 비재귀 · 이름/크기/시각/속성(Windows `ReadDirectoryChangesW` 필터와 같은 범위).
const MASK: u32 = IN_MODIFY
    | IN_ATTRIB
    | IN_CLOSE_WRITE
    | IN_MOVED_FROM
    | IN_MOVED_TO
    | IN_CREATE
    | IN_DELETE
    | IN_DELETE_SELF
    | IN_MOVE_SELF;

/// `struct inotify_event` 머리(이름은 뒤에 `len` 바이트).
const EVENT_HEAD: usize = 16;

pub(super) struct InotifyWatcher {
    /// inotify fd(-1 = 쓸 수 없음 → 전부 폴백).
    fd: c_int,
    /// wd → 폴더.
    wds: HashMap<c_int, PathBuf>,
    /// 마지막 `watch` 집합.
    dirs: Vec<PathBuf>,
    /// 감시가 끊긴 폴더(소실/이동) — 다음 `watch`에서 재시도.
    dead: HashSet<PathBuf>,
    fallback: PollWatcher,
    fallback_dirs: Vec<PathBuf>,
}

impl InotifyWatcher {
    pub(super) fn new() -> Self {
        // SAFETY: 인자만 넘기는 syscall.
        let fd = unsafe { inotify_init1(IN_NONBLOCK | IN_CLOEXEC) };
        InotifyWatcher {
            fd,
            wds: HashMap::new(),
            dirs: Vec::new(),
            dead: HashSet::new(),
            fallback: PollWatcher::default(),
            fallback_dirs: Vec::new(),
        }
    }

    /// 네이티브로 감시 중인 폴더(시험).
    #[cfg(test)]
    fn native_dirs(&self) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = self.wds.values().cloned().collect();
        v.sort();
        v
    }

    fn add(&mut self, dir: &Path) -> bool {
        if self.fd < 0 || !dir.is_dir() {
            return false;
        }
        let Ok(c) = CString::new(dir.as_os_str().as_encoded_bytes()) else {
            return false;
        };
        // SAFETY: 유효한 fd · NUL 종단 경로.
        let wd = unsafe { inotify_add_watch(self.fd, c.as_ptr(), MASK) };
        if wd < 0 {
            return false;
        }
        self.wds.insert(wd, dir.to_path_buf());
        true
    }

    fn remove_wd(&mut self, wd: c_int) {
        if self.wds.remove(&wd).is_some() && self.fd >= 0 {
            // SAFETY: 우리가 등록한 wd — 실패(이미 사라짐)는 무시.
            unsafe {
                let _ = inotify_rm_watch(self.fd, wd);
            }
        }
    }

    /// 큐를 비운다 — 바뀐 폴더 집합(오버플로 = 전부 · 소실 = 해제 + 통지).
    fn drain(&mut self) -> Vec<PathBuf> {
        let mut changed: Vec<PathBuf> = Vec::new();
        if self.fd < 0 {
            return changed;
        }
        let mut buf = vec![0u8; 64 * 1024];
        let mut gone: Vec<c_int> = Vec::new();
        loop {
            // SAFETY: 비차단 fd · 버퍼 길이 전달 · 음수 = 더 없음(EAGAIN)/오류.
            let n = unsafe { read(self.fd, buf.as_mut_ptr().cast(), buf.len()) };
            if n <= 0 {
                break;
            }
            let n = n as usize;
            let mut off = 0usize;
            while off + EVENT_HEAD <= n {
                let wd = c_int::from_ne_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]);
                let mask =
                    u32::from_ne_bytes([buf[off + 4], buf[off + 5], buf[off + 6], buf[off + 7]]);
                let len = u32::from_ne_bytes([
                    buf[off + 12],
                    buf[off + 13],
                    buf[off + 14],
                    buf[off + 15],
                ]) as usize;
                off += EVENT_HEAD + len;
                if mask & IN_Q_OVERFLOW != 0 {
                    for d in self.wds.values() {
                        if !changed.contains(d) {
                            changed.push(d.clone());
                        }
                    }
                    continue;
                }
                let Some(dir) = self.wds.get(&wd).cloned() else {
                    continue;
                };
                if !changed.contains(&dir) {
                    changed.push(dir.clone());
                }
                if mask & (IN_DELETE_SELF | IN_MOVE_SELF | IN_IGNORED) != 0 {
                    gone.push(wd);
                    self.dead.insert(dir);
                }
            }
        }
        for wd in gone {
            // IN_IGNORED 뒤에는 커널이 이미 지웠다 — rm_watch 실패는 무시.
            self.remove_wd(wd);
        }
        changed
    }
}

impl Drop for InotifyWatcher {
    fn drop(&mut self) {
        if self.fd >= 0 {
            // SAFETY: 우리가 연 fd.
            unsafe {
                let _ = close(self.fd);
            }
        }
    }
}

impl Watcher for InotifyWatcher {
    fn watch(&mut self, dirs: &[PathBuf]) {
        if self.dirs == dirs && self.dead.is_empty() {
            return;
        }
        self.dirs = dirs.to_vec();
        // 이탈분 해제.
        let stale: Vec<c_int> = self
            .wds
            .iter()
            .filter(|(_, d)| !dirs.contains(d))
            .map(|(wd, _)| *wd)
            .collect();
        for wd in stale {
            self.remove_wd(wd);
        }
        self.dead.clear();
        // 신규분(끊긴 것 포함) 등록 — 실패는 폴백.
        let mut fallback_dirs: Vec<PathBuf> = Vec::new();
        for d in dirs {
            if self.wds.values().any(|w| w == d) {
                continue;
            }
            if !self.add(d) {
                fallback_dirs.push(d.clone());
            }
        }
        if fallback_dirs != self.fallback_dirs {
            self.fallback_dirs = fallback_dirs;
            self.fallback.watch(&self.fallback_dirs);
        }
    }

    fn poll(&mut self) -> Vec<PathBuf> {
        let mut out = self.drain();
        for d in self.fallback.poll() {
            if !out.contains(&d) {
                out.push(d);
            }
        }
        out
    }

    fn poll_interval_ms(&self) -> u64 {
        if self.fd >= 0 && self.fallback_dirs.is_empty() {
            250
        } else {
            1000
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_for(w: &mut InotifyWatcher, dir: &Path, ms: u64) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        while std::time::Instant::now() < deadline {
            if w.poll().iter().any(|p| p == dir) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        false
    }

    /// 생성·삭제가 그 폴더로 보고된다 · 감시하지 않는 하위 폴더는 보고하지 않는다 · 같은 집합 재지정은 유지 · 없는 폴더 = 폴백(1 s) ·
    /// 폴더 소실 = 해제 + 다음 watch 재구독.
    #[test]
    fn inotify_watcher_reports_changes_and_falls_back() {
        let _g = crate::platform::os_test_guard();
        let base = std::env::temp_dir().join(format!("ndir-inotify-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("other")).unwrap();
        let mut w = InotifyWatcher::new();
        assert!(w.fd >= 0, "inotify_init1");
        w.watch(std::slice::from_ref(&base));
        assert_eq!(w.native_dirs(), vec![base.clone()]);
        assert_eq!(w.poll_interval_ms(), 250);
        let _ = w.poll();
        std::fs::write(base.join("a.txt"), b"1").unwrap();
        assert!(wait_for(&mut w, &base, 5000), "생성 통지");
        std::fs::write(base.join("other").join("b.txt"), b"1").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(
            !w.poll().iter().any(|p| p == &base.join("other")),
            "하위 폴더는 보고하지 않는다"
        );
        std::fs::remove_file(base.join("a.txt")).unwrap();
        assert!(wait_for(&mut w, &base, 5000), "삭제 통지");
        w.watch(std::slice::from_ref(&base));
        assert_eq!(w.native_dirs().len(), 1, "같은 집합 = 유지");
        // 없는 폴더 = 폴백 · 간격 1 s.
        let missing = base.join("missing");
        w.watch(&[base.clone(), missing.clone()]);
        assert_eq!(w.fallback_dirs, vec![missing]);
        assert_eq!(w.poll_interval_ms(), 1000);
        // 소실 → 해제 + 통지 · 재생성 뒤 watch = 재구독.
        let gone = base.join("gone");
        std::fs::create_dir_all(&gone).unwrap();
        w.watch(&[base.clone(), gone.clone()]);
        assert_eq!(w.native_dirs().len(), 2);
        let _ = w.poll();
        std::fs::remove_dir(&gone).unwrap();
        assert!(wait_for(&mut w, &gone, 5000), "소실 통지");
        assert_eq!(w.native_dirs().len(), 1, "소실 폴더는 해제");
        std::fs::create_dir_all(&gone).unwrap();
        w.watch(&[base.clone(), gone.clone()]);
        assert_eq!(w.native_dirs().len(), 2, "재구독");
        let _ = std::fs::remove_dir_all(&base);
    }
}
