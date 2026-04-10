use std::collections::BTreeMap;

use crate::model::{
    Document, LinkEntry, MetadataEntry, MetadataKeyCount, MetadataRow, MetadataValueCount, Node,
    RelationDirection, RelationRow, SearchMatch, TagCount,
};

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
                detail_snippet: matching_detail_snippet(&query, node),
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
    tag_counts_for_filter(document, None)
}

pub fn tag_counts_for_filter(document: &Document, filter: Option<&FilterQuery>) -> Vec<TagCount> {
    let mut counts = BTreeMap::<String, usize>::new();
    walk_nodes(&document.nodes, &mut Vec::new(), &mut |node, _| {
        if !node_is_in_scope(node, filter) {
            return;
        }
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

pub fn metadata_key_counts_for_filter(
    document: &Document,
    filter: Option<&FilterQuery>,
) -> Vec<MetadataKeyCount> {
    let mut counts = BTreeMap::<String, usize>::new();
    walk_nodes(&document.nodes, &mut Vec::new(), &mut |node, _| {
        if !node_is_in_scope(node, filter) {
            return;
        }
        for entry in &node.metadata {
            *counts.entry(entry.key.to_lowercase()).or_default() += 1;
        }
    });

    let mut entries: Vec<_> = counts
        .into_iter()
        .map(|(key, count)| MetadataKeyCount { key, count })
        .collect();
    entries.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.key.cmp(&right.key))
    });
    entries
}

pub fn metadata_value_counts_for_filter(
    document: &Document,
    filter: Option<&FilterQuery>,
) -> Vec<MetadataValueCount> {
    let mut counts = BTreeMap::<(String, String), usize>::new();
    walk_nodes(&document.nodes, &mut Vec::new(), &mut |node, _| {
        if !node_is_in_scope(node, filter) {
            return;
        }
        for entry in &node.metadata {
            *counts
                .entry((entry.key.to_lowercase(), entry.value.to_lowercase()))
                .or_default() += 1;
        }
    });

    let mut entries: Vec<_> = counts
        .into_iter()
        .map(|((key, value), count)| MetadataValueCount { key, value, count })
        .collect();
    entries.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.key.cmp(&right.key))
            .then_with(|| left.value.cmp(&right.value))
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

pub fn relation_entries(document: &Document) -> Vec<RelationRow> {
    let mut rows = Vec::new();
    walk_nodes(&document.nodes, &mut Vec::new(), &mut |node, breadcrumb| {
        let breadcrumb_text = breadcrumb.join(" / ");
        for relation in &node.relations {
            rows.push(RelationRow {
                direction: RelationDirection::Outgoing,
                line: node.line,
                breadcrumb: breadcrumb_text.clone(),
                text: node.text.clone(),
                id: node.id.clone(),
                relation: relation.label(),
                target: relation.target.clone(),
                resolved_path: find_relation_target_breadcrumb(document, &relation.target),
            });
        }
    });
    rows
}

pub fn relation_entries_for_anchor(document: &Document, anchor_id: &str) -> Vec<RelationRow> {
    let mut rows = Vec::new();

    walk_nodes(&document.nodes, &mut Vec::new(), &mut |node, breadcrumb| {
        let breadcrumb_text = breadcrumb.join(" / ");
        if node.id.as_deref() == Some(anchor_id) {
            for relation in &node.relations {
                rows.push(RelationRow {
                    direction: RelationDirection::Outgoing,
                    line: node.line,
                    breadcrumb: breadcrumb_text.clone(),
                    text: node.text.clone(),
                    id: node.id.clone(),
                    relation: relation.label(),
                    target: relation.target.clone(),
                    resolved_path: find_relation_target_breadcrumb(document, &relation.target),
                });
            }
        }

        for relation in &node.relations {
            if relation.target == anchor_id {
                rows.push(RelationRow {
                    direction: RelationDirection::Incoming,
                    line: node.line,
                    breadcrumb: breadcrumb_text.clone(),
                    text: node.text.clone(),
                    id: node.id.clone(),
                    relation: relation.label(),
                    target: relation.target.clone(),
                    resolved_path: find_relation_target_breadcrumb(document, &relation.target),
                });
            }
        }
    });

    rows.sort_by(|left, right| {
        left.line
            .cmp(&right.line)
            .then_with(|| left.breadcrumb.cmp(&right.breadcrumb))
    });
    rows
}

pub fn backlinks_to(document: &Document, target_id: &str) -> Vec<RelationRow> {
    relation_entries_for_anchor(document, target_id)
        .into_iter()
        .filter(|row| row.direction == RelationDirection::Incoming)
        .collect()
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

fn node_is_in_scope(node: &Node, filter: Option<&FilterQuery>) -> bool {
    filter.is_none_or(|filter| filter.matches(node))
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
                    .detail
                    .iter()
                    .any(|line| line.to_lowercase().contains(lowered))
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
                || node.relations.iter().any(|relation| {
                    relation.target.to_lowercase().contains(lowered)
                        || relation
                            .kind
                            .as_ref()
                            .is_some_and(|kind| kind.to_lowercase().contains(lowered))
                        || relation.display_token().to_lowercase().contains(lowered)
                })
        }
    }
}

fn matching_detail_snippet(query: &FilterQuery, node: &Node) -> Option<String> {
    let text_terms = query
        .terms
        .iter()
        .filter_map(|term| match term {
            QueryTerm::Text(lowered) => Some(lowered.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();

    node.detail
        .iter()
        .find(|line| {
            let lowered = line.to_lowercase();
            text_terms.iter().any(|term| lowered.contains(term))
        })
        .cloned()
}

fn find_relation_target_breadcrumb(document: &Document, target: &str) -> Option<String> {
    let mut resolved = None;
    walk_nodes(&document.nodes, &mut Vec::new(), &mut |node, breadcrumb| {
        if resolved.is_none() && node.id.as_deref() == Some(target) {
            resolved = Some(breadcrumb.join(" / "));
        }
    });
    resolved
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
