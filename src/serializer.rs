use crate::model::{Document, Node};

pub fn serialize_document(document: &Document) -> String {
    let mut lines = Vec::new();
    for node in &document.nodes {
        serialize_node(node, 0, &mut lines);
    }

    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn serialize_node(node: &Node, depth: usize, lines: &mut Vec<String>) {
    let indent = "  ".repeat(depth);
    lines.push(format!("{indent}- {}", node.display_line()));
    for detail_line in &node.detail {
        if detail_line.is_empty() {
            lines.push(format!("{indent}  |"));
        } else {
            lines.push(format!("{indent}  | {detail_line}"));
        }
    }

    for child in &node.children {
        serialize_node(child, depth + 1, lines);
    }
}
