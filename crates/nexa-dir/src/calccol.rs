//! 계산 컬럼(NEW-060 · T-213 · 사용자 11-14): 사용자가 식으로 정의한 열 — **탭에 종속** · 셀 표시 · 정렬.
//!
//! - 정의 = `이름 = 식`(식 문법 = [`ndir_core::calc`] · 파이썬 식 부분집합).
//! - 행 변수([`VARS`]): `name stem ext size mtime is_dir is_file is_link is_hidden path parent depth kind`.
//! - 열 key = [`COL_CALC0`] + 자리(탭마다 [`CALC_MAX`]개까지) · 세션 = `panel{i}.calc`(탭 `|` · 열 `;` · 퍼센트 인코딩).

use ndir_core::calc::{CalcError, Env, Expr, Value};
use ndir_core::FileKind;
use ndir_tree::{CustomKey, VisibleRowRef};
use std::path::Path;

/// 계산 컬럼의 첫 열 key(기본 열 0~7 뒤 · 자리마다 +1).
pub(crate) const COL_CALC0: u32 = 100;
/// 탭 하나의 계산 컬럼 상한.
pub(crate) const CALC_MAX: usize = 8;
/// 이름 길이 상한(글자).
const NAME_MAX: usize = 40;
/// 새 계산 컬럼의 기본 폭(DIP).
pub(crate) const DEFAULT_WIDTH: i32 = 110;

/// 식에서 쓸 수 있는 행 변수.
pub(crate) const VARS: [&str; 13] = [
    "name",
    "stem",
    "ext",
    "size",
    "mtime",
    "is_dir",
    "is_file",
    "is_link",
    "is_hidden",
    "path",
    "parent",
    "depth",
    "kind",
];

/// 열 key → 자리(계산 컬럼이 아니면 `None`).
pub(crate) fn slot_of(key: u32) -> Option<usize> {
    let i = key.checked_sub(COL_CALC0)? as usize;
    (i < CALC_MAX).then_some(i)
}

/// 자리 → 열 key.
pub(crate) fn key_of(slot: usize) -> u32 {
    COL_CALC0 + slot as u32
}

/// 세션 · 정렬 문자열에 쓰는 열 이름(`calc0`…).
pub(crate) const KEY_NAMES: [&str; CALC_MAX] = [
    "calc0", "calc1", "calc2", "calc3", "calc4", "calc5", "calc6", "calc7",
];

/// 계산 컬럼 하나.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CalcCol {
    pub name: String,
    pub src: String,
    /// 열 폭(px · 0 = 기본).
    pub width: i32,
    expr: Expr,
}

/// 정의 오류(창에 그대로 보인다 — 머리말은 i18n · 사유는 평가기의 영어 기술 문구).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DefError {
    /// `이름 = 식` 꼴이 아니다.
    Shape,
    /// 이름이 비었거나 너무 길다.
    Name,
    /// 식 오류(사유).
    Expr(String),
}

impl CalcCol {
    /// 이름과 식으로 만든다 — 식 문법 · 모르는 변수를 여기서 걸러 낸다.
    pub(crate) fn new(name: &str, src: &str) -> Result<CalcCol, DefError> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > NAME_MAX || name.contains(['\n', '\r']) {
            return Err(DefError::Name);
        }
        let src = src.trim();
        let expr = Expr::parse(src).map_err(|e| DefError::Expr(e.0))?;
        if let Some(bad) = expr
            .names()
            .into_iter()
            .find(|n| !VARS.contains(&n.as_str()))
        {
            return Err(DefError::Expr(format!("unknown name '{bad}'")));
        }
        // 견본 행으로 한 번 돌려 본다 — 모르는 함수 · 틀린 정규식은 행과 무관한 정의 실수라 여기서 알린다(종전 = 만든 뒤 칸마다
        // `#ERR`). 값에 따른 오류(형 불일치 · 0 나누기)는 행마다 다르므로 그대로 둔다.
        let probe = VisibleRowRef {
            id: 0,
            depth: 1,
            kind: FileKind::File,
            name: "probe.txt",
            size: 1,
            modified_unix_ms: 1_000,
            attrs: 0,
            expanded: false,
            has_children: false,
        };
        let env = RowEnv {
            row: &probe,
            path: Path::new("/probe.txt"),
            dir_size: None,
            clock: (0, 0),
        };
        if let Err(e) = expr.eval(&env) {
            if e.0.starts_with("unknown function") || e.0.starts_with("bad regex") {
                return Err(DefError::Expr(e.0));
            }
        }
        Ok(CalcCol {
            name: name.to_owned(),
            src: src.to_owned(),
            width: 0,
            expr,
        })
    }

    /// 입력란 글 `이름 = 식` → 열. 첫 `=`가 구분자(이름에는 `=`를 쓸 수 없다).
    pub(crate) fn parse_def(text: &str) -> Result<CalcCol, DefError> {
        let (name, src) = text.split_once('=').ok_or(DefError::Shape)?;
        if src.starts_with('=') {
            return Err(DefError::Shape); // `a == b`처럼 이름 없이 식만 쓴 경우
        }
        CalcCol::new(name, src)
    }

    /// 입력란에 다시 채울 글.
    pub(crate) fn def_text(&self) -> String {
        format!("{} = {}", self.name, self.src)
    }

    /// 세션 한 항목 = `이름=식@폭`(각 부분 퍼센트 인코딩).
    fn encode(&self) -> String {
        format!("{}={}@{}", enc(&self.name), enc(&self.src), self.width)
    }

    fn decode(item: &str) -> Option<CalcCol> {
        let (name, rest) = item.split_once('=')?;
        let (src, width) = rest.rsplit_once('@').unwrap_or((rest, "0"));
        let mut c = CalcCol::new(&dec(name), &dec(src)).ok()?;
        c.width = width.parse::<i32>().unwrap_or(0).clamp(0, 4000);
        Some(c)
    }

    /// 이 행의 값.
    pub(crate) fn eval(&self, row: &RowEnv<'_>) -> Result<Value, CalcError> {
        self.expr.eval(row)
    }
}

/// 구분자로 쓰는 글자 · 줄바꿈을 `%XX`로(나머지는 그대로 — 한글 등 비ASCII 포함).
fn enc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if matches!(ch, '%' | '|' | ';' | '=' | '@' | '\n' | '\r') {
            out.push_str(&format!("%{:02X}", ch as u32));
        } else {
            out.push(ch);
        }
    }
    out
}

fn dec(s: &str) -> String {
    let cs: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < cs.len() {
        if cs[i] == '%' && i + 3 <= cs.len() {
            let hex: String = cs[i + 1..i + 3].iter().collect();
            if let Ok(b) = u8::from_str_radix(&hex, 16) {
                out.push(char::from(b));
                i += 3;
                continue;
            }
        }
        out.push(cs[i]);
        i += 1;
    }
    out
}

/// 탭 하나의 계산 컬럼 목록 → 세션 글(`;` 구분).
pub(crate) fn encode_tab(cols: &[CalcCol]) -> String {
    cols.iter()
        .map(CalcCol::encode)
        .collect::<Vec<_>>()
        .join(";")
}

/// 세션 글 → 계산 컬럼 목록(손상 항목은 버린다 · 상한 [`CALC_MAX`]).
pub(crate) fn decode_tab(text: &str) -> Vec<CalcCol> {
    text.split(';')
        .filter(|t| !t.trim().is_empty())
        .filter_map(|t| CalcCol::decode(t.trim()))
        .take(CALC_MAX)
        .collect()
}

/// 지금 시각(유닉스 초)과 현지 시간대 오프셋(초).
pub(crate) fn clock() -> (i64, i64) {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let l = nexa_fs::local_time(now);
    let local = ndir_vfs::archive::ymd_hms_to_unix(
        i64::from(l.year),
        i64::from(l.month),
        i64::from(l.day),
        i64::from(l.hour),
        i64::from(l.min),
        i64::from(l.sec),
    );
    (secs, local - secs)
}

/// 행 하나의 변수 공급.
#[derive(Debug)]
pub(crate) struct RowEnv<'a> {
    pub row: &'a VisibleRowRef<'a>,
    pub path: &'a Path,
    /// 계산된 폴더 크기(폴더 크기 계산이 켜져 있고 값이 있을 때).
    pub dir_size: Option<u64>,
    /// ([`clock`]) 지금 · 시간대.
    pub clock: (i64, i64),
}

impl Env for RowEnv<'_> {
    /// 정규식 함수(`re.sub` · `re.search` …)는 `ndir-ops`가 맡는다(코어는 의존 0).
    fn call_ext(
        &self,
        name: &str,
        args: &[Value],
    ) -> Option<Result<Value, ndir_core::calc::CalcError>> {
        ndir_ops::calcre::call(name, args)
    }

    fn var(&self, name: &str) -> Option<Value> {
        let r = self.row;
        let is_dir = r.kind == FileKind::Dir;
        let (stem, ext) = if is_dir {
            (r.name, "")
        } else {
            match r.name.rsplit_once('.') {
                Some((s, e)) if !s.is_empty() && !e.is_empty() => (s, e),
                _ => (r.name, ""),
            }
        };
        Some(match name {
            "name" => Value::Str(r.name.to_owned()),
            "stem" => Value::Str(stem.to_owned()),
            "ext" => Value::Str(ext.to_owned()),
            "size" => {
                let bytes = if is_dir {
                    self.dir_size.unwrap_or(0)
                } else {
                    r.size
                };
                Value::Int(i64::try_from(bytes).unwrap_or(i64::MAX))
            }
            "mtime" => {
                if r.modified_unix_ms > 0 {
                    Value::Int(r.modified_unix_ms.div_euclid(1000))
                } else {
                    Value::None
                }
            }
            "is_dir" => Value::Bool(is_dir),
            "is_file" => Value::Bool(r.kind == FileKind::File),
            "is_link" => Value::Bool(r.kind == FileKind::Symlink),
            "is_hidden" => Value::Bool(
                r.attrs & ndir_vfs::ATTR_HIDDEN != 0 || (cfg!(unix) && r.name.starts_with('.')),
            ),
            "path" => Value::Str(self.path.to_string_lossy().into_owned()),
            "parent" => Value::Str(
                self.path
                    .parent()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            ),
            "depth" => Value::Int(i64::from(r.depth)),
            "kind" => Value::Str(
                match r.kind {
                    FileKind::Dir => "dir",
                    FileKind::File => "file",
                    FileKind::Symlink => "link",
                }
                .to_owned(),
            ),
            _ => return None,
        })
    }

    fn now(&self) -> i64 {
        self.clock.0
    }

    fn tz_offset(&self) -> i64 {
        self.clock.1
    }
}

/// 셀에 보일 글 — 오류 = `#ERR`.
pub(crate) fn cell_text(v: &Result<Value, CalcError>) -> String {
    match v {
        Ok(v) => v.display(),
        Err(_) => "#ERR".to_owned(),
    }
}

/// 정렬 값 — 오류 · 빈 값 = 빈 값(오름차순 맨 앞) · 수 · 글.
pub(crate) fn sort_key(v: Result<Value, CalcError>) -> CustomKey {
    match v {
        Ok(Value::None) | Err(_) => CustomKey::None,
        Ok(Value::Bool(b)) => CustomKey::Num(f64::from(u8::from(b))),
        #[allow(clippy::cast_precision_loss)]
        Ok(Value::Int(i)) => CustomKey::Num(i as f64),
        Ok(Value::Float(f)) => CustomKey::Num(f),
        Ok(Value::Str(s)) => CustomKey::Text(s),
        Ok(other @ Value::List(_)) => CustomKey::Text(other.to_text()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row<'a>(name: &'a str, kind: FileKind, size: u64) -> VisibleRowRef<'a> {
        VisibleRowRef {
            id: 0,
            depth: 1,
            kind,
            name,
            size,
            modified_unix_ms: 1_700_000_000_000,
            attrs: 0,
            expanded: false,
            has_children: kind == FileKind::Dir,
        }
    }

    /// 정의 파싱: `이름 = 식` · 모르는 변수 · 빈 이름 · 꼴 오류 · 식 안의 `==`는 구분자가 아니다.
    #[test]
    fn definition_parsing_and_validation() {
        let c = CalcCol::parse_def(" MB = round(size / 1024 / 1024, 1) ").unwrap();
        assert_eq!(
            (c.name.as_str(), c.src.as_str()),
            ("MB", "round(size / 1024 / 1024, 1)")
        );
        assert_eq!(c.def_text(), "MB = round(size / 1024 / 1024, 1)");
        let c = CalcCol::parse_def("pdf? = ext.lower() == 'pdf'").unwrap();
        assert_eq!(c.src, "ext.lower() == 'pdf'");
        assert_eq!(CalcCol::parse_def("size / 2"), Err(DefError::Shape));
        assert_eq!(CalcCol::parse_def("== 1"), Err(DefError::Shape));
        assert_eq!(CalcCol::parse_def(" = size"), Err(DefError::Name));
        assert!(matches!(
            CalcCol::parse_def("x = size +"),
            Err(DefError::Expr(_))
        ));
        assert_eq!(
            CalcCol::parse_def("x = sise * 2"),
            Err(DefError::Expr("unknown name 'sise'".into()))
        );
        assert_eq!(
            CalcCol::parse_def(&format!("{} = 1", "n".repeat(41))),
            Err(DefError::Name)
        );
        // 모르는 함수 · 틀린 정규식은 정의 때 알린다(견본 행 평가) · 값에 따른 오류는 통과(칸에서 `#ERR`).
        assert_eq!(
            CalcCol::parse_def("x = lenn(name)"),
            Err(DefError::Expr("unknown function 'lenn'".into()))
        );
        assert!(matches!(
            CalcCol::parse_def("x = re.sub('(', '', name)"),
            Err(DefError::Expr(m)) if m.starts_with("bad regex")
        ));
        assert!(CalcCol::parse_def("x = 1 / (size - 1)").is_ok());
    }

    /// 행 변수와 평가: 파일 · 폴더(계산된 크기) · 확장자 없는 이름 · 정렬 값 · 셀 글(오류 = `#ERR`).
    #[test]
    fn row_variables_and_values() {
        let clock = (1_700_000_000 + 86_400, 9 * 3600);
        let r = row("Report.final.PDF", FileKind::File, 3 * 1024 * 1024);
        let env = RowEnv {
            row: &r,
            path: Path::new("/data/docs/Report.final.PDF"),
            dir_size: None,
            clock,
        };
        let val = |def: &str| CalcCol::parse_def(def).unwrap().eval(&env);
        assert_eq!(
            cell_text(&val("a = stem + '.' + ext.lower()")),
            "Report.final.pdf"
        );
        assert_eq!(cell_text(&val("a = size // 1024")), "3072");
        // 정규식 함수(`ndir-ops::calcre`): 바꾸기 · 찾기 · 정렬용 숫자 뽑기.
        assert_eq!(
            cell_text(&val(r"a = re.sub('\\.final', '', name)")),
            "Report.PDF"
        );
        assert_eq!(
            cell_text(&val("a = re.search('(?i)pdf$', name) != None")),
            "True"
        );
        assert_eq!(
            cell_text(&val("a = f'{size / 1024 / 1024:.1f} MB'")),
            "3.0 MB"
        );
        assert_eq!(cell_text(&val("a = date(mtime)")), "2023-11-15");
        assert_eq!(cell_text(&val("a = round(age_days(mtime), 1)")), "1.0");
        assert_eq!(cell_text(&val("a = parent")), "/data/docs");
        assert_eq!(cell_text(&val("a = kind + str(depth)")), "file1");
        assert_eq!(cell_text(&val("a = is_file and not is_dir")), "True");
        assert_eq!(cell_text(&val("a = 1 / (size - size)")), "#ERR");
        assert_eq!(sort_key(val("a = size")), CustomKey::Num(3_145_728.0));
        assert_eq!(sort_key(val("a = ext")), CustomKey::Text("PDF".into()));
        assert_eq!(sort_key(val("a = 1 / (size - size)")), CustomKey::None);
        // 폴더: 확장자 없음 · 크기 = 계산된 값(없으면 0).
        let d = row("pack.v2", FileKind::Dir, 4096);
        let mut env = RowEnv {
            row: &d,
            path: Path::new("/data/pack.v2"),
            dir_size: Some(5000),
            clock,
        };
        let c = CalcCol::parse_def("a = f'{stem}|{ext}|{size}|{is_dir}'").unwrap();
        assert_eq!(cell_text(&c.eval(&env)), "pack.v2||5000|True");
        env.dir_size = None;
        assert_eq!(cell_text(&c.eval(&env)), "pack.v2||0|True");
        // 확장자 없는 파일 · 점으로 시작하는 이름.
        let f = row(".bashrc", FileKind::File, 1);
        let env = RowEnv {
            row: &f,
            path: Path::new("/h/.bashrc"),
            dir_size: None,
            clock,
        };
        let c = CalcCol::parse_def("a = stem + '|' + ext").unwrap();
        assert_eq!(cell_text(&c.eval(&env)), ".bashrc|");
    }

    /// 세션 왕복: 구분자(`| ; = @ %`) · 한글 · 폭 · 손상 항목은 버림 · 상한.
    #[test]
    fn session_round_trip() {
        let mut a = CalcCol::parse_def("크기(MB) = f'{size / 1048576:.1f}|MB;@100%'").unwrap();
        a.width = 140;
        let b = CalcCol::parse_def("is pdf = ext.lower() == 'pdf'").unwrap();
        let text = encode_tab(&[a.clone(), b.clone()]);
        assert!(
            !text.contains('|') && text.matches(';').count() == 1,
            "{text}"
        );
        assert_eq!(decode_tab(&text), vec![a, b]);
        assert_eq!(decode_tab(""), Vec::new());
        assert_eq!(decode_tab("broken;x=1 +@5;ok=size@0").len(), 1);
        let many = vec!["n=1@0"; 20].join(";");
        assert_eq!(decode_tab(&many).len(), CALC_MAX);
        assert_eq!(
            (slot_of(COL_CALC0 + 3), slot_of(7), slot_of(COL_CALC0 + 8)),
            (Some(3), None, None)
        );
        assert_eq!(key_of(2), COL_CALC0 + 2);
    }
}
