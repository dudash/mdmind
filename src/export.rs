use crate::model::{Document, ExportDocument, ExportNode};

pub fn export_document(document: &Document, format: &str) -> Result<String, String> {
    let exported = document.export();
    match format {
        "json" => {
            Ok(serde_json::to_string_pretty(&exported)
                .expect("export serialization should succeed"))
        }
        "mermaid" => Ok(render_mermaid(&exported)),
        "opml" => Ok(render_opml(&exported)),
        _ => Err(format!(
            "Unsupported export format '{format}'. Choose one of: json, mermaid, opml."
        )),
    }
}

fn render_mermaid(document: &ExportDocument) -> String {
    let mut lines = vec!["flowchart LR".to_string()];

    for (index, node) in document.nodes.iter().enumerate() {
        let path = vec![index];
        render_mermaid_node(node, None, &path, &mut lines);
    }

    lines.join("\n")
}

fn render_mermaid_node(
    node: &ExportNode,
    parent_ref: Option<&str>,
    path: &[usize],
    lines: &mut Vec<String>,
) {
    let node_ref = mermaid_node_ref(path);
    let label = escape_mermaid_label(&mermaid_label(node));
    lines.push(format!("    {node_ref}[\"{label}\"]"));
    if let Some(parent_ref) = parent_ref {
        lines.push(format!("    {parent_ref} --> {node_ref}"));
    }

    for (index, child) in node.children.iter().enumerate() {
        let mut child_path = path.to_vec();
        child_path.push(index);
        render_mermaid_node(child, Some(&node_ref), &child_path, lines);
    }
}

fn mermaid_node_ref(path: &[usize]) -> String {
    let path = path
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join("_");
    format!("node_{path}")
}

fn mermaid_label(node: &ExportNode) -> String {
    let mut parts = Vec::new();
    let mut primary = node.text.clone();
    if !node.detail.is_empty() {
        if !primary.is_empty() {
            primary.push_str("\\n");
        }
        primary.push_str(&node.detail.join("\\n"));
    }
    if !primary.is_empty() {
        parts.push(primary);
    }
    parts.extend(node.tags.iter().cloned());
    parts.extend(node.kv.iter().map(|(key, value)| format!("@{key}:{value}")));
    if let Some(id) = &node.id {
        parts.push(format!("[id:{id}]"));
    }

    if parts.is_empty() {
        "(empty)".to_string()
    } else {
        parts.join(" ")
    }
}

fn escape_mermaid_label(label: &str) -> String {
    label.replace('\\', "\\\\").replace('"', "\\\"")
}

fn render_opml(document: &ExportDocument) -> String {
    let mut lines = vec![
        r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_string(),
        r#"<opml version="2.0">"#.to_string(),
        "  <head>".to_string(),
        "    <title>mdmind export</title>".to_string(),
        "  </head>".to_string(),
        "  <body>".to_string(),
    ];

    for node in &document.nodes {
        render_opml_node(node, 2, &mut lines);
    }

    lines.push("  </body>".to_string());
    lines.push("</opml>".to_string());
    lines.join("\n")
}

fn render_opml_node(node: &ExportNode, depth: usize, lines: &mut Vec<String>) {
    let indent = "  ".repeat(depth);
    let attributes = opml_attributes(node);
    if node.children.is_empty() {
        lines.push(format!("{indent}<outline{attributes} />"));
        return;
    }

    lines.push(format!("{indent}<outline{attributes}>"));
    for child in &node.children {
        render_opml_node(child, depth + 1, lines);
    }
    lines.push(format!("{indent}</outline>"));
}

fn opml_attributes(node: &ExportNode) -> String {
    let mut attributes = vec![format!(r#" text="{}""#, escape_xml_attr(&node.text))];

    if let Some(id) = &node.id {
        attributes.push(format!(r#" mdm_id="{}""#, escape_xml_attr(id)));
    }
    if !node.tags.is_empty() {
        attributes.push(format!(
            r#" mdm_tags="{}""#,
            escape_xml_attr(&node.tags.join(" "))
        ));
    }
    if !node.detail.is_empty() {
        attributes.push(format!(
            r#" mdm_detail="{}""#,
            escape_xml_attr(&node.detail.join("\n"))
        ));
    }
    for (key, value) in &node.kv {
        attributes.push(format!(
            r#" {}="{}""#,
            sanitize_xml_attr_name(key),
            escape_xml_attr(value)
        ));
    }

    attributes.join("")
}

fn sanitize_xml_attr_name(key: &str) -> String {
    let Some(first) = key.chars().next() else {
        return "mdm_key".to_string();
    };

    let mut name = String::new();
    if is_xml_name_start(first) {
        name.push(first);
    } else {
        name.push_str("mdm_");
        if is_xml_name_char(first) {
            name.push(first);
        } else {
            name.push('_');
        }
    }

    for ch in key.chars().skip(1) {
        if is_xml_name_char(ch) {
            name.push(ch);
        } else {
            name.push('_');
        }
    }

    if name == "text" {
        "mdm_text".to_string()
    } else {
        name
    }
}

fn is_xml_name_start(ch: char) -> bool {
    ch == ':' || ch == '_' || ch.is_ascii_alphabetic()
}

fn is_xml_name_char(ch: char) -> bool {
    is_xml_name_start(ch) || ch == '-' || ch == '.' || ch.is_ascii_digit()
}

fn escape_xml_attr(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use crate::parser::parse_document;

    use super::export_document;

    #[test]
    fn mermaid_export_preserves_hierarchy_and_metadata() {
        let parsed = parse_document(
            "- Product Idea #idea [id:product]\n  | North star for the release.\n  - MVP Scope #todo @status:active [id:product/mvp]\n",
        );
        let rendered = export_document(&parsed.document, "mermaid").expect("export should work");

        assert!(rendered.starts_with("flowchart LR"));
        assert!(rendered.contains(
            r#"node_0["Product Idea\\nNorth star for the release. #idea [id:product]"]"#
        ));
        assert!(
            rendered.contains(r#"node_0_0["MVP Scope #todo @status:active [id:product/mvp]"]"#)
        );
        assert!(rendered.contains("node_0 --> node_0_0"));
    }

    #[test]
    fn opml_export_escapes_attributes_and_preserves_structure() {
        let parsed = parse_document(
            "- Root @status:ready [id:root]\n  | Ready for partner review.\n  - Child #todo @owner:me&you\n",
        );
        let rendered = export_document(&parsed.document, "opml").expect("export should work");

        assert!(rendered.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
        assert!(rendered.contains(r#"<outline text="Root" mdm_id="root""#));
        assert!(rendered.contains(r#"mdm_detail="Ready for partner review.""#));
        assert!(rendered.contains(r#"status="ready">"#));
        assert!(
            rendered.contains(r##"<outline text="Child" mdm_tags="#todo" owner="me&amp;you" />"##)
        );
    }
}
