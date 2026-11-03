//! Windows 가상 파일 추출(T-147 ② · 11-03 · dir2 `nexa-app/src/clipboard.rs` X-42 `read_descriptor`/`plan_dests`/`extract_virtual_from`/
//! `save_contents`/`write_stream` 이식 · SHELL-046~049 · 065): 데이터 객체의 `FileGroupDescriptorW` + `FileContents`(lindex) → 폴더에
//! **동기** 추출(드롭 = `Drop` 반환 전 확보 규약 · 소스가 `DoDragDrop` 종료 후 스트림을 무효화할 수 있다). 파싱 · 경로 방어 · 계획은
//! [`super::vfiles`](3-OS 시험). 클립보드 붙여넣기(워커 · 마샬링 · SHELL-046)는 다음 슬라이스.

use super::vfiles::{parse_group_descriptor, plan_dests, VirtualItem};
use super::windrag::global_bytes;
use ::windows::Win32::System::Com::{
    IDataObject, IStream, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL, TYMED_ISTREAM,
};
use ::windows::Win32::System::Ole::ReleaseStgMedium;
use std::path::{Path, PathBuf};

#[link(name = "user32")]
extern "system" {
    fn RegisterClipboardFormatW(name: *const u16) -> u32;
}

fn registered(name: &str) -> u16 {
    let w: Vec<u16> = name.encode_utf16().chain([0]).collect();
    // SAFETY: NUL 종단 이름 · 등록된 형식 번호는 프로세스 안에서 안정.
    unsafe { RegisterClipboardFormatW(w.as_ptr()) as u16 }
}

/// "FileGroupDescriptorW"(CFSTR_FILEDESCRIPTORW) 형식 번호.
pub(super) fn descriptor_format() -> u16 {
    registered("FileGroupDescriptorW")
}

/// "FileContents"(CFSTR_FILECONTENTS) 형식 번호.
pub(super) fn contents_format() -> u16 {
    registered("FileContents")
}

fn fmt_of(cf: u16, lindex: i32, tymed: u32) -> FORMATETC {
    FORMATETC {
        cfFormat: cf,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex,
        tymed,
    }
}

/// 데이터 객체가 가상 파일(FileGroupDescriptorW)을 광고하는가 — 수신 판정(CF_HDROP이 없을 때만 묻는다).
///
/// # Safety
/// `data`는 살아 있는 COM 객체여야 한다(OLE 콜백 인자).
pub(super) unsafe fn data_has_virtual_files(data: &IDataObject) -> bool {
    data.QueryGetData(&fmt_of(descriptor_format(), -1, TYMED_HGLOBAL.0 as u32))
        .is_ok()
}

/// 설명서 읽기 → 항목 목록(기각 항목 자리 `None` · lindex 정렬).
unsafe fn read_descriptor(data: &IDataObject) -> Vec<Option<VirtualItem>> {
    let Ok(mut medium) = data.GetData(&fmt_of(descriptor_format(), -1, TYMED_HGLOBAL.0 as u32))
    else {
        return Vec::new();
    };
    let bytes = global_bytes(medium.u.hGlobal.0);
    ReleaseStgMedium(&mut medium);
    parse_group_descriptor(&bytes)
}

/// 데이터 객체의 가상 파일을 `dest_dir`에 **동기** 추출 — 항목별 실패 격리(부분 성공 허용) · 실패 항목의 파편은 지운다.
/// 반환: 생성된 **최상위** 경로(전량 실패한 루트 제외).
///
/// # Safety
/// `data`는 살아 있는 COM 객체여야 한다 · UI 스레드(STA)에서 부른다.
pub(super) unsafe fn extract_virtual_from(data: &IDataObject, dest_dir: &Path) -> Vec<PathBuf> {
    let items = read_descriptor(data);
    let (dests, mut created) = plan_dests(&items, dest_dir);
    for (i, item) in items.iter().enumerate() {
        let (Some(item), Some(dest)) = (item, &dests[i]) else {
            continue;
        };
        if item.is_dir {
            let _ = std::fs::create_dir_all(dest);
            continue;
        }
        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if !save_contents(data, i, dest, item.size) {
            let _ = std::fs::remove_file(dest);
        }
    }
    created.retain(|p| p.exists());
    created
}

/// `FileContents`(lindex = 설명서 인덱스)를 `dest`로 — Outlook · rdpclip = IStream · 일부 소스 = HGLOBAL(둘 다 수용).
unsafe fn save_contents(data: &IDataObject, index: usize, dest: &Path, size: Option<u64>) -> bool {
    let fmt = fmt_of(
        contents_format(),
        index as i32,
        (TYMED_ISTREAM.0 | TYMED_HGLOBAL.0) as u32,
    );
    let Ok(mut medium) = data.GetData(&fmt) else {
        return false;
    };
    let ok = if medium.tymed == TYMED_ISTREAM.0 as u32 {
        match (*medium.u.pstm).as_ref() {
            Some(stream) => write_stream(stream, dest, size),
            None => false,
        }
    } else if medium.tymed == TYMED_HGLOBAL.0 as u32 {
        let bytes = global_bytes(medium.u.hGlobal.0);
        // HGLOBAL 관례상 여분 패딩이 붙을 수 있어 크기 힌트가 있으면 그만큼만.
        let len = size.map_or(bytes.len(), |s| (s as usize).min(bytes.len()));
        std::fs::write(dest, &bytes[..len]).is_ok()
    } else {
        false
    };
    ReleaseStgMedium(&mut medium);
    ok
}

/// IStream → 파일(64 KiB 버퍼) — 크기 미상은 EOF(read 0)까지 · 크기 힌트에 닿으면 멈춘다(EOF를 안 주는 소스 방어). 원격 소스(RDP)는
/// 여기서 실제 전송이 일어난다.
unsafe fn write_stream(stream: &IStream, dest: &Path, size: Option<u64>) -> bool {
    use std::io::Write;
    let Ok(mut file) = std::fs::File::create(dest) else {
        return false;
    };
    let mut buf = vec![0u8; 64 * 1024];
    let mut total: u64 = 0;
    loop {
        let mut read = 0u32;
        let hr = stream.Read(buf.as_mut_ptr().cast(), buf.len() as u32, Some(&mut read));
        if read > 0 {
            if file.write_all(&buf[..read as usize]).is_err() {
                return false;
            }
            total += u64::from(read);
        }
        if hr.is_err() {
            return false;
        }
        if read == 0 || size.is_some_and(|s| total >= s) {
            break;
        }
    }
    true
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::platform::vfiles::tests::{fd, group_bytes};
    use crate::platform::vfiles::{FD_ATTRIBUTES, FD_FILESIZE, FILE_ATTRIBUTE_DIRECTORY};
    use ::windows::core::{implement, Ref, BOOL, HRESULT};
    use ::windows::Win32::Foundation::HGLOBAL;
    use ::windows::Win32::Foundation::{
        DATA_S_SAMEFORMATETC, DV_E_FORMATETC, E_NOTIMPL, E_OUTOFMEMORY, OLE_E_ADVISENOTSUPPORTED,
        S_OK,
    };
    use ::windows::Win32::System::Com::{
        IAdviseSink, IDataObject_Impl, IEnumFORMATETC, IEnumSTATDATA, STGMEDIUM, STGMEDIUM_0,
    };
    use ::windows::Win32::UI::Shell::SHCreateMemStream;

    /// 가상 파일 소스 흉내(Outlook · rdpclip 형상) — 설명서 HGLOBAL + FileContents IStream(디렉터리 항목은 None).
    #[implement(IDataObject)]
    pub(super) struct VirtualSource {
        pub(super) descriptor: Vec<u8>,
        pub(super) contents: Vec<Option<Vec<u8>>>,
    }

    impl IDataObject_Impl for VirtualSource_Impl {
        fn GetData(&self, fmt: *const FORMATETC) -> ::windows::core::Result<STGMEDIUM> {
            // SAFETY: 호출자가 준 유효한 FORMATETC.
            let fmt = unsafe { &*fmt };
            if fmt.cfFormat == descriptor_format() {
                // SAFETY: 바이트 사본을 새 전역 블록에.
                let h = unsafe { super::super::windrag::global_from_bytes(&self.descriptor) }
                    .ok_or_else(|| ::windows::core::Error::from(E_OUTOFMEMORY))?;
                return Ok(STGMEDIUM {
                    tymed: TYMED_HGLOBAL.0 as u32,
                    u: STGMEDIUM_0 {
                        hGlobal: HGLOBAL(h),
                    },
                    pUnkForRelease: std::mem::ManuallyDrop::new(None),
                });
            }
            if fmt.cfFormat == contents_format() {
                let body = self
                    .contents
                    .get(fmt.lindex as usize)
                    .and_then(|c| c.as_ref())
                    .ok_or_else(|| ::windows::core::Error::from(DV_E_FORMATETC))?;
                // SAFETY: 바이트 조각으로 메모리 스트림.
                let stream = unsafe { SHCreateMemStream(Some(body)) }
                    .ok_or_else(|| ::windows::core::Error::from(E_OUTOFMEMORY))?;
                return Ok(STGMEDIUM {
                    tymed: TYMED_ISTREAM.0 as u32,
                    u: STGMEDIUM_0 {
                        pstm: std::mem::ManuallyDrop::new(Some(stream)),
                    },
                    pUnkForRelease: std::mem::ManuallyDrop::new(None),
                });
            }
            Err(DV_E_FORMATETC.into())
        }
        fn GetDataHere(
            &self,
            _: *const FORMATETC,
            _: *mut STGMEDIUM,
        ) -> ::windows::core::Result<()> {
            Err(E_NOTIMPL.into())
        }
        fn QueryGetData(&self, fmt: *const FORMATETC) -> HRESULT {
            // SAFETY: 호출자가 준 유효한 FORMATETC.
            let cf = unsafe { (*fmt).cfFormat };
            if cf == descriptor_format() || cf == contents_format() {
                S_OK
            } else {
                DV_E_FORMATETC
            }
        }
        fn GetCanonicalFormatEtc(&self, _: *const FORMATETC, _: *mut FORMATETC) -> HRESULT {
            DATA_S_SAMEFORMATETC
        }
        fn SetData(
            &self,
            _: *const FORMATETC,
            _: *const STGMEDIUM,
            _: BOOL,
        ) -> ::windows::core::Result<()> {
            Err(E_NOTIMPL.into())
        }
        fn EnumFormatEtc(&self, _: u32) -> ::windows::core::Result<IEnumFORMATETC> {
            Err(E_NOTIMPL.into())
        }
        fn DAdvise(
            &self,
            _: *const FORMATETC,
            _: u32,
            _: Ref<'_, IAdviseSink>,
        ) -> ::windows::core::Result<u32> {
            Err(OLE_E_ADVISENOTSUPPORTED.into())
        }
        fn DUnadvise(&self, _: u32) -> ::windows::core::Result<()> {
            Err(OLE_E_ADVISENOTSUPPORTED.into())
        }
        fn EnumDAdvise(&self) -> ::windows::core::Result<IEnumSTATDATA> {
            Err(OLE_E_ADVISENOTSUPPORTED.into())
        }
    }

    /// dir2 `extract_virtual_from_writes_files_dirs_and_resolves_collision`: 가짜 소스 → 실제 추출 종단 — 광고 판정 · 파일 내용 · 하위 폴더 ·
    /// 크기 미상 스트림(64 KiB 초과) · 기존 이름 충돌 " (2)" · 기각 항목(탈출)은 건너뛰고 lindex는 그대로.
    #[test]
    fn extract_virtual_from_writes_files_dirs_and_resolves_collision() {
        let dir = std::env::temp_dir().join(format!("ndir-winvirtual-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("문서.txt"), "이미 있음").unwrap();
        let fds = [
            fd("문서.txt", FD_FILESIZE, 0, 4),
            fd("..\\탈출.txt", 0, 0, 0),
            fd("폴더", FD_ATTRIBUTES, FILE_ATTRIBUTE_DIRECTORY, 0),
            fd("폴더\\안쪽.bin", 0, 0, 0),
        ];
        let data: IDataObject = VirtualSource {
            descriptor: group_bytes(&fds),
            contents: vec![
                Some(b"abcd".to_vec()),
                Some(b"never".to_vec()),
                None,
                Some(vec![7u8; 100_000]),
            ],
        }
        .into();
        // SAFETY: 방금 만든 COM 객체.
        unsafe {
            assert!(data_has_virtual_files(&data), "가상 파일 광고 판정");
            let created = extract_virtual_from(&data, &dir);
            assert_eq!(
                created,
                vec![dir.join("문서 (2).txt"), dir.join("폴더")],
                "최상위 2건 · 충돌 (2) · 탈출 기각"
            );
        }
        assert_eq!(std::fs::read(dir.join("문서 (2).txt")).unwrap(), b"abcd");
        assert_eq!(
            std::fs::read_to_string(dir.join("문서.txt")).unwrap(),
            "이미 있음",
            "기존 파일 무손상"
        );
        assert_eq!(
            std::fs::read(dir.join("폴더").join("안쪽.bin")).unwrap(),
            vec![7u8; 100_000],
            "64 KiB 초과 스트림"
        );
        assert!(!dir.join("탈출.txt").exists() && !dir.parent().unwrap().join("탈출.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

// ── 클립보드 가상 파일 붙여넣기(SHELL-046~049 · dir2 X-42 2차-ⓑ 워커화 이식) ─────────────────────────────
//
// `OleGetClipboard`의 IDataObject는 STA 구속이라 워커에서 직접 못 쓴다. UI 스레드가 **설명서 파싱 · 대상 계획 · 항목별 GetData(스트림 확보)**까지
// 마치고 IStream만 `CoMarshalInterThreadInterfaceInStream`으로 마샬링해 넘긴다 — out-of-proc 소스(rdpclip · Outlook)는 워커의 언마샬이 원
// 프로세스로 직접 이어지는 프록시가 되어 Read(실제 바이트 전송)가 UI 스레드 펌프에 의존하지 않는다. HGLOBAL 소스는 이미 메모리라 즉시 복사.

use ::windows::core::Interface as _;
use ::windows::Win32::System::Com::Marshal::CoMarshalInterThreadInterfaceInStream;
use ::windows::Win32::System::Com::StructuredStorage::CoGetInterfaceAndReleaseStream;
use ::windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
use ::windows::Win32::System::Ole::OleGetClipboard;

/// 워커 소스 1개 — 계획 시점에 확보 완료(Send 가능).
pub(crate) enum VSource {
    /// 마샬 패킷(IStream)의 raw 포인터 — 워커가 `CoGetInterfaceAndReleaseStream`으로 원 스트림 프록시를 되살린다(취소여도 정확히 1회 소비).
    Marshaled(isize),
    /// TYMED_HGLOBAL 소스 — 계획 시점 복사본.
    Bytes(Vec<u8>),
}

/// 워커 잡 항목(계획 순서 = 부모 우선).
pub(crate) enum VOp {
    Dir(PathBuf),
    File {
        dest: PathBuf,
        src: VSource,
        size: Option<u64>,
    },
}

/// 붙여넣기 계획 — UI 스레드가 만들고 워커가 소비.
pub(crate) struct VPastePlan {
    pub(crate) ops: Vec<VOp>,
    /// 생성될 최상위 경로(완료 뒤 undo 기록 · 재열람).
    pub(crate) roots: Vec<PathBuf>,
}

/// OS 클립보드의 가상 파일 붙여넣기 계획(UI 스레드 전용 · STA) — 가상 파일이 없으면 `None`.
pub(super) fn plan_clipboard_paste(dest_dir: &Path) -> Option<VPastePlan> {
    // SAFETY: UI 스레드 · 받은 데이터 객체는 이 함수 안에서만 쓴다.
    unsafe {
        let data = OleGetClipboard().ok()?;
        if !data_has_virtual_files(&data) {
            return None;
        }
        plan_virtual_paste(&data, dest_dir)
    }
}

/// 데이터 객체 기반 계획(UI 스레드) — 소스 확보(GetData)는 항목당 1회(스트림 핸들만) · 실제 바이트 전송은 워커의 Read에서. 확보 실패 항목만 격리.
///
/// # Safety
/// `data`는 살아 있는 COM 객체 · 호출 스레드의 아파트먼트에서 마샬링한다.
pub(super) unsafe fn plan_virtual_paste(data: &IDataObject, dest_dir: &Path) -> Option<VPastePlan> {
    let items = read_descriptor(data);
    if items.iter().all(Option::is_none) {
        return None;
    }
    let (dests, roots) = plan_dests(&items, dest_dir);
    let mut ops = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let (Some(item), Some(dest)) = (item, &dests[i]) else {
            continue;
        };
        if item.is_dir {
            ops.push(VOp::Dir(dest.clone()));
            continue;
        }
        let Some(src) = acquire_contents(data, i, item.size) else {
            continue;
        };
        ops.push(VOp::File {
            dest: dest.clone(),
            src,
            size: item.size,
        });
    }
    (!ops.is_empty()).then_some(VPastePlan { ops, roots })
}

/// FileContents 소스 확보(UI 스레드) — IStream = 마샬 패킷 · HGLOBAL = 즉시 복사(크기 힌트만큼).
unsafe fn acquire_contents(data: &IDataObject, index: usize, size: Option<u64>) -> Option<VSource> {
    let fmt = fmt_of(
        contents_format(),
        index as i32,
        (TYMED_ISTREAM.0 | TYMED_HGLOBAL.0) as u32,
    );
    let Ok(mut medium) = data.GetData(&fmt) else {
        return None;
    };
    let out = if medium.tymed == TYMED_ISTREAM.0 as u32 {
        (*medium.u.pstm).as_ref().and_then(|stream| {
            CoMarshalInterThreadInterfaceInStream(&IStream::IID, stream)
                .ok()
                .map(|pkt| VSource::Marshaled(pkt.into_raw() as isize))
        })
    } else if medium.tymed == TYMED_HGLOBAL.0 as u32 {
        let bytes = global_bytes(medium.u.hGlobal.0);
        let len = size.map_or(bytes.len(), |s| (s as usize).min(bytes.len()));
        Some(VSource::Bytes(bytes[..len].to_vec()))
    } else {
        None
    };
    ReleaseStgMedium(&mut medium);
    out
}

/// 계획 실행(워커 스레드 · MTA): 항목마다 `(index, ok)`를 보내고 끝나면 채널을 닫는다. `cancel`이 서면 남은 항목은 건너뛴다(마샬 패킷은
/// 누수 방지를 위해 그래도 소비). 쓰다 만 파일은 지운다.
pub(super) fn run_virtual_paste(
    plan: VPastePlan,
    tx: std::sync::mpsc::Sender<(usize, bool)>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> std::thread::JoinHandle<()> {
    use std::sync::atomic::Ordering;
    std::thread::spawn(move || {
        // SAFETY: 이 스레드의 COM 초기화/해제가 짝을 이룬다 · 패킷은 1회 소비.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            for (i, op) in plan.ops.into_iter().enumerate() {
                let cancelled = cancel.load(Ordering::Relaxed);
                let ok = match op {
                    VOp::Dir(d) => !cancelled && std::fs::create_dir_all(&d).is_ok(),
                    VOp::File { dest, src, size } => match src {
                        VSource::Bytes(b) => {
                            !cancelled
                                && dest
                                    .parent()
                                    .is_none_or(|p| std::fs::create_dir_all(p).is_ok())
                                && std::fs::write(&dest, &b).is_ok()
                        }
                        VSource::Marshaled(ptr) => {
                            // 패킷 스트림은 CoGetInterfaceAndReleaseStream이 **놓아준다** → 우리는 drop하지 않는다(이중 Release = 11-03 시험 크래시).
                            let pkt = std::mem::ManuallyDrop::new(IStream::from_raw(ptr as *mut _));
                            let stream = CoGetInterfaceAndReleaseStream::<_, IStream>(&*pkt).ok();
                            if cancelled {
                                false
                            } else {
                                if let Some(parent) = dest.parent() {
                                    let _ = std::fs::create_dir_all(parent);
                                }
                                let ok = stream.as_ref().is_some_and(|s| {
                                    write_stream_cancellable(s, &dest, size, &cancel)
                                });
                                if !ok {
                                    let _ = std::fs::remove_file(&dest);
                                }
                                ok
                            }
                        }
                    },
                };
                let _ = tx.send((i, ok));
            }
            CoUninitialize();
        }
    })
}

/// 스트림 → 파일(워커 · 256 KiB 버퍼 · 청크마다 취소 확인). 크기 미상은 EOF까지.
unsafe fn write_stream_cancellable(
    stream: &IStream,
    dest: &Path,
    size: Option<u64>,
    cancel: &std::sync::atomic::AtomicBool,
) -> bool {
    use std::io::Write;
    let Ok(mut file) = std::fs::File::create(dest) else {
        return false;
    };
    let mut buf = vec![0u8; 256 * 1024];
    let mut total: u64 = 0;
    loop {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return false;
        }
        let mut read = 0u32;
        let hr = stream.Read(buf.as_mut_ptr().cast(), buf.len() as u32, Some(&mut read));
        if read > 0 {
            if file.write_all(&buf[..read as usize]).is_err() {
                return false;
            }
            total += u64::from(read);
        }
        if hr.is_err() {
            return false;
        }
        if read == 0 || size.is_some_and(|s| total >= s) {
            break;
        }
    }
    true
}

#[cfg(test)]
mod paste_tests {
    use super::tests::VirtualSource;
    use super::*;
    use crate::platform::vfiles::tests::{fd, group_bytes};
    use crate::platform::vfiles::{FD_ATTRIBUTES, FD_FILESIZE, FILE_ATTRIBUTE_DIRECTORY};

    /// dir2 `plan_and_run_virtual_paste_worker_round_trip`: 계획(스트림 마샬링) → 워커(MTA · 언마샬 · 256 KiB 초과 다중 청크 · 하위 폴더) →
    /// 파일 내용 · 항목별 결과 · 끝 = 채널 닫힘. 실 클립보드 비접촉(가짜 소스).
    #[test]
    fn plan_and_run_virtual_paste_worker_round_trip() {
        let dir = std::env::temp_dir().join(format!("ndir-vpaste-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // SAFETY: 시험 스레드를 MTA로(앱 = UI STA · 워커 MTA — 표준 마샬링이 어느 쪽이든 잇는다).
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let big = vec![9u8; 300_000];
        let fds = [
            fd("워커.bin", FD_FILESIZE, 0, big.len() as u32),
            fd("폴더", FD_ATTRIBUTES, FILE_ATTRIBUTE_DIRECTORY, 0),
            fd("폴더\\작은.txt", 0, 0, 0),
        ];
        let data: IDataObject = VirtualSource {
            descriptor: group_bytes(&fds),
            contents: vec![Some(big.clone()), None, Some(b"ab".to_vec())],
        }
        .into();
        // SAFETY: 방금 만든 COM 객체.
        let plan = unsafe { plan_virtual_paste(&data, &dir) }.expect("plan");
        assert_eq!(plan.roots, vec![dir.join("워커.bin"), dir.join("폴더")]);
        assert_eq!(plan.ops.len(), 3);
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        run_virtual_paste(plan, tx, cancel).join().unwrap();
        let got: Vec<(usize, bool)> = rx.iter().collect();
        assert_eq!(
            got,
            vec![(0, true), (1, true), (2, true)],
            "항목별 결과 · 끝 = 채널 닫힘"
        );
        assert_eq!(
            std::fs::read(dir.join("워커.bin")).unwrap(),
            big,
            "256 KiB 초과 다중 청크"
        );
        assert_eq!(
            std::fs::read(dir.join("폴더").join("작은.txt")).unwrap(),
            b"ab"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
