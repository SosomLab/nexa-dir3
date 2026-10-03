//! macOS 폴더 감시(T-52 · docs/port/19 §4-5 · SHELL-030~033의 kqueue 대응): 폴더 fd(`O_EVTONLY`)에 `EVFILT_VNODE`(`NOTE_WRITE`·`DELETE`·
//! `RENAME`·`ATTRIB`·`EXTEND` · `EV_CLEAR`) — 항목 생성/삭제/이름 변경은 폴더 `NOTE_WRITE`로 온다(안의 파일 **내용**만 바뀐 것은 안 온다 —
//! FSEvents는 후속 · 크기/시각은 재열람 때 읽힌다) · 스레드 없음 — 호스트 틱 `poll`이 0 타임아웃 `kevent`로 거둔다 · 폴더 소실/이동 =
//! 해제 + 통지(다음 `watch` 재구독) · 등록 실패(권한 · 소실 · fd 한도)는 그 폴더만 `PollWatcher` 폴백 · 중지 = `close(fd)`(kqueue가 자동 해제).
//! libc 수동 extern(외부 crate 0).

use super::*;
use std::collections::HashSet;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

#[repr(C)]
struct Kevent {
    ident: usize,
    filter: i16,
    flags: u16,
    fflags: u32,
    data: isize,
    udata: *mut c_void,
}

#[repr(C)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

extern "C" {
    fn kqueue() -> c_int;
    fn kevent(
        kq: c_int,
        changelist: *const Kevent,
        nchanges: c_int,
        eventlist: *mut Kevent,
        nevents: c_int,
        timeout: *const Timespec,
    ) -> c_int;
    fn open(path: *const c_char, oflag: c_int, ...) -> c_int;
    fn close(fd: c_int) -> c_int;
}

const O_EVTONLY: c_int = 0x8000;
const O_CLOEXEC: c_int = 0x0100_0000;
const EVFILT_VNODE: i16 = -4;
const EV_ADD: u16 = 0x1;
const EV_CLEAR: u16 = 0x20;
const EV_ERROR: u16 = 0x4000;
const NOTE_DELETE: u32 = 0x1;
const NOTE_WRITE: u32 = 0x2;
const NOTE_EXTEND: u32 = 0x4;
const NOTE_ATTRIB: u32 = 0x8;
const NOTE_RENAME: u32 = 0x20;
const FFLAGS: u32 = NOTE_DELETE | NOTE_WRITE | NOTE_EXTEND | NOTE_ATTRIB | NOTE_RENAME;

pub(super) struct KqueueWatcher {
    /// kqueue fd(-1 = 쓸 수 없음 → 전부 폴백).
    kq: c_int,
    /// 폴더 fd → 폴더.
    fds: HashMap<c_int, PathBuf>,
    dirs: Vec<PathBuf>,
    dead: HashSet<PathBuf>,
    fallback: PollWatcher,
    fallback_dirs: Vec<PathBuf>,
}

impl KqueueWatcher {
    pub(super) fn new() -> Self {
        // SAFETY: 인자 없는 syscall.
        let kq = unsafe { kqueue() };
        KqueueWatcher {
            kq,
            fds: HashMap::new(),
            dirs: Vec::new(),
            dead: HashSet::new(),
            fallback: PollWatcher::default(),
            fallback_dirs: Vec::new(),
        }
    }

    #[cfg(test)]
    fn native_dirs(&self) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = self.fds.values().cloned().collect();
        v.sort();
        v
    }

    fn add(&mut self, dir: &Path) -> bool {
        if self.kq < 0 || !dir.is_dir() {
            return false;
        }
        let Ok(c) = CString::new(dir.as_os_str().as_encoded_bytes()) else {
            return false;
        };
        // SAFETY: NUL 종단 경로 · 실패 = -1.
        let fd = unsafe { open(c.as_ptr(), O_EVTONLY | O_CLOEXEC) };
        if fd < 0 {
            return false;
        }
        let ev = Kevent {
            ident: fd as usize,
            filter: EVFILT_VNODE,
            flags: EV_ADD | EV_CLEAR,
            fflags: FFLAGS,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        let zero = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: 유효한 kq · changelist 1개 · eventlist 없음 · 0 타임아웃.
        let rc = unsafe { kevent(self.kq, &ev, 1, std::ptr::null_mut(), 0, &zero) };
        if rc < 0 {
            // SAFETY: 우리가 연 fd.
            unsafe {
                let _ = close(fd);
            }
            return false;
        }
        self.fds.insert(fd, dir.to_path_buf());
        true
    }

    fn remove_fd(&mut self, fd: c_int) {
        if self.fds.remove(&fd).is_some() {
            // SAFETY: 우리가 연 fd — 닫으면 kqueue 등록도 사라진다.
            unsafe {
                let _ = close(fd);
            }
        }
    }

    fn drain(&mut self) -> Vec<PathBuf> {
        let mut changed: Vec<PathBuf> = Vec::new();
        if self.kq < 0 || self.fds.is_empty() {
            return changed;
        }
        let zero = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let mut gone: Vec<c_int> = Vec::new();
        loop {
            let mut evs: Vec<Kevent> = (0..64)
                .map(|_| Kevent {
                    ident: 0,
                    filter: 0,
                    flags: 0,
                    fflags: 0,
                    data: 0,
                    udata: std::ptr::null_mut(),
                })
                .collect();
            // SAFETY: 유효한 kq · eventlist 64 · 0 타임아웃(비차단).
            let n = unsafe {
                kevent(
                    self.kq,
                    std::ptr::null(),
                    0,
                    evs.as_mut_ptr(),
                    evs.len() as c_int,
                    &zero,
                )
            };
            if n <= 0 {
                break;
            }
            for ev in &evs[..n as usize] {
                let fd = ev.ident as c_int;
                let Some(dir) = self.fds.get(&fd).cloned() else {
                    continue;
                };
                if !changed.contains(&dir) {
                    changed.push(dir.clone());
                }
                if ev.flags & EV_ERROR != 0 || ev.fflags & (NOTE_DELETE | NOTE_RENAME) != 0 {
                    gone.push(fd);
                    self.dead.insert(dir);
                }
            }
            if (n as usize) < evs.len() {
                break;
            }
        }
        for fd in gone {
            self.remove_fd(fd);
        }
        changed
    }
}

impl Drop for KqueueWatcher {
    fn drop(&mut self) {
        let fds: Vec<c_int> = self.fds.keys().copied().collect();
        for fd in fds {
            self.remove_fd(fd);
        }
        if self.kq >= 0 {
            // SAFETY: 우리가 연 kqueue.
            unsafe {
                let _ = close(self.kq);
            }
        }
    }
}

impl Watcher for KqueueWatcher {
    fn watch(&mut self, dirs: &[PathBuf]) {
        if self.dirs == dirs && self.dead.is_empty() {
            return;
        }
        self.dirs = dirs.to_vec();
        let stale: Vec<c_int> = self
            .fds
            .iter()
            .filter(|(_, d)| !dirs.contains(d))
            .map(|(fd, _)| *fd)
            .collect();
        for fd in stale {
            self.remove_fd(fd);
        }
        self.dead.clear();
        let mut fallback_dirs: Vec<PathBuf> = Vec::new();
        for d in dirs {
            if self.fds.values().any(|w| w == d) {
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
        if self.kq >= 0 && self.fallback_dirs.is_empty() {
            250
        } else {
            1000
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_for(w: &mut KqueueWatcher, dir: &Path, ms: u64) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        while std::time::Instant::now() < deadline {
            if w.poll().iter().any(|p| p == dir) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        false
    }

    /// 생성·삭제가 그 폴더로 보고된다 · 하위 폴더는 보고하지 않는다 · 같은 집합 재지정은 유지 · 없는 폴더 = 폴백 · 소실 = 해제 + 재구독.
    #[test]
    fn kqueue_watcher_reports_changes_and_falls_back() {
        let _g = crate::platform::os_test_guard();
        let base = std::env::temp_dir().join(format!("ndir-kqueue-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("other")).unwrap();
        let mut w = KqueueWatcher::new();
        assert!(w.kq >= 0, "kqueue");
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
        let missing = base.join("missing");
        w.watch(&[base.clone(), missing.clone()]);
        assert_eq!(w.fallback_dirs, vec![missing]);
        assert_eq!(w.poll_interval_ms(), 1000);
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
