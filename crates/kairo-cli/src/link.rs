use anyhow::{ensure, Result};
use kairo_core::ProjectFs;
use kairo_link::{Host, HostEvent, SessionKey};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

pub fn validate(path: &Path) -> Result<()> {
    kairo_lua::check_project(&ProjectFs::new(path)?)?;
    Ok(())
}

pub fn host(path: PathBuf, bind: SocketAddr, token_file: PathBuf) -> Result<()> {
    let root = path.canonicalize()?;
    let parent = token_file
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()?;
    ensure!(
        !parent.starts_with(&root),
        "store the private session token outside the shared project"
    );
    let key = SessionKey::generate()?;
    let host = Host::start(&root, bind, key.clone(), validate)?;
    key.write_new(&token_file)?;
    log::info!("[link] Hosting on {}. Token stored in the selected file; share it securely. Ctrl+C stops hosting.", host.address);
    loop {
        for event in host.events() {
            match event {
                HostEvent::Published(revision) => {
                    log::info!("[link] Published {}", &revision[..12])
                }
                HostEvent::SourceError(error) => log::warn!("[link] {error}"),
                HostEvent::PeerLog { peer, text } => log::info!("[link][peer {peer}] {text}"),
                HostEvent::Connection(message) => log::info!("[link] {message}"),
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
