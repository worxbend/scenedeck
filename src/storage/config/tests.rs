use super::*;

#[test]
fn partial_legacy_config_cannot_rebind_registry_by_editing_its_endpoint() {
    let (mut config, _) = load_from_temp_file(r#"{"obs":{"host":"original.local","port":4456}}"#);
    let mut profile = config.connection_profiles()[0].clone();
    profile.obs.host = "replacement.local".into();
    assert!(config.save_connection(profile));
    assert_eq!(config.obs.host, "replacement.local");
    assert_eq!(
        config.legacy_registry_endpoint.as_deref(),
        Some("original.local:4456")
    );
    assert!(!apply_migrations(&mut config));
}

#[test]
fn ordinary_config_save_materializes_profiles_before_connection_reads() {
    let path = std::env::temp_dir().join(format!(
        "scenedeck-profile-save-{}.json",
        std::process::id()
    ));
    let config = AppConfig {
        obs: ObsConfig {
            host: "studio.local".into(),
            port: 4456,
        },
        ..Default::default()
    };
    assert!(config.connections.is_empty());
    write_config_to_path(&path, &config).unwrap();
    let before = std::fs::read_to_string(&path).unwrap();
    let raw: serde_json::Value = serde_json::from_str(&before).unwrap();
    assert_eq!(raw["connections"][0]["obs"]["host"], "studio.local");
    assert_eq!(raw["connections"][0]["id"], "default");
    assert_eq!(read_config_from_path(&path).config.obs, config.obs);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn reading_a_legacy_config_leaves_its_bytes_unchanged() {
    let dir = crate::storage::test_support::TempDir::new("config-legacy-read");
    let path = dir.path().join("config.json");
    let original = include_str!("../fixtures/config-legacy.json");
    std::fs::write(&path, original).unwrap();

    let loaded = read_config_from_path(&path);

    assert_eq!(loaded.config.version, CURRENT_VERSION);
    assert_eq!(loaded.pending_repair, Some(PendingRepair::WriteMigrated));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
}

#[test]
fn reading_an_unparsable_config_leaves_it_in_place_for_startup_to_move() {
    let dir = crate::storage::test_support::TempDir::new("config-unparsable-read");
    let path = dir.path().join("config.json");
    let original = r#"{ "obs": { "host": "192.168.1.42",, } }"#;
    std::fs::write(&path, original).unwrap();

    let loaded = read_config_from_path(&path);

    assert!(
        matches!(
            loaded.startup_notice,
            Some(ConfigStartupNotice::ParseFailed { backup: None, .. })
        ),
        "{:?}",
        loaded.startup_notice
    );
    assert_eq!(
        loaded.pending_repair,
        Some(PendingRepair::PreserveUnparsable)
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    assert!(!dir.path().join("config.json.invalid").exists());
}

#[test]
fn a_current_config_needs_no_startup_repair() {
    let loaded = read_config_from_path(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/storage/fixtures/config.json"),
    );

    assert_eq!(loaded.startup_notice, None);
    assert_eq!(loaded.pending_repair, None);
}

#[test]
fn future_config_is_not_rewritten_to_materialize_connection_profiles() {
    let original = r#"{"version":999,"obs":{"host":"future.local","port":4455},"future_setting":{"preserve":true}}"#;
    let (config, persisted) = load_from_temp_file(original);
    assert_eq!(config.version, 999);
    assert_eq!(persisted, original);
}

/// A downgrade meets a file from a newer build: an unknown field, and a
/// legacy stylesheet key that used to trigger a read-time rewrite.
const NEWER_CONFIG: &str = r#"{"version":999,"obs":{"host":"future.local","port":4455},"future_setting":{"preserve":true},"appearance":{"custom_css":{"enabled":true,"path":"/tmp/future.css"}}}"#;

#[test]
fn a_newer_config_stays_byte_identical_after_startup_and_a_settings_change() {
    let dir = crate::storage::test_support::TempDir::new("config-newer-save");
    let path = dir.path().join("config.json");
    std::fs::write(&path, NEWER_CONFIG).unwrap();

    let mut config = read_startup_config_from_path(&path).config;
    assert_eq!(std::fs::read_to_string(&path).unwrap(), NEWER_CONFIG);

    config.outputs.confirm_start_stream = true;
    let refused = write_config_to_path(&path, &config).unwrap_err();

    assert_eq!(refused.kind(), io::ErrorKind::Unsupported);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), NEWER_CONFIG);
}

#[test]
fn a_newer_config_is_not_moved_aside_and_raises_the_newer_version_notice() {
    let dir = crate::storage::test_support::TempDir::new("config-newer-notice");
    let path = dir.path().join("config.json");
    std::fs::write(&path, NEWER_CONFIG).unwrap();

    let loaded = read_startup_config_from_path(&path);

    assert_eq!(
        loaded.startup_notice,
        Some(ConfigStartupNotice::NewerVersion { version: 999 })
    );
    assert_eq!(loaded.config.obs.host, "future.local");
    assert!(path.exists());
    assert!(!dir.path().join("config.json.invalid").exists());
}

#[test]
fn an_unparsable_newer_config_is_left_in_place_rather_than_moved_aside() {
    let dir = crate::storage::test_support::TempDir::new("config-newer-unparsable");
    let path = dir.path().join("config.json");
    let original = r#"{"version":999,"obs":{"port":"renamed in a newer build"}}"#;
    std::fs::write(&path, original).unwrap();

    let loaded = read_startup_config_from_path(&path);

    assert!(
        matches!(
            loaded.startup_notice,
            Some(ConfigStartupNotice::ParseFailed { backup: None, .. })
        ),
        "{:?}",
        loaded.startup_notice
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    assert!(!dir.path().join("config.json.invalid").exists());
}

#[test]
fn a_config_read_from_a_newer_file_is_never_written_anywhere() {
    let dir = crate::storage::test_support::TempDir::new("config-newer-memory");
    let path = dir.path().join("config.json");
    let config = AppConfig {
        version: CURRENT_VERSION + 1,
        ..AppConfig::default()
    };

    let refused = write_config_to_path(&path, &config).unwrap_err();

    assert_eq!(refused.kind(), io::ErrorKind::Unsupported);
    assert!(!path.exists());
}

#[test]
fn workspace_migration_remembers_original_endpoint_before_settings_change() {
    let mut config: AppConfig =
        serde_json::from_str(r#"{"version":3,"obs":{"host":"studio.local","port":4456}}"#).unwrap();
    assert!(apply_migrations(&mut config));
    assert_eq!(config.connection_id, "default");
    assert_eq!(
        config.legacy_registry_endpoint.as_deref(),
        Some("studio.local:4456")
    );
    let mut profile = config.connection_profiles()[0].clone();
    profile.obs.host = "other.local".into();
    assert!(config.save_connection(profile));
    assert!(!apply_migrations(&mut config));
    assert_eq!(
        config.legacy_registry_endpoint.as_deref(),
        Some("studio.local:4456")
    );
}

#[test]
fn config_writes_cannot_replace_refused_or_unpreserved_inputs() {
    let path = std::env::temp_dir().join(format!(
        "scenedeck-refused-config-{}.json",
        std::process::id()
    ));
    std::fs::write(&path, b"invalid original").unwrap();
    assert!(write_config_to_path(&path, &AppConfig::default()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"invalid original");
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(crate::storage::small_file::CONFIG_LIMIT as u64 + 1)
        .unwrap();
    let loaded = read_config_from_path(&path);
    assert!(matches!(
        loaded.startup_notice,
        Some(ConfigStartupNotice::ReadFailed(_))
    ));
    assert!(write_config_to_path(&path, &AppConfig::default()).is_err());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn a_symlinked_config_reports_the_symlink_and_saves_never_replace_its_target() {
    let dir = crate::storage::test_support::TempDir::new("config-symlink");
    let target = dir.path().join("dotfiles-config.json");
    std::fs::write(&target, br#"{"version":5}"#).unwrap();
    let path = dir.path().join("config.json");
    std::os::unix::fs::symlink(&target, &path).unwrap();

    let loaded = read_config_from_path(&path);

    assert_eq!(loaded.startup_notice, Some(ConfigStartupNotice::Symlinked));
    assert_eq!(loaded.pending_repair, None);
    assert!(write_config_to_path(&path, &AppConfig::default()).is_err());
    assert!(std::fs::symlink_metadata(&path)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(std::fs::read(&target).unwrap(), br#"{"version":5}"#);
}

#[test]
fn a_config_write_while_another_process_holds_its_lock_is_busy_and_writes_nothing() {
    let dir = crate::storage::test_support::TempDir::new("config-busy");
    let path = dir.path().join("config.json");
    write_config_to_path(&path, &AppConfig::default()).unwrap();
    let before = std::fs::read(&path).unwrap();
    let other_process = crate::storage::lock::lock_store(&path).unwrap();
    let mut changed = AppConfig::default();
    changed.obs.host = "changed.local".into();

    let refused = write_config_to_path(&path, &changed).unwrap_err();

    assert!(crate::storage::lock::is_busy(&refused));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    drop(other_process);
    write_config_to_path(&path, &changed).unwrap();
    assert_eq!(
        read_config_from_path(&path).config.obs.host,
        "changed.local"
    );
}

#[test]
fn empty_object_uses_defaults() {
    let c: AppConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(c.version, CURRENT_VERSION);
    assert_eq!(c.obs.host, "127.0.0.1");
    assert_eq!(c.obs.port, 4455);
    assert!(!c.outputs.confirm_start_stream);
    assert!(c.outputs.confirm_stop_stream);
    assert!(!c.outputs.confirm_start_recording);
    assert!(c.outputs.confirm_stop_recording);
    assert_eq!(c.appearance.mode, ThemeMode::System);
    assert_eq!(c.appearance.selected_theme_id(), "adwaita-default");
    assert_eq!(c.mixer, MixerSelection::default());
    assert_eq!(c.hotkeys, SceneHotkeyConfig::default());
    assert!(!c.onboarding.welcome_shown);
}

/// Writes `contents` to a uniquely named temp file, loads it the way
/// startup does (the read, then the migration write the GTK side performs
/// when one is pending), and hands back both the loaded config and
/// whatever ended up on disk afterwards.  The file is removed before the
/// assertions run so a failing test does not leave litter behind.
fn load_from_temp_file(contents: &str) -> (AppConfig, String) {
    let path = std::env::temp_dir().join(format!(
        "scenedeck-config-test-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, contents).unwrap();
    let loaded = read_startup_config_from_path(&path);
    if loaded.pending_repair == Some(PendingRepair::WriteMigrated) {
        write_config_to_path(&path, &loaded.config).unwrap();
    }
    let persisted = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(path);
    (loaded.config, persisted)
}

#[test]
fn an_unparsable_config_is_kept_rather_than_overwritten() {
    // The scenario: a hand edit leaves a stray comma. The app starts on
    // defaults, and the very next save would land on this path — so the
    // original has to be somewhere else by then.
    let dir = std::env::temp_dir().join(format!(
        "scenedeck-config-invalid-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.json");
    let original = br#"{ "obs": { "host": "192.168.1.42",, } }"#;
    std::fs::write(&path, original).unwrap();

    let loaded = read_startup_config_from_path(&path);

    let backup = dir.join("config.json.invalid");
    assert!(backup.exists(), "the unparsable file was not preserved");
    assert_eq!(std::fs::read(&backup).unwrap(), original);
    assert!(
        !path.exists(),
        "the bad file was left where a save would hit it"
    );

    match loaded.startup_notice {
        Some(ConfigStartupNotice::ParseFailed {
            backup: Some(at), ..
        }) => {
            assert_eq!(at, backup);
        }
        other => panic!("expected a ParseFailed notice naming the backup, got {other:?}"),
    }
    // The app still starts, on defaults.
    assert_eq!(loaded.config.obs.host, "127.0.0.1");
    assert_eq!(loaded.pending_repair, None);

    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn existing_configs_are_not_greeted_on_upgrade() {
    // Anyone whose config predates the Help page has already been using
    // SceneDeck, so the welcome dialog must not interrupt their upgrade.
    let (config, persisted) = load_from_temp_file(r#"{ "version": 2 }"#);

    assert_eq!(config.version, CURRENT_VERSION);
    assert!(config.onboarding.welcome_shown);
    assert!(persisted.contains(r#""welcome_shown": true"#));
}

#[test]
fn a_fresh_install_is_greeted() {
    // No config file at all is the one case that means "new user".
    let missing = std::env::temp_dir().join(format!(
        "scenedeck-config-absent-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let loaded = read_config_from_path(&missing);

    assert_eq!(
        loaded.startup_notice,
        Some(ConfigStartupNotice::FirstLaunch)
    );
    assert!(!loaded.config.onboarding.welcome_shown);
}

#[test]
fn onboarding_flag_round_trips() {
    let parsed: AppConfig =
        serde_json::from_str(r#"{"onboarding":{"welcome_shown":true}}"#).unwrap();
    assert!(parsed.onboarding.welcome_shown);

    let json = serde_json::to_string(&parsed).unwrap();
    assert!(
        serde_json::from_str::<AppConfig>(&json)
            .unwrap()
            .onboarding
            .welcome_shown
    );
}

#[test]
fn hotkey_section_round_trips_and_tolerates_partial_objects() {
    use crate::domain::hotkey::{LeaderKey, SceneHotkeyStyle};

    let parsed: AppConfig =
        serde_json::from_str(r#"{"hotkeys":{"style":"leader","leader":"comma"}}"#).unwrap();

    assert!(parsed.hotkeys.enabled);
    assert_eq!(parsed.hotkeys.style, SceneHotkeyStyle::Leader);
    assert_eq!(parsed.hotkeys.leader, LeaderKey::Comma);
    assert_eq!(
        parsed.hotkeys.leader_timeout_ms,
        crate::domain::hotkey::DEFAULT_LEADER_TIMEOUT_MS
    );

    let json = serde_json::to_string(&parsed).unwrap();
    assert_eq!(
        serde_json::from_str::<AppConfig>(&json).unwrap().hotkeys,
        parsed.hotkeys
    );
}

#[test]
fn legacy_inventory_widths_preserve_preview_settings_without_being_saved() {
    let old: AppConfig = serde_json::from_str("{}").unwrap();
    assert!(!old.live.scene_previews);
    let parsed: AppConfig = serde_json::from_str(
        r#"{"inventory":{"name_width":420,"role_width":280},"live":{"scene_previews":true}}"#,
    )
    .unwrap();
    let saved = serde_json::to_value(&parsed).unwrap();
    assert!(
        saved.get("inventory").is_none(),
        "removed width settings must not be saved"
    );
    let reread: AppConfig = serde_json::from_value(saved).unwrap();
    assert!(reread.live.scene_previews);
    let partial: AppConfig = serde_json::from_str(r#"{"inventory":{"role_width":300}}"#).unwrap();
    assert!(!partial.live.scene_previews);
    assert!(serde_json::to_value(partial)
        .unwrap()
        .get("inventory")
        .is_none());
}

#[test]
fn legacy_inventory_widths_accept_invalid_values_without_affecting_settings() {
    let expected = serde_json::to_value(AppConfig::default()).unwrap();
    for legacy in [
        r#"{"inventory":{"name_width":-50,"role_width":2147483647}}"#,
        r#"{"inventory":{"name_width":1000,"role_width":600}}"#,
        r#"{"inventory":{}}"#,
    ] {
        let parsed: AppConfig = serde_json::from_str(legacy).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), expected);
    }
}

#[test]
fn round_trips_through_json() {
    let config = AppConfig {
        version: 1,
        obs: ObsConfig {
            host: "192.168.1.10".to_string(),
            port: 4455,
        },
        live: LiveConfig::default(),
        appearance: ThemePreference {
            mode: ThemeMode::Dark,
            ..ThemePreference::default()
        },
        ..AppConfig::default()
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: AppConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.obs.host, "192.168.1.10");
    assert_eq!(parsed.appearance.mode, ThemeMode::Dark);
}

#[test]
fn unknown_theme_mode_uses_system_fallback() {
    let parsed: AppConfig = serde_json::from_str(r#"{"appearance":{"mode":"future"}}"#).unwrap();

    assert_eq!(parsed.appearance.mode, ThemeMode::System);
}

#[test]
fn a_legacy_custom_css_path_survives_a_version_migration() {
    // Both migration steps have to run on the same load. The version bump
    // fires for every config this old, and it used to short-circuit the
    // custom-stylesheet step, which cost the user the path entirely: the
    // legacy `path` key is `skip_serializing`, so the rewrite that follows
    // a migration is what deleted it.
    let (config, persisted) = load_from_temp_file(
        r#"{
              "version": 2,
              "appearance": { "custom_css": { "enabled": true, "path": "/tmp/scenedeck.css" } }
            }"#,
    );

    assert_eq!(config.version, CURRENT_VERSION);
    // Assert the value, not the key: `light_path` is `#[serde(default)]`
    // with no `skip_serializing_if`, so it is written as `null` even when
    // the migration did not run. Checking only for the key name would
    // pass against the very bug this test exists to catch.
    assert!(
        persisted.contains(r#""light_path": "/tmp/scenedeck.css""#),
        "{persisted}"
    );
    assert!(
        persisted.contains(r#""dark_path": "/tmp/scenedeck.css""#),
        "{persisted}"
    );
}

#[test]
fn migrates_v1_theme_mode_to_v2_appearance() {
    let (config, persisted) = load_from_temp_file(
        r#"{
              "version": 1,
              "obs": { "host": "127.0.0.1", "port": 4455 },
              "live": {
                "show_roles": ["primary"],
                "audio_inputs": [],
                "allow_switching_only": ["primary"]
              },
              "theme_mode": "dark"
            }"#,
    );

    // A v1 config runs through every later step in one pass, so it lands on
    // the current version rather than stopping at 2.
    assert_eq!(config.version, CURRENT_VERSION);
    assert_eq!(config.appearance.mode, ThemeMode::Dark);
    assert!(persisted.contains(r#""appearance""#));
    assert!(!persisted.contains(r#""theme_mode""#));
    // It is an existing config, so it is not a first run.
    assert!(config.onboarding.welcome_shown);
}

#[test]
fn current_config_fixture_round_trips() {
    let fixture = include_str!("../fixtures/config.json");
    crate::storage::golden::json_round_trip::<AppConfig>(fixture);
    let config: AppConfig = serde_json::from_str(fixture).unwrap();
    assert_eq!(config.version, CURRENT_VERSION);
}

#[test]
fn legacy_config_fixture_migrates_theme_mode_and_custom_css_path() {
    // These keys are `skip_serializing` and rewritten by migration, so the
    // fixture must not byte-match the current serializer.
    let fixture = include_str!("../fixtures/config-legacy.json");
    let original: serde_json::Value = serde_json::from_str(fixture).unwrap();
    assert_eq!(original["theme_mode"], "dark");
    assert_eq!(
        original["appearance"]["custom_css"]["path"],
        "/tmp/scenedeck.css"
    );

    let (config, persisted) = load_from_temp_file(fixture);
    let saved: serde_json::Value = serde_json::from_str(&persisted).unwrap();
    assert_eq!(config.version, CURRENT_VERSION);
    assert_eq!(config.appearance.mode, ThemeMode::Dark);
    assert_eq!(
        config.appearance.custom_css.light_path.as_deref(),
        Some(std::path::Path::new("/tmp/scenedeck.css"))
    );
    assert_eq!(
        config.appearance.custom_css.dark_path.as_deref(),
        Some(std::path::Path::new("/tmp/scenedeck.css"))
    );
    assert!(saved.get("theme_mode").is_none());
    assert!(saved["appearance"]["custom_css"].get("path").is_none());
    assert_eq!(saved["appearance"]["mode"], "dark");
    assert_eq!(
        saved["appearance"]["custom_css"]["light_path"],
        "/tmp/scenedeck.css"
    );
    assert_eq!(
        saved["appearance"]["custom_css"]["dark_path"],
        "/tmp/scenedeck.css"
    );
    assert_eq!(saved["onboarding"]["welcome_shown"], true);
}
