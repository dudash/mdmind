use mdmind::views::{SavedView, SavedViewsState, load_views_for, save_views_for, views_path_for};

fn temp_map_path(name: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("mdmind-views-{nonce}-{name}"))
}

#[test]
fn saved_views_use_a_local_sidecar_next_to_the_map() {
    let root = temp_map_path("views-root");
    std::fs::create_dir_all(&root).expect("temp root should be creatable");
    let map_path = root.join("roadmap.md");
    let views_path = views_path_for(&map_path).expect("views path should be derivable");
    assert_eq!(
        views_path.file_name().and_then(|name| name.to_str()),
        Some(".roadmap.md.mdmind-views.json")
    );
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn saved_views_round_trip_through_disk() {
    let map_path = temp_map_path("product.md");
    std::fs::write(&map_path, "- Root\n").expect("fixture map should be writable");

    let state = SavedViewsState {
        table_columns: vec![
            "node".to_string(),
            "provider".to_string(),
            "aa_index".to_string(),
        ],
        views: vec![SavedView {
            name: "blocked".to_string(),
            query: "@status:blocked".to_string(),
        }],
    };
    save_views_for(&map_path, &state).expect("saved views should write");

    let loaded = load_views_for(&map_path).expect("saved views should load");
    assert_eq!(loaded, state);

    let views_path = views_path_for(&map_path).expect("views path should be derivable");
    if views_path.exists() {
        std::fs::remove_file(views_path).ok();
    }
    std::fs::remove_file(map_path).ok();
}

#[test]
fn saved_views_load_older_sidecars_without_table_columns() {
    let map_path = temp_map_path("legacy-product.md");
    std::fs::write(&map_path, "- Root\n").expect("fixture map should be writable");
    let views_path = views_path_for(&map_path).expect("views path should be derivable");
    std::fs::write(
        &views_path,
        r#"{
  "views": [
    {
      "name": "active",
      "query": "@status:active"
    }
  ]
}"#,
    )
    .expect("legacy views sidecar should be writable");

    let loaded = load_views_for(&map_path).expect("legacy saved views should load");

    assert!(loaded.table_columns.is_empty());
    assert_eq!(loaded.views.len(), 1);

    if views_path.exists() {
        std::fs::remove_file(views_path).ok();
    }
    std::fs::remove_file(map_path).ok();
}

#[test]
fn saved_views_load_column_only_sidecars_without_named_views() {
    let map_path = temp_map_path("column-only-product.md");
    std::fs::write(&map_path, "- Root\n").expect("fixture map should be writable");
    let views_path = views_path_for(&map_path).expect("views path should be derivable");
    std::fs::write(
        &views_path,
        r#"{
  "table_columns": ["node", "owner", "status"]
}"#,
    )
    .expect("column-only views sidecar should be writable");

    let loaded = load_views_for(&map_path).expect("column-only saved views should load");

    assert_eq!(
        loaded.table_columns,
        vec![
            "node".to_string(),
            "owner".to_string(),
            "status".to_string()
        ]
    );
    assert!(loaded.views.is_empty());

    if views_path.exists() {
        std::fs::remove_file(views_path).ok();
    }
    std::fs::remove_file(map_path).ok();
}
