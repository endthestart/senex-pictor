//! The owner's settings, kept as JSON in the platform's settings folder: the GPU processing runs
//! on, the memory runs may use and where new runs are written. Screens read them through the
//! egui context, so a change applies to the next job.

mod window;

pub use window::Window;

use pictor_engine::budget::Budget;
use pictor_engine::progress::Progress;
use pictor_engine::{Result, error};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GpuChoice {
    /// The most capable GPU, or the CPU when there is none.
    Automatic,
    /// The CPU only.
    Off,
    /// The adapter whose name contains this.
    Named(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub gpu: GpuChoice,
    /// GB runs may use; a share of the memory free when a run starts when absent.
    pub memory_gb: Option<f64>,
    /// Where a RAW folder's run goes unless another is chosen; `~/Pictures/Pictor` when absent.
    pub output: Option<PathBuf>,
    /// Folders opened lately, newest first.
    #[serde(default)]
    pub recent: Vec<PathBuf>,
    /// The file they are kept in; none keeps them in memory only (headless tests).
    #[serde(skip)]
    file: Option<PathBuf>,
}

/// Folders kept in the recent list.
const RECENT: usize = 6;

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            gpu: GpuChoice::Automatic,
            memory_gb: None,
            output: None,
            recent: Vec::new(),
            file: None,
        }
    }
}

/// The settings file: `~/Library/Application Support/Pictor` on macOS, `%APPDATA%\Pictor` on
/// Windows, `$XDG_CONFIG_HOME/pictor` (or `~/.config/pictor`) elsewhere.
fn file() -> Result<PathBuf> {
    let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
    let home = || var("HOME").ok_or_else(|| error("HOME is not set"));
    let dir = if cfg!(target_os = "macos") {
        home()?.join("Library/Application Support/Pictor")
    } else if cfg!(windows) {
        var("APPDATA")
            .ok_or_else(|| error("APPDATA is not set"))?
            .join("Pictor")
    } else {
        var("XDG_CONFIG_HOME")
            .map_or_else(|| home().map(|h| h.join(".config")), Ok)?
            .join("pictor")
    };
    Ok(dir.join("settings.json"))
}

const KEY: &str = "pictor.settings";

impl Settings {
    /// The saved settings, kept in the platform's settings file; the defaults when none were
    /// saved.
    pub fn load() -> Result<Settings> {
        let path = file()?;
        let mut s: Settings = if path.exists() {
            serde_json::from_slice(&std::fs::read(&path)?)
                .map_err(|e| error(format!("{}: {e}", path.display())))?
        } else {
            Settings::default()
        };
        s.file = Some(path);
        Ok(s)
    }

    pub fn save(&self) -> Result<()> {
        let Some(path) = self.file.clone() else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }

    /// The settings the app holds.
    pub fn get(ctx: &egui::Context) -> Settings {
        ctx.data(|d| d.get_temp::<Settings>(egui::Id::new(KEY)))
            .unwrap_or_default()
    }

    pub fn put(self, ctx: &egui::Context) {
        ctx.data_mut(|d| d.insert_temp(egui::Id::new(KEY), self));
    }

    /// `path` first in the recent list, saved.
    pub fn opened(&mut self, path: &std::path::Path) -> Result<()> {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_path_buf());
        self.recent.truncate(RECENT);
        self.save()
    }

    /// A run's memory budget, carrying `progress`.
    pub fn budget(&self, progress: Progress) -> Result<Budget> {
        let mut b = match self.memory_gb {
            Some(gb) => Budget::of(gb)?,
            None => Budget::from_free()?,
        };
        b.progress = progress;
        Ok(b)
    }

    /// The GPU a run uses: none when off, the named one (failing when it is missing), or the
    /// most capable, falling back to the CPU with a note on stderr.
    pub fn gpu(&self) -> Result<Option<pictor_engine::gpu::Gpu>> {
        use pictor_engine::gpu::Gpu;
        match &self.gpu {
            GpuChoice::Off => Ok(None),
            GpuChoice::Named(name) => Ok(Some(Gpu::new(Some(name))?)),
            GpuChoice::Automatic => match Gpu::new(None) {
                Ok(g) => Ok(Some(g)),
                Err(e) => {
                    eprintln!("pictor: no GPU for processing ({e}); the CPU does it");
                    Ok(None)
                }
            },
        }
    }
}
