//! Toolkit validation spike (docs: reports/Star Photos app research.md, "toolkit validation
//! spike"): one wgpu 30 device shared by the compute that renders two looks of a stack and by
//! eframe's window, a canvas of up to 45 MP drawn from GPU-resident textures, display output in
//! extended linear sRGB (P3 and EDR on macOS) through the patched egui-wgpu, and an instant A/B
//! swap.
//!
//! canvas-spike [--store STORE [--planes final] | --scene FILE] [--mp 45] [--gpu NAME]
//!              [--hdr yes|no] [--bench-window SECONDS | --bench-offscreen WxH | --save-scene FILE]
//! Without a store or scene file it shows a synthetic nightscape.

mod app;
mod bench;
mod canvas;
mod look;
mod mips;
mod source;
mod stats;
mod synthetic;

use eframe::egui_wgpu::{WgpuSetup, WgpuSetupExisting};
use star_photos::{Result, error};
use std::collections::HashMap;
use std::path::Path;

fn options() -> Result<HashMap<String, String>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = HashMap::new();
    for pair in args.chunks(2) {
        match pair {
            [name, value] if name.starts_with("--") => {
                out.insert(name.clone(), value.clone());
            }
            _ => {
                return Err(error(
                    "usage: canvas-spike [--store STORE [--planes final] | --scene FILE] [--mp 45] [--gpu NAME] [--hdr yes|no] [--bench-window SECONDS | --bench-offscreen WxH | --save-scene FILE]",
                ));
            }
        }
    }
    Ok(out)
}

/// The adapter whose name contains `name`, or the high-performance one.
fn adapter(instance: &wgpu::Instance, name: Option<&String>) -> Result<wgpu::Adapter> {
    match name {
        Some(wanted) => pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()))
            .into_iter()
            .find(|a| {
                a.get_info()
                    .name
                    .to_lowercase()
                    .contains(&wanted.to_lowercase())
            })
            .ok_or_else(|| error(format!("no GPU named like {wanted}"))),
        None => pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .map_err(|e| error(format!("no GPU adapter: {e}"))),
    }
}

/// The output size for `mp` megapixels at the scene's aspect ratio.
fn output_size(scene: &source::Scene, mp: Option<f64>) -> [u32; 2] {
    let (w, h) = (f64::from(scene.width), f64::from(scene.height));
    match mp {
        None => [scene.width, scene.height],
        Some(mp) => {
            let k = (mp * 1e6 / (w * h)).sqrt();
            [(w * k).round() as u32, (h * k).round() as u32]
        }
    }
}

fn run() -> Result<()> {
    let o = options()?;
    let planes = o.get("--planes").map_or("final", String::as_str);
    let mp = o.get("--mp").map(|v| v.parse::<f64>()).transpose()?;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = adapter(&instance, o.get("--gpu"))?;
    let info = adapter.get_info();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("star-photos"),
        required_limits: adapter.limits(),
        ..Default::default()
    }))?;
    let loaded = std::time::Instant::now();
    let scene = match (o.get("--store"), o.get("--scene")) {
        (Some(store), None) => source::load(Path::new(store), planes)?,
        (None, Some(file)) => source::open(Path::new(file))?,
        (None, None) => synthetic::scene(6048, 4024)?,
        _ => return Err(error("give --store or --scene, not both")),
    };
    if let Some(file) = o.get("--save-scene") {
        source::save(&scene, Path::new(file))?;
        eprintln!("spike: scene saved to {file}");
        return Ok(());
    }
    let size = output_size(&scene, mp);
    let looks = look::Looks::new(&device, &queue, &scene, size)?;
    let mut first = Vec::new();
    for (k, p) in scene.looks.iter().enumerate() {
        first.push(looks.apply(&device, &queue, k, p)?);
    }
    eprintln!(
        "spike: {} ({:?}); scene {}×{} loaded and both looks rendered at {}×{} in {:.1} s (looks {:.0} / {:.0} ms)",
        info.name,
        info.backend,
        scene.width,
        scene.height,
        size[0],
        size[1],
        loaded.elapsed().as_secs_f64(),
        first[0] * 1e3,
        first[1] * 1e3
    );
    if let Some(target) = o.get("--bench-offscreen") {
        let (w, h) = target
            .split_once('x')
            .ok_or_else(|| error("--bench-offscreen WxH"))?;
        let report = bench::run(
            &device,
            &queue,
            &looks,
            &scene.looks,
            size,
            [w.parse()?, h.parse()?],
        )?;
        println!(
            "{}",
            serde_json::json!({"gpu": info.name, "offscreen": report})
        );
        return Ok(());
    }
    let hdr = o.get("--hdr").is_none_or(|v| v == "yes");
    let mut wgpu_options = eframe::NativeOptions::default().wgpu_options;
    wgpu_options.wgpu_setup = WgpuSetup::Existing(WgpuSetupExisting {
        instance,
        adapter,
        device: device.clone(),
        queue: queue.clone(),
    });
    wgpu_options.hdr_output = hdr;
    let setup = app::Setup {
        looks,
        params: scene.looks,
        names: scene.names,
        device,
        queue,
        size,
        adapter: format!("{} ({:?})", info.name, info.backend),
        bench_seconds: o.get("--bench-window").map(|v| v.parse()).transpose()?,
    };
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        wgpu_options,
        viewport: egui::ViewportBuilder::default()
            .with_title("Star Photos canvas spike")
            .with_inner_size([1600.0, 1000.0]),
        ..Default::default()
    };
    eframe::run_native(
        "canvas-spike",
        options,
        Box::new(move |cc| Ok(Box::new(app::SpikeApp::new(cc, setup)?) as Box<dyn eframe::App>)),
    )
    .map_err(|e| error(format!("window: {e}")))
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("canvas-spike: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
