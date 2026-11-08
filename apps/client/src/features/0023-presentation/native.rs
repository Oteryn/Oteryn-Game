//! Windows playback through upstream Rodio. No device is opened by headless tests.
use super::{AudioSettings, SoundCatalog, SoundCue};
use oteryn_session::PresentationSource;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use std::io::Cursor;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct AudioOutput {
    stream: OutputStream,
    catalog: SoundCatalog,
    voices: Vec<(PresentationSource, Sink)>,
    ambient: Option<(u32, Sink)>,
    error: Option<String>,
}

impl AudioOutput {
    pub fn from_environment() -> Result<Self, String> {
        let manifest =
            std::env::var("OTERYN_SOUND_MANIFEST").map_err(|_| "sound manifest not configured")?;
        let pin = std::env::var("OTERYN_SOUND_MANIFEST_SHA256")
            .map_err(|_| "sound manifest SHA-256 not configured")?;
        let dir =
            std::env::var("OTERYN_SOUND_DIR").map_err(|_| "sound directory not configured")?;
        let catalog = SoundCatalog::open(Path::new(&dir), Path::new(&manifest), &pin)?;
        let stream = OutputStreamBuilder::open_default_stream().map_err(|e| e.to_string())?;
        Ok(Self {
            stream,
            catalog,
            voices: Vec::new(),
            ambient: None,
            error: None,
        })
    }

    pub fn stop(&mut self) {
        if let Some((_, voice)) = self.ambient.take() {
            voice.stop();
        }
        for (_, voice) in self.voices.drain(..) {
            voice.stop();
        }
    }

    pub fn status(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn update(&mut self, settings: AudioSettings, cues: Vec<SoundCue>) {
        self.voices.retain(|(_, voice)| !voice.empty());
        for (source, voice) in &self.voices {
            voice.set_volume(settings.source_gain(*source));
        }
        if let Some((_, voice)) = &self.ambient {
            voice.set_volume(settings.gain());
        }
        for cue in cues {
            if !cue.ambient && settings.source_gain(cue.source) == 0.0 {
                continue;
            }
            if let Err(error) = self.play(cue, settings.source_gain(cue.source)) {
                self.error = Some(error);
            }
        }
    }

    fn play(&mut self, cue: SoundCue, gain: f32) -> Result<(), String> {
        if cue.ambient {
            if self.ambient.as_ref().is_some_and(|(id, _)| *id == cue.id) {
                return Ok(());
            }
            if let Some((_, voice)) = self.ambient.take() {
                voice.stop();
            }
            if cue.id == 0 {
                return Ok(());
            }
            let Some(bytes) = self.catalog.read_channel(cue.id, 0, true)? else {
                return Ok(());
            };
            let source = Decoder::try_from(Cursor::new(bytes)).map_err(|e| e.to_string())?;
            let voice = Sink::connect_new(self.stream.mixer());
            voice.set_volume(gain);
            voice.append(source.repeat_infinite());
            self.ambient = Some((cue.id, voice));
            return Ok(());
        }
        let variant = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos() as usize);
        let Some(bytes) = self.catalog.read(cue.id, variant)? else {
            return Ok(());
        };
        let source = Decoder::try_from(Cursor::new(bytes)).map_err(|e| e.to_string())?;
        if self.voices.len() == 32 {
            self.voices.remove(0).1.stop();
        }
        let voice = Sink::connect_new(self.stream.mixer());
        voice.set_volume(gain);
        // Numeric cues are one-shots. Bound voice lifetime even for malformed/stream assets.
        voice.append(source.take_duration(Duration::from_secs(30)));
        self.voices.push((cue.source, voice));
        Ok(())
    }
}

/// Process-local audio preferences and output owned by the native presentation feature.
pub struct NativeAudio {
    output: Result<AudioOutput, String>,
    settings: AudioSettings,
}

impl Default for NativeAudio {
    fn default() -> Self {
        Self {
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
        self.settings = view.preferences().audio;
        let cues = view.take_sound_cues();
        let (status, ambient) = match &mut self.output {
            Ok(audio) => {
                audio.update(self.settings, cues);
                (audio.status().unwrap_or("ready"), audio.ambient.is_some())
            }
            Err(error) => (error.as_str(), false),
        };
        format!(
            "{} | Audio {}%{} ({status}, {}); F10 mute, F11/F12 volume",
            view.combat_status(),
            self.settings.master_volume,
            if self.settings.muted { " muted" } else { "" },
            oteryn_client_ui::ambient_sound::status(ambient)
        )
    }
}
