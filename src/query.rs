use std::collections::BTreeMap;

use crate::model::{Document, LinkEntry, MetadataEntry, MetadataRow, Node, SearchMatch, TagCount};

pub fn find_matches(document: &Document, query: &str) -> Vec<SearchMatch> {
    let Some(query) = FilterQuery::parse(query) else {
        return Vec::new();
    };
    let mut matches = Vec::new();
    walk_nodes(&document.nodes, &mut Vec::new(), &mut |node, breadcrumb| {
        if query.matches(node) {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterQuery {
    raw: String,
    terms: Vec<QueryTerm>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum QueryTerm {
    Text(String),
    Tag(String),
    Metadata { key: String, value: Option<String> },
}

impl FilterQuery {
    pub fn parse(input: &str) -> Option<Self> {
        let raw = input.trim().to_string();
        if raw.is_empty() {
            return None;
        }

        let terms = raw.split_whitespace().map(parse_term).collect::<Vec<_>>();

        Some(Self { raw, terms })
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn matches(&self, node: &Node) -> bool {
        self.terms.iter().all(|term| term_matches(term, node))
    }
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

fn metadata_matches(entry: &MetadataEntry, lowered: &str) -> bool {
    entry.key.to_lowercase().contains(lowered)
        || entry.value.to_lowercase().contains(lowered)
        || format!("@{}:{}", entry.key, entry.value)
            .to_lowercase()
            .contains(lowered)
}

fn parse_term(token: &str) -> QueryTerm {
    if let Some(tag) = token.strip_prefix('#') {
        return QueryTerm::Tag(tag.trim_start_matches('#').to_lowercase());
    }

    if let Some(metadata_query) = token.strip_prefix('@') {
        if let Some((key, value)) = metadata_query.split_once(':') {
            return QueryTerm::Metadata {
                key: key.to_lowercase(),
                value: Some(value.to_lowercase()),
            };
        }

        return QueryTerm::Metadata {
            key: metadata_query.to_lowercase(),
            value: None,
        };
    }

    QueryTerm::Text(token.to_lowercase())
}

fn term_matches(term: &QueryTerm, node: &Node) -> bool {
    match term {
        QueryTerm::Tag(tag) => node.tags.iter().any(|candidate| {
            candidate
                .trim_start_matches('#')
                .eq_ignore_ascii_case(tag.trim_start_matches('#'))
        }),
        QueryTerm::Metadata { key, value } => node.metadata.iter().any(|entry| {
            if !entry.key.eq_ignore_ascii_case(key) {
                return false;
            }
            match value {
                Some(value) => entry.value.eq_ignore_ascii_case(value),
                None => true,
            }
        }),
        QueryTerm::Text(lowered) => {
            node.text.to_lowercase().contains(lowered)
                || node
                    .id
                    .as_ref()
                    .is_some_and(|id| id.to_lowercase().contains(lowered))
                || node
                    .tags
                    .iter()
                    .any(|tag| tag.to_lowercase().contains(lowered))
                || node
                    .metadata
                    .iter()
                    .any(|entry| metadata_matches(entry, lowered))
        }
    }
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
