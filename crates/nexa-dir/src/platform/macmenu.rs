//! **macOS 컨텍스트 메뉴 항목**(`MacMenu` · ContextMenuProvider · T-114 ③ · 10-31): Finder처럼 "‹기본 앱›(으)로 열기" · "다른 앱으로 열기 ▸" ·
//! 폴더 = "터미널에서 열기" · "Finder에서 보기" · "정보 가져오기" · 배경 = 터미널 · Finder에서 열기 · 정보. Windows 셸 메뉴(동적 확장 항목)와
//! 달리 **고정 집합**이라 Linux `XdgMenu`와 같은 **`xdg.*` id 어휘**를 그대로 쓴다(어휘 = "고정 집합 셸 항목" · xdg 유래 — 호스트의
//! 폴더 "열기" 판정 · `ctxmenu.layout` 순서/표시 편집 · 라벨 표 · 시험이 그대로 통한다 · 협업 분석 10-31 (B)안).
//!
//! 앱 목록 = LaunchServices(`NSWorkspace URLForApplicationToOpenURL:` = 기본 · `URLsForApplicationsToOpenURL:` = 전부 · macOS 12+ →
//! `respondsToSelector:` 가드 · 11 = 기본 앱만). 같은 앱이 경로만 달리 두 번 오면(이 Mac 실측) 하나로 · 자기 번들 제외 · 기본 앱은 목록에서
//! 뺀다. 아이콘 = nexa-fs 246 `icon_for_path`(그 .app · Finder.app · Terminal.app). 실행 = `open -a <앱> <경로…>`(한 번에) →
//! `watch_open_child`(늦은 실패 토스트 · 좀비 회수). 압축은 내지 않는다(호스트 내장 zip이 붙는다) · 빠른 동작/서비스 · 전자메일 = 2차.
//! 캐시 = (확장자 소문자, 폴더 여부) → 앱 목록 · 앱 경로 → 아이콘(`invalidate`로 비움).
//! `xdg.app:` 뒤 값 = **번들 id**(`com.apple.Safari` · 공백 없음 — `ctx.dump`/`ctx.pick:<id>`가 id를 공백으로 잇는다 · 같은 앱이 여러 경로에
//! 있어도 하나) · 번들 id가 없으면 `path:<퍼센트 인코딩 경로>` 폴백. 실행 때 `URLForApplicationWithBundleIdentifier:`로 경로를 되찾는다.
//! 번들(`.app`) 선택은 열기/다른 앱으로 열기를 내지 않는다(기본 앱 = 자기 자신 · Finder도 없음).

use super::*;
use std::cell::RefCell;
use std::collections::HashMap;

/// "다른 앱으로 열기 ▸"에 보일 최대 개수(Linux와 같음).
const OPEN_WITH_MAX: usize = 12;
const FINDER_APP: &str = "/System/Library/CoreServices/Finder.app";
const TERMINAL_APPS: [&str; 2] = [
    "/System/Applications/Utilities/Terminal.app",
    "/Applications/Utilities/Terminal.app",
];

/// (기본 앱, 열 수 있는 앱 전부).
type AppsFor = (Option<AppRef>, Vec<AppRef>);

#[derive(Default)]
pub(super) struct MacMenu {
    /// (확장자 소문자 · 폴더 여부) → (기본 앱, 전체 목록).
    apps: RefCell<HashMap<(String, bool), AppsFor>>,
    icons: RefCell<HashMap<PathBuf, Option<ShellIcon>>>,
}

/// 열기 후보 앱: 메뉴 id 토큰(번들 id 또는 `path:…`) + 번들 경로.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppRef {
    pub token: String,
    pub path: PathBuf,
}

/// 경로 → `path:` 토큰(퍼센트 인코딩 · 공백 · `%` · 제어 문자 · 비ASCII를 `%XX`로 — id에 공백이 못 들어간다).
pub(crate) fn path_token(p: &Path) -> String {
    let mut out = String::from("path:");
    for b in p.to_string_lossy().bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// 토큰 → 경로(`path:` 폴백만 · 번들 id는 호출자가 LaunchServices에 묻는다). 깨진 `%`는 그대로 둔다.
pub(crate) fn path_from_token(token: &str) -> Option<PathBuf> {
    let enc = token.strip_prefix("path:")?;
    let bytes = enc.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&enc[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    Some(PathBuf::from(String::from_utf8_lossy(&out).into_owned()))
}

/// 앱 표시 이름 = 번들 파일 이름의 어간(`Visual Studio Code.app` → `Visual Studio Code`).
pub(crate) fn app_display_name(app: &Path) -> String {
    app.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| app.to_string_lossy().into_owned())
}

/// 열기 항목 구성(순수 · MC/DC 시험): 파일이면 기본 앱이 첫 항목(머리) · 나머지(기본 · 자기 번들 · 같은 토큰 중복 제거 · `max`개)는
/// "다른 앱으로 열기 ▸" 하위 · 폴더면 기본(Finder)도 하위 메뉴에(열기 = 앱 안 이동이 따로 있다).
pub(crate) fn build_open_with(
    default: Option<&AppRef>,
    apps: &[AppRef],
    self_token: Option<&str>,
    is_dir: bool,
    max: usize,
) -> (Option<AppRef>, Vec<AppRef>) {
    let is_self = |a: &AppRef| self_token.is_some_and(|s| s == a.token);
    let default = default.filter(|d| !is_self(d)).cloned();
    let mut rest: Vec<AppRef> = Vec::new();
    let head: Vec<&AppRef> = if is_dir {
        default.iter().collect()
    } else {
        Vec::new()
    };
    for a in head.into_iter().chain(apps.iter()) {
        if is_self(a) || rest.iter().any(|r| r.token == a.token) {
            continue;
        }
        if !is_dir && default.as_ref().is_some_and(|d| d.token == a.token) {
            continue;
        }
        rest.push(a.clone());
        if rest.len() >= max {
            break;
        }
    }
    (if is_dir { None } else { default }, rest)
}

impl MacMenu {
    /// 이 프로그램의 번들 토큰(`…/Nexa Dir.app/Contents/MacOS/nexa-dir` → 그 번들 id) — 자기 자신은 "다른 앱으로 열기"에 안 낸다.
    fn self_token() -> Option<String> {
        let exe = std::env::current_exe().ok()?;
        let bundle = exe
            .ancestors()
            .find(|a| a.extension().is_some_and(|e| e == "app"))?;
        Some(app_ref(bundle).token)
    }

    fn icon_of(&self, app: &Path) -> Option<ShellIcon> {
        let mut cache = self.icons.borrow_mut();
        if let Some(hit) = cache.get(app) {
            return hit.clone();
        }
        let got = nexa_fs::shell::icon_for_path(app, false).map(|i| ShellIcon {
            w: i.w,
            h: i.h,
            rgba: i.rgba,
        });
        cache.insert(app.to_path_buf(), got.clone());
        got
    }

    fn terminal_app() -> Option<&'static str> {
        TERMINAL_APPS.into_iter().find(|p| Path::new(p).is_dir())
    }

    /// LaunchServices 조회(캐시) — (기본 앱, 전체).
    fn apps_for(&self, path: &Path) -> AppsFor {
        let is_dir = path.is_dir();
        let key = (
            path.extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default(),
            is_dir,
        );
        if let Some(hit) = self.apps.borrow().get(&key) {
            return hit.clone();
        }
        let got = launch_services_apps(path);
        self.apps.borrow_mut().insert(key, got.clone());
        got
    }

    fn item(&self, id: &str, label: String, icon_app: Option<&str>) -> ShellMenuItem {
        ShellMenuItem {
            icon: icon_app.and_then(|a| self.icon_of(Path::new(a))),
            id: id.into(),
            label,
            enabled: true,
            ..Default::default()
        }
    }

    fn app_item(&self, app: &AppRef, label: String) -> ShellMenuItem {
        ShellMenuItem {
            icon: self.icon_of(&app.path),
            id: format!("xdg.app:{}", app.token),
            label,
            enabled: true,
            ..Default::default()
        }
    }
}

/// 번들 경로 → `AppRef`(번들 id 토큰 · 없으면 `path:` 폴백).
fn app_ref(path: &Path) -> AppRef {
    use objc2_foundation::{NSBundle, NSString};
    // SAFETY: 경로로 NSBundle 조회(없으면 None) · 식별자 문자열 복사.
    let id = unsafe {
        NSBundle::bundleWithPath(&NSString::from_str(&path.to_string_lossy()))
            .and_then(|b| b.bundleIdentifier())
            .map(|s| s.to_string())
    }
    .filter(|s| !s.is_empty() && !s.contains(char::is_whitespace));
    AppRef {
        token: id.unwrap_or_else(|| path_token(path)),
        path: path.to_path_buf(),
    }
}

/// 토큰 → 앱 경로(번들 id = LaunchServices · `path:` = 디코딩).
fn app_path_of(token: &str) -> Option<PathBuf> {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::NSString;
    if let Some(p) = path_from_token(token) {
        return Some(p);
    }
    // SAFETY: 공유 워크스페이스 · 식별자 문자열 · 반환 URL은 Retained.
    unsafe {
        NSWorkspace::sharedWorkspace()
            .URLForApplicationWithBundleIdentifier(&NSString::from_str(token))
            .and_then(|u| u.path())
            .map(|s| PathBuf::from(s.to_string()))
    }
}

/// `NSWorkspace`로 (기본 앱, 열 수 있는 앱 전부). macOS 12 미만이면 목록은 기본 앱 하나.
fn launch_services_apps(path: &Path) -> AppsFor {
    use objc2::runtime::NSObjectProtocol as _;
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSString, NSURL};
    let to_app = |u: &NSURL| -> Option<AppRef> {
        // SAFETY: 살아 있는 NSURL의 path 조회.
        unsafe { u.path() }.map(|s| app_ref(Path::new(&s.to_string())))
    };
    // SAFETY: 공유 워크스페이스 · 파일 URL · 반환 객체는 Retained로 수명 관리 · 12 미만 가드.
    unsafe {
        let ws = NSWorkspace::sharedWorkspace();
        let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
        let default = ws.URLForApplicationToOpenURL(&url).and_then(|u| to_app(&u));
        let all: Vec<AppRef> = if ws.respondsToSelector(objc2::sel!(URLsForApplicationsToOpenURL:))
        {
            ws.URLsForApplicationsToOpenURL(&url)
                .iter()
                .filter_map(to_app)
                .collect()
        } else {
            default.iter().cloned().collect()
        };
        (default, all)
    }
}

/// 자식 프로세스 시작 + 늦은 실패 감시(`open`은 성공하면 바로 끝나고 · 실패 코드는 수 초 뒤 올 수 있다).
fn spawn_watched(argv: &[String], report: &Path) -> Result<(), PlatformError> {
    let (prog, args) = argv
        .split_first()
        .ok_or_else(|| PlatformError::Failed("empty argv".into()))?;
    let child = std::process::Command::new(prog)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| PlatformError::Failed(format!("{prog}: {e}")))?;
    watch_open_child(child, report.to_path_buf());
    Ok(())
}

/// AppleScript 문자열 리터럴 이스케이프(`\` · `"`).
fn applescript_quote(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Finder "정보 가져오기" 창(각 경로마다) — `osascript`.
fn show_info(paths: &[PathBuf]) -> Result<(), PlatformError> {
    let mut script = String::from("tell application \"Finder\"\nactivate\n");
    for p in paths {
        script.push_str(&format!(
            "open information window of (POSIX file \"{}\" as alias)\n",
            applescript_quote(&p.to_string_lossy())
        ));
    }
    script.push_str("end tell");
    let first = paths.first().cloned().unwrap_or_default();
    spawn_watched(&["osascript".into(), "-e".into(), script], &first)
}

impl ContextMenuProvider for MacMenu {
    fn items(&self, paths: &[PathBuf]) -> Result<Vec<ShellMenuItem>, PlatformError> {
        let Some(first) = paths.first() else {
            return Ok(Vec::new());
        };
        let bundle = ndir_vfs::is_bundle_path(first);
        let is_dir = first.is_dir() && !bundle;
        // 번들(.app)은 열기/다른 앱으로 열기를 내지 않는다(기본 앱 = 자기 자신).
        let (head, rest) = if bundle {
            (None, Vec::new())
        } else {
            let (default, all) = self.apps_for(first);
            build_open_with(
                default.as_ref(),
                &all,
                Self::self_token().as_deref(),
                is_dir,
                OPEN_WITH_MAX,
            )
        };
        let mut out = Vec::new();
        if let Some(app) = &head {
            out.push(self.app_item(
                app,
                ndir_i18n::trf("ctx.openWithApp", &[&app_display_name(&app.path)]),
            ));
        }
        if !rest.is_empty() {
            out.push(ShellMenuItem {
                id: "xdg.openwith".into(),
                label: ndir_i18n::tr("ctx.openWith"),
                enabled: true,
                children: rest
                    .iter()
                    .map(|a| self.app_item(a, app_display_name(&a.path)))
                    .collect(),
                ..Default::default()
            });
        }
        if !out.is_empty() {
            out.push(ShellMenuItem {
                separator: true,
                ..Default::default()
            });
        }
        if is_dir && paths.len() == 1 {
            if let Some(term) = Self::terminal_app() {
                out.push(self.item(
                    "xdg.terminal",
                    ndir_i18n::tr("ctx.openTerminal"),
                    Some(term),
                ));
            }
        }
        out.push(self.item(
            "xdg.showin",
            ndir_i18n::tr("ctx.showInFileManager"),
            Some(FINDER_APP),
        ));
        out.push(self.item(
            "xdg.props",
            ndir_i18n::tr("ctx.properties"),
            Some(FINDER_APP),
        ));
        Ok(out)
    }

    fn bg_items(&self, _dir: &Path) -> Result<Vec<ShellMenuItem>, PlatformError> {
        let mut out = Vec::new();
        if let Some(term) = Self::terminal_app() {
            out.push(self.item(
                "xdg.terminal",
                ndir_i18n::tr("ctx.openTerminal"),
                Some(term),
            ));
        }
        out.push(self.item(
            "xdg.filemanager",
            ndir_i18n::tr("ctx.openFileManager"),
            Some(FINDER_APP),
        ));
        out.push(self.item(
            "xdg.props",
            ndir_i18n::tr("ctx.properties"),
            Some(FINDER_APP),
        ));
        Ok(out)
    }

    fn invoke_bg(&self, id: &str, dir: &Path) -> Result<Option<PathBuf>, PlatformError> {
        let d = dir.to_string_lossy().into_owned();
        match id {
            "xdg.terminal" => {
                spawn_watched(&["open".into(), "-a".into(), "Terminal".into(), d], dir)
            }
            "xdg.filemanager" => spawn_watched(&["open".into(), d], dir),
            "xdg.props" => show_info(std::slice::from_ref(&dir.to_path_buf())),
            other => Err(PlatformError::Failed(format!("unknown bg item: {other}"))),
        }
        .map(|()| None)
    }

    fn invoke(&self, id: &str, paths: &[PathBuf]) -> Result<(), PlatformError> {
        let first = paths
            .first()
            .ok_or_else(|| PlatformError::Failed("menu needs a selection".into()))?;
        let strs = || paths.iter().map(|p| p.to_string_lossy().into_owned());
        if let Some(token) = id.strip_prefix("xdg.app:") {
            let app = app_path_of(token)
                .ok_or_else(|| PlatformError::Failed(format!("no such app: {token}")))?;
            let mut argv = vec![
                "open".to_string(),
                "-a".into(),
                app.to_string_lossy().into_owned(),
            ];
            argv.extend(strs());
            return spawn_watched(&argv, first);
        }
        match id {
            "xdg.showin" => {
                let mut argv = vec!["open".to_string(), "-R".into()];
                argv.extend(strs());
                spawn_watched(&argv, first)
            }
            "xdg.terminal" => match paths {
                [one] => spawn_watched(
                    &[
                        "open".into(),
                        "-a".into(),
                        "Terminal".into(),
                        one.to_string_lossy().into_owned(),
                    ],
                    one,
                ),
                _ => Err(PlatformError::Failed("terminal needs one folder".into())),
            },
            "xdg.props" => show_info(paths),
            other => Err(PlatformError::Failed(format!("unknown menu item: {other}"))),
        }
    }

    fn invalidate(&self) {
        self.apps.borrow_mut().clear();
        self.icons.borrow_mut().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    fn a(token: &str, path: &str) -> AppRef {
        AppRef {
            token: token.into(),
            path: p(path),
        }
    }

    /// 열기 항목 구성(MC/DC): 파일 × 기본 있음 × 같은 토큰 중복(다른 경로) × 자기 번들 × 상한 · 폴더 = 기본도 하위 메뉴.
    #[test]
    #[allow(clippy::redundant_clone)]
    fn open_with_dedupes_and_orders() {
        let a1 = a("com.a", "/Applications/A.app");
        let a2 = a("com.a", "/Users/x/Applications/A.app");
        let b = a("com.b", "/Applications/B.app");
        let me = a("com.sosomlab.nexa-dir", "/Applications/Nexa Dir.app");
        // 파일: 기본 A가 머리 · 나머지 = B(같은 토큰 A2 · 자기 번들 제외).
        let (head, rest) = build_open_with(
            Some(&a1),
            &[a1.clone(), b.clone(), a2.clone(), me.clone()],
            Some("com.sosomlab.nexa-dir"),
            false,
            12,
        );
        assert_eq!(head.as_ref(), Some(&a1));
        assert_eq!(rest, vec![b.clone()]);
        // 폴더: 머리 없음 · 기본(Finder)이 하위 첫 항목 · 뒤에 나머지.
        let finder = a("com.apple.finder", FINDER_APP);
        let (head, rest) =
            build_open_with(Some(&finder), &[b.clone(), finder.clone()], None, true, 12);
        assert_eq!(head, None);
        assert_eq!(rest, vec![finder.clone(), b.clone()]);
        // 상한 · 기본 없음 · 자기 번들이 기본이면 머리 없음.
        let (head, rest) = build_open_with(None, &[a1.clone(), b.clone()], None, false, 1);
        assert_eq!((head, rest), (None, vec![a1.clone()]));
        let (head, rest) = build_open_with(
            Some(&me),
            &[me.clone(), a1.clone()],
            Some(&me.token),
            false,
            12,
        );
        assert_eq!((head, rest), (None, vec![a1.clone()]));
        assert_eq!(
            app_display_name(&p("/Applications/Visual Studio Code.app")),
            "Visual Studio Code"
        );
        assert_eq!(applescript_quote(r#"a"b\c"#), r#"a\"b\\c"#);
        // 경로 토큰 = 공백 없음 · 왕복.
        let t = path_token(&p("/Applications/Visual Studio Code.app"));
        assert!(!t.contains(' ') && t.starts_with("path:"), "{t}");
        assert_eq!(
            path_from_token(&t).as_deref(),
            Some(Path::new("/Applications/Visual Studio Code.app"))
        );
        assert_eq!(path_from_token("com.apple.Safari"), None);
    }

    /// 실제 LaunchServices(macOS CI 러너에도 Terminal · TextEdit 있음): /bin/ls = 기본 앱 Some · 항목 첫 id = `xdg.app:` · 폴더 = 터미널 · 보기 · 정보.
    #[test]
    fn launch_services_items_for_real_paths() {
        let m = MacMenu::default();
        let items = m.items(&[p("/bin/ls")]).unwrap();
        assert!(
            items
                .first()
                .is_some_and(|i| i.id.starts_with("xdg.app:") && !i.label.is_empty()),
            "{:?}",
            items.iter().map(|i| i.id.clone()).collect::<Vec<_>>()
        );
        assert!(
            items.iter().all(|i| !i.id.contains(' ')),
            "id에 공백 없음(ctx.dump): {:?}",
            items.iter().map(|i| i.id.clone()).collect::<Vec<_>>()
        );
        // 기본 앱 토큰(Terminal = com.apple.Terminal 류) → 경로 복원.
        let token = items[0].id.strip_prefix("xdg.app:").unwrap();
        assert!(app_path_of(token).is_some_and(|p| p.is_dir()), "{token}");
        // 번들 선택 = 열기 항목 없음.
        let app_items = m.items(&[p(FINDER_APP)]).unwrap();
        assert!(app_items
            .iter()
            .all(|i| !i.id.starts_with("xdg.app:") && i.id != "xdg.openwith"));
        assert!(
            items.iter().any(|i| i.id == "xdg.showin") && items.iter().any(|i| i.id == "xdg.props")
        );
        let dir_items = m.items(&[p("/tmp")]).unwrap();
        assert!(
            dir_items.iter().all(|i| !i.id.starts_with("xdg.app:")),
            "폴더 = 머리 항목 없음"
        );
        assert!(dir_items.iter().any(|i| i.id == "xdg.terminal"));
        let bg = m.bg_items(Path::new("/tmp")).unwrap();
        assert!(bg.iter().any(|i| i.id == "xdg.filemanager"));
        // 캐시 적중(같은 확장자) · 비우기.
        let _ = m.items(&[p("/bin/cat")]);
        assert_eq!(m.apps.borrow().len(), 2);
        m.invalidate();
        assert!(m.apps.borrow().is_empty());
        assert!(m.invoke("xdg.nope", &[p("/tmp")]).is_err());
    }
}
