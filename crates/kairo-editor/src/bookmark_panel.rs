use eframe::egui;
use kairo_core::profiler::{DebugCommand, Telemetry};
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct BookmarkPanel {
    label: String,
    note: String,
    identity: (u64, u64),
    drafts: BTreeMap<u64, (String, String)>,
    message: String,
}
impl BookmarkPanel {
    pub fn ui(&mut self, ui: &mut egui::Ui, peer: u64, sample: &Telemetry) -> Option<DebugCommand> {
        let session = sample.inspection.session;
        if self.identity != (peer, session) {
            self.identity = (peer, session);
            self.drafts.clear();
        }
        let mut command = None;
        ui.strong("Bug bookmarks  |  F11 in a running game");
        ui.small("Pins registered Lua state, input actions, RNG and supported physics state in this runtime. Eight bookmarks / 16 MiB; oldest pins are evicted. Reloading the VM removes them.");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.label)
                    .hint_text("Bookmark label")
                    .char_limit(96),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.note)
                    .hint_text("Note")
                    .char_limit(512),
            );
            if ui.button("Capture").clicked() {
                command = Some(DebugCommand::Bookmark {
                    label: if self.label.is_empty() {
                        "Bug bookmark".into()
                    } else {
                        self.label.clone()
                    },
                    note: self.note.clone(),
                });
            }
        });
        self.drafts
            .retain(|id, _| sample.bookmarks.iter().any(|b| b.id == *id));
        for bookmark in &sample.bookmarks {
            egui::CollapsingHeader::new(format!("{:.2}s - {}",bookmark.time,bookmark.label)).id_source(("bug",bookmark.id)).show(ui,|ui| {
                ui.small(format!("Frame {} | Scene {} | {:.2} MiB",bookmark.frame,bookmark.scene,bookmark.bytes as f64/1_048_576.0));
                let (label,note)=self.drafts.entry(bookmark.id).or_insert_with(||(bookmark.label.clone(),bookmark.note.clone()));
                ui.add(egui::TextEdit::singleline(label).char_limit(96));
                ui.add(egui::TextEdit::multiline(note).desired_rows(2).char_limit(512));
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Apply label/note").clicked(){command=Some(DebugCommand::BookmarkEdit{session,id:bookmark.id,label:label.clone(),note:note.clone()});}
                    if ui.button("Restore snapshot").clicked(){command=Some(DebugCommand::BookmarkJump{session,id:bookmark.id});}
                    if ui.button("Delete").clicked(){command=Some(DebugCommand::BookmarkDelete{session,id:bookmark.id});}
                    if ui.button("Export metadata JSON...").clicked(){
                        if let Some(path)=rfd::FileDialog::new().set_file_name("kairo-bug.json").add_filter("JSON",&["json"]).save_file(){
                            let result=(||->anyhow::Result<()>{
                                let value=serde_json::json!({"format":1,"kairo":kairo_core::VERSION,"runtime_session":session,"peer":peer,"snapshot_included":false,"bookmark":bookmark});
                                kairo_project::atomic_write(&path,&serde_json::to_vec_pretty(&value)?)?;Ok(())
                            })();
                            self.message=match result{Ok(())=>"Exported metadata only; the live snapshot remains in the runtime.".into(),Err(e)=>format!("{e:#}")};
                        }
                    }
                });
            });
        }
        ui.small("Restore requires the same scene stack and compatible physics bodies. Restoring a pin replaces the rolling timeline. No screenshot, arbitrary Lua stack or external I/O is captured.");
        if !self.message.is_empty() {
            ui.label(&self.message);
        }
        command
    }
}
