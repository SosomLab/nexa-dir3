//! 단축키 맵 엔진(출처: nexa-sql/crates/nexa-sql/src/keymap.rs — winit 변환 `Chord::from_winit`만 앱 크레이트(M3)로 분리 · 나머지 그대로).
//!
//! - [`Chord`] = 정규화된 조합(주 조합키 통일 · 소문자 키 이름) · `parse`/`display`/`code`.
//! - [`Keymap`] = 설정 반영본(`key.<id>`가 비어 있으면 프리셋 기본) · 단일 조합 · 2단 시퀀스 · 충돌 조회.

use crate::commands::{preset_default, setting_key, Preset, COMMANDS};
use crate::Settings;
use std::collections::{HashMap, HashSet};

/// 정규화된 조합(대소문자 무관 · 주 조합키 통일).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Chord {
    pub primary: bool,
    pub shift: bool,
    pub alt: bool,
    /// macOS **Control**(⌘와 별개). 코드에 `cmd`와 `ctrl`이 함께 있거나 `control` 단독일 때만 참.
    pub ctrl: bool,
    /// 소문자 글자 또는 이름(`enter` `tab` `f5` `pageup` `contextmenu` …).
    pub key: String,
}

impl Chord {
    /// `ctrl+shift+p` 같은 문자열 파싱(빈 문자열·모르는 토큰 = None).
    #[must_use]
    pub fn parse(s: &str) -> Option<Chord> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        let mut c = Chord {
            primary: false,
            shift: false,
            alt: false,
            ctrl: false,
            key: String::new(),
        };
        let toks: Vec<&str> = s.split('+').map(str::trim).collect();
        let (mods, key) = toks.split_at(toks.len().saturating_sub(1));
        let mut saw_ctrl = false;
        let mut saw_cmd = false;
        let mut saw_control = false;
        for m in mods {
            match m.to_ascii_lowercase().as_str() {
                "ctrl" => saw_ctrl = true,
                "control" => saw_control = true,
                "cmd" | "command" | "super" | "meta" | "primary" | "win" => saw_cmd = true,
                "shift" => c.shift = true,
                "alt" | "option" | "opt" => c.alt = true,
                _ => return None,
            }
        }
        c.primary = saw_cmd || saw_ctrl;
        c.ctrl = (saw_cmd && saw_ctrl) || saw_control;
        let k = key.first()?.to_ascii_lowercase();
        if k.is_empty() {
            return None;
        }
        c.key = k;
        Some(c)
    }

    /// 표시용(`Ctrl+Shift+P` · macOS `⌘⇧P`).
    #[must_use]
    pub fn display(&self) -> String {
        let key = match self.key.as_str() {
            "enter" => "Enter",
            "tab" => "Tab",
            "space" => "Space",
            "escape" => "Esc",
            "backspace" => {
                if cfg!(target_os = "macos") {
                    "⌫"
                } else {
                    "Backspace"
                }
            }
            "delete" => "Del",
            "pageup" => "PgUp",
            "pagedown" => "PgDn",
            "home" => "Home",
            "end" => "End",
            "up" => "↑",
            "down" => "↓",
            "left" => "←",
            "right" => "→",
            "contextmenu" => "Menu",
            other => {
                return self.prefix() + &other.to_ascii_uppercase();
            }
        };
        self.prefix() + key
    }

    fn prefix(&self) -> String {
        let mac = cfg!(target_os = "macos");
        let mut s = String::new();
        if self.ctrl {
            s.push_str(if mac { "⌃" } else { "Ctrl+" });
        }
        if self.primary {
            s.push_str(if mac { "⌘" } else { "Ctrl+" });
        }
        if self.alt {
            s.push_str(if mac { "⌥" } else { "Alt+" });
        }
        if self.shift {
            s.push_str(if mac { "⇧" } else { "Shift+" });
        }
        s
    }

    /// 저장용 문자열(`ctrl+shift+p` · macOS면 `cmd+…`).
    #[must_use]
    pub fn code(&self) -> String {
        let mut s = String::new();
        if self.ctrl && !self.primary {
            s.push_str("control+");
        } else if self.ctrl {
            s.push_str("ctrl+");
        }
        if self.primary {
            s.push_str(if cfg!(target_os = "macos") {
                "cmd+"
            } else {
                "ctrl+"
            });
        }
        if self.alt {
            s.push_str("alt+");
        }
        if self.shift {
            s.push_str("shift+");
        }
        s + &self.key
    }
}

/// 2단 코드 분리 — 쉼표 앞 글자가 `+`이면 그 쉼표는 **키**(`cmd+,` · `ctrl+alt+,`)라 자르지 않는다.
#[must_use]
pub fn split_seq(code: &str) -> Option<(&str, &str)> {
    let b = code.as_bytes();
    (0..b.len())
        .filter(|&i| b[i] == b',' && i > 0 && b[i - 1] != b'+')
        .map(|i| (&code[..i], &code[i + 1..]))
        .next()
}

/// 조합 → 명령 id 표(설정 반영본).
#[derive(Debug, Default)]
pub struct Keymap {
    map: HashMap<Chord, &'static str>,
    /// 2단 코드 — (첫 조합, 둘째 조합) → 명령.
    seq: HashMap<(Chord, Chord), &'static str>,
    /// 2단 코드의 첫 조합들(누르면 다음 키를 기다린다).
    prefixes: HashSet<Chord>,
    /// 명령별 현재 코드(표시용 · 설정 or 기본).
    codes: HashMap<&'static str, String>,
}

impl Keymap {
    /// 설정에서 조립 — `key.<id>`가 비어 있으면 플랫폼 기본.
    #[must_use]
    pub fn from_settings(s: &Settings) -> Self {
        let preset = Preset::parse(s.get("key.preset").unwrap_or("auto"));
        let mut km = Keymap::default();
        for c in COMMANDS {
            let code = s
                .get(&setting_key(c.id))
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map_or_else(|| preset_default(c, preset).to_string(), String::from);
            for part in code.split('|') {
                if part.trim().eq_ignore_ascii_case("none") {
                    continue;
                }
                if let Some((a, b)) = split_seq(part) {
                    if let (Some(a), Some(b)) = (Chord::parse(a), Chord::parse(b)) {
                        km.prefixes.insert(a.clone());
                        km.seq.insert((a, b), c.id);
                    }
                    continue;
                }
                if let Some(ch) = Chord::parse(part) {
                    km.map.insert(ch, c.id);
                }
            }
            km.codes.insert(c.id, code);
        }
        km
    }

    #[must_use]
    pub fn lookup(&self, ch: &Chord) -> Option<&'static str> {
        self.map.get(ch).copied()
    }

    /// 이 조합이 2단 코드의 첫 키인가(호스트가 다음 키를 기다린다).
    #[must_use]
    pub fn is_prefix(&self, ch: &Chord) -> bool {
        self.prefixes.contains(ch)
    }

    /// 2단 코드 조회.
    #[must_use]
    pub fn lookup_seq(&self, first: &Chord, second: &Chord) -> Option<&'static str> {
        self.seq.get(&(first.clone(), second.clone())).copied()
    }

    /// 명령의 현재 코드(설정 or 기본 · 여러 개면 `|`로).
    #[must_use]
    pub fn code_of(&self, id: &str) -> String {
        self.codes.get(id).cloned().unwrap_or_default()
    }

    /// 표시 문자열(`Ctrl+Shift+P` · 여러 개면 ` / `로 · 2단은 `Ctrl+K, Ctrl+U`).
    #[must_use]
    pub fn display_of(&self, id: &str) -> String {
        self.code_of(id)
            .split('|')
            .filter(|p| !p.trim().eq_ignore_ascii_case("none"))
            .filter_map(|p| {
                let parts: Vec<String> = p
                    .split(',')
                    .filter_map(Chord::parse)
                    .map(|c| c.display())
                    .collect();
                (!parts.is_empty() && parts.len() == p.split(',').count()).then(|| parts.join(", "))
            })
            .collect::<Vec<_>>()
            .join(" / ")
    }

    /// 메뉴 표기용 — 첫 코드만(`Ctrl+Y`).
    #[must_use]
    pub fn menu_display_of(&self, id: &str) -> String {
        self.display_of(id)
            .split(" / ")
            .next()
            .unwrap_or("")
            .to_string()
    }

    /// 이 조합을 이미 쓰는 다른 명령(충돌 표시).
    #[must_use]
    pub fn conflict(&self, ch: &Chord, except: &str) -> Option<&'static str> {
        self.lookup(ch).filter(|id| *id != except)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{command, repeatable};
    use crate::entry;
    use std::path::PathBuf;

    fn fresh() -> Settings {
        Settings::from_text(PathBuf::from("__keymap_test__.conf"), "")
    }

    /// ★ SET-130 교훈: 모든 명령 id에 `key.<id>` 레지스트리 항목이 있고(단축키 창의 재지정이 조용히 버려지지 않게) · 거꾸로 `key.*` 항목은
    /// 전부 명령이다 · id는 유일 · 라벨 i18n 키가 있다.
    #[test]
    fn every_command_has_key_entry_and_label() {
        let mut seen = HashSet::new();
        for c in COMMANDS {
            assert!(seen.insert(c.id), "중복 명령 id: {}", c.id);
            assert!(
                entry(&setting_key(c.id)).is_some(),
                "레지스트리에 key.{} 없음",
                c.id
            );
            assert!(
                ndir_i18n::has(c.label),
                "라벨 i18n 키 없음: {} ({})",
                c.label,
                c.id
            );
            assert!(
                c.id.split_once('.')
                    .is_some_and(|(a, b)| !a.is_empty() && !b.is_empty()),
                "id 꼴: {}",
                c.id
            );
        }
        for e in crate::REGISTRY
            .iter()
            .filter(|e| e.key.starts_with("key.") && e.key != "key.preset")
        {
            assert!(
                command(&e.key[4..]).is_some(),
                "명령이 없는 key 항목: {}",
                e.key
            );
        }
        assert!(COMMANDS.len() >= 45, "{}", COMMANDS.len());
    }

    /// 모든 기본 코드가 세 프리셋에서 파싱된다(대안 `|` · 2단 `,` 포함) · 빈 코드는 허용.
    #[test]
    fn every_default_code_parses() {
        for c in COMMANDS {
            for p in [Preset::Windows, Preset::Macos, Preset::Linux] {
                for alt in preset_default(c, p)
                    .split('|')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    let mut rest = alt;
                    while let Some((a, b)) = split_seq(rest) {
                        assert!(Chord::parse(a).is_some(), "{}: {a}", c.id);
                        rest = b;
                    }
                    assert!(Chord::parse(rest).is_some(), "{}: {rest} ({p:?})", c.id);
                }
            }
        }
    }

    /// 같은 프리셋 안에서 기본 코드 충돌 없음(한 조합 = 한 명령).
    #[test]
    fn no_default_conflicts_within_a_preset() {
        for p in [Preset::Windows, Preset::Macos, Preset::Linux] {
            let mut used: HashMap<String, &str> = HashMap::new();
            for c in COMMANDS {
                for alt in preset_default(c, p)
                    .split('|')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    let ch = Chord::parse(alt)
                        .map(|c| c.code())
                        .unwrap_or_else(|| alt.to_string());
                    if let Some(prev) = used.insert(ch.clone(), c.id) {
                        panic!("{p:?}: {ch} = {prev} 와 {} 충돌", c.id);
                    }
                }
            }
        }
    }

    /// dir2 단축키 전수(CMD-291~329 · Windows 프리셋) — 키 → 명령.
    #[test]
    fn dir2_windows_bindings_resolve() {
        let s = Settings::from_text(PathBuf::from("x"), "key.preset=windows\n");
        let km = Keymap::from_settings(&s);
        for (code, id) in [
            ("ctrl+t", "file.new_tab"),
            ("ctrl+w", "file.close_tab"),
            ("ctrl+shift+n", "file.new_folder"),
            ("ctrl+,", "file.prefs"),
            ("ctrl+z", "edit.undo"),
            ("ctrl+y", "edit.redo"),
            ("ctrl+shift+z", "edit.redo"),
            ("ctrl+x", "edit.cut"),
            ("ctrl+c", "edit.copy"),
            ("ctrl+v", "edit.paste"),
            ("ctrl+a", "edit.select_all"),
            ("ctrl+shift+r", "edit.bulk_rename"),
            ("f2", "edit.rename"),
            ("delete", "edit.delete"),
            ("shift+delete", "edit.delete_permanent"),
            ("ctrl+h", "view.hidden"),
            ("ctrl+.", "view.dot"),
            ("ctrl+`", "view.dock"),
            ("f5", "view.refresh"),
            ("f6", "view.theme_cycle"),
            ("f3", "view.preview_window"),
            ("alt+left", "nav.back"),
            ("alt+right", "nav.forward"),
            ("alt+up", "nav.up"),
            ("alt+down", "nav.activate"),
            ("enter", "nav.activate"),
            ("tab", "panel.switch"),
            ("ctrl+tab", "tab.next"),
            ("ctrl+shift+tab", "tab.prev"),
            ("shift+f10", "list.context_menu"),
            ("contextmenu", "list.context_menu"),
        ] {
            assert_eq!(km.lookup(&Chord::parse(code).unwrap()), Some(id), "{code}");
        }
        assert_eq!(km.lookup(&Chord::parse("ctrl+q").unwrap()), None);
    }

    /// macOS 대응안(CMD-480~497): ⌘ 치환 · ⌘⌫ 휴지통 · ⌃` 도크 · ⌘R 새로 고침 · ⌘[ ⌘] 탐색 · ⇧⌘. 숨김 · ⌘Q 종료.
    #[test]
    fn macos_preset_follows_cmd_480_497() {
        let s = Settings::from_text(PathBuf::from("x"), "key.preset=macos\n");
        let km = Keymap::from_settings(&s);
        let on_mac = cfg!(target_os = "macos");
        // 코드 문자열은 OS 무관하게 같은 Chord로 파싱된다(primary 통일).
        for (code, id) in [
            ("cmd+t", "file.new_tab"),
            ("cmd+q", "file.exit"),
            ("cmd+backspace", "edit.delete"),
            ("cmd+alt+backspace", "edit.delete_permanent"),
            ("control+`", "view.dock"),
            ("cmd+r", "view.refresh"),
            ("cmd+[", "nav.back"),
            ("cmd+]", "nav.forward"),
            ("cmd+up", "nav.up"),
            ("cmd+shift+.", "view.hidden"),
            ("cmd+shift+y", "view.preview_window"),
            ("control+tab", "tab.next"),
            ("control+enter", "list.context_menu"),
        ] {
            assert_eq!(km.lookup(&Chord::parse(code).unwrap()), Some(id), "{code}");
        }
        assert_eq!(
            km.lookup(&Chord::parse("ctrl+h").unwrap()),
            None,
            "⌘H는 앱 숨기기 — 바인딩 없음"
        );
        let g = Chord::parse("control+`").unwrap();
        assert!(g.ctrl && !g.primary, "Control 단독");
        assert_eq!(Chord::parse(&g.code()), Some(g.clone()), "코드 왕복");
        assert_ne!(g, Chord::parse("ctrl+`").unwrap(), "주 조합키와 다르다");
        if on_mac {
            assert!(km.display_of("edit.delete").starts_with("⌘⌫"));
        }
    }

    /// 설정 재정의 · `none` · 2단 코드 · 충돌 조회.
    #[test]
    fn overrides_none_sequences_and_conflicts() {
        let s = Settings::from_text(PathBuf::from("x"), "key.preset=windows\nkey.view.refresh=ctrl+k,ctrl+r\nkey.view.dot=none\nkey.file.new_file=ctrl+n\n");
        let km = Keymap::from_settings(&s);
        let (k, r) = (
            Chord::parse("ctrl+k").unwrap(),
            Chord::parse("ctrl+r").unwrap(),
        );
        assert!(km.is_prefix(&k));
        assert_eq!(km.lookup_seq(&k, &r), Some("view.refresh"));
        assert_eq!(km.lookup(&k), None, "첫 키 단독은 명령이 아니다");
        assert!(km.display_of("view.refresh").contains(", "));
        assert_eq!(
            km.lookup(&Chord::parse("f5").unwrap()),
            None,
            "재정의하면 기본은 사라진다"
        );
        assert_eq!(
            km.lookup(&Chord::parse("ctrl+.").unwrap()),
            None,
            "none = 없음"
        );
        assert_eq!(km.display_of("view.dot"), "");
        assert_eq!(
            km.lookup(&Chord::parse("ctrl+n").unwrap()),
            Some("file.new_file")
        );
        assert_eq!(
            km.conflict(&Chord::parse("ctrl+n").unwrap(), "edit.copy"),
            Some("file.new_file")
        );
        assert_eq!(
            km.conflict(&Chord::parse("ctrl+n").unwrap(), "file.new_file"),
            None
        );
        assert_eq!(
            km.menu_display_of("edit.redo"),
            if cfg!(target_os = "macos") {
                "⌘Y"
            } else {
                "Ctrl+Y"
            }
        );
    }

    #[test]
    fn parse_display_split_seq_and_repeatable() {
        let c = Chord::parse("Ctrl+Shift+P").expect("parse");
        assert!(c.primary && c.shift && !c.alt && c.key == "p");
        assert_eq!(
            Chord::parse("cmd+enter").map(|c| c.key),
            Some("enter".into())
        );
        assert_eq!(
            Chord::parse("f5").map(|c| (c.primary, c.key)),
            Some((false, "f5".into()))
        );
        assert!(Chord::parse("").is_none());
        assert!(Chord::parse("hyper+p").is_none());
        assert_eq!(Chord::parse(&c.code()).expect("code round trip"), c);
        assert_eq!(split_seq("cmd+,"), None);
        assert_eq!(split_seq("ctrl+alt+,"), None);
        assert_eq!(split_seq("ctrl+k,ctrl+u"), Some(("ctrl+k", "ctrl+u")));
        assert_eq!(split_seq("ctrl+k,ctrl+,"), Some(("ctrl+k", "ctrl+,")));
        for id in [
            "edit.undo",
            "edit.redo",
            "nav.back",
            "nav.up",
            "tab.next",
            "tab.prev",
        ] {
            assert!(repeatable(id), "{id}");
        }
        for id in [
            "file.new_tab",
            "file.close_tab",
            "file.prefs",
            "edit.delete",
            "view.refresh",
            "help.about",
            "nope",
        ] {
            assert!(!repeatable(id), "{id}");
        }
        let _ = fresh();
    }
}
