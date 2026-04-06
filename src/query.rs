use std::collections::BTreeMap;

use crate::model::{Document, LinkEntry, MetadataEntry, MetadataRow, Node, SearchMatch, TagCount};

pub fn find_matches(document: &Document, query: &str) -> Vec<SearchMatch> {
    let query = query.trim();
    let mut matches = Vec::new();
    walk_nodes(&document.nodes, &mut Vec::new(), &mut |node, breadcrumb| {
        if node_matches(node, breadcrumb, query) {
            matches.push(SearchMatch {
                line: node.line,
                breadcrumb: breadcrumb.join(" / "),
                text: node.text.clone(),
                id: node.id.clone(),
                tags: node.tags.clone(),
                metadata: node.metadata.clone(),
            });
        }
    });
    matches
}

pub fn tag_counts(document: &Document) -> Vec<TagCount> {
    let mut counts = BTreeMap::<String, usize>::new();
    walk_nodes(&document.nodes, &mut Vec::new(), &mut |node, _| {
        for tag in &node.tags {
            *counts.entry(tag.to_lowercase()).or_default() += 1;
        }
    });

    let mut entries: Vec<_> = counts
        .into_iter()
        .map(|(tag, count)| TagCount { tag, count })
        .collect();
    entries.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.tag.cmp(&right.tag))
    });
    entries
}

pub fn metadata_rows(document: &Document, keys: &[String]) -> Vec<MetadataRow> {
    let filters: Vec<String> = keys.iter().map(|key| key.to_lowercase()).collect();
    let mut rows = Vec::new();

    walk_nodes(&document.nodes, &mut Vec::new(), &mut |node, breadcrumb| {
        for entry in &node.metadata {
            if filters.is_empty() || filters.contains(&entry.key.to_lowercase()) {
                rows.push(MetadataRow {
                    line: node.line,
                    breadcrumb: breadcrumb.join(" / "),
                    key: entry.key.clone(),
                    value: entry.value.clone(),
                    id: node.id.clone(),
                });
            }
        }
    });

    rows
}

pub fn link_entries(document: &Document) -> Vec<LinkEntry> {
    let mut links = Vec::new();
    walk_nodes(&document.nodes, &mut Vec::new(), &mut |node, breadcrumb| {
        if let Some(id) = &node.id {
            links.push(LinkEntry {
                line: node.line,
                id: id.clone(),
                text: node.text.clone(),
                breadcrumb: breadcrumb.join(" / "),
            });
        }
    });
    links
}

pub fn find_by_id<'a>(nodes: &'a [Node], id: &str) -> Option<&'a Node> {
    for node in nodes {
        if node.id.as_deref() == Some(id) {
            return Some(node);
        }
        if let Some(found) = find_by_id(&node.children, id) {
            return Some(found);
        }
    }
    None
}

fn node_matches(node: &Node, _breadcrumb: &[String], query: &str) -> bool {
    if let Some(tag) = query.strip_prefix('#') {
        return node.tags.iter().any(|candidate| {
            candidate
                .trim_start_matches('#')
                .eq_ignore_ascii_case(tag.trim_start_matches('#'))
        });
    }

    if let Some(metadata_query) = query.strip_prefix('@') {
        if let Some((key, value)) = metadata_query.split_once(':') {
            return node.metadata.iter().any(|entry| {
                entry.key.eq_ignore_ascii_case(key) && entry.value.eq_ignore_ascii_case(value)
            });
        }
    }

    let lowered = query.to_lowercase();
    if node.text.to_lowercase().contains(&lowered) {
        return true;
    }
    if node
        .id
        .as_ref()
        .is_some_and(|id| id.to_lowercase().contains(&lowered))
    {
        return true;
    }
    if node
        .tags
        .iter()
        .any(|tag| tag.to_lowercase().contains(&lowered))
    {
        return true;
    }

    node.metadata
        .iter()
        .any(|entry| metadata_matches(entry, &lowered))
}

fn metadata_matches(entry: &MetadataEntry, lowered: &str) -> bool {
    entry.key.to_lowercase().contains(lowered)
        || entry.value.to_lowercase().contains(lowered)
        || format!("@{}:{}", entry.key, entry.value)
            .to_lowercase()
            .contains(lowered)
}

fn walk_nodes<F>(nodes: &[Node], breadcrumb: &mut Vec<String>, visitor: &mut F)
where
    F: FnMut(&Node, &[String]),
{
    for node in nodes {
        breadcrumb.push(if node.text.is_empty() {
            "(empty)".to_string()
        } else {
            node.text.clone()
        });
        visitor(node, breadcrumb);
        walk_nodes(&node.children, breadcrumb, visitor);
        breadcrumb.pop();
    }
}
