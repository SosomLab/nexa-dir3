//! OS 화면 언어 → 로캘 코드(출처: nexa-sql/crates/nsql-i18n/src/syslang.rs — 열거형 `Lang` 대신 **코드 문자열**을 돌려준다.
//! dir2는 추가 언어를 `.lang` 파일로 발견하므로(DR-14) 지원 언어가 열거형으로 닫혀 있지 않다 → 판정은 `resolve_code`가 한다).
//!
//! 원천(외부 crate 0):
//! - Windows = `GetUserDefaultUILanguage`(kernel32 · 표시 언어) — 시작 메뉴에서 띄운 GUI에는 `LANG`이 없다.
//! - macOS = `CFLocaleCopyPreferredLanguages`(CoreFoundation · 시스템 설정 ▸ 언어 목록의 첫째).
//! - 그 밖(Linux 등) = `LANGUAGE`(콜론 목록의 첫째) → `LC_ALL` → `LC_MESSAGES` → `LANG`(gettext 우선순위 · `C`/`POSIX`는 없음).

use std::sync::OnceLock;

/// 이 프로세스의 OS 언어 로캘(`ko-KR` · `en-US` · `ja` …). 모르면 `None`. 한 번만 묻는다(이후 캐시).
#[must_use]
pub fn system_locale() -> Option<&'static str> {
    static CACHE: OnceLock<Option<String>> = OnceLock::new();
    CACHE.get_or_init(detect).as_deref()
}

/// `system_locale()`의 1차 서브태그(`ko`) — 없으면 `en`.
#[must_use]
pub fn system_lang_code() -> String {
    system_locale()
        .and_then(|l| l.split(['-', '_', '.']).next())
        .filter(|s| !s.is_empty())
        .unwrap_or("en")
        .to_string()
}

/// 로캘 문자열 정규화 — `C`·`POSIX`·빈 값 = `None` · `ko_KR.UTF-8@euro` → `ko_KR`.
#[must_use]
pub fn normalize_locale(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("C") || s.eq_ignore_ascii_case("POSIX") {
        return None;
    }
    Some(s.split(['.', '@']).next().unwrap_or(s).to_string())
}

/// Windows LANGID → 로캘 코드(주 언어 = 하위 10비트 · 0x12 한국어 · 0x09 영어 · 0x11 일본어 · 그 외 = 모름).
#[must_use]
pub fn locale_from_langid(id: u16) -> Option<&'static str> {
    match id & 0x3ff {
        0x12 => Some("ko"),
        0x09 => Some("en"),
        0x11 => Some("ja"),
        _ => None,
    }
}

/// 유닉스 계열 환경 변수 순서(gettext): `LANGUAGE`(콜론 목록) → `LC_ALL` → `LC_MESSAGES` → `LANG`. 첫 "값 있는" 변수로 판정.
#[must_use]
pub fn locale_from_env(get: impl Fn(&str) -> Option<String>) -> Option<String> {
    for key in ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"] {
        let Some(v) = get(key).filter(|v| !v.trim().is_empty()) else {
            continue;
        };
        let first = if key == "LANGUAGE" {
            v.split(':')
                .find(|p| !p.trim().is_empty())
                .unwrap_or("")
                .to_string()
        } else {
            v
        };
        // 첫 "값 있는" 변수가 답이다 — C/POSIX면 다음 변수로 넘어가지 않고 "모름"(→ 영어).
        return normalize_locale(&first);
    }
    None
}

#[cfg(windows)]
fn detect() -> Option<String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }
    // SAFETY: 인자 없는 조회 함수 · 전역 상태를 바꾸지 않는다.
    let id = unsafe { GetUserDefaultUILanguage() };
    locale_from_langid(id).map(str::to_string)
}

#[cfg(target_os = "macos")]
fn detect() -> Option<String> {
    use std::ffi::{c_char, c_void, CStr};
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFLocaleCopyPreferredLanguages() -> *const c_void;
        fn CFArrayGetCount(a: *const c_void) -> isize;
        fn CFArrayGetValueAtIndex(a: *const c_void, i: isize) -> *const c_void;
        fn CFStringGetCString(s: *const c_void, buf: *mut c_char, len: isize, enc: u32) -> u8;
        fn CFRelease(p: *const c_void);
    }
    const UTF8: u32 = 0x0800_0100;
    // SAFETY: Copy 규칙 = 받은 배열은 우리가 CFRelease · 원소는 빌린 참조(배열이 살아 있는 동안만 읽는다) · 버퍼 길이를 넘겨 준다.
    unsafe {
        let arr = CFLocaleCopyPreferredLanguages();
        if arr.is_null() {
            return None;
        }
        let mut out = None;
        if CFArrayGetCount(arr) > 0 {
            let s = CFArrayGetValueAtIndex(arr, 0);
            let mut buf = [0 as c_char; 64];
            if !s.is_null()
                && CFStringGetCString(s, buf.as_mut_ptr(), buf.len() as isize, UTF8) != 0
            {
                let code = CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned();
                out = normalize_locale(&code);
            }
        }
        CFRelease(arr);
        out
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn detect() -> Option<String> {
    locale_from_env(|k| std::env::var(k).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_strings_normalize() {
        assert_eq!(normalize_locale("ko_KR.UTF-8").as_deref(), Some("ko_KR"));
        assert_eq!(normalize_locale("ko-KR").as_deref(), Some("ko-KR"));
        assert_eq!(
            normalize_locale("ja_JP.UTF-8@mod").as_deref(),
            Some("ja_JP")
        );
        assert_eq!(normalize_locale("C"), None);
        assert_eq!(normalize_locale("POSIX"), None);
        assert_eq!(normalize_locale(""), None);
    }

    #[test]
    fn windows_langids() {
        assert_eq!(locale_from_langid(0x0412), Some("ko"), "ko-KR");
        assert_eq!(locale_from_langid(0x0409), Some("en"), "en-US");
        assert_eq!(locale_from_langid(0x0809), Some("en"), "en-GB");
        assert_eq!(
            locale_from_langid(0x0411),
            Some("ja"),
            "ja-JP(dir2 내장 — nexa-sql과 다른 점)"
        );
        assert_eq!(locale_from_langid(0x0407), None, "de-DE");
    }

    #[test]
    fn env_priority_follows_gettext() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |k: &str| {
                pairs
                    .iter()
                    .find(|(n, _)| *n == k)
                    .map(|(_, v)| (*v).to_string())
            }
        };
        assert_eq!(
            locale_from_env(env(&[("LANG", "ko_KR.UTF-8")])).as_deref(),
            Some("ko_KR")
        );
        assert_eq!(
            locale_from_env(env(&[("LANG", "en_US.UTF-8"), ("LC_ALL", "ko_KR.UTF-8")])).as_deref(),
            Some("ko_KR"),
            "LC_ALL이 LANG보다 앞"
        );
        assert_eq!(
            locale_from_env(env(&[("LANGUAGE", "ko:en"), ("LANG", "en_US.UTF-8")])).as_deref(),
            Some("ko"),
            "LANGUAGE 목록의 첫째"
        );
        assert_eq!(
            locale_from_env(env(&[("LANG", "fr_FR.UTF-8")])).as_deref(),
            Some("fr_FR"),
            "지원 여부는 resolve_code가 판정"
        );
        assert_eq!(locale_from_env(env(&[("LANG", "C")])), None);
        assert_eq!(locale_from_env(env(&[])), None);
    }

    #[test]
    fn system_lang_code_is_stable() {
        let a = system_lang_code();
        assert!(!a.is_empty());
        assert_eq!(a, system_lang_code(), "캐시 = 같은 값");
    }
}
