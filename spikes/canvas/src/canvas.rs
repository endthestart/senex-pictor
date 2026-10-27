//! Drawing a GPU-resident rendering inside egui: a paint callback that writes its view to its own
//! uniform slot in `prepare` and draws one quad in `paint`. Both renderings stay bound, so a swap
//! changes only which bind group the next frame uses.

use eframe::egui_wgpu::{self, CallbackResources, CallbackTrait, ScreenDescriptor};

/// Uniform slots: the canvas and three EDR test swatches.
pub const SLOTS: u64 = 4;
const SLOT: u64 = 256;

/// `canvas.wgsl` `View`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct View {
    pub lo: [f32; 2],
    pub hi: [f32; 2],
    pub solid: [f32; 4],
    pub scale: f32,
    pub mode: u32,
    pub pad: [f32; 2],
}

pub const MODE_ENCODE: u32 = 1;
pub const MODE_SOLID: u32 = 2;

pub struct CanvasGpu {
    pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    uniform_group: wgpu::BindGroup,
    /// Per rendering, per sampler (trilinear, nearest).
    images: [[wgpu::BindGroup; 2]; 2],
}

impl CanvasGpu {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        renderings: &[wgpu::Texture; 2],
    ) -> CanvasGpu {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("canvas"),
            source: wgpu::ShaderSource::Wgsl(include_str!("canvas.wgsl").into()),
        });
        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("canvas view"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<View>() as u64),
                },
                count: None,
            }],
        });
        let image_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("canvas image"),
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
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("canvas"),
            bind_group_layouts: &[Some(&view_layout), Some(&image_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("canvas"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("canvas views"),
            size: SLOTS * SLOT,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("canvas views"),
            layout: &view_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform,
                    offset: 0,
                    size: wgpu::BufferSize::new(std::mem::size_of::<View>() as u64),
                }),
            }],
        });
        let sampler = |nearest: bool| {
            let f = if nearest {
                wgpu::FilterMode::Nearest
            } else {
                wgpu::FilterMode::Linear
            };
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("canvas"),
                mag_filter: f,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                ..Default::default()
            })
        };
        let samplers = [sampler(false), sampler(true)];
        let images = std::array::from_fn(|r| {
            let view = renderings[r].create_view(&Default::default());
            std::array::from_fn(|s| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("canvas image"),
                    layout: &image_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&samplers[s]),
                        },
                    ],
                })
            })
        });
        CanvasGpu {
            pipeline,
            uniform,
            uniform_group,
            images,
        }
    }

    pub fn set_view(&self, queue: &wgpu::Queue, slot: u64, view: &View) {
        queue.write_buffer(&self.uniform, slot * SLOT, bytemuck::bytes_of(view));
    }

    pub fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        slot: u64,
        rendering: usize,
        nearest: bool,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_group, &[(slot * SLOT) as u32]);
        pass.set_bind_group(1, &self.images[rendering][usize::from(nearest)], &[]);
        pass.draw(0..4, 0..1);
    }
}

/// One draw of the canvas (or a swatch) in an egui frame.
pub struct Draw {
    pub view: View,
    pub slot: u64,
    pub rendering: usize,
    pub nearest: bool,
}

impl CallbackTrait for Draw {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(gpu) = resources.get::<CanvasGpu>() {
            gpu.set_view(queue, self.slot, &self.view);
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        if let Some(gpu) = resources.get::<CanvasGpu>() {
            gpu.draw(pass, self.slot, self.rendering, self.nearest);
        }
    }
}

/// Size in whole physical pixels of the viewport egui gives a callback over `rect`.
pub fn viewport_px(rect: egui::Rect, ppp: f32) -> [f32; 2] {
    let px = |a: f32, b: f32| (b * ppp).round() - (a * ppp).round();
    [px(rect.min.x, rect.max.x), px(rect.min.y, rect.max.y)]
}

/// The image's quad in a viewport of `size` physical px, at `zoom` physical px per image px,
/// with image point `centre` in the middle. At whole-number zooms its corner lands on a whole
/// pixel, so 100% shows each image pixel on one screen pixel.
pub fn place(size: [f32; 2], image: [f32; 2], zoom: f32, centre: [f32; 2]) -> ([f32; 2], [f32; 2]) {
    let mut left = size[0] / 2.0 - centre[0] * zoom;
    let mut top = size[1] / 2.0 - centre[1] * zoom;
    if zoom.fract() == 0.0 {
        left = left.round();
        top = top.round();
    }
    let (right, bottom) = (left + image[0] * zoom, top + image[1] * zoom);
    let x = |px: f32| px / size[0] * 2.0 - 1.0;
    let y = |px: f32| 1.0 - px / size[1] * 2.0;
    ([x(left), y(bottom)], [x(right), y(top)])
}

/// A paint callback for `draw` over `rect`.
pub fn shape(rect: egui::Rect, draw: Draw) -> egui::Shape {
    egui::Shape::Callback(egui_wgpu::Callback::new_paint_callback(rect, draw))
}
