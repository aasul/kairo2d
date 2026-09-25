use eframe::egui::{self, Color32};
use kairo_core::config::MicroConfig;
use kairo_project::SettingsDocument;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsPage {
    General,
    Micro,
    Replay,
}

pub(crate) const PALETTES: [(&str, &str); 4] = [
    ("kairo16", "Kairo16 - 16 colors"),
    ("grayscale", "Grayscale - 4 colors"),
    ("olive4", "Olive - 4 colors"),
    ("none", "Unrestricted colors"),
];

pub(crate) const RESOLUTIONS: [(&str, u32, u32); 3] = [
    ("Widescreen", 320, 180),
    ("Square", 128, 128),
    ("Handheld", 160, 144),
];

pub(crate) fn apply_resolution(config: &mut MicroConfig, width: u32, height: u32) {
    config.width = width;
    config.height = height;
}

pub(crate) fn ui(ui: &mut egui::Ui, settings: &mut SettingsDocument, page: &mut SettingsPage) {
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(page, SettingsPage::General, "General");
        ui.selectable_value(page, SettingsPage::Micro, "Fantasy Console");
        ui.selectable_value(page, SettingsPage::Replay, "Replay");
    });
    ui.separator();
    match page {
        SettingsPage::General => general(ui, settings),
        SettingsPage::Micro => micro(ui, settings),
        SettingsPage::Replay => replay(ui, settings),
    }
}

fn general(ui: &mut egui::Ui, settings: &mut SettingsDocument) {
    let config = &mut settings.config;
    ui.label("Game title");
    ui.text_edit_singleline(&mut config.game.title);
    ui.horizontal(|ui| {
        ui.label("Window width");
        ui.add(egui::DragValue::new(&mut config.game.width).range(1..=8192));
        ui.label("Height");
        ui.add(egui::DragValue::new(&mut config.game.height).range(1..=8192));
    });
    ui.checkbox(&mut config.game.vsync, "Vsync");
    ui.checkbox(&mut config.game.resizable, "Resizable window");
    ui.checkbox(&mut config.audio.enabled, "Enable audio");
    ui.checkbox(
        &mut config.development.hot_reload,
        "Hot reload saved scripts and textures",
    );
    ui.separator();
    ui.strong("Physics");
    ui.horizontal(|ui| {
        ui.label("Gravity X");
        ui.add(egui::DragValue::new(&mut config.physics.gravity_x));
        ui.label("Y");
        ui.add(egui::DragValue::new(&mut config.physics.gravity_y));
    });
    ui.horizontal(|ui| {
        ui.label("Pixels per meter");
        ui.add(egui::DragValue::new(&mut config.physics.pixels_per_meter).range(0.01..=10_000.0));
    });
    ui.collapsing("Save data identity", |ui| {
        ui.label("Stable game identity: letters, digits, '-' and '_'.");
        ui.text_edit_singleline(&mut config.save.identity);
    });
}

fn micro(ui: &mut egui::Ui, settings: &mut SettingsDocument) {
    let window_size = [settings.config.game.width, settings.config.game.height];
    let micro = &mut settings.config.micro;
    ui.heading("Fantasy Console");
    ui.label("Kairo Micro gives your game a low-resolution pixel canvas.");
    ui.add_space(6.0);
    ui.checkbox(&mut micro.enabled, "Enable Fantasy Console (Kairo Micro)");
    ui.small(if micro.enabled {
        "ON for the next Run / Restart. Settings below are saved with the project."
    } else {
        "OFF. You can prepare the settings before enabling this mode."
    });
    ui.separator();
    ui.strong("Canvas resolution");
    ui.horizontal_wrapped(|ui| {
        for (name, width, height) in RESOLUTIONS {
            let selected = micro.width == width && micro.height == height;
            if ui
                .selectable_label(selected, format!("{name}  {width} x {height}"))
                .clicked()
            {
                apply_resolution(micro, width, height);
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Width");
        ui.add(egui::DragValue::new(&mut micro.width).range(1..=2048));
        ui.label("Height");
        ui.add(egui::DragValue::new(&mut micro.height).range(1..=2048));
    });
    ui.checkbox(
        &mut micro.integer_scaling,
        "Integer scaling with letterboxing",
    );
    ui.checkbox(&mut micro.pixel_snap, "Snap drawing to canvas pixels");
    ui.add_space(6.0);
    let label = PALETTES
        .iter()
        .find(|(name, _)| *name == micro.palette)
        .map_or(micro.palette.as_str(), |(_, label)| *label);
    egui::ComboBox::from_label("Palette")
        .selected_text(label)
        .show_ui(ui, |ui| {
            for (value, label) in PALETTES {
                ui.selectable_value(&mut micro.palette, value.to_owned(), label);
            }
        });
    if let Ok(colors) = kairo_core::micro::palette(&micro.palette) {
        ui.horizontal_wrapped(|ui| {
            for color in colors {
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(19.0, 19.0), egui::Sense::hover());
                let [r, g, b, _] = color.0;
                let color =
                    Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8);
                ui.painter().rect_filled(rect, 2.0, color);
                response.on_hover_text(format!(
                    "#{:02X}{:02X}{:02X}",
                    color.r(),
                    color.g(),
                    color.b()
                ));
            }
        });
    }
    let viewport = kairo_core::micro::viewport(
        window_size,
        [micro.width, micro.height],
        micro.integer_scaling,
    );
    ui.small(format!(
        "Window: {} x {}. Canvas displays at {} x {} with centered borders.",
        window_size[0], window_size[1], viewport.width, viewport.height
    ));
    ui.separator();
    ui.label("This changes the game's logical resolution, not its Lua layout. Use window.getSize() in game code or start from the Fantasy Console template.");
    ui.small("No code or assets are replaced. Disabling this mode restores normal window rendering on the next Run.");
}

fn replay(ui: &mut egui::Ui, settings: &mut SettingsDocument) {
    let replay = &mut settings.config.replay;
    ui.heading("Replay recording");
    ui.checkbox(
        &mut replay.enabled,
        "Start recording automatically when the game runs",
    );
    ui.add(egui::Slider::new(&mut replay.seconds, 0.1..=120.0).text("Seconds of history"));
    ui.add(egui::Slider::new(&mut replay.frequency, 1..=120).text("Snapshots per second"));
    ui.add(egui::Slider::new(&mut replay.memory_mib, 1..=256).text("Encoded memory budget (MiB)"));
    ui.separator();
    ui.label("Open Replay from the feature bar to record, pause, scrub snapshots and resume.");
    ui.label("Register the Lua tables that should be restored, usually inside game.load:");
    ui.code("replay.register(\"player\", player)");
    ui.small("Only supported registered state is restored. Functions, arbitrary locals, external I/O and complete physics solver state are not rewound.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use kairo_core::Config;

    #[test]
    fn every_palette_offered_by_the_editor_is_accepted_by_the_runtime() {
        for (name, _) in PALETTES {
            let mut config = Config::default();
            config.micro.enabled = true;
            config.micro.palette = name.into();
            config.validate().unwrap();
        }
    }

    #[test]
    fn resolution_presets_preserve_palette_and_enable_choice() {
        let mut micro = MicroConfig {
            palette: "olive4".into(),
            ..Default::default()
        };
        for (_, width, height) in RESOLUTIONS {
            apply_resolution(&mut micro, width, height);
            assert_eq!((micro.width, micro.height), (width, height));
            assert_eq!(micro.palette, "olive4");
            assert!(!micro.enabled);
        }
    }
}
