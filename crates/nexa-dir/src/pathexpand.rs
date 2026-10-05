//! 경로 입력 **확장**(dir3 신규 · 사용자 10-05 "경로명에 쓸 수 있는 기능을 확장" — dir2 `pathinput::expand_env`의 윗단).
//!
//! 경로 바에 친 글을 실제 경로로 바꾼다. **순수 로직**(환경 · 현재 폴더 · 홈을 주입 받는다 → 전 플랫폼 시험) · 외부 crate 0 ·
//! **셸을 실행하지 않는다**(`$(…)`는 아래 내장 목록만 — 임의 명령 실행은 멈춤 · 보안 위험이라 하지 않는다. 모르는 명령은 원문 그대로).
//!
//! | 입력 | 뜻 |
//! | --- | --- |
//! | `%APPDATA%` | CMD 환경변수 |
//! | `$env:APPDATA` · `${env:ProgramFiles(x86)}` | PowerShell 환경변수 |
//! | `$HOME` · `${HOME}` · `$NAME` · `${NAME}` | 변수(Bash/PowerShell 공통 꼴) — `HOME` = 홈 폴더 · `PWD` = 이 패널의 현재 폴더 · 그 밖 = 환경변수 |
//! | `${NAME:-기본}` | 비었거나 없으면 기본값 |
//! | `${NAME%패턴}` · `%%` · `#` · `##` | 뒤/앞에서 패턴(`*` · `?`) 떼기 — 짧게/길게(예: `${PWD%/*}` = 부모 폴더 · `${PWD##*/}` = 폴더 이름) |
//! | `$(basename X [접미])` · `$(dirname X)` · `$(pwd)` | Bash 꼴 |
//! | `$(Split-Path -Leaf X)` · `-Parent` · `-LeafBase` · `-Extension` · `$(Get-Location)` · `$(Join-Path A B)` | PowerShell 꼴 |
//! | `$([IO.Path]::GetFileName(X))` · `GetFileNameWithoutExtension` · `GetExtension` · `GetDirectoryName` | .NET 꼴 |
//! | `~` · `~/x` · `~\x` | 홈 폴더 |
//! | `.` · `..` · `sub\x` | 이 패널의 현재 폴더 기준 상대 경로 |
//! | `"…"` · `'…'` | 감싼 따옴표는 벗긴다 |
//!
//! 정의되지 않은 변수 · 모르는 명령 · 닫히지 않은 괄호는 **원문 그대로** 둔다(열기 실패로 드러난다 — dir2 규약).
//! `shell:` 별칭은 이 단계 뒤에 호스트가 OS 포트로 푼다([`crate::pathinput::is_shell_scheme`]).

use std::path::{Path, PathBuf};

/// 확장에 쓰는 바깥 세계(주입).
pub(crate) struct Ctx<'a> {
    /// 환경변수 조회(없으면 `None`).
    pub env: &'a dyn Fn(&str) -> Option<String>,
    /// 이 패널의 현재 폴더(`$PWD` · 상대 경로의 기준) — 가상 최상위(내 PC)면 `None`.
    pub pwd: Option<&'a Path>,
    /// 홈 폴더(`~` · `$HOME`).
    pub home: Option<PathBuf>,
    /// Windows 규칙인가(구분자 `\` 인식 · 드라이브 문자 · 결과 구분자).
    pub windows: bool,
}

impl Ctx<'_> {
    fn is_sep(&self, c: char) -> bool {
        c == '/' || (self.windows && c == '\\')
    }

    fn sep(&self) -> char {
        if self.windows {
            '\\'
        } else {
            '/'
        }
    }

    /// 변수 값: `HOME` · `PWD`는 주입 값이 먼저 · `USER`는 Windows에서 `USERNAME`으로도 · 그 밖 = 환경변수.
    fn var(&self, name: &str) -> Option<String> {
        match name {
            "HOME" => self
                .home
                .as_ref()
                .map(|h| h.to_string_lossy().into_owned())
                .or_else(|| (self.env)("HOME"))
                .or_else(|| (self.env)("USERPROFILE")),
            "PWD" => self.pwd.map(|p| p.to_string_lossy().into_owned()),
            "USER" => (self.env)("USER").or_else(|| (self.env)("USERNAME")),
            _ => (self.env)(name),
        }
    }
}

/// 경로 입력 → 확장된 경로 글(따옴표 벗기기 → 변수 · 명령 치환 → `~` → 상대 경로).
pub(crate) fn expand(input: &str, ctx: &Ctx) -> String {
    let s = strip_quotes(input.trim());
    let s = expand_vars(s, ctx);
    let s = s.trim().to_string();
    if s.is_empty() || crate::pathinput::is_shell_scheme(&s) || s.starts_with("::") {
        return s;
    }
    let s = expand_tilde(&s, ctx);
    resolve_relative(&s, ctx)
}

/// 감싼 따옴표 한 겹 벗기기(`"…"` · `'…'`).
fn strip_quotes(s: &str) -> &str {
    let b = s.as_bytes();
    if s.len() >= 2
        && ((b[0] == b'"' && b[s.len() - 1] == b'"') || (b[0] == b'\'' && b[s.len() - 1] == b'\''))
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// 변수 · 명령 치환(한 번 훑기 · 치환 결과는 다시 훑지 않는다 — 값 안의 `$` · `%`가 또 풀리지 않게).
fn expand_vars(s: &str, ctx: &Ctx) -> String {
    let chars: Vec<char> = s.chars().collect();
    let text = |a: usize, b: usize| chars[a..b].iter().collect::<String>();
    let mut out = String::with_capacity(s.len());
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c == '$' && chars.get(i + 1) == Some(&'(') {
            // `$( … )` — 괄호 짝(중첩 포함)을 찾는다.
            if let Some(end) = matching(&chars, i + 1, '(', ')') {
                let inner = text(i + 2, end);
                match eval_command(&inner, ctx) {
                    Some(v) => out.push_str(&v),
                    None => out.push_str(&text(i, end + 1)),
                }
                i = end + 1;
                continue;
            }
        } else if c == '$' && chars.get(i + 1) == Some(&'{') {
            if let Some(end) = matching(&chars, i + 1, '{', '}') {
                let body = text(i + 2, end);
                match eval_braced(&body, ctx) {
                    Some(v) => out.push_str(&v),
                    None => out.push_str(&text(i, end + 1)),
                }
                i = end + 1;
                continue;
            }
        } else if c == '$' {
            // `$env:NAME`(대소문자 무시) 또는 `$NAME`.
            let rest = text(i + 1, chars.len().min(i + 5));
            let (skip, env_only) = if rest.eq_ignore_ascii_case("env:") {
                (5, true)
            } else {
                (1, false)
            };
            let start = i + skip;
            let mut end = start;
            while end < chars.len() && is_name_char(chars[end]) {
                end += 1;
            }
            let valid = end > start && (env_only || is_name_start(chars[start]));
            if valid {
                let name = text(start, end);
                let val = if env_only {
                    (ctx.env)(&name)
                } else {
                    ctx.var(&name)
                };
                match val {
                    Some(v) => out.push_str(&v),
                    None => out.push_str(&text(i, end)),
                }
                i = end;
                continue;
            }
        } else if c == '%' {
            // `%NAME%` — 이름에 구분자 · 공백이 끼면 변수가 아니다(`50%\\x%` 같은 글자 그대로의 경로).
            if let Some(rel) = chars[i + 1..].iter().position(|&x| x == '%') {
                let name = text(i + 1, i + 1 + rel);
                let plain = !name.is_empty()
                    && !name
                        .chars()
                        .any(|x| x == '/' || x == '\\' || x.is_whitespace());
                if plain {
                    if let Some(v) = (ctx.env)(&name) {
                        out.push_str(&v);
                        i += rel + 2;
                        continue;
                    }
                }
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// `chars[open_at]`이 여는 괄호일 때 그 짝의 자리(중첩 포함 · 따옴표 안의 괄호는 세지 않는다).
fn matching(chars: &[char], open_at: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    for (k, &c) in chars.iter().enumerate().skip(open_at) {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == open => depth += 1,
            None if c == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(k);
                }
            }
            None => {}
        }
    }
    None
}

/// `${ … }` 안: `env:NAME` · `NAME` · `NAME:-기본` · `NAME%패턴` · `NAME%%패턴` · `NAME#패턴` · `NAME##패턴`.
fn eval_braced(body: &str, ctx: &Ctx) -> Option<String> {
    // 바이트 슬라이스 금지 — 4번째 바이트가 다중 바이트 글자 중간이면 panic(`get`은 경계가 아니면 None).
    if body.len() > 4
        && body
            .get(..4)
            .is_some_and(|p| p.eq_ignore_ascii_case("env:"))
    {
        return (ctx.env)(&body[4..]);
    }
    let name_len = body.chars().take_while(|&c| is_name_char(c)).count();
    if name_len == 0 || !body.chars().next().is_some_and(is_name_start) {
        return None;
    }
    let (name, rest) = body.split_at(name_len); // 이름은 ASCII라 글자 수 = 바이트 수
    if rest.is_empty() {
        return ctx.var(name);
    }
    if let Some(default) = rest.strip_prefix(":-") {
        return Some(match ctx.var(name).filter(|v| !v.is_empty()) {
            Some(v) => v,
            None => expand_vars(default, ctx),
        });
    }
    let value = ctx.var(name)?;
    let (pattern, suffix, longest) = if let Some(p) = rest.strip_prefix("%%") {
        (p, true, true)
    } else if let Some(p) = rest.strip_prefix('%') {
        (p, true, false)
    } else if let Some(p) = rest.strip_prefix("##") {
        (p, false, true)
    } else {
        (rest.strip_prefix('#')?, false, false)
    };
    Some(strip_pattern(&value, pattern, suffix, longest))
}

/// 값의 뒤(`suffix`) 또는 앞에서 패턴에 맞는 부분을 뗀다 — `longest`면 가장 길게 · 아니면 가장 짧게 · 맞는 곳이 없으면 그대로.
fn strip_pattern(value: &str, pattern: &str, suffix: bool, longest: bool) -> String {
    let v: Vec<char> = value.chars().collect();
    let p: Vec<char> = pattern.chars().collect();
    let n = v.len();
    // 자를 자리 후보: 뒤에서 떼기 = v[k..]가 패턴 · 앞에서 떼기 = v[..k]가 패턴.
    let cuts: Box<dyn Iterator<Item = usize>> = match (suffix, longest) {
        (true, false) => Box::new((0..=n).rev()), // 짧은 꼬리부터
        (true, true) => Box::new(0..=n),          // 긴 꼬리부터
        (false, false) => Box::new(0..=n),        // 짧은 머리부터
        (false, true) => Box::new((0..=n).rev()), // 긴 머리부터
    };
    for k in cuts {
        let hit = if suffix {
            glob(&p, &v[k..])
        } else {
            glob(&p, &v[..k])
        };
        if hit {
            return if suffix {
                v[..k].iter().collect()
            } else {
                v[k..].iter().collect()
            };
        }
    }
    value.to_string()
}

/// 셸 글롭(`*` = 아무 글자 0개 이상 · `?` = 한 글자 · 나머지는 그대로) — 전체 일치.
fn glob(p: &[char], s: &[char]) -> bool {
    match p.first() {
        None => s.is_empty(),
        Some('*') => (0..=s.len()).any(|k| glob(&p[1..], &s[k..])),
        Some('?') => !s.is_empty() && glob(&p[1..], &s[1..]),
        Some(c) => s.first() == Some(c) && glob(&p[1..], &s[1..]),
    }
}

/// 명령 줄을 낱말로(따옴표로 묶은 것은 한 낱말 · 따옴표는 벗긴다 · 괄호 안(`$(…)` 중첩)의 빈칸은 낱말을 나누지 않는다).
fn words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut any = false;
    let mut depth = 0usize;
    for c in s.chars() {
        match quote {
            Some(q) if c == q => {
                quote = None;
                if depth > 0 {
                    cur.push(c); // 중첩 명령 안의 따옴표는 그 명령이 벗긴다
                }
            }
            Some(_) => cur.push(c),
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                if depth > 0 {
                    cur.push(c);
                } else {
                    any = true;
                }
            }
            None if c == '(' => {
                depth += 1;
                cur.push(c);
            }
            None if c == ')' => {
                depth = depth.saturating_sub(1);
                cur.push(c);
            }
            None if c.is_whitespace() && depth > 0 => cur.push(c),
            None if c.is_whitespace() => {
                if any || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            None => cur.push(c),
        }
    }
    if any || !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// 경로의 마지막 이름(끝 구분자는 무시 · 구분자가 없으면 전체).
fn leaf(path: &str, ctx: &Ctx) -> String {
    let t = path.trim_end_matches(|c| ctx.is_sep(c));
    match t.rfind(|c| ctx.is_sep(c)) {
        Some(k) => t[k + 1..].to_string(),
        None => t.to_string(),
    }
}

/// 부모 경로(끝 구분자는 무시 · 구분자가 없으면 `None` · 최상위(`/` · `C:\`)의 자식이면 최상위를 구분자째로).
fn parent(path: &str, ctx: &Ctx) -> Option<String> {
    let t = path.trim_end_matches(|c| ctx.is_sep(c));
    let k = t.rfind(|c| ctx.is_sep(c))?;
    let head = &t[..k];
    // `/x` → `/` · `C:\x` → `C:\`
    if head.is_empty() || (ctx.windows && head.len() == 2 && head.ends_with(':')) {
        Some(t[..=k].to_string())
    } else {
        Some(head.to_string())
    }
}

/// (확장자 뺀 이름, 확장자(`.` 포함 · 없으면 빈 글)) — 점으로 시작하는 이름(`.bashrc`)은 확장자가 없다.
fn split_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(k) if k > 0 => name.split_at(k),
        _ => (name, ""),
    }
}

/// `$( … )` 내장 명령(셸을 실행하지 않는다) — 모르는 명령 · 인자 부족 = `None`(원문 유지).
fn eval_command(inner: &str, ctx: &Ctx) -> Option<String> {
    let inner = inner.trim();
    // .NET 정적 메서드: `[IO.Path]::Name(arg)` · `[System.IO.Path]::Name(arg)`.
    let lower = inner.to_ascii_lowercase();
    for prefix in ["[io.path]::", "[system.io.path]::"] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let open = rest.find('(')?;
            let method = &rest[..open];
            let call = &inner[prefix.len()..];
            let close = call.rfind(')')?;
            let arg = expand_vars(strip_quotes(call[open + 1..close].trim()), ctx);
            let name = leaf(&arg, ctx);
            return match method.trim() {
                "getfilename" => Some(name),
                "getfilenamewithoutextension" => Some(split_ext(&name).0.to_string()),
                "getextension" => Some(split_ext(&name).1.to_string()),
                "getdirectoryname" => parent(&arg, ctx),
                _ => None,
            };
        }
    }
    let w: Vec<String> = words(inner)
        .into_iter()
        .map(|t| expand_vars(&t, ctx))
        .collect();
    let cmd = w.first()?.to_ascii_lowercase();
    let args = &w[1..];
    match cmd.as_str() {
        "pwd" | "get-location" | "gl" => ctx.var("PWD"),
        "echo" | "write-output" => Some(args.join(" ")),
        "basename" => {
            let name = leaf(args.first()?, ctx);
            Some(match args.get(1) {
                Some(suffix) if name.len() > suffix.len() && name.ends_with(suffix.as_str()) => {
                    name[..name.len() - suffix.len()].to_string()
                }
                _ => name,
            })
        }
        "dirname" => Some(parent(args.first()?, ctx).unwrap_or_else(|| ".".into())),
        "join-path" => {
            let (a, b) = (args.first()?, args.get(1)?);
            let a = a.trim_end_matches(|c| ctx.is_sep(c));
            let b = b.trim_start_matches(|c| ctx.is_sep(c));
            Some(format!("{a}{}{b}", ctx.sep()))
        }
        "split-path" => {
            // 스위치(-Leaf · -Parent · -LeafBase · -Extension) 하나 + 경로 하나(`-Path X`도 받는다) · 스위치가 없으면 부모.
            let mut mode = "parent".to_string();
            let mut path: Option<&String> = None;
            let mut it = args.iter();
            while let Some(a) = it.next() {
                let l = a.to_ascii_lowercase();
                match l.as_str() {
                    "-leaf" | "-parent" | "-leafbase" | "-extension" => mode = l[1..].to_string(),
                    "-path" | "-literalpath" => path = it.next(),
                    _ if l.starts_with('-') => return None,
                    _ => path = Some(a),
                }
            }
            let path = path?;
            let name = leaf(path, ctx);
            match mode.as_str() {
                "leaf" => Some(name),
                "leafbase" => Some(split_ext(&name).0.to_string()),
                "extension" => Some(split_ext(&name).1.to_string()),
                _ => parent(path, ctx),
            }
        }
        _ => None,
    }
}

/// `~` · `~/x` · `~\x` → 홈 폴더(홈을 모르면 그대로 · `~user`는 지원하지 않는다).
fn expand_tilde(s: &str, ctx: &Ctx) -> String {
    let Some(rest) = s.strip_prefix('~') else {
        return s.to_string();
    };
    let Some(home) = ctx.var("HOME") else {
        return s.to_string();
    };
    match rest.chars().next() {
        None => home,
        Some(c) if ctx.is_sep(c) => {
            let home = home.trim_end_matches(|c| ctx.is_sep(c));
            format!("{home}{}{}", ctx.sep(), &rest[c.len_utf8()..])
        }
        Some(_) => s.to_string(),
    }
}

/// 절대 경로인가(주입한 OS 규칙으로): Unix `/…` · Windows `C:…` · `\\server\…` · `\…`.
fn is_absolute(s: &str, ctx: &Ctx) -> bool {
    let b = s.as_bytes();
    if ctx.windows {
        (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':')
            || s.starts_with('\\')
            || s.starts_with('/')
    } else {
        s.starts_with('/')
    }
}

/// 상대 경로 = 이 패널의 현재 폴더 기준(`.` · `..`는 글자 수준에서 접는다 — 파일 시스템을 건드리지 않는다). 현재 폴더를
/// 모르면(가상 최상위) 그대로.
fn resolve_relative(s: &str, ctx: &Ctx) -> String {
    if is_absolute(s, ctx) {
        return s.to_string();
    }
    let Some(pwd) = ctx.pwd else {
        return s.to_string();
    };
    let base = pwd.to_string_lossy().into_owned();
    // 최상위 머리(`/` · `C:\` · `\\server\share`)는 접지 않는다 — 그 아래 조각만 `.`/`..` 처리.
    let root_len = root_prefix_len(&base, ctx);
    let (root, tail) = base.split_at(root_len);
    let mut parts: Vec<&str> = tail
        .split(|c| ctx.is_sep(c))
        .filter(|p| !p.is_empty())
        .collect();
    for seg in s.split(|c| ctx.is_sep(c)) {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    let sep = ctx.sep().to_string();
    let joined = parts.join(&sep);
    if root.is_empty() {
        joined
    } else if root.ends_with(|c| ctx.is_sep(c)) {
        format!("{root}{joined}")
    } else if joined.is_empty() {
        root.to_string()
    } else {
        format!("{root}{sep}{joined}")
    }
}

/// 경로 머리(접지 않는 부분)의 길이: Unix `/` = 1 · Windows `C:\` = 3 · `C:` = 2 · `\\server\share` = 그만큼 · `\` = 1.
fn root_prefix_len(base: &str, ctx: &Ctx) -> usize {
    let b = base.as_bytes();
    if !ctx.windows {
        return usize::from(base.starts_with('/'));
    }
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        return if b.len() >= 3 && (b[2] == b'\\' || b[2] == b'/') {
            3
        } else {
            2
        };
    }
    if base.starts_with("\\\\") || base.starts_with("//") {
        // \\server\share
        let mut seps = 0;
        for (k, c) in base.char_indices().skip(2) {
            if ctx.is_sep(c) {
                seps += 1;
                if seps == 2 {
                    return k;
                }
            }
        }
        return base.len();
    }
    usize::from(base.starts_with('\\') || base.starts_with('/'))
}

/// 운영 문맥으로 확장(실제 환경변수 · 홈 · 이 OS 규칙) — `pwd` = 이 패널의 현재 폴더(가상 최상위면 `None`).
pub(crate) fn expand_native(input: &str, pwd: Option<&Path>) -> String {
    let env = |name: &str| std::env::var(name).ok();
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from);
    expand(
        input,
        &Ctx {
            env: &env,
            pwd,
            home,
            windows: cfg!(windows),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(name: &str) -> Option<String> {
        // Windows 환경변수 이름은 대소문자를 가리지 않는다 — 시험 표도 그렇게.
        match name.to_ascii_uppercase().as_str() {
            "APPDATA" => Some(r"C:\Users\me\AppData\Roaming".into()),
            "PROGRAMFILES(X86)" => Some(r"C:\Program Files (x86)".into()),
            "EMPTY" => Some(String::new()),
            "FILE" => Some("report.final.txt".into()),
            "USERNAME" => Some("me".into()),
            _ => None,
        }
    }

    fn win(input: &str) -> String {
        expand(
            input,
            &Ctx {
                env: &env,
                pwd: Some(Path::new(r"D:\Work\proj")),
                home: Some(PathBuf::from(r"C:\Users\me")),
                windows: true,
            },
        )
    }

    fn unix(input: &str) -> String {
        expand(
            input,
            &Ctx {
                env: &env,
                pwd: Some(Path::new("/home/me/work/proj")),
                home: Some(PathBuf::from("/home/me")),
                windows: false,
            },
        )
    }

    /// 변수 꼴 전부: CMD · PowerShell · Bash · 중괄호 · 정의 안 된 것은 원문 · 따옴표 벗기기.
    #[test]
    fn variables_in_every_spelling() {
        assert_eq!(win(r"%APPDATA%\x"), r"C:\Users\me\AppData\Roaming\x");
        assert_eq!(win(r"%appdata%"), r"C:\Users\me\AppData\Roaming");
        assert_eq!(win(r"$env:APPDATA\x"), r"C:\Users\me\AppData\Roaming\x");
        assert_eq!(win(r"$ENV:APPDATA"), r"C:\Users\me\AppData\Roaming");
        assert_eq!(
            win(r"${env:ProgramFiles(x86)}\a"),
            r"C:\Program Files (x86)\a"
        );
        assert_eq!(win(r"$HOME\Documents"), r"C:\Users\me\Documents");
        assert_eq!(win(r"${HOME}\Documents"), r"C:\Users\me\Documents");
        assert_eq!(win("$PWD"), r"D:\Work\proj");
        assert_eq!(win(r"${PWD}\sub"), r"D:\Work\proj\sub");
        assert_eq!(
            win(r"C:\Users\$USER"),
            r"C:\Users\me",
            "USER = USERNAME(Windows)"
        );
        assert_eq!(unix("$HOME/docs"), "/home/me/docs");
        assert_eq!(unix("${HOME}/docs"), "/home/me/docs");
        assert_eq!(unix("$PWD/sub"), "/home/me/work/proj/sub");
        // 정의 안 됨 · 이름이 아님 = 원문(상대 경로로 붙지 않게 절대 경로 안에서 본다).
        assert_eq!(win(r"C:\%NOPE%\x"), r"C:\%NOPE%\x");
        assert_eq!(win(r"C:\$NOPE\x"), r"C:\$NOPE\x");
        assert_eq!(win(r"C:\${NOPE}\x"), r"C:\${NOPE}\x");
        assert_eq!(win(r"C:\$env:NOPE"), r"C:\$env:NOPE");
        assert_eq!(win(r"C:\cost$\100%"), r"C:\cost$\100%");
        assert_eq!(
            win(r"C:\50%\x%"),
            r"C:\50%\x%",
            "구분자가 낀 % 쌍은 변수가 아니다"
        );
        assert_eq!(win(r"C:\a$1"), r"C:\a$1", "숫자로 시작 = 이름 아님");
        // 따옴표.
        assert_eq!(win(r#""%APPDATA%\x""#), r"C:\Users\me\AppData\Roaming\x");
        assert_eq!(win(r"'$HOME'"), r"C:\Users\me");
        // 값 안의 기호는 다시 풀지 않는다.
        let tricky = |n: &str| (n == "T").then(|| "%APPDATA%".to_string());
        let ctx = Ctx {
            env: &tricky,
            pwd: None,
            home: None,
            windows: true,
        };
        assert_eq!(expand(r"C:\$T", &ctx), r"C:\%APPDATA%");
    }

    /// `${NAME:-기본}` · 패턴 떼기(`%` `%%` `#` `##`).
    #[test]
    fn braced_defaults_and_pattern_strip() {
        assert_eq!(win(r"${NOPE:-C:\Temp}"), r"C:\Temp");
        assert_eq!(win(r"${EMPTY:-C:\Temp}"), r"C:\Temp", "빈 값도 기본으로");
        assert_eq!(win(r"${APPDATA:-C:\Temp}"), r"C:\Users\me\AppData\Roaming");
        assert_eq!(
            win(r"${NOPE:-$HOME}\x"),
            r"C:\Users\me\x",
            "기본값 안의 변수도 푼다"
        );
        assert_eq!(unix("${PWD%/*}"), "/home/me/work", "부모 폴더");
        assert_eq!(unix("${PWD%%/work*}"), "/home/me");
        assert_eq!(unix("/tmp/${PWD##*/}"), "/tmp/proj", "폴더 이름");
        assert_eq!(unix("/tmp/${PWD#/home/}"), "/tmp/me/work/proj");
        assert_eq!(
            unix("/x/${FILE%.*}"),
            "/x/report.final",
            "확장자 떼기(짧게)"
        );
        assert_eq!(unix("/x/${FILE%%.*}"), "/x/report", "길게");
        assert_eq!(unix("/x/${FILE##*.}"), "/x/txt", "확장자만");
        assert_eq!(
            unix("/x/${FILE%.zip}"),
            "/x/report.final.txt",
            "안 맞으면 그대로"
        );
        assert_eq!(unix("/x/${FILE%.???}"), "/x/report.final", "? = 한 글자");
        assert_eq!(unix("/x/${1bad}"), "/x/${1bad}", "이름이 아님 = 원문");
        assert_eq!(unix("/x/${FILE^^}"), "/x/${FILE^^}", "모르는 연산 = 원문");
        assert_eq!(unix("/x/${HOME"), "/x/${HOME", "닫히지 않음 = 원문");
    }

    /// `$( … )` 내장: Bash · PowerShell · .NET 꼴 — 셸은 실행하지 않는다 · 모르는 명령 = 원문.
    #[test]
    fn command_substitution_builtins_only() {
        assert_eq!(unix("/tmp/$(basename $PWD)"), "/tmp/proj");
        assert_eq!(unix("$(dirname $PWD)"), "/home/me/work");
        assert_eq!(unix("$(dirname $PWD)/other"), "/home/me/work/other");
        assert_eq!(unix("$(pwd)/sub"), "/home/me/work/proj/sub");
        assert_eq!(unix("/x/$(basename /a/b/report.txt .txt)"), "/x/report");
        assert_eq!(
            unix("/x/$(basename '/a/my dir/')"),
            "/x/my dir",
            "따옴표 · 끝 구분자"
        );
        assert_eq!(unix("$(dirname /top)"), "/", "최상위의 자식 = 최상위");
        assert_eq!(unix("/x/$(dirname name)"), "/x/.", "구분자 없음 = .");
        assert_eq!(
            win(r"D:\Backup\$(Split-Path -Leaf $PWD)"),
            r"D:\Backup\proj"
        );
        assert_eq!(win(r"$(Split-Path -Parent $PWD)\other"), r"D:\Work\other");
        assert_eq!(win(r"$(Split-Path $PWD)"), r"D:\Work", "스위치 없음 = 부모");
        assert_eq!(
            win(r"$(split-path -leaf -path 'D:\a b\c')"),
            r"D:\Work\proj\c"
        );
        assert_eq!(
            win(r"D:\$(Split-Path -LeafBase C:\x\report.txt)"),
            r"D:\report"
        );
        assert_eq!(
            win(r"D:\x$(Split-Path -Extension C:\x\report.txt)"),
            r"D:\x.txt"
        );
        assert_eq!(
            win(r"$(Split-Path -Parent C:\x)"),
            r"C:\",
            "드라이브 최상위"
        );
        assert_eq!(win(r"$(Get-Location)\sub"), r"D:\Work\proj\sub");
        assert_eq!(
            win(r"$(Join-Path $HOME Documents)"),
            r"C:\Users\me\Documents"
        );
        assert_eq!(
            win(r"D:\$([IO.Path]::GetFileNameWithoutExtension('C:\x\report.final.txt'))"),
            r"D:\report.final"
        );
        assert_eq!(
            win(r"D:\x$([IO.Path]::GetExtension($env:FILE))"),
            r"D:\x.txt"
        );
        assert_eq!(
            win(r"D:\$([System.IO.Path]::GetFileName($PWD))"),
            r"D:\proj"
        );
        assert_eq!(win(r"$([IO.Path]::GetDirectoryName($PWD))"), r"D:\Work");
        // 중첩.
        assert_eq!(unix("/x/$(basename $(dirname $PWD))"), "/x/work");
        // 모르는 명령 · 인자 부족 · 닫히지 않음 = 원문 그대로(실행하지 않는다).
        assert_eq!(unix("/x/$(rm -rf /)"), "/x/$(rm -rf /)");
        assert_eq!(unix("/x/$(basename)"), "/x/$(basename)");
        assert_eq!(
            unix("/x/$(basename $PWD"),
            "/x/$(basename /home/me/work/proj"
        );
        assert_eq!(
            win(r"D:\$(Split-Path -Qualifier $PWD)"),
            r"D:\$(Split-Path -Qualifier $PWD)",
            "모르는 스위치 = 통째로 원문"
        );
        assert_eq!(
            win(r"D:\$([IO.Path]::Nope(1))"),
            r"D:\$([IO.Path]::Nope(1))"
        );
    }

    /// `~` · 상대 경로(`.` · `..` · 하위) · 절대 경로 · `shell:` · 가상 최상위.
    #[test]
    fn tilde_and_relative_paths() {
        assert_eq!(win("~"), r"C:\Users\me");
        assert_eq!(win(r"~\Documents"), r"C:\Users\me\Documents");
        assert_eq!(win("~/Documents"), r"C:\Users\me\Documents");
        assert_eq!(unix("~/docs"), "/home/me/docs");
        assert_eq!(
            unix("~other/x"),
            "/home/me/work/proj/~other/x",
            "~user는 지원 안 함 = 이름으로"
        );
        assert_eq!(win("."), r"D:\Work\proj");
        assert_eq!(win(".."), r"D:\Work");
        assert_eq!(win(r"..\.."), r"D:\");
        assert_eq!(win(r"..\..\..\.."), r"D:\", "최상위 위로는 가지 않는다");
        assert_eq!(win("sub"), r"D:\Work\proj\sub");
        assert_eq!(win(r".\sub\..\other"), r"D:\Work\proj\other");
        assert_eq!(
            win("../sibling/x"),
            r"D:\Work\sibling\x",
            "/도 구분자(Windows)"
        );
        assert_eq!(unix(".."), "/home/me/work");
        assert_eq!(unix("../../../../.."), "/");
        assert_eq!(unix("a/./b/../c"), "/home/me/work/proj/a/c");
        assert_eq!(
            unix(r"a\b"),
            r"/home/me/work/proj/a\b",
            "Unix에서 \\는 이름 글자"
        );
        // 절대 경로 · 스킴 · 가상 = 그대로.
        assert_eq!(win(r"C:\Windows"), r"C:\Windows");
        assert_eq!(win(r"\\server\share\x"), r"\\server\share\x");
        assert_eq!(win("shell:startup"), "shell:startup");
        assert_eq!(win("Shell:Downloads"), "Shell:Downloads");
        assert_eq!(win("::PC::"), "::PC::");
        assert_eq!(unix("/etc"), "/etc");
        assert_eq!(win(""), "");
        assert_eq!(win("   "), "");
        // 현재 폴더를 모르면(내 PC) 상대 경로는 그대로.
        let none = Ctx {
            env: &env,
            pwd: None,
            home: None,
            windows: true,
        };
        assert_eq!(expand("sub", &none), "sub");
        assert_eq!(expand("~", &none), "~", "홈을 모르면 그대로");
        // UNC 아래에서의 상대 경로: 공유 머리는 접지 않는다.
        let unc = Ctx {
            env: &env,
            pwd: Some(Path::new(r"\\srv\share\a")),
            home: None,
            windows: true,
        };
        assert_eq!(expand(r"..\..\..", &unc), r"\\srv\share");
        assert_eq!(expand("b", &unc), r"\\srv\share\a\b");
    }

    /// 낱말 나누기 · 글롭 · 다중 바이트 글자(panic 없음).
    #[test]
    fn helpers_and_non_ascii() {
        assert_eq!(words(r#"a "b c" 'd e' f"#), ["a", "b c", "d e", "f"]);
        assert_eq!(words(r#"x """#), ["x", ""]);
        let g = |p: &str, s: &str| {
            glob(
                &p.chars().collect::<Vec<_>>(),
                &s.chars().collect::<Vec<_>>(),
            )
        };
        assert!(g("*.txt", "a.txt") && g("a?c", "abc") && g("*", "") && !g("a?c", "ac"));
        for s in [
            r"C:\ㅔ",
            "다운로드",
            r"D:\프로젝트\$없음",
            "%한글%",
            "${한글}",
            "$(한글)",
        ] {
            let _ = win(s);
            let _ = unix(s);
        }
        assert_eq!(win(r"D:\프로젝트\$HOME"), r"D:\프로젝트\C:\Users\me");
        assert_eq!(unix("/tmp/$(basename /가/나/다.txt .txt)"), "/tmp/다");
    }
}
