mod cli;

use star_photos::budget::Budget;
use star_photos::{Result, error, input, lens, output, pipeline, sequence, store};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// `--memory GB` taken out of `args`, wherever it appears.
fn memory(args: &mut Vec<OsString>) -> Result<Option<f64>> {
    let Some(at) = args.iter().position(|a| a == "--memory") else {
        return Ok(None);
    };
    if at + 1 >= args.len() {
        return Err(error("--memory needs a size in GB"));
    }
    let value = args.remove(at + 1);
    args.remove(at);
    let gb = value
        .to_string_lossy()
        .parse()
        .map_err(|e| error(format!("--memory: {e}")))?;
    Ok(Some(gb))
}

/// The files from FIRST to LAST (by file name), when given.
fn select(paths: Vec<PathBuf>, bounds: &[OsString]) -> Result<Vec<PathBuf>> {
    let index = |name: &OsString| {
        paths
            .iter()
            .position(|p| p.file_name() == Some(name.as_os_str()))
            .ok_or_else(|| error(format!("{} is not in the folder", name.to_string_lossy())))
    };
    let first = bounds.first().map(index).transpose()?.unwrap_or(0);
    let last = bounds
        .get(1)
        .map(index)
        .transpose()?
        .unwrap_or(paths.len() - 1);
    if first > last {
        return Err(error("FIRST comes after LAST"));
    }
    Ok(paths[first..=last].to_vec())
}

fn run() -> Result<()> {
    let mut args: Vec<_> = std::env::args_os().skip(1).collect();
    let budget = match memory(&mut args)? {
        Some(gb) => Budget::of(gb)?,
        None => Budget::from_free()?,
    };
    match args.as_slice() {
        [command, path] if command == "inspect" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&input::inspect(Path::new(path))?)?
            );
        }
        [command, path] if command == "levels" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&input::levels(Path::new(path))?)?
            );
        }
        [command, path, dir] if command == "develop" => {
            if Path::new(dir).exists() {
                return Err(error("output directory already exists"));
            }
            let start = std::time::Instant::now();
            let (raw, identity) = input::decode(Path::new(path))?;
            let (image, report) = pipeline::develop(&raw)?;
            drop(raw);
            output::save(Path::new(dir), &image, &identity, &report)?;
            println!(
                "{}",
                serde_json::json!({"width": image.width, "height": image.height,
                "elapsed_seconds": start.elapsed().as_secs_f64(), "output": dir.to_string_lossy(),
                "stage": "camera RGB diagnostic; not color-managed"})
            );
        }
        [command, dir] if command == "check" => {
            let (captures, steps) =
                sequence::capture_all(&input::raw_files(Path::new(dir))?, &budget)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&sequence::check(&captures, &steps)?)?
            );
        }
        [command, dir, out, rest @ ..] if command == "ingest" && rest.len() <= 2 => {
            let paths = select(input::raw_files(Path::new(dir))?, rest)?;
            let start = std::time::Instant::now();
            let store = store::ingest(&paths, Path::new(out), &budget)?;
            println!(
                "{}",
                serde_json::json!({"frames": store.manifest.frames.len(), "width": store.manifest.width,
                "height": store.manifest.height, "elapsed_seconds": start.elapsed().as_secs_f64()})
            );
        }
        [command, raw, out, rest @ ..] if command == "process" => {
            let report = cli::process(Path::new(raw), Path::new(out), rest, &budget)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        [command, dir, rest @ ..]
            if ["register", "separate", "stack", "hero", "moment"]
                .contains(&command.to_string_lossy().as_ref()) =>
        {
            let report = cli::stage(&command.to_string_lossy(), Path::new(dir), rest, &budget)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        [command, dir, planes, out, rest @ ..] if command == "render" => {
            let report = cli::render(
                Path::new(dir),
                &planes.to_string_lossy(),
                Path::new(out),
                rest,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        [command, path, rest @ ..] if command == "flat" && rest.len() <= 1 => {
            let info = input::inspect(Path::new(path))?;
            let lens = match rest {
                [name] => name.to_string_lossy().into_owned(),
                _ => lcc_raw::probe_info(&std::fs::read(path)?)?
                    .metadata
                    .lens_model
                    .ok_or_else(|| {
                        error("no lens model in the file; pass it: star-photos flat RAW \"LENS\"")
                    })?,
            };
            let (focal, aperture) = info
                .focal_length_mm
                .zip(info.aperture)
                .ok_or_else(|| error("no focal length or aperture"))?;
            let maker = info
                .make
                .as_deref()
                .and_then(|m| m.split_whitespace().next())
                .unwrap_or("");
            println!(
                "{}",
                serde_json::to_string_pretty(&lens::find(
                    &lens::database()?,
                    maker,
                    &lens,
                    focal,
                    aperture
                )?)?
            );
        }
        _ => {
            return Err(error(
                "usage: star-photos [--memory GB] process RAW_DIR NEW_OUTPUT_DIR [--run N] [--first NAME] [--last NAME] [--hero NAME] [--lens NAME] \
                 [--labels PLANES] [--gpu yes|NAME] [--matrix JSON] [--look natural|dark] | inspect INPUT | levels INPUT | develop INPUT NEW_OUTPUT_DIRECTORY | check RAW_DIR \
                 | ingest RAW_DIR NEW_STORE [FIRST [LAST]] | register STORE [--lens-from MODEL_JSON] \
                 | separate STORE [--labels PLANES] [--lens NAME] \
                 | stack STORE [--sigma PX] [--radius PX] [--gpu yes|NAME] [--lit NAME,NAME…] \
                 | hero STORE --hero NAME --first NAME [--foreground frame|long] [--lit NAME,…] [--labels PLANES] [--lens NAME] [--gpu yes|NAME] \
                 | moment STORE --centre NAME [--half N] \
                 | render STORE PLANES OUT_PREFIX [--matrix JSON] [--look natural|dark] [--side PX] [--single yes] [--linear yes] \
                 | flat INPUT [LENS]",
            ));
        }
    }
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("star-photos: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
