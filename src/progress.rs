//! A run's progress and its cancel flag, shared between the thread doing the work and whoever
//! shows it (the app's run view). Cheap to clone. A run plans its stages, each with a weight
//! (its usual share of the run's time); each stage names its steps and counts their items, and
//! a cancelled run fails at its next item. Time left is estimated from how long this run's
//! finished stages took against their weights.

use crate::{Result, error};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

/// The message of the error a cancelled run fails with.
pub const CANCELLED: &str = "cancelled";

#[derive(Clone, Default)]
pub struct Progress {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for Progress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Progress")
    }
}

#[derive(Default)]
struct Inner {
    cancelled: AtomicBool,
    done: AtomicU64,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    stages: Vec<Planned>,
    step: Option<(String, u64)>,
}

struct Planned {
    name: String,
    weight: f64,
    state: StageState,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum StageState {
    Waiting,
    Running {
        #[serde(skip)]
        since: Instant,
    },
    Done {
        seconds: f64,
    },
    /// Finished by an earlier, interrupted run of the same folder.
    Resumed {
        seconds: f64,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct StageView {
    pub name: String,
    #[serde(flatten)]
    pub state: StageState,
}

/// What a run is doing now.
#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub stages: Vec<StageView>,
    /// The running step and its items done and in all (0 when not counted).
    pub step: Option<String>,
    pub done: u64,
    pub total: u64,
    /// 0..=1 over the whole run.
    pub fraction: f64,
    /// Seconds left, once there is a measured rate.
    pub remaining_s: Option<f64>,
}

impl Progress {
    fn state(&self) -> MutexGuard<'_, State> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// The stages a run will go through, with their weights.
    pub fn plan(&self, stages: &[(&str, f64)]) {
        self.state().stages = stages
            .iter()
            .map(|(name, weight)| Planned {
                name: (*name).into(),
                weight: *weight,
                state: StageState::Waiting,
            })
            .collect();
    }

    fn set(&self, name: &str, state: StageState) {
        let mut s = self.state();
        match s.stages.iter_mut().find(|p| p.name == name) {
            Some(p) => p.state = state,
            None => s.stages.push(Planned {
                name: name.into(),
                weight: 0.0,
                state,
            }),
        }
        if !matches!(state, StageState::Running { .. }) {
            s.step = None;
        }
    }

    /// Renames a planned stage once the run knows which way it goes (the stack becomes a
    /// composite around the reference); a stage already known by the new name takes the
    /// planned one's weight.
    pub fn rename(&self, planned: &str, to: &str) {
        let mut s = self.state();
        let Some(at) = s.stages.iter().position(|p| p.name == planned) else {
            return;
        };
        match s.stages.iter().position(|p| p.name == to) {
            Some(existing) => {
                s.stages[existing].weight = s.stages[at].weight;
                s.stages.remove(at);
            }
            None => s.stages[at].name = to.into(),
        }
    }

    pub fn begin(&self, stage: &str) {
        self.set(
            stage,
            StageState::Running {
                since: Instant::now(),
            },
        );
    }

    pub fn finish(&self, stage: &str, seconds: f64) {
        self.set(stage, StageState::Done { seconds });
    }

    /// A stage an earlier run of the same folder finished.
    pub fn resumed(&self, stage: &str, seconds: f64) {
        self.set(stage, StageState::Resumed { seconds });
    }

    /// A step of the running stage, with `total` items (0 when they are not counted).
    pub fn step(&self, label: &str, total: usize) {
        self.inner.done.store(0, Ordering::Relaxed);
        self.state().step = Some((label.into(), total as u64));
    }

    /// One item of the step done; fails once the run is cancelled.
    pub fn advance(&self) -> Result<()> {
        self.inner.done.fetch_add(1, Ordering::Relaxed);
        self.check()
    }

    /// Fails once the run is cancelled.
    pub fn check(&self) -> Result<()> {
        if self.is_cancelled() {
            return Err(error(CANCELLED));
        }
        Ok(())
    }

    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> Snapshot {
        let s = self.state();
        let done = self.inner.done.load(Ordering::Relaxed);
        let (step, total) = s
            .step
            .clone()
            .map_or((None, 0), |(label, total)| (Some(label), total));
        let counted = (total > 0).then(|| (done as f64 / total as f64).min(1.0));
        let (fraction, remaining_s) = estimate(&s.stages, counted);
        Snapshot {
            stages: s
                .stages
                .iter()
                .map(|p| StageView {
                    name: p.name.clone(),
                    state: p.state,
                })
                .collect(),
            step,
            done,
            total,
            fraction,
            remaining_s,
        }
    }
}

/// The run's fraction done and its seconds left. The rate (seconds per unit of weight) comes
/// from this run's finished stages, or from the running stage's counted items before any has
/// finished.
fn estimate(stages: &[Planned], counted: Option<f64>) -> (f64, Option<f64>) {
    let all: f64 = stages.iter().map(|p| p.weight).sum();
    if all <= 0.0 {
        return (0.0, None);
    }
    let (mut measured_s, mut measured_w, mut finished_w) = (0.0, 0.0, 0.0);
    let mut running: Option<(f64, f64)> = None;
    for p in stages {
        match p.state {
            StageState::Done { seconds } => {
                measured_s += seconds;
                measured_w += p.weight;
                finished_w += p.weight;
            }
            StageState::Resumed { .. } => finished_w += p.weight,
            StageState::Running { since } => {
                running = Some((p.weight, since.elapsed().as_secs_f64()));
            }
            StageState::Waiting => {}
        }
    }
    let rate = if measured_w > 0.0 {
        Some(measured_s / measured_w)
    } else {
        match (running, counted) {
            (Some((w, elapsed)), Some(c)) if c > 0.02 && w > 0.0 => Some(elapsed / (c * w)),
            _ => None,
        }
    };
    let within = match (running, rate, counted) {
        (Some((w, elapsed)), Some(r), _) if w > 0.0 => (elapsed / (w * r)).min(0.95),
        (_, _, Some(c)) => c,
        _ => 0.0,
    };
    let current_w = running.map_or(0.0, |(w, _)| w);
    let fraction = ((finished_w + current_w * within) / all).clamp(0.0, 1.0);
    let left_w = all - finished_w - current_w * within;
    (fraction, rate.map(|r| (r * left_w).max(0.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cancelled_run_fails_at_its_next_item_and_estimates_from_finished_stages() {
        let p = Progress::default();
        p.plan(&[("a", 1.0), ("b", 3.0)]);
        p.finish("a", 10.0);
        p.begin("b");
        p.step("items", 4);
        assert!(p.advance().is_ok());
        let s = p.snapshot();
        assert_eq!((s.done, s.total), (1, 4));
        assert!((s.fraction - 0.25).abs() < 0.01, "{}", s.fraction);
        let left = s.remaining_s.unwrap_or(f64::NAN);
        assert!((left - 30.0).abs() < 1.0, "{left}");
        p.cancel();
        let e = p.advance().err().map(|e| e.to_string());
        assert_eq!(e.as_deref(), Some(CANCELLED));
    }
}
