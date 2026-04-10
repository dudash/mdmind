use mdmind::locations::{
    FrequentLocation, LocationMemoryAnchor, LocationMemoryState, load_locations_for,
    locations_path_for, save_locations_for,
};

fn temp_map_path(name: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("mdmind-locations-{nonce}-{name}"))
}

#[test]
fn locations_use_a_local_sidecar_next_to_the_map() {
    let root = temp_map_path("locations-root");
    std::fs::create_dir_all(&root).expect("temp root should be creatable");
    let map_path = root.join("roadmap.md");
    let locations_path = locations_path_for(&map_path).expect("locations path should be derivable");
    assert_eq!(
        locations_path.file_name().and_then(|name| name.to_str()),
        Some(".roadmap.md.mdmind-locations.json")
    );
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn locations_round_trip_through_disk() {
    let map_path = temp_map_path("product.md");
    std::fs::write(&map_path, "- Root\n").expect("fixture map should be writable");

    let state = LocationMemoryState {
        frequent: vec![FrequentLocation {
            anchor: LocationMemoryAnchor {
                path: vec![0, 1],
                id: Some("product/tasks".to_string()),
            },
            visits: 4,
            last_seen: 9,
        }],
    };
    save_locations_for(&map_path, &state).expect("locations should write");

    let loaded = load_locations_for(&map_path).expect("locations should load");
    assert_eq!(loaded, state);

    let locations_path = locations_path_for(&map_path).expect("locations path should be derivable");
    if locations_path.exists() {
        std::fs::remove_file(locations_path).ok();
    }
    std::fs::remove_file(map_path).ok();
}
