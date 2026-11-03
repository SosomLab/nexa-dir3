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
mod tests {
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
    struct VirtualSource {
        descriptor: Vec<u8>,
        contents: Vec<Option<Vec<u8>>>,
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
