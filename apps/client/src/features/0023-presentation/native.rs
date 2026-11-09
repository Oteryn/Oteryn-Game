//! Windows playback through upstream Rodio. No device is opened by headless tests.
use super::{
    AudioSettings, BattleClass, SoundCatalog, SoundChannel, SoundCue, select_audio_device,
};
use oteryn_session::PresentationSource;
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use std::io::Cursor;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct AudioOutput {
    stream: OutputStream,
    catalog: SoundCatalog,
    voices: Vec<(PresentationSource, SoundChannel, Option<BattleClass>, Sink)>,
    ambient: Option<(u32, Sink)>,
    error: Option<String>,
}

impl AudioOutput {
    pub fn from_environment() -> Result<Self, String> {
        Self::with_device("")
    }

    fn with_device(requested: &str) -> Result<Self, String> {
        let manifest =
            std::env::var("OTERYN_SOUND_MANIFEST").map_err(|_| tr!("sound manifest not configured"))?;
        let pin = std::env::var("OTERYN_SOUND_MANIFEST_SHA256")
            .map_err(|_| tr!("sound manifest SHA-256 not configured"))?;
        let dir =
            std::env::var("OTERYN_SOUND_DIR").map_err(|_| tr!("sound directory not configured"))?;
        let catalog = SoundCatalog::open(Path::new(&dir), Path::new(&manifest), &pin)?;
        let devices: Vec<_> = rodio::cpal::default_host()
            .output_devices()
            .map_err(|e| e.to_string())?
            .collect();
        let names = devices
            .iter()
            .map(|device| device.name().map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        let stream = match select_audio_device(requested, &names)? {
            None => OutputStreamBuilder::open_default_stream(),
            Some(index) => OutputStreamBuilder::from_device(devices[index].clone())
                .and_then(|builder| builder.open_stream()),
        }
        .map_err(|e| e.to_string())?;
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
        for (_, _, _, voice) in self.voices.drain(..) {
            voice.stop();
        }
    }

    pub fn status(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn update(&mut self, settings: AudioSettings, cues: Vec<SoundCue>) {
        self.voices.retain(|(_, _, _, voice)| !voice.empty());
        for (source, channel, class, voice) in &self.voices {
            voice.set_volume(settings.classified_gain(*source, *channel, *class));
        }
        if let Some((_, voice)) = &self.ambient {
            voice.set_volume(settings.ambient_gain());
        }
        for cue in cues {
            if !cue.ambient
                && settings.classified_gain(
                    cue.source,
                    self.catalog.channel(cue.id),
                    self.catalog.battle_class(cue.id),
                ) == 0.0
            {
                continue;
            }
            if let Err(error) = self.play(
                cue,
                if cue.ambient {
                    settings.ambient_gain()
                } else {
                    settings.classified_gain(
                        cue.source,
                        self.catalog.channel(cue.id),
                        self.catalog.battle_class(cue.id),
                    )
                },
            ) {
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
            self.voices.remove(0).3.stop();
        }
        let voice = Sink::connect_new(self.stream.mixer());
        voice.set_volume(gain);
        // Numeric cues are one-shots. Bound voice lifetime even for malformed/stream assets.
        voice.append(source.take_duration(Duration::from_secs(30)));
        self.voices.push((
            cue.source,
            self.catalog.channel(cue.id),
            self.catalog.battle_class(cue.id),
            voice,
        ));
        Ok(())
    }
}

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
                .and_then(|audio| audio.ambient.as_ref().map(|(id, _)| *id));
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
                for cue in ui {
                    let voice = Sink::connect_new(audio.stream.mixer());
                    voice.set_volume(
                        self.settings
                            .channel_gain(PresentationSource::Global, SoundChannel::Ui)
                            * 0.15,
                    );
                    voice.append(
                        rodio::source::SineWave::new(cue.frequency())
                            .take_duration(Duration::from_millis(70)),
                    );
                    voice.detach();
                }
                (audio.status().unwrap_or("ready"), audio.ambient.is_some())
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

/// Device names for the Sound page; saved missing devices remain explicit selections.
pub fn audio_device_names() -> Vec<String> {
    rodio::cpal::default_host()
        .output_devices()
        .map(|devices| devices.filter_map(|device| device.name().ok()).collect())
        .unwrap_or_default()
}
