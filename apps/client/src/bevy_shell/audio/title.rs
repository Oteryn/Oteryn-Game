//! The title stream belongs to its worker. The frame system only publishes settings.
use super::super::owner::Phase;
use crate::play::{AudioOutput, AudioSettings, ReferencePreferences};
use std::sync::mpsc::{self, Receiver, Sender};

struct Worker {
    device: String,
    settings: AudioSettings,
    send: Sender<AudioSettings>,
    errors: Receiver<String>,
}

#[derive(Default)]
pub(super) struct Title {
    worker: Option<Worker>,
    error: Option<String>,
}

impl Title {
    pub(super) fn status(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(super) fn poll(&mut self, phase: Phase, focused: bool, prefs: &ReferencePreferences) {
        if phase != Phase::Offline || !focused || !prefs.audio.anthem {
            self.worker = None;
            self.error = None;
            return;
        }
        if let Some(worker) = &mut self.worker
            && worker.device == prefs.audio_device
        {
            if worker.settings != prefs.audio {
                worker.settings = prefs.audio;
                let _ = worker.send.send(prefs.audio);
            }
            if let Ok(error) = worker.errors.try_recv() {
                self.error = Some(error);
            }
            return;
        }
        self.error = None;
        let device = prefs.audio_device.clone();
        let (send, receive) = mpsc::channel();
        let (errors, report) = mpsc::channel();
        let initial = prefs.audio;
        let result = std::thread::Builder::new()
            .name("client-title-audio".into())
            .spawn(move || {
                let run = || -> Result<(), String> {
                    let mut output = AudioOutput::with_device(&device)?;
                    output.start_title()?;
                    // A closed sender cancels a load that finished after admission.
                    let mut settings = initial;
                    loop {
                        match receive.try_recv() {
                            Ok(next) => settings = next,
                            Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
                            Err(mpsc::TryRecvError::Empty) => {
                                output.update(settings, Vec::new());
                                let Ok(next) = receive.recv() else {
                                    return Ok(());
                                };
                                settings = next;
                            }
                        }
                    }
                };
                if let Err(error) = run() {
                    let _ = errors.send(error);
                }
            });
        if let Err(error) = result {
            self.error = Some(error.to_string());
        }
        self.worker = Some(Worker {
            device: prefs.audio_device.clone(),
            settings: initial,
            send,
            errors: report,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_anthem_worker_follows_preference_focus_and_admission() {
        let mut title = Title::default();
        let mut prefs = ReferencePreferences::default();
        for (phase, focused, enabled, active) in [
            (Phase::Offline, true, false, false),
            (Phase::Offline, true, true, true),
            (Phase::Playing, true, true, false),
            (Phase::Offline, false, true, false),
            (Phase::Offline, true, true, true),
            (Phase::Offline, true, false, false),
        ] {
            prefs.audio.anthem = enabled;
            title.poll(phase, focused, &prefs);
            assert_eq!(title.worker.is_some(), active);
        }
    }
}
