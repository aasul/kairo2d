//! Project operations used by the CLI and desktop editor.
pub mod dependencies;
mod files;
mod package;
pub mod runtime;
pub mod saves;
pub mod search;
mod settings;
mod templates;

pub use files::{atomic_write, ProjectFiles, ProjectNode};
pub use package::{
    build_package, build_package_with_profile, packaged_profile, packaged_project, PackageReport,
};
pub use settings::SettingsDocument;
pub use templates::{create_project, Template};
