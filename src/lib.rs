//! Shared contrast, palette, and terminal logic.
//! The window crate depends on this library and does not link a second model.

pub mod app;
pub mod cli;
pub mod color;
pub mod commands;
pub mod contrast;
pub mod fix;
pub mod layout;
pub mod palette;
pub mod theme;
pub mod toast_chrome;
pub mod ui;
pub mod web_preview;

mod tui;

pub use tui::run;
