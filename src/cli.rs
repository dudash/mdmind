use std::path::PathBuf;
use std::process::ExitCode;

use clap::builder::PossibleValuesParser;
use clap::error::ErrorKind;
use clap::{ArgAction, Parser, Subcommand};

use crate::APP_VERSION;
use crate::app::{
    AppError, create_from_template, diagnostics_for_validate, diagnostics_have_errors,
    ensure_parseable, load_document, resolve_anchor_path, select_document,
};
use crate::examples::{
    all as bundled_examples, discover_examples_dir, find as find_example,
    readme_contents as examples_readme_contents,
};
use crate::export::export_document;
use crate::interactive::run_interactive;
use crate::query::{
    filter_document, find_matches, link_entries, metadata_rows, relation_entries,
    relation_entries_for_path, tag_counts,
};
use crate::render::{
    render_find, render_find_plain, render_links, render_links_plain, render_metadata,
    render_metadata_plain, render_relations, render_relations_plain, render_tags,
    render_tags_plain, render_tree, render_validate, render_validate_plain,
};
use crate::templates::TemplateKind;

#[derive(Debug, Parser)]
#[command(
    name = "mdm",
    version,
    about = "Inspect and validate local markdown-like thought maps.",
    long_about = "mdm is the CLI for local-first structured maps. It reads plain-text tree files, renders them for humans, and exports machine-friendly output when you ask for --json or --plain.",
    after_help = "Examples:\n  mdm version\n  mdm init ideas.md --template product\n  mdm view ideas.md\n  mdm find ideas.md \"rate limit\"\n  mdm find ideas.md \"#todo\" --plain\n  mdm kv ideas.md --keys status,owner\n  mdm links ideas.md\n  mdm relations ideas.md#product/api-design\n  mdm validate ideas.md\n  mdm export ideas.md --format json\n  mdm export ideas.md#product/mvp --format mermaid\n  mdm export ideas.md --format opml\n  mdm export ideas.md --query \"#todo @status:active\" --format json\n  mdm open ideas.md#product/api-design"
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
            value_parser = PossibleValuesParser::new(["json", "mermaid", "opml"])
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
    #[command(about = "List, locate, or copy the bundled example maps.")]
    Examples {
        #[command(subcommand)]
        command: ExampleCommands,
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
    #[command(about = "Print the mdm version.")]
    Version,
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
                json,
                plain,
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
        Commands::Examples { command } => dispatch_examples(command),
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
        Commands::Version => {
            println!("mdm {APP_VERSION}");
            Ok(())
        }
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
            export_document(&document, "json").expect("json export should succeed")
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
        TemplateKind::Writing => "writing",
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
