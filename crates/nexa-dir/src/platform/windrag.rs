//! Windows OLE **드래그 발신**(dir2 `nexa-app/src/dnd.rs:240-389` 이식 · SHELL-063~066): 선택한 파일을 다른 프로그램(탐색기 ·
//! VMware 같은 가상 머신 창 · Windows Terminal · 편집기)으로 끌어다 놓는다.
//!
//! - 데이터 = **CF_HDROP을 직접 내는 최소 `IDataObject`**. dir2 실기(07-13): `SHCreateDataObject`(절대 PIDL)는 셸 IDList만 내고
//!   CF_HDROP을 렌더링하지 않아 탐색기와 자기 수신부가 드롭을 거부했다 → 직접 구현. 덤으로 "같은 부모 폴더" 제약이 없어
//!   여러 폴더에 걸친 선택도 끌 수 있다.
//! - **드래그 그림**(T-147 후속 · 10-18 · dir2에는 없음): 앱이 그린 이름 상자(`App::drag_image` · Linux XDND 이름 상자와 같은 그림)를
//!   셸 `IDragSourceHelper::InitializeFromBitmap`에 넘긴다. 헬퍼는 그림과 상태를 **사설 형식(`DragImageBits` · `DragContext` ·
//!   `IsShowingLayered` · `DropDescription` …)으로 데이터 객체에 `SetData`** 해 두므로 데이터 객체가 그 형식을 **저장했다가 그대로
//!   돌려줘야** 한다(종전 `SetData = E_NOTIMPL`이면 헬퍼가 조용히 실패해 그림이 안 뜬다). 그림은 셸이 레이어드 창으로 띄워
//!   포인터를 따라다니며(Vista+ · 대상이 `IDropTargetHelper`를 안 써도 보인다) 탐색기 같은 대상은 "복사 → 폴더" 글을 덧붙인다
//!   (`DSH_ALLOWDROPDESCRIPTIONTEXT`). 포인터와 그림 사이 빈틈 = 투명 여백 `IMAGE_GAP`(Linux `ICON_OFFSET` 14와 같은 느낌).
//! - `DoDragDrop`은 **모달**이다(버튼을 뗄 때까지 돌아오지 않는다). 그동안 창 메시지는 OLE가 돌리고, winit은 그사이의 사건을
//!   모아 두었다가 돌아온 뒤에 준다 → 호출부(`App`)는 돌아온 뒤 누름 상태를 정리한다.
//! - 이동 결과: 대상이 최적화 이동을 하면 NONE이 돌아온다(파일은 이미 옮겨졌다). MOVE가 돌아와도 **원본을 지우지 않는다**
//!   (dir2 α — 최적화하지 않는 대상의 이동은 복사로 남는 안전한 쪽). 호출부는 결과와 무관하게 다시 읽어 맞춘다.

use super::windows::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
use super::{DragImage, DragOutcome, DragSource, PlatformError};
use ::windows::core::{implement, Interface as _, Ref, BOOL, HRESULT};
use ::windows::Win32::Foundation::{
    COLORREF, DATA_S_SAMEFORMATETC, DRAGDROP_S_CANCEL, DRAGDROP_S_DROP,
    DRAGDROP_S_USEDEFAULTCURSORS, DV_E_FORMATETC, DV_E_TYMED, E_INVALIDARG, E_NOTIMPL,
    E_OUTOFMEMORY, HGLOBAL, OLE_E_ADVISENOTSUPPORTED, POINT, SIZE, S_OK,
};
use ::windows::Win32::Graphics::Gdi::{
    CreateDIBSection, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
    HGDIOBJ,
};
use ::windows::Win32::System::Com::{
    CoCreateInstance, IAdviseSink, IDataObject, IDataObject_Impl, IEnumFORMATETC, IEnumSTATDATA,
    IStream, CLSCTX_INPROC_SERVER, DATADIR_GET, DVASPECT_CONTENT, FORMATETC, STGMEDIUM,
    STGMEDIUM_0, TYMED_HGLOBAL, TYMED_ISTREAM,
};
use ::windows::Win32::System::Ole::{
    DoDragDrop, IDropSource, IDropSource_Impl, OleInitialize, ReleaseStgMedium, CF_HDROP,
    DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_MOVE, DROPEFFECT_NONE,
};
use ::windows::Win32::System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS};
use ::windows::Win32::UI::Shell::{
    CLSID_DragDropHelper, IDragSourceHelper, IDragSourceHelper2, SHCreateStdEnumFmtEtc,
    DSH_ALLOWDROPDESCRIPTIONTEXT, SHDRAGIMAGE,
};
use std::cell::RefCell;
use std::ffi::c_void;
use std::path::PathBuf;

pub(super) struct NativeDrag;

impl DragSource for NativeDrag {
    fn begin_drag(
        &self,
        paths: &[PathBuf],
        image: Option<DragImage>,
    ) -> Result<DragOutcome, PlatformError> {
        if paths.is_empty() {
            return Ok(DragOutcome::Cancelled);
        }
        let data: IDataObject = FileListDataObject::new(paths.to_vec()).into();
        let source: IDropSource = DropSource.into();
        // SAFETY: UI 스레드(STA)에서 부른다 — OleInitialize는 이미 초기화돼 있으면 S_FALSE(무해 · winit도 창 스레드에서 부른다).
        unsafe {
            let _ = OleInitialize(None);
        }
        // 드래그 그림(실패 = 그림 없이 진행 · 헬퍼는 DoDragDrop 동안 살아 있어야 한다).
        let _helper = image.and_then(|img| attach_drag_image(&data, &img));
        let mut effect = DROPEFFECT_NONE;
        // SAFETY: 두 COM 객체는 이 호출 동안 살아 있고, `effect`는 유효한 출력 자리다.
        let hr = unsafe {
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

// ── 드래그 그림 ────────────────────────────────────────────────────────────

/// 포인터와 그림 사이 투명 여백(px · 그림 비트맵의 왼쪽·위에 붙는다 · `ptOffset` = 0,0 → 포인터가 여백의 왼쪽 위 모서리).
const IMAGE_GAP: i32 = 14;

/// 앱 그림(0x00RRGGBB · 행 우선) → 셸 비트맵 바이트(순수): 32bpp **premultiplied BGRA** · 위에서 아래(top-down DIB) · 왼쪽·위
/// `gap`만큼 투명(알파 0) 여백 · 그림 픽셀은 알파 255. 돌려주는 값 = (폭, 높이, 바이트).
pub(super) fn premultiplied_bgra(img: &DragImage, gap: i32) -> (i32, i32, Vec<u8>) {
    let (iw, ih) = (i32::from(img.w), i32::from(img.h));
    let (w, h) = (iw + gap, ih + gap);
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..ih {
        for x in 0..iw {
            let Some(&px) = img.pixels.get((y * iw + x) as usize) else {
                continue;
            };
            let at = (((y + gap) * w + (x + gap)) * 4) as usize;
            out[at] = (px & 0xFF) as u8; // B
            out[at + 1] = ((px >> 8) & 0xFF) as u8; // G
            out[at + 2] = ((px >> 16) & 0xFF) as u8; // R
            out[at + 3] = 0xFF; // A
        }
    }
    (w, h, out)
}

/// 32bpp top-down DIB 섹션을 만들어 바이트를 채운다(실패 = None). 돌려준 핸들의 소유권은 호출자(헬퍼에 넘기면 헬퍼 것).
unsafe fn make_bitmap(w: i32, h: i32, bgra: &[u8]) -> Option<HBITMAP> {
    if w <= 0 || h <= 0 || bgra.len() != (w * h * 4) as usize {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)]
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        bmiColors: [Default::default()],
    };
    let mut bits: *mut c_void = std::ptr::null_mut();
    let hbmp = CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
    if bits.is_null() {
        let _ = DeleteObject(HGDIOBJ(hbmp.0));
        return None;
    }
    std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits as *mut u8, bgra.len());
    Some(hbmp)
}

/// 셸 드래그 헬퍼에 그림을 붙인다(실패 = None · 그림 없이 드래그). 헬퍼는 `SetData`로 데이터 객체에 사설 형식을 넣어 두고,
/// `DoDragDrop` 동안 레이어드 창으로 그림을 띄운다.
fn attach_drag_image(data: &IDataObject, image: &DragImage) -> Option<IDragSourceHelper> {
    let (w, h, bgra) = premultiplied_bgra(image, IMAGE_GAP);
    // SAFETY: STA UI 스레드 · 비트맵은 성공 시 헬퍼 소유 · 실패 시 여기서 지운다.
    unsafe {
        let helper: IDragSourceHelper =
            CoCreateInstance(&CLSID_DragDropHelper, None, CLSCTX_INPROC_SERVER).ok()?;
        if let Ok(h2) = helper.cast::<IDragSourceHelper2>() {
            #[allow(clippy::cast_sign_loss)]
            let _ = h2.SetFlags(DSH_ALLOWDROPDESCRIPTIONTEXT.0 as u32);
        }
        let hbmp = make_bitmap(w, h, &bgra)?;
        let shdi = SHDRAGIMAGE {
            sizeDragImage: SIZE { cx: w, cy: h },
            ptOffset: POINT { x: 0, y: 0 },
            hbmpDragImage: hbmp,
            // 32bpp 알파 비트맵 = 색 키 없음(CLR_NONE).
            crColorKey: COLORREF(0xFFFF_FFFF),
        };
        if helper.InitializeFromBitmap(&shdi, data).is_err() {
            let _ = DeleteObject(HGDIOBJ(hbmp.0));
            return None;
        }
        Some(helper)
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

// ── 데이터 객체 ────────────────────────────────────────────────────────────

/// 남이 `SetData`로 넣어 둔 형식 하나(드래그 헬퍼의 사설 형식 · 대상의 `Performed DropEffect` 등).
struct Stored {
    cf: u16,
    aspect: u32,
    payload: Payload,
}

/// 저장 매체: HGLOBAL은 바이트 사본(우리가 새 HGLOBAL로 돌려준다) · IStream은 인터페이스 참조.
enum Payload {
    Global(Vec<u8>),
    Stream(IStream),
}

impl Payload {
    fn tymed(&self) -> u32 {
        match self {
            Payload::Global(_) => TYMED_HGLOBAL.0 as u32,
            Payload::Stream(_) => TYMED_ISTREAM.0 as u32,
        }
    }
}

/// 발신 데이터 객체 — CF_HDROP(파일 목록)을 내고, 남이 `SetData`로 넣은 형식은 저장했다가 그대로 돌려준다.
#[implement(IDataObject)]
struct FileListDataObject {
    paths: Vec<PathBuf>,
    extra: RefCell<Vec<Stored>>,
}

impl FileListDataObject {
    fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            paths,
            extra: RefCell::new(Vec::new()),
        }
    }
}

/// CF_HDROP · TYMED_HGLOBAL · DVASPECT_CONTENT 요청인가.
fn is_hdrop_fmt(fmt: &FORMATETC) -> bool {
    fmt.cfFormat == CF_HDROP.0
        && fmt.tymed & TYMED_HGLOBAL.0 as u32 != 0
        && fmt.dwAspect == DVASPECT_CONTENT.0
}

/// 저장된 형식이 요청과 맞는가(형식 번호 · 측면 · 매체 비트 겹침 · `ptd`/`lindex`는 보지 않는다 — 헬퍼는 -1로 묻는다).
fn stored_matches(s: &Stored, fmt: &FORMATETC) -> bool {
    s.cf == fmt.cfFormat && s.aspect == fmt.dwAspect && fmt.tymed & s.payload.tymed() != 0
}

/// HGLOBAL 내용 사본(잠금 실패 = 빈 벡터).
unsafe fn global_bytes(h: *mut c_void) -> Vec<u8> {
    if h.is_null() {
        return Vec::new();
    }
    let n = GlobalSize(h);
    let p = GlobalLock(h) as *const u8;
    if p.is_null() {
        return Vec::new();
    }
    let v = std::slice::from_raw_parts(p, n).to_vec();
    GlobalUnlock(h);
    v
}

/// 바이트 → 새 HGLOBAL(받는 쪽이 `ReleaseStgMedium`으로 푼다 · 실패 = None).
unsafe fn global_from_bytes(bytes: &[u8]) -> Option<*mut c_void> {
    let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1));
    if h.is_null() {
        return None;
    }
    let p = GlobalLock(h) as *mut u8;
    if p.is_null() {
        return None;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
    GlobalUnlock(h);
    Some(h)
}

impl IDataObject_Impl for FileListDataObject_Impl {
    fn GetData(&self, fmt: *const FORMATETC) -> ::windows::core::Result<STGMEDIUM> {
        if fmt.is_null() {
            return Err(DV_E_FORMATETC.into());
        }
        // SAFETY: 널이 아니면 호출자가 준 유효한 FORMATETC다.
        let fmt = unsafe { &*fmt };
        if is_hdrop_fmt(fmt) {
            let hmem = super::windows::hdrop_global(&self.paths)
                .ok_or_else(|| ::windows::core::Error::from(E_OUTOFMEMORY))?;
            return Ok(STGMEDIUM {
                tymed: TYMED_HGLOBAL.0 as u32,
                // 소유권은 받는 쪽(ReleaseStgMedium)으로 넘어간다.
                u: STGMEDIUM_0 {
                    hGlobal: HGLOBAL(hmem),
                },
                pUnkForRelease: std::mem::ManuallyDrop::new(None),
            });
        }
        let extra = self.extra.borrow();
        let Some(s) = extra.iter().find(|s| stored_matches(s, fmt)) else {
            return Err(DV_E_FORMATETC.into());
        };
        match &s.payload {
            Payload::Global(bytes) => {
                // SAFETY: 바이트 사본을 새 전역 블록에 담는다.
                let h = unsafe { global_from_bytes(bytes) }
                    .ok_or_else(|| ::windows::core::Error::from(E_OUTOFMEMORY))?;
                Ok(STGMEDIUM {
                    tymed: TYMED_HGLOBAL.0 as u32,
                    u: STGMEDIUM_0 {
                        hGlobal: HGLOBAL(h),
                    },
                    pUnkForRelease: std::mem::ManuallyDrop::new(None),
                })
            }
            Payload::Stream(st) => Ok(STGMEDIUM {
                tymed: TYMED_ISTREAM.0 as u32,
                u: STGMEDIUM_0 {
                    pstm: std::mem::ManuallyDrop::new(Some(st.clone())),
                },
                pUnkForRelease: std::mem::ManuallyDrop::new(None),
            }),
        }
    }

    fn GetDataHere(&self, _: *const FORMATETC, _: *mut STGMEDIUM) -> ::windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn QueryGetData(&self, fmt: *const FORMATETC) -> HRESULT {
        if fmt.is_null() {
            return DV_E_FORMATETC;
        }
        // SAFETY: 널이 아니면 호출자가 준 유효한 FORMATETC다.
        let fmt = unsafe { &*fmt };
        if is_hdrop_fmt(fmt) || self.extra.borrow().iter().any(|s| stored_matches(s, fmt)) {
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

    /// 남이 넣는 형식을 저장한다(드래그 헬퍼 사설 형식 · 대상의 `Performed DropEffect` — 후자는 저장만 하고 원본을 지우지 않는다).
    /// HGLOBAL = 바이트 사본 · IStream = 참조 · 그 밖 매체 = DV_E_TYMED. `release`가 참이면 받은 매체는 우리가 푼다.
    fn SetData(
        &self,
        fmt: *const FORMATETC,
        medium: *const STGMEDIUM,
        release: BOOL,
    ) -> ::windows::core::Result<()> {
        if fmt.is_null() || medium.is_null() {
            return Err(E_INVALIDARG.into());
        }
        // SAFETY: 널이 아니면 호출자가 준 유효한 구조체들이다.
        let (fmt, med) = unsafe { (&*fmt, &*medium) };
        let payload = if med.tymed == TYMED_HGLOBAL.0 as u32 {
            // SAFETY: 매체가 HGLOBAL이라 했으니 union의 hGlobal이 유효하다.
            Payload::Global(unsafe { global_bytes(med.u.hGlobal.0) })
        } else if med.tymed == TYMED_ISTREAM.0 as u32 {
            // SAFETY: 매체가 IStream이라 했으니 union의 pstm이 유효하다(참조 하나를 더 든다).
            match unsafe { (*med.u.pstm).clone() } {
                Some(st) => Payload::Stream(st),
                None => return Err(DV_E_TYMED.into()),
            }
        } else {
            return Err(DV_E_TYMED.into());
        };
        {
            let mut extra = self.extra.borrow_mut();
            extra.retain(|s| !(s.cf == fmt.cfFormat && s.aspect == fmt.dwAspect));
            extra.push(Stored {
                cf: fmt.cfFormat,
                aspect: fmt.dwAspect,
                payload,
            });
        }
        if release.as_bool() {
            // SAFETY: fRelease = 매체 소유권이 우리에게 왔다 → 사본을 떴으니 푼다.
            unsafe { ReleaseStgMedium(medium.cast_mut()) };
        }
        Ok(())
    }

    fn EnumFormatEtc(&self, direction: u32) -> ::windows::core::Result<IEnumFORMATETC> {
        if direction != DATADIR_GET.0 as u32 {
            return Err(E_NOTIMPL.into());
        }
        let mut fmts = vec![FORMATETC {
            cfFormat: CF_HDROP.0,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        }];
        fmts.extend(self.extra.borrow().iter().map(|s| FORMATETC {
            cfFormat: s.cf,
            ptd: std::ptr::null_mut(),
            dwAspect: s.aspect,
            lindex: -1,
            tymed: s.payload.tymed(),
        }));
        // SAFETY: 유효한 FORMATETC 조각들.
        unsafe { SHCreateStdEnumFmtEtc(&fmts) }
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

    fn hdrop_fmt() -> FORMATETC {
        FORMATETC {
            cfFormat: CF_HDROP.0,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        }
    }

    /// 결과 해석 · 형식 판정 · 데이터 객체가 CF_HDROP을 내고 넣은 경로를 그대로 돌려준다(OS 드래그 없이 COM 객체만 검증).
    #[test]
    fn data_object_offers_hdrop() {
        assert_eq!(outcome(false, true), DragOutcome::Cancelled);
        assert_eq!(outcome(true, true), DragOutcome::Moved);
        assert_eq!(outcome(true, false), DragOutcome::Copied);
        let hdrop = hdrop_fmt();
        let text = FORMATETC {
            cfFormat: 13, // CF_UNICODETEXT
            ..hdrop
        };
        assert!(is_hdrop_fmt(&hdrop) && !is_hdrop_fmt(&text));
        let paths = vec![PathBuf::from(r"C:\a\one.txt"), PathBuf::from(r"C:\b\둘.md")];
        let data: IDataObject = FileListDataObject::new(paths.clone()).into();
        // SAFETY: 방금 만든 COM 객체에 유효한 FORMATETC로 묻는다 · 받은 매체는 ReleaseStgMedium으로 푼다.
        unsafe {
            assert_eq!(data.QueryGetData(&hdrop), S_OK);
            assert_eq!(data.QueryGetData(&text), DV_E_FORMATETC);
            assert!(data.GetData(&text).is_err());
            let mut medium = data.GetData(&hdrop).expect("hdrop");
            assert_eq!(medium.tymed, TYMED_HGLOBAL.0 as u32);
            let back = super::super::windows::paths_from_hdrop_global(medium.u.hGlobal.0);
            assert_eq!(back, paths, "넣은 경로 그대로(여러 폴더 · 한글)");
            ReleaseStgMedium(&mut medium);
        }
    }

    /// 드래그 헬퍼처럼 남이 `SetData`로 넣은 HGLOBAL 형식을 저장했다가 같은 바이트로 돌려주고 열거에도 나온다(10-18 드래그 그림 전제).
    #[test]
    fn data_object_keeps_formats_set_by_others() {
        let data: IDataObject = FileListDataObject::new(vec![PathBuf::from(r"C:\x")]).into();
        let private = FORMATETC {
            cfFormat: 0xC123, // 등록 형식 대역의 임의 번호(우리 객체만 본다)
            ..hdrop_fmt()
        };
        // SAFETY: 유효한 전역 블록을 만들어 넣고(fRelease = TRUE · 객체가 푼다), 돌려받은 매체는 우리가 푼다.
        unsafe {
            assert_eq!(data.QueryGetData(&private), DV_E_FORMATETC, "넣기 전");
            let h = global_from_bytes(&[1, 2, 3, 4, 5]).expect("global");
            let medium = STGMEDIUM {
                tymed: TYMED_HGLOBAL.0 as u32,
                u: STGMEDIUM_0 {
                    hGlobal: HGLOBAL(h),
                },
                pUnkForRelease: std::mem::ManuallyDrop::new(None),
            };
            data.SetData(&private, &medium, true).expect("set");
            assert_eq!(data.QueryGetData(&private), S_OK, "넣은 뒤");
            let mut back = data.GetData(&private).expect("get");
            assert_eq!(back.tymed, TYMED_HGLOBAL.0 as u32);
            assert_eq!(global_bytes(back.u.hGlobal.0), vec![1, 2, 3, 4, 5]);
            ReleaseStgMedium(&mut back);
            // 같은 형식을 다시 넣으면 덮어쓴다(중복 없음) · 열거 = HDROP + 사설 1.
            let h2 = global_from_bytes(&[9]).expect("global");
            let medium2 = STGMEDIUM {
                tymed: TYMED_HGLOBAL.0 as u32,
                u: STGMEDIUM_0 {
                    hGlobal: HGLOBAL(h2),
                },
                pUnkForRelease: std::mem::ManuallyDrop::new(None),
            };
            data.SetData(&private, &medium2, true).expect("set2");
            let e = data.EnumFormatEtc(DATADIR_GET.0 as u32).expect("enum");
            let mut buf = [FORMATETC::default(); 8];
            let mut n = 0u32;
            let _ = e.Next(&mut buf, Some(&mut n));
            assert_eq!(n, 2, "HDROP + 사설 형식 하나");
            // 모르는 매체는 거부.
            let bad = STGMEDIUM {
                tymed: 0x40, // TYMED_MFPICT
                u: STGMEDIUM_0 {
                    hGlobal: HGLOBAL(std::ptr::null_mut()),
                },
                pUnkForRelease: std::mem::ManuallyDrop::new(None),
            };
            assert!(data.SetData(&private, &bad, false).is_err());
        }
    }

    /// 셸 드래그 헬퍼가 우리 데이터 객체를 받아들여(`SetData` 저장) 사설 형식을 넣는다 — 그림이 뜨는 전제(OS 드래그 없이 COM만 ·
    /// `InitializeFromBitmap`까지). 종전 `SetData = E_NOTIMPL`이면 여기서 실패했다.
    #[test]
    fn drag_helper_stores_its_formats_in_data_object() {
        let data: IDataObject = FileListDataObject::new(vec![PathBuf::from("C:/x")]).into();
        let img = DragImage {
            w: 8,
            h: 4,
            pixels: vec![0x0012_3456; 32],
        };
        // SAFETY: 시험 스레드를 STA로(이미 다른 모델이면 S_FALSE/RPC_E_CHANGED_MODE — 인프로세스 셸 객체는 그래도 만들어진다).
        unsafe {
            let _ = OleInitialize(None);
        }
        let helper = attach_drag_image(&data, &img);
        assert!(helper.is_some(), "DragDropHelper + InitializeFromBitmap");
        // SAFETY: 방금 만든 COM 객체의 열거를 읽는다.
        unsafe {
            let e = data.EnumFormatEtc(DATADIR_GET.0 as u32).expect("enum");
            let mut buf = [FORMATETC::default(); 16];
            let mut n = 0u32;
            let _ = e.Next(&mut buf, Some(&mut n));
            assert!(n >= 2, "헬퍼가 사설 형식을 넣었다(HDROP 외 {n})");
        }
        drop(helper);
    }

    /// 앱 그림 → 셸 비트맵 바이트(순수): 여백은 투명 · 그림 픽셀은 BGRA + 알파 255 · top-down 행 순서.
    #[test]
    fn premultiplied_bgra_pads_and_swizzles() {
        let img = DragImage {
            w: 2,
            h: 1,
            pixels: vec![0x0011_2233, 0x00FF_0000],
        };
        let (w, h, bytes) = premultiplied_bgra(&img, 2);
        assert_eq!((w, h), (4, 3));
        assert_eq!(bytes.len(), 4 * 3 * 4);
        assert_eq!(&bytes[0..4], &[0, 0, 0, 0], "왼쪽 위 여백 = 투명");
        let at = ((2 * w + 2) * 4) as usize;
        assert_eq!(&bytes[at..at + 4], &[0x33, 0x22, 0x11, 0xFF], "B G R A");
        assert_eq!(&bytes[at + 4..at + 8], &[0x00, 0x00, 0xFF, 0xFF], "빨강");
    }
}
