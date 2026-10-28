//! Star Photos desktop app (ADR 0011): egui on a wgpu device the app creates and shares with the
//! window, so rendered looks stay on the GPU. Opens a `process` output folder or a store.
//!
//! star-photos-app [FOLDER] [--demo yes] [--scene FILE] [--mp 45] [--gpu NAME] [--hdr no]
//! Developer runs: --bench-window SECONDS (scripted pan, zoom and swap; prints JSON),
//! --bench-offscreen WxH (GPU times without a window), --save-scene FILE, --screenshot FILE
//! (save the window once the comparison is up, then quit).

use eframe::egui_wgpu::{WgpuSetup, WgpuSetupExisting};
use star_photos::{Result, error};
use star_photos_app::{app, bench, gpu, project, scene};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const USAGE: &str = "usage: star-photos-app [FOLDER] [--demo yes] [--scene FILE] [--mp 45] [--gpu NAME] [--hdr no] \
                     [--bench-window SECONDS | --bench-offscreen WxH | --save-scene FILE | --screenshot FILE]";

fn options() -> Result<(Option<PathBuf>, HashMap<String, String>)> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let folder =
        (!args.is_empty() && !args[0].starts_with("--")).then(|| PathBuf::from(args.remove(0)));
    let mut out = HashMap::new();
    for pair in args.chunks(2) {
        match pair {
            [name, value] if name.starts_with("--") => {
                out.insert(name.clone(), value.clone());
            }
            _ => return Err(error(USAGE)),
        }
    }
    Ok((folder, out))
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

/// Developer runs without a window: save a scene file or time the GPU.
fn headless(
    o: &HashMap<String, String>,
    folder: Option<PathBuf>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    gpu: &str,
) -> Result<()> {
    let p = match (folder, o.get("--scene")) {
        (Some(f), _) => project::open(f).recv()??,
        (None, Some(file)) => project::from_file(Path::new(file))?,
        (None, None) => project::demo([6048, 4024]).recv()??,
    };
    if let Some(file) = o.get("--save-scene") {
        scene::save(&p.scene, Path::new(file))?;
        eprintln!("star-photos-app: scene saved to {file}");
        return Ok(());
    }
    let mp = o.get("--mp").map(|v| v.parse()).transpose()?;
    let size = scene::output_size(&p.scene, mp);
    let looks = gpu::look::Looks::new(device, queue, &p.scene, size)?;
    let target = o.get("--bench-offscreen").ok_or_else(|| error(USAGE))?;
    let (w, h) = target
        .split_once('x')
        .ok_or_else(|| error("--bench-offscreen WxH"))?;
    let report = bench::run(
        device,
        queue,
        &looks,
        &p.scene.looks,
        size,
        [w.parse()?, h.parse()?],
    )?;
    println!("{}", serde_json::json!({"gpu": gpu, "offscreen": report}));
    Ok(())
}

fn run() -> Result<()> {
    let (folder, o) = options()?;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = adapter(&instance, o.get("--gpu"))?;
    let info = adapter.get_info();
    let gpu_name = format!("{} ({:?})", info.name, info.backend);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("star-photos"),
        required_limits: adapter.limits(),
        ..Default::default()
    }))?;
    if o.contains_key("--save-scene") || o.contains_key("--bench-offscreen") {
        return headless(&o, folder, &device, &queue, &gpu_name);
    }
    let project = o
        .get("--scene")
        .map(|f| project::from_file(Path::new(f)))
        .transpose()?;
    let mut wgpu_options = eframe::NativeOptions::default().wgpu_options;
    wgpu_options.wgpu_setup = WgpuSetup::Existing(WgpuSetupExisting {
        instance,
        adapter,
        device: device.clone(),
        queue,
    });
    wgpu_options.hdr_output = o.get("--hdr").is_none_or(|v| v != "no");
    let start = app::Start {
        folder,
        project,
        demo: o
            .get("--demo")
            .is_some_and(|v| v == "yes")
            .then_some([6048, 4024]),
        mp: o.get("--mp").map(|v| v.parse()).transpose()?,
        gpu_name,
        device,
        bench_seconds: o.get("--bench-window").map(|v| v.parse()).transpose()?,
        screenshot: o.get("--screenshot").map(PathBuf::from),
    };
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        wgpu_options,
        viewport: egui::ViewportBuilder::default()
            .with_title("Star Photos")
            .with_inner_size([1600.0, 1000.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "star-photos",
        options,
        Box::new(move |cc| Ok(Box::new(app::StarPhotos::new(cc, start)) as Box<dyn eframe::App>)),
    )
    .map_err(|e| error(format!("window: {e}")))
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("star-photos-app: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
