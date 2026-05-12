use std::collections::HashMap;
use std::path::Path;

use crate::model::{Diagnostic, Document, Node, Severity, TaskState};

pub fn validate_document(document: &Document) -> Vec<Diagnostic> {
    validate_document_with_base_path(document, None)
}

pub fn validate_document_with_base_path(
    document: &Document,
    base_path: Option<&Path>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen_ids = HashMap::<String, usize>::new();

    walk_nodes(&document.nodes, &mut |node| {
        if node.text.trim().is_empty() {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                line: node.line,
                message: "Nodes must include visible text.".to_string(),
            });
        }

        for entry in &node.metadata {
            if entry.key != entry.key.to_lowercase() {
                diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    line: node.line,
                    message: format!(
                        "Metadata key '{}' should be lowercase for consistency.",
                        entry.key
                    ),
                });
            }
        }

        validate_task_state(node, &mut diagnostics);
        validate_references(node, base_path, &mut diagnostics);

        if let Some(id) = &node.id {
            if let Some(first_seen_line) = seen_ids.insert(id.clone(), node.line) {
                diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    line: node.line,
                    message: format!(
                        "Duplicate id '{id}' found; first declared on line {first_seen_line}."
                    ),
                });
            }
        }
    });

    let id_counts = seen_ids;
    walk_nodes(&document.nodes, &mut |node| {
        for relation in &node.relations {
            match id_counts.get(&relation.target) {
                Some(_) => {}
                None => diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    line: node.line,
                    message: format!(
                        "Relation target '{}' does not match any node id.",
                        relation.target
                    ),
                }),
            }
        }
    });

    diagnostics
}

fn validate_references(node: &Node, base_path: Option<&Path>, diagnostics: &mut Vec<Diagnostic>) {
    let Some(base_path) = base_path else {
        return;
    };

    for reference in &node.references {
        if reference.is_url() || reference.target.starts_with('#') {
            continue;
        }

        let target_without_fragment = reference
            .target
            .split_once('#')
            .map(|(path, _)| path)
            .unwrap_or(reference.target.as_str());
        if target_without_fragment.is_empty() {
            continue;
        }

        let candidate = Path::new(target_without_fragment);
        let resolved = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            base_path.join(candidate)
        };

        if !resolved.exists() {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                line: node.line,
                message: format!(
                    "Reference target '{}' does not exist relative to '{}'.",
                    reference.target,
                    base_path.display()
                ),
            });
        }
    }
}

fn validate_task_state(node: &Node, diagnostics: &mut Vec<Diagnostic>) {
    let has_todo = node.has_tag("#todo");
    let has_done = node.has_tag("#done");
    let done_value = node.metadata_value("done");
    let status = node.metadata_value("status");

    if has_todo && has_done {
        push_task_warning(
            diagnostics,
            node.line,
            "Task state conflict: node has both #todo and #done.",
        );
    }

    if done_value.is_some_and(|value| value.eq_ignore_ascii_case("true")) && has_todo {
        push_task_warning(
            diagnostics,
            node.line,
            "Task state conflict: @done:true appears with #todo.",
        );
    }

    if done_value.is_some_and(|value| value.eq_ignore_ascii_case("false")) && has_done {
        push_task_warning(
            diagnostics,
            node.line,
            "Task state conflict: @done:false appears with #done.",
        );
    }

    if status.is_some_and(is_open_task_status) && has_done {
        push_task_warning(
            diagnostics,
            node.line,
            "Task state conflict: open @status appears with #done.",
        );
    }

    if status.is_some_and(|value| value.eq_ignore_ascii_case("done")) && has_todo {
        push_task_warning(
            diagnostics,
            node.line,
            "Task state conflict: @status:done appears with #todo.",
        );
    }

    match node.task {
        Some(TaskState::Open)
            if has_done
                || done_value.is_some_and(|value| value.eq_ignore_ascii_case("true"))
                || status.is_some_and(|value| value.eq_ignore_ascii_case("done")) =>
        {
            push_task_warning(
                diagnostics,
                node.line,
                "Task state conflict: [ ] appears with done task metadata.",
            );
        }
        Some(TaskState::Done)
            if has_todo
                || done_value.is_some_and(|value| value.eq_ignore_ascii_case("false"))
                || status.is_some_and(is_open_task_status) =>
        {
            push_task_warning(
                diagnostics,
                node.line,
                "Task state conflict: [x] appears with open task metadata.",
            );
        }
        Some(_) => {}
        None => {}
    }
}

fn is_open_task_status(value: &str) -> bool {
    value.eq_ignore_ascii_case("todo")
        || value.eq_ignore_ascii_case("active")
        || value.eq_ignore_ascii_case("blocked")
}

fn push_task_warning(diagnostics: &mut Vec<Diagnostic>, line: usize, message: &str) {
    diagnostics.push(Diagnostic {
        severity: Severity::Warning,
        line,
        message: message.to_string(),
    });
}

fn walk_nodes<F>(nodes: &[Node], visitor: &mut F)
where
    F: FnMut(&Node),
{
    for node in nodes {
        visitor(node);
        walk_nodes(&node.children, visitor);
    }
}
