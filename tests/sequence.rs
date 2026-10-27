use star_photos::Result;
use star_photos::sequence::{Capture, Settings, Steps, check};

fn settings() -> Settings {
    Settings {
        model: Some("Z 6_2".into()),
        lens: Some("NIKKOR Z 20mm f/1.8 S".into()),
        exposure_s: Some(20.0),
        iso: Some(800),
        aperture: Some(1.8),
        focal_mm: Some(20.0),
    }
}

/// `n` frames 25 s apart with a dark, steady sky and foreground.
fn frames(n: usize) -> Vec<Capture> {
    (0..n)
        .map(|i| Capture {
            name: format!("F{i:03}.NEF"),
            time_s: 1000.0 + 25.0 * i as f64,
            settings: settings(),
            saturated_share: 0.0,
            sky_level: 400.0 + (i % 3) as f64,
            ground_rgb: [50.0, 60.0, 40.0],
        })
        .collect()
}

#[test]
fn gaps_and_setting_changes_split_runs_and_the_longest_is_recommended() -> Result<()> {
    let mut c = frames(30);
    for f in &mut c[10..] {
        f.time_s += 600.0;
    }
    c[25].settings.exposure_s = Some(10.0);
    for f in &mut c[26..] {
        f.settings.exposure_s = Some(10.0);
    }
    let report = check(&c, &Steps::still(c.len()))?;
    let runs: Vec<(String, usize, &str)> = report
        .runs
        .iter()
        .map(|r| (r.first.clone(), r.frames, r.starts_after))
        .collect();
    assert_eq!(
        runs,
        vec![
            ("F000.NEF".into(), 10, "start"),
            ("F010.NEF".into(), 15, "time gap"),
            ("F025.NEF".into(), 5, "settings change")
        ]
    );
    assert_eq!(report.recommended, 1);
    Ok(())
}

#[test]
fn overexposed_frames_are_flagged_and_left_out() -> Result<()> {
    let mut c = frames(20);
    for f in &mut c[15..] {
        f.saturated_share = 0.4;
    }
    let report = check(&c, &Steps::still(c.len()))?;
    let run = &report.runs[0];
    assert_eq!(run.overexposed.len(), 5);
    assert_eq!((run.use_last.as_str(), run.use_frames), ("F014.NEF", 15));
    Ok(())
}

#[test]
fn a_steady_twilight_ramp_recommends_the_dark_stretch_and_aurora_swings_do_not() -> Result<()> {
    let mut c = frames(40);
    for (i, f) in c.iter_mut().enumerate() {
        f.sky_level = 400.0 * 1.1f64.powi(i as i32);
    }
    let run = &check(&c, &Steps::still(c.len()))?.runs[0];
    let twilight = run
        .twilight
        .as_ref()
        .ok_or("a 41-fold steady climb is twilight")?;
    // The 5-frame median at the start covers three frames, so the darkest smoothed level is
    // 400·1.1; frames stay within 2× of it while 1.1^k ≤ 2.2, through k = 8.
    assert_eq!(
        (twilight.dark_first.as_str(), twilight.dark_last.as_str()),
        ("F000.NEF", "F008.NEF")
    );
    assert_eq!(run.use_frames, 9);

    let mut aurora = frames(40);
    for (i, f) in aurora.iter_mut().enumerate() {
        f.sky_level = if (i / 5) % 2 == 0 { 400.0 } else { 1600.0 };
    }
    assert!(
        check(&aurora, &Steps::still(aurora.len()))?.runs[0]
            .twilight
            .is_none()
    );
    Ok(())
}

#[test]
fn lit_frames_are_found_alone_and_in_clusters() -> Result<()> {
    let mut c = frames(40);
    c[5].ground_rgb = [150.0, 70.0, 45.0];
    for f in &mut c[20..23] {
        f.ground_rgb = [120.0, 130.0, 90.0];
    }
    let lit = check(&c, &Steps::still(c.len()))?.runs.remove(0).lit;
    assert_eq!(lit, vec!["F005.NEF", "F020.NEF", "F021.NEF", "F022.NEF"]);
    Ok(())
}
