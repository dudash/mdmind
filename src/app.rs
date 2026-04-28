use std::fs;
use std::path::{Path, PathBuf};

use crate::editor::get_node;
use crate::model::{Diagnostic, Document, Node, Severity, has_errors};
use crate::parser::parse_document;
use crate::templates::TemplateKind;
use crate::validate::validate_document;

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
    let validation_diagnostics = validate_document(&parsed.document);

    Ok(LoadedDocument {
        target,
        document: parsed.document,
        parser_diagnostics: parsed.diagnostics,
        validation_diagnostics,
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

    fn parse(source: &str) -> Document {
        parse_document(source).document
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
}
