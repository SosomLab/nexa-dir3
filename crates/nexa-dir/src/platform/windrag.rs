//! Windows OLE **드래그 발신**(dir2 `nexa-app/src/dnd.rs:240-389` 이식 · SHELL-063~066): 선택한 파일을 다른 프로그램(탐색기 ·
//! VMware 같은 가상 머신 창 · Windows Terminal · 편집기)으로 끌어다 놓는다.
//!
//! - 데이터 = **CF_HDROP을 직접 내는 최소 `IDataObject`**. dir2 실기(07-13): `SHCreateDataObject`(절대 PIDL)는 셸 IDList만 내고
//!   CF_HDROP을 렌더링하지 않아 탐색기와 자기 수신부가 드롭을 거부했다 → 직접 구현. 덤으로 "같은 부모 폴더" 제약이 없어
//!   여러 폴더에 걸친 선택도 끌 수 있다.
//! - `DoDragDrop`은 **모달**이다(버튼을 뗄 때까지 돌아오지 않는다). 그동안 창 메시지는 OLE가 돌리고, winit은 그사이의 사건을
//!   모아 두었다가 돌아온 뒤에 준다 → 호출부(`App`)는 돌아온 뒤 누름 상태를 정리한다.
//! - 이동 결과: 대상이 최적화 이동을 하면 NONE이 돌아온다(파일은 이미 옮겨졌다). MOVE가 돌아와도 **원본을 지우지 않는다**
//!   (dir2 α — 최적화하지 않는 대상의 이동은 복사로 남는 안전한 쪽). 호출부는 결과와 무관하게 다시 읽어 맞춘다.

use super::{DragOutcome, DragSource, PlatformError};
use ::windows::core::{implement, Ref, BOOL, HRESULT};
use ::windows::Win32::Foundation::{
    DATA_S_SAMEFORMATETC, DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS,
    DV_E_FORMATETC, E_NOTIMPL, E_OUTOFMEMORY, HGLOBAL, OLE_E_ADVISENOTSUPPORTED, S_OK,
};
use ::windows::Win32::System::Com::{
    IAdviseSink, IDataObject, IDataObject_Impl, IEnumFORMATETC, IEnumSTATDATA, DATADIR_GET,
    DVASPECT_CONTENT, FORMATETC, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL,
};
use ::windows::Win32::System::Ole::{
    DoDragDrop, IDropSource, IDropSource_Impl, OleInitialize, CF_HDROP, DROPEFFECT,
    DROPEFFECT_COPY, DROPEFFECT_MOVE, DROPEFFECT_NONE,
};
use ::windows::Win32::System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS};
use ::windows::Win32::UI::Shell::SHCreateStdEnumFmtEtc;
use std::path::PathBuf;

pub(super) struct NativeDrag;

impl DragSource for NativeDrag {
    fn begin_drag(&self, paths: &[PathBuf]) -> Result<DragOutcome, PlatformError> {
        if paths.is_empty() {
            return Ok(DragOutcome::Cancelled);
        }
        let data: IDataObject = FileListDataObject {
            paths: paths.to_vec(),
        }
        .into();
        let source: IDropSource = DropSource.into();
        let mut effect = DROPEFFECT_NONE;
        // SAFETY: UI 스레드(STA)에서 부른다 — OleInitialize는 이미 초기화돼 있으면 S_FALSE(무해 · winit도 창 스레드에서 부른다).
        // 두 COM 객체는 이 호출 동안 살아 있고, `effect`는 유효한 출력 자리다.
        let hr = unsafe {
            let _ = OleInitialize(None);
            DoDragDrop(
                &data,
                &source,
                DROPEFFECT_COPY | DROPEFFECT_MOVE,
                &mut effect,
            )
        };
        Ok(outcome(
            hr == DRAGDROP_S_DROP,
            effect.0 & DROPEFFECT_MOVE.0 != 0,
        ))
    }
}

/// `DoDragDrop` 결과 → 호출부가 볼 결과(순수): 놓지 않았으면 취소 · 놓았고 대상이 이동이라 했으면 이동 · 그 밖 = 복사
/// (최적화 이동은 NONE으로 돌아오므로 "복사"로 보인다 — 호출부는 어느 쪽이든 다시 읽는다).
pub(super) fn outcome(dropped: bool, moved: bool) -> DragOutcome {
    match (dropped, moved) {
        (false, _) => DragOutcome::Cancelled,
        (true, true) => DragOutcome::Moved,
        (true, false) => DragOutcome::Copied,
    }
}

/// 드래그 발신원 — 표준 종료 판정(Esc = 취소 · 왼쪽 버튼을 떼면 놓기) · 기본 커서.
#[implement(IDropSource)]
struct DropSource;

impl IDropSource_Impl for DropSource_Impl {
    fn QueryContinueDrag(&self, escape_pressed: BOOL, keys: MODIFIERKEYS_FLAGS) -> HRESULT {
        if escape_pressed.as_bool() {
            return DRAGDROP_S_CANCEL;
        }
        if keys.0 & MK_LBUTTON.0 == 0 {
            return DRAGDROP_S_DROP;
        }
        S_OK
    }

    fn GiveFeedback(&self, _effect: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

/// 발신 데이터 객체 — CF_HDROP(파일 목록) 하나만 낸다.
#[implement(IDataObject)]
struct FileListDataObject {
    paths: Vec<PathBuf>,
}

/// CF_HDROP · TYMED_HGLOBAL · DVASPECT_CONTENT 요청인가.
fn is_hdrop_fmt(fmt: &FORMATETC) -> bool {
    fmt.cfFormat == CF_HDROP.0
        && fmt.tymed & TYMED_HGLOBAL.0 as u32 != 0
        && fmt.dwAspect == DVASPECT_CONTENT.0
}

impl IDataObject_Impl for FileListDataObject_Impl {
    fn GetData(&self, fmt: *const FORMATETC) -> ::windows::core::Result<STGMEDIUM> {
        // SAFETY: 널이 아니면 호출자가 준 유효한 FORMATETC다.
        if fmt.is_null() || !is_hdrop_fmt(unsafe { &*fmt }) {
            return Err(DV_E_FORMATETC.into());
        }
        let hmem = super::windows::hdrop_global(&self.paths)
            .ok_or_else(|| ::windows::core::Error::from(E_OUTOFMEMORY))?;
        Ok(STGMEDIUM {
            tymed: TYMED_HGLOBAL.0 as u32,
            // 소유권은 받는 쪽(ReleaseStgMedium)으로 넘어간다.
            u: STGMEDIUM_0 {
                hGlobal: HGLOBAL(hmem),
            },
            pUnkForRelease: std::mem::ManuallyDrop::new(None),
        })
    }

    fn GetDataHere(&self, _: *const FORMATETC, _: *mut STGMEDIUM) -> ::windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn QueryGetData(&self, fmt: *const FORMATETC) -> HRESULT {
        // SAFETY: 널이 아니면 호출자가 준 유효한 FORMATETC다.
        if !fmt.is_null() && is_hdrop_fmt(unsafe { &*fmt }) {
            S_OK
        } else {
            DV_E_FORMATETC
        }
    }

    fn GetCanonicalFormatEtc(&self, _: *const FORMATETC, out: *mut FORMATETC) -> HRESULT {
        if !out.is_null() {
            // SAFETY: 널이 아니면 호출자가 준 유효한 출력 자리다.
            unsafe { (*out).ptd = std::ptr::null_mut() };
        }
        DATA_S_SAMEFORMATETC
    }

    fn SetData(
        &self,
        _: *const FORMATETC,
        _: *const STGMEDIUM,
        _: BOOL,
    ) -> ::windows::core::Result<()> {
        // 대상의 Performed DropEffect 통지는 받지 않는다(원본을 지우지 않는 쪽).
        Err(E_NOTIMPL.into())
    }

    fn EnumFormatEtc(&self, direction: u32) -> ::windows::core::Result<IEnumFORMATETC> {
        if direction != DATADIR_GET.0 as u32 {
            return Err(E_NOTIMPL.into());
        }
        let fmt = FORMATETC {
            cfFormat: CF_HDROP.0,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };
        // SAFETY: 유효한 FORMATETC 조각 하나.
        unsafe { SHCreateStdEnumFmtEtc(&[fmt]) }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 결과 해석 · 형식 판정 · 데이터 객체가 CF_HDROP만 받아들이고 넣은 경로를 그대로 돌려준다(OS 드래그 없이 COM 객체만 검증).
    #[test]
    fn data_object_offers_hdrop_only() {
        assert_eq!(outcome(false, true), DragOutcome::Cancelled);
        assert_eq!(outcome(true, true), DragOutcome::Moved);
        assert_eq!(outcome(true, false), DragOutcome::Copied);
        let hdrop = FORMATETC {
            cfFormat: CF_HDROP.0,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };
        let text = FORMATETC {
            cfFormat: 13, // CF_UNICODETEXT
            ..hdrop
        };
        assert!(is_hdrop_fmt(&hdrop) && !is_hdrop_fmt(&text));
        let paths = vec![PathBuf::from(r"C:\a\one.txt"), PathBuf::from(r"C:\b\둘.md")];
        let data: IDataObject = FileListDataObject {
            paths: paths.clone(),
        }
        .into();
        // SAFETY: 방금 만든 COM 객체에 유효한 FORMATETC로 묻는다 · 받은 매체는 ReleaseStgMedium으로 푼다.
        unsafe {
            assert_eq!(data.QueryGetData(&hdrop), S_OK);
            assert_eq!(data.QueryGetData(&text), DV_E_FORMATETC);
            assert!(data.GetData(&text).is_err());
            let mut medium = data.GetData(&hdrop).expect("hdrop");
            assert_eq!(medium.tymed, TYMED_HGLOBAL.0 as u32);
            let back = super::super::windows::paths_from_hdrop_global(medium.u.hGlobal.0);
            assert_eq!(back, paths, "넣은 경로 그대로(여러 폴더 · 한글)");
            ::windows::Win32::System::Ole::ReleaseStgMedium(&mut medium);
        }
    }
}
