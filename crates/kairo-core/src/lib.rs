//! Core engine services shared across platforms.
pub mod animation;
pub mod bookmarks;
pub mod config;
pub mod filesystem;
pub mod frame;
pub mod gamepad;
pub mod handles;
pub mod input;
pub mod micro;
pub mod physics;
pub mod profile;
pub mod profiler;
pub mod scene_graph;

pub use config::Config;
pub use filesystem::ProjectFs;
pub use frame::{Camera, Color, DrawCommand, Frame, PixelRect, Quad, Transform};
pub use handles::{BodyHandle, FontHandle, SoundHandle, TextureHandle, VoiceHandle};
pub use input::InputState;
pub use physics::PhysicsWorld;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn finite(name: &str, values: &[f32]) -> anyhow::Result<()> {
    anyhow::ensure!(
        values.iter().all(|value| value.is_finite()),
        "{name} must be finite"
    );
    Ok(())
}

pub mod debugui;

pub mod extensions;

pub mod actions;
pub mod debug_draw;
pub mod inspector;
pub mod mixer;
pub mod particles;
