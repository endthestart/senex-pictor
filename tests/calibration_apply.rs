mod support;

use pictor_engine::Result;
use pictor_engine::calibration::{
    Calibrated, FlatField, Master, Role, build_flat, build_master, calibrate,
};
use support::*;

struct Setup {
    sensor: Sensor,
    dark: Master,
    flat: FlatField,
}

fn setup(tint: [f64; 3]) -> Result<Setup> {
    let sensor = Sensor::new();
    let dark = build_master(
        Role::Dark,
        &frames(&sensor, &dark_scene(20.0), 16, 101)?,
        CLIPPED,
    )?;
    let flat_dark = build_master(
        Role::FlatDark,
        &frames(&sensor, &dark_scene(0.5), 16, 102)?,
        CLIPPED,
    )?;
    let flat = build_flat(
        &frames(&sensor, &flat_scene(tint), 16, 103)?,
        Some(&flat_dark),
        CLIPPED,
        0.2,
    )?;
    Ok(Setup { sensor, dark, flat })
}

/// Expected calibrated ADU per site: gained scene electrons at the flat normalization.
fn truth(sensor: &Sensor, scene: &Scene) -> Vec<f64> {
    let median = sensor.response_median();
    (0..W * H)
        .map(|s| {
            let c = color(s % W, s / W);
            GAIN[c] * scene.exposure * scene.light[s] * median[c]
        })
        .collect()
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

/// Median of `values` over sites of color `c` inside the box `[x0, x1) × [y0, y1)`.
fn region(values: &[f64], c: usize, (x0, x1, y0, y1): (usize, usize, usize, usize)) -> f64 {
    median(
        (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .filter(|&(x, y)| color(x, y) == c)
            .map(|(x, y)| values[site(x, y)])
            .collect(),
    )
}

fn calibrated_values(c: &Calibrated) -> Vec<f64> {
    c.data.iter().map(|v| f64::from(*v)).collect()
}

#[test]
fn calibration_recovers_flat_background_star_flux_and_hot_sites() -> Result<()> {
    let s = setup([0.6, 1.0, 0.8])?;
    let scene = light_scene(BACKGROUND);
    let light = mosaic(expose(&s.sensor, &scene, &mut Rng::new(104)), &scene)?;
    let result = calibrate(&light, Some(&s.dark), Some(&s.flat))?;
    let values = calibrated_values(&result);
    let raw: Vec<f64> = light
        .data
        .iter()
        .map(|v| f64::from(*v) - f64::from(BLACK))
        .collect();
    let (center, corner) = ((24, 40, 18, 30), (0, 10, 0, 8));
    for c in 0..3 {
        let ratio = region(&values, c, corner) / region(&values, c, center);
        assert!(
            (ratio - 1.0).abs() < 0.04,
            "color {c}: calibrated corner/center {ratio}"
        );
        let uncalibrated = region(&raw, c, corner) / region(&raw, c, center);
        assert!(
            uncalibrated < 0.85,
            "color {c}: vignetting absent ({uncalibrated}); test cannot fail"
        );
    }
    let expected = truth(&s.sensor, &scene);
    let background = truth(
        &s.sensor,
        &Scene {
            light: sky(BACKGROUND, &[]),
            ..light_scene(BACKGROUND)
        },
    );
    for &(sx, sy, _) in &STARS {
        let (x0, y0) = (sx.round() as usize - 4, sy.round() as usize - 4);
        let aperture: Vec<usize> = (y0..y0 + 9)
            .flat_map(|y| (x0..x0 + 9).map(move |x| site(x, y)))
            .collect();
        let measured: f64 = aperture.iter().map(|&i| values[i] - background[i]).sum();
        let wanted: f64 = aperture.iter().map(|&i| expected[i] - background[i]).sum();
        assert!(
            (measured / wanted - 1.0).abs() < 0.03,
            "star at {sx},{sy}: {measured} vs {wanted}"
        );
    }
    for (x, y) in HOT {
        let i = site(x, y);
        assert!(
            (values[i] - expected[i]).abs() < 230.0,
            "hot site {x},{y} not removed"
        );
    }
    assert!(result.valid.iter().all(|v| *v));
    assert_eq!(result.report.pedestal, "frame black and master dark");
    assert!(result.report.uncorrected.is_empty());
    Ok(())
}

#[test]
fn flat_source_color_does_not_change_calibrated_color() -> Result<()> {
    let scene = light_scene(BACKGROUND);
    let warm = setup([1.0, 0.5, 0.3])?;
    let cool = setup([0.3, 0.6, 1.0])?;
    let light = mosaic(expose(&warm.sensor, &scene, &mut Rng::new(105)), &scene)?;
    let a = calibrated_values(&calibrate(&light, Some(&warm.dark), Some(&warm.flat))?);
    let b = calibrated_values(&calibrate(&light, Some(&cool.dark), Some(&cool.flat))?);
    let whole = (0, W, 0, H);
    for c in 0..3 {
        let ratio = region(&a, c, whole) / region(&b, c, whole);
        assert!(
            (ratio - 1.0).abs() < 0.005,
            "color {c}: flat tint changed level by {ratio}"
        );
    }
    Ok(())
}

#[test]
fn dark_subtraction_preserves_negative_noise_without_bias() -> Result<()> {
    let s = setup([0.6, 1.0, 0.8])?;
    let scene = Scene {
        exposure: 20.0,
        iso: 1600,
        light: sky([0.02, 0.02, 0.02], &[]),
    };
    let mut rng = Rng::new(106);
    let lights: Vec<_> = (0..8)
        .map(|_| mosaic(expose(&s.sensor, &scene, &mut rng), &scene))
        .collect::<Result<_>>()?;
    let expected = truth(&s.sensor, &scene);
    let mut total = 0.0;
    let mut negative = 0usize;
    for light in &lights {
        let result = calibrate(light, Some(&s.dark), Some(&s.flat))?;
        for (i, v) in result.data.iter().enumerate() {
            total += f64::from(*v) - expected[i];
            negative += usize::from(*v < 0.0);
        }
    }
    let samples = (lights.len() * W * H) as f64;
    assert!(
        negative as f64 > 0.2 * samples,
        "negative values clipped: {negative}"
    );
    let bias_adu = total / samples;
    assert!(bias_adu.abs() < 0.15, "mean calibrated bias {bias_adu} ADU");
    Ok(())
}

#[test]
fn without_masters_frame_black_is_used_and_disclosed() -> Result<()> {
    let s = setup([0.6, 1.0, 0.8])?;
    let scene = light_scene(BACKGROUND);
    let light = mosaic(expose(&s.sensor, &scene, &mut Rng::new(107)), &scene)?;
    let result = calibrate(&light, None, None)?;
    assert_eq!(result.report.pedestal, "frame black");
    assert_eq!(result.report.uncorrected.len(), 2);
    let i = site(HOT[0].0, HOT[0].1);
    let expected = f64::from(light.data[i]) - f64::from(BLACK);
    assert!(
        (f64::from(result.data[i]) - expected).abs() < 1e-6,
        "hot site altered without a dark"
    );
    Ok(())
}

#[test]
fn saturation_is_flagged_from_stored_values() -> Result<()> {
    let s = setup([0.6, 1.0, 0.8])?;
    let scene = Scene {
        exposure: 20.0,
        iso: 1600,
        light: sky(BACKGROUND, &[(32.0, 24.0, 400_000.0)]),
    };
    let light = mosaic(expose(&s.sensor, &scene, &mut Rng::new(108)), &scene)?;
    let result = calibrate(&light, Some(&s.dark), Some(&s.flat))?;
    let clipped: Vec<usize> = (0..W * H).filter(|&i| light.data[i] >= WHITE).collect();
    assert!(
        !clipped.is_empty(),
        "no stored value reached white; test cannot fail"
    );
    assert_eq!(result.report.saturated_sites, clipped.len());
    assert!(
        clipped
            .iter()
            .all(|&i| result.saturated[i] && result.data[i].is_finite())
    );
    Ok(())
}

#[test]
fn the_plane_above_black_matches_the_black_level_site_by_site() -> Result<()> {
    let sensor = Sensor::new();
    let scene = dark_scene(1.0);
    let mut rng = Rng::new(9);
    let mut mosaic = mosaic(expose(&sensor, &scene, &mut rng), &scene)?;
    mosaic.black.repeat_rows = 2;
    mosaic.black.repeat_cols = 2;
    mosaic.black.values = vec![600.25, 601.5, 599.75, 602.0];
    mosaic.black.delta_h = (0..mosaic.width).map(|x| (x % 7) as f32 * 0.37).collect();
    mosaic.black.delta_v = (0..mosaic.height).map(|y| (y % 5) as f32 * -0.21).collect();
    let crop = (3, 5, mosaic.width - 7, mosaic.height - 9);
    let plane = mosaic.plane_above_black(crop);
    let (cx, cy, cw, ch) = crop;
    for y in 0..ch {
        for x in 0..cw {
            let (ax, ay) = (cx + x, cy + y);
            let want = mosaic.data[ay * mosaic.width + ax] - mosaic.black.at(ax, ay, 0, 1);
            assert_eq!(
                plane[y * cw + x].to_bits(),
                want.to_bits(),
                "site ({ax}, {ay})"
            );
        }
    }
    let colours = mosaic.colours(crop);
    for y in 0..ch {
        let row = &colours[(cy + y) % colours.len()];
        for (x, &got) in row.iter().enumerate() {
            assert_eq!(
                got,
                mosaic.cfa.color_at(cx + x, cy + y),
                "colour at ({x}, {y})"
            );
        }
    }
    Ok(())
}
