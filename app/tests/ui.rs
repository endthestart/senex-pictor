use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pictor::app::{Pictor, Start};
use pictor::shell::Step;
use pictor_engine::{Result, error};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const SIZE: [f32; 2] = [1440.0, 900.0];

fn harness(open: Option<PathBuf>, demo: Option<[u32; 2]>) -> Harness<'static, Pictor> {
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
                    menu: false,
                },
            )
        })
}

async fn test_device() -> wgpu::Device {
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
fn wait(
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

fn rgb(image: &image::RgbaImage, x: f32, y: f32) -> [u8; 3] {
    let p = image.get_pixel(x as u32, y as u32);
    [p[0], p[1], p[2]]
}

/// Saves `image` as `name` in `PICTOR_UI_SHOTS`, when it is set.
fn save(image: &image::RgbaImage, name: &str) -> Result<()> {
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

#[test]
fn the_first_screen_offers_to_open_a_folder() -> Result<()> {
    let mut h = harness(None, None);
    h.step();
    let shot = h.render().map_err(error)?;
    save(&shot, "welcome.jpg")?;
    assert!(h.query_by_label("Open a folder…").is_some());
    assert!(!h.state().opened());
    Ok(())
}

#[test]
fn space_swaps_the_two_renderings_on_the_compare_screen() -> Result<()> {
    let mut h = harness(None, Some([1200, 800]));
    wait(&mut h, 120, Pictor::comparing)?;
    let before = h.render().map_err(error)?;
    save(&before, "compare.jpg")?;
    h.key_press(egui::Key::Space);
    h.step();
    let after = h.render().map_err(error)?;
    save(&after, "compare-swapped.jpg")?;
    let centre = (SIZE[0] * 0.4, SIZE[1] * 0.3);
    assert_ne!(
        rgb(&before, centre.0, centre.1),
        rgb(&after, centre.0, centre.1),
        "the sky should change when Space swaps natural for dark"
    );
    Ok(())
}

/// Every step of a processed folder draws (`PICTOR_UI_FOLDER`; skipped without it), and
/// the overlay key changes the Sky & ground canvas.
#[test]
fn every_step_of_a_processed_folder_draws() -> Result<()> {
    let Ok(folder) = std::env::var("PICTOR_UI_FOLDER") else {
        eprintln!("PICTOR_UI_FOLDER is not set; skipped");
        return Ok(());
    };
    let mut h = harness(Some(PathBuf::from(folder)), None);
    wait(&mut h, 300, Pictor::opened)?;
    for step in Step::ALL {
        h.state_mut().show_step(step);
        wait(&mut h, 60, Pictor::idle)?;
        let shot = h.render().map_err(error)?;
        save(&shot, &format!("step-{step:?}.jpg").to_lowercase())?;
        if step == Step::Sky {
            h.key_press(egui::Key::O);
            h.step();
            let plain = h.render().map_err(error)?;
            save(&plain, "step-sky-plain.jpg")?;
            assert_ne!(shot.as_raw(), plain.as_raw(), "O should toggle the overlay");
        }
    }
    Ok(())
}

/// A folder of RAW files opens on Import and is checked in the background
/// (`PICTOR_UI_RAW`; skipped without it).
#[test]
fn a_raw_folder_is_checked_before_anything_is_processed() -> Result<()> {
    let Ok(folder) = std::env::var("PICTOR_UI_RAW") else {
        eprintln!("PICTOR_UI_RAW is not set; skipped");
        return Ok(());
    };
    let mut h = harness(Some(PathBuf::from(folder)), None);
    h.step();
    assert!(!h.state().idle(), "the check should run in the background");
    let checking = h.render().map_err(error)?;
    save(&checking, "import-checking.jpg")?;
    wait(&mut h, 600, Pictor::idle)?;
    assert!(
        !h.state().opened(),
        "nothing is processed or opened before Start"
    );
    let ready = h.render().map_err(error)?;
    save(&ready, "import.jpg")?;
    Ok(())
}

/// A stopped run opens on Import with Resume; resuming shows the run's progress and Cancel
/// stops it again (`PICTOR_UI_STOPPED`, an output folder whose run stopped; skipped
/// without it).
#[test]
fn a_stopped_run_resumes_and_cancels_from_import() -> Result<()> {
    let Ok(folder) = std::env::var("PICTOR_UI_STOPPED") else {
        eprintln!("PICTOR_UI_STOPPED is not set; skipped");
        return Ok(());
    };
    let mut h = harness(Some(PathBuf::from(folder)), None);
    wait(&mut h, 60, Pictor::idle)?;
    let stopped = h.render().map_err(error)?;
    save(&stopped, "import-stopped.jpg")?;
    h.get_by_label("Resume").click();
    h.step();
    assert!(!h.state().idle(), "Resume should start the run");
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(8) {
        h.step();
        std::thread::sleep(Duration::from_millis(100));
    }
    let running = h.render().map_err(error)?;
    save(&running, "import-running.jpg")?;
    h.get_by_label("Cancel").click();
    wait(&mut h, 300, Pictor::idle)?;
    assert!(!h.state().opened(), "a cancelled run opens nothing");
    let cancelled = h.render().map_err(error)?;
    save(&cancelled, "import-cancelled.jpg")?;
    Ok(())
}

/// From the Scene step a hero composite is made in the background and opens in Compare
/// (`PICTOR_UI_FOLDER`; skipped without it; adds the hero result to that folder).
#[test]
fn a_hero_frame_is_made_from_the_scene_step() -> Result<()> {
    let Ok(folder) = std::env::var("PICTOR_UI_FOLDER") else {
        eprintln!("PICTOR_UI_FOLDER is not set; skipped");
        return Ok(());
    };
    let mut h = harness(Some(PathBuf::from(folder)), None);
    wait(&mut h, 300, Pictor::opened)?;
    h.state_mut().show_step(Step::Scene);
    wait(&mut h, 60, Pictor::idle)?;
    let before = h.render().map_err(error)?;
    save(&before, "scene.jpg")?;
    let made = h.query_all_by_label("Open in Compare").next().is_some();
    if made {
        h.get_by_label("Open in Compare").click();
    } else {
        h.get_all_by_label("Make it")
            .next()
            .ok_or_else(|| error("no Make it"))?
            .click();
        h.step();
        let making = h.render().map_err(error)?;
        save(&making, "scene-making.jpg")?;
    }
    wait(&mut h, 600, |a| a.idle() && a.comparing())?;
    let opened = h.render().map_err(error)?;
    save(&opened, "scene-hero-compare.jpg")?;
    Ok(())
}

/// From the Scene step a short-window moment is made and opens in Compare
/// (`PICTOR_UI_STACKED`, a processed folder with a whole-sequence stack; skipped without
/// it; adds the moment to that folder).
#[test]
fn a_moment_is_made_from_the_scene_step() -> Result<()> {
    let Ok(folder) = std::env::var("PICTOR_UI_STACKED") else {
        eprintln!("PICTOR_UI_STACKED is not set; skipped");
        return Ok(());
    };
    let mut h = harness(Some(PathBuf::from(folder)), None);
    wait(&mut h, 300, Pictor::opened)?;
    h.state_mut().show_step(Step::Scene);
    wait(&mut h, 60, Pictor::idle)?;
    let buttons = h.query_all_by_label("Make it").count();
    assert!(
        buttons >= 1,
        "the moment should be offered for a stacked run"
    );
    h.get_all_by_label("Make it")
        .last()
        .ok_or_else(|| error("no Make it"))?
        .click();
    h.step();
    assert!(!h.state().idle(), "Make it should start the moment");
    wait(&mut h, 900, |a| a.idle() && a.comparing())?;
    let opened = h.render().map_err(error)?;
    save(&opened, "scene-moment-compare.jpg")?;
    Ok(())
}

/// A drag with the primary button from `from` to `to`, a frame per tenth of the way.
fn drag(h: &mut Harness<'static, Pictor>, from: [f32; 2], to: [f32; 2]) {
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

/// Ground painted on Sky & ground is applied in the background and the run reopens there with
/// the stroke saved (`PICTOR_UI_CORRECT`, a processed folder the test changes; skipped
/// without it).
#[test]
fn a_painted_correction_is_applied_and_the_run_reopens_on_sky() -> Result<()> {
    let Ok(folder) = std::env::var("PICTOR_UI_CORRECT") else {
        eprintln!("PICTOR_UI_CORRECT is not set; skipped");
        return Ok(());
    };
    let mut h = harness(Some(PathBuf::from(folder)), None);
    wait(&mut h, 300, Pictor::opened)?;
    h.state_mut().show_step(Step::Sky);
    wait(&mut h, 60, Pictor::idle)?;
    let saved = |p: &Pictor| p.session().map(|s| s.sky.brush.saved.len());
    let before = saved(h.state()).ok_or("the folder should open")?;
    h.key_press(egui::Key::G);
    h.step();
    drag(&mut h, [500.0, 300.0], [800.0, 340.0]);
    let strokes = h.state().session().map_or(0, |s| s.sky.brush.strokes.len());
    assert_eq!(strokes, before + 1, "the drag should paint one stroke");
    save(&h.render().map_err(error)?, "sky-painted.jpg")?;
    h.get_by_label("Apply corrections").click();
    h.step();
    assert!(!h.state().idle(), "applying runs in the background");
    wait(&mut h, 1800, |p| p.idle() && saved(p) == Some(before + 1))?;
    assert_eq!(
        h.state().step(),
        Step::Sky,
        "the run reopens on Sky & ground"
    );
    save(&h.render().map_err(error)?, "sky-corrected.jpg")?;
    Ok(())
}

/// ⌘, opens Settings over the first screen and Cancel closes it without saving.
#[test]
fn settings_open_with_command_comma_and_cancel_closes_them() -> Result<()> {
    let mut h = harness(None, None);
    h.step();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Comma);
    h.step();
    h.step();
    save(&h.render().map_err(error)?, "settings.jpg")?;
    assert!(h.query_by_label("Save").is_some(), "Settings should open");
    h.get_by_label("Cancel").click();
    h.step();
    h.step();
    assert!(
        h.query_by_label("Save").is_none(),
        "Cancel should close Settings"
    );
    Ok(())
}
