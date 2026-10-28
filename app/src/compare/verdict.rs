//! Per-region verdicts between the two renderings, kept in the project folder as `compare.json`
//! so a comparison survives closing the app. Verdicts belong to a pair of renderings; a
//! different pair starts afresh.

use serde::{Deserialize, Serialize};
use star_photos::Result;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FILE: &str = "compare.json";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Choice {
    First,
    Same,
    Second,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Verdicts {
    pub first: String,
    pub second: String,
    pub regions: BTreeMap<String, Choice>,
    #[serde(skip)]
    file: Option<PathBuf>,
}

impl Verdicts {
    /// The verdicts saved in `dir` for this pair, or none yet; without a folder (the demo
    /// scene) nothing is kept.
    pub fn load(dir: Option<&Path>, first: &str, second: &str) -> Result<Verdicts> {
        let file = dir.map(|d| d.join(FILE));
        let mut v = match &file {
            Some(f) if f.exists() => serde_json::from_slice(&std::fs::read(f)?)?,
            _ => Verdicts::default(),
        };
        if v.first != first || v.second != second {
            v = Verdicts {
                first: first.into(),
                second: second.into(),
                ..Verdicts::default()
            };
        }
        v.file = file;
        Ok(v)
    }

    pub fn get(&self, region: &str) -> Option<Choice> {
        self.regions.get(region).copied()
    }

    /// Records `choice` for `region` (or clears it when chosen again) and saves.
    pub fn set(&mut self, region: &str, choice: Choice) -> Result<()> {
        if self.get(region) == Some(choice) {
            self.regions.remove(region);
        } else {
            self.regions.insert(region.into(), choice);
        }
        match &self.file {
            Some(f) => Ok(std::fs::write(f, serde_json::to_vec_pretty(self)?)?),
            None => Ok(()),
        }
    }

    /// Regions where the first is better, the same, the second is better.
    pub fn tally(&self) -> [usize; 3] {
        let count = |c: Choice| self.regions.values().filter(|&&v| v == c).count();
        [
            count(Choice::First),
            count(Choice::Same),
            count(Choice::Second),
        ]
    }

    pub fn saved(&self) -> bool {
        self.file.is_some()
    }
}
