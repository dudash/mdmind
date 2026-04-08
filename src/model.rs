use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub text: String,
    pub tags: Vec<String>,
    pub metadata: Vec<MetadataEntry>,
    pub id: Option<String>,
    pub children: Vec<Node>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchMatch {
    pub line: usize,
    pub breadcrumb: String,
    pub text: String,
    pub id: Option<String>,
    pub tags: Vec<String>,
    pub metadata: Vec<MetadataEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TagCount {
    pub tag: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MetadataRow {
    pub line: usize,
    pub breadcrumb: String,
    pub key: String,
    pub value: String,
    pub id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MetadataKeyCount {
    pub key: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MetadataValueCount {
    pub key: String,
    pub value: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LinkEntry {
    pub line: usize,
    pub id: String,
    pub text: String,
    pub breadcrumb: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportDocument {
    pub nodes: Vec<ExportNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportNode {
    pub text: String,
    pub tags: Vec<String>,
    pub kv: BTreeMap<String, String>,
    pub id: Option<String>,
    pub children: Vec<ExportNode>,
}

impl Document {
    pub fn export(&self) -> ExportDocument {
        ExportDocument {
            nodes: self.nodes.iter().map(Node::export).collect(),
        }
    }
}

impl Node {
    pub fn export(&self) -> ExportNode {
        let kv = self
            .metadata
            .iter()
            .map(|entry| (entry.key.clone(), entry.value.clone()))
            .collect();

        ExportNode {
            text: self.text.clone(),
            tags: self.tags.clone(),
            kv,
            id: self.id.clone(),
            children: self.children.iter().map(Node::export).collect(),
        }
    }

    pub fn display_line(&self) -> String {
        let mut parts = Vec::new();
        if !self.text.is_empty() {
            parts.push(self.text.clone());
        }
        parts.extend(self.tags.iter().cloned());
        parts.extend(
            self.metadata
                .iter()
                .map(|entry| format!("@{}:{}", entry.key, entry.value)),
        );
        if let Some(id) = &self.id {
            parts.push(format!("[id:{id}]"));
        }

        if parts.is_empty() {
            "(empty)".to_string()
        } else {
            parts.join(" ")
        }
    }
}

pub fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
}
