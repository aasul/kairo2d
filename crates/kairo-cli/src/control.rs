use anyhow::{Context, Result};
use kairo_core::profiler::DebugCommand;
use std::io::{self, Read};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, SyncSender},
    Arc,
};

pub struct ShutdownSignal {
    stopped: Arc<AtomicBool>,
    commands: Receiver<DebugCommand>,
}

impl ShutdownSignal {
    pub fn listen() -> Result<Self> {
        let stopped = Arc::new(AtomicBool::new(false));
        let signal = stopped.clone();
        let (sender, commands) = mpsc::sync_channel(32);
        std::thread::Builder::new()
            .name("kairo-control".into())
            .spawn(move || {
                read_commands(io::stdin().lock(), &signal, &sender);
            })
            .context("cannot start editor control channel")?;
        Ok(Self { stopped, commands })
    }

    pub fn requested(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }
    pub fn drain(&self) -> Vec<DebugCommand> {
        self.commands.try_iter().take(32).collect()
    }
}

fn read_commands(mut reader: impl Read, stopped: &AtomicBool, sender: &SyncSender<DebugCommand>) {
    let mut buffer = [0_u8; 256];
    let mut line = Vec::new();
    loop {
        let count = match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(count) => count,
        };
        for &byte in &buffer[..count] {
            if byte == b'\n' {
                if line == b"stop" {
                    stopped.store(true, Ordering::Relaxed);
                    return;
                }
                if let Ok(command) = serde_json::from_slice::<DebugCommand>(&line) {
                    let _ = sender.try_send(command);
                }
                line.clear();
            } else if byte != b'\r' {
                if line.len() >= 8192 {
                    stopped.store(true, Ordering::Relaxed);
                    return;
                }
                line.push(byte);
            }
        }
    }
    // EOF also closes a runtime whose owning editor crashed.
    stopped.store(true, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn control_channel_is_bounded_and_eof_stops_runtime() {
        let stopped = AtomicBool::new(false);
        let (sender, receiver) = mpsc::sync_channel(4);
        read_commands(
            b"{\"op\":\"pause\"}\ninvalid\nstop\n".as_slice(),
            &stopped,
            &sender,
        );
        assert!(stopped.load(Ordering::Relaxed));
        assert!(matches!(receiver.try_recv(), Ok(DebugCommand::Pause)));
        assert!(receiver.try_recv().is_err());
    }
}
