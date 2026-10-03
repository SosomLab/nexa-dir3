//! `impl App` 조각(docs/port/40 SKEL-402) — 상태는 `App` 한 곳(main.rs) · 여기는 동작만.

pub(crate) mod bulk;
pub(crate) mod dialogs;
pub(crate) mod dnd;
mod event_loop;
pub(crate) mod fonts;
mod input;
pub(crate) mod keywinit;
pub(crate) mod launcher_icons;
pub(crate) mod license;
mod menus;
pub(crate) mod ops;
pub(crate) mod order;
mod paint;
pub(crate) mod plugins;
mod previewcmd;
pub(crate) mod row_icons;
mod sessions;
mod settings;
mod startup_cmd;
pub(crate) mod statusline;
mod term;
mod watch;
mod windows;

#[cfg(test)]
mod core_tests;
pub(crate) mod ctxmenu;
