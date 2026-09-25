use crate::bundle::{fingerprint, Bundle};
use crate::wire::{Channel, ControlEnvelope, Exchange, Pull, SessionKey, VERSION};
use anyhow::{ensure, Context, Result};
use std::collections::{BTreeMap, VecDeque};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{self, Receiver, SyncSender},
    Arc, Mutex, RwLock,
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct PeerStatus {
    pub id: u64,
    pub address: SocketAddr,
    pub authenticated: bool,
    pub revision: Option<String>,
    pub result: String,
    pub label: String,
    pub telemetry: Option<Box<kairo_core::profiler::Telemetry>>,
    pub latency_ms: Option<f64>,
    pub last_seen: Instant,
    pub auto_reload: bool,
    pub last_command: String,
}
#[derive(Clone, Debug)]
pub enum HostEvent {
    Published(String),
    SourceError(String),
    PeerLog { peer: u64, text: String },
    Connection(String),
}

struct Shared {
    stopped: AtomicBool,
    bundle: RwLock<Arc<Bundle>>,
    sockets: Mutex<BTreeMap<u64, TcpStream>>,
    peers: Mutex<BTreeMap<u64, PeerStatus>>,
    events: SyncSender<HostEvent>,
    pending: Mutex<BTreeMap<u64, VecDeque<ControlEnvelope>>>,
    reload_once: Mutex<std::collections::BTreeSet<u64>>,
    next_command: AtomicU64,
}

pub struct Host {
    pub address: SocketAddr,
    key: SessionKey,
    shared: Arc<Shared>,
    events: Receiver<HostEvent>,
    thread: Option<JoinHandle<()>>,
}

impl Host {
    /// The validator receives an immutable staged revision, not the changing source tree.
    pub fn start(
        root: &Path,
        bind: SocketAddr,
        key: SessionKey,
        validate: fn(&Path) -> Result<()>,
    ) -> Result<Self> {
        let root = root.canonicalize()?;
        let bundle = Bundle::capture(&root)?;
        validate(bundle.stage()?.path())?;
        let listener = TcpListener::bind(bind).context("cannot bind Kairo Link listener")?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let (events_tx, events) = mpsc::sync_channel(256);
        let shared = Arc::new(Shared {
            stopped: AtomicBool::new(false),
            bundle: RwLock::new(Arc::new(bundle)),
            sockets: Mutex::new(BTreeMap::new()),
            peers: Mutex::new(BTreeMap::new()),
            events: events_tx,
            pending: Mutex::new(BTreeMap::new()),
            reload_once: Mutex::new(Default::default()),
            next_command: AtomicU64::new(1),
        });
        let worker_shared = shared.clone();
        let worker_key = key.clone();
        let thread = std::thread::Builder::new()
            .name("kairo-link-host".into())
            .spawn(move || {
                if let Err(error) = serve(listener, &root, &worker_key, &worker_shared, validate) {
                    let _ = worker_shared
                        .events
                        .try_send(HostEvent::SourceError(format!(
                            "Link host stopped: {error:#}"
                        )));
                }
            })?;
        Ok(Self {
            address,
            key,
            shared,
            events,
            thread: Some(thread),
        })
    }

    pub fn command(&self, id: u64, command: kairo_core::profiler::DebugCommand) -> Result<u64> {
        command.validate()?;
        ensure!(
            self.shared
                .peers
                .lock()
                .map_err(|_| anyhow::anyhow!("peer registry unavailable"))?
                .get(&id)
                .is_some_and(|p| p.authenticated),
            "tester is not connected/authenticated"
        );
        let mut queues = self
            .shared
            .pending
            .lock()
            .map_err(|_| anyhow::anyhow!("command queue unavailable"))?;
        let queue = queues.entry(id).or_default();
        ensure!(queue.len() < 32, "tester command queue is full");
        let sequence = self
            .shared
            .next_command
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| anyhow::anyhow!("command IDs exhausted"))?;
        queue.push_back(ControlEnvelope {
            id: sequence,
            command,
        });
        Ok(sequence)
    }
    pub fn configure_peer(&self, id: u64, label: String, auto_reload: bool) -> Result<()> {
        ensure!(
            !label.trim().is_empty() && label.len() <= 64 && !label.chars().any(char::is_control),
            "tester label must contain 1..64 printable bytes"
        );
        let mut peers = self
            .shared
            .peers
            .lock()
            .map_err(|_| anyhow::anyhow!("peer registry unavailable"))?;
        let peer = peers.get_mut(&id).context("tester disconnected")?;
        peer.label = label;
        peer.auto_reload = auto_reload;
        Ok(())
    }
    pub fn push_latest(&self, id: u64) -> Result<()> {
        ensure!(
            self.peers().iter().any(|p| p.id == id && p.authenticated),
            "tester disconnected"
        );
        self.shared
            .reload_once
            .lock()
            .map_err(|_| anyhow::anyhow!("reload queue unavailable"))?
            .insert(id);
        Ok(())
    }
    pub fn token(&self) -> String {
        self.key.expose()
    }
    pub fn peers(&self) -> Vec<PeerStatus> {
        self.shared
            .peers
            .lock()
            .map(|p| p.values().cloned().collect())
            .unwrap_or_default()
    }
    pub fn events(&self) -> Vec<HostEvent> {
        self.events.try_iter().take(256).collect()
    }
    pub fn disconnect(&self, id: u64) -> Result<()> {
        let sockets = self
            .shared
            .sockets
            .lock()
            .map_err(|_| anyhow::anyhow!("Link socket registry unavailable"))?;
        sockets
            .get(&id)
            .context("peer already disconnected")?
            .shutdown(Shutdown::Both)?;
        Ok(())
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        self.shared.stopped.store(true, Ordering::Relaxed);
        if let Ok(sockets) = self.shared.sockets.lock() {
            for socket in sockets.values() {
                let _ = socket.shutdown(Shutdown::Both);
            }
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(
    listener: TcpListener,
    root: &Path,
    key: &SessionKey,
    shared: &Arc<Shared>,
    validate: fn(&Path) -> Result<()>,
) -> Result<()> {
    let mut workers: Vec<JoinHandle<()>> = Vec::new();
    let mut next_id = 1_u64;
    let mut last_scan = Instant::now();
    let result = (|| -> Result<()> {
        let mut signature = fingerprint(root)?;
        let mut candidate_seen = None;
        while !shared.stopped.load(Ordering::Relaxed) {
            workers.retain(|worker| !worker.is_finished());
            match listener.accept() {
                Ok((socket, address)) => {
                    let mut sockets = shared
                        .sockets
                        .lock()
                        .map_err(|_| anyhow::anyhow!("Link socket registry unavailable"))?;
                    if sockets.len() >= 4 {
                        let _ = socket.shutdown(Shutdown::Both);
                    } else {
                        let id = next_id;
                        next_id = next_id.checked_add(1).context("Link peer IDs exhausted")?;
                        sockets.insert(id, socket.try_clone()?);
                        if let Ok(mut peers) = shared.peers.lock() {
                            peers.insert(
                                id,
                                PeerStatus {
                                    id,
                                    address,
                                    authenticated: false,
                                    revision: None,
                                    result: "Authenticating".into(),
                                    label: format!("Tester {id}"),
                                    telemetry: None,
                                    latency_ms: None,
                                    last_seen: Instant::now(),
                                    auto_reload: true,
                                    last_command: String::new(),
                                },
                            );
                        }
                        let key = key.clone();
                        let shared = shared.clone();
                        workers.push(std::thread::spawn(move || {
                            let result = serve_peer(socket, &key, id, &shared);
                            if let Err(error) = result {
                                if !shared.stopped.load(Ordering::Relaxed) {
                                    let _ = shared.events.try_send(HostEvent::Connection(format!(
                                        "Peer {id} disconnected: {error}"
                                    )));
                                }
                            }
                            if let Ok(mut sockets) = shared.sockets.lock() {
                                sockets.remove(&id);
                            }
                            if let Ok(mut peers) = shared.peers.lock() {
                                peers.remove(&id);
                            }
                            if let Ok(mut pending) = shared.pending.lock() {
                                pending.remove(&id);
                            }
                            if let Ok(mut reload) = shared.reload_once.lock() {
                                reload.remove(&id);
                            }
                        }));
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(error.into()),
            }
            if last_scan.elapsed() >= Duration::from_millis(500) {
                last_scan = Instant::now();
                match fingerprint(root) {
                    Ok(next) if next != signature => {
                        signature = next;
                        candidate_seen = Some(Instant::now());
                    }
                    Ok(_) => {}
                    Err(error) => {
                        let _ = shared.events.try_send(HostEvent::SourceError(format!(
                            "Cannot scan project: {error:#}"
                        )));
                    }
                }
            }
            if candidate_seen.is_some_and(|when| when.elapsed() >= Duration::from_millis(300)) {
                candidate_seen = None;
                let update = (|| -> Result<()> {
                    let bundle = Bundle::capture(root)?;
                    if shared
                        .bundle
                        .read()
                        .map_err(|_| anyhow::anyhow!("Link source unavailable"))?
                        .revision
                        == bundle.revision
                    {
                        return Ok(());
                    }
                    validate(bundle.stage()?.path())?;
                    let revision = bundle.revision.clone();
                    *shared
                        .bundle
                        .write()
                        .map_err(|_| anyhow::anyhow!("Link source unavailable"))? =
                        Arc::new(bundle);
                    let _ = shared.events.try_send(HostEvent::Published(revision));
                    Ok(())
                })();
                if let Err(error) = update {
                    let _ = shared.events.try_send(HostEvent::SourceError(format!(
                        "Previous revision retained: {error:#}"
                    )));
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    })();
    shared.stopped.store(true, Ordering::Relaxed);
    if let Ok(sockets) = shared.sockets.lock() {
        for socket in sockets.values() {
            let _ = socket.shutdown(Shutdown::Both);
        }
    }
    for worker in workers {
        let _ = worker.join();
    }
    result
}

fn serve_peer(socket: TcpStream, key: &SessionKey, id: u64, shared: &Shared) -> Result<()> {
    let mut channel = Channel::handshake(socket, key, false).context("authentication failed")?;
    if let Ok(mut peers) = shared.peers.lock() {
        if let Some(peer) = peers.get_mut(&id) {
            peer.authenticated = true;
            peer.result = "Connected".into();
        }
    }
    let _ = shared
        .events
        .try_send(HostEvent::Connection(format!("Peer {id} authenticated")));
    while !shared.stopped.load(Ordering::Relaxed) {
        let pull: Pull = channel.receive()?;
        pull.validate()?;
        ensure!(!shared.stopped.load(Ordering::Relaxed), "host stopped");
        if let Ok(mut peers) = shared.peers.lock() {
            if let Some(peer) = peers.get_mut(&id) {
                peer.last_seen = Instant::now();
                peer.latency_ms = pull.latency_ms;
                if let Some(mut sample) = pull.telemetry {
                    if pull.inspection_unchanged {
                        sample.inspection = peer
                            .telemetry
                            .as_ref()
                            .context("missing initial inspection snapshot")?
                            .inspection
                            .clone();
                    }
                    peer.telemetry = Some(Box::new(sample));
                }
                for result in &pull.results {
                    peer.last_command = format!(
                        "#{} {}: {}",
                        result.id,
                        if result.accepted {
                            "accepted"
                        } else {
                            "rejected"
                        },
                        result.message
                    );
                    let _ = shared.events.try_send(HostEvent::PeerLog {
                        peer: id,
                        text: peer.last_command.clone(),
                    });
                }
            }
        }
        if let Some(report) = pull.report {
            if let Ok(mut peers) = shared.peers.lock() {
                if let Some(peer) = peers.get_mut(&id) {
                    peer.revision = Some(report.revision);
                    peer.result = format!(
                        "{}: {}",
                        if report.accepted {
                            "Applied"
                        } else {
                            "Rejected"
                        },
                        report.message
                    );
                }
            }
        }
        for text in pull.logs {
            let _ = shared
                .events
                .try_send(HostEvent::PeerLog { peer: id, text });
        }
        let bundle = shared
            .bundle
            .read()
            .map_err(|_| anyhow::anyhow!("Link source unavailable"))?
            .clone();
        let commands = shared
            .pending
            .lock()
            .map_err(|_| anyhow::anyhow!("command queue unavailable"))?
            .get_mut(&id)
            .map(|q| {
                let count = q.len().min(8);
                q.drain(..count).collect()
            })
            .unwrap_or_default();
        channel.send(&Exchange {
            version: VERSION,
            commands,
        })?;
        let automatic = shared
            .peers
            .lock()
            .map_err(|_| anyhow::anyhow!("peer registry unavailable"))?
            .get(&id)
            .is_some_and(|p| p.auto_reload);
        let forced = shared
            .reload_once
            .lock()
            .map_err(|_| anyhow::anyhow!("reload queue unavailable"))?
            .remove(&id);
        let changed = pull.revision.as_deref() != Some(bundle.revision.as_str());
        channel.send_bundle(
            if forced || (changed && (automatic || pull.revision.is_none())) {
                Some(&bundle)
            } else {
                None
            },
        )?;
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kairo_core::profiler::DebugCommand;
    fn validate(root: &Path) -> Result<()> {
        ensure!(root.join("main.lua").is_file(), "missing entrypoint");
        Ok(())
    }
    fn pull(channel: &mut Channel) -> Exchange {
        channel.send(&Pull::new(None, None, Vec::new())).unwrap();
        let result = channel.receive::<Exchange>().unwrap();
        result.validate().unwrap();
        channel.receive_bundle().unwrap();
        result
    }
    #[test]
    fn authenticated_testers_receive_only_their_commands_and_disconnect_independently() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("main.lua"), "-- test project").unwrap();
        let key = SessionKey::generate().unwrap();
        let host = Host::start(
            root.path(),
            "127.0.0.1:0".parse().unwrap(),
            key.clone(),
            validate,
        )
        .unwrap();
        let a_socket = TcpStream::connect(host.address).unwrap();
        let a_address = a_socket.local_addr().unwrap();
        let mut a = Channel::handshake(a_socket, &key, true).unwrap();
        let mut b =
            Channel::handshake(TcpStream::connect(host.address).unwrap(), &key, true).unwrap();
        pull(&mut a);
        pull(&mut b);
        let peers = host.peers();
        assert_eq!(peers.len(), 2);
        let a_id = peers.iter().find(|p| p.address == a_address).unwrap().id;
        host.configure_peer(a_id, "Tester A".into(), false).unwrap();
        let id = host.command(a_id, DebugCommand::Pause).unwrap();
        assert!(pull(&mut b).commands.is_empty());
        let exchange = pull(&mut a);
        assert_eq!(exchange.commands.len(), 1);
        assert_eq!(exchange.commands[0].id, id);
        assert!(matches!(&exchange.commands[0].command, DebugCommand::Pause));
        host.disconnect(a_id).unwrap();
        assert!(pull(&mut b).commands.is_empty());
        assert!(host.command(u64::MAX, DebugCommand::Pause).is_err());
    }
}
