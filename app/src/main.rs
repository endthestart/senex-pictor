//! Pictor (ADR 0011): egui on a wgpu device the app creates and shares with the window, so
//! rendered looks stay on the GPU. Opens a folder of RAW files, a `process` output folder or a
//! store; with a command (`pictor process …`, `pictor stack …`) it runs that tool instead.
//!
//! pictor [FOLDER] [--demo yes] [--scene FILE] [--step NAME] [--mp 45] [--gpu NAME] [--hdr no]
//! Developer runs: --bench-window SECONDS (scripted pan, zoom and swap; prints JSON),
//! --bench-offscreen WxH (GPU times without a window), --save-scene FILE, --screenshot FILE
//! (save the window once a result is up, then quit). --step opens on import, frames, sky,
//! scene, compare or look.

use eframe::egui_wgpu::{WgpuSetup, WgpuSetupExisting};
use pictor::project::{self, Kind, Loaded};
use pictor::shell::Step;
use pictor::{app, bench, gpu, scene};
use pictor_engine::{Result, command, error};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const USAGE: &str = "usage: pictor [FOLDER] [--demo yes] [--scene FILE] [--step NAME] [--mp 45] [--gpu NAME] [--hdr no] \
                     [--bench-window SECONDS | --bench-offscreen WxH | --save-scene FILE | --screenshot FILE]";

fn options(mut args: Vec<String>) -> Result<(Option<PathBuf>, HashMap<String, String>)> {
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

fn step(name: &str) -> Result<Step> {
    Step::ALL
        .into_iter()
        .find(|s| format!("{s:?}").eq_ignore_ascii_case(name))
        .ok_or_else(|| {
            error(format!(
                "--step {name}: import, frames, sky, scene, compare or look"
            ))
        })
}

/// What a developer run opens without a window.
fn loaded(folder: Option<PathBuf>, scene_file: Option<&String>) -> Result<Loaded> {
    match (folder, scene_file) {
        (Some(f), _) => match project::classify(&f)? {
            Kind::Processed(out) => project::load_run(&out, None),
            Kind::Store(dir) => project::load_store(&dir),
            Kind::SceneFile(file) => project::load_scene_file(&file),
            Kind::Raw(_) | Kind::Interrupted(..) => {
                Err(error(format!("{} has no result yet", f.display())))
            }
        },
        (None, Some(file)) => project::load_scene_file(Path::new(file)),
        (None, None) => project::demo([6048, 4024]),
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
    let p = loaded(folder, o.get("--scene"))?;
    if let Some(file) = o.get("--save-scene") {
        scene::save(&p.scene, Path::new(file))?;
        eprintln!("pictor: scene saved to {file}");
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

fn run(args: Vec<String>) -> Result<()> {
    let (folder, o) = options(args)?;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = adapter(&instance, o.get("--gpu"))?;
    let info = adapter.get_info();
    let gpu_name = format!("{} ({:?})", info.name, info.backend);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("pictor"),
        required_limits: adapter.limits(),
        ..Default::default()
    }))?;
    if o.contains_key("--save-scene") || o.contains_key("--bench-offscreen") {
        return headless(&o, folder, &device, &queue, &gpu_name);
    }
    let mut wgpu_options = eframe::NativeOptions::default().wgpu_options;
    wgpu_options.wgpu_setup = WgpuSetup::Existing(WgpuSetupExisting {
        instance,
        adapter,
        device: device.clone(),
        queue,
    });
    wgpu_options.hdr_output = o.get("--hdr").is_none_or(|v| v != "no");
    let start = app::Start {
        open: folder.or_else(|| o.get("--scene").map(PathBuf::from)),
        step: o.get("--step").map(|s| step(s)).transpose()?,
        demo: o
            .get("--demo")
            .is_some_and(|v| v == "yes")
            .then_some([6048, 4024]),
        mp: o.get("--mp").map(|v| v.parse()).transpose()?,
        gpu_name,
        device,
        bench_seconds: o.get("--bench-window").map(|v| v.parse()).transpose()?,
        screenshot: o.get("--screenshot").map(PathBuf::from),
        desktop: true,
    };
    let icon = image::load_from_memory(include_bytes!("../assets/icon/pictor.png"))?.into_rgba8();
    let icon = egui::IconData {
        width: icon.width(),
        height: icon.height(),
        rgba: icon.into_raw(),
    };
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        wgpu_options,
        viewport: egui::ViewportBuilder::default()
            .with_title("Pictor")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1100.0, 700.0])
            .with_icon(std::sync::Arc::new(icon))
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "pictor",
        options,
        Box::new(move |cc| Ok(Box::new(app::Pictor::new(cc, start)) as Box<dyn eframe::App>)),
    )
    .map_err(|e| error(format!("window: {e}")))
}

fn main() -> std::process::ExitCode {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let result = if command::wanted(&args) {
        command::run(args)
    } else {
        args.into_iter()
            .map(|a| {
                a.into_string()
                    .map_err(|a| error(format!("{} is not UTF-8", a.to_string_lossy())))
            })
            .collect::<Result<Vec<_>>>()
            .and_then(run)
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("pictor: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
