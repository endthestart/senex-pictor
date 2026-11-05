mod support;

use lcc_raw::{DngWriteOptions, write_dng};
use pictor_engine::cameras::{MAKERS, built_in_matrix, table};
use pictor_engine::lens::{Database, find};
use pictor_engine::{Result, error, input};
use support::*;

/// A synthetic RAW (DNG without a colour matrix of its own) whose files say `make` and `model`.
fn synthetic_raw(dir: &std::path::Path, make: &str, model: &str) -> Result<std::path::PathBuf> {
    let mut raw = raw_image(vec![1008; W * H], &dark_scene(20.0));
    raw.metadata.make = Some(make.into());
    raw.metadata.model = Some(model.into());
    let path = dir.join(format!("{}.dng", model.replace(' ', "_")));
    std::fs::write(&path, write_dng(&raw, &DngWriteOptions::default())?)?;
    Ok(path)
}

#[test]
fn cameras_from_other_makers_find_their_matrix_by_the_names_their_files_write() -> Result<()> {
    let dir = std::env::temp_dir().join(format!("pictor-cameras-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let known = [
        ("SONY", "ILCE-7M3", 0.7374),
        ("Canon", "Canon EOS R5", 0.9766),
        ("Canon", "Canon EOS Rebel T7i", 0.697),
        ("FUJIFILM", "X-T20", 1.1434),
        ("OLYMPUS CORPORATION", "E-M1MarkII", 0.9383),
        ("OM Digital Solutions", "OM-1", 0.9488),
        ("RICOH IMAGING COMPANY, LTD.", "PENTAX K-1 Mark II", 0.8596),
        ("NIKON CORPORATION", "NIKON Z 6_2", 0.9943),
    ];
    for (make, model, first) in known {
        let shot = input::inspect(&synthetic_raw(&dir, make, model)?)?;
        assert!(!shot.embedded_color_matrix);
        let (make, model) = shot.make.zip(shot.model).ok_or_else(|| error("no make"))?;
        let m = built_in_matrix(&make, &model)?
            .ok_or_else(|| error(format!("no matrix for {model}")))?;
        assert_eq!(m[0][0], first, "{model}");
    }
    // Newer than LibRaw 0.22.1's table, or not in it: no matrix rather than an older model's.
    for (make, model) in [
        ("SONY", "ILCE-7M5"),
        ("FUJIFILM", "X-T30 III"),
        ("OM Digital Solutions", "OM-1MarkII"),
        ("Canon", "Canon EOS R8 Mark II"),
        ("Synthetic", "Sensor"),
    ] {
        let shot = input::inspect(&synthetic_raw(&dir, make, model)?)?;
        let (make, model) = shot.make.zip(shot.model).ok_or_else(|| error("no make"))?;
        assert!(built_in_matrix(&make, &model)?.is_none(), "{model}");
    }
    std::fs::remove_dir_all(dir)?;
    Ok(())
}

#[test]
fn every_maker_in_the_table_can_be_named_by_a_file() -> Result<()> {
    let table = table()?;
    for camera in table {
        assert!(
            MAKERS.iter().any(|(_, m)| *m == camera.maker),
            "{} has no Make words",
            camera.maker
        );
    }
    for (_, maker) in MAKERS {
        assert!(
            table.iter().any(|c| c.maker == *maker),
            "{maker} has no matrix"
        );
    }
    Ok(())
}

#[test]
fn the_built_in_lens_database_profiles_lenses_of_several_makers() -> Result<()> {
    let built_in = Database::BuiltIn;
    for (maker, lens, focal, aperture) in [
        ("NIKON", "NIKKOR Z 20mm f/1.8 S", 20.0, 1.8),
        ("NIKON", "AF Nikkor 24mm f/2.8D", 24.0, 2.8),
        ("Canon", "Canon RF 15-35mm F2.8L IS USM", 15.0, 2.8),
        ("FUJIFILM", "XF16mmF1.4 R WR", 16.0, 1.4),
        ("SONY", "Samyang 14mm f/2.8 AE ED AS IF UMC", 14.0, 2.8),
    ] {
        let v = find(&built_in, maker, lens, focal, aperture)?;
        assert_eq!((v.focal_mm, v.aperture), (focal, aperture), "{lens}");
        assert!(v.k[0] < 0.0, "{lens} darkens toward the corners");
        assert!(v.database.starts_with("built-in lensfun database"));
    }
    assert!(find(&built_in, "NIKON", "NIKKOR Z 21mm f/1.8 S", 21.0, 1.8).is_err());
    Ok(())
}
