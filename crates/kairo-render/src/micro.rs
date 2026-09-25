use anyhow::{ensure, Result};
use kairo_core::config::MicroConfig;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Palette {
    colors: [[f32; 4]; 16],
    info: [u32; 4],
}

pub(crate) struct MicroTarget {
    pub view: wgpu::TextureView,
    pub size: [u32; 2],
    pub pixel_snap: bool,
    integer_scaling: bool,
    image: wgpu::BindGroup,
    palette: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
}

impl MicroTarget {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        texture_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        config: &MicroConfig,
    ) -> Result<Self> {
        ensure!(
            config.width > 0
                && config.height > 0
                && config.width <= device.limits().max_texture_dimension_2d
                && config.height <= device.limits().max_texture_dimension_2d,
            "Micro canvas exceeds device limits"
        );
        let colors = kairo_core::micro::palette(&config.palette)?;
        let mut palette = Palette {
            colors: [[0.0; 4]; 16],
            info: [colors.len() as u32, 0, 0, 0],
        };
        for (target, color) in palette.colors.iter_mut().zip(colors) {
            *target = color.linear();
        }
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Micro palette"),
            contents: bytemuck::bytes_of(&palette),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let palette_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Micro palette layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let palette = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Micro palette binding"),
            layout: &palette_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Micro virtual canvas"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let image = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Micro canvas binding"),
            layout: texture_layout,
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Micro presentation"),
            source: wgpu::ShaderSource::Wgsl(include_str!("micro.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Micro pipeline layout"),
            bind_group_layouts: &[texture_layout, &palette_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Micro nearest/palette presentation"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
        });
        Ok(Self {
            view,
            size: [config.width, config.height],
            pixel_snap: config.pixel_snap,
            integer_scaling: config.integer_scaling,
            image,
            palette,
            pipeline,
        })
    }

    pub fn present(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        screen: [u32; 2],
    ) {
        let rect = kairo_core::micro::viewport(screen, self.size, self.integer_scaling);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Micro presentation pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_viewport(
            rect.x as f32,
            rect.y as f32,
            rect.width as f32,
            rect.height as f32,
            0.0,
            1.0,
        );
        pass.set_scissor_rect(rect.x, rect.y, rect.width, rect.height);
        pass.set_bind_group(0, &self.image, &[]);
        pass.set_bind_group(1, &self.palette, &[]);
        pass.draw(0..3, 0..1);
    }
}
