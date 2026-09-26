use crate::app::Action;
use eframe::egui;
use kairo_core::Config;
use kairo_project::Template;

pub(crate) struct FeatureStatus {
    pub busy: bool,
    pub running: bool,
    pub recording: bool,
    pub paused: bool,
    pub link: String,
    pub runtime_ready: bool,
}

struct FeatureCard {
    title: &'static str,
    status: String,
    description: &'static str,
    primary: (&'static str, Action),
    secondary: Option<(&'static str, Action)>,
}

pub(crate) fn overview(
    ui: &mut egui::Ui,
    config: &Config,
    state: &FeatureStatus,
) -> Option<Action> {
    let mut action = None;
    egui::ScrollArea::vertical().id_source("feature-overview").show(ui, |ui| {
        ui.heading(&config.game.title);
        ui.label("Build, play and experiment. The feature bar stays visible while you edit.");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Open main.lua").clicked() {
                action = Some(Action::OpenFile("main.lua".into()));
            }
            if ui.add_enabled(!state.busy && state.runtime_ready, egui::Button::new("Run game  F5")).clicked() {
                action = Some(Action::Run);
            }
            if ui.button("Project settings").clicked() {
                action = Some(Action::Settings);
            }
        });
        if !state.runtime_ready {
            ui.group(|ui| {
                ui.label("The runtime has not been found. Editing and settings still work.");
                if ui.button("Locate runtime in Preferences").clicked() {
                    action = Some(Action::Preferences);
                }
                ui.small("Build both kairo-cli and kairo-editor; the runtime is separate from the editor.");
            });
        }
        ui.add_space(12.0);
        let cards = [
            FeatureCard {
                title: "Fantasy Console",
                status: if config.micro.enabled { format!("ON in project | {} x {}", config.micro.width, config.micro.height) } else { "OFF in project".into() },
                description: "Choose a low-resolution canvas and palette, then run your game with pixel snapping and integer scaling.",
                primary: ("Configure Fantasy Console", Action::Micro),
                secondary: Some(("New fantasy project", Action::NewProject(Template::Micro))),
            },
            FeatureCard {
                title: "Kairo Link",
                status: state.link.clone(),
                description: "Host your saved project or connect as a tester. Lua/assets update through an opt-in, authenticated development session.",
                primary: ("Open Link", Action::Link),
                secondary: Some(("New Link demo", Action::NewProject(Template::Link))),
            },
            FeatureCard {
                title: "Kairo Replay",
                status: if state.paused { "PAUSED".into() } else if state.recording { "RECORDING".into() } else if config.replay.enabled { "Auto-record enabled in project".into() } else { "Recording is off".into() },
                description: "Save snapshots of registered game state, pause, and resume from an earlier point to try another path.",
                primary: ("Open Replay", Action::Replay),
                secondary: Some(("New Replay demo", Action::NewProject(Template::Replay))),
            },
            FeatureCard {
                title: "Profiler",
                status: if state.running { "Local runtime active".into() } else { "Run a game to collect samples".into() },
                description: "Inspect FPS, frame time, Lua callbacks, physics, batching and texture memory. Timings are CPU measurements.",
                primary: ("Open Profiler", Action::Profiler),
                secondary: None,
            },
            FeatureCard {
                title: "Sprites & Animation",
                status: "Built-in asset tools".into(),
                description: "Draw pixels, fill, select, copy/paste and save PNGs. Slice a sprite sheet and preview animation timing.",
                primary: ("New sprite", Action::NewSprite),
                secondary: Some(("Open animation tool", Action::NewAnimation)),
            },
            FeatureCard {
                title: "Lua UI",
                status: "Game UI and debug controls".into(),
                description: "Build in-game panels and HUDs with Lua, and add sliders or checkboxes to the development UI.",
                primary: ("UI setup & snippets", Action::UiGuide),
                secondary: Some(("New UI demo", Action::NewProject(Template::Ui))),
            },
            FeatureCard {
                title: "Build & Export",
                status: "Host platform only".into(),
                description: "Package your game with a runtime built for this computer. Players do not need Rust or Cargo installed.",
                primary: ("Export game", Action::Export),
                secondary: None,
            },
        ];
        let columns = if ui.available_width() >= 650.0 { 2 } else { 1 };
        for row in cards.chunks(columns) {
            ui.columns(columns, |uis| {
                for (column, card) in uis.iter_mut().zip(row) {
                    column.group(|ui| {
                        ui.set_min_height(162.0);
                        ui.strong(card.title);
                        ui.small(&card.status);
                        ui.add_space(4.0);
                        ui.label(card.description);
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            if ui.button(card.primary.0).clicked() {
                                action = Some(card.primary.1.clone());
                            }
                            if let Some((label, value)) = &card.secondary {
                                if ui.button(*label).clicked() {
                                    action = Some(value.clone());
                                }
                            }
                        });
                    });
                }
            });
            ui.add_space(8.0);
        }
        ui.small("Choose a demo to open the new-project screen. Templates are written to a new folder.");
    });
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overview_renders_without_a_runtime_or_gpu() {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 900.0),
            )),
            ..Default::default()
        };
        let state = FeatureStatus {
            busy: false,
            running: false,
            recording: false,
            paused: false,
            link: "OFF".into(),
            runtime_ready: false,
        };
        let output = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                assert!(overview(ui, &Config::default(), &state).is_none());
            });
        });
        assert!(!output.shapes.is_empty());
    }
}
