//! 명령행 인자 해석 — 순수 함수(창·OS 없음 · 단위 시험 가능).
//!
//! 어휘(docs/18 §3·§5): `--version` · `--help` · `--smoke` · `--selfcheck [--ci] [--json] [--only <그룹>] [--with-clipboard]`.
//! 그 밖의 인자(시작 경로 등)는 M3에서 `Mode::Gui`에 실어 보낸다.

use crate::selfcheck::Options;

pub(crate) const USAGE: &str =
    "usage: nexa-dir [--version | --help | --smoke | --selfcheck [--ci] [--json] [--only <group>] [--with-clipboard]]";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Version,
    Help,
    Smoke,
    SelfCheck(Options),
    Gui,
}

pub(crate) fn parse(args: &[String]) -> Result<Mode, String> {
    let mut it = args.iter().map(String::as_str).peekable();
    let Some(first) = it.next() else {
        return Ok(Mode::Gui);
    };
    match first {
        "--version" | "-V" => Ok(Mode::Version),
        "--help" | "-h" => Ok(Mode::Help),
        "--smoke" => Ok(Mode::Smoke),
        "--selfcheck" => {
            let mut o = Options::default();
            while let Some(a) = it.next() {
                match a {
                    "--ci" => o.ci = true,
                    "--json" => o.json = true,
                    "--with-clipboard" => o.with_clipboard = true,
                    "--only" => {
                        let g = it
                            .next()
                            .ok_or_else(|| "--only needs a group name".to_string())?;
                        o.only = Some(g.to_string());
                    }
                    other => return Err(format!("unknown --selfcheck option: {other}")),
                }
            }
            Ok(Mode::SelfCheck(o))
        }
        other if other.starts_with("--") => Err(format!("unknown option: {other}")),
        // 경로 인자 등 — M3에서 해석. 지금은 GUI 모드로 흘린다.
        _ => Ok(Mode::Gui),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &[&str]) -> Result<Mode, String> {
        parse(&s.iter().map(|x| x.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn no_args_is_gui() {
        assert_eq!(p(&[]), Ok(Mode::Gui));
        assert_eq!(p(&["C:/"]), Ok(Mode::Gui));
    }

    #[test]
    fn simple_modes() {
        assert_eq!(p(&["--version"]), Ok(Mode::Version));
        assert_eq!(p(&["-V"]), Ok(Mode::Version));
        assert_eq!(p(&["--help"]), Ok(Mode::Help));
        assert_eq!(p(&["--smoke"]), Ok(Mode::Smoke));
    }

    #[test]
    fn selfcheck_options() {
        assert_eq!(p(&["--selfcheck"]), Ok(Mode::SelfCheck(Options::default())));
        let Ok(Mode::SelfCheck(o)) = p(&[
            "--selfcheck",
            "--ci",
            "--json",
            "--only",
            "fs",
            "--with-clipboard",
        ]) else {
            panic!()
        };
        assert!(o.ci && o.json && o.with_clipboard);
        assert_eq!(o.only.as_deref(), Some("fs"));
    }

    #[test]
    fn selfcheck_errors() {
        assert!(p(&["--selfcheck", "--only"]).is_err());
        assert!(p(&["--selfcheck", "--bogus"]).is_err());
        assert!(p(&["--bogus"]).is_err());
    }
}
