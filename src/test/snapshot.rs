use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct SnapshotManager {
    snapshot_dir: PathBuf,
    snapshots: HashMap<String, String>,
    update_mode: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct SnapshotFile {
    snapshots: HashMap<String, String>,
}

impl SnapshotManager {
    pub fn new(test_file: &Path, update_mode: bool) -> Result<Self> {
        let snapshot_dir = test_file
            .parent()
            .unwrap_or(Path::new("."))
            .join("__snapshots__");

        fs::create_dir_all(&snapshot_dir).with_context(|| {
            format!(
                "failed to create snapshot directory {}",
                snapshot_dir.display()
            )
        })?;

        let snapshot_file = snapshot_dir.join(format!(
            "{}.snap",
            test_file.file_stem().unwrap_or_default().to_string_lossy()
        ));

        let snapshots = if update_mode {
            HashMap::new()
        } else {
            Self::load_snapshots(&snapshot_file)?
        };

        Ok(Self {
            snapshot_dir,
            snapshots,
            update_mode,
        })
    }

    fn load_snapshots(snapshot_file: &Path) -> Result<HashMap<String, String>> {
        if !snapshot_file.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(snapshot_file)
            .with_context(|| format!("failed to read snapshot file {}", snapshot_file.display()))?;

        let snapshot_data: SnapshotFile =
            serde_json::from_str(&content).with_context(|| "failed to parse snapshot file")?;

        Ok(snapshot_data.snapshots)
    }

    pub fn save_snapshots(&self, test_file: &Path) -> Result<()> {
        if !self.update_mode {
            return Ok(());
        }

        let snapshot_file = self.snapshot_dir.join(format!(
            "{}.snap",
            test_file.file_stem().unwrap_or_default().to_string_lossy()
        ));

        let snapshot_data = SnapshotFile {
            snapshots: self.snapshots.clone(),
        };

        let content = serde_json::to_string_pretty(&snapshot_data)
            .context("failed to serialize snapshots")?;

        fs::write(&snapshot_file, content).with_context(|| {
            format!("failed to write snapshot file {}", snapshot_file.display())
        })?;

        Ok(())
    }

    pub fn assert_snapshot(&mut self, name: &str, value: &str) -> Result<SnapshotResult> {
        if self.update_mode {
            self.snapshots.insert(name.to_string(), value.to_string());
            return Ok(SnapshotResult::Updated);
        }

        match self.snapshots.get(name) {
            Some(expected) => {
                if expected == value {
                    Ok(SnapshotResult::Match)
                } else {
                    Ok(SnapshotResult::Mismatch {
                        expected: expected.clone(),
                        actual: value.to_string(),
                    })
                }
            }
            None => Ok(SnapshotResult::Missing {
                actual: value.to_string(),
            }),
        }
    }
}

#[derive(Debug)]
pub enum SnapshotResult {
    Match,
    Updated,
    Mismatch { expected: String, actual: String },
    Missing { actual: String },
}
