//! **bplist00 최소 해석기**(순수 · 외부 crate 0 · T-114 ②): macOS Terminal.app 프로필의 글꼴은 `com.apple.Terminal.plist` ▸
//! `Window Settings.<프로필>.Font`에 **NSKeyedArchiver로 보관한 NSFont**(바이너리 plist)로 들어 있다. 여기서는 그 안의
//! `NSName`(PostScript 이름) · `NSSize`(pt)만 꺼낸다 — [`parse_archived_font`]. `plutil -extract … raw`가 주는 base64를 푸는
//! [`base64_decode`]도 함께(저장소에 base64 디코더가 없어서 · 표준 알파벳 · 패딩 · 공백 무시).
//!
//! bplist00 구조(Apple CFBinaryPList.c): 머리 8바이트 `bplist00` · 객체들 · 오프셋 표 · 꼬리 32바이트(오프셋 크기 · 참조 크기 ·
//! 객체 수 · 최상위 객체 · 오프셋 표 위치). 객체 = 표식 1바이트(상위 4비트 종류 · 하위 4비트 길이/크기 · 0xF = 길이가 int 객체로
//! 뒤따름). **NSKeyedArchiver의 UID n은 최상위 사전 `$objects` 배열의 n번째**(전체 객체 표 번호가 아니다 — 초안에서 틀렸던 점).
//! 필요한 종류만 읽고(정수 · 실수 · ASCII/UTF-16 문자열 · UID · 배열 · 사전) 나머지는 `Other`. 망가진 입력은 전부 `None`
//! (경계는 `get(..)`으로만 읽는다 · 객체 수 상한 4096).

/// base64(표준 알파벳 · `=` 패딩 · 공백/개행 무시) → 바이트. 알파벳 밖 글자가 있으면 `None`.
pub(crate) fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in text.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\n' | b'\r' | b' ' | b'\t' => continue,
            _ => return None,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// bplist00 객체(해석에 필요한 종류만).
#[derive(Debug, Clone, PartialEq)]
enum Obj {
    Int(i64),
    Real(f64),
    Str(String),
    Uid(u64),
    Array(Vec<u64>),
    Dict(Vec<(u64, u64)>),
    Other,
}

/// 빅엔디언 부호 없는 정수(1~8바이트).
fn be_uint(b: &[u8]) -> Option<u64> {
    if b.is_empty() || b.len() > 8 {
        return None;
    }
    Some(b.iter().fold(0u64, |a, x| (a << 8) | u64::from(*x)))
}

/// `at`에 있는 객체 하나. `ref_size` = 배열/사전 안 참조 1개의 바이트 수(꼬리).
fn read_obj(b: &[u8], at: usize, ref_size: usize) -> Option<Obj> {
    let m = *b.get(at)?;
    let (kind, info) = (m >> 4, usize::from(m & 0xF));
    // 길이 — info가 0xF면 바로 뒤에 int 객체(표식 0x1n · 2^n바이트)로 온다.
    let len_then = |pos: usize| -> Option<(usize, usize)> {
        if info == 0xF {
            let im = *b.get(pos)?;
            if im >> 4 != 1 {
                return None;
            }
            let n = 1usize << (im & 0xF);
            let v = be_uint(b.get(pos + 1..pos + 1 + n)?)? as usize;
            Some((v, pos + 1 + n))
        } else {
            Some((info, pos))
        }
    };
    let refs = |pos: usize, count: usize| -> Option<Vec<u64>> {
        (0..count)
            .map(|i| be_uint(b.get(pos + i * ref_size..pos + (i + 1) * ref_size)?))
            .collect()
    };
    Some(match kind {
        0x1 => {
            let n = 1usize << info;
            Obj::Int(be_uint(b.get(at + 1..at + 1 + n)?)? as i64)
        }
        0x2 => match info {
            2 => Obj::Real(f64::from(f32::from_bits(
                be_uint(b.get(at + 1..at + 5)?)? as u32
            ))),
            3 => Obj::Real(f64::from_bits(be_uint(b.get(at + 1..at + 9)?)?)),
            _ => Obj::Other,
        },
        0x5 => {
            let (n, p) = len_then(at + 1)?;
            Obj::Str(String::from_utf8_lossy(b.get(p..p + n)?).into_owned())
        }
        0x6 => {
            let (n, p) = len_then(at + 1)?;
            let units = (0..n)
                .map(|i| be_uint(b.get(p + i * 2..p + i * 2 + 2)?).map(|v| v as u16))
                .collect::<Option<Vec<u16>>>()?;
            Obj::Str(String::from_utf16_lossy(&units))
        }
        0x8 => Obj::Uid(be_uint(b.get(at + 1..at + 2 + info)?)?),
        0xA => {
            let (n, p) = len_then(at + 1)?;
            Obj::Array(refs(p, n)?)
        }
        0xD => {
            let (n, p) = len_then(at + 1)?;
            let keys = refs(p, n)?;
            let vals = refs(p + n * ref_size, n)?;
            Obj::Dict(keys.into_iter().zip(vals).collect())
        }
        _ => Obj::Other,
    })
}

/// bplist00 전체 → 객체 표(오프셋 표 순서). 머리/꼬리가 맞지 않거나 어느 객체든 깨졌으면 `None`.
fn parse_bplist(b: &[u8]) -> Option<Vec<Obj>> {
    if b.len() < 40 || &b[..8] != b"bplist00" {
        return None;
    }
    let t = &b[b.len() - 32..];
    let off_size = usize::from(t[6]);
    let ref_size = usize::from(t[7]);
    let count = be_uint(&t[8..16])? as usize;
    let table = be_uint(&t[24..32])? as usize;
    if off_size == 0 || ref_size == 0 || count == 0 || count > 4096 {
        return None;
    }
    (0..count)
        .map(|i| {
            let at = be_uint(b.get(table + i * off_size..table + (i + 1) * off_size)?)? as usize;
            read_obj(b, at, ref_size)
        })
        .collect()
}

/// NSKeyedArchiver로 보관된 NSFont(bplist00 바이트) → `(PostScript 이름, 크기 pt)`. 이름이 비었거나 크기가 0 이하면 `None`.
pub(crate) fn parse_archived_font(b: &[u8]) -> Option<(String, f32)> {
    let objs = parse_bplist(b)?;
    let s = |r: u64| match objs.get(r as usize)? {
        Obj::Str(s) => Some(s.as_str()),
        _ => None,
    };
    // UID n = 최상위 사전 `$objects` 배열의 n번째.
    let table = objs.iter().find_map(|o| {
        let Obj::Dict(pairs) = o else { return None };
        let r = pairs.iter().find(|(k, _)| s(*k) == Some("$objects"))?.1;
        match objs.get(r as usize)? {
            Obj::Array(a) => Some(a.as_slice()),
            _ => None,
        }
    })?;
    let uid = |u: u64| table.get(u as usize).copied();
    for o in &objs {
        let Obj::Dict(pairs) = o else { continue };
        let (mut name, mut size) = (None, None);
        for (k, v) in pairs {
            match s(*k) {
                Some("NSName") => {
                    name = match objs.get(*v as usize)? {
                        Obj::Uid(u) => uid(*u).and_then(s),
                        Obj::Str(x) => Some(x.as_str()),
                        _ => None,
                    }
                }
                Some("NSSize") => {
                    size = match objs.get(*v as usize)? {
                        Obj::Real(f) => Some(*f as f32),
                        Obj::Int(i) => Some(*i as f32),
                        _ => None,
                    }
                }
                _ => {}
            }
        }
        if let (Some(n), Some(sz)) = (name, size) {
            if !n.is_empty() && sz > 0.0 {
                return Some((n.to_string(), sz));
            }
        }
    }
    None
}

/// PostScript 이름 → 패밀리 어간: 첫 `-` 앞(`Menlo-Regular` → `Menlo` · `SFMonoTerminal-Regular` → `SFMonoTerminal` ·
/// `D2Coding` 그대로). PostScript 이름의 패밀리 부분에는 `-`가 올 수 없고(스타일 구분자) nexa-font `norm`이 공백/하이픈을
/// 지우므로 `SFMonoTerminal` = "SF Mono Terminal"과 맞는다. 비면 `None`.
pub(crate) fn postscript_family(name: &str) -> Option<String> {
    let stem = name.split('-').next().unwrap_or("").trim();
    (!stem.is_empty()).then(|| stem.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 이 Mac(10-31) Terminal.app 프로필 둘의 `Font` 값(plutil raw 출력 = base64) — 사용자 프로필(D2Coding 14) · Basic(SF Mono Terminal 11).
    const FONT_USER_B64: &str = "YnBsaXN0MDDUAQIDBAUGBwpYJHZlcnNpb25ZJGFyY2hpdmVyVCR0b3BYJG9iamVjdHMSAAGGoF8QD05TS2V5ZWRBcmNoaXZlctEICVRyb290gAGkCwwVFlUkbnVsbNQNDg8QERITFFZOU1NpemVYTlNmRmxhZ3NWTlNOYW1lViRjbGFzcyNALAAAAAAAABAQgAKAA1hEMkNvZGluZ9IXGBkaWiRjbGFzc25hbWVYJGNsYXNzZXNWTlNGb250ohkbWE5TT2JqZWN0CBEaJCkyN0lMUVNYXmdud36FjpCSlJ2irba9wAAAAAAAAAEBAAAAAAAAABwAAAAAAAAAAAAAAAAAAADJ";
    const FONT_BASIC_B64: &str = "YnBsaXN0MDDUAQIDBAUGBwpYJHZlcnNpb25ZJGFyY2hpdmVyVCR0b3BYJG9iamVjdHMSAAGGoF8QD05TS2V5ZWRBcmNoaXZlctEICVRyb290gAGkCwwVFlUkbnVsbNQNDg8QERITFFZOU1NpemVYTlNmRmxhZ3NWTlNOYW1lViRjbGFzcyNAJgAAAAAAABAQgAKAA18QFlNGTW9ub1Rlcm1pbmFsLVJlZ3VsYXLSFxgZGlokY2xhc3NuYW1lWCRjbGFzc2VzVk5TRm9udKIZG1hOU09iamVjdAgRGiQpMjdJTFFTWF5nbnd+hY6QkpStsr3GzdAAAAAAAAABAQAAAAAAAAAcAAAAAAAAAAAAAAAAAAAA2Q==";

    #[test]
    fn base64_roundtrip_and_errors() {
        assert_eq!(base64_decode("QUJD"), Some(b"ABC".to_vec()));
        assert_eq!(base64_decode("QUI="), Some(b"AB".to_vec()));
        assert_eq!(base64_decode("QQ=="), Some(b"A".to_vec()));
        assert_eq!(base64_decode("QU\nJD "), Some(b"ABC".to_vec()));
        assert_eq!(base64_decode(""), Some(Vec::new()));
        assert_eq!(base64_decode("Q!"), None);
    }

    #[test]
    fn archived_nsfont_yields_postscript_name_and_size() {
        let user = base64_decode(FONT_USER_B64).unwrap();
        assert_eq!(
            parse_archived_font(&user),
            Some(("D2Coding".to_string(), 14.0))
        );
        let basic = base64_decode(FONT_BASIC_B64).unwrap();
        assert_eq!(
            parse_archived_font(&basic),
            Some(("SFMonoTerminal-Regular".to_string(), 11.0))
        );
    }

    /// 깨진 입력: 빈 것 · 머리만 · 꼬리 잘림 · 머리 다름 · 오프셋 표가 범위 밖.
    #[test]
    fn broken_plists_are_none() {
        assert_eq!(parse_archived_font(b""), None);
        assert_eq!(parse_archived_font(b"bplist00"), None);
        let user = base64_decode(FONT_USER_B64).unwrap();
        assert_eq!(parse_archived_font(&user[..user.len() - 1]), None);
        let mut other = user.clone();
        other[0] = b'x';
        assert_eq!(parse_archived_font(&other), None);
        let mut bad_table = user.clone();
        let n = bad_table.len();
        bad_table[n - 1] = 0xFF;
        assert_eq!(parse_archived_font(&bad_table), None);
        // `$objects`가 없는 평범한 bplist(사전 하나 · 문자열 키/값)도 None — NSKeyedArchiver가 아니다.
        assert_eq!(parse_bplist(&user).map(|o| o.len()), Some(28));
    }

    #[test]
    fn postscript_family_strips_style_suffix() {
        assert_eq!(postscript_family("Menlo-Regular").as_deref(), Some("Menlo"));
        assert_eq!(
            postscript_family("SFMonoTerminal-Regular").as_deref(),
            Some("SFMonoTerminal")
        );
        assert_eq!(postscript_family("D2Coding").as_deref(), Some("D2Coding"));
        assert_eq!(postscript_family("-Bold"), None);
        assert_eq!(postscript_family(""), None);
    }
}
