//! winit 키 사건 → [`Chord`](ndir_settings::keymap::Chord) — nexa-sql `keymap.rs`의 `Chord::from_winit`을 앱 크레이트로 분리(설정 크레이트는 winit을 모른다 · DR-3).
//!
//! ★ 논리 키가 ASCII가 아니면(한글·일본어 IME 모드 — 맥에서 ⌘+T가 `Character("ㅅ")`로 온다 · nexa-sql 09-16) **물리 키**
//! (US 배열 위치 · [`physical_name`])로 이름을 정한다 → Windows처럼 IME 모드와 무관하게 `cmd+t`.
//! 숫자 물리 키는 물리 이름으로(Shift+2의 논리 키는 `@`) · 구두점은 논리 키 그대로(바인딩을 Shift 결과 글자로 정의).

use ndir_settings::keymap::Chord;
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};

fn physical_name(p: &PhysicalKey) -> Option<&'static str> {
    let PhysicalKey::Code(code) = p else {
        return None;
    };
    Some(match code {
        KeyCode::KeyA => "a",
        KeyCode::KeyB => "b",
        KeyCode::KeyC => "c",
        KeyCode::KeyD => "d",
        KeyCode::KeyE => "e",
        KeyCode::KeyF => "f",
        KeyCode::KeyG => "g",
        KeyCode::KeyH => "h",
        KeyCode::KeyI => "i",
        KeyCode::KeyJ => "j",
        KeyCode::KeyK => "k",
        KeyCode::KeyL => "l",
        KeyCode::KeyM => "m",
        KeyCode::KeyN => "n",
        KeyCode::KeyO => "o",
        KeyCode::KeyP => "p",
        KeyCode::KeyQ => "q",
        KeyCode::KeyR => "r",
        KeyCode::KeyS => "s",
        KeyCode::KeyT => "t",
        KeyCode::KeyU => "u",
        KeyCode::KeyV => "v",
        KeyCode::KeyW => "w",
        KeyCode::KeyX => "x",
        KeyCode::KeyY => "y",
        KeyCode::KeyZ => "z",
        KeyCode::Digit0 => "0",
        KeyCode::Digit1 => "1",
        KeyCode::Digit2 => "2",
        KeyCode::Digit3 => "3",
        KeyCode::Digit4 => "4",
        KeyCode::Digit5 => "5",
        KeyCode::Digit6 => "6",
        KeyCode::Digit7 => "7",
        KeyCode::Digit8 => "8",
        KeyCode::Digit9 => "9",
        KeyCode::Backquote => "`",
        KeyCode::Minus => "-",
        KeyCode::Equal => "=",
        KeyCode::BracketLeft => "[",
        KeyCode::BracketRight => "]",
        KeyCode::Backslash => "\\",
        KeyCode::Semicolon => ";",
        KeyCode::Quote => "'",
        KeyCode::Comma => ",",
        KeyCode::Period => ".",
        KeyCode::Slash => "/",
        _ => return None,
    })
}

/// 키 사건 → 조합. 글자 없는 수식키·IME 처리 중인 키는 `None`.
pub(crate) fn chord_from_winit(
    key: &Key,
    physical: &PhysicalKey,
    primary: bool,
    shift: bool,
    alt: bool,
    ctrl: bool,
) -> Option<Chord> {
    let name = match key {
        Key::Character(t) => {
            let c = t.chars().next()?;
            if c.is_control() || c.is_whitespace() {
                return None;
            }
            let digit = physical_name(physical)
                .filter(|n| n.len() == 1 && n.as_bytes()[0].is_ascii_digit());
            if let Some(d) = digit {
                d.to_string()
            } else if c.is_ascii() {
                c.to_ascii_lowercase().to_string()
            } else if let Some(n) = physical_name(physical) {
                n.to_string()
            } else {
                c.to_lowercase().to_string()
            }
        }
        Key::Named(n) => match n {
            NamedKey::Enter => "enter".into(),
            NamedKey::Tab => "tab".into(),
            NamedKey::Space => "space".into(),
            NamedKey::Escape => "escape".into(),
            NamedKey::Backspace => "backspace".into(),
            NamedKey::Delete => "delete".into(),
            NamedKey::PageUp => "pageup".into(),
            NamedKey::PageDown => "pagedown".into(),
            NamedKey::Home => "home".into(),
            NamedKey::End => "end".into(),
            NamedKey::ArrowUp => "up".into(),
            NamedKey::ArrowDown => "down".into(),
            NamedKey::ArrowLeft => "left".into(),
            NamedKey::ArrowRight => "right".into(),
            NamedKey::ContextMenu => "contextmenu".into(),
            NamedKey::F1 => "f1".into(),
            NamedKey::F2 => "f2".into(),
            NamedKey::F3 => "f3".into(),
            NamedKey::F4 => "f4".into(),
            NamedKey::F5 => "f5".into(),
            NamedKey::F6 => "f6".into(),
            NamedKey::F7 => "f7".into(),
            NamedKey::F8 => "f8".into(),
            NamedKey::F9 => "f9".into(),
            NamedKey::F10 => "f10".into(),
            NamedKey::F11 => "f11".into(),
            NamedKey::F12 => "f12".into(),
            _ => return None,
        },
        _ => return None,
    };
    Some(Chord {
        primary,
        shift,
        alt,
        ctrl,
        key: name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::SmolStr;

    fn ch(s: &str) -> Key {
        Key::Character(SmolStr::new(s))
    }

    #[test]
    fn ascii_letters_and_named_keys() {
        let c = chord_from_winit(
            &ch("T"),
            &PhysicalKey::Code(KeyCode::KeyT),
            true,
            false,
            false,
            false,
        )
        .unwrap();
        assert_eq!(c, Chord::parse("ctrl+t").unwrap());
        let f5 = chord_from_winit(
            &Key::Named(NamedKey::F5),
            &PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
            false,
            false,
            false,
            false,
        )
        .unwrap();
        assert_eq!(f5.key, "f5");
        assert!(chord_from_winit(
            &Key::Named(NamedKey::Shift),
            &PhysicalKey::Code(KeyCode::ShiftLeft),
            false,
            true,
            false,
            false
        )
        .is_none());
    }

    /// 한글 IME 모드의 ⌘+T(`ㅅ` · 물리 KeyT) = `cmd+t` · Shift+2(`@` · 물리 Digit2) = `shift+2` · 구두점은 논리 키 그대로.
    #[test]
    fn ime_and_digit_use_physical_key() {
        let c = chord_from_winit(
            &ch("ㅅ"),
            &PhysicalKey::Code(KeyCode::KeyT),
            true,
            false,
            false,
            false,
        )
        .unwrap();
        assert_eq!(c.key, "t");
        let d = chord_from_winit(
            &ch("@"),
            &PhysicalKey::Code(KeyCode::Digit2),
            true,
            true,
            false,
            false,
        )
        .unwrap();
        assert_eq!(d.key, "2");
        let p = chord_from_winit(
            &ch("?"),
            &PhysicalKey::Code(KeyCode::Slash),
            true,
            true,
            false,
            false,
        )
        .unwrap();
        assert_eq!(p.key, "?");
        assert!(chord_from_winit(
            &ch(" "),
            &PhysicalKey::Code(KeyCode::Space),
            false,
            false,
            false,
            false
        )
        .is_none());
    }
}
