use serde::Serialize;

pub const CHANGELOG_MARKDOWN: &str = include_str!("../CHANGELOG.md");

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ChangelogEntry {
    pub version: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    pub body: String,
}

impl ChangelogEntry {
    pub fn is_unreleased(&self) -> bool {
        self.version.eq_ignore_ascii_case("unreleased")
    }

    pub fn has_material(&self) -> bool {
        self.body.lines().any(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && trimmed != "_Nothing yet._"
                && trimmed != "Nothing yet."
                && trimmed != "- Nothing yet."
        })
    }
}

pub fn changelog_entries() -> Vec<ChangelogEntry> {
    parse_changelog(CHANGELOG_MARKDOWN)
}

pub fn changelog_entry(version: &str) -> Option<ChangelogEntry> {
    let normalized = normalize_version(version);
    changelog_entries()
        .into_iter()
        .find(|entry| normalize_version(&entry.version) == normalized)
}

pub fn default_changelog_entry(app_version: &str) -> Option<ChangelogEntry> {
    let entries = changelog_entries();
    entries
        .iter()
        .find(|entry| normalize_version(&entry.version) == normalize_version(app_version))
        .cloned()
        .or_else(|| {
            entries
                .iter()
                .find(|entry| entry.is_unreleased() && entry.has_material())
                .cloned()
        })
        .or_else(|| entries.into_iter().find(ChangelogEntry::has_material))
}

pub fn render_changelog_entry(entry: &ChangelogEntry) -> String {
    let heading = match &entry.date {
        Some(date) => format!("## [{}] - {date}", entry.version),
        None => format!("## [{}]", entry.version),
    };
    let body = entry.body.trim();
    if body.is_empty() {
        heading
    } else {
        format!("{heading}\n\n{body}")
    }
}

pub fn render_full_changelog() -> String {
    CHANGELOG_MARKDOWN.trim_end().to_string()
}

fn parse_changelog(markdown: &str) -> Vec<ChangelogEntry> {
    let mut entries = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_body: Vec<String> = Vec::new();

    for line in markdown.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            push_entry(
                &mut entries,
                current_heading.take(),
                std::mem::take(&mut current_body),
            );
            current_heading = Some(heading.trim().to_string());
        } else if current_heading.is_some() {
            current_body.push(line.to_string());
        }
    }

    push_entry(&mut entries, current_heading, current_body);
    entries
}

fn push_entry(entries: &mut Vec<ChangelogEntry>, heading: Option<String>, body: Vec<String>) {
    let Some(heading) = heading else {
        return;
    };
    let (version, date) = parse_heading(&heading);
    let body = body.join("\n").trim().to_string();
    entries.push(ChangelogEntry {
        title: heading,
        version,
        date,
        body,
    });
}

fn parse_heading(heading: &str) -> (String, Option<String>) {
    let (version, rest) = if let Some(after_open) = heading.strip_prefix('[') {
        match after_open.split_once(']') {
            Some((version, rest)) => (version.trim().to_string(), rest.trim()),
            None => (heading.trim().to_string(), ""),
        }
    } else {
        match heading.split_once(" - ") {
            Some((version, rest)) => (version.trim().to_string(), rest.trim()),
            None => (heading.trim().to_string(), ""),
        }
    };

    let date = rest
        .strip_prefix('-')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            if rest.is_empty() {
                None
            } else {
                Some(rest.to_string())
            }
        });

    (version, date)
}

fn normalize_version(version: &str) -> String {
    version
        .trim()
        .strip_prefix('v')
        .unwrap_or_else(|| version.trim())
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versioned_changelog_sections() {
        let entries = parse_changelog(
            "# Changelog\n\n## [Unreleased]\n\n### Features\n\n- Work\n\n## [0.7.0] - 2026-05-13\n\n### Fixed\n\n- Bug\n",
        );

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].version, "Unreleased");
        assert_eq!(entries[0].date, None);
        assert_eq!(entries[1].version, "0.7.0");
        assert_eq!(entries[1].date.as_deref(), Some("2026-05-13"));
    }

    #[test]
    fn finds_versions_with_or_without_v_prefix() {
        let entry = changelog_entry("v0.7.0").expect("0.7.0 should exist");
        assert_eq!(entry.version, "0.7.0");
    }

    #[test]
    fn default_entry_prefers_the_build_version() {
        let entry = default_changelog_entry("0.7.0").expect("0.7.0 should exist");
        assert_eq!(entry.version, "0.7.0");
    }
}
