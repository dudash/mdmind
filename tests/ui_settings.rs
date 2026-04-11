use mdmind::ui_settings::{
    ThemeId, UiSettings, load_ui_settings_for, save_ui_settings_for, ui_settings_path_for,
};

fn temp_map_path(name: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("mdmind-ui-{nonce}-{name}"))
}

#[test]
fn ui_settings_use_a_local_sidecar_next_to_the_map() {
    let root = temp_map_path("ui-root");
    std::fs::create_dir_all(&root).expect("temp root should be creatable");
    let map_path = root.join("roadmap.md");
    let settings_path =
        ui_settings_path_for(&map_path).expect("ui settings path should be derivable");
    assert_eq!(
        settings_path.file_name().and_then(|name| name.to_str()),
        Some(".roadmap.md.mdmind-ui.json")
    );
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn ui_settings_round_trip_through_disk() {
    let map_path = temp_map_path("product.md");
    std::fs::write(&map_path, "- Root\n").expect("fixture map should be writable");

    let settings = UiSettings {
        theme: ThemeId::Blueprint,
        motion_enabled: true,
        ascii_accents: false,
        minimal_mode: true,
        reading_mode: true,
    };
    save_ui_settings_for(&map_path, &settings).expect("ui settings should write");

    let loaded = load_ui_settings_for(&map_path).expect("ui settings should load");
    assert_eq!(loaded, settings);

    let settings_path =
        ui_settings_path_for(&map_path).expect("ui settings path should be derivable");
    if settings_path.exists() {
        std::fs::remove_file(settings_path).ok();
    }
    std::fs::remove_file(map_path).ok();
}

#[test]
fn ui_settings_load_older_sidecars_without_minimal_mode() {
    let map_path = temp_map_path("older-product.md");
    std::fs::write(&map_path, "- Root\n").expect("fixture map should be writable");
    let settings_path =
        ui_settings_path_for(&map_path).expect("ui settings path should be derivable");
    std::fs::write(
        &settings_path,
        r#"{
  "theme": "paper",
  "motion_enabled": true,
  "ascii_accents": true
}"#,
    )
    .expect("older ui settings should write");

    let loaded = load_ui_settings_for(&map_path).expect("older settings should still load");
    assert_eq!(loaded.theme, ThemeId::Paper);
    assert!(loaded.motion_enabled);
    assert!(loaded.ascii_accents);
    assert!(!loaded.minimal_mode);
    assert!(!loaded.reading_mode);

    std::fs::remove_file(settings_path).ok();
    std::fs::remove_file(map_path).ok();
}

#[test]
fn ui_settings_load_reduced_motion_sidecars() {
    let map_path = temp_map_path("reduced-motion.md");
    std::fs::write(&map_path, "- Root\n").expect("fixture map should be writable");
    let settings_path =
        ui_settings_path_for(&map_path).expect("ui settings path should be derivable");
    std::fs::write(
        &settings_path,
        r#"{
  "theme": "monograph",
  "reduced_motion": true,
  "ascii_accents": false
}"#,
    )
    .expect("reduced-motion ui settings should write");

    let loaded =
        load_ui_settings_for(&map_path).expect("reduced-motion settings should still load");
    assert_eq!(loaded.theme, ThemeId::Monograph);
    assert!(!loaded.motion_enabled);
    assert!(!loaded.ascii_accents);
    assert!(!loaded.minimal_mode);
    assert!(!loaded.reading_mode);

    std::fs::remove_file(settings_path).ok();
    std::fs::remove_file(map_path).ok();
}
