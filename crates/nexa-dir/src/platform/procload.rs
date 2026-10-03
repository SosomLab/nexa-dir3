//! **이 프로세스의 부하**(상태줄 [CPU][메모리][디스크 I/O] 칸 · docs/22 NEW-003 · DR-23) — 누적값 표본([`sample`])과
//! 두 표본 사이의 비율 계산([`load_between`] · 순수).
//!
//! - Windows = `GetProcessTimes` · `K32GetProcessMemoryInfo` · `GetProcessIoCounters`
//! - Linux = `/proc/self/stat`(utime + stime) · `/proc/self/status`(VmRSS) · `/proc/self/io`(rchar · wchar)
//! - macOS = `proc_pid_rusage(RUSAGE_INFO_V2)`
//!
//! 조회는 호스트가 **그릴 일이 있을 때만**(주기 = `statusbar.load_interval_ms`) 부른다 — 스레드 · 타이머 없음.

use std::time::Duration;

/// 누적 표본(프로세스 시작부터).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ProcSample {
    /// CPU 시간(사용자 + 커널 · ns).
    pub cpu_ns: u64,
    /// 지금 물리 메모리(바이트).
    pub rss: u64,
    /// 읽은 바이트(누적).
    pub read: u64,
    /// 쓴 바이트(누적).
    pub write: u64,
}

/// 두 표본 사이의 부하.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Load {
    /// CPU 사용률(%) — 전체 코어 기준(작업 관리자 방식 · 0~100).
    pub cpu_pct: f32,
    /// 물리 메모리(바이트).
    pub rss: u64,
    /// 읽기 바이트/초.
    pub read_bps: u64,
    /// 쓰기 바이트/초.
    pub write_bps: u64,
}

/// 두 표본 → 부하(순수). `dt` = 표본 사이 시간 · `cores` = 논리 코어 수(0 = 1로 본다). 누적값이 줄었으면(있을 수 없지만) 0.
pub(crate) fn load_between(prev: ProcSample, cur: ProcSample, dt: Duration, cores: usize) -> Load {
    let secs = dt.as_secs_f64();
    if secs <= 0.0 {
        return Load {
            rss: cur.rss,
            ..Load::default()
        };
    }
    let cpu = cur.cpu_ns.saturating_sub(prev.cpu_ns) as f64 / 1e9;
    let pct = (cpu / secs / cores.max(1) as f64 * 100.0).clamp(0.0, 100.0);
    let rate = |a: u64, b: u64| (b.saturating_sub(a) as f64 / secs).round() as u64;
    Load {
        cpu_pct: pct as f32,
        rss: cur.rss,
        read_bps: rate(prev.read, cur.read),
        write_bps: rate(prev.write, cur.write),
    }
}

/// `/proc/self/stat` 한 줄 → CPU 시간(ns · 순수). 실행 파일 이름에 공백 · 괄호가 있을 수 있어 **마지막 `)`** 뒤부터 센다:
/// 그 뒤 12 · 13번째(0부터 11 · 12) = utime · stime(클럭 틱). `tick_hz` = 초당 틱(보통 100).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn parse_proc_stat(text: &str, tick_hz: u64) -> Option<u64> {
    let rest = &text[text.rfind(')')? + 1..];
    let mut it = rest.split_whitespace();
    let utime: u64 = it.nth(11)?.parse().ok()?;
    let stime: u64 = it.next()?.parse().ok()?;
    Some((utime + stime).saturating_mul(1_000_000_000 / tick_hz.max(1)))
}

/// `키:  값 [kB]` 꼴 본문에서 그 키의 숫자(순수 · `/proc/self/status` · `/proc/self/io`).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn parse_kv_number(text: &str, key: &str) -> Option<u64> {
    text.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        (k.trim() == key)
            .then(|| v.split_whitespace().next()?.parse().ok())
            .flatten()
    })
}

/// 지금 표본(조회 실패 · 미지원 OS = `None`).
pub(crate) fn sample() -> Option<ProcSample> {
    imp::sample()
}

#[cfg(target_os = "linux")]
mod imp {
    use super::*;

    extern "C" {
        fn sysconf(name: i32) -> i64;
    }
    /// `_SC_CLK_TCK`(Linux).
    const SC_CLK_TCK: i32 = 2;

    pub(super) fn sample() -> Option<ProcSample> {
        // SAFETY: sysconf는 인자 하나짜리 순수 조회다.
        let hz = unsafe { sysconf(SC_CLK_TCK) };
        let hz = if hz > 0 { hz as u64 } else { 100 };
        let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
        let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
        // io는 권한에 따라 못 읽을 수 있다(그때는 0).
        let io = std::fs::read_to_string("/proc/self/io").unwrap_or_default();
        Some(ProcSample {
            cpu_ns: parse_proc_stat(&stat, hz)?,
            rss: parse_kv_number(&status, "VmRSS").unwrap_or(0) * 1024,
            read: parse_kv_number(&io, "rchar").unwrap_or(0),
            write: parse_kv_number(&io, "wchar").unwrap_or(0),
        })
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    impl FileTime {
        /// 100 ns 단위 → ns.
        fn ns(&self) -> u64 {
            ((u64::from(self.high) << 32) | u64::from(self.low)).saturating_mul(100)
        }
    }

    #[repr(C)]
    #[derive(Default)]
    struct Pmc {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[repr(C)]
    #[derive(Default)]
    struct IoCounters {
        read_ops: u64,
        write_ops: u64,
        other_ops: u64,
        read_bytes: u64,
        write_bytes: u64,
        other_bytes: u64,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn GetProcessTimes(
            process: *mut c_void,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
        fn K32GetProcessMemoryInfo(process: *mut c_void, counters: *mut Pmc, cb: u32) -> i32;
        fn GetProcessIoCounters(process: *mut c_void, counters: *mut IoCounters) -> i32;
    }

    pub(super) fn sample() -> Option<ProcSample> {
        let (mut c, mut e, mut k, mut u) = (
            FileTime::default(),
            FileTime::default(),
            FileTime::default(),
            FileTime::default(),
        );
        let mut pmc = Pmc {
            cb: std::mem::size_of::<Pmc>() as u32,
            ..Pmc::default()
        };
        let mut io = IoCounters::default();
        // SAFETY: 현재 프로세스의 의사 핸들 + 크기가 맞는 출력 구조체 포인터만 넘긴다.
        unsafe {
            let h = GetCurrentProcess();
            if GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u) == 0 {
                return None;
            }
            let _ = K32GetProcessMemoryInfo(h, &mut pmc, pmc.cb);
            let _ = GetProcessIoCounters(h, &mut io);
        }
        Some(ProcSample {
            cpu_ns: k.ns().saturating_add(u.ns()),
            rss: pmc.working_set_size as u64,
            read: io.read_bytes,
            write: io.write_bytes,
        })
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use std::ffi::c_void;

    /// `rusage_info_v2`(`<sys/resource.h>`).
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

    #[repr(C)]
    #[derive(Default)]
    struct Timebase {
        numer: u32,
        denom: u32,
    }

    extern "C" {
        fn getpid() -> i32;
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut c_void) -> i32;
        fn mach_timebase_info(info: *mut Timebase) -> i32;
    }
    const RUSAGE_INFO_V2: i32 = 2;

    pub(super) fn sample() -> Option<ProcSample> {
        let mut ru = RusageInfoV2::default();
        let mut tb = Timebase::default();
        // SAFETY: 자기 pid + 크기가 맞는 출력 구조체 포인터만 넘긴다.
        let ok = unsafe {
            let _ = mach_timebase_info(&mut tb);
            proc_pid_rusage(
                getpid(),
                RUSAGE_INFO_V2,
                (&mut ru as *mut RusageInfoV2).cast(),
            ) == 0
        };
        if !ok {
            return None;
        }
        // CPU 시간은 mach 절대 시간 단위(Apple Silicon = 125/3 ns) → ns.
        let (n, d) = (u128::from(tb.numer.max(1)), u128::from(tb.denom.max(1)));
        let abs = u128::from(ru.user_time) + u128::from(ru.system_time);
        Some(ProcSample {
            cpu_ns: (abs * n / d) as u64,
            rss: ru.resident_size,
            read: ru.diskio_bytesread,
            write: ru.diskio_byteswritten,
        })
    }
}

#[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
mod imp {
    use super::*;

    pub(super) fn sample() -> Option<ProcSample> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 부하 계산: CPU % = Δcpu / Δt / 코어 · 읽기/쓰기 = Δ바이트/초 · dt 0 · 누적 역행 = 0.
    #[test]
    fn load_between_rates() {
        let a = ProcSample {
            cpu_ns: 1_000_000_000,
            rss: 10,
            read: 1000,
            write: 500,
        };
        let b = ProcSample {
            cpu_ns: 2_000_000_000,
            rss: 4096,
            read: 5000,
            write: 500,
        };
        let l = load_between(a, b, Duration::from_secs(2), 4);
        assert!((l.cpu_pct - 12.5).abs() < 0.01, "{l:?}");
        assert_eq!((l.rss, l.read_bps, l.write_bps), (4096, 2000, 0));
        assert_eq!(load_between(a, b, Duration::ZERO, 4).cpu_pct, 0.0);
        let back = load_between(b, a, Duration::from_secs(1), 0);
        assert_eq!((back.cpu_pct, back.read_bps), (0.0, 0));
        // 한 코어를 꽉 채워도 100을 넘지 않는다.
        let hot = ProcSample {
            cpu_ns: 9_000_000_000,
            ..a
        };
        assert_eq!(
            load_between(a, hot, Duration::from_secs(1), 1).cpu_pct,
            100.0
        );
    }

    /// `/proc/self/stat`: 이름에 공백 · 괄호가 있어도 마지막 `)` 뒤 12 · 13번째가 utime · stime.
    #[test]
    fn proc_stat_and_kv_parsers() {
        let stat = "1234 (my (odd) name) S 1 2 3 4 5 6 7 8 9 10 250 50 0 0 20 0 8 0 100 200 300";
        assert_eq!(parse_proc_stat(stat, 100), Some(3_000_000_000));
        assert_eq!(parse_proc_stat("garbage", 100), None);
        let status = "Name:\tnexa-dir\nVmRSS:\t   51234 kB\nThreads:\t8\n";
        assert_eq!(parse_kv_number(status, "VmRSS"), Some(51234));
        assert_eq!(parse_kv_number(status, "VmSwap"), None);
        let io = "rchar: 123456\nwchar: 789\nread_bytes: 4096\n";
        assert_eq!(parse_kv_number(io, "rchar"), Some(123_456));
        assert_eq!(parse_kv_number(io, "wchar"), Some(789));
    }

    /// 실제 조회(3-OS): 표본이 나오고 메모리가 0이 아니며 CPU 시간이 줄지 않는다.
    #[test]
    fn sample_is_available_on_supported_os() {
        if cfg!(any(target_os = "linux", windows, target_os = "macos")) {
            let a = sample().expect("sample");
            assert!(a.rss > 0, "{a:?}");
            let b = sample().expect("sample");
            assert!(b.cpu_ns >= a.cpu_ns);
        } else {
            assert_eq!(sample(), None);
        }
    }
}
