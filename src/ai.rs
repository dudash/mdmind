use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::app::AppError;
use crate::model::{Node, TaskQuery};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiAdapterType {
    OpenAiCompatibleHttp,
    LocalHttp,
    LocalCli,
    Process,
    McpAgent,
}

impl AiAdapterType {
    pub fn label(self) -> &'static str {
        match self {
            Self::OpenAiCompatibleHttp => "OpenAI-compatible HTTP",
            Self::LocalHttp => "Local HTTP",
            Self::LocalCli => "Local CLI",
            Self::Process => "Process",
            Self::McpAgent => "MCP Agent",
        }
    }

    pub fn is_local(self) -> bool {
        matches!(self, Self::LocalHttp | Self::LocalCli | Self::Process)
    }

    pub fn may_use_network(self) -> bool {
        !matches!(self, Self::LocalHttp)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AiAuthScheme {
    #[default]
    None,
    Bearer,
}

impl AiAuthScheme {
    fn is_none(value: &Self) -> bool {
        *value == Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiIntent {
    Ask,
    Suggest,
    Transform,
    Extract,
    Audit,
    Prepare,
}

impl AiIntent {
    pub const ALL: [AiIntent; 6] = [
        AiIntent::Ask,
        AiIntent::Suggest,
        AiIntent::Transform,
        AiIntent::Extract,
        AiIntent::Audit,
        AiIntent::Prepare,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Ask => "Ask",
            Self::Suggest => "Suggest changes",
            Self::Transform => "Transform",
            Self::Extract => "Extract",
            Self::Audit => "Audit",
            Self::Prepare => "Prepare",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::Ask => "Answer a scoped question about the map.",
            Self::Suggest => "Propose map-native changes for the selected scope.",
            Self::Transform => "Reshape rough notes into cleaner structure.",
            Self::Extract => "Pull tasks, risks, questions, or decisions into structured nodes.",
            Self::Audit => "Review a scope for gaps, conflicts, or missing fields.",
            Self::Prepare => "Create a concise packet for handoff or follow-up.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiScope {
    CurrentNode,
    CurrentBranch,
    VisibleMap,
    WholeMap,
}

impl AiScope {
    pub fn label(self) -> &'static str {
        match self {
            Self::CurrentNode => "current node",
            Self::CurrentBranch => "current branch",
            Self::VisibleMap => "visible map",
            Self::WholeMap => "whole map",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiOutput {
    Answer,
    ReviewableMapEdits,
    Summary,
    HandoffPacket,
    Checklist,
    Observations,
}

impl AiOutput {
    pub fn label(self) -> &'static str {
        match self {
            Self::Answer => "answer",
            Self::ReviewableMapEdits => "reviewable map edits",
            Self::Summary => "summary",
            Self::HandoffPacket => "handoff packet",
            Self::Checklist => "checklist",
            Self::Observations => "observations",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiRequest {
    pub intent: AiIntent,
    pub scope: AiScope,
    pub output: AiOutput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instruction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

impl AiRequest {
    pub fn new(intent: AiIntent, scope: AiScope, output: AiOutput) -> Self {
        Self {
            intent,
            scope,
            output,
            instruction: None,
            profile: None,
        }
    }

    pub fn with_instruction(mut self, instruction: impl Into<String>) -> Self {
        let instruction = instruction.into();
        let trimmed = instruction.trim();
        self.instruction = (!trimmed.is_empty()).then(|| trimmed.to_string());
        self
    }

    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        let profile = profile.into();
        let trimmed = profile.trim();
        self.profile = (!trimmed.is_empty()).then(|| trimmed.to_string());
        self
    }

    pub fn display_title(&self) -> String {
        match &self.instruction {
            Some(instruction) => format!("{}: {instruction}", self.intent.label()),
            None => self.intent.label().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPreset {
    pub id: String,
    pub title: String,
    pub reason: String,
    pub request: AiRequest,
}

impl AiPreset {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        reason: impl Into<String>,
        request: AiRequest,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            reason: reason.into(),
            request,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiSuggestionTarget {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub label: String,
}

impl AiSuggestionTarget {
    pub fn new(path: Vec<usize>, id: Option<String>, label: impl Into<String>) -> Self {
        Self {
            path,
            id,
            label: label.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiSuggestion {
    pub target: AiSuggestionTarget,
    pub profile_label: String,
    pub question: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    pub changes: Vec<AiSuggestedChange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<AiSuggestionSource>,
}

impl AiSuggestion {
    pub fn new(
        target: AiSuggestionTarget,
        profile_label: impl Into<String>,
        question: impl Into<String>,
        changes: Vec<AiSuggestedChange>,
    ) -> Self {
        Self {
            target,
            profile_label: profile_label.into(),
            question: question.into(),
            answer: None,
            changes,
            source: None,
        }
    }

    pub fn answer(
        target: AiSuggestionTarget,
        profile_label: impl Into<String>,
        question: impl Into<String>,
        answer: impl Into<String>,
    ) -> Self {
        let answer = answer.into();
        Self {
            target,
            profile_label: profile_label.into(),
            question: question.into(),
            answer: (!answer.trim().is_empty()).then(|| answer.trim().to_string()),
            changes: Vec::new(),
            source: None,
        }
    }

    pub fn is_conversational_answer(&self) -> bool {
        self.answer.is_some() && self.changes.is_empty()
    }

    pub fn with_source(mut self, source: AiSuggestionSource) -> Self {
        self.source = Some(source);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AiSuggestionSource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_turn: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_estimate: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum AiSuggestedChange {
    AddChild {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<AiSuggestionTarget>,
        fragment: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        detail: String,
    },
    UpdateNode {
        target: AiSuggestionTarget,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fragment: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    RemoveNode {
        target: AiSuggestionTarget,
    },
}

impl AiSuggestedChange {
    pub fn operation_label(&self) -> &'static str {
        match self {
            Self::AddChild { .. } => "add child",
            Self::UpdateNode { .. } => "update node",
            Self::RemoveNode { .. } => "remove node",
        }
    }
}

pub fn map_assistant_system_prompt() -> &'static str {
    "You are helping inside mdmind, a local-first TUI for structured maps. Be concise, practical, and map-aware. Do not claim you changed the map directly. Default to conversational answers. The word \"suggest\" is an explicit signal when paired with map, branch, outline, node, edit, change, apply, review, or stage language, or when the user asks for new, additional, more, or missing items to add. Only propose reviewable map edits when the user explicitly asks for suggested map/branch/node/outline changes, additive suggestions such as \"suggest new characters\", reviewable map edits, staged suggestions, or changes they can apply. When proposing map edits, match the supplied chat context branch's local conventions for labels, task markers, tags, metadata keys, ids, detail lines, relations, and external references. Prefer repairing or enriching existing nodes over adding duplicate parallel structure."
}

pub fn reviewable_map_suggestion_contract() -> &'static str {
    r#"Map edit suggestion mode is ON because the user explicitly asked for reviewable map edits.
This fenced mdmind-suggestions block is mdmind's structured suggestion channel and the only tool-call-like output mdmind can apply from AI Chat today. Do not emit provider-native tool_calls. When the user asks you to suggest map/branch/node/outline changes, you must include exactly one fenced block labeled mdmind-suggestions after a concise prose answer:
```mdmind-suggestions
{
  "changes": [
    {
      "operation": "add_child",
      "target": {"id": "existing-node-id", "label": "Existing branch"},
      "fragment": "Short node label #optional-tag @optional:key",
      "detail": "Optional detail text."
    },
    {
      "operation": "update_node",
      "target": {"id": "existing-node-id", "label": "Existing branch"},
      "fragment": "Improved node label #optional-tag",
      "detail": "Optional replacement detail text."
    },
    {
      "operation": "remove_node",
      "target": {"id": "duplicate-or-obsolete-node-id", "label": "Duplicate or obsolete branch"}
    }
  ]
}
```
Choose the smallest honest operation for each useful edit:
- use add_child for missing branches, tasks, risks, examples, or supporting structure;
- use update_node when an existing node should be renamed, retagged, retasked, re-id'd, or have its detail text replaced;
- use remove_node only for clearly duplicate, obsolete, empty, or misleading nodes.
Before adding a child, check whether the supplied map context already has a node that should be updated instead. Do not express every idea as add_child. Omit target only for add_child rows that should land under the chat context branch. Include target for add_child rows that belong under an existing descendant, and always include target for update_node and remove_node rows. Prefer a stable id from the provided mdmind context, and include a readable label. Do not invent ids for targets.

Authoring style:
- Build a readable tree first; keep node labels short, scannable, and map-native.
- Match nearby branch conventions from the supplied mdmind context and style notes before inventing new structure.
- Use task markers, tags, metadata, ids, detail lines, relations, and external references only when they add clear value or match local patterns.
- Add ids only for durable branches that are likely to be revisited, linked, exported, or referenced; avoid ids on every new node.
- Put rationale, source notes, scene notes, and longer context in detail text instead of long labels.
- Add relations only to visible existing targets and only when the lateral meaning is valuable; prefer sparse, meaningful links.
- Preserve the user's framing and vocabulary unless the user asks for a new framework.

If there are no useful map edits, return an empty changes array. Do not claim the map was changed."#
}

pub fn conversational_only_contract() -> &'static str {
    "Map edit suggestion mode is OFF. Answer conversationally only. You may suggest ideas in prose, but do not include mdmind-suggestions JSON, tool calls, patches, or add/update/remove operations unless the output contract says suggestion mode is ON."
}

#[derive(Debug, Deserialize)]
struct AiSuggestedChangeEnvelope {
    #[serde(default)]
    changes: Vec<AiSuggestedChange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewableSuggestionExtraction {
    pub answer: String,
    pub changes: Vec<AiSuggestedChange>,
    pub warning: Option<String>,
}

pub fn split_reviewable_map_suggestions(answer: &str) -> (String, Vec<AiSuggestedChange>) {
    let extraction = split_reviewable_map_suggestions_with_warning(answer);
    (extraction.answer, extraction.changes)
}

pub fn split_reviewable_map_suggestions_with_warning(
    answer: &str,
) -> ReviewableSuggestionExtraction {
    let trimmed = answer.trim();
    if let Ok(changes) = parse_reviewable_map_suggestions(trimmed) {
        return ReviewableSuggestionExtraction {
            answer: String::new(),
            changes,
            warning: None,
        };
    }

    let mut cleaned = Vec::new();
    let mut changes = Vec::new();
    let mut warning = None;
    let mut lines = answer.lines();

    while let Some(line) = lines.next() {
        let trimmed_line = line.trim();
        let Some(fence_info) = trimmed_line.strip_prefix("```") else {
            cleaned.push(line.to_string());
            continue;
        };
        if !fence_info.trim().eq_ignore_ascii_case("mdmind-suggestions") {
            cleaned.push(line.to_string());
            continue;
        }

        let mut block = String::new();
        let mut closed = false;
        for block_line in lines.by_ref() {
            if block_line.trim_start().starts_with("```") {
                closed = true;
                break;
            }
            block.push_str(block_line);
            block.push('\n');
        }

        if closed {
            match parse_reviewable_map_suggestions(&block) {
                Ok(parsed) => {
                    changes.extend(parsed);
                    continue;
                }
                Err(error) => {
                    warning = Some(format!(
                        "Could not stage suggestions from the suggestion block: {}",
                        error.message()
                    ));
                    continue;
                }
            }
        }

        warning = Some(
            "Could not stage suggestions because the suggestion block was not closed.".to_string(),
        );
    }

    let mut answer = cleaned.join("\n").trim().to_string();
    if let Some(warning_text) = &warning {
        if !answer.is_empty() {
            answer.push_str("\n\n");
        }
        answer.push('[');
        answer.push_str(warning_text);
        answer.push_str(" Ask \"stage these as map edits\" to retry.]");
    }

    ReviewableSuggestionExtraction {
        answer,
        changes,
        warning,
    }
}

pub fn parse_reviewable_map_suggestions(
    json_text: &str,
) -> Result<Vec<AiSuggestedChange>, AppError> {
    let value: Value = serde_json::from_str(json_text).map_err(|error| {
        AppError::new(format!(
            "Could not parse reviewable AI suggestions JSON: {error}"
        ))
    })?;
    let mut changes: Vec<AiSuggestedChange> = if value.is_array() {
        serde_json::from_value(value).map_err(|error| {
            AppError::new(format!(
                "Could not parse reviewable AI suggestion rows: {error}"
            ))
        })?
    } else {
        serde_json::from_value::<AiSuggestedChangeEnvelope>(value)
            .map_err(|error| {
                AppError::new(format!(
                    "Could not parse reviewable AI suggestion envelope: {error}"
                ))
            })?
            .changes
    };
    changes.retain(ai_suggested_change_is_usable);
    Ok(changes)
}

fn ai_suggested_change_is_usable(change: &AiSuggestedChange) -> bool {
    match change {
        AiSuggestedChange::AddChild { fragment, .. } => !fragment.trim().is_empty(),
        AiSuggestedChange::UpdateNode {
            target,
            fragment,
            detail,
        } => {
            !target.label.trim().is_empty()
                && (fragment
                    .as_deref()
                    .is_some_and(|fragment| !fragment.trim().is_empty())
                    || detail
                        .as_deref()
                        .is_some_and(|detail| !detail.trim().is_empty()))
        }
        AiSuggestedChange::RemoveNode { target } => !target.label.trim().is_empty(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AiContextSignals {
    pub child_count: usize,
    pub detail_line_count: usize,
    pub task_like_count: usize,
    pub risk_like_count: usize,
    pub question_like_count: usize,
    pub has_stable_id: bool,
    pub reference_count: usize,
    pub relation_count: usize,
}

impl AiContextSignals {
    pub fn from_node(node: &Node) -> Self {
        let mut signals = Self {
            child_count: node.children.len(),
            detail_line_count: node
                .detail
                .iter()
                .filter(|line| !line.trim().is_empty())
                .count(),
            task_like_count: node
                .children
                .iter()
                .filter(|child| child.task_query_matches(TaskQuery::Any))
                .count(),
            risk_like_count: node
                .children
                .iter()
                .filter(|child| looks_like_risk(child))
                .count(),
            question_like_count: node
                .children
                .iter()
                .filter(|child| looks_like_question(child))
                .count(),
            has_stable_id: node.id.is_some(),
            reference_count: node.references.len(),
            relation_count: node.relations.len(),
        };

        if node.task_query_matches(TaskQuery::Any) {
            signals.task_like_count += 1;
        }
        if looks_like_risk(node) {
            signals.risk_like_count += 1;
        }
        if looks_like_question(node) {
            signals.question_like_count += 1;
        }

        signals
    }
}

pub fn contextual_presets_for_node(node: &Node) -> Vec<AiPreset> {
    contextual_presets_for_signals(&AiContextSignals::from_node(node))
}

pub fn contextual_presets_for_signals(signals: &AiContextSignals) -> Vec<AiPreset> {
    let mut presets = Vec::new();

    if signals.child_count == 0 {
        presets.push(AiPreset::new(
            "suggest-children",
            "Suggest children for this branch",
            "The branch has no children yet.",
            AiRequest::new(
                AiIntent::Suggest,
                AiScope::CurrentBranch,
                AiOutput::ReviewableMapEdits,
            )
            .with_instruction("Suggest useful child nodes for this branch."),
        ));
    }

    if signals.detail_line_count > 0 {
        presets.push(AiPreset::new(
            "extract-from-details",
            "Extract structure from details",
            "The selected branch has detail notes that may contain tasks, risks, or questions.",
            AiRequest::new(
                AiIntent::Extract,
                AiScope::CurrentBranch,
                AiOutput::ReviewableMapEdits,
            )
            .with_instruction(
                "Extract TODOs, risks, decisions, and open questions from the details.",
            ),
        ));
        presets.push(AiPreset::new(
            "summarize-details",
            "Summarize this branch as a detail",
            "The selected branch has prose details that can be compressed for handoff.",
            AiRequest::new(AiIntent::Prepare, AiScope::CurrentBranch, AiOutput::Summary)
                .with_instruction("Summarize the current branch without changing its structure."),
        ));
    }

    if signals.child_count >= 4 || signals.task_like_count > 0 || signals.risk_like_count > 0 {
        presets.push(AiPreset::new(
            "audit-gaps",
            "Audit this branch for gaps",
            "This branch has enough structure to review for missing owners, risks, or conflicts.",
            AiRequest::new(
                AiIntent::Audit,
                AiScope::CurrentBranch,
                AiOutput::Observations,
            )
            .with_instruction(
                "Find gaps, inconsistencies, missing owners, risks, and open questions.",
            ),
        ));
    }

    if signals.has_stable_id || signals.reference_count > 0 || signals.relation_count > 0 {
        presets.push(AiPreset::new(
            "prepare-handoff",
            "Prepare this branch for handoff",
            "Stable ids, references, or relations make this branch a good handoff candidate.",
            AiRequest::new(
                AiIntent::Prepare,
                AiScope::CurrentBranch,
                AiOutput::HandoffPacket,
            )
            .with_instruction("Create a concise handoff packet for another agent or teammate."),
        ));
    }

    presets
}

fn looks_like_risk(node: &Node) -> bool {
    node.has_tag("#risk")
        || node
            .metadata_value("type")
            .is_some_and(|value| value.eq_ignore_ascii_case("risk"))
        || node.text.to_ascii_lowercase().contains("risk")
}

fn looks_like_question(node: &Node) -> bool {
    node.has_tag("#question")
        || node.text.contains('?')
        || node
            .metadata_value("type")
            .is_some_and(|value| value.eq_ignore_ascii_case("question"))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AiCapabilities {
    #[serde(default)]
    pub streaming: bool,
    #[serde(default)]
    pub json_mode: bool,
    #[serde(default)]
    pub tool_calling: bool,
    #[serde(default)]
    pub local_execution: bool,
    #[serde(default)]
    pub filesystem_access: bool,
    #[serde(default)]
    pub web_access: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AiModelParameters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

impl AiModelParameters {
    fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiProfile {
    pub id: String,
    pub label: String,
    pub adapter_type: AiAdapterType,
    #[serde(default, skip_serializing_if = "AiAuthScheme::is_none")]
    pub auth_scheme: AiAuthScheme,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command_args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub small_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_ref: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub capabilities: AiCapabilities,
    #[serde(default, skip_serializing_if = "AiModelParameters::is_empty")]
    pub model_parameters: AiModelParameters,
}

impl AiProfile {
    pub fn openai_compatible(
        id: impl Into<String>,
        label: impl Into<String>,
        endpoint: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            adapter_type: AiAdapterType::OpenAiCompatibleHttp,
            auth_scheme: AiAuthScheme::Bearer,
            endpoint: Some(endpoint.into()),
            command: None,
            command_args: Vec::new(),
            model: Some(model.into()),
            small_model: None,
            secret_ref: None,
            headers: BTreeMap::new(),
            env: BTreeMap::new(),
            cwd: None,
            enabled: true,
            capabilities: AiCapabilities {
                json_mode: true,
                ..AiCapabilities::default()
            },
            model_parameters: AiModelParameters::default(),
        }
    }

    pub fn local_cli(
        id: impl Into<String>,
        label: impl Into<String>,
        command: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            adapter_type: AiAdapterType::LocalCli,
            auth_scheme: AiAuthScheme::None,
            endpoint: None,
            command: Some(command.into()),
            command_args: Vec::new(),
            model: None,
            small_model: None,
            secret_ref: None,
            headers: BTreeMap::new(),
            env: BTreeMap::new(),
            cwd: None,
            enabled: true,
            capabilities: AiCapabilities {
                local_execution: true,
                filesystem_access: true,
                ..AiCapabilities::default()
            },
            model_parameters: AiModelParameters::default(),
        }
    }

    pub fn nvidia_nim(secret_ref: Option<String>) -> Self {
        let mut profile = Self::openai_compatible(
            "nvidia-nim",
            "NVIDIA NIM",
            "https://integrate.api.nvidia.com/v1",
            "nvidia/llama-3.3-nemotron-super-49b-v1.5",
        );
        profile.secret_ref = Some(secret_ref.unwrap_or_else(|| "env:NVIDIA_API_KEY".to_string()));
        profile.model_parameters = AiModelParameters {
            temperature: Some("0.6".to_string()),
            top_p: Some("0.95".to_string()),
            max_tokens: Some(65_536),
            frequency_penalty: Some("0".to_string()),
            presence_penalty: Some("0".to_string()),
            stream: Some(false),
        };
        profile
    }

    pub fn codex_local() -> Self {
        let mut profile = Self::local_cli("codex-local", "Codex Local", "codex");
        profile.command_args = vec!["exec".to_string()];
        profile
    }

    pub fn claude_local() -> Self {
        let mut profile = Self::local_cli("claude-local", "Claude Local", "claude");
        profile.command_args = vec!["-p".to_string()];
        profile.capabilities.filesystem_access = false;
        profile
    }

    pub fn ollama_local(model: impl Into<String>) -> Self {
        Self {
            id: OLLAMA_LOCAL_QUICK_ADD_ID.to_string(),
            label: "Ollama Local".to_string(),
            adapter_type: AiAdapterType::LocalHttp,
            auth_scheme: AiAuthScheme::None,
            endpoint: Some(OLLAMA_OPENAI_COMPAT_ENDPOINT.to_string()),
            command: None,
            command_args: Vec::new(),
            model: Some(model.into()),
            small_model: None,
            secret_ref: None,
            headers: BTreeMap::new(),
            env: BTreeMap::new(),
            cwd: None,
            enabled: true,
            capabilities: AiCapabilities {
                streaming: true,
                local_execution: true,
                ..AiCapabilities::default()
            },
            model_parameters: AiModelParameters {
                temperature: Some("0.2".to_string()),
                ..AiModelParameters::default()
            },
        }
    }

    pub fn has_secret_material(&self) -> bool {
        self.headers
            .keys()
            .chain(self.env.keys())
            .any(|key| key.to_ascii_lowercase().contains("key"))
            || self
                .headers
                .values()
                .chain(self.env.values())
                .any(|value| looks_like_inline_secret(value))
            || self
                .secret_ref
                .as_deref()
                .is_some_and(looks_like_inline_secret)
    }
}

fn looks_like_inline_secret(value: &str) -> bool {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("bearer ") || lower.starts_with("sk-") || lower.starts_with("nvapi-")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AiQuickAddPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub summary: &'static str,
    pub adapter_type: AiAdapterType,
    pub default_model: Option<&'static str>,
    pub default_secret_ref: Option<&'static str>,
    pub docs_url: Option<&'static str>,
}

pub const NVIDIA_NIM_QUICK_ADD_ID: &str = "nvidia-nim";
pub const CODEX_LOCAL_QUICK_ADD_ID: &str = "codex-local";
pub const CLAUDE_LOCAL_QUICK_ADD_ID: &str = "claude-local";
pub const OLLAMA_LOCAL_QUICK_ADD_ID: &str = "ollama-local";
pub const NVIDIA_NIM_LOCAL_SECRET_ID: &str = "mdmind.ai.nvidia-nim";
pub const OLLAMA_DEFAULT_BASE_URL: &str = "http://127.0.0.1:11434";
pub const OLLAMA_OPENAI_COMPAT_ENDPOINT: &str = "http://127.0.0.1:11434/v1";
pub const OLLAMA_FALLBACK_MODEL: &str = "llama3.2:latest";

pub fn local_ai_secret_ref(secret_id: &str) -> String {
    format!("local:{secret_id}")
}

pub fn quick_add_presets() -> Vec<AiQuickAddPreset> {
    vec![
        AiQuickAddPreset {
            id: NVIDIA_NIM_QUICK_ADD_ID,
            label: "NVIDIA NIM",
            summary: "Use NVIDIA's OpenAI-compatible Chat Completions endpoint.",
            adapter_type: AiAdapterType::OpenAiCompatibleHttp,
            default_model: Some("nvidia/llama-3.3-nemotron-super-49b-v1.5"),
            default_secret_ref: Some("env:NVIDIA_API_KEY"),
            docs_url: Some("https://build.nvidia.com/settings/api-keys"),
        },
        AiQuickAddPreset {
            id: CODEX_LOCAL_QUICK_ADD_ID,
            label: "Codex Local",
            summary: "Use an installed local `codex exec` command as a CLI bridge.",
            adapter_type: AiAdapterType::LocalCli,
            default_model: None,
            default_secret_ref: None,
            docs_url: None,
        },
        AiQuickAddPreset {
            id: CLAUDE_LOCAL_QUICK_ADD_ID,
            label: "Claude Local",
            summary: "Use an installed local `claude -p` command as a constrained CLI bridge.",
            adapter_type: AiAdapterType::LocalCli,
            default_model: None,
            default_secret_ref: None,
            docs_url: Some("https://code.claude.com/docs/en/cli-reference"),
        },
        AiQuickAddPreset {
            id: OLLAMA_LOCAL_QUICK_ADD_ID,
            label: "Ollama Local",
            summary: "Use a running local Ollama server and one of its installed chat models.",
            adapter_type: AiAdapterType::LocalHttp,
            default_model: Some(OLLAMA_FALLBACK_MODEL),
            default_secret_ref: None,
            docs_url: Some("https://docs.ollama.com/api"),
        },
    ]
}

pub fn quick_add_profile(id: &str, secret_ref: Option<String>) -> Result<AiProfile, AppError> {
    match id {
        NVIDIA_NIM_QUICK_ADD_ID => Ok(AiProfile::nvidia_nim(secret_ref)),
        CODEX_LOCAL_QUICK_ADD_ID => {
            if secret_ref.is_some() {
                return Err(AppError::new(
                    "The Codex Local quick-add does not use an mdmind API secret.",
                ));
            }
            Ok(AiProfile::codex_local())
        }
        CLAUDE_LOCAL_QUICK_ADD_ID => {
            if secret_ref.is_some() {
                return Err(AppError::new(
                    "The Claude Local quick-add does not use an mdmind API secret.",
                ));
            }
            Ok(AiProfile::claude_local())
        }
        OLLAMA_LOCAL_QUICK_ADD_ID => {
            if secret_ref.is_some() {
                return Err(AppError::new(
                    "The Ollama Local quick-add does not use an mdmind API secret.",
                ));
            }
            Ok(AiProfile::ollama_local(OLLAMA_FALLBACK_MODEL))
        }
        _ => Err(AppError::new(format!(
            "Unknown AI quick-add preset '{id}'. Run `mdm ai presets` to list available presets."
        ))),
    }
}

pub fn ai_profile_is_codex_local_bridge(profile: &AiProfile) -> bool {
    let first_arg_is_exec = profile
        .command_args
        .first()
        .map(|arg| arg == "exec")
        .unwrap_or(true);
    profile.enabled
        && profile.adapter_type == AiAdapterType::LocalCli
        && profile
            .command
            .as_deref()
            .is_some_and(command_name_is_codex)
        && first_arg_is_exec
}

fn command_name_is_codex(command: &str) -> bool {
    Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name == "codex" || name == "codex.exe")
        .unwrap_or(false)
}

pub fn ai_profile_is_claude_local_bridge(profile: &AiProfile) -> bool {
    let first_arg_is_print = profile
        .command_args
        .first()
        .map(|arg| arg == "-p" || arg == "--print")
        .unwrap_or(true);
    profile.enabled
        && profile.adapter_type == AiAdapterType::LocalCli
        && profile
            .command
            .as_deref()
            .is_some_and(command_name_is_claude)
        && first_arg_is_print
}

fn command_name_is_claude(command: &str) -> bool {
    Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name == "claude" || name == "claude.exe")
        .unwrap_or(false)
}

pub fn ai_profile_is_ollama_local(profile: &AiProfile) -> bool {
    profile.enabled
        && profile.id == OLLAMA_LOCAL_QUICK_ADD_ID
        && profile.adapter_type == AiAdapterType::LocalHttp
        && profile.endpoint.as_deref().is_some_and(|endpoint| {
            let endpoint = endpoint.trim_end_matches('/');
            endpoint.ends_with("/v1") && is_local_http_url(endpoint)
        })
}

pub fn ai_profile_uses_chat_completions_http(profile: &AiProfile) -> bool {
    matches!(
        profile.adapter_type,
        AiAdapterType::OpenAiCompatibleHttp | AiAdapterType::LocalHttp
    )
}

pub fn openai_compatible_chat_url(profile: &AiProfile) -> Result<String, AppError> {
    if !ai_profile_uses_chat_completions_http(profile) {
        return Err(AppError::new(format!(
            "AI profile '{}' is not a chat-completions HTTP profile.",
            profile.label
        )));
    }

    let endpoint = profile
        .endpoint
        .as_deref()
        .ok_or_else(|| AppError::new(format!("AI profile '{}' has no endpoint.", profile.label)))?
        .trim_end_matches('/');
    if profile.adapter_type == AiAdapterType::LocalHttp && !is_local_http_url(endpoint) {
        return Err(AppError::new(format!(
            "AI profile '{}' is marked local-http, but its endpoint is not local.",
            profile.label
        )));
    }
    Ok(format!("{endpoint}/chat/completions"))
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AiLocalProviderDetection {
    pub ollama: Option<AiOllamaDiscovery>,
    pub codex_cli: bool,
    pub claude_cli: bool,
}

impl AiLocalProviderDetection {
    pub fn has_ollama_chat_model(&self) -> bool {
        self.ollama
            .as_ref()
            .and_then(|ollama| ollama.recommended_model.as_deref())
            .is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiOllamaDiscovery {
    pub base_url: String,
    pub openai_endpoint: String,
    pub models: Vec<AiOllamaModel>,
    pub recommended_model: Option<String>,
}

impl AiOllamaDiscovery {
    pub fn profile(&self) -> Option<AiProfile> {
        let model = self.recommended_model.as_deref()?;
        let mut profile = AiProfile::ollama_local(model);
        profile.endpoint = Some(self.openai_endpoint.clone());
        Some(profile)
    }

    pub fn model_count(&self) -> usize {
        self.models.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiOllamaModel {
    pub name: String,
    pub size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    #[serde(default)]
    models: Vec<OllamaTagsModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsModel {
    name: Option<String>,
    model: Option<String>,
    size: Option<u64>,
}

pub fn discover_local_ai_providers() -> AiLocalProviderDetection {
    AiLocalProviderDetection {
        ollama: discover_local_ollama(),
        codex_cli: local_command_available("codex"),
        claude_cli: local_command_available("claude"),
    }
}

pub fn discover_local_ollama() -> Option<AiOllamaDiscovery> {
    let base_url = ollama_base_url();
    let tags_url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(450))
        .build()
        .ok()?;
    let response = client.get(&tags_url).send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let text = response.text().ok()?;
    let payload: OllamaTagsResponse = serde_json::from_str(&text).ok()?;
    let mut models = Vec::new();
    for model in payload.models {
        let Some(name) = model.name.or(model.model) else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        models.push(AiOllamaModel {
            name: name.to_string(),
            size: model.size,
        });
    }
    let recommended_model =
        recommended_ollama_model(models.iter().map(|model| model.name.as_str()));
    Some(AiOllamaDiscovery {
        openai_endpoint: format!("{}/v1", base_url.trim_end_matches('/')),
        base_url,
        models,
        recommended_model,
    })
}

fn ollama_base_url() -> String {
    std::env::var("OLLAMA_HOST")
        .ok()
        .and_then(|host| normalize_local_ollama_host(&host))
        .unwrap_or_else(|| OLLAMA_DEFAULT_BASE_URL.to_string())
}

fn normalize_local_ollama_host(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    };
    is_local_http_url(&with_scheme).then_some(with_scheme)
}

fn is_local_http_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    local_http_host_matches(&lower, "localhost")
        || lower.starts_with("http://127.")
        || local_http_host_matches(&lower, "[::1]")
        || local_http_host_matches(&lower, "0.0.0.0")
}

fn local_http_host_matches(url: &str, host: &str) -> bool {
    let prefix = format!("http://{host}");
    let Some(rest) = url.strip_prefix(&prefix) else {
        return false;
    };
    rest.is_empty() || rest.starts_with(':') || rest.starts_with('/')
}

pub fn recommended_ollama_model<'a>(models: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let mut best: Option<(i32, String)> = None;
    for model in models {
        let model = model.trim();
        if model.is_empty() {
            continue;
        }
        let score = ollama_model_score(model);
        if score < 0 {
            continue;
        }
        match &best {
            Some((best_score, _)) if *best_score >= score => {}
            _ => best = Some((score, model.to_string())),
        }
    }
    best.map(|(_, model)| model)
}

fn ollama_model_score(model: &str) -> i32 {
    let lower = model.to_ascii_lowercase();
    let mut score = 10;

    if [
        "embed",
        "embedding",
        "nomic-embed",
        "all-minilm",
        "bge-",
        "mxbai",
        "rerank",
        "whisper",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        return -1000;
    }

    for (needle, weight) in [
        ("qwen3", 90),
        ("qwen2.5", 82),
        ("llama3.3", 80),
        ("llama3.2", 76),
        ("llama3.1", 74),
        ("llama3", 70),
        ("mistral", 66),
        ("gemma3", 64),
        ("gemma2", 58),
        ("phi4", 56),
        ("deepseek", 54),
        ("mixtral", 52),
        ("codellama", 42),
        ("llava", 30),
    ] {
        if lower.contains(needle) {
            score += weight;
            break;
        }
    }

    if lower.contains("instruct") {
        score += 20;
    }
    if lower.contains("chat") {
        score += 16;
    }
    if lower.contains(":latest") {
        score += 4;
    }
    if lower.contains(":70b") || lower.contains(":72b") || lower.contains(":90b") {
        score -= 8;
    }
    if lower.contains(":1b") || lower.contains(":0.") {
        score -= 6;
    }

    score
}

pub fn local_command_available(command: &str) -> bool {
    let command = command.trim();
    if command.is_empty() {
        return false;
    }
    let command_path = Path::new(command);
    if command_path.components().count() > 1 {
        return executable_file_exists(command_path);
    }

    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };

    std::env::split_paths(&paths).any(|directory| {
        let candidate = directory.join(command);
        if executable_file_exists(&candidate) {
            return true;
        }
        #[cfg(windows)]
        {
            if candidate.extension().is_none() {
                return ["exe", "cmd", "bat"]
                    .iter()
                    .any(|extension| executable_file_exists(&candidate.with_extension(extension)));
            }
        }
        false
    })
}

fn executable_file_exists(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub fn openai_compatible_chat_body(
    profile: &AiProfile,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<Value, AppError> {
    openai_compatible_chat_body_with_stream(profile, system_prompt, user_prompt, None)
}

pub fn openai_compatible_chat_body_with_stream(
    profile: &AiProfile,
    system_prompt: &str,
    user_prompt: &str,
    stream_override: Option<bool>,
) -> Result<Value, AppError> {
    let model = profile
        .model
        .as_deref()
        .ok_or_else(|| AppError::new(format!("AI profile '{}' has no model.", profile.label)))?;

    let mut body = serde_json::Map::new();
    body.insert("model".to_string(), json!(model));
    body.insert(
        "messages".to_string(),
        json!([
            {
                "role": "system",
                "content": system_prompt,
            },
            {
                "role": "user",
                "content": user_prompt,
            }
        ]),
    );
    insert_optional_number_or_string(
        &mut body,
        "temperature",
        profile.model_parameters.temperature.as_deref(),
    );
    insert_optional_number_or_string(
        &mut body,
        "top_p",
        profile.model_parameters.top_p.as_deref(),
    );
    if let Some(max_tokens) = profile.model_parameters.max_tokens {
        body.insert("max_tokens".to_string(), json!(max_tokens));
    }
    insert_optional_number_or_string(
        &mut body,
        "frequency_penalty",
        profile.model_parameters.frequency_penalty.as_deref(),
    );
    insert_optional_number_or_string(
        &mut body,
        "presence_penalty",
        profile.model_parameters.presence_penalty.as_deref(),
    );
    if let Some(stream) = stream_override.or(profile.model_parameters.stream) {
        body.insert("stream".to_string(), json!(stream));
    }

    Ok(Value::Object(body))
}

fn insert_optional_number_or_string(
    body: &mut serde_json::Map<String, Value>,
    key: &str,
    value: Option<&str>,
) {
    if let Some(value) = value {
        let json_value = value
            .parse::<f64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::from(value.to_string()));
        body.insert(key.to_string(), json_value);
    }
}

pub fn send_openai_compatible_chat(
    profile: &AiProfile,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, AppError> {
    let body = openai_compatible_chat_body(profile, system_prompt, user_prompt)?;
    let response = send_openai_compatible_chat_request(profile, body)?;
    let status = response.status();
    let text = response.text().map_err(|error| {
        AppError::new(format!(
            "Could not read AI response from '{}': {error}",
            profile.label
        ))
    })?;

    if !status.is_success() {
        return Err(AppError::new(format!(
            "AI request failed with HTTP {status}: {}",
            truncate_for_error(&text, 700)
        )));
    }

    chat_response_content(&text)
}

pub fn send_openai_compatible_chat_streaming(
    profile: &AiProfile,
    system_prompt: &str,
    user_prompt: &str,
    mut on_delta: impl FnMut(&str),
) -> Result<String, AppError> {
    send_openai_compatible_chat_streaming_cancellable(
        profile,
        system_prompt,
        user_prompt,
        |delta| on_delta(delta),
        || false,
    )
}

pub fn send_openai_compatible_chat_streaming_cancellable(
    profile: &AiProfile,
    system_prompt: &str,
    user_prompt: &str,
    mut on_delta: impl FnMut(&str),
    mut is_cancelled: impl FnMut() -> bool,
) -> Result<String, AppError> {
    let body =
        openai_compatible_chat_body_with_stream(profile, system_prompt, user_prompt, Some(true))?;
    let response = send_openai_compatible_chat_request(profile, body)?;
    let status = response.status();
    if !status.is_success() {
        let text = response.text().map_err(|error| {
            AppError::new(format!(
                "Could not read AI response from '{}': {error}",
                profile.label
            ))
        })?;
        return Err(AppError::new(format!(
            "AI request failed with HTTP {status}: {}",
            truncate_for_error(&text, 700)
        )));
    }

    let mut answer = String::new();
    let reader = BufReader::new(response);
    for line in reader.lines() {
        if is_cancelled() {
            return Err(AppError::new("AI request was cancelled."));
        }
        let line = line.map_err(|error| {
            AppError::new(format!(
                "Could not read streaming AI response from '{}': {error}",
                profile.label
            ))
        })?;
        let trimmed = line.trim();
        let Some(data) = trimmed.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data == "[DONE]" {
            break;
        }
        if data.is_empty() {
            continue;
        }
        if let Some(delta) = chat_stream_delta_content(data)? {
            if is_cancelled() {
                return Err(AppError::new("AI request was cancelled."));
            }
            on_delta(&delta);
            if !is_cancelled() {
                answer.push_str(&delta);
            }
        }
    }

    let answer = answer.trim().to_string();
    if answer.is_empty() {
        return Err(AppError::new("AI streaming response was empty."));
    }
    Ok(answer)
}

pub fn send_ai_chat_streaming_cancellable(
    profile: &AiProfile,
    system_prompt: &str,
    user_prompt: &str,
    workspace_cwd: Option<&Path>,
    on_delta: impl FnMut(&str),
    is_cancelled: impl FnMut() -> bool,
) -> Result<String, AppError> {
    match profile.adapter_type {
        AiAdapterType::OpenAiCompatibleHttp | AiAdapterType::LocalHttp => {
            send_openai_compatible_chat_streaming_cancellable(
                profile,
                system_prompt,
                user_prompt,
                on_delta,
                is_cancelled,
            )
        }
        AiAdapterType::LocalCli if ai_profile_is_codex_local_bridge(profile) => {
            send_codex_local_chat_streaming_cancellable(
                profile,
                system_prompt,
                user_prompt,
                workspace_cwd,
                on_delta,
                is_cancelled,
            )
        }
        AiAdapterType::LocalCli if ai_profile_is_claude_local_bridge(profile) => {
            send_claude_local_chat_streaming_cancellable(
                profile,
                system_prompt,
                user_prompt,
                workspace_cwd,
                on_delta,
                is_cancelled,
            )
        }
        AiAdapterType::LocalCli => Err(AppError::new(format!(
            "AI profile '{}' is a Local CLI profile, but only the Codex Local and Claude Local bridges are callable from AI Chat right now.",
            profile.label
        ))),
        _ => Err(AppError::new(format!(
            "AI profile '{}' cannot power AI Chat from the TUI yet.",
            profile.label
        ))),
    }
}

pub fn codex_local_exec_args(
    profile: &AiProfile,
    workspace_cwd: Option<&Path>,
    output_last_message: Option<&Path>,
) -> Result<Vec<String>, AppError> {
    if !ai_profile_is_codex_local_bridge(profile) {
        return Err(AppError::new(format!(
            "AI profile '{}' is not a Codex Local bridge.",
            profile.label
        )));
    }
    reject_dangerous_codex_args(profile)?;

    let mut args = if profile.command_args.is_empty() {
        vec!["exec".to_string()]
    } else {
        profile.command_args.clone()
    };
    args.extend([
        "--json".to_string(),
        "--ephemeral".to_string(),
        "--sandbox".to_string(),
        "read-only".to_string(),
        "--color".to_string(),
        "never".to_string(),
        "--skip-git-repo-check".to_string(),
    ]);
    if let Some(model) = profile
        .model
        .as_deref()
        .filter(|model| !model.trim().is_empty())
    {
        args.extend(["--model".to_string(), model.trim().to_string()]);
    }
    if let Some(cwd) = profile
        .cwd
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| workspace_cwd.map(Path::to_path_buf))
    {
        args.extend(["--cd".to_string(), cwd.display().to_string()]);
    }
    if let Some(output_last_message) = output_last_message {
        args.extend([
            "--output-last-message".to_string(),
            output_last_message.display().to_string(),
        ]);
    }
    args.push("-".to_string());
    Ok(args)
}

pub fn claude_local_exec_args(profile: &AiProfile) -> Result<Vec<String>, AppError> {
    if !ai_profile_is_claude_local_bridge(profile) {
        return Err(AppError::new(format!(
            "AI profile '{}' is not a Claude Local bridge.",
            profile.label
        )));
    }
    reject_dangerous_claude_args(profile)?;

    let mut args = if profile.command_args.is_empty() {
        vec!["-p".to_string()]
    } else {
        profile.command_args.clone()
    };
    args.extend([
        "--output-format".to_string(),
        "stream-json".to_string(),
        "--verbose".to_string(),
        "--include-partial-messages".to_string(),
        "--permission-mode".to_string(),
        "plan".to_string(),
        "--tools".to_string(),
        String::new(),
        "--max-turns".to_string(),
        "1".to_string(),
        "--no-session-persistence".to_string(),
    ]);
    if let Some(model) = profile
        .model
        .as_deref()
        .filter(|model| !model.trim().is_empty())
    {
        args.extend(["--model".to_string(), model.trim().to_string()]);
    }
    args.push(
        "Answer the mdmind AI Chat prompt supplied on stdin. Return only the response that should appear in AI Chat."
            .to_string(),
    );
    Ok(args)
}

fn reject_dangerous_codex_args(profile: &AiProfile) -> Result<(), AppError> {
    let mut args = profile.command_args.iter().map(String::as_str).peekable();
    while let Some(arg) = args.next() {
        if matches!(
            arg,
            "--dangerously-bypass-approvals-and-sandbox" | "--dangerously-bypass-hook-trust"
        ) {
            return Err(AppError::new(format!(
                "AI profile '{}' uses dangerous Codex flags. Remove {arg} before using it in mdmind.",
                profile.label
            )));
        }
        if arg == "--sandbox" {
            let Some(value) = args.next() else {
                continue;
            };
            if value != "read-only" {
                return Err(AppError::new(format!(
                    "AI profile '{}' sets Codex sandbox to {value}. mdmind only allows read-only Codex Local chat.",
                    profile.label
                )));
            }
        } else if let Some(value) = arg.strip_prefix("--sandbox=")
            && value != "read-only"
        {
            return Err(AppError::new(format!(
                "AI profile '{}' sets Codex sandbox to {value}. mdmind only allows read-only Codex Local chat.",
                profile.label
            )));
        }
    }
    Ok(())
}

fn reject_dangerous_claude_args(profile: &AiProfile) -> Result<(), AppError> {
    let mut args = profile.command_args.iter().map(String::as_str).peekable();
    while let Some(arg) = args.next() {
        if matches!(
            arg,
            "--dangerously-skip-permissions" | "--allow-dangerously-skip-permissions"
        ) {
            return Err(AppError::new(format!(
                "AI profile '{}' uses dangerous Claude flags. Remove {arg} before using it in mdmind.",
                profile.label
            )));
        }
        if arg == "--permission-mode" {
            let Some(value) = args.next() else {
                continue;
            };
            if value != "plan" {
                return Err(AppError::new(format!(
                    "AI profile '{}' sets Claude permission mode to {value}. mdmind only allows Claude Local chat in plan mode.",
                    profile.label
                )));
            }
        } else if let Some(value) = arg.strip_prefix("--permission-mode=")
            && value != "plan"
        {
            return Err(AppError::new(format!(
                "AI profile '{}' sets Claude permission mode to {value}. mdmind only allows Claude Local chat in plan mode.",
                profile.label
            )));
        }
        if arg == "--tools" {
            let Some(value) = args.next() else {
                continue;
            };
            if !value.is_empty() {
                return Err(AppError::new(format!(
                    "AI profile '{}' customizes Claude tools. mdmind only allows Claude Local chat with tools disabled.",
                    profile.label
                )));
            }
        } else if let Some(value) = arg.strip_prefix("--tools=")
            && !value.is_empty()
        {
            return Err(AppError::new(format!(
                "AI profile '{}' customizes Claude tools. mdmind only allows Claude Local chat with tools disabled.",
                profile.label
            )));
        }
    }
    Ok(())
}

pub fn local_codex_bridge_prompt(system_prompt: &str, user_prompt: &str) -> String {
    format!(
        "You are Codex Local running as a read-only AI Chat provider inside mdmind.\n\
Do not modify files, write patches, run destructive commands, or claim you changed the map.\n\
Use the mdmind branch and chat context supplied below as the source of truth.\n\
Return only the response that should appear in AI Chat. If the user explicitly asks you to suggest map, branch, node, or outline edits, or asks for additive suggestions such as new, additional, more, or missing items, follow the mdmind-suggestions contract in the prompt.\n\n\
mdmind system instruction:\n{system_prompt}\n\n\
mdmind user/context payload:\n{user_prompt}\n"
    )
}

pub fn local_claude_bridge_prompt(system_prompt: &str, user_prompt: &str) -> String {
    format!(
        "You are Claude Local running as a constrained AI Chat provider inside mdmind.\n\
Do not modify files, write patches, run commands, or claim you changed the map.\n\
Use only the mdmind branch and chat context supplied below as the source of truth.\n\
Return only the response that should appear in AI Chat. If the user explicitly asks you to suggest map, branch, node, or outline edits, or asks for additive suggestions such as new, additional, more, or missing items, follow the mdmind-suggestions contract in the prompt.\n\n\
mdmind system instruction:\n{system_prompt}\n\n\
mdmind user/context payload:\n{user_prompt}\n"
    )
}

fn send_codex_local_chat_streaming_cancellable(
    profile: &AiProfile,
    system_prompt: &str,
    user_prompt: &str,
    workspace_cwd: Option<&Path>,
    mut on_delta: impl FnMut(&str),
    mut is_cancelled: impl FnMut() -> bool,
) -> Result<String, AppError> {
    let command_name = profile.command.as_deref().ok_or_else(|| {
        AppError::new(format!(
            "AI profile '{}' has no local command configured.",
            profile.label
        ))
    })?;
    let output_path = unique_codex_output_path();
    let args = codex_local_exec_args(profile, workspace_cwd, Some(&output_path))?;
    let prompt = local_codex_bridge_prompt(system_prompt, user_prompt);

    let mut command = Command::new(command_name);
    command
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = profile
        .cwd
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| workspace_cwd.map(Path::to_path_buf))
    {
        command.current_dir(cwd);
    }
    for (key, value) in &profile.env {
        command.env(key, value);
    }

    let mut child = command.spawn().map_err(|error| {
        AppError::new(format!(
            "Could not start Codex Local bridge '{}': {error}",
            command_name
        ))
    })?;
    let mut stdin = child.stdin.take().ok_or_else(|| {
        AppError::new(format!(
            "Could not open stdin for Codex Local bridge '{}'.",
            profile.label
        ))
    })?;
    let stdout = child.stdout.take().ok_or_else(|| {
        AppError::new(format!(
            "Could not read stdout from Codex Local bridge '{}'.",
            profile.label
        ))
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        AppError::new(format!(
            "Could not read stderr from Codex Local bridge '{}'.",
            profile.label
        ))
    })?;

    thread::spawn(move || {
        let _ = stdin.write_all(prompt.as_bytes());
    });

    let (line_sender, line_receiver) = mpsc::channel();
    spawn_local_cli_reader(
        stdout,
        line_sender.clone(),
        LocalCliStream::Stdout,
        "Codex Local",
    );
    spawn_local_cli_reader(stderr, line_sender, LocalCliStream::Stderr, "Codex Local");

    let mut answer = String::new();
    let mut stderr_text = String::new();
    let mut stdout_done = false;
    let mut stderr_done = false;
    let mut exit_status = None;

    loop {
        if is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = fs::remove_file(&output_path);
            return Err(AppError::new("AI request was cancelled."));
        }

        match line_receiver.recv_timeout(Duration::from_millis(80)) {
            Ok(LocalCliLine::Stdout(line)) => {
                if let Some(delta) = codex_exec_json_event_text(&line) {
                    if !delta.is_empty() {
                        on_delta(&delta);
                        answer.push_str(&delta);
                    }
                } else if !line.trim().is_empty() && !line.trim_start().starts_with('{') {
                    let delta = if answer.is_empty() {
                        line
                    } else {
                        format!("\n{line}")
                    };
                    on_delta(&delta);
                    answer.push_str(&delta);
                }
            }
            Ok(LocalCliLine::Stderr(line)) => {
                if !stderr_text.is_empty() {
                    stderr_text.push('\n');
                }
                stderr_text.push_str(&line);
            }
            Ok(LocalCliLine::Done(LocalCliStream::Stdout)) => stdout_done = true,
            Ok(LocalCliLine::Done(LocalCliStream::Stderr)) => stderr_done = true,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                stdout_done = true;
                stderr_done = true;
            }
        }

        if exit_status.is_none() {
            exit_status = child.try_wait().map_err(|error| {
                AppError::new(format!("Could not poll Codex Local bridge: {error}"))
            })?;
        }
        if exit_status.is_some() && stdout_done && stderr_done {
            break;
        }
    }

    let status = exit_status.unwrap_or_else(|| {
        child
            .wait()
            .expect("Codex Local bridge should be waitable after spawn")
    });
    let final_answer = fs::read_to_string(&output_path)
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());
    let _ = fs::remove_file(&output_path);

    if !status.success() {
        let detail = if !stderr_text.trim().is_empty() {
            stderr_text.trim()
        } else if !answer.trim().is_empty() {
            answer.trim()
        } else {
            "Codex exited without a diagnostic."
        };
        return Err(AppError::new(format!(
            "Codex Local bridge failed with status {status}: {}",
            truncate_for_error(detail, 700)
        )));
    }

    if let Some(final_answer) = final_answer {
        answer = final_answer;
    }
    let answer = answer.trim().to_string();
    if answer.is_empty() {
        let suffix = if stderr_text.trim().is_empty() {
            String::new()
        } else {
            format!(" Stderr: {}", truncate_for_error(stderr_text.trim(), 300))
        };
        return Err(AppError::new(format!(
            "Codex Local bridge returned an empty response.{suffix}"
        )));
    }
    Ok(answer)
}

fn send_claude_local_chat_streaming_cancellable(
    profile: &AiProfile,
    system_prompt: &str,
    user_prompt: &str,
    workspace_cwd: Option<&Path>,
    mut on_delta: impl FnMut(&str),
    mut is_cancelled: impl FnMut() -> bool,
) -> Result<String, AppError> {
    let command_name = profile.command.as_deref().ok_or_else(|| {
        AppError::new(format!(
            "AI profile '{}' has no local command configured.",
            profile.label
        ))
    })?;
    let args = claude_local_exec_args(profile)?;
    let prompt = local_claude_bridge_prompt(system_prompt, user_prompt);

    let mut command = Command::new(command_name);
    command
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = profile
        .cwd
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| workspace_cwd.map(Path::to_path_buf))
    {
        command.current_dir(cwd);
    }
    for (key, value) in &profile.env {
        command.env(key, value);
    }

    let mut child = command.spawn().map_err(|error| {
        AppError::new(format!(
            "Could not start Claude Local bridge '{}': {error}",
            command_name
        ))
    })?;
    let mut stdin = child.stdin.take().ok_or_else(|| {
        AppError::new(format!(
            "Could not open stdin for Claude Local bridge '{}'.",
            profile.label
        ))
    })?;
    let stdout = child.stdout.take().ok_or_else(|| {
        AppError::new(format!(
            "Could not read stdout from Claude Local bridge '{}'.",
            profile.label
        ))
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        AppError::new(format!(
            "Could not read stderr from Claude Local bridge '{}'.",
            profile.label
        ))
    })?;

    thread::spawn(move || {
        let _ = stdin.write_all(prompt.as_bytes());
    });

    let (line_sender, line_receiver) = mpsc::channel();
    spawn_local_cli_reader(
        stdout,
        line_sender.clone(),
        LocalCliStream::Stdout,
        "Claude Local",
    );
    spawn_local_cli_reader(stderr, line_sender, LocalCliStream::Stderr, "Claude Local");

    let mut answer = String::new();
    let mut stderr_text = String::new();
    let mut stdout_done = false;
    let mut stderr_done = false;
    let mut exit_status = None;

    loop {
        if is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::new("AI request was cancelled."));
        }

        match line_receiver.recv_timeout(Duration::from_millis(80)) {
            Ok(LocalCliLine::Stdout(line)) => {
                if let Some(delta) = claude_code_stream_json_event_text(&line)
                    && !delta.is_empty()
                {
                    on_delta(&delta);
                    answer.push_str(&delta);
                }
            }
            Ok(LocalCliLine::Stderr(line)) => {
                if !stderr_text.is_empty() {
                    stderr_text.push('\n');
                }
                stderr_text.push_str(&line);
            }
            Ok(LocalCliLine::Done(LocalCliStream::Stdout)) => stdout_done = true,
            Ok(LocalCliLine::Done(LocalCliStream::Stderr)) => stderr_done = true,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                stdout_done = true;
                stderr_done = true;
            }
        }

        if exit_status.is_none() {
            exit_status = child.try_wait().map_err(|error| {
                AppError::new(format!("Could not poll Claude Local bridge: {error}"))
            })?;
        }
        if exit_status.is_some() && stdout_done && stderr_done {
            break;
        }
    }

    let status = exit_status.unwrap_or_else(|| {
        child
            .wait()
            .expect("Claude Local bridge should be waitable after spawn")
    });

    if !status.success() {
        let detail = if !stderr_text.trim().is_empty() {
            stderr_text.trim()
        } else if !answer.trim().is_empty() {
            answer.trim()
        } else {
            "Claude exited without a diagnostic."
        };
        return Err(AppError::new(format!(
            "Claude Local bridge failed with status {status}: {}",
            truncate_for_error(detail, 700)
        )));
    }

    let answer = answer.trim().to_string();
    if answer.is_empty() {
        let suffix = if stderr_text.trim().is_empty() {
            String::new()
        } else {
            format!(" Stderr: {}", truncate_for_error(stderr_text.trim(), 300))
        };
        return Err(AppError::new(format!(
            "Claude Local bridge returned an empty response.{suffix}"
        )));
    }
    Ok(answer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalCliStream {
    Stdout,
    Stderr,
}

#[derive(Debug)]
enum LocalCliLine {
    Stdout(String),
    Stderr(String),
    Done(LocalCliStream),
}

fn spawn_local_cli_reader<R>(
    reader: R,
    sender: mpsc::Sender<LocalCliLine>,
    stream: LocalCliStream,
    label: &'static str,
) where
    R: std::io::Read + Send + 'static,
{
    thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            match line {
                Ok(line) => {
                    let event = match stream {
                        LocalCliStream::Stdout => LocalCliLine::Stdout(line),
                        LocalCliStream::Stderr => LocalCliLine::Stderr(line),
                    };
                    if sender.send(event).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    let _ = sender.send(LocalCliLine::Stderr(format!(
                        "Could not read {label} output: {error}"
                    )));
                    break;
                }
            }
        }
        let _ = sender.send(LocalCliLine::Done(stream));
    });
}

fn unique_codex_output_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "mdmind-codex-local-{}-{nonce}.txt",
        std::process::id()
    ))
}

pub fn codex_exec_json_event_text(line: &str) -> Option<String> {
    let value: Value = serde_json::from_str(line).ok()?;
    let event_type = codex_event_type(&value);
    let item = value.get("item");
    let item_type = item
        .and_then(|item| item.get("type"))
        .and_then(Value::as_str);
    let item_role = item
        .and_then(|item| item.get("role"))
        .and_then(Value::as_str);
    let is_assistant_text = event_type.contains("agent_message")
        || event_type.contains("assistant")
        || event_type.contains("output_text")
        || event_type.contains("final_answer")
        || item_role == Some("assistant")
        || item_type.is_some_and(|kind| kind == "assistant_message");
    if !is_assistant_text {
        return None;
    }

    [
        value.get("delta"),
        value.get("text"),
        value.get("message"),
        value.get("content"),
        item.and_then(|item| item.get("delta")),
        item.and_then(|item| item.get("text")),
        item.and_then(|item| item.get("message")),
        item.and_then(|item| item.get("content")),
    ]
    .into_iter()
    .flatten()
    .find_map(collect_json_text)
    .map(|text| text.to_string())
    .filter(|text| !text.is_empty())
}

pub fn claude_code_stream_json_event_text(line: &str) -> Option<String> {
    let value: Value = serde_json::from_str(line).ok()?;
    let event = value.get("event").unwrap_or(&value);
    if event.get("type").and_then(Value::as_str) != Some("content_block_delta") {
        return None;
    }
    let delta = event.get("delta")?;
    if delta.get("type").and_then(Value::as_str) != Some("text_delta") {
        return None;
    }
    delta
        .get("text")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .filter(|text| !text.is_empty())
}

fn codex_event_type(value: &Value) -> String {
    value
        .get("type")
        .or_else(|| value.get("event"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn collect_json_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => {
            let text = items
                .iter()
                .filter_map(collect_json_text)
                .collect::<Vec<_>>()
                .join("");
            (!text.is_empty()).then_some(text)
        }
        Value::Object(object) => object
            .get("text")
            .or_else(|| object.get("content"))
            .or_else(|| object.get("delta"))
            .and_then(collect_json_text),
        _ => None,
    }
}

fn send_openai_compatible_chat_request(
    profile: &AiProfile,
    body: Value,
) -> Result<reqwest::blocking::Response, AppError> {
    let url = openai_compatible_chat_url(profile)?;

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| AppError::new(format!("Could not initialize AI HTTP client: {error}")))?;
    let mut request = client.post(&url).header("Content-Type", "application/json");

    for (key, value) in &profile.headers {
        request = request.header(key, value);
    }

    if profile.auth_scheme == AiAuthScheme::Bearer {
        let secret_ref = profile.secret_ref.as_deref().ok_or_else(|| {
            AppError::new(format!(
                "AI profile '{}' needs a secret before it can send requests.",
                profile.label
            ))
        })?;
        request = request.bearer_auth(resolve_ai_secret_ref(secret_ref)?);
    }

    let request_body =
        serde_json::to_string(&body).expect("OpenAI-compatible chat body should serialize");
    request.body(request_body).send().map_err(|error| {
        AppError::new(format!(
            "Could not reach AI endpoint for '{}': {error}",
            profile.label
        ))
    })
}

fn chat_response_content(text: &str) -> Result<String, AppError> {
    let value: Value = serde_json::from_str(text)
        .map_err(|error| AppError::new(format!("Could not parse AI response JSON: {error}")))?;
    let content = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::new("AI response did not include choices[0].message.content."))?;
    let content = content.trim();
    if content.is_empty() {
        return Err(AppError::new("AI response was empty."));
    }
    Ok(content.to_string())
}

fn chat_stream_delta_content(data: &str) -> Result<Option<String>, AppError> {
    let value: Value = serde_json::from_str(data).map_err(|error| {
        AppError::new(format!(
            "Could not parse streaming AI response JSON: {error}"
        ))
    })?;
    let Some(choice) = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
    else {
        return Ok(None);
    };
    if let Some(content) = choice
        .get("delta")
        .and_then(|delta| delta.get("content"))
        .and_then(Value::as_str)
    {
        return Ok(Some(content.to_string()));
    }
    if let Some(content) = choice
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
    {
        return Ok(Some(content.to_string()));
    }
    Ok(None)
}

fn truncate_for_error(value: &str, max_chars: usize) -> String {
    let mut result = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        result.push_str("...");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::chat_stream_delta_content;

    #[test]
    fn parses_openai_compatible_streaming_delta_content() {
        let data = r#"{"choices":[{"delta":{"content":"hello"}}]}"#;

        assert_eq!(
            chat_stream_delta_content(data).expect("delta JSON should parse"),
            Some("hello".to_string())
        );
    }

    #[test]
    fn ignores_streaming_finish_chunks_without_content() {
        let data = r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#;

        assert_eq!(
            chat_stream_delta_content(data).expect("finish JSON should parse"),
            None
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiProfilesConfig {
    #[serde(
        default = "ai_profiles_enabled_default",
        skip_serializing_if = "is_true"
    )]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_profile: Option<String>,
    #[serde(default)]
    pub profiles: Vec<AiProfile>,
}

#[derive(Debug, Deserialize)]
struct LegacyAiProfilesConfig {
    #[serde(default = "ai_profiles_enabled_default")]
    enabled: bool,
    #[serde(default)]
    default_profile: Option<String>,
    #[serde(default)]
    active_profile_id: Option<String>,
    #[serde(default)]
    profiles: Vec<LegacyAiProfile>,
}

#[derive(Debug, Deserialize)]
struct LegacyAiProfile {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    endpoint: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    command_args: Vec<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    small_model: Option<String>,
    #[serde(default)]
    secret_ref: Option<String>,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default = "ai_profiles_enabled_default")]
    enabled: bool,
}

impl Default for AiProfilesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_profile: None,
            profiles: Vec::new(),
        }
    }
}

fn ai_profiles_enabled_default() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

impl AiProfilesConfig {
    pub fn default_profile(&self) -> Option<&AiProfile> {
        let id = self.default_profile.as_ref()?;
        self.profiles.iter().find(|profile| &profile.id == id)
    }

    pub fn profile(&self, id: &str) -> Option<&AiProfile> {
        self.profiles.iter().find(|profile| profile.id == id)
    }

    pub fn set_default_profile(&mut self, id: &str) -> Result<(), AppError> {
        let profile = self
            .profile(id)
            .ok_or_else(|| AppError::new(format!("No AI profile matches '{id}'.")))?;
        if !profile.enabled {
            return Err(AppError::new(format!(
                "AI profile '{}' is disabled.",
                profile.label
            )));
        }
        self.enabled = true;
        self.default_profile = Some(id.to_string());
        Ok(())
    }

    pub fn upsert_profile(&mut self, profile: AiProfile, make_default: bool) {
        let id = profile.id.clone();
        match self
            .profiles
            .iter()
            .position(|existing| existing.id == profile.id)
        {
            Some(index) => self.profiles[index] = profile,
            None => self.profiles.push(profile),
        }

        if make_default || self.default_profile.is_none() {
            self.default_profile = Some(id);
        }
        if make_default {
            self.enabled = true;
        }
    }

    pub fn turn_off(&mut self) {
        self.enabled = false;
    }
}

pub fn ai_profiles_path_for_config_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("mdmind").join("ai-profiles.json")
}

pub fn ai_secrets_path_for_config_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("mdmind").join("ai-secrets.json")
}

pub fn ai_profiles_path() -> Result<PathBuf, AppError> {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        return Ok(ai_profiles_path_for_config_dir(Path::new(&config_home)));
    }

    if let Some(home) = std::env::var_os("HOME") {
        return Ok(Path::new(&home)
            .join(".config")
            .join("mdmind")
            .join("ai-profiles.json"));
    }

    Err(AppError::new(
        "Could not find a config directory for mdmind AI profiles.",
    ))
}

pub fn ai_secrets_path() -> Result<PathBuf, AppError> {
    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        return Ok(ai_secrets_path_for_config_dir(Path::new(&config_home)));
    }

    if let Some(home) = std::env::var_os("HOME") {
        return Ok(Path::new(&home)
            .join(".config")
            .join("mdmind")
            .join("ai-secrets.json"));
    }

    Err(AppError::new(
        "Could not find a config directory for mdmind AI secrets.",
    ))
}

fn migrate_legacy_ai_profiles_config(contents: &str) -> Result<AiProfilesConfig, String> {
    let legacy: LegacyAiProfilesConfig =
        serde_json::from_str(contents).map_err(|error| error.to_string())?;
    let original_profile_count = legacy.profiles.len();
    let mut profiles = Vec::new();

    for legacy_profile in legacy.profiles {
        let Some(profile) = legacy_ai_profile_to_current(legacy_profile) else {
            continue;
        };
        match profiles
            .iter()
            .position(|existing: &AiProfile| existing.id == profile.id)
        {
            Some(index) => profiles[index] = profile,
            None => profiles.push(profile),
        }
    }

    if original_profile_count > 0 && profiles.is_empty() {
        return Err("no recognizable legacy AI profiles".to_string());
    }

    let requested_default = non_empty_legacy_string(legacy.default_profile)
        .or_else(|| non_empty_legacy_string(legacy.active_profile_id));
    let default_profile = legacy_default_profile_id(requested_default.as_deref(), &profiles)
        .or_else(|| {
            profiles
                .iter()
                .find(|profile| profile.enabled)
                .map(|profile| profile.id.clone())
        })
        .or_else(|| profiles.first().map(|profile| profile.id.clone()));

    Ok(AiProfilesConfig {
        enabled: legacy.enabled,
        default_profile,
        profiles,
    })
}

fn legacy_ai_profile_to_current(legacy: LegacyAiProfile) -> Option<AiProfile> {
    let LegacyAiProfile {
        id,
        label,
        provider,
        endpoint,
        command,
        command_args,
        model,
        small_model,
        secret_ref,
        headers,
        env,
        cwd,
        enabled,
    } = legacy;

    let provider = non_empty_legacy_string(provider);
    let id = non_empty_legacy_string(id).or_else(|| provider.clone())?;
    let label = non_empty_legacy_string(label);
    let endpoint = non_empty_legacy_string(endpoint);
    let command = non_empty_legacy_string(command);
    let model = non_empty_legacy_string(model);
    let small_model = non_empty_legacy_string(small_model);
    let secret_ref = non_empty_legacy_string(secret_ref);
    let cwd = non_empty_legacy_string(cwd);

    let id_key = legacy_profile_key(&id);
    let provider_key = provider.as_deref().map(legacy_profile_key);
    let command_is_codex = command.as_deref().is_some_and(command_name_is_codex)
        || endpoint
            .as_deref()
            .filter(|value| !legacy_string_looks_like_url(value))
            .is_some_and(command_name_is_codex);
    let command_is_claude = command.as_deref().is_some_and(command_name_is_claude)
        || endpoint
            .as_deref()
            .filter(|value| !legacy_string_looks_like_url(value))
            .is_some_and(command_name_is_claude);

    let mut profile = if legacy_profile_matches(&id_key, provider_key.as_deref(), "codex-local")
        || legacy_profile_matches(&id_key, provider_key.as_deref(), "codex")
        || command_is_codex
    {
        let mut profile = AiProfile::codex_local();
        if let Some(command) = command.or_else(|| {
            endpoint
                .clone()
                .filter(|value| !legacy_string_looks_like_url(value))
        }) {
            profile.command = Some(command);
        }
        if !command_args.is_empty() {
            profile.command_args = command_args;
        }
        profile
    } else if legacy_profile_matches(&id_key, provider_key.as_deref(), "claude-local")
        || legacy_profile_matches(&id_key, provider_key.as_deref(), "claude")
        || command_is_claude
    {
        let mut profile = AiProfile::claude_local();
        if let Some(command) = command.or_else(|| {
            endpoint
                .clone()
                .filter(|value| !legacy_string_looks_like_url(value))
        }) {
            profile.command = Some(command);
        }
        if !command_args.is_empty() {
            profile.command_args = command_args;
        }
        profile
    } else if legacy_profile_matches(&id_key, provider_key.as_deref(), OLLAMA_LOCAL_QUICK_ADD_ID)
        || legacy_profile_matches(&id_key, provider_key.as_deref(), "ollama")
    {
        let mut profile = AiProfile::ollama_local(
            model
                .clone()
                .unwrap_or_else(|| OLLAMA_FALLBACK_MODEL.to_string()),
        );
        if let Some(endpoint) = endpoint
            .clone()
            .filter(|value| legacy_string_looks_like_url(value))
        {
            profile.endpoint = Some(legacy_ollama_openai_endpoint(&endpoint));
        }
        profile
    } else if legacy_profile_matches(&id_key, provider_key.as_deref(), NVIDIA_NIM_QUICK_ADD_ID)
        || legacy_profile_matches(&id_key, provider_key.as_deref(), "nvidia")
        || legacy_profile_matches(&id_key, provider_key.as_deref(), "nim")
    {
        let mut profile = AiProfile::nvidia_nim(secret_ref.clone());
        if let Some(endpoint) = endpoint
            .clone()
            .filter(|value| legacy_string_looks_like_url(value))
        {
            profile.endpoint = Some(endpoint);
        }
        if let Some(model) = model.clone() {
            profile.model = Some(model);
        }
        profile
    } else {
        let endpoint = endpoint.filter(|value| legacy_string_looks_like_url(value))?;
        let mut profile = AiProfile::openai_compatible(
            id,
            label.clone().unwrap_or_else(|| "AI Provider".to_string()),
            endpoint,
            model.clone().unwrap_or_default(),
        );
        if model.is_none() {
            profile.model = None;
        }
        profile
    };

    if let Some(label) = label {
        profile.label = label;
    }
    profile.small_model = small_model;
    profile.secret_ref = secret_ref.or(profile.secret_ref);
    profile.headers = headers;
    profile.env = env;
    profile.cwd = cwd;
    profile.enabled = enabled;

    Some(profile)
}

fn non_empty_legacy_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn legacy_profile_key(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(['_', ' '], "-")
}

fn legacy_profile_matches(id_key: &str, provider_key: Option<&str>, expected: &str) -> bool {
    id_key == expected || provider_key == Some(expected)
}

fn legacy_default_profile_id(requested: Option<&str>, profiles: &[AiProfile]) -> Option<String> {
    let requested = requested?;
    if let Some(profile) = profiles.iter().find(|profile| profile.id == requested) {
        return Some(profile.id.clone());
    }

    let requested_key = legacy_profile_key(requested);
    let canonical_id = match requested_key.as_str() {
        "codex" | CODEX_LOCAL_QUICK_ADD_ID => Some(CODEX_LOCAL_QUICK_ADD_ID),
        "claude" | CLAUDE_LOCAL_QUICK_ADD_ID => Some(CLAUDE_LOCAL_QUICK_ADD_ID),
        "ollama" | OLLAMA_LOCAL_QUICK_ADD_ID => Some(OLLAMA_LOCAL_QUICK_ADD_ID),
        "nvidia" | "nim" | NVIDIA_NIM_QUICK_ADD_ID => Some(NVIDIA_NIM_QUICK_ADD_ID),
        _ => None,
    }?;

    profiles
        .iter()
        .find(|profile| profile.id == canonical_id)
        .map(|profile| profile.id.clone())
}

fn legacy_string_looks_like_url(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

fn legacy_ollama_openai_endpoint(endpoint: &str) -> String {
    let trimmed = endpoint.trim().trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}

pub fn load_ai_profiles_from_path(path: &Path) -> Result<AiProfilesConfig, AppError> {
    if !path.exists() {
        return Ok(AiProfilesConfig::default());
    }

    let contents = fs::read_to_string(path).map_err(|error| {
        AppError::new(format!(
            "Could not read AI profiles '{}': {error}",
            path.display()
        ))
    })?;

    match serde_json::from_str(&contents) {
        Ok(config) => Ok(config),
        Err(error) => migrate_legacy_ai_profiles_config(&contents).map_err(|_| {
            AppError::new(format!(
                "Could not parse AI profiles '{}': {error}",
                path.display()
            ))
        }),
    }
}

pub fn save_ai_profiles_to_path(path: &Path, config: &AiProfilesConfig) -> Result<(), AppError> {
    if config.profiles.iter().any(AiProfile::has_secret_material) {
        return Err(AppError::new(
            "AI profiles cannot store raw API key material; use secret_ref instead.",
        ));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::new(format!(
                "Could not create AI profile directory '{}': {error}",
                parent.display()
            ))
        })?;
    }

    let contents = serde_json::to_string_pretty(config).expect("AI profiles should serialize");
    fs::write(path, contents).map_err(|error| {
        AppError::new(format!(
            "Could not write AI profiles '{}': {error}",
            path.display()
        ))
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AiLocalSecrets {
    #[serde(default)]
    pub secrets: BTreeMap<String, String>,
}

pub fn load_local_ai_secrets_from_path(path: &Path) -> Result<AiLocalSecrets, AppError> {
    if !path.exists() {
        return Ok(AiLocalSecrets::default());
    }

    let contents = fs::read_to_string(path).map_err(|error| {
        AppError::new(format!(
            "Could not read AI secrets '{}': {error}",
            path.display()
        ))
    })?;

    serde_json::from_str(&contents).map_err(|error| {
        AppError::new(format!(
            "Could not parse AI secrets '{}': {error}",
            path.display()
        ))
    })
}

pub fn resolve_ai_secret_ref(secret_ref: &str) -> Result<String, AppError> {
    if let Some(env_name) = secret_ref.strip_prefix("env:") {
        return std::env::var(env_name).map_err(|_| {
            AppError::new(format!(
                "AI secret environment variable '{env_name}' is not set."
            ))
        });
    }

    if let Some(secret_id) = secret_ref.strip_prefix("local:") {
        let path = ai_secrets_path()?;
        return resolve_local_ai_secret_from_path(&path, secret_id);
    }

    Err(AppError::new(format!(
        "Unsupported AI secret reference '{secret_ref}'. Use env:NAME or local:secret-id."
    )))
}

pub fn resolve_local_ai_secret_from_path(path: &Path, secret_id: &str) -> Result<String, AppError> {
    validate_local_secret_id(secret_id)?;
    let config = load_local_ai_secrets_from_path(path)?;
    config.secrets.get(secret_id).cloned().ok_or_else(|| {
        AppError::new(format!(
            "AI secret '{secret_id}' was not found in '{}'.",
            path.display()
        ))
    })
}

pub fn save_local_ai_secret(secret_id: &str, secret_value: &str) -> Result<PathBuf, AppError> {
    let path = ai_secrets_path()?;
    save_local_ai_secret_to_path(&path, secret_id, secret_value)?;
    Ok(path)
}

pub fn save_local_ai_secret_to_path(
    path: &Path,
    secret_id: &str,
    secret_value: &str,
) -> Result<(), AppError> {
    validate_local_secret_id(secret_id)?;
    let secret_value = secret_value.trim();
    if secret_value.is_empty() {
        return Err(AppError::new("AI secret value must not be empty."));
    }

    let mut config = load_local_ai_secrets_from_path(path)?;
    config
        .secrets
        .insert(secret_id.to_string(), secret_value.to_string());
    save_local_ai_secrets_to_path(path, &config)
}

fn validate_local_secret_id(secret_id: &str) -> Result<(), AppError> {
    if secret_id.is_empty() {
        return Err(AppError::new("AI secret id must not be empty."));
    }

    if !secret_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
    {
        return Err(AppError::new(
            "AI secret ids may only contain letters, numbers, '.', '-', and '_'.",
        ));
    }

    Ok(())
}

fn save_local_ai_secrets_to_path(path: &Path, config: &AiLocalSecrets) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::new(format!(
                "Could not create AI secret directory '{}': {error}",
                parent.display()
            ))
        })?;
    }

    let contents = serde_json::to_string_pretty(config).expect("AI secrets should serialize");

    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .map_err(|error| {
                AppError::new(format!(
                    "Could not write AI secrets '{}': {error}",
                    path.display()
                ))
            })?;
        file.write_all(contents.as_bytes()).map_err(|error| {
            AppError::new(format!(
                "Could not write AI secrets '{}': {error}",
                path.display()
            ))
        })?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|error| {
            AppError::new(format!(
                "Could not secure AI secrets '{}': {error}",
                path.display()
            ))
        })
    }

    #[cfg(not(unix))]
    {
        fs::write(path, contents).map_err(|error| {
            AppError::new(format!(
                "Could not write AI secrets '{}': {error}",
                path.display()
            ))
        })
    }
}
