//! Order statistics of float samples (numpy conventions).

/// Median; the mean of the two middle values for an even count.
pub fn median(v: &mut [f32]) -> f32 {
    let n = v.len();
    let (_, hi, _) = v.select_nth_unstable_by(n / 2, f32::total_cmp);
    let hi = *hi;
    if n % 2 == 1 {
        return hi;
    }
    let lo = v[..n / 2].iter().copied().fold(f32::NEG_INFINITY, f32::max);
    (lo + hi) / 2.0
}

/// numpy's linear-interpolated percentile.
pub fn percentile(v: &mut [f32], q: f64) -> f32 {
    let rank = q / 100.0 * (v.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let (_, a, rest) = v.select_nth_unstable_by(lo, f32::total_cmp);
    let a = *a;
    let b = rest.iter().copied().fold(f32::INFINITY, f32::min);
    if rank == lo as f64 || rest.is_empty() {
        return a;
    }
    (f64::from(a) + (f64::from(b) - f64::from(a)) * (rank - lo as f64)) as f32
}
