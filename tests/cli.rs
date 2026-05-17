use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
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

fn serve_once(body: &'static str) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let address = listener
        .local_addr()
        .expect("test server should have an address");
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("test server should accept");
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("test server should respond");
    });
    (format!("http://{address}/article"), handle)
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf-8")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be utf-8")
}

fn json_stdout(output: &std::process::Output) -> serde_json::Value {
    serde_json::from_str(&stdout(output)).expect("stdout should be valid json")
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
fn find_json_uses_a_success_envelope() {
    let output = run_mdm(&["find", &fixture("sample.md"), "#prompt", "--json"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "find");
    assert_eq!(value["format"], "search_matches.v1");
    assert_eq!(value["target"], fixture("sample.md"));
    assert_eq!(value["summary"]["count"], 1);
    assert_eq!(value["data"][0]["text"], "Prompt Library");
    assert_eq!(value["data"][0]["id"], "prompts/library");
}

#[test]
fn find_json_query_miss_returns_an_empty_success_envelope() {
    let output = run_mdm(&[
        "find",
        &fixture("sample.md"),
        "no-such-query-token",
        "--json",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let value = json_stdout(&output);

    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "find");
    assert_eq!(value["summary"]["count"], 0);
    assert_eq!(
        value["data"]
            .as_array()
            .expect("data should be an array")
            .len(),
        0
    );
}

#[test]
fn json_mode_invalid_output_flags_return_an_error_envelope() {
    let output = run_mdm(&[
        "find",
        &fixture("sample.md"),
        "#prompt",
        "--json",
        "--plain",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["ok"], false);
    assert_eq!(value["command"], "find");
    assert_eq!(value["format"], "error.v1");
    assert_eq!(value["error"]["code"], "invalid_output_mode");
    assert_eq!(value["error"]["category"], "usage");
    assert!(
        value["error"]["message"]
            .as_str()
            .expect("message should be a string")
            .contains("Choose either --json or --plain")
    );
}

#[test]
fn json_mode_runtime_failures_return_an_error_envelope() {
    let missing = temp_file("missing.md");
    let output = run_mdm(&["view", missing.to_str().unwrap(), "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["ok"], false);
    assert_eq!(value["command"], "view");
    assert_eq!(value["format"], "error.v1");
    assert_eq!(value["target"], missing.to_string_lossy().as_ref());
    assert_eq!(value["error"]["code"], "file_read_failed");
    assert_eq!(value["error"]["category"], "filesystem");
}

#[test]
fn find_supports_task_state_queries() {
    let map_path = temp_file("task-query.md");
    std::fs::write(
        &map_path,
        "- Project\n  - [ ] Open checkbox\n  - [x] Done checkbox\n  - Blocked task #todo @status:blocked\n  - Decision @status:active\n",
    )
    .expect("task query fixture should be writable");

    let output = run_mdm(&["find", map_path.to_str().unwrap(), "task:open", "--plain"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let open_stdout = stdout(&output);
    assert!(open_stdout.contains("Open checkbox"));
    assert!(open_stdout.contains("Blocked task"));
    assert!(!open_stdout.contains("Done checkbox"));
    assert!(!open_stdout.contains("Decision"));

    let output = run_mdm(&["find", map_path.to_str().unwrap(), "task:done", "--plain"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let done_stdout = stdout(&output);
    assert!(done_stdout.contains("Done checkbox"));
    assert!(!done_stdout.contains("Open checkbox"));

    std::fs::remove_file(map_path).ok();
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
fn refs_can_list_external_references() {
    let map_path = temp_file("refs.md");
    std::fs::write(
        &map_path,
        "- Research [brief note](docs/project brief.md) ![diagram](assets/diagram.png)\n",
    )
    .expect("reference fixture should be writable");

    let output = run_mdm(&["refs", map_path.to_str().unwrap(), "--plain"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("brief note\tdocs/project brief.md"));
    assert!(stdout.contains("image\tdiagram\tassets/diagram.png"));

    std::fs::remove_file(map_path).ok();
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
fn import_opml_writes_native_map() {
    let source = temp_file("source.opml");
    let destination = temp_file("imported.md");
    std::fs::write(
        &source,
        r##"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <body>
    <outline text="Imported Project" mdm_id="imported" mdm_tags="#idea">
      <outline text="MVP Scope" mdm_task="open" status="active" mdm_detail="Keep this note" />
      <outline text="Reference" url="https://example.com/article" />
    </outline>
  </body>
</opml>
"##,
    )
    .expect("opml fixture should be writable");

    let source_str = source.to_string_lossy().into_owned();
    let destination_str = destination.to_string_lossy().into_owned();
    let output = run_mdm(&[
        "import",
        &source_str,
        "--from",
        "opml",
        "-o",
        &destination_str,
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(Path::new(&destination).exists());

    let contents = std::fs::read_to_string(&destination).expect("import should write a map");
    assert!(contents.contains("- Imported Project #idea [id:imported]"));
    assert!(contents.contains("  - [ ] MVP Scope @status:active"));
    assert!(contents.contains("    | Keep this note"));
    assert!(contents.contains("  - Reference [url](https://example.com/article)"));

    let validate = run_mdm(&["validate", &destination_str]);
    assert!(
        validate.status.success(),
        "stderr: {}\nstdout: {}",
        stderr(&validate),
        stdout(&validate)
    );

    std::fs::remove_file(source).ok();
    std::fs::remove_file(destination).ok();
}

#[test]
fn import_refuses_to_overwrite_without_force() {
    let source = temp_file("source.opml");
    let destination = temp_file("existing.md");
    std::fs::write(
        &source,
        r#"<opml version="2.0"><body><outline text="Imported" /></body></opml>"#,
    )
    .expect("opml fixture should be writable");
    std::fs::write(&destination, "- Existing\n").expect("existing output should be writable");

    let output = run_mdm(&[
        "import",
        source.to_str().unwrap(),
        "--from",
        "opml",
        "-o",
        destination.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("Refusing to overwrite"));

    let contents = std::fs::read_to_string(&destination).expect("existing file should remain");
    assert_eq!(contents, "- Existing\n");

    std::fs::remove_file(source).ok();
    std::fs::remove_file(destination).ok();
}

#[test]
fn import_opml_fixture_pack_writes_valid_maps() {
    let fixture_names = [
        "mdmind-roundtrip.opml",
        "feed-subscriptions.opml",
        "research-notes.opml",
        "desktop-outliner.opml",
    ];

    for fixture_name in fixture_names {
        let source = fixture(&format!("import/opml/{fixture_name}"));
        let destination = temp_file(&format!("{fixture_name}.md"));
        let destination_str = destination.to_string_lossy().into_owned();
        let output = run_mdm(&["import", &source, "--from", "opml", "-o", &destination_str]);
        assert!(
            output.status.success(),
            "{fixture_name} import stderr: {}",
            stderr(&output)
        );

        let validate = run_mdm(&["validate", &destination_str]);
        assert!(
            validate.status.success(),
            "{fixture_name} validate stderr: {}\nstdout: {}",
            stderr(&validate),
            stdout(&validate)
        );

        let contents = std::fs::read_to_string(&destination).expect("imported map should exist");
        assert!(
            contents.starts_with("- "),
            "{fixture_name} should write native map nodes"
        );
        std::fs::remove_file(destination).ok();
    }
}

#[test]
fn import_markdown_writes_native_map() {
    let source = temp_file("source.md");
    let destination = temp_file("imported-markdown.md");
    std::fs::write(
        &source,
        "# Imported Project [id:imported]\n\nOpening detail.\n\n## Tasks\n\n- [ ] First task #todo @status:active\n  - Nested note\n",
    )
    .expect("markdown fixture should be writable");

    let source_str = source.to_string_lossy().into_owned();
    let destination_str = destination.to_string_lossy().into_owned();
    let output = run_mdm(&[
        "import",
        &source_str,
        "--from",
        "markdown",
        "-o",
        &destination_str,
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    let contents = std::fs::read_to_string(&destination).expect("import should write a map");
    assert!(contents.contains("- Imported Project [id:imported]"));
    assert!(contents.contains("  | Opening detail."));
    assert!(contents.contains("  - Tasks"));
    assert!(contents.contains("    - [ ] First task #todo @status:active"));
    assert!(contents.contains("      - Nested note"));

    let validate = run_mdm(&["validate", &destination_str]);
    assert!(
        validate.status.success(),
        "stderr: {}\nstdout: {}",
        stderr(&validate),
        stdout(&validate)
    );

    std::fs::remove_file(source).ok();
    std::fs::remove_file(destination).ok();
}

#[test]
fn import_markdown_fixture_pack_writes_valid_maps() {
    let fixture_names = ["headings.md", "bullets.md", "mixed-notes.md"];

    for fixture_name in fixture_names {
        let source = fixture(&format!("import/markdown/{fixture_name}"));
        let destination = temp_file(&format!("{fixture_name}.imported.md"));
        let destination_str = destination.to_string_lossy().into_owned();
        let output = run_mdm(&[
            "import",
            &source,
            "--from",
            "markdown",
            "-o",
            &destination_str,
        ]);
        assert!(
            output.status.success(),
            "{fixture_name} import stderr: {}",
            stderr(&output)
        );

        let validate = run_mdm(&["validate", &destination_str]);
        assert!(
            validate.status.success(),
            "{fixture_name} validate stderr: {}\nstdout: {}",
            stderr(&validate),
            stdout(&validate)
        );

        let contents = std::fs::read_to_string(&destination).expect("imported map should exist");
        assert!(
            contents.starts_with("- "),
            "{fixture_name} should write native map nodes"
        );
        std::fs::remove_file(destination).ok();
    }
}

#[test]
fn import_freemind_fixture_pack_writes_valid_maps() {
    let fixture_names = ["basic.mm", "multiple-roots.mm", "freeplane-style.mm"];

    for fixture_name in fixture_names {
        let source = fixture(&format!("import/freemind/{fixture_name}"));
        let destination = temp_file(&format!("{fixture_name}.imported.md"));
        let destination_str = destination.to_string_lossy().into_owned();
        let output = run_mdm(&[
            "import",
            &source,
            "--from",
            "freemind",
            "-o",
            &destination_str,
        ]);
        assert!(
            output.status.success(),
            "{fixture_name} import stderr: {}",
            stderr(&output)
        );

        let validate = run_mdm(&["validate", &destination_str]);
        assert!(
            validate.status.success(),
            "{fixture_name} validate stderr: {}\nstdout: {}",
            stderr(&validate),
            stdout(&validate)
        );

        let contents = std::fs::read_to_string(&destination).expect("imported map should exist");
        assert!(
            contents.starts_with("- "),
            "{fixture_name} should write native map nodes"
        );
        std::fs::remove_file(destination).ok();
    }
}

#[test]
fn import_html_fixture_pack_writes_valid_maps() {
    let fixture_names = ["article.html", "browser-export.html"];

    for fixture_name in fixture_names {
        let source = fixture(&format!("import/html/{fixture_name}"));
        let destination = temp_file(&format!("{fixture_name}.imported.md"));
        let destination_str = destination.to_string_lossy().into_owned();
        let output = run_mdm(&["import", &source, "-o", &destination_str]);
        assert!(
            output.status.success(),
            "{fixture_name} import stderr: {}",
            stderr(&output)
        );

        let contents = std::fs::read_to_string(&destination).expect("imported map should exist");
        assert!(
            contents.starts_with("- "),
            "{fixture_name} should write native map nodes"
        );

        let validate = run_mdm(&["validate", &destination_str]);
        assert!(
            validate.status.success(),
            "{fixture_name} validate stderr: {}\nstdout: {}",
            stderr(&validate),
            stdout(&validate)
        );

        std::fs::remove_file(destination).ok();
    }
}

#[test]
fn import_preview_prints_map_without_output_path() {
    let source = temp_file("preview-source.md");
    std::fs::write(&source, "# Preview Map\n\n- Child\n")
        .expect("preview source should be writable");

    let output = run_mdm(&[
        "import",
        source.to_str().unwrap(),
        "--from",
        "markdown",
        "--preview",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("- Preview Map"));
    assert!(stdout.contains("  - Child"));

    std::fs::remove_file(source).ok();
}

#[test]
fn import_infers_format_and_default_output_path() {
    let source = temp_file("auto.md");
    std::fs::write(&source, "# Auto Import\n\n- Child\n").expect("source should be writable");
    let expected_output = source.with_file_name(format!(
        "{}-mind.md",
        source
            .file_stem()
            .expect("source should have a stem")
            .to_string_lossy()
    ));

    let output = run_mdm(&["import", source.to_str().unwrap()]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output).trim(), expected_output.to_string_lossy());
    assert!(expected_output.exists());

    let contents = std::fs::read_to_string(&expected_output).expect("default output should exist");
    assert!(contents.contains("- Auto Import"));
    assert!(contents.contains("  - Child"));

    std::fs::remove_file(source).ok();
    std::fs::remove_file(expected_output).ok();
}

#[test]
fn import_preview_can_infer_format_without_output_path() {
    let source = temp_file("auto-preview.opml");
    std::fs::write(
        &source,
        r#"<opml version="2.0"><body><outline text="Auto Preview" /></body></opml>"#,
    )
    .expect("source should be writable");

    let output = run_mdm(&["import", source.to_str().unwrap(), "--preview"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("- Auto Preview"));

    std::fs::remove_file(source).ok();
}

#[test]
fn import_can_infer_freemind_format() {
    let source = temp_file("auto.mm");
    std::fs::write(
        &source,
        r#"<map version="1.0.1"><node TEXT="Auto FreeMind" /></map>"#,
    )
    .expect("source should be writable");

    let output = run_mdm(&["import", source.to_str().unwrap(), "--preview"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("- Auto FreeMind"));

    std::fs::remove_file(source).ok();
}

#[test]
fn import_requires_from_when_extension_is_unknown() {
    let source = temp_file("unknown.data");
    std::fs::write(&source, "# Unknown\n").expect("source should be writable");

    let output = run_mdm(&["import", source.to_str().unwrap(), "--preview"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("Could not infer import format"));

    std::fs::remove_file(source).ok();
}

#[test]
fn import_guides_planned_archive_and_pdf_formats() {
    let cases = [
        ("archive.xmind", "XMind `.xmind` import is planned"),
        (
            "archive.mmap",
            "MindManager `.mmap` import is not implemented",
        ),
        (
            "report.pdf",
            "PDF ingestion is intentionally agent-authored",
        ),
    ];

    for (name, expected) in cases {
        let source = temp_file(name);
        std::fs::write(&source, "placeholder").expect("source should be writable");

        let output = run_mdm(&["import", source.to_str().unwrap(), "--preview"]);
        assert_eq!(output.status.code(), Some(1), "{name} should fail");
        assert!(
            stderr(&output).contains(expected),
            "{name} stderr should contain {expected:?}; got {}",
            stderr(&output)
        );

        std::fs::remove_file(source).ok();
    }
}

#[test]
fn import_fetches_remote_web_sources_with_agent_guidance() {
    let (url, handle) = serve_once(
        "<!doctype html><html><body><h1>Remote Article</h1><p>Fetched body.</p><ul><li>Point one</li></ul></body></html>",
    );

    let output = run_mdm(&["import", &url, "--preview", "--report"]);
    handle.join().expect("test server should finish");
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).contains("warning: web import is rough structural extraction"));
    assert!(stderr(&output).contains("- format: web"));
    assert!(stdout(&output).contains("- Remote Article"));
    assert!(stdout(&output).contains("  | Fetched body."));
    assert!(stdout(&output).contains("  - Point one"));
}

#[test]
fn import_fetches_remote_html_when_format_is_explicit() {
    let (url, handle) =
        serve_once("<!doctype html><html><body><h1>Explicit HTML</h1></body></html>");

    let output = run_mdm(&["import", &url, "--from", "html", "--preview"]);
    handle.join().expect("test server should finish");
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).contains("warning: web import is rough structural extraction"));
    assert!(stdout(&output).contains("- Explicit HTML"));
}

#[test]
fn import_report_summarizes_imported_map() {
    let source = temp_file("report-source.md");
    let destination = temp_file("report-output.md");
    std::fs::write(
        &source,
        "# Report Map [id:report]\n\nDetail line.\n\n- [ ] Task #todo @status:active [brief](docs/brief.md)\n",
    )
    .expect("report source should be writable");

    let output = run_mdm(&[
        "import",
        source.to_str().unwrap(),
        "--from",
        "markdown",
        "-o",
        destination.to_str().unwrap(),
        "--report",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let report = stderr(&output);
    assert!(report.contains("Import report"));
    assert!(report.contains("- format: markdown"));
    assert!(report.contains("- nodes: 2"));
    assert!(report.contains("- roots: 1"));
    assert!(report.contains("- leaves: 1"));
    assert!(report.contains("- detail_lines: 1"));
    assert!(report.contains("- detail_nodes: 1"));
    assert!(report.contains("- tags: 1"));
    assert!(report.contains("- metadata: 1"));
    assert!(report.contains("- ids: 1"));
    assert!(report.contains("- duplicate_ids: 0"));
    assert!(report.contains("- references: 1"));
    assert!(report.contains("- reference_links: 1"));
    assert!(report.contains("- reference_images: 0"));
    assert!(report.contains("- reference_urls: 0"));
    assert!(report.contains("- reference_local: 1"));
    assert!(report.contains("- tasks: 1"));
    assert!(report.contains("- task_open: 1"));
    assert!(report.contains("- task_done: 0"));
    assert!(report.contains("- validation_errors: 0"));
    assert!(report.contains("- validation_warnings: 0"));
    assert!(report.contains("- tag_breakdown: #todo=1"));
    assert!(report.contains("- metadata_keys: status=1"));
    assert!(report.contains("Imported '"));

    std::fs::remove_file(source).ok();
    std::fs::remove_file(destination).ok();
}

#[test]
fn import_report_flags_duplicate_ids() {
    let source = temp_file("report-duplicates.md");
    std::fs::write(
        &source,
        "# Duplicate A [id:dupe]\n\n# Duplicate B [id:dupe]\n",
    )
    .expect("report source should be writable");

    let output = run_mdm(&[
        "import",
        source.to_str().unwrap(),
        "--from",
        "markdown",
        "--preview",
        "--report",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let report = stderr(&output);
    assert!(report.contains("- duplicate_ids: 1"));
    assert!(report.contains("- validation_errors: 1"));

    std::fs::remove_file(source).ok();
}

#[test]
fn import_help_lists_formats_defaults_and_reporting() {
    let output = run_mdm(&["import", "--help"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let help = stdout(&output);

    assert!(help.contains("freemind"));
    assert!(help.contains("html"));
    assert!(help.contains("markdown"));
    assert!(help.contains("mindmanager"));
    assert!(help.contains("opml"));
    assert!(help.contains("pdf"));
    assert!(help.contains("web"));
    assert!(help.contains("xmind"));
    assert!(help.contains(".opml"));
    assert!(help.contains(".html"));
    assert!(help.contains(".xmind"));
    assert!(help.contains(".pdf"));
    assert!(help.contains("<source-stem>-mind.md"));
    assert!(help.contains("--preview"));
    assert!(help.contains("--report"));
}

#[test]
fn export_supports_query_filtered_scope() {
    let output = run_mdm(&[
        "export",
        &example("meeting-notes-action-map.md"),
        "--query",
        "#todo @owner:maya",
        "--format",
        "json",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("export should be valid json");

    let root = &value["nodes"][0];
    assert_eq!(root["text"], "Harbor Team Weekly Notes");
    let children = root["children"]
        .as_array()
        .expect("children should serialize as an array");
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["text"], "Action Items");
    assert_eq!(children[0]["children"][0]["text"], "Draft field card copy");
    assert_eq!(
        children[0]["children"]
            .as_array()
            .expect("children should serialize as an array")
            .len(),
        1
    );
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
fn validate_json_failure_includes_diagnostics_in_an_error_envelope() {
    let output = run_mdm(&["validate", &fixture("invalid.md"), "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["ok"], false);
    assert_eq!(value["command"], "validate");
    assert_eq!(value["format"], "diagnostics.v1");
    assert_eq!(value["error"]["code"], "validation_failed");
    assert_eq!(value["summary"]["errors"], 3);
    assert!(
        value["data"]
            .as_array()
            .expect("diagnostics should be an array")
            .iter()
            .any(|diagnostic| diagnostic["message"]
                .as_str()
                .unwrap_or_default()
                .contains("Duplicate id"))
    );
    assert_eq!(
        value["next_actions"][0]["command"][0],
        serde_json::Value::String("mdm".to_string())
    );
}

#[test]
fn unsupported_export_formats_remain_human_readable_errors() {
    let output = run_mdm(&["export", &fixture("sample.md"), "--format", "yaml"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty());
    assert!(stderr(&output).contains("Unsupported export format 'yaml'"));
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
fn init_supports_the_todo_template() {
    let destination = temp_file("TODO.md");
    let destination_str = destination.to_string_lossy().into_owned();
    let output = run_mdm(&["init", &destination_str, "--template", "todo"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let contents = std::fs::read_to_string(&destination).expect("template should be written");
    assert!(contents.contains("- Project TODO Map #todo-map @status:active [id:todo]"));
    assert!(contents.contains("- [ ] Define next slice #todo @status:active"));
    assert!(contents.contains("mdm find TODO.md \"#todo @status:active\" --plain"));
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
fn mdmind_preview_without_a_target_returns_a_runtime_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdmind"))
        .arg("--preview")
        .output()
        .expect("mdmind command should run");
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("`mdmind --preview` needs a target path."));
}

#[test]
fn mdmind_without_a_target_requires_an_interactive_terminal_for_startup() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdmind"))
        .output()
        .expect("mdmind command should run");
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains(
        "No target was provided. Run `mdmind path/to/map.md`, or start `mdmind` in an interactive terminal to create one."
    ));
}

#[test]
fn mdmind_key_diagnostics_requires_an_interactive_terminal() {
    let output = Command::new(env!("CARGO_BIN_EXE_mdmind"))
        .arg("--check-keys")
        .output()
        .expect("mdmind command should run");
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("Key diagnostics need an interactive terminal."));
}

#[test]
fn mdm_key_diagnostics_requires_an_interactive_terminal() {
    let output = run_mdm(&["check-keys"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("Key diagnostics need an interactive terminal."));
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
    assert_eq!(contents, include_str!("../examples/demo.md"));

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
    assert!(destination.join("model-benchmark-comparison.md").is_file());
    assert!(destination.join("meeting-notes-action-map.md").is_file());
    assert!(destination.join("agent-research-handoff.md").is_file());
    assert!(destination.join("agent-todo-workflow.md").is_file());

    std::fs::remove_file(destination.join("README.md")).expect("README should be removable");
    for file_name in [
        "demo.md",
        "product-status.md",
        "model-benchmark-comparison.md",
        "meeting-notes-action-map.md",
        "agent-research-handoff.md",
        "agent-todo-workflow.md",
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
