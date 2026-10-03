//! Windows 폴더 감시(T-51 B-2b · dir2 `watcher.rs` 이식 — docs/port/19 SHELL-030~033): 폴더 1개당 스레드 1개 ·
//! `CreateFileW(FILE_LIST_DIRECTORY · 전체 공유 · BACKUP_SEMANTICS|OVERLAPPED)` + `ReadDirectoryChangesW`(**비재귀** · 이름/크기/시각/속성) ·
//! 중지 = 수동 리셋 이벤트(`SetEvent` 논블로킹 · 정리는 스레드 몫 — dir2 QA 07-14 UI 프리즈 수정 계승) · 버퍼 오버플로(`ERROR_NOTIFY_ENUM_DIR`)는
//! 통지 1회 후 **계속**(SHELL-033) · 죽음은 `alive`로 관측 → 다음 `watch`가 재구독(SHELL-031 자가 치유).
//!
//! dir3 차이: 창 메시지(`PostMessageW`) 대신 **공유 집합**(`Arc<Mutex<HashSet<PathBuf>>>`)에 폴더를 적고 호스트 틱이 `poll`로 거둔다
//! (창 없는 시험 가능 · 디바운스 = 틱 간격 `poll_interval_ms` 250). 시작 실패(권한 · 네트워크 · 소실) 폴더는 공용 `PollWatcher`(서명 프로브 · SHELL-035)로 폴백.

use super::*;
use ::windows::core::PCWSTR;
use ::windows::Win32::Foundation::{
    CloseHandle, DuplicateHandle, GetLastError, DUPLICATE_SAME_ACCESS, ERROR_NOTIFY_ENUM_DIR,
    HANDLE, WAIT_OBJECT_0,
};
use ::windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadDirectoryChangesW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED,
    FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_ATTRIBUTES, FILE_NOTIFY_CHANGE_DIR_NAME,
    FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use ::windows::Win32::System::Threading::{
    CreateEventW, GetCurrentProcess, SetEvent, WaitForMultipleObjects, INFINITE,
};
use ::windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use std::collections::HashSet;
use std::os::windows::ffi::OsStrExt as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// 변경 폴더 집합(스레드 → 호스트 틱).
type Pending = Arc<Mutex<HashSet<PathBuf>>>;

/// 폴더 1개 감시 — drop 시 중지 이벤트만 신호(논블로킹).
struct DirWatcher {
    path: PathBuf,
    /// 중지 이벤트(수동 리셋) 원시값 — 소유는 이 구조체.
    stop: isize,
    alive: Arc<AtomicBool>,
}

impl DirWatcher {
    fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    /// 감시 시작. 실패(권한·소실·네트워크) = `None` → 폴백.
    fn start(path: &Path, pending: Pending) -> Option<DirWatcher> {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: Win32 핸들 API — 입력은 유효 버퍼 · 실패 경로마다 만든 핸들을 닫는다.
        unsafe {
            let dir = CreateFileW(
                PCWSTR(wide.as_ptr()),
                FILE_LIST_DIRECTORY.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
                None,
            )
            .ok()?;
            let Ok(stop) = CreateEventW(None, true, false, None) else {
                let _ = CloseHandle(dir);
                return None;
            };
            let Ok(io) = CreateEventW(None, false, false, None) else {
                let _ = CloseHandle(dir);
                let _ = CloseHandle(stop);
                return None;
            };
            // 스레드에는 복제 핸들 — drop이 원본을 닫아도 이벤트 객체는 유지(핸들 값 재활용 경쟁 차단).
            let mut stop_thread = HANDLE::default();
            if DuplicateHandle(
                GetCurrentProcess(),
                stop,
                GetCurrentProcess(),
                &mut stop_thread,
                0,
                false,
                DUPLICATE_SAME_ACCESS,
            )
            .is_err()
            {
                let _ = CloseHandle(dir);
                let _ = CloseHandle(stop);
                let _ = CloseHandle(io);
                return None;
            }
            let (dir_raw, stop_raw, io_raw) =
                (dir.0 as isize, stop_thread.0 as isize, io.0 as isize);
            let alive = Arc::new(AtomicBool::new(true));
            let alive_thread = Arc::clone(&alive);
            let dir_path = path.to_path_buf();
            std::thread::Builder::new()
                .name("ndir-watch".into())
                .spawn(move || {
                    Self::run(dir_raw, stop_raw, io_raw, &dir_path, &pending);
                    alive_thread.store(false, Ordering::Relaxed);
                })
                .ok()?;
            Some(DirWatcher {
                path: path.to_path_buf(),
                stop: stop.0 as isize,
                alive,
            })
        }
    }

    /// 감시 스레드 본체 — 종료 때 핸들 3개를 닫는다.
    fn run(dir_raw: isize, stop_raw: isize, io_raw: isize, path: &Path, pending: &Pending) {
        let dir = HANDLE(dir_raw as *mut core::ffi::c_void);
        let io = HANDLE(io_raw as *mut core::ffi::c_void);
        let stop = HANDLE(stop_raw as *mut core::ffi::c_void);
        let mut buf = vec![0u8; 8 * 1024];
        let notify = || {
            if let Ok(mut p) = pending.lock() {
                p.insert(path.to_path_buf());
            }
        };
        loop {
            let mut ov = OVERLAPPED {
                hEvent: io,
                ..Default::default()
            };
            // SAFETY: 핸들·버퍼·OVERLAPPED는 이 루프 동안 유효 · IO 완료/취소 뒤에만 다음 반복.
            let queued = unsafe {
                ReadDirectoryChangesW(
                    dir,
                    buf.as_mut_ptr() as *mut core::ffi::c_void,
                    buf.len() as u32,
                    false,
                    FILE_NOTIFY_CHANGE_FILE_NAME
                        | FILE_NOTIFY_CHANGE_DIR_NAME
                        | FILE_NOTIFY_CHANGE_SIZE
                        | FILE_NOTIFY_CHANGE_LAST_WRITE
                        | FILE_NOTIFY_CHANGE_ATTRIBUTES,
                    None,
                    Some(&mut ov),
                    None,
                )
            };
            if queued.is_err() {
                break;
            }
            let wait = unsafe { WaitForMultipleObjects(&[stop, io], false, INFINITE) };
            if wait.0 != WAIT_OBJECT_0.0 + 1 {
                unsafe {
                    let _ = CancelIoEx(dir, Some(&ov));
                    let mut n = 0u32;
                    let _ = GetOverlappedResult(dir, &ov, &mut n, true);
                }
                break;
            }
            let mut got = 0u32;
            if unsafe { GetOverlappedResult(dir, &ov, &mut got, false) }.is_err() {
                // 변경 폭주로 버퍼가 넘침 = 전체 재열거 신호 — 통지 1회 후 계속(SHELL-033).
                if unsafe { GetLastError() } == ERROR_NOTIFY_ENUM_DIR {
                    notify();
                    continue;
                }
                break;
            }
            // 개별 엔트리는 해석하지 않는다 — 전체 재열거로 수렴.
            notify();
        }
        unsafe {
            let _ = CloseHandle(dir);
            let _ = CloseHandle(io);
            let _ = CloseHandle(stop);
        }
    }
}

impl Drop for DirWatcher {
    fn drop(&mut self) {
        // SAFETY: 우리가 만든 이벤트 핸들 — 신호만 보내고 닫는다(스레드는 복제분을 쓴다).
        unsafe {
            let stop = HANDLE(self.stop as *mut core::ffi::c_void);
            let _ = SetEvent(stop);
            let _ = CloseHandle(stop);
        }
    }
}

/// Windows 감시자 = 네이티브 통지(폴더별 스레드) + 시작 실패 폴더의 폴링 폴백.
pub(super) struct NativeWatcher {
    watchers: Vec<DirWatcher>,
    pending: Pending,
    fallback: PollWatcher,
    fallback_dirs: Vec<PathBuf>,
}

impl NativeWatcher {
    pub(super) fn new() -> Self {
        NativeWatcher {
            watchers: Vec::new(),
            pending: Arc::new(Mutex::new(HashSet::new())),
            fallback: PollWatcher::default(),
            fallback_dirs: Vec::new(),
        }
    }

    /// 네이티브로 감시 중인 폴더(시험·자가 점검).
    pub(super) fn native_dirs(&self) -> Vec<PathBuf> {
        self.watchers
            .iter()
            .filter(|w| w.is_alive())
            .map(|w| w.path.clone())
            .collect()
    }
}

impl Watcher for NativeWatcher {
    fn watch(&mut self, dirs: &[PathBuf]) {
        // 이탈분 drop · 죽은 것 drop(재구독) · 신규분 start — 유지분은 그대로(SHELL-031 diff 재구독).
        self.watchers
            .retain(|w| w.is_alive() && dirs.contains(&w.path));
        let mut fallback_dirs: Vec<PathBuf> = Vec::new();
        for d in dirs {
            if self.watchers.iter().any(|w| &w.path == d) {
                continue;
            }
            match DirWatcher::start(d, Arc::clone(&self.pending)) {
                Some(w) => self.watchers.push(w),
                None => fallback_dirs.push(d.clone()),
            }
        }
        if fallback_dirs != self.fallback_dirs {
            self.fallback_dirs = fallback_dirs;
            self.fallback.watch(&self.fallback_dirs);
        }
    }

    fn poll(&mut self) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = match self.pending.lock() {
            Ok(mut p) => p.drain().collect(),
            Err(_) => Vec::new(),
        };
        for d in self.fallback.poll() {
            if !out.contains(&d) {
                out.push(d);
            }
        }
        out
    }

    fn poll_interval_ms(&self) -> u64 {
        if self.fallback_dirs.is_empty() {
            250
        } else {
            1000
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_for(w: &mut NativeWatcher, dir: &Path, ms: u64) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        while std::time::Instant::now() < deadline {
            if w.poll().iter().any(|p| p == dir) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        false
    }

    /// 파일 생성·수정·삭제가 1 s 안에 그 폴더로 보고된다 · 다른 폴더는 보고하지 않는다 · 같은 집합 재지정은 유지.
    #[test]
    fn native_watcher_reports_changes_in_watched_dir() {
        let base = std::env::temp_dir().join(format!("ndir-winwatch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("other")).unwrap();
        let mut w = NativeWatcher::new();
        w.watch(std::slice::from_ref(&base));
        assert_eq!(w.native_dirs(), vec![base.clone()]);
        assert_eq!(w.poll_interval_ms(), 250);
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = w.poll();
        std::fs::write(base.join("a.txt"), b"1").unwrap();
        assert!(wait_for(&mut w, &base, 2000), "생성 통지");
        std::fs::write(base.join("other").join("b.txt"), b"1").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(150));
        assert!(
            !w.poll().iter().any(|p| p == &base.join("other")),
            "감시하지 않는 하위 폴더는 보고하지 않는다"
        );
        std::fs::remove_file(base.join("a.txt")).unwrap();
        assert!(wait_for(&mut w, &base, 2000), "삭제 통지");
        w.watch(std::slice::from_ref(&base));
        assert_eq!(w.native_dirs().len(), 1, "같은 집합 = 유지");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// 폴더 소실 = 스레드 죽음이 관측되고 다음 `watch`가 정리/재구독한다 · 없는 폴더 = 폴링 폴백(간격 1 s) · 패닉 없음.
    #[test]
    fn watcher_death_is_observable_and_missing_dir_falls_back() {
        let base = std::env::temp_dir().join(format!("ndir-winwatch-die-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let mut w = NativeWatcher::new();
        w.watch(std::slice::from_ref(&base));
        assert_eq!(w.native_dirs().len(), 1);
        std::fs::remove_dir(&base).unwrap();
        let mut dead = false;
        for _ in 0..100 {
            if w.native_dirs().is_empty() {
                dead = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(dead, "폴더 소실 5 s 뒤에도 생존 = 죽음 은폐");
        let missing = std::env::temp_dir().join("ndir-winwatch-missing-xyz");
        w.watch(std::slice::from_ref(&missing));
        assert!(w.native_dirs().is_empty());
        assert_eq!(w.poll_interval_ms(), 1000, "폴백만 = 폴링 간격");
        let _ = w.poll();
    }
}
