//! macOS 파일 클립보드(T-52 · docs/port/19 §4-4 "파일 목록 쓰기/읽기/잘라내기 표식" · SHELL-040/041의 AppKit 대응):
//! `NSPasteboard generalPasteboard` — 쓰기 = `clearContents` → `writeObjects(NSURL × n)`(`public.file-url`) + 앱 전용 타입
//! `com.sosomlab.nexa-dir.cut`("1") 동시 게시 + 그때의 `changeCount` 기억 · 읽기 = `readObjectsForClasses(NSURL)` → 파일 URL만 → `path` ·
//! 잘라내기 판정 = cut 타입 존재 **그리고** changeCount 일치(Finder 등이 그 뒤 바꿨으면 복사 취급 — OS에 잘라내기 개념이 없다 · Finder의
//! 잘라내기는 감지 불가). AppKit 호출은 UI 스레드 — 호출부(명령·틱)가 그 스레드다. objc2-app-kit `NSPasteboard` 기능(이미 승인된 의존).

use super::*;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, ProtocolObject};
use objc2::{class, msg_send_id, ClassType as _};
use objc2_app_kit::{NSPasteboard, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSString, NSURL};
use std::cell::Cell;

/// 앱 전용 잘라내기 표식 타입(UTI 꼴).
const CUT_TYPE: &str = "com.sosomlab.nexa-dir.cut";

pub(super) struct PasteboardFiles {
    /// 잘라내기로 게시했을 때의 changeCount(복사 게시 = None).
    cut_change: Cell<Option<isize>>,
}

impl PasteboardFiles {
    pub(super) fn new() -> Self {
        PasteboardFiles {
            cut_change: Cell::new(None),
        }
    }
}

impl FileClipboard for PasteboardFiles {
    fn read_files(&self) -> Option<(Vec<PathBuf>, bool)> {
        // SAFETY: AppKit 호출 — 전역 보드 · 인자는 살아 있는 객체 · 반환은 소유권 규약대로 Retained.
        unsafe {
            let pb = NSPasteboard::generalPasteboard();
            // 클래스 객체 배열(`[NSArray arrayWithObject:NSURL.class]`) — AnyClass는 Retained 배열로 못 담아 메시지로 만든다.
            let cls: &AnyClass = NSURL::class();
            let classes: Retained<NSArray<AnyObject>> =
                msg_send_id![class!(NSArray), arrayWithObject: cls];
            let objs = pb.readObjectsForClasses_options(&classes, None)?;
            let mut paths: Vec<PathBuf> = Vec::new();
            for obj in objs.iter() {
                // 요청한 클래스가 NSURL뿐이므로 원소는 NSURL이다.
                let url: &NSURL = &*(obj as *const AnyObject as *const NSURL);
                if !url.isFileURL() {
                    continue;
                }
                if let Some(p) = url.path() {
                    paths.push(PathBuf::from(p.to_string()));
                }
            }
            if paths.is_empty() {
                return None;
            }
            let cut = pb.stringForType(&NSString::from_str(CUT_TYPE)).is_some()
                && self.cut_change.get() == Some(pb.changeCount());
            Some((paths, cut))
        }
    }

    fn write_files(&self, paths: &[PathBuf], cut: bool) -> Result<(), PlatformError> {
        if paths.is_empty() {
            return Ok(());
        }
        // SAFETY: AppKit 호출 — 비우기 → 파일 URL 객체 쓰기 → 표식 타입 추가(첫 항목) · 모두 UI 스레드.
        unsafe {
            let pb = NSPasteboard::generalPasteboard();
            pb.clearContents();
            let urls: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> = paths
                .iter()
                .map(|p| {
                    ProtocolObject::from_retained(NSURL::fileURLWithPath(&NSString::from_str(
                        &p.to_string_lossy(),
                    )))
                })
                .collect();
            let arr = NSArray::from_vec(urls);
            if !pb.writeObjects(&arr) {
                return Err(PlatformError::Failed("NSPasteboard writeObjects".into()));
            }
            if cut {
                let _ =
                    pb.setString_forType(&NSString::from_str("1"), &NSString::from_str(CUT_TYPE));
            }
            self.cut_change.set(cut.then(|| pb.changeCount()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 실제 보드 왕복(macOS 러너): 2경로 잘라내기 → 같은 경로 + cut · 복사로 다시 쓰면 cut=false · 빈 목록 쓰기 = 무동작.
    #[test]
    fn pasteboard_round_trip_with_cut_marker() {
        let _g = crate::platform::os_test_guard();
        let base = std::env::temp_dir().join(format!("ndir-macclip-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&base);
        let a = base.join("a.txt");
        let b = base.join("b.txt");
        std::fs::write(&a, b"a").unwrap();
        std::fs::write(&b, b"b").unwrap();
        let clip = PasteboardFiles::new();
        clip.write_files(&[a.clone(), b.clone()], true)
            .expect("write");
        let (paths, cut) = clip.read_files().expect("read");
        assert_eq!(paths, vec![a.clone(), b]);
        assert!(cut, "잘라내기 표식 + changeCount 일치");
        clip.write_files(std::slice::from_ref(&a), false)
            .expect("write");
        let (paths, cut) = clip.read_files().expect("read");
        assert_eq!(paths, vec![a]);
        assert!(!cut);
        assert!(clip.write_files(&[], true).is_ok());
        let _ = std::fs::remove_dir_all(&base);
    }
}
