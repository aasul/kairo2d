use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect};
use kairo_core::debugui::{DebugUi, Response, Widget};
use std::time::Instant;

pub(crate) struct DebugPainter {
    context: egui::Context,
    renderer: egui_wgpu::Renderer,
    events: Vec<Event>,
    pointer: Pos2,
    start: Instant,
}

impl DebugPainter {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self {
            context: egui::Context::default(),
            renderer: egui_wgpu::Renderer::new(device, format, None, 1),
            events: Vec::new(),
            pointer: Pos2::ZERO,
            start: Instant::now(),
        }
    }
    pub fn pointer(&mut self, x: f32, y: f32) {
        self.pointer = Pos2::new(x, y);
        if self.events.len() < 1024 {
            self.events.push(Event::PointerMoved(self.pointer));
        }
    }
    pub fn button(&mut self, button: u8, pressed: bool) {
        let button = match button {
            1 => PointerButton::Primary,
            2 => PointerButton::Secondary,
            3 => PointerButton::Middle,
            _ => return,
        };
        if self.events.len() < 1024 {
            self.events.push(Event::PointerButton {
                pos: self.pointer,
                button,
                pressed,
                modifiers: Modifiers::NONE,
            });
        }
    }
    pub fn focus_lost(&mut self) {
        for button in 1..=3 {
            self.button(button, false);
        }
        self.events.push(Event::PointerGone);
    }
    pub fn paint(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        size: [u32; 2],
        model: &mut DebugUi,
    ) -> Vec<wgpu::CommandBuffer> {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(size[0] as f32, size[1] as f32),
            )),
            time: Some(self.start.elapsed().as_secs_f64()),
            events: std::mem::take(&mut self.events),
            focused: true,
            ..Default::default()
        };
        let output = self.context.run(input, |ctx| {
            for window in &model.windows {
                egui::Window::new(&window.title).show(ctx, |ui| {
                    for widget in &window.widgets {
                        match widget {
                            Widget::Text(text) => {
                                ui.label(text);
                            }
                            Widget::Button { label, key } => {
                                if ui.button(label).clicked() {
                                    model.responses.insert(key.clone(), Response::Click);
                                }
                            }
                            Widget::Checkbox { label, key, value } => {
                                let mut value = *value;
                                if ui.checkbox(&mut value, label).changed() {
                                    model.responses.insert(key.clone(), Response::Bool(value));
                                }
                            }
                            Widget::Slider {
                                label,
                                key,
                                value,
                                min,
                                max,
                            } => {
                                let mut value = *value;
                                if ui
                                    .add(egui::Slider::new(&mut value, *min..=*max).text(label))
                                    .changed()
                                {
                                    model.responses.insert(key.clone(), Response::Number(value));
                                }
                            }
                        }
                    }
                });
            }
        });
        model.wants_mouse = self.context.wants_pointer_input();
        let jobs = self
            .context
            .tessellate(output.shapes, output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: size,
            pixels_per_point: output.pixels_per_point,
        };
        for (id, delta) in &output.textures_delta.set {
            self.renderer.update_texture(device, queue, *id, delta);
        }
        let commands = self
            .renderer
            .update_buffers(device, queue, encoder, &jobs, &screen);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Kairo debug UI"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.renderer.render(&mut pass, &jobs, &screen);
        }
        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }
        commands
    }
}
