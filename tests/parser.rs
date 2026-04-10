use mdmind::parser::parse_document;
use mdmind::query::{
    find_matches, link_entries, metadata_rows, relation_entries, relation_entries_for_anchor,
    tag_counts,
};
use mdmind::validate::validate_document;

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{name}")).expect("fixture should be readable")
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
fn parser_rejects_misaligned_detail_lines() {
    let parsed = parse_document("- Root\n  - Child\n  | This detail is too late.\n");
    assert!(parsed.diagnostics.iter().any(|diagnostic| {
        diagnostic
            .message
            .contains("Detail lines must appear directly under their node")
    }));
}
