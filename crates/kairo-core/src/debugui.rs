use anyhow::{ensure, Context, Result};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum Widget {
    Text(String),
    Button {
        label: String,
        key: String,
    },
    Checkbox {
        label: String,
        key: String,
        value: bool,
    },
    Slider {
        label: String,
        key: String,
        value: f64,
        min: f64,
        max: f64,
    },
}
#[derive(Clone, Copy, Debug)]
pub enum Response {
    Click,
    Bool(bool),
    Number(f64),
}
#[derive(Default)]
pub struct DebugWindow {
    pub title: String,
    pub widgets: Vec<Widget>,
}
#[derive(Default)]
pub struct DebugUi {
    pub windows: Vec<DebugWindow>,
    pub responses: HashMap<String, Response>,
    pub wants_mouse: bool,
    pub building: bool,
    pub current: Option<usize>,
}
impl DebugUi {
    pub fn key(&self, label: &str) -> Result<String> {
        ensure!(self.building, "debugui widgets belong in game.debugUI");
        ensure!(label.len() <= 512, "debugui label exceeds 512 bytes");
        let window = self
            .current
            .and_then(|i| self.windows.get(i))
            .context("debugui widget must be inside debugui.window")?;
        Ok(format!("{}\0{}", window.title, label))
    }
    pub fn push(&mut self, widget: Widget) -> Result<()> {
        let window = self
            .current
            .and_then(|i| self.windows.get_mut(i))
            .context("debugui widget needs a window")?;
        ensure!(
            window.widgets.len() < 128,
            "debugui window exceeds 128 widgets"
        );
        window.widgets.push(widget);
        Ok(())
    }
}
