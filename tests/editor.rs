use mdmind::editor::{Editor, default_focus_path};
use mdmind::parser::parse_document;
use mdmind::serializer::serialize_document;
use mdmind::session::{SessionState, resolve_session_focus};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{name}")).expect("fixture should be readable")
}

#[test]
fn serializer_round_trips_the_document() {
    let parsed = parse_document(&fixture("sample.md"));
    let source = serialize_document(&parsed.document);
    let reparsed = parse_document(&source);

    assert!(reparsed.diagnostics.is_empty());
    assert_eq!(parsed.document.export(), reparsed.document.export());
}

#[test]
fn editor_can_add_and_edit_nodes() {
    let parsed = parse_document(&fixture("sample.md"));
    let focus_path = default_focus_path(&parsed.document);
    let mut editor = Editor::new(parsed.document, focus_path);

    editor.move_child(1).expect("root child should exist");
    editor
        .add_child("Validation #todo @status:next [id:product/mvp/validation]")
        .expect("should add child");
    editor
        .edit_current("Validation Pass #todo @status:active [id:product/mvp/validation]")
        .expect("should edit current node");

    let current = editor.current().expect("focus should exist");
    assert_eq!(current.text, "Validation Pass");
    assert_eq!(current.id.as_deref(), Some("product/mvp/validation"));
    assert_eq!(current.metadata[0].value, "active");
}

#[test]
fn editor_rejects_duplicate_ids() {
    let parsed = parse_document(&fixture("sample.md"));
    let mut editor = Editor::new(parsed.document, vec![0]);

    let error = editor
        .add_child("Broken Copy [id:product/mvp]")
        .expect_err("duplicate ids should be rejected");
    assert!(error.message().contains("Duplicate id"));
}

#[test]
fn editor_can_reorder_and_reparent_nodes() {
    let parsed = parse_document(&fixture("sample.md"));
    let mut editor = Editor::new(parsed.document, vec![0, 1]);

    editor
        .move_node_up()
        .expect("should move up among siblings");
    assert_eq!(editor.focus_path(), &[0, 0]);
    assert_eq!(
        editor.current().expect("focus should exist").text,
        "Prompt Library"
    );

    editor.outdent_node().expect("should outdent to root");
    assert_eq!(editor.focus_path(), &[1]);
    assert_eq!(
        editor.current().expect("focus should exist").text,
        "Prompt Library"
    );

    editor
        .move_node_up()
        .expect("should move to the first root position");
    assert_eq!(editor.focus_path(), &[0]);

    editor
        .move_node_down()
        .expect("should move back down among root siblings");
    assert_eq!(editor.focus_path(), &[1]);

    editor
        .indent_node()
        .expect("should indent into previous sibling");
    assert_eq!(editor.focus_path(), &[0, 1]);
    assert_eq!(
        editor.current().expect("focus should exist").text,
        "Prompt Library"
    );
}

#[test]
fn session_restore_prefers_id_and_falls_back_to_path() {
    let parsed = parse_document(&fixture("sample.md"));
    let by_id = resolve_session_focus(
        &parsed.document,
        &SessionState {
            focus_path: vec![0, 0, 1],
            focus_id: Some("prompts/library".to_string()),
        },
    )
    .expect("id should resolve");
    assert_eq!(by_id, vec![0, 1]);

    let by_path = resolve_session_focus(
        &parsed.document,
        &SessionState {
            focus_path: vec![0, 0],
            focus_id: Some("missing/id".to_string()),
        },
    )
    .expect("path should resolve");
    assert_eq!(by_path, vec![0, 0]);
}
