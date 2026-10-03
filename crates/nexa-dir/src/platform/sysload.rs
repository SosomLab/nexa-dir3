//! **시스템 부하**(상태줄 [CPU][메모리][디스크][네트워크] 칸 · docs/22 NEW-003 · 사용자 10-04 "각각의 정보는 시스템 상태값") —
//! 누적값 표본([`sample`])과 두 표본 사이의 비율 계산([`load_between`] · 순수). 프로세스가 아니라 **PC 전체** 값이다.
//!
//! | | CPU | 메모리 | 디스크 | 네트워크 |
//! |---|---|---|---|---|
//! | Linux | `/proc/stat` | `/proc/meminfo` | `/proc/diskstats`(디스크 단위) | `/proc/net/dev` |
//! | Windows | `GetSystemTimes` | `GlobalMemoryStatusEx` | `IOCTL_DISK_PERFORMANCE`(물리 드라이브) | `GetIfTable` |
//! | macOS | `host_statistics` | `host_statistics64` + `hw.memsize` | (미구현 — IOKit 필요) | `getifaddrs`(AF_LINK) |
//!
//! 조회는 호스트가 주기(`statusbar.load_interval_ms`)마다 부른다 — 스레드 · 타이머 없음. 못 구한 항목은 `None`(칸에 `–`).

use std::time::Duration;

/// 누적 표본(부팅 뒤 누적 · 메모리만 지금 값).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SysSample {
    /// CPU 시간 — 일한 시간 · 전체 시간(같은 단위 · 모든 코어 합).
    pub cpu_busy: u64,
    pub cpu_total: u64,
    /// 메모리 — 쓰는 양 · 전체(바이트).
    pub mem_used: u64,
    pub mem_total: u64,
    /// **이 프로그램**이 쓰는 물리 메모리(바이트 · nexa-sql 상태줄의 메모리 칸과 같은 뜻).
    pub mem_app: Option<u64>,
    /// 디스크 읽기 · 쓰기 누적 바이트(모든 디스크 합).
    pub disk: Option<(u64, u64)>,
    /// 네트워크 받기 · 보내기 누적 바이트(루프백 제외).
    pub net: Option<(u64, u64)>,
}

/// 두 표본 사이의 부하.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct SysLoad {
    /// CPU 사용률(% · 0~100 · 전체 코어).
    pub cpu_pct: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    /// 이 프로그램의 메모리(바이트).
    pub mem_app: Option<u64>,
    /// 디스크 읽기 · 쓰기 바이트/초.
    pub disk_bps: Option<(u64, u64)>,
    /// 네트워크 받기(다운로드) · 보내기(업로드) 바이트/초.
    pub net_bps: Option<(u64, u64)>,
}

impl SysLoad {
    /// 메모리 사용률(% · 전체를 모르면 0).
    pub(crate) fn mem_pct(&self) -> f32 {
        if self.mem_total == 0 {
            0.0
        } else {
            (self.mem_used as f64 / self.mem_total as f64 * 100.0) as f32
        }
    }
}

/// 두 표본 → 부하(순수). 누적값이 줄었으면(카운터 넘침 · 장치 제거) 그 항목은 0.
pub(crate) fn load_between(prev: SysSample, cur: SysSample, dt: Duration) -> SysLoad {
    let secs = dt.as_secs_f64();
    let total = cur.cpu_total.saturating_sub(prev.cpu_total);
    let busy = cur.cpu_busy.saturating_sub(prev.cpu_busy);
    let cpu_pct = if total == 0 {
        0.0
    } else {
        (busy as f64 / total as f64 * 100.0).clamp(0.0, 100.0) as f32
    };
    let rate = |a: u64, b: u64| {
        if secs <= 0.0 {
            0
        } else {
            (b.saturating_sub(a) as f64 / secs).round() as u64
        }
    };
    let pair = |p: Option<(u64, u64)>, c: Option<(u64, u64)>| match (p, c) {
        (Some(p), Some(c)) => Some((rate(p.0, c.0), rate(p.1, c.1))),
        (None, Some(_)) => Some((0, 0)),
        _ => None,
    };
    SysLoad {
        cpu_pct,
        mem_used: cur.mem_used,
        mem_total: cur.mem_total,
        mem_app: cur.mem_app,
        disk_bps: pair(prev.disk, cur.disk),
        net_bps: pair(prev.net, cur.net),
    }
}

/// `/proc/stat` 첫 줄(`cpu  user nice system idle iowait irq softirq steal …`) → `(일한 틱, 전체 틱)`(순수).
/// 전체 = 앞 8칸 합(guest는 user에 이미 들어 있다) · 쉰 시간 = idle + iowait.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn parse_proc_stat_cpu(text: &str) -> Option<(u64, u64)> {
    let line = text.lines().find(|l| l.starts_with("cpu "))?;
    let v: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .take(8)
        .filter_map(|x| x.parse().ok())
        .collect();
    if v.len() < 4 {
        return None;
    }
    let total: u64 = v.iter().sum();
    let idle = v[3] + v.get(4).copied().unwrap_or(0);
    Some((total.saturating_sub(idle), total))
}

/// `키:  값 kB` 꼴 본문에서 그 키의 숫자(순수 · `/proc/meminfo` · `/proc/self/status`).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn parse_meminfo_key(text: &str, key: &str) -> Option<u64> {
    text.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        (k.trim() == key)
            .then(|| v.split_whitespace().next()?.parse().ok())
            .flatten()
    })
}

/// `/proc/meminfo` → `(쓰는 바이트, 전체 바이트)`(순수): 쓰는 양 = MemTotal − MemAvailable(없으면 MemFree).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn parse_meminfo(text: &str) -> Option<(u64, u64)> {
    let kb = |key: &str| parse_meminfo_key(text, key);
    let total = kb("MemTotal")?;
    let avail = kb("MemAvailable").or_else(|| kb("MemFree"))?;
    Some((total.saturating_sub(avail) * 1024, total * 1024))
}

/// `/proc/diskstats` → `(읽은 바이트, 쓴 바이트)`(순수). `is_disk(이름)`이 참인 줄만 더한다(파티션 · loop · dm은 디스크와
/// 겹쳐 세지 않게 호출부가 거른다). 칸 = `major minor name reads merged sectors_read ms writes merged sectors_written …` · 섹터 = 512.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn parse_diskstats(text: &str, is_disk: &dyn Fn(&str) -> bool) -> (u64, u64) {
    let mut sum = (0u64, 0u64);
    for l in text.lines() {
        let f: Vec<&str> = l.split_whitespace().collect();
        if f.len() < 10 || !is_disk(f[2]) {
            continue;
        }
        let (Ok(r), Ok(w)) = (f[5].parse::<u64>(), f[9].parse::<u64>()) else {
            continue;
        };
        sum.0 += r * 512;
        sum.1 += w * 512;
    }
    sum
}

/// 실제 통신에 쓰이는 인터페이스인가(순수) — 루프백 · 컨테이너/가상 다리(같은 바이트를 한 번 더 센다)는 뺀다.
#[cfg_attr(not(unix), allow(dead_code))]
pub(crate) fn is_real_iface(name: &str) -> bool {
    !(name == "lo"
        || name.starts_with("lo0")
        || ["veth", "docker", "br-", "virbr"]
            .iter()
            .any(|p| name.starts_with(p)))
}

/// `/proc/net/dev` → `(받은 바이트, 보낸 바이트)`(순수 · [`is_real_iface`]만): `이름: rx_bytes … (8칸) tx_bytes …`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn parse_net_dev(text: &str) -> (u64, u64) {
    let mut sum = (0u64, 0u64);
    for l in text.lines() {
        let Some((name, rest)) = l.split_once(':') else {
            continue;
        };
        if !is_real_iface(name.trim()) {
            continue;
        }
        let f: Vec<&str> = rest.split_whitespace().collect();
        if f.len() < 9 {
            continue;
        }
        let (Ok(rx), Ok(tx)) = (f[0].parse::<u64>(), f[8].parse::<u64>()) else {
            continue;
        };
        sum.0 += rx;
        sum.1 += tx;
    }
    sum
}

/// 지금 표본(CPU · 메모리를 못 구하면 `None`).
pub(crate) fn sample() -> Option<SysSample> {
    imp::sample()
}

#[cfg(target_os = "linux")]
mod imp {
    use super::*;

    pub(super) fn sample() -> Option<SysSample> {
        let stat = std::fs::read_to_string("/proc/stat").ok()?;
        let (cpu_busy, cpu_total) = parse_proc_stat_cpu(&stat)?;
        let (mem_used, mem_total) = parse_meminfo(&std::fs::read_to_string("/proc/meminfo").ok()?)?;
        // 디스크 = `/sys/block/<이름>`이 있는 것(파티션 제외) 중 실제 장치(loop · ram · zram · dm은 겹치거나 가상).
        let is_disk = |name: &str| {
            !["loop", "ram", "zram", "dm-", "md"]
                .iter()
                .any(|p| name.starts_with(p))
                && std::path::Path::new("/sys/block").join(name).exists()
        };
        let disk = std::fs::read_to_string("/proc/diskstats")
            .ok()
            .map(|t| parse_diskstats(&t, &is_disk));
        let net = std::fs::read_to_string("/proc/net/dev")
            .ok()
            .map(|t| parse_net_dev(&t));
        // 이 프로그램의 메모리 = `/proc/self/status`의 VmRSS(kB).
        let mem_app = std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|t| parse_meminfo_key(&t, "VmRSS"))
            .map(|kb| kb * 1024);
        Some(SysSample {
            cpu_busy,
            cpu_total,
            mem_used,
            mem_total,
            mem_app,
            disk,
            net,
        })
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    impl FileTime {
        fn ticks(self) -> u64 {
            (u64::from(self.high) << 32) | u64::from(self.low)
        }
    }

    #[repr(C)]
    #[derive(Default)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page_file: u64,
        avail_page_file: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended_virtual: u64,
    }

    /// `DISK_PERFORMANCE`(winioctl.h).
    #[repr(C)]
    struct DiskPerformance {
        bytes_read: i64,
        bytes_written: i64,
        read_time: i64,
        write_time: i64,
        idle_time: i64,
        read_count: u32,
        write_count: u32,
        queue_depth: u32,
        split_count: u32,
        query_time: i64,
        storage_device_number: u32,
        storage_manager_name: [u16; 8],
    }

    /// `PROCESS_MEMORY_COUNTERS`(psapi.h).
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

    const IOCTL_DISK_PERFORMANCE: u32 = 0x0007_0020;
    const FILE_SHARE_READ_WRITE: u32 = 0x1 | 0x2;
    const OPEN_EXISTING: u32 = 3;
    const INVALID_HANDLE: isize = -1;
    /// `MIB_IFROW` 한 줄 크기 · 그 안의 칸 자리(iprtrmib.h — wszName 512 · 5×u32 · bPhysAddr 8 · 3×u32 = 552).
    const IFROW_SIZE: usize = 860;
    const IFROW_TYPE: usize = 516;
    const IFROW_IN_OCTETS: usize = 552;
    const IFROW_OUT_OCTETS: usize = 576;
    const IF_TYPE_LOOPBACK: u32 = 24;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemTimes(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
        fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
        fn GetCurrentProcess() -> *mut c_void;
        fn K32GetProcessMemoryInfo(process: *mut c_void, counters: *mut Pmc, cb: u32) -> i32;
        fn CreateFileW(
            name: *const u16,
            access: u32,
            share: u32,
            security: *const c_void,
            disposition: u32,
            flags: u32,
            template: *const c_void,
        ) -> isize;
        fn DeviceIoControl(
            device: isize,
            code: u32,
            input: *const c_void,
            input_len: u32,
            output: *mut c_void,
            output_len: u32,
            returned: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
        fn CloseHandle(handle: isize) -> i32;
    }

    #[link(name = "iphlpapi")]
    extern "system" {
        fn GetIfTable(table: *mut c_void, size: *mut u32, order: i32) -> u32;
    }

    /// 물리 드라이브 0~15의 누적 읽기/쓰기 바이트 합(하나도 못 열면 `None`).
    fn disks() -> Option<(u64, u64)> {
        let mut sum = (0u64, 0u64);
        let mut any = false;
        let mut misses = 0;
        for n in 0..16 {
            let name: Vec<u16> = format!("\\\\.\\PhysicalDrive{n}\0")
                .encode_utf16()
                .collect();
            // SAFETY: NUL로 끝나는 경로 · 접근 권한 0(조회만) · 받은 핸들은 바로 닫는다.
            let h = unsafe {
                CreateFileW(
                    name.as_ptr(),
                    0,
                    FILE_SHARE_READ_WRITE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    0,
                    std::ptr::null(),
                )
            };
            if h == INVALID_HANDLE || h == 0 {
                misses += 1;
                if misses >= 2 {
                    break;
                }
                continue;
            }
            misses = 0;
            // SAFETY: 0으로 채운 출력 구조체 + 그 크기 · 동기 호출.
            unsafe {
                let mut perf: DiskPerformance = std::mem::zeroed();
                let mut ret = 0u32;
                if DeviceIoControl(
                    h,
                    IOCTL_DISK_PERFORMANCE,
                    std::ptr::null(),
                    0,
                    (&mut perf as *mut DiskPerformance).cast(),
                    std::mem::size_of::<DiskPerformance>() as u32,
                    &mut ret,
                    std::ptr::null_mut(),
                ) != 0
                {
                    sum.0 += perf.bytes_read.max(0) as u64;
                    sum.1 += perf.bytes_written.max(0) as u64;
                    any = true;
                }
                CloseHandle(h);
            }
        }
        any.then_some(sum)
    }

    /// 인터페이스 표의 받은/보낸 바이트 합(루프백 제외 · 필터 드라이버가 같은 값을 여러 줄로 내므로 같은 쌍은 한 번만).
    fn net() -> Option<(u64, u64)> {
        let mut size = 0u32;
        // SAFETY: 첫 호출 = 필요한 크기만 묻는다(버퍼 없음).
        unsafe { GetIfTable(std::ptr::null_mut(), &mut size, 0) };
        if size < 4 {
            return None;
        }
        // u32 정렬 버퍼(표의 첫 칸이 u32 개수).
        let mut buf = vec![0u32; (size as usize).div_ceil(4) + 1];
        // SAFETY: 방금 물은 크기 이상의 버퍼.
        if unsafe { GetIfTable(buf.as_mut_ptr().cast(), &mut size, 0) } != 0 {
            return None;
        }
        let count = buf[0] as usize;
        // SAFETY: u32 버퍼를 바이트로 본다(길이 = 원소 수 × 4).
        let bytes: &[u8] =
            unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), buf.len() * 4) };
        let u32_at = |off: usize| -> Option<u32> {
            bytes
                .get(off..off + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        let mut seen: Vec<(u32, u32)> = Vec::new();
        let mut sum = (0u64, 0u64);
        for i in 0..count {
            let row = 4 + i * IFROW_SIZE;
            let (Some(ty), Some(rx), Some(tx)) = (
                u32_at(row + IFROW_TYPE),
                u32_at(row + IFROW_IN_OCTETS),
                u32_at(row + IFROW_OUT_OCTETS),
            ) else {
                break;
            };
            if ty == IF_TYPE_LOOPBACK || (rx == 0 && tx == 0) || seen.contains(&(rx, tx)) {
                continue;
            }
            seen.push((rx, tx));
            sum.0 += u64::from(rx);
            sum.1 += u64::from(tx);
        }
        Some(sum)
    }

    pub(super) fn sample() -> Option<SysSample> {
        let (mut idle, mut kernel, mut user) = (
            FileTime::default(),
            FileTime::default(),
            FileTime::default(),
        );
        let mut mem = MemoryStatusEx {
            length: std::mem::size_of::<MemoryStatusEx>() as u32,
            ..MemoryStatusEx::default()
        };
        // SAFETY: 크기가 맞는 출력 구조체 포인터만 넘긴다.
        unsafe {
            if GetSystemTimes(&mut idle, &mut kernel, &mut user) == 0 {
                return None;
            }
            if GlobalMemoryStatusEx(&mut mem) == 0 {
                return None;
            }
        }
        let mut pmc = Pmc {
            cb: std::mem::size_of::<Pmc>() as u32,
            ..Pmc::default()
        };
        // SAFETY: 현재 프로세스의 의사 핸들 + 크기가 맞는 출력 구조체.
        let mem_app = unsafe {
            (K32GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, pmc.cb) != 0)
                .then_some(pmc.working_set_size as u64)
        };
        // 커널 시간에는 쉰 시간이 들어 있다.
        let total = kernel.ticks().saturating_add(user.ticks());
        Some(SysSample {
            cpu_busy: total.saturating_sub(idle.ticks()),
            cpu_total: total,
            mem_used: mem.total_phys.saturating_sub(mem.avail_phys),
            mem_total: mem.total_phys,
            mem_app,
            disk: disks(),
            net: net(),
        })
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use std::ffi::{c_char, c_void, CStr};

    /// `vm_statistics64`(mach/vm_statistics.h) — 152바이트 = integer_t 38개.
    #[repr(C)]
    #[derive(Default)]
    struct VmStatistics64 {
        free_count: u32,
        active_count: u32,
        inactive_count: u32,
        wire_count: u32,
        zero_fill_count: u64,
        reactivations: u64,
        pageins: u64,
        pageouts: u64,
        faults: u64,
        cow_faults: u64,
        lookups: u64,
        hits: u64,
        purges: u64,
        purgeable_count: u32,
        speculative_count: u32,
        decompressions: u64,
        compressions: u64,
        swapins: u64,
        swapouts: u64,
        compressor_page_count: u32,
        throttled_count: u32,
        external_page_count: u32,
        internal_page_count: u32,
        total_uncompressed_pages_in_compressor: u64,
    }

    #[repr(C)]
    struct IfAddrs {
        next: *mut IfAddrs,
        name: *const c_char,
        flags: u32,
        addr: *const SockAddr,
        netmask: *const c_void,
        dstaddr: *const c_void,
        data: *const c_void,
    }

    #[repr(C)]
    struct SockAddr {
        len: u8,
        family: u8,
    }

    /// `rusage_info_v2`(sys/resource.h) — resident_size만 쓴다.
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

    const HOST_CPU_LOAD_INFO: i32 = 3;
    const HOST_VM_INFO64: i32 = 4;
    const AF_LINK: u8 = 18;
    /// `if_data`의 ifi_ibytes · ifi_obytes 자리(net/if_var.h — u8 8개 + u32 8개 뒤).
    const IFDATA_IBYTES: usize = 40;
    const IFDATA_OBYTES: usize = 44;
    /// `_SC_PAGESIZE`(macOS).
    const SC_PAGESIZE: i32 = 29;

    extern "C" {
        fn mach_host_self() -> u32;
        fn host_statistics(host: u32, flavor: i32, info: *mut u32, count: *mut u32) -> i32;
        fn host_statistics64(host: u32, flavor: i32, info: *mut u32, count: *mut u32) -> i32;
        fn sysctlbyname(
            name: *const c_char,
            oldp: *mut c_void,
            oldlenp: *mut usize,
            newp: *mut c_void,
            newlen: usize,
        ) -> i32;
        fn sysconf(name: i32) -> i64;
        fn getpid() -> i32;
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut c_void) -> i32;
        fn getifaddrs(out: *mut *mut IfAddrs) -> i32;
        fn freeifaddrs(list: *mut IfAddrs);
    }

    fn net() -> Option<(u64, u64)> {
        let mut list: *mut IfAddrs = std::ptr::null_mut();
        // SAFETY: getifaddrs가 준 목록을 읽기만 하고 freeifaddrs로 돌려준다 · 포인터는 쓰기 전에 널 검사.
        unsafe {
            if getifaddrs(&mut list) != 0 {
                return None;
            }
            let mut sum = (0u64, 0u64);
            let mut cur = list;
            while !cur.is_null() {
                let a = &*cur;
                cur = a.next;
                if a.addr.is_null() || a.data.is_null() || a.name.is_null() {
                    continue;
                }
                if (*a.addr).family != AF_LINK {
                    continue;
                }
                let name = CStr::from_ptr(a.name).to_string_lossy();
                if !is_real_iface(&name) {
                    continue;
                }
                let d = a.data.cast::<u8>();
                let rx = std::ptr::read_unaligned(d.add(IFDATA_IBYTES).cast::<u32>());
                let tx = std::ptr::read_unaligned(d.add(IFDATA_OBYTES).cast::<u32>());
                sum.0 += u64::from(rx);
                sum.1 += u64::from(tx);
            }
            freeifaddrs(list);
            Some(sum)
        }
    }

    pub(super) fn sample() -> Option<SysSample> {
        // SAFETY: 크기(정수 개수)가 맞는 출력 버퍼만 넘긴다.
        unsafe {
            let host = mach_host_self();
            // user · system · idle · nice 틱.
            let mut cpu = [0u32; 4];
            let mut count = 4u32;
            if host_statistics(host, HOST_CPU_LOAD_INFO, cpu.as_mut_ptr(), &mut count) != 0 {
                return None;
            }
            let total: u64 = cpu.iter().map(|v| u64::from(*v)).sum();
            let busy = total.saturating_sub(u64::from(cpu[2]));
            let mut vm = VmStatistics64::default();
            let mut count = (std::mem::size_of::<VmStatistics64>() / 4) as u32;
            if host_statistics64(
                host,
                HOST_VM_INFO64,
                (&mut vm as *mut VmStatistics64).cast(),
                &mut count,
            ) != 0
            {
                return None;
            }
            let mut mem_total = 0u64;
            let mut len = std::mem::size_of::<u64>();
            if sysctlbyname(
                c"hw.memsize".as_ptr(),
                (&mut mem_total as *mut u64).cast(),
                &mut len,
                std::ptr::null_mut(),
                0,
            ) != 0
            {
                return None;
            }
            let page = sysconf(SC_PAGESIZE).max(4096) as u64;
            // 쓰는 양 = 활성 + 고정 + 압축(활성 상태 보기의 "사용한 메모리"와 같은 뜻).
            let used = (u64::from(vm.active_count)
                + u64::from(vm.wire_count)
                + u64::from(vm.compressor_page_count))
                * page;
            let mut ru = RusageInfoV2::default();
            let mem_app = (proc_pid_rusage(
                getpid(),
                RUSAGE_INFO_V2,
                (&mut ru as *mut RusageInfoV2).cast(),
            ) == 0)
                .then_some(ru.resident_size);
            Some(SysSample {
                cpu_busy: busy,
                cpu_total: total,
                mem_used: used.min(mem_total),
                mem_total,
                mem_app,
                disk: None, // 디스크 누적 바이트는 IOKit이 필요하다 — 후속
                net: net(),
            })
        }
    }
}

#[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
mod imp {
    use super::*;

    pub(super) fn sample() -> Option<SysSample> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 부하 계산: CPU % = Δ일한 시간 / Δ전체 · 속도 = Δ바이트/초 · 역행 = 0 · 없는 항목 = None · 처음 생긴 항목 = 0.
    #[test]
    fn load_between_rates() {
        let a = SysSample {
            cpu_busy: 100,
            cpu_total: 1000,
            mem_used: 1,
            mem_total: 8,
            mem_app: None,
            disk: Some((1000, 500)),
            net: None,
        };
        let b = SysSample {
            cpu_busy: 150,
            cpu_total: 1200,
            mem_used: 2,
            mem_total: 8,
            mem_app: Some(77),
            disk: Some((5000, 500)),
            net: Some((10, 10)),
        };
        let l = load_between(a, b, Duration::from_secs(2));
        assert!((l.cpu_pct - 25.0).abs() < 0.01, "{l:?}");
        assert_eq!((l.mem_used, l.mem_total, l.mem_app), (2, 8, Some(77)));
        assert!((l.mem_pct() - 25.0).abs() < 0.01);
        assert_eq!(l.disk_bps, Some((2000, 0)));
        assert_eq!(l.net_bps, Some((0, 0)), "직전에 없던 항목 = 0부터");
        let back = load_between(b, a, Duration::from_secs(1));
        assert_eq!(
            (back.cpu_pct, back.disk_bps, back.net_bps),
            (0.0, Some((0, 0)), None)
        );
        assert_eq!(load_between(a, b, Duration::ZERO).disk_bps, Some((0, 0)));
        assert_eq!(SysLoad::default().mem_pct(), 0.0);
    }

    /// Linux 원문 파서(고정물): CPU 줄 · meminfo · diskstats(디스크만) · net/dev(루프백 · 가상 다리 제외).
    #[test]
    fn linux_text_parsers() {
        let stat = "cpu  100 10 50 800 40 0 0 0 0 0\ncpu0 1 2 3 4 5 6 7 8 9 10\nintr 5\n";
        assert_eq!(parse_proc_stat_cpu(stat), Some((160, 1000)));
        assert_eq!(parse_proc_stat_cpu("nope"), None);
        let mem = "MemTotal:       16000 kB\nMemFree:         1000 kB\nMemAvailable:   12000 kB\n";
        assert_eq!(parse_meminfo(mem), Some((4000 * 1024, 16000 * 1024)));
        assert_eq!(
            parse_meminfo("MemTotal: 100 kB\nMemFree: 30 kB\n"),
            Some((70 * 1024, 100 * 1024)),
            "MemAvailable 없음 = MemFree"
        );
        assert_eq!(parse_meminfo("MemFree: 1 kB\n"), None);
        let disks = "   8       0 sda 10 0 2000 5 20 0 4000 9 0 1 1\n   8       1 sda1 9 0 1900 5 19 0 3900 9 0 1 1\n   7       0 loop0 14 0 34 1 0 0 0 0 0 1 1\n 259       0 nvme0n1 1 0 100 1 1 0 300 1 0 1 1\n";
        let only = |n: &str| n == "sda" || n == "nvme0n1";
        assert_eq!(parse_diskstats(disks, &only), (2100 * 512, 4300 * 512));
        assert_eq!(parse_diskstats("garbage\n", &only), (0, 0));
        let net = "Inter-|   Receive  |  Transmit\n face |bytes ...\n    lo:  800 8 0 0 0 0 0 0  800 8 0 0 0 0 0 0\n ens33: 5000 12 0 0 0 0 0 0 7000 21 0 0 0 0 0 0\ndocker0: 1 1 0 0 0 0 0 0 2 1 0 0 0 0 0 0\n wlan0: 100 1 0 0 0 0 0 0 200 1 0 0 0 0 0 0\n";
        assert_eq!(parse_net_dev(net), (5100, 7200));
        assert!(is_real_iface("en0") && is_real_iface("eth0"));
        assert!(!is_real_iface("lo") && !is_real_iface("lo0") && !is_real_iface("veth12ab"));
    }

    /// 실제 조회(3-OS): 표본이 나오고 메모리 전체 > 0 · 쓰는 양 ≤ 전체 · CPU 누적이 줄지 않는다.
    #[test]
    fn sample_is_available_on_supported_os() {
        if cfg!(any(target_os = "linux", windows, target_os = "macos")) {
            let a = sample().expect("sample");
            assert!(a.mem_total > 0 && a.mem_used <= a.mem_total, "{a:?}");
            assert!(a.cpu_total > 0 && a.cpu_busy <= a.cpu_total, "{a:?}");
            assert!(
                a.mem_app.is_some_and(|m| m > 0 && m <= a.mem_total),
                "{a:?}"
            );
            let b = sample().expect("sample");
            assert!(b.cpu_total >= a.cpu_total);
            if cfg!(target_os = "linux") {
                assert!(a.net.is_some() && a.disk.is_some(), "{a:?}");
            }
        } else {
            assert_eq!(sample(), None);
        }
    }
}
