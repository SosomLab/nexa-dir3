//! 임베드 툴바 아이콘(T-30 · dir2 `icons.rs::EMBEDDED_SVG` + `assets/toolbar` 규격 그대로 — 32 viewBox · 콘텐츠 1..31 · stroke 2 · currentColor).
//!
//! dir2는 GDI+ `svg_to_hicon`(Windows 전용)으로 요청 크기에 즉석 래스터했다 — dir3는 nexa-gfx `svg::render_mask`(3-OS CPU)로
//! **알파 마스크**를 만들고 nexa-ctl `ToolIcon::Mask`가 테마 기준색 틴트 · hover/pressed = accent · 비활성 = 흐림을 맡는다
//! (dir2 잉크 색상표 = 기본 검정/흰 · 켜짐 accent 38 % 배경 · 비활성 알파 38 % — 토글 배경은 `set_item_checked`가 그린다).
//! 다크 변형(`-dark.svg`)은 흰 잉크를 하드코딩한 같은 그림이라 쓰지 않는다(마스크 + 테마색이 같은 결과).
//!
//! 마스크는 `(이름, px)`마다 한 번 렌더해 `'static`으로 보관(`ToolIcon::Mask`가 `&'static [u8]`를 요구 — 13종 × 배율 몇 개 · 수 KB).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// 임베드 SVG(이름 · 본문) — 등록 = 한 줄(dir2 규약).
pub(crate) const EMBEDDED_SVG: &[(&str, &str)] = &[
    (
        "panel-toggle",
        include_str!("../assets/toolbar/panel-toggle.svg"),
    ),
    ("dock", include_str!("../assets/toolbar/dock.svg")),
    (
        "always-on-top",
        include_str!("../assets/toolbar/always-on-top.svg"),
    ),
    (
        "info-toggle",
        include_str!("../assets/toolbar/info-toggle.svg"),
    ),
    ("colsync", include_str!("../assets/toolbar/colsync.svg")),
    ("view-tree", include_str!("../assets/toolbar/view-tree.svg")),
    ("view-flat", include_str!("../assets/toolbar/view-flat.svg")),
    (
        "view-tiles",
        include_str!("../assets/toolbar/view-tiles.svg"),
    ),
    ("refresh", include_str!("../assets/toolbar/refresh.svg")),
    ("settings", include_str!("../assets/toolbar/settings.svg")),
    ("hidden", include_str!("../assets/toolbar/hidden.svg")),
    ("dotfiles", include_str!("../assets/toolbar/dotfiles.svg")),
    (
        "folders-first",
        include_str!("../assets/toolbar/folders-first.svg"),
    ),
    (
        "case-sensitive",
        include_str!("../assets/toolbar/case-sensitive.svg"),
    ),
    (
        "natural-sort",
        include_str!("../assets/toolbar/natural-sort.svg"),
    ),
    // 네비 버튼 4종(아이콘 글꼴 Segoe MDL2가 없는 OS용 — MDL2 HomeSolid · Back · Forward · Up 모양 · dir3 신규 10-03).
    ("nav-home", include_str!("../assets/toolbar/nav-home.svg")),
    ("nav-back", include_str!("../assets/toolbar/nav-back.svg")),
    (
        "nav-forward",
        include_str!("../assets/toolbar/nav-forward.svg"),
    ),
    ("nav-up", include_str!("../assets/toolbar/nav-up.svg")),
    // 상태 열 아이콘(온라인 전용 · 로컬에 있음 · 항상 유지 · 네트워크 — dir3 신규 10-03).
    (
        "status-cloud",
        include_str!("../assets/toolbar/status-cloud.svg"),
    ),
    (
        "status-local",
        include_str!("../assets/toolbar/status-local.svg"),
    ),
    (
        "status-pinned",
        include_str!("../assets/toolbar/status-pinned.svg"),
    ),
    (
        "status-check",
        include_str!("../assets/toolbar/status-check.svg"),
    ),
    (
        "status-network",
        include_str!("../assets/toolbar/status-network.svg"),
    ),
    // 도크 미리보기 ↗ "크게"(dir2 07-26) — InfoDock 버튼은 nexa-explorer가 글리프로 그린다 · 등록만.
    ("popout", include_str!("../assets/toolbar/popout.svg")),
    // 로그 창 토글(T-92 · dir3 신규 · 사용자 10-06 "nexa-sql처럼 오른쪽 끝에").
    ("log", include_str!("../assets/toolbar/log.svg")),
];

/// 툴바 명령 id → 자산 이름(dir2 README 매핑표).
pub(crate) fn asset_of(cmd_id: &str) -> Option<&'static str> {
    Some(match cmd_id {
        "view.panel_toggle" => "panel-toggle",
        "view.dock" => "dock",
        "view.always_on_top" => "always-on-top",
        "view.info_toggle" => "info-toggle",
        "view.col_width_sync" => "colsync",
        "view.mode_tree" => "view-tree",
        "view.mode_flat" => "view-flat",
        "view.mode_tiles" => "view-tiles",
        "view.refresh" => "refresh",
        "file.prefs" => "settings",
        "view.hidden" => "hidden",
        "view.dot" => "dotfiles",
        "view.folders_first" => "folders-first",
        "view.case_sensitive" => "case-sensitive",
        "view.natural_sort" => "natural-sort",
        "view.log" => "log",
        _ => return None,
    })
}

type Key = (&'static str, u32);
type Masks = HashMap<Key, Option<&'static [u8]>>;

fn cache() -> &'static Mutex<Masks> {
    static CACHE: OnceLock<Mutex<Masks>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 자산 이름 → `px × px` 알파 마스크(정사각 · 실패 = None → 호출자는 글리프 폴백). 크기별 1회 렌더.
pub(crate) fn toolbar_mask(name: &str, px: u32) -> Option<(u32, u32, &'static [u8])> {
    let (key_name, svg) = EMBEDDED_SVG.iter().find(|(n, _)| *n == name)?;
    let px = px.clamp(8, 256);
    let key = (*key_name, px);
    let Ok(mut c) = cache().lock() else {
        return None;
    };
    if let Some(hit) = c.get(&key) {
        return hit.map(|a| (px, px, a));
    }
    let mask = nexa_gfx::svg::parse(svg).map(|doc| {
        let v: Vec<u8> = nexa_gfx::svg::render_mask(&doc, px, px, None);
        &*Box::leak(v.into_boxed_slice())
    });
    c.insert(key, mask);
    mask.map(|a| (px, px, a))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 임베드 13 + popout 전부 파싱되고 마스크가 비어 있지 않다(잉크 픽셀 ≥ 5 %) · 같은 키 = 같은 포인터(캐시).
    #[test]
    fn all_embedded_icons_render_masks() {
        for (name, svg) in EMBEDDED_SVG {
            let doc = nexa_gfx::svg::parse(svg).unwrap_or_else(|| panic!("{name}: parse"));
            assert_eq!(
                doc.viewbox,
                (0.0, 0.0, 32.0, 32.0),
                "{name}: 32 viewBox 규격"
            );
            let (w, h, a) = toolbar_mask(name, 20).unwrap_or_else(|| panic!("{name}: mask"));
            assert_eq!((w, h, a.len()), (20, 20, 400));
            let ink = a.iter().filter(|v| **v > 64).count();
            assert!(ink >= 20, "{name}: 잉크 픽셀 {ink}");
            let again = toolbar_mask(name, 20).unwrap();
            assert!(std::ptr::eq(again.2, a), "{name}: 캐시");
        }
        assert!(toolbar_mask("nope", 20).is_none());
    }

    /// 툴바 명령 13개 전부 자산이 있다.
    #[test]
    fn toolbar_commands_map_to_assets() {
        for id in [
            "view.panel_toggle",
            "view.dock",
            "view.always_on_top",
            "view.info_toggle",
            "view.col_width_sync",
            "view.mode_tree",
            "view.mode_flat",
            "view.mode_tiles",
            "view.refresh",
            "file.prefs",
            "view.hidden",
            "view.dot",
            "view.folders_first",
            "view.case_sensitive",
            "view.natural_sort",
        ] {
            let a = asset_of(id).unwrap_or_else(|| panic!("{id}"));
            assert!(EMBEDDED_SVG.iter().any(|(n, _)| *n == a), "{id} → {a}");
        }
        assert!(asset_of("edit.copy").is_none());
    }
}
