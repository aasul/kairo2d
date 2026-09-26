use crate::document::Document;
use crate::highlight;
use eframe::egui::{
    self,
    text::{CCursor, CCursorRange},
    Align, Color32, FontId, Id, Key, Modifiers, Pos2, Rect, Vec2,
};

#[derive(Default)]
pub struct CodeTools {
    pub find_open: bool,
    pub replace_open: bool,
    pub goto_open: bool,
    query: String,
    replacement: String,
    line: usize,
    status: String,
}

impl CodeTools {
    pub fn ui(&mut self, ui: &mut egui::Ui, document: &mut Document, font_size: f32) {
        let id = Id::new(("code", &document.path));
        let focused = ui.memory(|memory| memory.has_focus(id));
        if focused {
            ui.input_mut(|input| {
                if input.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)
                    || input.consume_key(Modifiers::COMMAND, Key::Y)
                {
                    document.redo();
                } else if input.consume_key(Modifiers::COMMAND, Key::Z) {
                    document.undo();
                }
                if input.consume_key(Modifiers::COMMAND, Key::Slash) {
                    document.toggle_comment();
                }
                if input.consume_key(Modifiers::COMMAND, Key::D) {
                    document.duplicate_lines();
                }
                if input.consume_key(Modifiers::NONE, Key::Enter) {
                    document.insert_newline();
                }
                if input.consume_key(Modifiers::SHIFT, Key::Tab) {
                    document.indent(true);
                } else if input.consume_key(Modifiers::NONE, Key::Tab) {
                    document.indent(false);
                }
            });
        }
        ui.horizontal(|ui| {
            ui.menu_button("Functions", |ui| {
                let symbols: Vec<_> = document
                    .text
                    .lines()
                    .enumerate()
                    .filter(|(_, line)| {
                        let text = line.trim();
                        text.starts_with("function ")
                            || text.starts_with("local function ")
                            || text.contains("= function(")
                    })
                    .take(256)
                    .map(|(index, line)| {
                        (index + 1, line.trim().chars().take(120).collect::<String>())
                    })
                    .collect();
                if symbols.is_empty() {
                    ui.label("No line-based function declarations found.");
                }
                for (line, name) in symbols {
                    if ui.button(format!("{line}: {name}")).clicked() {
                        document.goto_line(line);
                        ui.close_menu();
                    }
                }
            });
            ui.small("Ctrl/Cmd+/ comment  |  Ctrl/Cmd+D duplicate line");
        });
        if self.find_open {
            ui.horizontal(|ui| {
                ui.label("Find");
                ui.add(egui::TextEdit::singleline(&mut self.query).desired_width(180.0));
                if ui.button("Next").clicked() {
                    self.status = if document.find_next(&self.query) {
                        String::new()
                    } else {
                        "No match".into()
                    };
                }
                ui.checkbox(&mut self.replace_open, "Replace");
                if ui.button("Close").clicked() {
                    self.find_open = false;
                }
                ui.small(&self.status);
            });
            if self.replace_open {
                ui.horizontal(|ui| {
                    ui.label("With");
                    ui.add(egui::TextEdit::singleline(&mut self.replacement).desired_width(180.0));
                    if ui.button("Replace selected").clicked() {
                        let start =
                            crate::document::byte_at_char(&document.text, document.selection.start);
                        let end =
                            crate::document::byte_at_char(&document.text, document.selection.end);
                        if document.text[start..end] == self.query {
                            document.replace_selection(&self.replacement);
                        }
                    }
                    if ui.button("Replace all").clicked() {
                        self.status = format!(
                            "{} replacements",
                            document.replace_all(&self.query, &self.replacement)
                        );
                    }
                });
            }
            ui.separator();
        }
        if self.goto_open {
            ui.horizontal(|ui| {
                ui.label("Line");
                ui.add(
                    egui::DragValue::new(&mut self.line)
                        .range(1..=document.text.lines().count().max(1)),
                );
                if ui.button("Go").clicked() {
                    document.goto_line(self.line.max(1));
                    self.goto_open = false;
                }
            });
        }
        let pending = document.pending_selection.take();
        if let Some(range) = &pending {
            let mut state = egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
            state.cursor.set_char_range(Some(CCursorRange::two(
                CCursor::new(range.start),
                CCursor::new(range.end),
            )));
            state.store(ui.ctx(), id);
            ui.memory_mut(|memory| memory.request_focus(id));
        }
        let before = document.text.clone();
        let previous_selection = document.selection.clone();
        let lua = document
            .path
            .extension()
            .is_some_and(|extension| extension == "lua");
        let row_height = ui.fonts(|fonts| fonts.row_height(&FontId::monospace(font_size)));
        let count = document.text.bytes().filter(|b| *b == b'\n').count() + 1;
        let current_line = document.line_column().0;
        egui::ScrollArea::both()
            .id_source(("code-scroll", &document.path))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    let (gutter, _) = ui.allocate_exact_size(
                        Vec2::new(46.0, count as f32 * row_height + 8.0),
                        egui::Sense::hover(),
                    );
                    let first = ((ui.clip_rect().top() - gutter.top()) / row_height)
                        .floor()
                        .max(0.0) as usize;
                    let last = ((ui.clip_rect().bottom() - gutter.top()) / row_height)
                        .ceil()
                        .max(0.0) as usize;
                    for row in first..=last.min(count.saturating_sub(1)) {
                        let color = if row + 1 == current_line {
                            Color32::from_rgb(152, 195, 231)
                        } else {
                            Color32::from_rgb(99, 112, 130)
                        };
                        ui.painter().text(
                            Pos2::new(
                                gutter.right() - 6.0,
                                gutter.top() + 4.0 + row as f32 * row_height,
                            ),
                            egui::Align2::RIGHT_TOP,
                            (row + 1).to_string(),
                            FontId::monospace(font_size),
                            color,
                        );
                    }
                    let mut layouter = |ui: &egui::Ui, text: &str, _wrap_width: f32| {
                        ui.fonts(|fonts| fonts.layout_job(highlight::layout(text, font_size, lua)))
                    };
                    let output = egui::TextEdit::multiline(&mut document.text)
                        .id(id)
                        .code_editor()
                        .font(FontId::monospace(font_size))
                        .frame(false)
                        .desired_rows(28)
                        .desired_width(640.0)
                        .char_limit(2 * 1024 * 1024)
                        .layouter(&mut layouter)
                        .show(ui);
                    if let Some(cursor) = output.cursor_range {
                        let a = cursor.primary.ccursor.index;
                        let b = cursor.secondary.ccursor.index;
                        document.selection = a.min(b)..a.max(b);
                    }
                    if pending.is_some() {
                        let line = document.line_column().0;
                        let top = output.response.rect.top() + (line - 1) as f32 * row_height;
                        ui.scroll_to_rect(
                            Rect::from_min_size(
                                Pos2::new(output.response.rect.left(), top),
                                Vec2::new(10.0, row_height),
                            ),
                            Some(Align::Center),
                        );
                    }
                });
            });
        document.record_edit(before, previous_selection, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kairo_project::ProjectFiles;

    #[test]
    fn text_input_reaches_the_real_editor_widget_and_saves() {
        let root = tempfile::tempdir().unwrap();
        let files = ProjectFiles::open(root.path()).unwrap();
        files.create_file("main.lua", b"").unwrap();
        let mut document = Document::load(&files, "main.lua".into()).unwrap();
        document.pending_selection = Some(0..0);
        let mut tools = CodeTools::default();
        let ctx = egui::Context::default();
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 600.0))),
            ..Default::default()
        };
        let _ = ctx.run(raw.clone(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| tools.ui(ui, &mut document, 15.0));
        });
        let mut input = raw;
        input
            .events
            .push(egui::Event::Text("local score = 1".into()));
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| tools.ui(ui, &mut document, 15.0));
        });
        assert_eq!(document.text, "local score = 1");
        assert!(document.dirty());
        document.save(&files).unwrap();
        assert_eq!(files.read("main.lua").unwrap(), b"local score = 1");
        document.undo();
        assert_eq!(document.text, "");
    }
}
