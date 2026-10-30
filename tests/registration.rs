use pictor_engine::Result;
use pictor_engine::sky::Camera;
use pictor_engine::sky::drift::free_rotations;
use pictor_engine::sky::fit::{Start, fit};
use pictor_engine::sky::model::{SIDEREAL_RAD_PER_S, apply, rotation};
use pictor_engine::sky::report::held_out;
use pictor_engine::stars::similarity::Random;
use pictor_engine::stars::{Star, detect, track};

fn uniform(r: &mut Random) -> f64 {
    (r.next_u64() >> 11) as f64 / (1u64 << 53) as f64
}

fn gaussian(r: &mut Random) -> f64 {
    let (u, v) = (uniform(r).max(1e-300), uniform(r));
    (-2.0 * u.ln()).sqrt() * (2.0 * std::f64::consts::PI * v).cos()
}

/// A noisy frame with a sloped background and Gaussian stars of σ 1.2 px at `stars`.
fn frame(width: usize, height: usize, stars: &[(f64, f64, f64)], random: &mut Random) -> Vec<f32> {
    let mut image: Vec<f32> = (0..width * height)
        .map(|p| (1000.0 + 0.2 * (p % width) as f64 + 10.0 * gaussian(random)) as f32)
        .collect();
    for &(sx, sy, peak) in stars {
        for y in 0..height {
            for x in 0..width {
                let r2 = (x as f64 - sx).powi(2) + (y as f64 - sy).powi(2);
                if r2 < 100.0 {
                    image[y * width + x] += (peak * (-r2 / (2.0 * 1.44)).exp()) as f32;
                }
            }
        }
    }
    image
}

#[test]
fn stars_are_found_to_a_twentieth_of_a_pixel_and_blends_edges_and_streaks_are_dropped() -> Result<()>
{
    let (w, h) = (320, 256);
    let mut random = Random::new(3);
    let single: Vec<(f64, f64, f64)> = (0..12)
        .map(|i| {
            (
                30.3 + 24.0 * i as f64,
                40.0 + 15.7 * i as f64,
                2000.0 + 100.0 * i as f64,
            )
        })
        .collect();
    let mut all = single.clone();
    all.extend([
        (200.0, 220.0, 500.0),
        (204.0, 222.0, 450.0),
        (0.4, 120.0, 600.0),
    ]);
    let mut image = frame(w, h, &all, &mut random);
    for x in 40..70 {
        image[230 * w + x] += 300.0;
        image[231 * w + x] += 300.0;
    }
    let found = detect(&image, &vec![false; w * h], w, h)?;
    assert_eq!(found.len(), single.len(), "found {found:?}");
    for (x, y, _) in &single {
        let nearest = found
            .iter()
            .map(|s| (s.x - x).hypot(s.y - y))
            .fold(f64::INFINITY, f64::min);
        assert!(nearest < 0.05, "star at ({x}, {y}) found {nearest} px away");
    }
    Ok(())
}

struct Sky {
    camera: Camera,
    axis: [f64; 3],
    times: Vec<f64>,
    stars: Vec<Vec<Star>>,
}

/// Stars on a rotating sky seen by `camera` every 10 s, with 0.1 px centroid noise, plus
/// static lights; `drift` tilts the camera about its x axis by drift·i² at frame i (a
/// steady drift would be absorbed into a tilted fixed axis).
fn sky(frames: usize, drift: f64) -> Sky {
    let camera = Camera {
        width: 3000,
        height: 2000,
        f: 2400.0,
        k1: -0.05,
        k2: 0.015,
    };
    let axis = {
        let a: [f64; 3] = [0.1, -0.75, 0.6];
        let n = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
        a.map(|v| v / n)
    };
    let mut random = Random::new(11);
    let rays: Vec<([f64; 3], f64)> = (0..1500)
        .map(|_| {
            let p = [uniform(&mut random) * 3000.0, uniform(&mut random) * 1500.0];
            (camera.ray(p), 100.0 + 1e4 * uniform(&mut random).powi(4))
        })
        .collect();
    let lights: Vec<[f64; 2]> = (0..60)
        .map(|i| [40.0 + 48.0 * i as f64, 1700.0 + (i % 7) as f64 * 30.0])
        .collect();
    let times: Vec<f64> = (0..frames).map(|i| 10.0 * i as f64).collect();
    let stars = times
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let turn = rotation(axis, SIDEREAL_RAD_PER_S * t);
            let wobble = rotation([1.0, 0.0, 0.0], drift * (i * i) as f64);
            let mut list: Vec<Star> = rays
                .iter()
                .filter_map(|(v, flux)| {
                    let p = camera.pixel(apply(&wobble, apply(&turn, *v)))?;
                    let inside = p[0] > 5.0 && p[1] > 5.0 && p[0] < 2995.0 && p[1] < 1995.0;
                    inside.then(|| Star {
                        x: p[0] + 0.1 * gaussian(&mut random),
                        y: p[1] + 0.1 * gaussian(&mut random),
                        flux: *flux,
                        saturated: false,
                    })
                })
                .collect();
            list.extend(lights.iter().map(|l| Star {
                x: l[0],
                y: l[1],
                flux: 50.0,
                saturated: false,
            }));
            list
        })
        .collect();
    Sky {
        camera,
        axis,
        times,
        stars,
    }
}

#[test]
fn a_rotating_sky_is_registered_to_a_twentieth_of_a_pixel_despite_static_lights() -> Result<()> {
    let s = sky(31, 0.0);
    let tracks = track(&s.stars)?;
    let reference = 15;
    let subset: Vec<usize> = (0..31).step_by(10).filter(|&i| i != reference).collect();
    let start = Start {
        f: 2300.0,
        k1: -0.116,
        k2: 0.044,
        fix_distortion: false,
    };
    let model = fit(
        &s.stars,
        &tracks,
        &s.times,
        reference,
        &subset,
        (3000, 2000),
        start,
    )?;
    for (i, t) in s.times.iter().enumerate() {
        let turn = rotation(s.axis, SIDEREAL_RAD_PER_S * (t - s.times[reference]));
        for y in (100..1500).step_by(200) {
            for x in (100..2900).step_by(200) {
                let p = [x as f64, y as f64];
                let truth = s.camera.pixel(apply(&turn, s.camera.ray(p)));
                let (Some(a), Some(b)) = (model.predict(i, p), truth) else {
                    continue;
                };
                let off = (a[0] - b[0]).hypot(a[1] - b[1]);
                assert!(off < 0.05, "frame {i} at ({x}, {y}): {off} px off");
            }
        }
    }
    Ok(())
}

#[test]
fn drift_breaks_the_fixed_axis_and_free_rotations_follow_it() -> Result<()> {
    let s = sky(31, 3e-5);
    let tracks = track(&s.stars)?;
    let reference = 15;
    let subset: Vec<usize> = (0..31).step_by(10).filter(|&i| i != reference).collect();
    let start = Start {
        f: 2400.0,
        k1: -0.05,
        k2: 0.015,
        fix_distortion: true,
    };
    let fixed = fit(
        &s.stars,
        &tracks,
        &s.times,
        reference,
        &subset,
        (3000, 2000),
        start,
    )?;
    let centre = |m: &_, frame| -> Result<f64> {
        let zones = held_out(m, &s.stars, &tracks, frame, 1)?;
        zones[0]
            .median_px
            .ok_or_else(|| "no held-out stars in the centre".into())
    };
    assert!(
        centre(&fixed, 0)? > 1.5,
        "the fixed axis should miss a drifting camera"
    );
    let free = free_rotations(&fixed, &s.stars, &tracks, reference)?;
    for frame in [0, 30] {
        assert!(
            centre(&free, frame)? < 0.2,
            "free rotations should follow the drift"
        );
    }
    Ok(())
}
