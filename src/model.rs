use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskState>,
    #[serde(default)]
    pub detail: Vec<String>,
    pub tags: Vec<String>,
    pub metadata: Vec<MetadataEntry>,
    pub id: Option<String>,
    #[serde(default)]
    pub relations: Vec<Relation>,
    pub children: Vec<Node>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    Open,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TaskProgress {
    pub total: usize,
    pub done: usize,
    pub blocked: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relation {
    pub kind: Option<String>,
    pub target: String,
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
    pub detail_snippet: Option<String>,
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
#[serde(rename_all = "lowercase")]
pub enum RelationDirection {
    Outgoing,
    Incoming,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RelationRow {
    pub direction: RelationDirection,
    pub line: usize,
    pub breadcrumb: String,
    pub text: String,
    pub id: Option<String>,
    pub relation: String,
    pub target: String,
    pub resolved_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportDocument {
    pub nodes: Vec<ExportNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportNode {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_progress: Option<TaskProgressSummary>,
    pub detail: Vec<String>,
    pub tags: Vec<String>,
    pub kv: BTreeMap<String, String>,
    pub id: Option<String>,
    pub relations: Vec<ExportRelation>,
    pub children: Vec<ExportNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportRelation {
    pub kind: Option<String>,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskProgressSummary {
    pub done: usize,
    pub total: usize,
    pub blocked: usize,
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
            task: self.task,
            task_progress: self
                .child_task_progress()
                .has_tasks()
                .then(|| TaskProgressSummary::from(self.child_task_progress())),
            detail: self.detail.clone(),
            tags: self.tags.clone(),
            kv,
            id: self.id.clone(),
            relations: self
                .relations
                .iter()
                .map(|relation| ExportRelation {
                    kind: relation.kind.clone(),
                    target: relation.target.clone(),
                })
                .collect(),
            children: self.children.iter().map(Node::export).collect(),
        }
    }

    pub fn display_line(&self) -> String {
        let mut parts = Vec::new();
        if let Some(task) = self.task {
            parts.push(task.marker().to_string());
        }
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
        parts.extend(self.relations.iter().map(Relation::display_token));

        if parts.is_empty() {
            "(empty)".to_string()
        } else {
            parts.join(" ")
        }
    }

    pub fn display_line_with_task_rollup(&self) -> String {
        let mut line = self.display_line();
        let progress = self.child_task_progress();
        if progress.has_tasks() {
            line.push(' ');
            line.push_str(&progress.display_suffix());
        }
        line
    }

    pub fn task_progress(&self) -> TaskProgress {
        let mut progress = TaskProgress::default();
        if let Some(task) = self.task {
            progress.total += 1;
            if task == TaskState::Done {
                progress.done += 1;
            }
            if self
                .metadata
                .iter()
                .any(|entry| entry.key == "status" && entry.value == "blocked")
            {
                progress.blocked += 1;
            }
        }
        for child in &self.children {
            progress.add(child.task_progress());
        }
        progress
    }

    pub fn child_task_progress(&self) -> TaskProgress {
        let mut progress = TaskProgress::default();
        for child in &self.children {
            progress.add(child.task_progress());
        }
        progress
    }

    pub fn detail_text(&self) -> String {
        self.detail.join("\n")
    }

    pub fn detail_preview(&self) -> Option<String> {
        self.detail
            .iter()
            .find(|line| !line.trim().is_empty())
            .cloned()
            .or_else(|| (!self.detail.is_empty()).then(String::new))
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(tag))
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        self.metadata
            .iter()
            .find(|entry| entry.key.eq_ignore_ascii_case(key))
            .map(|entry| entry.value.as_str())
    }

    pub fn has_status(&self, status: &str) -> bool {
        self.metadata_value("status")
            .is_some_and(|value| value.eq_ignore_ascii_case(status))
    }

    pub fn task_query_matches(&self, query: TaskQuery) -> bool {
        let has_task_signal = self.task.is_some()
            || self.has_tag("#todo")
            || self.has_tag("#done")
            || self.has_tag("#blocked")
            || self.metadata_value("done").is_some();

        match query {
            TaskQuery::Any => has_task_signal,
            TaskQuery::Open => {
                self.task == Some(TaskState::Open)
                    || self.has_tag("#todo")
                    || self
                        .metadata_value("done")
                        .is_some_and(|value| value.eq_ignore_ascii_case("false"))
                    || (has_task_signal
                        && (self.has_status("todo")
                            || self.has_status("active")
                            || self.has_status("blocked")))
            }
            TaskQuery::Active => has_task_signal && self.has_status("active"),
            TaskQuery::Blocked => {
                self.has_tag("#blocked") || (has_task_signal && self.has_status("blocked"))
            }
            TaskQuery::Done => {
                self.task == Some(TaskState::Done)
                    || self.has_tag("#done")
                    || self
                        .metadata_value("done")
                        .is_some_and(|value| value.eq_ignore_ascii_case("true"))
                    || (has_task_signal && self.has_status("done"))
            }
        }
    }
}

impl TaskState {
    pub fn marker(self) -> &'static str {
        match self {
            Self::Open => "[ ]",
            Self::Done => "[x]",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Self::Open => Self::Done,
            Self::Done => Self::Open,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Done => "done",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskQuery {
    Any,
    Open,
    Active,
    Blocked,
    Done,
}

impl TaskProgress {
    pub fn add(&mut self, other: Self) {
        self.total += other.total;
        self.done += other.done;
        self.blocked += other.blocked;
    }

    pub fn has_tasks(self) -> bool {
        self.total > 0
    }

    pub fn display_suffix(self) -> String {
        let base = format!("({}/{} done", self.done, self.total);
        if self.blocked == 0 {
            format!("{base})")
        } else {
            format!("{base}, {} blocked)", self.blocked)
        }
    }
}

impl From<TaskProgress> for TaskProgressSummary {
    fn from(progress: TaskProgress) -> Self {
        Self {
            done: progress.done,
            total: progress.total,
            blocked: progress.blocked,
        }
    }
}

impl Relation {
    pub fn display_token(&self) -> String {
        match &self.kind {
            Some(kind) => format!("[[rel:{kind}->{}]]", self.target),
            None => format!("[[{}]]", self.target),
        }
    }

    pub fn label(&self) -> String {
        self.kind.clone().unwrap_or_else(|| "ref".to_string())
    }
}

pub fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
}
