use std::fs;
use std::path::{Path, PathBuf};

use crate::checkpoints::checkpoints_path_for;
use crate::editor::get_node;
use crate::locations::locations_path_for;
use crate::model::{Diagnostic, Document, Node, Severity, has_errors};
use crate::parser::parse_document;
use crate::session::session_path_for;
use crate::templates::TemplateKind;
use crate::ui_settings::ui_settings_path_for;
use crate::validate::validate_document_with_base_path;
use crate::views::views_path_for;

#[derive(Debug, Clone)]
pub struct TargetRef {
    pub path: PathBuf,
    pub anchor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LoadedDocument {
    pub target: TargetRef,
    pub document: Document,
    pub parser_diagnostics: Vec<Diagnostic>,
    pub validation_diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenTargetMode {
    Auto,
    Map,
    Markdown,
}

#[derive(Debug, Clone)]
pub enum ClassifiedTarget {
    NativeMap(LoadedDocument),
    OrdinaryMarkdown {
        target: TargetRef,
        source: String,
    },
    NearMissMap {
        target: TargetRef,
        diagnostics: Vec<Diagnostic>,
        score: i32,
    },
}

#[derive(Debug, Clone)]
pub struct AppError {
    message: String,
}

impl AppError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

pub fn parse_target(raw: &str) -> TargetRef {
    let (path, anchor) = match raw.split_once('#') {
        Some((path, anchor)) => (path, Some(anchor.to_string())),
        None => (raw, None),
    };

    TargetRef {
        path: PathBuf::from(path),
        anchor,
    }
}

pub fn load_document(target: &str) -> Result<LoadedDocument, AppError> {
    let target = parse_target(target);
    let source = fs::read_to_string(&target.path).map_err(|error| {
        AppError::new(format!(
            "Could not read '{}': {error}",
            target.path.display()
        ))
    })?;

    let parsed = parse_document(&source);
    let validation_base_path = target
        .path
        .parent()
        .filter(|path| !path.as_os_str().is_empty());
    let validation_diagnostics =
        validate_document_with_base_path(&parsed.document, validation_base_path);

    Ok(LoadedDocument {
        target,
        document: parsed.document,
        parser_diagnostics: parsed.diagnostics,
        validation_diagnostics,
    })
}

pub fn classify_open_target(
    raw_target: &str,
    mode: OpenTargetMode,
) -> Result<ClassifiedTarget, AppError> {
    let target = parse_target(raw_target);
    let source = fs::read_to_string(&target.path).map_err(|error| {
        AppError::new(format!(
            "Could not read '{}': {error}",
            target.path.display()
        ))
    })?;

    if mode == OpenTargetMode::Markdown {
        return Ok(ClassifiedTarget::OrdinaryMarkdown { target, source });
    }

    let parsed = parse_document(&source);
    let validation_base_path = target
        .path
        .parent()
        .filter(|path| !path.as_os_str().is_empty());
    let validation_diagnostics =
        validate_document_with_base_path(&parsed.document, validation_base_path);
    let loaded = LoadedDocument {
        target: target.clone(),
        document: parsed.document,
        parser_diagnostics: parsed.diagnostics,
        validation_diagnostics,
    };

    if mode == OpenTargetMode::Map || !has_errors(&loaded.parser_diagnostics) {
        return Ok(ClassifiedTarget::NativeMap(loaded));
    }

    let score = mdmind_intent_score(&source, &target.path);
    if score >= 5 {
        Ok(ClassifiedTarget::NearMissMap {
            target,
            diagnostics: loaded.parser_diagnostics,
            score,
        })
    } else {
        Ok(ClassifiedTarget::OrdinaryMarkdown { target, source })
    }
}

pub fn near_miss_guidance(target: &TargetRef, diagnostics: &[Diagnostic], score: i32) -> String {
    let first_error = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| {
            format!(
                " First issue: line {}: {}",
                diagnostic.line, diagnostic.message
            )
        })
        .unwrap_or_default();

    format!(
        "This looks like a damaged mdmind map, so mdmind did not open it as ordinary Markdown.\n\nRecommended:\n  mdm validate {path}\n\nOther options:\n  mdmind --as markdown {path}\n  mdm import {path} --from markdown --preview --report\n\n{first_error}\nClassifier score: {score}",
        path = target.path.display(),
        first_error = first_error.trim()
    )
}

fn mdmind_intent_score(source: &str, path: &Path) -> i32 {
    let mut score = 0;
    let signal_source = source_without_fenced_code(source);
    if signal_source.contains("[id:") {
        score += 5;
    }
    if signal_source.contains("[[rel:") {
        score += 5;
    }
    if signal_source.contains("[[") {
        score += 3;
    }
    if has_known_sidecar(path) {
        score += 4;
    }

    let mut metadata_lines = 0;
    let mut tag_lines = 0;
    let mut task_outline_lines = 0;
    for line in signal_source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('|') {
            score += 3;
        }
        if trimmed.starts_with("- ") && trimmed.contains('@') && trimmed.contains(':') {
            metadata_lines += 1;
        }
        if trimmed.starts_with("- ") && trimmed.matches('#').count() >= 2 {
            tag_lines += 1;
        }
        if line.starts_with("  ")
            && (trimmed.starts_with("- [ ]")
                || trimmed.starts_with("- [x]")
                || trimmed.starts_with("- [X]"))
        {
            task_outline_lines += 1;
        }
    }
    if metadata_lines >= 2 {
        score += 2;
    }
    if task_outline_lines >= 2 {
        score += 2;
    }
    if tag_lines >= 2 {
        score += 1;
    }

    if starts_like_markdown_document(source) {
        score -= 12;
    }
    if source.contains("\n```") {
        score -= 2;
    }
    if source.contains("\n|") && source.contains("\n| ---") {
        score -= 2;
    }
    if source.starts_with("---\n") {
        score -= 2;
    }

    score
}

fn source_without_fenced_code(source: &str) -> String {
    let mut stripped = String::new();
    let mut fence_marker: Option<&'static str> = None;
    for line in source.lines() {
        let trimmed = line.trim_start();
        let line_fence = if trimmed.starts_with("```") {
            Some("```")
        } else if trimmed.starts_with("~~~") {
            Some("~~~")
        } else {
            None
        };
        if let Some(line_fence) = line_fence {
            if fence_marker == Some(line_fence) {
                fence_marker = None;
            } else if fence_marker.is_none() {
                fence_marker = Some(line_fence);
            }
            continue;
        }
        if fence_marker.is_none() {
            stripped.push_str(line);
            stripped.push('\n');
        }
    }
    stripped
}

fn has_known_sidecar(path: &Path) -> bool {
    [
        session_path_for(path),
        ui_settings_path_for(path),
        checkpoints_path_for(path),
        views_path_for(path),
        locations_path_for(path),
    ]
    .into_iter()
    .filter_map(Result::ok)
    .any(|path| path.exists())
}

fn starts_like_markdown_document(source: &str) -> bool {
    let mut meaningful = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('<'));
    let Some(first) = meaningful.next() else {
        return false;
    };
    let Some(second) = meaningful.next() else {
        return first.starts_with('#');
    };
    if first.starts_with('#') && !second.starts_with("- ") {
        return true;
    }

    source.lines().map(str::trim_start).any(|line| {
        line.starts_with("# ")
            || line.starts_with("## ")
            || line.starts_with("### ")
            || line.starts_with("#### ")
    })
}

pub fn ensure_parseable(loaded: &LoadedDocument) -> Result<(), AppError> {
    if has_errors(&loaded.parser_diagnostics) {
        return Err(AppError::new(format!(
            "The map contains parser errors. Run `mdm validate {}` for details.",
            loaded.target.path.display()
        )));
    }

    Ok(())
}

pub fn diagnostics_for_validate(loaded: &LoadedDocument) -> Vec<Diagnostic> {
    let mut diagnostics = loaded.parser_diagnostics.clone();
    diagnostics.extend(loaded.validation_diagnostics.clone());
    diagnostics.sort_by_key(|left| left.line);
    diagnostics
}

pub fn select_document(loaded: &LoadedDocument) -> Result<Document, AppError> {
    ensure_parseable(loaded)?;
    match &loaded.target.anchor {
        Some(anchor) => {
            let path = resolve_anchor_path(&loaded.document, anchor)?;
            let node =
                get_node(&loaded.document.nodes, &path).expect("resolved anchor path should exist");
            Ok(Document {
                nodes: vec![node.clone()],
            })
        }
        None => Ok(loaded.document.clone()),
    }
}

pub fn resolve_anchor_path(document: &Document, anchor: &str) -> Result<Vec<usize>, AppError> {
    let matches = count_id_occurrences(&document.nodes, anchor);
    if matches > 1 {
        return Err(AppError::new(format!(
            "Anchor '{anchor}' is ambiguous because the file contains duplicate ids."
        )));
    }
    if matches == 1 {
        return Ok(find_path_by_anchor_id(&document.nodes, anchor).expect("count ensured a match"));
    }

    let segments = normalized_anchor_segments(anchor);
    if segments.is_empty() {
        return Err(AppError::new(format!(
            "No node id or label path matches anchor '{anchor}'."
        )));
    }

    let mut label_matches = Vec::new();
    collect_label_path_matches(&document.nodes, &segments, Vec::new(), &mut label_matches);
    match label_matches.len() {
        0 => Err(AppError::new(format!(
            "No node id or label path matches anchor '{anchor}'."
        ))),
        1 => Ok(label_matches.remove(0)),
        _ => {
            let candidates = label_matches
                .iter()
                .take(5)
                .map(|path| format!("- {}", breadcrumb_for_path(document, path)))
                .collect::<Vec<_>>()
                .join("\n");
            let suffix = if label_matches.len() > 5 {
                format!("\n- … and {} more", label_matches.len() - 5)
            } else {
                String::new()
            };
            Err(AppError::new(format!(
                "Anchor '{anchor}' is ambiguous as a label path. Candidates:\n{candidates}{suffix}"
            )))
        }
    }
}

pub fn create_from_template(
    path: &Path,
    template: TemplateKind,
    force: bool,
) -> Result<(), AppError> {
    if path.exists() && !force {
        return Err(AppError::new(format!(
            "'{}' already exists. Use --force to overwrite it.",
            path.display()
        )));
    }

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::new(format!(
                "Could not create parent directory '{}': {error}",
                parent.display()
            ))
        })?;
    }

    fs::write(path, template.file_contents()).map_err(|error| {
        AppError::new(format!(
            "Could not write template to '{}': {error}",
            path.display()
        ))
    })
}

pub fn diagnostics_have_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
}

fn count_id_occurrences(nodes: &[Node], target_id: &str) -> usize {
    let mut count = 0usize;
    for node in nodes {
        if node.id.as_deref() == Some(target_id) {
            count += 1;
        }
        count += count_id_occurrences(&node.children, target_id);
    }
    count
}

fn find_path_by_anchor_id(nodes: &[Node], id: &str) -> Option<Vec<usize>> {
    for (index, node) in nodes.iter().enumerate() {
        let path = vec![index];
        if node.id.as_deref() == Some(id) {
            return Some(path);
        }
        if let Some(found) = find_path_by_anchor_id_with_prefix(&node.children, id, path.clone()) {
            return Some(found);
        }
    }
    None
}

fn find_path_by_anchor_id_with_prefix(
    nodes: &[Node],
    id: &str,
    prefix: Vec<usize>,
) -> Option<Vec<usize>> {
    for (index, node) in nodes.iter().enumerate() {
        let mut path = prefix.clone();
        path.push(index);
        if node.id.as_deref() == Some(id) {
            return Some(path);
        }
        if let Some(found) = find_path_by_anchor_id_with_prefix(&node.children, id, path.clone()) {
            return Some(found);
        }
    }
    None
}

fn normalized_anchor_segments(anchor: &str) -> Vec<String> {
    anchor
        .split('/')
        .map(normalize_anchor_segment)
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn normalize_anchor_segment(raw: &str) -> String {
    let normalized = raw
        .trim()
        .chars()
        .map(|ch| match ch {
            '-' | '_' => ' ',
            other => other.to_ascii_lowercase(),
        })
        .collect::<String>();
    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collect_label_path_matches(
    nodes: &[Node],
    segments: &[String],
    prefix: Vec<usize>,
    matches: &mut Vec<Vec<usize>>,
) {
    let Some((first, rest)) = segments.split_first() else {
        return;
    };

    for (index, node) in nodes.iter().enumerate() {
        if normalize_anchor_segment(&node.text) != *first {
            continue;
        }

        let mut path = prefix.clone();
        path.push(index);
        if rest.is_empty() {
            matches.push(path);
        } else {
            collect_label_path_matches(&node.children, rest, path, matches);
        }
    }
}

fn breadcrumb_for_path(document: &Document, path: &[usize]) -> String {
    let mut breadcrumb = Vec::new();
    let mut nodes = &document.nodes;
    for index in path {
        let Some(node) = nodes.get(*index) else {
            break;
        };
        breadcrumb.push(if node.text.is_empty() {
            "(empty)".to_string()
        } else {
            node.text.clone()
        });
        nodes = &node.children;
    }
    breadcrumb.join(" / ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn parse(source: &str) -> Document {
        parse_document(source).document
    }

    fn temp_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("mdmind-classify-{nonce}-{name}"))
    }

    #[test]
    fn anchor_resolution_prefers_ids_before_label_paths() {
        let document = parse("- Product Idea [id:product]\n  - Tasks [id:product/tasks]\n");
        assert_eq!(
            resolve_anchor_path(&document, "product/tasks").expect("id should resolve"),
            vec![0, 0]
        );
    }

    #[test]
    fn anchor_resolution_falls_back_to_label_paths() {
        let document = parse("- Product Idea\n  - Tasks\n    - Ship tests\n");
        assert_eq!(
            resolve_anchor_path(&document, "Product Idea/Tasks/Ship tests")
                .expect("label path should resolve"),
            vec![0, 0, 0]
        );
    }

    #[test]
    fn anchor_resolution_normalizes_case_spacing_and_separators() {
        let document = parse("- Product Idea\n  - Ship Tests_Now\n");
        assert_eq!(
            resolve_anchor_path(&document, " product   idea / ship-tests now ")
                .expect("normalized label path should resolve"),
            vec![0, 0]
        );
    }

    #[test]
    fn anchor_resolution_reports_ambiguous_label_paths() {
        let document = parse("- Product Idea\n  - Tasks\n- Product Idea\n  - Tasks\n");
        let error = resolve_anchor_path(&document, "Product Idea/Tasks")
            .expect_err("duplicate label path should be ambiguous");
        assert!(error.message().contains("ambiguous as a label path"));
        assert!(error.message().contains("Product Idea / Tasks"));
    }

    #[test]
    fn classify_open_target_routes_readme_markdown_to_document_view() {
        let path = temp_path("README.md");
        fs::write(
            &path,
            "# Project\n\nNormal prose.\n\n~~~text\n- Map [id:map] [[rel:mentions->other]]\n~~~\n",
        )
        .expect("fixture should be writable");

        let classified = classify_open_target(path.to_str().unwrap(), OpenTargetMode::Auto)
            .expect("classification should succeed");
        assert!(matches!(
            classified,
            ClassifiedTarget::OrdinaryMarkdown { .. }
        ));
        assert!(
            !session_path_for(&path)
                .expect("session path should resolve")
                .exists()
        );

        fs::remove_file(path).ok();
    }

    #[test]
    fn classify_open_target_routes_markdown_checklists_to_document_view() {
        let path = temp_path("checklist.md");
        fs::write(
            &path,
            "# Launch Checklist\n\n- [ ] Write docs\n- [x] Ship build\n\nNotes for the release crew.\n",
        )
        .expect("fixture should be writable");

        let classified = classify_open_target(path.to_str().unwrap(), OpenTargetMode::Auto)
            .expect("classification should succeed");
        assert!(matches!(
            classified,
            ClassifiedTarget::OrdinaryMarkdown { .. }
        ));

        fs::remove_file(path).ok();
    }

    #[test]
    fn classify_open_target_keeps_valid_maps_native() {
        let path = temp_path("roadmap.md");
        fs::write(&path, "- Roadmap\n  - Ship reader [id:reader]\n")
            .expect("fixture should be writable");

        let classified = classify_open_target(path.to_str().unwrap(), OpenTargetMode::Auto)
            .expect("classification should succeed");
        assert!(matches!(classified, ClassifiedTarget::NativeMap(_)));

        fs::remove_file(path).ok();
    }

    #[test]
    fn classify_open_target_protects_near_miss_maps() {
        let path = temp_path("broken-map.md");
        fs::write(&path, "- Roadmap [id:roadmap]\n  Missing dash\n")
            .expect("fixture should be writable");

        let classified = classify_open_target(path.to_str().unwrap(), OpenTargetMode::Auto)
            .expect("classification should succeed");
        assert!(matches!(classified, ClassifiedTarget::NearMissMap { .. }));

        let forced = classify_open_target(path.to_str().unwrap(), OpenTargetMode::Markdown)
            .expect("forced markdown should succeed");
        assert!(matches!(forced, ClassifiedTarget::OrdinaryMarkdown { .. }));

        fs::remove_file(path).ok();
    }

    #[test]
    fn classify_open_target_force_map_attempts_strict_map_parse() {
        let path = temp_path("README-map.md");
        fs::write(&path, "# Project\n\nNormal prose.\n").expect("fixture should be writable");

        let classified = classify_open_target(path.to_str().unwrap(), OpenTargetMode::Map)
            .expect("forced map classification should load");
        let ClassifiedTarget::NativeMap(loaded) = classified else {
            panic!("forced map should return native map load");
        };
        assert!(has_errors(&loaded.parser_diagnostics));

        fs::remove_file(path).ok();
    }
}
