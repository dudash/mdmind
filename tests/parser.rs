use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use mdmind::model::{ExternalRefKind, RelationTarget, TaskState};
use mdmind::parser::parse_document;
use mdmind::query::{
    find_matches, link_entries, metadata_rows, reference_entries, relation_entries,
    relation_entries_for_anchor, tag_counts,
};
use mdmind::validate::{validate_document, validate_document_with_base_path};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{name}")).expect("fixture should be readable")
}

fn temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("mdmind-{nonce}-{name}"));
    std::fs::create_dir_all(&path).expect("temp dir should be writable");
    path
}

#[test]
fn parser_extracts_tree_annotations_and_ids() {
    let parsed = parse_document(&fixture("sample.md"));
    assert!(
        parsed.diagnostics.is_empty(),
        "parser diagnostics: {:?}",
        parsed.diagnostics
    );

    let document = parsed.document;
    assert_eq!(document.nodes.len(), 1);
    let root = &document.nodes[0];
    assert_eq!(root.text, "Product Idea");
    assert_eq!(root.tags, vec!["#idea"]);
    assert_eq!(root.id.as_deref(), Some("product"));
    assert_eq!(root.children.len(), 2);

    let scope = &root.children[0];
    assert_eq!(scope.text, "MVP Scope");
    assert_eq!(scope.tags, vec!["#todo"]);
    assert_eq!(scope.metadata[0].key, "status");
    assert_eq!(scope.metadata[0].value, "active");
    assert_eq!(scope.id.as_deref(), Some("product/mvp"));
}

#[test]
fn parser_attaches_detail_lines_to_the_previous_node() {
    let parsed = parse_document(
        "- API Design #backend [id:product/api-design]\n  | We need one stable auth flow before launch.\n  |\n  | Open question: should refresh tokens be per workspace?\n  - Auth Flow\n",
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "parser diagnostics: {:?}",
        parsed.diagnostics
    );

    let node = &parsed.document.nodes[0];
    assert_eq!(
        node.detail,
        vec![
            "We need one stable auth flow before launch.".to_string(),
            String::new(),
            "Open question: should refresh tokens be per workspace?".to_string(),
        ]
    );
    assert_eq!(node.children[0].text, "Auth Flow");
}

#[test]
fn parser_extracts_explicit_task_markers() {
    let parsed = parse_document(
        "- Project\n  - [ ] Open item #todo @status:active [id:project/open]\n  - [x] Done item #done @status:done [id:project/done]\n",
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "parser diagnostics: {:?}",
        parsed.diagnostics
    );

    let open = &parsed.document.nodes[0].children[0];
    assert_eq!(open.task, Some(TaskState::Open));
    assert_eq!(open.text, "Open item");
    assert_eq!(open.tags, vec!["#todo"]);

    let done = &parsed.document.nodes[0].children[1];
    assert_eq!(done.task, Some(TaskState::Done));
    assert_eq!(done.text, "Done item");
    assert_eq!(done.tags, vec!["#done"]);

    let progress = parsed.document.nodes[0].child_task_progress();
    assert_eq!(progress.total, 2);
    assert_eq!(progress.done, 1);
}

#[test]
fn query_helpers_cover_tags_metadata_links_and_text() {
    let parsed = parse_document(&fixture("sample.md"));
    let document = parsed.document;

    let matches = find_matches(&document, "prompt");
    assert_eq!(matches.len(), 2);
    assert!(
        matches
            .iter()
            .any(|entry| entry.breadcrumb.contains("Prompt Library"))
    );
    assert!(
        matches
            .iter()
            .any(|entry| entry.text.contains("System prompts"))
    );

    let tags = tag_counts(&document);
    assert_eq!(tags[0].tag, "#idea");
    assert_eq!(tags.len(), 3);

    let metadata = metadata_rows(&document, &[String::from("owner")]);
    assert_eq!(metadata.len(), 1);
    assert_eq!(metadata[0].value, "jason");

    let links = link_entries(&document);
    assert_eq!(links.len(), 3);
    assert_eq!(links[1].id, "product/mvp");
}

#[test]
fn validate_reports_parser_and_duplicate_id_problems() {
    let parsed = parse_document(&fixture("invalid.md"));
    assert!(
        parsed.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("Indentation must use multiples of two spaces")),
        "expected a structural indentation error"
    );

    let diagnostics = validate_document(&parsed.document);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Duplicate id")),
        "expected duplicate id validation diagnostics, got: {:?}",
        diagnostics
    );
}

#[test]
fn parser_extracts_inline_relations_and_backlinks_are_queryable() {
    let parsed = parse_document(&fixture("relations.md"));
    assert!(
        parsed.diagnostics.is_empty(),
        "parser diagnostics: {:?}",
        parsed.diagnostics
    );

    let document = parsed.document;
    let mvp = &document.nodes[0].children[0];
    assert_eq!(mvp.relations.len(), 2);
    assert_eq!(mvp.relations[0].kind, None);
    assert_eq!(mvp.relations[0].target, "prompts/library");
    assert_eq!(mvp.relations[1].kind.as_deref(), Some("supports"));
    assert_eq!(mvp.relations[1].target, "product/requirements");

    let rows = relation_entries(&document);
    assert_eq!(rows.len(), 3);
    assert!(
        rows.iter()
            .any(|row| row.relation == "supports" && row.target == "product/requirements")
    );

    let anchor_rows = relation_entries_for_anchor(&document, "product/mvp");
    assert!(
        anchor_rows
            .iter()
            .any(|row| row.direction == mdmind::model::RelationDirection::Incoming)
    );
    assert!(
        anchor_rows
            .iter()
            .any(|row| row.direction == mdmind::model::RelationDirection::Outgoing)
    );
}

#[test]
fn parser_classifies_path_qualified_branch_relations() {
    let parsed = parse_document(
        "- Research [id:research] [[maps/decisions.md#decision/api-shape]] [[rel:implements->maps/tasks.md#todo/focus]]\n",
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "parser diagnostics: {:?}",
        parsed.diagnostics
    );

    let node = &parsed.document.nodes[0];
    assert_eq!(node.relations.len(), 2);
    assert_eq!(
        node.relations[0].target_kind(),
        RelationTarget::PathQualifiedBranch {
            path: "maps/decisions.md",
            id: "decision/api-shape"
        }
    );
    assert_eq!(node.relations[1].kind.as_deref(), Some("implements"));
    assert_eq!(
        node.relations[1].target_kind().label(),
        "path_qualified_branch"
    );

    let rows = relation_entries(&parsed.document);
    assert_eq!(rows[0].target_kind, "path_qualified_branch");
    assert_eq!(rows[0].target, "maps/decisions.md#decision/api-shape");
    assert!(rows[0].resolved_path.is_none());
}

#[test]
fn parser_extracts_markdown_file_and_image_references() {
    let parsed = parse_document(
        "- Research [brief](docs/brief.md) ![diagram](assets/diagram.png) [id:research]\n",
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "parser diagnostics: {:?}",
        parsed.diagnostics
    );

    let node = &parsed.document.nodes[0];
    assert_eq!(node.text, "Research");
    assert_eq!(node.references.len(), 2);
    assert_eq!(node.references[0].label, "brief");
    assert_eq!(node.references[0].target, "docs/brief.md");
    assert_eq!(node.references[0].kind, ExternalRefKind::Link);
    assert_eq!(node.references[1].label, "diagram");
    assert_eq!(node.references[1].target, "assets/diagram.png");
    assert_eq!(node.references[1].kind, ExternalRefKind::Image);
    assert_eq!(
        node.display_line(),
        "Research [id:research] [brief](docs/brief.md) ![diagram](assets/diagram.png)"
    );

    let rows = reference_entries(&parsed.document);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].breadcrumb, "Research");

    let matches = find_matches(&parsed.document, "diagram.png");
    assert_eq!(matches.len(), 1);
}

#[test]
fn parser_extracts_markdown_references_with_spaces() {
    let parsed = parse_document(
        "- Research [brief note](docs/project brief.md) ![flow diagram](assets/flow chart.png) #tag [id:research]\n",
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "parser diagnostics: {:?}",
        parsed.diagnostics
    );

    let node = &parsed.document.nodes[0];
    assert_eq!(node.text, "Research");
    assert_eq!(node.tags, vec!["#tag"]);
    assert_eq!(node.references.len(), 2);
    assert_eq!(node.references[0].label, "brief note");
    assert_eq!(node.references[0].target, "docs/project brief.md");
    assert_eq!(node.references[0].kind, ExternalRefKind::Link);
    assert_eq!(node.references[1].label, "flow diagram");
    assert_eq!(node.references[1].target, "assets/flow chart.png");
    assert_eq!(node.references[1].kind, ExternalRefKind::Image);
    assert_eq!(
        node.display_line(),
        "Research #tag [id:research] [brief note](docs/project brief.md) ![flow diagram](assets/flow chart.png)"
    );

    let rows = reference_entries(&parsed.document);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].label, "brief note");
    assert_eq!(rows[0].target, "docs/project brief.md");

    let matches = find_matches(&parsed.document, "project brief");
    assert_eq!(matches.len(), 1);
}

#[test]
fn validate_reports_unresolved_relation_targets() {
    let parsed = parse_document("- Root [id:root] [[missing/target]]\n");
    let diagnostics = validate_document(&parsed.document);
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("Relation target 'missing/target'")),
        "expected unresolved relation diagnostic, got: {:?}",
        diagnostics
    );
}

#[test]
fn dotted_same_file_relation_ids_stay_file_scoped() {
    let parsed = parse_document("- Release [id:release/v1.0]\n- Note [[release/v1.0]]\n");
    let diagnostics = validate_document(&parsed.document);

    assert!(
        diagnostics.is_empty(),
        "dotted same-file ids should not be treated as external files: {:?}",
        diagnostics
    );
}

#[test]
fn validate_accepts_existing_path_qualified_branch_relation_targets() {
    let root = temp_dir("cross-file-relations");
    let maps_dir = root.join("maps");
    std::fs::create_dir_all(&maps_dir).expect("maps dir should be writable");
    std::fs::write(
        maps_dir.join("decisions.md"),
        "- Decision Log [id:decision]\n  - API Shape [id:decision/api-shape]\n",
    )
    .expect("target map should be writable");

    let parsed = parse_document(
        "- Research [id:research] [[maps/decisions.md#decision/api-shape]] [[rel:implements->maps/decisions.md#decision/api-shape]]\n",
    );
    let diagnostics = validate_document_with_base_path(&parsed.document, Some(&root));

    assert!(
        diagnostics.is_empty(),
        "expected valid cross-file relation target, got: {:?}",
        diagnostics
    );

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn validate_reports_missing_path_qualified_branch_ids() {
    let root = temp_dir("missing-cross-file-id");
    let maps_dir = root.join("maps");
    std::fs::create_dir_all(&maps_dir).expect("maps dir should be writable");
    std::fs::write(
        maps_dir.join("decisions.md"),
        "- Decision Log [id:decision]\n  - Other Shape [id:decision/other]\n",
    )
    .expect("target map should be writable");

    let parsed = parse_document("- Research [[maps/decisions.md#decision/api-shape]]\n");
    let diagnostics = validate_document_with_base_path(&parsed.document, Some(&root));

    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("maps/decisions.md#decision/api-shape")),
        "expected missing cross-file branch id diagnostic, got: {:?}",
        diagnostics
    );

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn validate_reports_missing_path_qualified_relation_files() {
    let root = temp_dir("missing-cross-file");
    let parsed = parse_document("- Research [[maps/decisions.md#decision/api-shape]]\n");
    let diagnostics = validate_document_with_base_path(&parsed.document, Some(&root));

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("maps/decisions.md")),
        "expected missing cross-file relation target diagnostic, got: {:?}",
        diagnostics
    );

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn validate_reports_missing_local_reference_targets_when_base_path_is_known() {
    let parsed = parse_document("- Research [brief](missing.md)\n");
    let diagnostics =
        validate_document_with_base_path(&parsed.document, Some(std::path::Path::new(".")));
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Reference target 'missing.md'")),
        "expected missing reference diagnostic, got: {:?}",
        diagnostics
    );
}

#[test]
fn validate_reports_conflicting_task_state_warnings() {
    let parsed = parse_document(
        "- Project\n  - [x] Active conflict #todo @status:active\n  - [ ] Done conflict #done @done:true\n  - Tag conflict #todo #done\n",
    );
    let diagnostics = validate_document(&parsed.document);

    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("[x] appears with open task metadata")),
        "expected checkbox done/open metadata conflict, got: {:?}",
        diagnostics
    );
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("[ ] appears with done task metadata")),
        "expected checkbox open/done metadata conflict, got: {:?}",
        diagnostics
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("both #todo and #done")),
        "expected tag conflict, got: {:?}",
        diagnostics
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity == mdmind::model::Severity::Warning),
        "task conflicts should be warnings, got: {:?}",
        diagnostics
    );
}

#[test]
fn parser_rejects_misaligned_detail_lines() {
    let parsed = parse_document("- Root\n  - Child\n  | This detail is too late.\n");
    assert!(parsed.diagnostics.iter().any(|diagnostic| {
        diagnostic
            .message
            .contains("Detail lines must appear directly under their node")
    }));
}
