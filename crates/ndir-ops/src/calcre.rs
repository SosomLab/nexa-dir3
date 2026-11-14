//! 계산 컬럼의 정규식 함수(NEW-060 후속 · 11-14) — `ndir_core::calc`의 확장 호출([`ndir_core::calc::Env::call_ext`]) 구현.
//!
//! 코어는 의존 0이라 정규식 엔진을 품지 않는다 → 이미 `regex-lite`를 쓰는 이 크레이트가 맡는다(DR-8 · 새 의존 없음).
//! 모양은 파이썬 `re` 모듈을 따르되 **Match 객체는 없다**: 찾기 함수는 찾은 글(또는 고른 그룹)을 돌려주고 없으면 `None`.
//!
//! | 함수 | 뜻 |
//! | --- | --- |
//! | `re.search(pat, s[, group])` | 첫 일치의 글(그룹 번호를 주면 그 그룹) · 없으면 `None` |
//! | `re.match(pat, s[, group])` | 글 맨 앞에서만 |
//! | `re.fullmatch(pat, s[, group])` | 글 전체가 맞아야 |
//! | `re.sub(pat, repl, s[, count])` | 바꾸기 — `repl`의 `\1` · `\g<이름>` = 그룹 |
//! | `re.findall(pat, s)` | 일치 목록(그룹이 정확히 1개면 그 그룹) |
//! | `re.split(pat, s)` | 나눈 목록 |
//!
//! 대소문자 무시 등은 패턴 안 `(?i)`로. 패턴은 [`MAX_PATTERN`]자까지 · 컴파일한 것은 스레드별로 [`CACHE`]개까지 기억한다
//! (행마다 같은 패턴을 다시 컴파일하지 않게).

use ndir_core::calc::{CalcError, Value};
use regex_lite::Regex;
use std::cell::RefCell;

/// 패턴 길이 상한(글자).
pub const MAX_PATTERN: usize = 500;
/// 스레드별 컴파일 캐시 크기.
const CACHE: usize = 16;
/// 결과 목록 상한(코어의 목록 한도와 같다).
const MAX_LIST: usize = 10_000;

thread_local! {
    static COMPILED: RefCell<Vec<(String, Regex)>> = const { RefCell::new(Vec::new()) };
}

fn err<T>(msg: impl Into<String>) -> Result<T, CalcError> {
    Err(CalcError(msg.into()))
}

/// 컴파일(캐시 · 최근 쓴 것이 뒤).
fn compiled(pat: &str) -> Result<Regex, CalcError> {
    if pat.chars().count() > MAX_PATTERN {
        return err("regex pattern too long");
    }
    COMPILED.with(|c| {
        let mut c = c.borrow_mut();
        if let Some(i) = c.iter().position(|(p, _)| p == pat) {
            let hit = c.remove(i);
            let re = hit.1.clone();
            c.push(hit);
            return Ok(re);
        }
        let re = Regex::new(pat).map_err(|e| CalcError(format!("bad regex: {e}")))?;
        if c.len() >= CACHE {
            c.remove(0);
        }
        c.push((pat.to_owned(), re.clone()));
        Ok(re)
    })
}

fn text<'a>(name: &str, v: &'a Value) -> Result<&'a str, CalcError> {
    match v {
        Value::Str(s) => Ok(s),
        _ => err(format!("{name}() needs a str")),
    }
}

fn argc(name: &str, a: &[Value], lo: usize, hi: usize) -> Result<(), CalcError> {
    if a.len() < lo || a.len() > hi {
        return err(format!("{name}() takes {lo} to {hi} arguments"));
    }
    Ok(())
}

/// 파이썬 바꿀 글 → regex-lite 꼴: `\1` → `${1}` · `\g<이름>` → `${이름}` · `$` → `$$` · `\\` → `\` · 그 밖의 `\x` = 그대로.
fn python_repl(repl: &str) -> String {
    let cs: Vec<char> = repl.chars().collect();
    let mut out = String::with_capacity(repl.len() + 8);
    let mut i = 0;
    while i < cs.len() {
        match cs[i] {
            '$' => out.push_str("$$"),
            '\\' if i + 1 < cs.len() => {
                let n = cs[i + 1];
                if n.is_ascii_digit() {
                    let mut j = i + 1;
                    let mut num = String::new();
                    while j < cs.len() && cs[j].is_ascii_digit() && num.len() < 2 {
                        num.push(cs[j]);
                        j += 1;
                    }
                    out.push_str(&format!("${{{num}}}"));
                    i = j;
                    continue;
                }
                if n == 'g' && cs.get(i + 2) == Some(&'<') {
                    if let Some(end) = cs[i + 3..].iter().position(|&c| c == '>') {
                        let name: String = cs[i + 3..i + 3 + end].iter().collect();
                        out.push_str(&format!("${{{name}}}"));
                        i += 4 + end;
                        continue;
                    }
                }
                if n == '\\' {
                    out.push('\\');
                } else {
                    out.push('\\');
                    out.push(n);
                }
                i += 2;
                continue;
            }
            c => out.push(c),
        }
        i += 1;
    }
    out
}

/// 찾기 3종의 공통 — `anchor`: 0 = 어디서나 · 1 = 맨 앞 · 2 = 전체.
fn find(name: &str, a: &[Value], anchor: u8) -> Result<Value, CalcError> {
    argc(name, a, 2, 3)?;
    let re = compiled(text(name, &a[0])?)?;
    let s = text(name, &a[1])?;
    let group = match a.get(2) {
        None => 0usize,
        Some(Value::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(usize::MAX),
        Some(_) => return err(format!("{name}(): group must be a non-negative integer")),
    };
    if group >= re.captures_len() {
        return err(format!("{name}(): no such group {group}"));
    }
    let Some(caps) = re.captures(s) else {
        return Ok(Value::None);
    };
    let whole = caps.get(0).expect("group 0 always exists");
    let ok = match anchor {
        0 => true,
        1 => whole.start() == 0,
        _ => whole.start() == 0 && whole.end() == s.len(),
    };
    if !ok {
        // 맨 앞/전체 조건은 첫 일치로만 판정하면 놓칠 수 있다(더 뒤에서 시작하는 일치가 먼저 잡힌 경우는 없지만, 더 짧은 대안이
        // 먼저 잡힌 경우) → 닻을 건 패턴으로 다시.
        let anchored = if anchor == 1 {
            format!(r"\A(?:{})", re.as_str())
        } else {
            format!(r"\A(?:{})\z", re.as_str())
        };
        let re2 = compiled(&anchored)?;
        return Ok(match re2.captures(s) {
            Some(c) => c
                .get(group)
                .map_or(Value::None, |m| Value::Str(m.as_str().to_owned())),
            None => Value::None,
        });
    }
    Ok(caps
        .get(group)
        .map_or(Value::None, |m| Value::Str(m.as_str().to_owned())))
}

/// 확장 호출 — 이 모듈의 함수면 `Some(결과)` · 아니면 `None`(코어가 "unknown function").
pub fn call(name: &str, a: &[Value]) -> Option<Result<Value, CalcError>> {
    Some(match name {
        "re.search" => find(name, a, 0),
        "re.match" => find(name, a, 1),
        "re.fullmatch" => find(name, a, 2),
        "re.sub" => (|| {
            argc(name, a, 3, 4)?;
            let re = compiled(text(name, &a[0])?)?;
            let repl = python_repl(text(name, &a[1])?);
            let s = text(name, &a[2])?;
            let count = match a.get(3) {
                None => 0usize,
                Some(Value::Int(n)) if *n >= 0 => usize::try_from(*n).unwrap_or(0),
                Some(_) => return err("re.sub(): count must be a non-negative integer"),
            };
            Ok(Value::Str(
                re.replacen(s, count, repl.as_str()).into_owned(),
            ))
        })(),
        "re.findall" => (|| {
            argc(name, a, 2, 2)?;
            let re = compiled(text(name, &a[0])?)?;
            let s = text(name, &a[1])?;
            let group = usize::from(re.captures_len() == 2);
            let out: Vec<Value> = re
                .captures_iter(s)
                .take(MAX_LIST + 1)
                .map(|c| Value::Str(c.get(group).map_or("", |m| m.as_str()).to_owned()))
                .collect();
            if out.len() > MAX_LIST {
                return err("re.findall(): too many matches");
            }
            Ok(Value::List(out))
        })(),
        "re.split" => (|| {
            argc(name, a, 2, 2)?;
            let re = compiled(text(name, &a[0])?)?;
            let s = text(name, &a[1])?;
            let out: Vec<Value> = re
                .split(s)
                .take(MAX_LIST + 1)
                .map(|p| Value::Str(p.to_owned()))
                .collect();
            if out.len() > MAX_LIST {
                return err("re.split(): too many parts");
            }
            Ok(Value::List(out))
        })(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndir_core::calc::{Env, Expr};

    struct E;
    impl Env for E {
        fn var(&self, name: &str) -> Option<Value> {
            (name == "name").then(|| Value::Str("IMG_2024-03-17_0042.final.JPG".into()))
        }
        fn call_ext(&self, name: &str, args: &[Value]) -> Option<Result<Value, CalcError>> {
            call(name, args)
        }
    }

    fn ev(src: &str) -> Value {
        Expr::parse(src)
            .unwrap_or_else(|e| panic!("{src}: {e}"))
            .eval(&E)
            .unwrap_or_else(|e| panic!("{src}: {e}"))
    }

    fn s(v: &str) -> Value {
        Value::Str(v.into())
    }

    /// 파이썬 `re`와 같은 결과(Match 객체 대신 글): search · match · fullmatch · 그룹 · 없으면 None.
    #[test]
    fn search_match_fullmatch_follow_python() {
        assert_eq!(ev(r"re.search('\\d{4}', name)"), s("2024"));
        assert_eq!(ev(r"re.search('(\\d+)-(\\d+)', name, 2)"), s("03"));
        assert_eq!(ev(r"re.search('xyz', name)"), Value::None);
        assert_eq!(ev(r"re.match('\\d+', name)"), Value::None, "맨 앞이 아님");
        assert_eq!(ev(r"re.match('[A-Z]+', name)"), s("IMG"));
        assert_eq!(ev(r"re.fullmatch('[A-Z]+', name)"), Value::None);
        assert_eq!(
            ev(r"re.fullmatch('a|ab', 'ab')"),
            s("ab"),
            "짧은 대안에 속지 않는다"
        );
        assert_eq!(ev(r"re.match('b|ab', 'ab')"), s("ab"));
        assert_eq!(ev(r"re.search('(?i)jpg$', name)"), s("JPG"));
        // 조건식 · 정렬 키로 쓰기.
        assert_eq!(
            ev(r"int(re.search('_(\\d+)\\.', name, 1)) if re.search('_(\\d+)\\.', name) else 0"),
            Value::Int(42)
        );
    }

    #[test]
    fn sub_findall_split() {
        assert_eq!(
            ev(r"re.sub('(\\d+)-(\\d+)-(\\d+)', '\\3.\\2.\\1', name)"),
            s("IMG_17.03.2024_0042.final.JPG")
        );
        assert_eq!(ev(r"re.sub('\\d', '#', 'a1b22', 2)"), s("a#b#2"));
        assert_eq!(
            ev(r"re.sub('(?P<y>\\d{4})', '<\\g<y>>', 'x2024')"),
            s("x<2024>")
        );
        assert_eq!(
            ev(r"re.sub('a', '$1', 'banana')"),
            s("b$1n$1n$1"),
            "$는 글자"
        );
        assert_eq!(
            ev(r"re.findall('\\d+', name)"),
            Value::List(vec![s("2024"), s("03"), s("17"), s("0042")])
        );
        assert_eq!(
            ev(r"re.findall('_(\\d+)', name)"),
            Value::List(vec![s("2024"), s("0042")]),
            "그룹 1개 = 그 그룹"
        );
        assert_eq!(ev(r"len(re.split('[._-]', name))"), Value::Int(7));
        assert_eq!(ev(r"re.split(',\\s*', 'a, b,c')[1]"), s("b"));
    }

    #[test]
    fn errors_are_reported_not_panicked() {
        let fails = |src: &str| Expr::parse(src).unwrap().eval(&E).unwrap_err().0;
        assert!(fails("re.search('(', name)").contains("bad regex"));
        assert!(fails("re.search('a', 1)").contains("needs a str"));
        assert!(fails("re.search('a', name, 3)").contains("no such group"));
        assert!(fails("re.sub('a', name)").contains("arguments"));
        assert!(fails("re.nope('a', name)").contains("unknown function"));
        assert!(fails(&format!("re.search('{}', name)", "a".repeat(600))).contains("too long"));
        // 캐시가 넘쳐도 동작한다.
        for i in 0..40 {
            assert_eq!(ev(&format!("re.search('{i}x?', '{i}')")), s(&i.to_string()));
        }
    }
}
