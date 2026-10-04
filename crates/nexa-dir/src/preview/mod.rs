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

thread_local! {
    /// SVG 텍스트 글꼴(앱 UI 글꼴 — `App::new`가 주입 · 없으면 텍스트 없이 그린다).
    static SVG_FONT: std::cell::RefCell<Option<std::rc::Rc<nexa_gfx::Font>>> = const { std::cell::RefCell::new(None) };
}

/// `render_svg`가 쓸 글꼴 주입(UI 스레드).
pub(crate) fn set_svg_font(font: std::rc::Rc<nexa_gfx::Font>) {
    SVG_FONT.with(|f| *f.borrow_mut() = Some(font));
}

/// SVG → 이미지 파일(플러그인 `render_svg` · dir2 PLUG-011/114 · T-62 C-2): nexa-gfx `svg` **3-OS CPU 래스터**(dir2는 GDI+) ·
/// viewBox 크기 그대로(한 변 ≤ 2000) · 잉크/배경 = 현재 테마(dark 흰 글/어두운 배경) · 결과 = `<temp>/nexa-preview/d<해시>.bmp`
/// (32bpp top-down · 내용 해시 이름 = 같은 그림은 다시 쓰지 않는다 · 도크/창은 `draw_image_hint` 캐시로 읽는다).
pub(crate) fn render_svg_impl(svg: &str) -> Option<String> {
    if svg.len() > 256 * 1024 {
        return None;
    }
    let doc = nexa_gfx::svg::parse(svg)?;
    let dark = is_dark_now();
    let opts = nexa_gfx::svg::RenderOpts {
        ink: if dark {
            nexa_gfx::Color::from_rgb(0xE6, 0xE6, 0xE6)
        } else {
            nexa_gfx::Color::from_rgb(0x20, 0x20, 0x20)
        },
        bg: if dark {
            nexa_gfx::Color::from_rgb(0x1E, 0x1E, 0x1E)
        } else {
            nexa_gfx::Color::from_rgb(0xFF, 0xFF, 0xFF)
        },
    };
    use std::hash::{Hash, Hasher};
    let mut hsh = std::collections::hash_map::DefaultHasher::new();
    svg.hash(&mut hsh);
    dark.hash(&mut hsh);
    let dir = std::env::temp_dir().join("nexa-preview");
    let path = dir.join(format!("d{:016x}.bmp", hsh.finish()));
    if path.exists() {
        return Some(path.to_string_lossy().into_owned());
    }
    let img = SVG_FONT.with(|f| {
        let f = f.borrow();
        nexa_gfx::svg::render_natural(&doc, opts, f.as_deref(), 2000)
    })?;
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(&path, bmp_bytes(&img)).ok()?;
    Some(path.to_string_lossy().into_owned())
}

/// RGBA → 32bpp top-down BMP(BGRA · 알파 255 · nexa-gfx `image::decode`가 읽는 형식).
pub(crate) fn bmp_bytes(img: &nexa_gfx::IconImage) -> Vec<u8> {
    let (w, h) = (img.w as i32, img.h as i32);
    let n = img.rgba.len();
    let mut f = Vec::with_capacity(54 + n);
    f.extend_from_slice(b"BM");
    f.extend_from_slice(&(54 + n as u32).to_le_bytes());
    f.extend_from_slice(&[0u8; 4]);
    f.extend_from_slice(&54u32.to_le_bytes());
    f.extend_from_slice(&40u32.to_le_bytes());
    f.extend_from_slice(&w.to_le_bytes());
    f.extend_from_slice(&(-h).to_le_bytes()); // top-down
    f.extend_from_slice(&1u16.to_le_bytes());
    f.extend_from_slice(&32u16.to_le_bytes());
    f.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    f.extend_from_slice(&(n as u32).to_le_bytes());
    f.extend_from_slice(&[0u8; 16]);
    for p in img.rgba.as_chunks::<4>().0 {
        f.extend_from_slice(&[p[2], p[1], p[0], 255]);
    }
    f
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
    /// 로드한 파일(사용자 설치분 = 사용자 폴더 안 · 매니저 삭제 대상).
    pub path: PathBuf,
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
    if let Some(d) = USER_DIR.with(|u| u.borrow().clone()) {
        // 시험 재지정 — 그 폴더 + 동봉 폴더만(사용자 설정 폴더는 보지 않는다).
        let mut v = vec![d];
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                v.push(exe_dir.join("plugins"));
            }
        }
        v.push(PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../plugins"
        )));
        return v;
    }
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

thread_local! {
    /// 공급자 캐시(스레드 로컬 · 최초 사용 시 지연 구성 · [`invalidate`]로 재구성 — EXT-409 무재시작 · T-63).
    static PROVIDERS: std::cell::RefCell<Option<Cache>> = const { std::cell::RefCell::new(None) };
    /// 사용자 플러그인 폴더 재지정(시험 — 실제 설정 폴더를 건드리지 않는다).
    static USER_DIR: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// 현재 공급자 전체로 콜백 실행.
fn with_providers<R>(
    f: impl FnOnce(&[Box<dyn PreviewProvider>], &[PluginInfo], &[String]) -> R,
) -> R {
    PROVIDERS.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(build_cache());
        }
        let g = c.borrow();
        match g.as_ref() {
            Some((providers, infos, notes)) => f(providers, infos, notes),
            None => f(&[], &[], &[]),
        }
    })
}

/// 올라와 있는 플러그인 모듈의 어림 바이트(메모리 창) = 로드한 `.wasm` 파일 크기 합. 공급자 캐시가 아직 없으면 0 —
/// 재려고 플러그인을 올리지 않는다(처음 미리보기를 쓸 때 늘어나는 것이 보인다).
pub(crate) fn loaded_plugin_bytes() -> u64 {
    PROVIDERS.with(|c| {
        c.borrow().as_ref().map_or(0, |(_, infos, _)| {
            infos
                .iter()
                .filter_map(|i| std::fs::metadata(&i.path).ok())
                .map(|m| m.len())
                .sum()
        })
    })
}

/// 공급자 캐시를 버린다(설치/삭제 뒤 — 다음 사용 때 폴더를 다시 읽는다).
pub(crate) fn invalidate() {
    PROVIDERS.with(|c| *c.borrow_mut() = None);
}

/// 사용자 플러그인 폴더(설치 대상 · 탐색 경로 1순위): 시험 재지정 → `NDIR_PLUGINS_DIR` → `<설정 폴더>/plugins`.
pub(crate) fn user_plugin_dir() -> PathBuf {
    if let Some(d) = USER_DIR.with(|u| u.borrow().clone()) {
        return d;
    }
    if let Some(d) = std::env::var_os("NDIR_PLUGINS_DIR") {
        return PathBuf::from(d);
    }
    ndir_settings::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("plugins")
}

/// 시험용 사용자 폴더 재지정(스레드 로컬 — 병렬 시험에 새지 않는다) + 캐시 무효화.
#[cfg(test)]
pub(crate) fn set_user_plugin_dir(dir: Option<PathBuf>) {
    USER_DIR.with(|u| *u.borrow_mut() = dir);
    invalidate();
}

/// 설치 전 검증 — 모듈 로드 + `nx_meta` 호출이 되면 id.
pub(crate) fn validate_plugin(path: &Path) -> Result<String, String> {
    wasm::load_one(path).map(|p| p.id)
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
            path: p.path.clone(),
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

    /// T-62 C-2: `render_svg` = nexa-gfx svg 래스터 → 임시 BMP(해시 이름) · 디코드 크기 = viewBox · 같은 입력 = 같은 경로 · 손상 SVG = None.
    #[test]
    fn render_svg_writes_bmp_and_caches() {
        set_dark(true);
        let svg = r##"<svg viewBox="0 0 40 24" stroke-width="1"><rect x="1" y="1" width="38" height="22" rx="4" fill="#336699"/><text x="20" y="16" font-size="12" text-anchor="middle" fill="#ffffff">ok</text></svg>"##;
        let p1 = render_svg_impl(svg).expect("render");
        assert!(p1.ends_with(".bmp") && Path::new(&p1).is_file(), "{p1}");
        let bytes = std::fs::read(&p1).unwrap();
        let img = nexa_gfx::image::decode(&bytes, 1 << 20).expect("decode");
        assert_eq!((img.w, img.h), (40, 24));
        let i = ((12 * 40 + 5) * 4) as usize;
        assert_eq!(&img.rgba[i..i + 3], &[0x33, 0x66, 0x99], "채움 색");
        assert_eq!(
            render_svg_impl(svg).as_deref(),
            Some(p1.as_str()),
            "같은 입력 = 같은 파일"
        );
        assert!(
            render_svg_impl("<svg><rect/></svg>").is_none(),
            "viewBox 없음 = None(폴백)"
        );
        assert!(
            render_svg_impl(r##"<svg viewBox="0 0 5000 10"><rect width="1" height="1"/></svg>"##)
                .is_none(),
            "상한"
        );
    }

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
