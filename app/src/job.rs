//! Work that takes seconds to hours (checking a folder, processing it, making a hero
//! composite, exporting) on its own thread, with the engine's progress and cancel flag shared
//! with the window.

use pictor_engine::Result;
use pictor_engine::progress::{Progress, Snapshot};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::Instant;

pub struct Job<T> {
    /// What is being done, for the status line: "Checking the folder".
    pub label: String,
    pub progress: Progress,
    pub started: Instant,
    rx: Receiver<Result<T>>,
}

impl<T: Send + 'static> Job<T> {
    pub fn start(
        label: impl Into<String>,
        work: impl FnOnce(Progress) -> Result<T> + Send + 'static,
    ) -> Job<T> {
        let progress = Progress::default();
        let (tx, rx) = channel();
        let shared = progress.clone();
        std::thread::spawn(move || {
            let _ = tx.send(work(shared));
        });
        Job {
            label: label.into(),
            progress,
            started: Instant::now(),
            rx,
        }
    }

    /// The result once the work has ended.
    pub fn poll(&self) -> Option<Result<T>> {
        match self.rx.try_recv() {
            Ok(r) => Some(r),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(pictor_engine::error(format!(
                "{} stopped without a result",
                self.label
            )))),
        }
    }

    pub fn cancel(&self) {
        self.progress.cancel();
    }

    pub fn snapshot(&self) -> Snapshot {
        self.progress.snapshot()
    }

    pub fn cancelled(&self) -> bool {
        self.progress.is_cancelled()
    }
}
