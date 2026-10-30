use pictor_engine::flow::features;

/// The feature image as first written: quantised log levels, then twice a 5×5 median by
/// sorting each window of replicated-edge samples.
fn by_sorting(image: &[f32], width: usize, height: usize, scale: f32) -> Vec<f32> {
    let mut v: Vec<f32> = image.iter().map(|x| (x.max(0.0) / scale).ln_1p()).collect();
    let lo = v.iter().copied().fold(f32::INFINITY, f32::min);
    let hi = v.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let span = (hi - lo).max(f32::MIN_POSITIVE);
    for x in &mut v {
        *x = ((*x - lo) / span * 255.0).round();
    }
    let median = |g: &[f32]| -> Vec<f32> {
        let at = |x: isize, y: isize| {
            g[y.clamp(0, height as isize - 1) as usize * width
                + x.clamp(0, width as isize - 1) as usize]
        };
        (0..width * height)
            .map(|p| {
                let (x, y) = ((p % width) as isize, (p / width) as isize);
                let mut window: Vec<f32> = (-2..=2)
                    .flat_map(|dy| (-2..=2).map(move |dx| at(x + dx, y + dy)))
                    .collect();
                window.sort_by(f32::total_cmp);
                window[12]
            })
            .collect()
    };
    median(&median(&v))
}

#[test]
fn the_running_median_matches_sorting_every_window() {
    let mut state = 0x2545_f491_4f6c_dd1du64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % 10_000) as f32 - 500.0
    };
    let mut middle = 0;
    for (w, h) in [(1, 1), (2, 3), (7, 1), (17, 11), (64, 40)] {
        let image: Vec<f32> = (0..w * h)
            .map(|p| next() + if p % 9 == 0 { 40_000.0 } else { 0.0 })
            .collect();
        let got = features(&image, w, h, 40.0);
        assert_eq!(got.width, w);
        let want = by_sorting(&image, w, h, 40.0);
        middle += want.iter().filter(|v| **v > 0.0 && **v < 255.0).count();
        let bits = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        assert_eq!(bits(&got.data), bits(&want), "{w}×{h}");
    }
    assert!(middle > 1000, "{middle} middle levels; test cannot fail");
}
