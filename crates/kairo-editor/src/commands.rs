use crate::app::Action;
use crate::develop::ToolTab;
use eframe::egui;

#[derive(Default)]
pub(crate) struct CommandPalette {
    pub open: bool,
    query: String,
    focus: bool,
}
impl CommandPalette {
    pub fn show(&mut self) {
        self.open = true;
        self.focus = true;
        self.query.clear();
    }
    pub fn ui(&mut self, ctx: &egui::Context) -> Option<Action> {
        if !self.open {
            return None;
        }
        let mut open = true;
        let mut action = None;
        egui::Window::new("Command palette")
            .open(&mut open)
            .collapsible(false)
            .default_width(560.0)
            .show(ctx, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Type a command...")
                        .desired_width(f32::INFINITY),
                );
                if self.focus {
                    response.request_focus();
                    self.focus = false;
                }
                let commands = [
                    ("Run Project", Action::Run),
                    ("Stop Project", Action::Stop),
                    ("Restart Project", Action::Restart),
                    ("Build / Export Project", Action::Export),
                    ("Open Project", Action::OpenDialog),
                    ("Save All", Action::SaveAll),
                    ("Open Fantasy Console", Action::Micro),
                    ("Open Kairo Link", Action::Link),
                    ("Open Replay", Action::Replay),
                    ("Open Profiler", Action::Profiler),
                    (
                        "Live Inspector / Physics Debug",
                        Action::Tool(ToolTab::Inspector),
                    ),
                    ("Input Mappings", Action::Tool(ToolTab::Input)),
                    ("Audio Mixer", Action::Tool(ToolTab::Mixer)),
                    ("Particle Editor", Action::Tool(ToolTab::Particles)),
                    ("Project-wide Search", Action::Tool(ToolTab::Search)),
                    ("Offline Lua API / Docs", Action::Api),
                    ("New Sprite", Action::NewSprite),
                    ("Animate selected PNG", Action::NewAnimation),
                    ("Project Settings", Action::Settings),
                    ("Runtime Preferences", Action::Preferences),
                ];
                let mut results: Vec<_> = commands
                    .into_iter()
                    .filter_map(|(label, action)| {
                        kairo_project::search::fuzzy_score(&self.query, label)
                            .map(|score| (score, label, action))
                    })
                    .collect();
                results.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .show(ui, |ui| {
                        for (index, (_, label, value)) in results.into_iter().enumerate() {
                            if ui.selectable_label(index == 0, label).clicked()
                                || (index == 0 && enter)
                            {
                                action = Some(value);
                            }
                        }
                    });
            });
        self.open = open && action.is_none();
        action
    }
}
