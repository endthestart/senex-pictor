//! The CFA sky stack on the GPU (`stack.wgsl`), band by band with the same band reads,
//! selection rules and robust combination as `integrate::cfa`.

use super::Gpu;
use crate::budget::Budget;
use crate::integrate::cfa::{CLEARANCE_PX, Inputs, Rows, SkyStack, band_rows, bands};
use crate::separate::glow::{BLOCK, CLEAR};
use crate::sky::model::Matrix;
use crate::store::file;
use crate::{Result, error};
use rayon::prelude::*;

const WORKGROUP: u32 = 128;
const OUTPUTS: usize = 4;
/// The kernel's mark for a mean without weight.
const NONE: f32 = -1e30;
/// Output rows per dispatch: large bands amortise each dispatch's fixed cost.
const BAND: usize = 512;

fn words(values: &[u32]) -> &[u8] {
    bytemuck::cast_slice(values)
}

fn params(inputs: &Inputs, b0: usize, rows: usize, words_per_row: usize) -> Vec<u32> {
    let m = &inputs.store.manifest;
    let c = &inputs.model.camera;
    let centre = c.centre();
    let grid = &inputs.glow[0];
    let f = |v: f64| (v as f32).to_bits();
    let mut p = vec![
        m.width as u32,
        m.height as u32,
        b0 as u32,
        rows as u32,
        m.frames.len() as u32,
        inputs.model.reference as u32,
        grid.columns as u32,
        grid.rows as u32,
    ];
    p.extend(m.cfa.iter().map(|&v| u32::from(v)));
    p.extend([
        f(c.f),
        f(c.k1),
        f(c.k2),
        f(c.scale()),
        f(centre[0]),
        f(centre[1]),
    ]);
    p.extend([f(inputs.kernel.sigma), f(inputs.kernel.radius)]);
    p.extend([
        CLEARANCE_PX.to_bits(),
        CLEAR.to_bits(),
        (BLOCK as f32).to_bits(),
        words_per_row as u32,
    ]);
    p
}

/// Frame records (`Frame` in the kernel) and the packed CFA and occlusion words of a band.
fn pack(
    inputs: &Inputs,
    rotations: &[Matrix],
    rows: &[Option<Rows>],
) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let m = &inputs.store.manifest;
    let w = m.width;
    let (mut frames, mut cfa, mut occ) = (Vec::new(), Vec::new(), Vec::new());
    for (i, band) in rows.iter().enumerate() {
        let r = &rotations[i];
        for row in r {
            frames.extend([row[0] as f32, row[1] as f32, row[2] as f32, 0.0].map(f32::to_bits));
        }
        let b = &m.frames[i].black;
        frames.extend([b[0], b[1], b[2], b[3]].map(f32::to_bits));
        let (cfa_offset, occ_offset) = (2 * cfa.len(), occ.len());
        let (y0, count) = match band {
            Some(band) => {
                cfa.extend(
                    band.cfa
                        .chunks(2)
                        .map(|p| u32::from(p[0]) | (u32::from(*p.get(1).unwrap_or(&0)) << 16)),
                );
                for line in band.occluded.chunks(file::bit_row(w)) {
                    occ.extend(line.chunks(4).map(|bytes| {
                        bytes.iter().enumerate().fold(0u32, |acc, (k, &b)| {
                            acc | (u32::from(b.reverse_bits()) << (8 * k))
                        })
                    }));
                }
                (band.y0, band.cfa.len() / w)
            }
            None => (0, 0),
        };
        frames.extend([
            m.frames[i].white.to_bits(),
            cfa_offset as u32,
            occ_offset as u32,
            y0 as u32,
            count as u32,
            u32::from(band.is_some()),
            0,
            0,
        ]);
    }
    (frames, cfa, occ)
}

/// The kernel with its per-invocation sample arrays sized for `frames` (two green sites
/// per frame), so few frames do not pay for many in GPU occupancy.
fn kernel_source(frames: usize) -> String {
    let samples = (2 * frames).next_multiple_of(32).max(64);
    include_str!("stack.wgsl").replace(
        "const MAX_SAMPLES: u32 = 640u;",
        &format!("const MAX_SAMPLES: u32 = {samples}u;"),
    )
}

/// Appends a band's results to `out`.
fn unpack(values: &[f32], pixels: usize, out: &mut SkyStack) -> Result<()> {
    if values.len() < pixels * 3 * OUTPUTS {
        return Err(error("GPU stack: short readback"));
    }
    let mean = |v: f32| if v == NONE { f32::NAN } else { v };
    for p in 0..pixels {
        let at = |c: usize, k: usize| values[(p * 3 + c) * OUTPUTS + k];
        if (0..3).any(|c| at(c, 3) < 0.0) {
            return Err(error(
                "a pixel has more samples than the GPU stack holds; use the CPU stack",
            ));
        }
        out.rgb.push(std::array::from_fn(|c| mean(at(c, 0))));
        out.halves[0].push(std::array::from_fn(|c| mean(at(c, 1))));
        out.halves[1].push(std::array::from_fn(|c| mean(at(c, 2))));
        out.kept
            .push(std::array::from_fn(|c| at(c, 3).floor() as u32));
        out.clipped.push((0..3).any(|c| at(c, 3).fract() > 0.25));
    }
    Ok(())
}

pub fn stack(gpu: &Gpu, inputs: &Inputs, budget: &Budget) -> Result<SkyStack> {
    let m = &inputs.store.manifest;
    let (w, h, n) = (m.width, m.height, m.frames.len());
    if inputs.frames.len() != n
        || m.cfa_width != 2
        || m.cfa_height != 2
        || m.cfa.iter().any(|&c| c > 2)
    {
        return Err(error(
            "the GPU stack needs a 2×2 RGB pattern and a frame selection per frame",
        ));
    }
    if w % 2 == 1 {
        return Err(error(
            "the GPU stack packs CFA rows in pairs and needs an even width",
        ));
    }
    let words_per_row = w.div_ceil(32);
    let rotations: Vec<Matrix> = (0..n).map(|i| inputs.model.rotation(i)).collect();
    let floats = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<u32>>();
    let alpha = gpu.storage("alpha", words(&floats(inputs.alpha)))?;
    let clearance = gpu.storage("clearance", words(&floats(inputs.clearance)))?;
    let flat = gpu.storage("flat", words(&floats(inputs.flat)))?;
    let glow: Vec<u32> = inputs
        .glow
        .iter()
        .flat_map(|g| {
            g.rgb.iter().flat_map(|c| {
                let ok = c.iter().all(|v| v.is_finite());
                [c[0], c[1], c[2], f32::from(u8::from(ok))]
                    .map(|v| if ok { v.to_bits() } else { 0 })
            })
        })
        .collect();
    let glow = gpu.storage("glow", words(&glow))?;
    let source = kernel_source(n);
    let (pipeline, layout) = gpu.scoped("kernel build", || {
        let module = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("stack"),
                source: wgpu::ShaderSource::Wgsl(source.as_str().into()),
            });
        let pipeline = gpu
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("stack"),
                layout: None,
                module: &module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
        let layout = pipeline.get_bind_group_layout(0);
        (pipeline, layout)
    })?;
    let held = Budget::held(w * h);
    let room = budget.room("the GPU sky stack", held)?;
    let mut out = SkyStack {
        rgb: Vec::with_capacity(w * h),
        kept: Vec::with_capacity(w * h),
        halves: [Vec::with_capacity(w * h), Vec::with_capacity(w * h)],
        clipped: Vec::with_capacity(w * h),
    };
    let mut pending: Option<(super::Pending, usize)> = None;
    // Each band's rows are held twice (as read and packed for upload) beside two bands'
    // results in flight.
    let per_row = (2 * w * 3 * OUTPUTS * 4) as u64;
    let bands = bands(inputs, &rotations, BAND, per_row / 2, room / 2)?;
    budget
        .progress
        .step("Stacking the sky on the GPU", bands.len());
    for (b0, b1, bytes) in bands {
        budget.progress.advance()?;
        let pool = budget.pool("the GPU sky stack", held + 2 * bytes, 1)?;
        let rows: Vec<Option<Rows>> = pool.install(|| {
            (0..n)
                .into_par_iter()
                .map(|i| band_rows(inputs, &rotations[i], i, b0, b1))
                .collect::<Result<_>>()
        })?;
        let (frames, cfa, occ) = pack(inputs, &rotations, &rows);
        drop(rows);
        let pixels = (b1 - b0) * w;
        let submitted = gpu.scoped("stack dispatch", || -> Result<super::Pending> {
            let uniform = gpu.uniform("params", words(&params(inputs, b0, b1 - b0, words_per_row)));
            let buffers = [
                gpu.storage("frames", words(&frames))?,
                gpu.storage("cfa", words(&cfa))?,
                gpu.storage("occ", words(&occ))?,
            ];
            let result = gpu.output("result", (pixels * 3 * OUTPUTS * 4) as u64)?;
            let entries: Vec<wgpu::BindGroupEntry> = [
                &uniform,
                &buffers[0],
                &buffers[1],
                &buffers[2],
                &alpha,
                &clearance,
                &flat,
                &glow,
                &result,
            ]
            .iter()
            .enumerate()
            .map(|(k, b)| wgpu::BindGroupEntry {
                binding: k as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
            let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("stack"),
                layout: &layout,
                entries: &entries,
            });
            let mut encoder = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("stack"),
                });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("stack"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &group, &[]);
                pass.dispatch_workgroups((pixels as u32).div_ceil(WORKGROUP), 3, 1);
            }
            Ok(gpu.submit_read(encoder, &result, pixels * 3 * OUTPUTS))
        })??;
        if let Some((previous, count)) = pending.replace((submitted, pixels)) {
            unpack(&previous.wait(gpu)?, count, &mut out)?;
        }
    }
    if let Some((last, count)) = pending {
        unpack(&last.wait(gpu)?, count, &mut out)?;
    }
    Ok(out)
}
