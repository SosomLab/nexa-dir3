//! 하단 도크 내용 공급(dir2 `fileinfo.rs` + `win.rs::dock_info/preview_content` 축약 · docs/port/19 §1-F · port/20 §1.3 · port/14 GUI-112).
//!
//! - **정보**(`info_lines`): 기본 정보 8줄(이름·종류·경로·크기·디스크 할당 크기·만든/수정한/액세스한 날짜) — std 메타데이터만(동기·즉시).
//!   디스크 할당 크기·형식별 상세(Windows 속성 시스템)는 T-5x. 다중 선택 = `info.selected` · 선택 없음 = `info.currentFolder`.
//! - **미리보기**(`preview_content`): 미리보기 시임([`crate::preview`] — 플러그인 > 내장 archive/image/text · `preview.map` · `plugins.disabled`)
//!   결과를 도크 줄로(태그 벗기기 PLUG-042 · 압축 = 요약 60행 · 이미지 = 경로 → `InfoDock::set_image`).
//!
//! 순수 함수 = 시험(임시 트리).

use crate::filelist::{format_size, format_time};
use crate::preview::{self, PreviewDoc};
use ndir_i18n::{tr, trf};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 기본 정보(파일 시스템 메타데이터). 시각은 unix ms.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Basic {
    pub is_dir: bool,
    pub size: Option<u64>,
    pub created: Option<i64>,
    pub modified: Option<i64>,
    pub accessed: Option<i64>,
}

fn unix_ms(t: std::io::Result<SystemTime>) -> Option<i64> {
    let t = t.ok()?;
    let d = t.duration_since(UNIX_EPOCH).ok()?;
    i64::try_from(d.as_millis()).ok()
}

/// 기본 정보 — 실패(접근 불가)면 None. 폴더는 크기 항목 없음(탐색기 동일 — 계산 비용).
pub(crate) fn basic(path: &Path) -> Option<Basic> {
    let md = std::fs::metadata(path).ok()?;
    let is_dir = md.is_dir();
    Some(Basic {
        is_dir,
        size: (!is_dir).then_some(md.len()),
        created: unix_ms(md.created()),
        modified: unix_ms(md.modified()),
        accessed: unix_ms(md.accessed()),
    })
}

/// 천 단위 구분(dir2 `group_thousands`).
pub(crate) fn group_thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// 긴 크기 표기(dir2 `fmt_size_long` · 탐색기 속성 창): `1.5 KB (1,536 bytes)` · 1 KiB 미만은 `512 bytes`.
pub(crate) fn fmt_size_long(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} bytes", group_thousands(bytes))
    } else {
        format!("{} ({} bytes)", format_size(bytes), group_thousands(bytes))
    }
}

/// 종류 라벨(폴더 · 확장자 대문자 · 파일).
fn kind_of(path: &Path, b: &Basic) -> String {
    if b.is_dir {
        return tr("kind.folder");
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) if !ext.is_empty() => ext.to_ascii_uppercase(),
        _ => tr("kind.file"),
    }
}

/// 정보 줄(dir2 `dock_info`): 선택 1 = 기본 정보 8줄 · 여럿 = 개수 · 없음 = 현재 폴더.
pub(crate) fn info_lines(selected: &[PathBuf], current: &Path) -> Vec<String> {
    match selected {
        // 가상 최상위는 내부 표식(`::PC::`)이 아니라 경로 바와 같은 표시명으로(10-03 캡처 검토에서 적발).
        [] => {
            let shown = if ndir_vfs::is_virtual_root(current) {
                ndir_i18n::tr("nav.mypc")
            } else {
                current.display().to_string()
            };
            vec![trf("info.currentFolder", &[&shown])]
        }
        [one] => match basic(one) {
            Some(b) => {
                let name = one
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| one.display().to_string());
                let mut v = vec![
                    trf("info.name", &[&name]),
                    trf("info.kind", &[&kind_of(one, &b)]),
                    trf("info.path", &[&one.display().to_string()]),
                ];
                if let Some(sz) = b.size {
                    v.push(trf("info.size", &[&fmt_size_long(sz)]));
                }
                let t = |ms: Option<i64>| ms.map(format_time).unwrap_or_default();
                v.push(trf("info.created", &[&t(b.created)]));
                v.push(trf("info.modified", &[&t(b.modified)]));
                v.push(trf("info.accessed", &[&t(b.accessed)]));
                v
            }
            None => vec![
                trf("info.name", &[&one.display().to_string()]),
                tr("preview.fail"),
            ],
        },
        many => vec![trf("info.selected", &[&many.len().to_string()])],
    }
}

/// 도크 요약 뷰의 압축 항목 상한(dir2 60).
pub(crate) const ARCHIVE_ROWS: usize = 60;

/// 미리보기 산출(도크용) — 줄 · 이미지 경로 · 공급자 id(덤프·시나리오 검증).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PreviewOut {
    pub lines: Vec<String>,
    pub image: Option<String>,
    pub provider: String,
}

/// 미리보기 내용(dir2 `preview_content` + PLUG-042): 선택 1개 파일만 · 폴더/없음 = `preview.none` ·
/// 시임(`preview.map` 오버라이드 · `plugins.disabled`) → Lines = 태그 벗기기 · Image = 경로 · Archive = 요약.
pub(crate) fn preview_content(
    selected: &[PathBuf],
    preview_map: &str,
    disabled: &str,
) -> PreviewOut {
    let none = || PreviewOut {
        lines: vec![tr("preview.none")],
        ..Default::default()
    };
    let [path] = selected else {
        return none();
    };
    if path.is_dir() {
        return none();
    }
    let (doc, provider) = preview::preview_for(path, preview_map, disabled);
    match doc {
        PreviewDoc::Lines(l) => PreviewOut {
            lines: preview::dock_lines(&l),
            image: None,
            provider,
        },
        PreviewDoc::Image(p) => PreviewOut {
            lines: vec![tr("preview.window.image")],
            image: Some(p),
            provider,
        },
        PreviewDoc::Archive(doc) => PreviewOut {
            lines: preview::archive::summary_lines(&doc, 0, ARCHIVE_ROWS),
            image: None,
            provider,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_formats() {
        assert_eq!(group_thousands(0), "0");
        assert_eq!(group_thousands(1234567), "1,234,567");
        assert_eq!(fmt_size_long(512), "512 bytes");
        assert_eq!(fmt_size_long(1536), "1.5 KB (1,536 bytes)");
    }

    #[test]
    fn info_and_preview_on_temp_tree() {
        ndir_i18n::activate(ndir_i18n::load("en", Path::new("nowhere")));
        let dir = std::env::temp_dir().join(format!("ndir-dockinfo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.txt"), b"hello\nworld").unwrap();
        std::fs::write(dir.join("p.png"), b"\x89PNG").unwrap();
        std::fs::write(dir.join("e.txt"), b"").unwrap();
        std::fs::write(dir.join("b.bin"), b"a\0b").unwrap();
        let a = dir.join("a.txt");
        let lines = info_lines(std::slice::from_ref(&a), &dir);
        assert_eq!(lines[0], "Name: a.txt");
        assert_eq!(lines[1], "Kind: TXT");
        assert!(lines[2].starts_with("Path: "));
        assert_eq!(lines[3], "Size: 11 bytes");
        assert!(lines.iter().any(|l| l.starts_with("Modified: 20")));
        assert_eq!(
            info_lines(&[], &dir)[0],
            format!("Current folder: {}", dir.display())
        );
        assert_eq!(info_lines(&[a, dir.join("sub")], &dir), vec!["2 selected"]);
        assert_eq!(
            info_lines(std::slice::from_ref(&dir.join("sub")), &dir)[1],
            "Kind: Folder"
        );
        let one = |name: &str, map: &str, dis: &str| {
            preview_content(std::slice::from_ref(&dir.join(name)), map, dis)
        };
        let out = one("a.txt", "", "");
        assert_eq!(
            (out.lines, out.image, out.provider.as_str()),
            (
                vec!["hello".to_string(), "world".to_string()],
                None,
                "builtin.text"
            )
        );
        assert_eq!(
            preview_content(&[], "", "").lines,
            vec!["nothing to preview"]
        );
        assert_eq!(one("sub", "", "").lines, vec!["nothing to preview"]);
        let out = one("p.png", "", "");
        assert!(out.image.is_some_and(|p| p.ends_with("p.png")) && out.provider == "builtin.image");
        assert_eq!(one("e.txt", "", "").lines, vec!["(empty file)"]);
        assert_eq!(
            one("b.bin", "", "").lines,
            vec!["binary file \u{2014} no text preview"]
        );
        assert_eq!(one("nope.txt", "", "").lines, vec!["read failed"]);
        // 오버라이드: png를 텍스트로 → 이진 안내 1줄(이미지 없음).
        let out = one("p.png", "png:builtin.text", "");
        assert!(out.image.is_none() && out.provider == "builtin.text");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
