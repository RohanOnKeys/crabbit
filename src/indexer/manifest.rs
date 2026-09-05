use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::CrabbitError;

/// Tracks which segments are live and the next id to hand out.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub segments: Vec<u32>,
    pub next_segment_id: u32,
}

impl Manifest {
    pub fn empty() -> Self {
        Self { segments: Vec::new(), next_segment_id: 0 }
    }

    pub fn load(path: &Path) -> Result<Self, CrabbitError> {
        if !path.exists() {
            return Ok(Self::empty());
        }
        Ok(serde_json::from_slice(&std::fs::read(path)?)?)
    }

    pub fn save(&self, path: &Path) -> Result<(), CrabbitError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

pub fn manifest_path(data_dir: &Path) -> PathBuf {
    data_dir.join("manifest.json")
}

pub fn segment_dir(data_dir: &Path, segment_id: u32) -> PathBuf {
    data_dir.join("segments").join(format!("seg_{segment_id:06}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_manifest_loads_as_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest = Manifest::load(&manifest_path(tmp.path())).unwrap();
        assert!(manifest.segments.is_empty());
        assert_eq!(manifest.next_segment_id, 0);
    }

    #[test]
    fn save_and_load_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = manifest_path(tmp.path());
        let manifest = Manifest { segments: vec![0, 1, 2], next_segment_id: 3 };
        manifest.save(&path).unwrap();
        let loaded = Manifest::load(&path).unwrap();
        assert_eq!(loaded.segments, vec![0, 1, 2]);
        assert_eq!(loaded.next_segment_id, 3);
    }
}
