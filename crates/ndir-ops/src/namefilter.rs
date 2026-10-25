//! 빠른 필터 이름 판정기(사용자 10-25 "nexa-sql 방식의 대소문자/단어/정규식 설정이 들어간 검색창 · 그냥 검색 = Fuzzy"): 패널 필터 바가
//! 글과 토글 셋으로 [`NameMatcher`]를 만들어 ndir-tree에 넣는다(ndir-tree는 regex 의존이 없어 판정기만 받는다 · DR-8 regex-lite = ops).
//!
//! 규칙(nexa-sql `filterbar`/`rx::compile` 계승 + dir2 글롭):
//! - 글을 `;`로 나눈 조각 중 **하나라도** 맞으면 통과(OR · 종전 NEW-057 그대로) · 빈 글 = 판정기 없음(호출자가 끔).
//! - **정규식** 켬: 조각 = 정규식(`regex-lite`) · 대소문자 끔이면 `(?i)` · 단어 켬이면 `\b…\b` · 컴파일 실패 = [`compile`] `Err`(호출자가 틀을 붉게).
//! - 정규식 끔 · 조각에 `*`/`?`가 있으면 **글롭**(전체 일치 · 대소문자는 토글대로).
//! - 정규식 끔 · **단어** 켬: 조각이 낱말 경계(글자·숫자·`_` 아닌 곳) 사이에 통째로 들어 있으면.
//! - 그 밖(기본): **퍼지(Fuzzy)** = 조각의 글자가 이름에 **순서대로** 나타나면(`rprt` → `report` · 대소문자는 토글대로). **한글은 자모열**
//!   (nexa-sql `nsql-core/src/hangul.rs` 규칙 이식 · 협업 검토 10-25): 음절을 타자 순서 자모(초·중·종 · 겹모음/겹받침은 둘)로 펴서 비교하므로
//!   조합 중 글자(`보ㄱ`)나 초성(`ㅂㄱㅅ`)도 `보고서`에 맞는다.

use regex_lite::Regex;

/// 토글 셋(필터 바의 Aa · ab · .*).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FilterOpts {
    pub case: bool,
    pub word: bool,
    pub regex: bool,
}

impl FilterOpts {
    /// 설정 비트(`list.filter_case/word/regex`) · 판정기 열쇠용 짧은 표기.
    #[must_use]
    pub fn tag(self) -> &'static str {
        match (self.case, self.word, self.regex) {
            (false, false, false) => "",
            (true, false, false) => "c",
            (false, true, false) => "w",
            (false, false, true) => "r",
            (true, true, false) => "cw",
            (true, false, true) => "cr",
            (false, true, true) => "wr",
            (true, true, true) => "cwr",
        }
    }
}

#[derive(Debug, Clone)]
enum Rule {
    Fuzzy(Vec<char>),
    Word(Vec<char>),
    Glob(Vec<char>),
    Rx(Regex),
}

/// 컴파일된 판정기(토큰 OR). 빈 글이면 만들지 않는다.
#[derive(Debug, Clone)]
pub struct NameMatcher {
    rules: Vec<Rule>,
    case: bool,
}

/// 글 + 토글 → 판정기. `Ok(None)` = 빈 글(필터 끔) · `Err(msg)` = 정규식 오류(첫 조각의 오류 글).
pub fn compile(spec: &str, opts: FilterOpts) -> Result<Option<NameMatcher>, String> {
    let mut rules = Vec::new();
    for tok in spec.split(';').map(str::trim).filter(|t| !t.is_empty()) {
        let rule = if opts.regex {
            let mut pat = String::new();
            if !opts.case {
                pat.push_str("(?i)");
            }
            if opts.word {
                pat.push_str(r"\b(?:");
                pat.push_str(tok);
                pat.push_str(r")\b");
            } else {
                pat.push_str(tok);
            }
            Rule::Rx(Regex::new(&pat).map_err(|e| e.to_string())?)
        } else {
            let chars: Vec<char> = fold(tok, opts.case).collect();
            if tok.contains(['*', '?']) {
                Rule::Glob(chars)
            } else if opts.word {
                Rule::Word(chars)
            } else {
                Rule::Fuzzy(hangul::decompose_chars(&chars))
            }
        };
        rules.push(rule);
    }
    if rules.is_empty() {
        return Ok(None);
    }
    Ok(Some(NameMatcher {
        rules,
        case: opts.case,
    }))
}

fn fold(s: &str, case: bool) -> impl Iterator<Item = char> + '_ {
    s.chars().flat_map(move |c| {
        if case {
            vec![c]
        } else {
            c.to_lowercase().collect()
        }
    })
}

impl NameMatcher {
    /// 이름이 조각 중 하나에 맞는가.
    #[must_use]
    pub fn matches(&self, name: &str) -> bool {
        let folded: Vec<char> = fold(name, self.case).collect();
        self.rules.iter().any(|r| match r {
            Rule::Fuzzy(p) => fuzzy(p, &hangul::decompose_chars(&folded)),
            Rule::Word(p) => whole_word(p, &folded),
            Rule::Glob(p) => glob(p, &folded),
            Rule::Rx(rx) => rx.is_match(name),
        })
    }
}

/// 한글 자모열(출처 nexa-sql `crates/nsql-core/src/hangul.rs` · 의존 0): 음절 → 타자 순서 자모(초·중·종 · 겹모음·겹받침은 둘) · 호환 겹자모도 둘 ·
/// 그 밖은 그대로. 퍼지 비교 양쪽(패턴 · 이름)에 같이 적용한다.
pub mod hangul {
    const CHO: [char; 19] = [
        'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ',
        'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
    ];
    const JUNG: [char; 21] = [
        'ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅘ', 'ㅙ', 'ㅚ', 'ㅛ', 'ㅜ', 'ㅝ',
        'ㅞ', 'ㅟ', 'ㅠ', 'ㅡ', 'ㅢ', 'ㅣ',
    ];
    const JONG: [char; 27] = [
        'ㄱ', 'ㄲ', 'ㄳ', 'ㄴ', 'ㄵ', 'ㄶ', 'ㄷ', 'ㄹ', 'ㄺ', 'ㄻ', 'ㄼ', 'ㄽ', 'ㄾ', 'ㄿ', 'ㅀ',
        'ㅁ', 'ㅂ', 'ㅄ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
    ];
    const JUNG_SPLIT: [(char, char, char); 7] = [
        ('ㅘ', 'ㅗ', 'ㅏ'),
        ('ㅙ', 'ㅗ', 'ㅐ'),
        ('ㅚ', 'ㅗ', 'ㅣ'),
        ('ㅝ', 'ㅜ', 'ㅓ'),
        ('ㅞ', 'ㅜ', 'ㅔ'),
        ('ㅟ', 'ㅜ', 'ㅣ'),
        ('ㅢ', 'ㅡ', 'ㅣ'),
    ];
    const JONG_SPLIT: [(char, char, char); 11] = [
        ('ㄳ', 'ㄱ', 'ㅅ'),
        ('ㄵ', 'ㄴ', 'ㅈ'),
        ('ㄶ', 'ㄴ', 'ㅎ'),
        ('ㄺ', 'ㄹ', 'ㄱ'),
        ('ㄻ', 'ㄹ', 'ㅁ'),
        ('ㄼ', 'ㄹ', 'ㅂ'),
        ('ㄽ', 'ㄹ', 'ㅅ'),
        ('ㄾ', 'ㄹ', 'ㅌ'),
        ('ㄿ', 'ㄹ', 'ㅍ'),
        ('ㅀ', 'ㄹ', 'ㅎ'),
        ('ㅄ', 'ㅂ', 'ㅅ'),
    ];

    fn split2(table: &[(char, char, char)], c: char) -> Option<(char, char)> {
        table
            .iter()
            .find(|&&(z, _, _)| z == c)
            .map(|&(_, a, b)| (a, b))
    }

    /// 완성 음절(가~힣)인가.
    #[must_use]
    pub fn is_syllable(c: char) -> bool {
        ('\u{AC00}'..='\u{D7A3}').contains(&c)
    }

    /// 글자 하나 → 자모열(한글 아니면 그대로).
    pub fn decompose_into(c: char, out: &mut Vec<char>) {
        if is_syllable(c) {
            let s = c as u32 - 0xAC00;
            let (ci, vi, ti) = (
                (s / 588) as usize,
                ((s % 588) / 28) as usize,
                (s % 28) as usize,
            );
            out.push(CHO[ci]);
            match split2(&JUNG_SPLIT, JUNG[vi]) {
                Some((a, b)) => out.extend([a, b]),
                None => out.push(JUNG[vi]),
            }
            if ti > 0 {
                match split2(&JONG_SPLIT, JONG[ti - 1]) {
                    Some((a, b)) => out.extend([a, b]),
                    None => out.push(JONG[ti - 1]),
                }
            }
        } else if let Some((a, b)) = split2(&JUNG_SPLIT, c).or_else(|| split2(&JONG_SPLIT, c)) {
            out.extend([a, b]);
        } else {
            out.push(c);
        }
    }

    /// 글자 배열 → 자모열(한글이 없으면 그대로 복사).
    #[must_use]
    pub fn decompose_chars(s: &[char]) -> Vec<char> {
        let mut out = Vec::with_capacity(s.len() * 2);
        for &c in s {
            decompose_into(c, &mut out);
        }
        out
    }
}

/// 순서대로 나타나면(부분열) — 빈 패턴 = 참.
fn fuzzy(p: &[char], s: &[char]) -> bool {
    let mut i = 0;
    for &c in s {
        if i < p.len() && p[i] == c {
            i += 1;
        }
    }
    i == p.len()
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// 낱말 경계 사이에 통째로(연속) 들어 있으면.
fn whole_word(p: &[char], s: &[char]) -> bool {
    if p.is_empty() || p.len() > s.len() {
        return p.is_empty();
    }
    (0..=s.len() - p.len()).any(|at| {
        s[at..at + p.len()] == *p
            && (at == 0 || !is_word_char(s[at - 1]))
            && (at + p.len() == s.len() || !is_word_char(s[at + p.len()]))
    })
}

/// 글롭(`*` = 0개 이상 · `?` = 1개) 전체 일치.
fn glob(p: &[char], s: &[char]) -> bool {
    match p.first() {
        None => s.is_empty(),
        Some('*') => (0..=s.len()).any(|k| glob(&p[1..], &s[k..])),
        Some('?') => !s.is_empty() && glob(&p[1..], &s[1..]),
        Some(&c) => s.first() == Some(&c) && glob(&p[1..], &s[1..]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(spec: &str, case: bool, word: bool, regex: bool) -> NameMatcher {
        compile(spec, FilterOpts { case, word, regex })
            .expect("compile")
            .expect("non-empty")
    }

    /// Fuzzy 기본(순서대로 · 대소문자 무시) · 대소문자 켬 · `;` OR · 빈 글 = None.
    #[test]
    fn fuzzy_default_and_case_toggle() {
        let f = m("rprt", false, false, false);
        assert!(f.matches("report.pdf") && f.matches("RePoRt") && !f.matches("trpr"));
        assert!(
            m("한글", false, false, false).matches("나의 한 글.txt"),
            "부분열(사이에 글자)"
        );
        // 한글 자모열(nexa-sql 규칙 · 협업 검토 10-25): 초성 · 조합 중 글자 · 겹받침 타자 순서.
        let ko = |q: &str| m(q, false, false, false);
        assert!(ko("ㅂㄱㅅ").matches("보고서.txt"), "초성");
        assert!(ko("보ㄱ").matches("보고서.txt"), "조합 중 글자(보 + ㄱ)");
        assert!(
            ko("보곳").matches("보고서.txt"),
            "받침 ㅅ = 다음 글자 초성과 같은 자모열"
        );
        assert!(
            ko("달").matches("닭.txt") && !ko("닭").matches("달.txt"),
            "겹받침 = ㄹ,ㄱ 순서"
        );
        assert!(!ko("ㄷㄱ").matches("보고서.txt"));
        assert!(hangul::decompose_chars(&['과', 'ㅄ']) == vec!['ㄱ', 'ㅗ', 'ㅏ', 'ㅂ', 'ㅅ']);
        let c = m("Rep", true, false, false);
        assert!(c.matches("Report") && !c.matches("report"));
        let or = m("abc; xyz", false, false, false);
        assert!(or.matches("a-b-c") && or.matches("XYZ") && !or.matches("ab"));
        assert!(
            compile("  ;  ", FilterOpts::default()).unwrap().is_none(),
            "빈 글 = 없음"
        );
    }

    /// 단어: 낱말 경계 사이 통째(fuzzy 아님) · 대소문자 토글.
    #[test]
    fn whole_word_mode() {
        let w = m("log", false, true, false);
        assert!(w.matches("app.log") && !w.matches("log_2024") && w.matches("my log file"));
        assert!(!w.matches("catalog.txt"), "부분 문자열은 안 됨");
        assert!(!w.matches("lg"), "fuzzy 아님");
        let wc = m("Log", true, true, false);
        assert!(wc.matches("Log.txt") && !wc.matches("log.txt"));
    }

    /// 글롭(정규식 끔 · `*`/`?` 있으면 전체 일치) · 정규식(대소문자 `(?i)` · 단어 `\b`) · 오류 = Err.
    #[test]
    fn glob_and_regex_modes() {
        let g = m("*.txt; img_??.png", false, false, false);
        assert!(
            g.matches("A.TXT")
                && g.matches("img_01.png")
                && !g.matches("img_1.png")
                && !g.matches("a.txt.bak")
        );
        let r = m(r"^\d{4}-\d{2}", false, false, true);
        assert!(r.matches("2024-10 보고서.md") && !r.matches("x2024-10"));
        let ri = m("REPORT", false, false, true);
        assert!(ri.matches("report.pdf"), "(?i)");
        let rc = m("REPORT", true, false, true);
        assert!(!rc.matches("report.pdf"));
        let rw = m("log", false, true, true);
        assert!(rw.matches("app.log") && !rw.matches("catalog"), "단어 경계");
        assert!(
            compile(
                "(",
                FilterOpts {
                    regex: true,
                    ..FilterOpts::default()
                }
            )
            .is_err(),
            "정규식 오류"
        );
        assert_eq!(
            FilterOpts {
                case: true,
                word: false,
                regex: true
            }
            .tag(),
            "cr"
        );
    }
}
