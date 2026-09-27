mod app;
mod control;
mod gamepads;
mod input;
mod link;
mod log_tap;
mod watch;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use kairo_core::{Config, ProjectFs};
use kairo_lua::{GameSession, WindowCommand};
use kairo_project::{create_project, Template};
use kairo_render::mesh::Mesh;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "kairo",
    version,
    about = "Kairo2D - a small Rust engine for Lua games"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Host an opt-in authenticated development session.
    LinkHost {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "127.0.0.1:7743")]
        bind: std::net::SocketAddr,
        /// A NEW private token file outside the project. Transfer it securely to testers.
        #[arg(long)]
        token_file: PathBuf,
    },
    /// Report runtime version, executable path, and project checks.
    Doctor {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Open a game project and run its main.lua.
    Run(RunOptions),
    /// Validate configuration and compile every Lua source without executing it.
    Check {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Create a new game directory. Existing paths are never overwritten.
    New {
        path: PathBuf,
        #[arg(long, default_value = "hello-world")]
        template: String,
        #[arg(long)]
        title: Option<String>,
    },
    /// Validate and package a game with this host runtime.
    Build {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long, default_value = "release")]
        profile: String,
    },
}

#[derive(Args)]
struct RunOptions {
    #[arg(long, default_value = "development")]
    profile: String,
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Connect to an explicitly trusted Kairo Link host (development only).
    #[arg(long, requires = "link_token_file")]
    link: Option<String>,
    /// File containing the 64-hex-character shared session token. Never use a command-line token.
    #[arg(long, requires = "link")]
    link_token_file: Option<PathBuf>,
    /// Internal editor control channel: stop on stdin command or EOF.
    #[arg(long, hide = true)]
    controlled: bool,
    /// Disable automatic script reload and exit on the first game error.
    #[arg(long)]
    no_watch: bool,
    /// Do not open an audio output device.
    #[arg(long)]
    no_audio: bool,
    /// Run Lua, physics, asset decoding, and CPU geometry without a window or GPU.
    #[arg(long)]
    headless: bool,
    /// Exit after this many frames; headless mode defaults to 120.
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
    frames: Option<u64>,
}

fn main() -> std::process::ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,kairo=trace"))
        .format(|buffer, record| {
            let message = record.args().to_string();
            let category = match record.target().split("::").next().unwrap_or_default() {
                "kairo_lua" => "lua",
                "kairo_assets" => "assets",
                "kairo_audio" => "audio",
                "kairo_link" => "link",
                "kairo_replay" => "replay",
                "kairo_render" => "render",
                _ => "runtime",
            };
            log_tap::capture(record.level(), &format!("[{category}] {message}"));
            writeln!(buffer, "[{}][{category}] {message}", record.level())
        })
        .init();
    if std::env::var_os("RUST_LOG").is_none() {
        log::set_max_level(log::LevelFilter::Info);
    }
    match entry() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            log::error!("{error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn entry() -> Result<()> {
    if std::env::args_os().len() == 1 {
        if let Some(path) = kairo_project::packaged_project(&std::env::current_exe()?)? {
            return run(Cli {
                command: Command::Run(RunOptions {
                    path,
                    profile: kairo_project::packaged_profile(&std::env::current_exe()?)?
                        .name()
                        .into(),
                    no_watch: true,
                    no_audio: false,
                    headless: false,
                    frames: None,
                    controlled: false,
                    link: None,
                    link_token_file: None,
                }),
            });
        }
    }
    run(Cli::parse())
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::LinkHost {
            path,
            bind,
            token_file,
        } => link::host(path, bind, token_file),
        Command::Doctor { path } => {
            let fs = ProjectFs::new(path)?;
            println!("Kairo runtime {}", kairo_core::VERSION);
            println!("Executable: {}", std::env::current_exe()?.display());
            println!("Project: {}", fs.root().display());
            println!("Lua sources checked: {}", kairo_lua::check_project(&fs)?);
            Ok(())
        }
        Command::Check { path } => {
            let fs = ProjectFs::new(path)?;
            let count = kairo_lua::check_project(&fs)?;
            log::info!("Configuration and {count} Lua source files are valid");
            Ok(())
        }
        Command::New {
            path,
            template,
            title,
        } => {
            let title = title.unwrap_or_else(|| {
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            });
            create_project(&path, &title, Template::parse(&template)?)?;
            log::info!("Created {}", path.display());
            Ok(())
        }
        Command::Build {
            path,
            output,
            profile,
        } => {
            let fs = ProjectFs::new(&path)?;
            let count = kairo_lua::check_project(&fs)?;
            log::info!("Validated {count} Lua sources; packaging host runtime");
            let output = output.unwrap_or_else(|| fs.root().join("dist/game"));
            let report = kairo_project::build_package_with_profile(
                &fs,
                &std::env::current_exe()?,
                &output,
                kairo_core::profile::BuildProfile::parse(&profile)?,
            )?;
            log::info!(
                "Exported {} files ({} bytes) to {}",
                report.files,
                report.bytes,
                report.directory.display()
            );
            log::info!("Launch: {}", report.executable.display());
            Ok(())
        }
        Command::Run(options) => {
            let fs = ProjectFs::new(&options.path)?;
            if !fs.exists("kairo.toml")? {
                log::warn!("Missing optional kairo.toml; using defaults");
            }
            let config = Config::load_profile(
                &fs,
                kairo_core::profile::BuildProfile::parse(&options.profile)?,
            )?;
            anyhow::ensure!(
                options.link.is_none() || config.development.link,
                "Kairo Link is disabled in this profile"
            );
            if std::env::var_os("RUST_LOG").is_none() {
                log::set_max_level(config.development.log_level.parse::<log::LevelFilter>()?);
            }
            log::info!("Kairo2D {} started", kairo_core::VERSION);
            anyhow::ensure!(
                !options.headless || options.link.is_none(),
                "Kairo Link currently requires a windowed runtime"
            );
            if options.headless {
                run_headless(fs, &config, options.frames.unwrap_or(120))
            } else {
                app::run(fs, config, options)
            }
        }
    }
}

fn run_headless(fs: ProjectFs, config: &Config, frames: u64) -> Result<()> {
    let session = GameSession::load(fs, config, false)?;
    let mut mesh = Mesh {
        pixel_snap: config.micro.enabled && config.micro.pixel_snap,
        ..Default::default()
    };
    let mut completed = 0;
    for _ in 0..frames {
        session.tick(1.0 / 60.0)?;
        let commands = std::mem::take(&mut session.state().borrow_mut().window_commands);
        let mut close = false;
        for command in commands {
            match command {
                WindowCommand::Close => close = true,
                WindowCommand::Size(width, height) => {
                    let logical_size = {
                        let mut state = session.state().borrow_mut();
                        state.physical_size = [width, height];
                        if !state.micro.enabled {
                            state.width = width;
                            state.height = height;
                        }
                        (state.width, state.height)
                    };
                    session.call("resized", logical_size)?;
                }
                WindowCommand::Title(_) | WindowCommand::Vsync(_) => {}
            }
        }
        let state = session.state().borrow();
        mesh.build(&state.frame, state.width, state.height)?;
        completed += 1;
        if close {
            break;
        }
    }
    session.call("quit", ())?;
    log::info!("Headless validation completed: {completed} frames (no GPU or audio output)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_requires_positive_frame_counts() {
        assert!(Cli::try_parse_from(["kairo", "run", ".", "--frames", "0"]).is_err());
        assert!(Cli::try_parse_from(["kairo", "run", ".", "--headless", "--frames", "60"]).is_ok());
    }

    #[test]
    fn new_project_is_valid_and_never_overwrites() {
        let parent = tempfile::tempdir().unwrap();
        let path = parent.path().join("game");
        create_project(&path, "Test", Template::HelloWorld).unwrap();
        assert!(create_project(&path, "No overwrite", Template::Empty).is_err());
        let fs = ProjectFs::new(&path).unwrap();
        assert_eq!(kairo_lua::check_project(&fs).unwrap(), 1);
        run_headless(fs.clone(), &Config::load(&fs).unwrap(), 2).unwrap();
    }
}
