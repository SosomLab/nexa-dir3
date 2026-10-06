//! App — 명령 팔레트(T-138 · UIK-104 · 사용자 10-04 "Ctrl+Shift+P" · 컨트롤 = nexa-ctl `Palette`): 명령 = `COMMANDS` 표 전체
//! (메뉴에 있는 것은 "메뉴: 항목" · 오른쪽에 단축키) + 언어 항목(`lang:<code>`) · 최근 실행이 같은 점수 안에서 앞 · 열려 있는 동안
//! 메인 창 입력은 팔레트가 먼저 받는다(모달 · 조합키 명령은 삼키고 편집 4종만 입력란으로 · 토글은 통과).

use crate::*;
use ndir_settings::commands::COMMANDS;
use nexa_ctl::{EditCtxAction, PaletteAction, PaletteItem, PaletteStrings};

/// 최근 실행 기억 개수.
const RECENT_KEEP: usize = 10;

/// 팔레트 글(언어 전환 때마다 다시 넣는다).
pub(crate) fn palette_strings() -> PaletteStrings {
    PaletteStrings {
        placeholder: tr("pal.placeholder"),
        no_match: tr("pal.noMatch"),
        goto_line: String::new(),
        goto_hint: String::new(),
    }
}

/// 열려 있을 때 키맵 조합의 처리(순수): 조합키 없는 키(Enter · Esc · ↑↓ · Delete …)는 사건으로 팔레트에 보내고(`None`) ·
/// 토글 · 편집 4종은 팔레트가 처리(`Some(id)`) · 그 밖의 조합키 명령은 삼킨다(`Some("")`).
pub(crate) fn palette_chord_plan(open: bool, modified: bool, id: &str) -> Option<&str> {
    if !open || !modified {
        return None;
    }
    match id {
        "view.palette" | "edit.copy" | "edit.cut" | "edit.paste" | "edit.select_all" => Some(id),
        _ => Some(""),
    }
}

impl App {
    /// 항목 전체(열 때마다 새로 — 라벨 · 단축키 · 언어 목록이 설정에 따라 바뀐다).
    pub(crate) fn palette_items(&self) -> Vec<PaletteItem> {
        let dot = platform::has_dotfile_toggle();
        let mut out: Vec<PaletteItem> = COMMANDS
            .iter()
            .filter(|c| c.id != "view.palette" && app::menus::menu_has(c.id, dot))
            .map(|c| {
                let item = tr(c.label);
                let label = match app::menus::menu_of(c.id) {
                    Some(m) => format!("{}: {item}", tr(m)),
                    None => item,
                };
                PaletteItem::new(c.id, label).detail(self.keymap.display_of(c.id))
            })
            .collect();
        let home = ndir_settings::config_dir().unwrap_or_else(std::env::temp_dir);
        for (code, name) in ndir_i18n::discover(&home) {
            out.push(PaletteItem::new(
                format!("lang:{code}"),
                format!("{}: {name}", tr("menu.view")),
            ));
        }
        out
    }

    /// `view.palette` — 열려 있으면 닫고 아니면 연다.
    pub(crate) fn toggle_palette(&mut self) {
        if self.palette.is_open() {
            self.palette.close();
        } else {
            self.open_palette("");
        }
        self.redraw();
    }

    pub(crate) fn open_palette(&mut self, prefill: &str) {
        self.palette.set_strings(palette_strings());
        self.palette.set_items(self.palette_items());
        self.palette.set_recent(self.palette_recent.clone());
        self.palette.open(prefill);
        self.redraw();
    }

    /// 입력 고리: 열려 있으면 모든 입력은 팔레트가 먼저(모달). 가져갔으면 `true`.
    pub(crate) fn route_palette(&mut self, ev: &InputEvent, inv: &mut Invalidations) -> bool {
        if !self.palette.is_open() {
            return false;
        }
        let act = self.palette.on_event(ev, inv);
        if let Some(e) = self.palette.take_edit_ctx() {
            self.palette_clip(e, inv);
        }
        match act {
            PaletteAction::None => {}
            PaletteAction::Pick(id) => {
                self.palette.close();
                self.palette_remember(&id);
                self.command(&id);
            }
            PaletteAction::Close | PaletteAction::Goto(_) | PaletteAction::Prompt { .. } => {
                self.palette.close();
            }
        }
        inv.push(Rect::new(0, 0, self.viewport.0, self.viewport.1));
        true
    }

    fn palette_remember(&mut self, id: &str) {
        self.palette_recent.retain(|r| r != id);
        self.palette_recent.insert(0, id.to_string());
        self.palette_recent.truncate(RECENT_KEEP);
    }

    /// 입력란의 클립보드 동작(편집 메뉴 · 단축키).
    fn palette_clip(&mut self, act: EditCtxAction, inv: &mut Invalidations) {
        match act {
            EditCtxAction::Copy => {
                if let Some(t) = self.palette.copy_selection() {
                    let _ = clipboard::write_text(&t);
                }
            }
            EditCtxAction::Cut => {
                if let Some(t) = self.palette.cut_selection(inv) {
                    let _ = clipboard::write_text(&t);
                }
            }
            EditCtxAction::Paste => {
                if let Some(t) = clipboard::read_text() {
                    self.palette.paste(&t, inv);
                }
            }
            EditCtxAction::Custom(_) => {}
        }
    }

    /// 키맵 조합이 왔을 때(열려 있으면): `Some(true)` = 처리/삼킴 · `Some(false)` = 사건으로 흘림 · `None` = 닫혀 있음.
    pub(crate) fn palette_chord(&mut self, ch: &ndir_settings::Chord, id: &str) -> Option<bool> {
        let modified = ch.primary || ch.alt || ch.ctrl;
        if !self.palette.is_open() {
            return None;
        }
        let Some(which) = palette_chord_plan(true, modified, id) else {
            return Some(false);
        };
        let mut inv = Invalidations::default();
        match which {
            "view.palette" => self.toggle_palette(),
            "edit.copy" => self.palette_clip(EditCtxAction::Copy, &mut inv),
            "edit.cut" => self.palette_clip(EditCtxAction::Cut, &mut inv),
            "edit.paste" => self.palette_clip(EditCtxAction::Paste, &mut inv),
            "edit.select_all" => self.palette.select_all(&mut inv),
            _ => {}
        }
        self.redraw();
        Some(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MC/DC: 닫힘 → None · 조합키 없음 → None(사건으로) · 토글/편집 4종 → 그 id · 그 밖 → 삼킴("").
    #[test]
    fn palette_chord_plan_mcdc() {
        assert_eq!(palette_chord_plan(false, true, "edit.copy"), None);
        assert_eq!(palette_chord_plan(true, false, "nav.activate"), None);
        assert_eq!(
            palette_chord_plan(true, true, "view.palette"),
            Some("view.palette")
        );
        assert_eq!(
            palette_chord_plan(true, true, "edit.paste"),
            Some("edit.paste")
        );
        assert_eq!(palette_chord_plan(true, true, "file.new_tab"), Some(""));
    }
}
