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

fn run_mdmind(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mdmind"))
        .args(args)
        .output()
        .expect("mdmind command should run")
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
fn view_redirects_ordinary_markdown_to_view_markdown() {
    let markdown_path = temp_file("README-view.md");
    std::fs::write(&markdown_path, "# Project\n\nNormal Markdown prose.\n")
        .expect("markdown fixture should be writable");

    let output = run_mdm(&["view", markdown_path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr(&output);
    assert!(stderr.contains("ordinary Markdown"));
    assert!(stderr.contains("mdm view-markdown"));
    assert!(stderr.contains("mdm import"));
    assert!(!stderr.contains("The map contains parser errors"));

    std::fs::remove_file(markdown_path).ok();
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
fn commands_json_lists_agent_command_catalog() {
    let output = run_mdm(&["commands", "--json"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "commands");
    assert_eq!(value["format"], "command_catalog.v1");
    assert_eq!(value["data"]["version"], env!("CARGO_PKG_VERSION"));

    let commands = value["data"]["commands"]
        .as_array()
        .expect("commands should be an array");
    let names = commands
        .iter()
        .filter_map(|command| command["name"].as_str())
        .collect::<Vec<_>>();

    for expected in [
        "view",
        "view-markdown",
        "find",
        "tags",
        "kv",
        "links",
        "refs",
        "relations",
        "validate",
        "export",
        "init",
        "import",
        "examples",
        "examples list",
        "examples path",
        "examples copy",
        "ai",
        "ai presets",
        "ai quick-add",
        "skills",
        "skills install",
        "mindspace",
        "mindspace scan",
        "mindspace lint",
        "mindspace setup",
        "mindspace context",
        "mindspace template",
        "mindspace template list",
        "mindspace template show",
        "commands",
        "changelog",
        "open",
        "check-keys",
        "version",
    ] {
        assert!(
            names.contains(&expected),
            "catalog should include {expected}; got {names:?}"
        );
    }

    let find = commands
        .iter()
        .find(|command| command["name"] == "find")
        .expect("find should be cataloged");
    assert_eq!(find["interactive"], false);
    assert!(
        find["output_modes"]
            .as_array()
            .unwrap()
            .contains(&"json".into())
    );
    assert!(find["args"].as_array().unwrap().len() >= 2);

    let open = commands
        .iter()
        .find(|command| command["name"] == "open")
        .expect("open should be cataloged");
    assert_eq!(open["interactive"], true);
    assert!(
        open["writes"]
            .as_array()
            .unwrap()
            .contains(&"session_sidecars".into())
    );
}

#[test]
fn mindspace_scan_json_inventories_mixed_folder_without_writing() {
    let root = temp_file("mindspace-scan");
    std::fs::create_dir_all(root.join("maps")).expect("maps directory should be writable");
    std::fs::create_dir_all(root.join("docs")).expect("docs directory should be writable");
    std::fs::create_dir_all(root.join("sources")).expect("sources directory should be writable");
    std::fs::create_dir_all(root.join("inbox")).expect("inbox directory should be writable");

    std::fs::write(
        root.join("maps").join("tasks.md"),
        "- Launch plan #launch [id:launch]\n  - Pricing work [[rel:depends_on->maps/decisions.md#decision/pricing]] [id:launch/pricing]\n",
    )
    .expect("map fixture should be writable");
    std::fs::write(
        root.join("maps").join("decisions.md"),
        "- Decisions [id:decisions]\n  - Billing model [id:decision/billing]\n",
    )
    .expect("map fixture should be writable");
    std::fs::write(
        root.join("docs").join("brief.md"),
        "# Brief\n\nOrdinary page.\n",
    )
    .expect("page fixture should be writable");
    std::fs::write(
        root.join("sources").join("interview.md"),
        "# Interview\n\nRaw notes.\n",
    )
    .expect("source fixture should be writable");
    std::fs::write(root.join("inbox").join("capture.md"), "- loose capture\n")
        .expect("inbox fixture should be writable");
    std::fs::write(root.join("AGENTS.md"), "# Agent guidance\n")
        .expect("instruction fixture should be writable");
    std::fs::write(root.join("index.md"), "# Index\n").expect("index fixture should be writable");
    std::fs::write(root.join("log.md"), "# Log\n").expect("log fixture should be writable");

    let output = run_mdm(&["mindspace", "scan", root.to_str().unwrap(), "--json"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "mindspace scan");
    assert_eq!(value["format"], "mindspace_scan.v1");
    assert_eq!(value["target"], root.to_string_lossy().as_ref());
    assert_eq!(value["data"]["manifest"]["present"], false);
    assert_eq!(value["summary"]["roles"]["maps"], 2);
    assert_eq!(value["summary"]["roles"]["pages"], 1);
    assert!(value["summary"]["roles"]["sources"].as_u64().unwrap() >= 1);
    assert!(value["summary"]["roles"]["inbox"].as_u64().unwrap() >= 1);
    assert_eq!(value["summary"]["roles"]["instructions"], 1);
    assert_eq!(value["summary"]["roles"]["indexes"], 1);
    assert_eq!(value["summary"]["roles"]["logs"], 1);
    assert_eq!(value["summary"]["diagnostics"]["warnings"], 1);

    let role_paths = value["data"]["roles"]
        .as_array()
        .expect("roles should be an array")
        .iter()
        .filter_map(|role| role["path"].as_str())
        .collect::<Vec<_>>();
    assert!(role_paths.contains(&"maps/tasks.md"));
    assert!(role_paths.contains(&"docs/brief.md"));
    assert!(role_paths.contains(&"sources/interview.md"));
    assert!(role_paths.contains(&"inbox/capture.md"));
    assert!(role_paths.contains(&"AGENTS.md"));

    let diagnostic_codes = value["data"]["diagnostics"]
        .as_array()
        .expect("diagnostics should be an array")
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .collect::<Vec<_>>();
    assert!(diagnostic_codes.contains(&"map_validation_warning"));
    assert!(
        !root.join(".mdmind").exists(),
        "scan should not create a mindspace manifest directory"
    );

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn mindspace_setup_preview_empty_folder_does_not_write() {
    let root = temp_file("mindspace-setup-empty");
    std::fs::create_dir_all(&root).expect("empty mindspace root should be writable");

    let output = run_mdm(&[
        "mindspace",
        "setup",
        root.to_str().unwrap(),
        "--preview",
        "--json",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "mindspace setup");
    assert_eq!(value["format"], "mindspace_setup.v1");
    assert_eq!(value["target"], root.to_string_lossy().as_ref());
    assert_eq!(value["data"]["mode"], "preview");
    assert_eq!(value["data"]["written"], false);
    assert_eq!(
        value["data"]["manifest"]["schema_version"],
        "mdmind.mindspace.v1"
    );
    assert_eq!(value["data"]["manifest"]["root"], "..");
    assert_eq!(value["data"]["summary"]["roles"], 1);
    assert!(
        value["data"]["notes"]
            .as_array()
            .expect("notes should be an array")
            .iter()
            .any(|note| note.as_str() == Some("Preview mode did not write files."))
    );

    let role_paths = value["data"]["manifest"]["roles"]
        .as_array()
        .expect("roles should be an array")
        .iter()
        .filter_map(|role| role["path"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(role_paths, vec![".mdmind/reports"]);
    assert!(
        !root.join(".mdmind").exists(),
        "setup preview should not create the manifest directory"
    );

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn mindspace_setup_write_native_folder_creates_manifest_only() {
    let root = temp_file("mindspace-setup-native");
    std::fs::create_dir_all(root.join("maps")).expect("maps directory should be writable");
    let map_path = root.join("maps").join("tasks.md");
    let map_source = "- Tasks [id:tasks]\n  - Ship setup [id:tasks/setup]\n";
    std::fs::write(&map_path, map_source).expect("map fixture should be writable");

    let output = run_mdm(&[
        "mindspace",
        "setup",
        root.to_str().unwrap(),
        "--write",
        "--json",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "mindspace setup");
    assert_eq!(value["format"], "mindspace_setup.v1");
    assert_eq!(value["data"]["mode"], "write");
    assert_eq!(value["data"]["written"], true);
    assert_eq!(value["data"]["created_directory"], true);
    assert_eq!(std::fs::read_to_string(&map_path).unwrap(), map_source);
    assert!(root.join(".mdmind").join("mindspace.json").exists());
    assert!(
        !root.join(".mdmind").join("reports").exists(),
        "setup should not create generated report directories"
    );

    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".mdmind").join("mindspace.json")).unwrap(),
    )
    .expect("written manifest should be valid json");
    assert_eq!(manifest["schema_version"], "mdmind.mindspace.v1");
    assert_eq!(manifest["root"], "..");
    assert!(
        manifest["roles"]
            .as_array()
            .expect("roles should be an array")
            .iter()
            .any(|role| role["role"] == "map" && role["path"] == "maps/tasks.md")
    );
    assert!(
        manifest["roles"]
            .as_array()
            .expect("roles should be an array")
            .iter()
            .any(|role| role["role"] == "report" && role["generated"] == true)
    );

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn mindspace_setup_preview_mixed_folder_infers_roles_without_adopting() {
    let root = temp_file("mindspace-setup-mixed");
    std::fs::create_dir_all(root.join("maps")).expect("maps directory should be writable");
    std::fs::create_dir_all(root.join("docs")).expect("docs directory should be writable");
    std::fs::create_dir_all(root.join("sources")).expect("sources directory should be writable");
    std::fs::create_dir_all(root.join("inbox")).expect("inbox directory should be writable");
    std::fs::write(
        root.join("maps").join("roadmap.md"),
        "- Roadmap [id:roadmap]\n",
    )
    .expect("map fixture should be writable");
    std::fs::write(
        root.join("docs").join("brief.md"),
        "# Brief\n\nOrdinary Markdown.\n",
    )
    .expect("page fixture should be writable");
    std::fs::write(
        root.join("sources").join("interview.md"),
        "# Interview\n\nRaw notes.\n",
    )
    .expect("source fixture should be writable");
    std::fs::write(root.join("inbox").join("capture.md"), "- loose capture\n")
        .expect("inbox fixture should be writable");
    std::fs::write(root.join("AGENTS.md"), "# Agent guidance\n")
        .expect("instruction fixture should be writable");
    std::fs::write(root.join("index.md"), "# Index\n").expect("index fixture should be writable");
    std::fs::write(root.join("log.md"), "# Log\n").expect("log fixture should be writable");

    let output = run_mdm(&[
        "mindspace",
        "setup",
        root.to_str().unwrap(),
        "--template",
        "launch-planning",
        "--preview",
        "--json",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["data"]["mode"], "preview");
    assert_eq!(value["data"]["template"]["id"], "launch-planning");
    assert_eq!(value["data"]["template"]["persona_fit"], "Priya Planner");
    let roles = value["data"]["manifest"]["roles"]
        .as_array()
        .expect("roles should be an array");
    let role_pairs = roles
        .iter()
        .filter_map(|role| Some((role["role"].as_str()?, role["path"].as_str()?)))
        .collect::<Vec<_>>();
    for expected in [
        ("instruction", "AGENTS.md"),
        ("index", "index.md"),
        ("log", "log.md"),
        ("map", "maps/roadmap.md"),
        ("page", "docs/brief.md"),
        ("source", "sources"),
        ("inbox", "inbox"),
        ("report", ".mdmind/reports"),
    ] {
        assert!(
            role_pairs.contains(&expected),
            "setup preview should include role {expected:?}; got {role_pairs:?}"
        );
    }
    assert!(
        !role_pairs.contains(&("source", "sources/interview.md")),
        "directory roles should cover source files without duplicate manifest entries"
    );
    assert!(
        !root.join(".mdmind").exists(),
        "setup preview should not create a manifest directory"
    );

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn mindspace_setup_write_existing_manifest_preserves_overrides() {
    let root = temp_file("mindspace-setup-existing");
    std::fs::create_dir_all(root.join(".mdmind")).expect("manifest directory should be writable");
    std::fs::create_dir_all(root.join("maps")).expect("maps directory should be writable");
    std::fs::create_dir_all(root.join("docs")).expect("docs directory should be writable");
    std::fs::write(root.join("maps").join("tasks.md"), "- Tasks [id:tasks]\n")
        .expect("map fixture should be writable");
    std::fs::write(root.join("docs").join("brief.md"), "# Brief\n")
        .expect("page fixture should be writable");
    std::fs::write(
        root.join(".mdmind").join("mindspace.json"),
        r#"{
  "schema_version": "mdmind.mindspace.v1",
  "name": "Existing Brain",
  "root": "..",
  "roles": [
    {"role": "source", "path": "docs/brief.md", "read_only": true}
  ],
  "settings": {
    "source_read_only_default": false,
    "custom_setting": "keep"
  }
}
"#,
    )
    .expect("existing manifest should be writable");

    let output = run_mdm(&[
        "mindspace",
        "setup",
        root.to_str().unwrap(),
        "--write",
        "--json",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["data"]["existing_manifest"], true);
    assert_eq!(value["data"]["created_directory"], false);
    assert_eq!(value["data"]["summary"]["preserved_roles"], 1);
    let manifest = &value["data"]["manifest"];
    assert_eq!(manifest["name"], "Existing Brain");
    assert_eq!(manifest["settings"]["source_read_only_default"], false);
    assert_eq!(manifest["settings"]["custom_setting"], "keep");

    let roles = manifest["roles"]
        .as_array()
        .expect("roles should be an array");
    assert!(
        roles
            .iter()
            .any(|role| role["role"] == "source" && role["path"] == "docs/brief.md")
    );
    assert!(
        !roles
            .iter()
            .any(|role| role["role"] == "page" && role["path"] == "docs/brief.md"),
        "existing role override should prevent an inferred page duplicate"
    );
    assert!(
        roles
            .iter()
            .any(|role| role["role"] == "map" && role["path"] == "maps/tasks.md")
    );

    let written_manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".mdmind").join("mindspace.json")).unwrap(),
    )
    .expect("written manifest should be valid json");
    assert_eq!(&written_manifest, manifest);

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn mindspace_setup_write_rejects_malformed_existing_manifest_roles() {
    let root = temp_file("mindspace-setup-invalid-existing");
    std::fs::create_dir_all(root.join(".mdmind")).expect("manifest directory should be writable");
    let manifest_path = root.join(".mdmind").join("mindspace.json");
    let source = r#"{
  "schema_version": "mdmind.mindspace.v1",
  "name": "Needs Review",
  "root": "..",
  "roles": "source/*.md",
  "settings": {}
}
"#;
    std::fs::write(&manifest_path, source).expect("existing manifest should be writable");

    let output = run_mdm(&[
        "mindspace",
        "setup",
        root.to_str().unwrap(),
        "--write",
        "--json",
    ]);

    assert!(!output.status.success());
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"]["code"], "runtime_error");
    assert!(
        value["error"]["message"]
            .as_str()
            .expect("json error should include a message")
            .contains("roles field, but it is not an array")
    );
    assert_eq!(std::fs::read_to_string(&manifest_path).unwrap(), source);

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn mindspace_context_target_bundle_includes_relation_and_bounded_source() {
    let root = temp_file("mindspace-context-target");
    std::fs::create_dir_all(root.join("maps")).expect("maps directory should be writable");
    std::fs::create_dir_all(root.join("sources")).expect("sources directory should be writable");
    std::fs::write(
        root.join("maps").join("roadmap.md"),
        "- Launch [id:launch]\n  - Pricing [id:launch/pricing] [pricing source](sources/pricing.md) [[rel:depends-on->maps/decisions.md#decision/pricing]]\n    | Pricing needs the decision record and a source excerpt.\n",
    )
    .expect("roadmap map should be writable");
    std::fs::write(
        root.join("maps").join("decisions.md"),
        "- Decisions [id:decision]\n  - Pricing Decision [id:decision/pricing]\n    | Use the simple packaging model.\n",
    )
    .expect("decisions map should be writable");
    std::fs::write(
        root.join("sources").join("pricing.md"),
        "# Pricing interview\n\nCustomers asked for simple packaging and fewer tiers.\n",
    )
    .expect("source should be writable");

    let output = run_mdm(&[
        "mindspace",
        "context",
        "maps/roadmap.md#launch/pricing",
        "--root",
        root.to_str().unwrap(),
        "--include-source-refs",
        "--max-source-chars",
        "24",
        "--json",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "mindspace context");
    assert_eq!(value["format"], "mindspace_context.v1");
    assert_eq!(value["data"]["summary"]["branches"], 2);
    assert_eq!(value["data"]["summary"]["files"], 2);
    assert_eq!(value["data"]["summary"]["sources"], 1);

    let branches = value["data"]["branches"]
        .as_array()
        .expect("branches should be an array");
    let branch_refs = branches
        .iter()
        .map(|branch| {
            (
                branch["file"].as_str().unwrap(),
                branch["id"].as_str().unwrap(),
                branch["reason"].as_str().unwrap(),
            )
        })
        .collect::<Vec<_>>();
    assert!(branch_refs.iter().any(|(file, id, reason)| {
        *file == "maps/roadmap.md" && *id == "launch/pricing" && reason.contains("target branch")
    }));
    assert!(branch_refs.iter().any(|(file, id, reason)| {
        *file == "maps/decisions.md" && *id == "decision/pricing" && reason.contains("relation")
    }));

    let source = &value["data"]["sources"][0];
    assert_eq!(source["target"], "sources/pricing.md");
    assert_eq!(source["kind"], "local_file");
    assert_eq!(source["read_only"], true);
    assert!(
        source["excerpt"]
            .as_str()
            .unwrap()
            .contains("# Pricing interview")
    );
    assert!(source["omitted_chars"].as_u64().unwrap() > 0);

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn mindspace_context_query_bundle_spans_maps_and_bounds_details() {
    let root = temp_file("mindspace-context-query");
    std::fs::create_dir_all(root.join("maps")).expect("maps directory should be writable");
    std::fs::write(
        root.join("maps").join("roadmap.md"),
        "- Roadmap [id:roadmap]\n  - Activation work @owner:maya [id:roadmap/activation]\n    | Activation detail is intentionally long enough to be clipped.\n",
    )
    .expect("roadmap map should be writable");
    std::fs::write(
        root.join("maps").join("risks.md"),
        "- Risks [id:risks]\n  - Activation risk @owner:maya [id:risks/activation]\n    | Another activation detail that should not fully fit.\n",
    )
    .expect("risks map should be writable");

    let output = run_mdm(&[
        "mindspace",
        "context",
        ".",
        "--root",
        root.to_str().unwrap(),
        "--query",
        "@owner:maya",
        "--max-detail-chars",
        "16",
        "--json",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["query"], "@owner:maya");
    assert_eq!(value["data"]["summary"]["branches"], 2);
    assert_eq!(value["data"]["summary"]["files"], 2);

    let branches = value["data"]["branches"]
        .as_array()
        .expect("branches should be an array");
    let files = branches
        .iter()
        .filter_map(|branch| branch["file"].as_str())
        .collect::<Vec<_>>();
    assert!(files.contains(&"maps/roadmap.md"));
    assert!(files.contains(&"maps/risks.md"));
    assert!(branches.iter().any(|branch| {
        branch["node"]["detail_omitted_chars"]
            .as_u64()
            .is_some_and(|count| count > 0)
    }));

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn mindspace_template_list_json_returns_built_in_persona_templates() {
    let output = run_mdm(&["mindspace", "template", "list", "--json"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "mindspace template list");
    assert_eq!(value["format"], "mindspace_template_catalog.v1");
    assert_eq!(value["summary"]["count"], 4);

    let ids = value["data"]["templates"]
        .as_array()
        .expect("templates should be an array")
        .iter()
        .filter_map(|template| template["id"].as_str())
        .collect::<Vec<_>>();
    for expected in [
        "launch-planning",
        "project-memory",
        "story-continuity",
        "claims-evidence",
    ] {
        assert!(
            ids.contains(&expected),
            "template list should include {expected}"
        );
    }
}

#[test]
fn mindspace_template_show_json_returns_full_template_contract() {
    let output = run_mdm(&["mindspace", "template", "show", "launch-planning", "--json"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "mindspace template show");
    assert_eq!(value["format"], "mindspace_template.v1");
    assert_eq!(value["target"], "launch-planning");
    assert_eq!(value["data"]["persona_fit"], "Priya Planner");
    assert!(
        value["data"]["starting_prompt"]
            .as_str()
            .expect("starting prompt should be a string")
            .contains("Keep source material read-only")
    );
    assert!(
        value["data"]["map_shapes"]
            .as_array()
            .expect("map shapes should be an array")
            .iter()
            .any(|shape| shape["path"] == "maps/roadmap.md")
    );
    assert!(
        value["data"]["customization_knobs"]
            .as_array()
            .expect("knobs should be an array")
            .iter()
            .any(|knob| knob["name"] == "write_mode")
    );
}

#[test]
fn mindspace_template_show_prompt_prints_copyable_agent_guidance() {
    let output = run_mdm(&[
        "mindspace",
        "template",
        "show",
        "claims-evidence",
        "--prompt",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).is_empty());
    let stdout = stdout(&output);

    assert!(stdout.contains("Use the claims and evidence template."));
    assert!(stdout.contains("Keep sources read-only"));
    assert!(stdout.contains("Agent workflow:"));
    assert!(stdout.contains("Review in mdmind:"));
}

#[test]
fn mindspace_template_show_unknown_json_returns_error_envelope() {
    let output = run_mdm(&[
        "mindspace",
        "template",
        "show",
        "unknown-template",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["ok"], false);
    assert_eq!(value["command"], "mindspace template show");
    assert_eq!(value["format"], "error.v1");
    assert_eq!(value["target"], "unknown-template");
    assert_eq!(value["error"]["code"], "runtime_error");
    assert!(
        value["error"]["message"]
            .as_str()
            .expect("message should be a string")
            .contains("Unknown Mindspace template")
    );
}

#[test]
fn mindspace_lint_returns_nonzero_for_parser_errors() {
    let root = temp_file("mindspace-lint");
    std::fs::create_dir_all(root.join("maps")).expect("maps directory should be writable");
    std::fs::write(
        root.join("maps").join("broken.md"),
        "- Valid root [id:root]\n   - Bad indentation\n",
    )
    .expect("broken map fixture should be writable");

    let output = run_mdm(&["mindspace", "lint", root.to_str().unwrap(), "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["ok"], false);
    assert_eq!(value["command"], "mindspace lint");
    assert_eq!(value["format"], "mindspace_diagnostics.v1");
    assert_eq!(value["error"]["code"], "mindspace_lint_failed");
    assert_eq!(value["summary"]["errors"], 1);
    assert_eq!(value["data"]["diagnostics"][0]["code"], "map_parse_error");

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn skills_install_prints_the_underlying_npx_command() {
    let output = run_mdm(&["skills", "install", "--print"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).is_empty());
    assert_eq!(stdout(&output), "npx skills add dudash/mdmind\n");
}

#[test]
fn ai_presets_list_nvidia_nim_ollama_codex_local_and_claude_local() {
    let output = run_mdm(&["ai", "presets"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);

    assert!(stdout.contains("nvidia-nim"));
    assert!(stdout.contains("NVIDIA NIM"));
    assert!(stdout.contains("env:NVIDIA_API_KEY"));
    assert!(stdout.contains("codex-local"));
    assert!(stdout.contains("Codex Local"));
    assert!(stdout.contains("claude-local"));
    assert!(stdout.contains("Claude Local"));
    assert!(stdout.contains("ollama-local"));
    assert!(stdout.contains("Ollama Local"));
}

#[test]
fn ai_presets_json_uses_an_envelope() {
    let output = run_mdm(&["ai", "presets", "--json"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "ai presets");
    assert_eq!(value["format"], "ai_quick_add_presets.v1");
    assert!(value["summary"]["count"].as_u64().unwrap() >= 4);
    assert!(
        value["data"]
            .as_array()
            .expect("data should be an array")
            .iter()
            .any(|preset| preset["id"] == "nvidia-nim")
    );
}

#[test]
fn ai_quick_add_writes_nvidia_nim_profile_without_api_key() {
    let config_path = temp_file("ai-profiles.json");
    let output = run_mdm(&[
        "ai",
        "quick-add",
        "nvidia-nim",
        "--default",
        "--config",
        config_path.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output).trim(), config_path.to_string_lossy());

    let contents = std::fs::read_to_string(&config_path).expect("config should be written");
    assert!(contents.contains("\"default_profile\": \"nvidia-nim\""));
    assert!(contents.contains("\"endpoint\": \"https://integrate.api.nvidia.com/v1\""));
    assert!(contents.contains("\"secret_ref\": \"env:NVIDIA_API_KEY\""));
    assert!(contents.contains("\"model\": \"nvidia/llama-3.3-nemotron-super-49b-v1.5\""));
    assert!(contents.contains("\"max_tokens\": 65536"));
    assert!(!contents.contains("nvapi-"));

    std::fs::remove_file(config_path).ok();
}

#[test]
fn ai_quick_add_writes_codex_local_profile() {
    let config_path = temp_file("codex-ai-profiles.json");
    let output = run_mdm(&[
        "ai",
        "quick-add",
        "codex-local",
        "--config",
        config_path.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    let contents = std::fs::read_to_string(&config_path).expect("config should be written");
    assert!(contents.contains("\"id\": \"codex-local\""));
    assert!(contents.contains("\"command\": \"codex\""));
    assert!(contents.contains("\"exec\""));
    assert!(!contents.contains("secret_ref"));

    std::fs::remove_file(config_path).ok();
}

#[test]
fn ai_quick_add_writes_claude_local_profile() {
    let config_path = temp_file("claude-ai-profiles.json");
    let output = run_mdm(&[
        "ai",
        "quick-add",
        "claude-local",
        "--config",
        config_path.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    let contents = std::fs::read_to_string(&config_path).expect("config should be written");
    assert!(contents.contains("\"id\": \"claude-local\""));
    assert!(contents.contains("\"command\": \"claude\""));
    assert!(contents.contains("\"-p\""));
    assert!(!contents.contains("secret_ref"));

    std::fs::remove_file(config_path).ok();
}

#[test]
fn ai_quick_add_writes_ollama_local_profile() {
    let config_path = temp_file("ollama-ai-profiles.json");
    let output = run_mdm(&[
        "ai",
        "quick-add",
        "ollama-local",
        "--default",
        "--config",
        config_path.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));

    let contents = std::fs::read_to_string(&config_path).expect("config should be written");
    assert!(contents.contains("\"default_profile\": \"ollama-local\""));
    assert!(contents.contains("\"adapter_type\": \"local-http\""));
    assert!(contents.contains("\"endpoint\": \"http://127.0.0.1:11434/v1\""));
    assert!(contents.contains("\"model\": \"llama3.2:latest\""));
    assert!(!contents.contains("secret_ref"));

    std::fs::remove_file(config_path).ok();
}

#[test]
fn changelog_prints_latest_curated_entry() {
    let output = run_mdm(&["changelog"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains(&format!("[{}]", env!("CARGO_PKG_VERSION"))));
    assert!(!stdout.contains(&format!("## [{}]", env!("CARGO_PKG_VERSION"))));
    assert!(stdout.contains("Features"));
}

#[test]
fn changelog_can_select_legacy_release() {
    let output = run_mdm(&["changelog", "--version", "0.7.0"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("[0.7.0] - 2026-05-13"));
    assert!(!stdout.contains("## [0.7.0]"));
    assert!(stdout.contains("Metadata Table View"));
}

#[test]
fn changelog_can_select_080_release_notes() {
    let output = run_mdm(&["changelog", "--version", "0.8.0"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("[0.8.0] - 2026-05-23"));
    assert!(!stdout.contains("## [0.8.0]"));
    assert!(stdout.contains("mdm changelog"));
}

#[test]
fn changelog_pretty_renders_markdown_for_humans() {
    let output = run_mdm(&["changelog", "--version", "0.8.0", "--pretty"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("[0.8.0] - 2026-05-23"));
    assert!(stdout.contains("Features"));
    assert!(!stdout.contains("## [0.8.0]"));
}

#[test]
fn changelog_plain_preserves_raw_markdown() {
    let output = run_mdm(&["changelog", "--version", "0.8.0", "--plain"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("## [0.8.0] - 2026-05-23"));
    assert!(stdout.contains("### Features"));
}

#[test]
fn changelog_json_uses_a_success_envelope() {
    let output = run_mdm(&["changelog", "--version", "v0.7.0", "--json"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stderr(&output).is_empty());
    let value = json_stdout(&output);

    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "changelog");
    assert_eq!(value["format"], "changelog_entry.v1");
    assert_eq!(value["target"], "v0.7.0");
    assert_eq!(value["data"]["version"], "0.7.0");
}

#[test]
fn view_markdown_pretty_prints_ordinary_markdown() {
    let markdown_path = temp_file("ordinary.md");
    std::fs::write(
        &markdown_path,
        "---\ntitle: Project Notes\n---\n\n# Project Notes\n\nA [brief](docs/brief.md) with context.\n\n- [x] Drafted\n- Review\n\n| Area | Status |\n| --- | --- |\n| CLI | Done |\n\n```rust\nlet ready = true;\n```\n",
    )
    .expect("markdown fixture should be writable");

    let output = run_mdm(&[
        "view-markdown",
        markdown_path.to_str().unwrap(),
        "--width",
        "48",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("╭─ metadata\n│ title: Project Notes\n╰─"));
    assert!(stdout.contains("Project Notes\n━━━━━━━━━━━━━"));
    assert!(stdout.contains("A brief (docs/brief.md) with context."));
    assert!(stdout.contains("☑ Drafted"));
    assert!(stdout.contains("│ Area │ Status │"));
    assert!(stdout.contains("╭─ code · rust"));
    assert!(stdout.contains("│ let ready = true;"));

    std::fs::remove_file(markdown_path).ok();
}

#[test]
fn view_markdown_plain_prints_raw_markdown() {
    let markdown_path = temp_file("ordinary-plain.md");
    let source = "# Project Notes\n\n- Raw\n";
    std::fs::write(&markdown_path, source).expect("markdown fixture should be writable");

    let output = run_mdm(&["view-markdown", markdown_path.to_str().unwrap(), "--plain"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), source);

    std::fs::remove_file(markdown_path).ok();
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
fn relations_cli_reports_path_qualified_branch_targets() {
    let root = temp_file("mindspace-relations");
    std::fs::create_dir_all(root.join("maps")).expect("temp mindspace should be writable");
    std::fs::write(
        root.join("maps/decisions.md"),
        "- Decision Log [id:decision]\n  - API Shape [id:decision/api-shape]\n",
    )
    .expect("target map should be writable");
    let source = root.join("maps/tasks.md");
    std::fs::write(
        &source,
        "- Research [id:research] [[rel:implements->maps/decisions.md#decision/api-shape]]\n",
    )
    .expect("source map should be writable");

    let relations = run_mdm(&["relations", source.to_str().unwrap(), "--plain"]);
    assert!(relations.status.success(), "stderr: {}", stderr(&relations));
    let relations_stdout = stdout(&relations);
    assert!(relations_stdout.contains("path_qualified_branch"));
    assert!(relations_stdout.contains("maps/decisions.md#decision/api-shape"));

    let validate = run_mdm(&["validate", source.to_str().unwrap()]);
    assert!(validate.status.success(), "stderr: {}", stderr(&validate));

    std::fs::remove_dir_all(root).ok();
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
    let output = run_mdmind(&[&fixture("sample.md"), "--preview", "--max-depth", "1"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("MVP Scope"));
}

#[test]
fn mdmind_preview_renders_ordinary_markdown() {
    let markdown_path = temp_file("README.md");
    std::fs::write(
        &markdown_path,
        "# Project Notes\n\nA normal Markdown document.\n\n```rust\nlet ready = true;\n```\n",
    )
    .expect("markdown fixture should be writable");

    let output = run_mdmind(&["--preview", markdown_path.to_str().unwrap()]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("Project Notes"));
    assert!(stdout.contains("╭─ code · rust"));
    assert!(!stdout.contains("The map contains parser errors"));
    assert!(!sidecar_path(&markdown_path, "session").exists());
    assert!(!sidecar_path(&markdown_path, "ui").exists());
    assert!(!sidecar_path(&markdown_path, "checkpoints").exists());

    std::fs::remove_file(markdown_path).ok();
}

#[test]
fn mdmind_preview_force_map_rejects_readme_markdown() {
    let markdown_path = temp_file("README-force-map.md");
    std::fs::write(
        &markdown_path,
        "# Project Notes\n\nA normal Markdown document.\n",
    )
    .expect("markdown fixture should be writable");

    let output = run_mdmind(&["--as", "map", "--preview", markdown_path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("The map contains parser errors"));

    std::fs::remove_file(markdown_path).ok();
}

#[test]
fn mdmind_preview_near_miss_map_shows_recovery_guidance() {
    let map_path = temp_file("broken-map.md");
    std::fs::write(&map_path, "- Roadmap [id:roadmap]\n  Missing dash\n")
        .expect("map fixture should be writable");

    let output = run_mdmind(&["--preview", map_path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    let error_output = stderr(&output);
    assert!(error_output.contains("damaged mdmind map"));
    assert!(error_output.contains("mdm validate"));
    assert!(error_output.contains("mdmind --as markdown"));

    let forced = run_mdmind(&["--as", "markdown", "--preview", map_path.to_str().unwrap()]);
    assert!(forced.status.success(), "stderr: {}", stderr(&forced));
    assert!(stdout(&forced).contains("Roadmap [id:roadmap]"));

    std::fs::remove_file(map_path).ok();
}

#[test]
fn mdmind_near_miss_map_prints_recovery_guidance_without_a_tty() {
    let map_path = temp_file("broken-map-non-tty.md");
    std::fs::write(&map_path, "- Roadmap [id:roadmap]\n  Missing dash\n")
        .expect("map fixture should be writable");

    let output = run_mdmind(&[map_path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    let error_output = stderr(&output);
    assert!(error_output.contains("damaged mdmind map"));
    assert!(error_output.contains("Recommended:"));
    assert!(error_output.contains("mdm validate"));
    assert!(error_output.contains("mdm import"));
    assert!(!error_output.contains("interactive terminal"));

    std::fs::remove_file(map_path).ok();
}

#[test]
fn mdmind_preview_without_a_target_returns_a_runtime_error() {
    let output = run_mdmind(&["--preview"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("`mdmind --preview` needs a target path."));
}

#[test]
fn mdmind_without_a_target_requires_an_interactive_terminal_for_startup() {
    let output = run_mdmind(&[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains(
        "No target was provided. Run `mdmind path/to/map.md`, or start `mdmind` in an interactive terminal to create one."
    ));
}

#[test]
fn mdmind_key_diagnostics_requires_an_interactive_terminal() {
    let output = run_mdmind(&["--check-keys"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("Key diagnostics need an interactive terminal."));
}

fn sidecar_path(map_path: &Path, suffix: &str) -> PathBuf {
    let file_name = map_path
        .file_name()
        .expect("fixture should have a file name")
        .to_string_lossy();
    map_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".{file_name}.mdmind-{suffix}.json"))
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
fn version_json_prints_the_cli_version_without_network() {
    let output = run_mdm(&["version", "--json"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("version json should parse");

    assert_eq!(value["ok"], true);
    assert_eq!(value["command"], "version");
    assert_eq!(value["format"], "version.v1");
    assert_eq!(value["data"]["current_version"], env!("CARGO_PKG_VERSION"));
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
