//! `impl App` 조각(docs/port/40 SKEL-402) — 상태는 `App` 한 곳(main.rs) · 여기는 동작만.

pub(crate) mod dialogs;
mod event_loop;
mod input;
pub(crate) mod keywinit;
mod menus;
pub(crate) mod ops;
mod paint;
mod sessions;
mod settings;
mod startup_cmd;
mod term;
mod watch;
mod windows;

#[cfg(test)]
mod core_tests;
