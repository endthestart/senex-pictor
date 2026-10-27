mod support;

use star_photos::Result;
use star_photos::calibration::{Estimator, Mosaic, Role, build_flat, build_master, calibrate};
use support::*;

#[test]
fn low_flat_response_is_invalid_not_replaced() -> Result<()> {
    let sensor = Sensor::new();
    let flat_dark = build_master(
        Role::FlatDark,
        &frames(&sensor, &dark_scene(0.5), 8, 109)?,
        CLIPPED,
    )?;
    let mut flats = frames(&sensor, &flat_scene([0.6, 1.0, 0.8]), 8, 110)?;
    let dust: Vec<usize> = (10..20).map(|y| site(40, y)).collect();
    for flat in &mut flats {
        for &i in &dust {
            flat.data[i] = flat_dark.data[i] + 50.0;
        }
    }
    let flat = build_flat(&flats, Some(&flat_dark), CLIPPED, 0.2)?;
    assert_eq!(flat.summary.invalid_sites, dust.len());
    assert!(dust.iter().all(|&i| !flat.valid[i]));
    let dark = build_master(
        Role::Dark,
        &frames(&sensor, &dark_scene(20.0), 8, 111)?,
        CLIPPED,
    )?;
    let scene = light_scene(BACKGROUND);
    let light = mosaic(expose(&sensor, &scene, &mut Rng::new(112)), &scene)?;
    let result = calibrate(&light, Some(&dark), Some(&flat))?;
    assert_eq!(result.report.invalid_sites, dust.len());
    assert!(
        dust.iter()
            .all(|&i| !result.valid[i] && result.data[i].is_nan())
    );
    Ok(())
}

#[test]
fn incompatible_masters_are_rejected_and_differences_reported() -> Result<()> {
    let sensor = Sensor::new();
    let scene = light_scene(BACKGROUND);
    let light = mosaic(expose(&sensor, &scene, &mut Rng::new(113)), &scene)?;
    let dark = build_master(
        Role::Dark,
        &frames(&sensor, &dark_scene(20.0), 4, 114)?,
        Estimator::Mean,
    )?;
    let bias_frames = frames(&sensor, &dark_scene(1.0 / 8000.0), 4, 115)?;
    let bias = build_master(Role::Bias, &bias_frames, Estimator::Mean)?;
    assert!(
        calibrate(&light, Some(&bias), None).is_err(),
        "bias accepted as a light dark"
    );
    let short = build_master(
        Role::Dark,
        &frames(&sensor, &dark_scene(10.0), 4, 116)?,
        Estimator::Mean,
    )?;
    assert!(
        calibrate(&light, Some(&short), None).is_err(),
        "exposure mismatch accepted"
    );
    let mut other_iso = light.clone();
    other_iso.capture.iso = Some(800);
    assert!(
        calibrate(&other_iso, Some(&dark), None).is_err(),
        "ISO mismatch accepted"
    );
    let mut unknown_nr = light.clone();
    unknown_nr.capture.in_camera_nr = None;
    assert!(
        calibrate(&unknown_nr, Some(&dark), None).is_err(),
        "unverifiable NR accepted"
    );
    let flats = frames(&sensor, &flat_scene([0.6, 1.0, 0.8]), 4, 117)?;
    assert!(
        build_flat(&flats, Some(&dark), Estimator::Mean, 0.2).is_err(),
        "light dark used for flats"
    );
    let flat = build_flat(&flats, Some(&bias), Estimator::Mean, 0.2)?;
    assert!(
        flat.warnings
            .iter()
            .any(|w| w.subject == "flat dark signal")
    );
    let mut other_aperture = light.clone();
    other_aperture.capture.f_number = Some(2.8);
    assert!(
        calibrate(&other_aperture, Some(&dark), Some(&flat)).is_err(),
        "aperture mismatch accepted"
    );
    let result = calibrate(&light, Some(&dark), Some(&flat))?;
    assert!(result.report.warnings.iter().any(|w| w.subject == "focus"));
    Ok(())
}

#[test]
fn per_frame_black_drift_cancels() -> Result<()> {
    let sensor = Sensor::new();
    let mut darks = frames(&sensor, &dark_scene(20.0), 16, 120)?;
    for (index, dark) in darks.iter_mut().enumerate() {
        shift_pedestal(dark, if index % 2 == 0 { 0.6 } else { -0.6 });
    }
    let dark = build_master(Role::Dark, &darks, CLIPPED)?;
    let scene = Scene {
        exposure: 20.0,
        iso: 1600,
        light: sky([0.02, 0.02, 0.02], &[]),
    };
    let mut rng = Rng::new(121);
    let mut total = 0.0;
    for _ in 0..8 {
        let mut light = mosaic(expose(&sensor, &scene, &mut rng), &scene)?;
        shift_pedestal(&mut light, 3.0);
        let result = calibrate(&light, Some(&dark), None)?;
        total += result.data.iter().map(|v| f64::from(*v)).sum::<f64>();
    }
    let expected: f64 = (0..W * H)
        .map(|s| GAIN[color(s % W, s / W)] * scene.exposure * scene.light[s] * sensor.response[s])
        .sum();
    let bias_adu = (total - 8.0 * expected) / (8 * W * H) as f64;
    assert!(bias_adu.abs() < 0.2, "pedestal drift left {bias_adu} ADU");
    Ok(())
}

#[test]
fn implausible_black_levels_are_rejected() -> Result<()> {
    let sensor = Sensor::new();
    let mut darks = frames(&sensor, &dark_scene(20.0), 4, 122)?;
    let dark = build_master(Role::Dark, &darks, Estimator::Mean)?;
    let scene = light_scene(BACKGROUND);
    let mut light = mosaic(expose(&sensor, &scene, &mut Rng::new(123)), &scene)?;
    light.black.values = vec![0.0];
    light.capture.black = vec![0.0];
    assert!(
        calibrate(&light, Some(&dark), None).is_err(),
        "zero black accepted"
    );
    darks[2].black.values = vec![0.0];
    darks[2].capture.black = vec![0.0];
    assert!(
        build_master(Role::Dark, &darks, Estimator::Mean).is_err(),
        "zero black frame combined"
    );
    Ok(())
}

#[test]
fn flats_without_flat_dark_are_disclosed() -> Result<()> {
    let sensor = Sensor::new();
    let flats = frames(&sensor, &flat_scene([0.6, 1.0, 0.8]), 8, 124)?;
    let flat = build_flat(&flats, None, CLIPPED, 0.2)?;
    assert_eq!(flat.dark_role, None);
    assert!(flat.warnings.iter().any(|w| w.subject == "flat dark"));
    let medians = sensor.response_median();
    let center = site(W / 2, H / 2);
    let expected = sensor.response[center] / medians[color(W / 2, H / 2)];
    assert!((f64::from(flat.data[center]) / expected - 1.0).abs() < 0.02);
    Ok(())
}

#[test]
fn unrecorded_optics_are_disclosed_but_one_sided_optics_reject() -> Result<()> {
    let sensor = Sensor::new();
    let scene = light_scene(BACKGROUND);
    let strip = |m: &mut Mosaic| {
        m.capture.lens_model = None;
        m.capture.focal_length_mm = None;
        m.capture.f_number = None;
    };
    let mut flats = frames(&sensor, &flat_scene([0.6, 1.0, 0.8]), 4, 125)?;
    flats.iter_mut().for_each(strip);
    let flat = build_flat(&flats, None, Estimator::Mean, 0.2)?;
    let mut light = mosaic(expose(&sensor, &scene, &mut Rng::new(126)), &scene)?;
    assert!(
        calibrate(&light, None, Some(&flat)).is_err(),
        "lens recorded only on the light"
    );
    strip(&mut light);
    let result = calibrate(&light, None, Some(&flat))?;
    for subject in ["lens", "focal length", "aperture"] {
        assert!(
            result.report.warnings.iter().any(|w| w.subject == subject),
            "{subject} not disclosed"
        );
    }
    Ok(())
}

#[test]
fn sites_saturated_in_any_flat_are_invalid() -> Result<()> {
    let sensor = Sensor::new();
    let mut flats = frames(&sensor, &flat_scene([0.6, 1.0, 0.8]), 6, 127)?;
    let clipped: Vec<usize> = (0..12).map(|x| site(20 + x, 30)).collect();
    for &i in &clipped {
        flats[3].data[i] = WHITE;
    }
    let flat = build_flat(&flats, None, CLIPPED, 0.2)?;
    assert!(
        clipped.iter().all(|&i| !flat.valid[i]),
        "saturated flat site kept"
    );
    assert_eq!(flat.summary.invalid_sites, clipped.len());
    Ok(())
}

#[test]
fn decoded_white_level_changes_only_saturation_flags() -> Result<()> {
    let sensor = Sensor::new();
    let dark = build_master(
        Role::Dark,
        &frames(&sensor, &dark_scene(20.0), 4, 128)?,
        Estimator::Mean,
    )?;
    let scene = light_scene(BACKGROUND);
    let light = mosaic(expose(&sensor, &scene, &mut Rng::new(129)), &scene)?;
    let mut inferred = light.clone();
    inferred.white = 2000.0;
    inferred.capture.white = 2000.0;
    let a = calibrate(&light, Some(&dark), None)?;
    let b = calibrate(&inferred, Some(&dark), None)?;
    assert_eq!(a.data, b.data, "white level rescaled calibrated values");
    assert!(b.report.saturated_sites > a.report.saturated_sites);
    Ok(())
}

#[test]
fn dark_temperature_differences_are_reported() -> Result<()> {
    let sensor = Sensor::new();
    let mut darks = frames(&sensor, &dark_scene(20.0), 4, 130)?;
    darks
        .iter_mut()
        .for_each(|d| d.capture.temperature_c = Some(11.0));
    let dark = build_master(Role::Dark, &darks, Estimator::Mean)?;
    let scene = light_scene(BACKGROUND);
    let mut light = mosaic(expose(&sensor, &scene, &mut Rng::new(131)), &scene)?;
    let temperature = |r: &star_photos::calibration::Report| {
        r.warnings
            .iter()
            .find(|w| w.subject == "temperature")
            .map(|w| w.detail.clone())
    };
    let unrecorded = calibrate(&light, Some(&dark), None)?;
    assert_eq!(
        temperature(&unrecorded.report).as_deref(),
        Some("not recorded")
    );
    light.capture.temperature_c = Some(10.0);
    let differing = calibrate(&light, Some(&dark), None)?;
    assert!(temperature(&differing.report).is_some_and(|d| d.starts_with("11 °C vs 10 °C")));
    light.capture.temperature_c = Some(11.0);
    assert_eq!(
        temperature(&calibrate(&light, Some(&dark), None)?.report),
        None
    );
    Ok(())
}
