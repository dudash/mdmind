use std::collections::HashMap;

use crate::model::{Diagnostic, Document, Node, Severity};

pub fn validate_document(document: &Document) -> Vec<Diagnostic> {
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

fn walk_nodes<F>(nodes: &[Node], visitor: &mut F)
where
    F: FnMut(&Node),
{
    for node in nodes {
        visitor(node);
        walk_nodes(&node.children, visitor);
    }
}
