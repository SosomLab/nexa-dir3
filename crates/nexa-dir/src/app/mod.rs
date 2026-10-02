//! `impl App` 조각(docs/port/40 SKEL-402) — 상태는 `App` 한 곳(main.rs) · 여기는 동작만.

mod event_loop;
mod input;
mod keywinit;
mod menus;
mod paint;
mod sessions;
mod startup_cmd;

#[cfg(test)]
mod core_tests;
