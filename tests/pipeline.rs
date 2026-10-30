use lcc_raw::{
    BlackLevel, ColorData, Metadata, OpcodeLists, Orientation, RawData, RawFormat, RawImage, Rect,
};
use pictor_engine::{Result, pipeline};

fn fixture() -> RawImage {
    RawImage {
        format: RawFormat::Dng,
        width: 8,
        height: 6,
        cpp: 3,
        data: RawData::F32(
            vec![[90.0, 600.0, 1200.0]; 48]
                .into_iter()
                .flatten()
                .collect(),
        ),
        cfa: None,
        bits: 32,
        black: BlackLevel::uniform(100.0),
        white: vec![1100.0],
        active_area: Rect::new(0, 0, 8, 6),
        crop: Rect::new(1, 1, 6, 4),
        orientation: Orientation::Normal,
        color: ColorData::default(),
        wb_multipliers: Some([2.0, 1.0, 1.5]),
        linearized: true,
        pixel_aspect: 1.0,
        opcodes: OpcodeLists::default(),
        metadata: Metadata::default(),
        lens_metadata: Default::default(),
    }
}

#[test]
fn normalization_preserves_negative_values_headroom_and_camera_rgb() -> Result<()> {
    let raw = fixture();
    let (image, report) = pipeline::develop(&raw)?;
    assert_eq!([image.width, image.height], [6, 4]);
    for pixel in &image.data {
        for (actual, expected) in pixel.iter().zip([-0.01, 0.5, 1.1]) {
            assert!((actual - expected).abs() < 1e-6);
        }
    }
    assert_eq!(report.wb_multipliers_not_applied, raw.wb_multipliers);
    assert!(report.minimum[0] < 0.0);
    assert!(report.maximum[2] > 1.0);
    Ok(())
}

#[test]
fn invalid_sensor_levels_and_samples_are_rejected() {
    let mut raw = fixture();
    raw.white = vec![100.0];
    assert!(pipeline::develop(&raw).is_err());
    raw.white = vec![1100.0];
    raw.black.values = vec![f32::NAN];
    assert!(pipeline::develop(&raw).is_err());
    raw.black = BlackLevel::uniform(100.0);
    raw.data = RawData::F32(vec![f32::NAN; 8 * 6 * 3]);
    assert!(pipeline::develop(&raw).is_err());
}

#[test]
fn float_tiff_roundtrip_preserves_values_and_existing_output() -> Result<()> {
    use pictor_engine::{input::Identity, output};
    use std::{fs::File, io::BufReader};
    use tiff::decoder::{Decoder, DecodingResult};
    let (image, report) = pipeline::develop(&fixture())?;
    let parent = std::env::temp_dir().join(format!("pictor-test-{}", std::process::id()));
    std::fs::create_dir(&parent)?;
    let dir = parent.join("output");
    let source = Identity {
        filename: "synthetic.dng".into(),
        bytes: 0,
        sha256: "synthetic fixture".into(),
    };
    let result = (|| -> Result<()> {
        output::save(&dir, &image, &source, &report)?;
        assert!(!dir.join("INCOMPLETE").exists());
        let mut decoder = Decoder::new(BufReader::new(File::open(dir.join("camera-linear.tif"))?))?;
        assert_eq!(decoder.dimensions()?, (6, 4));
        let DecodingResult::F32(samples) = decoder.read_image()? else {
            return Err(pictor_engine::error("TIFF did not preserve float samples"));
        };
        assert_eq!(samples, image.data.as_flattened());
        assert!(output::save(&dir, &image, &source, &report).is_err());
        assert!(dir.join("report.json").is_file());
        Ok(())
    })();
    std::fs::remove_dir_all(parent)?;
    result
}

#[test]
fn bayer_demosaic_preserves_constant_channel_values() -> Result<()> {
    let mut raw = fixture();
    let cfa =
        lcc_raw::Cfa::bayer("RGGB").ok_or_else(|| pictor_engine::error("invalid test CFA"))?;
    raw.cpp = 1;
    raw.data = RawData::F32(
        (0..48)
            .map(|i| [300.0, 600.0, 900.0][usize::from(cfa.color_at(i % 8, i / 8))])
            .collect(),
    );
    raw.cfa = Some(cfa);
    let (image, _) = pipeline::develop(&raw)?;
    for pixel in image.data {
        for (actual, expected) in pixel.iter().zip([0.2, 0.5, 0.8]) {
            assert!((actual - expected).abs() < 1e-5, "{pixel:?}");
        }
    }
    Ok(())
}

#[test]
fn crop_and_rotation_preserve_coordinate_mapping() -> Result<()> {
    let mut raw = fixture();
    raw.black = BlackLevel::uniform(0.0);
    raw.white = vec![100.0];
    raw.data = RawData::F32((0..48).flat_map(|i| [i as f32; 3]).collect());
    raw.orientation = Orientation::Rotate90;
    let (image, _) = pipeline::develop(&raw)?;
    assert_eq!([image.width, image.height], [4, 6]);
    assert!((image.data[0][0] - 0.33).abs() < 1e-6);
    assert!((image.data[23][0] - 0.14).abs() < 1e-6);
    Ok(())
}
