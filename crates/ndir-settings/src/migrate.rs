//! dir2 `data\settings.cfg` → dir3 키 변환(순수 함수 · DR-3 · docs/port/15 PREFS-101~170 · docs/port/31 §5-3 KEY-501~570).
//!
//! dir2 파일 = 1레벨 키 `key=value` · `#` 주석 · 불리언 `0/1` · 위치 정수 `0..8` · 비율 f32 · 동적 키 군(`launcher{N}` · `cloud{N}` ·
//! `cloud_client_*_{kind}`). 변환은 값만 바꾸고 검증은 호출자(`Settings::set` = `normalize`)가 한다. dir2는 범위 밖 정수를 **클램프**했으므로
//! 여기서도 클램프해 사용자 값이 사라지지 않게 한다(port/31 §6-5).

/// 값 변환 방식.
#[derive(Clone, Copy, Debug)]
enum Conv {
    /// 그대로(불리언 0/1·선택지는 `normalize`가 받는다).
    Same,
    /// 정수 클램프.
    Int(i64, i64),
    /// 비율 f32(0.0~1.0) → 정수 퍼센트 클램프.
    Pct(i64, i64),
    /// 위치 정수 0..8 → `POSITIONS` 이름.
    Position,
    /// 옛 단위 × 배수 후 정수 클램프(`transfer_close_secs` → ms).
    Scale(i64, i64, i64),
}

/// (dir2 키, dir3 키, 변환).
const MAP: &[(&str, &str, Conv)] = &[
    ("theme", "ui.theme", Conv::Same),
    ("lang", "ui.lang", Conv::Same),
    ("show_hidden", "list.show_hidden", Conv::Same),
    ("show_dotfiles", "list.show_dotfiles", Conv::Same),
    ("split", "layout.panel_split_pct", Conv::Pct(10, 90)),
    ("dock", "dock.visible", Conv::Same),
    ("dock_ratio", "layout.dock_height_pct", Conv::Pct(15, 50)),
    ("dock_split", "layout.dock_split_pct", Conv::Pct(15, 85)),
    ("term_font", "term.font_face", Conv::Same),
    ("term_font_size", "term.font_size", Conv::Int(8, 32)),
    ("dlg_font", "ui.dialog_font_face", Conv::Same),
    ("dlg_font_size", "ui.dialog_font_size", Conv::Int(7, 24)), // pt — 접미는 아래에서 붙인다
    ("launcher", "launcher.visible", Conv::Same),
    ("term_wrap", "term.wrap", Conv::Same),
    ("term_cols", "term.cols", Conv::Int(80, 1000)),
    ("term_theme", "term.theme", Conv::Same),
    ("term_theme_dark", "term.theme_dark", Conv::Same),
    ("term_theme_light", "term.theme_light", Conv::Same),
    ("term_copy_format", "term.copy_format", Conv::Same),
    (
        "transfer_close_ms",
        "transfer.close_ms",
        Conv::Int(0, 10000),
    ),
    (
        "transfer_close_secs",
        "transfer.close_ms",
        Conv::Scale(1000, 0, 10000),
    ),
    (
        "dnd_hover_ms",
        "transfer.dnd_hover_ms",
        Conv::Int(200, 10000),
    ),
    ("preview_map", "preview.map", Conv::Same),
    ("plugins_disabled", "plugins.disabled", Conv::Same),
    ("sort_folders_first", "list.folders_first", Conv::Same),
    (
        "sort_case_sensitive",
        "list.sort_case_sensitive",
        Conv::Same,
    ),
    ("nav_up_align", "list.nav_up_align", Conv::Same),
    ("tab_dblclick", "tabs.dblclick", Conv::Same),
    ("view_mode", "list.view_mode", Conv::Same),
    ("panel_mode", "layout.panel_mode", Conv::Same),
    ("info_mode", "layout.info_mode", Conv::Same),
    ("view_scope", "list.view_scope", Conv::Same),
    ("hide_empty_glyph", "list.hide_empty_glyph", Conv::Same),
    ("always_on_top", "window.always_on_top", Conv::Same),
    ("col_width_sync", "list.col_width_sync", Conv::Same),
    (
        "col_autofit_max",
        "list.col_autofit_max",
        Conv::Int(50, 2000),
    ),
    ("toolbar_order", "toolbar.layout", Conv::Same),
    ("ctx_menu_order", "ctxmenu.layout", Conv::Same),
    ("typeahead_scope", "typeahead.scope", Conv::Same),
    (
        "typeahead_reset_ms",
        "typeahead.reset_ms",
        Conv::Int(200, 10000),
    ),
    ("typeahead_pos", "typeahead.hud_pos", Conv::Position),
    ("typeahead_special", "typeahead.special", Conv::Same),
    ("typeahead_space", "typeahead.space", Conv::Same),
    ("typeahead_backspace", "typeahead.backspace", Conv::Same),
    ("fast_scroll", "scroll.fast", Conv::Same),
    ("fast_scroll_step", "scroll.fast_step", Conv::Int(1, 50)),
    ("fast_scroll_max", "scroll.fast_max", Conv::Int(1, 32)),
    (
        "fast_scroll_window_ms",
        "scroll.fast_window_ms",
        Conv::Int(20, 2000),
    ),
    ("fast_scroll_hud", "scroll.fast_hud", Conv::Same),
    ("fast_scroll_hud_pos", "scroll.fast_hud_pos", Conv::Position),
    (
        "fast_scroll_hud_hold_ms",
        "scroll.fast_hud_hold_ms",
        Conv::Int(0, 10000),
    ),
    (
        "fast_scroll_hud_fade_ms",
        "scroll.fast_hud_fade_ms",
        Conv::Int(0, 10000),
    ),
    (
        "fast_scroll_grid_extra",
        "scroll.fast_grid_extra",
        Conv::Same,
    ),
    ("base_font", "ui.font_face", Conv::Same),
    ("base_font_size", "ui.font_size", Conv::Int(8, 32)),
    ("ctx_font", "ui.menu_font_face", Conv::Same),
    ("ctx_font_size", "ui.menu_font_size", Conv::Int(8, 32)),
    ("status_font", "statusbar.font_face", Conv::Same),
    ("status_font_size", "statusbar.font_size", Conv::Int(8, 32)),
    ("list_font", "list.font_face", Conv::Same),
    ("list_font_size", "list.font_size", Conv::Int(8, 32)),
    ("list_folder_bold", "list.folder_bold", Conv::Same),
    ("header_bold", "list.header_bold", Conv::Same),
    ("header_italic", "list.header_italic", Conv::Same),
    ("launcher_seed", "launcher.seed", Conv::Int(0, 9999)),
];

/// dir2 기본 글꼴(Windows 전용 이름) — dir3 기본은 빈 값(OS 기본)·OS별 고정폭이므로 같은 값이면 옮기지 않는다.
const WINDOWS_ONLY_FONT_DEFAULTS: &[(&str, &str)] = &[
    ("base_font", "Segoe UI"),
    ("ctx_font", "Segoe UI"),
    ("status_font", "Segoe UI"),
    ("list_font", "Segoe UI"),
    ("dlg_font", "Segoe UI"),
    // 터미널 글꼴은 OS_DEFAULTS(Consolas/Menlo/DejaVu Sans Mono) — dir2 기본 Consolas는 Windows 밖에서 뜻이 없다(CI mac/linux 실측 10-03).
    ("term_font", "Consolas"),
];

fn clamp_int(v: &str, min: i64, max: i64) -> Option<String> {
    let n: i64 = v.trim().parse().ok()?;
    Some(n.clamp(min, max).to_string())
}

/// dir2 `settings.cfg` 본문 → `(dir3 키, 값)` 목록(파일 순서 · 동적 키 군은 끝에 묶음).
#[must_use]
pub fn import_dir2(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut launcher: Vec<(u32, String)> = Vec::new();
    let mut cloud: Vec<(u32, String)> = Vec::new();
    for raw in text.strip_prefix('\u{feff}').unwrap_or(text).lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let (k, v) = (k.trim(), v.trim());
        if let Some(n) = k
            .strip_prefix("launcher")
            .and_then(|s| s.parse::<u32>().ok())
        {
            launcher.push((n, v.to_string()));
            continue;
        }
        if let Some(n) = k.strip_prefix("cloud").and_then(|s| s.parse::<u32>().ok()) {
            cloud.push((n, v.to_string()));
            continue;
        }
        if let Some(kind) = k.strip_prefix("cloud_client_id_") {
            if !v.is_empty() {
                out.push((format!("cloud.client_id_{kind}"), v.to_string()));
            }
            continue;
        }
        if let Some(kind) = k.strip_prefix("cloud_client_secret_") {
            if !v.is_empty() {
                out.push((format!("cloud.client_secret_{kind}"), v.to_string()));
            }
            continue;
        }
        if WINDOWS_ONLY_FONT_DEFAULTS
            .iter()
            .any(|(fk, fv)| *fk == k && *fv == v)
        {
            continue;
        }
        let Some((_, new, conv)) = MAP.iter().find(|(old, _, _)| *old == k) else {
            continue;
        };
        let value = match conv {
            Conv::Same => Some(v.to_string()),
            Conv::Int(a, b) => clamp_int(v, *a, *b),
            Conv::Pct(a, b) => v
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|f| f.is_finite())
                .map(|f| ((f * 100.0).round() as i64).clamp(*a, *b).to_string()),
            Conv::Position => v
                .trim()
                .parse::<usize>()
                .ok()
                .and_then(|i| crate::POSITIONS.get(i))
                .map(|p| (*p).to_string()),
            Conv::Scale(k, a, b) => v
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|f| f.is_finite())
                .map(|f| ((f * *k as f64).round() as i64).clamp(*a, *b).to_string()),
        };
        let Some(mut value) = value else { continue };
        if *new == "ui.dialog_font_size" {
            value.push_str("pt"); // dir2 대화상자 글꼴 크기는 pt 단위(PREFS-112)
        }
        out.push(((*new).to_string(), value));
    }
    launcher.sort_by_key(|(n, _)| *n);
    cloud.sort_by_key(|(n, _)| *n);
    if !launcher.is_empty() {
        out.push((
            "launcher.items".into(),
            launcher
                .into_iter()
                .map(|(_, v)| v)
                .collect::<Vec<_>>()
                .join("\n"),
        ));
    }
    if !cloud.is_empty() {
        out.push((
            "cloud.conns".into(),
            cloud
                .into_iter()
                .map(|(_, v)| v)
                .collect::<Vec<_>>()
                .join("\n"),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{entry, normalize, Settings};

    /// 원장 전수(T-90 · docs/port/31 §1-1 KEY-001~071): dir2 `settings.cfg` 키 **전부**가 변환표의 옛 이름이거나 레지스트리 키 그대로다.
    /// 의도된 예외 = `launcher_count`(dir3는 `launcher.items` 목록이 개수를 대신한다 · ⚠).
    #[test]
    fn dir2_catalog_settings_keys_are_mapped() {
        let doc = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/port/31-catalog-settings-i18n.md"
        ))
        .expect("docs/port/31");
        let mut keys: Vec<String> = Vec::new();
        let mut in_sec = false;
        for line in doc.lines() {
            if line.starts_with("### 1-1.") {
                in_sec = true;
                continue;
            }
            if in_sec && line.starts_with("### 1-2.") {
                break;
            }
            if !in_sec || !line.starts_with("| KEY-0") {
                continue;
            }
            let mut cells = line.split('|').map(str::trim);
            let _ = cells.next();
            let _ = cells.next();
            if let Some(k) = cells.next() {
                // 칸 = `키` + 설명 꼬리(구 키 표기 등) → 첫 백틱 쌍 안만. 머리 주석 행·동적 키 군(`launcher<N>` …)은 변환기가 패턴으로 다룬다.
                let k = k.trim_start_matches('`');
                let k = k.split('`').next().unwrap_or("");
                if !k.is_empty() && !k.starts_with('#') && !k.contains('<') && !k.contains(' ') {
                    keys.push(k.to_string());
                }
            }
        }
        assert!(keys.len() >= 60, "원장 settings 행 수: {}", keys.len());
        // 의도된 예외: `launcher_count`(목록이 개수를 대신) · `transfer_close_secs`(구 키 · dir2도 읽기 전용 호환 — 변환표에 있으면 무해).
        const DROPPED: &[&str] = &["launcher_count", "transfer_close_secs"];
        let unmapped: Vec<&String> = keys
            .iter()
            .filter(|k| {
                !MAP.iter().any(|(old, _, _)| old == k)
                    && entry(k).is_none()
                    && !DROPPED.contains(&k.as_str())
            })
            .collect();
        assert!(unmapped.is_empty(), "대응 없는 dir2 키: {unmapped:?}");
    }

    /// 변환표 전수: 새 키가 레지스트리에 있고, dir2 기본값(PREFS-101~)을 넣으면 전부 검증을 통과한다.
    #[test]
    fn map_targets_exist_and_dir2_defaults_validate() {
        for (old, new, _) in MAP {
            assert!(entry(new).is_some(), "{old} → {new}: 레지스트리에 없음");
        }
        let dir2_defaults = "# nexa-dir settings v1\ntheme=dark\nlang=system\nshow_hidden=1\nshow_dotfiles=1\nsplit=0.500\ndock=1\ndock_ratio=0.300\ndock_split=0.500\nterm_font=Consolas\nterm_font_size=12\ndlg_font=Segoe UI\ndlg_font_size=9\nlauncher=1\nterm_wrap=1\nterm_cols=240\nterm_theme=system\nterm_theme_dark=campbell\nterm_theme_light=github-light\nterm_copy_format=text\ntransfer_close_ms=2000\ndnd_hover_ms=3000\nsort_folders_first=1\nsort_case_sensitive=0\nnav_up_align=center\ntab_dblclick=close\nview_mode=tree\npanel_mode=dual\ninfo_mode=dual\nview_scope=panel\nhide_empty_glyph=1\nalways_on_top=0\ncol_width_sync=1\ncol_autofit_max=400\ntypeahead_scope=visible\ntypeahead_reset_ms=1000\ntypeahead_pos=6\ntypeahead_special=1\ntypeahead_space=1\ntypeahead_backspace=1\nfast_scroll=1\nfast_scroll_step=3\nfast_scroll_max=16\nfast_scroll_window_ms=160\nfast_scroll_hud=1\nfast_scroll_hud_pos=2\nfast_scroll_hud_hold_ms=250\nfast_scroll_hud_fade_ms=600\nfast_scroll_grid_extra=1\nbase_font=Segoe UI\nbase_font_size=12\nctx_font=Segoe UI\nctx_font_size=12\nstatus_font=Segoe UI\nstatus_font_size=12\nlist_font=Segoe UI\nlist_font_size=12\nlist_folder_bold=0\nheader_bold=0\nheader_italic=0\nlauncher_seed=2\n";
        let pairs = import_dir2(dir2_defaults);
        for (k, v) in &pairs {
            let e = entry(k).unwrap();
            assert!(normalize(e.kind, v).is_some(), "{k}={v}");
        }
        let mut s = Settings::from_text(std::path::PathBuf::from("x"), "");
        let n = s.import_dir2(dir2_defaults);
        assert!(n >= 50, "{n}");
        // dir2 기본값 = dir3 기본값 → 변경분 0에 가깝다(Windows 전용 글꼴 이름은 건너뛴다 · 시드 2와, dir3가 기본을 바꾼
        // 보기 옵션 범위(dir2 `panel` → dir3 `tab` · 사용자 10-03 "탭별 설정으로 관리")만 남는다 — dir2에서 쓰던 범위를 지킨다).
        let modified: Vec<_> = s
            .list()
            .into_iter()
            .filter(|(_, _, m)| *m)
            .map(|(e, v, _)| (e.key, v.to_string()))
            .collect();
        assert_eq!(
            modified,
            vec![
                ("launcher.seed", "2".to_string()),
                ("list.view_scope", "panel".to_string())
            ],
            "{modified:?}"
        );
    }

    #[test]
    fn converts_values_and_dynamic_groups() {
        let pairs = import_dir2(
            "theme=light\nsplit=0.333\ndock_ratio=0.9\ntypeahead_pos=8\nfast_scroll_hud_pos=99\nterm_cols=5\ntransfer_close_secs=3\ndlg_font_size=11\nbase_font=Malgun Gothic\nlauncher1=Notepad|notepad.exe|\nlauncher0=-\ncloud0=onedrive|Work|/docs|me@x\ncloud_client_id_dropbox=abc\ncloud_client_secret_dropbox=\nunknown_key=1\n",
        );
        let get = |k: &str| {
            pairs
                .iter()
                .find(|(kk, _)| kk == k)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(get("ui.theme"), Some("light"));
        assert_eq!(get("layout.panel_split_pct"), Some("33"));
        assert_eq!(get("layout.dock_height_pct"), Some("50"), "클램프 상한");
        assert_eq!(get("typeahead.hud_pos"), Some("bottom_right"));
        assert_eq!(
            get("scroll.fast_hud_pos"),
            None,
            "범위 밖 위치는 버림(기본 유지)"
        );
        assert_eq!(get("term.cols"), Some("80"), "클램프 하한");
        assert_eq!(get("transfer.close_ms"), Some("3000"), "구 키 초 → ms");
        assert_eq!(get("ui.dialog_font_size"), Some("11pt"));
        assert_eq!(
            get("ui.font_face"),
            Some("Malgun Gothic"),
            "Windows 기본이 아닌 글꼴은 옮긴다"
        );
        assert_eq!(
            get("launcher.items"),
            Some("-\nNotepad|notepad.exe|"),
            "번호순 · 줄 분리"
        );
        assert_eq!(get("cloud.conns"), Some("onedrive|Work|/docs|me@x"));
        assert_eq!(get("cloud.client_id_dropbox"), Some("abc"));
        assert_eq!(get("cloud.client_secret_dropbox"), None, "빈 값은 건너뜀");
        assert!(!pairs.iter().any(|(k, _)| k.contains("unknown")));
    }

    #[test]
    fn import_does_not_override_existing_values() {
        let mut s = Settings::from_text(std::path::PathBuf::from("x"), "ui.theme=light\n");
        s.import_dir2("theme=system\nshow_hidden=0\n");
        assert_eq!(
            s.get("ui.theme"),
            Some("light"),
            "dir3에서 정한 값이 이긴다"
        );
        assert_eq!(s.get("list.show_hidden"), Some("off"));
    }
}
