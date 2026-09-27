use serde::{Deserialize, Serialize};
pub const BUSES: &[&str] = &["master", "music", "sfx", "ui"];
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BusStatus {
    pub name: String,
    pub volume: f32,
    pub muted: bool,
    pub voices: usize,
}
impl BusStatus {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            volume: 1.0,
            muted: false,
            voices: 0,
        }
    }
    pub fn gain(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.volume
        }
    }
}
