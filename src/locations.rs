use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationMemoryAnchor {
    pub path: Vec<usize>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrequentLocation {
    pub anchor: LocationMemoryAnchor,
    pub visits: usize,
    pub last_seen: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LocationMemoryState {
    pub frequent: Vec<FrequentLocation>,
}

pub fn locations_path_for(map_path: &Path) -> Result<PathBuf, AppError> {
    let file_name = map_path.file_name().ok_or_else(|| {
        AppError::new(format!(
            "Could not derive a locations file path from '{}'.",
            map_path.display()
        ))
    })?;

    let memory_name = format!(".{}.mdmind-locations.json", file_name.to_string_lossy());
    Ok(match map_path.parent() {
        Some(parent) => parent.join(memory_name),
        None => PathBuf::from(memory_name),
    })
}

pub fn load_locations_for(map_path: &Path) -> Result<LocationMemoryState, AppError> {
    let locations_path = locations_path_for(map_path)?;
    if !locations_path.exists() {
        return Ok(LocationMemoryState::default());
    }

    let contents = fs::read_to_string(&locations_path).map_err(|error| {
        AppError::new(format!(
            "Could not read locations '{}': {error}",
            locations_path.display()
        ))
    })?;

    serde_json::from_str(&contents).map_err(|error| {
        AppError::new(format!(
            "Could not parse locations '{}': {error}",
            locations_path.display()
        ))
    })
}

pub fn save_locations_for(map_path: &Path, state: &LocationMemoryState) -> Result<(), AppError> {
    let locations_path = locations_path_for(map_path)?;
    let contents = serde_json::to_string_pretty(state).expect("locations should serialize");
    fs::write(&locations_path, contents).map_err(|error| {
        AppError::new(format!(
            "Could not write locations '{}': {error}",
            locations_path.display()
        ))
    })
}
