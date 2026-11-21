use crate::play::{MiscellaneousPreferences, PreferenceStore, SettingsState};
use std::path::PathBuf;
type TestResult = Result<(), Box<dyn std::error::Error>>;
use crate::play::screenshot::TestDir as Temp;
impl Temp {
    fn preferences(&self) -> PathBuf {
        self.join("client-preferences.json")
    }
    fn sidecar(&self) -> PathBuf {
        self.preferences().with_extension("session-options.json")
    }
}

#[test]
fn miscellaneous_apply_restores_confirmation_from_the_selected_sidecar() -> TestResult {
    let temp = Temp::new()?;
    let mut state = SettingsState::new(Some(temp.preferences()));
    let mut p = state.preferences.clone();
    p.miscellaneous.confirm_logout = false;
    state.apply(p);
    assert!(!state.preferences.miscellaneous.confirm_logout);
    assert_eq!(state.feedback, "Applied and saved");
    let restored = SettingsState::new(Some(temp.preferences()));
    assert!(
        !restored.preferences.miscellaneous.confirm_logout,
        "accepted confirmation must survive restart"
    );
    assert_eq!(
        MiscellaneousPreferences::read(&temp.sidecar())?,
        state.preferences.miscellaneous
    );
    Ok(())
}
#[test]
fn miscellaneous_sidecar_failure_is_reported_and_an_unchanged_retry_persists() -> TestResult {
    let temp = Temp::new()?;
    std::fs::create_dir(temp.sidecar())?;
    let mut state = SettingsState::new(Some(temp.preferences()));
    let mut p = state.preferences.clone();
    p.miscellaneous.confirm_logout = false;
    p.graphics.fullscreen = true;
    state.apply(p.clone());
    assert_eq!(
        state.preferences, p,
        "disk failure retains accepted live settings"
    );
    assert!(
        state
            .feedback
            .starts_with("Applied for this session; could not save:"),
        "{}",
        state.feedback
    );
    assert!(
        PreferenceStore::read_transfer(&temp.preferences())?
            .graphics
            .fullscreen,
        "main file was saved before the sidecar failed"
    );
    std::fs::remove_dir(temp.sidecar())?;
    state.apply(p);
    assert_eq!(state.feedback, "Applied and saved");
    assert!(
        !SettingsState::new(Some(temp.preferences()))
            .preferences
            .miscellaneous
            .confirm_logout
    );
    Ok(())
}
#[test]
fn miscellaneous_main_failure_does_not_mutate_the_existing_sidecar() -> TestResult {
    let temp = Temp::new()?;
    std::fs::create_dir(temp.preferences())?;
    MiscellaneousPreferences::default().save(&temp.sidecar())?;
    let before = std::fs::read(temp.sidecar())?;
    let mut state = SettingsState::new(Some(temp.preferences()));
    let mut p = state.preferences.clone();
    p.miscellaneous.confirm_logout = false;
    state.apply(p);
    assert!(!state.preferences.miscellaneous.confirm_logout);
    assert!(
        state
            .feedback
            .starts_with("Applied for this session; could not save:")
    );
    assert_eq!(std::fs::read(temp.sidecar())?, before);
    Ok(())
}
#[test]
fn miscellaneous_invalid_preferences_write_neither_file() -> TestResult {
    let temp = Temp::new()?;
    let mut state = SettingsState::new(Some(temp.preferences()));
    state.apply(state.preferences.clone());
    // Seed the old owner's sidecar explicitly so this also exercises the pre-repair path.
    MiscellaneousPreferences::default().save(&temp.sidecar())?;
    let main = std::fs::read(temp.preferences())?;
    let sidecar = std::fs::read(temp.sidecar())?;
    let before = state.preferences.clone();
    let mut invalid = before.clone();
    invalid.miscellaneous.confirm_logout = false;
    invalid.ui_scale = 999;
    state.apply(invalid);
    assert_eq!(state.preferences, before);
    assert_ne!(state.feedback, "Applied and saved");
    assert_eq!(std::fs::read(temp.preferences())?, main);
    assert_eq!(std::fs::read(temp.sidecar())?, sidecar);
    Ok(())
}
#[test]
fn miscellaneous_character_scope_uses_the_original_base_sidecar_and_retains_flags() -> TestResult {
    let temp = Temp::new()?;
    let flags = MiscellaneousPreferences {
        stay_logged_in: true,
        confirm_logout: false,
        ..Default::default()
    };
    flags.save(&temp.sidecar())?;
    let mut state = SettingsState::new(Some(temp.preferences()));
    state.apply(state.preferences.clone());
    let character = [7; 16];
    let character_path = PreferenceStore::character_path(&temp.preferences(), character);
    state.select_character(Some(character));
    assert_eq!(state.preferences.miscellaneous, flags);
    assert_eq!(
        MiscellaneousPreferences::read(&temp.sidecar())?,
        flags,
        "template adoption must not reset base flags"
    );
    let mut p = state.preferences.clone();
    p.miscellaneous.confirm_logout = true;
    p.graphics.fullscreen = true;
    state.apply(p);
    let restored = SettingsState::new(Some(temp.preferences()));
    assert!(restored.preferences.miscellaneous.confirm_logout);
    assert!(restored.preferences.miscellaneous.stay_logged_in);
    assert!(
        !restored.preferences.graphics.fullscreen,
        "main character settings remain isolated"
    );
    assert!(
        PreferenceStore::read_transfer(&character_path)?
            .graphics
            .fullscreen
    );
    assert!(
        !character_path
            .with_extension("session-options.json")
            .exists(),
        "session choices belong to the original local base, not actor files"
    );
    Ok(())
}

#[test]
fn portable_import_then_apply_preserves_session_options() -> TestResult {
    let temp = Temp::new()?;
    let flags = MiscellaneousPreferences {
        stay_logged_in: true,
        remember_device: true,
        quick_login: true,
        confirm_logout: false,
        confirm_exit: false,
    };
    flags.save(&temp.sidecar())?;
    let mut state = SettingsState::new(Some(temp.preferences()));
    let transfer = temp.join("portable.json");
    let mut portable = crate::play::ReferencePreferences::default();
    portable.graphics.fullscreen = true;
    PreferenceStore::export_to(&portable, &transfer)?;
    state.import_from(&transfer)?;
    assert!(state.preferences.graphics.fullscreen);
    state.apply(state.preferences.clone());
    assert_eq!(state.feedback, "Applied and saved");
    assert_eq!(MiscellaneousPreferences::read(&temp.sidecar())?, flags);
    assert_eq!(state.preferences.miscellaneous, flags);
    state.restore_backup()?;
    state.apply(state.preferences.clone());
    assert_eq!(MiscellaneousPreferences::read(&temp.sidecar())?, flags);
    state.reset_defaults()?;
    state.apply(state.preferences.clone());
    assert_eq!(MiscellaneousPreferences::read(&temp.sidecar())?, flags);
    Ok(())
}

#[test]
fn independent_settings_saves_preserve_disjoint_preferences_and_session_options() -> TestResult {
    let temp = Temp::new()?;
    let mut first = SettingsState::new(Some(temp.preferences()));
    let mut second = SettingsState::new(Some(temp.preferences()));
    let mut edited = first.preferences.clone();
    edited.audio.master_volume = 35;
    edited.miscellaneous.confirm_logout = false;
    first.apply(edited);
    let mut edited = second.preferences.clone();
    edited.ui_scale = 125;
    edited.miscellaneous.confirm_exit = false;
    second.apply(edited);
    let restored = SettingsState::new(Some(temp.preferences()));
    assert_eq!(restored.preferences.ui_scale, 125);
    assert_eq!(restored.preferences.audio.master_volume, 35);
    assert!(!restored.preferences.miscellaneous.confirm_logout);
    assert!(!restored.preferences.miscellaneous.confirm_exit);
    Ok(())
}

#[cfg(bevy_shell)]
#[test]
fn import_from_active_file_waits_for_the_pending_save() -> TestResult {
    use std::time::{Duration, Instant};

    bevy_tasks::IoTaskPool::get_or_init(|| {
        bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
    });
    let temp = Temp::new()?;
    let path = temp.preferences();
    let mut state = SettingsState::new(Some(path.clone()));
    state.use_background_preferences();
    let held = crate::atomic_file::lock(&path)?;
    let mut edited = state.preferences.clone();
    edited.ui_scale = 125;
    state.apply(edited);
    state.import_from(&path)?;
    assert!(state.preference_transfer_pending());
    drop(held);
    let deadline = Instant::now() + Duration::from_secs(2);
    while state.local_preference_writes_pending() {
        assert!(Instant::now() < deadline, "pending import did not finish");
        assert!(state.poll_background_save().error.is_none());
        std::thread::yield_now();
    }
    assert_eq!(PreferenceStore::read_transfer(&path)?.ui_scale, 125);
    assert_eq!(state.preferences.ui_scale, 125);
    Ok(())
}

#[test]
fn review_3376_malformed_session_options_recover_on_apply() -> TestResult {
    let temp = Temp::new()?;
    std::fs::write(temp.sidecar(), b"{")?;
    let mut state = SettingsState::new(Some(temp.preferences()));
    let mut edited = state.preferences.clone();
    edited.miscellaneous.confirm_exit = false;
    state.apply(edited);
    assert!(
        MiscellaneousPreferences::read(&temp.sidecar()).is_ok_and(|options| !options.confirm_exit)
    );
    Ok(())
}
