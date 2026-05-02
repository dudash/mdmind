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

#[test]
fn ui_settings_load_amethyst_theme_sidecars() {
    let map_path = temp_map_path("amethyst-theme.md");
    std::fs::write(&map_path, "- Root\n").expect("fixture map should be writable");
    let settings_path =
        ui_settings_path_for(&map_path).expect("ui settings path should be derivable");
    std::fs::write(
        &settings_path,
        r#"{
  "theme": "amethyst",
  "motion_enabled": true,
  "ascii_accents": false
}"#,
    )
    .expect("amethyst ui settings should write");

    let loaded = load_ui_settings_for(&map_path).expect("amethyst settings should load");
    assert_eq!(loaded.theme, ThemeId::Amethyst);
    assert_eq!(ThemeId::Amethyst.label(), "Amethyst");
    assert!(ThemeId::Amethyst.summary().contains("sea-glass"));

    std::fs::remove_file(settings_path).ok();
    std::fs::remove_file(map_path).ok();
}

#[test]
fn ui_settings_load_atelier_theme_sidecars() {
    let map_path = temp_map_path("atelier-theme.md");
    std::fs::write(&map_path, "- Root\n").expect("fixture map should be writable");
    let settings_path =
        ui_settings_path_for(&map_path).expect("ui settings path should be derivable");
    std::fs::write(
        &settings_path,
        r#"{
  "theme": "atelier",
  "motion_enabled": true,
  "ascii_accents": false
}"#,
    )
    .expect("atelier ui settings should write");

    let loaded = load_ui_settings_for(&map_path).expect("atelier settings should load");
    assert_eq!(loaded.theme, ThemeId::Atelier);
    assert_eq!(ThemeId::Atelier.label(), "Atelier");
    assert!(ThemeId::Atelier.summary().contains("tape-and-clay"));

    std::fs::remove_file(settings_path).ok();
    std::fs::remove_file(map_path).ok();
}

#[test]
fn ui_settings_load_archive_theme_sidecars() {
    let map_path = temp_map_path("archive-theme.md");
    std::fs::write(&map_path, "- Root\n").expect("fixture map should be writable");
    let settings_path =
        ui_settings_path_for(&map_path).expect("ui settings path should be derivable");
    std::fs::write(
        &settings_path,
        r#"{
  "theme": "archive",
  "motion_enabled": true,
  "ascii_accents": false
}"#,
    )
    .expect("archive ui settings should write");

    let loaded = load_ui_settings_for(&map_path).expect("archive settings should load");
    assert_eq!(loaded.theme, ThemeId::Archive);
    assert_eq!(ThemeId::Archive.label(), "Archive");
    assert!(ThemeId::Archive.summary().contains("walnut depth"));

    std::fs::remove_file(settings_path).ok();
    std::fs::remove_file(map_path).ok();
}

#[test]
fn ui_settings_load_signal_theme_sidecars() {
    let map_path = temp_map_path("signal-theme.md");
    std::fs::write(&map_path, "- Root\n").expect("fixture map should be writable");
    let settings_path =
        ui_settings_path_for(&map_path).expect("ui settings path should be derivable");
    std::fs::write(
        &settings_path,
        r#"{
  "theme": "signal",
  "motion_enabled": true,
  "ascii_accents": false
}"#,
    )
    .expect("signal ui settings should write");

    let loaded = load_ui_settings_for(&map_path).expect("signal settings should load");
    assert_eq!(loaded.theme, ThemeId::Signal);
    assert_eq!(ThemeId::Signal.label(), "Signal");
    assert!(ThemeId::Signal.summary().contains("control-room"));

    std::fs::remove_file(settings_path).ok();
    std::fs::remove_file(map_path).ok();
}

#[test]
fn ui_settings_load_tokyo_mind_theme_sidecars() {
    let map_path = temp_map_path("tokyo-mind-theme.md");
    std::fs::write(&map_path, "- Root\n").expect("fixture map should be writable");
    let settings_path =
        ui_settings_path_for(&map_path).expect("ui settings path should be derivable");
    std::fs::write(
        &settings_path,
        r#"{
  "theme": "tokyo-mind",
  "motion_enabled": true,
  "ascii_accents": false
}"#,
    )
    .expect("tokyo mind ui settings should write");

    let loaded = load_ui_settings_for(&map_path).expect("tokyo mind settings should load");
    assert_eq!(loaded.theme, ThemeId::TokyoMind);
    assert_eq!(ThemeId::TokyoMind.label(), "Tokyo Mind");
    assert!(
        ThemeId::TokyoMind
            .summary()
            .contains("Tokyo Night-inspired")
    );

    std::fs::remove_file(settings_path).ok();
    std::fs::remove_file(map_path).ok();
}
