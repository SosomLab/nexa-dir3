//! 파일/폴더 고르기 창(T-29 B · T-80 LIC-106 · nexa-sql `file_win.rs` 이식) — **자체 대화상자**(네이티브 0 · 3-OS 동일).
//!
//! 본체는 nexa-dlg [`FilePicker`](복합 컨트롤). 이 모듈은 창(winit + softbuffer)과 이벤트 번역만 맡는다. Esc = 취소 · Enter = 확정.
//! 용도(라이선스 파일 · 설정의 폴더 찾아보기)는 호스트 `App.file_purpose`가 쥔다.

use ndir_i18n::tr;
use nexa_ctl::draw::{DrawCtx, FontSlot};
use nexa_ctl::geom::Rect;
use nexa_ctl::raster::RasterCtx;
use nexa_ctl::theme::{FontPrefs, Theme};
use nexa_ctl::{Control, InputEvent, Invalidations, Key as CtlKey, TextBox, Widget};
use nexa_dlg::{FileFilter, FilePicker, PickerAction, PickerLabels, PickerMode};
use nexa_gfx::{Font, Surface};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// 창 → 호스트.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileWinAction {
    None,
    Paint,
    /// 확정 경로(열기 = 파일 · 폴더 고르기 = 폴더).
    Confirm(PathBuf),
    /// 취소/닫힘.
    Cancel,
    /// 클립보드에 쓸 텍스트(경로/이름 복사).
    CopyText(String),
}

pub(crate) struct FileWin {
    window: Option<Rc<Window>>,
    surface: Option<crate::present::Presenter>,
    scale: f32,
    cursor: (i32, i32),
    shift: bool,
    /// 주 수식키(Windows·Linux = Ctrl · macOS = ⌘) — Ctrl+A.
    primary: bool,
    picker: Option<FilePicker>,
}

/// 앱 문자열 → 선택기 라벨(i18n 규칙: 리터럴은 여기 없다).
pub(crate) fn labels() -> PickerLabels {
    PickerLabels {
        file_name: tr("fdlg.fileName"),
        file_type: tr("fdlg.fileType"),
        ok_open: tr("fdlg.ok.open"),
        ok_save: tr("fdlg.ok.save"),
        ok_folder: tr("fdlg.ok.folder"),
        folder_name: tr("fdlg.folderName"),
        cancel: tr("dlg.cancel"),
        new_folder: tr("fdlg.newFolder"),
        new_folder_name: tr("fdlg.newFolderName"),
        show_hidden: tr("fdlg.showHidden"),
        show_dot: tr("fdlg.showDot"),
        col_name: tr("fdlg.col.name"),
        col_modified: tr("fdlg.col.modified"),
        col_size: tr("fdlg.col.size"),
        col_kind: tr("fdlg.col.kind"),
        kind_folder: tr("fdlg.kind.folder"),
        kind_file: tr("fdlg.kind.file"),
        place_home: tr("fdlg.place.home"),
        place_desktop: tr("fdlg.place.desktop"),
        place_documents: tr("fdlg.place.documents"),
        place_downloads: tr("fdlg.place.downloads"),
        place_drives: tr("fdlg.place.drives"),
        kind_drive: tr("fdlg.kind.drive"),
        place_recent: tr("fdlg.place.recent"),
        path_hint: tr("fdlg.pathHint"),
        err_not_found: tr("fdlg.err.notFound"),
        err_exists: tr("fdlg.err.exists"),
        overwrite_ask: tr("fdlg.overwrite.ask"),
        overwrite_yes: tr("fdlg.overwrite.yes"),
        err_bad_name: tr("fdlg.err.badName"),
        err_list: tr("fdlg.err.list"),
        err_mkdir: tr("fdlg.err.mkdir"),
        menu_open: tr("fdlg.menu.open"),
        menu_copy_path: tr("fdlg.menu.copyPath"),
        menu_copy_name: tr("fdlg.menu.copyName"),
        menu_refresh: tr("fdlg.menu.refresh"),
        multi_selected: tr("fdlg.multiSelected"),
    }
}

/// 라이선스 파일 필터 — `.license` 기본 · 전체.
pub(crate) fn license_filters() -> Vec<FileFilter> {
    vec![
        FileFilter::new(tr("license.filter"), &["license"]),
        FileFilter::new(tr("fdlg.filter.all"), &[]),
    ]
}

impl FileWin {
    pub(crate) fn new() -> Self {
        FileWin {
            window: None,
            surface: None,
            scale: 1.0,
            cursor: (0, 0),
            shift: false,
            primary: false,
            picker: None,
        }
    }

    pub(crate) fn is(&self, id: WindowId) -> bool {
        self.window.as_ref().is_some_and(|w| w.id() == id)
    }

    pub(crate) fn is_open(&self) -> bool {
        self.window.is_some()
    }

    pub(crate) fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    pub(crate) fn tick(&mut self, now_ms: u64) -> bool {
        self.window.is_some() && self.picker.as_mut().is_some_and(|p| p.tick(now_ms))
    }

    pub(crate) fn animating(&self) -> bool {
        self.window.is_some() && self.picker.as_ref().is_some_and(FilePicker::animating)
    }

    /// 창 열기 — `mode`(열기/폴더) · `start` 폴더 · 필터(폴더 모드는 무시).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn open(
        &mut self,
        el: &ActiveEventLoop,
        theme: Option<winit::window::Theme>,
        over: Option<(i32, i32, u32, u32)>,
        owner: Option<&Window>,
        mode: PickerMode,
        start: Option<&Path>,
        filters: Vec<FileFilter>,
    ) {
        if let Some(w) = &self.window {
            w.focus_window();
            return;
        }
        // 폴더 고르기 = 파일 필터가 뜻이 없다(목록에 폴더만) → "폴더" 한 줄.
        let filters = if mode == PickerMode::Folder {
            vec![FileFilter::new(tr("fdlg.filter.folders"), &[])]
        } else {
            filters
        };
        let mut picker = FilePicker::new(mode, start, filters, labels());
        picker.set_multi(false);
        picker.set_show_dot(true);
        self.picker = Some(picker);
        let (lw, lh) = (900.0, 580.0);
        let title = match mode {
            PickerMode::Open | PickerMode::Save => tr("fdlg.title.open"),
            PickerMode::Folder => tr("fdlg.title.folder"),
        };
        let mut attrs = Window::default_attributes()
            .with_title(format!("Nexa Dir — {title}"))
            .with_theme(theme)
            .with_resizable(true)
            .with_inner_size(winit::dpi::LogicalSize::new(lw, lh))
            .with_min_inner_size(winit::dpi::LogicalSize::new(640.0, 420.0));
        if let Some((x, y, w, h)) = over {
            let cx = x + (w as i32 - lw as i32) / 2;
            let cy = y + (h as i32 - lh as i32) / 2;
            attrs = attrs.with_position(winit::dpi::PhysicalPosition::new(cx.max(0), cy.max(0)));
        }
        let attrs = crate::winfocus::owned_by(crate::icon::with_icon(attrs), owner);
        let Ok(win) = el.create_window(attrs) else {
            self.picker = None;
            return;
        };
        let win = Rc::new(win);
        self.scale = win.scale_factor() as f32;
        self.surface = crate::present::Presenter::new(win.clone()).ok();
        win.set_ime_allowed(crate::input::system_ime());
        self.window = Some(win);
        self.layout();
        self.redraw();
    }

    pub(crate) fn close(&mut self) {
        self.surface = None;
        self.window = None;
        self.picker = None;
    }

    fn layout(&mut self) {
        let Some(w) = &self.window else { return };
        let sz = w.inner_size();
        let r = Rect::new(0, 0, sz.width as i32, sz.height as i32);
        if let Some(p) = &mut self.picker {
            p.set_scale(self.scale);
            let mut inv = Invalidations::default();
            p.set_bounds(r, &mut inv);
        }
    }

    fn focused_textbox(&mut self) -> Option<&mut TextBox> {
        self.picker.as_mut().and_then(FilePicker::focused_textbox)
    }

    fn to_input(&self, ev: &WindowEvent) -> Option<InputEvent> {
        let (x, y) = self.cursor;
        let key = |k: CtlKey| InputEvent::Key {
            key: k,
            shift: self.shift,
            primary: self.primary,
        };
        Some(match ev {
            WindowEvent::CursorMoved { position, .. } => InputEvent::MouseMove {
                x: position.x as i32,
                y: position.y as i32,
            },
            WindowEvent::MouseInput { state, button, .. } => match (state, button) {
                (ElementState::Pressed, MouseButton::Left) => InputEvent::MouseDown {
                    x,
                    y,
                    shift: self.shift,
                    primary: self.primary,
                },
                (ElementState::Released, MouseButton::Left) => InputEvent::MouseUp { x, y },
                (ElementState::Pressed, MouseButton::Right) => InputEvent::RightDown { x, y },
                _ => return None,
            },
            WindowEvent::MouseWheel { delta, .. } => crate::input::wheel_event(delta, self.shift),
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => {
                match kev.logical_key.as_ref() {
                    Key::Named(NamedKey::Enter) => key(CtlKey::Enter),
                    Key::Named(NamedKey::ArrowLeft) => key(CtlKey::Left),
                    Key::Named(NamedKey::ArrowRight) => key(CtlKey::Right),
                    Key::Named(NamedKey::ArrowUp) => key(CtlKey::Up),
                    Key::Named(NamedKey::ArrowDown) => key(CtlKey::Down),
                    Key::Named(NamedKey::Home) => key(CtlKey::Home),
                    Key::Named(NamedKey::End) => key(CtlKey::End),
                    Key::Named(NamedKey::Delete) => key(CtlKey::Delete),
                    Key::Named(NamedKey::Backspace) => InputEvent::Char {
                        c: '\u{8}',
                        now_ms: 0,
                    },
                    Key::Named(NamedKey::PageUp) => key(CtlKey::PageUp),
                    Key::Named(NamedKey::PageDown) => key(CtlKey::PageDown),
                    Key::Named(NamedKey::Space) if self.primary => key(CtlKey::Space),
                    Key::Named(NamedKey::Space) => InputEvent::Char { c: ' ', now_ms: 0 },
                    Key::Character(t) => {
                        let c = t.chars().next()?;
                        if c.is_control() {
                            return None;
                        }
                        // 주 수식키 + 글자 = 단축키(Ctrl+A 전체 선택) — 글자로 넣지 않는다.
                        if self.primary {
                            return (crate::input::shortcut_letter(kev) == Some('a'))
                                .then_some(InputEvent::SelectAll);
                        }
                        InputEvent::Char { c, now_ms: 0 }
                    }
                    _ => return None,
                }
            }
            _ => return None,
        })
    }

    pub(crate) fn handle(&mut self, ev: &WindowEvent) -> FileWinAction {
        match ev {
            WindowEvent::CloseRequested => {
                self.close();
                return FileWinAction::Cancel;
            }
            WindowEvent::Resized(_) => {
                self.layout();
                self.redraw();
                return FileWinAction::None;
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = *scale_factor as f32;
                self.layout();
                self.redraw();
                return FileWinAction::None;
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.primary = if cfg!(target_os = "macos") {
                    m.state().super_key()
                } else {
                    m.state().control_key()
                };
                return FileWinAction::None;
            }
            WindowEvent::Ime(ime) => {
                let mut inv = Invalidations::default();
                if let Some(tb) = self.focused_textbox() {
                    match ime {
                        winit::event::Ime::Preedit(t, _) => tb.set_preedit(t, &mut inv),
                        winit::event::Ime::Commit(t) => {
                            tb.set_preedit("", &mut inv);
                            for c in t.chars().filter(|c| !c.is_control()) {
                                tb.on_event(&InputEvent::Char { c, now_ms: 0 }, &mut inv);
                            }
                        }
                        _ => {}
                    }
                    self.redraw();
                }
                return FileWinAction::None;
            }
            WindowEvent::KeyboardInput { event: kev, .. }
                if kev.state == ElementState::Pressed
                    && matches!(kev.logical_key.as_ref(), Key::Named(NamedKey::Escape)) =>
            {
                if self.picker.as_ref().is_some_and(FilePicker::popup_open) {
                    // 열린 콤보가 Esc를 받는다(컨트롤 몫).
                } else {
                    self.close();
                    return FileWinAction::Cancel;
                }
            }
            WindowEvent::RedrawRequested => return FileWinAction::Paint,
            _ => {}
        }
        if let WindowEvent::CursorMoved { position, .. } = ev {
            self.cursor = (position.x as i32, position.y as i32);
            // 헤더 경계 위 = ↔ 커서(컬럼 폭 조절).
            let over = self
                .picker
                .as_ref()
                .is_some_and(|p| p.header_edge_hover(self.cursor.0, self.cursor.1));
            if let Some(w) = &self.window {
                w.set_cursor(if over {
                    winit::window::CursorIcon::ColResize
                } else {
                    winit::window::CursorIcon::Default
                });
            }
        }
        let Some(ie) = self.to_input(ev) else {
            return FileWinAction::None;
        };
        let mut inv = Invalidations::default();
        let Some(p) = &mut self.picker else {
            return FileWinAction::None;
        };
        p.on_event(&ie, &mut inv);
        let a = p.take_action();
        self.redraw();
        match a {
            PickerAction::Confirm(path) => {
                self.close();
                FileWinAction::Confirm(path)
            }
            PickerAction::ConfirmMany(mut paths) => {
                self.close();
                paths
                    .pop()
                    .map_or(FileWinAction::Cancel, FileWinAction::Confirm)
            }
            PickerAction::Cancel => {
                self.close();
                FileWinAction::Cancel
            }
            PickerAction::CopyText(t) => FileWinAction::CopyText(t),
            PickerAction::None => FileWinAction::None,
        }
    }

    pub(crate) fn paint(&mut self, ui: &Font, th: &Theme, font_px: f32) {
        let (Some(win), Some(surface)) = (self.window.clone(), self.surface.as_mut()) else {
            return;
        };
        let size = win.inner_size();
        let Some(mut buf) = surface.frame(size) else {
            return;
        };
        {
            let mut gfx = Surface::new(&mut buf, size.width as usize, size.height as usize);
            let prefs = FontPrefs::with_base(font_px);
            let mut dc = RasterCtx::new(&mut gfx, ui, self.scale).with_fonts(prefs);
            dc.fill_rect(
                Rect::new(0, 0, size.width as i32, size.height as i32),
                th.window_bg,
            );
            dc.select_font(FontSlot::Base, false);
            if let Some(p) = &self.picker {
                p.paint(&mut dc, th);
            }
        }
        let _ = buf.present();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 라벨은 전부 i18n 키에서(빈 라벨 0) · 라이선스 필터 = `.license` 기본 + 전체.
    #[test]
    fn labels_and_license_filters() {
        ndir_i18n::activate(ndir_i18n::load("en", std::path::Path::new("nowhere")));
        let l = labels();
        assert_eq!(l.ok_open, "Open");
        assert_eq!(l.cancel, "Cancel");
        assert!(!l.multi_selected.is_empty() && !l.err_mkdir.is_empty());
        let f = license_filters();
        assert_eq!(f[0].exts, vec!["license".to_string()]);
        assert!(f[1].exts.is_empty());
        let w = FileWin::new();
        assert!(!w.is_open() && !w.animating());
    }
}
