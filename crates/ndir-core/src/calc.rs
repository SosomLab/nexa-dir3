//! 계산 컬럼 식(NEW-060 · T-213) — **Python 식 부분집합** 평가기. 외부 crate 0(DR-8) · 플랫폼 중립 · 부작용 없음.
//!
//! 쓰임: 탭마다 사용자가 정의한 컬럼의 값 = 행(파일 항목)의 변수로 식을 평가한 결과. 식은 한 번 `Expr::parse`하고
//! 행마다 `Expr::eval(&env)`한다. 변수는 호출부가 [`Env`]로 준다(이 모듈은 파일을 모른다).
//!
//! 문법(파이썬과 같은 뜻 · 같은 우선순위):
//! - 리터럴: `12` · `1.5` · `'글'` · `"글"` · `f"{name}-{size:,}"` · `True` · `False` · `None` · `[a, b]`
//! - 연산: `+ - * / // % **` · 단항 `- +` · 비교 `== != < <= > >=`(연쇄 `a < b < c`) · `in` · `not in` · `and or not`
//!   · 조건식 `a if c else b` · 괄호 · 색인 `s[i]` · 조각 `s[a:b]`
//! - 함수: `len str int float bool round abs min max sum format` · `size_h` · `date strftime year month day hour minute
//!   second weekday age_days age_hours now`
//! - 문자열 메서드: `upper lower title capitalize strip lstrip rstrip replace startswith endswith find count split join
//!   zfill ljust rjust center isdigit removeprefix removesuffix`
//!
//! 한도(멈춤 방지): 식 길이 2000자 · 중첩 깊이 64 · 문자열 64 KiB · 목록 10000개. 넘으면 오류.

use std::cmp::Ordering;

/// 식의 값.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Value>),
}

/// 평가 환경 — 변수 · 현재 시각 · 현지 시간대(초).
pub trait Env {
    /// 변수 값. 모르는 이름 = `None`(평가 오류 "unknown name").
    fn var(&self, name: &str) -> Option<Value>;
    /// 현재 시각(유닉스 초). `now()` · `age_days()`가 쓴다.
    fn now(&self) -> i64 {
        0
    }
    /// 현지 시간대 오프셋(초 · UTC 기준). 날짜 함수가 쓴다.
    fn tz_offset(&self) -> i64 {
        0
    }
}

/// 구문 · 평가 오류(영어 기술 문구 — 호출부가 i18n 머리말을 붙여 보여 준다).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalcError(pub String);

impl std::fmt::Display for CalcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn err<T>(msg: impl Into<String>) -> Result<T, CalcError> {
    Err(CalcError(msg.into()))
}

const MAX_SRC: usize = 2000;
const MAX_DEPTH: usize = 64;
const MAX_STR: usize = 64 * 1024;
const MAX_LIST: usize = 10_000;

// ───────────────────────── 값 ─────────────────────────

impl Value {
    /// 파이썬 진릿값.
    pub fn truthy(&self) -> bool {
        match self {
            Value::None => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Value::None => "None",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "str",
            Value::List(_) => "list",
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Bool(b) => Some(f64::from(u8::from(*b))),
            #[allow(clippy::cast_precision_loss)]
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    fn as_int(&self) -> Option<i64> {
        match self {
            Value::Bool(b) => Some(i64::from(*b)),
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// `str(x)` 표기(파이썬과 같음 — `None` = "None" · 실수는 소수점 유지).
    pub fn to_text(&self) -> String {
        match self {
            Value::None => "None".into(),
            Value::Bool(true) => "True".into(),
            Value::Bool(false) => "False".into(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => float_text(*f),
            Value::Str(s) => s.clone(),
            Value::List(l) => {
                let parts: Vec<String> = l
                    .iter()
                    .map(|v| match v {
                        Value::Str(s) => format!("'{s}'"),
                        other => other.to_text(),
                    })
                    .collect();
                format!("[{}]", parts.join(", "))
            }
        }
    }

    /// 셀 표시용 글 — `None` = 빈칸 · 그 밖 = `to_text`.
    pub fn display(&self) -> String {
        match self {
            Value::None => String::new(),
            other => other.to_text(),
        }
    }

    /// 정렬 순서: 빈 값(None) < 수(불 포함 · 값 순) < 글(호출부가 준 비교) < 목록. NaN은 수의 맨 앞.
    pub fn sort_cmp(&self, other: &Value, text: &dyn Fn(&str, &str) -> Ordering) -> Ordering {
        fn rank(v: &Value) -> u8 {
            match v {
                Value::None => 0,
                Value::Bool(_) | Value::Int(_) | Value::Float(_) => 1,
                Value::Str(_) => 2,
                Value::List(_) => 3,
            }
        }
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a.cmp(b),
            (Value::Str(a), Value::Str(b)) => text(a, b),
            (Value::List(a), Value::List(b)) => {
                for (x, y) in a.iter().zip(b) {
                    let o = x.sort_cmp(y, text);
                    if o != Ordering::Equal {
                        return o;
                    }
                }
                a.len().cmp(&b.len())
            }
            _ => match (self.as_f64(), other.as_f64()) {
                (Some(a), Some(b)) => a.total_cmp(&b),
                _ => rank(self).cmp(&rank(other)),
            },
        }
    }
}

fn float_text(f: f64) -> String {
    if f.is_nan() {
        return "nan".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "inf" } else { "-inf" }.into();
    }
    let s = format!("{f}");
    if s.contains('.') || s.contains('e') {
        s
    } else {
        format!("{s}.0")
    }
}

// ───────────────────────── 구문 트리 ─────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Node {
    Lit(Value),
    Name(String),
    List(Vec<Node>),
    FStr(Vec<FPart>),
    Neg(Box<Node>),
    Pos(Box<Node>),
    Not(Box<Node>),
    Bin(&'static str, Box<Node>, Box<Node>),
    And(Box<Node>, Box<Node>),
    Or(Box<Node>, Box<Node>),
    Cmp(Box<Node>, Vec<(&'static str, Node)>),
    If(Box<Node>, Box<Node>, Box<Node>),
    Call(String, Vec<Node>),
    Method(Box<Node>, String, Vec<Node>),
    Index(Box<Node>, Box<Node>),
    Slice(Box<Node>, Option<Box<Node>>, Option<Box<Node>>),
}

#[derive(Debug, Clone, PartialEq)]
enum FPart {
    Lit(String),
    Expr(Node, String),
}

/// 구문 분석이 끝난 식.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    root: Node,
    src: String,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(Value),
    Str(String),
    FStr(String),
    Name(String),
    Op(&'static str),
    End,
}

const OPS: [&str; 22] = [
    "**", "//", "==", "!=", "<=", ">=", "+", "-", "*", "/", "%", "<", ">", "(", ")", "[", "]", ",",
    ".", ":", "{", "}",
];

fn lex(src: &str) -> Result<Vec<Tok>, CalcError> {
    let cs: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit()
            || (c == '.' && cs.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            let st = i;
            let mut float = false;
            while i < cs.len() && cs[i].is_ascii_digit() {
                i += 1;
            }
            if i < cs.len() && cs[i] == '.' && cs.get(i + 1).is_none_or(|n| !n.is_alphabetic()) {
                float = true;
                i += 1;
                while i < cs.len() && cs[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i < cs.len() && (cs[i] == 'e' || cs[i] == 'E') {
                let mut j = i + 1;
                if j < cs.len() && (cs[j] == '+' || cs[j] == '-') {
                    j += 1;
                }
                if j < cs.len() && cs[j].is_ascii_digit() {
                    float = true;
                    i = j;
                    while i < cs.len() && cs[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let text: String = cs[st..i].iter().collect();
            let v = if float {
                text.parse::<f64>().ok().map(Value::Float)
            } else {
                text.parse::<i64>()
                    .ok()
                    .map(Value::Int)
                    .or_else(|| text.parse::<f64>().ok().map(Value::Float))
            };
            match v {
                Some(v) => out.push(Tok::Num(v)),
                None => return err(format!("bad number '{text}'")),
            }
        } else if c == '\'' || c == '"' {
            let (s, next) = lex_string(&cs, i)?;
            out.push(Tok::Str(s));
            i = next;
        } else if c.is_alphabetic() || c == '_' {
            let st = i;
            while i < cs.len() && (cs[i].is_alphanumeric() || cs[i] == '_') {
                i += 1;
            }
            let name: String = cs[st..i].iter().collect();
            if (name == "f" || name == "F") && i < cs.len() && (cs[i] == '\'' || cs[i] == '"') {
                let (s, next) = lex_string(&cs, i)?;
                out.push(Tok::FStr(s));
                i = next;
            } else {
                out.push(Tok::Name(name));
            }
        } else {
            let two: String = cs[i..(i + 2).min(cs.len())].iter().collect();
            let one = c.to_string();
            if let Some(op) = OPS.iter().find(|o| **o == two) {
                out.push(Tok::Op(op));
                i += 2;
            } else if let Some(op) = OPS.iter().find(|o| **o == one) {
                out.push(Tok::Op(op));
                i += 1;
            } else {
                return err(format!("unexpected character '{c}'"));
            }
        }
    }
    out.push(Tok::End);
    Ok(out)
}

/// 따옴표 글 읽기(`cs[at]` = 여는 따옴표) → (내용, 다음 위치). 이스케이프 `\n \t \\ \' \"`.
fn lex_string(cs: &[char], at: usize) -> Result<(String, usize), CalcError> {
    let q = cs[at];
    let mut s = String::new();
    let mut i = at + 1;
    while i < cs.len() {
        let c = cs[i];
        if c == q {
            return Ok((s, i + 1));
        }
        if c == '\\' && i + 1 < cs.len() {
            i += 1;
            s.push(match cs[i] {
                'n' => '\n',
                't' => '\t',
                other => other,
            });
        } else {
            s.push(c);
        }
        i += 1;
    }
    err("unterminated string")
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
    depth: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos]
    }

    fn next(&mut self) -> Tok {
        let t = self.toks[self.pos].clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }

    fn is_op(&self, op: &str) -> bool {
        matches!(self.peek(), Tok::Op(o) if *o == op)
    }

    fn is_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Tok::Name(n) if n == kw)
    }

    fn eat_op(&mut self, op: &str) -> bool {
        let hit = self.is_op(op);
        if hit {
            self.next();
        }
        hit
    }

    fn eat_kw(&mut self, kw: &str) -> bool {
        let hit = self.is_kw(kw);
        if hit {
            self.next();
        }
        hit
    }

    fn expect_op(&mut self, op: &str) -> Result<(), CalcError> {
        if self.eat_op(op) {
            Ok(())
        } else {
            err(format!("expected '{op}'"))
        }
    }

    fn expr(&mut self) -> Result<Node, CalcError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return err("expression too deeply nested");
        }
        let body = self.or()?;
        let node = if self.eat_kw("if") {
            let cond = self.or()?;
            if !self.eat_kw("else") {
                return err("expected 'else'");
            }
            let els = self.expr()?;
            Node::If(Box::new(cond), Box::new(body), Box::new(els))
        } else {
            body
        };
        self.depth -= 1;
        Ok(node)
    }

    fn or(&mut self) -> Result<Node, CalcError> {
        let mut l = self.and()?;
        while self.eat_kw("or") {
            l = Node::Or(Box::new(l), Box::new(self.and()?));
        }
        Ok(l)
    }

    fn and(&mut self) -> Result<Node, CalcError> {
        let mut l = self.not()?;
        while self.eat_kw("and") {
            l = Node::And(Box::new(l), Box::new(self.not()?));
        }
        Ok(l)
    }

    fn not(&mut self) -> Result<Node, CalcError> {
        if self.eat_kw("not") {
            self.depth += 1;
            if self.depth > MAX_DEPTH {
                return err("expression too deeply nested");
            }
            let n = self.not()?;
            self.depth -= 1;
            return Ok(Node::Not(Box::new(n)));
        }
        self.cmp()
    }

    fn cmp(&mut self) -> Result<Node, CalcError> {
        let first = self.arith()?;
        let mut rest = Vec::new();
        loop {
            let op = if let Some(op) = ["==", "!=", "<=", ">=", "<", ">"]
                .into_iter()
                .find(|o| self.is_op(o))
            {
                self.next();
                op
            } else if self.eat_kw("in") {
                "in"
            } else if self.is_kw("not")
                && matches!(self.toks.get(self.pos + 1), Some(Tok::Name(n)) if n == "in")
            {
                self.next();
                self.next();
                "not in"
            } else {
                break;
            };
            rest.push((op, self.arith()?));
        }
        Ok(if rest.is_empty() {
            first
        } else {
            Node::Cmp(Box::new(first), rest)
        })
    }

    fn arith(&mut self) -> Result<Node, CalcError> {
        let mut l = self.term()?;
        while let Some(op) = ["+", "-"].into_iter().find(|o| self.is_op(o)) {
            self.next();
            l = Node::Bin(op, Box::new(l), Box::new(self.term()?));
        }
        Ok(l)
    }

    fn term(&mut self) -> Result<Node, CalcError> {
        let mut l = self.unary()?;
        while let Some(op) = ["*", "//", "/", "%"].into_iter().find(|o| self.is_op(o)) {
            self.next();
            l = Node::Bin(op, Box::new(l), Box::new(self.unary()?));
        }
        Ok(l)
    }

    fn unary(&mut self) -> Result<Node, CalcError> {
        let neg = self.is_op("-");
        if neg || self.is_op("+") {
            self.next();
            self.depth += 1;
            if self.depth > MAX_DEPTH {
                return err("expression too deeply nested");
            }
            let n = Box::new(self.unary()?);
            self.depth -= 1;
            return Ok(if neg { Node::Neg(n) } else { Node::Pos(n) });
        }
        let base = self.postfix()?;
        if self.eat_op("**") {
            // 오른쪽 결합 · 지수의 단항 부호 허용(`2 ** -1`).
            let exp = self.unary()?;
            return Ok(Node::Bin("**", Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    fn args(&mut self) -> Result<Vec<Node>, CalcError> {
        let mut v = Vec::new();
        if self.eat_op(")") {
            return Ok(v);
        }
        loop {
            v.push(self.expr()?);
            if self.eat_op(")") {
                return Ok(v);
            }
            self.expect_op(",")?;
        }
    }

    fn postfix(&mut self) -> Result<Node, CalcError> {
        let mut n = self.atom()?;
        loop {
            if self.eat_op("(") {
                let Node::Name(name) = n else {
                    return err("only named functions can be called");
                };
                n = Node::Call(name, self.args()?);
            } else if self.eat_op("[") {
                let lo = if self.is_op(":") {
                    None
                } else {
                    Some(Box::new(self.expr()?))
                };
                if self.eat_op(":") {
                    let hi = if self.is_op("]") {
                        None
                    } else {
                        Some(Box::new(self.expr()?))
                    };
                    self.expect_op("]")?;
                    n = Node::Slice(Box::new(n), lo, hi);
                } else {
                    self.expect_op("]")?;
                    match lo {
                        Some(i) => n = Node::Index(Box::new(n), i),
                        None => return err("empty index"),
                    }
                }
            } else if self.eat_op(".") {
                let Tok::Name(name) = self.next() else {
                    return err("expected method name after '.'");
                };
                self.expect_op("(")?;
                n = Node::Method(Box::new(n), name, self.args()?);
            } else {
                return Ok(n);
            }
        }
    }

    fn atom(&mut self) -> Result<Node, CalcError> {
        match self.next() {
            Tok::Num(v) => Ok(Node::Lit(v)),
            Tok::Str(s) => Ok(Node::Lit(Value::Str(s))),
            Tok::FStr(s) => Ok(Node::FStr(parse_fstring(&s, self.depth)?)),
            Tok::Name(n) => Ok(match n.as_str() {
                "True" => Node::Lit(Value::Bool(true)),
                "False" => Node::Lit(Value::Bool(false)),
                "None" => Node::Lit(Value::None),
                "if" | "else" | "and" | "or" | "not" | "in" => {
                    return err(format!("unexpected '{n}'"))
                }
                _ => Node::Name(n),
            }),
            Tok::Op("(") => {
                let e = self.expr()?;
                self.expect_op(")")?;
                Ok(e)
            }
            Tok::Op("[") => {
                let mut v = Vec::new();
                if !self.eat_op("]") {
                    loop {
                        v.push(self.expr()?);
                        if self.eat_op("]") {
                            break;
                        }
                        self.expect_op(",")?;
                    }
                }
                Ok(Node::List(v))
            }
            Tok::Op(o) => err(format!("unexpected '{o}'")),
            Tok::End => err("unexpected end of expression"),
        }
    }
}

fn parse_node(src: &str, depth: usize) -> Result<Node, CalcError> {
    let mut p = Parser {
        toks: lex(src)?,
        pos: 0,
        depth,
    };
    let n = p.expr()?;
    match p.peek() {
        Tok::End => Ok(n),
        Tok::Op(o) => err(format!("unexpected '{o}'")),
        Tok::Name(s) => err(format!("unexpected '{s}'")),
        _ => err("unexpected token"),
    }
}

/// f-문자열 내용 → 조각. `{{` · `}}` = 글자 그대로 · `{식}` · `{식:서식}`.
fn parse_fstring(s: &str, depth: usize) -> Result<Vec<FPart>, CalcError> {
    let cs: Vec<char> = s.chars().collect();
    let mut parts = Vec::new();
    let mut lit = String::new();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c == '{' && cs.get(i + 1) == Some(&'{') {
            lit.push('{');
            i += 2;
        } else if c == '}' && cs.get(i + 1) == Some(&'}') {
            lit.push('}');
            i += 2;
        } else if c == '{' {
            let mut nest = 0usize;
            let mut colon = None;
            let mut j = i + 1;
            let mut quote: Option<char> = None;
            while j < cs.len() {
                let d = cs[j];
                if let Some(q) = quote {
                    if d == q {
                        quote = None;
                    }
                } else if d == '\'' || d == '"' {
                    quote = Some(d);
                } else if d == '(' || d == '[' {
                    nest += 1;
                } else if d == ')' || d == ']' {
                    nest = nest.saturating_sub(1);
                } else if d == ':' && nest == 0 && colon.is_none() {
                    colon = Some(j);
                } else if d == '}' && nest == 0 {
                    break;
                }
                j += 1;
            }
            if j >= cs.len() {
                return err("f-string: missing '}'");
            }
            let end = colon.unwrap_or(j);
            let body: String = cs[i + 1..end].iter().collect();
            let spec: String = match colon {
                Some(k) => cs[k + 1..j].iter().collect(),
                None => String::new(),
            };
            if !lit.is_empty() {
                parts.push(FPart::Lit(std::mem::take(&mut lit)));
            }
            parts.push(FPart::Expr(parse_node(&body, depth + 1)?, spec));
            i = j + 1;
        } else if c == '}' {
            return err("f-string: single '}'");
        } else {
            lit.push(c);
            i += 1;
        }
    }
    if !lit.is_empty() {
        parts.push(FPart::Lit(lit));
    }
    Ok(parts)
}

impl Expr {
    /// 구문 분석. 빈 식 · 한도 초과 · 문법 오류 = `Err`.
    pub fn parse(src: &str) -> Result<Expr, CalcError> {
        if src.trim().is_empty() {
            return err("empty expression");
        }
        if src.chars().count() > MAX_SRC {
            return err("expression too long");
        }
        Ok(Expr {
            root: parse_node(src, 0)?,
            src: src.to_owned(),
        })
    }

    /// 원문.
    pub fn source(&self) -> &str {
        &self.src
    }

    /// 식이 읽는 변수 이름(중복 제거 · 나온 순서). 정의 창이 모르는 이름을 미리 알린다.
    pub fn names(&self) -> Vec<String> {
        fn walk(n: &Node, out: &mut Vec<String>) {
            match n {
                Node::Lit(_) => {}
                Node::Name(s) => {
                    if !out.contains(s) {
                        out.push(s.clone());
                    }
                }
                Node::List(v) | Node::Call(_, v) => v.iter().for_each(|x| walk(x, out)),
                Node::FStr(parts) => {
                    for p in parts {
                        if let FPart::Expr(e, _) = p {
                            walk(e, out);
                        }
                    }
                }
                Node::Neg(a) | Node::Pos(a) | Node::Not(a) => walk(a, out),
                Node::Bin(_, a, b) | Node::And(a, b) | Node::Or(a, b) | Node::Index(a, b) => {
                    walk(a, out);
                    walk(b, out);
                }
                Node::Cmp(a, rest) => {
                    walk(a, out);
                    rest.iter().for_each(|(_, x)| walk(x, out));
                }
                Node::If(c, a, b) => {
                    walk(a, out);
                    walk(c, out);
                    walk(b, out);
                }
                Node::Method(a, _, v) => {
                    walk(a, out);
                    v.iter().for_each(|x| walk(x, out));
                }
                Node::Slice(a, lo, hi) => {
                    walk(a, out);
                    if let Some(x) = lo {
                        walk(x, out);
                    }
                    if let Some(x) = hi {
                        walk(x, out);
                    }
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.root, &mut out);
        out
    }

    /// 평가.
    pub fn eval(&self, env: &dyn Env) -> Result<Value, CalcError> {
        eval(&self.root, env)
    }
}

// ───────────────────────── 평가 ─────────────────────────

fn cap_str(s: String) -> Result<Value, CalcError> {
    if s.len() > MAX_STR {
        err("string too long")
    } else {
        Ok(Value::Str(s))
    }
}

fn cap_list(v: Vec<Value>) -> Result<Value, CalcError> {
    if v.len() > MAX_LIST {
        err("list too long")
    } else {
        Ok(Value::List(v))
    }
}

fn eval(n: &Node, env: &dyn Env) -> Result<Value, CalcError> {
    match n {
        Node::Lit(v) => Ok(v.clone()),
        Node::Name(s) => env
            .var(s)
            .ok_or_else(|| CalcError(format!("unknown name '{s}'"))),
        Node::List(items) => cap_list(
            items
                .iter()
                .map(|x| eval(x, env))
                .collect::<Result<_, _>>()?,
        ),
        Node::FStr(parts) => {
            let mut s = String::new();
            for p in parts {
                match p {
                    FPart::Lit(t) => s.push_str(t),
                    FPart::Expr(e, spec) => s.push_str(&format_value(&eval(e, env)?, spec)?),
                }
            }
            cap_str(s)
        }
        Node::Neg(a) => match eval(a, env)? {
            Value::Float(f) => Ok(Value::Float(-f)),
            v => match v.as_int() {
                #[allow(clippy::cast_precision_loss)]
                Some(i) => Ok(i
                    .checked_neg()
                    .map_or(Value::Float(-(i as f64)), Value::Int)),
                None => err(format!("bad operand for unary -: {}", v.type_name())),
            },
        },
        Node::Pos(a) => {
            let v = eval(a, env)?;
            if v.as_f64().is_some() {
                Ok(v)
            } else {
                err(format!("bad operand for unary +: {}", v.type_name()))
            }
        }
        Node::Not(a) => Ok(Value::Bool(!eval(a, env)?.truthy())),
        Node::And(a, b) => {
            let l = eval(a, env)?;
            if l.truthy() {
                eval(b, env)
            } else {
                Ok(l)
            }
        }
        Node::Or(a, b) => {
            let l = eval(a, env)?;
            if l.truthy() {
                Ok(l)
            } else {
                eval(b, env)
            }
        }
        Node::If(c, a, b) => {
            if eval(c, env)?.truthy() {
                eval(a, env)
            } else {
                eval(b, env)
            }
        }
        Node::Bin(op, a, b) => binary(op, &eval(a, env)?, &eval(b, env)?),
        Node::Cmp(first, rest) => {
            let mut l = eval(first, env)?;
            for (op, rn) in rest {
                let r = eval(rn, env)?;
                if !compare(op, &l, &r)? {
                    return Ok(Value::Bool(false));
                }
                l = r;
            }
            Ok(Value::Bool(true))
        }
        Node::Call(name, args) => {
            let vals: Vec<Value> = args
                .iter()
                .map(|x| eval(x, env))
                .collect::<Result<_, _>>()?;
            call(name, &vals, env)
        }
        Node::Method(recv, name, args) => {
            let r = eval(recv, env)?;
            let vals: Vec<Value> = args
                .iter()
                .map(|x| eval(x, env))
                .collect::<Result<_, _>>()?;
            method(&r, name, &vals)
        }
        Node::Index(a, i) => {
            let v = eval(a, env)?;
            let Some(i) = eval(i, env)?.as_int() else {
                return err("index must be an integer");
            };
            let pick = |len: usize| -> Result<usize, CalcError> {
                let len_i = i64::try_from(len).unwrap_or(i64::MAX);
                let k = if i < 0 { i + len_i } else { i };
                if k < 0 || k >= len_i {
                    err("index out of range")
                } else {
                    Ok(usize::try_from(k).unwrap_or(0))
                }
            };
            match v {
                Value::Str(s) => {
                    let cs: Vec<char> = s.chars().collect();
                    Ok(Value::Str(cs[pick(cs.len())?].to_string()))
                }
                Value::List(l) => Ok(l[pick(l.len())?].clone()),
                other => err(format!("{} is not indexable", other.type_name())),
            }
        }
        Node::Slice(a, lo, hi) => {
            let v = eval(a, env)?;
            let bound = |n: &Option<Box<Node>>| -> Result<Option<i64>, CalcError> {
                match n {
                    None => Ok(None),
                    Some(x) => match eval(x, env)? {
                        Value::None => Ok(None),
                        v => v
                            .as_int()
                            .map(Some)
                            .ok_or_else(|| CalcError("slice bounds must be integers".into())),
                    },
                }
            };
            let (lo, hi) = (bound(lo)?, bound(hi)?);
            let range = |len: usize| -> (usize, usize) {
                let len_i = i64::try_from(len).unwrap_or(i64::MAX);
                let fix = |b: Option<i64>, default: i64| -> usize {
                    let k = b.map_or(default, |k| if k < 0 { k + len_i } else { k });
                    usize::try_from(k.clamp(0, len_i)).unwrap_or(0)
                };
                let (a, b) = (fix(lo, 0), fix(hi, len_i));
                (a, b.max(a))
            };
            match v {
                Value::Str(s) => {
                    let cs: Vec<char> = s.chars().collect();
                    let (a, b) = range(cs.len());
                    Ok(Value::Str(cs[a..b].iter().collect()))
                }
                Value::List(l) => {
                    let (a, b) = range(l.len());
                    Ok(Value::List(l[a..b].to_vec()))
                }
                other => err(format!("{} is not sliceable", other.type_name())),
            }
        }
    }
}

fn floor_div(a: i64, b: i64) -> Option<i64> {
    let q = a.checked_div(b)?;
    Some(if a % b != 0 && ((a < 0) != (b < 0)) {
        q - 1
    } else {
        q
    })
}

#[allow(clippy::cast_precision_loss)]
fn binary(op: &str, l: &Value, r: &Value) -> Result<Value, CalcError> {
    // 글 · 목록.
    match (op, l, r) {
        ("+", Value::Str(a), Value::Str(b)) => return cap_str(format!("{a}{b}")),
        ("+", Value::List(a), Value::List(b)) => {
            return cap_list(a.iter().chain(b).cloned().collect())
        }
        ("*", Value::Str(s), n) | ("*", n, Value::Str(s)) if n.as_int().is_some() => {
            let k = usize::try_from(n.as_int().unwrap_or(0)).unwrap_or(0);
            if s.len().saturating_mul(k) > MAX_STR {
                return err("string too long");
            }
            return Ok(Value::Str(s.repeat(k)));
        }
        ("%", Value::Str(_), _) => {
            return err("'%' string formatting is not supported; use f\"…\"")
        }
        _ => {}
    }
    let bad = || -> Result<Value, CalcError> {
        err(format!(
            "unsupported operand types for {op}: {} and {}",
            l.type_name(),
            r.type_name()
        ))
    };
    if let (Some(a), Some(b)) = (l.as_int(), r.as_int()) {
        let (fa, fb) = (a as f64, b as f64);
        return Ok(match op {
            "+" => a.checked_add(b).map_or(Value::Float(fa + fb), Value::Int),
            "-" => a.checked_sub(b).map_or(Value::Float(fa - fb), Value::Int),
            "*" => a.checked_mul(b).map_or(Value::Float(fa * fb), Value::Int),
            "/" => {
                if b == 0 {
                    return err("division by zero");
                }
                Value::Float(fa / fb)
            }
            "//" => {
                if b == 0 {
                    return err("division by zero");
                }
                floor_div(a, b).map_or(Value::Float((fa / fb).floor()), Value::Int)
            }
            "%" => {
                if b == 0 {
                    return err("division by zero");
                }
                match floor_div(a, b).and_then(|q| q.checked_mul(b)) {
                    Some(qb) => Value::Int(a - qb),
                    None => Value::Int(0),
                }
            }
            "**" => {
                if b >= 0 {
                    u32::try_from(b)
                        .ok()
                        .and_then(|e| a.checked_pow(e))
                        .map_or(Value::Float(fa.powf(fb)), Value::Int)
                } else if a == 0 {
                    return err("division by zero");
                } else {
                    Value::Float(fa.powf(fb))
                }
            }
            _ => return bad(),
        });
    }
    let (Some(a), Some(b)) = (l.as_f64(), r.as_f64()) else {
        return bad();
    };
    Ok(Value::Float(match op {
        "+" => a + b,
        "-" => a - b,
        "*" => a * b,
        "/" | "//" | "%" if b == 0.0 => return err("division by zero"),
        "/" => a / b,
        "//" => (a / b).floor(),
        "%" => a - (a / b).floor() * b,
        "**" => a.powf(b),
        _ => return bad(),
    }))
}

fn compare(op: &str, l: &Value, r: &Value) -> Result<bool, CalcError> {
    if op == "in" || op == "not in" {
        let hit = match (l, r) {
            (Value::Str(a), Value::Str(b)) => b.contains(a.as_str()),
            (_, Value::List(items)) => items.iter().any(|x| values_equal(l, x)),
            _ => return err(format!("'in' needs a str or list, not {}", r.type_name())),
        };
        return Ok(hit == (op == "in"));
    }
    if op == "==" {
        return Ok(values_equal(l, r));
    }
    if op == "!=" {
        return Ok(!values_equal(l, r));
    }
    let ord = match (l, r) {
        (Value::Str(a), Value::Str(b)) => a.cmp(b),
        (Value::Int(a), Value::Int(b)) => a.cmp(b),
        _ => match (l.as_f64(), r.as_f64()) {
            (Some(a), Some(b)) => match a.partial_cmp(&b) {
                Some(o) => o,
                None => return Ok(false),
            },
            _ => {
                return err(format!(
                    "'{op}' not supported between {} and {}",
                    l.type_name(),
                    r.type_name()
                ))
            }
        },
    };
    Ok(match op {
        "<" => ord == Ordering::Less,
        "<=" => ord != Ordering::Greater,
        ">" => ord == Ordering::Greater,
        _ => ord != Ordering::Less,
    })
}

fn values_equal(l: &Value, r: &Value) -> bool {
    match (l, r) {
        (Value::Str(a), Value::Str(b)) => a == b,
        (Value::None, Value::None) => true,
        (Value::List(a), Value::List(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| values_equal(x, y))
        }
        (Value::Int(a), Value::Int(b)) => a == b,
        _ => match (l.as_f64(), r.as_f64()) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        },
    }
}

fn argc(name: &str, args: &[Value], lo: usize, hi: usize) -> Result<(), CalcError> {
    if args.len() < lo || args.len() > hi {
        if lo == hi {
            err(format!(
                "{name}() takes {lo} argument(s), got {}",
                args.len()
            ))
        } else {
            err(format!(
                "{name}() takes {lo} to {hi} arguments, got {}",
                args.len()
            ))
        }
    } else {
        Ok(())
    }
}

fn num_arg(name: &str, v: &Value) -> Result<f64, CalcError> {
    v.as_f64()
        .ok_or_else(|| CalcError(format!("{name}() needs a number, not {}", v.type_name())))
}

fn int_arg(name: &str, v: &Value) -> Result<i64, CalcError> {
    v.as_int()
        .ok_or_else(|| CalcError(format!("{name}() needs an integer, not {}", v.type_name())))
}

fn str_arg<'a>(name: &str, v: &'a Value) -> Result<&'a str, CalcError> {
    match v {
        Value::Str(s) => Ok(s),
        other => err(format!("{name}() needs a str, not {}", other.type_name())),
    }
}

/// 실수 → 정수(범위 밖 · NaN = 오류).
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn f_to_i(f: f64) -> Result<i64, CalcError> {
    if f.is_finite() && f.abs() < 9.2e18 {
        Ok(f as i64)
    } else {
        err("number out of integer range")
    }
}

fn call(name: &str, a: &[Value], env: &dyn Env) -> Result<Value, CalcError> {
    match name {
        "len" => {
            argc(name, a, 1, 1)?;
            let n = match &a[0] {
                Value::Str(s) => s.chars().count(),
                Value::List(l) => l.len(),
                other => return err(format!("{} has no len()", other.type_name())),
            };
            Ok(Value::Int(i64::try_from(n).unwrap_or(i64::MAX)))
        }
        "str" => {
            argc(name, a, 0, 1)?;
            Ok(Value::Str(
                a.first().map(Value::to_text).unwrap_or_default(),
            ))
        }
        "bool" => {
            argc(name, a, 0, 1)?;
            Ok(Value::Bool(a.first().is_some_and(Value::truthy)))
        }
        "int" => {
            argc(name, a, 0, 1)?;
            match a.first() {
                None => Ok(Value::Int(0)),
                Some(Value::Float(f)) => Ok(Value::Int(f_to_i(f.trunc())?)),
                Some(Value::Str(s)) => s
                    .trim()
                    .parse::<i64>()
                    .map(Value::Int)
                    .map_err(|_| CalcError(format!("int(): invalid literal '{s}'"))),
                Some(v) => v
                    .as_int()
                    .map(Value::Int)
                    .ok_or_else(|| CalcError(format!("int(): cannot convert {}", v.type_name()))),
            }
        }
        "float" => {
            argc(name, a, 0, 1)?;
            match a.first() {
                None => Ok(Value::Float(0.0)),
                Some(Value::Str(s)) => s
                    .trim()
                    .parse::<f64>()
                    .map(Value::Float)
                    .map_err(|_| CalcError(format!("float(): invalid literal '{s}'"))),
                Some(v) => v
                    .as_f64()
                    .map(Value::Float)
                    .ok_or_else(|| CalcError(format!("float(): cannot convert {}", v.type_name()))),
            }
        }
        "abs" => {
            argc(name, a, 1, 1)?;
            match &a[0] {
                Value::Float(f) => Ok(Value::Float(f.abs())),
                v => match v.as_int() {
                    Some(i) => Ok(Value::Int(i.saturating_abs())),
                    None => err(format!("abs() needs a number, not {}", v.type_name())),
                },
            }
        }
        "round" => {
            argc(name, a, 1, 2)?;
            let x = num_arg(name, &a[0])?;
            match a.get(1) {
                None | Some(Value::None) => match &a[0] {
                    Value::Float(_) => Ok(Value::Int(f_to_i(x.round_ties_even())?)),
                    v => Ok(Value::Int(v.as_int().unwrap_or(0))),
                },
                Some(nd) => {
                    let nd = int_arg(name, nd)?.clamp(-300, 300);
                    let m = 10f64.powi(i32::try_from(nd).unwrap_or(0));
                    let y = (x * m).round_ties_even() / m;
                    Ok(if matches!(a[0], Value::Float(_)) {
                        Value::Float(y)
                    } else {
                        Value::Int(f_to_i(y)?)
                    })
                }
            }
        }
        "min" | "max" | "sum" => {
            let items: &[Value] = match a {
                [Value::List(l)] => l,
                other => other,
            };
            if name == "sum" {
                let mut acc = Value::Int(0);
                for v in items {
                    acc = binary("+", &acc, v)?;
                }
                return Ok(acc);
            }
            let mut it = items.iter();
            let Some(mut best) = it.next() else {
                return err(format!("{name}() of an empty sequence"));
            };
            for v in it {
                let better = compare(if name == "min" { "<" } else { ">" }, v, best)?;
                if better {
                    best = v;
                }
            }
            Ok(best.clone())
        }
        "format" => {
            argc(name, a, 1, 2)?;
            let spec = match a.get(1) {
                Some(v) => str_arg(name, v)?,
                None => "",
            };
            cap_str(format_value(&a[0], spec)?)
        }
        "size_h" => {
            argc(name, a, 1, 1)?;
            Ok(Value::Str(size_human(num_arg(name, &a[0])?)))
        }
        "now" => {
            argc(name, a, 0, 0)?;
            Ok(Value::Int(env.now()))
        }
        "age_days" | "age_hours" => {
            argc(name, a, 1, 1)?;
            if matches!(a[0], Value::None) {
                return Ok(Value::None);
            }
            let t = num_arg(name, &a[0])?;
            let unit = if name == "age_days" { 86400.0 } else { 3600.0 };
            #[allow(clippy::cast_precision_loss)]
            let now = env.now() as f64;
            Ok(Value::Float((now - t) / unit))
        }
        "date" | "strftime" => {
            argc(name, a, 1, 2)?;
            if matches!(a[0], Value::None) {
                return Ok(Value::Str(String::new()));
            }
            let t = f_to_i(num_arg(name, &a[0])?.floor())?;
            let fmt = match a.get(1) {
                Some(v) => str_arg(name, v)?,
                None => "%Y-%m-%d",
            };
            cap_str(strftime(t, env.tz_offset(), fmt))
        }
        "year" | "month" | "day" | "hour" | "minute" | "second" | "weekday" => {
            argc(name, a, 1, 1)?;
            if matches!(a[0], Value::None) {
                return Ok(Value::None);
            }
            let t = f_to_i(num_arg(name, &a[0])?.floor())?;
            let c = Civil::from_unix(t, env.tz_offset());
            Ok(Value::Int(match name {
                "year" => c.y,
                "month" => c.mo,
                "day" => c.d,
                "hour" => c.h,
                "minute" => c.mi,
                "second" => c.s,
                _ => c.wd,
            }))
        }
        _ => err(format!("unknown function '{name}'")),
    }
}

fn method(recv: &Value, name: &str, a: &[Value]) -> Result<Value, CalcError> {
    let Value::Str(s) = recv else {
        return err(format!("{} has no method '{name}'", recv.type_name()));
    };
    let sarg = |i: usize| -> Result<&str, CalcError> { str_arg(name, &a[i]) };
    let width = |i: usize| -> Result<usize, CalcError> {
        let w = usize::try_from(int_arg(name, &a[i])?.max(0)).unwrap_or(0);
        if w > MAX_STR {
            err("string too long")
        } else {
            Ok(w)
        }
    };
    let fill = |i: usize| -> Result<char, CalcError> {
        match a.get(i) {
            None => Ok(' '),
            Some(v) => {
                let f = str_arg(name, v)?;
                let mut cs = f.chars();
                match (cs.next(), cs.next()) {
                    (Some(c), None) => Ok(c),
                    _ => err("fill character must be exactly one character"),
                }
            }
        }
    };
    let pad = |w: usize| w.saturating_sub(s.chars().count());
    Ok(match name {
        "upper" => {
            argc(name, a, 0, 0)?;
            Value::Str(s.to_uppercase())
        }
        "lower" => {
            argc(name, a, 0, 0)?;
            Value::Str(s.to_lowercase())
        }
        "capitalize" => {
            argc(name, a, 0, 0)?;
            let mut cs = s.chars();
            Value::Str(match cs.next() {
                Some(c) => c
                    .to_uppercase()
                    .chain(cs.flat_map(char::to_lowercase))
                    .collect(),
                None => String::new(),
            })
        }
        "title" => {
            argc(name, a, 0, 0)?;
            let mut out = String::with_capacity(s.len());
            let mut start = true;
            for c in s.chars() {
                if c.is_alphabetic() {
                    if start {
                        out.extend(c.to_uppercase());
                    } else {
                        out.extend(c.to_lowercase());
                    }
                    start = false;
                } else {
                    out.push(c);
                    start = true;
                }
            }
            Value::Str(out)
        }
        "strip" | "lstrip" | "rstrip" => {
            argc(name, a, 0, 1)?;
            let set: Option<Vec<char>> = match a.first() {
                Some(v) => Some(str_arg(name, v)?.chars().collect()),
                None => None,
            };
            let hit = |c: char| set.as_ref().map_or(c.is_whitespace(), |cs| cs.contains(&c));
            Value::Str(
                match name {
                    "strip" => s.trim_matches(hit),
                    "lstrip" => s.trim_start_matches(hit),
                    _ => s.trim_end_matches(hit),
                }
                .to_owned(),
            )
        }
        "replace" => {
            argc(name, a, 2, 2)?;
            let (from, to) = (sarg(0)?, sarg(1)?);
            if from.is_empty() {
                return err("replace(): empty search string");
            }
            return cap_str(s.replace(from, to));
        }
        "startswith" => {
            argc(name, a, 1, 1)?;
            Value::Bool(s.starts_with(sarg(0)?))
        }
        "endswith" => {
            argc(name, a, 1, 1)?;
            Value::Bool(s.ends_with(sarg(0)?))
        }
        "removeprefix" => {
            argc(name, a, 1, 1)?;
            Value::Str(s.strip_prefix(sarg(0)?).unwrap_or(s).to_owned())
        }
        "removesuffix" => {
            argc(name, a, 1, 1)?;
            Value::Str(s.strip_suffix(sarg(0)?).unwrap_or(s).to_owned())
        }
        "find" => {
            argc(name, a, 1, 1)?;
            match s.find(sarg(0)?) {
                Some(b) => Value::Int(i64::try_from(s[..b].chars().count()).unwrap_or(-1)),
                None => Value::Int(-1),
            }
        }
        "count" => {
            argc(name, a, 1, 1)?;
            let needle = sarg(0)?;
            if needle.is_empty() {
                return err("count(): empty search string");
            }
            Value::Int(i64::try_from(s.matches(needle).count()).unwrap_or(i64::MAX))
        }
        "isdigit" => {
            argc(name, a, 0, 0)?;
            Value::Bool(!s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
        }
        "split" => {
            argc(name, a, 0, 1)?;
            let parts: Vec<Value> = match a.first() {
                None | Some(Value::None) => s
                    .split_whitespace()
                    .map(|p| Value::Str(p.to_owned()))
                    .collect(),
                Some(v) => {
                    let sep = str_arg(name, v)?;
                    if sep.is_empty() {
                        return err("split(): empty separator");
                    }
                    s.split(sep).map(|p| Value::Str(p.to_owned())).collect()
                }
            };
            return cap_list(parts);
        }
        "join" => {
            argc(name, a, 1, 1)?;
            let Value::List(items) = &a[0] else {
                return err("join() needs a list");
            };
            let mut parts = Vec::with_capacity(items.len());
            for v in items {
                parts.push(str_arg(name, v)?);
            }
            return cap_str(parts.join(s));
        }
        "zfill" => {
            argc(name, a, 1, 1)?;
            let n = pad(width(0)?);
            let (sign, body) = match s.strip_prefix(['-', '+']) {
                Some(rest) => (&s[..1], rest),
                None => ("", s.as_str()),
            };
            Value::Str(format!("{sign}{}{body}", "0".repeat(n)))
        }
        "ljust" | "rjust" | "center" => {
            argc(name, a, 1, 2)?;
            let (n, c) = (pad(width(0)?), fill(1)?);
            let rep = |k: usize| c.to_string().repeat(k);
            Value::Str(match name {
                "ljust" => format!("{s}{}", rep(n)),
                "rjust" => format!("{}{s}", rep(n)),
                _ => format!("{}{s}{}", rep(n / 2), rep(n - n / 2)),
            })
        }
        _ => return err(format!("str has no method '{name}'")),
    })
}

/// 사람이 읽는 크기(1024 단위 · `1.5 MB`). 음수 · NaN = 빈칸.
fn size_human(bytes: f64) -> String {
    if !bytes.is_finite() || bytes < 0.0 {
        return String::new();
    }
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut v = bytes;
    let mut u = 0;
    while v >= 1024.0 && u + 1 < UNITS.len() {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{v:.0} B")
    } else if v >= 100.0 {
        format!("{v:.0} {}", UNITS[u])
    } else if v >= 10.0 {
        format!("{v:.1} {}", UNITS[u])
    } else {
        format!("{v:.2} {}", UNITS[u])
    }
}

/// 서식 미니 언어(파이썬 `format` 부분집합): `[[채움]정렬][부호][0][폭][,][.정밀도][d f s x %]`.
fn format_value(v: &Value, spec: &str) -> Result<String, CalcError> {
    if spec.is_empty() {
        return Ok(v.to_text());
    }
    let cs: Vec<char> = spec.chars().collect();
    let mut i = 0;
    let (mut fill, mut align) = (' ', None);
    if cs.len() >= 2 && matches!(cs[1], '<' | '>' | '^') {
        fill = cs[0];
        align = Some(cs[1]);
        i = 2;
    } else if matches!(cs[0], '<' | '>' | '^') {
        align = Some(cs[0]);
        i = 1;
    }
    let mut plus = false;
    if cs.get(i) == Some(&'+') {
        plus = true;
        i += 1;
    } else if cs.get(i) == Some(&'-') {
        i += 1;
    }
    if cs.get(i) == Some(&'0') && align.is_none() {
        fill = '0';
        align = Some('=');
        i += 1;
    }
    let mut width = 0usize;
    while let Some(d) = cs.get(i).and_then(|c| c.to_digit(10)) {
        width = width.saturating_mul(10).saturating_add(d as usize);
        i += 1;
    }
    if width > 4096 {
        return err("format width too large");
    }
    let comma = cs.get(i) == Some(&',');
    if comma {
        i += 1;
    }
    let mut prec = None;
    if cs.get(i) == Some(&'.') {
        i += 1;
        let mut p = 0usize;
        let st = i;
        while let Some(d) = cs.get(i).and_then(|c| c.to_digit(10)) {
            p = p.saturating_mul(10).saturating_add(d as usize);
            i += 1;
        }
        if i == st || p > 100 {
            return err("bad format precision");
        }
        prec = Some(p);
    }
    let ty = cs.get(i).copied();
    if ty.is_some() {
        i += 1;
    }
    if i != cs.len() {
        return err(format!("bad format spec '{spec}'"));
    }
    let group = |digits: &str| -> String {
        let n = digits.len();
        let mut out = String::with_capacity(n + n / 3);
        for (k, c) in digits.chars().enumerate() {
            if k > 0 && (n - k).is_multiple_of(3) {
                out.push(',');
            }
            out.push(c);
        }
        out
    };
    // (부호, 본문)으로 만든 뒤 폭을 맞춘다.
    let (sign, body, numeric) = match (v, ty) {
        (Value::Str(s), None | Some('s')) => {
            let t: String = match prec {
                Some(p) => s.chars().take(p).collect(),
                None => s.clone(),
            };
            (String::new(), t, false)
        }
        (_, Some('s')) => (String::new(), v.to_text(), false),
        (_, Some('d' | 'x')) | (Value::Int(_) | Value::Bool(_), None) if prec.is_none() => {
            let Some(n) = v.as_int() else {
                return err(format!("format '{spec}' needs an integer"));
            };
            let mag = n.unsigned_abs();
            let digits = if ty == Some('x') {
                format!("{mag:x}")
            } else {
                mag.to_string()
            };
            let digits = if comma { group(&digits) } else { digits };
            (sign_of(n < 0, plus), digits, true)
        }
        (_, Some('f' | '%')) | (Value::Float(_) | Value::Int(_) | Value::Bool(_), None) => {
            let Some(mut f) = v.as_f64() else {
                return err(format!("format '{spec}' needs a number"));
            };
            if ty == Some('%') {
                f *= 100.0;
            }
            let text = match (prec, ty) {
                (Some(p), _) => format!("{:.p$}", f.abs()),
                (None, Some(_)) => format!("{:.6}", f.abs()),
                (None, None) => float_text(f.abs()),
            };
            let text = if comma {
                match text.split_once('.') {
                    Some((int, frac)) => format!("{}.{frac}", group(int)),
                    None => group(&text),
                }
            } else {
                text
            };
            let text = if ty == Some('%') {
                format!("{text}%")
            } else {
                text
            };
            (sign_of(f.is_sign_negative() && f != 0.0, plus), text, true)
        }
        _ => return err(format!("bad format spec '{spec}' for {}", v.type_name())),
    };
    let len = sign.chars().count() + body.chars().count();
    let padn = width.saturating_sub(len);
    let rep = |k: usize| fill.to_string().repeat(k);
    Ok(match align.unwrap_or(if numeric { '>' } else { '<' }) {
        '<' => format!("{sign}{body}{}", rep(padn)),
        '^' => format!("{}{sign}{body}{}", rep(padn / 2), rep(padn - padn / 2)),
        '=' => format!("{sign}{}{body}", rep(padn)),
        _ => format!("{}{sign}{body}", rep(padn)),
    })
}

fn sign_of(negative: bool, plus: bool) -> String {
    if negative {
        "-".into()
    } else if plus {
        "+".into()
    } else {
        String::new()
    }
}

/// 달력 시각(현지 · 그레고리력).
#[derive(Debug, Clone, Copy)]
struct Civil {
    y: i64,
    mo: i64,
    d: i64,
    h: i64,
    mi: i64,
    s: i64,
    /// 요일(월 = 0 … 일 = 6 · 파이썬 `weekday()`).
    wd: i64,
    /// 1월 1일 = 1.
    yday: i64,
}

impl Civil {
    fn from_unix(t: i64, tz: i64) -> Civil {
        let t = t.saturating_add(tz);
        let days = t.div_euclid(86400);
        let rem = t.rem_euclid(86400);
        // Howard Hinnant `civil_from_days`.
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let mo = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + i64::from(mo <= 2);
        let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        const CUM: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
        let yday = CUM[usize::try_from(mo - 1).unwrap_or(0)] + d + i64::from(leap && mo > 2);
        Civil {
            y,
            mo,
            d,
            h: rem / 3600,
            mi: rem % 3600 / 60,
            s: rem % 60,
            wd: (days + 3).rem_euclid(7),
            yday,
        }
    }
}

/// `strftime` 부분집합: `%Y %y %m %d %H %M %S %j %a %b %%`(요일 · 달 이름은 영어 약어).
fn strftime(t: i64, tz: i64, fmt: &str) -> String {
    const WD: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    const MON: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let c = Civil::from_unix(t, tz);
    let mut out = String::new();
    let mut it = fmt.chars();
    while let Some(ch) = it.next() {
        if ch != '%' {
            out.push(ch);
            continue;
        }
        match it.next() {
            Some('Y') => out.push_str(&c.y.to_string()),
            Some('y') => out.push_str(&format!("{:02}", c.y.rem_euclid(100))),
            Some('m') => out.push_str(&format!("{:02}", c.mo)),
            Some('d') => out.push_str(&format!("{:02}", c.d)),
            Some('H') => out.push_str(&format!("{:02}", c.h)),
            Some('M') => out.push_str(&format!("{:02}", c.mi)),
            Some('S') => out.push_str(&format!("{:02}", c.s)),
            Some('j') => out.push_str(&format!("{:03}", c.yday)),
            Some('a') => out.push_str(WD[usize::try_from(c.wd).unwrap_or(0)]),
            Some('b') => out.push_str(MON[usize::try_from(c.mo - 1).unwrap_or(0)]),
            Some('%') => out.push('%'),
            Some(other) => {
                out.push('%');
                out.push(other);
            }
            None => out.push('%'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    struct E;
    impl Env for E {
        fn var(&self, name: &str) -> Option<Value> {
            Some(match name {
                "name" => Value::Str("Report 2024.final.PDF".into()),
                "stem" => Value::Str("Report 2024.final".into()),
                "ext" => Value::Str("PDF".into()),
                "size" => Value::Int(1_572_864),
                "mtime" => Value::Int(1_700_000_000),
                "is_dir" => Value::Bool(false),
                "none" => Value::None,
                _ => return None,
            })
        }
        fn now(&self) -> i64 {
            1_700_000_000 + 86400 * 3
        }
        fn tz_offset(&self) -> i64 {
            9 * 3600
        }
    }

    fn ev(src: &str) -> Value {
        Expr::parse(src)
            .unwrap_or_else(|e| panic!("parse {src}: {e}"))
            .eval(&E)
            .unwrap_or_else(|e| panic!("eval {src}: {e}"))
    }

    fn txt(src: &str) -> String {
        ev(src).to_text()
    }

    fn fails(src: &str) -> String {
        match Expr::parse(src) {
            Err(e) => e.0,
            Ok(x) => x.eval(&E).expect_err(src).0,
        }
    }

    #[test]
    fn arithmetic_follows_python() {
        assert_eq!(ev("1 + 2 * 3"), Value::Int(7));
        assert_eq!(ev("(1 + 2) * 3"), Value::Int(9));
        assert_eq!(ev("7 / 2"), Value::Float(3.5));
        assert_eq!(ev("7 // 2"), Value::Int(3));
        assert_eq!(ev("-7 // 2"), Value::Int(-4));
        assert_eq!(ev("-7 % 3"), Value::Int(2));
        assert_eq!(ev("7 % -3"), Value::Int(-2));
        assert_eq!(ev("2 ** 10"), Value::Int(1024));
        assert_eq!(ev("-2 ** 2"), Value::Int(-4), "단항 -는 **보다 약하다");
        assert_eq!(ev("2 ** -1"), Value::Float(0.5));
        assert_eq!(ev("2 ** 3 ** 2"), Value::Int(512), "오른쪽 결합");
        assert_eq!(ev("1.5 + 1"), Value::Float(2.5));
        assert_eq!(txt("4 / 2"), "2.0");
        assert_eq!(ev("size / 1024 / 1024"), Value::Float(1.5));
        assert_eq!(ev("True + 1"), Value::Int(2));
        assert!(matches!(ev("9223372036854775807 + 1"), Value::Float(_)));
        assert!(fails("1 / 0").contains("division by zero"));
        assert!(fails("1 % 0").contains("division by zero"));
        assert!(fails("1 + 'a'").contains("unsupported operand"));
    }

    #[test]
    fn strings_concat_slice_and_methods() {
        assert_eq!(txt("stem + '.' + ext.lower()"), "Report 2024.final.pdf");
        assert_eq!(txt("'ab' * 3"), "ababab");
        assert_eq!(txt("name[0]"), "R");
        assert_eq!(txt("name[-3:]"), "PDF");
        assert_eq!(txt("name[:6]"), "Report");
        assert_eq!(txt("name[7:11]"), "2024");
        assert_eq!(txt("'한글이름'[1:3]"), "글이", "글자 단위");
        assert_eq!(ev("len('한글')"), Value::Int(2));
        assert_eq!(txt("name.upper()"), "REPORT 2024.FINAL.PDF");
        assert_eq!(txt("'hello world'.title()"), "Hello World");
        assert_eq!(txt("'  x '.strip()"), "x");
        assert_eq!(txt("'xxaxx'.strip('x')"), "a");
        assert_eq!(txt("name.replace(' ', '_')"), "Report_2024.final.PDF");
        assert_eq!(ev("name.startswith('Rep')"), Value::Bool(true));
        assert_eq!(ev("name.endswith('.pdf')"), Value::Bool(false));
        assert_eq!(ev("name.find('2024')"), Value::Int(7));
        assert_eq!(ev("name.count('.')"), Value::Int(2));
        assert_eq!(txt("name.split('.')[1]"), "final");
        assert_eq!(txt("'-'.join(name.split())"), "Report-2024.final.PDF");
        assert_eq!(txt("'7'.zfill(3)"), "007");
        assert_eq!(txt("'-7'.zfill(4)"), "-007");
        assert_eq!(txt("'a'.rjust(3, '.')"), "..a");
        assert_eq!(txt("'a'.center(4)"), " a  ");
        assert_eq!(txt("name.removesuffix('.PDF')"), "Report 2024.final");
        assert_eq!(ev("'2024'.isdigit()"), Value::Bool(true));
        assert_eq!(txt("'a\\tb'"), "a\tb");
        assert!(fails("name.nope()").contains("no method"));
        assert!(fails("name[99]").contains("out of range"));
    }

    #[test]
    fn comparisons_logic_and_conditionals() {
        assert_eq!(ev("1 < 2 < 3"), Value::Bool(true));
        assert_eq!(ev("1 < 2 > 5"), Value::Bool(false));
        assert_eq!(ev("ext == 'PDF' and size > 1000"), Value::Bool(true));
        assert_eq!(ev("not is_dir"), Value::Bool(true));
        assert_eq!(ev("'fin' in name"), Value::Bool(true));
        assert_eq!(ev("ext.lower() not in ['jpg', 'png']"), Value::Bool(true));
        assert_eq!(ev("ext.lower() in ['pdf', 'doc']"), Value::Bool(true));
        assert_eq!(txt("'big' if size > 1024 * 1024 else 'small'"), "big");
        assert_eq!(txt("'d' if is_dir else 'f' if size else 'empty'"), "f");
        assert_eq!(txt("none or 'fallback'"), "fallback");
        assert_eq!(ev("1 == 1.0"), Value::Bool(true));
        assert_eq!(ev("'a' == 1"), Value::Bool(false));
        assert_eq!(ev("none == None"), Value::Bool(true));
        assert!(fails("'a' < 1").contains("not supported"));
    }

    #[test]
    fn builtins_and_formatting() {
        assert_eq!(ev("int('42') + 1"), Value::Int(43));
        assert_eq!(ev("int(3.9)"), Value::Int(3));
        assert_eq!(ev("float('1.5')"), Value::Float(1.5));
        assert_eq!(txt("str(size)"), "1572864");
        assert_eq!(ev("round(2.5)"), Value::Int(2), "은행원 반올림");
        assert_eq!(ev("round(3.5)"), Value::Int(4));
        assert_eq!(ev("round(size / 1024 / 1024, 1)"), Value::Float(1.5));
        assert_eq!(ev("abs(-3)"), Value::Int(3));
        assert_eq!(ev("min(3, 1, 2)"), Value::Int(1));
        assert_eq!(ev("max([3, 1, 2])"), Value::Int(3));
        assert_eq!(ev("sum([1, 2, 3.5])"), Value::Float(6.5));
        assert_eq!(txt("size_h(size)"), "1.50 MB");
        assert_eq!(txt("size_h(512)"), "512 B");
        assert_eq!(txt("f'{stem}-{size:,}'"), "Report 2024.final-1,572,864");
        assert_eq!(txt("f\"{size / 1024:.1f} KB\""), "1536.0 KB");
        assert_eq!(txt("f'{{{ext}}}'"), "{PDF}");
        assert_eq!(
            txt("f'{7:03d}|{ext:>5}|{ext:_<5}|{0.256:.1%}'"),
            "007|  PDF|PDF__|25.6%"
        );
        assert_eq!(txt("format(255, 'x')"), "ff");
        assert_eq!(txt("format(-1234.5, ',.2f')"), "-1,234.50");
        assert_eq!(txt("f'{name.split(\".\")[0]:.3}'"), "Rep");
        assert!(fails("f'{size:zz}'").contains("bad format"));
        assert!(fails("nope(1)").contains("unknown function"));
        assert!(fails("len(1, 2)").contains("argument"));
    }

    #[test]
    fn dates_use_env_timezone() {
        // 1700000000 = 2023-11-14 22:13:20 UTC = 2023-11-15 07:13:20 KST(수요일).
        assert_eq!(txt("date(mtime)"), "2023-11-15");
        assert_eq!(
            txt("date(mtime, '%Y/%m/%d %H:%M:%S %a %b %j %y %%')"),
            "2023/11/15 07:13:20 Wed Nov 319 23 %"
        );
        assert_eq!(ev("year(mtime)"), Value::Int(2023));
        assert_eq!(ev("month(mtime)"), Value::Int(11));
        assert_eq!(ev("day(mtime)"), Value::Int(15));
        assert_eq!(ev("hour(mtime)"), Value::Int(7));
        assert_eq!(ev("weekday(mtime)"), Value::Int(2));
        assert_eq!(ev("age_days(mtime)"), Value::Float(3.0));
        assert_eq!(ev("now() - mtime"), Value::Int(86400 * 3));
        assert_eq!(txt("date(0, '%Y-%m-%d %H')"), "1970-01-01 09");
        assert_eq!(txt("date(951782400 - 9 * 3600)"), "2000-02-29", "윤년");
        assert_eq!(ev("year(none)"), Value::None);
        assert_eq!(txt("date(none)"), "");
    }

    #[test]
    fn errors_limits_and_names() {
        assert!(fails("").contains("empty"));
        assert!(fails("1 +").contains("unexpected end"));
        assert!(fails("(1").contains("expected ')'"));
        assert!(fails("1 2").contains("unexpected"));
        assert!(fails("'abc").contains("unterminated"));
        assert!(fails("a $ b").contains("unexpected character"));
        assert!(fails("unknown_var + 1").contains("unknown name"));
        assert!(fails("name.upper").contains("expected '('"));
        assert!(fails("(1)(2)").contains("only named functions"));
        assert!(fails("'a' * 99999999").contains("too long"));
        assert!(fails(&"(".repeat(100)).contains("too deeply"));
        assert!(fails(&format!("1{}", " + 1".repeat(600))).contains("too long"));
        let e = Expr::parse("f'{stem}' + ext.lower() if size > lim else name[a:b]").unwrap();
        assert_eq!(e.names(), ["stem", "ext", "size", "lim", "name", "a", "b"]);
        assert_eq!(
            e.source(),
            "f'{stem}' + ext.lower() if size > lim else name[a:b]"
        );
    }

    #[test]
    fn sort_order_and_display() {
        let text = |a: &str, b: &str| a.cmp(b);
        let mut v = vec![
            Value::Str("b".into()),
            Value::Float(1.5),
            Value::None,
            Value::Int(2),
            Value::Str("a".into()),
            Value::Bool(true),
        ];
        v.sort_by(|a, b| a.sort_cmp(b, &text));
        assert_eq!(
            v,
            [
                Value::None,
                Value::Bool(true),
                Value::Float(1.5),
                Value::Int(2),
                Value::Str("a".into()),
                Value::Str("b".into()),
            ]
        );
        assert_eq!(Value::None.display(), "");
        assert_eq!(Value::Float(2.0).display(), "2.0");
        assert_eq!(Value::Bool(false).display(), "False");
        assert_eq!(ev("[1, 'a']").to_text(), "[1, 'a']");
    }
}
