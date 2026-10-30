mod common;

use common::{SIZE, harness, rgb, save, wait};
use egui_kittest::kittest::Queryable;
use pictor::app::Pictor;
use pictor::shell::Step;
use pictor_engine::{Result, error};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[test]
fn the_first_screen_offers_to_open_a_folder() -> Result<()> {
    let mut h = harness(None, None);
    h.step();
    let shot = h.render().map_err(error)?;
    save(&shot, "welcome.jpg")?;
    assert!(h.query_by_label("Open Folder…").is_some());
    assert!(!h.state().opened());
    Ok(())
}

#[test]
fn space_swaps_the_two_renderings_on_the_compare_screen() -> Result<()> {
    let mut h = harness(None, Some([1200, 800]));
    wait(&mut h, 120, Pictor::comparing)?;
    let before = h.render().map_err(error)?;
    save(&before, "compare.jpg")?;
    h.key_press(egui::Key::Space);
    h.step();
    let after = h.render().map_err(error)?;
    save(&after, "compare-swapped.jpg")?;
    let centre = (SIZE[0] * 0.4, SIZE[1] * 0.3);
    assert_ne!(
        rgb(&before, centre.0, centre.1),
        rgb(&after, centre.0, centre.1),
        "the sky should change when Space swaps natural for dark"
    );
    Ok(())
}

/// Every step of a processed folder draws (`PICTOR_UI_FOLDER`; skipped without it), and
/// the overlay key changes the Sky & ground canvas.
#[test]
fn every_step_of_a_processed_folder_draws() -> Result<()> {
    let Ok(folder) = std::env::var("PICTOR_UI_FOLDER") else {
        eprintln!("PICTOR_UI_FOLDER is not set; skipped");
        return Ok(());
    };
    let mut h = harness(Some(PathBuf::from(folder)), None);
    wait(&mut h, 300, Pictor::opened)?;
    for step in Step::ALL {
        h.state_mut().show_step(step);
        wait(&mut h, 60, Pictor::idle)?;
        let shot = h.render().map_err(error)?;
        save(&shot, &format!("step-{step:?}.jpg").to_lowercase())?;
        if step == Step::Sky {
            h.key_press(egui::Key::O);
            h.step();
            let plain = h.render().map_err(error)?;
            save(&plain, "step-sky-plain.jpg")?;
            assert_ne!(shot.as_raw(), plain.as_raw(), "O should toggle the overlay");
        }
    }
    Ok(())
}

/// A folder of RAW files opens on Import and is checked in the background
/// (`PICTOR_UI_RAW`; skipped without it).
#[test]
fn a_raw_folder_is_checked_before_anything_is_processed() -> Result<()> {
    let Ok(folder) = std::env::var("PICTOR_UI_RAW") else {
        eprintln!("PICTOR_UI_RAW is not set; skipped");
        return Ok(());
    };
    let mut h = harness(Some(PathBuf::from(folder)), None);
    h.step();
    assert!(!h.state().idle(), "the check should run in the background");
    let checking = h.render().map_err(error)?;
    save(&checking, "import-checking.jpg")?;
    wait(&mut h, 600, Pictor::idle)?;
    assert!(
        !h.state().opened(),
        "nothing is processed or opened before Start"
    );
    let ready = h.render().map_err(error)?;
    save(&ready, "import.jpg")?;
    Ok(())
}

/// A stopped run opens on Import with Resume; resuming shows the run's progress and Cancel
/// stops it again (`PICTOR_UI_STOPPED`, an output folder whose run stopped; skipped
/// without it).
#[test]
fn a_stopped_run_resumes_and_cancels_from_import() -> Result<()> {
    let Ok(folder) = std::env::var("PICTOR_UI_STOPPED") else {
        eprintln!("PICTOR_UI_STOPPED is not set; skipped");
        return Ok(());
    };
    let mut h = harness(Some(PathBuf::from(folder)), None);
    wait(&mut h, 60, Pictor::idle)?;
    let stopped = h.render().map_err(error)?;
    save(&stopped, "import-stopped.jpg")?;
    h.get_by_label("Resume").click();
    h.step();
    assert!(!h.state().idle(), "Resume should start the run");
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(8) {
        h.step();
        std::thread::sleep(Duration::from_millis(100));
    }
    let running = h.render().map_err(error)?;
    save(&running, "import-running.jpg")?;
    h.get_by_label("Cancel").click();
    wait(&mut h, 300, Pictor::idle)?;
    assert!(!h.state().opened(), "a cancelled run opens nothing");
    let cancelled = h.render().map_err(error)?;
    save(&cancelled, "import-cancelled.jpg")?;
    Ok(())
}

/// ⌘, opens Settings over the first screen and Cancel closes it without saving.
#[test]
fn settings_open_with_command_comma_and_cancel_closes_them() -> Result<()> {
    let mut h = harness(None, None);
    h.step();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Comma);
    h.step();
    h.step();
    save(&h.render().map_err(error)?, "settings.jpg")?;
    assert!(h.query_by_label("Save").is_some(), "Settings should open");
    h.get_by_label("Cancel").click();
    h.step();
    h.step();
    assert!(
        h.query_by_label("Save").is_none(),
        "Cancel should close Settings"
    );
    Ok(())
}

/// Settings links to Third-Party Notices, which lists the licences and closes with Esc.
#[test]
fn third_party_notices_open_from_settings() -> Result<()> {
    let mut h = harness(None, None);
    h.step();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Comma);
    h.step();
    h.get_by_label("Third-Party Notices…").click();
    for _ in 0..4 {
        h.step();
    }
    save(&h.render().map_err(error)?, "notices.jpg")?;
    assert!(h.query_by_label("Save").is_none(), "Settings should close");
    assert!(
        h.query_by_label_contains("Used by: ").is_some(),
        "the notices should list what each licence covers"
    );
    h.key_press(egui::Key::Escape);
    h.step();
    h.step();
    assert!(
        h.query_by_label("Close").is_none(),
        "Esc should close the notices"
    );
    Ok(())
}
