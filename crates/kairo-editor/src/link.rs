use crate::app::Action;
use anyhow::{ensure, Context, Result};
use eframe::egui;
use kairo_core::ProjectFs;
use kairo_link::{Host, HostEvent, SessionKey};
use std::ffi::OsString;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub(crate) struct LinkConnection {
    pub address: String,
    pub token_file: PathBuf,
}

impl LinkConnection {
    pub(crate) fn validate(&self, project: &Path) -> Result<()> {
        validate_address(&self.address)?;
        let token = self
            .token_file
            .canonicalize()
            .context("choose a readable session token file")?;
        ensure!(
            !token.starts_with(project.canonicalize()?),
            "keep the private token file outside the project"
        );
        SessionKey::read(&token)?;
        Ok(())
    }

    pub(crate) fn arguments(&self) -> Vec<OsString> {
        vec![
            "--link".into(),
            self.address.clone().into(),
            "--link-token-file".into(),
            self.token_file.clone().into_os_string(),
        ]
    }
}

fn validate_address(address: &str) -> Result<()> {
    ensure!(
        address.len() <= 260 && !address.chars().any(char::is_whitespace),
        "enter HOST:PORT without spaces or a URL prefix"
    );
    if let Ok(address) = address.parse::<SocketAddr>() {
        ensure!(
            address.port() != 0 && !address.ip().is_unspecified(),
            "use the host's reachable address and a nonzero port, not a bind wildcard"
        );
        return Ok(());
    }
    let (host, port) = address
        .rsplit_once(':')
        .context("enter a hostname and port, for example dev-pc:7743")?;
    ensure!(
        !host.is_empty()
            && host.len() <= 253
            && host.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'-')
            }),
        "use a valid hostname, IPv4 address, or bracketed IPv6 address"
    );
    ensure!(
        port.parse::<u16>().is_ok_and(|port| port != 0),
        "port must be in 1..=65535"
    );
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LinkMode {
    Host,
    Tester,
}

pub struct LinkTools {
    host: Option<Host>,
    mode: LinkMode,
    bind: String,
    address: String,
    token_file: Option<PathBuf>,
    trust_host: bool,
    show_token: bool,
    error: Option<String>,
}

impl Default for LinkTools {
    fn default() -> Self {
        Self {
            host: None,
            mode: LinkMode::Host,
            bind: "127.0.0.1:7743".into(),
            address: "127.0.0.1:7743".into(),
            token_file: None,
            trust_host: false,
            show_token: false,
            error: None,
        }
    }
}

impl LinkTools {
    pub fn stop(&mut self) {
        self.host = None;
        self.show_token = false;
    }

    pub fn reset(&mut self) {
        self.stop();
        self.trust_host = false;
        self.token_file = None;
        self.error = None;
    }

    pub fn peers(&self) -> Vec<kairo_link::PeerStatus> {
        self.host.as_ref().map_or_else(Vec::new, Host::peers)
    }
    pub fn command(&self, peer: u64, command: kairo_core::profiler::DebugCommand) -> Result<u64> {
        self.host
            .as_ref()
            .context("no Link host is active")?
            .command(peer, command)
    }

    pub fn hosting(&self) -> bool {
        self.host.is_some()
    }

    pub fn status(&self) -> String {
        self.host.as_ref().map_or_else(
            || "OFF".into(),
            |host| format!("HOSTING | {} tester(s)", host.peers().len()),
        )
    }

    pub fn start(&mut self, project: &Path, bind: SocketAddr) -> Result<()> {
        ensure!(
            self.host.is_none(),
            "stop the current Link session before starting another"
        );
        self.host = Some(Host::start(
            project,
            bind,
            SessionKey::generate()?,
            validate_project,
        )?);
        self.show_token = false;
        self.error = None;
        Ok(())
    }

    pub fn logs(&self) -> Vec<String> {
        self.host
            .as_ref()
            .map(|host| {
                host.events()
                    .into_iter()
                    .map(|event| match event {
                        HostEvent::Published(revision) => {
                            format!("[INFO][link] Published {}", &revision[..12])
                        }
                        HostEvent::SourceError(message) => format!("[WARN][link] {message}"),
                        HostEvent::Connection(message) => format!("[INFO][link] {message}"),
                        HostEvent::PeerLog { peer, text } => format!("[REMOTE {peer}] {text}"),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn ui(
        &mut self,
        ctx: &egui::Context,
        open: &mut bool,
        project: Option<&Path>,
        busy: bool,
        tester: Option<&str>,
    ) -> Option<Action> {
        let mut action = None;
        egui::Window::new("Kairo Link").id(egui::Id::new("link-window"))
            .open(open).default_width(540.0).vscroll(true).show(ctx, |ui| {
                ui.label("Live testing - experimental");
                ui.small("Closing this panel does not stop a session. Use Stop hosting or Stop tester below.");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.mode, LinkMode::Host, "Host a session");
                    ui.selectable_value(&mut self.mode, LinkMode::Tester, "Join as tester");
                });
                ui.separator();
                if let Some(project) = project {
                    ui.small(format!("Project: {}", project.display()));
                } else {
                    ui.label("Open a project first. The host and tester need the same starting project/config.");
                    if ui.button("Open project...").clicked() { action = Some(Action::OpenDialog); }
                }
                match self.mode {
                    LinkMode::Host => self.host_ui(ui, project, &mut action),
                    LinkMode::Tester => self.tester_ui(ui, project, busy, tester, &mut action),
                }
                if let Some(error) = &self.error {
                    ui.separator();
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
            });
        action
    }

    fn host_ui(&mut self, ui: &mut egui::Ui, project: Option<&Path>, action: &mut Option<Action>) {
        if let Some(host) = &self.host {
            ui.strong(format!("Hosting on {}", host.address));
            if host.address.ip().is_unspecified() {
                ui.label("Give your tester your PC's LAN IP address, not 0.0.0.0 or ::.");
            }
            ui.horizontal_wrapped(|ui| {
                if ui.button("Save private token file...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("kairo-session.token")
                        .save_file()
                    {
                        let result = (|| -> Result<()> {
                            let parent = path
                                .parent()
                                .context("invalid token path")?
                                .canonicalize()?;
                            if let Some(project) = project {
                                ensure!(
                                    !parent.starts_with(project),
                                    "keep the token outside the shared project"
                                );
                            }
                            SessionKey::parse(&host.token())?.write_new(&path)
                        })();
                        match result {
                            Ok(()) => self.error = None,
                            Err(error) => self.error = Some(format!("{error:#}")),
                        }
                    }
                }
                if ui.button("Copy token").clicked() {
                    ui.output_mut(|out| out.copied_text = host.token());
                }
                ui.checkbox(&mut self.show_token, "Reveal token");
            });
            if self.show_token {
                ui.monospace(host.token());
            }
            ui.label("Tester: open the same project in Kairo, choose Link > Join as tester, enter your address and select the private token file.");
            ui.small("Share the token privately, outside the project. Stopping and hosting again creates a new token.");
            ui.separator();
            let peers = host.peers();
            if peers.is_empty() {
                ui.label("Waiting for a tester. Save Lua or textures to publish changes.");
            }
            for peer in peers {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let mut label = peer.label.clone();
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut label)
                                    .desired_width(180.0)
                                    .char_limit(64),
                            )
                            .changed()
                        {
                            if let Err(e) = host.configure_peer(peer.id, label, peer.auto_reload) {
                                self.error = Some(e.to_string());
                            }
                        }
                        ui.small(peer.address.to_string());
                        if ui.button("Inspect").clicked() {
                            *action = Some(Action::InspectPeer(peer.id));
                        }
                        if ui.button("Disconnect").clicked() {
                            if let Err(error) = host.disconnect(peer.id) {
                                self.error = Some(error.to_string());
                            }
                        }
                    });
                    ui.label(&peer.result);
                    ui.horizontal(|ui| {
                        let mut auto = peer.auto_reload;
                        if ui.checkbox(&mut auto, "Automatic reload").changed() {
                            if let Err(e) = host.configure_peer(peer.id, peer.label.clone(), auto) {
                                self.error = Some(e.to_string());
                            }
                        }
                        if ui.button("Push latest").clicked() {
                            if let Err(e) = host.push_latest(peer.id) {
                                self.error = Some(e.to_string());
                            }
                        }
                        if let Some(ms) = peer.latency_ms {
                            ui.small(format!("{ms:.0} ms exchange RTT"));
                        }
                    });
                    ui.small(format!(
                        "Last seen {:.1}s ago | {}",
                        peer.last_seen.elapsed().as_secs_f32(),
                        peer.last_command
                    ));
                    if let Some(sample) = &peer.telemetry {
                        ui.small(format!(
                            "Scene {} | {:.0} FPS | replay {} snapshots",
                            sample.inspection.scenes.current,
                            sample.profile.fps,
                            sample.replay.snapshots
                        ));
                    }
                    if let Some(revision) = peer.revision {
                        ui.small(format!("Revision {}", &revision[..12]));
                    }
                });
            }
            if ui.button("Push latest to all testers").clicked() {
                for peer in host.peers() {
                    if let Err(e) = host.push_latest(peer.id) {
                        self.error = Some(e.to_string());
                    }
                }
            }
            if ui.button("Stop hosting").clicked() {
                self.stop();
            }
        } else {
            ui.label("1. Choose who can reach this session");
            ui.horizontal_wrapped(|ui| {
                if ui
                    .selectable_label(self.bind == "127.0.0.1:7743", "This PC only")
                    .clicked()
                {
                    self.bind = "127.0.0.1:7743".into();
                }
                if ui
                    .selectable_label(self.bind == "0.0.0.0:7743", "LAN / other PCs")
                    .clicked()
                {
                    self.bind = "0.0.0.0:7743".into();
                }
            });
            ui.horizontal(|ui| {
                ui.label("Bind address");
                ui.text_edit_singleline(&mut self.bind);
            });
            ui.small("Localhost is the default. LAN hosting requires an appropriate TCP firewall rule. This does not create public matchmaking or NAT traversal.");
            ui.add_space(8.0);
            ui.label("2. Start, then securely share the private token with your tester");
            if ui
                .add_enabled(
                    project.is_some(),
                    egui::Button::new("Save all & Start hosting"),
                )
                .clicked()
            {
                match self.bind.parse::<SocketAddr>() {
                    Ok(bind) => {
                        self.error = None;
                        *action = Some(Action::HostLink(bind));
                    }
                    Err(error) => self.error = Some(format!("Invalid bind address: {error}")),
                }
            }
            ui.small("Hosting never starts automatically. Project scripts/assets are shared, not editor access or a remote shell.");
        }
    }

    fn tester_ui(
        &mut self,
        ui: &mut egui::Ui,
        project: Option<&Path>,
        busy: bool,
        tester: Option<&str>,
        action: &mut Option<Action>,
    ) {
        if let Some(address) = tester {
            ui.strong(format!("Tester runtime launched for {address}"));
            ui.label("Connection, authentication and reload results appear in the Console. A launched runtime is not proof that the remote host accepted it.");
            if ui.button("Stop tester").clicked() {
                *action = Some(Action::Stop);
            }
        }
        ui.label("1. Host address");
        if ui
            .add(
                egui::TextEdit::singleline(&mut self.address)
                    .hint_text("192.168.1.20:7743 or dev-pc:7743"),
            )
            .changed()
        {
            self.trust_host = false;
        }
        ui.label("2. Private token file received from the host");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Choose token file...").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Private Kairo Link token (outside the project)")
                    .pick_file()
                {
                    self.token_file = Some(path);
                    self.trust_host = false;
                }
            }
            ui.small(self.token_file.as_ref().map_or_else(
                || "No file selected".into(),
                |path| path.display().to_string(),
            ));
        });
        ui.label("3. Accept remote game code");
        ui.checkbox(
            &mut self.trust_host,
            "I trust this host to send Lua that runs on my machine",
        );
        ui.small("Use only trusted development sessions. The token is read from a file; it is not placed in process arguments or saved in Preferences.");
        let ready = project.is_some() && !busy && self.token_file.is_some() && self.trust_host;
        if ui
            .add_enabled(ready, egui::Button::new("Connect & Run as tester"))
            .clicked()
        {
            if let (Some(project), Some(token_file)) = (project, &self.token_file) {
                let connection = LinkConnection {
                    address: self.address.trim().to_owned(),
                    token_file: token_file.clone(),
                };
                match connection.validate(project) {
                    Ok(()) => {
                        self.error = None;
                        *action = Some(Action::ConnectLink(connection));
                    }
                    Err(error) => self.error = Some(format!("{error:#}")),
                }
            }
        }
        if busy && tester.is_none() {
            ui.small("Stop the active Run / Check / Build before joining.");
        }
        ui.separator();
        ui.small("Start with the same project/config. Runtime reconnect is automatic. State survives reload only through supported saveState/restoreState hooks.");
    }
}

fn validate_project(path: &Path) -> Result<()> {
    kairo_lua::check_project(&ProjectFs::new(path)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tester_address_validation_is_local_and_rejects_wildcards_and_urls() {
        for address in [
            "127.0.0.1:7743",
            "dev-pc:7743",
            "dev.local:443",
            "[::1]:7743",
        ] {
            validate_address(address).unwrap();
        }
        for address in [
            "",
            "0.0.0.0:7743",
            "[::]:7743",
            "dev-pc:0",
            "dev:70000",
            "https://dev:7743",
            "dev pc:7743",
            "--run:123",
        ] {
            assert!(validate_address(address).is_err(), "accepted {address}");
        }
    }

    #[test]
    fn connection_arguments_contain_a_file_path_not_a_secret() {
        let connection = LinkConnection {
            address: "dev-pc:7743".into(),
            token_file: PathBuf::from("private session.token"),
        };
        assert_eq!(
            connection.arguments(),
            vec![
                OsString::from("--link"),
                OsString::from("dev-pc:7743"),
                OsString::from("--link-token-file"),
                OsString::from("private session.token")
            ]
        );
    }

    #[test]
    fn joining_rejects_tokens_inside_the_shared_project() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("game");
        std::fs::create_dir(&project).unwrap();
        let token = project.join("session.token");
        SessionKey::generate().unwrap().write_new(&token).unwrap();
        let mut connection = LinkConnection {
            address: "localhost:7743".into(),
            token_file: token,
        };
        assert!(connection.validate(&project).is_err());
        connection.token_file = temp.path().join("session.token");
        SessionKey::generate()
            .unwrap()
            .write_new(&connection.token_file)
            .unwrap();
        connection.validate(&project).unwrap();
    }
}
