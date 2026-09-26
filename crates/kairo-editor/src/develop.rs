use crate::app::Action;
use crate::inspector::{mixer_ui, InspectorPanel};
use crate::mappings::MappingEditor;
use crate::particle_tool::ParticleTool;
use crate::workspace::Workspace;
use anyhow::Result;
use eframe::egui;
use kairo_core::profiler::Telemetry;
use std::sync::mpsc::{self, Receiver};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ToolTab {
    #[default]
    Inspector,
    Input,
    Particles,
    Mixer,
    Search,
}
impl ToolTab {
    pub const ALL: [Self; 5] = [
        Self::Inspector,
        Self::Input,
        Self::Particles,
        Self::Mixer,
        Self::Search,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Inspector => "Live Inspector",
            Self::Input => "Input Mappings",
            Self::Particles => "Particles",
            Self::Mixer => "Audio Mixer",
            Self::Search => "Project Search",
        }
    }
}
#[derive(Default)]
pub(crate) struct DevelopTools {
    pub open: bool,
    pub tab: ToolTab,
    pub target: u64,
    inspector: InspectorPanel,
    mapping: MappingEditor,
    particles: Option<ParticleTool>,
    query: String,
    case_sensitive: bool,
    search: Option<Receiver<Result<kairo_project::search::SearchReport>>>,
    results: kairo_project::search::SearchReport,
    message: String,
}
impl DevelopTools {
    pub fn dirty(&self) -> bool {
        self.mapping.dirty() || self.particles.as_ref().is_some_and(ParticleTool::dirty)
    }
    pub fn save(&mut self, workspace: &mut Workspace) -> Result<()> {
        if self.mapping.dirty() {
            self.mapping.save(workspace)?;
        }
        if let Some(tool) = &mut self.particles {
            if tool.dirty() {
                tool.save(workspace)?;
            }
        }
        Ok(())
    }
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        workspace: &mut Workspace,
        local: Option<&Telemetry>,
        peers: &[kairo_link::PeerStatus],
    ) -> Option<Action> {
        if !self.open {
            return None;
        }
        let mut open = true;
        let mut action = None;
        egui::Window::new("Development tools").id(egui::Id::new("develop-tools")).open(&mut open)
            .default_size([680.0,600.0]).vscroll(true).show(ctx,|ui| {
            ui.horizontal_wrapped(|ui| {for tab in ToolTab::ALL {ui.selectable_value(&mut self.tab,tab,tab.name());}});
            ui.separator();
            if matches!(self.tab,ToolTab::Inspector|ToolTab::Mixer) {
                egui::ComboBox::from_id_source("runtime-target").selected_text(if self.target==0{"Local game".into()}else{format!("Tester {}",self.target)}).show_ui(ui,|ui| {
                    ui.selectable_value(&mut self.target,0,"Local game");
                    for peer in peers {ui.selectable_value(&mut self.target,peer.id,format!("{} ({})",peer.label,peer.id));}
                });
                let sample=if self.target==0{local}else{peers.iter().find(|peer|peer.id==self.target).and_then(|p|p.telemetry.as_deref())};
                if let Some(sample)=sample {
                    let mut command=None;
                    if !sample.tools_enabled {ui.label("Runtime tools are disabled by this profile. Restart with Development or Debug to edit live state.");}
                    ui.add_enabled_ui(sample.tools_enabled,|ui| {
                        command=if self.tab==ToolTab::Inspector{self.inspector.ui(ui,self.target,sample)}else{mixer_ui(ui,sample)};
                    });
                    if let Some(command)=command {action=Some(Action::Control {peer:self.target,command:Box::new(command)});}
                    if let Some(peer)=peers.iter().find(|p|p.id==self.target){
                        ui.small(format!("Last response {:.1}s ago | {}",peer.last_seen.elapsed().as_secs_f32(),peer.last_command));
                        if let Some(ms)=peer.latency_ms{ui.small(format!("Control exchange round trip {ms:.1} ms (not raw network ping)"));}
                    }
                } else {
                    ui.label("No telemetry for this target. Run a local game or connect an authenticated Link tester.");
                    if ui.button("Run local project").clicked(){action=Some(Action::Run);}
                    if ui.button("Open Kairo Link").clicked(){action=Some(Action::Link);}
                }
            } else {
                let result=match self.tab {
                    ToolTab::Input=>self.mapping.ui(ui,workspace),
                    ToolTab::Particles=>{
                        if self.particles.is_none(){match ParticleTool::new(){Ok(tool)=>self.particles=Some(tool),Err(e)=>self.message=e.to_string()}}
                        self.particles.as_mut().map_or(Ok(()),|tool|tool.ui(ui,workspace))
                    }
                    ToolTab::Search=>{self.search_ui(ui,workspace,&mut action);Ok(())}
                    _=>Ok(()),
                };
                if let Err(error)=result{self.message=format!("{error:#}");}
            }
            if !self.message.is_empty(){ui.separator();ui.label(&self.message);}
            if self.dirty(){ui.small("Tool edits are unsaved. Save All includes input mappings and the particle preset.");}
        });
        // Hiding a tool does not discard its document. Project switching/quit use the shared unsaved prompt.
        self.open = open;
        action
    }
    fn search_ui(&mut self, ui: &mut egui::Ui, workspace: &Workspace, action: &mut Option<Action>) {
        if let Some(receiver) = &self.search {
            match receiver.try_recv() {
                Ok(Ok(report)) => {
                    self.message = format!(
                        "{} files searched{}",
                        report.scanned,
                        if report.limited {
                            " (limit reached)"
                        } else {
                            ""
                        }
                    );
                    self.results = report;
                    self.search = None;
                }
                Ok(Err(error)) => {
                    self.message = format!("Search failed: {error:#}");
                    self.search = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.message = "Search worker stopped".into();
                    self.search = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Literal text in project files")
                    .char_limit(256),
            );
            ui.checkbox(&mut self.case_sensitive, "Match case");
            if ui
                .add_enabled(
                    self.search.is_none() && !self.query.is_empty(),
                    egui::Button::new("Search"),
                )
                .clicked()
            {
                let files = workspace.files.clone();
                let query = self.query.clone();
                let case = self.case_sensitive;
                let (sender, receiver) = mpsc::sync_channel(1);
                self.search = Some(receiver);
                std::thread::spawn(move || {
                    let _ = sender.send(kairo_project::search::search(&files, &query, case));
                });
            }
        });
        ui.small("Saved UTF-8 source/config only. Ignores build outputs, .git and binary assets; bounded to 1000 hits / 64 MiB.");
        if self.search.is_some() {
            ui.spinner();
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
        for hit in &self.results.hits {
            if ui
                .selectable_label(
                    false,
                    format!("{}:{}   {}", hit.path.display(), hit.line, hit.preview),
                )
                .clicked()
            {
                *action = Some(Action::OpenLocation(hit.path.clone(), hit.line));
            }
        }
    }
}
