//! 이 프로세스의 메모리(운영체제가 보는 값) · 힙 정리 — 메모리 창(T-93 · docs/22 NEW-002)의 OS 쪽. nexa-sql `memstat.rs`(os 모듈) ·
//! `memtrim.rs` 이식(외부 crate 0 · 수동 extern).
//!
//! - [`sys`]: 전용(풋프린트) · 상주 · 익명 · 파일 매핑 · 압축 · 힙 사용/여유. 모르는 칸 = 0(다른 OS = 전부 0).
//! - [`trim`]: 할당자가 들고 있는 빈 조각을 운영체제에 돌려준다(Windows `HeapCompact` · glibc `malloc_trim` · macOS
//!   `malloc_zone_pressure_relief`). 워킹셋 트림은 하지 않는다 — 숫자만 작아지고 돌아올 때 페이지 폴트로 느려진다.

/// OS가 말하는 프로세스 메모리(모르는 칸 = 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SysMem {
    /// 작업 관리자의 "메모리"에 해당하는 값(Windows Private Bytes · macOS phys_footprint · Linux resident − shared).
    pub footprint: u64,
    /// 상주(워킹 셋).
    pub resident: u64,
    /// 익명 페이지(힙 · 스택).
    pub anon: u64,
    /// 파일 매핑(실행 파일 · 라이브러리 · 매핑된 글꼴).
    pub file_backed: u64,
    /// 압축돼 있는 몫(macOS).
    pub compressed: u64,
    /// 할당자가 **쓰고 있는** 바이트.
    pub heap_used: u64,
    /// 할당자가 **들고 있지만 안 쓰는** 바이트([`trim`]이 돌려줄 수 있는 몫의 상한).
    pub heap_held: u64,
}

/// 지금 값(≈ µs · 시스템 호출 한두 번).
pub(crate) fn sys() -> SysMem {
    imp::sys()
}

/// 힙을 정리해 운영체제에 돌려준다. 돌려주는 값 = 걸린 시간(µs).
pub(crate) fn trim() -> u128 {
    let t = std::time::Instant::now();
    imp::trim();
    t.elapsed().as_micros()
}

#[cfg(windows)]
mod imp {
    use super::SysMem;
    use std::ffi::c_void;

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
        private_usage: usize,
    }
    #[repr(C)]
    #[derive(Default)]
    struct HeapSummaryT {
        cb: u32,
        cb_allocated: usize,
        cb_committed: usize,
        cb_reserved: usize,
        cb_max_reserve: usize,
    }
    #[repr(C)]
    struct HeapOptimizeResourcesInformation {
        version: u32,
        flags: u32,
    }
    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn GetProcessHeap() -> *mut c_void;
        fn GetProcessHeaps(count: u32, heaps: *mut *mut c_void) -> u32;
        fn HeapCompact(heap: *mut c_void, flags: u32) -> usize;
        fn HeapSetInformation(
            heap: *mut c_void,
            class: u32,
            info: *const c_void,
            len: usize,
        ) -> i32;
        fn K32GetProcessMemoryInfo(process: *mut c_void, counters: *mut Pmc, cb: u32) -> i32;
        fn HeapSummary(heap: *mut c_void, flags: u32, summary: *mut HeapSummaryT) -> i32;
    }
    /// `HeapOptimizeResources` 정보 클래스(Windows 8.1+ · 실패해도 무해).
    const HEAP_OPTIMIZE_RESOURCES: u32 = 3;

    pub(super) fn sys() -> SysMem {
        let mut pmc = Pmc {
            cb: std::mem::size_of::<Pmc>() as u32,
            ..Default::default()
        };
        let mut hs = HeapSummaryT {
            cb: std::mem::size_of::<HeapSummaryT>() as u32,
            ..Default::default()
        };
        // SAFETY: 구조체 크기를 `cb`로 알린다 · 현재 프로세스의 의사 핸들과 프로세스 힙 핸들은 늘 유효.
        let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, pmc.cb) } != 0;
        // SAFETY: 위와 같다(출력 구조체는 호출 동안 살아 있다).
        let hok = unsafe { HeapSummary(GetProcessHeap(), 0, &mut hs) } != 0;
        let private = if ok { pmc.private_usage as u64 } else { 0 };
        let ws = if ok { pmc.working_set_size as u64 } else { 0 };
        SysMem {
            footprint: private,
            resident: ws,
            anon: private,
            file_backed: ws.saturating_sub(private),
            compressed: 0,
            heap_used: if hok { hs.cb_allocated as u64 } else { 0 },
            heap_held: if hok {
                (hs.cb_committed as u64).saturating_sub(hs.cb_allocated as u64)
            } else {
                0
            },
        }
    }

    pub(super) fn trim() {
        // SAFETY: 문서화된 Win32 호출 — 힙 핸들은 이 프로세스의 것이고, 정보 구조체는 호출 동안 살아 있다.
        //   실패(옛 Windows · 잠긴 힙)는 반환값으로만 알려지고 부작용이 없다.
        unsafe {
            let info = HeapOptimizeResourcesInformation {
                version: 1,
                flags: 0,
            };
            // 힙 핸들을 null로 주면 프로세스의 모든 힙에 적용된다.
            HeapSetInformation(
                std::ptr::null_mut(),
                HEAP_OPTIMIZE_RESOURCES,
                std::ptr::addr_of!(info).cast(),
                std::mem::size_of::<HeapOptimizeResourcesInformation>(),
            );
            let mut heaps: [*mut c_void; 64] = [std::ptr::null_mut(); 64];
            let n = GetProcessHeaps(64, heaps.as_mut_ptr()) as usize;
            if n == 0 || n > heaps.len() {
                HeapCompact(GetProcessHeap(), 0);
            } else {
                for h in &heaps[..n] {
                    HeapCompact(*h, 0);
                }
            }
        }
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
mod imp {
    use super::SysMem;

    #[repr(C)]
    #[derive(Default)]
    struct MallInfo2 {
        arena: usize,
        ordblks: usize,
        smblks: usize,
        hblks: usize,
        hblkhd: usize,
        usmblks: usize,
        fsmblks: usize,
        uordblks: usize,
        fordblks: usize,
        keepcost: usize,
    }
    extern "C" {
        fn mallinfo2() -> MallInfo2;
        fn malloc_trim(pad: usize) -> i32;
    }

    pub(super) fn sys() -> SysMem {
        let page = 4096u64;
        // /proc/self/statm: size resident shared …(페이지 수).
        let (res, shared) = std::fs::read_to_string("/proc/self/statm")
            .ok()
            .and_then(|s| {
                let mut it = s.split_whitespace().filter_map(|x| x.parse::<u64>().ok());
                let (_size, res, shared) = (it.next()?, it.next()?, it.next()?);
                Some((res * page, shared * page))
            })
            .unwrap_or((0, 0));
        // SAFETY: 인자 없는 glibc 2.33+ 통계 호출.
        let mi = unsafe { mallinfo2() };
        SysMem {
            footprint: res.saturating_sub(shared),
            resident: res,
            anon: res.saturating_sub(shared),
            file_backed: shared,
            compressed: 0,
            heap_used: (mi.uordblks + mi.hblkhd) as u64,
            heap_held: mi.fordblks as u64,
        }
    }

    pub(super) fn trim() {
        // SAFETY: glibc의 정리 호출(인자 = 남길 여유 0).
        unsafe {
            malloc_trim(0);
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::SysMem;
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Default)]
    struct MStats {
        bytes_total: usize,
        chunks_used: usize,
        bytes_used: usize,
        chunks_free: usize,
        bytes_free: usize,
    }
    extern "C" {
        /// `mach_task_self()` 매크로의 실체.
        static mach_task_self_: u32;
        fn task_info(task: u32, flavor: u32, info: *mut u64, count: *mut u32) -> i32;
        fn mstats() -> MStats;
        fn malloc_zone_pressure_relief(zone: *mut c_void, goal: usize) -> usize;
    }
    const TASK_VM_INFO: u32 = 22;
    /// rev1 = `phys_footprint`까지(8바이트 19칸). 칸: 2 resident · 6 internal · 8 external · 15 compressed · 18 phys_footprint.
    const WORDS: usize = 19;

    pub(super) fn sys() -> SysMem {
        let mut info = [0u64; WORDS];
        let mut count = (WORDS * 2) as u32;
        // SAFETY: 버퍼 길이를 natural_t 단위로 알리고 커널은 그만큼만 채운다 · 자기 태스크 포트는 늘 유효.
        let kr = unsafe { task_info(mach_task_self_, TASK_VM_INFO, info.as_mut_ptr(), &mut count) };
        // SAFETY: 인자 없는 통계 호출.
        let ms = unsafe { mstats() };
        let ok = kr == 0 && (count as usize) >= WORDS * 2;
        SysMem {
            footprint: if ok { info[18] } else { 0 },
            resident: if ok { info[2] } else { 0 },
            anon: if ok { info[6] } else { 0 },
            file_backed: if ok { info[8] } else { 0 },
            compressed: if ok { info[15] } else { 0 },
            heap_used: ms.bytes_used as u64,
            heap_held: ms.bytes_free as u64,
        }
    }

    pub(super) fn trim() {
        // SAFETY: zone = NULL(모든 영역) · goal = 0(가능한 만큼).
        unsafe {
            malloc_zone_pressure_relief(std::ptr::null_mut(), 0);
        }
    }
}

#[cfg(not(any(
    windows,
    all(target_os = "linux", target_env = "gnu"),
    target_os = "macos"
)))]
mod imp {
    use super::SysMem;

    pub(super) fn sys() -> SysMem {
        SysMem::default()
    }

    pub(super) fn trim() {}
}

#[cfg(test)]
mod tests {
    /// 큰 조각 다수를 놓은 뒤 정리 — 어느 OS든 실패 없이 돌고, 값을 읽을 수 있는 OS에서는 터무니없지 않은 값이 나온다.
    #[test]
    fn sys_reads_and_trim_runs() {
        let junk: Vec<String> = (0..50_000).map(|i| format!("cell value {i:08}")).collect();
        drop(junk);
        let _us = super::trim();
        let s = super::sys();
        if cfg!(any(
            windows,
            target_os = "macos",
            all(target_os = "linux", target_env = "gnu")
        )) {
            assert!(s.footprint > 0 && s.resident > 0, "{s:?}");
            // 칸을 잘못 읽지 않았는지 — 시험 프로세스는 1 MB ~ 8 GB 사이.
            assert!((1 << 20..8u64 << 30).contains(&s.resident), "{s:?}");
            assert!(s.heap_used > 0, "{s:?}");
        } else {
            assert_eq!(s, super::SysMem::default());
        }
    }
}
