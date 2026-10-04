//! ndir-i18n — 메시지 카탈로그(출처: nexa-dir2/crates/nexa-app/src/i18n.rs · 원본 nexa-dir docs/42 §3).
//!
//! dir2와 같은 것: 외부 `.lang`(properties 스타일) 3개 임베드 · 키 단위 사용자 오버레이(`<HOME>/lang/<code>.lang`) ·
//! 추가 언어 발견(`discover`) · 폴백 현재 → en → 키 · `{0}` 자리표 · 재시작 없는 전환(`activate`).
//! dir2와 다른 것(DR-14 · docs/port/43 §2):
//! - 활성 표는 **프로세스 전역**(`RwLock`) — dir2는 UI `thread_local`이라 워커 스레드의 `tr()`가 영어로 나왔다.
//! - OS 언어 감지(`syslang`)가 3-OS(dir2는 Windows `GetUserDefaultLocaleName`만).
//! - 키 파리티·자리표·`=` 없는 줄 검사를 **빌드 시**(`build.rs`)에 — 시험이 아니라 컴파일이 막는다.
//! - 오버레이 폴더 = 설정 폴더(`ndir-settings::config_dir()`)의 `lang/` — 호출자가 경로를 넘긴다(이 크레이트는 의존 0).

use std::collections::HashMap;
use std::path::Path;
use std::sync::{OnceLock, RwLock};

pub mod syslang;

/// 내장 언어(빌드 산출물에 임베드 — `lang/` 폴더 없이도 붕괴하지 않는 안전망).
const BUILTIN_EN: &str = include_str!("../lang/en.lang");
const BUILTIN_KO: &str = include_str!("../lang/ko.lang");
const BUILTIN_JA: &str = include_str!("../lang/ja.lang");

/// 내장 언어 코드와 자기 언어 표기(발견 목록의 앞자리 · 설정 창 `ui.lang` 후보).
pub const BUILTIN: &[(&str, &str)] = &[("en", "English"), ("ko", "한국어"), ("ja", "日本語")];

/// 내장 키 수(빌드 검사가 센 값 — 스모크·자가 점검이 표 완전성 지표로 쓴다).
pub const KEY_COUNT: usize = match usize::from_str_radix(env!("NDIR_I18N_KEYS"), 10) {
    Ok(n) => n,
    Err(_) => 0,
};

fn builtin(code: &str) -> Option<&'static str> {
    match code {
        "en" => Some(BUILTIN_EN),
        "ko" => Some(BUILTIN_KO),
        "ja" => Some(BUILTIN_JA),
        _ => None,
    }
}

/// `.lang` 파일 1개의 파싱 결과 — `@` 메타 + 문자열 테이블.
#[derive(Default, Debug)]
pub struct LangFile {
    pub meta: HashMap<String, String>,
    pub strings: HashMap<String, String>,
}

/// 값 이스케이프 해석: `\n`·`\t`·`\\`. 그 외 시퀀스는 리터럴 유지(`data\settings.cfg` 같은 값 보호).
fn unescape(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut chars = v.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// 파싱 규칙: BOM 스킵 · `#` 주석 · 빈 줄 무시 · 첫 `=` 분리 · 키/값 trim · 중복은 마지막 승리 ·
/// 깨진 줄(`=` 없음)은 스킵(라인 격리) · `@` 메타 값의 후행 `# 주석`은 제거.
pub fn parse(text: &str) -> LangFile {
    let mut f = LangFile::default();
    for line in text.strip_prefix('\u{feff}').unwrap_or(text).lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue; // 파손 줄 스킵
        };
        let (k, v) = (k.trim(), v.trim());
        if let Some(mk) = k.strip_prefix('@') {
            let v = v.split('#').next().unwrap_or("").trim();
            f.meta.insert(mk.trim().to_string(), v.to_string());
        } else {
            f.strings.insert(k.to_string(), unescape(v));
        }
    }
    f
}

/// 활성 언어 — 현재 테이블 + en 폴백 테이블(폴백 체인: 현재 → en → 키).
#[derive(Debug)]
pub struct Lang {
    code: String,
    table: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

impl Lang {
    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.table
            .get(key)
            .or_else(|| self.fallback.get(key))
            .map(String::as_str)
    }

    /// 내장 en만(테스트·기동 초기 · 오버레이 없음).
    fn builtin_en() -> Self {
        Lang {
            code: "en".into(),
            table: parse(BUILTIN_EN).strings,
            fallback: HashMap::new(),
        }
    }
}

/// 코드의 테이블 = 내장(있으면) 위에 사용자 `<home>/lang/{code}.lang`을 **키 단위** 덮어쓰기.
fn merged_table(code: &str, home: &Path) -> HashMap<String, String> {
    let mut t = builtin(code).map(|s| parse(s).strings).unwrap_or_default();
    let user = home.join("lang").join(format!("{code}.lang"));
    if let Ok(text) = std::fs::read_to_string(&user) {
        for (k, v) in parse(&text).strings {
            t.insert(k, v);
        }
    }
    t
}

/// 언어 로드 — 폴백은 en(기준 언어). en 자신은 폴백 없음. `home` = 설정 폴더(오버레이 `lang/` 하위).
pub fn load(code: &str, home: &Path) -> Lang {
    load_with_system(code, "", home)
}

/// 언어 로드 + **대체 언어 순서 = 시스템 기본 언어 → 영어**(사용자 10-04 결정 — 언어 파일의 `@fallback` 지정은 쓰지 않는다):
/// 고른 언어에 없는 글은 시스템 언어의 글로 · 그것도 없으면 영어로. `system` = 시스템 언어 코드(모르면 빈 글 · 쓸 수 있는
/// 언어가 아니면 건너뛴다 — 표가 비면 영어만 남는다).
pub fn load_with_system(code: &str, system: &str, home: &Path) -> Lang {
    let mut fallback = if code == "en" {
        HashMap::new()
    } else {
        merged_table("en", home)
    };
    if !system.is_empty() && system != code && system != "en" {
        for (k, v) in merged_table(system, home) {
            fallback.insert(k, v);
        }
    }
    Lang {
        code: code.to_string(),
        table: merged_table(code, home),
        fallback,
    }
}

/// 사용 가능한 언어 발견: 내장 + `<home>/lang/*.lang`(파일명 = 코드 · `@name` 표기).
/// 반환 (code, 자기 언어 표기). 내장이 앞, 추가 발견분은 코드순.
pub fn discover(home: &Path) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = BUILTIN
        .iter()
        .map(|(c, n)| (c.to_string(), n.to_string()))
        .collect();
    let mut extra: Vec<(String, String)> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(home.join("lang")) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_none_or(|x| x != "lang") {
                continue;
            }
            let Some(code) = p.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if builtin(code).is_some() {
                continue; // 내장 오버라이드는 목록 중복 없이 병합만
            }
            let name = std::fs::read_to_string(&p)
                .ok()
                .and_then(|t| parse(&t).meta.get("name").cloned())
                .unwrap_or_else(|| code.to_string());
            extra.push((code.to_string(), name));
        }
    }
    extra.sort();
    out.extend(extra);
    out
}

/// 설정값 → 실제 코드. `system` = OS 로캘의 1차 서브태그(`ko-KR` → `ko`), 미보유 시 en(기준).
pub fn resolve_code(setting: &str, system: &str, available: &[(String, String)]) -> String {
    let want = if setting == "system" {
        system.split(['-', '_', '.']).next().unwrap_or("en")
    } else {
        setting
    };
    if available.iter().any(|(c, _)| c == want) {
        want.to_string()
    } else {
        "en".to_string()
    }
}

/// 활성 표(프로세스 전역). 기본 = 내장 en — 테스트·기동 초기에도 빈 키 붕괴 없음.
fn active() -> &'static RwLock<Lang> {
    static ACTIVE: OnceLock<RwLock<Lang>> = OnceLock::new();
    ACTIVE.get_or_init(|| RwLock::new(Lang::builtin_en()))
}

/// 언어 전환 — 테이블 스왑. 호출 측이 메뉴/컬럼 재구성 + 전체 재그리기를 수행한다.
pub fn activate(lang: Lang) {
    let mut g = active()
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *g = lang;
}

/// 활성 언어 코드.
pub fn current_code() -> String {
    active()
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .code
        .clone()
}

/// 키 → 문자열(현재 → en 폴백 → 키 그대로).
pub fn tr(key: &str) -> String {
    let g = active()
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    g.get(key)
        .map(str::to_string)
        .unwrap_or_else(|| key.to_string())
}

/// 키가 활성 표(폴백 포함)에 있는가 — 자가 점검·동적 키 조립의 사전 확인용.
pub fn has(key: &str) -> bool {
    active()
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(key)
        .is_some()
}

/// `{0}`·`{1}`… 자리표 치환.
pub fn trf(key: &str, args: &[&str]) -> String {
    let mut s = tr(key);
    for (i, a) in args.iter().enumerate() {
        s = s.replace(&format!("{{{i}}}"), a);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 대체 언어 순서(사용자 10-04 결정): 고른 언어에 없는 글 = 시스템 기본 언어 → 영어. 시스템 언어를 모르거나 쓸 수 없으면 영어.
    #[test]
    fn fallback_is_system_language_then_english() {
        let home = std::env::temp_dir().join(format!("ndir-i18n-fb-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join("lang")).unwrap();
        // 사용자 언어 파일: 글 하나만 번역했다.
        std::fs::write(
            home.join("lang").join("xx.lang"),
            "@name = Test\nmenu.file.exit = XX-EXIT\n",
        )
        .unwrap();
        let (ko, en) = (load("ko", &home), load("en", &home));
        let key = "menu.file.prefs";
        assert_ne!(
            ko.get(key),
            en.get(key),
            "두 언어의 글이 달라야 시험이 뜻이 있다"
        );
        let with_ko = load_with_system("xx", "ko", &home);
        assert_eq!(
            with_ko.get("menu.file.exit"),
            Some("XX-EXIT"),
            "고른 언어가 먼저"
        );
        assert_eq!(with_ko.get(key), ko.get(key), "없으면 시스템 언어");
        assert_eq!(load_with_system("xx", "", &home).get(key), en.get(key));
        assert_eq!(load_with_system("xx", "zz", &home).get(key), en.get(key));
        assert_eq!(load_with_system("xx", "en", &home).get(key), en.get(key));
        // 고른 언어 = 시스템 언어면 영어만 대체로 남는다 · load()는 종전과 같다.
        assert_eq!(load_with_system("ko", "ko", &home).get(key), ko.get(key));
        assert_eq!(load("xx", &home).get(key), en.get(key));
        let _ = std::fs::remove_dir_all(&home);
    }

    /// 전역 표를 바꾸는 시험은 직렬화한다(시험은 병렬로 돈다 — nexa-ui 규칙).
    static GLOBAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn parse_rules_meta_comment_escape_dup() {
        let f = parse(
            "\u{feff}# 주석\n@code = ko   # 후행 주석\n@name= 한국어\n\na.b = 값1\nbroken line\na.b = 값2\nesc = 줄\\n바꿈\\t탭\\\\역슬래시\nlit = 그대로\\x\n",
        );
        assert_eq!(f.meta["code"], "ko");
        assert_eq!(f.meta["name"], "한국어");
        assert_eq!(f.strings["a.b"], "값2", "중복 키 = 마지막 승리");
        assert_eq!(f.strings["esc"], "줄\n바꿈\t탭\\역슬래시");
        assert_eq!(f.strings["lit"], "그대로\\x", "미지 이스케이프는 리터럴");
        assert_eq!(f.strings.len(), 3, "파손 줄 스킵");
    }

    #[test]
    fn builtin_langs_parse_and_key_parity() {
        let en = parse(BUILTIN_EN);
        assert_eq!(en.meta["code"], "en");
        assert_eq!(en.strings.len(), KEY_COUNT, "빌드 검사가 센 키 수와 일치");
        let n = en.strings.len();
        assert!(n >= 490, "dir2 498키 계승(KEY-1601~): {n}");
        for (code, text) in [("ko", BUILTIN_KO), ("ja", BUILTIN_JA)] {
            let l = parse(text);
            assert_eq!(l.meta["code"], code);
            for k in en.strings.keys() {
                assert!(l.strings.contains_key(k), "{code}.lang에 키 누락: {k}");
            }
            for k in l.strings.keys() {
                assert!(en.strings.contains_key(k), "en.lang에 키 누락({code}): {k}");
            }
        }
    }

    /// 원장 전수(T-90 · docs/port/31 §2-6 KEY-1001~1498): dir2 i18n 키 **전부**가 세 언어 내장 카탈로그에 있다(자원 유지 — CLAUDE.md §1).
    /// 원장 표 행 = `| KEY-NNNN | `키` | en | ko | 비고 |`.
    #[test]
    fn dir2_catalog_i18n_keys_present_in_all_langs() {
        let doc = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/port/31-catalog-settings-i18n.md"
        ))
        .expect("docs/port/31");
        let mut keys: Vec<String> = Vec::new();
        let mut in_sec = false;
        for line in doc.lines() {
            if line.starts_with("### 2-6.") {
                in_sec = true;
                continue;
            }
            if in_sec && line.starts_with("## 3.") {
                break;
            }
            if !in_sec || !line.starts_with("| KEY-1") {
                continue;
            }
            let mut cells = line.split('|').map(str::trim);
            let _ = cells.next();
            let _ = cells.next();
            if let Some(k) = cells.next() {
                let k = k.trim_matches('`');
                if !k.is_empty() {
                    keys.push(k.to_string());
                }
            }
        }
        assert!(keys.len() >= 490, "원장 i18n 행 수: {}", keys.len());
        for (code, text) in [("en", BUILTIN_EN), ("ko", BUILTIN_KO), ("ja", BUILTIN_JA)] {
            let l = parse(text);
            let missing: Vec<&String> = keys
                .iter()
                .filter(|k| !l.strings.contains_key(k.as_str()))
                .collect();
            assert!(
                missing.is_empty(),
                "{code}.lang에 원장 키 누락 {}개: {missing:?}",
                missing.len()
            );
        }
    }

    /// port/31 §2-3 결함 수정 확인: `del.lockedMsg`/`del.failMsg`의 `{1}`이 값 안에 있다.
    #[test]
    fn locked_and_fail_messages_carry_list_placeholder() {
        for text in [BUILTIN_EN, BUILTIN_KO, BUILTIN_JA] {
            let f = parse(text);
            for k in ["del.lockedMsg", "del.failMsg"] {
                let v = &f.strings[k];
                assert!(v.contains("{0}") && v.ends_with("\n\n{1}"), "{k}: {v:?}");
            }
        }
    }

    #[test]
    fn merge_override_fallback_and_resolve() {
        let dir = std::env::temp_dir().join(format!("ndir_lang_{}", std::process::id()));
        let sub = dir.join("lang");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("ko.lang"), "menu.file = 화일\n").unwrap();
        std::fs::write(
            sub.join("fr.lang"),
            "@code = fr\n@name = Français\nmenu.file = Fichier\n",
        )
        .unwrap();

        let ko = load("ko", &dir);
        assert_eq!(
            ko.get("menu.file"),
            Some("화일"),
            "사용자 키 단위 오버라이드"
        );
        assert_eq!(ko.get("menu.view"), Some("보기"), "나머지는 내장 유지");
        let ja = load("ja", &dir);
        assert_eq!(ja.get("menu.file"), Some("ファイル"));
        let fr = load("fr", &dir);
        assert_eq!(fr.get("menu.file"), Some("Fichier"));
        assert_eq!(fr.get("menu.view"), Some("View"), "누락 키 = en 폴백");
        assert_eq!(fr.get("no.such.key"), None);

        let avail = discover(&dir);
        assert_eq!(avail[0].0, "en");
        assert!(avail.iter().any(|(c, n)| c == "ja" && n == "日本語"));
        assert!(avail.iter().any(|(c, n)| c == "fr" && n == "Français"));
        assert_eq!(
            avail.iter().filter(|(c, _)| c == "ko").count(),
            1,
            "오버라이드는 중복 미등재"
        );

        assert_eq!(resolve_code("system", "ko-KR", &avail), "ko");
        assert_eq!(
            resolve_code("system", "ko_KR.UTF-8", &avail),
            "ko",
            "유닉스 로캘"
        );
        assert_eq!(resolve_code("system", "ja-JP", &avail), "ja");
        assert_eq!(resolve_code("system", "de-DE", &avail), "en", "미보유 = en");
        assert_eq!(resolve_code("fr", "ko-KR", &avail), "fr");
        assert_eq!(resolve_code("zz", "ko-KR", &avail), "en");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn trf_placeholders_and_global_switch() {
        let _g = GLOBAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        activate(Lang::builtin_en());
        assert_eq!(current_code(), "en");
        assert_eq!(trf("status.tab", &["2", "3"]), "Tab 2/3");
        assert_eq!(trf("kind.extFile", &["TXT"]), "TXT file");
        assert_eq!(tr("no.such.key"), "no.such.key", "최후 폴백 = 키");
        assert!(has("menu.file") && !has("no.such.key"));

        let nowhere = std::env::temp_dir().join("ndir_lang_nowhere_zzz");
        activate(load("ko", &nowhere));
        assert_eq!(current_code(), "ko");
        assert_eq!(tr("menu.file"), "파일");
        // 워커 스레드에서도 같은 표(dir2 thread_local과 다른 점).
        let from_worker = std::thread::spawn(|| tr("menu.file")).join().unwrap();
        assert_eq!(from_worker, "파일");
        activate(Lang::builtin_en());
    }
}
