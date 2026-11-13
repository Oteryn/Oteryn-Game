//! Compatibility wrapper for the existing Windows shell.
use super::{AudioOutput, AudioSettings, SoundCue};
use oteryn_session::PresentationSource;

/// Process-local audio preferences and output owned by the native presentation feature.
pub struct NativeAudio {
    device_name: String,
    output: Result<AudioOutput, String>,
    settings: AudioSettings,
}

impl Default for NativeAudio {
    fn default() -> Self {
        Self {
            device_name: String::new(),
            output: AudioOutput::from_environment(),
            settings: AudioSettings::default(),
        }
    }
}

impl NativeAudio {
    pub fn restore(&self, view: &mut crate::play::PlayView) {
        let mut preferences = view.preferences().clone();
        preferences.audio = self.settings;
        let _ = view.set_preferences(preferences);
    }

    pub fn stop(&mut self) {
        if let Ok(audio) = &mut self.output {
            audio.stop();
        }
    }

    /// Drains transient cues and formats native feedback on the shell thread.
    pub fn status(&mut self, view: &mut crate::play::PlayView) -> String {
        let requested = &view.preferences().audio_device;
        if self.device_name != *requested {
            let ambient = self
                .output
                .as_ref()
                .ok()
                .and_then(|audio| audio.ambient_id());
            self.output = AudioOutput::with_device(requested);
            if let (Some(id), Ok(audio)) = (ambient, &mut self.output) {
                audio.update(
                    view.preferences().audio,
                    vec![SoundCue {
                        id,
                        source: PresentationSource::Global,
                        ambient: true,
                    }],
                );
            }
            self.device_name = requested.clone();
        }
        self.settings = view.preferences().audio;
        let ui = view.take_ui_sounds();
        let cues = view.take_sound_cues();
        let (status, ambient) = match &mut self.output {
            Ok(audio) => {
                audio.update(self.settings, cues);
                audio.play_ui(self.settings, ui);
                (
                    audio.status().unwrap_or("ready"),
                    audio.ambient_id().is_some(),
                )
            }
            Err(error) => (error.as_str(), false),
        };
        lfmt!(
            "{} | Audio {}%{} ({status}, {}); F10 mute, F11/F12 volume",
            view.combat_status(),
            self.settings.master_volume,
            if self.settings.muted { " muted" } else { "" },
            oteryn_client_ui::ambient_sound::status(ambient),
            status = status
        )
    }
}
