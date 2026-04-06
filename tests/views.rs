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
