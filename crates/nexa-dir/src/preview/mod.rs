//! 미리보기 공급자 시임(dir2 `preview/mod.rs` 이식 — docs/port/20 §1.1 PLUG-001~013 · DR-7).
//! 내장 공급자(archive → image → text) + 확장자→공급자 레지스트리. 우선순위(dir2 07-26):
//! **설정 `preview.map` 오버라이드 > 플러그인/내장 선언 매치(로드 순) > 내장 텍스트 폴백**.
//!
//! WASM 런타임(`wasm.rs` · wasmi 1.1 · dir2 ABI 바이트 호환)이 내장 앞에 합류한다. 탐색 경로는
//! [`plugin_dirs`](EXT-401 변형 — 환경변수 `NDIR_PLUGINS_DIR` · 설정 폴더 `plugins/` · 동봉 `<exe>/plugins`).
//! 라인 태그 계약(`\u{2}종류|` · `\u{1}img|`)은 런타임 중립 자산.

// F3 창·압축 그리드·설정 플러그인 페이지(T-62 B · T-63)가 차례로 소비한다 — 그때까지 미사용 경고를 끈다.
#![allow(dead_code)]

pub(crate) mod archive;
#[cfg(test)]
mod sample_tests;
pub(crate) mod wasm;

use ndir_i18n::tr;
use std::path::{Path, PathBuf};

/// 공급자 산출물 — 도크/독립 창이 해석.
#[derive(Debug)]
pub(crate) enum PreviewDoc {
    /// 텍스트 라인들(종류 태그 `\u{2}`·이미지 마커 `\u{1}` 포함 가능).
    Lines(Vec<String>),
    /// 이미지 경로 — 호스트 디코더 위임(`InfoDock::set_image`).
    Image(String),
    /// 압축 파일 목록(X-46) — 도크 = 요약 텍스트 · 독립 창 = 그리드(T-62 B).
    Archive(Box<archive::ArchiveDoc>),
}

/// 미리보기 공급자 계약(dir2 ADR-0004 S1) — 확장자 선언 + 생성.
pub(crate) trait PreviewProvider {
    /// 안정 식별자(설정 `preview.map`·사용 여부 키. 내장 = `builtin.*`).
    fn id(&self) -> &str;
    /// 선언 확장자(소문자·점 없음) — 내부 기본값. 외부 재정의는 `preview.map`.
    fn exts(&self) -> &[String];
    fn preview(&self, path: &Path) -> PreviewDoc;
}

/// 선언 이미지 확장자(dir2 WIC 9종 그대로 — 디코드 가능 여부는 표시측 `nexa_gfx::image` · 미지원은 표시 생략).
const IMAGE_EXTS: [&str; 9] = [
    "png", "jpg", "jpeg", "bmp", "gif", "ico", "tif", "tiff", "webp",
];

const TEXT_READ_CAP: usize = 16 * 1024;
const TEXT_LINE_CAP: usize = 200;

/// 상한까지 읽어 (내용, 이진 여부) — 공급자·호스트 API 공용. `Err` = 열기 실패.
pub(crate) fn read_text(path: &Path, cap: usize) -> Result<(String, bool), ()> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).map_err(|_| ())?;
    let mut buf = vec![0u8; cap];
    let n = f.read(&mut buf).unwrap_or(0);
    buf.truncate(n);
    let binary = buf[..n.min(1024)].contains(&0);
    Ok((String::from_utf8_lossy(&buf).into_owned(), binary))
}

thread_local! {
    /// 현재 테마 다크 여부(호스트 주입 — 플러그인 `is_dark()`).
    static DARK: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// 테마 신호 주입(호스트 — 미리보기 전 호출).
pub(crate) fn set_dark(dark: bool) {
    DARK.with(|d| d.set(dark));
}

pub(crate) fn is_dark_now() -> bool {
    DARK.with(|d| d.get())
}

/// 문자 표시 폭 합(콘솔 셀 — CJK/이모지 2칸. 표·다이어그램 정렬용 호스트 API).
pub(crate) fn disp_width_impl(s: &str) -> usize {
    s.chars()
        .map(|c| {
            let u = c as u32;
            if matches!(
                u,
                0x1100..=0x115F | 0x2E80..=0x303E | 0x3041..=0x33FF | 0x3400..=0x4DBF
                    | 0x4E00..=0x9FFF | 0xA000..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF
                    | 0xFE30..=0xFE4F | 0xFF00..=0xFF60 | 0xFFE0..=0xFFE6
                    | 0x1F300..=0x1FAFF | 0x20000..=0x3FFFD
            ) {
                2
            } else {
                1
            }
        })
        .sum()
}

/// SVG → 이미지 캐시(플러그인 `render_svg`). dir2는 GDI+ 래스터(Windows 전용)였다 — dir3는 3-OS 공통
/// CPU 래스터(nexa-gfx svg · docs/port/20 §3 "추가 필요")가 T-62 B. 그때까지 `None` = Mermaid 3단 폴백(아트·원문).
pub(crate) fn render_svg_impl(svg: &str) -> Option<String> {
    let _ = svg;
    None
}

/// 내장 텍스트 공급자 — 첫 16KB·이진 판정·200줄·탭 4칸.
struct BuiltinText {
    exts: Vec<String>,
}

impl PreviewProvider for BuiltinText {
    fn id(&self) -> &str {
        "builtin.text"
    }
    fn exts(&self) -> &[String] {
        &self.exts // 빈 목록 = 폴백 전용
    }
    fn preview(&self, path: &Path) -> PreviewDoc {
        let Ok((text, binary)) = read_text(path, TEXT_READ_CAP) else {
            return PreviewDoc::Lines(vec![tr("preview.fail")]);
        };
        if text.is_empty() {
            return PreviewDoc::Lines(vec![tr("preview.empty")]);
        }
        if binary {
            return PreviewDoc::Lines(vec![tr("preview.binary")]);
        }
        PreviewDoc::Lines(
            text.lines()
                .take(TEXT_LINE_CAP)
                .map(|l| l.replace('\t', "    "))
                .collect(),
        )
    }
}

/// 내장 이미지 공급자 — 디코드는 표시 백엔드 소관, 경로만 위임.
struct BuiltinImage {
    exts: Vec<String>,
}

impl PreviewProvider for BuiltinImage {
    fn id(&self) -> &str {
        "builtin.image"
    }
    fn exts(&self) -> &[String] {
        &self.exts
    }
    fn preview(&self, path: &Path) -> PreviewDoc {
        PreviewDoc::Image(path.to_string_lossy().into_owned())
    }
}

/// 내장 공급자 목록(로드 순 = 선언 매치 우선순위. 텍스트는 마지막 폴백 전용).
fn builtins() -> Vec<Box<dyn PreviewProvider>> {
    vec![
        // 압축이 먼저 — "구조를 아는 공급자 우선"(포맷 추가 시에도 동일).
        Box::new(archive::BuiltinArchive::new()),
        Box::new(BuiltinImage {
            exts: IMAGE_EXTS.iter().map(|s| s.to_string()).collect(),
        }),
        Box::new(BuiltinText { exts: Vec::new() }),
    ]
}

/// 플러그인 사용 안 함 판정(설정 `plugins.disabled` — 내장은 폴백 안전망이라 면역).
fn is_disabled(id: &str, disabled: &str) -> bool {
    !id.starts_with("builtin.") && disabled.split('|').any(|d| d.trim() == id)
}

/// 공급자 결정 — `preview.map` 오버라이드 > 선언 매치(로드 순) > 텍스트 폴백.
fn resolve<'a>(
    providers: &'a [Box<dyn PreviewProvider>],
    ext: &str,
    preview_map: &str,
    disabled: &str,
) -> &'a dyn PreviewProvider {
    if !ext.is_empty() {
        for pair in preview_map.split('|') {
            if let Some((e, id)) = pair.split_once(':') {
                if e.trim().eq_ignore_ascii_case(ext) && !is_disabled(id.trim(), disabled) {
                    if let Some(p) = providers.iter().find(|p| p.id() == id.trim()) {
                        return p.as_ref();
                    }
                }
            }
        }
    }
    if let Some(p) = providers
        .iter()
        .filter(|p| !is_disabled(p.id(), disabled))
        .find(|p| p.exts().iter().any(|e| e == ext))
    {
        return p.as_ref();
    }
    providers
        .iter()
        .find(|p| p.id() == "builtin.text")
        .expect("builtin.text 폴백은 항상 등재")
        .as_ref()
}

/// 미리보기 생성(시임 진입점) — (문서, 공급자 id).
pub(crate) fn preview_for(path: &Path, preview_map: &str, disabled: &str) -> (PreviewDoc, String) {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    with_providers(|providers, _, _| {
        let p = resolve(providers, &ext, preview_map, disabled);
        (p.preview(path), p.id().to_string())
    })
}

/// 설정 UI용 플러그인 메타(설정 창 "플러그인" 페이지 목록 · T-63).
#[derive(Clone, Debug)]
pub(crate) struct PluginInfo {
    pub id: String,
    pub name: String,
    pub exts: Vec<String>,
}

/// 로드된 플러그인 목록.
pub(crate) fn plugin_infos() -> Vec<PluginInfo> {
    with_providers(|_, infos, _| infos.to_vec())
}

/// 로드 오류(`파일명: 사유`) — dir2는 버렸다(port/20 §6-18) · nexa-sql처럼 표면화(자가 점검·설정 페이지).
pub(crate) fn load_notes() -> Vec<String> {
    with_providers(|_, _, notes| notes.to_vec())
}

/// 플러그인 탐색 경로(우선순위 순 — 앞이 이긴다 · 같은 id는 앞선 것만 채택 · EXT-401 변형):
///
/// 0. `NDIR_PLUGINS_DIR`(환경변수 — 시험·시나리오 러너·포터블 재지정)
/// 1. `<설정 폴더>/plugins`(사용자 드롭인 · 관리 설치본은 하위 폴더 — T-63)
/// 2. 동봉 `<exe 폴더>/plugins` · macOS 번들 `<exe>/../Resources/plugins` · Linux 패키지 `<exe>/../share/nexa-dir/plugins`
pub(crate) fn plugin_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut push = |p: PathBuf| {
        if !dirs.contains(&p) {
            dirs.push(p);
        }
    };
    if let Some(d) = std::env::var_os("NDIR_PLUGINS_DIR") {
        if !d.is_empty() {
            push(PathBuf::from(d));
        }
    }
    if let Some(cfg) = ndir_settings::config_dir() {
        push(cfg.join("plugins"));
    }
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
    {
        push(exe_dir.join("plugins"));
        if cfg!(target_os = "macos") {
            push(exe_dir.join("../Resources/plugins"));
        }
        if cfg!(all(unix, not(target_os = "macos"))) {
            push(exe_dir.join("../share/nexa-dir/plugins"));
        }
    }
    dirs
}

type Cache = (Vec<Box<dyn PreviewProvider>>, Vec<PluginInfo>, Vec<String>);

/// 현재 공급자 전체로 콜백 실행 — 미리보기 최초 사용 시 지연 구성(스레드 로컬 · 재구성은 재시작 — EXT-409는 T-63).
fn with_providers<R>(
    f: impl FnOnce(&[Box<dyn PreviewProvider>], &[PluginInfo], &[String]) -> R,
) -> R {
    thread_local! {
        static PROVIDERS: std::cell::OnceCell<Cache> = const { std::cell::OnceCell::new() };
    }
    PROVIDERS.with(|c| {
        let (providers, infos, notes) = c.get_or_init(build_cache);
        f(providers, infos, notes)
    })
}

/// 경로 우선순위대로 로드하고 **같은 id는 앞선 것만** 채택(동봉분 위에 사용자 설치분이 얹힌다).
fn build_cache() -> Cache {
    let mut plugins: Vec<wasm::WasmPlugin> = Vec::new();
    let mut notes = Vec::new();
    for dir in plugin_dirs() {
        let (found, errors) = wasm::load_dir(&dir);
        notes.extend(errors);
        for p in found {
            if plugins.iter().any(|q| q.id == p.id) {
                continue;
            }
            plugins.push(p);
        }
    }
    let infos: Vec<PluginInfo> = plugins
        .iter()
        .map(|p| PluginInfo {
            id: p.id.clone(),
            name: p.name.clone(),
            exts: p.exts.clone(),
        })
        .collect();
    let mut v: Vec<Box<dyn PreviewProvider>> = plugins
        .into_iter()
        .map(|p| Box::new(wasm::WasmProvider::new(p)) as Box<dyn PreviewProvider>)
        .collect();
    v.extend(builtins());
    (v, infos, notes)
}

/// 도크 축약 뷰 줄(dir2 PLUG-042 · 순수): `\u{2}종류|` 태그 제거 · `hr` → `─`×40 · `q` → `│ ` 접두 · 이미지 마커 줄은 그대로(도크가 그린다).
pub(crate) fn dock_lines(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(
            |l| match l.strip_prefix('\u{2}').and_then(|r| r.split_once('|')) {
                Some(("hr", _)) => "─".repeat(40),
                Some(("q", body)) => format!("│ {body}"),
                Some((_, body)) => body.to_string(),
                None => l.clone(),
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str, bytes: &[u8]) -> PathBuf {
        let p = std::env::temp_dir().join(format!("ndir_prev_{}_{}", std::process::id(), name));
        std::fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn declared_ext_routes_and_text_falls_back() {
        ndir_i18n::activate(ndir_i18n::load("en", Path::new("nowhere")));
        let img = tmp("a.png", b"\x89PNG");
        match preview_for(&img, "", "") {
            (PreviewDoc::Image(p), id) => {
                assert!(p.ends_with("a.png"));
                assert_eq!(id, "builtin.image");
            }
            _ => panic!("이미지 확장자는 이미지 공급자"),
        }
        let txt = tmp("a.rs", "fn main() {}\tok".as_bytes());
        match preview_for(&txt, "", "") {
            (PreviewDoc::Lines(lines), _) => {
                assert_eq!(lines[0], "fn main() {}    ok", "탭 4칸 치환 유지")
            }
            _ => panic!("비매치 확장자는 텍스트 폴백"),
        }
        for p in [img, txt] {
            let _ = std::fs::remove_file(p);
        }
    }

    #[test]
    fn preview_map_overrides_declared_match() {
        ndir_i18n::activate(ndir_i18n::load("en", Path::new("nowhere")));
        let img = tmp("b.png", &[0x89u8, 0x50, 0x00, 0x47]);
        match preview_for(&img, "png:builtin.text", "").0 {
            PreviewDoc::Lines(lines) => assert_eq!(lines.len(), 1, "이진 안내 1줄"),
            _ => panic!("오버라이드가 선언 매치보다 우선해야 함"),
        }
        match preview_for(&img, "png:no.such.plugin", "").0 {
            PreviewDoc::Image(_) => {}
            _ => panic!("무효 id는 무시 — 선언 매치 유지"),
        }
        let _ = std::fs::remove_file(img);
    }

    struct Fake {
        id: &'static str,
        exts: Vec<String>,
    }
    impl PreviewProvider for Fake {
        fn id(&self) -> &str {
            self.id
        }
        fn exts(&self) -> &[String] {
            &self.exts
        }
        fn preview(&self, _path: &Path) -> PreviewDoc {
            PreviewDoc::Lines(vec![self.id.to_string()])
        }
    }

    #[test]
    fn plugin_dirs_are_plugins_folders_without_duplicates() {
        let dirs = plugin_dirs();
        assert!(!dirs.is_empty());
        assert!(
            dirs.iter().all(|d| d.ends_with("plugins")),
            "모든 후보는 plugins 폴더: {dirs:?}"
        );
        let mut uniq = dirs.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(uniq.len(), dirs.len(), "같은 경로 중복 등재 금지");
    }

    #[test]
    fn disabled_plugin_is_skipped_but_builtin_immune() {
        let providers: Vec<Box<dyn PreviewProvider>> = {
            let mut v: Vec<Box<dyn PreviewProvider>> = vec![Box::new(Fake {
                id: "markdown",
                exts: vec!["md".into()],
            })];
            v.extend(builtins());
            v
        };
        assert_eq!(resolve(&providers, "md", "", "").id(), "markdown");
        assert_eq!(
            resolve(&providers, "md", "", "markdown").id(),
            "builtin.text"
        );
        assert_eq!(
            resolve(&providers, "md", "md:markdown", "markdown").id(),
            "builtin.text"
        );
        assert_eq!(
            resolve(&providers, "png", "", "builtin.image|markdown").id(),
            "builtin.image"
        );
    }

    #[test]
    fn dock_lines_strip_tags() {
        let src = vec![
            "\u{2}h1|Title".to_string(),
            "\u{2}hr|".into(),
            "\u{2}q|quote".into(),
            "plain".into(),
            "\u{1}img|x.bmp".into(),
        ];
        assert_eq!(
            dock_lines(&src),
            [
                "Title",
                &"─".repeat(40),
                "│ quote",
                "plain",
                "\u{1}img|x.bmp"
            ]
        );
    }
}
