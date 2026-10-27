//! Frame-time summaries: median, 95th percentile and worst, in milliseconds.

fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    sorted[((q / 100.0) * (sorted.len() - 1) as f64).round() as usize]
}

pub fn summary(values: &[f64]) -> serde_json::Value {
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    serde_json::json!({
        "frames": v.len(),
        "median_ms": percentile(&v, 50.0) * 1e3,
        "p95_ms": percentile(&v, 95.0) * 1e3,
        "max_ms": v.last().copied().unwrap_or(f64::NAN) * 1e3,
    })
}

pub fn summary_ms(values: &[f64]) -> String {
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    format!(
        "median {:.1} ms, p95 {:.1} ms ({} frames)",
        percentile(&v, 50.0) * 1e3,
        percentile(&v, 95.0) * 1e3,
        v.len()
    )
}

/// Summaries per named phase, in the order the phases first appear.
pub fn by_phase(samples: &[(&'static str, f64)]) -> serde_json::Value {
    let mut names: Vec<&str> = Vec::new();
    for (name, _) in samples {
        if !names.contains(name) {
            names.push(name);
        }
    }
    names
        .iter()
        .map(|name| {
            let v: Vec<f64> = samples
                .iter()
                .filter(|(n, _)| n == name)
                .map(|(_, t)| *t)
                .collect();
            ((*name).to_string(), summary(&v))
        })
        .collect::<serde_json::Map<_, _>>()
        .into()
}
