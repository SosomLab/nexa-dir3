//! 프로세스 식별 포트(DR-5 · T-202 ④ 슬라이스 3 · docs/26 §12.2): `queue/owner.lock`의 소유자가 살아 있는지 — pid만으로는 재사용을 못
//! 가르므로 **pid + 기동 시각**을 함께 대조한다(11-01 기술 결정). 외부 crate 0(DR-8 · FFI 직접 선언 · `procmem` 선례).
//!
//! - [`process_alive`]: ndir-ops 것(unix `kill(0)` · Windows `OpenProcess`)을 감싼 포트.
//! - [`process_start_time`]: Linux `/proc/<pid>/stat` 22번째 칸(부팅 뒤 클럭 틱) · macOS `proc_pid_rusage(RUSAGE_INFO_V2)` `ri_proc_start_abstime` ·
//!   Windows `GetProcessTimes` 생성 FILETIME. 단위는 OS마다 다르지만 **같은 OS 안에서 같은 프로세스면 같은 값**이라 대조에만 쓴다 · 모르면 None.

/// 프로세스가 살아 있나(pid 재사용은 못 가른다 — [`process_start_time`]과 함께).
pub(crate) fn process_alive(pid: u32) -> bool {
    ndir_ops::process_alive(pid)
}

/// 프로세스 기동 시각(OS 고유 단위 · 대조 전용) — 없는 pid · 권한 없음 · 모르는 OS = None.
pub(crate) fn process_start_time(pid: u32) -> Option<u64> {
    imp::start_time(pid)
}

/// `/proc/<pid>/stat` 한 줄 → 22번째 칸 starttime(순수). 실행 파일 이름에 공백·괄호가 있을 수 있어 **마지막 `)`** 뒤부터 센다 —
/// 그 뒤 첫 칸이 3번째(state)라 22번째 = 0부터 19.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn parse_stat_starttime(text: &str) -> Option<u64> {
    let rest = &text[text.rfind(')')? + 1..];
    rest.split_whitespace().nth(19)?.parse().ok()
}

#[cfg(target_os = "linux")]
mod imp {
    pub(super) fn start_time(pid: u32) -> Option<u64> {
        let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        super::parse_stat_starttime(&text)
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::ffi::c_void;

    /// `rusage_info_v2`(sys/resource.h · `sysload.rs` 선례와 같은 배치) — `proc_start_abstime`(mach 절대 시각 · 부팅 기준 · pid별 고정)만 쓴다.
    #[repr(C)]
    #[derive(Default)]
    struct RusageInfoV2 {
        uuid: [u8; 16],
        user_time: u64,
        system_time: u64,
        pkg_idle_wkups: u64,
        interrupt_wkups: u64,
        pageins: u64,
        wired_size: u64,
        resident_size: u64,
        phys_footprint: u64,
        proc_start_abstime: u64,
        proc_exit_abstime: u64,
        child_user_time: u64,
        child_system_time: u64,
        child_pkg_idle_wkups: u64,
        child_interrupt_wkups: u64,
        child_pageins: u64,
        child_elapsed_abstime: u64,
        diskio_bytesread: u64,
        diskio_byteswritten: u64,
    }
    const RUSAGE_INFO_V2: i32 = 2;
    extern "C" {
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut c_void) -> i32;
    }

    pub(super) fn start_time(pid: u32) -> Option<u64> {
        let pid = i32::try_from(pid).ok()?;
        let mut info = RusageInfoV2::default();
        // SAFETY: V2 크기의 버퍼를 넘긴다 · 0이 아니면(없는 pid · 권한) 읽지 않는다.
        let r = unsafe {
            proc_pid_rusage(pid, RUSAGE_INFO_V2, (&mut info as *mut RusageInfoV2).cast())
        };
        (r == 0 && info.proc_start_abstime != 0).then_some(info.proc_start_abstime)
    }
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    /// `FILETIME` — `sysload.rs`와 같은 배치(같은 크레이트의 extern 선언은 서명이 같아야 한다 · `clashing_extern_declarations`).
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn GetProcessTimes(
            process: *mut c_void,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
        fn CloseHandle(handle: isize) -> i32;
    }
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    pub(super) fn start_time(pid: u32) -> Option<u64> {
        // SAFETY: 핸들을 열어 시각만 읽고 바로 닫는다 · 포인터는 지역 변수.
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return None;
            }
            let mut c = FileTime::default();
            let (mut e, mut k, mut u) = (
                FileTime::default(),
                FileTime::default(),
                FileTime::default(),
            );
            let ok = GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u);
            CloseHandle(h as isize);
            (ok != 0).then(|| (u64::from(c.high) << 32) | u64::from(c.low))
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod imp {
    pub(super) fn start_time(_pid: u32) -> Option<u64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 이름에 공백·괄호가 있어도 마지막 `)` 뒤 20번째 칸.
    #[test]
    fn parse_stat_picks_field_22() {
        let line = "4242 (a (b) c) S 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 777 0 0\n";
        assert_eq!(parse_stat_starttime(line), Some(777));
        assert_eq!(parse_stat_starttime("4242 (x) S 1"), None);
        assert_eq!(parse_stat_starttime("no parens"), None);
    }

    /// 자기 pid = 살아 있고 기동 시각이 안정(두 번 같음) · 없는 pid = 죽음 · None.
    #[cfg(any(target_os = "linux", target_os = "macos", windows))]
    #[test]
    fn self_pid_alive_with_stable_start_time() {
        let me = std::process::id();
        assert!(process_alive(me));
        let a = process_start_time(me);
        assert!(a.is_some(), "자기 기동 시각");
        assert_eq!(a, process_start_time(me));
        let gone = u32::MAX - 7;
        assert!(!process_alive(gone));
        assert_eq!(process_start_time(gone), None);
    }
}
