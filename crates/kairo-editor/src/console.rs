use eframe::egui::{self, Color32, RichText};
use std::collections::VecDeque;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub path: PathBuf,
    pub line: usize,
}

struct Entry {
    message: String,
    severity: Severity,
    location: Option<Location>,
}

pub struct Console {
    entries: VecDeque<Entry>,
    info: bool,
    warnings: bool,
    errors: bool,
    filter: String,
    last_severity: Severity,
}

impl Default for Console {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            info: true,
            warnings: true,
            errors: true,
            filter: String::new(),
            last_severity: Severity::Info,
        }
    }
}

impl Console {
    pub fn push(&mut self, message: impl Into<String>) {
        let message = message.into();
        for line in message.lines() {
            let severity = if line.contains("[ERROR]") {
                Severity::Error
            } else if line.contains("[WARN]") {
                Severity::Warning
            } else if line.contains("[INFO]") || line.trim().is_empty() {
                Severity::Info
            } else {
                self.last_severity
            };
            self.last_severity = severity;
            self.entries.push_back(Entry {
                message: line.to_owned(),
                severity,
                location: parse_location(line),
            });
        }
        while self.entries.len() > 2000 {
            self.entries.pop_front();
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<Location> {
        ui.horizontal(|ui| {
            ui.strong("OUTPUT");
            ui.checkbox(&mut self.info, "Info");
            ui.checkbox(&mut self.warnings, "Warnings");
            ui.checkbox(&mut self.errors, "Errors");
            if ui.button("Clear").clicked() {
                self.entries.clear();
            }
            ui.add(
                egui::TextEdit::singleline(&mut self.filter)
                    .hint_text("Filter output")
                    .desired_width(180.0),
            );
        });
        ui.separator();
        let mut open = None;
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .id_source("console-lines")
            .show(ui, |ui| {
                for entry in &self.entries {
                    let shown = match entry.severity {
                        Severity::Info => self.info,
                        Severity::Warning => self.warnings,
                        Severity::Error => self.errors,
                    };
                    if !shown
                        || (!self.filter.is_empty()
                            && !entry
                                .message
                                .to_lowercase()
                                .contains(&self.filter.to_lowercase()))
                    {
                        continue;
                    }
                    let color = match entry.severity {
                        Severity::Info => Color32::from_rgb(184, 193, 207),
                        Severity::Warning => Color32::from_rgb(222, 182, 116),
                        Severity::Error => Color32::from_rgb(233, 132, 140),
                    };
                    let text = RichText::new(&entry.message)
                        .monospace()
                        .size(12.5)
                        .color(color);
                    if entry.location.is_some() {
                        if ui
                            .add(egui::Label::new(text).sense(egui::Sense::click()))
                            .on_hover_text("Open source location")
                            .clicked()
                        {
                            open = entry.location.clone();
                        }
                    } else {
                        ui.label(text);
                    }
                }
            });
        open
    }
}

pub fn parse_location(message: &str) -> Option<Location> {
    for suffix in [".lua:", ".toml:"] {
        let Some(end) = message.find(suffix) else {
            continue;
        };
        let after = end + suffix.len();
        let digits: String = message[after..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if digits.is_empty() {
            continue;
        }
        let line = digits.parse::<usize>().ok()?;
        let before = &message[..end + suffix.len() - 1];
        let start = before
            .rfind(|c: char| c.is_whitespace() || matches!(c, '\'' | '"' | '@' | '[' | '('))
            .map_or(0, |i| i + 1);
        let path = before[start..].replace('\\', "/");
        if line > 0 && !path.is_empty() {
            return Some(Location {
                path: path.into(),
                line,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_traceback_locations_are_clickable() {
        assert_eq!(
            parse_location("[ERROR] scripts/player.lua:18: unexpected symbol").unwrap(),
            Location {
                path: "scripts/player.lua".into(),
                line: 18
            }
        );
        assert_eq!(
            parse_location("\tmain.lua:9: in function 'game.draw'")
                .unwrap()
                .line,
            9
        );
        assert!(parse_location("[INFO] loaded project").is_none());
    }
}
