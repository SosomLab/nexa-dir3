//! App — 퀵 런처 exe 아이콘(T-30 B · dir2 TERM-124 `win.rs:794-813`): 버튼 = **exe 셸 아이콘 16×16 정사각**(비동기 로딩 · 틱마다 바 갱신) ·
//! 미로드/실패/비Windows = **라벨 앞 2자 글리프** 폴백. 조회는 nexa-ui `nexa_fs::shell::IconService`(워커 스레드 · 캐시 512 · 실패 기억 ·
//! `version()` 폴링) — 이벤트 루프는 조회 중일 때만 150 ms 간격으로 깬다(결과가 오면 `set_item_icon`).

use crate::*;
use nexa_fs::shell::{IconKey, IconService, Lookup};

/// 폴링 간격(ms) — 아이콘 결과는 수십 ms 안에 오고 바뀌는 건 몇 개뿐.
pub(crate) const ICON_POLL_MS: u64 = 150;

/// 라벨 앞 2자(빈 라벨 = `?`).
pub(crate) fn fallback_glyph(label: &str) -> String {
    let g: String = label.trim().chars().take(2).collect();
    if g.is_empty() {
        "?".into()
    } else {
        g
    }
}

thread_local! {
    /// 앱 아이콘 파일 → 디코드한 이미지(실패도 기억 · 런처 항목 수만큼만 생긴다).
    static APP_ICONS: std::cell::RefCell<std::collections::HashMap<PathBuf, Option<Rc<nexa_gfx::IconImage>>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// 빠른 실행 아이콘 캐시가 쥔 픽셀 바이트(메모리 창).
pub(crate) fn cache_bytes() -> u64 {
    APP_ICONS.with(|c| {
        c.borrow()
            .values()
            .flatten()
            .map(|i| u64::from(i.w) * u64::from(i.h) * 4)
            .sum()
    })
}

/// 아이콘 테마의 앱 아이콘(Linux — `nexa_fs::icontheme::app_icon_file`) · 파일마다 한 번 디코드.
fn theme_app_icon(exe: &std::path::Path, px: u32) -> Option<Rc<nexa_gfx::IconImage>> {
    let file = nexa_fs::icontheme::app_icon_file(exe, px)?;
    APP_ICONS.with(|c| {
        c.borrow_mut()
            .entry(file)
            .or_insert_with_key(|f| {
                std::fs::read(f)
                    .ok()
                    .and_then(|b| nexa_gfx::image::decode(&b, 4 * 1024 * 1024).ok())
                    .map(Rc::new)
            })
            .clone()
    })
}

/// 항목 아이콘 — `(아이콘, 조회 중인가)`. 실행 파일을 PATH로 풀어 셸에 묻는다(없으면 즉시 글리프).
/// `logical` = 아이콘 논리 크기(16/20/24/32) — 테마 아이콘은 그 크기로 · 셸 아이콘은 20 초과면 큰 것(32).
pub(crate) fn launcher_icon(item: &launcher::LauncherItem, logical: i32) -> (ToolIcon, bool) {
    let large = logical > 20;
    let glyph = ToolIcon::Glyph(fallback_glyph(&item.label));
    let found = launcher::exe_path(&item.exe);
    // Linux = 설치된 앱의 아이콘(`.desktop`의 Icon → 아이콘 테마 · pixmaps) · 못 찾으면 일반 실행 파일 아이콘 — 글자 두 개 대신
    // 버튼으로 보인다(사용자 10-03 "아이콘이 없는 경우 기본 아이콘을 입혀서" · nexa-ui 128차 · 다른 OS는 즉시 None).
    let exe = found
        .clone()
        .unwrap_or_else(|| PathBuf::from(item.exe.trim()));
    if let Some(img) = theme_app_icon(&exe, logical.max(16) as u32) {
        return (ToolIcon::Image(img), false);
    }
    let Some(path) = found else {
        return (glyph, false);
    };
    match IconService::global().icon(&IconKey::PathPlain(path), large) {
        Lookup::Ready(Some(icon)) => (
            ToolIcon::Image(Rc::new(nexa_gfx::IconImage {
                w: icon.w,
                h: icon.h,
                rgba: icon.rgba.clone(),
            })),
            false,
        ),
        Lookup::Ready(None) => (glyph, false),
        Lookup::Pending => (glyph, true),
    }
}

impl App {
    /// 설정 `launcher.icon_size`(16/20/24/32 · 그 밖 = 20 — 상단 툴바 기본과 같게 · 사용자 10-03 · dir2는 16).
    pub(crate) fn launcher_icon_logical(settings: &Settings) -> i32 {
        match settings.get("launcher.icon_size").unwrap_or("20") {
            "16" => 16,
            "24" => 24,
            "32" => 32,
            _ => 20,
        }
    }

    /// 런처 바 생성(dir2 기본: 바 24 · 아이콘 16 · 여백 2) — 아이콘 크기 = `launcher.icon_size` · 항목 간격 = `launcher.item_gap`.
    pub(crate) fn make_launcherbar(
        items: &[launcher::LauncherItem],
        settings: &Settings,
    ) -> (Toolbar, bool) {
        let large = App::launcher_icon_logical(settings);
        let mut pending = false;
        let tool_items: Vec<ToolItem> = items
            .iter()
            .enumerate()
            .map(|(i, it)| {
                if it.is_separator() {
                    ToolItem::separator()
                } else {
                    let (icon, p) = launcher_icon(it, large);
                    pending |= p;
                    ToolItem::new(format!("launch:{i}"), icon).tip(it.label.clone())
                }
            })
            .collect();
        let mut bar = Toolbar::new(tool_items);
        bar.set_icon_size(App::launcher_icon_logical(settings));
        bar.set_padding(2, 2);
        bar.set_item_gap(
            settings
                .get("launcher.item_gap")
                .and_then(|v| v.trim().parse::<i32>().ok())
                .unwrap_or(4)
                .clamp(0, 16),
        );
        (bar, pending)
    }

    /// 유휴 틱 — 조회 중이면 서비스 버전이 바뀔 때 아이콘을 다시 묻고, 아직 남았으면 다음 폴링 시각을 돌려준다.
    pub(crate) fn launcher_icons_tick(&mut self, now: Instant) -> Option<Instant> {
        if !self.launcher_icons_pending {
            return None;
        }
        let v = IconService::global().version();
        if v != self.launcher_icon_ver {
            self.launcher_icon_ver = v;
            let large = App::launcher_icon_logical(&self.settings);
            let mut inv = Invalidations::default();
            let mut pending = false;
            for (i, it) in self.launcher_items.iter().enumerate() {
                if it.is_separator() {
                    continue;
                }
                let (icon, p) = launcher_icon(it, large);
                pending |= p;
                self.launcherbar
                    .set_item_icon(&format!("launch:{i}"), icon, &mut inv);
            }
            self.launcher_icons_pending = pending;
            self.redraw();
        }
        self.launcher_icons_pending
            .then(|| now + Duration::from_millis(ICON_POLL_MS))
    }

    /// 런처 바 항목 아이콘 요약(시험 · 덤프): `launch:<i>=image|glyph:<자>`.
    pub(crate) fn launcher_icon_summary(&self) -> Vec<String> {
        self.launcherbar
            .items()
            .iter()
            .filter(|it| !it.id.is_empty())
            .map(|it| match &it.icon {
                ToolIcon::Image(img) => format!("{}=image {}x{}", it.id, img.w, img.h),
                ToolIcon::Glyph(g) => format!("{}=glyph:{g}", it.id),
                _ => format!("{}=other", it.id),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_fallback_is_first_two_chars() {
        assert_eq!(fallback_glyph("Explorer"), "Ex");
        assert_eq!(fallback_glyph(" 코드 "), "코드");
        assert_eq!(fallback_glyph("a"), "a");
        assert_eq!(fallback_glyph("   "), "?");
    }

    #[test]
    fn missing_exe_is_glyph_without_lookup() {
        let it = launcher::LauncherItem {
            label: "Bad".into(),
            exe: "nope-xyz-program-ndir".into(),
            args: String::new(),
        };
        let (icon, pending) = launcher_icon(&it, 16);
        // 없는 exe: 아이콘 테마가 있는 Linux = 일반 실행 파일 아이콘(버튼으로 보인다) · 그 밖 = 라벨 앞 2자 글리프.
        if nexa_fs::icontheme::theme_name().is_some() {
            assert!(matches!(icon, ToolIcon::Image(_)));
        } else {
            assert!(matches!(icon, ToolIcon::Glyph(ref g) if g == "Ba"));
        }
        assert!(!pending);
    }
}
