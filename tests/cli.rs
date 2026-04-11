use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn example(name: &str) -> String {
    format!("{}/examples/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn run_mdm(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mdm"))
        .args(args)
        .output()
        .expect("mdm command should run")
}

fn temp_file(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("mdmind-{nonce}-{name}"))
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf-8")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be utf-8")
}

#[test]
fn view_renders_tree_output() {
    let output = run_mdm(&["view", &fixture("sample.md")]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("Product Idea #idea [id:product]"));
    assert!(stdout.contains("└── File format"));
}

#[test]
fn view_supports_label_path_fallback_when_no_id_exists() {
    let output = run_mdm(&[
        "view",
        &format!("{}#Product Idea/Prompt Library", fixture("sample.md")),
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("Prompt Library #prompt @owner:jason [id:prompts/library]"));
    assert!(!stdout.contains("Product Idea #idea [id:product]"));
}

#[test]
fn find_supports_plain_output() {
    let output = run_mdm(&["find", &fixture("sample.md"), "#prompt", "--plain"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("Prompt Library"));
    assert!(stdout.contains("prompts/library"));
}

#[test]
fn find_can_inspect_example_metadata_workflows() {
    let output = run_mdm(&[
        "find",
        &example("lantern-studio-map.md"),
        "@owner:mira",
        "--plain",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("make volunteer briefing cards feel elegant under low light"));
    assert!(stdout.contains("lantern/team/mira"));
}

#[test]
fn kv_can_inspect_example_owner_and_region_metadata() {
    let output = run_mdm(&[
        "kv",
        &example("game-world-moonwake.md"),
        "--keys",
        "owner,region",
        "--plain",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("moonwake/world/glass-marsh"));
    assert!(stdout.contains("\towner\tnora\t"));
}

#[test]
fn tags_can_summarize_the_writing_example() {
    let output = run_mdm(&["tags", &example("novel-research-writing-map.md"), "--plain"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("#chapter"));
    assert!(stdout.contains("#quote"));
    assert!(stdout.contains("#theme"));
}

#[test]
fn links_can_list_deep_link_targets_for_examples() {
    let output = run_mdm(&["links", &example("lantern-studio-map.md"), "--plain"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("lantern/execution/now"));
    assert!(stdout.contains("lantern/team/leah"));
}

#[test]
fn relations_can_list_outgoing_links_and_backlinks() {
    let output = run_mdm(&["relations", &fixture("relations.md"), "--plain"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let plain_output = stdout(&output);
    assert!(plain_output.contains("out\t"));
    assert!(plain_output.contains("prompts/library"));
    assert!(plain_output.contains("supports"));

    let focused = run_mdm(&[
        "relations",
        &format!("{}#product/mvp", fixture("relations.md")),
        "--plain",
    ]);
    assert!(focused.status.success(), "stderr: {}", stderr(&focused));
    let focused_stdout = stdout(&focused);
    assert!(focused_stdout.contains("in\t"));
    assert!(focused_stdout.contains("out\t"));
    assert!(focused_stdout.contains("Prompt Library"));

    let focused_by_label = run_mdm(&[
        "relations",
        &format!("{}#Product Idea/MVP Scope", fixture("relations.md")),
        "--plain",
    ]);
    assert!(
        focused_by_label.status.success(),
        "stderr: {}",
        stderr(&focused_by_label)
    );
    let focused_by_label_stdout = stdout(&focused_by_label);
    assert!(focused_by_label_stdout.contains("in\t"));
    assert!(focused_by_label_stdout.contains("out\t"));
}

#[test]
fn export_outputs_json() {
    let output = run_mdm(&["export", &fixture("sample.md"), "--format", "json"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("export should be valid json");
    assert_eq!(value["nodes"][0]["text"], "Product Idea");
    assert_eq!(value["nodes"][0]["children"][0]["kv"]["status"], "active");
}

#[test]
fn export_outputs_mermaid_for_a_subtree() {
    let output = run_mdm(&[
        "export",
        &format!("{}#product/mvp", fixture("sample.md")),
        "--format",
        "mermaid",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.starts_with("flowchart LR\n"));
    assert!(stdout.contains(r#"node_0["MVP Scope #todo @status:active [id:product/mvp]"]"#));
    assert!(stdout.contains("node_0 --> node_0_0"));
    assert!(!stdout.contains("Product Idea #idea"));
}

#[test]
fn export_outputs_opml() {
    let output = run_mdm(&["export", &fixture("sample.md"), "--format", "opml"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
    assert!(
        stdout.contains(r##"<outline text="Product Idea" mdm_id="product" mdm_tags="#idea">"##)
    );
    assert!(stdout.contains(
        r##"<outline text="MVP Scope" mdm_id="product/mvp" mdm_tags="#todo" status="active">"##
    ));
}

#[test]
fn validate_fails_with_exit_code_one_for_invalid_maps() {
    let output = run_mdm(&["validate", &fixture("invalid.md")]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = stdout(&output);
    assert!(stdout.contains("error"));
    assert!(stdout.contains("Duplicate id"));
}

#[test]
fn init_writes_selected_template() {
    let destination = temp_file("product.md");
    let destination_str = destination.to_string_lossy().into_owned();
    let output = run_mdm(&["init", &destination_str, "--template", "product"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(Path::new(&destination).exists());
    let contents = std::fs::read_to_string(&destination).expect("template should be written");
    assert!(contents.contains("- Product Roadmap [id:product]"));
    assert!(contents.contains("[[rel:supports->product/requirements]]"));
    std::fs::remove_file(destination).expect("temp file should be removable");
}

#[test]
fn init_supports_the_writing_template() {
    let destination = temp_file("writing.md");
    let destination_str = destination.to_string_lossy().into_owned();
    let output = run_mdm(&["init", &destination_str, "--template", "writing"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let contents = std::fs::read_to_string(&destination).expect("template should be written");
    assert!(contents.contains("- Story Map [id:story]"));
    assert!(contents.contains("[[story/characters/lead]]"));
    std::fs::remove_file(destination).expect("temp file should be removable");
}

#[test]
fn mdmind_binary_falls_back_to_preview() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdmind"))
        .args([
            fixture("sample.md"),
            String::from("--preview"),
            String::from("--max-depth"),
            String::from("1"),
        ])
        .output()
        .expect("mdmind command should run");
    assert!(output.status.success());
    assert!(stdout(&output).contains("MVP Scope"));
}

#[test]
fn version_command_prints_the_cli_version() {
    let output = run_mdm(&["version"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        stdout(&output),
        format!("mdm {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn examples_list_shows_bundled_examples() {
    let output = run_mdm(&["examples", "list"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("Bundled examples:"));
    assert!(stdout.contains("demo"));
    assert!(stdout.contains("novel-research-writing-map"));
}

#[test]
fn examples_copy_one_writes_requested_map() {
    let destination = temp_file("examples-one");
    let destination_str = destination.to_string_lossy().into_owned();
    let output = run_mdm(&["examples", "copy", "demo", "--to", &destination_str]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    let copied = destination.join("demo.md");
    let contents = std::fs::read_to_string(&copied).expect("copied example should exist");
    assert!(contents.contains("- mdmind Demo [id:demo]"));

    std::fs::remove_file(copied).expect("copied example should be removable");
    std::fs::remove_dir(destination).expect("temp directory should be removable");
}

#[test]
fn examples_copy_all_writes_gallery_and_maps() {
    let destination = temp_file("examples-all");
    let destination_str = destination.to_string_lossy().into_owned();
    let output = run_mdm(&["examples", "copy", "all", "--to", &destination_str]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    assert!(destination.join("README.md").is_file());
    assert!(destination.join("demo.md").is_file());
    assert!(destination.join("product-status.md").is_file());
    assert!(destination.join("meeting-notes-action-map.md").is_file());
    assert!(destination.join("agent-research-handoff.md").is_file());

    std::fs::remove_file(destination.join("README.md")).expect("README should be removable");
    for file_name in [
        "demo.md",
        "product-status.md",
        "meeting-notes-action-map.md",
        "agent-research-handoff.md",
        "lantern-studio-map.md",
        "game-world-moonwake.md",
        "novel-research-writing-map.md",
        "prompt-ops.md",
        "decision-log.md",
    ] {
        std::fs::remove_file(destination.join(file_name))
            .unwrap_or_else(|error| panic!("could not remove {file_name}: {error}"));
    }
    std::fs::remove_dir(destination).expect("temp directory should be removable");
}
