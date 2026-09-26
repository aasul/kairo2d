use crate::bundle::{validate_path, Bundle, SourceFile, MAX_BYTES, MAX_FILES, MAX_FILE_BYTES};
use anyhow::{ensure, Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::{Duration, Instant};

pub(crate) const VERSION: u32 = 2;
const FRAME_LIMIT: usize = 65_535;
const CHUNK: usize = 8192;
const PROLOGUE: &[u8] = b"KairoLink/v2/Lua54/Kairo3.4";

#[derive(Clone)]
pub struct SessionKey([u8; 32]);
impl SessionKey {
    pub fn generate() -> Result<Self> {
        let mut key = [0; 32];
        getrandom::fill(&mut key)
            .map_err(|e| anyhow::anyhow!("cannot generate session key: {e}"))?;
        Ok(Self(key))
    }
    pub fn parse(text: &str) -> Result<Self> {
        let text = text.trim();
        ensure!(
            text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit()),
            "session token must be 64 hexadecimal characters"
        );
        let mut key = [0; 32];
        for (index, byte) in key.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)?;
        }
        Ok(Self(key))
    }
    pub fn read(path: &Path) -> Result<Self> {
        ensure!(
            path.metadata()?.len() <= 128,
            "session token file is too large"
        );
        Self::parse(&std::fs::read_to_string(path)?)
    }
    /// Intentionally opt-in: callers must not put this text in logs.
    pub fn expose(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
    pub fn write_new(&self, path: &Path) -> Result<()> {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(path)
            .context("choose a new token file outside the project")?;
        file.write_all(self.expose().as_bytes())?;
        file.sync_all()?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReloadReport {
    pub revision: String,
    pub accepted: bool,
    pub message: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Pull {
    pub version: u32,
    pub revision: Option<String>,
    pub report: Option<ReloadReport>,
    pub logs: Vec<String>,
    pub telemetry: Option<kairo_core::profiler::Telemetry>,
    pub inspection_unchanged: bool,
    pub results: Vec<ControlResult>,
    pub latency_ms: Option<f64>,
}
impl Pull {
    pub fn new(revision: Option<String>, report: Option<ReloadReport>, logs: Vec<String>) -> Self {
        Self {
            version: VERSION,
            revision,
            report,
            logs,
            telemetry: None,
            inspection_unchanged: false,
            results: Vec::new(),
            latency_ms: None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == VERSION,
            "incompatible Link protocol: Kairo 3.4 requires protocol 2"
        );
        if let Some(sample) = &self.telemetry {
            sample.validate()?;
        }
        ensure!(
            self.latency_ms
                .is_none_or(|v| v.is_finite() && (0.0..=120000.0).contains(&v)),
            "invalid latency"
        );
        ensure!(
            self.results.len() <= 16 && self.results.iter().all(|r| r.message.len() <= 1024),
            "debug result limit exceeded"
        );
        ensure!(
            serde_json::to_vec(self)?.len() <= FRAME_LIMIT - 16,
            "Link status message exceeds limit"
        );
        ensure!(
            self.revision
                .as_ref()
                .is_none_or(|r| r.len() == 64 && r.bytes().all(|b| b.is_ascii_hexdigit())),
            "invalid revision"
        );
        ensure!(
            self.logs.len() <= 16 && self.logs.iter().all(|l| l.len() <= 2048),
            "remote log limit exceeded"
        );
        if let Some(report) = &self.report {
            ensure!(
                report.revision.len() == 64
                    && report.revision.bytes().all(|b| b.is_ascii_hexdigit())
                    && report.message.len() <= 8192,
                "invalid reload report"
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlEnvelope {
    pub id: u64,
    pub command: kairo_core::profiler::DebugCommand,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlResult {
    pub id: u64,
    pub accepted: bool,
    pub message: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Exchange {
    pub version: u32,
    pub commands: Vec<ControlEnvelope>,
}
impl Exchange {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == VERSION && self.commands.len() <= 8,
            "invalid Link control exchange/version"
        );
        for command in &self.commands {
            command.command.validate()?;
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Message {
    Unchanged {
        version: u32,
    },
    Begin {
        version: u32,
        revision: String,
        files: usize,
        bytes: usize,
    },
    File {
        path: String,
        bytes: usize,
    },
    Chunk {
        data: Vec<u8>,
    },
    End,
}

pub(crate) struct Channel {
    stream: TcpStream,
    transport: snow::TransportState,
}
impl Channel {
    pub fn handshake(mut stream: TcpStream, key: &SessionKey, initiator: bool) -> Result<Self> {
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        stream.set_nodelay(true)?;
        stream.write_all(b"KLINK002")?;
        let mut protocol = [0_u8; 8];
        stream.read_exact(&mut protocol)?;
        ensure!(
            &protocol == b"KLINK002",
            "incompatible Kairo Link version; both peers must use Kairo 3.4 / protocol 2"
        );
        let builder = snow::Builder::new("Noise_NNpsk0_25519_ChaChaPoly_BLAKE2s".parse()?)
            .psk(0, &key.0)?
            .prologue(PROLOGUE)?;
        let mut noise = if initiator {
            builder.build_initiator()?
        } else {
            builder.build_responder()?
        };
        let mut plain = vec![0; FRAME_LIMIT];
        let mut encoded = vec![0; FRAME_LIMIT];
        if initiator {
            let count = noise.write_message(&[], &mut encoded)?;
            write_frame(&mut stream, &encoded[..count])?;
            noise.read_message(&read_frame(&mut stream)?, &mut plain)?;
        } else {
            noise.read_message(&read_frame(&mut stream)?, &mut plain)?;
            let count = noise.write_message(&[], &mut encoded)?;
            write_frame(&mut stream, &encoded[..count])?;
        }
        ensure!(
            noise.is_handshake_finished(),
            "incomplete authenticated handshake"
        );
        Ok(Self {
            stream,
            transport: noise.into_transport_mode()?,
        })
    }
    pub fn send<T: Serialize>(&mut self, value: &T) -> Result<()> {
        let plaintext = serde_json::to_vec(value)?;
        ensure!(
            plaintext.len() <= FRAME_LIMIT - 16,
            "Link message exceeds limit"
        );
        let mut encoded = vec![0; plaintext.len() + 16];
        let count = self.transport.write_message(&plaintext, &mut encoded)?;
        write_frame(&mut self.stream, &encoded[..count])
    }
    pub fn receive<T: DeserializeOwned>(&mut self) -> Result<T> {
        let encoded = read_frame(&mut self.stream)?;
        let mut plaintext = vec![0; encoded.len()];
        let count = self.transport.read_message(&encoded, &mut plaintext)?;
        serde_json::from_slice(&plaintext[..count]).context("malformed Link message")
    }
    pub fn send_bundle(&mut self, bundle: Option<&Bundle>) -> Result<()> {
        let Some(bundle) = bundle else {
            return self.send(&Message::Unchanged { version: VERSION });
        };
        let started = Instant::now();
        self.send(&Message::Begin {
            version: VERSION,
            revision: bundle.revision.clone(),
            files: bundle.files.len(),
            bytes: bundle.bytes(),
        })?;
        for file in &bundle.files {
            ensure!(
                started.elapsed() < Duration::from_secs(60),
                "Link transfer timed out"
            );
            self.send(&Message::File {
                path: file.path.clone(),
                bytes: file.bytes.len(),
            })?;
            for chunk in file.bytes.chunks(CHUNK) {
                ensure!(
                    started.elapsed() < Duration::from_secs(60),
                    "Link transfer timed out"
                );
                self.send(&Message::Chunk {
                    data: chunk.to_vec(),
                })?;
            }
        }
        self.send(&Message::End)
    }
    pub fn receive_bundle(&mut self) -> Result<Option<Bundle>> {
        let (revision, files, total) = match self.receive::<Message>()? {
            Message::Unchanged { version } => {
                ensure!(version == VERSION, "unsupported Link version");
                return Ok(None);
            }
            Message::Begin {
                version,
                revision,
                files,
                bytes,
            } => {
                ensure!(
                    version == VERSION
                        && revision.len() == 64
                        && revision.bytes().all(|b| b.is_ascii_hexdigit())
                        && files > 0
                        && files <= MAX_FILES
                        && bytes <= MAX_BYTES,
                    "invalid Link bundle header"
                );
                (revision, files, bytes)
            }
            _ => anyhow::bail!("expected Link bundle header"),
        };
        let started = Instant::now();
        let mut entries = Vec::with_capacity(files);
        let mut received = 0_usize;
        for _ in 0..files {
            ensure!(
                started.elapsed() < Duration::from_secs(60),
                "Link transfer timed out"
            );
            let Message::File { path, bytes } = self.receive::<Message>()? else {
                anyhow::bail!("expected Link file");
            };
            validate_path(&path)?;
            received = received.checked_add(bytes).context("Link size overflow")?;
            ensure!(
                bytes <= MAX_FILE_BYTES && received <= total,
                "Link file exceeds transfer budget"
            );
            let mut content = Vec::with_capacity(bytes);
            while content.len() < bytes {
                ensure!(
                    started.elapsed() < Duration::from_secs(60),
                    "Link transfer timed out"
                );
                let Message::Chunk { data } = self.receive::<Message>()? else {
                    anyhow::bail!("expected Link data chunk");
                };
                ensure!(
                    !data.is_empty() && data.len() <= CHUNK && content.len() + data.len() <= bytes,
                    "invalid Link data chunk"
                );
                content.extend_from_slice(&data);
            }
            entries.push(SourceFile {
                path,
                bytes: content,
            });
        }
        ensure!(
            started.elapsed() < Duration::from_secs(60),
            "Link transfer timed out"
        );
        ensure!(
            received == total && matches!(self.receive::<Message>()?, Message::End),
            "incomplete Link bundle"
        );
        let bundle = Bundle::from_files(entries)?;
        ensure!(bundle.revision == revision, "Link bundle digest mismatch");
        Ok(Some(bundle))
    }
}
fn write_frame(stream: &mut impl Write, bytes: &[u8]) -> Result<()> {
    ensure!(
        !bytes.is_empty() && bytes.len() <= FRAME_LIMIT,
        "invalid frame size"
    );
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(bytes)?;
    Ok(())
}
fn read_frame(stream: &mut impl Read) -> Result<Vec<u8>> {
    let mut header = [0; 4];
    stream.read_exact(&mut header)?;
    let length = u32::from_be_bytes(header) as usize;
    ensure!((1..=FRAME_LIMIT).contains(&length), "invalid frame size");
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn token_roundtrip_and_frame_limits() {
        let key = SessionKey::generate().unwrap();
        assert_eq!(
            SessionKey::parse(&key.expose()).unwrap().expose(),
            key.expose()
        );
        assert!(SessionKey::parse("short-password").is_err());
        assert!(read_frame(&mut std::io::Cursor::new(u32::MAX.to_be_bytes())).is_err());
        assert!(Pull::new(None, None, vec!["x".repeat(2049)])
            .validate()
            .is_err());
    }
    #[test]
    fn unknown_protocol_and_non_ascii_revisions_are_rejected() {
        let mut pull = Pull::new(None, None, Vec::new());
        pull.version = VERSION + 1;
        assert!(pull.validate().is_err());
        let revision = "\u{e9}".repeat(32);
        assert_eq!(revision.len(), 64);
        assert!(Pull::new(Some(revision.clone()), None, Vec::new())
            .validate()
            .is_err());
        assert!(Pull::new(
            None,
            Some(ReloadReport {
                revision,
                accepted: false,
                message: String::new()
            }),
            Vec::new()
        )
        .validate()
        .is_err());
    }

    #[test]
    fn encrypted_loopback_transfers_a_verified_bundle() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let key = SessionKey::generate().unwrap();
        let server_key = key.clone();
        let server = std::thread::spawn(move || {
            let mut channel =
                Channel::handshake(listener.accept().unwrap().0, &server_key, false).unwrap();
            channel.receive::<Pull>().unwrap().validate().unwrap();
            let bundle = Bundle::from_files(vec![SourceFile {
                path: "main.lua".into(),
                bytes: vec![b' '; 20_000],
            }])
            .unwrap();
            channel.send_bundle(Some(&bundle)).unwrap();
        });
        let mut channel =
            Channel::handshake(TcpStream::connect(address).unwrap(), &key, true).unwrap();
        channel.send(&Pull::new(None, None, Vec::new())).unwrap();
        assert_eq!(channel.receive_bundle().unwrap().unwrap().bytes(), 20_000);
        server.join().unwrap();
    }
    #[test]
    fn wrong_shared_key_is_rejected() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            Channel::handshake(
                listener.accept().unwrap().0,
                &SessionKey::generate().unwrap(),
                false,
            )
            .is_err()
        });
        assert!(Channel::handshake(
            TcpStream::connect(address).unwrap(),
            &SessionKey::generate().unwrap(),
            true
        )
        .is_err());
        assert!(server.join().unwrap());
    }
}
