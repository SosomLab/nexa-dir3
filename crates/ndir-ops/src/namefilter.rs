//! 빠른 필터 이름 판정기(사용자 10-25 "nexa-sql 방식의 대소문자/단어/정규식 설정이 들어간 검색창" · **10-26 판정 규칙 = nexa-sql과 동일**
//! (사용자 "퍼지 개념이 nexa-sql과 다르다 — `log 18`이 nexa-sql에서는 검색되던 대상을 포함하지 않는다")): 패널 검색창이 글과 토글 셋으로
//! [`NameMatcher`]를 만들어 ndir-tree에 넣는다(ndir-tree는 regex 의존이 없어 판정기만 받는다 · DR-8 regex-lite = ops).
//!
//! 규칙(nexa-sql `filterbar::Matcher` + `rx::compile` 그대로 · dir2 유산 = `;` OR · 글롭 낱말):
//! - 토글 **전부 꺼짐**(기본): 글을 **공백으로 나눈 낱말 전부가 이름에 포함**되면(순서 무관 · 대소문자 무시) — `lib 18` → `libndir_i18n.d` ✓
//!   (`lib` ⊂ · `18` ⊂ `i18n`) · `rprt` → `report` ✗(글자 부분열 퍼지가 아니다). 낱말에 `*`/`?`가 있으면 그 낱말은 **글롭**(이름 전체 일치 · dir2).
//!   **한글 낱말은 자모열**(nsql-core `hangul` 규칙): 음절을 타자 순서 자모로 펴서 **글자 경계에서 시작해 이어지는** 자모열이 들어 있으면 —
//!   `보ㄱ` · `ㅂ` · `보고` → `보고서` ✓ · `ㅂㄱ` ✗(ㅂ 다음은 ㅗ). `;`로 나눈 조각은 **OR**(NEW-057 유산 · nexa-sql에는 없음).
//! - 토글 **하나라도 켜짐**: 글 **전체**(공백 포함 · `;` 특별 취급 없음)가 정규식 하나 — 정규식 끔 = 글자 그대로(`regex_lite::escape`) ·
//!   단어 = `\b(?:…)\b` · 대소문자 끔 = `(?i)`. 컴파일 실패 = [`compile`] `Err`(호출자가 틀을 붉게 · 0행).

use regex_lite::Regex;

/// 토글 셋(검색창의 Aa · ab · .*).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FilterOpts {
    pub case: bool,
    pub word: bool,
    pub regex: bool,
    /// 하위 폴더까지(검색창 넷째 토글 · 사용자 10-26) — 판정기와 무관한 **범위**(트리가 접힌 폴더의 자손 이름도 본다 · ndir-tree
    /// `set_deep_scope`) · [`Self::tag`]에 들어가지 않는다(판정 열쇠는 같다 → 범위만 바꿔도 깊은 판정 결과는 그대로).
    pub subdirs: bool,
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

/// 낱말 하나(소문자).
#[derive(Debug, Clone)]
enum Word {
    /// 글자 그대로 포함.
    Text(Vec<char>),
    /// 한글 = 자모열(글자 경계에서 시작해 이어짐).
    Jamo(Vec<char>),
    /// `*`/`?` = 이름 전체 글롭(dir2).
    Glob(Vec<char>),
}

#[derive(Debug, Clone)]
enum Rule {
    /// 공백 낱말 전부 포함(`;` 조각 하나).
    Words(Vec<Word>),
    /// 토글이 하나라도 켜졌을 때 글 전체.
    Rx(Regex),
}

/// 컴파일된 판정기. 빈 글이면 만들지 않는다.
#[derive(Debug, Clone)]
pub struct NameMatcher {
    rules: Vec<Rule>,
}

fn is_compat_jamo(c: char) -> bool {
    ('\u{3131}'..='\u{318E}').contains(&c)
}

fn lower_chars(s: &str) -> Vec<char> {
    s.chars().flat_map(char::to_lowercase).collect()
}

/// 글 + 토글 → 판정기. `Ok(None)` = 빈 글(필터 끔) · `Err(msg)` = 정규식 오류.
pub fn compile(spec: &str, opts: FilterOpts) -> Result<Option<NameMatcher>, String> {
    let text = spec.trim();
    if text.is_empty() {
        return Ok(None);
    }
    if opts.case || opts.word || opts.regex {
        // nexa-sql `rx::compile`: 글 전체가 패턴 하나.
        let body = if opts.regex {
            text.to_string()
        } else {
            regex_lite::escape(text)
        };
        let body = if opts.word {
            format!(r"\b(?:{body})\b")
        } else {
            body
        };
        let full = if opts.case {
            body
        } else {
            format!("(?i){body}")
        };
        let rx = Regex::new(&full).map_err(|e| e.to_string())?;
        return Ok(Some(NameMatcher {
            rules: vec![Rule::Rx(rx)],
        }));
    }
    let mut rules = Vec::new();
    for group in text.split(';') {
        let words: Vec<Word> = group
            .split_whitespace()
            .map(|w| {
                let lower = lower_chars(w);
                if w.contains(['*', '?']) {
                    Word::Glob(lower)
                } else if lower
                    .iter()
                    .any(|&c| hangul::is_syllable(c) || is_compat_jamo(c))
                {
                    Word::Jamo(hangul::decompose_chars(&lower))
                } else {
                    Word::Text(lower)
                }
            })
            .collect();
        if !words.is_empty() {
            rules.push(Rule::Words(words));
        }
    }
    if rules.is_empty() {
        return Ok(None);
    }
    Ok(Some(NameMatcher { rules }))
}

impl NameMatcher {
    /// 이름이 조각 중 하나에 맞는가(조각 = 낱말 전부 포함 · 토글이 켜졌으면 정규식).
    #[must_use]
    pub fn matches(&self, name: &str) -> bool {
        self.rules.iter().any(|r| match r {
            Rule::Rx(rx) => rx.is_match(name),
            Rule::Words(ws) => {
                let lower = lower_chars(name);
                ws.iter().all(|w| match w {
                    Word::Text(t) => contains_seq(&lower, t),
                    Word::Glob(g) => glob(g, &lower),
                    Word::Jamo(q) => hangul::contains_jamo(&lower, q),
                })
            }
        })
    }
}

/// 글자열 포함(빈 패턴 = 참).
fn contains_seq(h: &[char], n: &[char]) -> bool {
    n.is_empty() || h.windows(n.len()).any(|w| w == n)
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

    /// `hay`(이미 소문자화한 글자열)에 자모열 `q`(이미 [`decompose_chars`]한 질의)가 **글자 경계에서 시작해 이어지는** 구간으로 들어 있는가
    /// (nsql-core `hangul::find_jamo` 규칙 · 10-26).
    pub fn contains_jamo(hay: &[char], q: &[char]) -> bool {
        if q.is_empty() {
            return true;
        }
        let mut jam: Vec<char> = Vec::with_capacity(hay.len() * 2);
        let mut origin: Vec<usize> = Vec::with_capacity(hay.len() * 2);
        for (i, &c) in hay.iter().enumerate() {
            let n0 = jam.len();
            decompose_into(c, &mut jam);
            origin.extend(std::iter::repeat_n(i, jam.len() - n0));
        }
        let n = q.len();
        if n > jam.len() {
            return false;
        }
        (0..=jam.len() - n).any(|j| (j == 0 || origin[j] != origin[j - 1]) && jam[j..j + n] == *q)
    }
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
        compile(
            spec,
            FilterOpts {
                case,
                word,
                regex,
                subdirs: false,
            },
        )
        .expect("compile")
        .expect("non-empty")
    }

    fn plain(spec: &str) -> NameMatcher {
        m(spec, false, false, false)
    }

    /// 기본(토글 전부 끔) = nexa-sql: 공백 낱말 **전부 포함** · 순서 무관 · 대소문자 무시 · 부분열 퍼지 아님 · 글롭 낱말 · `;` OR · 빈 글 = None.
    #[test]
    fn words_all_contained_like_nexa_sql() {
        let f = plain("lib 18");
        assert!(
            f.matches("libndir_i18n.d"),
            "사용자 예 10-26: lib ⊂ · 18 ⊂ i18n"
        );
        assert!(f.matches("18-lib.txt"), "순서 무관");
        assert!(
            !f.matches("libndir.d") && !f.matches("i18n.d"),
            "낱말 하나 빠지면 ✗"
        );
        let l = plain("log 18");
        assert!(l.matches("log_2024-10-18.txt") && !l.matches("log.txt"));
        assert!(plain("REPORT").matches("report.pdf"), "대소문자 무시");
        assert!(
            !plain("rprt").matches("report.pdf"),
            "글자 부분열 퍼지가 아니다(nexa-sql)"
        );
        assert!(plain("  rep   pdf ").matches("report.pdf"), "공백 여럿");
        // 글롭 낱말(dir2) · `;` OR.
        let g = plain("*.txt; img_??.png");
        assert!(
            g.matches("A.TXT")
                && g.matches("img_01.png")
                && !g.matches("img_1.png")
                && !g.matches("a.txt.bak")
        );
        let gm = plain("*.md 2024");
        assert!(gm.matches("2024 노트.md") && !gm.matches("2024.txt"));
        // 우선순위(협업 검토 10-26): `a b;c` = (a AND b) OR c.
        let mix = plain("a b;c");
        assert!(mix.matches("b-a.txt") && mix.matches("c.txt") && !mix.matches("a.txt"));
        assert!(
            compile("  ;  ", FilterOpts::default()).unwrap().is_none(),
            "빈 글 = 없음"
        );
    }

    /// 한글 낱말 = 자모열(nsql-core `hangul::contains_jamo`): 글자 경계에서 시작해 **이어지는** 자모열 · 조합 중 글자 · 초성 하나 · 겹받침 순서.
    #[test]
    fn hangul_words_match_contiguous_jamo_from_char_boundary() {
        let ko = plain;
        assert!(ko("보ㄱ").matches("보고서.txt"), "조합 중 글자(보 + ㄱ)");
        assert!(ko("ㅂ").matches("보고서.txt"), "초성 하나");
        assert!(ko("보고").matches("보고서.txt") && ko("고서").matches("보고서.txt"));
        assert!(
            !ko("ㅂㄱ").matches("보고서.txt"),
            "nexa-sql: 자모가 이어져야 한다(ㅂ 다음은 ㅗ) — 초성 모음 검색이 아니다"
        );
        assert!(!ko("ㅗㄱ").matches("보고서.txt"), "글자 경계에서 시작");
        assert!(
            !ko("ㅂㄱㅅ").matches("보고서.txt"),
            "초성만 입력 검색은 nexa-sql에도 없다(10-25 부분열 퍼지 때는 맞았음 · 10-26 사용자 'nexa-sql과 같게')"
        );
        assert!(
            ko("달").matches("닭.txt") && !ko("닭").matches("달.txt"),
            "겹받침 = ㄹ,ㄱ"
        );
        assert!(
            ko("한 글").matches("나의 한 글.txt") && ko("글 한").matches("나의 한 글.txt"),
            "낱말 둘 · 순서 무관"
        );
        assert!(ko("한글 txt").matches("한글.TXT"), "한글 + 라틴 섞임");
    }

    /// 토글이 하나라도 켜지면 글 **전체**가 정규식 하나(nexa-sql `rx::compile`): 글자 그대로(escape) · `\b` · `(?i)` · 오류 = Err · tag.
    #[test]
    fn options_make_whole_text_one_regex() {
        let c = m("Rep", true, false, false);
        assert!(c.matches("Report") && !c.matches("report"));
        let c2 = m("log 18", true, false, false);
        assert!(
            c2.matches("log 18.txt") && !c2.matches("log_18.txt"),
            "켜지면 글 전체가 글자 그대로(공백 포함)"
        );
        let lit = m("a.b", true, false, false);
        assert!(
            lit.matches("a.b") && !lit.matches("axb"),
            "정규식 끔 = escape"
        );
        let w = m("log", false, true, false);
        assert!(w.matches("app.log") && !w.matches("log_2024") && w.matches("my log file"));
        assert!(!w.matches("catalog.txt") && !w.matches("lg"));
        let wc = m("Log", true, true, false);
        assert!(wc.matches("Log.txt") && !wc.matches("log.txt"));
        let r = m(r"^\d{4}-\d{2}", false, false, true);
        assert!(r.matches("2024-10 보고서.md") && !r.matches("x2024-10"));
        assert!(
            m("REPORT", false, false, true).matches("report.pdf"),
            "(?i)"
        );
        assert!(!m("REPORT", true, false, true).matches("report.pdf"));
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
                regex: true,
                subdirs: true
            }
            .tag(),
            "cr"
        );
    }
}
