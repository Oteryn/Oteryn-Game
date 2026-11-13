use crate::play::{MiscellaneousPreferences, PreferenceStore, SettingsState};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> std::io::Result<Self> {
        let path = std::env::temp_dir().join(format!(
            "miscellaneous-settings-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path)?;
        Ok(Self(path))
    }
    fn preferences(&self) -> PathBuf {
        self.0.join("client-preferences.json")
    }
    fn sidecar(&self) -> PathBuf {
        self.preferences().with_extension("session-options.json")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
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
