//! Portable native playback through upstream Rodio. No device is opened by headless tests.
#[cfg(bevy_shell)]
use super::UiSound;
use super::{
    AudioSettings, BattleClass, SoundCatalog, SoundChannel, SoundCue, select_audio_device,
};
use oteryn_session::PresentationSource;
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use std::io::Cursor;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct Voice {
    source: PresentationSource,
    channel: SoundChannel,
    class: Option<BattleClass>,
    sink: Sink,
    scale: f32,
}

fn update_voices(voices: &mut Vec<Voice>, settings: AudioSettings) {
    voices.retain(|voice| !voice.sink.empty());
    for voice in voices {
        voice.sink.set_volume(
            settings.classified_gain(voice.source, voice.channel, voice.class) * voice.scale,
        );
    }
}

pub struct AudioOutput {
    mixer: rodio::mixer::Mixer,
    _stream: Option<OutputStream>,
    catalog: SoundCatalog,
    voices: Vec<Voice>,
    ambient: Option<(u32, Sink)>,
    error: Option<String>,
}

impl AudioOutput {
    pub fn from_environment() -> Result<Self, String> {
        Self::open_reporting("", std::sync::mpsc::sync_channel(1).0)
    }

    pub(crate) fn device_error_callback(
        report: std::sync::mpsc::SyncSender<String>,
    ) -> impl FnMut(rodio::cpal::StreamError) + Send + Clone + 'static {
        move |error| {
            let _ = report.try_send(error.to_string());
        }
    }
    pub(crate) fn open_reporting(
        requested: &str,
        report: std::sync::mpsc::SyncSender<String>,
    ) -> Result<Self, String> {
        #[cfg(test)]
        verify_audio_thread();
        let manifest = std::env::var("OTERYN_SOUND_MANIFEST")
            .map_err(|_| tr!("sound manifest not configured"))?;
        let pin = std::env::var("OTERYN_SOUND_MANIFEST_SHA256")
            .map_err(|_| tr!("sound manifest SHA-256 not configured"))?;
        let dir =
            std::env::var("OTERYN_SOUND_DIR").map_err(|_| tr!("sound directory not configured"))?;
        let catalog = SoundCatalog::open(Path::new(&dir), Path::new(&manifest), &pin)?;
        pin_audio_host();
        let devices: Vec<_> = rodio::cpal::default_host()
            .output_devices()
            .map_err(|e| e.to_string())?
            .collect();
        let names = devices
            .iter()
            .map(|device| device.name().map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        let callback = Self::device_error_callback(report);
        let open = |device| {
            OutputStreamBuilder::from_device(device).and_then(|builder| {
                builder
                    .with_error_callback(callback.clone())
                    .open_stream_or_fallback()
            })
        };
        let stream = match select_audio_device(requested, &names)? {
            None => OutputStreamBuilder::from_default_device()
                .and_then(|builder| builder.with_error_callback(callback.clone()).open_stream())
                .or_else(|error| devices.into_iter().find_map(|d| open(d).ok()).ok_or(error)),
            Some(index) => open(devices[index].clone()),
        }
        .map_err(|e| e.to_string())?;
        Ok(Self {
            mixer: stream.mixer().clone(),
            _stream: Some(stream),
            catalog,
            voices: Vec::new(),
            ambient: None,
            error: None,
        })
    }

    /// Confirmed local UI tones share the bounded voices and stop with the session.
    #[cfg(bevy_shell)]
    pub(crate) fn play_ui(&mut self, settings: AudioSettings, cues: Vec<UiSound>) {
        let gain = settings.channel_gain(PresentationSource::Global, SoundChannel::Ui) * 0.15;
        if gain == 0.0 {
            return;
        }
        self.voices.retain(|voice| !voice.sink.empty());
        for cue in cues {
            if self.voices.len() == 32 {
                self.voices.remove(0).sink.stop();
            }
            let voice = Sink::connect_new(&self.mixer);
            voice.set_volume(gain);
            voice.append(
                rodio::source::SineWave::new(cue.frequency())
                    .take_duration(Duration::from_millis(70)),
            );
            self.voices.push(Voice {
                source: PresentationSource::Global,
                channel: SoundChannel::Ui,
                class: None,
                sink: voice,
                scale: 0.15,
            });
        }
    }

    #[cfg(bevy_shell)]
    pub(crate) fn start_title(&mut self) -> Result<(), String> {
        if let Some(bytes) = self.catalog.read_title()? {
            let source = Decoder::try_from(Cursor::new(bytes)).map_err(|e| e.to_string())?;
            let sink = Sink::connect_new(&self.mixer);
            sink.set_volume(0.0);
            sink.append(source);
            self.voices.push(Voice {
                source: PresentationSource::Global,
                channel: SoundChannel::Music,
                class: None,
                sink,
                scale: 1.0,
            });
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some((_, voice)) = self.ambient.take() {
            voice.stop();
        }
        for voice in self.voices.drain(..) {
            voice.sink.stop();
        }
    }

    pub fn status(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn update(&mut self, settings: AudioSettings, cues: Vec<SoundCue>) {
        update_voices(&mut self.voices, settings);
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
            let voice = Sink::connect_new(&self.mixer);
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
            self.voices.remove(0).sink.stop();
        }
        let voice = Sink::connect_new(&self.mixer);
        voice.set_volume(gain);
        // Numeric cues are one-shots. Bound voice lifetime even for malformed/stream assets.
        voice.append(source.take_duration(Duration::from_secs(30)));
        self.voices.push(Voice {
            source: cue.source,
            channel: self.catalog.channel(cue.id),
            class: self.catalog.battle_class(cue.id),
            sink: voice,
            scale: 1.0,
        });
        Ok(())
    }
}

/// cpal's WASAPI host keeps one process-wide device enumerator, created in the COM apartment of
/// the first thread that touches it and released when that thread exits. Creating it on a thread
/// that lives as long as the process keeps every later enumeration and stream open valid, whatever
/// thread (a Bevy task, a test) asks first.
#[cfg(windows)]
fn pin_audio_host() {
    static PINNED: std::sync::Mutex<bool> = std::sync::Mutex::new(false);
    pin_once(&PINNED, || {
        let (ready, wait) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("oteryn-audio-host".into())
            .spawn(move || {
                let _ = rodio::cpal::default_host().output_devices();
                let _ = ready.send(());
                loop {
                    std::thread::park();
                }
            })
            .is_ok()
            && wait.recv().is_ok()
    });
}

/// Runs `pin` until it succeeds once; a failed pin (the host thread could not be spawned)
/// leaves `pinned` unset, so a later call tries again.
#[cfg(any(windows, test))]
fn pin_once(pinned: &std::sync::Mutex<bool>, pin: impl FnOnce() -> bool) {
    let mut pinned = pinned
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !*pinned {
        *pinned = pin();
    }
}

#[cfg(not(windows))]
fn pin_audio_host() {}

/// Device names for the Sound page; saved missing devices remain explicit selections.
pub fn audio_device_names() -> Vec<String> {
    #[cfg(test)]
    verify_audio_thread();
    pin_audio_host();
    rodio::cpal::default_host()
        .output_devices()
        .map(|devices| devices.filter_map(|device| device.name().ok()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;

#[cfg(test)]
thread_local! { static TEST_FRAME: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
#[cfg(test)]
fn verify_audio_thread() {
    assert!(
        !TEST_FRAME.get(),
        "blocking audio operation reached the frame thread"
    );
}
#[cfg(all(test, bevy_shell))]
pub(crate) fn check_audio_frame(frame: impl FnOnce()) {
    TEST_FRAME.set(true);
    frame();
    TEST_FRAME.set(false);
}
