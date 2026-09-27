use crate::pixels::{PixelBlock, Pixels};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, TextureHandle, Vec2};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Pencil,
    Eraser,
    Fill,
    Picker,
    Line,
    Rectangle,
    Select,
}

pub struct SpriteEditor {
    pub pixels: Pixels,
    zoom: f32,
    pan: Vec2,
    grid: bool,
    color: [u8; 4],
    tool: Tool,
    texture: Option<TextureHandle>,
    uploaded_revision: u64,
    last_pixel: Option<(i32, i32)>,
    drag_start: Option<(i32, i32)>,
    selection: Option<((i32, i32), (i32, i32))>,
    clipboard: Option<PixelBlock>,
    resize_size: [u32; 2],
    resizing: bool,
    error: Option<String>,
}

impl SpriteEditor {
    pub fn new(pixels: Pixels) -> Self {
        let zoom = (360.0 / pixels.width.max(pixels.height) as f32)
            .clamp(1.0, 16.0)
            .floor();
        let resize_size = [pixels.width, pixels.height];
        Self {
            resize_size,
            resizing: false,
            error: None,
            drag_start: None,
            selection: None,
            clipboard: None,
            pixels,
            zoom,
            pan: Vec2::ZERO,
            grid: true,
            color: [225, 235, 250, 255],
            tool: Tool::Pencil,
            texture: None,
            uploaded_revision: 0,
            last_pixel: None,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (tool, name) in [
                (Tool::Pencil, "Pencil"),
                (Tool::Eraser, "Eraser"),
                (Tool::Fill, "Fill"),
                (Tool::Picker, "Pick"),
                (Tool::Line, "Line"),
                (Tool::Rectangle, "Rect"),
                (Tool::Select, "Select"),
            ] {
                ui.selectable_value(&mut self.tool, tool, name);
            }
            ui.separator();
            if ui.button("Undo").clicked() {
                self.pixels.undo();
            }
            if ui.button("Redo").clicked() {
                self.pixels.redo();
            }
            if ui.button("Flip X").clicked() {
                self.pixels.flip(true);
            }
            if ui.button("Flip Y").clicked() {
                self.pixels.flip(false);
            }
            ui.checkbox(&mut self.grid, "Grid");
            if ui.button("Copy").clicked() {
                let (from, to) = self.selection.unwrap_or((
                    (0, 0),
                    (self.pixels.width as i32 - 1, self.pixels.height as i32 - 1),
                ));
                self.clipboard = Some(self.pixels.copy_region(from, to));
            }
            if ui
                .add_enabled(self.clipboard.is_some(), egui::Button::new("Paste"))
                .clicked()
            {
                if let Some(block) = &self.clipboard {
                    let at = self
                        .selection
                        .map_or((0, 0), |(a, b)| (a.0.min(b.0), a.1.min(b.1)));
                    self.pixels.paste(block, at);
                }
            }
            if ui.button("Canvas size").clicked() {
                self.resize_size = [self.pixels.width, self.pixels.height];
                self.resizing = !self.resizing;
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.color_edit_button_srgba_unmultiplied(&mut self.color);
            for color in [
                [235, 242, 255, 255],
                [26, 31, 41, 255],
                [231, 91, 104, 255],
                [240, 175, 81, 255],
                [86, 190, 152, 255],
                [92, 155, 221, 255],
                [174, 130, 218, 255],
                [0, 0, 0, 0],
            ] {
                let fill = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                if ui
                    .add(egui::Button::new("  ").fill(fill))
                    .on_hover_text(format!("RGBA {color:?}"))
                    .clicked()
                {
                    self.color = color;
                }
            }
            ui.add(egui::Slider::new(&mut self.zoom, 1.0..=32.0).text("Zoom"));
            if ui.button("Center").clicked() {
                self.pan = Vec2::ZERO;
            }
        });
        if self.resizing {
            ui.horizontal(|ui| {
                ui.label("Width / height");
                ui.add(egui::DragValue::new(&mut self.resize_size[0]).range(1..=1024));
                ui.add(egui::DragValue::new(&mut self.resize_size[1]).range(1..=1024));
                if ui.button("Apply (top-left anchor)").clicked() {
                    match self.pixels.resize(self.resize_size[0], self.resize_size[1]) {
                        Ok(()) => {
                            self.selection = None;
                            self.resizing = false;
                            self.error = None;
                        }
                        Err(error) => self.error = Some(error.to_string()),
                    }
                }
            });
        }
        if let Some(error) = &self.error {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
        ui.small(format!(
            "{} x {}   PNG   Middle-drag to pan; scroll to zoom; Ctrl/Cmd+S saves",
            self.pixels.width, self.pixels.height
        ));
        ui.separator();
        let available = ui.available_size().max(Vec2::splat(40.0));
        let (canvas, response) = ui.allocate_exact_size(available, Sense::click_and_drag());
        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let previous = self.zoom;
                self.zoom = (self.zoom * (scroll * 0.005).exp()).clamp(1.0, 32.0);
                if let Some(pointer) = response.hover_pos() {
                    let local = pointer - canvas.min - Vec2::splat(24.0) - self.pan;
                    self.pan += local * (1.0 - self.zoom / previous);
                }
            }
        }
        if response.dragged_by(egui::PointerButton::Middle) {
            self.pan += ui.input(|input| input.pointer.delta());
        }
        let image_rect = Rect::from_min_size(
            canvas.min + Vec2::splat(24.0) + self.pan,
            Vec2::new(self.pixels.width as f32, self.pixels.height as f32) * self.zoom,
        );
        let drawing = ui.input(|input| input.pointer.primary_down())
            && (response.hovered() || response.dragged_by(egui::PointerButton::Primary));
        if drawing {
            if let Some(position) = ui
                .input(|input| input.pointer.interact_pos())
                .filter(|p| image_rect.contains(*p))
            {
                let relative = (position - image_rect.min) / self.zoom;
                let point = (relative.x.floor() as i32, relative.y.floor() as i32);
                if self.drag_start.is_none() {
                    self.drag_start = Some(point);
                }
                match self.tool {
                    Tool::Line | Tool::Rectangle => {}
                    Tool::Select => {
                        self.selection = self.drag_start.map(|start| (start, point));
                    }
                    Tool::Picker => {
                        if let Some(color) = self.pixels.color(point.0, point.1) {
                            self.color = color;
                        }
                    }
                    Tool::Pencil | Tool::Eraser => {
                        self.pixels.begin();
                        let color = if self.tool == Tool::Eraser {
                            [0; 4]
                        } else {
                            self.color
                        };
                        self.pixels
                            .line(self.last_pixel.unwrap_or(point), point, color);
                    }
                    Tool::Fill => {
                        if self.last_pixel.is_none() {
                            self.pixels.begin();
                            self.pixels.fill(point.0, point.1, self.color);
                        }
                    }
                }
                self.last_pixel = Some(point);
            }
        } else if !ui.input(|input| input.pointer.primary_down()) {
            if let (Some(from), Some(to)) = (self.drag_start, self.last_pixel) {
                if matches!(self.tool, Tool::Line | Tool::Rectangle) {
                    self.pixels.begin();
                    if self.tool == Tool::Line {
                        self.pixels.line(from, to, self.color);
                    } else {
                        self.pixels.rectangle(from, to, self.color);
                    }
                }
            }
            self.pixels.commit();
            self.last_pixel = None;
            self.drag_start = None;
        }

        if self.texture.is_none() || self.uploaded_revision != self.pixels.revision {
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [self.pixels.width as usize, self.pixels.height as usize],
                &self.pixels.rgba,
            );
            if let Some(texture) = &mut self.texture {
                texture.set(image, egui::TextureOptions::NEAREST);
            } else {
                self.texture = Some(ui.ctx().load_texture(
                    self.pixels.path.display().to_string(),
                    image,
                    egui::TextureOptions::NEAREST,
                ));
            }
            self.uploaded_revision = self.pixels.revision;
        }
        let painter = ui.painter_at(canvas);
        painter.rect_filled(canvas, 0.0, Color32::from_rgb(17, 20, 25));
        let clipped = canvas.intersect(image_rect);
        if clipped.is_positive() {
            let checker = 12.0;
            let mut y = clipped.top();
            let mut row = 0;
            while y < clipped.bottom() {
                let mut x = clipped.left();
                let mut column = 0;
                while x < clipped.right() {
                    let value = if (row + column) % 2 == 0 { 48 } else { 61 };
                    painter.rect_filled(
                        Rect::from_min_max(
                            Pos2::new(x, y),
                            Pos2::new(
                                (x + checker).min(clipped.right()),
                                (y + checker).min(clipped.bottom()),
                            ),
                        ),
                        0.0,
                        Color32::from_gray(value),
                    );
                    x += checker;
                    column += 1;
                }
                y += checker;
                row += 1;
            }
        }
        if let Some(texture) = &self.texture {
            painter.image(
                texture.id(),
                image_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        if let Some((from, to)) = self.selection {
            let a = image_rect.min
                + Vec2::new(from.0.min(to.0) as f32, from.1.min(to.1) as f32) * self.zoom;
            let b = image_rect.min
                + Vec2::new((from.0.max(to.0) + 1) as f32, (from.1.max(to.1) + 1) as f32)
                    * self.zoom;
            painter.rect_stroke(
                Rect::from_min_max(a, b),
                0.0,
                Stroke::new(1.0_f32, Color32::YELLOW),
            );
        }
        if let (Some(from), Some(to)) = (self.drag_start, self.last_pixel) {
            let a =
                image_rect.min + Vec2::new(from.0 as f32 + 0.5, from.1 as f32 + 0.5) * self.zoom;
            let b = image_rect.min + Vec2::new(to.0 as f32 + 0.5, to.1 as f32 + 0.5) * self.zoom;
            let stroke = Stroke::new(1.0_f32, Color32::WHITE);
            if self.tool == Tool::Line {
                painter.line_segment([a, b], stroke);
            }
            if self.tool == Tool::Rectangle {
                painter.rect_stroke(Rect::from_two_pos(a, b), 0.0, stroke);
            }
        }
        if self.grid && self.zoom >= 5.0 {
            let first_x = ((canvas.left() - image_rect.left()) / self.zoom)
                .floor()
                .max(0.0) as u32;
            let last_x = ((canvas.right() - image_rect.left()) / self.zoom)
                .ceil()
                .max(0.0) as u32;
            let first_y = ((canvas.top() - image_rect.top()) / self.zoom)
                .floor()
                .max(0.0) as u32;
            let last_y = ((canvas.bottom() - image_rect.top()) / self.zoom)
                .ceil()
                .max(0.0) as u32;
            let stroke = Stroke::new(0.5_f32, Color32::from_black_alpha(90));
            for x in first_x..=last_x.min(self.pixels.width) {
                let x = image_rect.left() + x as f32 * self.zoom;
                painter.line_segment(
                    [
                        Pos2::new(x, image_rect.top()),
                        Pos2::new(x, image_rect.bottom()),
                    ],
                    stroke,
                );
            }
            for y in first_y..=last_y.min(self.pixels.height) {
                let y = image_rect.top() + y as f32 * self.zoom;
                painter.line_segment(
                    [
                        Pos2::new(image_rect.left(), y),
                        Pos2::new(image_rect.right(), y),
                    ],
                    stroke,
                );
            }
        }
    }
}
