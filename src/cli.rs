use std::path::PathBuf;
use std::process::ExitCode;

use clap::builder::PossibleValuesParser;
use clap::error::ErrorKind;
use clap::{ArgAction, Parser, Subcommand};

use crate::app::{
    AppError, create_from_template, diagnostics_for_validate, diagnostics_have_errors,
    ensure_parseable, load_document, select_document,
};
use crate::interactive::run_interactive;
use crate::query::{find_matches, link_entries, metadata_rows, tag_counts};
use crate::render::{
    render_find, render_find_plain, render_links, render_links_plain, render_metadata,
    render_metadata_plain, render_tags, render_tags_plain, render_tree, render_validate,
    render_validate_plain,
};
use crate::templates::TemplateKind;

#[derive(Debug, Parser)]
#[command(
    name = "mdm",
    version,
    about = "Inspect and validate local markdown-like thought maps.",
    long_about = "mdm is the CLI for local-first structured maps. It reads plain-text tree files, renders them for humans, and exports machine-friendly output when you ask for --json or --plain.",
    after_help = "Examples:\n  mdm init ideas.md --template product\n  mdm view ideas.md\n  mdm find ideas.md \"rate limit\"\n  mdm find ideas.md \"#todo\" --plain\n  mdm kv ideas.md --keys status,owner\n  mdm validate ideas.md\n  mdm export ideas.md --format json\n  mdm open ideas.md#product/api-design"
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
        #[arg(long, default_value = "json")]
        format: String,
    },
    #[command(about = "Create a new map from a starter template.")]
    Init {
        path: PathBuf,
        #[arg(long, value_parser = PossibleValuesParser::new(TemplateKind::names()))]
        template: String,
        #[arg(long, action = ArgAction::SetTrue)]
        force: bool,
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
}

#[derive(Debug, Parser)]
#[command(
    name = "mdmind",
    version,
    about = "Navigate and edit a map in a focused interactive terminal flow."
)]
struct TuiPreviewCli {
    target: String,
    #[arg(long)]
    preview: bool,
    #[arg(long)]
    autosave: bool,
    #[arg(long)]
    max_depth: Option<usize>,
}

struct CliError {
    message: Option<String>,
    exit_code: u8,
}

impl CliError {
    fn runtime(message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
            exit_code: 1,
        }
    }

    fn from_app(error: AppError) -> Self {
        Self::runtime(error.message().to_string())
    }

    fn silent(exit_code: u8) -> Self {
        Self {
            message: None,
            exit_code,
        }
    }
}

pub fn run_mdm() -> ExitCode {
    match Cli::try_parse() {
        Ok(cli) => finish(dispatch(cli)),
        Err(error) => finish_clap_error(error),
    }
}

pub fn run_mdmind() -> ExitCode {
    match TuiPreviewCli::try_parse() {
        Ok(cli) => finish(dispatch_tui_preview(cli)),
        Err(error) => finish_clap_error(error),
    }
}

fn dispatch(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::View {
            target,
            json,
            max_depth,
        } => render_view_like(&target, json, max_depth),
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
                json,
                plain,
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
                json,
                plain,
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
                json,
                plain,
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
                json,
                plain,
                &links,
                || render_links(&links),
                || render_links_plain(&links),
            )
        }
        Commands::Validate {
            target,
            json,
            plain,
        } => {
            let loaded = load_document(&target).map_err(CliError::from_app)?;
            let diagnostics = diagnostics_for_validate(&loaded);
            print_output(
                json,
                plain,
                &diagnostics,
                || render_validate(&diagnostics),
                || render_validate_plain(&diagnostics),
            )?;

            if diagnostics_have_errors(&diagnostics) {
                return Err(CliError::silent(1));
            }

            Ok(())
        }
        Commands::Export { target, format } => {
            if format != "json" {
                return Err(CliError::runtime(format!(
                    "Unsupported export format '{format}'. Only 'json' is available in the MVP."
                )));
            }
            let loaded = load_document(&target).map_err(CliError::from_app)?;
            let document = select_document(&loaded).map_err(CliError::from_app)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&document.export())
                    .expect("export serialization should succeed")
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
        Commands::Open {
            target,
            preview,
            autosave,
            json,
            max_depth,
        } => {
            if preview || json {
                render_view_like(&target, json, max_depth)
            } else {
                run_interactive(&target, autosave).map_err(CliError::from_app)
            }
        }
    }
}

fn dispatch_tui_preview(cli: TuiPreviewCli) -> Result<(), CliError> {
    if cli.preview {
        render_view_like(&cli.target, false, cli.max_depth)
    } else {
        run_interactive(&cli.target, cli.autosave).map_err(CliError::from_app)
    }
}

fn render_view_like(target: &str, json: bool, max_depth: Option<usize>) -> Result<(), CliError> {
    let loaded = load_document(target).map_err(CliError::from_app)?;
    let document = select_document(&loaded).map_err(CliError::from_app)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&document.export())
                .expect("export serialization should succeed")
        );
    } else {
        println!("{}", render_tree(&document, max_depth));
    }
    Ok(())
}

fn print_output<T: serde::Serialize>(
    json: bool,
    plain: bool,
    value: &T,
    pretty: impl FnOnce() -> String,
    plain_renderer: impl FnOnce() -> String,
) -> Result<(), CliError> {
    if json && plain {
        return Err(CliError::runtime(
            "Choose either --json or --plain, not both.",
        ));
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(value).expect("serialization should succeed")
        );
    } else if plain {
        println!("{}", plain_renderer());
    } else {
        println!("{}", pretty());
    }
    Ok(())
}

fn template_name(template: TemplateKind) -> &'static str {
    match template {
        TemplateKind::Product => "product",
        TemplateKind::Feature => "feature",
        TemplateKind::Prompts => "prompts",
        TemplateKind::Backlog => "backlog",
    }
}

fn finish(result: Result<(), CliError>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if let Some(message) = error.message {
                eprintln!("error: {message}");
            }
            ExitCode::from(error.exit_code)
        }
    }
}

fn finish_clap_error(error: clap::Error) -> ExitCode {
    let exit_code = match error.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => 0,
        _ => 2,
    };
    error.print().expect("clap should print help");
    ExitCode::from(exit_code)
}
