use std::cmp::Ordering;
use std::time::Duration;

use serde::{Deserialize, Serialize};

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/dudash/mdmind/releases/latest";
const UPDATE_CHECK_TIMEOUT: Duration = Duration::from_secs(4);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateCheck {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheckError {
    message: String,
}

impl UpdateCheckError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

pub fn check_for_updates(current_version: &str) -> Result<UpdateCheck, UpdateCheckError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(UPDATE_CHECK_TIMEOUT)
        .user_agent(format!("mdmind/{current_version}"))
        .build()
        .map_err(|error| {
            UpdateCheckError::new(format!("Could not create update client: {error}"))
        })?;
    let response = client.get(LATEST_RELEASE_URL).send().map_err(|error| {
        UpdateCheckError::new(format!("Could not reach GitHub releases: {error}"))
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(UpdateCheckError::new(format!(
            "GitHub releases returned HTTP {status}"
        )));
    }

    let body = response.text().map_err(|error| {
        UpdateCheckError::new(format!("Could not read GitHub response: {error}"))
    })?;
    let release = serde_json::from_str::<GitHubRelease>(&body).map_err(|error| {
        UpdateCheckError::new(format!("Could not read GitHub release data: {error}"))
    })?;
    Ok(update_check_from_release(
        current_version,
        &release.tag_name,
        &release.html_url,
    ))
}

fn update_check_from_release(
    current_version: &str,
    latest_tag: &str,
    release_url: &str,
) -> UpdateCheck {
    let latest_version = normalize_version_label(latest_tag);
    UpdateCheck {
        current_version: normalize_version_label(current_version),
        latest_version: latest_version.clone(),
        update_available: compare_versions(&latest_version, current_version) == Ordering::Greater,
        release_url: release_url.to_string(),
    }
}

fn normalize_version_label(version: &str) -> String {
    version
        .trim()
        .strip_prefix('v')
        .or_else(|| version.trim().strip_prefix('V'))
        .unwrap_or_else(|| version.trim())
        .to_string()
}

fn compare_versions(left: &str, right: &str) -> Ordering {
    let left_parts = version_numbers(left);
    let right_parts = version_numbers(right);
    let width = left_parts.len().max(right_parts.len()).max(3);
    for index in 0..width {
        let left = *left_parts.get(index).unwrap_or(&0);
        let right = *right_parts.get(index).unwrap_or(&0);
        match left.cmp(&right) {
            Ordering::Equal => {}
            ordering => return ordering,
        }
    }
    Ordering::Equal
}

fn version_numbers(version: &str) -> Vec<u64> {
    normalize_version_label(version)
        .split(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
        .next()
        .unwrap_or_default()
        .split('.')
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_release_versions_with_optional_v_prefix() {
        assert_eq!(compare_versions("v0.8.0", "0.7.0"), Ordering::Greater);
        assert_eq!(compare_versions("0.7.0", "v0.7.0"), Ordering::Equal);
        assert_eq!(compare_versions("0.7.0", "0.7.1"), Ordering::Less);
    }

    #[test]
    fn update_check_reports_when_latest_release_is_newer() {
        let check = update_check_from_release(
            "0.7.0",
            "v0.8.0",
            "https://github.com/dudash/mdmind/releases/tag/v0.8.0",
        );

        assert!(check.update_available);
        assert_eq!(check.current_version, "0.7.0");
        assert_eq!(check.latest_version, "0.8.0");
    }

    #[test]
    fn update_check_ignores_equivalent_latest_release() {
        let check = update_check_from_release(
            "0.7.0",
            "v0.7.0",
            "https://github.com/dudash/mdmind/releases/tag/v0.7.0",
        );

        assert!(!check.update_available);
        assert_eq!(check.latest_version, "0.7.0");
    }
}
