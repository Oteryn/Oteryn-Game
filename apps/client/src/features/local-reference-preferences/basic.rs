//! A compact options page that edits the existing, live preference consumers together.
use super::{MousePreset, MovementKeys, PreferenceError, ReferencePreferences, SettingsState};
pub const BASIC_GROUPS: [&str; 4] = ["Gameplay", "Interface", "Graphics", "Sound"];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasicOptions {
    pub movement: MovementKeys,
    pub mouse: MousePreset,
    pub world_zoom: u16,
    pub foreground_fps: u16,
    pub background_fps: u16,
    pub master_volume: u8,
    pub muted: bool,
}
impl From<&ReferencePreferences> for BasicOptions {
    fn from(p: &ReferencePreferences) -> Self {
        Self {
            movement: p.controls.movement,
            mouse: p.controls.mouse,
            world_zoom: p.ui_scale,
            foreground_fps: p.frames.foreground,
            background_fps: p.frames.background,
            master_volume: p.audio.master_volume,
            muted: p.audio.muted,
        }
    }
}
impl SettingsState {
    pub fn apply_basic(&mut self, options: &BasicOptions) -> Result<(), PreferenceError> {
        let mut prefs = self.preferences.clone();
        prefs.controls.movement = options.movement;
        prefs.controls.mouse = options.mouse;
        prefs.ui_scale = options.world_zoom;
        prefs.frames.foreground = options.foreground_fps;
        prefs.frames.background = options.background_fps;
        prefs.audio.master_volume = options.master_volume;
        prefs.audio.muted = options.muted;
        prefs.validate()?;
        self.apply(prefs);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::{MousePreset, MovementKeys, SettingsState};
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    #[test]
    fn basic_options_groups_save_atomically_and_drive_live_consumers()
    -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(BASIC_GROUPS, ["Gameplay", "Interface", "Graphics", "Sound"]);
        let path = std::env::temp_dir().join(format!("basic-options-{}.json", std::process::id()));
        let mut state = SettingsState::new(Some(path.clone()));
        let mut options = BasicOptions::from(&state.preferences);
        options.movement = MovementKeys::ArrowsAndWasd;
        options.mouse = MousePreset::LeftHanded;
        options.world_zoom = 150;
        options.foreground_fps = 120;
        options.background_fps = 12;
        options.master_volume = 37;
        options.muted = true;
        state.apply_basic(&options)?;
        assert_eq!(
            BasicOptions::from(&SettingsState::new(Some(path.clone())).preferences),
            options
        );
        assert_eq!(state.preferences.audio.gain(), 0.0);
        assert_eq!(state.preferences.frames.foreground, 120);
        let w = NormalizedInputEvent::Key {
            code: KeyCode::new(26)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat: false,
        };
        assert!(
            matches!(state.control_events(&[w],false).as_slice(),[NormalizedInputEvent::Key{code,..}] if code.get()==82)
        );
        let previous = state.preferences.clone();
        options.world_zoom = 999;
        assert!(state.apply_basic(&options).is_err());
        assert_eq!(state.preferences, previous);
        std::fs::remove_file(path)?;
        Ok(())
    }
}
