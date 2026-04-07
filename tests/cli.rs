use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
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
fn find_supports_plain_output() {
    let output = run_mdm(&["find", &fixture("sample.md"), "#prompt", "--plain"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("Prompt Library"));
    assert!(stdout.contains("prompts/library"));
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
    assert!(contents.contains("- Product Idea [id:product]"));
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
