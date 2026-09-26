use anyhow::{ensure, Context, Result};
use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Run,
    Check,
    Build,
}

pub struct Process {
    child: Child,
    receiver: Receiver<String>,
    pub operation: Operation,
    stop_requested: Option<Instant>,
    finished: bool,
}

impl Process {
    pub fn start(
        runtime: &Path,
        project: &Path,
        arguments: &[OsString],
        operation: Operation,
    ) -> Result<Self> {
        ensure!(
            runtime.is_file(),
            "Kairo runtime is missing; build kairo-cli or select it in Preferences"
        );
        let mut command = Command::new(runtime);
        command
            .args(arguments)
            .current_dir(project)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("RUST_LOG", "warn,kairo=info");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW, not a shell invocation.
        }
        let mut child = command
            .spawn()
            .with_context(|| format!("cannot launch {}", runtime.display()))?;
        let (sender, receiver) = mpsc::sync_channel(2048);
        if let Some(stdout) = child.stdout.take() {
            read_output(stdout, sender.clone());
        }
        if let Some(stderr) = child.stderr.take() {
            read_output(stderr, sender);
        }
        Ok(Self {
            child,
            receiver,
            operation,
            stop_requested: None,
            finished: false,
        })
    }

    pub fn drain(&self) -> Vec<String> {
        self.receiver.try_iter().take(500).collect()
    }

    pub fn debug_command(&mut self, command: &kairo_core::profiler::DebugCommand) -> Result<()> {
        ensure!(
            self.operation == Operation::Run && !self.finished,
            "no running game"
        );
        let input = self
            .child
            .stdin
            .as_mut()
            .context("runtime input is closed")?;
        let mut bytes = serde_json::to_vec(command)?;
        bytes.push(b'\n');
        input
            .write_all(&bytes)
            .context("cannot send debugger command")
    }

    pub fn stop(&mut self) {
        if self.stop_requested.is_some() || self.finished {
            return;
        }
        if let Some(stdin) = &mut self.child.stdin {
            let _ = stdin.write_all(b"stop\n");
        }
        self.stop_requested = Some(Instant::now());
    }

    pub fn poll(&mut self) -> Result<Option<String>> {
        if self.finished {
            return Ok(None);
        }
        if let Some(status) = self.child.try_wait()? {
            self.finished = true;
            let prefix = if status.success() || self.stop_requested.is_some() {
                "[INFO]"
            } else {
                "[ERROR]"
            };
            return Ok(Some(format!(
                "{prefix} {:?} finished: {status}",
                self.operation
            )));
        }
        if self
            .stop_requested
            .is_some_and(|start| start.elapsed() > Duration::from_secs(3))
        {
            self.child
                .kill()
                .context("cannot terminate unresponsive runtime")?;
        }
        Ok(None)
    }

    pub fn finished(&self) -> bool {
        self.finished
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn read_output(mut stream: impl Read + Send + 'static, sender: SyncSender<String>) {
    std::thread::spawn(move || {
        let mut buffer = [0; 4096];
        let mut line = Vec::new();
        let mut truncated = false;
        loop {
            let count = match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            };
            for byte in &buffer[..count] {
                if *byte == b'\n' {
                    let mut message = String::from_utf8_lossy(&line)
                        .trim_end_matches('\r')
                        .to_owned();
                    if truncated {
                        message.push_str(" [line truncated at 128 KiB]");
                    }
                    // Never block the child on a saturated UI console.
                    let _ = sender.try_send(message);
                    line.clear();
                    truncated = false;
                } else if line.len() < 128 * 1024 {
                    line.push(*byte);
                } else {
                    truncated = true;
                }
            }
        }
        if !line.is_empty() {
            let _ = sender.try_send(String::from_utf8_lossy(&line).into_owned());
        }
    });
}

pub fn runtime_path(override_path: Option<&Path>) -> Result<PathBuf> {
    kairo_project::runtime::current_runtime(override_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_reader_bounds_lines_and_decodes_invalid_utf8() {
        let mut bytes = vec![b'x'; 200_000];
        bytes.extend_from_slice(b"\n\xff\nlast");
        let (sender, receiver) = mpsc::sync_channel(10);
        read_output(std::io::Cursor::new(bytes), sender);
        let lines: Vec<_> = receiver.iter().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].len() < 132_000);
        assert!(lines[0].contains("truncated"));
        assert_eq!(lines[2], "last");
    }
}
