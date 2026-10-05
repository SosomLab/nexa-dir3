//! Windows OLE **드롭 수신**(dir2 `nexa-app/src/dnd.rs` `DropTarget` 이식 · SHELL-046~049 · 060~062 · T-147).
//!
//! winit의 기본 수신부(`HoveredFile`/`DroppedFile`)는 **포인터 자리 · 수식키를 주지 않고 효과(커서 모양)를 늘 "복사"로** 돌려준다 →
//! 메인 창은 `with_drag_and_drop(false)`로 만들고 이 `IDropTarget`을 직접 등록한다.
//!
//! - `DragEnter`/`DragOver`/`Drop`마다 [`DropEvent`](창 안 좌표 · Ctrl · Shift · 끌려오는 경로)를 만들어 **효과를 돌려준다**
//!   (복사 = + 커서 · 이동 = 화살표 · 불가 = 금지 표시 — OS가 그린다).
//! - 사건이 가는 곳은 둘 중 하나([`super::drop_dispatch`]):
//! - ① **우리 창에서 시작한 드래그**(`DoDragDrop`이 도는 동안 앱은 그 안에 멈춰 있다) = 호출부가 걸어 둔 실시간 수신기로 바로 —
//!   앱이 그 자리에서 추적 · 그리기 · 효과 판정을 한다.
//! - ② **다른 프로그램의 드래그** = 큐에 쌓고(앱이 다음 틱에 거둔다) 효과는 패널 구역 요약으로 여기서 정한다.
//! - 이동으로 놓였을 때는 소스에 **`DROPEFFECT_NONE`**을 돌려준다(최적화 이동 규약 — 옮기는 일은 우리 전송 엔진이 한다.
//!   MOVE를 돌려주면 소스(탐색기)가 원본을 지워 비동기 전송과 경쟁한다 · dir2 dnd.rs:466-475).

use super::{DropChoice, DropEvent, DropShared};
use ::windows::core::{implement, Ref};
use ::windows::Win32::Foundation::{HWND, POINT, POINTL};
use ::windows::Win32::Graphics::Gdi::ScreenToClient;
use ::windows::Win32::System::Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL};
use ::windows::Win32::System::Ole::{
    IDropTarget, IDropTarget_Impl, OleInitialize, RegisterDragDrop, ReleaseStgMedium, CF_HDROP,
    DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_MOVE, DROPEFFECT_NONE,
};
use ::windows::Win32::System::SystemServices::{MK_CONTROL, MK_SHIFT, MODIFIERKEYS_FLAGS};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

/// 메인 창에 드롭 수신부를 등록한다 — 성공하면 앱과 나눠 쓰는 상태(사건 큐 · 구역 요약)를 돌려준다.
pub(super) fn register(hwnd: isize) -> Option<Rc<RefCell<DropShared>>> {
    let shared = Rc::new(RefCell::new(DropShared::default()));
    let target: IDropTarget = DropTarget {
        hwnd: HWND(hwnd as *mut core::ffi::c_void),
        shared: Rc::clone(&shared),
        paths: RefCell::new(Vec::new()),
    }
    .into();
    // SAFETY: UI 스레드(STA)에서 부른다 · OleInitialize는 이미 돼 있으면 S_FALSE(무해) · 등록된 COM 객체의 수명은 OLE가 쥔다.
    unsafe {
        let _ = OleInitialize(None);
        RegisterDragDrop(HWND(hwnd as *mut core::ffi::c_void), &target).ok()?;
    }
    Some(shared)
}

#[implement(IDropTarget)]
struct DropTarget {
    hwnd: HWND,
    shared: Rc<RefCell<DropShared>>,
    /// `DragEnter`에서 읽은 경로(CF_HDROP) — `DragOver`가 판정에 쓰고 `Drop`이 다시 읽어 갱신한다.
    paths: RefCell<Vec<PathBuf>>,
}

/// 데이터 객체의 CF_HDROP 경로 목록(없거나 실패 = 빈 목록).
///
/// # Safety
/// `data`는 살아 있는 COM 객체여야 한다(OLE 콜백 인자).
unsafe fn hdrop_paths(data: &IDataObject) -> Vec<PathBuf> {
    let fmt = FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };
    let Ok(mut medium) = data.GetData(&fmt) else {
        return Vec::new();
    };
    let paths = super::windows::paths_from_hdrop_global(medium.u.hGlobal.0);
    ReleaseStgMedium(&mut medium);
    paths
}

impl DropTarget_Impl {
    /// 화면 좌표 → 창 안 좌표 + 수식키.
    fn locate(&self, keys: MODIFIERKEYS_FLAGS, pt: &POINTL) -> ((i32, i32), bool, bool) {
        let mut p = POINT { x: pt.x, y: pt.y };
        // SAFETY: 유효한 창 핸들과 지역 변수(실패하면 화면 좌표 그대로 — 판정이 "창 밖"이 될 뿐이다).
        unsafe {
            let _ = ScreenToClient(self.hwnd, &mut p);
        }
        (
            (p.x, p.y),
            keys.0 & MK_CONTROL.0 != 0,
            keys.0 & MK_SHIFT.0 != 0,
        )
    }

    fn send(&self, ev: DropEvent) -> DropChoice {
        super::drop_dispatch(&self.shared, ev)
    }
}

/// 판정 → OLE 효과(순수): `on_drop`이면 이동을 NONE으로 바꾼다(최적화 이동 — 소스가 원본을 지우지 않게).
pub(super) fn effect_of(choice: DropChoice, allowed: DROPEFFECT, on_drop: bool) -> DROPEFFECT {
    let want = match choice {
        DropChoice::None => DROPEFFECT_NONE,
        DropChoice::Copy => DROPEFFECT_COPY,
        DropChoice::Move if on_drop => DROPEFFECT_NONE,
        DropChoice::Move => DROPEFFECT_MOVE,
    };
    // 소스가 허용하지 않는 효과는 낼 수 없다(이동만 막힌 소스 = 복사로 · 둘 다 안 되면 불가).
    if want.0 & allowed.0 != 0 || want == DROPEFFECT_NONE {
        want
    } else if choice == DropChoice::Move && allowed.0 & DROPEFFECT_COPY.0 != 0 {
        DROPEFFECT_COPY
    } else {
        DROPEFFECT_NONE
    }
}

impl IDropTarget_Impl for DropTarget_Impl {
    fn DragEnter(
        &self,
        data: Ref<'_, IDataObject>,
        keys: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> ::windows::core::Result<()> {
        // SAFETY: OLE가 준 데이터 객체 · 출력 자리는 널이 아니면 유효하다.
        let paths = data
            .as_ref()
            .map(|d| unsafe { hdrop_paths(d) })
            .unwrap_or_default();
        *self.paths.borrow_mut() = paths.clone();
        let (at, ctrl, shift) = self.locate(keys, pt);
        let choice = if paths.is_empty() {
            DropChoice::None // 파일이 아닌 것(글자 · 가상 파일)은 아직 받지 않는다
        } else {
            self.send(DropEvent::Enter {
                paths,
                at,
                ctrl,
                shift,
            })
        };
        if !effect.is_null() {
            // SAFETY: 위 주석.
            unsafe { *effect = effect_of(choice, *effect, false) };
        }
        Ok(())
    }

    fn DragOver(
        &self,
        keys: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> ::windows::core::Result<()> {
        let (at, ctrl, shift) = self.locate(keys, pt);
        let choice = if self.paths.borrow().is_empty() {
            DropChoice::None
        } else {
            self.send(DropEvent::Over { at, ctrl, shift })
        };
        if !effect.is_null() {
            // SAFETY: 널이 아니면 OLE가 준 유효한 출력 자리.
            unsafe { *effect = effect_of(choice, *effect, false) };
        }
        Ok(())
    }

    fn DragLeave(&self) -> ::windows::core::Result<()> {
        if !self.paths.borrow().is_empty() {
            self.paths.borrow_mut().clear();
            let _ = self.send(DropEvent::Leave);
        }
        Ok(())
    }

    fn Drop(
        &self,
        data: Ref<'_, IDataObject>,
        keys: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> ::windows::core::Result<()> {
        // 놓는 순간 다시 읽는다(지연 렌더링 소스는 이때 최종 경로를 준다 — 실패하면 들어올 때 읽은 것).
        // SAFETY: OLE가 준 데이터 객체.
        let fresh = data
            .as_ref()
            .map(|d| unsafe { hdrop_paths(d) })
            .unwrap_or_default();
        let paths = if fresh.is_empty() {
            std::mem::take(&mut *self.paths.borrow_mut())
        } else {
            self.paths.borrow_mut().clear();
            fresh
        };
        let (at, ctrl, shift) = self.locate(keys, pt);
        let choice = if paths.is_empty() {
            DropChoice::None
        } else {
            self.send(DropEvent::Drop {
                paths,
                at,
                ctrl,
                shift,
            })
        };
        if !effect.is_null() {
            // SAFETY: 널이 아니면 OLE가 준 유효한 출력 자리.
            unsafe { *effect = effect_of(choice, *effect, true) };
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 효과 변환: 놓을 때의 이동 = NONE(최적화 이동) · 소스가 이동을 막으면 복사로 · 아무것도 안 되면 불가.
    #[test]
    fn effect_mapping() {
        let both = DROPEFFECT_COPY | DROPEFFECT_MOVE;
        assert_eq!(effect_of(DropChoice::Copy, both, false), DROPEFFECT_COPY);
        assert_eq!(effect_of(DropChoice::Move, both, false), DROPEFFECT_MOVE);
        assert_eq!(effect_of(DropChoice::None, both, false), DROPEFFECT_NONE);
        assert_eq!(
            effect_of(DropChoice::Move, both, true),
            DROPEFFECT_NONE,
            "놓을 때 이동 = NONE"
        );
        assert_eq!(effect_of(DropChoice::Copy, both, true), DROPEFFECT_COPY);
        assert_eq!(
            effect_of(DropChoice::Move, DROPEFFECT_COPY, false),
            DROPEFFECT_COPY
        );
        assert_eq!(
            effect_of(DropChoice::Copy, DROPEFFECT_MOVE, false),
            DROPEFFECT_NONE
        );
    }
}
