use pictor_engine::Result;
use pictor_engine::budget::Budget;
use pictor_engine::flow::rotation::{self, Turn};
use pictor_engine::integrate::ground::{self, Combine, Source};
use pictor_engine::sky::Camera;
use pictor_engine::store::{FrameInfo, Manifest, Store, file};

const W: usize = 96;
const H: usize = 64;

/// A tree edge: dark foreground left of `edge`, bright sky right of it.
fn scene(edge: f64) -> Vec<f32> {
    (0..W * H)
        .flat_map(|p| {
            let x = (p % W) as f64;
            let v = if x < edge { 20.0 } else { 220.0 };
            [v as f32; 3]
        })
        .collect()
}

fn store(frames: &[Vec<f32>]) -> Result<Store> {
    let dir = std::env::temp_dir().join(format!("pictor-ground-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(dir.join("rgb"))?;
    for (i, rgb) in frames.iter().enumerate() {
        file::create(
            &dir.join("rgb").join(format!("{i:05}.rgb")),
            &file::f16_bytes(rgb.iter().copied()),
        )?;
    }
    let info = |i: usize| FrameInfo {
        name: format!("F{i}.NEF"),
        sha256: String::new(),
        time_s: 30.0 * i as f64,
        exposure_s: Some(25.0),
        iso: Some(3200),
        aperture: Some(2.8),
        focal_mm: Some(20.0),
        lens: None,
        wb: [2.0, 1.0, 1.5],
        scale_adu: 15_000.0,
        black: vec![0.0; 4],
        white: 15_000.0,
        saturated_pixels: 0,
    };
    Ok(Store {
        dir,
        manifest: Manifest {
            decoder: "test".into(),
            make: None,
            model: None,
            width: W,
            height: H,
            crop: lcc_raw::Rect::new(0, 0, W, H),
            cfa_width: 2,
            cfa_height: 2,
            cfa: vec![0, 1, 1, 2],
            frames: (0..frames.len()).map(info).collect(),
        },
    })
}

/// A long lens (nearly no perspective over the frame), no distortion.
const CAMERA: Camera = Camera {
    width: W,
    height: H,
    f: 1e6,
    k1: 0.0,
    k2: 0.0,
};

/// The turn fitted to corners that all moved `dx` px to the right.
fn shift(dx: f64) -> Result<Turn> {
    let from: Vec<[f64; 2]> = (0..20)
        .map(|k| [5.0 + 4.0 * k as f64, 10.0 + 2.0 * k as f64])
        .collect();
    let to: Vec<[f64; 2]> = from.iter().map(|p| [p[0] + dx, p[1]]).collect();
    rotation::fit(&CAMERA, &from, &to).ok_or_else(|| pictor_engine::error("no turn"))
}

/// Pixels of the middle row more than 10 % of the step away from either level.
fn blurred(ground: &[[f32; 3]]) -> usize {
    (0..W)
        .map(|x| ground[(H / 2) * W + x][1])
        .filter(|v| (*v - 20.0).abs() > 20.0 && (*v - 220.0).abs() > 20.0)
        .count()
}

#[test]
fn drifting_frames_are_moved_back_so_the_edge_stays_sharp() -> Result<()> {
    let drift: Vec<f64> = (0..9).map(|i| 2.0 * i as f64).collect();
    let frames: Vec<Vec<f32>> = drift.iter().map(|d| scene(30.0 + d)).collect();
    let store = store(&frames)?;
    let flat = vec![1.0; W * H];
    let budget = Budget::of(1.0)?;
    let fields: Vec<Turn> = drift.iter().map(|&d| shift(d)).collect::<Result<_>>()?;
    let as_shot: Vec<Source> = (0..frames.len())
        .map(|frame| Source { frame, moved: None })
        .collect();
    let moved: Vec<Source> = fields
        .iter()
        .enumerate()
        .map(|(frame, d)| Source {
            frame,
            moved: Some(d),
        })
        .collect();
    let (smeared, _, _) = ground::stack(&store, &flat, &CAMERA, &as_shot, Combine::Mean, &budget)?;
    let (sharp, _, _) = ground::stack(&store, &flat, &CAMERA, &moved, Combine::Mean, &budget)?;
    std::fs::remove_dir_all(&store.dir)?;
    assert!(
        blurred(&smeared) >= 15,
        "the unmoved stack should smear the edge"
    );
    assert_eq!(
        blurred(&sharp),
        0,
        "moved frames should keep the edge sharp"
    );
    let edge = (0..W).position(|x| sharp[(H / 2) * W + x][1] > 120.0);
    assert_eq!(
        edge,
        Some(30),
        "the edge should stay where the reference saw it"
    );
    Ok(())
}
