use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::app::{AppError, ClassifiedTarget, OpenTargetMode, classify_open_target};
use crate::model::{Diagnostic, Document, Node, Severity, TaskState};

pub const MINDSPACE_SCAN_FORMAT: &str = "mindspace_scan.v1";
pub const MINDSPACE_DIAGNOSTICS_FORMAT: &str = "mindspace_diagnostics.v1";
const MANIFEST_SCHEMA_VERSION: &str = "mdmind.mindspace.v1";

#[derive(Debug, Clone, Serialize)]
pub struct MindspaceScan {
    pub root: String,
    pub manifest: MindspaceManifestStatus,
    pub summary: MindspaceSummary,
    pub roles: Vec<MindspaceRoleRecord>,
    pub maps: Vec<MindspaceMapRecord>,
    pub diagnostics: Vec<MindspaceDiagnostic>,
    pub skipped: Vec<MindspaceSkippedPath>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MindspaceManifestStatus {
    pub present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<usize>,
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MindspaceSummary {
    pub files_scanned: usize,
    pub directories_scanned: usize,
    pub roles: MindspaceRoleCounts,
    pub diagnostics: MindspaceDiagnosticCounts,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct MindspaceRoleCounts {
    pub maps: usize,
    pub pages: usize,
    pub sources: usize,
    pub inbox: usize,
    pub indexes: usize,
    pub logs: usize,
    pub instructions: usize,
    pub reports: usize,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct MindspaceDiagnosticCounts {
    pub errors: usize,
    pub warnings: usize,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MindspaceRoleRecord {
    pub role: MindspaceRole,
    pub path: String,
    pub kind: MindspaceEntryKind,
    pub read_only: bool,
    pub generated: bool,
    pub append_only: bool,
    pub trusted: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MindspaceRole {
    Map,
    Page,
    Source,
    Inbox,
    Index,
    Log,
    Instruction,
    Report,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MindspaceEntryKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize)]
pub struct MindspaceMapRecord {
    pub path: String,
    pub parse_status: MindspaceMapParseStatus,
    pub validation: MindspaceDiagnosticCounts,
    pub stats: MindspaceMapStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MindspaceMapParseStatus {
    Ok,
    Errors,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MindspaceMapStats {
    pub nodes: usize,
    pub ids: Vec<String>,
    pub tags: Vec<String>,
    pub metadata_keys: Vec<String>,
    pub references: usize,
    pub relations: usize,
    pub open_tasks: usize,
    pub done_tasks: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MindspaceDiagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MindspaceSkippedPath {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MindspaceDiagnosticsReport {
    pub root: String,
    pub summary: MindspaceDiagnosticCounts,
    pub diagnostics: Vec<MindspaceDiagnostic>,
}

impl MindspaceScan {
    pub fn diagnostics_report(&self) -> MindspaceDiagnosticsReport {
        MindspaceDiagnosticsReport {
            root: self.root.clone(),
            summary: self.summary.diagnostics,
            diagnostics: self.diagnostics.clone(),
        }
    }

    pub fn has_error_diagnostics(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }
}

pub fn scan_mindspace(root: &Path) -> Result<MindspaceScan, AppError> {
    let canonical_root = root.canonicalize().map_err(|error| {
        AppError::new(format!(
            "Could not read mindspace root '{}': {error}",
            root.display()
        ))
    })?;

    if !canonical_root.is_dir() {
        return Err(AppError::new(format!(
            "Mindspace root '{}' is not a directory.",
            canonical_root.display()
        )));
    }

    let mut roles = Vec::new();
    let mut maps = Vec::new();
    let mut diagnostics = Vec::new();
    let mut skipped = Vec::new();
    let mut counters = ScanCounters::default();
    let manifest = inspect_manifest(&canonical_root, &mut diagnostics);

    walk_directory(
        &canonical_root,
        &canonical_root,
        &mut roles,
        &mut maps,
        &mut diagnostics,
        &mut skipped,
        &mut counters,
    );

    roles.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| role_name(left.role).cmp(role_name(right.role)))
    });
    maps.sort_by(|left, right| left.path.cmp(&right.path));
    diagnostics.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.code.cmp(right.code))
    });
    skipped.sort_by(|left, right| left.path.cmp(&right.path));

    let role_counts = count_roles(&roles);
    let diagnostic_counts = count_diagnostics(&diagnostics);
    Ok(MindspaceScan {
        root: canonical_root.to_string_lossy().to_string(),
        manifest,
        summary: MindspaceSummary {
            files_scanned: counters.files,
            directories_scanned: counters.directories,
            roles: role_counts,
            diagnostics: diagnostic_counts,
        },
        roles,
        maps,
        diagnostics,
        skipped,
    })
}

pub fn render_mindspace_scan(scan: &MindspaceScan) -> String {
    let mut lines = Vec::new();
    lines.push(format!("Mindspace scan: {}", scan.root));
    lines.push(format!(
        "Manifest: {}",
        if scan.manifest.present {
            if scan.manifest.valid {
                "present"
            } else {
                "present with issues"
            }
        } else {
            "missing"
        }
    ));
    lines.push(format!(
        "Roles: {} maps, {} pages, {} sources, {} inbox, {} indexes, {} logs, {} instructions, {} reports",
        scan.summary.roles.maps,
        scan.summary.roles.pages,
        scan.summary.roles.sources,
        scan.summary.roles.inbox,
        scan.summary.roles.indexes,
        scan.summary.roles.logs,
        scan.summary.roles.instructions,
        scan.summary.roles.reports
    ));
    lines.push(format!(
        "Diagnostics: {} errors, {} warnings",
        scan.summary.diagnostics.errors, scan.summary.diagnostics.warnings
    ));

    if !scan.maps.is_empty() {
        lines.push(String::new());
        lines.push("Maps:".to_string());
        for map in &scan.maps {
            lines.push(format!(
                "  {:<6} {} ({} nodes, {} ids, {} refs, {} relations)",
                parse_status_name(map.parse_status),
                map.path,
                map.stats.nodes,
                map.stats.ids.len(),
                map.stats.references,
                map.stats.relations
            ));
        }
    }

    let supporting_roles = scan
        .roles
        .iter()
        .filter(|record| record.role != MindspaceRole::Map)
        .collect::<Vec<_>>();
    if !supporting_roles.is_empty() {
        lines.push(String::new());
        lines.push("Workspace roles:".to_string());
        for record in supporting_roles {
            lines.push(format!(
                "  {:<11} {:<9} {}",
                role_name(record.role),
                entry_kind_name(record.kind),
                record.path
            ));
        }
    }

    if !scan.diagnostics.is_empty() {
        lines.push(String::new());
        lines.push("Diagnostics:".to_string());
        for diagnostic in &scan.diagnostics {
            lines.push(format_diagnostic(diagnostic));
        }
    }

    if !scan.skipped.is_empty() {
        lines.push(String::new());
        lines.push("Skipped:".to_string());
        for skipped in &scan.skipped {
            lines.push(format!("  {} - {}", skipped.path, skipped.reason));
        }
    }

    lines.join("\n")
}

pub fn render_mindspace_scan_plain(scan: &MindspaceScan) -> String {
    scan.roles
        .iter()
        .map(|record| {
            format!(
                "{}\t{}\t{}\t{}",
                role_name(record.role),
                entry_kind_name(record.kind),
                record.path,
                record.reason
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn render_mindspace_diagnostics(report: &MindspaceDiagnosticsReport) -> String {
    if report.diagnostics.is_empty() {
        return "No deterministic mindspace diagnostics.".to_string();
    }

    let mut lines = vec![format!(
        "Mindspace diagnostics: {} errors, {} warnings",
        report.summary.errors, report.summary.warnings
    )];
    for diagnostic in &report.diagnostics {
        lines.push(format_diagnostic(diagnostic));
    }
    lines.join("\n")
}

pub fn render_mindspace_diagnostics_plain(report: &MindspaceDiagnosticsReport) -> String {
    report
        .diagnostics
        .iter()
        .map(|diagnostic| {
            format!(
                "{}\t{}\t{}\t{}\t{}",
                severity_name(&diagnostic.severity),
                diagnostic.code,
                diagnostic.path,
                diagnostic
                    .line
                    .map(|line| line.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                diagnostic.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn inspect_manifest(
    root: &Path,
    diagnostics: &mut Vec<MindspaceDiagnostic>,
) -> MindspaceManifestStatus {
    let manifest_path = root.join(".mdmind").join("mindspace.json");
    let display_path = display_path(root, &manifest_path);
    let source = match fs::read_to_string(&manifest_path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return MindspaceManifestStatus {
                present: false,
                path: None,
                schema_version: None,
                name: None,
                roles: None,
                valid: false,
            };
        }
        Err(error) => {
            diagnostics.push(MindspaceDiagnostic {
                code: "manifest_read_failed",
                severity: Severity::Error,
                path: display_path.clone(),
                line: None,
                message: format!("Could not read manifest: {error}"),
            });
            return MindspaceManifestStatus {
                present: true,
                path: Some(display_path),
                schema_version: None,
                name: None,
                roles: None,
                valid: false,
            };
        }
    };

    let value = match serde_json::from_str::<Value>(&source) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(MindspaceDiagnostic {
                code: "manifest_invalid_json",
                severity: Severity::Error,
                path: display_path.clone(),
                line: Some(error.line()),
                message: format!("Manifest is not valid JSON: {error}"),
            });
            return MindspaceManifestStatus {
                present: true,
                path: Some(display_path),
                schema_version: None,
                name: None,
                roles: None,
                valid: false,
            };
        }
    };

    let schema_version = value
        .get("schema_version")
        .and_then(Value::as_str)
        .map(str::to_string);
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_string);
    let roles = value.get("roles").and_then(Value::as_array).map(Vec::len);
    let mut valid = true;

    if schema_version.as_deref() != Some(MANIFEST_SCHEMA_VERSION) {
        valid = false;
        diagnostics.push(MindspaceDiagnostic {
            code: "manifest_schema_version",
            severity: Severity::Warning,
            path: display_path.clone(),
            line: None,
            message: format!("Manifest schema_version should be {MANIFEST_SCHEMA_VERSION}."),
        });
    }
    if roles.is_none() {
        valid = false;
        diagnostics.push(MindspaceDiagnostic {
            code: "manifest_roles_missing",
            severity: Severity::Warning,
            path: display_path.clone(),
            line: None,
            message: "Manifest roles should be an array.".to_string(),
        });
    }

    MindspaceManifestStatus {
        present: true,
        path: Some(display_path),
        schema_version,
        name,
        roles,
        valid,
    }
}

fn walk_directory(
    root: &Path,
    directory: &Path,
    roles: &mut Vec<MindspaceRoleRecord>,
    maps: &mut Vec<MindspaceMapRecord>,
    diagnostics: &mut Vec<MindspaceDiagnostic>,
    skipped: &mut Vec<MindspaceSkippedPath>,
    counters: &mut ScanCounters,
) {
    counters.directories += 1;

    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            diagnostics.push(MindspaceDiagnostic {
                code: "directory_read_failed",
                severity: Severity::Warning,
                path: display_path(root, directory),
                line: None,
                message: format!("Could not read directory: {error}"),
            });
            return;
        }
    };

    let mut entries = entries
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry),
            Err(error) => {
                diagnostics.push(MindspaceDiagnostic {
                    code: "directory_entry_read_failed",
                    severity: Severity::Warning,
                    path: display_path(root, directory),
                    line: None,
                    message: format!("Could not inspect directory entry: {error}"),
                });
                None
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                diagnostics.push(MindspaceDiagnostic {
                    code: "file_type_read_failed",
                    severity: Severity::Warning,
                    path: display_path(root, &path),
                    line: None,
                    message: format!("Could not inspect file type: {error}"),
                });
                continue;
            }
        };

        if file_type.is_dir() {
            if let Some(record) = directory_role_record(root, &path) {
                roles.push(record);
            }
            if should_skip_directory(root, &path) {
                skipped.push(MindspaceSkippedPath {
                    path: display_path(root, &path),
                    reason: "ignored generated or dependency directory".to_string(),
                });
                continue;
            }
            walk_directory(root, &path, roles, maps, diagnostics, skipped, counters);
        } else if file_type.is_file() {
            counters.files += 1;
            inspect_file(root, &path, roles, maps, diagnostics);
        }
    }
}

fn inspect_file(
    root: &Path,
    path: &Path,
    roles: &mut Vec<MindspaceRoleRecord>,
    maps: &mut Vec<MindspaceMapRecord>,
    diagnostics: &mut Vec<MindspaceDiagnostic>,
) {
    if is_manifest_file(root, path) {
        return;
    }

    if let Some(record) = explicit_file_role_record(root, path) {
        roles.push(record);
        return;
    }

    if let Some(record) = inherited_role_record(root, path, MindspaceEntryKind::File) {
        roles.push(record);
        return;
    }

    if !is_markdown_path(path) {
        return;
    }

    let raw_path = path.to_string_lossy();
    match classify_open_target(&raw_path, OpenTargetMode::Auto) {
        Ok(ClassifiedTarget::NativeMap(loaded)) => {
            let path_display = display_path(root, path);
            roles.push(role_record(
                MindspaceRole::Map,
                MindspaceEntryKind::File,
                path_display.clone(),
                "valid native mdmind map",
            ));
            let mut diagnostics_for_map = map_diagnostics(
                &path_display,
                &loaded.parser_diagnostics,
                "map_parse_warning",
                "map_parse_error",
            );
            diagnostics_for_map.extend(map_diagnostics(
                &path_display,
                &loaded.validation_diagnostics,
                "map_validation_warning",
                "map_validation_error",
            ));
            diagnostics.extend(diagnostics_for_map.clone());
            maps.push(MindspaceMapRecord {
                path: path_display,
                parse_status: MindspaceMapParseStatus::Ok,
                validation: count_diagnostics(&diagnostics_for_map),
                stats: collect_map_stats(&loaded.document),
            });
        }
        Ok(ClassifiedTarget::NearMissMap {
            diagnostics: parser_diagnostics,
            ..
        }) if should_classify_near_miss_as_map(root, path) => {
            let path_display = display_path(root, path);
            roles.push(role_record(
                MindspaceRole::Map,
                MindspaceEntryKind::File,
                path_display.clone(),
                "damaged native mdmind map",
            ));
            let map_diagnostics = map_diagnostics(
                &path_display,
                &parser_diagnostics,
                "map_parse_warning",
                "map_parse_error",
            );
            diagnostics.extend(map_diagnostics.clone());
            maps.push(MindspaceMapRecord {
                path: path_display,
                parse_status: MindspaceMapParseStatus::Errors,
                validation: count_diagnostics(&map_diagnostics),
                stats: MindspaceMapStats::default(),
            });
        }
        Ok(ClassifiedTarget::NearMissMap { .. }) => {
            roles.push(role_record(
                MindspaceRole::Page,
                MindspaceEntryKind::File,
                display_path(root, path),
                "ordinary Markdown page with mdmind examples",
            ));
        }
        Ok(ClassifiedTarget::OrdinaryMarkdown { .. }) => {
            roles.push(role_record(
                MindspaceRole::Page,
                MindspaceEntryKind::File,
                display_path(root, path),
                "ordinary Markdown page",
            ));
        }
        Err(error) => {
            diagnostics.push(MindspaceDiagnostic {
                code: "file_read_failed",
                severity: Severity::Warning,
                path: display_path(root, path),
                line: None,
                message: error.message().to_string(),
            });
        }
    }
}

fn directory_role_record(root: &Path, path: &Path) -> Option<MindspaceRoleRecord> {
    inherited_container_role(root, path).map(|role| {
        role_record(
            role,
            MindspaceEntryKind::Directory,
            display_path(root, path),
            inherited_role_reason(role),
        )
    })
}

fn explicit_file_role_record(root: &Path, path: &Path) -> Option<MindspaceRoleRecord> {
    let file_name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
    let role = match file_name.as_str() {
        "agents.md" | "claude.md" | "gemini.md" => MindspaceRole::Instruction,
        "index.md" => MindspaceRole::Index,
        "log.md" | "activity.md" | "journal.md" => MindspaceRole::Log,
        _ => return None,
    };
    Some(role_record(
        role,
        MindspaceEntryKind::File,
        display_path(root, path),
        explicit_role_reason(role),
    ))
}

fn inherited_role_record(
    root: &Path,
    path: &Path,
    kind: MindspaceEntryKind,
) -> Option<MindspaceRoleRecord> {
    inherited_container_role(root, path).map(|role| {
        role_record(
            role,
            kind,
            display_path(root, path),
            inherited_role_reason(role),
        )
    })
}

fn inherited_container_role(root: &Path, path: &Path) -> Option<MindspaceRole> {
    let relative = path.strip_prefix(root).ok()?;
    let parts = relative
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => Some(part.to_string_lossy().to_ascii_lowercase()),
            _ => None,
        })
        .collect::<Vec<_>>();

    if parts.len() >= 2 && parts[0] == ".mdmind" && parts[1] == "reports" {
        return Some(MindspaceRole::Report);
    }

    for part in parts {
        match part.as_str() {
            "source" | "sources" | "raw" | "references" => return Some(MindspaceRole::Source),
            "inbox" | "_inbox" => return Some(MindspaceRole::Inbox),
            _ => {}
        }
    }

    None
}

fn role_record(
    role: MindspaceRole,
    kind: MindspaceEntryKind,
    path: String,
    reason: impl Into<String>,
) -> MindspaceRoleRecord {
    MindspaceRoleRecord {
        role,
        path,
        kind,
        read_only: role == MindspaceRole::Source,
        generated: role == MindspaceRole::Report || role == MindspaceRole::Index,
        append_only: role == MindspaceRole::Log,
        trusted: role == MindspaceRole::Instruction,
        reason: reason.into(),
    }
}

fn should_skip_directory(root: &Path, path: &Path) -> bool {
    let Some(name) = path
        .file_name()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
    else {
        return false;
    };

    if matches!(
        name.as_str(),
        ".git" | "target" | "node_modules" | ".venv" | "dist" | "build"
    ) {
        return true;
    }

    if name.starts_with('.') && name != ".mdmind" {
        return true;
    }

    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    let parts = relative
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => Some(part.to_string_lossy().to_ascii_lowercase()),
            _ => None,
        })
        .collect::<Vec<_>>();
    parts.len() >= 2 && parts[0] == ".mdmind" && parts[1] != "reports"
}

fn is_manifest_file(root: &Path, path: &Path) -> bool {
    path == root.join(".mdmind").join("mindspace.json")
}

fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "mdown" | "mkd"
            )
        })
        .unwrap_or(false)
}

fn should_classify_near_miss_as_map(root: &Path, path: &Path) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let in_map_container = relative.components().any(|component| match component {
        std::path::Component::Normal(part) => {
            matches!(
                part.to_string_lossy().to_ascii_lowercase().as_str(),
                "map" | "maps" | "mindmap" | "mindmaps" | "outline" | "outlines"
            )
        }
        _ => false,
    });
    if in_map_container {
        return true;
    }

    let Some(stem) = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_ascii_lowercase())
    else {
        return false;
    };
    [
        "map", "roadmap", "todo", "tasks", "backlog", "plan", "outline",
    ]
    .iter()
    .any(|hint| stem.contains(hint))
}

fn display_path(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let text = relative
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    if text.is_empty() {
        ".".to_string()
    } else {
        text
    }
}

fn map_diagnostics(
    path: &str,
    diagnostics: &[Diagnostic],
    warning_code: &'static str,
    error_code: &'static str,
) -> Vec<MindspaceDiagnostic> {
    diagnostics
        .iter()
        .map(|diagnostic| MindspaceDiagnostic {
            code: if diagnostic.severity == Severity::Error {
                error_code
            } else {
                warning_code
            },
            severity: diagnostic.severity.clone(),
            path: path.to_string(),
            line: Some(diagnostic.line),
            message: diagnostic.message.clone(),
        })
        .collect()
}

fn collect_map_stats(document: &Document) -> MindspaceMapStats {
    let mut stats = MapStatsCollector::default();
    for node in &document.nodes {
        collect_node_stats(node, &mut stats);
    }

    MindspaceMapStats {
        nodes: stats.nodes,
        ids: stats.ids.into_iter().collect(),
        tags: stats.tags.into_iter().collect(),
        metadata_keys: stats.metadata_keys.into_iter().collect(),
        references: stats.references,
        relations: stats.relations,
        open_tasks: stats.open_tasks,
        done_tasks: stats.done_tasks,
    }
}

fn collect_node_stats(node: &Node, stats: &mut MapStatsCollector) {
    stats.nodes += 1;
    if let Some(id) = &node.id {
        stats.ids.insert(id.clone());
    }
    for tag in &node.tags {
        stats.tags.insert(tag.clone());
    }
    for metadata in &node.metadata {
        stats.metadata_keys.insert(metadata.key.clone());
    }
    stats.references += node.references.len();
    stats.relations += node.relations.len();
    match node.task {
        Some(TaskState::Open) => stats.open_tasks += 1,
        Some(TaskState::Done) => stats.done_tasks += 1,
        None => {}
    }
    for child in &node.children {
        collect_node_stats(child, stats);
    }
}

fn count_roles(roles: &[MindspaceRoleRecord]) -> MindspaceRoleCounts {
    let mut counts = MindspaceRoleCounts::default();
    for record in roles {
        match record.role {
            MindspaceRole::Map => counts.maps += 1,
            MindspaceRole::Page => counts.pages += 1,
            MindspaceRole::Source => counts.sources += 1,
            MindspaceRole::Inbox => counts.inbox += 1,
            MindspaceRole::Index => counts.indexes += 1,
            MindspaceRole::Log => counts.logs += 1,
            MindspaceRole::Instruction => counts.instructions += 1,
            MindspaceRole::Report => counts.reports += 1,
        }
    }
    counts
}

fn count_diagnostics(diagnostics: &[MindspaceDiagnostic]) -> MindspaceDiagnosticCounts {
    let mut counts = MindspaceDiagnosticCounts::default();
    for diagnostic in diagnostics {
        counts.count += 1;
        match &diagnostic.severity {
            Severity::Error => counts.errors += 1,
            Severity::Warning => counts.warnings += 1,
        }
    }
    counts
}

fn format_diagnostic(diagnostic: &MindspaceDiagnostic) -> String {
    let line = diagnostic
        .line
        .map(|line| format!(":{line}"))
        .unwrap_or_default();
    format!(
        "  {} {}{} {} - {}",
        severity_name(&diagnostic.severity),
        diagnostic.path,
        line,
        diagnostic.code,
        diagnostic.message
    )
}

fn role_name(role: MindspaceRole) -> &'static str {
    match role {
        MindspaceRole::Map => "map",
        MindspaceRole::Page => "page",
        MindspaceRole::Source => "source",
        MindspaceRole::Inbox => "inbox",
        MindspaceRole::Index => "index",
        MindspaceRole::Log => "log",
        MindspaceRole::Instruction => "instruction",
        MindspaceRole::Report => "report",
    }
}

fn parse_status_name(status: MindspaceMapParseStatus) -> &'static str {
    match status {
        MindspaceMapParseStatus::Ok => "ok",
        MindspaceMapParseStatus::Errors => "errors",
    }
}

fn entry_kind_name(kind: MindspaceEntryKind) -> &'static str {
    match kind {
        MindspaceEntryKind::File => "file",
        MindspaceEntryKind::Directory => "dir",
    }
}

fn severity_name(severity: &Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

fn explicit_role_reason(role: MindspaceRole) -> &'static str {
    match role {
        MindspaceRole::Instruction => "trusted local instruction file",
        MindspaceRole::Index => "workspace navigation entrypoint",
        MindspaceRole::Log => "append-oriented workspace log",
        _ => "explicit workspace role",
    }
}

fn inherited_role_reason(role: MindspaceRole) -> &'static str {
    match role {
        MindspaceRole::Source => "inside source folder",
        MindspaceRole::Inbox => "inside inbox folder",
        MindspaceRole::Report => "inside generated report folder",
        _ => "inside role folder",
    }
}

#[derive(Default)]
struct ScanCounters {
    files: usize,
    directories: usize,
}

#[derive(Default)]
struct MapStatsCollector {
    nodes: usize,
    ids: BTreeSet<String>,
    tags: BTreeSet<String>,
    metadata_keys: BTreeSet<String>,
    references: usize,
    relations: usize,
    open_tasks: usize,
    done_tasks: usize,
}
