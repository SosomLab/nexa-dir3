//! 패널 행 아이콘(GAP-003 · dir2 M1 "셸 아이콘(LRU)" · PANEL-064 · RENDER-040): nexa-grid `Adapt::draw_icon`이 부르는 **리졸버**.
//!
//! 키(`filelist::icon_key`) → nexa-fs [`IconService`](OS 셸 아이콘 · 워커 스레드 · 캐시 · 타 OS는 폴백) 조회:
//! `dir` = 폴더 종류 · `file` = 확장자 없는 파일 · 확장자 = 그 종류 · 그 밖(경로) = 파일별 아이콘(exe·lnk·ico… · 드라이브).
//! 조회 중(`Pending`)이거나 OS가 아이콘을 안 주면 **자체 그림**(nexa-ctl `fallback_file_icon`)을 돌려준다 — 칸이 비지 않는다.
//! 이미지는 스레드 로컬에 `Rc`로 캐시하고(매 프레임 RGBA 복사 금지) 서비스 `version()`이 바뀌면 비운다. 조회 중이 있었으면
//! [`App::row_icons_tick`]이 버전 변화를 보고 다시 그린다(150 ms 폴링 · 런처 아이콘과 같은 규약).
//! `L|` 접두(타일 보기) = 큰 아이콘.
//!
//! ★ **아이콘 계층**(사용자 10-03 — 위가 이긴다):
//! 1. **직접 설정한 아이콘** — 설정 `list.icon_overrides`(규칙 `종류:패턴=이미지 파일` · `;` 구분). 종류 = `path`(전체 경로) ·
//!    `name`(파일/폴더 이름) · `dir`(폴더 이름) · `ext`(확장자). 이미지 = PNG·BMP·GIF·JPEG(nexa-gfx 디코더 · 상대 경로는 설정 폴더 `icons/` 기준).
//! 2. **파일에 등록된 아이콘** — 파일마다 다른 것(exe·lnk·ico·cur·msi·scr · 드라이브) = 셸에 그 경로를 묻는다.
//! 3. **확장자에 등록된 아이콘** — 연결 프로그램이 등록한 종류 아이콘(확장자 단위 캐시).
//! 4. **시스템 제공 아이콘** — 폴더 · 확장자 없는 파일의 OS 기본 아이콘.
//! 5. 자체 그림(조회 중 · OS가 안 줄 때 · 다른 OS의 폴백).

use crate::*;
use nexa_fs::shell::{IconKey, IconService, Lookup};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

const ICON_POLL_MS: u64 = 150;
/// 캐시 상한(dir2 `CAPACITY 256`의 여유분 — 넘으면 통째로 비운다 · 다음 프레임에 서비스 캐시에서 다시 채워진다).
const CACHE_MAX: usize = 512;

/// 사용자 지정 아이콘 규칙 1건(계층 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IconRule {
    pub kind: RuleKind,
    /// 소문자 패턴(경로는 구분자 `/`로 통일 · 끝 구분자 제거).
    pub pattern: String,
    /// 이미지 파일 절대 경로.
    pub image: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuleKind {
    Path,
    Name,
    Dir,
    Ext,
}

fn norm_path(p: &str) -> String {
    p.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

/// 설정 문자열 → 규칙 목록(순수). 모르는 종류·빈 패턴·빈 이미지는 버린다. 상대 이미지 경로는 `base`(설정 폴더의 `icons/`) 기준.
pub(crate) fn parse_overrides(spec: &str, base: &std::path::Path) -> Vec<IconRule> {
    spec.split(';')
        .filter_map(|entry| {
            let (lhs, image) = entry.trim().split_once('=')?;
            let (kind, pattern) = lhs.trim().split_once(':')?;
            let kind = match kind.trim().to_ascii_lowercase().as_str() {
                "path" => RuleKind::Path,
                "name" => RuleKind::Name,
                "dir" => RuleKind::Dir,
                "ext" => RuleKind::Ext,
                _ => return None,
            };
            let (pattern, image) = (pattern.trim(), image.trim());
            if pattern.is_empty() || image.is_empty() {
                return None;
            }
            let pattern = match kind {
                RuleKind::Path => norm_path(pattern),
                RuleKind::Ext => pattern.trim_start_matches('.').to_lowercase(),
                _ => pattern.to_lowercase(),
            };
            let img = std::path::Path::new(image);
            let image = if img.is_absolute() {
                img.to_path_buf()
            } else {
                base.join(img)
            };
            Some(IconRule {
                kind,
                pattern,
                image,
            })
        })
        .collect()
}

/// 항목에 맞는 규칙(순수) — 우선순위 `path` > `name` > (`dir` 폴더 | `ext` 파일). 같은 종류끼리는 먼저 적은 것.
pub(crate) fn match_override<'a>(
    rules: &'a [IconRule],
    path: &str,
    is_dir: bool,
) -> Option<&'a IconRule> {
    if rules.is_empty() {
        return None;
    }
    let full = norm_path(path);
    let name = full.rsplit('/').next().unwrap_or("").to_string();
    let ext = if is_dir {
        ""
    } else {
        name.rsplit_once('.')
            .map_or("", |(stem, e)| if stem.is_empty() { "" } else { e })
    };
    let find = |kind: RuleKind, pat: &str| {
        (!pat.is_empty())
            .then(|| rules.iter().find(|r| r.kind == kind && r.pattern == pat))
            .flatten()
    };
    find(RuleKind::Path, &full)
        .or_else(|| find(RuleKind::Name, &name))
        .or_else(|| {
            if is_dir {
                find(RuleKind::Dir, &name)
            } else {
                find(RuleKind::Ext, ext)
            }
        })
}

thread_local! {
    static RULES: RefCell<Vec<IconRule>> = const { RefCell::new(Vec::new()) };
}

/// 사용자 지정 규칙 설치(기동 · 설정 `list.icon_overrides` 변경 시).
pub(crate) fn set_overrides(rules: Vec<IconRule>) {
    RULES.with(|r| *r.borrow_mut() = rules);
}

/// 계층 1: 사용자 지정 이미지(디코드·캐시는 nexa-ctl `image_cache` — 파일이 바뀌면 다시 읽는다 · 못 읽으면 다음 계층으로).
fn user_icon(hint: &str, is_dir: bool) -> Option<Rc<nexa_gfx::IconImage>> {
    let image = RULES.with(|r| {
        let r = r.borrow();
        match_override(&r, hint, is_dir).map(|rule| rule.image.clone())
    })?;
    nexa_ctl::raster::image_cache::get(&image.to_string_lossy())
}

thread_local! {
    static CACHE: RefCell<(u64, HashMap<String, Rc<nexa_gfx::IconImage>>)> = RefCell::new((0, HashMap::new()));
    static PENDING: Cell<bool> = const { Cell::new(false) };
    static FALLBACK: RefCell<[Option<Rc<nexa_gfx::IconImage>>; 2]> = const { RefCell::new([None, None]) };
}

/// 경로별 아이콘에 링크 화살표를 붙이는가: **실제 링크**(.lnk · 심볼릭 링크 · 정션)만. 드라이브 루트 · exe · ico 같은 경로별
/// 아이콘에는 붙이지 않는다(사용자 10-06 "내 PC의 드라이브에 바로가기 화살표" — `SHGFI_LINKOVERLAY`는 링크 여부를 보지 않고
/// 늘 붙인다).
pub(crate) fn link_overlay_for(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("lnk"))
        || ndir_ops::is_link(path)
}

/// 키 → 서비스 조회 키(순수). `(IconKey, 큰 아이콘인가, 폴더인가)`.
pub(crate) fn service_key(key: &str, hint: &str) -> (IconKey, bool, bool) {
    let (large, key) = match key.strip_prefix("L|") {
        Some(k) => (true, k),
        None => (false, key),
    };
    let is_dir_hint = || std::path::Path::new(hint).is_dir();
    match key {
        "dir" => (
            IconKey::Kind {
                ext: String::new(),
                is_dir: true,
            },
            large,
            true,
        ),
        "file" => (
            IconKey::Kind {
                ext: String::new(),
                is_dir: false,
            },
            large,
            false,
        ),
        k if k.contains(['\\', '/', ':']) => {
            let path = PathBuf::from(hint);
            let key = if link_overlay_for(&path) {
                IconKey::Path(path)
            } else {
                IconKey::PathPlain(path)
            };
            (key, large, is_dir_hint())
        }
        ext => (
            IconKey::Kind {
                ext: ext.to_string(),
                is_dir: false,
            },
            large,
            false,
        ),
    }
}

fn fallback(is_dir: bool) -> Rc<nexa_gfx::IconImage> {
    FALLBACK.with(|f| {
        f.borrow_mut()[usize::from(is_dir)]
            .get_or_insert_with(|| Rc::new(nexa_ctl::controls::fallback_file_icon(is_dir)))
            .clone()
    })
}

thread_local! {
    /// 테마 아이콘 파일 → 디코드한 이미지(실패도 기억 · 파일 수는 테마의 아이콘 종류 수로 묶인다).
    static THEME_IMAGES: RefCell<HashMap<PathBuf, Option<Rc<nexa_gfx::IconImage>>>> = RefCell::new(HashMap::new());
}

/// 계층 2~4의 Linux 판: 아이콘 테마(폴더 · 홈 · 다운로드 같은 사용자 폴더 · 파일 종류)의 PNG. 조회는 nexa-fs가 캐시하고
/// (폴더와 확장자 있는 파일은 디스크를 건드리지 않는다) 디코드는 파일마다 한 번.
fn theme_icon(key: &str, hint: &str, size: i32) -> Option<Rc<nexa_gfx::IconImage>> {
    let bare = key.strip_prefix("L|").unwrap_or(key);
    let path = std::path::Path::new(hint);
    let is_dir = bare == "dir" || (bare.contains(['\\', '/', ':']) && path.is_dir());
    let file = nexa_fs::icontheme::icon_file(path, is_dir, size.max(1) as u32)?;
    THEME_IMAGES.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > CACHE_MAX {
            c.clear();
        }
        c.entry(file)
            .or_insert_with_key(|f| {
                std::fs::read(f)
                    .ok()
                    .and_then(|b| nexa_gfx::image::decode(&b, 1024 * 1024).ok())
                    .map(Rc::new)
            })
            .clone()
    })
}

/// 행 아이콘 캐시가 쥔 픽셀 바이트(메모리 창 · 가로 × 세로 × 4 — 셸/테마/상태 아이콘 합).
pub(crate) fn cache_bytes() -> u64 {
    let px = |img: &nexa_gfx::IconImage| u64::from(img.w) * u64::from(img.h) * 4;
    let shell: u64 = CACHE.with(|c| c.borrow().1.values().map(|i| px(i)).sum());
    let theme: u64 = THEME_IMAGES.with(|c| c.borrow().values().flatten().map(|i| px(i)).sum());
    let status: u64 = STATUS_IMAGES.with(|c| c.borrow().values().flatten().map(|i| px(i)).sum());
    shell + theme + status
}

/// 상태 아이콘 캐시: (이름, px) → 색 입힌 이미지(실패도 기억).
type StatusImages = HashMap<(String, i32), Option<Rc<nexa_gfx::IconImage>>>;

thread_local! {
    static STATUS_IMAGES: RefCell<StatusImages> = RefCell::new(HashMap::new());
}

/// 상태 아이콘 색(RGB): 온라인 전용 = 파랑 · 로컬에 있음/항상 유지 = 초록 · 네트워크 = 회색(탐색기 상태 열 색).
pub(crate) fn status_color(name: &str) -> Option<(u8, u8, u8)> {
    Some(match name {
        "cloud" => (0x3B, 0x9B, 0xF4),
        "local" | "pinned" => (0x2E, 0xA0, 0x43),
        "network" => (0x8A, 0x91, 0x9C),
        _ => return None,
    })
}

/// 상태 아이콘: `status-<이름>.svg` 마스크에 색을 입힌 `size`×`size` 그림("항상 유지"는 채운 원 위에 흰 체크).
fn status_icon(name: &str, size: i32) -> Option<Rc<nexa_gfx::IconImage>> {
    STATUS_IMAGES.with(|c| {
        c.borrow_mut()
            .entry((name.to_string(), size))
            .or_insert_with(|| {
                let (r, g, b) = status_color(name)?;
                let px = size.clamp(8, 256) as u32;
                let (w, h, alpha) = crate::icons::toolbar_mask(&format!("status-{name}"), px)?;
                let mut rgba = Vec::with_capacity((w * h * 4) as usize);
                for a in alpha {
                    rgba.extend_from_slice(&[r, g, b, *a]);
                }
                if name == "pinned" {
                    // 채운 원 위에 흰 체크 = "로컬에 있음" 마스크의 체크 선만 겹친다(원 테두리는 같은 자리라 묻힌다).
                    if let Some((_, _, check)) = crate::icons::toolbar_mask("status-check", px) {
                        for (i, a) in check.iter().enumerate() {
                            let k = f32::from(*a) / 255.0;
                            for ch in 0..3 {
                                let v = f32::from(rgba[i * 4 + ch]);
                                rgba[i * 4 + ch] = (v + (255.0 - v) * k).round() as u8;
                            }
                        }
                    }
                }
                Some(Rc::new(nexa_gfx::IconImage { w, h, rgba }))
            })
            .clone()
    })
}

/// 리졸버 본체 — 항상 이미지를 준다(셸 아이콘 · 없으면 자체 그림).
pub(crate) fn resolve(key: &str, hint: &str, size: i32) -> Option<Rc<nexa_gfx::IconImage>> {
    // 상태 열 아이콘(`status:<이름>` — 파일 아이콘 계층과 무관한 색 입힌 SVG 마스크).
    if let Some(name) = key.strip_prefix("status:") {
        return status_icon(name, size);
    }
    // 계층 1 — 직접 설정한 아이콘(규칙이 없으면 비용 0).
    if RULES.with(|r| !r.borrow().is_empty()) {
        let bare = key.strip_prefix("L|").unwrap_or(key);
        let is_dir = bare == "dir"
            || (bare.contains(['\\', '/', ':']) && std::path::Path::new(hint).is_dir());
        if let Some(img) = user_icon(hint, is_dir) {
            return Some(img);
        }
    }
    // Linux = 아이콘 테마의 그림 파일(nexa-ui 124차 `nexa_fs::icontheme` — 다른 OS는 즉시 None · 테마에 없으면 아래 계층으로).
    if let Some(img) = theme_icon(key, hint, size) {
        return Some(img);
    }
    let svc = IconService::global();
    let ver = svc.version();
    let hit = CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.0 != ver || c.1.len() > CACHE_MAX {
            c.0 = ver;
            c.1.clear();
        }
        c.1.get(key).cloned()
    });
    if hit.is_some() {
        return hit;
    }
    let (ikey, large, is_dir) = service_key(key, hint);
    match svc.icon(&ikey, large) {
        Lookup::Ready(Some(icon)) => {
            let img = Rc::new(nexa_gfx::IconImage {
                w: icon.w,
                h: icon.h,
                rgba: icon.rgba.clone(),
            });
            CACHE.with(|c| c.borrow_mut().1.insert(key.to_string(), img.clone()));
            Some(img)
        }
        Lookup::Ready(None) => {
            let img = fallback(is_dir);
            CACHE.with(|c| c.borrow_mut().1.insert(key.to_string(), img.clone()));
            Some(img)
        }
        Lookup::Pending => {
            PENDING.with(|p| p.set(true));
            Some(fallback(is_dir))
        }
    }
}

/// 기동 때 1회 — nexa-grid에 리졸버 등록.
pub(crate) fn install() {
    // 링크(.lnk · 심볼릭 링크 · 정션) 아이콘에 탐색기처럼 화살표 오버레이(GAP-008) — 첫 조회 전에 켠다.
    nexa_fs::shell::set_link_overlay(true);
    nexa_grid::draw::set_icon_resolver(Some(Rc::new(resolve)));
}

impl App {
    /// 설정 `list.icon_overrides` → 규칙 설치(상대 경로 기준 = 설정 폴더 `icons/`).
    pub(crate) fn apply_icon_overrides(&mut self) {
        let base = ndir_settings::config_dir()
            .unwrap_or_default()
            .join("icons");
        let spec = self.settings.get("list.icon_overrides").unwrap_or("");
        set_overrides(parse_overrides(spec, &base));
        self.redraw();
    }

    /// 조회 중이던 행 아이콘이 도착했는지(서비스 버전 변화) 보고 다시 그린다 — 남았으면 다음 폴링 시각.
    pub(crate) fn row_icons_tick(&mut self, now: Instant) -> Option<Instant> {
        if !PENDING.with(Cell::get) {
            return None;
        }
        let v = IconService::global().version();
        if v != self.row_icon_ver {
            self.row_icon_ver = v;
            PENDING.with(|p| p.set(false)); // 다음 그리기가 아직 조회 중인 것이 있으면 다시 켠다
            self.redraw();
        }
        Some(now + Duration::from_millis(ICON_POLL_MS))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 키 해석: dir/file/확장자 = 종류 · 경로 = 파일별 · `L|` = 큰 아이콘.
    /// Linux 아이콘 테마: 테마가 설치돼 있으면 폴더·파일 행이 테마 PNG(디코드 성공 · 정사각)를 받는다 — 폴더와 파일은 다른 그림.
    /// 테마가 없는 환경(최소 CI 이미지)에서는 None이라 자체 그림으로 내려간다(실패 아님).
    #[cfg(target_os = "linux")]
    #[test]
    fn linux_rows_use_icon_theme_images() {
        let Some(theme) = nexa_fs::icontheme::theme_name() else {
            eprintln!("icon theme 없음 — 건너뜀");
            return;
        };
        let dir = theme_icon("dir", "/usr", 16);
        let file = theme_icon("txt", "/tmp/nexa-dir-no-such.txt", 16);
        eprintln!(
            "theme {theme} · dir {:?} · file {:?}",
            dir.as_ref().map(|i| (i.w, i.h)),
            file.as_ref().map(|i| (i.w, i.h))
        );
        for img in [&dir, &file].into_iter().flatten() {
            assert!(img.w == img.h && img.w >= 16);
            assert_eq!(img.rgba.len(), (img.w * img.h * 4) as usize);
        }
        if let (Some(d), Some(f)) = (&dir, &file) {
            assert_ne!(d.rgba, f.rgba);
            // 같은 파일은 디코드한 이미지를 공유한다.
            let again = theme_icon("dir", "/usr", 16).expect("cached");
            assert!(Rc::ptr_eq(d, &again));
        }
        // 리졸버 전체 경로도 같은 그림을 준다.
        if let Some(d) = &dir {
            assert!(Rc::ptr_eq(d, &resolve("dir", "/usr", 16).expect("resolve")));
        }
    }

    #[test]
    fn service_key_maps_dir2_icon_keys() {
        assert_eq!(
            service_key("dir", "C:/x"),
            (
                IconKey::Kind {
                    ext: String::new(),
                    is_dir: true
                },
                false,
                true
            )
        );
        assert_eq!(
            service_key("file", "C:/x/README").0,
            IconKey::Kind {
                ext: String::new(),
                is_dir: false
            }
        );
        assert_eq!(
            service_key("L|txt", "C:/x/a.txt"),
            (
                IconKey::Kind {
                    ext: "txt".into(),
                    is_dir: false
                },
                true,
                false
            )
        );
        let (k, large, _) = service_key("c:/x/app.exe", "C:/x/App.exe");
        assert_eq!(
            k,
            IconKey::PathPlain(PathBuf::from("C:/x/App.exe")),
            "exe = 화살표 없이"
        );
        assert!(!large);
        // 링크 화살표는 실제 링크에만(.lnk · 정션) — 드라이브 루트는 아니다(사용자 10-06).
        let (k, _, _) = service_key("c:/x/go.lnk", "C:/x/Go.lnk");
        assert_eq!(k, IconKey::Path(PathBuf::from("C:/x/Go.lnk")));
        assert!(link_overlay_for(std::path::Path::new("C:/x/Go.LNK")));
        assert!(!link_overlay_for(std::path::Path::new("C:/")));
        assert!(!link_overlay_for(std::path::Path::new("C:/x/App.exe")));
        let (k, _, _) = service_key("c:/", "C:/");
        assert_eq!(k, IconKey::PathPlain(PathBuf::from("C:/")));
    }

    /// 계층 1 규칙: 문법 · 우선순위(path > name > dir/ext) · 상대 경로 기준 · 잘못된 항목 무시.
    #[test]
    fn override_rules_parse_and_match_by_priority() {
        let base = std::path::Path::new("/cfg/icons");
        let rules = parse_overrides(
            "ext:.RS=rust.png; name:Cargo.toml=cargo.png ; dir:node_modules=nm.png;path:C:\\Work\\Proj\\=/abs/proj.png; bogus:x=y; ext:=z; name:a",
            base,
        );
        assert_eq!(rules.len(), 4, "{rules:?}");
        assert_eq!(rules[0].pattern, "rs");
        assert_eq!(rules[0].image, base.join("rust.png"));
        assert_eq!(rules[3].pattern, "c:/work/proj");
        let img = |p: &str, d: bool| match_override(&rules, p, d).map(|r| r.image.clone());
        assert_eq!(img("C:\\x\\main.RS", false), Some(base.join("rust.png")));
        assert_eq!(
            img("C:/x/Cargo.toml", false),
            Some(base.join("cargo.png")),
            "name > ext"
        );
        assert_eq!(img("C:/x/node_modules", true), Some(base.join("nm.png")));
        assert_eq!(img("C:/x/node_modules", false), None, "dir 규칙은 폴더만");
        assert_eq!(
            img("c:/work/proj/", true),
            Some(PathBuf::from("/abs/proj.png")),
            "path가 최우선"
        );
        assert_eq!(img("C:/x/.gitignore", false), None, "점 파일은 확장자 없음");
        assert_eq!(img("C:/x/readme", false), None);
        assert!(match_override(&[], "C:/x/a.rs", false).is_none());
    }

    /// 리졸버는 항상 이미지를 준다(셸 아이콘이 아직 없거나 OS가 안 줘도 자체 그림) — 칸이 비지 않는다.
    #[test]
    fn resolver_always_yields_an_image() {
        let d = resolve("dir", "", 16).expect("folder icon");
        let f = resolve("zzznoext", "", 16).expect("file icon");
        assert!(d.w > 0 && f.w > 0);
        assert_eq!(d.rgba.len(), (d.w * d.h * 4) as usize);
    }
}
