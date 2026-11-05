use pictor_engine::Result;
use pictor_engine::lens::{Database, field, find, same_lens, vignetting_entries};
use pictor_engine::store::file::{f16_bytes, f16_values, pack_bits, unpack_bits};
use pictor_engine::store::seconds;
use std::path::PathBuf;

const LENSFUN: &str = r#"<lensdatabase version="1">
  <lens>
    <maker>Nikon</maker>
    <model>Nikon AF Nikkor 24mm f/2.8D</model>
    <model lang="en">Nikkor AF 24mm f/2.8D</model>
    <calibration>
      <distortion model="ptlens" focal="24" a="0.022" b="-0.056" c="0.022"/>
      <vignetting model="pa" focal="24.0" aperture="2.8" distance="10" k1="-0.5" k2="-0.4" k3="0.3"/>
      <vignetting model="pa" focal="24.0" aperture="5.6" distance="10" k1="-0.3" k2="0.1" k3="-0.1"/>
    </calibration>
  </lens>
  <lens>
    <maker>Nikon</maker>
    <model>Nikkor Z 20mm f/1.8 S</model>
    <calibration>
      <vignetting model="pa" focal="20" aperture="1.8" distance="1000" k1="-1.0" k2="0.2" k3="0.1"/>
    </calibration>
  </lens>
</lensdatabase>"#;

fn database(name: &str) -> Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("pictor-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("slr-nikon.xml"), LENSFUN)?;
    Ok(dir)
}

#[test]
fn bit_masks_round_trip_across_byte_boundaries() {
    let width = 13;
    let mask: Vec<bool> = (0..width * 3).map(|i| i % 3 == 0 || i == 12).collect();
    let packed = pack_bits(&mask, width);
    assert_eq!(packed.len(), 3 * 2);
    assert_eq!(unpack_bits(&packed, width), mask);
}

#[test]
fn half_floats_keep_adu_to_their_precision() {
    let values = [0.0f32, -3.5, 1008.25, 15375.0];
    let back = f16_values(&f16_bytes(values.iter().copied()));
    for (a, b) in values.iter().zip(&back) {
        assert!((a - b).abs() <= a.abs() / 1024.0, "{a} came back as {b}");
    }
}

#[test]
fn capture_times_count_days_across_leap_years() {
    let t = |year, month, day, hour, millis| lcc_meta::DateTime {
        year,
        month,
        day,
        hour,
        minute: 0,
        second: 0,
        millis,
        offset_minutes: None,
    };
    assert_eq!(seconds(&t(2000, 3, 1, 0, 0)), 951_868_800.0);
    assert_eq!(
        seconds(&t(2024, 3, 1, 0, 250)) - seconds(&t(2024, 2, 28, 0, 0)),
        2.0 * 86_400.0 + 0.25
    );
}

#[test]
fn lens_names_match_with_or_without_the_maker_word() {
    assert!(same_lens(
        "Nikon AF Nikkor 24mm f/2.8D",
        "AF Nikkor 24mm f/2.8D",
        "NIKON"
    ));
    assert!(same_lens(
        "Nikkor Z 20mm f/1.8 S",
        "NIKKOR  Z 20mm f/1.8 S",
        "Nikon"
    ));
    assert!(!same_lens(
        "Nikkor Z 20mm f/1.8 S",
        "Nikkor Z 24mm f/1.8 S",
        "Nikon"
    ));
}

#[test]
fn the_profile_at_the_nearest_aperture_is_chosen_and_a_missing_one_fails() -> Result<()> {
    let entries = vignetting_entries(LENSFUN)?;
    assert_eq!(entries.len(), 3);
    let dir = database("lens")?;
    let lensfun = Database::Dir(dir.clone());
    let near = find(&lensfun, "NIKON", "AF Nikkor 24mm f/2.8D", 24.0, 4.5)?;
    assert_eq!((near.aperture, near.k), (5.6, [-0.3, 0.1, -0.1]));
    assert!(find(&lensfun, "NIKON", "AF Nikkor 24mm f/2.8D", 35.0, 2.8).is_err());
    assert!(find(&lensfun, "NIKON", "AF Nikkor 28mm f/2.8D", 24.0, 2.8).is_err());
    std::fs::remove_dir_all(dir)?;
    Ok(())
}

#[test]
fn the_flat_field_is_one_at_the_centre_and_one_plus_k_at_the_corners() {
    let k = [-0.5, -0.4, 0.3];
    let v = field(5, 3, k);
    let centre = 5 + 2; // row 1, column 2 of a 5-wide field
    assert!((v[centre] - 1.0).abs() < 1e-6);
    for corner in [0, 4, 10, 14] {
        assert!(
            (v[corner] - 0.4).abs() < 1e-6,
            "corner {corner} is {}",
            v[corner]
        );
    }
}
