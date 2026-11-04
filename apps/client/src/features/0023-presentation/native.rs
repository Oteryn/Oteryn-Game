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
            error: None,
        })
    }

    pub fn stop(&mut self) {
        for (_, voice) in self.voices.drain(..) {
            voice.stop();
        }
    }

    pub fn status(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn update(&mut self, settings: AudioSettings, cues: Vec<SoundCue>) {
        self.voices.retain(|(_, voice)| !voice.empty());
        for (_, voice) in &self.voices {
            voice.set_volume(settings.gain());
        }
        if settings.gain() == 0.0 {
            return;
        }
        for cue in cues {
            if let Err(error) = self.play(cue, settings.gain()) {
                self.error = Some(error);
            }
        }
    }

    fn play(&mut self, cue: SoundCue, gain: f32) -> Result<(), String> {
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
