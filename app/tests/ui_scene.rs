mod common;

use common::{drag, harness, save, wait};
use egui_kittest::kittest::Queryable;
use pictor::app::Pictor;
use pictor::shell::Step;
use pictor_engine::{Result, error};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

/// `PICTOR_UI_CORRECT`, which two of these tests change: one test at a time uses it.
static CORRECT: Mutex<()> = Mutex::new(());

fn correct_folder() -> MutexGuard<'static, ()> {
    CORRECT.lock().unwrap_or_else(PoisonError::into_inner)
}

/// From the Scene step a hero composite is made in the background and opens in Compare
/// (`PICTOR_UI_FOLDER`; skipped without it; adds the hero result to that folder).
#[test]
fn a_hero_frame_is_made_from_the_scene_step() -> Result<()> {
    let Ok(folder) = std::env::var("PICTOR_UI_FOLDER") else {
        eprintln!("PICTOR_UI_FOLDER is not set; skipped");
        return Ok(());
    };
    let mut h = harness(Some(PathBuf::from(folder)), None);
    wait(&mut h, 300, Pictor::opened)?;
    h.state_mut().show_step(Step::Scene);
    wait(&mut h, 60, Pictor::idle)?;
    let before = h.render().map_err(error)?;
    save(&before, "scene.jpg")?;
    let made = h.query_all_by_label("Open in Compare").next().is_some();
    if made {
        h.get_by_label("Open in Compare").click();
    } else {
        h.get_all_by_label("Make It")
            .next()
            .ok_or_else(|| error("no Make It"))?
            .click();
        h.step();
        let making = h.render().map_err(error)?;
        save(&making, "scene-making.jpg")?;
    }
    wait(&mut h, 600, |a| a.idle() && a.comparing())?;
    let opened = h.render().map_err(error)?;
    save(&opened, "scene-hero-compare.jpg")?;
    Ok(())
}

/// From the Scene step a short-window moment is made and opens in Compare
/// (`PICTOR_UI_STACKED`, a processed folder with a whole-sequence stack; skipped without
/// it; adds the moment to that folder).
#[test]
fn a_moment_is_made_from_the_scene_step() -> Result<()> {
    let Ok(folder) = std::env::var("PICTOR_UI_STACKED") else {
        eprintln!("PICTOR_UI_STACKED is not set; skipped");
        return Ok(());
    };
    let mut h = harness(Some(PathBuf::from(folder)), None);
    wait(&mut h, 300, Pictor::opened)?;
    h.state_mut().show_step(Step::Scene);
    wait(&mut h, 60, Pictor::idle)?;
    let buttons = h.query_all_by_label("Make It").count();
    assert!(
        buttons >= 1,
        "the moment should be offered for a stacked run"
    );
    h.get_all_by_label("Make It")
        .last()
        .ok_or_else(|| error("no Make It"))?
        .click();
    h.step();
    assert!(!h.state().idle(), "Make It should start the moment");
    wait(&mut h, 900, |a| a.idle() && a.comparing())?;
    let opened = h.render().map_err(error)?;
    save(&opened, "scene-moment-compare.jpg")?;
    Ok(())
}

/// Ground painted on Sky & ground is applied in the background and the run reopens there with
/// the stroke saved (`PICTOR_UI_CORRECT`, a processed folder the test changes; skipped
/// without it).
#[test]
fn a_painted_correction_is_applied_and_the_run_reopens_on_sky() -> Result<()> {
    let Ok(folder) = std::env::var("PICTOR_UI_CORRECT") else {
        eprintln!("PICTOR_UI_CORRECT is not set; skipped");
        return Ok(());
    };
    let _only = correct_folder();
    let mut h = harness(Some(PathBuf::from(folder)), None);
    wait(&mut h, 300, Pictor::opened)?;
    h.state_mut().show_step(Step::Sky);
    wait(&mut h, 60, Pictor::idle)?;
    let saved = |p: &Pictor| p.session().map(|s| s.sky.brush.saved.len());
    let before = saved(h.state()).ok_or("the folder should open")?;
    h.key_press(egui::Key::G);
    h.step();
    drag(&mut h, [500.0, 300.0], [800.0, 340.0]);
    let strokes = h.state().session().map_or(0, |s| s.sky.brush.strokes.len());
    assert_eq!(strokes, before + 1, "the drag should paint one stroke");
    save(&h.render().map_err(error)?, "sky-painted.jpg")?;
    h.get_by_label("Apply Corrections").click();
    h.step();
    assert!(!h.state().idle(), "applying runs in the background");
    wait(&mut h, 1800, |p| p.idle() && saved(p) == Some(before + 1))?;
    assert_eq!(
        h.state().step(),
        Step::Sky,
        "the run reopens on Sky & ground"
    );
    save(&h.render().map_err(error)?, "sky-corrected.jpg")?;
    Ok(())
}

/// Compare puts another result of the run in B (`PICTOR_UI_FOLDER` with a second result, such
/// as the hero frame the Scene test makes; skipped without one).
#[test]
fn compare_shows_another_result_of_the_run_in_b() -> Result<()> {
    let Ok(folder) = std::env::var("PICTOR_UI_FOLDER") else {
        eprintln!("PICTOR_UI_FOLDER is not set; skipped");
        return Ok(());
    };
    let store = PathBuf::from(&folder).join("store");
    let Some(other) = std::fs::read_dir(&store)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            e.file_name()
                .to_string_lossy()
                .strip_suffix(".planes")
                .map(String::from)
        })
        .find(|n| n.starts_with("hero-") || n.starts_with("moment-"))
    else {
        eprintln!("{folder} has no second result; skipped");
        return Ok(());
    };
    let mut h = harness(Some(PathBuf::from(folder)), None);
    wait(&mut h, 300, Pictor::opened)?;
    h.state_mut().show_step(Step::Compare);
    wait(&mut h, 60, Pictor::idle)?;
    h.get_by_label("B shows").click();
    h.step();
    h.get_by_label(&other).click();
    h.step();
    wait(&mut h, 300, |p| {
        p.idle() && p.session().is_some_and(|s| s.viewer.other.is_some())
    })?;
    save(&h.render().map_err(error)?, "compare-other.jpg")?;
    let key = h
        .state()
        .session()
        .map(|s| s.viewer.key(1))
        .unwrap_or_default();
    assert!(key.ends_with(&other), "B should show {other}, not {key}");
    Ok(())
}

/// Sky & ground lists the regions the colour check made ground, and Show zooms to one with
/// them tinted (`PICTOR_UI_CORRECT`, a run whose matte keeps them; skipped without it).
#[test]
fn show_zooms_to_a_region_the_colour_check_made_ground() -> Result<()> {
    let Ok(folder) = std::env::var("PICTOR_UI_CORRECT") else {
        eprintln!("PICTOR_UI_CORRECT is not set; skipped");
        return Ok(());
    };
    let _only = correct_folder();
    let mut h = harness(Some(PathBuf::from(folder)), None);
    wait(&mut h, 300, Pictor::opened)?;
    h.state_mut().show_step(Step::Sky);
    wait(&mut h, 60, Pictor::idle)?;
    let Some(show) = h.query_all_by_label("Show").next() else {
        return Err(error("no regions listed"));
    };
    show.click();
    h.step();
    h.step();
    let shown = h
        .state()
        .session()
        .map(|s| (s.sky.review.show, s.viewer.zoom));
    assert!(
        matches!(shown, Some((true, pictor::viewer::Zoom::Factor(_)))),
        "Show should tint the regions and zoom in"
    );
    save(&h.render().map_err(error)?, "sky-region.jpg")?;
    Ok(())
}

/// Scene's long exposure switches moving branches between sharp and blurred by redoing the
/// composite (`PICTOR_UI_CORRECT`, a run the test changes; skipped without it).
#[test]
fn moving_branches_switch_redoes_the_composite() -> Result<()> {
    let Ok(folder) = std::env::var("PICTOR_UI_CORRECT") else {
        eprintln!("PICTOR_UI_CORRECT is not set; skipped");
        return Ok(());
    };
    let _only = correct_folder();
    let mut h = harness(Some(PathBuf::from(folder)), None);
    wait(&mut h, 300, Pictor::opened)?;
    let sharp = |p: &Pictor| {
        p.session()
            .and_then(|s| s.run.as_ref())
            .map(|r| r.report.sharp_moving)
    };
    let before = sharp(h.state()).ok_or("the run should open")?;
    h.state_mut().show_step(Step::Scene);
    wait(&mut h, 60, Pictor::idle)?;
    h.get_by_label(if before { "Blurred" } else { "Sharp" })
        .click();
    h.step();
    assert!(
        !h.state().idle(),
        "the composite is redone in the background"
    );
    wait(&mut h, 1800, |p| p.idle() && sharp(p) == Some(!before))?;
    save(&h.render().map_err(error)?, "scene-moving.jpg")?;
    Ok(())
}
