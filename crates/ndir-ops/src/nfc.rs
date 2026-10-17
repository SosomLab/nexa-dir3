//! 유니코드 정규화(NFC 표시 · NFD 식별 · 변환 계획 · 10-18 사용자 "맥에서 NFD 이름의 자모가 깨져 보임" · ndir-ops에 두는 이유 = 이름 바꾸기 엔진
//! `batch_rename::apply_renames`가 "정규화만 다른 이름"을 같은 항목으로 판정해야 한다(APFS 정규화 둔감)): 외부 crate 0(DR-8) · 범위 =
//! **한글 자모 합성**(알고리즘 · 표 없음)과 흔한 라틴 결합 기호 7종(표). Finder는 모든 이름을 정규화해 보이고, HFS+/SMB에서 온 NFD 이름은
//! 다른 앱에서 자모가 흩어져 보인다 → 설정 `list.nfc_display`(켬 = 합성해 표시) · 상태 열 "NFD" 표식 · 행 메뉴 "이름을 NFC로 변환".

use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// 이름에 분해(결합) 문자가 있는가 = "NFD" 식별: 조합형 한글 자모(U+1100~U+11FF) 또는 결합 기호(U+0300~U+036F).
pub fn is_decomposed(s: &str) -> bool {
    s.chars()
        .any(|c| matches!(c, '\u{1100}'..='\u{11FF}' | '\u{0300}'..='\u{036F}'))
}

const L_BASE: u32 = 0x1100;
const V_BASE: u32 = 0x1161;
const T_BASE: u32 = 0x11A7;
const S_BASE: u32 = 0xAC00;
const L_COUNT: u32 = 19;
const V_COUNT: u32 = 21;
const T_COUNT: u32 = 28;
const S_COUNT: u32 = L_COUNT * V_COUNT * T_COUNT;

/// (라틴 글자, 결합 기호) → 미리 조합된 글자(Latin-1 보충 · 일부 확장-A). 없는 조합 = None(그대로 둔다).
fn latin(base: char, mark: char) -> Option<char> {
    let col = match mark {
        '\u{0300}' => 0, // grave
        '\u{0301}' => 1, // acute
        '\u{0302}' => 2, // circumflex
        '\u{0303}' => 3, // tilde
        '\u{0308}' => 4, // diaeresis
        '\u{030A}' => 5, // ring
        '\u{0327}' => 6, // cedilla
        _ => return None,
    };
    const NONE: char = '\0';
    let table: &[(char, [char; 7])] = &[
        ('A', ['À', 'Á', 'Â', 'Ã', 'Ä', 'Å', NONE]),
        ('a', ['à', 'á', 'â', 'ã', 'ä', 'å', NONE]),
        ('E', ['È', 'É', 'Ê', NONE, 'Ë', NONE, NONE]),
        ('e', ['è', 'é', 'ê', NONE, 'ë', NONE, NONE]),
        ('I', ['Ì', 'Í', 'Î', NONE, 'Ï', NONE, NONE]),
        ('i', ['ì', 'í', 'î', NONE, 'ï', NONE, NONE]),
        ('O', ['Ò', 'Ó', 'Ô', 'Õ', 'Ö', NONE, NONE]),
        ('o', ['ò', 'ó', 'ô', 'õ', 'ö', NONE, NONE]),
        ('U', ['Ù', 'Ú', 'Û', NONE, 'Ü', NONE, NONE]),
        ('u', ['ù', 'ú', 'û', NONE, 'ü', NONE, NONE]),
        ('N', [NONE, NONE, NONE, 'Ñ', NONE, NONE, NONE]),
        ('n', [NONE, NONE, NONE, 'ñ', NONE, NONE, NONE]),
        ('Y', [NONE, 'Ý', NONE, NONE, 'Ÿ', NONE, NONE]),
        ('y', [NONE, 'ý', NONE, NONE, 'ÿ', NONE, NONE]),
        ('C', [NONE, NONE, NONE, NONE, NONE, NONE, 'Ç']),
        ('c', [NONE, NONE, NONE, NONE, NONE, NONE, 'ç']),
    ];
    table
        .iter()
        .find(|(b, _)| *b == base)
        .map(|(_, row)| row[col])
        .filter(|c| *c != NONE)
}

/// NFC 합성(표시 · 변환용): 한글 초성+중성(+종성) → 완성형 · 완성형 + 종성 자모 → 완성형 · 라틴 글자 + 결합 기호 → 미리 조합된 글자.
/// 바뀌는 것이 없으면 빌림 그대로(할당 0).
pub fn compose(s: &str) -> Cow<'_, str> {
    if !is_decomposed(s) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        let cp = c as u32;
        // 초성 + 중성 (+ 종성).
        if (L_BASE..L_BASE + L_COUNT).contains(&cp) {
            if let Some(&v) = chars.peek() {
                let vp = v as u32;
                if (V_BASE..V_BASE + V_COUNT).contains(&vp) {
                    chars.next();
                    let mut syl = S_BASE + ((cp - L_BASE) * V_COUNT + (vp - V_BASE)) * T_COUNT;
                    if let Some(&t) = chars.peek() {
                        let tp = t as u32;
                        if (T_BASE + 1..T_BASE + T_COUNT).contains(&tp) {
                            chars.next();
                            syl += tp - T_BASE;
                        }
                    }
                    out.push(char::from_u32(syl).unwrap_or(c));
                    continue;
                }
            }
            out.push(c);
            continue;
        }
        // 완성형(종성 없음) + 종성 자모.
        if (S_BASE..S_BASE + S_COUNT).contains(&cp) && (cp - S_BASE).is_multiple_of(T_COUNT) {
            if let Some(&t) = chars.peek() {
                let tp = t as u32;
                if (T_BASE + 1..T_BASE + T_COUNT).contains(&tp) {
                    chars.next();
                    out.push(char::from_u32(cp + tp - T_BASE).unwrap_or(c));
                    continue;
                }
            }
        }
        // 라틴 글자 + 결합 기호.
        if let Some(&m) = chars.peek() {
            if ('\u{0300}'..='\u{036F}').contains(&m) {
                if let Some(pre) = latin(c, m) {
                    chars.next();
                    out.push(pre);
                    continue;
                }
            }
        }
        out.push(c);
    }
    Cow::Owned(out)
}

/// 경로의 이름이 분해형인가(파일 이름만 본다).
pub fn path_is_decomposed(p: &Path) -> bool {
    p.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(is_decomposed)
}

/// 폴더 바로 아래에 분해형 이름이 있는가(얕은 1단계 · 메뉴 표시 판정용 · 읽기 실패 = false).
pub fn dir_has_decomposed_child(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|rd| {
        rd.flatten()
            .any(|e| e.file_name().to_str().is_some_and(is_decomposed))
    })
}

/// 변환 계획(순수 · 파일 시스템 읽기만): 선택한 항목들 중 분해형 이름 → (경로, NFC 이름). `recursive`면 폴더 안을 **후위 순회**(자식 먼저 ·
/// 부모 나중)해 부모 이름이 바뀌어도 자식 경로가 유효하다. 합성해도 같은 이름은 뺀다.
pub fn rename_plan(paths: &[PathBuf], recursive: bool) -> Vec<(PathBuf, String)> {
    fn walk(p: &Path, recursive: bool, out: &mut Vec<(PathBuf, String)>) {
        if recursive && p.is_dir() {
            if let Ok(rd) = std::fs::read_dir(p) {
                let mut kids: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
                kids.sort();
                for k in kids {
                    walk(&k, true, out);
                }
            }
        }
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if let Cow::Owned(nfc) = compose(name) {
                if nfc != name {
                    out.push((p.to_path_buf(), nfc));
                }
            }
        }
    }
    let mut out = Vec::new();
    for p in paths {
        walk(p, recursive, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const HANGUL_NFD: &str = "\u{1112}\u{1161}\u{11AB}\u{1100}\u{1173}\u{11AF}"; // 한글
    const HANGUL_NFC: &str = "한글";

    /// 한글 자모 합성(종성 있음/없음 · 완성형+종성) · 라틴 결합 기호 · 바뀜 없음 = 빌림.
    #[test]
    fn composes_hangul_and_latin() {
        assert!(is_decomposed(HANGUL_NFD) && !is_decomposed(HANGUL_NFC));
        assert_eq!(compose(HANGUL_NFD), HANGUL_NFC);
        assert_eq!(compose("\u{1100}\u{1161}"), "가", "종성 없음");
        assert_eq!(compose("가\u{11A8}"), "각", "완성형 + 종성");
        assert_eq!(compose("cafe\u{0301}.txt"), "café.txt");
        assert_eq!(compose("Stra\u{0308}sse"), "Strässe");
        assert!(
            matches!(compose("plain.txt"), Cow::Borrowed(_)),
            "바뀜 없음 = 할당 0"
        );
        assert_eq!(compose("x\u{0301}"), "x\u{0301}", "표에 없는 조합 = 그대로");
        assert!(
            !is_decomposed("ㅎㅏㄴ"),
            "호환 자모(U+3131~)는 분해형이 아니다"
        );
    }

    /// 변환 계획: 후위 순회(자식 먼저) · 분해형만 · 재귀 끄면 선택한 것만.
    #[test]
    fn rename_plan_is_post_order_and_filters() {
        let base = std::env::temp_dir().join(format!("ndir-nfc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join(format!("{HANGUL_NFD}_dir"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{HANGUL_NFD}.txt")), b"x").unwrap();
        std::fs::write(dir.join("plain.txt"), b"x").unwrap();
        let plan = rename_plan(std::slice::from_ref(&dir), true);
        assert_eq!(plan.len(), 2);
        assert_eq!(
            plan[0].0,
            dir.join(format!("{HANGUL_NFD}.txt")),
            "자식 먼저"
        );
        assert_eq!(plan[0].1, format!("{HANGUL_NFC}.txt"));
        assert_eq!(plan[1].0, dir, "부모 나중");
        assert_eq!(plan[1].1, format!("{HANGUL_NFC}_dir"));
        assert_eq!(
            rename_plan(std::slice::from_ref(&dir), false).len(),
            1,
            "재귀 끔 = 선택만"
        );
        assert!(dir_has_decomposed_child(&dir) && path_is_decomposed(&dir));
        let _ = std::fs::remove_dir_all(&base);
    }
}
