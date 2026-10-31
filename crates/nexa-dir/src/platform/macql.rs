//! macOS **인-프로세스 Quick Look**(T-187 · 10-31 사용자 결정 ② · 협업 조사 journal §20): `QLPreviewPanel`(Quartz ▸ QuickLookUI · 앱당 하나)
//! + 데이터소스 객체 `NexaDirQLSource`(선택 경로 → `NSURL`) + **winit 뷰 클래스에 응답자 메서드 3개 추가**(`acceptsPreviewPanelControl:` ·
//!   `beginPreviewPanelControl:` · `endPreviewPanelControl:` — 패널은 응답자 체인에서 컨트롤러를 찾고 그때만 데이터소스를 바꿀 수 있다 · 헤더 규칙).
//!
//! - 종전(`qlmanage -p` 자식 프로세스)은 선택을 따라가지 못했다 → 여기서는 선택이 바뀌면 `reloadData`(패널이 열린 동안).
//! - 패널 안 ↑/↓/Space 키는 델리게이트 `previewPanel:handleEvent:`로 와서 **원자 큐**에 쌓고 앱 틱이 거둔다(AppKit 콜백에서 앱 상태 무접촉 ·
//!   macdrag.rs 규약). ←/→는 패널이 여러 항목 사이 이동에 직접 쓴다.
//! - 외부 crate 0: 패널은 `objc_getClass("QLPreviewPanel")`(런타임 · 없으면 `None` → 호출부가 종전 `qlmanage` 폴백) · 메서드 추가 = `class_addMethod`.
//! - 메인 스레드 전용(`MainThreadMarker`) · `dataSource`/`delegate`는 `assign`이라 우리 객체를 **영구 보유**(`Retained::into_raw` · 종료 때 `orderOut:` 먼저).

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool, Sel};
use objc2::{declare_class, msg_send, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{NSEvent, NSEventType};
use objc2_foundation::{MainThreadMarker, NSInteger, NSObject, NSObjectProtocol, NSString, NSURL};
use std::ffi::{c_char, c_void, CStr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[link(name = "Quartz", kind = "framework")]
extern "C" {}

#[link(name = "objc")]
extern "C" {
    fn object_getClass(obj: *mut c_void) -> *mut c_void;
    fn class_addMethod(
        cls: *mut c_void,
        sel: Sel,
        imp: unsafe extern "C" fn(),
        types: *const c_char,
    ) -> i8;
}

/// 패널이 보여 줄 경로(데이터소스가 읽는다 · 메인 스레드).
static ITEMS: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
/// 패널 안 키(델리게이트가 쌓고 앱 틱이 거둔다).
static KEYS: Mutex<Vec<QlKey>> = Mutex::new(Vec::new());
/// 메인 창 NSView(응답자 메서드가 "우리 창인가"를 판정) · 0 = 미설치.
static MAIN_VIEW: AtomicUsize = AtomicUsize::new(0);
/// 데이터소스 객체(영구 보유 · 포인터).
static SOURCE: AtomicUsize = AtomicUsize::new(0);

/// 패널 안에서 눌린 키(앱이 커서를 옮기거나 닫는다).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QlKey {
    Up,
    Down,
    /// Space = Finder처럼 패널 닫기(토글).
    Toggle,
}

/// 키 코드(순수 · 가상 키코드 `kVK_*`): 126 = ↑ · 125 = ↓ · 49 = Space · 그 밖 = 패널 몫(None).
pub(crate) fn key_action(key_code: u16) -> Option<QlKey> {
    match key_code {
        126 => Some(QlKey::Up),
        125 => Some(QlKey::Down),
        49 => Some(QlKey::Toggle),
        _ => None,
    }
}

declare_class!(
    /// `QLPreviewPanelDataSource` + `QLPreviewPanelDelegate`(비공식 프로토콜 · 메서드만 있으면 된다).
    struct QlSource;

    unsafe impl ClassType for QlSource {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "NexaDirQLSource";
    }

    impl DeclaredClass for QlSource {
        type Ivars = ();
    }

    unsafe impl NSObjectProtocol for QlSource {}

    unsafe impl QlSource {
        #[method(numberOfPreviewItemsInPreviewPanel:)]
        unsafe fn count(&self, _panel: *mut AnyObject) -> NSInteger {
            ITEMS.lock().map_or(0, |v| v.len() as NSInteger)
        }

        #[method_id(previewPanel:previewItemAtIndex:)]
        unsafe fn item(&self, _panel: *mut AnyObject, index: NSInteger) -> Option<Retained<NSURL>> {
            // `#[method_id]`는 꼬리 식만 변환한다(`return` 불가) → 식 하나로.
            ITEMS.lock().ok().and_then(|items| {
                usize::try_from(index)
                    .ok()
                    .and_then(|i| items.get(i).cloned())
                    .map(|p| NSURL::fileURLWithPath(&NSString::from_str(&p.to_string_lossy())))
            })
        }

        #[method(previewPanel:handleEvent:)]
        unsafe fn handle_event(&self, _panel: *mut AnyObject, event: &NSEvent) -> Bool {
            if event.r#type() != NSEventType::KeyDown {
                return Bool::NO;
            }
            match key_action(event.keyCode()) {
                Some(k) => {
                    if let Ok(mut q) = KEYS.lock() {
                        q.push(k);
                    }
                    Bool::YES
                }
                None => Bool::NO,
            }
        }
    }
);

unsafe extern "C" fn accepts_imp(this: *mut AnyObject, _sel: Sel, _panel: *mut AnyObject) -> Bool {
    Bool::new(this as usize == MAIN_VIEW.load(Ordering::Relaxed))
}

unsafe extern "C" fn begin_imp(_this: *mut AnyObject, _sel: Sel, panel: *mut AnyObject) {
    let src = SOURCE.load(Ordering::Relaxed) as *mut AnyObject;
    if !panel.is_null() && !src.is_null() {
        let _: () = msg_send![panel, setDataSource: src];
        let _: () = msg_send![panel, setDelegate: src];
    }
}

unsafe extern "C" fn end_imp(_this: *mut AnyObject, _sel: Sel, panel: *mut AnyObject) {
    if !panel.is_null() {
        let nil: *mut AnyObject = std::ptr::null_mut();
        let _: () = msg_send![panel, setDataSource: nil];
        let _: () = msg_send![panel, setDelegate: nil];
    }
}

type ImpBool = unsafe extern "C" fn(*mut AnyObject, Sel, *mut AnyObject) -> Bool;
type ImpVoid = unsafe extern "C" fn(*mut AnyObject, Sel, *mut AnyObject);
type Imp = unsafe extern "C" fn();

/// IMP 포인터 꼴로(`class_addMethod` 인자) — 호출 규약은 ObjC 런타임이 인코딩으로 맞춘다.
fn imp_bool(f: ImpBool) -> Imp {
    // SAFETY: 함수 포인터 간 변환(크기 같음) · 런타임이 인코딩대로 부른다.
    unsafe { std::mem::transmute::<ImpBool, Imp>(f) }
}
fn imp_void(f: ImpVoid) -> Imp {
    // SAFETY: 위와 같다.
    unsafe { std::mem::transmute::<ImpVoid, Imp>(f) }
}

/// 클래스(인스턴스의)에 메서드를 더한다 — 그 클래스 자체에 이미 있으면 NO(상위 클래스는 안 본다 · 협업 함정 ①).
unsafe fn add(cls: *mut c_void, name: &CStr, imp: Imp, types: &CStr) {
    let sel = Sel::register(name.to_str().unwrap_or("x"));
    let _ = class_addMethod(cls, sel, imp, types.as_ptr());
}

/// 패널 클래스(없으면 None = 폴백 · Quartz 미적재).
fn panel_class() -> Option<&'static AnyClass> {
    AnyClass::get("QLPreviewPanel")
}

/// 공유 패널(없으면 None).
unsafe fn panel() -> Option<*mut AnyObject> {
    let cls = panel_class()?;
    let p: *mut AnyObject = msg_send![cls, sharedPreviewPanel];
    (!p.is_null()).then_some(p)
}

/// 쓸 수 있는가(Quartz의 QLPreviewPanel 클래스가 있고 설치가 끝났다).
pub(crate) fn available() -> bool {
    panel_class().is_some() && SOURCE.load(Ordering::Relaxed) != 0
}

/// 설치(창 생성 뒤 한 번 · 메인 스레드): 데이터소스 객체 생성 · winit 뷰(+창) 클래스에 응답자 메서드 3개. 두 번째부터는 무시.
pub(crate) fn install(ns_view: *mut c_void) -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    if ns_view.is_null() || panel_class().is_none() {
        return false;
    }
    if SOURCE.load(Ordering::Relaxed) != 0 {
        return true;
    }
    MAIN_VIEW.store(ns_view as usize, Ordering::Relaxed);
    let this = mtm.alloc::<QlSource>().set_ivars(());
    // SAFETY: NSObject init · 영구 보유(패널의 assign 참조가 끊기지 않게).
    let src: Retained<QlSource> = unsafe { msg_send_id![super(this), init] };
    SOURCE.store(Retained::into_raw(src) as usize, Ordering::Relaxed);
    // SAFETY: 살아 있는 뷰 · 셀렉터·인코딩은 정적 · IMP는 위 extern "C" 함수.
    unsafe {
        let accepts_types: &CStr = if cfg!(target_arch = "aarch64") {
            c"B@:@"
        } else {
            c"c@:@"
        };
        let view_cls = object_getClass(ns_view);
        let window: *mut AnyObject = msg_send![ns_view as *mut AnyObject, window];
        let window_cls = if window.is_null() {
            std::ptr::null_mut()
        } else {
            object_getClass(window as *mut c_void)
        };
        let targets = [view_cls, window_cls];
        for cls in targets.into_iter().filter(|c| !c.is_null()) {
            add(
                cls,
                c"acceptsPreviewPanelControl:",
                imp_bool(accepts_imp),
                accepts_types,
            );
            add(
                cls,
                c"beginPreviewPanelControl:",
                imp_void(begin_imp),
                c"v@:@",
            );
            add(cls, c"endPreviewPanelControl:", imp_void(end_imp), c"v@:@");
        }
    }
    true
}

/// 패널이 보이는가.
pub(crate) fn is_visible() -> bool {
    // SAFETY: 메인 스레드 · 공유 패널 조회.
    unsafe {
        match panel() {
            Some(p) => {
                let v: Bool = msg_send![p, isVisible];
                v.as_bool()
            }
            None => false,
        }
    }
}

/// 경로를 싣고 패널을 연다(이미 보이면 내용만 갱신). 실패 = false(호출부 폴백).
pub(crate) fn show(paths: &[PathBuf]) -> bool {
    if paths.is_empty() || MainThreadMarker::new().is_none() {
        return false;
    }
    set_items(paths);
    // SAFETY: 메인 스레드 · 공유 패널.
    unsafe {
        let Some(p) = panel() else {
            return false;
        };
        let nil: *mut AnyObject = std::ptr::null_mut();
        let _: () = msg_send![p, makeKeyAndOrderFront: nil];
        let _: () = msg_send![p, reloadData];
    }
    true
}

/// 선택 추종: 패널이 보이면 경로를 바꾸고 다시 읽는다(비어 있으면 닫는다).
pub(crate) fn follow(paths: &[PathBuf]) {
    if !is_visible() {
        return;
    }
    if paths.is_empty() {
        hide();
        return;
    }
    set_items(paths);
    // SAFETY: 메인 스레드 · 공유 패널.
    unsafe {
        if let Some(p) = panel() {
            let _: () = msg_send![p, reloadData];
            let _: () = msg_send![p, setCurrentPreviewItemIndex: 0 as NSInteger];
        }
    }
}

/// 패널 닫기.
pub(crate) fn hide() {
    // SAFETY: 메인 스레드 · 공유 패널.
    unsafe {
        if let Some(p) = panel() {
            let nil: *mut AnyObject = std::ptr::null_mut();
            let _: () = msg_send![p, orderOut: nil];
        }
    }
}

/// 패널 안에서 눌린 키를 거둔다(틱).
pub(crate) fn take_keys() -> Vec<QlKey> {
    KEYS.lock()
        .map(|mut q| std::mem::take(&mut *q))
        .unwrap_or_default()
}

fn set_items(paths: &[PathBuf]) {
    if let Ok(mut v) = ITEMS.lock() {
        *v = paths.to_vec();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 패널 키 분류(순수): ↑/↓ = 커서 · Space = 토글 · 그 밖(←→·Esc)은 패널 몫.
    #[test]
    fn key_action_maps_arrows_and_space() {
        assert_eq!(key_action(126), Some(QlKey::Up));
        assert_eq!(key_action(125), Some(QlKey::Down));
        assert_eq!(key_action(49), Some(QlKey::Toggle));
        assert_eq!(key_action(123), None, "← = 패널");
        assert_eq!(key_action(53), None, "Esc = 패널");
    }
}
