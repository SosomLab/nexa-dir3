//! 글자 드롭(T-147 잔여 · 10-31 사용자 결정 ⑥ · 1차 = macOS `platform::mactextdrop` · 규칙은 3-OS 공통으로 둔다):
//! 놓인 글자가 **존재하는 경로**(한 줄 · 따옴표 벗김 · `file://` · `~`)면 폴더 = 그 폴더로 이동 · 파일 = 부모로 이동 뒤 선택 ·
//! 그 밖(여러 줄 · 없는 경로 · URL)은 놓은 자리가 터미널 도크면 입력(붙여넣기와 같은 줄바꿈 규칙) · 아니면 무시(상태줄 안내).

use crate::*;
use std::path::Path;

/// 글자 분류(순수).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DropText {
    Folder(PathBuf),
    File(PathBuf),
    Text(String),
}

/// 글자 → 분류(순수 · `probe` = 경로의 종류: `Some(true)` 폴더 · `Some(false)` 파일 · `None` 없음): 한 줄만 경로 후보 · 양끝 공백/따옴표 제거 ·
/// `file://`(퍼센트 복원) · `~`/환경 변수 전개.
pub(crate) fn classify_drop_text(text: &str, probe: &dyn Fn(&Path) -> Option<bool>) -> DropText {
    let t = text.trim();
    if t.is_empty() || t.contains('\n') || t.contains('\r') {
        return DropText::Text(text.to_string());
    }
    let unq = t
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| t.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(t);
    let candidate: PathBuf = if unq.starts_with("file://") {
        match crate::pathexpand::file_uri_path(unq) {
            Some(p) => p,
            None => return DropText::Text(text.to_string()),
        }
    } else if unq.contains("://") {
        return DropText::Text(text.to_string());
    } else {
        PathBuf::from(crate::pathexpand::expand_native(unq, None))
    };
    match probe(&candidate) {
        Some(true) => DropText::Folder(candidate),
        Some(false) => DropText::File(candidate),
        None => DropText::Text(text.to_string()),
    }
}

/// 실제 파일 시스템 조사.
pub(crate) fn fs_probe(p: &Path) -> Option<bool> {
    std::fs::metadata(p).ok().map(|m| m.is_dir())
}

impl App {
    /// 글자 드롭 적용(창 안 물리 px): 경로 = 놓은 자리 패널(없으면 활성)로 이동/선택 · 글 = 그 자리 터미널 도크(없으면 포커스 터미널)에 입력 ·
    /// 그 밖 = 상태줄 `dnd.cannot`. 돌려주는 값 = 무엇을 했나(시험).
    pub(crate) fn apply_text_drop(&mut self, text: &str, at: Point) -> DropText {
        let kind = classify_drop_text(text, &fs_probe);
        let mut inv = Invalidations::default();
        let panel = if self.dual && self.panels[1].bounds().contains(at) {
            1
        } else if self.panels[0].bounds().contains(at) {
            0
        } else {
            self.active
        };
        match &kind {
            DropText::Folder(p) => {
                if panel != self.active {
                    self.set_active(panel);
                }
                self.panels[panel].navigate_to(p.clone(), &mut inv);
                self.update_status();
                self.redraw();
            }
            DropText::File(p) => {
                if panel != self.active {
                    self.set_active(panel);
                }
                if let Some(parent) = p.parent() {
                    self.panels[panel].navigate_to(parent.to_path_buf(), &mut inv);
                }
                self.panels[panel].select_path(p, &mut inv);
                self.update_status();
                self.redraw();
            }
            DropText::Text(s) => {
                let dock =
                    (0..2).find(|&i| self.term_shown(i) && self.docks[i].bounds().contains(at));
                match dock.or_else(|| self.term_focused()) {
                    Some(i) => {
                        self.term_write_text(i, s);
                        self.redraw();
                    }
                    None => self.note_status_text(&tr("dnd.cannot")),
                }
            }
        }
        kind
    }

    /// 유휴 틱: 플랫폼 큐의 글자 드롭을 적용한다(macOS) — 처리했으면 true.
    pub(crate) fn text_drop_tick(&mut self) -> bool {
        #[cfg(target_os = "macos")]
        {
            let drops = platform::mactextdrop::take();
            if drops.is_empty() {
                return false;
            }
            for d in drops {
                self.apply_text_drop(&d.text, Point { x: d.x, y: d.y });
            }
            return true;
        }
        #[allow(unreachable_code)]
        false
    }

    fn note_status_text(&mut self, text: &str) {
        let mut inv = Invalidations::default();
        self.statusbar.set_left(text, &mut inv);
        self.redraw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 분류(순수): 폴더/파일/따옴표/`file://` 퍼센트/`~`/여러 줄/없는 경로/URL.
    #[test]
    fn classify_paths_and_text() {
        let probe = |p: &Path| -> Option<bool> {
            let s = p.to_string_lossy();
            if s.ends_with("/dir") || s == "/dir" {
                Some(true)
            } else if s.ends_with("/f.txt") || s.ends_with("/a b.txt") {
                Some(false)
            } else {
                None
            }
        };
        assert_eq!(
            classify_drop_text("  /dir  ", &probe),
            DropText::Folder(PathBuf::from("/dir"))
        );
        assert_eq!(
            classify_drop_text("\"/x/f.txt\"", &probe),
            DropText::File(PathBuf::from("/x/f.txt")),
            "따옴표 벗김"
        );
        assert_eq!(
            classify_drop_text("file:///x/a%20b.txt", &probe),
            DropText::File(PathBuf::from("/x/a b.txt")),
            "file:// 퍼센트 복원"
        );
        assert_eq!(
            classify_drop_text("/dir\n/x/f.txt", &probe),
            DropText::Text("/dir\n/x/f.txt".into()),
            "여러 줄 = 글"
        );
        assert_eq!(
            classify_drop_text("/nope", &probe),
            DropText::Text("/nope".into()),
            "없는 경로 = 글"
        );
        assert_eq!(
            classify_drop_text("https://example.com/dir", &probe),
            DropText::Text("https://example.com/dir".into()),
            "URL = 글"
        );
        assert_eq!(
            classify_drop_text("   ", &probe),
            DropText::Text("   ".into())
        );
        let home = std::env::var("HOME").unwrap_or_default();
        let home_probe = |p: &Path| -> Option<bool> { (p == Path::new(&home)).then_some(true) };
        if !home.is_empty() {
            assert_eq!(
                classify_drop_text("~", &home_probe),
                DropText::Folder(PathBuf::from(&home)),
                "~ 전개"
            );
        }
    }
}
