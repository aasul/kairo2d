use std::sync::{
    mpsc::{self, Receiver, SyncSender},
    OnceLock,
};
static SENDER: OnceLock<SyncSender<String>> = OnceLock::new();
pub fn listen() -> Receiver<String> {
    let (sender, receiver) = mpsc::sync_channel(256);
    let _ = SENDER.set(sender);
    receiver
}
pub fn capture(level: log::Level, message: &str) {
    if let Some(sender) = SENDER.get() {
        let mut end = message.len().min(1800);
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        let _ = sender.try_send(format!("[{level}] {}", &message[..end]));
    }
}
