use crate::model::{Diagnostic, Document, MetadataEntry, Node, Relation, Severity};

#[derive(Debug, Clone)]
pub struct ParseOutput {
    pub document: Document,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
struct FlatNode {
    level: usize,
    node: Node,
}

pub fn parse_document(source: &str) -> ParseOutput {
    let mut diagnostics = Vec::new();
    let mut flat_nodes: Vec<FlatNode> = Vec::new();
    let mut previous_level = 0usize;
    let mut saw_node = false;

    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        if raw_line.trim().is_empty() {
            continue;
        }

        if raw_line.contains('\t') {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                line: line_number,
                message: "Tabs are not supported for indentation; use two spaces per level."
                    .to_string(),
            });
            continue;
        }

        let indent = raw_line
            .as_bytes()
            .iter()
            .take_while(|byte| **byte == b' ')
            .count();

        if indent % 2 != 0 {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                line: line_number,
                message: "Indentation must use multiples of two spaces.".to_string(),
            });
            continue;
        }

        let trimmed = raw_line.trim_start();
        if trimmed.starts_with('|') {
            let Some(last_node) = flat_nodes.last_mut() else {
                diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    line: line_number,
                    message: "Detail lines must follow a node.".to_string(),
                });
                continue;
            };

            let expected_level = last_node.level + 1;
            let actual_level = indent / 2;
            if actual_level != expected_level {
                diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    line: line_number,
                    message:
                        "Detail lines must appear directly under their node before any children."
                            .to_string(),
                });
                continue;
            }

            last_node.node.detail.push(parse_detail_content(trimmed));
            continue;
        }

        let Some(content) = trimmed.strip_prefix("- ") else {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                line: line_number,
                message: "Each non-empty line must begin with '- ' or '| '.".to_string(),
            });
            continue;
        };

        let level = indent / 2;
        if !saw_node && level != 0 {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                line: line_number,
                message: "The first node must start at indentation level 0.".to_string(),
            });
        }
        if saw_node && level > previous_level + 1 {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                line: line_number,
                message: "Indentation cannot jump by more than one level at a time.".to_string(),
            });
        }

        let node = parse_node_content(content, line_number, &mut diagnostics);
        flat_nodes.push(FlatNode { level, node });
        previous_level = level;
        saw_node = true;
    }

    let nodes = build_nodes(&flat_nodes, 0, 0).0;

    ParseOutput {
        document: Document { nodes },
        diagnostics,
    }
}

pub fn parse_node_fragment(fragment: &str) -> Result<Node, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let node = parse_node_content(fragment.trim(), 1, &mut diagnostics);
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        Err(diagnostics)
    } else {
        Ok(node)
    }
}

fn build_nodes(flat_nodes: &[FlatNode], start: usize, level: usize) -> (Vec<Node>, usize) {
    let mut nodes = Vec::new();
    let mut index = start;

    while index < flat_nodes.len() {
        let current = &flat_nodes[index];
        if current.level < level {
            break;
        }
        if current.level > level {
            break;
        }

        let mut node = current.node.clone();
        index += 1;

        if index < flat_nodes.len() && flat_nodes[index].level > level {
            let (children, next_index) = build_nodes(flat_nodes, index, level + 1);
            node.children = children;
            index = next_index;
        }

        nodes.push(node);
    }

    (nodes, index)
}

fn parse_node_content(content: &str, line: usize, diagnostics: &mut Vec<Diagnostic>) -> Node {
    let mut text_parts = Vec::new();
    let mut tags = Vec::new();
    let mut metadata = Vec::new();
    let mut id = None;
    let mut relations = Vec::new();

    for token in content.split_whitespace() {
        if token.starts_with('#') {
            if token.len() == 1 {
                diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    line,
                    message: "Tag tokens must include a name after '#'.".to_string(),
                });
                text_parts.push(token.to_string());
            } else {
                tags.push(token.to_string());
            }
            continue;
        }

        if let Some(stripped) = token.strip_prefix('@') {
            if let Some((key, value)) = stripped.split_once(':') {
                if key.is_empty() || value.is_empty() {
                    diagnostics.push(Diagnostic {
                        severity: Severity::Error,
                        line,
                        message: format!("Invalid metadata token '{token}'."),
                    });
                    text_parts.push(token.to_string());
                } else {
                    metadata.push(MetadataEntry {
                        key: key.to_string(),
                        value: value.to_string(),
                    });
                }
            } else {
                diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    line,
                    message: format!("Invalid metadata token '{token}'."),
                });
                text_parts.push(token.to_string());
            }
            continue;
        }

        if token.starts_with("[id:") {
            if token.ends_with(']') && token.len() > 5 {
                let candidate = &token[4..token.len() - 1];
                if candidate.is_empty() {
                    diagnostics.push(Diagnostic {
                        severity: Severity::Error,
                        line,
                        message: "Node ids cannot be empty.".to_string(),
                    });
                    text_parts.push(token.to_string());
                } else if id.is_some() {
                    diagnostics.push(Diagnostic {
                        severity: Severity::Error,
                        line,
                        message: "Only one [id:...] token is allowed per node.".to_string(),
                    });
                } else {
                    id = Some(candidate.to_string());
                }
            } else {
                diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    line,
                    message: format!("Invalid id token '{token}'."),
                });
                text_parts.push(token.to_string());
            }
            continue;
        }

        if token.starts_with("[[") {
            match parse_relation_token(token) {
                Ok(relation) => relations.push(relation),
                Err(message) => {
                    diagnostics.push(Diagnostic {
                        severity: Severity::Error,
                        line,
                        message,
                    });
                    text_parts.push(token.to_string());
                }
            }
            continue;
        }

        text_parts.push(token.to_string());
    }

    Node {
        text: text_parts.join(" ").trim().to_string(),
        detail: Vec::new(),
        tags,
        metadata,
        id,
        relations,
        children: Vec::new(),
        line,
    }
}

fn parse_detail_content(content: &str) -> String {
    let after_bar = content.strip_prefix('|').unwrap_or(content);
    after_bar.strip_prefix(' ').unwrap_or(after_bar).to_string()
}

fn parse_relation_token(token: &str) -> Result<Relation, String> {
    if !token.ends_with("]]") || token.len() <= 4 {
        return Err(format!("Invalid relation token '{token}'."));
    }

    let inner = &token[2..token.len() - 2];
    if inner.is_empty() {
        return Err(format!("Invalid relation token '{token}'."));
    }

    if let Some(rest) = inner.strip_prefix("rel:") {
        let Some((kind, target)) = rest.split_once("->") else {
            return Err(format!("Invalid relation token '{token}'."));
        };
        if kind.is_empty() || target.is_empty() {
            return Err(format!("Invalid relation token '{token}'."));
        }
        return Ok(Relation {
            kind: Some(kind.to_string()),
            target: target.to_string(),
        });
    }

    Ok(Relation {
        kind: None,
        target: inner.to_string(),
    })
}
