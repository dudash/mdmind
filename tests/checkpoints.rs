use mdmind::checkpoints::{
    Checkpoint, CheckpointAnchor, CheckpointViewMode, CheckpointsState, checkpoints_path_for,
    load_checkpoints_for, save_checkpoints_for,
};
use mdmind::parser::parse_document;

fn temp_map_path(name: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("mdmind-checkpoints-{nonce}-{name}"))
}

#[test]
fn checkpoints_use_a_local_sidecar_next_to_the_map() {
    let root = temp_map_path("checkpoints-root");
    std::fs::create_dir_all(&root).expect("temp root should be creatable");
    let map_path = root.join("roadmap.md");
    let checkpoints_path =
        checkpoints_path_for(&map_path).expect("checkpoints path should be derivable");
    assert_eq!(
        checkpoints_path.file_name().and_then(|name| name.to_str()),
        Some(".roadmap.md.mdmind-checkpoints.json")
    );
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn checkpoints_round_trip_through_disk() {
    let map_path = temp_map_path("product.md");
    std::fs::write(&map_path, "- Root\n").expect("fixture map should be writable");
    let document =
        parse_document("- Root\n  - Child #todo @status:active [id:root/child]\n").document;
    let checkpoints = CheckpointsState {
        checkpoints: vec![Checkpoint {
            name: "Before delete: Child".to_string(),
            document,
            focus_path: vec![0, 0],
            dirty: true,
            expanded_paths: vec![vec![0]],
            view_mode: CheckpointViewMode::SubtreeOnly,
            subtree_root: Some(CheckpointAnchor {
                path: vec![0],
                id: Some("root".to_string()),
            }),
            filter_query: Some("#todo".to_string()),
        }],
    };

    save_checkpoints_for(&map_path, &checkpoints).expect("checkpoints should write");

    let loaded = load_checkpoints_for(&map_path).expect("checkpoints should load");
    assert_eq!(loaded, checkpoints);

    let checkpoints_path =
        checkpoints_path_for(&map_path).expect("checkpoints path should be derivable");
    if checkpoints_path.exists() {
        std::fs::remove_file(checkpoints_path).ok();
    }
    std::fs::remove_file(map_path).ok();
}
