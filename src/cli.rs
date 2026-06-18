use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitCode};

#[cfg(test)]
use clap::CommandFactory;
use clap::builder::PossibleValuesParser;
use clap::error::ErrorKind;
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use serde::Serialize;
use serde_json::json;

use crate::APP_VERSION;
use crate::ai::{
    ai_profiles_path, load_ai_profiles_from_path, quick_add_presets, quick_add_profile,
    save_ai_profiles_to_path,
};
use crate::app::{
    AppError, ClassifiedTarget, OpenTargetMode, classify_open_target, create_from_template,
    diagnostics_for_validate, diagnostics_have_errors, ensure_parseable, load_document,
    near_miss_guidance, resolve_anchor_path, select_document,
};
use crate::changelog::{
    changelog_entries, changelog_entry, default_changelog_entry, render_changelog_entry,
    render_full_changelog,
};
use crate::examples::{
    all as bundled_examples, discover_examples_dir, find as find_example,
    readme_contents as examples_readme_contents,
};
use crate::export::export_document;
use crate::importer::import_document;
use crate::interactive::{
    TuiExperiment, TuiFeatureFlags, run_interactive_with_mode,
    run_interactive_with_mode_and_features, run_key_diagnostics,
};
use crate::markdown_render::{ColorMode, RenderOptions, RenderTarget, render_markdown};
use crate::mindspace::{
    MINDSPACE_CONTEXT_FORMAT, MINDSPACE_DIAGNOSTICS_FORMAT, MINDSPACE_REVIEW_FORMAT,
    MINDSPACE_SCAN_FORMAT, MINDSPACE_SESSION_FORMAT, MINDSPACE_SETUP_FORMAT,
    MINDSPACE_TEMPLATE_CATALOG_FORMAT, MINDSPACE_TEMPLATE_FORMAT, MindspaceContextOptions,
    MindspaceDiagnosticsReport, MindspaceSetupMode, approve_mindspace_review,
    close_mindspace_session, context_mindspace, list_mindspace_reviews, mindspace_template,
    mindspace_template_catalog, plan_mindspace_session, preview_mindspace_session_apply,
    reject_mindspace_review, render_mindspace_context, render_mindspace_context_plain,
    render_mindspace_diagnostics, render_mindspace_diagnostics_plain, render_mindspace_review,
    render_mindspace_review_list, render_mindspace_review_list_plain,
    render_mindspace_review_plain, render_mindspace_scan, render_mindspace_scan_plain,
    render_mindspace_session_apply, render_mindspace_session_apply_plain,
    render_mindspace_session_report, render_mindspace_session_report_plain, render_mindspace_setup,
    render_mindspace_setup_plain, render_mindspace_template, render_mindspace_template_catalog,
    render_mindspace_template_catalog_plain, render_mindspace_template_plain,
    render_mindspace_template_prompt, render_mindspace_workspace_landing, scan_mindspace,
    setup_mindspace, start_mindspace_session, submit_mindspace_session, workspace_mindspace,
};
use crate::model::{Document, ExternalRefKind, Node, Severity, TaskState};
use crate::query::{
    filter_document, find_matches, link_entries, metadata_rows, reference_entries,
    relation_entries, relation_entries_for_path, tag_counts,
};
use crate::render::{
    render_find, render_find_plain, render_links, render_links_plain, render_metadata,
    render_metadata_plain, render_references, render_references_plain, render_relations,
    render_relations_plain, render_tags, render_tags_plain, render_tree, render_validate,
    render_validate_plain,
};
use crate::serializer::serialize_document;
use crate::startup::{choose_mindspace_target, choose_startup_target};
use crate::templates::TemplateKind;
use crate::updates::{UpdateCheck, check_for_updates};
use crate::validate::validate_document;

const MDMIND_SKILL_INSTALL_PROGRAM: &str = "npx";
const MDMIND_SKILL_INSTALL_ARGS: &[&str] = &["skills", "add", "dudash/mdmind"];

#[derive(Debug, Parser)]
#[command(
    name = "mdm",
    version,
    about = "Inspect and validate local markdown-like thought maps.",
    long_about = "mdm is the CLI for local-first structured maps. It reads plain-text tree files, renders them for humans, and exports machine-friendly output when you ask for --json or --plain.",
    after_help = "Examples:\n  mdm version\n  mdm changelog\n  mdm skills install\n  mdm view-markdown README.md\n  mdm init ideas.md --template product\n  mdm init TODO.md --template todo\n  mdm import notes.opml\n  mdm import map.mm\n  mdm import article.html --preview --report\n  mdm import outline.md --from markdown -o map.md\n  mdm view ideas.md\n  mdm find ideas.md \"rate limit\"\n  mdm find ideas.md \"#todo\" --plain\n  mdm kv ideas.md --keys status,owner\n  mdm links ideas.md\n  mdm refs ideas.md\n  mdm relations ideas.md#product/api-design\n  mdm validate ideas.md\n  mdm export ideas.md --format json\n  mdm export ideas.md#product/mvp --format mermaid\n  mdm export ideas.md --format opml\n  mdm export ideas.md --query \"#todo @status:active\" --format json\n  mdm open ideas.md#product/api-design"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "Render a read-only tree view for a map or deep link.")]
    View {
        target: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        max_depth: Option<usize>,
    },
    #[command(
        about = "View an ordinary Markdown file in the terminal.",
        after_help = "Examples:\n  mdm view-markdown README.md\n  mdm view-markdown CHANGELOG.md --width 100\n  mdm view-markdown README.md --plain"
    )]
    ViewMarkdown {
        target: PathBuf,
        #[arg(long, action = ArgAction::SetTrue, help = "Show formatted Markdown output.")]
        pretty: bool,
        #[arg(long, action = ArgAction::SetTrue, help = "Print the raw Markdown source unchanged.")]
        plain: bool,
        #[arg(
            long,
            default_value_t = 88,
            help = "Preferred wrap width for pretty output."
        )]
        width: usize,
        #[arg(long, action = ArgAction::SetTrue, help = "Disable color output.")]
        no_color: bool,
    },
    #[command(about = "Search labels, tags, metadata, and ids.")]
    Find {
        target: String,
        query: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "List tag counts across a map.")]
    Tags {
        target: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "List inline key:value metadata.")]
    Kv {
        target: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        keys: Vec<String>,
    },
    #[command(about = "List every id and its deep-linkable path.")]
    Links {
        target: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "List external file, URL, and image references.")]
    Refs {
        target: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "List outgoing relations, or incoming backlinks for a deep-linked node.")]
    Relations {
        target: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "Validate structure, ids, and metadata conventions.")]
    Validate {
        target: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "Export normalized representations.")]
    Export {
        target: String,
        #[arg(
            long,
            default_value = "json",
            help = "Export format: json, mermaid, or opml."
        )]
        format: String,
        #[arg(long)]
        query: Option<String>,
    },
    #[command(about = "Create a new map from a starter template.")]
    Init {
        path: PathBuf,
        #[arg(long, value_parser = PossibleValuesParser::new(TemplateKind::names()))]
        template: String,
        #[arg(long, action = ArgAction::SetTrue)]
        force: bool,
    },
    #[command(
        about = "Import an external outline or map into a native mdmind map.",
        long_about = "Import OPML, FreeMind .mm, Markdown outlines, local HTML files, or remote web pages into mdmind's native Markdown map format. OPML, FreeMind, and Markdown are faithful outline import paths. HTML and web are lossy structural ingestion paths that keep headings, lists, and paragraphs. XMind, MindManager, and PDF are recognized as planned paths and return guided errors.",
        after_help = "Examples:\n  mdm import notes.opml\n  mdm import map.mm\n  mdm import https://example.com --preview --report\n  mdm import article.html --preview --report\n  mdm import outline.md --from markdown -o outline-mind.md\n\nCurrent writing formats: opml, freemind, markdown, html, web.\nPlanned/guided formats: xmind, mindmanager, pdf.\nFormats can be inferred from URLs and from .opml, .mm, .html, .htm, .md, .markdown, .mdown, .mkd, .xmind, .mmap, and .pdf. Use --from when the extension is missing or ambiguous. If -o/--output is omitted for a writing import, mdm writes beside the source as <source-stem>-mind.md."
    )]
    Import {
        #[arg(help = "Source file or URL to import.")]
        source: PathBuf,
        #[arg(
            long,
            value_parser = PossibleValuesParser::new([
                "freemind",
                "html",
                "markdown",
                "mindmanager",
                "opml",
                "pdf",
                "web",
                "xmind"
            ]),
            help = "Source format. Inferred from the extension when omitted."
        )]
        from: Option<String>,
        #[arg(
            short = 'o',
            long = "output",
            help = "Destination native .md map. Defaults to <source-stem>-mind.md unless --preview is set."
        )]
        output: Option<PathBuf>,
        #[arg(long, action = ArgAction::SetTrue, help = "Overwrite an existing output file.")]
        force: bool,
        #[arg(
            long,
            action = ArgAction::SetTrue,
            help = "Print the generated map to stdout without writing a file."
        )]
        preview: bool,
        #[arg(
            long,
            action = ArgAction::SetTrue,
            help = "Print import quality and preservation stats to stderr."
        )]
        report: bool,
    },
    #[command(about = "List, locate, or copy the bundled example maps.")]
    Examples {
        #[command(subcommand)]
        command: ExampleCommands,
    },
    #[command(about = "Manage experimental AI provider profiles and quick-add presets.")]
    Ai {
        #[command(subcommand)]
        command: AiCommands,
    },
    #[command(about = "Install or print agent skill setup commands.")]
    Skills {
        #[command(subcommand)]
        command: SkillCommands,
    },
    #[command(
        about = "Inspect and lint optional folder-level Mindspace workspaces.",
        after_help = "Examples:\n  mdm mindspace scan .\n  mdm mindspace setup . --preview\n  mdm mindspace setup . --write\n  mdm mindspace context maps/roadmap.md#roadmap/current\n  mdm mindspace lint .\n  mdm mindspace template list\n  mdm mindspace template show launch-planning --prompt\n  mdm mindspace session start maps/roadmap.md#roadmap/current --role implementer --json\n  mdm mindspace review list --json"
    )]
    Mindspace {
        #[command(subcommand)]
        command: MindspaceCommands,
    },
    #[command(
        name = "commands",
        about = "Print the mdm command catalog for agents and scripts."
    )]
    Catalog {
        #[arg(long)]
        json: bool,
    },
    #[command(
        about = "Read curated mdmind changelog entries.",
        after_help = "Examples:\n  mdm changelog\n  mdm changelog --version 0.8.0\n  mdm changelog --plain\n  mdm changelog --all\n  mdm changelog --json"
    )]
    Changelog {
        #[arg(long, help = "Show the changelog entry for a version, such as 0.8.0.")]
        version: Option<String>,
        #[arg(long, action = ArgAction::SetTrue, help = "Print the full changelog.")]
        all: bool,
        #[arg(long, action = ArgAction::SetTrue, help = "Render terminal-friendly Markdown output explicitly.")]
        pretty: bool,
        #[arg(long, action = ArgAction::SetTrue, help = "Print raw Markdown output.")]
        plain: bool,
        #[arg(
            long,
            action = ArgAction::SetTrue,
            help = "Print an agent-readable JSON envelope."
        )]
        json: bool,
    },
    #[command(about = "Open a map or deep link in the interactive navigator.")]
    Open {
        target: String,
        #[arg(long)]
        preview: bool,
        #[arg(long)]
        autosave: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        max_depth: Option<usize>,
    },
    #[command(about = "Check how this terminal reports Alt+arrow keys.")]
    CheckKeys,
    #[command(about = "Print the mdm version or check for newer releases.")]
    Version {
        #[arg(long, action = ArgAction::SetTrue, help = "Check GitHub Releases for the latest mdmind version.")]
        check: bool,
        #[arg(long, action = ArgAction::SetTrue, help = "Print an agent-readable JSON envelope.")]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ExampleCommands {
    #[command(about = "List the bundled example maps.")]
    List,
    #[command(about = "Print the installed on-disk examples directory, when available.")]
    Path,
    #[command(about = "Copy one bundled example, or all examples, into a directory.")]
    Copy {
        name: String,
        #[arg(long)]
        to: Option<PathBuf>,
        #[arg(long, action = ArgAction::SetTrue)]
        force: bool,
    },
}

#[derive(Debug, Subcommand)]
enum AiCommands {
    #[command(about = "List built-in AI quick-add presets.")]
    Presets {
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Add a built-in AI profile preset to the local mdmind config.")]
    QuickAdd {
        #[arg(help = "Preset id, such as nvidia-nim or codex-local.")]
        preset: String,
        #[arg(
            long,
            help = "Secret reference, such as env:NVIDIA_API_KEY or keychain:mdmind.ai.nvidia-nim. Raw API keys are rejected."
        )]
        secret_ref: Option<String>,
        #[arg(long, action = ArgAction::SetTrue, help = "Make the added profile the default.")]
        default: bool,
        #[arg(long, help = "Override the AI profile config path.")]
        config: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum SkillCommands {
    #[command(
        about = "Install the bundled mdmind agent skills through the skills CLI.",
        after_help = "Runs:\n  npx skills add dudash/mdmind\n\nExamples:\n  mdm skills install\n  mdm skills install --print"
    )]
    Install {
        #[arg(
            long,
            action = ArgAction::SetTrue,
            help = "Print the underlying npx command without running it."
        )]
        print: bool,
    },
}

#[derive(Debug, Subcommand)]
enum MindspaceCommands {
    #[command(about = "Inspect a folder read-only, with or without a Mindspace manifest.")]
    Scan {
        root: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "Report deterministic Mindspace diagnostics without AI judgment.")]
    Lint {
        root: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "Preview or write a minimal Mindspace manifest.")]
    Setup {
        root: PathBuf,
        #[arg(long, action = ArgAction::SetTrue, help = "Print the proposed manifest without writing.")]
        preview: bool,
        #[arg(long, action = ArgAction::SetTrue, help = "Write only .mdmind/mindspace.json.")]
        write: bool,
        #[arg(long, help = "Optional job template id used for setup guidance.")]
        template: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "Export a bounded Mindspace context bundle with provenance.")]
    Context {
        target: String,
        #[arg(
            long,
            help = "Mindspace root to scan; defaults to the current directory."
        )]
        root: Option<PathBuf>,
        #[arg(long, help = "Filter query to include matching branches across maps.")]
        query: Option<String>,
        #[arg(long, help = "Optional job template id used for context guidance.")]
        template: Option<String>,
        #[arg(
            long,
            default_value_t = 1,
            help = "Outgoing relation depth to include."
        )]
        relation_depth: usize,
        #[arg(long, action = ArgAction::SetTrue, help = "Include incoming relation sources.")]
        include_backlinks: bool,
        #[arg(long, action = ArgAction::SetTrue, help = "Include bounded local source reference excerpts.")]
        include_source_refs: bool,
        #[arg(
            long,
            default_value_t = 8,
            help = "Maximum distinct map files to include."
        )]
        max_files: usize,
        #[arg(
            long,
            default_value_t = 24,
            help = "Maximum branch records to include."
        )]
        max_branches: usize,
        #[arg(
            long,
            default_value_t = 4000,
            help = "Maximum detail characters across included branches."
        )]
        max_detail_chars: usize,
        #[arg(
            long,
            default_value_t = 800,
            help = "Maximum characters per included source excerpt."
        )]
        max_source_chars: usize,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "Manage durable Mindspace agent session records.")]
    Session {
        #[command(subcommand)]
        command: MindspaceSessionCommands,
    },
    #[command(about = "List and decide durable Mindspace review records.")]
    Review {
        #[command(subcommand)]
        command: MindspaceReviewCommands,
    },
    #[command(about = "List and inspect built-in Mindspace job templates.")]
    Template {
        #[command(subcommand)]
        command: MindspaceTemplateCommands,
    },
}

#[derive(Debug, Subcommand)]
enum MindspaceSessionCommands {
    #[command(about = "Start a durable agent session record for a target.")]
    Start {
        target: String,
        #[arg(long)]
        role: String,
        #[arg(long)]
        goal: Option<String>,
        #[arg(long, help = "Mindspace root; defaults to the current directory.")]
        root: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "Show a session plan and current target digest.")]
    Plan {
        session_id: String,
        #[arg(long, help = "Mindspace root; defaults to the current directory.")]
        root: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "Preview session review state before future writeback.")]
    Apply {
        session_id: String,
        #[arg(long, action = ArgAction::SetTrue)]
        preview: bool,
        #[arg(long, help = "Mindspace root; defaults to the current directory.")]
        root: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "Submit a session result as a durable review item.")]
    Submit {
        session_id: String,
        #[arg(long)]
        rationale: String,
        #[arg(long)]
        proposal: Option<String>,
        #[arg(long, help = "Mindspace root; defaults to the current directory.")]
        root: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "Close a durable agent session record.")]
    Close {
        session_id: String,
        #[arg(long, help = "Mindspace root; defaults to the current directory.")]
        root: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
}

#[derive(Debug, Subcommand)]
enum MindspaceReviewCommands {
    #[command(about = "List durable review items.")]
    List {
        #[arg(long, help = "Mindspace root; defaults to the current directory.")]
        root: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "Approve a review item after digest checks.")]
    Approve {
        review_id: String,
        #[arg(long, help = "Mindspace root; defaults to the current directory.")]
        root: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "Reject a review item with a reason.")]
    Reject {
        review_id: String,
        #[arg(long)]
        reason: String,
        #[arg(long, help = "Mindspace root; defaults to the current directory.")]
        root: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
}

#[derive(Debug, Subcommand)]
enum MindspaceTemplateCommands {
    #[command(about = "List built-in Mindspace job templates.")]
    List {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
    },
    #[command(about = "Show one built-in Mindspace job template.")]
    Show {
        #[arg(help = "Template id, such as launch-planning or claims-evidence.")]
        id: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        plain: bool,
        #[arg(
            long,
            action = ArgAction::SetTrue,
            help = "Print a copyable agent starting prompt plus safety and review guidance."
        )]
        prompt: bool,
    },
}

#[derive(Debug, Parser)]
#[command(
    name = "mdmind",
    version,
    about = "Navigate and edit a map in a focused interactive terminal flow.",
    after_help = "Examples:\n  mdmind\n  mdmind .\n  mdmind roadmap.md\n  mdmind roadmap.md#product/mvp\n  mdmind --preview .\n  mdmind --preview roadmap.md\n  mdmind --as markdown README.md\n  mdmind --autosave TODO.md"
)]
struct TuiPreviewCli {
    #[arg(
        help = "Map, deep link, Markdown file, or Mindspace folder to open. Omit it to choose or create a map interactively."
    )]
    target: Option<String>,
    #[arg(
        long = "as",
        value_enum,
        help = "Force how the target opens: map or markdown."
    )]
    open_as: Option<TuiOpenAs>,
    #[arg(
        long,
        help = "Print a read-only tree preview instead of opening the full TUI."
    )]
    preview: bool,
    #[arg(
        long,
        help = "Start with AUTOSAVE enabled; future edits write to disk immediately."
    )]
    autosave: bool,
    #[arg(
        long = "experimental",
        value_enum,
        value_delimiter = ',',
        help = "Enable named experimental TUI features, such as ai."
    )]
    experimental: Vec<TuiExperimentalArg>,
    #[arg(
        long = "experimental-ai",
        action = ArgAction::SetTrue,
        help = "Enable the experimental in-TUI AI panel, chat, and review surfaces."
    )]
    experimental_ai: bool,
    #[arg(
        long,
        help = "Show how this terminal reports keys to mdmind, including Alt+arrow compatibility."
    )]
    check_keys: bool,
    #[arg(long, help = "Limit preview depth when using --preview.")]
    max_depth: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum TuiOpenAs {
    Map,
    Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum TuiExperimentalArg {
    Ai,
}

impl From<Option<TuiOpenAs>> for OpenTargetMode {
    fn from(value: Option<TuiOpenAs>) -> Self {
        match value {
            Some(TuiOpenAs::Map) => Self::Map,
            Some(TuiOpenAs::Markdown) => Self::Markdown,
            None => Self::Auto,
        }
    }
}

#[derive(Debug)]
struct CliError {
    message: Option<String>,
    exit_code: u8,
    code: &'static str,
    category: &'static str,
}

impl CliError {
    fn runtime(message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
            exit_code: 1,
            code: "runtime_error",
            category: "runtime",
        }
    }

    fn usage(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
            exit_code: 2,
            code,
            category: "usage",
        }
    }

    fn from_app(error: AppError) -> Self {
        let mut cli_error = Self::runtime(error.message().to_string());
        let message = error.message();
        if message.starts_with("Could not read ") {
            cli_error.code = "file_read_failed";
            cli_error.category = "filesystem";
        } else if message.starts_with("The map contains parser errors") {
            cli_error.code = "parser_errors";
            cli_error.category = "parse";
        } else if message.starts_with("No node id or label path matches anchor")
            || message.starts_with("Anchor ")
        {
            cli_error.code = "anchor_resolution_failed";
        }
        cli_error
    }

    fn silent(exit_code: u8) -> Self {
        Self {
            message: None,
            exit_code,
            code: "silent",
            category: "runtime",
        }
    }
}

pub fn run_mdm() -> ExitCode {
    match Cli::try_parse() {
        Ok(cli) => {
            let json_context = cli.json_context();
            finish(dispatch(cli), json_context)
        }
        Err(error) => finish_clap_error(error, raw_args_json_context()),
    }
}

pub fn run_mdmind() -> ExitCode {
    match TuiPreviewCli::try_parse() {
        Ok(cli) => finish(dispatch_tui_preview(cli), None),
        Err(error) => finish_clap_error(error, None),
    }
}

#[derive(Debug, Clone)]
struct JsonContext {
    command: &'static str,
    target: Option<String>,
}

impl Cli {
    fn json_context(&self) -> Option<JsonContext> {
        match &self.command {
            Commands::View { target, json, .. } if *json => Some(JsonContext {
                command: "view",
                target: Some(target.clone()),
            }),
            Commands::Find { target, json, .. } if *json => Some(JsonContext {
                command: "find",
                target: Some(target.clone()),
            }),
            Commands::Tags { target, json, .. } if *json => Some(JsonContext {
                command: "tags",
                target: Some(target.clone()),
            }),
            Commands::Kv { target, json, .. } if *json => Some(JsonContext {
                command: "kv",
                target: Some(target.clone()),
            }),
            Commands::Links { target, json, .. } if *json => Some(JsonContext {
                command: "links",
                target: Some(target.clone()),
            }),
            Commands::Refs { target, json, .. } if *json => Some(JsonContext {
                command: "refs",
                target: Some(target.clone()),
            }),
            Commands::Relations { target, json, .. } if *json => Some(JsonContext {
                command: "relations",
                target: Some(target.clone()),
            }),
            Commands::Validate { target, json, .. } if *json => Some(JsonContext {
                command: "validate",
                target: Some(target.clone()),
            }),
            Commands::Catalog { json } if *json => Some(JsonContext {
                command: "commands",
                target: None,
            }),
            Commands::Ai {
                command: AiCommands::Presets { json },
            } if *json => Some(JsonContext {
                command: "ai presets",
                target: None,
            }),
            Commands::Changelog { version, json, .. } if *json => Some(JsonContext {
                command: "changelog",
                target: version.clone(),
            }),
            Commands::Version { json, .. } if *json => Some(JsonContext {
                command: "version",
                target: None,
            }),
            Commands::Open { target, json, .. } if *json => Some(JsonContext {
                command: "open",
                target: Some(target.clone()),
            }),
            Commands::Mindspace {
                command: MindspaceCommands::Scan { root, json, .. },
            } if *json => Some(JsonContext {
                command: "mindspace scan",
                target: Some(root.to_string_lossy().to_string()),
            }),
            Commands::Mindspace {
                command: MindspaceCommands::Lint { root, json, .. },
            } if *json => Some(JsonContext {
                command: "mindspace lint",
                target: Some(root.to_string_lossy().to_string()),
            }),
            Commands::Mindspace {
                command: MindspaceCommands::Setup { root, json, .. },
            } if *json => Some(JsonContext {
                command: "mindspace setup",
                target: Some(root.to_string_lossy().to_string()),
            }),
            Commands::Mindspace {
                command: MindspaceCommands::Context { target, json, .. },
            } if *json => Some(JsonContext {
                command: "mindspace context",
                target: Some(target.clone()),
            }),
            Commands::Mindspace {
                command:
                    MindspaceCommands::Session {
                        command:
                            MindspaceSessionCommands::Start { target, json, .. }
                            | MindspaceSessionCommands::Plan {
                                session_id: target,
                                json,
                                ..
                            }
                            | MindspaceSessionCommands::Apply {
                                session_id: target,
                                json,
                                ..
                            }
                            | MindspaceSessionCommands::Submit {
                                session_id: target,
                                json,
                                ..
                            }
                            | MindspaceSessionCommands::Close {
                                session_id: target,
                                json,
                                ..
                            },
                    },
            } if *json => Some(JsonContext {
                command: "mindspace session",
                target: Some(target.clone()),
            }),
            Commands::Mindspace {
                command:
                    MindspaceCommands::Review {
                        command:
                            MindspaceReviewCommands::List { json, .. }
                            | MindspaceReviewCommands::Approve { json, .. }
                            | MindspaceReviewCommands::Reject { json, .. },
                    },
            } if *json => Some(JsonContext {
                command: "mindspace review",
                target: None,
            }),
            Commands::Mindspace {
                command:
                    MindspaceCommands::Template {
                        command: MindspaceTemplateCommands::List { json, .. },
                    },
            } if *json => Some(JsonContext {
                command: "mindspace template list",
                target: None,
            }),
            Commands::Mindspace {
                command:
                    MindspaceCommands::Template {
                        command: MindspaceTemplateCommands::Show { id, json, .. },
                    },
            } if *json => Some(JsonContext {
                command: "mindspace template show",
                target: Some(id.clone()),
            }),
            _ => None,
        }
    }
}

fn dispatch(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::View {
            target,
            json,
            max_depth,
        } => render_view_like("view", &target, json, max_depth),
        Commands::ViewMarkdown {
            target,
            pretty,
            plain,
            width,
            no_color,
        } => dispatch_view_markdown(&target, pretty, plain, width, no_color),
        Commands::Find {
            target,
            query,
            json,
            plain,
        } => {
            let loaded = load_document(&target).map_err(CliError::from_app)?;
            ensure_parseable(&loaded).map_err(CliError::from_app)?;
            let matches = find_matches(
                &select_document(&loaded).map_err(CliError::from_app)?,
                &query,
            );
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "find",
                    target: Some(&target),
                    format: "search_matches.v1",
                    summary: Some(count_summary(matches.len())),
                },
                &matches,
                || render_find(&matches),
                || render_find_plain(&matches),
            )
        }
        Commands::Tags {
            target,
            json,
            plain,
        } => {
            let loaded = load_document(&target).map_err(CliError::from_app)?;
            ensure_parseable(&loaded).map_err(CliError::from_app)?;
            let tags = tag_counts(&select_document(&loaded).map_err(CliError::from_app)?);
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "tags",
                    target: Some(&target),
                    format: "tag_counts.v1",
                    summary: Some(count_summary(tags.len())),
                },
                &tags,
                || render_tags(&tags),
                || render_tags_plain(&tags),
            )
        }
        Commands::Kv {
            target,
            json,
            plain,
            keys,
        } => {
            let loaded = load_document(&target).map_err(CliError::from_app)?;
            ensure_parseable(&loaded).map_err(CliError::from_app)?;
            let rows = metadata_rows(
                &select_document(&loaded).map_err(CliError::from_app)?,
                &keys,
            );
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "kv",
                    target: Some(&target),
                    format: "metadata_rows.v1",
                    summary: Some(count_summary(rows.len())),
                },
                &rows,
                || render_metadata(&rows),
                || render_metadata_plain(&rows),
            )
        }
        Commands::Links {
            target,
            json,
            plain,
        } => {
            let loaded = load_document(&target).map_err(CliError::from_app)?;
            ensure_parseable(&loaded).map_err(CliError::from_app)?;
            let links = link_entries(&select_document(&loaded).map_err(CliError::from_app)?);
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "links",
                    target: Some(&target),
                    format: "link_entries.v1",
                    summary: Some(count_summary(links.len())),
                },
                &links,
                || render_links(&links),
                || render_links_plain(&links),
            )
        }
        Commands::Refs {
            target,
            json,
            plain,
        } => {
            let loaded = load_document(&target).map_err(CliError::from_app)?;
            ensure_parseable(&loaded).map_err(CliError::from_app)?;
            let refs = reference_entries(&select_document(&loaded).map_err(CliError::from_app)?);
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "refs",
                    target: Some(&target),
                    format: "reference_rows.v1",
                    summary: Some(count_summary(refs.len())),
                },
                &refs,
                || render_references(&refs),
                || render_references_plain(&refs),
            )
        }
        Commands::Relations {
            target,
            json,
            plain,
        } => {
            let loaded = load_document(&target).map_err(CliError::from_app)?;
            ensure_parseable(&loaded).map_err(CliError::from_app)?;
            let rows = match loaded.target.anchor.as_deref() {
                Some(anchor) => {
                    let path = resolve_anchor_path(&loaded.document, anchor)
                        .map_err(CliError::from_app)?;
                    relation_entries_for_path(&loaded.document, &path)
                }
                None => relation_entries(&select_document(&loaded).map_err(CliError::from_app)?),
            };
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "relations",
                    target: Some(&target),
                    format: "relation_rows.v1",
                    summary: Some(count_summary(rows.len())),
                },
                &rows,
                || render_relations(&rows),
                || render_relations_plain(&rows),
            )
        }
        Commands::Validate {
            target,
            json,
            plain,
        } => {
            let loaded = load_document(&target).map_err(CliError::from_app)?;
            let diagnostics = diagnostics_for_validate(&loaded);
            print_validate_output(json, plain, &target, &diagnostics)?;

            if diagnostics_have_errors(&diagnostics) {
                return Err(CliError::silent(1));
            }

            Ok(())
        }
        Commands::Export {
            target,
            format,
            query,
        } => {
            let loaded = load_document(&target).map_err(CliError::from_app)?;
            let document = select_document(&loaded).map_err(CliError::from_app)?;
            let document = if let Some(query) = query.as_deref() {
                let filtered = filter_document(&document, query)
                    .ok_or_else(|| CliError::runtime("Export query must not be empty."))?;
                if filtered.nodes.is_empty() {
                    return Err(CliError::runtime(format!(
                        "No nodes matched export query '{}'.",
                        query
                    )));
                }
                filtered
            } else {
                document
            };
            println!(
                "{}",
                export_document(&document, &format).map_err(CliError::runtime)?
            );
            Ok(())
        }
        Commands::Init {
            path,
            template,
            force,
        } => {
            let template = TemplateKind::parse(&template).expect("validated by clap");
            create_from_template(&path, template, force).map_err(CliError::from_app)?;
            eprintln!(
                "Created '{}' from the '{}' template.",
                path.display(),
                template_name(template)
            );
            println!("{}", path.display());
            Ok(())
        }
        Commands::Import {
            source,
            from,
            output,
            force,
            preview,
            report,
        } => import_source(
            &source,
            from.as_deref(),
            output.as_deref(),
            force,
            preview,
            report,
        ),
        Commands::Examples { command } => dispatch_examples(command),
        Commands::Ai { command } => dispatch_ai(command),
        Commands::Skills { command } => dispatch_skills(command),
        Commands::Mindspace { command } => dispatch_mindspace(command),
        Commands::Catalog { json } => dispatch_commands(json),
        Commands::Changelog {
            version,
            all,
            pretty,
            plain,
            json,
        } => dispatch_changelog(version.as_deref(), all, pretty, plain, json),
        Commands::Open {
            target,
            preview,
            autosave,
            json,
            max_depth,
        } => {
            if preview || json {
                render_view_like("open", &target, json, max_depth)
            } else {
                run_interactive_with_mode(&target, autosave, OpenTargetMode::Auto)
                    .map_err(CliError::from_app)
            }
        }
        Commands::CheckKeys => run_key_diagnostics().map_err(CliError::from_app),
        Commands::Version { check, json } => dispatch_version(check, json),
    }
}

fn skill_install_command_text() -> String {
    std::iter::once(MDMIND_SKILL_INSTALL_PROGRAM)
        .chain(MDMIND_SKILL_INSTALL_ARGS.iter().copied())
        .collect::<Vec<_>>()
        .join(" ")
}

fn dispatch_skills(command: SkillCommands) -> Result<(), CliError> {
    match command {
        SkillCommands::Install { print } => dispatch_skills_install(print),
    }
}

fn dispatch_skills_install(print: bool) -> Result<(), CliError> {
    let command_text = skill_install_command_text();
    if print {
        println!("{command_text}");
        return Ok(());
    }

    eprintln!("Running: {command_text}");
    let status = ProcessCommand::new(MDMIND_SKILL_INSTALL_PROGRAM)
        .args(MDMIND_SKILL_INSTALL_ARGS)
        .status()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                CliError::runtime(format!(
                    "Could not find `npx`. Install Node.js/npm, then run `{command_text}` manually."
                ))
            } else {
                CliError::runtime(format!("Could not run `{command_text}`: {error}"))
            }
        })?;

    if status.success() {
        eprintln!("Installed mdmind agent skills.");
        return Ok(());
    }

    let exit_summary = status
        .code()
        .map(|code| format!("exit code {code}"))
        .unwrap_or_else(|| "no exit code".to_string());
    Err(CliError {
        message: Some(format!(
            "Skill install command failed with {exit_summary}. You can run it manually: {command_text}"
        )),
        exit_code: status
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .filter(|code| *code != 0)
            .unwrap_or(1),
        code: "skill_install_failed",
        category: "external_command",
    })
}

fn dispatch_mindspace(command: MindspaceCommands) -> Result<(), CliError> {
    match command {
        MindspaceCommands::Scan { root, json, plain } => {
            let target = root.to_string_lossy().to_string();
            let scan = scan_mindspace(&root).map_err(CliError::from_app)?;
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "mindspace scan",
                    target: Some(&target),
                    format: MINDSPACE_SCAN_FORMAT,
                    summary: Some(
                        serde_json::to_value(&scan.summary)
                            .expect("mindspace scan summary should serialize"),
                    ),
                },
                &scan,
                || render_mindspace_scan(&scan),
                || render_mindspace_scan_plain(&scan),
            )
        }
        MindspaceCommands::Lint { root, json, plain } => {
            let target = root.to_string_lossy().to_string();
            let scan = scan_mindspace(&root).map_err(CliError::from_app)?;
            let has_errors = scan.has_error_diagnostics();
            let report = scan.diagnostics_report();
            print_mindspace_lint_output(json, plain, &target, &report, has_errors)?;
            if has_errors {
                return Err(CliError::silent(1));
            }
            Ok(())
        }
        MindspaceCommands::Setup {
            root,
            preview,
            write,
            template,
            json,
            plain,
        } => {
            if preview == write {
                return Err(CliError::usage(
                    "invalid_setup_mode",
                    "Choose exactly one of --preview or --write.",
                ));
            }
            let target = root.to_string_lossy().to_string();
            let mode = if write {
                MindspaceSetupMode::Write
            } else {
                MindspaceSetupMode::Preview
            };
            let report =
                setup_mindspace(&root, mode, template.as_deref()).map_err(CliError::from_app)?;
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "mindspace setup",
                    target: Some(&target),
                    format: MINDSPACE_SETUP_FORMAT,
                    summary: Some(
                        serde_json::to_value(report.summary)
                            .expect("mindspace setup summary should serialize"),
                    ),
                },
                &report,
                || render_mindspace_setup(&report),
                || render_mindspace_setup_plain(&report),
            )
        }
        MindspaceCommands::Context {
            target,
            root,
            query,
            template,
            relation_depth,
            include_backlinks,
            include_source_refs,
            max_files,
            max_branches,
            max_detail_chars,
            max_source_chars,
            json,
            plain,
        } => {
            let root = root.unwrap_or_else(|| PathBuf::from("."));
            let options = MindspaceContextOptions {
                relation_depth,
                include_backlinks,
                include_source_refs,
                max_files,
                max_branches,
                max_detail_chars,
                max_source_chars,
            };
            let bundle = context_mindspace(
                &root,
                &target,
                query.as_deref(),
                template.as_deref(),
                options,
            )
            .map_err(CliError::from_app)?;
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "mindspace context",
                    target: Some(&target),
                    format: MINDSPACE_CONTEXT_FORMAT,
                    summary: Some(
                        serde_json::to_value(bundle.summary)
                            .expect("mindspace context summary should serialize"),
                    ),
                },
                &bundle,
                || render_mindspace_context(&bundle),
                || render_mindspace_context_plain(&bundle),
            )
        }
        MindspaceCommands::Session { command } => dispatch_mindspace_session(command),
        MindspaceCommands::Review { command } => dispatch_mindspace_review(command),
        MindspaceCommands::Template { command } => dispatch_mindspace_template(command),
    }
}

fn dispatch_mindspace_session(command: MindspaceSessionCommands) -> Result<(), CliError> {
    match command {
        MindspaceSessionCommands::Start {
            target,
            role,
            goal,
            root,
            json,
            plain,
        } => {
            let root = root.unwrap_or_else(|| PathBuf::from("."));
            let report = start_mindspace_session(&root, &target, &role, goal.as_deref())
                .map_err(CliError::from_app)?;
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "mindspace session start",
                    target: Some(&target),
                    format: MINDSPACE_SESSION_FORMAT,
                    summary: Some(json!({
                        "session_id": report.session.id,
                        "status": report.session.status,
                        "stale": report.stale
                    })),
                },
                &report,
                || render_mindspace_session_report(&report),
                || render_mindspace_session_report_plain(&report),
            )
        }
        MindspaceSessionCommands::Plan {
            session_id,
            root,
            json,
            plain,
        } => {
            let root = root.unwrap_or_else(|| PathBuf::from("."));
            let report = plan_mindspace_session(&root, &session_id).map_err(CliError::from_app)?;
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "mindspace session plan",
                    target: Some(&session_id),
                    format: MINDSPACE_SESSION_FORMAT,
                    summary: Some(json!({
                        "session_id": report.session.id,
                        "status": report.session.status,
                        "stale": report.stale
                    })),
                },
                &report,
                || render_mindspace_session_report(&report),
                || render_mindspace_session_report_plain(&report),
            )
        }
        MindspaceSessionCommands::Apply {
            session_id,
            preview,
            root,
            json,
            plain,
        } => {
            if !preview {
                return Err(CliError::usage(
                    "invalid_session_apply_mode",
                    "Only `mdm mindspace session apply <session-id> --preview` is supported in this substrate slice.",
                ));
            }
            let root = root.unwrap_or_else(|| PathBuf::from("."));
            let report =
                preview_mindspace_session_apply(&root, &session_id).map_err(CliError::from_app)?;
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "mindspace session apply",
                    target: Some(&session_id),
                    format: MINDSPACE_SESSION_FORMAT,
                    summary: Some(json!({
                        "session_id": report.session.id,
                        "preview": report.preview,
                        "stale": report.stale,
                        "reviews": report.reviews.len()
                    })),
                },
                &report,
                || render_mindspace_session_apply(&report),
                || render_mindspace_session_apply_plain(&report),
            )
        }
        MindspaceSessionCommands::Submit {
            session_id,
            rationale,
            proposal,
            root,
            json,
            plain,
        } => {
            let root = root.unwrap_or_else(|| PathBuf::from("."));
            let review =
                submit_mindspace_session(&root, &session_id, &rationale, proposal.as_deref())
                    .map_err(CliError::from_app)?;
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "mindspace session submit",
                    target: Some(&session_id),
                    format: MINDSPACE_REVIEW_FORMAT,
                    summary: Some(json!({
                        "review_id": review.id,
                        "session_id": review.session_id,
                        "status": review.status,
                        "stale": review.stale
                    })),
                },
                &review,
                || render_mindspace_review(&review),
                || render_mindspace_review_plain(&review),
            )
        }
        MindspaceSessionCommands::Close {
            session_id,
            root,
            json,
            plain,
        } => {
            let root = root.unwrap_or_else(|| PathBuf::from("."));
            let session =
                close_mindspace_session(&root, &session_id).map_err(CliError::from_app)?;
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "mindspace session close",
                    target: Some(&session_id),
                    format: MINDSPACE_SESSION_FORMAT,
                    summary: Some(json!({
                        "session_id": session.id,
                        "status": session.status
                    })),
                },
                &session,
                || serde_json::to_string_pretty(&session).expect("session should serialize"),
                || {
                    format!(
                        "session\t{}\nstatus\t{}\ntarget\t{}",
                        session.id,
                        serde_json::to_value(session.status)
                            .expect("session status should serialize")
                            .as_str()
                            .unwrap_or("closed"),
                        session.target
                    )
                },
            )
        }
    }
}

fn dispatch_mindspace_review(command: MindspaceReviewCommands) -> Result<(), CliError> {
    match command {
        MindspaceReviewCommands::List { root, json, plain } => {
            let root = root.unwrap_or_else(|| PathBuf::from("."));
            let list = list_mindspace_reviews(&root).map_err(CliError::from_app)?;
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "mindspace review list",
                    target: None,
                    format: MINDSPACE_REVIEW_FORMAT,
                    summary: Some(
                        serde_json::to_value(list.summary)
                            .expect("review summary should serialize"),
                    ),
                },
                &list,
                || render_mindspace_review_list(&list),
                || render_mindspace_review_list_plain(&list),
            )
        }
        MindspaceReviewCommands::Approve {
            review_id,
            root,
            json,
            plain,
        } => {
            let root = root.unwrap_or_else(|| PathBuf::from("."));
            let review = approve_mindspace_review(&root, &review_id).map_err(CliError::from_app)?;
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "mindspace review approve",
                    target: Some(&review_id),
                    format: MINDSPACE_REVIEW_FORMAT,
                    summary: Some(json!({
                        "review_id": review.id,
                        "status": review.status,
                        "stale": review.stale
                    })),
                },
                &review,
                || render_mindspace_review(&review),
                || render_mindspace_review_plain(&review),
            )
        }
        MindspaceReviewCommands::Reject {
            review_id,
            reason,
            root,
            json,
            plain,
        } => {
            let root = root.unwrap_or_else(|| PathBuf::from("."));
            let review =
                reject_mindspace_review(&root, &review_id, &reason).map_err(CliError::from_app)?;
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "mindspace review reject",
                    target: Some(&review_id),
                    format: MINDSPACE_REVIEW_FORMAT,
                    summary: Some(json!({
                        "review_id": review.id,
                        "status": review.status,
                        "stale": review.stale
                    })),
                },
                &review,
                || render_mindspace_review(&review),
                || render_mindspace_review_plain(&review),
            )
        }
    }
}

fn dispatch_mindspace_template(command: MindspaceTemplateCommands) -> Result<(), CliError> {
    match command {
        MindspaceTemplateCommands::List { json, plain } => {
            let catalog = mindspace_template_catalog();
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "mindspace template list",
                    target: None,
                    format: MINDSPACE_TEMPLATE_CATALOG_FORMAT,
                    summary: Some(count_summary(catalog.templates.len())),
                },
                &catalog,
                || render_mindspace_template_catalog(&catalog),
                || render_mindspace_template_catalog_plain(&catalog),
            )
        }
        MindspaceTemplateCommands::Show {
            id,
            json,
            plain,
            prompt,
        } => {
            if prompt && (json || plain) {
                return Err(CliError::usage(
                    "invalid_output_mode",
                    "Choose --prompt, --json, or --plain, not more than one.",
                ));
            }
            let template = mindspace_template(&id).ok_or_else(|| {
                CliError::runtime(format!(
                    "Unknown Mindspace template '{}'. Run `mdm mindspace template list`.",
                    id
                ))
            })?;
            if prompt {
                println!("{}", render_mindspace_template_prompt(&template));
                return Ok(());
            }
            print_output(
                OutputSpec {
                    json,
                    plain,
                    command: "mindspace template show",
                    target: Some(&id),
                    format: MINDSPACE_TEMPLATE_FORMAT,
                    summary: Some(json!({
                        "id": template.id,
                        "name": template.name,
                        "persona_fit": template.persona_fit,
                    })),
                },
                &template,
                || render_mindspace_template(&template),
                || render_mindspace_template_plain(&template),
            )
        }
    }
}

fn import_source(
    source_path: &std::path::Path,
    format: Option<&str>,
    output_path: Option<&std::path::Path>,
    force: bool,
    preview: bool,
    report: bool,
) -> Result<(), CliError> {
    let format = match format {
        Some(format) => format.to_string(),
        None => infer_import_format(source_path)?,
    };
    if let Some(error) = unsupported_import_error(&format) {
        return Err(error);
    }
    let import_format = match format.as_str() {
        "web" => "html",
        other => other,
    };
    let resolved_output_path = if preview {
        output_path.map(std::path::Path::to_path_buf)
    } else {
        Some(
            output_path
                .map(std::path::Path::to_path_buf)
                .unwrap_or_else(|| default_import_output_path(source_path)),
        )
    };
    let output_path = resolved_output_path.as_deref();

    if let Some(output_path) = output_path
        && output_path.exists()
        && !force
    {
        return Err(CliError::runtime(format!(
            "Refusing to overwrite '{}'. Use --force to replace it.",
            output_path.display()
        )));
    }

    let source = read_import_source(source_path, &format)?;
    let document = import_document(&source, import_format).map_err(CliError::runtime)?;
    let serialized = serialize_document(&document);

    if report {
        eprintln!(
            "{}",
            render_import_report(&format, source_path, output_path, preview, &document)
        );
    }

    if preview {
        print!("{serialized}");
        return Ok(());
    }

    let Some(output_path) = output_path else {
        return Err(CliError::runtime(
            "Import needs an output path unless --preview is set.",
        ));
    };
    write_imported_map(output_path, &serialized)?;
    eprintln!(
        "Imported '{}' as {} into '{}'.",
        source_path.display(),
        format,
        output_path.display()
    );
    println!("{}", output_path.display());
    Ok(())
}

fn infer_import_format(source_path: &std::path::Path) -> Result<String, CliError> {
    if is_remote_source(source_path) {
        return Ok("web".to_string());
    }
    let extension = source_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("opml") => Ok("opml".to_string()),
        Some("mm") => Ok("freemind".to_string()),
        Some("html" | "htm") => Ok("html".to_string()),
        Some("md" | "markdown" | "mdown" | "mkd") => Ok("markdown".to_string()),
        Some("xmind") => Ok("xmind".to_string()),
        Some("mmap") => Ok("mindmanager".to_string()),
        Some("pdf") => Ok("pdf".to_string()),
        Some(extension) => Err(CliError::runtime(format!(
            "Could not infer import format from extension '.{extension}'. Use --from freemind, --from html, --from markdown, --from mindmanager, --from opml, --from pdf, --from web, or --from xmind."
        ))),
        None => Err(CliError::runtime(
            "Could not infer import format because the source has no extension. Use --from freemind, --from html, --from markdown, --from mindmanager, --from opml, --from pdf, --from web, or --from xmind.",
        )),
    }
}

fn is_remote_source(source_path: &std::path::Path) -> bool {
    let source = source_path.to_string_lossy();
    source.starts_with("http://") || source.starts_with("https://")
}

fn unsupported_import_error(format: &str) -> Option<CliError> {
    match format {
        "freemind" | "html" | "markdown" | "opml" | "web" => None,
        "xmind" => Some(CliError::runtime(
            "XMind `.xmind` import is planned but not implemented yet. `.xmind` files are ZIP archives; export OPML from XMind for now, or track this as the archive-reader follow-up.",
        )),
        "mindmanager" => Some(CliError::runtime(
            "MindManager `.mmap` import is not implemented. Prefer exporting OPML from MindManager for now; first-class `.mmap` support needs real samples and a format decision.",
        )),
        "pdf" => Some(CliError::runtime(
            "PDF ingestion is intentionally agent-authored for now. Have an agent read the PDF, create a mdmind map with the map-authoring guidance, then run `mdm validate` on the result.",
        )),
        _ => Some(CliError::runtime(format!(
            "Unsupported import format '{format}'. Choose one of: freemind, html, markdown, opml."
        ))),
    }
}

fn read_import_source(source_path: &std::path::Path, format: &str) -> Result<String, CliError> {
    if is_remote_source(source_path) {
        eprintln!(
            "warning: web import is rough structural extraction. Agents should usually read messy pages and author a mdmind map directly when meaning matters."
        );
        return fetch_remote_import_source(&source_path.to_string_lossy());
    }

    if format == "web" {
        eprintln!(
            "warning: local --from web uses the HTML structural extractor. Use --from html when you want to be explicit."
        );
    }

    std::fs::read_to_string(source_path).map_err(|error| {
        CliError::runtime(format!(
            "Could not read '{}': {error}",
            source_path.display()
        ))
    })
}

fn fetch_remote_import_source(url: &str) -> Result<String, CliError> {
    let response = reqwest::blocking::get(url)
        .map_err(|error| CliError::runtime(format!("Could not fetch '{url}': {error}")))?;
    let status = response.status();
    if !status.is_success() {
        return Err(CliError::runtime(format!(
            "Could not fetch '{url}': HTTP {status}"
        )));
    }
    response.text().map_err(|error| {
        CliError::runtime(format!("Could not read response from '{url}': {error}"))
    })
}

fn default_import_output_path(source_path: &std::path::Path) -> PathBuf {
    if is_remote_source(source_path) {
        return PathBuf::from(format!("{}-mind.md", remote_import_stem(source_path)));
    }

    let stem = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("imported");
    let file_name = format!("{stem}-mind.md");
    match source_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        Some(parent) => parent.join(file_name),
        None => PathBuf::from(file_name),
    }
}

fn remote_import_stem(source_path: &std::path::Path) -> String {
    let source = source_path.to_string_lossy();
    let without_scheme = source
        .strip_prefix("https://")
        .or_else(|| source.strip_prefix("http://"))
        .unwrap_or(&source);
    let without_query = without_scheme
        .split(['?', '#'])
        .next()
        .unwrap_or(without_scheme);
    let candidate = without_query
        .trim_end_matches('/')
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or("web");
    let sanitized = candidate
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    if sanitized.is_empty() {
        "web".to_string()
    } else {
        sanitized
    }
}

fn write_imported_map(output_path: &std::path::Path, serialized: &str) -> Result<(), CliError> {
    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|error| {
            CliError::runtime(format!(
                "Could not create parent directory '{}': {error}",
                parent.display()
            ))
        })?;
    }

    std::fs::write(output_path, serialized).map_err(|error| {
        CliError::runtime(format!(
            "Could not write imported map to '{}': {error}",
            output_path.display()
        ))
    })
}

#[derive(Debug, Default)]
struct ImportStats {
    nodes: usize,
    roots: usize,
    leaves: usize,
    max_depth: usize,
    detail_lines: usize,
    detail_nodes: usize,
    tags: usize,
    metadata: usize,
    ids: usize,
    duplicate_ids: usize,
    references: usize,
    reference_links: usize,
    reference_images: usize,
    reference_urls: usize,
    reference_local: usize,
    relations: usize,
    tasks: usize,
    task_open: usize,
    task_done: usize,
    validation_errors: usize,
    validation_warnings: usize,
    tag_counts: BTreeMap<String, usize>,
    metadata_key_counts: BTreeMap<String, usize>,
    id_counts: BTreeMap<String, usize>,
}

fn render_import_report(
    format: &str,
    source_path: &std::path::Path,
    output_path: Option<&std::path::Path>,
    preview: bool,
    document: &Document,
) -> String {
    let mut stats = ImportStats {
        roots: document.nodes.len(),
        ..Default::default()
    };
    collect_import_stats(&document.nodes, 1, &mut stats);
    stats.duplicate_ids = stats.id_counts.values().filter(|count| **count > 1).count();
    for diagnostic in validate_document(document) {
        match diagnostic.severity {
            Severity::Error => stats.validation_errors += 1,
            Severity::Warning => stats.validation_warnings += 1,
        }
    }
    let destination = if preview {
        "preview only".to_string()
    } else {
        output_path
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "(missing output)".to_string())
    };

    [
        "Import report".to_string(),
        format!("- source: {}", source_path.display()),
        format!("- format: {format}"),
        format!("- destination: {destination}"),
        format!("- nodes: {}", stats.nodes),
        format!("- roots: {}", stats.roots),
        format!("- leaves: {}", stats.leaves),
        format!("- max_depth: {}", stats.max_depth),
        format!("- detail_lines: {}", stats.detail_lines),
        format!("- detail_nodes: {}", stats.detail_nodes),
        format!("- tags: {}", stats.tags),
        format!("- metadata: {}", stats.metadata),
        format!("- ids: {}", stats.ids),
        format!("- duplicate_ids: {}", stats.duplicate_ids),
        format!("- references: {}", stats.references),
        format!("- reference_links: {}", stats.reference_links),
        format!("- reference_images: {}", stats.reference_images),
        format!("- reference_urls: {}", stats.reference_urls),
        format!("- reference_local: {}", stats.reference_local),
        format!("- relations: {}", stats.relations),
        format!("- tasks: {}", stats.tasks),
        format!("- task_open: {}", stats.task_open),
        format!("- task_done: {}", stats.task_done),
        format!("- validation_errors: {}", stats.validation_errors),
        format!("- validation_warnings: {}", stats.validation_warnings),
        format!(
            "- tag_breakdown: {}",
            render_count_breakdown(&stats.tag_counts)
        ),
        format!(
            "- metadata_keys: {}",
            render_count_breakdown(&stats.metadata_key_counts)
        ),
    ]
    .join("\n")
}

fn collect_import_stats(nodes: &[Node], depth: usize, stats: &mut ImportStats) {
    for node in nodes {
        stats.nodes += 1;
        if node.children.is_empty() {
            stats.leaves += 1;
        }
        stats.max_depth = stats.max_depth.max(depth);
        stats.detail_lines += node.detail.len();
        stats.detail_nodes += usize::from(!node.detail.is_empty());
        stats.tags += node.tags.len();
        for tag in &node.tags {
            *stats.tag_counts.entry(tag.clone()).or_default() += 1;
        }
        stats.metadata += node.metadata.len();
        for entry in &node.metadata {
            *stats
                .metadata_key_counts
                .entry(entry.key.clone())
                .or_default() += 1;
        }
        if let Some(id) = &node.id {
            stats.ids += 1;
            *stats.id_counts.entry(id.clone()).or_default() += 1;
        }
        stats.references += node.references.len();
        for reference in &node.references {
            match reference.kind {
                ExternalRefKind::Link => stats.reference_links += 1,
                ExternalRefKind::Image => stats.reference_images += 1,
            }
            if reference.is_url() {
                stats.reference_urls += 1;
            } else {
                stats.reference_local += 1;
            }
        }
        stats.relations += node.relations.len();
        match node.task {
            Some(TaskState::Open) => {
                stats.tasks += 1;
                stats.task_open += 1;
            }
            Some(TaskState::Done) => {
                stats.tasks += 1;
                stats.task_done += 1;
            }
            None => {}
        }
        collect_import_stats(&node.children, depth + 1, stats);
    }
}

fn render_count_breakdown(counts: &BTreeMap<String, usize>) -> String {
    if counts.is_empty() {
        "none".to_string()
    } else {
        counts
            .iter()
            .map(|(key, count)| format!("{key}={count}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn dispatch_examples(command: ExampleCommands) -> Result<(), CliError> {
    match command {
        ExampleCommands::List => {
            println!("Bundled examples:");
            for asset in bundled_examples() {
                println!(
                    "- {:<28} {:<34} {}",
                    asset.name, asset.file_name, asset.description
                );
            }
            Ok(())
        }
        ExampleCommands::Path => {
            let Some(path) = discover_examples_dir() else {
                return Err(CliError::runtime(
                    "No installed examples directory was found. Use `mdm examples copy all` to materialize local copies.",
                ));
            };
            println!("{}", path.display());
            Ok(())
        }
        ExampleCommands::Copy { name, to, force } => copy_examples(&name, to, force),
    }
}

fn dispatch_ai(command: AiCommands) -> Result<(), CliError> {
    match command {
        AiCommands::Presets { json } => {
            let presets = quick_add_presets();
            if json {
                print_json_envelope(
                    "ai presets",
                    None,
                    "ai_quick_add_presets.v1",
                    Some(count_summary(presets.len())),
                    Some(&presets),
                    None,
                    Vec::new(),
                );
            } else {
                println!("AI quick-add presets:");
                for preset in presets {
                    println!(
                        "- {:<14} {:<20} {}",
                        preset.id, preset.label, preset.summary
                    );
                    if let Some(model) = preset.default_model {
                        println!("  model: {model}");
                    }
                    if let Some(secret_ref) = preset.default_secret_ref {
                        println!("  secret: {secret_ref}");
                    }
                    if let Some(docs_url) = preset.docs_url {
                        println!("  keys: {docs_url}");
                    }
                }
            }
            Ok(())
        }
        AiCommands::QuickAdd {
            preset,
            secret_ref,
            default,
            config,
        } => {
            let config_path = match config {
                Some(path) => path,
                None => ai_profiles_path().map_err(CliError::from_app)?,
            };
            let mut profiles =
                load_ai_profiles_from_path(&config_path).map_err(CliError::from_app)?;
            let profile = quick_add_profile(&preset, secret_ref).map_err(CliError::from_app)?;
            let profile_id = profile.id.clone();
            let profile_label = profile.label.clone();
            let secret_ref = profile.secret_ref.clone();

            profiles.upsert_profile(profile, default);
            save_ai_profiles_to_path(&config_path, &profiles).map_err(CliError::from_app)?;

            eprintln!(
                "Added AI profile '{}' ({}) to '{}'.",
                profile_label,
                profile_id,
                config_path.display()
            );
            if profile_id == "nvidia-nim" {
                eprintln!(
                    "NVIDIA NIM uses {}. Set NVIDIA_API_KEY from https://build.nvidia.com/settings/api-keys before use.",
                    secret_ref.as_deref().unwrap_or("env:NVIDIA_API_KEY")
                );
            }
            println!("{}", config_path.display());
            Ok(())
        }
    }
}

fn dispatch_commands(json: bool) -> Result<(), CliError> {
    let catalog = command_catalog();
    if json {
        print_json_envelope(
            "commands",
            None,
            "command_catalog.v1",
            Some(count_summary(catalog.commands.len())),
            Some(&catalog),
            None,
            Vec::new(),
        );
        return Ok(());
    }

    println!("mdm commands");
    for command in &catalog.commands {
        println!("- {}: {}", command.name, command.summary);
    }
    println!("\nUse `mdm commands --json` for agent-readable safety and output metadata.");
    Ok(())
}

fn dispatch_view_markdown(
    target: &std::path::Path,
    pretty: bool,
    plain: bool,
    width: usize,
    no_color: bool,
) -> Result<(), CliError> {
    if pretty && plain {
        return Err(CliError::usage(
            "invalid_output_mode",
            "Choose either --pretty or --plain, not both.",
        ));
    }

    let source = fs::read_to_string(target).map_err(|error| {
        CliError::runtime(format!("Could not read '{}': {error}", target.display()))
    })?;

    if plain {
        print!("{source}");
        return Ok(());
    }

    let rendered = render_markdown(
        &source,
        markdown_render_options(width, no_color, RenderTarget::CliAnsi),
    );
    println!("{rendered}");
    Ok(())
}

fn dispatch_changelog(
    version: Option<&str>,
    all: bool,
    pretty: bool,
    plain: bool,
    json: bool,
) -> Result<(), CliError> {
    if all && version.is_some() {
        return Err(CliError::usage(
            "invalid_changelog_selection",
            "Choose either --all or --version, not both.",
        ));
    }
    if [json, pretty, plain]
        .into_iter()
        .filter(|enabled| *enabled)
        .count()
        > 1
    {
        return Err(CliError::usage(
            "invalid_output_mode",
            "Choose only one of --json, --pretty, or --plain.",
        ));
    }

    if all {
        let entries = changelog_entries();
        if json {
            print_json_envelope(
                "changelog",
                None,
                "changelog.v1",
                Some(count_summary(entries.len())),
                Some(&entries),
                None,
                Vec::new(),
            );
        } else if plain {
            println!("{}", render_full_changelog());
        } else {
            println!(
                "{}",
                render_markdown(
                    &render_full_changelog(),
                    markdown_render_options(88, false, RenderTarget::CliAnsi)
                )
            );
        }
        return Ok(());
    }

    let entry = match version {
        Some(version) => changelog_entry(version).ok_or_else(|| {
            CliError::runtime(format!(
                "No changelog entry found for version '{version}'. Try `mdm changelog --all`."
            ))
        })?,
        None => default_changelog_entry(APP_VERSION).ok_or_else(|| {
            CliError::runtime("No changelog entries are bundled with this build.")
        })?,
    };

    if json {
        print_json_envelope(
            "changelog",
            version,
            "changelog_entry.v1",
            Some(json!({ "version": entry.version.as_str() })),
            Some(&entry),
            None,
            Vec::new(),
        );
    } else if plain {
        println!("{}", render_changelog_entry(&entry));
    } else {
        println!(
            "{}",
            render_markdown(
                &render_changelog_entry(&entry),
                markdown_render_options(88, false, RenderTarget::CliAnsi)
            )
        );
    }
    Ok(())
}

fn markdown_render_options(width: usize, no_color: bool, target: RenderTarget) -> RenderOptions {
    RenderOptions {
        width,
        color: if no_color {
            ColorMode::Never
        } else {
            ColorMode::Auto
        },
        target,
    }
}

fn dispatch_version(check: bool, json: bool) -> Result<(), CliError> {
    if !check {
        if json {
            let data = VersionInfo {
                current_version: APP_VERSION.to_string(),
            };
            print_json_envelope(
                "version",
                None,
                "version.v1",
                Some(json!({ "version": APP_VERSION })),
                Some(&data),
                None,
                Vec::new(),
            );
        } else {
            println!("mdm {APP_VERSION}");
        }
        return Ok(());
    }

    let check = check_for_updates(APP_VERSION).map_err(|error| CliError {
        message: Some(error.message().to_string()),
        exit_code: 1,
        code: "update_check_failed",
        category: "network",
    })?;

    if json {
        print_json_envelope(
            "version",
            None,
            "update_check.v1",
            Some(json!({
                "current_version": check.current_version.as_str(),
                "latest_version": check.latest_version.as_str(),
                "update_available": check.update_available,
            })),
            Some(&check),
            None,
            Vec::new(),
        );
    } else {
        println!("{}", render_update_check(&check));
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct VersionInfo {
    current_version: String,
}

fn render_update_check(check: &UpdateCheck) -> String {
    if check.update_available {
        format!(
            "mdm {}\nNew version {} available: {}",
            check.current_version, check.latest_version, check.release_url
        )
    } else {
        format!(
            "mdm {}\nYou are on the latest release.",
            check.current_version
        )
    }
}

#[derive(Debug, Serialize)]
struct CommandCatalog {
    version: &'static str,
    commands: Vec<CommandInfo>,
}

#[derive(Debug, Serialize)]
struct CommandInfo {
    name: &'static str,
    summary: &'static str,
    reads: Vec<&'static str>,
    writes: Vec<&'static str>,
    network: bool,
    interactive: bool,
    output_modes: Vec<&'static str>,
    args: Vec<CommandArgInfo>,
    flags: Vec<CommandFlagInfo>,
    formats: Vec<&'static str>,
    examples: Vec<&'static str>,
    docs: Vec<&'static str>,
    skills: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
struct CommandArgInfo {
    name: &'static str,
    required: bool,
}

#[derive(Debug, Clone, Serialize)]
struct CommandFlagInfo {
    name: &'static str,
    takes_value: bool,
    values: Vec<&'static str>,
}

macro_rules! command_info {
    (
        $name:expr,
        $summary:expr,
        $reads:expr,
        $writes:expr,
        $network:expr,
        $interactive:expr,
        $output_modes:expr,
        $args:expr,
        $flags:expr,
        $formats:expr,
        $examples:expr $(,)?
    ) => {
        CommandInfo {
            name: $name,
            summary: $summary,
            reads: $reads.to_vec(),
            writes: $writes.to_vec(),
            network: $network,
            interactive: $interactive,
            output_modes: $output_modes.to_vec(),
            args: $args.to_vec(),
            flags: $flags.to_vec(),
            formats: $formats.to_vec(),
            examples: $examples.to_vec(),
            docs: vec!["docs/agents/AGENT_CLI_CONTRACT.md"],
            skills: vec!["plugins/mdmind/skills/mdm-cli-inspection"],
        }
    };
}

fn command_catalog() -> CommandCatalog {
    CommandCatalog {
        version: APP_VERSION,
        commands: vec![
            command_info!(
                "view",
                "Render a read-only tree view for a map or deep link.",
                &["map"],
                &[],
                false,
                false,
                &["pretty", "json"],
                &[arg("target", true)],
                &[flag("--json"), flag_value("--max-depth", &["usize"])],
                &[],
                &["mdm view ideas.md", "mdm view ideas.md#product/mvp --json"],
            ),
            command_info!(
                "view-markdown",
                "View an ordinary Markdown file in the terminal.",
                &["markdown"],
                &[],
                false,
                false,
                &["pretty", "plain"],
                &[arg("target", true)],
                &[
                    flag("--pretty"),
                    flag("--plain"),
                    flag_value("--width", &["usize"]),
                    flag("--no-color"),
                ],
                &[],
                &[
                    "mdm view-markdown README.md",
                    "mdm view-markdown README.md --plain",
                ],
            ),
            command_info!(
                "find",
                "Search labels, tags, metadata, ids, details, and task state.",
                &["map"],
                &[],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("target", true), arg("query", true)],
                &[flag("--json"), flag("--plain")],
                &[],
                &[
                    "mdm find TODO.md \"task:open\" --plain",
                    "mdm find ideas.md \"#todo @status:active\" --json",
                ],
            ),
            command_info!(
                "tags",
                "List tag counts across a map or deep link.",
                &["map"],
                &[],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("target", true)],
                &[flag("--json"), flag("--plain")],
                &[],
                &["mdm tags ideas.md --plain"],
            ),
            command_info!(
                "kv",
                "List inline key:value metadata rows.",
                &["map"],
                &[],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("target", true)],
                &[
                    flag("--json"),
                    flag("--plain"),
                    flag_value("--keys", &["key,..."]),
                ],
                &[],
                &["mdm kv ideas.md --keys status,owner --plain"],
            ),
            command_info!(
                "links",
                "List every node id and its deep-linkable path.",
                &["map"],
                &[],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("target", true)],
                &[flag("--json"), flag("--plain")],
                &[],
                &["mdm links ideas.md --plain"],
            ),
            command_info!(
                "refs",
                "List external Markdown links, files, URLs, and image references.",
                &["map"],
                &[],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("target", true)],
                &[flag("--json"), flag("--plain")],
                &[],
                &["mdm refs ideas.md --json"],
            ),
            command_info!(
                "relations",
                "List outgoing relations, or incoming backlinks for a deep-linked node.",
                &["map"],
                &[],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("target", true)],
                &[flag("--json"), flag("--plain")],
                &[],
                &[
                    "mdm relations ideas.md --plain",
                    "mdm relations ideas.md#product/mvp --json",
                ],
            ),
            command_info!(
                "validate",
                "Validate parser structure, ids, references, relations, and metadata conventions.",
                &["map"],
                &[],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("target", true)],
                &[flag("--json"), flag("--plain")],
                &[],
                &["mdm validate ideas.md", "mdm validate ideas.md --json"],
            ),
            command_info!(
                "export",
                "Export normalized map data, diagrams, or outliner interchange formats.",
                &["map"],
                &[],
                false,
                false,
                &["json", "mermaid", "opml"],
                &[arg("target", true)],
                &[
                    flag_value("--format", &["json", "mermaid", "opml"]),
                    flag_value("--query", &["query"]),
                ],
                &["json", "mermaid", "opml"],
                &[
                    "mdm export ideas.md --format json",
                    "mdm export ideas.md --query \"#todo @status:active\" --format json",
                ],
            ),
            command_info!(
                "init",
                "Create a new map from a starter template.",
                &["template"],
                &["map"],
                false,
                false,
                &["path"],
                &[arg("path", true)],
                &[
                    flag_value("--template", TemplateKind::names()),
                    flag("--force"),
                ],
                &[],
                &["mdm init TODO.md --template todo"],
            ),
            command_info!(
                "import",
                "Import an external outline or rough structural source into a native mdmind map.",
                &["source"],
                &["map"],
                true,
                false,
                &["path", "preview"],
                &[arg("source", true)],
                &[
                    flag_value(
                        "--from",
                        &[
                            "freemind",
                            "html",
                            "markdown",
                            "mindmanager",
                            "opml",
                            "pdf",
                            "web",
                            "xmind",
                        ],
                    ),
                    flag_value("--output", &["path"]),
                    flag("--force"),
                    flag("--preview"),
                    flag("--report"),
                ],
                &["freemind", "html", "markdown", "opml"],
                &[
                    "mdm import notes.opml",
                    "mdm import outline.md --from markdown --preview",
                ],
            ),
            command_info!(
                "examples",
                "List, locate, or copy the bundled example maps.",
                &["bundled_examples"],
                &[],
                false,
                false,
                &["pretty"],
                &[],
                &[],
                &[],
                &["mdm examples list", "mdm examples copy all"],
            ),
            command_info!(
                "examples list",
                "List the bundled example maps.",
                &["bundled_examples"],
                &[],
                false,
                false,
                &["pretty"],
                &[],
                &[],
                &[],
                &["mdm examples list"],
            ),
            command_info!(
                "examples path",
                "Print the installed on-disk examples directory, when available.",
                &["installed_examples"],
                &[],
                false,
                false,
                &["path"],
                &[],
                &[],
                &[],
                &["mdm examples path"],
            ),
            command_info!(
                "examples copy",
                "Copy one bundled example, or all examples, into a directory.",
                &["bundled_examples"],
                &["files"],
                false,
                false,
                &["path"],
                &[arg("name", true)],
                &[flag_value("--to", &["path"]), flag("--force")],
                &[],
                &[
                    "mdm examples copy all",
                    "mdm examples copy demo --to scratch",
                ],
            ),
            command_info!(
                "ai",
                "Manage experimental AI provider profiles and quick-add presets.",
                &["ai_config"],
                &["ai_config"],
                false,
                false,
                &["pretty"],
                &[],
                &[],
                &[],
                &["mdm ai presets", "mdm ai quick-add nvidia-nim"],
            ),
            command_info!(
                "ai presets",
                "List built-in AI quick-add profile presets.",
                &["bundled_ai_presets"],
                &[],
                false,
                false,
                &["pretty", "json"],
                &[],
                &[flag("--json")],
                &["ai_quick_add_presets.v1"],
                &["mdm ai presets --json"],
            ),
            command_info!(
                "ai quick-add",
                "Add a built-in AI profile preset to the local mdmind config.",
                &["ai_config", "bundled_ai_presets"],
                &["ai_config"],
                false,
                false,
                &["path"],
                &[arg("preset", true)],
                &[
                    flag_value("--secret-ref", &["secret-ref"]),
                    flag("--default"),
                    flag_value("--config", &["path"]),
                ],
                &[],
                &[
                    "mdm ai quick-add nvidia-nim --secret-ref env:NVIDIA_API_KEY --default",
                    "mdm ai quick-add codex-local --default",
                ],
            ),
            command_info!(
                "skills",
                "Install or print agent skill setup commands.",
                &["bundled_skill_reference"],
                &["agent_skills"],
                true,
                false,
                &["pretty"],
                &[],
                &[],
                &[],
                &["mdm skills install", "mdm skills install --print"],
            ),
            command_info!(
                "skills install",
                "Install the bundled mdmind agent skills through the skills CLI.",
                &["bundled_skill_reference"],
                &["agent_skills"],
                true,
                false,
                &["pretty"],
                &[],
                &[flag("--print")],
                &[],
                &["mdm skills install", "mdm skills install --print"],
            ),
            command_info!(
                "mindspace",
                "Inspect, set up, and export optional folder-level Mindspace workspaces.",
                &[
                    "folder",
                    "mindspace_manifest",
                    "maps",
                    "markdown",
                    "built_in_mindspace_templates"
                ],
                &["mindspace_manifest"],
                false,
                false,
                &["pretty", "plain", "json"],
                &[],
                &[],
                &[
                    "mindspace_scan.v1",
                    "mindspace_diagnostics.v1",
                    "mindspace_setup.v1",
                    "mindspace_context.v1",
                    "mindspace_session.v1",
                    "mindspace_review.v1",
                    "mindspace_template_catalog.v1",
                    "mindspace_template.v1",
                ],
                &[
                    "mdm mindspace scan .",
                    "mdm mindspace setup . --preview",
                    "mdm mindspace context maps/roadmap.md#roadmap/current",
                    "mdm mindspace session start maps/tasks.md#todo/focus --role implementer",
                    "mdm mindspace review list",
                    "mdm mindspace lint .",
                    "mdm mindspace template list",
                ],
            ),
            command_info!(
                "mindspace scan",
                "Inspect a folder read-only, with or without a Mindspace manifest.",
                &["folder", "mindspace_manifest", "maps", "markdown"],
                &[],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("root", true)],
                &[flag("--json"), flag("--plain")],
                &["mindspace_scan.v1"],
                &["mdm mindspace scan .", "mdm mindspace scan . --json",],
            ),
            command_info!(
                "mindspace lint",
                "Report deterministic Mindspace diagnostics without AI judgment.",
                &["folder", "mindspace_manifest", "maps", "markdown"],
                &[],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("root", true)],
                &[flag("--json"), flag("--plain")],
                &["mindspace_diagnostics.v1"],
                &["mdm mindspace lint .", "mdm mindspace lint . --json",],
            ),
            command_info!(
                "mindspace setup",
                "Preview or write a minimal Mindspace manifest.",
                &["folder", "mindspace_manifest", "maps", "markdown"],
                &["mindspace_manifest"],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("root", true)],
                &[
                    flag("--preview"),
                    flag("--write"),
                    flag_value("--template", &["template-id"]),
                    flag("--json"),
                    flag("--plain"),
                ],
                &["mindspace_setup.v1"],
                &[
                    "mdm mindspace setup . --preview",
                    "mdm mindspace setup . --write",
                    "mdm mindspace setup . --template launch-planning --preview --json",
                ],
            ),
            command_info!(
                "mindspace context",
                "Export a bounded Mindspace context bundle with provenance.",
                &[
                    "folder",
                    "mindspace_manifest",
                    "maps",
                    "markdown",
                    "sources"
                ],
                &[],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("target", true)],
                &[
                    flag_value("--root", &["path"]),
                    flag_value("--query", &["query"]),
                    flag_value("--template", &["template-id"]),
                    flag_value("--relation-depth", &["count"]),
                    flag("--include-backlinks"),
                    flag("--include-source-refs"),
                    flag_value("--max-files", &["count"]),
                    flag_value("--max-branches", &["count"]),
                    flag_value("--max-detail-chars", &["count"]),
                    flag_value("--max-source-chars", &["count"]),
                    flag("--json"),
                    flag("--plain"),
                ],
                &["mindspace_context.v1"],
                &[
                    "mdm mindspace context maps/roadmap.md#roadmap/current --json",
                    "mdm mindspace context . --query '@owner:jason' --max-branches 8 --json",
                    "mdm mindspace context maps/roadmap.md#roadmap/current --include-source-refs",
                ],
            ),
            command_info!(
                "mindspace session",
                "Manage durable Mindspace agent session records.",
                &["folder", "maps", "mindspace_sessions", "mindspace_reviews"],
                &["mindspace_sessions", "mindspace_reviews"],
                false,
                false,
                &["pretty", "plain", "json"],
                &[],
                &[],
                &["mindspace_session.v1", "mindspace_review.v1"],
                &[
                    "mdm mindspace session start maps/tasks.md#todo/focus --role implementer",
                    "mdm mindspace session apply <session-id> --preview",
                ],
            ),
            command_info!(
                "mindspace session start",
                "Start a durable agent session record for a target.",
                &["folder", "maps"],
                &["mindspace_sessions"],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("target", true)],
                &[
                    flag_value("--role", &["role"]),
                    flag_value("--goal", &["goal"]),
                    flag_value("--root", &["path"]),
                    flag("--json"),
                    flag("--plain"),
                ],
                &["mindspace_session.v1"],
                &["mdm mindspace session start maps/tasks.md#todo/focus --role implementer --json",],
            ),
            command_info!(
                "mindspace session plan",
                "Show a session plan and current target digest.",
                &["mindspace_sessions", "maps"],
                &[],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("session-id", true)],
                &[
                    flag_value("--root", &["path"]),
                    flag("--json"),
                    flag("--plain")
                ],
                &["mindspace_session.v1"],
                &["mdm mindspace session plan <session-id> --json"],
            ),
            command_info!(
                "mindspace session apply",
                "Preview session review state before future writeback.",
                &["mindspace_sessions", "mindspace_reviews", "maps"],
                &[],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("session-id", true)],
                &[
                    flag("--preview"),
                    flag_value("--root", &["path"]),
                    flag("--json"),
                    flag("--plain"),
                ],
                &["mindspace_session.v1"],
                &["mdm mindspace session apply <session-id> --preview --json"],
            ),
            command_info!(
                "mindspace session submit",
                "Submit a session result as a durable review item.",
                &["mindspace_sessions", "maps"],
                &["mindspace_sessions", "mindspace_reviews"],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("session-id", true)],
                &[
                    flag_value("--rationale", &["text"]),
                    flag_value("--proposal", &["text"]),
                    flag_value("--root", &["path"]),
                    flag("--json"),
                    flag("--plain"),
                ],
                &["mindspace_review.v1"],
                &[
                    "mdm mindspace session submit <session-id> --rationale \"ready for review\" --json"
                ],
            ),
            command_info!(
                "mindspace session close",
                "Close a durable agent session record.",
                &["mindspace_sessions"],
                &["mindspace_sessions"],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("session-id", true)],
                &[
                    flag_value("--root", &["path"]),
                    flag("--json"),
                    flag("--plain")
                ],
                &["mindspace_session.v1"],
                &["mdm mindspace session close <session-id> --json"],
            ),
            command_info!(
                "mindspace review",
                "List and decide durable Mindspace review records.",
                &["mindspace_reviews", "maps"],
                &["mindspace_reviews"],
                false,
                false,
                &["pretty", "plain", "json"],
                &[],
                &[],
                &["mindspace_review.v1"],
                &["mdm mindspace review list --json"],
            ),
            command_info!(
                "mindspace review list",
                "List durable review items.",
                &["mindspace_reviews"],
                &[],
                false,
                false,
                &["pretty", "plain", "json"],
                &[],
                &[
                    flag_value("--root", &["path"]),
                    flag("--json"),
                    flag("--plain")
                ],
                &["mindspace_review.v1"],
                &["mdm mindspace review list --json"],
            ),
            command_info!(
                "mindspace review approve",
                "Approve a review item after digest checks.",
                &["mindspace_reviews", "maps"],
                &["mindspace_reviews"],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("review-id", true)],
                &[
                    flag_value("--root", &["path"]),
                    flag("--json"),
                    flag("--plain")
                ],
                &["mindspace_review.v1"],
                &["mdm mindspace review approve <review-id> --json"],
            ),
            command_info!(
                "mindspace review reject",
                "Reject a review item with a reason.",
                &["mindspace_reviews", "maps"],
                &["mindspace_reviews"],
                false,
                false,
                &["pretty", "plain", "json"],
                &[arg("review-id", true)],
                &[
                    flag_value("--reason", &["text"]),
                    flag_value("--root", &["path"]),
                    flag("--json"),
                    flag("--plain"),
                ],
                &["mindspace_review.v1"],
                &[
                    "mdm mindspace review reject <review-id> --reason \"wrong target branch\" --json"
                ],
            ),
            command_info!(
                "mindspace template",
                "List and inspect built-in Mindspace job templates.",
                &["built_in_mindspace_templates"],
                &[],
                false,
                false,
                &["pretty", "plain", "json"],
                &[],
                &[],
                &["mindspace_template_catalog.v1", "mindspace_template.v1"],
                &[
                    "mdm mindspace template list",
                    "mdm mindspace template show launch-planning",
                ],
            ),
            command_info!(
                "mindspace template list",
                "List built-in Mindspace job templates.",
                &["built_in_mindspace_templates"],
                &[],
                false,
                false,
                &["pretty", "plain", "json"],
                &[],
                &[flag("--json"), flag("--plain")],
                &["mindspace_template_catalog.v1"],
                &[
                    "mdm mindspace template list",
                    "mdm mindspace template list --json",
                ],
            ),
            command_info!(
                "mindspace template show",
                "Show one built-in Mindspace job template.",
                &["built_in_mindspace_templates"],
                &[],
                false,
                false,
                &["pretty", "plain", "json", "prompt"],
                &[arg("id", true)],
                &[flag("--json"), flag("--plain"), flag("--prompt")],
                &["mindspace_template.v1"],
                &[
                    "mdm mindspace template show launch-planning",
                    "mdm mindspace template show claims-evidence --prompt",
                    "mdm mindspace template show project-memory --json",
                ],
            ),
            command_info!(
                "commands",
                "Print the mdm command catalog for agents and scripts.",
                &[],
                &[],
                false,
                false,
                &["pretty", "json"],
                &[],
                &[flag("--json")],
                &["command_catalog.v1"],
                &["mdm commands --json"],
            ),
            command_info!(
                "changelog",
                "Read curated mdmind changelog entries.",
                &["bundled_changelog"],
                &[],
                false,
                false,
                &["pretty", "plain", "json"],
                &[],
                &[
                    flag_value("--version", &["version"]),
                    flag("--all"),
                    flag("--pretty"),
                    flag("--plain"),
                    flag("--json"),
                ],
                &["changelog_entry.v1", "changelog.v1"],
                &[
                    "mdm changelog",
                    "mdm changelog --version 0.8.0",
                    "mdm changelog --plain",
                    "mdm changelog --json",
                ],
            ),
            command_info!(
                "open",
                "Open a map or deep link in the interactive navigator.",
                &["map"],
                &["session_sidecars"],
                false,
                true,
                &["interactive", "preview", "json"],
                &[arg("target", true)],
                &[
                    flag("--preview"),
                    flag("--autosave"),
                    flag("--json"),
                    flag_value("--max-depth", &["usize"]),
                ],
                &[],
                &["mdm open ideas.md", "mdm open ideas.md --json"],
            ),
            command_info!(
                "check-keys",
                "Check how this terminal reports Alt+arrow keys.",
                &["terminal_input"],
                &[],
                false,
                true,
                &["interactive"],
                &[],
                &[],
                &[],
                &["mdm check-keys"],
            ),
            command_info!(
                "version",
                "Print the mdm version, or check GitHub Releases for newer builds.",
                &[],
                &[],
                true,
                false,
                &["plain", "json"],
                &[],
                &[flag("--check"), flag("--json")],
                &["version.v1", "update_check.v1"],
                &["mdm version", "mdm version --check --json"],
            ),
        ],
    }
}

fn arg(name: &'static str, required: bool) -> CommandArgInfo {
    CommandArgInfo { name, required }
}

fn flag(name: &'static str) -> CommandFlagInfo {
    CommandFlagInfo {
        name,
        takes_value: false,
        values: Vec::new(),
    }
}

fn flag_value(name: &'static str, values: &[&'static str]) -> CommandFlagInfo {
    CommandFlagInfo {
        name,
        takes_value: true,
        values: values.to_vec(),
    }
}

fn copy_examples(name: &str, to: Option<PathBuf>, force: bool) -> Result<(), CliError> {
    if name.eq_ignore_ascii_case("all") {
        let destination = to.unwrap_or_else(|| PathBuf::from("mdmind-examples"));
        std::fs::create_dir_all(&destination).map_err(|error| {
            CliError::runtime(format!(
                "Could not create examples directory '{}': {error}",
                destination.display()
            ))
        })?;

        let readme_path = destination.join("README.md");
        write_asset_file(&readme_path, examples_readme_contents(), force)?;

        for asset in bundled_examples() {
            let path = destination.join(asset.file_name);
            write_asset_file(&path, asset.contents, force)?;
        }

        println!("{}", destination.display());
        return Ok(());
    }

    let asset = find_example(name).ok_or_else(|| {
        let available = bundled_examples()
            .iter()
            .map(|asset| asset.name)
            .collect::<Vec<_>>()
            .join(", ");
        CliError::runtime(format!(
            "Unknown example '{name}'. Try one of: {available}, or use `all`."
        ))
    })?;

    let destination_dir = to.unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&destination_dir).map_err(|error| {
        CliError::runtime(format!(
            "Could not create destination directory '{}': {error}",
            destination_dir.display()
        ))
    })?;
    let destination = destination_dir.join(asset.file_name);
    write_asset_file(&destination, asset.contents, force)?;
    println!("{}", destination.display());
    Ok(())
}

fn write_asset_file(path: &std::path::Path, contents: &str, force: bool) -> Result<(), CliError> {
    if path.exists() && !force {
        return Err(CliError::runtime(format!(
            "Refusing to overwrite '{}'. Use --force to replace it.",
            path.display()
        )));
    }

    std::fs::write(path, contents).map_err(|error| {
        CliError::runtime(format!("Could not write '{}': {error}", path.display()))
    })?;
    Ok(())
}

fn dispatch_tui_preview(cli: TuiPreviewCli) -> Result<(), CliError> {
    if cli.check_keys {
        return run_key_diagnostics().map_err(CliError::from_app);
    }

    let feature_flags = tui_feature_flags(&cli);
    let target = match cli.target {
        Some(target) => target,
        None => {
            if cli.preview {
                return Err(CliError::runtime("`mdmind --preview` needs a target path."));
            }
            let Some(target) = choose_startup_target().map_err(CliError::from_app)? else {
                return Err(CliError::silent(0));
            };
            target
        }
    };

    let open_mode = OpenTargetMode::from(cli.open_as);
    if open_mode == OpenTargetMode::Auto
        && let Some(root) = directory_target_path(&target)
    {
        if cli.preview {
            render_mdmind_workspace_preview(&root)
        } else {
            let Some(chosen_target) = choose_mindspace_target(&root).map_err(CliError::from_app)?
            else {
                return Err(CliError::silent(0));
            };
            run_interactive_with_mode_and_features(
                &chosen_target,
                cli.autosave,
                OpenTargetMode::Auto,
                feature_flags,
            )
            .map_err(CliError::from_app)
        }
    } else if cli.preview {
        render_mdmind_preview(&target, open_mode, cli.max_depth)
    } else {
        run_interactive_with_mode_and_features(&target, cli.autosave, open_mode, feature_flags)
            .map_err(CliError::from_app)
    }
}

fn tui_feature_flags(cli: &TuiPreviewCli) -> TuiFeatureFlags {
    let mut flags = TuiFeatureFlags::new();
    if cli.experimental_ai {
        flags.enable(TuiExperiment::Ai);
    }
    for experiment in &cli.experimental {
        match experiment {
            TuiExperimentalArg::Ai => flags.enable(TuiExperiment::Ai),
        }
    }
    flags
}

fn render_mdmind_preview(
    target: &str,
    open_mode: OpenTargetMode,
    max_depth: Option<usize>,
) -> Result<(), CliError> {
    match classify_open_target(target, open_mode).map_err(CliError::from_app)? {
        ClassifiedTarget::NativeMap(loaded) => {
            ensure_parseable(&loaded).map_err(CliError::from_app)?;
            let document = select_document(&loaded).map_err(CliError::from_app)?;
            println!("{}", render_tree(&document, max_depth));
            Ok(())
        }
        ClassifiedTarget::OrdinaryMarkdown { source, .. } => {
            println!(
                "{}",
                render_markdown(
                    &source,
                    markdown_render_options(88, false, RenderTarget::CliAnsi)
                )
            );
            Ok(())
        }
        ClassifiedTarget::NearMissMap {
            target,
            diagnostics,
            score,
        } => Err(CliError::runtime(near_miss_guidance(
            &target,
            &diagnostics,
            score,
        ))),
    }
}

fn render_mdmind_workspace_preview(root: &Path) -> Result<(), CliError> {
    let landing = workspace_mindspace(root).map_err(CliError::from_app)?;
    println!("{}", render_mindspace_workspace_landing(&landing));
    Ok(())
}

fn directory_target_path(target: &str) -> Option<PathBuf> {
    let path = target
        .split_once('#')
        .map(|(path, _)| path)
        .unwrap_or(target);
    let path = PathBuf::from(path);
    path.is_dir().then_some(path)
}

fn render_view_like(
    command: &'static str,
    target: &str,
    json: bool,
    max_depth: Option<usize>,
) -> Result<(), CliError> {
    let loaded = if command == "view" && !json {
        match classify_open_target(target, OpenTargetMode::Auto).map_err(CliError::from_app)? {
            ClassifiedTarget::NativeMap(loaded) => loaded,
            ClassifiedTarget::OrdinaryMarkdown { target, .. } => {
                return Err(CliError::runtime(format!(
                    "{} is ordinary Markdown, not a native mdmind map.\n\nTo read it:\n  mdm view-markdown {}\n\nTo convert an outline:\n  mdm import {} --from markdown --preview",
                    target.path.display(),
                    target.path.display(),
                    target.path.display()
                )));
            }
            ClassifiedTarget::NearMissMap {
                target,
                diagnostics,
                score,
            } => {
                return Err(CliError::runtime(near_miss_guidance(
                    &target,
                    &diagnostics,
                    score,
                )));
            }
        }
    } else {
        load_document(target).map_err(CliError::from_app)?
    };
    let document = select_document(&loaded).map_err(CliError::from_app)?;
    if json {
        let exported = document.export();
        print_json_envelope(
            command,
            Some(target),
            "export_document.v1",
            Some(count_summary(exported.nodes.len())),
            Some(&exported),
            None,
            Vec::new(),
        );
    } else {
        println!("{}", render_tree(&document, max_depth));
    }
    Ok(())
}

struct OutputSpec<'a> {
    json: bool,
    plain: bool,
    command: &'static str,
    target: Option<&'a str>,
    format: &'static str,
    summary: Option<serde_json::Value>,
}

fn print_output<T: serde::Serialize>(
    spec: OutputSpec<'_>,
    value: &T,
    pretty: impl FnOnce() -> String,
    plain_renderer: impl FnOnce() -> String,
) -> Result<(), CliError> {
    if spec.json && spec.plain {
        return Err(CliError::usage(
            "invalid_output_mode",
            "Choose either --json or --plain, not both.",
        ));
    }

    if spec.json {
        print_json_envelope(
            spec.command,
            spec.target,
            spec.format,
            spec.summary,
            Some(value),
            None,
            Vec::new(),
        );
    } else if spec.plain {
        println!("{}", plain_renderer());
    } else {
        println!("{}", pretty());
    }
    Ok(())
}

fn print_validate_output(
    json: bool,
    plain: bool,
    target: &str,
    diagnostics: &[crate::model::Diagnostic],
) -> Result<(), CliError> {
    if json && plain {
        return Err(CliError::usage(
            "invalid_output_mode",
            "Choose either --json or --plain, not both.",
        ));
    }

    if json {
        let has_errors = diagnostics_have_errors(diagnostics);
        let error = has_errors.then(|| JsonError {
            code: "validation_failed",
            category: "validation",
            message: "Map validation reported one or more errors.".to_string(),
            path: Some(target.to_string()),
            line: None,
            details: None,
        });
        let next_actions = if has_errors {
            vec![JsonNextAction {
                label: "Review validation diagnostics as plain text".to_string(),
                command: vec![
                    "mdm".to_string(),
                    "validate".to_string(),
                    target.to_string(),
                    "--plain".to_string(),
                ],
                writes: false,
            }]
        } else {
            Vec::new()
        };
        print_json_envelope(
            "validate",
            Some(target),
            "diagnostics.v1",
            Some(diagnostics_summary(diagnostics)),
            Some(diagnostics),
            error,
            next_actions,
        );
    } else if plain {
        println!("{}", render_validate_plain(diagnostics));
    } else {
        println!("{}", render_validate(diagnostics));
    }
    Ok(())
}

fn print_mindspace_lint_output(
    json: bool,
    plain: bool,
    target: &str,
    report: &MindspaceDiagnosticsReport,
    has_errors: bool,
) -> Result<(), CliError> {
    if json && plain {
        return Err(CliError::usage(
            "invalid_output_mode",
            "Choose either --json or --plain, not both.",
        ));
    }

    if json {
        let error = has_errors.then(|| JsonError {
            code: "mindspace_lint_failed",
            category: "validation",
            message: "Mindspace lint reported one or more errors.".to_string(),
            path: Some(target.to_string()),
            line: None,
            details: None,
        });
        let next_actions = if has_errors {
            vec![JsonNextAction {
                label: "Review the read-only mindspace scan".to_string(),
                command: vec![
                    "mdm".to_string(),
                    "mindspace".to_string(),
                    "scan".to_string(),
                    target.to_string(),
                    "--plain".to_string(),
                ],
                writes: false,
            }]
        } else {
            Vec::new()
        };
        print_json_envelope(
            "mindspace lint",
            Some(target),
            MINDSPACE_DIAGNOSTICS_FORMAT,
            Some(
                serde_json::to_value(report.summary)
                    .expect("mindspace lint summary should serialize"),
            ),
            Some(report),
            error,
            next_actions,
        );
    } else if plain {
        println!("{}", render_mindspace_diagnostics_plain(report));
    } else {
        println!("{}", render_mindspace_diagnostics(report));
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct JsonEnvelope<'a, T: Serialize + ?Sized> {
    ok: bool,
    command: &'static str,
    format: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<&'a T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonError>,
    next_actions: Vec<JsonNextAction>,
}

#[derive(Debug, Serialize)]
struct JsonError {
    code: &'static str,
    category: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct JsonNextAction {
    label: String,
    command: Vec<String>,
    writes: bool,
}

fn print_json_envelope<T: Serialize + ?Sized>(
    command: &'static str,
    target: Option<&str>,
    format: &'static str,
    summary: Option<serde_json::Value>,
    data: Option<&T>,
    error: Option<JsonError>,
    next_actions: Vec<JsonNextAction>,
) {
    let envelope = JsonEnvelope {
        ok: error.is_none(),
        command,
        format,
        target,
        data,
        summary,
        error,
        next_actions,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&envelope).expect("json envelope should serialize")
    );
}

fn count_summary(count: usize) -> serde_json::Value {
    json!({ "count": count })
}

fn diagnostics_summary(diagnostics: &[crate::model::Diagnostic]) -> serde_json::Value {
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .count();
    json!({
        "errors": errors,
        "warnings": warnings,
        "count": diagnostics.len()
    })
}

fn template_name(template: TemplateKind) -> &'static str {
    template.name()
}

fn finish(result: Result<(), CliError>, json_context: Option<JsonContext>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if let Some(message) = &error.message {
                if let Some(context) = json_context {
                    print_json_error(&context, &error, message.clone());
                } else {
                    eprintln!("error: {message}");
                }
            }
            ExitCode::from(error.exit_code)
        }
    }
}

fn finish_clap_error(error: clap::Error, json_context: Option<JsonContext>) -> ExitCode {
    let exit_code = match error.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => 0,
        _ => 2,
    };
    if exit_code == 0 || json_context.is_none() {
        error.print().expect("clap should print help");
    } else if let Some(context) = json_context {
        let cli_error = CliError::usage("invalid_usage", error.to_string());
        print_json_error(&context, &cli_error, error.to_string());
    }
    ExitCode::from(exit_code)
}

fn print_json_error(context: &JsonContext, error: &CliError, message: String) {
    let json_error = JsonError {
        code: error.code,
        category: error.category,
        message,
        path: context.target.clone(),
        line: None,
        details: None,
    };
    print_json_envelope::<serde_json::Value>(
        context.command,
        context.target.as_deref(),
        "error.v1",
        None,
        None,
        Some(json_error),
        recovery_actions(context, error),
    );
}

fn recovery_actions(context: &JsonContext, error: &CliError) -> Vec<JsonNextAction> {
    let Some(target) = &context.target else {
        return Vec::new();
    };

    match error.code {
        "anchor_resolution_failed" => vec![JsonNextAction {
            label: "Inspect available ids".to_string(),
            command: vec![
                "mdm".to_string(),
                "links".to_string(),
                target_path_without_anchor(target),
                "--plain".to_string(),
            ],
            writes: false,
        }],
        "parser_errors" => vec![JsonNextAction {
            label: "Review validation diagnostics".to_string(),
            command: vec![
                "mdm".to_string(),
                "validate".to_string(),
                target_path_without_anchor(target),
                "--plain".to_string(),
            ],
            writes: false,
        }],
        _ => Vec::new(),
    }
}

fn target_path_without_anchor(target: &str) -> String {
    target
        .split_once('#')
        .map(|(path, _)| path)
        .unwrap_or(target)
        .to_string()
}

fn raw_args_json_context() -> Option<JsonContext> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !args.iter().any(|arg| arg == "--json") {
        return None;
    }

    if args.first().map(String::as_str) == Some("mindspace") {
        let command = match args.get(1).map(String::as_str) {
            Some("scan") => "mindspace scan",
            Some("lint") => "mindspace lint",
            Some("setup") => "mindspace setup",
            Some("context") => "mindspace context",
            Some("session") => "mindspace session",
            Some("review") => "mindspace review",
            Some("template") if args.get(2).is_some_and(|arg| arg == "list") => {
                "mindspace template list"
            }
            Some("template") if args.get(2).is_some_and(|arg| arg == "show") => {
                "mindspace template show"
            }
            _ => "mindspace",
        };
        let target = match command {
            "mindspace scan" | "mindspace lint" | "mindspace setup" | "mindspace context" => {
                args.get(2)
            }
            "mindspace session" | "mindspace review" => args.get(3),
            "mindspace template show" => args.get(3),
            _ => None,
        }
        .filter(|value| !value.starts_with('-'))
        .cloned();
        return Some(JsonContext { command, target });
    }

    let command = match args.first().map(String::as_str) {
        Some("view") => "view",
        Some("find") => "find",
        Some("tags") => "tags",
        Some("kv") => "kv",
        Some("links") => "links",
        Some("refs") => "refs",
        Some("relations") => "relations",
        Some("validate") => "validate",
        Some("ai") if args.get(1).is_some_and(|arg| arg == "presets") => "ai presets",
        Some("commands") => "commands",
        Some("version") => "version",
        Some("open") => "open",
        _ => "mdm",
    };
    let target = args.get(1).filter(|value| !value.starts_with('-')).cloned();

    Some(JsonContext { command, target })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_catalog_covers_clap_subcommands() {
        let mut clap_paths = BTreeSet::new();
        collect_clap_subcommands(&Cli::command(), None, &mut clap_paths);
        let catalog_paths = command_catalog()
            .commands
            .iter()
            .map(|command| command.name.to_string())
            .collect::<BTreeSet<_>>();

        assert_eq!(catalog_paths, clap_paths);
    }

    fn collect_clap_subcommands(
        command: &clap::Command,
        parent: Option<String>,
        paths: &mut BTreeSet<String>,
    ) {
        for subcommand in command.get_subcommands() {
            let path = match &parent {
                Some(parent) => format!("{} {}", parent, subcommand.get_name()),
                None => subcommand.get_name().to_string(),
            };
            paths.insert(path.clone());
            collect_clap_subcommands(subcommand, Some(path), paths);
        }
    }

    #[test]
    fn command_catalog_marks_interactive_and_writing_commands() {
        let catalog = command_catalog();
        let open = catalog
            .commands
            .iter()
            .find(|command| command.name == "open")
            .expect("open should be cataloged");
        assert!(open.interactive);
        assert!(open.output_modes.contains(&"json"));

        let init = catalog
            .commands
            .iter()
            .find(|command| command.name == "init")
            .expect("init should be cataloged");
        assert!(init.writes.contains(&"map"));
        assert!(!init.interactive);

        let command_catalog = catalog
            .commands
            .iter()
            .find(|command| command.name == "commands")
            .expect("commands should be cataloged");
        assert!(command_catalog.formats.contains(&"command_catalog.v1"));
    }
}
