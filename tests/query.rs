use mdmind::parser::parse_document;
use mdmind::query::{FilterQuery, find_matches};

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
