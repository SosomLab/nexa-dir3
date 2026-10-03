//! freedesktop 앱 연결(Linux 우클릭 메뉴 통합 1차 · 사용자 10-03 "리눅스의 우클릭 기능을 윈도우처럼 통합") — **순수 해석**.
//!
//! Windows는 셸이 컨텍스트 메뉴 항목을 내주지만(IContextMenu) Linux에는 그런 통로가 없다(노틸러스 메뉴는 노틸러스 안의 것).
//! 대신 노틸러스가 쓰는 것과 같은 표준 정보로 같은 항목을 만든다:
//!
//! - **기본 앱 · 다른 앱**: `mimeapps.list`(`[Default Applications]` · `[Added Associations]`) + `mimeinfo.cache`(`[MIME Cache]`)
//!   → MIME별 `.desktop` id 목록 · 앱 이름/실행 줄은 `.desktop`의 `Name`(현지화 `Name[ko]`)·`Exec`.
//! - **실행 줄**: Desktop Entry `Exec`의 필드 코드(`%f %F %u %U` = 경로 · `%i %c %k` = 버림).
//!
//! 이 파일은 파일을 읽지 않는다(문자열 → 값) — 어느 OS에서나 시험한다. 읽기·실행은 `linux.rs`의 `XdgMenu`.

use std::collections::HashMap;
use std::path::PathBuf;

/// `.desktop`에서 메뉴에 필요한 것.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopApp {
    /// 표시 이름(현지화 우선).
    pub name: String,
    /// `Exec` 줄 원문.
    pub exec: String,
    /// 메뉴에 내지 않는 항목(`NoDisplay` · `Hidden`).
    pub hidden: bool,
}

/// `.desktop` 본문 → 앱(`[Desktop Entry]` 절만 · `Exec` 없으면 `None`). `lang` = 현지화 이름을 찾을 언어 코드(`ko` · `ko_KR`).
pub(crate) fn parse_desktop_app(text: &str, lang: &str) -> Option<DesktopApp> {
    let short = lang.split(['_', '-', '.', '@']).next().unwrap_or("");
    let (mut name, mut loc_full, mut loc_short, mut exec) = (None, None, None, None);
    let mut hidden = false;
    let mut in_entry = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let (k, v) = (k.trim(), v.trim());
        match k {
            "Name" => name = name.or_else(|| Some(v.to_string())),
            "Exec" => exec = exec.or_else(|| Some(v.to_string())),
            "NoDisplay" | "Hidden" => hidden |= v.eq_ignore_ascii_case("true"),
            _ => {
                if let Some(l) = k.strip_prefix("Name[").and_then(|r| r.strip_suffix(']')) {
                    if !lang.is_empty() && l == lang {
                        loc_full = Some(v.to_string());
                    } else if !short.is_empty() && l == short {
                        loc_short = Some(v.to_string());
                    }
                }
            }
        }
    }
    Some(DesktopApp {
        name: loc_full.or(loc_short).or(name)?,
        exec: exec.filter(|e| !e.is_empty())?,
        hidden,
    })
}

/// `key=a.desktop;b.desktop;` 꼴 절 하나를 읽어 `(MIME, [id…])`로(순수). `sections` = 읽을 절 이름들.
pub(crate) fn parse_mime_lists(text: &str, sections: &[&str]) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let mut on = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(sec) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            on = sections.contains(&sec);
            continue;
        }
        if !on || line.starts_with('#') {
            continue;
        }
        let Some((mime, ids)) = line.split_once('=') else {
            continue;
        };
        let ids: Vec<String> = ids
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if !ids.is_empty() {
            out.push((mime.trim().to_string(), ids));
        }
    }
    out
}

/// MIME → 앱 id 목록 표(앞이 우선): 사용자/배포판 기본(`mimeapps.list`들 — 앞 파일이 우선) → 캐시(`mimeinfo.cache`).
#[derive(Debug, Default)]
pub(crate) struct MimeApps {
    by_mime: HashMap<String, Vec<String>>,
}

impl MimeApps {
    /// `mimeapps` = `mimeapps.list` 본문들(우선순위 순) · `caches` = `mimeinfo.cache` 본문들.
    pub(crate) fn build(mimeapps: &[String], caches: &[String]) -> Self {
        let mut by_mime: HashMap<String, Vec<String>> = HashMap::new();
        let mut add = |lists: Vec<(String, Vec<String>)>| {
            for (mime, ids) in lists {
                let slot = by_mime.entry(mime).or_default();
                for id in ids {
                    if !slot.contains(&id) {
                        slot.push(id);
                    }
                }
            }
        };
        for t in mimeapps {
            add(parse_mime_lists(t, &["Default Applications"]));
        }
        for t in mimeapps {
            add(parse_mime_lists(t, &["Added Associations"]));
        }
        for t in caches {
            add(parse_mime_lists(t, &["MIME Cache"]));
        }
        Self { by_mime }
    }

    /// 그 MIME을 여는 앱 id들(첫 번째 = 기본 앱). 모르는 MIME = 빈 목록.
    pub(crate) fn apps_for(&self, mime: &str) -> &[String] {
        self.by_mime.get(mime).map_or(&[], Vec::as_slice)
    }
}

/// `Exec` 줄 → 실행 인자(순수 · Desktop Entry 규격의 필드 코드): `%f`/`%u` = 첫 경로 · `%F`/`%U` = 모든 경로 ·
/// `%i %c %k` 등 = 버림 · `%%` = `%` · 따옴표 묶음 · 경로 자리가 하나도 없으면 끝에 모든 경로를 붙인다.
pub(crate) fn exec_argv(exec: &str, paths: &[PathBuf]) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let mut cur = String::new();
    let (mut quoted, mut have) = (false, false);
    let mut chars = exec.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                have = true;
            }
            '\\' if quoted => {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            c if c.is_whitespace() && !quoted => {
                if have || !cur.is_empty() {
                    words.push(std::mem::take(&mut cur));
                    have = false;
                }
            }
            c => cur.push(c),
        }
    }
    if have || !cur.is_empty() {
        words.push(cur);
    }
    let all = || paths.iter().map(|p| p.to_string_lossy().into_owned());
    let mut out: Vec<String> = Vec::new();
    let mut placed = false;
    for w in words {
        match w.as_str() {
            "%f" | "%u" => {
                placed = true;
                out.extend(all().take(1));
            }
            "%F" | "%U" => {
                placed = true;
                out.extend(all());
            }
            "%i" | "%c" | "%k" | "%d" | "%D" | "%n" | "%N" | "%v" | "%m" => {}
            _ => out.push(w.replace("%%", "%")),
        }
    }
    if !placed {
        out.extend(all());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `.desktop`: 현지화 이름(전체 코드 → 언어만 → 기본) · Exec · 숨김 · 다른 절 무시.
    #[test]
    fn desktop_app_name_exec_and_hidden() {
        let text = "[Desktop Entry]\nName=Text Editor\nName[ko]=텍스트 편집기\nName[ja]=テキストエディター\n\
                    Exec=gnome-text-editor %U\nNoDisplay=false\n\n[Desktop Action new]\nName=New Window\nExec=other\n";
        let app = parse_desktop_app(text, "ko_KR").expect("app");
        assert_eq!(
            (app.name.as_str(), app.exec.as_str(), app.hidden),
            ("텍스트 편집기", "gnome-text-editor %U", false)
        );
        assert_eq!(parse_desktop_app(text, "fr").unwrap().name, "Text Editor");
        assert_eq!(parse_desktop_app(text, "").unwrap().name, "Text Editor");
        let hidden =
            parse_desktop_app("[Desktop Entry]\nName=X\nExec=x\nNoDisplay=true\n", "en").unwrap();
        assert!(hidden.hidden);
        assert_eq!(
            parse_desktop_app("[Desktop Entry]\nName=X\n", "en"),
            None,
            "Exec 없음"
        );
        assert_eq!(parse_desktop_app("[Other]\nName=X\nExec=x\n", "en"), None);
    }

    /// 앱 표: 기본 앱이 맨 앞 · 추가 연결 → 캐시 순 · 중복 제거 · 모르는 MIME = 빈 목록.
    #[test]
    fn mime_apps_default_first_then_cache() {
        let user = "[Default Applications]\ntext/plain=code.desktop;\n\n[Added Associations]\ntext/plain=vim.desktop;code.desktop;\n".to_string();
        let distro = "[Default Applications]\ntext/plain=org.gnome.TextEditor.desktop\ninode/directory=org.gnome.Nautilus.desktop\n".to_string();
        let cache = "[MIME Cache]\ntext/plain=org.gnome.TextEditor.desktop;sublime_text.desktop;vim.desktop;\n\
                     inode/directory=org.gnome.Nautilus.desktop;org.gnome.baobab.desktop;\n"
            .to_string();
        let apps = MimeApps::build(&[user, distro], &[cache]);
        assert_eq!(
            apps.apps_for("text/plain"),
            [
                "code.desktop",
                "org.gnome.TextEditor.desktop",
                "vim.desktop",
                "sublime_text.desktop"
            ]
        );
        assert_eq!(
            apps.apps_for("inode/directory"),
            ["org.gnome.Nautilus.desktop", "org.gnome.baobab.desktop"]
        );
        assert!(apps.apps_for("image/x-unknown").is_empty());
        assert_eq!(
            parse_mime_lists(
                "[A]\nx/y=a.desktop;;b.desktop\n[B]\nx/z=c.desktop\n",
                &["A"]
            ),
            vec![(
                "x/y".to_string(),
                vec!["a.desktop".to_string(), "b.desktop".to_string()]
            )]
        );
    }

    /// 실행 줄: 필드 코드 치환 · 따옴표 · 버리는 코드 · 경로 자리 없으면 끝에 붙임.
    #[test]
    fn exec_field_codes() {
        let p = |v: &[&str]| -> Vec<PathBuf> { v.iter().map(PathBuf::from).collect() };
        let two = p(&["/a/b c.txt", "/d.txt"]);
        assert_eq!(
            exec_argv("gedit %U", &two),
            ["gedit", "/a/b c.txt", "/d.txt"]
        );
        assert_eq!(exec_argv("gedit %f", &two), ["gedit", "/a/b c.txt"]);
        assert_eq!(
            exec_argv("/usr/share/code/code --new-window %F", &two),
            [
                "/usr/share/code/code",
                "--new-window",
                "/a/b c.txt",
                "/d.txt"
            ]
        );
        assert_eq!(
            exec_argv("\"/opt/My App/run\" --icon %i -c %c %u", &p(&["/x"])),
            ["/opt/My App/run", "--icon", "-c", "/x"]
        );
        assert_eq!(
            exec_argv("tool --pct 100%%", &p(&["/x"])),
            ["tool", "--pct", "100%", "/x"]
        );
        assert_eq!(exec_argv("plain", &two), ["plain", "/a/b c.txt", "/d.txt"]);
        assert_eq!(exec_argv("", &p(&[])), Vec::<String>::new());
    }
}
