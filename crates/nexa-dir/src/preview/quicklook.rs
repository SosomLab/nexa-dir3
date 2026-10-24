//! macOS Quick Look 썸네일 공급자(사용자 10-18 "미리보기 패널을 맥의 Preview와 연계"): 자체 렌더러가 없는 형식(PDF · 오피스 · iWork · HEIC · PSD ·
//! 동영상 …)을 `qlmanage -t -s 1024 -o <캐시> <파일>`로 PNG 썸네일을 만들어 도크 미리보기에 보인다 — Finder 미리보기 패널과 같은 원천(Quick Look).
//!
//! 생성은 **백그라운드 스레드**(도크 계산 `update_docks`는 UI 스레드 동기 · qlmanage는 수백 ms) · 끝나면 [`poll`]이 참 → 호스트가 도크를 다시
//! 계산한다. 캐시 = 임시 폴더 `nexa-dir-ql/<해시(경로+mtime)>/<이름>.png`(qlmanage는 출력 이름을 `<이름>.png`로 고정) · 실패 = `<이름>.png.fail`
//! 표식(같은 파일을 다시 시도하지 않는다). 다른 OS에서는 공급자를 등록하지 않는다(모듈은 공통 · 순수 함수 시험).

use super::{PreviewDoc, PreviewProvider};
use ndir_i18n::tr;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Quick Look에 맡기는 확장자(자체 공급자 = 이미지 9종 · 압축 · 텍스트 · 플러그인 외) — Finder가 썸네일을 내는 흔한 문서/미디어.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) const EXTS: [&str; 22] = [
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "pages", "numbers", "key", "rtf", "rtfd",
    "heic", "heif", "psd", "ai", "eps", "mov", "mp4", "m4v", "epub", "svg",
];
/// 썸네일 한 변(px) — 도크 미리보기 폭에 넉넉한 값(qlmanage `-s`).
const THUMB_PX: u32 = 1024;

/// 생성 중인 원본(중복 스폰 방지) · 끝남 깃발(호스트 틱이 1회성으로 거둔다).
static PENDING: Mutex<Option<HashSet<PathBuf>>> = Mutex::new(None);
static DONE: AtomicBool = AtomicBool::new(false);

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) struct QuickLook {
    exts: Vec<String>,
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
impl QuickLook {
    pub(crate) fn new() -> Self {
        QuickLook {
            exts: EXTS.iter().map(|s| (*s).to_string()).collect(),
        }
    }
}

/// 캐시 자리(순수): 경로 + 수정 시각(FNV-1a) → `<root>/<해시>/<이름>.png`.
pub(crate) fn thumb_path(cache_root: &Path, path: &Path, mtime_ms: u64) -> PathBuf {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in path.to_string_lossy().bytes().chain(mtime_ms.to_le_bytes()) {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    let name = path
        .file_name()
        .map_or_else(|| "file".to_string(), |n| n.to_string_lossy().into_owned());
    cache_root
        .join(format!("{h:016x}"))
        .join(format!("{name}.png"))
}

pub(crate) fn cache_root() -> PathBuf {
    std::env::temp_dir().join("nexa-dir-ql")
}

fn mtime_ms(p: &Path) -> u64 {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as u64)
}

/// 썸네일이 하나라도 끝났는가(호스트 틱 · 1회성) → 도크를 다시 계산한다.
pub(crate) fn poll() -> bool {
    DONE.swap(false, Ordering::AcqRel)
}

impl PreviewProvider for QuickLook {
    fn id(&self) -> &str {
        "builtin.quicklook"
    }
    fn exts(&self) -> &[String] {
        &self.exts
    }
    fn preview(&self, path: &Path) -> PreviewDoc {
        let thumb = thumb_path(&cache_root(), path, mtime_ms(path));
        if thumb.is_file() {
            return PreviewDoc::Image(thumb.to_string_lossy().into_owned());
        }
        if thumb.with_extension("png.fail").is_file() {
            return PreviewDoc::Lines(vec![tr("preview.ql.none")]);
        }
        let mut pend = PENDING.lock().unwrap_or_else(|e| e.into_inner());
        if pend
            .get_or_insert_with(HashSet::new)
            .insert(path.to_path_buf())
        {
            let src = path.to_path_buf();
            let out_dir = thumb.parent().map(Path::to_path_buf).unwrap_or_default();
            let fail = thumb.with_extension("png.fail");
            std::thread::spawn(move || {
                let _ = std::fs::create_dir_all(&out_dir);
                let _ = std::process::Command::new("qlmanage")
                    .args(["-t", "-s", &THUMB_PX.to_string(), "-o"])
                    .arg(&out_dir)
                    .arg(&src)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
                if !thumb.is_file() {
                    let _ = std::fs::write(&fail, b"");
                }
                if let Ok(mut p) = PENDING.lock() {
                    if let Some(s) = p.as_mut() {
                        s.remove(&src);
                    }
                }
                DONE.store(true, Ordering::Release);
            });
        }
        PreviewDoc::Lines(vec![tr("preview.ql.generating")])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 캐시 자리: 같은 경로·시각 = 같은 자리 · 시각이 바뀌면 다른 폴더 · 파일 이름은 `<이름>.png`(qlmanage 출력 규약) · 끝남 깃발은 1회성.
    #[test]
    fn thumb_path_is_stable_and_named_after_source() {
        let root = Path::new("/tmp/ql");
        let a = thumb_path(root, Path::new("/docs/보고서.pdf"), 1000);
        assert_eq!(a, thumb_path(root, Path::new("/docs/보고서.pdf"), 1000));
        assert_ne!(
            a,
            thumb_path(root, Path::new("/docs/보고서.pdf"), 1001),
            "수정 시각 = 새 썸네일"
        );
        assert_eq!(a.file_name().unwrap().to_str().unwrap(), "보고서.pdf.png");
        assert_eq!(a.parent().unwrap().parent().unwrap(), root);
        assert!(!poll());
        DONE.store(true, Ordering::Release);
        assert!(poll() && !poll(), "1회성");
    }
}
