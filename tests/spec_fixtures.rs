use std::fs;
use std::path::PathBuf;

use mdmind::model::{Document, Node, Severity};
use mdmind::parser::parse_document;
use mdmind::validate::validate_document;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ManifestCase {
    name: String,
    input: String,
    outcome: Outcome,
    #[serde(default)]
    diagnostics: Vec<ExpectedDiagnostic>,
    #[serde(default)]
    root_count: Option<usize>,
    #[serde(default)]
    ids: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    metadata: std::collections::BTreeMap<String, Vec<String>>,
    #[serde(default)]
    detail_lines: Vec<String>,
    #[serde(default)]
    relations: Vec<ExpectedRelation>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum Outcome {
    Valid,
    Warning,
    Invalid,
}

#[derive(Debug, Deserialize)]
struct ExpectedDiagnostic {
    severity: ExpectedSeverity,
    contains: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct ExpectedRelation {
    source: String,
    kind: Option<String>,
    target: String,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum ExpectedSeverity {
    Error,
    Warning,
}

fn spec_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spec")
}

#[test]
fn spec_manifest_fixtures_match_expected_diagnostics() {
    let manifest_path = spec_root().join("tests.json");
    let manifest = fs::read_to_string(&manifest_path).expect("spec manifest should be readable");
    let cases: Vec<ManifestCase> =
        serde_json::from_str(&manifest).expect("spec manifest should be valid json");

    assert!(
        !cases.is_empty(),
        "spec manifest should include at least one fixture"
    );

    for case in cases {
        let source_path = spec_root().join(&case.input);
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|error| panic!("{} should be readable: {error}", case.input));
        let parsed = parse_document(&source);
        let mut diagnostics = parsed.diagnostics.clone();
        diagnostics.extend(validate_document(&parsed.document));

        match case.outcome {
            Outcome::Valid => {
                assert!(
                    diagnostics.is_empty(),
                    "{} should be valid, got diagnostics: {:?}",
                    case.name,
                    diagnostics
                );
                assert_expected_structure(&case, &parsed.document);
            }
            Outcome::Warning => {
                assert!(
                    diagnostics
                        .iter()
                        .all(|diagnostic| diagnostic.severity == Severity::Warning),
                    "{} should only produce warnings, got diagnostics: {:?}",
                    case.name,
                    diagnostics
                );
                assert_expected_diagnostics(&case, &diagnostics);
                assert_expected_structure(&case, &parsed.document);
            }
            Outcome::Invalid => {
                assert!(
                    diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.severity == Severity::Error),
                    "{} should produce at least one error, got diagnostics: {:?}",
                    case.name,
                    diagnostics
                );
                assert_expected_diagnostics(&case, &diagnostics);
            }
        }
    }
}

fn assert_expected_structure(case: &ManifestCase, document: &Document) {
    if let Some(root_count) = case.root_count {
        assert_eq!(
            document.nodes.len(),
            root_count,
            "{} root count mismatch",
            case.name
        );
    }

    if !case.ids.is_empty() {
        let mut actual = Vec::new();
        walk_nodes(&document.nodes, &mut |node| {
            if let Some(id) = &node.id {
                actual.push(id.clone());
            }
        });
        assert_eq!(actual, case.ids, "{} id list mismatch", case.name);
    }

    if !case.tags.is_empty() {
        let mut actual = Vec::new();
        walk_nodes(&document.nodes, &mut |node| {
            actual.extend(node.tags.iter().cloned());
        });
        assert_eq!(actual, case.tags, "{} tag list mismatch", case.name);
    }

    if !case.metadata.is_empty() {
        let mut actual = std::collections::BTreeMap::<String, Vec<String>>::new();
        walk_nodes(&document.nodes, &mut |node| {
            for entry in &node.metadata {
                actual
                    .entry(entry.key.clone())
                    .or_default()
                    .push(entry.value.clone());
            }
        });
        assert_eq!(actual, case.metadata, "{} metadata mismatch", case.name);
    }

    if !case.detail_lines.is_empty() {
        let mut actual = Vec::new();
        walk_nodes(&document.nodes, &mut |node| {
            actual.extend(node.detail.iter().cloned());
        });
        assert_eq!(actual, case.detail_lines, "{} detail mismatch", case.name);
    }

    if !case.relations.is_empty() {
        let mut actual = Vec::new();
        walk_nodes(&document.nodes, &mut |node| {
            if let Some(source) = &node.id {
                actual.extend(node.relations.iter().map(|relation| ExpectedRelation {
                    source: source.clone(),
                    kind: relation.kind.clone(),
                    target: relation.target.clone(),
                }));
            }
        });
        assert_eq!(actual, case.relations, "{} relation mismatch", case.name);
    }
}

fn assert_expected_diagnostics(case: &ManifestCase, diagnostics: &[mdmind::model::Diagnostic]) {
    for expected in &case.diagnostics {
        let severity = match expected.severity {
            ExpectedSeverity::Error => Severity::Error,
            ExpectedSeverity::Warning => Severity::Warning,
        };
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.severity == severity && diagnostic.message.contains(&expected.contains)
            }),
            "{} missing expected diagnostic {:?} containing {:?}; got {:?}",
            case.name,
            severity,
            expected.contains,
            diagnostics
        );
    }
}

fn walk_nodes<F>(nodes: &[Node], visitor: &mut F)
where
    F: FnMut(&Node),
{
    for node in nodes {
        visitor(node);
        walk_nodes(&node.children, visitor);
    }
}
