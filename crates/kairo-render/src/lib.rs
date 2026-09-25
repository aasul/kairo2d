//! Batched scene rendering, optional virtual canvas, and one submission per frame.
mod debugui;
pub mod mesh;
mod micro;

use anyhow::{bail, ensure, Context, Result};
use kairo_assets::{AssetManager, TextureFilter};
use kairo_core::{Frame, TextureHandle};
use mesh::{Mesh, Vertex};
use std::collections::HashMap;
use std::sync::Arc;
use winit::window::Window;

struct GpuTexture {
    bind_group: wgpu::BindGroup,
    revision: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RenderStats {
    pub draw_calls: usize,
    pub vertices: usize,
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    linear_sampler: wgpu::Sampler,
    white: GpuTexture,
    textures: HashMap<TextureHandle, GpuTexture>,
    vertices: wgpu::Buffer,
    vertex_capacity: u64,
    mesh: Mesh,
    suspended: bool,
    micro: Option<micro::MicroTarget>,
    debug: debugui::DebugPainter,
}

impl Renderer {
    pub async fn new(window: Arc<Window>, vsync: bool) -> Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window)
            .context("cannot create graphics surface")?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: false,
            })
            .await
            .context(
                "no compatible GPU found; install a Vulkan, Metal, DirectX 12, or OpenGL driver",
            )?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Kairo2D device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .context("cannot initialize graphics device")?;
        log::info!(
            "GPU: {} ({:?})",
            adapter.get_info().name,
            adapter.get_info().backend
        );
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .context("GPU surface has no supported sRGB format")?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: present_mode(vsync),
            alpha_mode: capabilities
                .alpha_modes
                .first()
                .copied()
                .context("surface has no alpha modes")?,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        ensure!(
            config.width <= device.limits().max_texture_dimension_2d
                && config.height <= device.limits().max_texture_dimension_2d,
            "window is larger than the graphics device supports"
        );
        surface.configure(&device, &config);
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite texture layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nearest pixel sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear sprite sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let white = upload(&device, &queue, &texture_layout, &sampler, 1, 1, &[255; 4])?;
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Kairo2D sprite shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sprite.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite pipeline layout"),
            bind_group_layouts: &[&texture_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite pipeline"), layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader, entry_point: "vs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: "fs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview: None,
        });
        if let Some(error) = device.pop_error_scope().await {
            bail!("cannot create sprite pipeline: {error}");
        }
        let vertex_capacity = 64 * 1024;
        let vertices = vertex_buffer(&device, vertex_capacity);
        let debug = debugui::DebugPainter::new(&device, format);
        Ok(Self {
            debug,
            surface,
            device,
            queue,
            config,
            pipeline,
            texture_layout,
            sampler,
            linear_sampler,
            white,
            textures: HashMap::new(),
            vertices,
            vertex_capacity,
            mesh: Mesh::default(),
            suspended: false,
            micro: None,
        })
    }

    pub fn configure_micro(&mut self, config: &kairo_core::config::MicroConfig) -> Result<()> {
        self.micro = if config.enabled {
            Some(micro::MicroTarget::new(
                &self.device,
                self.config.format,
                &self.texture_layout,
                &self.sampler,
                config,
            )?)
        } else {
            None
        };
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.suspended = width == 0 || height == 0;
        if self.suspended {
            return Ok(());
        }
        ensure!(
            width <= self.device.limits().max_texture_dimension_2d
                && height <= self.device.limits().max_texture_dimension_2d,
            "window is larger than the graphics device supports"
        );
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        Ok(())
    }

    pub fn set_vsync(&mut self, enabled: bool) {
        self.config.present_mode = present_mode(enabled);
        if !self.suspended {
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn clear_assets(&mut self) {
        self.textures.clear();
    }

    pub fn debug_pointer(&mut self, x: f32, y: f32) {
        self.debug.pointer(x, y);
    }
    pub fn debug_button(&mut self, button: u8, down: bool) {
        self.debug.button(button, down);
    }
    pub fn debug_focus_lost(&mut self) {
        self.debug.focus_lost();
    }

    pub fn render(&mut self, frame: &Frame, assets: &AssetManager) -> Result<RenderStats> {
        self.render_with_debug(frame, assets, None)
    }

    pub fn render_with_debug(
        &mut self,
        frame: &Frame,
        assets: &AssetManager,
        debug: Option<&mut kairo_core::debugui::DebugUi>,
    ) -> Result<RenderStats> {
        if self.suspended {
            return Ok(RenderStats::default());
        }
        let size = self
            .micro
            .as_ref()
            .map(|micro| micro.size)
            .unwrap_or([self.config.width, self.config.height]);
        self.mesh.pixel_snap = self.micro.as_ref().is_some_and(|micro| micro.pixel_snap);
        self.mesh.build(frame, size[0], size[1])?;
        self.textures.retain(|handle, _| assets.contains(*handle));
        for batch in &self.mesh.batches {
            if let Some(handle) = batch.texture {
                let asset = assets.texture(handle)?;
                if self
                    .textures
                    .get(&handle)
                    .is_none_or(|gpu| gpu.revision != asset.revision)
                {
                    let sampler = match asset.filter {
                        TextureFilter::Nearest => &self.sampler,
                        TextureFilter::Linear => &self.linear_sampler,
                    };
                    let mut texture = upload(
                        &self.device,
                        &self.queue,
                        &self.texture_layout,
                        sampler,
                        asset.width,
                        asset.height,
                        &asset.rgba,
                    )?;
                    texture.revision = asset.revision;
                    self.textures.insert(handle, texture);
                }
            }
        }
        let output = match self.surface.get_current_texture() {
            Ok(output) => output,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(RenderStats::default());
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(RenderStats::default()),
            Err(wgpu::SurfaceError::OutOfMemory) => bail!("graphics device ran out of memory"),
        };
        let bytes = bytemuck::cast_slice(&self.mesh.vertices);
        if bytes.len() as u64 > self.vertex_capacity {
            self.vertex_capacity = (bytes.len() as u64).next_power_of_two();
            ensure!(
                self.vertex_capacity <= self.device.limits().max_buffer_size,
                "frame exceeds GPU buffer limit"
            );
            self.vertices = vertex_buffer(&self.device, self.vertex_capacity);
        }
        if !bytes.is_empty() {
            self.queue.write_buffer(&self.vertices, 0, bytes);
        }
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Kairo2D frame"),
            });
        let scene_view = self
            .micro
            .as_ref()
            .map(|micro| &micro.view)
            .unwrap_or(&view);
        let clear = frame.clear.linear();
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("2D render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: scene_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear[0] as f64,
                            g: clear[1] as f64,
                            b: clear[2] as f64,
                            a: clear[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.vertices.slice(..));
            for batch in &self.mesh.batches {
                let view = batch.viewport;
                let Some(clip) = batch.scissor.unwrap_or(view).intersection(view) else {
                    continue;
                };
                pass.set_viewport(
                    view.x as f32,
                    view.y as f32,
                    view.width as f32,
                    view.height as f32,
                    0.0,
                    1.0,
                );
                pass.set_scissor_rect(clip.x, clip.y, clip.width, clip.height);
                let texture = match batch.texture {
                    Some(handle) => self
                        .textures
                        .get(&handle)
                        .context("texture upload is missing")?,
                    None => &self.white,
                };
                pass.set_bind_group(0, &texture.bind_group, &[]);
                pass.draw(batch.vertices.clone(), 0..1);
            }
        }
        if let Some(micro) = &self.micro {
            micro.present(&mut encoder, &view, [self.config.width, self.config.height]);
        }
        let extra = match debug {
            Some(model) => self.debug.paint(
                &self.device,
                &self.queue,
                &mut encoder,
                &view,
                [self.config.width, self.config.height],
                model,
            ),
            None => Vec::new(),
        };
        self.queue
            .submit(extra.into_iter().chain(Some(encoder.finish())));
        output.present();
        Ok(RenderStats {
            draw_calls: self.mesh.batches.len(),
            vertices: self.mesh.vertices.len(),
        })
    }
}

fn present_mode(vsync: bool) -> wgpu::PresentMode {
    if vsync {
        wgpu::PresentMode::AutoVsync
    } else {
        wgpu::PresentMode::AutoNoVsync
    }
}

fn vertex_buffer(device: &wgpu::Device, capacity: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("2D vertices"),
        size: capacity,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<GpuTexture> {
    ensure!(
        width > 0
            && height > 0
            && width <= device.limits().max_texture_dimension_2d
            && height <= device.limits().max_texture_dimension_2d,
        "texture dimensions exceed GPU limits"
    );
    ensure!(
        rgba.len() as u64 == u64::from(width) * u64::from(height) * 4,
        "invalid texture byte count"
    );
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("2D texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        size,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("2D texture bindings"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    Ok(GpuTexture {
        bind_group,
        revision: 1,
    })
}

#[cfg(test)]
mod shader_tests {
    #[test]
    fn built_in_shaders_parse_and_validate_without_a_gpu() {
        for source in [include_str!("sprite.wgsl"), include_str!("micro.wgsl")] {
            let module = naga::front::wgsl::parse_str(source).expect("WGSL parse failed");
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::empty(),
            )
            .validate(&module)
            .expect("WGSL validation failed");
        }
    }
}
