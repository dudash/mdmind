use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::AppError;
use crate::model::Document;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckpointViewMode {
    FullMap,
    FocusBranch,
    SubtreeOnly,
    FilteredFocus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointAnchor {
    pub path: Vec<usize>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub name: String,
    pub document: Document,
    pub focus_path: Vec<usize>,
    pub dirty: bool,
    pub expanded_paths: Vec<Vec<usize>>,
    pub view_mode: CheckpointViewMode,
    pub subtree_root: Option<CheckpointAnchor>,
    pub filter_query: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CheckpointsState {
    pub checkpoints: Vec<Checkpoint>,
}

pub fn checkpoints_path_for(map_path: &Path) -> Result<PathBuf, AppError> {
    let file_name = map_path.file_name().ok_or_else(|| {
        AppError::new(format!(
            "Could not derive a checkpoints file path from '{}'.",
            map_path.display()
        ))
    })?;

    let checkpoints_name = format!(".{}.mdmind-checkpoints.json", file_name.to_string_lossy());
    Ok(match map_path.parent() {
        Some(parent) => parent.join(checkpoints_name),
        None => PathBuf::from(checkpoints_name),
    })
}

pub fn load_checkpoints_for(map_path: &Path) -> Result<CheckpointsState, AppError> {
    let checkpoints_path = checkpoints_path_for(map_path)?;
    if !checkpoints_path.exists() {
        return Ok(CheckpointsState::default());
    }

    let contents = fs::read_to_string(&checkpoints_path).map_err(|error| {
        AppError::new(format!(
            "Could not read checkpoints '{}': {error}",
            checkpoints_path.display()
        ))
    })?;

    serde_json::from_str(&contents).map_err(|error| {
        AppError::new(format!(
            "Could not parse checkpoints '{}': {error}",
            checkpoints_path.display()
        ))
    })
}

pub fn save_checkpoints_for(map_path: &Path, state: &CheckpointsState) -> Result<(), AppError> {
    let checkpoints_path = checkpoints_path_for(map_path)?;
    let contents = serde_json::to_string_pretty(state).expect("checkpoints should serialize");
    fs::write(&checkpoints_path, contents).map_err(|error| {
        AppError::new(format!(
            "Could not write checkpoints '{}': {error}",
            checkpoints_path.display()
        ))
    })
}
