//! `impl App` 조각(docs/port/40 SKEL-402) — 상태는 `App` 한 곳(main.rs) · 여기는 동작만.

pub(crate) mod bulk;
pub(crate) mod dialogs;
mod event_loop;
mod fonts;
mod input;
pub(crate) mod keywinit;
pub(crate) mod license;
mod menus;
pub(crate) mod ops;
pub(crate) mod order;
mod paint;
mod previewcmd;
mod sessions;
mod settings;
mod startup_cmd;
mod term;
mod watch;
mod windows;

#[cfg(test)]
mod core_tests;
pub(crate) mod ctxmenu;
