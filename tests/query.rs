use mdmind::parser::parse_document;
use mdmind::query::{
    FilterQuery, find_matches, metadata_key_counts_for_filter, metadata_value_counts_for_filter,
    tag_counts_for_filter,
};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{name}")).expect("fixture should be readable")
}

#[test]
fn filter_query_supports_multi_term_tag_and_metadata_search() {
    let parsed = parse_document(&fixture("sample.md"));
    let query = FilterQuery::parse("#prompt @owner:jason").expect("query should parse");

    let root = &parsed.document.nodes[0];
    assert!(!query.matches(root));

    let prompt_library = &root.children[1];
    assert!(query.matches(prompt_library));
}

#[test]
fn find_matches_uses_the_shared_query_language() {
    let parsed = parse_document(&fixture("sample.md"));
    let matches = find_matches(&parsed.document, "#todo @status:active");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].text, "MVP Scope");
}

#[test]
fn find_matches_and_filter_queries_can_match_node_details() {
    let parsed = parse_document(
        "- Launch Readiness [id:launch]\n  | Partner auth still depends on the same token model.\n  - Notes\n",
    );
    let matches = find_matches(&parsed.document, "token model");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].text, "Launch Readiness");
    assert_eq!(
        matches[0].detail_snippet.as_deref(),
        Some("Partner auth still depends on the same token model.")
    );

    let query = FilterQuery::parse("token model").expect("query should parse");
    assert!(query.matches(&parsed.document.nodes[0]));
}

#[test]
fn facet_counts_can_be_scoped_by_the_active_filter() {
    let parsed = parse_document(&fixture("sample.md"));
    let filter = FilterQuery::parse("#prompt").expect("query should parse");

    let tags = tag_counts_for_filter(&parsed.document, Some(&filter));
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].tag, "#prompt");

    let keys = metadata_key_counts_for_filter(&parsed.document, Some(&filter));
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].key, "owner");

    let values = metadata_value_counts_for_filter(&parsed.document, Some(&filter));
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].key, "owner");
    assert_eq!(values[0].value, "jason");
}
