use crate::wire::{
    Channel, ControlEnvelope, ControlResult, Exchange, Pull, ReloadReport, SessionKey,
};
use crate::Bundle;
use anyhow::{Context, Result};
use std::collections::VecDeque;
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, SyncSender},
    Arc, Mutex,
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub struct Client {
    incoming: Receiver<Bundle>,
    reports: SyncSender<ReloadReport>,
    logs: Arc<Mutex<VecDeque<String>>>,
    status: Arc<Mutex<String>>,
    telemetry: Arc<Mutex<Option<kairo_core::profiler::Telemetry>>>,
    results: Arc<Mutex<VecDeque<ControlResult>>>,
    commands: Receiver<ControlEnvelope>,
    stopped: Arc<AtomicBool>,
    socket: Arc<Mutex<Option<TcpStream>>>,
    thread: Option<JoinHandle<()>>,
}
impl Client {
    pub fn connect(address: String, key: SessionKey) -> Result<Self> {
        let (incoming_tx, incoming) = mpsc::sync_channel(1);
        let (reports, report_rx) = mpsc::sync_channel(4);
        let (commands_tx, commands) = mpsc::sync_channel(32);
        let telemetry = Arc::new(Mutex::new(None));
        let results = Arc::new(Mutex::new(VecDeque::new()));
        let logs = Arc::new(Mutex::new(VecDeque::new()));
        let status = Arc::new(Mutex::new("Connecting".to_owned()));
        let stopped = Arc::new(AtomicBool::new(false));
        let socket = Arc::new(Mutex::new(None));
        let worker = Worker {
            address,
            key,
            incoming: incoming_tx,
            reports: report_rx,
            logs: logs.clone(),
            status: status.clone(),
            telemetry: telemetry.clone(),
            results: results.clone(),
            commands: commands_tx,
            stopped: stopped.clone(),
            socket: socket.clone(),
        };
        let thread = std::thread::Builder::new()
            .name("kairo-link-client".into())
            .spawn(move || worker.run())?;
        Ok(Self {
            incoming,
            reports,
            logs,
            status,
            telemetry,
            results,
            commands,
            stopped,
            socket,
            thread: Some(thread),
        })
    }

    pub fn update_telemetry(&self, sample: kairo_core::profiler::Telemetry) -> Result<()> {
        sample.validate()?;
        *self
            .telemetry
            .lock()
            .map_err(|_| anyhow::anyhow!("telemetry unavailable"))? = Some(sample);
        Ok(())
    }
    pub fn take_commands(&self) -> Vec<ControlEnvelope> {
        self.commands.try_iter().take(32).collect()
    }
    pub fn command_result(&self, id: u64, result: Result<()>) {
        if let Ok(mut results) = self.results.lock() {
            if results.len() >= 32 {
                results.pop_front();
            }
            results.push_back(ControlResult {
                id,
                accepted: result.is_ok(),
                message: match result {
                    Ok(()) => "Accepted; deferred operations run at the next frame boundary".into(),
                    Err(error) => truncate(&format!("{error:#}"), 1024),
                },
            });
        }
    }
    pub fn take_update(&self) -> Option<Bundle> {
        self.incoming.try_recv().ok()
    }
    pub fn report(&self, mut report: ReloadReport) {
        report.message = truncate(&report.message, 8192);
        let _ = self.reports.try_send(report);
    }
    pub fn log(&self, text: &str) {
        if let Ok(mut logs) = self.logs.lock() {
            if logs.len() >= 128 {
                logs.pop_front();
            }
            logs.push_back(truncate(text, 2048));
        }
    }
    pub fn status(&self) -> String {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|_| "Unavailable".into())
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        if let Ok(socket) = self.socket.lock() {
            if let Some(socket) = &*socket {
                let _ = socket.shutdown(Shutdown::Both);
            }
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn truncate(text: &str, max: usize) -> String {
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_owned()
}
struct Worker {
    address: String,
    key: SessionKey,
    incoming: SyncSender<Bundle>,
    reports: Receiver<ReloadReport>,
    logs: Arc<Mutex<VecDeque<String>>>,
    status: Arc<Mutex<String>>,
    telemetry: Arc<Mutex<Option<kairo_core::profiler::Telemetry>>>,
    results: Arc<Mutex<VecDeque<ControlResult>>>,
    commands: SyncSender<ControlEnvelope>,
    stopped: Arc<AtomicBool>,
    socket: Arc<Mutex<Option<TcpStream>>>,
}
impl Worker {
    fn run(self) {
        let mut revision = None;
        while !self.stopped.load(Ordering::Relaxed) {
            let result = self.connected(&mut revision);
            if let Err(error) = result {
                if let Ok(mut status) = self.status.lock() {
                    *status = format!("Disconnected; retrying: {error}");
                }
            }
            if let Ok(mut socket) = self.socket.lock() {
                *socket = None;
            }
            let wait = Instant::now();
            while wait.elapsed() < Duration::from_secs(2) && !self.stopped.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    fn connected(&self, revision: &mut Option<String>) -> Result<()> {
        let addresses: Vec<_> = self.address.to_socket_addrs()?.take(4).collect();
        let mut connection = None;
        for address in addresses {
            if self.stopped.load(Ordering::Relaxed) {
                return Ok(());
            }
            if let Ok(socket) = TcpStream::connect_timeout(&address, Duration::from_secs(3)) {
                connection = Some(socket);
                break;
            }
        }
        let socket = connection.context("cannot connect to Link host")?;
        if let Ok(mut slot) = self.socket.lock() {
            *slot = Some(socket.try_clone()?);
        }
        let mut channel =
            Channel::handshake(socket, &self.key, true).context("Link authentication failed")?;
        if let Ok(mut status) = self.status.lock() {
            *status = "Connected".into();
        }
        let mut last_inspection = Vec::new();
        let mut latency_ms = None;
        while !self.stopped.load(Ordering::Relaxed) {
            let report = self.reports.try_iter().last();
            let logs = self
                .logs
                .lock()
                .map(|mut logs| {
                    let count = logs.len().min(4);
                    logs.drain(..count).collect()
                })
                .unwrap_or_default();
            let mut pull = Pull::new(revision.clone(), report, logs);
            pull.telemetry = self.telemetry.lock().ok().and_then(|sample| sample.clone());
            if let Some(sample) = &mut pull.telemetry {
                let encoded = serde_json::to_vec(&sample.inspection)?;
                if encoded == last_inspection {
                    sample.inspection = Default::default();
                    pull.inspection_unchanged = true;
                } else {
                    last_inspection = encoded;
                }
            }
            pull.results = self
                .results
                .lock()
                .map(|mut q| {
                    let n = q.len().min(8);
                    q.drain(..n).collect()
                })
                .unwrap_or_default();
            pull.latency_ms = latency_ms;
            // Logs are best-effort. Never tear down an authenticated session just
            // because a large inspector snapshot and a burst of logs share a poll.
            while serde_json::to_vec(&pull)?.len() > 65_519 && !pull.logs.is_empty() {
                pull.logs.pop();
            }
            if serde_json::to_vec(&pull)?.len() > 65_519 {
                for result in &mut pull.results {
                    result.message = truncate(&result.message, 128);
                }
                if let Some(report) = &mut pull.report {
                    report.message = truncate(&report.message, 512);
                }
            }
            pull.validate()?;
            let sent = Instant::now();
            channel.send(&pull)?;
            let exchange: Exchange = channel.receive()?;
            exchange.validate()?;
            for command in exchange.commands {
                if let Err(error) = self.commands.try_send(command) {
                    if let mpsc::TrySendError::Full(command) = error {
                        if let Ok(mut results) = self.results.lock() {
                            if results.len() >= 32 {
                                results.pop_front();
                            }
                            results.push_back(ControlResult {
                                id: command.id,
                                accepted: false,
                                message: "Runtime control queue is full".into(),
                            });
                        }
                    } else {
                        return Ok(());
                    }
                }
            }
            latency_ms = Some(sent.elapsed().as_secs_f64() * 1000.0);
            if let Some(bundle) = channel.receive_bundle()? {
                let next = bundle.revision.clone();
                match self.incoming.try_send(bundle) {
                    Ok(()) => *revision = Some(next),
                    Err(mpsc::TrySendError::Full(_)) => {}
                    Err(mpsc::TrySendError::Disconnected(_)) => return Ok(()),
                }
            }
            let wait = Instant::now();
            while wait.elapsed() < Duration::from_millis(500)
                && !self.stopped.load(Ordering::Relaxed)
            {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        Ok(())
    }
}
