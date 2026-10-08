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
    /// ★ **전용 워킹 셋**(사용자 10-07 "작업 관리자 메모리와 전용 메모리가 다르다" · nexa-sql 10-07 bin63 계승) = 지금 RAM에 있는
    /// 페이지 중 공유 아닌 것 — Windows 작업 관리자 "메모리(개인 작업 집합)"의 정의. [`footprint`](커밋 · Private Bytes)와 **축이
    /// 달라** 두 숫자가 다른 것이 정상(유휴 트림 뒤에는 상주만 내려가 차이가 더 벌어진다). [`sys`]는 0으로 두고 [`private_ws`]로
    /// 따로 센다(페이지 수 비례 비용 · 창이 열려 있을 때의 전체 표본에서만). macOS = 활성 상태 보기 "메모리"가 풋프린트라 0(행 숨김).
    pub private_ws: u64,
}

/// 지금 값(≈ µs · 시스템 호출 한두 번). `private_ws`는 0 — [`private_ws`]로 따로.
pub(crate) fn sys() -> SysMem {
    imp::sys()
}

/// 전용 워킹 셋(작업 관리자 "메모리" 축 · [`SysMem::private_ws`]) — Windows `K32QueryWorkingSet`(Shared 비트 없는 페이지 ×
/// 페이지 크기) · Linux `/proc/self/smaps_rollup`(Private_Clean + Private_Dirty) · 그 밖 = 0. 비용 = 상주 페이지 수 비례(300 MB ≈
/// 77k 항목 · 수백 µs)라 전체 표본에서만 부른다.
pub(crate) fn private_ws(s: &SysMem) -> u64 {
    imp::private_ws(s)
}

/// 주어진 메모리 범위들 가운데 **지금 상주하는** 바이트(파일 매핑 — 글꼴 — 의 실제 점유 · 사용자 10-06 "글꼴 파일이 Private에
/// 드는가"). 페이지 단위로 센다 · 잴 수 없는 OS = `None`(호출자는 매핑 크기로 대신).
pub(crate) fn resident_bytes(ranges: &[&[u8]]) -> Option<u64> {
    imp::resident_bytes(ranges)
}

/// "런타임 · 라이브러리 · 미집계"의 **분해**(사용자 10-08 "메모리 사용량이 너무 많다 — 전체 점검": 메모리 창의 미집계 49.8 MB가 무엇인지
/// 보이도록) — 전용(커밋) 가운데 우리 영역 밖의 큰 덩어리 셋 + 모듈/스레드 수. 모르는 OS = 전부 0(행 숨김).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RtBreak {
    /// 라이브러리(DLL) 이미지의 **쓰기 가능 페이지**(.data · 재배치 · 복사-쓰기) — 모듈이 많이 올라오면(셸 확장) 커진다.
    pub image_private: u64,
    /// 기본 프로세스 힙 **밖의** 힙들(라이브러리가 만든 힙 · CRT 힙)의 커밋 합 — "힙 사용 중"에는 안 잡힌다.
    pub heaps_other: u64,
    /// 스레드 스택 커밋 합(가드 페이지가 있는 전용 영역).
    pub stacks: u64,
    /// 올라와 있는 모듈(exe + DLL) 수.
    pub modules: u32,
    /// 스레드 수(스택 영역 수).
    pub threads: u32,
}

impl RtBreak {
    /// 분해가 가능한 OS였는가(모듈 수 0 = 모름 · 행 숨김).
    pub(crate) fn known(&self) -> bool {
        self.modules > 0
    }
    /// 세 덩어리 합.
    pub(crate) fn sum(&self) -> u64 {
        self.image_private
            .saturating_add(self.heaps_other)
            .saturating_add(self.stacks)
    }
}

/// 미집계 분해([`RtBreak`]) — Windows = `VirtualQuery` 주소 공간 훑기(≈ 수천 영역 · ms 미만) + `GetProcessHeaps`/`HeapSummary` ·
/// 그 밖 OS = 0. 비용이 있으므로 메모리 창이 열려 있을 때의 전체 표본에서만 부른다.
pub(crate) fn runtime_breakdown() -> RtBreak {
    imp::runtime_breakdown()
}

/// 힙을 정리해 운영체제에 돌려준다. 돌려주는 값 = 걸린 시간(µs).
pub(crate) fn trim() -> u128 {
    let t = std::time::Instant::now();
    imp::trim();
    t.elapsed().as_micros()
}

#[cfg(windows)]
mod imp {
    use super::{RtBreak, SysMem};
    use std::ffi::c_void;

    /// `MEMORY_BASIC_INFORMATION`.
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct Mbi {
        base: usize,
        alloc_base: usize,
        alloc_protect: u32,
        partition_id: u16,
        region_size: usize,
        state: u32,
        protect: u32,
        kind: u32,
    }
    const MEM_COMMIT: u32 = 0x1000;
    const MEM_PRIVATE: u32 = 0x20000;
    const MEM_IMAGE: u32 = 0x100_0000;
    const PAGE_GUARD: u32 = 0x100;
    /// 쓰기 가능 보호(READWRITE · WRITECOPY · EXECUTE_READWRITE · EXECUTE_WRITECOPY) = 커밋 과금 대상.
    const PAGE_WRITABLE_MASK: u32 = 0x04 | 0x08 | 0x40 | 0x80;
    /// 사용자 주소 공간 상한(x64 · 128 TB).
    const USER_SPACE_END: usize = 0x7FFF_FFFF_FFFF;

    extern "system" {
        fn VirtualQuery(addr: *const c_void, mbi: *mut Mbi, len: usize) -> usize;
    }

    pub(super) fn runtime_breakdown() -> RtBreak {
        let mut rt = RtBreak::default();
        let mut addr = 0usize;
        let mut mbi = Mbi::default();
        // 전용 영역을 할당 기준(AllocationBase)으로 묶어 가드 페이지가 있는 묶음 = 스레드 스택.
        let mut privs: Vec<(usize, u64, bool)> = Vec::new();
        let mut guard = 0u32;
        while addr < USER_SPACE_END {
            // SAFETY: 출력 구조체와 그 크기를 넘기는 문서화된 호출 — 자기 프로세스 주소 공간 조회.
            let n = unsafe {
                VirtualQuery(addr as *const c_void, &mut mbi, std::mem::size_of::<Mbi>())
            };
            if n == 0 || mbi.region_size == 0 {
                break;
            }
            if mbi.state == MEM_COMMIT {
                match mbi.kind {
                    MEM_IMAGE => {
                        if mbi.base == mbi.alloc_base {
                            rt.modules += 1;
                        }
                        if mbi.protect & PAGE_WRITABLE_MASK != 0 {
                            rt.image_private += mbi.region_size as u64;
                        }
                    }
                    MEM_PRIVATE => {
                        let is_guard = mbi.protect & PAGE_GUARD != 0;
                        match privs.last_mut() {
                            Some((b, sz, g)) if *b == mbi.alloc_base => {
                                *sz += mbi.region_size as u64;
                                *g |= is_guard;
                            }
                            _ => privs.push((mbi.alloc_base, mbi.region_size as u64, is_guard)),
                        }
                        if is_guard {
                            guard += 1;
                        }
                    }
                    _ => {}
                }
            }
            addr = match mbi.base.checked_add(mbi.region_size) {
                Some(a) => a,
                None => break,
            };
        }
        for (_, sz, g) in &privs {
            if *g {
                rt.stacks += *sz;
            }
        }
        rt.threads = guard;
        // 기본 힙 밖의 힙들.
        // SAFETY: 자기 프로세스의 힙 핸들 목록 · 요약 구조체는 호출 동안 살아 있다.
        unsafe {
            let mut heaps: [*mut c_void; 64] = [std::ptr::null_mut(); 64];
            let n = GetProcessHeaps(64, heaps.as_mut_ptr()) as usize;
            let main = GetProcessHeap();
            for h in heaps.iter().take(n.min(heaps.len())) {
                if *h == main {
                    continue;
                }
                let mut hs = HeapSummaryT {
                    cb: std::mem::size_of::<HeapSummaryT>() as u32,
                    ..Default::default()
                };
                if HeapSummary(*h, 0, &mut hs) != 0 {
                    rt.heaps_other += hs.cb_committed as u64;
                }
            }
        }
        rt
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
        fn K32QueryWorkingSetEx(process: *mut c_void, info: *mut WsExInfo, cb: u32) -> i32;
        fn K32QueryWorkingSet(process: *mut c_void, pv: *mut c_void, cb: u32) -> i32;
    }

    /// 작업 관리자 "메모리(개인 작업 집합)" = 워킹 셋 페이지 중 **Shared 비트(8) 없는** 것 × 페이지 크기(`PSAPI_WORKING_SET_BLOCK`).
    pub(super) fn private_ws(s: &SysMem) -> u64 {
        // 첫 칸 = 항목 수 · 이어서 항목들(ULONG_PTR) — 상주 페이지 수 + 여유로 잡고, 모자라면 첫 칸이 알려 주는 수로 한 번 더.
        let mut cap = (s.resident / PAGE as u64) as usize + 4096;
        for _ in 0..2 {
            let mut buf = vec![0usize; cap + 1];
            let cb = (buf.len() * std::mem::size_of::<usize>()) as u32;
            // SAFETY: 버퍼 길이를 바이트로 알리고 커널은 그만큼만 채운다 · 자기 프로세스 핸들은 늘 유효.
            let ok =
                unsafe { K32QueryWorkingSet(GetCurrentProcess(), buf.as_mut_ptr().cast(), cb) }
                    != 0;
            let n = buf[0];
            if ok && n <= cap {
                let private = buf[1..=n].iter().filter(|e| (*e >> 8) & 1 == 0).count() as u64;
                return private * PAGE as u64;
            }
            if n == 0 || n > (1usize << 26) {
                break;
            }
            cap = n + 1024;
        }
        0
    }
    /// `PSAPI_WORKING_SET_EX_INFORMATION` — 주소 · 속성(비트 0 = 상주).
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct WsExInfo {
        va: *mut c_void,
        attrs: usize,
    }
    const PAGE: usize = 4096;

    pub(super) fn resident_bytes(ranges: &[&[u8]]) -> Option<u64> {
        let mut pages: Vec<WsExInfo> = Vec::new();
        for r in ranges {
            let start = r.as_ptr() as usize & !(PAGE - 1);
            let end = (r.as_ptr() as usize).saturating_add(r.len());
            pages.extend((start..end).step_by(PAGE).map(|a| WsExInfo {
                va: a as *mut c_void,
                attrs: 0,
            }));
        }
        if pages.is_empty() {
            return Some(0);
        }
        let mut resident = 0u64;
        for chunk in pages.chunks_mut(4096) {
            // SAFETY: 배열과 바이트 길이를 함께 넘기는 문서화된 호출 — 출력은 호출 동안 살아 있는 우리 버퍼.
            let ok = unsafe {
                K32QueryWorkingSetEx(
                    GetCurrentProcess(),
                    chunk.as_mut_ptr(),
                    std::mem::size_of_val(chunk) as u32,
                )
            } != 0;
            if !ok {
                return None;
            }
            resident += chunk.iter().filter(|p| p.attrs & 1 == 1).count() as u64 * PAGE as u64;
        }
        Some(resident)
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
            private_ws: 0,
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
            private_ws: 0,
        }
    }

    pub(super) fn resident_bytes(_ranges: &[&[u8]]) -> Option<u64> {
        None
    }

    pub(super) fn runtime_breakdown() -> super::RtBreak {
        super::RtBreak::default()
    }

    /// `/proc/self/smaps_rollup`의 Private_Clean + Private_Dirty(kB) — 없으면 0.
    pub(super) fn private_ws(_s: &SysMem) -> u64 {
        std::fs::read_to_string("/proc/self/smaps_rollup")
            .ok()
            .map(|s| {
                s.lines()
                    .filter(|l| l.starts_with("Private_Clean:") || l.starts_with("Private_Dirty:"))
                    .filter_map(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
                    .sum::<u64>()
                    * 1024
            })
            .unwrap_or(0)
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
            private_ws: 0,
        }
    }

    pub(super) fn resident_bytes(_ranges: &[&[u8]]) -> Option<u64> {
        None
    }

    /// 활성 상태 보기의 "메모리" = 풋프린트 그대로 — 따로 세지 않는다(행 숨김).
    pub(super) fn private_ws(_s: &SysMem) -> u64 {
        0
    }

    pub(super) fn runtime_breakdown() -> super::RtBreak {
        super::RtBreak::default()
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

    pub(super) fn resident_bytes(_ranges: &[&[u8]]) -> Option<u64> {
        None
    }

    pub(super) fn private_ws(_s: &SysMem) -> u64 {
        0
    }

    pub(super) fn runtime_breakdown() -> super::RtBreak {
        super::RtBreak::default()
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
        // `sys()`는 전용 워킹 셋을 세지 않는다(비용 분리) — 따로 세면 Windows·Linux glibc에서 0 < 값 ≤ 상주.
        assert_eq!(s.private_ws, 0);
        let pws = super::private_ws(&s);
        if cfg!(any(windows, all(target_os = "linux", target_env = "gnu"))) {
            assert!(pws > 0 && pws <= s.resident, "pws {pws} · {s:?}");
        } else {
            assert_eq!(pws, 0);
        }
    }

    /// 미집계 분해: Windows = 모듈(exe + 시스템 DLL) 여럿 · 스레드(이 시험 스레드 포함) ≥ 1 · 스택 커밋 > 0 · 세 덩어리 합 ≤ 전용 ·
    /// 다른 OS = 전부 0(모름).
    #[test]
    fn runtime_breakdown_is_sane() {
        let rt = super::runtime_breakdown();
        if cfg!(windows) {
            assert!(rt.known() && rt.modules >= 3, "{rt:?}");
            assert!(rt.threads >= 1 && rt.stacks > 0, "{rt:?}");
            assert!(rt.image_private > 0, "{rt:?}");
            let s = super::sys();
            assert!(rt.sum() <= s.footprint, "{rt:?} · {s:?}");
        } else {
            assert_eq!(rt, super::RtBreak::default());
        }
    }
}
