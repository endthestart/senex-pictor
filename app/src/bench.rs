//! The off-screen benchmark: the GPU time to recompute a look (with mips) and to draw the canvas
//! into a window-sized target at fit, 100% and 200%, swapping renderings every frame. Each frame
//! is submitted and waited for, so the times are what the GPU needs, independent of the display.

use crate::gpu::canvas::{self, CanvasGpu, View};
use crate::gpu::look::{FORMAT, Looks};
use crate::scene::LookParams;
use crate::stats;
use star_photos::{Result, error};
use std::time::Instant;

const FRAMES: usize = 120;
const LOOK_RUNS: usize = 5;

fn wait(device: &wgpu::Device, index: wgpu::SubmissionIndex) -> Result<()> {
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(index),
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .map(|_| ())
        .map_err(|e| error(format!("GPU wait failed: {e}")))
}

pub fn run(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    looks: &Looks,
    params: &[LookParams; 2],
    image: [u32; 2],
    target: [u32; 2],
) -> Result<serde_json::Value> {
    let mut look_times = Vec::new();
    for _ in 0..LOOK_RUNS {
        for (k, p) in params.iter().enumerate() {
            look_times.push(looks.apply(device, queue, k, p)?);
        }
    }
    let gpu = CanvasGpu::new(device, FORMAT, &looks.outputs);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bench target"),
        size: wgpu::Extent3d {
            width: target[0],
            height: target[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let size = [target[0] as f32, target[1] as f32];
    let img = [image[0] as f32, image[1] as f32];
    let fit = (size[0] / img[0]).min(size[1] / img[1]);
    let mut phases = serde_json::Map::new();
    for (name, zoom) in [("fit", fit), ("100%", 1.0), ("200%", 2.0)] {
        let mut times = Vec::with_capacity(FRAMES);
        for f in 0..FRAMES {
            let sweep = (f as f32 / FRAMES as f32 * std::f32::consts::TAU).sin() * 0.4 + 0.5;
            let (lo, hi) = canvas::place(size, img, zoom, [img[0] * sweep, img[1] * 0.5]);
            let start = Instant::now();
            gpu.set_view(queue, 0, &View::image(lo, hi, 1.0, 0, f % 2 == 1));
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bench"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });
                gpu.draw(&mut pass, 0, zoom >= 2.0);
            }
            wait(device, queue.submit([encoder.finish()]))?;
            times.push(start.elapsed().as_secs_f64());
        }
        phases.insert(name.to_string(), stats::summary(&times));
    }
    Ok(serde_json::json!({
        "image": image,
        "target": target,
        "look_recompute": stats::summary(&look_times),
        "canvas_frames": phases,
    }))
}
