//! The memory a run may use and how many workers each parallel step gets within it. By default
//! a share of the memory free when the run starts and every core: a small machine runs fewer
//! workers (slower, same result), a large one uses every core. A step that cannot fit even one
//! worker fails with what it needs rather than swapping.

use crate::{Result, error};
use serde::Serialize;

/// Share of the free memory a run takes unless told otherwise, leaving room for other
/// applications.
pub const SHARE_OF_FREE: f64 = 0.75;
/// Memory held outside any worker, per frame pixel: the whole-image planes of the heaviest
/// stage (stacking keeps the sky stack, its sample counts and halves, the ground, the matte,
/// clearance, flat field and the composite at once).
pub const HELD_PER_PIXEL: u64 = 160;
const GB: f64 = 1e9;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Budget {
    pub bytes: u64,
    pub threads: usize,
}

fn cores() -> Result<usize> {
    Ok(std::thread::available_parallelism()?.get())
}

impl Budget {
    /// `gigabytes` and every core.
    pub fn of(gigabytes: f64) -> Result<Budget> {
        if !(gigabytes.is_finite() && gigabytes > 0.0) {
            return Err(error(format!(
                "memory budget {gigabytes} GB: give a positive size"
            )));
        }
        Ok(Budget {
            bytes: (gigabytes * GB) as u64,
            threads: cores()?,
        })
    }

    /// SHARE_OF_FREE of the memory free now, and every core.
    pub fn from_free() -> Result<Budget> {
        let mut system = sysinfo::System::new();
        system.refresh_memory();
        let free = system.available_memory();
        if free == 0 {
            return Err(error(
                "could not read the free memory; give a budget (--memory GB)",
            ));
        }
        Budget::of(free as f64 * SHARE_OF_FREE / GB)
    }

    /// Memory held outside workers for frames of `pixels` pixels.
    pub fn held(pixels: usize) -> u64 {
        HELD_PER_PIXEL * pixels as u64
    }

    /// Workers for `step`, which holds `held` bytes besides `each` bytes per worker.
    pub fn workers(&self, step: &str, held: u64, each: u64) -> Result<usize> {
        let room = self.bytes.saturating_sub(held);
        if room < each {
            return Err(error(format!(
                "{step} needs at least {:.1} GB and the memory budget is {:.1} GB; \
                 close other applications or give a larger budget (--memory GB)",
                (held + each) as f64 / GB,
                self.bytes as f64 / GB
            )));
        }
        Ok(((room / each.max(1)) as usize).clamp(1, self.threads))
    }

    /// A thread pool of `workers(step, held, each)` threads.
    pub fn pool(&self, step: &str, held: u64, each: u64) -> Result<rayon::ThreadPool> {
        Ok(rayon::ThreadPoolBuilder::new()
            .num_threads(self.workers(step, held, each)?)
            .build()?)
    }

    /// Bytes left for `step` after `held`, or the same failure as `workers`.
    pub fn room(&self, step: &str, held: u64) -> Result<u64> {
        self.workers(step, held, 1)?;
        Ok(self.bytes - held)
    }
}
