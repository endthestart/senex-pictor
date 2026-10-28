use egui_kittest::Harness;
use star_photos::{Result, error};
use star_photos_app::app::{StarPhotos, Start};
use std::time::{Duration, Instant};

const SIZE: [f32; 2] = [1400.0, 900.0];

fn harness() -> Harness<'static, StarPhotos> {
    Harness::builder()
        .with_size(SIZE)
        .with_pixels_per_point(1.0)
        .wgpu()
        .build_eframe(|cc| {
            let device = cc.wgpu_render_state.as_ref().map(|r| r.device.clone());
            StarPhotos::new(
                cc,
                Start {
                    folder: None,
                    project: None,
                    demo: Some([1200, 800]),
                    mp: None,
                    gpu_name: "test".into(),
                    device: device
                        .unwrap_or_else(|| pollster::block_on(async { test_device().await })),
                    bench_seconds: None,
                    screenshot: None,
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

fn rgb(image: &image::RgbaImage, x: f32, y: f32) -> [u8; 3] {
    let p = image.get_pixel(x as u32, y as u32);
    [p[0], p[1], p[2]]
}

fn save(image: &image::RgbaImage, name: &str) -> Result<()> {
    let Ok(dir) = std::env::var("STAR_PHOTOS_UI_SHOTS") else {
        return Ok(());
    };
    let (w, h) = image.dimensions();
    let display: Vec<[f32; 3]> = image
        .pixels()
        .map(|p| [p[0], p[1], p[2]].map(|v| f32::from(v) / 255.0))
        .collect();
    star_photos::export::jpeg(
        &std::path::Path::new(&dir).join(name),
        &display,
        w as usize,
        h as usize,
        90,
    )
}

#[test]
fn space_swaps_the_two_renderings_on_the_compare_screen() -> Result<()> {
    let mut h = harness();
    let start = Instant::now();
    while !h.state().comparing() {
        if start.elapsed() > Duration::from_secs(120) {
            return Err(error("the demo scene did not open"));
        }
        h.step();
        std::thread::sleep(Duration::from_millis(50));
    }
    for _ in 0..3 {
        h.step();
    }
    let before = h.render().map_err(error)?;
    save(&before, "compare.jpg")?;
    h.key_press(egui::Key::Space);
    h.step();
    let after = h.render().map_err(error)?;
    save(&after, "compare-swapped.jpg")?;
    let centre = (SIZE[0] * 0.45, SIZE[1] * 0.3);
    assert_ne!(
        rgb(&before, centre.0, centre.1),
        rgb(&after, centre.0, centre.1),
        "the sky should change when Space swaps natural for dark"
    );
    Ok(())
}
