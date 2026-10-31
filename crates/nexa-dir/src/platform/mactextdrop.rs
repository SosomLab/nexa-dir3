//! macOS **글자 드롭 수신**(T-147 잔여 · 10-31 사용자 결정 ⑥ · 협업 조사 journal §20 ⓑ): winit 0.30은 창 델리게이트 `WinitWindowDelegate`가
//! `NSDraggingDestination`을 맡고 `NSFilenamesPboardType`만 등록·수락한다(글자 드래그는 `draggingEntered:`에서 거절). 여기서는 창 생성 뒤 한 번
//! ① 창에 `NSPasteboardTypeString`을 더 등록하고 ② 델리게이트 클래스의 `draggingEntered:`/`performDragOperation:` IMP를 감싼다(원래를 **먼저**
//! 부르고 거짓일 때만 글자 처리 = 파일 드롭 경로 불변 · Finder 드래그에 문자열이 같이 실려도 파일 우선). 글자는 (창 · 글 · 창 안 물리 px)을
//! 정적 큐에 넣고 앱 틱이 거둔다(AppKit 콜백에서 앱 무접촉 · macdrag.rs 규약) · 메인 창이 아니면 무시.
//! 외부 crate 0(`objc` 런타임 + objc2 msg_send) · winit 판 고정 전제 · 클래스/메서드가 없으면 조용히 포기(파일 드롭은 종전대로).

use objc2::runtime::{AnyObject, Bool, Sel};
use objc2::{msg_send, msg_send_id};
use objc2_app_kit::{NSFilenamesPboardType, NSPasteboardTypeFileURL, NSPasteboardTypeString};
use objc2_foundation::{MainThreadMarker, NSArray, NSCopying, NSPoint, NSRect, NSString};
use std::ffi::{c_char, c_void, CStr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[link(name = "objc")]
extern "C" {
    fn object_getClass(obj: *mut c_void) -> *mut c_void;
    fn class_getName(cls: *mut c_void) -> *const c_char;
    fn class_getInstanceMethod(cls: *mut c_void, sel: Sel) -> *mut c_void;
    fn method_getImplementation(m: *mut c_void) -> unsafe extern "C" fn();
    fn method_setImplementation(
        m: *mut c_void,
        imp: unsafe extern "C" fn(),
    ) -> unsafe extern "C" fn();
}

type Imp = unsafe extern "C" fn(*mut AnyObject, Sel, *mut AnyObject) -> Bool;

static ORIG_ENTERED: AtomicUsize = AtomicUsize::new(0);
static ORIG_PERFORM: AtomicUsize = AtomicUsize::new(0);
/// 메인 창(NSWindow) — 이 창의 드롭만 받는다.
static MAIN_WINDOW: AtomicUsize = AtomicUsize::new(0);
static QUEUE: Mutex<Vec<TextDrop>> = Mutex::new(Vec::new());

/// 놓인 글자 하나(창 안 물리 px · 왼쪽 위 원점).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextDrop {
    pub text: String,
    pub x: i32,
    pub y: i32,
}

/// 창 좌표(포인트 · 왼쪽 아래 원점) → 창 안 물리 px(왼쪽 위 원점 · 순수).
pub(crate) fn to_window_px(x_pt: f64, y_pt: f64, content_h_pt: f64, scale: f64) -> (i32, i32) {
    let s = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    (
        (x_pt * s).round() as i32,
        ((content_h_pt - y_pt) * s).round() as i32,
    )
}

unsafe fn pasteboard_string(sender: *mut AnyObject) -> Option<String> {
    let pb: *mut AnyObject = msg_send![sender, draggingPasteboard];
    if pb.is_null() {
        return None;
    }
    let s: Option<objc2::rc::Retained<NSString>> =
        msg_send_id![pb, stringForType: NSPasteboardTypeString];
    s.map(|s| s.to_string()).filter(|s| !s.trim().is_empty())
}

unsafe fn is_main_window(sender: *mut AnyObject) -> bool {
    let w: *mut AnyObject = msg_send![sender, draggingDestinationWindow];
    !w.is_null() && w as usize == MAIN_WINDOW.load(Ordering::Relaxed)
}

unsafe extern "C" fn entered_imp(this: *mut AnyObject, sel: Sel, sender: *mut AnyObject) -> Bool {
    let orig: Imp = std::mem::transmute::<usize, Imp>(ORIG_ENTERED.load(Ordering::Relaxed));
    let r = orig(this, sel, sender);
    if r.as_bool() {
        return r; // 파일 우선(원래 경로)
    }
    Bool::new(is_main_window(sender) && pasteboard_string(sender).is_some())
}

unsafe extern "C" fn perform_imp(this: *mut AnyObject, sel: Sel, sender: *mut AnyObject) -> Bool {
    let orig: Imp = std::mem::transmute::<usize, Imp>(ORIG_PERFORM.load(Ordering::Relaxed));
    let r = orig(this, sel, sender);
    if r.as_bool() || !is_main_window(sender) {
        return r;
    }
    let Some(text) = pasteboard_string(sender) else {
        return Bool::NO;
    };
    let w: *mut AnyObject = msg_send![sender, draggingDestinationWindow];
    let p: NSPoint = msg_send![sender, draggingLocation];
    let cv: *mut AnyObject = msg_send![w, contentView];
    let frame: NSRect = msg_send![cv, frame];
    let scale: f64 = msg_send![w, backingScaleFactor];
    let (x, y) = to_window_px(p.x, p.y, frame.size.height, scale);
    if let Ok(mut q) = QUEUE.lock() {
        q.push(TextDrop { text, x, y });
    }
    Bool::YES
}

/// 설치(창 생성 뒤 한 번 · 메인 스레드) — 성공 = true. 두 번째는 무시(true).
pub(crate) fn install(ns_view: *mut c_void) -> bool {
    if MainThreadMarker::new().is_none() || ns_view.is_null() {
        return false;
    }
    if MAIN_WINDOW.load(Ordering::Relaxed) != 0 {
        return true;
    }
    // SAFETY: 살아 있는 뷰·창·델리게이트 · 런타임 조회 · IMP 교체는 같은 시그니처(id, SEL, id) -> BOOL.
    unsafe {
        let window: *mut AnyObject = msg_send![ns_view as *mut AnyObject, window];
        if window.is_null() {
            return false;
        }
        let delegate: *mut AnyObject = msg_send![window, delegate];
        if delegate.is_null() {
            return false;
        }
        let cls = object_getClass(delegate as *mut c_void);
        let name = CStr::from_ptr(class_getName(cls)).to_string_lossy();
        if name != "WinitWindowDelegate" {
            return false; // winit 판이 바뀌었다 — 포기(파일 드롭은 종전대로).
        }
        let m_enter = class_getInstanceMethod(cls, Sel::register("draggingEntered:"));
        let m_perf = class_getInstanceMethod(cls, Sel::register("performDragOperation:"));
        if m_enter.is_null() || m_perf.is_null() {
            return false;
        }
        MAIN_WINDOW.store(window as usize, Ordering::Relaxed);
        let e = method_setImplementation(
            m_enter,
            std::mem::transmute::<Imp, unsafe extern "C" fn()>(entered_imp),
        );
        ORIG_ENTERED.store(e as usize, Ordering::Relaxed);
        let p = method_setImplementation(
            m_perf,
            std::mem::transmute::<Imp, unsafe extern "C" fn()>(perform_imp),
        );
        ORIG_PERFORM.store(p as usize, Ordering::Relaxed);
        // 기존 타입(파일 이름 · 파일 URL)을 포함해 글자까지 — 창에 등록(뷰가 아니라 창이 수신자 · 협업 조사).
        let types = NSArray::from_id_slice(&[
            NSFilenamesPboardType.copy(),
            NSPasteboardTypeFileURL.copy(),
            NSPasteboardTypeString.copy(),
        ]);
        let _: () = msg_send![window, registerForDraggedTypes: &*types];
    }
    true
}

/// 모인 글자 드롭을 거둔다(틱).
pub(crate) fn take() -> Vec<TextDrop> {
    QUEUE
        .lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 창 좌표 → 물리 px(순수): 아래 원점을 위 원점으로 · 배율 곱 · 비정상 배율 = 1.
    #[test]
    fn window_point_to_top_left_px() {
        assert_eq!(to_window_px(10.0, 20.0, 100.0, 2.0), (20, 160));
        assert_eq!(to_window_px(10.0, 100.0, 100.0, 1.0), (10, 0), "맨 위");
        assert_eq!(to_window_px(3.0, 3.0, 50.0, 0.0), (3, 47), "배율 0 = 1");
    }
}
