use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{Diagnostic, Document, Node, Severity, has_errors};
use crate::parser::parse_document;
use crate::query::find_by_id;
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
    diagnostics.sort_by(|left, right| left.line.cmp(&right.line));
    diagnostics
}

pub fn select_document(loaded: &LoadedDocument) -> Result<Document, AppError> {
    ensure_parseable(loaded)?;
    match &loaded.target.anchor {
        Some(anchor) => {
            let matches = count_id_occurrences(&loaded.document.nodes, anchor);
            if matches == 0 {
                return Err(AppError::new(format!(
                    "No node id matches anchor '{anchor}'."
                )));
            }
            if matches > 1 {
                return Err(AppError::new(format!(
                    "Anchor '{anchor}' is ambiguous because the file contains duplicate ids."
                )));
            }

            let node = find_by_id(&loaded.document.nodes, anchor).expect("count ensured a match");
            Ok(Document {
                nodes: vec![node.clone()],
            })
        }
        None => Ok(loaded.document.clone()),
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
