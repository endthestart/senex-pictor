mod support;

use lcc_raw::BlackLevel;
use pictor_engine::Result;
use pictor_engine::calibration::{Estimator, Master, Mosaic, Role, build_master};
use support::*;

/// Expected dark level above black and its single-frame standard deviation at each site.
fn dark_truth(sensor: &Sensor, exposure: f64) -> Vec<(f64, f64)> {
    (0..W * H)
        .map(|s| {
            let g = GAIN[color(s % W, s / W)];
            let electrons = exposure * sensor.dark_rate[s];
            let level = sensor.offset[s] + g * electrons;
            (
                level,
                (READ_NOISE_ADU.powi(2) + g * g * electrons + 1.0 / 12.0).sqrt(),
            )
        })
        .collect()
}

#[test]
fn master_dark_recovers_pedestal_offsets_and_dark_signal() -> Result<()> {
    let sensor = Sensor::new();
    let darks = frames(&sensor, &dark_scene(20.0), 16, 11)?;
    let master = build_master(Role::Dark, &darks, Estimator::Mean)?;
    let truth = dark_truth(&sensor, 20.0);
    let outside = (0..W * H)
        .filter(|&s| (f64::from(master.data[s]) - truth[s].0).abs() > 4.5 * truth[s].1 / 4.0)
        .count();
    assert!(outside <= 3, "{outside} sites outside 4.5 standard errors");
    for (x, y) in HOT {
        let s = site(x, y);
        assert!((f64::from(master.data[s]) - truth[s].0).abs() < 4.5 * truth[s].1 / 4.0);
        assert!(f64::from(master.data[s]) > 700.0, "hot site not recovered");
    }
    let mut ratios: Vec<f64> = (0..W * H)
        .map(|s| f64::from(master.standard_error[s]) / (truth[s].1 / 4.0))
        .collect();
    ratios.sort_by(f64::total_cmp);
    let median = ratios[ratios.len() / 2];
    assert!((median - 1.0).abs() < 0.1, "standard error ratio {median}");
    assert!(master.accepted.iter().all(|&n| n == 16));
    Ok(())
}

#[test]
fn sigma_clipping_rejects_transient_hits_without_degrading_clean_sites() -> Result<()> {
    let sensor = Sensor::new();
    let mut darks = frames(&sensor, &dark_scene(20.0), 12, 21)?;
    let hits: Vec<usize> = (0..20).map(|i| site(3 + 3 * i, 5 + 2 * i)).collect();
    for &s in &hits {
        darks[4].data[s] += 3000.0;
    }
    let truth = dark_truth(&sensor, 20.0);
    let clipped = build_master(Role::Dark, &darks, CLIPPED)?;
    let mean = build_master(Role::Dark, &darks, Estimator::Mean)?;
    for &s in &hits {
        let tolerance = 5.0 * truth[s].1 / 11f64.sqrt();
        assert!((f64::from(clipped.data[s]) - truth[s].0).abs() < tolerance);
        assert!(clipped.accepted[s] <= 11);
        assert!(
            f64::from(mean.data[s]) - truth[s].0 > 200.0,
            "mean unaffected; test cannot fail"
        );
    }
    let errors = |m: &Master| -> (f64, f64) {
        let e: Vec<f64> = (0..W * H)
            .filter(|s| !hits.contains(s))
            .map(|s| f64::from(m.data[s]) - truth[s].0)
            .collect();
        let n = e.len() as f64;
        (
            (e.iter().map(|v| v * v).sum::<f64>() / n).sqrt(),
            e.iter().sum::<f64>() / n,
        )
    };
    let ((clipped_rms, clipped_bias), (mean_rms, _)) = (errors(&clipped), errors(&mean));
    assert!(
        clipped_rms < 1.06 * mean_rms,
        "clipping noise {clipped_rms} vs mean {mean_rms}"
    );
    assert!(
        clipped_bias.abs() < 0.1,
        "clipping bias {clipped_bias} ADU on clean sites"
    );
    Ok(())
}

#[test]
fn mismatched_or_insufficient_master_frames_are_rejected() -> Result<()> {
    let sensor = Sensor::new();
    let darks = frames(&sensor, &dark_scene(20.0), 4, 31)?;
    assert!(build_master(Role::Dark, &darks[..1], Estimator::Mean).is_err());
    assert!(
        build_master(
            Role::Dark,
            &darks,
            Estimator::WinsorizedSigmaClip {
                k: 0.5,
                iterations: 3
            }
        )
        .is_err()
    );
    let mut other_exposure = frames(&sensor, &dark_scene(10.0), 1, 32)?;
    let mut mixed = darks.clone();
    mixed.append(&mut other_exposure);
    assert!(build_master(Role::Dark, &mixed, Estimator::Mean).is_err());
    let mut unrecorded = darks.clone();
    unrecorded[2].capture.in_camera_nr = None;
    assert!(build_master(Role::Dark, &unrecorded, Estimator::Mean).is_err());
    let mut other_iso = darks.clone();
    other_iso[1].capture.iso = Some(800);
    assert!(build_master(Role::Dark, &other_iso, Estimator::Mean).is_err());
    let mut other_shutter = darks.clone();
    other_shutter[3].capture.shutter_mode = Some("Mechanical".into());
    let master = build_master(Role::Dark, &other_shutter, Estimator::Mean)?;
    assert!(master.warnings.iter().any(|w| w.subject == "shutter mode"));
    Ok(())
}

#[test]
fn mosaic_rejects_unreviewed_sensor_layouts() -> Result<()> {
    let scene = dark_scene(20.0);
    let samples = || vec![1008u16; W * H];
    assert!(Mosaic::from_raw(&raw_image(samples(), &scene)).is_ok());
    let mut deltas = raw_image(samples(), &scene);
    deltas.black = BlackLevel {
        delta_h: vec![0.5; W],
        ..BlackLevel::uniform(BLACK)
    };
    assert!(Mosaic::from_raw(&deltas).is_err());
    let mut short_pattern = raw_image(samples(), &scene);
    short_pattern.black = BlackLevel {
        repeat_rows: 2,
        repeat_cols: 2,
        ..BlackLevel::uniform(BLACK)
    };
    assert!(Mosaic::from_raw(&short_pattern).is_err());
    let mut no_cfa = raw_image(samples(), &scene);
    no_cfa.cfa = None;
    assert!(Mosaic::from_raw(&no_cfa).is_err());
    let mut mosaic = Mosaic::from_raw(&raw_image(samples(), &scene))?;
    mosaic.data[5] = f32::NAN;
    assert!(mosaic.validate().is_err());
    Ok(())
}
