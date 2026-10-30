use egui::Color32;
use pictor::theme::{self, Role, radius, role};

const DOC: &str = include_str!("../../docs/DESIGN-TOKENS.md");

/// The cells of each row of the table under `## heading`, header and rule left out.
fn table(heading: &str) -> Result<Vec<Vec<String>>, String> {
    let start = DOC
        .find(&format!("\n## {heading}\n"))
        .ok_or(format!("no {heading} section"))?;
    let rows: Vec<Vec<String>> = DOC[start..]
        .lines()
        .skip_while(|l| !l.starts_with('|'))
        .take_while(|l| l.starts_with('|'))
        .skip(2)
        .map(|l| {
            l.trim_matches('|')
                .split('|')
                .map(|c| c.trim().to_string())
                .collect()
        })
        .collect();
    if rows.is_empty() {
        return Err(format!("no rows under {heading}"));
    }
    Ok(rows)
}

/// Checks that the table's first column names exactly `ours`, each matching by `same`.
fn matches<T>(
    heading: &str,
    ours: &[(&str, T)],
    same: impl Fn(&[String], &T) -> bool,
) -> Result<(), String> {
    let rows = table(heading)?;
    let mut names: Vec<&str> = rows.iter().map(|r| r[0].as_str()).collect();
    let mut expected: Vec<&str> = ours.iter().map(|(n, _)| *n).collect();
    names.sort_unstable();
    expected.sort_unstable();
    if names != expected {
        return Err(format!(
            "{heading}: table has {names:?}, theme has {expected:?}"
        ));
    }
    for (name, value) in ours {
        let row = rows
            .iter()
            .find(|r| r[0] == *name)
            .ok_or(format!("{heading}: no {name}"))?;
        if !same(row, value) {
            return Err(format!("{heading}: {name} differs from {row:?}"));
        }
    }
    Ok(())
}

fn hex(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

#[test]
fn the_theme_follows_the_family_design_tokens() -> Result<(), String> {
    let title = DOC.lines().next().unwrap_or_default();
    if title != format!("# SENEX family design tokens {}", theme::TOKENS) {
        return Err(format!(
            "the theme follows {}, the file is {title}",
            theme::TOKENS
        ));
    }
    let colours = [
        ("GROUND", theme::GROUND),
        ("CHROME", theme::CHROME),
        ("CARD", theme::CARD),
        ("RAISED", theme::RAISED),
        ("LINE", theme::LINE),
        ("LINE_STRONG", theme::LINE_STRONG),
        ("TEXT", theme::TEXT),
        ("MUTED", theme::MUTED),
        ("ACCENT", theme::ACCENT),
        ("ACCENT_DIM", theme::ACCENT_DIM),
        ("GOOD", theme::GOOD),
        ("WARN", theme::WARN),
        ("BAD", theme::BAD),
    ];
    matches("Colours", &colours, |row, c| row[1] == hex(*c))?;
    let radii = [
        ("card", radius::CARD),
        ("button", radius::BUTTON),
        ("tile", radius::TILE),
        ("control", radius::CONTROL),
        ("key", radius::KEY),
    ];
    matches("Corner radii", &radii, |row, r| row[1] == r.to_string())?;
    let roles: [(&str, Role); 7] = [
        ("heading", role::HEADING),
        ("panel", role::PANEL),
        ("strong", role::STRONG),
        ("secondary", role::SECONDARY),
        ("section", role::SECTION),
        ("badge", role::BADGE),
        ("note", role::NOTE),
    ];
    matches("Type roles", &roles, |row, r| {
        row[1].parse::<f32>() == Ok(r.size)
            && row[2] == if r.semibold { "Semibold" } else { "Regular" }
    })
}
