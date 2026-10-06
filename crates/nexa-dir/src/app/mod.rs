//! `impl App` 조각(docs/port/40 SKEL-402) — 상태는 `App` 한 곳(main.rs) · 여기는 동작만.

pub(crate) mod bulk;
pub(crate) mod checksum;
pub(crate) mod compare;
pub(crate) mod dialogs;
pub(crate) mod dirsize;
pub(crate) mod dnd;
pub(crate) mod dupes;
mod event_loop;
pub(crate) mod extract;
pub(crate) mod favorites;
pub(crate) mod fonts;
pub(crate) mod input;
pub(crate) mod keywinit;
pub(crate) mod launcher_icons;
pub(crate) mod license;
pub(crate) mod log;
pub(crate) mod memory;
pub(crate) mod menu_icons;
pub(crate) mod menus;
pub(crate) mod ops;
pub(crate) mod order;
mod paint;
pub(crate) mod palette;
pub(crate) mod plugins;
mod previewcmd;
pub(crate) mod row_icons;
mod sessions;
mod settings;
pub(crate) mod slowclick;
mod startup_cmd;
pub(crate) mod statusline;
pub(crate) mod term;
pub(crate) mod watch;
mod windows;

#[cfg(test)]
mod core_tests;
pub(crate) mod ctxmenu;
