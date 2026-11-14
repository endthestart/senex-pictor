//! The headless app harness and helpers the UI test files share; each file uses some of them.
#![allow(dead_code)]

use egui_kittest::Harness;
use pictor::app::{Pictor, Start};
use pictor_engine::{Result, error};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const SIZE: [f32; 2] = [1440.0, 900.0];

pub fn harness(open: Option<PathBuf>, demo: Option<[u32; 2]>) -> Harness<'static, Pictor> {
    Harness::builder()
        .with_size(SIZE)
        .with_pixels_per_point(1.0)
        .wgpu()
        .build_eframe(move |cc| {
            let device = cc.wgpu_render_state.as_ref().map(|r| r.device.clone());
            Pictor::new(
                cc,
                Start {
                    open,
                    demo,
                    step: None,
                    mp: None,
                    gpu_name: "test".into(),
                    device: device
                        .unwrap_or_else(|| pollster::block_on(async { test_device().await })),
                    bench_seconds: None,
                    screenshot: None,
                    desktop: false,
                },
            )
        })
}

pub async fn test_device() -> wgpu::Device {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance.request_adapter(&Default::default()).await;
    match adapter {
        Ok(a) => match a.request_device(&Default::default()).await {
            Ok((d, _)) => d,
            Err(e) => unreachable!("no device: {e}"),
        },
        Err(e) => unreachable!("no adapter: {e}"),
    }
}

/// Steps the harness until `done` holds or `seconds` pass.
pub fn wait(
    h: &mut Harness<'static, Pictor>,
    seconds: u64,
    done: impl Fn(&Pictor) -> bool,
) -> Result<()> {
    let start = Instant::now();
    while !done(h.state()) {
        if start.elapsed() > Duration::from_secs(seconds) {
            return Err(error("timed out waiting for the app"));
        }
        h.step();
        std::thread::sleep(Duration::from_millis(50));
    }
    for _ in 0..3 {
        h.step();
    }
    Ok(())
}

pub fn rgb(image: &image::RgbaImage, x: f32, y: f32) -> [u8; 3] {
    let p = image.get_pixel(x as u32, y as u32);
    [p[0], p[1], p[2]]
}

/// Saves `image` as `name` in `PICTOR_UI_SHOTS`, when it is set.
pub fn save(image: &image::RgbaImage, name: &str) -> Result<()> {
    let Ok(dir) = std::env::var("PICTOR_UI_SHOTS") else {
        return Ok(());
    };
    let (w, h) = image.dimensions();
    let display: Vec<[f32; 3]> = image
        .pixels()
        .map(|p| [p[0], p[1], p[2]].map(|v| f32::from(v) / 255.0))
        .collect();
    pictor_engine::export::jpeg(
        &std::path::Path::new(&dir).join(name),
        &display,
        w as usize,
        h as usize,
        90,
    )
}

/// A copy of a test folder beside it, removed when dropped, so a test can change the copy and
/// leave the folder as it was. On APFS the files are clones and take no space.
pub struct Scratch(pub PathBuf);

impl Scratch {
    /// Copies `folder`, leaving out every entry whose name starts with one of `skip`.
    pub fn of(folder: &Path, skip: &[&str]) -> Result<Scratch> {
        let name = folder
            .file_name()
            .ok_or_else(|| error(format!("{} has no name", folder.display())))?;
        let copy = folder.with_file_name(format!(
            "{}-test-{}",
            name.to_string_lossy(),
            std::process::id()
        ));
        let scratch = Scratch(copy);
        copy_dir(folder, &scratch.0, skip)?;
        Ok(scratch)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_dir_all(&self.0) {
            eprintln!("could not remove {}: {e}", self.0.display());
        }
    }
}

fn copy_dir(from: &Path, to: &Path, skip: &[&str]) -> Result<()> {
    std::fs::create_dir(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if skip.iter().any(|s| name.to_string_lossy().starts_with(s)) {
            continue;
        }
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &to.join(&name), skip)?;
        } else {
            std::fs::copy(entry.path(), to.join(&name))?;
        }
    }
    Ok(())
}

/// A drag with the primary button from `from` to `to`, a frame per tenth of the way.
pub fn drag(h: &mut Harness<'static, Pictor>, from: [f32; 2], to: [f32; 2]) {
    let at = |t: f32| {
        egui::pos2(
            from[0] + t * (to[0] - from[0]),
            from[1] + t * (to[1] - from[1]),
        )
    };
    h.hover_at(at(0.0));
    h.step();
    h.drag_at(at(0.0));
    h.step();
    for k in 1..=10 {
        h.hover_at(at(k as f32 / 10.0));
        h.step();
    }
    h.drop_at(at(1.0));
    h.step();
}
