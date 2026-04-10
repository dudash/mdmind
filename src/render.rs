use crate::model::{
    Diagnostic, Document, LinkEntry, MetadataRow, RelationDirection, RelationRow, SearchMatch,
    TagCount,
};

pub fn render_tree(document: &Document, max_depth: Option<usize>) -> String {
    let mut lines = Vec::new();
    for (index, node) in document.nodes.iter().enumerate() {
        let is_last = index + 1 == document.nodes.len();
        render_node(node, "", is_last, 0, max_depth, &mut lines);
    }
    if lines.is_empty() {
        "(empty map)".to_string()
    } else {
        lines.join("\n")
    }
}

pub fn render_find(matches: &[SearchMatch]) -> String {
    if matches.is_empty() {
        return "No matches found.".to_string();
    }

    render_table(
        &["line", "path", "id", "text", "detail"],
        &matches
            .iter()
            .map(|entry| {
                vec![
                    entry.line.to_string(),
                    entry.breadcrumb.clone(),
                    entry.id.clone().unwrap_or_else(|| "-".to_string()),
                    entry.text.clone(),
                    entry
                        .detail_snippet
                        .clone()
                        .unwrap_or_else(|| "-".to_string()),
                ]
            })
            .collect::<Vec<_>>(),
    )
}

pub fn render_find_plain(matches: &[SearchMatch]) -> String {
    render_plain_rows(matches.iter().map(|entry| {
        vec![
            entry.line.to_string(),
            entry.breadcrumb.clone(),
            entry.id.clone().unwrap_or_default(),
            entry.text.clone(),
            entry.detail_snippet.clone().unwrap_or_default(),
        ]
    }))
}

pub fn render_tags(tags: &[TagCount]) -> String {
    if tags.is_empty() {
        return "No tags found.".to_string();
    }

    render_table(
        &["tag", "count"],
        &tags
            .iter()
            .map(|entry| vec![entry.tag.clone(), entry.count.to_string()])
            .collect::<Vec<_>>(),
    )
}

pub fn render_tags_plain(tags: &[TagCount]) -> String {
    render_plain_rows(
        tags.iter()
            .map(|entry| vec![entry.tag.clone(), entry.count.to_string()]),
    )
}

pub fn render_metadata(rows: &[MetadataRow]) -> String {
    if rows.is_empty() {
        return "No metadata found.".to_string();
    }

    render_table(
        &["line", "path", "key", "value", "id"],
        &rows
            .iter()
            .map(|entry| {
                vec![
                    entry.line.to_string(),
                    entry.breadcrumb.clone(),
                    entry.key.clone(),
                    entry.value.clone(),
                    entry.id.clone().unwrap_or_else(|| "-".to_string()),
                ]
            })
            .collect::<Vec<_>>(),
    )
}

pub fn render_metadata_plain(rows: &[MetadataRow]) -> String {
    render_plain_rows(rows.iter().map(|entry| {
        vec![
            entry.line.to_string(),
            entry.breadcrumb.clone(),
            entry.key.clone(),
            entry.value.clone(),
            entry.id.clone().unwrap_or_default(),
        ]
    }))
}

pub fn render_links(rows: &[LinkEntry]) -> String {
    if rows.is_empty() {
        return "No ids found.".to_string();
    }

    render_table(
        &["line", "id", "path", "text"],
        &rows
            .iter()
            .map(|entry| {
                vec![
                    entry.line.to_string(),
                    entry.id.clone(),
                    entry.breadcrumb.clone(),
                    entry.text.clone(),
                ]
            })
            .collect::<Vec<_>>(),
    )
}

pub fn render_links_plain(rows: &[LinkEntry]) -> String {
    render_plain_rows(rows.iter().map(|entry| {
        vec![
            entry.line.to_string(),
            entry.id.clone(),
            entry.breadcrumb.clone(),
            entry.text.clone(),
        ]
    }))
}

pub fn render_validate(diagnostics: &[Diagnostic]) -> String {
    if diagnostics.is_empty() {
        return "Valid: no structural issues found.".to_string();
    }

    render_table(
        &["severity", "line", "message"],
        &diagnostics
            .iter()
            .map(|entry| {
                vec![
                    format!("{:?}", entry.severity).to_lowercase(),
                    entry.line.to_string(),
                    entry.message.clone(),
                ]
            })
            .collect::<Vec<_>>(),
    )
}

pub fn render_validate_plain(diagnostics: &[Diagnostic]) -> String {
    render_plain_rows(diagnostics.iter().map(|entry| {
        vec![
            format!("{:?}", entry.severity).to_lowercase(),
            entry.line.to_string(),
            entry.message.clone(),
        ]
    }))
}

pub fn render_relations(rows: &[RelationRow]) -> String {
    if rows.is_empty() {
        return "No relations found.".to_string();
    }

    render_table(
        &["dir", "line", "path", "relation", "target", "resolved"],
        &rows
            .iter()
            .map(|entry| {
                vec![
                    match entry.direction {
                        RelationDirection::Outgoing => "out".to_string(),
                        RelationDirection::Incoming => "in".to_string(),
                    },
                    entry.line.to_string(),
                    entry.breadcrumb.clone(),
                    entry.relation.clone(),
                    entry.target.clone(),
                    entry
                        .resolved_path
                        .clone()
                        .unwrap_or_else(|| "-".to_string()),
                ]
            })
            .collect::<Vec<_>>(),
    )
}

pub fn render_relations_plain(rows: &[RelationRow]) -> String {
    render_plain_rows(rows.iter().map(|entry| {
        vec![
            match entry.direction {
                RelationDirection::Outgoing => "out".to_string(),
                RelationDirection::Incoming => "in".to_string(),
            },
            entry.line.to_string(),
            entry.breadcrumb.clone(),
            entry.relation.clone(),
            entry.target.clone(),
            entry.resolved_path.clone().unwrap_or_default(),
        ]
    }))
}

fn render_node(
    node: &crate::model::Node,
    prefix: &str,
    is_last: bool,
    depth: usize,
    max_depth: Option<usize>,
    lines: &mut Vec<String>,
) {
    let branch = if depth == 0 {
        String::new()
    } else if is_last {
        "└── ".to_string()
    } else {
        "├── ".to_string()
    };
    lines.push(format!("{prefix}{branch}{}", node.display_line()));
    let detail_prefix = if depth == 0 {
        "│ ".to_string()
    } else if is_last {
        format!("{prefix}    │ ")
    } else {
        format!("{prefix}│   │ ")
    };
    for detail_line in &node.detail {
        let rendered = if detail_line.is_empty() {
            " ".to_string()
        } else {
            detail_line.clone()
        };
        lines.push(format!("{detail_prefix}{rendered}"));
    }

    if max_depth.is_some_and(|limit| depth >= limit) {
        return;
    }

    let next_prefix = if depth == 0 {
        String::new()
    } else if is_last {
        format!("{prefix}    ")
    } else {
        format!("{prefix}│   ")
    };

    for (index, child) in node.children.iter().enumerate() {
        let child_is_last = index + 1 == node.children.len();
        render_node(
            child,
            &next_prefix,
            child_is_last,
            depth + 1,
            max_depth,
            lines,
        );
    }
}

fn render_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.len());
        }
    }

    let header_row = headers
        .iter()
        .enumerate()
        .map(|(index, header)| format!("{header:<width$}", width = widths[index]))
        .collect::<Vec<_>>()
        .join("  ");

    let divider = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<_>>()
        .join("  ");

    let body = rows
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(index, cell)| format!("{cell:<width$}", width = widths[index]))
                .collect::<Vec<_>>()
                .join("  ")
        })
        .collect::<Vec<_>>();

    let mut lines = vec![header_row, divider];
    lines.extend(body);
    lines.join("\n")
}

fn render_plain_rows<I>(rows: I) -> String
where
    I: IntoIterator<Item = Vec<String>>,
{
    let lines: Vec<String> = rows.into_iter().map(|row| row.join("\t")).collect();
    if lines.is_empty() {
        String::new()
    } else {
        lines.join("\n")
    }
}
