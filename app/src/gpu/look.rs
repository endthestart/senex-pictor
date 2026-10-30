//! The two renderings, computed on the shared device from the uploaded scene and kept there:
//! a compute pass applies a look at the output size (sampling the scene, so a 24 MP stack can
//! stand in for a 45 MP one), then the mip levels are rebuilt. Nothing is read back.

use super::mips::{self, Mips};
use crate::scene::{LookParams, Scene};
use pictor_engine::{Result, error};
use std::time::Instant;

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const WORKGROUP: u32 = 16;

pub struct Looks {
    pipeline: wgpu::ComputePipeline,
    groups: [wgpu::BindGroup; 2],
    uniforms: [wgpu::Buffer; 2],
    pub outputs: [wgpu::Texture; 2],
    mips: Mips,
}

fn source(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &Scene,
    data: &[u16],
    label: &str,
) -> wgpu::Texture {
    let size = wgpu::Extent3d {
        width: scene.width,
        height: scene.height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(data),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(8 * scene.width),
            rows_per_image: Some(scene.height),
        },
        size,
    );
    texture
}

impl Looks {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &Scene,
        size: [u32; 2],
    ) -> Result<Looks> {
        let limit = device.limits().max_texture_dimension_2d;
        if size[0] > limit || size[1] > limit {
            return Err(error(format!(
                "{}×{} exceeds this GPU's {limit} px texture limit; tiles would be needed",
                size[0], size[1]
            )));
        }
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("look"),
            source: wgpu::ShaderSource::Wgsl(include_str!("look.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("look"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let base = source(device, queue, scene, &scene.base, "scene");
        let glow = source(device, queue, scene, &scene.glow, "glow");
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("scene"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let output = |label: &str| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: mips::levels(size[0], size[1]),
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: FORMAT,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
        };
        let outputs = [output("rendering A"), output("rendering B")];
        let uniforms = std::array::from_fn(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("look"),
                size: std::mem::size_of::<LookParams>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        let layout = pipeline.get_bind_group_layout(0);
        let (base_view, glow_view) = (
            base.create_view(&Default::default()),
            glow.create_view(&Default::default()),
        );
        let groups = std::array::from_fn(|k| {
            let top = outputs[k].create_view(&wgpu::TextureViewDescriptor {
                mip_level_count: Some(1),
                ..Default::default()
            });
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("look"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniforms[k].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&base_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&glow_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&top),
                    },
                ],
            })
        });
        Ok(Looks {
            pipeline,
            groups,
            uniforms,
            outputs,
            mips: Mips::new(device, FORMAT),
        })
    }

    /// Recomputes rendering `k` with `params` and its mip levels; returns the seconds the GPU
    /// took (submission to completion).
    pub fn apply(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        k: usize,
        params: &LookParams,
    ) -> Result<f64> {
        let start = Instant::now();
        queue.write_buffer(&self.uniforms[k], 0, bytemuck::bytes_of(params));
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.groups[k], &[]);
            let size = self.outputs[k].size();
            pass.dispatch_workgroups(
                size.width.div_ceil(WORKGROUP),
                size.height.div_ceil(WORKGROUP),
                1,
            );
        }
        self.mips.generate(device, &mut encoder, &self.outputs[k]);
        let index = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(index),
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .map_err(|e| error(format!("GPU wait failed: {e}")))?;
        Ok(start.elapsed().as_secs_f64())
    }
}
