use super::state::AudioSettings;
use crate::play::PlayView;
use oteryn_input_actions::{ButtonState, Modifiers, NormalizedInputEvent};

/// Physical F10 toggles mute; F11/F12 change master volume by five percentage points.
pub fn audio_input(settings: &mut AudioSettings, events: &[NormalizedInputEvent]) -> bool {
    let mut handled = false;
    for event in events {
        if let NormalizedInputEvent::Key {
            code,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat: false,
        } = event
        {
            match code.get() {
                67 => settings.muted = !settings.muted,
                68 => settings.master_volume = settings.master_volume.saturating_sub(5),
                69 => settings.master_volume = settings.master_volume.saturating_add(5).min(100),
                _ => continue,
            }
            handled = true;
        }
    }
    handled
}

impl PlayView {
    pub(in crate::play) fn enqueue(&mut self, cues: Vec<super::state::SoundCue>) {
        super::protocol::absorb_sounds(&mut self.sounds, cues);
    }

    pub fn audio_input(&mut self, events: &[NormalizedInputEvent]) -> bool {
        let mut preferences = self.preferences().clone();
        if !audio_input(&mut preferences.audio, events) {
            return false;
        }
        self.set_preferences(preferences).is_ok()
    }

    pub fn take_sound_cues(&mut self) -> Vec<super::state::SoundCue> {
        std::mem::take(&mut self.sounds)
    }
}
