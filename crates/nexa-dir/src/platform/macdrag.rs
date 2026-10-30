//! **macOS OS 드래그 발신 세션**(`NSDraggingSession` · T-52/T-147 macOS · 10-31 · 협업 분석 하이브리드(C)안):
//! 창 안에서는 앱의 창 안 드래그(`dnd_internal` · 폴더 행 강조 · 탭 머묾)를 그대로 쓰고, 포인터가 **창 밖으로 나가는 첫 포인터 사건**
//! 처리 안에서 `NSView beginDraggingSessionWithItems:event:source:`를 시작한다 — AppKit이 요구하는 트리거 NSEvent(mouseDragged)는
//! 그 순간의 `NSApp.currentEvent`다(틱·타이머에서 시작하면 안 된다 → 마우스 사건이 아니면 fail-soft로 시작하지 않는다).
//! 시작 뒤에는 AppKit이 포인터를 추적한다(우리 뷰는 mouseDragged/mouseUp을 못 받는다 · winit의 관찰자/타이머는 공통 모드라 틱은 돈다) ·
//! 우리 창으로 돌아오면 winit `HoveredFile`/`DroppedFile`(기존 수신 경로) · 밖에서 놓으면 소스의 `draggingSession:endedAtPoint:operation:`이
//! 결과를 **원자값**에 적고 [`Session::tick`]이 거둔다(아키텍처 규약 "공유 상태 폴링" · 콜백에서 앱을 만지지 않는다).
//! 아이템 = 파일 URL(`NSURL` = NSPasteboardWriting → Finder · 다른 앱이 파일로 받는다 · 가상 파일 `NSFilePromiseProvider`는 2차) ·
//! 그림 = 앱이 그린 [`DragImage`](0x00RRGGBB · 물리 px) → NSImage(점 단위 = px/배율) · 첫 아이템에 붙인다.
//! 수식키 = `ignoreModifierKeysForDraggingSession` NO → Finder 규칙(Option 복사 · Cmd 이동)을 받는 쪽이 적용(8d929f8과 일치) ·
//! 소스 마스크 = Copy | Move | Generic. 우클릭 드래그("묻기")는 macOS에 없어 일반 드래그.
//! 실기 검증 거리(입력 주입 금지): Finder로 끌어 놓기 · Option/Cmd · 우리 창 복귀 · 그림 위치/방향.

use super::*;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{declare_class, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{
    NSApplication, NSBitmapImageRep, NSDragOperation, NSDraggingContext, NSDraggingItem,
    NSDraggingSession, NSDraggingSource, NSEventType, NSImage, NSPasteboardWriting, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSArray, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString, NSURL,
};
use std::ffi::c_void;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// 세션 결과(원자값): 0 = 진행 중 · 1 = 취소/없음 · 2 = 복사(Copy · Generic · Link · 그 밖) · 3 = 이동.
const PENDING: u8 = 0;
const CANCELLED: u8 = 1;
const COPIED: u8 = 2;
const MOVED: u8 = 3;

/// `NSDragOperation` → 결과 부호(순수 · MC/DC 시험).
pub(crate) fn outcome_code(op: usize) -> u8 {
    const MOVE: usize = 16;
    if op == 0 {
        CANCELLED
    } else if op & MOVE != 0 && op & !MOVE == 0 {
        MOVED
    } else {
        COPIED
    }
}

/// 결과 부호 → [`DragOutcome`](진행 중 = None).
pub(crate) fn outcome_of(code: u8) -> Option<DragOutcome> {
    match code {
        PENDING => None,
        MOVED => Some(DragOutcome::Moved),
        COPIED => Some(DragOutcome::Copied),
        _ => Some(DragOutcome::Cancelled),
    }
}

/// 0x00RRGGBB(물리 px) → RGBA(알파 255 · 그림은 배경까지 채워져 있다).
pub(crate) fn rgba_of(pixels: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() * 4);
    for p in pixels {
        out.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8, 255]);
    }
    out
}

struct SourceIvars {
    done: Arc<AtomicU8>,
}

declare_class!(
    /// `NSDraggingSource` — 결과만 원자값에 적는다(앱 상태는 틱이 본다).
    struct DragSource;

    unsafe impl ClassType for DragSource {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "NexaDirDragSource";
    }

    impl DeclaredClass for DragSource {
        type Ivars = SourceIvars;
    }

    unsafe impl NSObjectProtocol for DragSource {}

    unsafe impl NSDraggingSource for DragSource {
        #[method(draggingSession:sourceOperationMaskForDraggingContext:)]
        unsafe fn source_mask(
            &self,
            _session: &NSDraggingSession,
            _context: NSDraggingContext,
        ) -> NSDragOperation {
            NSDragOperation::Copy | NSDragOperation::Move | NSDragOperation::Generic
        }

        #[method(draggingSession:endedAtPoint:operation:)]
        unsafe fn ended(
            &self,
            _session: &NSDraggingSession,
            _point: NSPoint,
            operation: NSDragOperation,
        ) {
            self.ivars()
                .done
                .store(outcome_code(operation.0), Ordering::Release);
        }

        #[method(ignoreModifierKeysForDraggingSession:)]
        unsafe fn ignore_modifier_keys(&self, _session: &NSDraggingSession) -> bool {
            false
        }
    }
);

impl DragSource {
    fn new(mtm: MainThreadMarker, done: Arc<AtomicU8>) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(SourceIvars { done });
        // SAFETY: NSObject init.
        unsafe { objc2::msg_send_id![super(this), init] }
    }
}

/// 발신 세션 — [`super::XdndSession`]의 macOS 구현(Linux `xdnd::Session`과 같은 메서드 집합 · 이동/수식키는 AppKit이 하므로 빈 동작).
pub(crate) struct Session {
    _source: Retained<DragSource>,
    _session: Retained<NSDraggingSession>,
    done: Arc<AtomicU8>,
    started: Instant,
}

/// DragImage → NSImage(점 단위 크기 = px/배율).
fn image_of(img: &DragImage, scale: f64) -> Option<Retained<NSImage>> {
    let (w, h) = (usize::from(img.w), usize::from(img.h));
    if w == 0 || h == 0 || img.pixels.len() != w * h {
        return None;
    }
    let rgba = rgba_of(&img.pixels);
    // SAFETY: AppKit 생성자 · rep 버퍼 = w*h*4(bitmapData 널 검사 뒤 복사) · icon.rs `image_from_rgba`와 같은 길.
    unsafe {
        let rep: Option<Retained<NSBitmapImageRep>> = objc2::msg_send_id![
            NSBitmapImageRep::alloc(),
            initWithBitmapDataPlanes: std::ptr::null_mut::<*mut u8>(),
            pixelsWide: w as isize,
            pixelsHigh: h as isize,
            bitsPerSample: 8_isize,
            samplesPerPixel: 4_isize,
            hasAlpha: true,
            isPlanar: false,
            colorSpaceName: &*NSString::from_str("NSDeviceRGBColorSpace"),
            bytesPerRow: (w * 4) as isize,
            bitsPerPixel: 32_isize,
        ];
        let rep = rep?;
        let data = rep.bitmapData();
        if data.is_null() {
            return None;
        }
        std::ptr::copy_nonoverlapping(rgba.as_ptr(), data, rgba.len());
        let size = NSSize::new(w as f64 / scale, h as f64 / scale);
        let image = NSImage::initWithSize(NSImage::alloc(), size);
        rep.setSize(size);
        image.addRepresentation(&rep);
        Some(image)
    }
}

impl Session {
    /// 시작(메인 스레드 · 지금 처리 중인 사건이 mouseDragged일 때만). `ns_view` = winit 창의 NSView(`winfocus::ns_view`) ·
    /// `scale` = 창 배율(DragImage px → 점).
    pub(crate) fn start(
        ns_view: *mut c_void,
        paths: &[PathBuf],
        image: Option<DragImage>,
        scale: f64,
    ) -> Result<Session, String> {
        if paths.is_empty() {
            return Err("empty drag".into());
        }
        let mtm = MainThreadMarker::new().ok_or("not on the main thread")?;
        if ns_view.is_null() {
            return Err("no NSView".into());
        }
        // SAFETY: AppKit 호출은 전부 메인 스레드 · 뷰는 winit 창이 살아 있는 동안 유효(호출자가 창 참조를 쥔다) · 반환 객체는 Retained.
        unsafe {
            let app = NSApplication::sharedApplication(mtm);
            let event = app.currentEvent().ok_or("no current event")?;
            let ty = event.r#type();
            if ty != NSEventType::LeftMouseDragged && ty != NSEventType::RightMouseDragged {
                return Err(format!("current event is not a drag ({})", ty.0));
            }
            let view: &NSView = &*ns_view.cast::<NSView>();
            // 포인터 자리(뷰 좌표 · winit 뷰는 flipped = 왼쪽 위 원점).
            let at = view.convertPoint_fromView(event.locationInWindow(), None);
            let scale = if scale.is_finite() && scale > 0.0 {
                scale
            } else {
                1.0
            };
            let picture = image.as_ref().and_then(|i| image_of(i, scale));
            let (pw, ph) = image.as_ref().map_or((0.0, 0.0), |i| {
                (f64::from(i.w) / scale, f64::from(i.h) / scale)
            });
            let mut items: Vec<Retained<NSDraggingItem>> = Vec::with_capacity(paths.len());
            for (i, p) in paths.iter().enumerate() {
                let url = NSURL::fileURLWithPath(&NSString::from_str(&p.to_string_lossy()));
                let writer: &ProtocolObject<dyn NSPasteboardWriting> =
                    ProtocolObject::from_ref(&*url);
                let item =
                    NSDraggingItem::initWithPasteboardWriter(NSDraggingItem::alloc(), writer);
                // 그림은 첫 아이템에 · 포인터가 그림 왼쪽 위에서 조금 안쪽(Finder 꼴).
                let frame = NSRect::new(
                    NSPoint::new(at.x - 8.0, at.y - 8.0),
                    NSSize::new(pw.max(1.0), ph.max(1.0)),
                );
                let contents = if i == 0 {
                    picture
                        .as_deref()
                        .map(|im| im as &objc2::runtime::AnyObject)
                } else {
                    None
                };
                item.setDraggingFrame_contents(frame, contents);
                items.push(item);
            }
            let array = NSArray::from_vec(items);
            let done = Arc::new(AtomicU8::new(PENDING));
            let source = DragSource::new(mtm, Arc::clone(&done));
            let session = view.beginDraggingSessionWithItems_event_source(
                &array,
                &event,
                ProtocolObject::from_ref(&*source),
            );
            Ok(Session {
                _source: source,
                _session: session,
                done,
                started: Instant::now(),
            })
        }
    }

    pub(crate) fn motion(&mut self, _x: i16, _y: i16, _ctrl: bool, _shift: bool) {}
    pub(crate) fn modifiers(&mut self, _ctrl: bool, _shift: bool) {}
    pub(crate) fn set_image(&mut self, _image: DragImage) {}
    /// AppKit이 추적 중이라 앱이 제안 동작을 알 수 없다(받는 쪽이 정한다) → None(이름만).
    pub(crate) fn external_action(&self) -> Option<DropChoice> {
        None
    }
    /// 세션을 연 순간부터 "놓음 대기" 상태(창 안 드래그는 이미 접었다).
    pub(crate) fn is_dropped(&self) -> bool {
        true
    }
    pub(crate) fn external_accepts(&self) -> bool {
        true
    }
    pub(crate) fn release(&mut self) -> bool {
        true
    }
    pub(crate) fn cancel(&mut self) {
        let _ = self
            .done
            .compare_exchange(PENDING, CANCELLED, Ordering::AcqRel, Ordering::Relaxed);
    }
    /// 결과(끝났으면 한 번) — 60초가 지나도 안 끝나면 취소로 본다(세션 객체가 사라진 경우의 복구).
    pub(crate) fn tick(&mut self) -> Option<DragOutcome> {
        let out = outcome_of(self.done.load(Ordering::Acquire));
        if out.is_none() && self.started.elapsed().as_secs() >= 60 {
            return Some(DragOutcome::Cancelled);
        }
        out
    }
    pub(crate) fn waiting(&self) -> bool {
        self.done.load(Ordering::Acquire) == PENDING
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 결과 부호(MC/DC): None → 취소 · Move만 → 이동 · Copy/Generic/Link/Move+Copy → 복사 · 부호 → DragOutcome.
    #[test]
    fn drag_operation_maps_to_outcome() {
        assert_eq!(outcome_code(0), CANCELLED);
        assert_eq!(outcome_code(16), MOVED);
        assert_eq!(outcome_code(1), COPIED);
        assert_eq!(outcome_code(4), COPIED);
        assert_eq!(outcome_code(2), COPIED);
        assert_eq!(outcome_code(17), COPIED);
        assert_eq!(outcome_of(PENDING), None);
        assert_eq!(outcome_of(MOVED), Some(DragOutcome::Moved));
        assert_eq!(outcome_of(COPIED), Some(DragOutcome::Copied));
        assert_eq!(outcome_of(CANCELLED), Some(DragOutcome::Cancelled));
        assert_eq!(
            rgba_of(&[0x0011_2233, 0]),
            vec![0x11, 0x22, 0x33, 255, 0, 0, 0, 255]
        );
    }

    /// 시작 조건: 빈 경로 · 널 뷰 · 현재 사건이 드래그가 아님(시험 스레드 = 메인 스레드가 아니거나 사건 없음) → Err(죽지 않음).
    #[test]
    fn start_fails_soft_without_drag_event() {
        assert!(Session::start(std::ptr::null_mut(), &[], None, 2.0).is_err());
        assert!(Session::start(std::ptr::null_mut(), &[PathBuf::from("/tmp")], None, 2.0).is_err());
    }
}
