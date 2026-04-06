use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::AppError;
use crate::editor::find_path_by_id;
use crate::model::Document;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionState {
    pub focus_path: Vec<usize>,
    pub focus_id: Option<String>,
}

pub fn session_path_for(map_path: &Path) -> Result<PathBuf, AppError> {
    let file_name = map_path.file_name().ok_or_else(|| {
        AppError::new(format!(
            "Could not derive a session file path from '{}'.",
            map_path.display()
        ))
    })?;

    let session_name = format!(".{}.mdmind-session.json", file_name.to_string_lossy());
    Ok(match map_path.parent() {
        Some(parent) => parent.join(session_name),
        None => PathBuf::from(session_name),
    })
}

pub fn load_session_for(map_path: &Path) -> Result<Option<SessionState>, AppError> {
    let session_path = session_path_for(map_path)?;
    if !session_path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&session_path).map_err(|error| {
        AppError::new(format!(
            "Could not read session state '{}': {error}",
            session_path.display()
        ))
    })?;

    let state = serde_json::from_str(&contents).map_err(|error| {
        AppError::new(format!(
            "Could not parse session state '{}': {error}",
            session_path.display()
        ))
    })?;

    Ok(Some(state))
}

pub fn save_session_for(map_path: &Path, state: &SessionState) -> Result<(), AppError> {
    let session_path = session_path_for(map_path)?;
    let contents = serde_json::to_string_pretty(state).expect("session serialization should work");
    fs::write(&session_path, contents).map_err(|error| {
        AppError::new(format!(
            "Could not write session state '{}': {error}",
            session_path.display()
        ))
    })
}

pub fn resolve_session_focus(document: &Document, state: &SessionState) -> Option<Vec<usize>> {
    if let Some(id) = &state.focus_id
        && let Some(path) = find_path_by_id(&document.nodes, id)
    {
        return Some(path);
    }

    if path_exists(document, &state.focus_path) {
        return Some(state.focus_path.clone());
    }

    None
}

fn path_exists(document: &Document, path: &[usize]) -> bool {
    if path.is_empty() {
        return document.nodes.is_empty();
    }

    let mut current_nodes = &document.nodes;
    for (index, segment) in path.iter().enumerate() {
        let Some(node) = current_nodes.get(*segment) else {
            return false;
        };

        if index + 1 == path.len() {
            return true;
        }

        current_nodes = &node.children;
    }

    false
}
