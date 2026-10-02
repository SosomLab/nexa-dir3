//! 빌드 시 언어 파일 검사(DR-14 · docs/port/43 EXT-203의 "생성기 검사"만 — 표 생성은 하지 않는다).
//!
//! 실패 조건(= 빌드 실패):
//! 1. 언어 간 키 집합 불일치(en 기준 · ko·ja 누락/초과)
//! 2. en 값이 비어 있음
//! 3. 자리표(`{0}`·`{1}`…) 집합이 언어 간 다름
//! 4. `=` 없는 비주석·비공백 줄(dir2 `del.lockedMsg`의 `{1}` 단독 줄 결함 재발 방지 — port/31 §2-3)
//! 5. 같은 파일 안 중복 키
//!
//! 외부 crate 0 · 파서는 `src/lib.rs`와 같은 규칙(BOM · `#` · 첫 `=` 분리 · trim · `@` 메타).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const LANGS: &[&str] = &["en", "ko", "ja"];

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("lang");
    let mut tables: BTreeMap<&str, BTreeMap<String, String>> = BTreeMap::new();
    let mut errors = Vec::new();
    for code in LANGS {
        let path = dir.join(format!("{code}.lang"));
        println!("cargo:rerun-if-changed={}", path.display());
        let text =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let mut t = BTreeMap::new();
        for (n, raw) in text
            .strip_prefix('\u{feff}')
            .unwrap_or(&text)
            .lines()
            .enumerate()
        {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                errors.push(format!("{code}.lang:{}: '=' 없는 줄 — \"{line}\"", n + 1));
                continue;
            };
            let (k, v) = (k.trim(), v.trim());
            if k.starts_with('@') {
                continue;
            }
            if t.insert(k.to_string(), v.to_string()).is_some() {
                errors.push(format!("{code}.lang:{}: 중복 키 {k}", n + 1));
            }
        }
        tables.insert(code, t);
    }
    let en = &tables["en"];
    for (k, v) in en {
        if v.is_empty() {
            errors.push(format!("en.lang: 빈 값 {k}"));
        }
    }
    for code in LANGS.iter().filter(|c| **c != "en") {
        let t = &tables[code];
        for k in en.keys() {
            if !t.contains_key(k) {
                errors.push(format!("{code}.lang: 키 누락 {k}"));
            }
        }
        for k in t.keys() {
            if !en.contains_key(k) {
                errors.push(format!("{code}.lang: en에 없는 키 {k}"));
            }
        }
        for (k, v) in t {
            if let Some(ev) = en.get(k) {
                if placeholders(ev) != placeholders(v) {
                    errors.push(format!(
                        "{code}.lang: 자리표 불일치 {k} — en {:?} vs {code} {:?}",
                        placeholders(ev),
                        placeholders(v)
                    ));
                }
            }
        }
    }
    if !errors.is_empty() {
        panic!(
            "ndir-i18n 언어 파일 검사 실패 {}건:\n  {}",
            errors.len(),
            errors.join("\n  ")
        );
    }
    println!("cargo:rustc-env=NDIR_I18N_KEYS={}", en.len());
}

fn placeholders(v: &str) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    let b = v.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'{' {
            let mut j = i + 1;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 && j < b.len() && b[j] == b'}' {
                if let Ok(n) = v[i + 1..j].parse() {
                    out.insert(n);
                }
                i = j;
            }
        }
        i += 1;
    }
    out
}
