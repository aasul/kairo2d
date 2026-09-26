//! Kairo's desktop editor. Games run in a separate process managed by the editor.
mod app;
mod code;
mod console;
mod dialogs;
mod document;
mod highlight;
mod hub;
mod pixels;
mod process;
mod sprite;
mod tabs;
mod workspace;

pub use app::KairoApp;

mod link;

mod animation;

mod feature_settings;
mod features;

mod inspector;

mod mappings;

mod particle_tool;

mod develop;

mod bookmark_panel;
mod commands;
