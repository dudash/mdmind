use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedView {
    pub name: String,
    pub query: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SavedViewsState {
    pub views: Vec<SavedView>,
}

pub fn views_path_for(map_path: &Path) -> Result<PathBuf, AppError> {
    let file_name = map_path.file_name().ok_or_else(|| {
        AppError::new(format!(
            "Could not derive a saved-views file path from '{}'.",
            map_path.display()
        ))
    })?;

    let views_name = format!(".{}.mdmind-views.json", file_name.to_string_lossy());
    Ok(match map_path.parent() {
        Some(parent) => parent.join(views_name),
        None => PathBuf::from(views_name),
    })
}

pub fn load_views_for(map_path: &Path) -> Result<SavedViewsState, AppError> {
    let views_path = views_path_for(map_path)?;
    if !views_path.exists() {
        return Ok(SavedViewsState::default());
    }

    let contents = fs::read_to_string(&views_path).map_err(|error| {
        AppError::new(format!(
            "Could not read saved views '{}': {error}",
            views_path.display()
        ))
    })?;

    serde_json::from_str(&contents).map_err(|error| {
        AppError::new(format!(
            "Could not parse saved views '{}': {error}",
            views_path.display()
        ))
    })
}

pub fn save_views_for(map_path: &Path, state: &SavedViewsState) -> Result<(), AppError> {
    let views_path = views_path_for(map_path)?;
    let contents = serde_json::to_string_pretty(state).expect("saved views should serialize");
    fs::write(&views_path, contents).map_err(|error| {
        AppError::new(format!(
            "Could not write saved views '{}': {error}",
            views_path.display()
        ))
    })
}
