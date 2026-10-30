use pictor_engine::Result;
use pictor_engine::integrate::combine::{Sample, luminance_clipped, weighted};
use pictor_engine::separate::filters::distance_squared;
use pictor_engine::separate::matte::matte;
use pictor_engine::separate::stats::Stats;
use pictor_engine::stars::similarity::Random;

#[test]
fn the_distance_transform_is_exact_and_names_the_nearest_pixel() -> Result<()> {
    let (w, h) = (37, 23);
    let mut random = Random::new(5);
    let known: Vec<bool> = (0..w * h).map(|_| random.below(17) == 0).collect();
    let (dist, index) = distance_squared(&known, w, h).ok_or("some pixels are known")?;
    for p in 0..w * h {
        let (x, y) = ((p % w) as f64, (p / w) as f64);
        let best = (0..w * h)
            .filter(|&q| known[q])
            .map(|q| ((q % w) as f64 - x).powi(2) + ((q / w) as f64 - y).powi(2))
            .fold(f64::INFINITY, f64::min);
        assert_eq!(dist[p], best, "pixel {p}");
        assert!(known[index[p]]);
        let (qx, qy) = ((index[p] % w) as f64, (index[p] / w) as f64);
        assert_eq!((qx - x).powi(2) + (qy - y).powi(2), best);
    }
    Ok(())
}

#[test]
fn clipping_drops_a_satellite_trail_and_keeps_the_noise() {
    let mut samples: Vec<Sample> = (0..40)
        .map(|i| Sample {
            value: 100.0 + ((i * 7) % 11) as f32 - 5.0,
            weight: 1.0,
            frame: i,
        })
        .collect();
    samples[17].value = 900.0;
    let (mean, kept, halves) = weighted(&samples);
    assert_eq!(kept, 39);
    assert!((mean - 100.0).abs() < 1.0, "mean {mean}");
    assert!(halves.iter().all(|h| (h - 100.0).abs() < 2.0));
    let mut rgb: Vec<[f32; 3]> = (0..20)
        .map(|i| [10.0 + (i % 3) as f32, 20.0, 30.0])
        .collect();
    rgb[3] = [500.0, 500.0, 500.0];
    rgb[4] = [f32::NAN, 1.0, 1.0];
    let (mean, kept, valid) = luminance_clipped(&rgb);
    assert_eq!((kept, valid), (18, 19));
    assert!((mean[1] - 20.0).abs() < 1e-4);
}

/// Sky in the upper half with star transits everywhere but two patches and one dead pixel, a
/// dark tree trunk through it, a half-covered row along its lower edge, dark ground in the
/// lower half and a bright patch there without transits (lit bark). Of the two sky patches
/// no star crosses, one is lit red (leaves under a lamp), the other has the sky's colour.
fn scene() -> Stats {
    let (w, h) = (96, 64);
    let p = |f: &dyn Fn(usize, usize) -> f32| -> Vec<f32> {
        (0..w * h).map(|i| f(i % w, i / w)).collect()
    };
    let sky =
        |x: usize, y: usize| (y < 32 && !(40..48).contains(&x) || y < 8) && (x, y) != (10, 20);
    let lamp = |x: usize, y: usize| (14..38).contains(&x) && (8..30).contains(&y);
    let starless = |x: usize, y: usize| (54..78).contains(&x) && (8..30).contains(&y);
    let crossed = move |x: usize, y: usize| sky(x, y) && !lamp(x, y) && !starless(x, y);
    let level = |x: usize, y: usize| {
        if sky(x, y) {
            1000.0
        } else if y == 32 && x < 40 {
            550.0
        } else if (60..70).contains(&x) && (44..54).contains(&y) {
            900.0
        } else {
            100.0
        }
    };
    Stats {
        width: w,
        height: h,
        median: p(&level),
        low: p(&level),
        sigma: p(&|_, _| 20.0),
        excess: p(&|x, y| if crossed(x, y) { 0.02 } else { 0.0 }),
        transit: p(&|x, y| if crossed(x, y) { 0.02 } else { 0.0 }),
        scatter: p(&|_, _| 20.0),
        resolvable: vec![true; w * h],
        red: p(&|x, y| if lamp(x, y) { 0.6 } else { 0.3 }),
        blue: p(&|x, y| if lamp(x, y) { 0.15 } else { 0.3 }),
    }
}

#[test]
fn the_matte_is_sky_where_stars_cross_and_ground_where_dark() -> Result<()> {
    let s = scene();
    let w = s.width;
    let m = matte(&s, 50, None)?;
    let at = |x: usize, y: usize| m.alpha[y * w + x];
    assert_eq!(at(10, 10), 1.0, "clear sky");
    assert_eq!(
        at(26, 18),
        0.0,
        "light of its own colour where no star crosses"
    );
    assert_eq!(at(66, 18), 1.0, "starless sky of the sky's colour");
    assert_eq!(
        at(15, 18),
        0.0,
        "lit pixels beside the transits go with their core"
    );
    assert_eq!(at(44, 20), 0.0, "the trunk between sky");
    assert_eq!(at(5, 60), 0.0, "dark ground");
    assert_eq!(
        at(10, 20),
        1.0,
        "a one-pixel ground island in the sky counts as sky"
    );
    assert!(
        (0.3..0.7).contains(&at(5, 32)),
        "a half-covered edge beside the transits keeps its share of sky"
    );
    assert_eq!(
        at(65, 49),
        0.0,
        "motion outranks brightness: a lit patch far from any transit is ground"
    );
    let mut labels = vec![0u32; s.width * s.height];
    for (i, l) in labels.iter_mut().enumerate() {
        if i / w >= 32 {
            *l = 1;
        }
    }
    let segmented = matte(&s, 50, Some(&labels))?;
    assert_eq!(
        segmented.alpha[49 * w + 65],
        0.0,
        "motion outranks brightness inside a dark segment"
    );
    Ok(())
}

#[test]
fn painted_strokes_override_the_matte_later_over_earlier() {
    use pictor_engine::separate::corrections::{Stroke, apply, force};
    let (w, h) = (40, 20);
    let strokes = [
        Stroke {
            sky: true,
            radius: 3.0,
            points: vec![[10.0, 10.0], [30.0, 10.0]],
        },
        Stroke {
            sky: false,
            radius: 2.0,
            points: vec![[20.0, 10.0]],
        },
    ];
    let f = force(&strokes, w, h);
    let at = |x: usize, y: usize| f[y * w + x];
    assert_eq!(at(14, 12), 1, "inside the sky stroke");
    assert_eq!(at(14, 14), 0, "beyond its radius");
    assert_eq!(at(20, 10), -1, "the later ground dab wins");
    let mut alpha = vec![0.5; w * h];
    assert_eq!(apply(&mut alpha, &f), f.iter().filter(|&&v| v != 0).count());
    assert_eq!(
        (alpha[12 * w + 14], alpha[10 * w + 20], alpha[0]),
        (1.0, 0.0, 0.5)
    );
}
