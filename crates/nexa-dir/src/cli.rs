//! 명령행 인자 해석 — 순수 함수(창·OS 없음 · 단위 시험 가능).
//!
//! 어휘(docs/18 §3·§5): `--version` · `--help` · `--smoke` · `--selfcheck [--ci] [--json] [--only <그룹>] [--with-clipboard]`.
//! 그 밖의 첫 인자 = **시작 경로**(dir2 KEY-301 · WINA-042) — `Mode::Gui(Some(경로))`로 실어 보낸다(폴더 = 그 폴더 · 파일 = 그 파일의
//! 폴더 — [`start_dir`]). 주면 세션 복원보다 우선한다.

use crate::selfcheck::Options;

pub(crate) const USAGE: &str =
    "usage: nexa-dir [<path>] [--version | --help | --smoke | --selfcheck [--ci] [--json] [--only <group>] [--with-clipboard]]";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Version,
    Help,
    Smoke,
    SelfCheck(Options),
    /// 창 실행 — `Some` = 명령행으로 준 시작 경로(따옴표는 셸이 벗긴다 · 해석은 [`start_dir`]).
    Gui(Option<String>),
}

/// 시작 경로 인자 → 열 폴더(순수 판정 + 파일 시스템 조회): 폴더면 그 폴더 · 파일이면 그 파일이 든 폴더 · 없는 경로면 `None`
/// (세션 복원 · 현재 폴더로 넘어간다).
pub(crate) fn start_dir(arg: &str) -> Option<std::path::PathBuf> {
    let p = std::path::PathBuf::from(arg.trim().trim_matches('"'));
    if p.is_dir() {
        Some(p)
    } else if p.is_file() {
        p.parent()
            .filter(|d| !d.as_os_str().is_empty())
            .map(std::path::Path::to_path_buf)
    } else {
        None
    }
}

pub(crate) fn parse(args: &[String]) -> Result<Mode, String> {
    let mut it = args.iter().map(String::as_str).peekable();
    let Some(first) = it.next() else {
        return Ok(Mode::Gui(None));
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
        // 시작 경로(dir2 KEY-301) — 종전에는 버렸다("M3에서 해석" 주석만 있고 구현이 빠져 있었다 · 10-05 매트릭스 대조 적발).
        path => Ok(Mode::Gui(Some(path.to_string()))),
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
        assert_eq!(p(&[]), Ok(Mode::Gui(None)));
        assert_eq!(p(&["C:/"]), Ok(Mode::Gui(Some("C:/".into()))));
        // 시작 경로 해석: 폴더 = 그대로 · 파일 = 그 폴더 · 없는 경로 = 없음 · 따옴표/빈칸은 벗긴다.
        let dir = std::env::temp_dir().join(format!("ndir-cli-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let file = dir.join("a.txt");
        std::fs::write(&file, b"x").expect("write");
        assert_eq!(start_dir(&dir.display().to_string()), Some(dir.clone()));
        assert_eq!(start_dir(&file.display().to_string()), Some(dir.clone()));
        assert_eq!(
            start_dir(&format!(" \"{}\" ", dir.display())),
            Some(dir.clone())
        );
        assert_eq!(start_dir(&dir.join("nope").display().to_string()), None);
        let _ = std::fs::remove_dir_all(&dir);
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
