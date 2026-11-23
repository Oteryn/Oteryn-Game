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
    retry_after: Option<std::time::Instant>,
}

impl Title {
    pub(super) fn status(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub(super) fn poll(&mut self, phase: Phase, focused: bool, prefs: &ReferencePreferences) {
        self.poll_with(phase, focused, prefs, |device, errors| {
            if std::env::var_os("OTERYN_SOUND_MANIFEST").is_none() {
                return Ok(None);
            }
            let mut output = AudioOutput::open_reporting(device, errors)?;
            output.start_title()?;
            Ok(Some(move |settings| output.update(settings, Vec::new())))
        });
    }

    fn poll_with<F: FnMut(AudioSettings)>(
        &mut self,
        phase: Phase,
        focused: bool,
        prefs: &ReferencePreferences,
        open: impl FnOnce(&str, mpsc::SyncSender<String>) -> Result<Option<F>, String> + Send + 'static,
    ) {
        if phase != Phase::Offline || !focused || !prefs.audio.anthem {
            self.retry_after = None;
            self.worker = None;
            self.error = None;
            return;
        }
        if let Some(worker) = &mut self.worker
            && worker.device == prefs.audio_device
            && self
                .retry_after
                .is_none_or(|when| std::time::Instant::now() < when)
        {
            if worker.settings != prefs.audio {
                worker.settings = prefs.audio;
                let _ = worker.send.send(prefs.audio);
            }
            if self.retry_after.is_none()
                && let Ok(error) = worker.errors.try_recv()
            {
                self.retry_after =
                    Some(std::time::Instant::now() + std::time::Duration::from_secs(1));
                self.error = Some(error);
            }
            return;
        }
        self.error = None;
        self.retry_after = None;
        let device = prefs.audio_device.clone();
        let (send, receive) = mpsc::channel();
        let (errors, report) = mpsc::sync_channel(1);
        let initial = prefs.audio;
        let result = std::thread::Builder::new()
            .name("client-title-audio".into())
            .spawn(move || {
                let run = || -> Result<(), String> {
                    let Some(mut update) = open(&device, errors.clone())? else {
                        return Ok(());
                    };
                    // A closed sender cancels a load that finished after admission.
                    let mut settings = initial;
                    loop {
                        match receive.try_recv() {
                            Ok(next) => settings = next,
                            Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
                            Err(mpsc::TryRecvError::Empty) => {
                                update(settings);
                                let Ok(next) = receive.recv() else {
                                    return Ok(());
                                };
                                settings = next;
                            }
                        }
                    }
                };
                if let Err(error) = run() {
                    let _ = errors.try_send(error);
                }
            });
        if let Err(error) = result {
            self.retry_after = Some(std::time::Instant::now() + std::time::Duration::from_secs(1));
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
    #[test]
    fn title_reopens_after_transient_output_failure_without_focus_change()
    -> Result<(), Box<dyn std::error::Error>> {
        let prefs = ReferencePreferences::default();
        let mut title = Title::default();
        let (started, running) = mpsc::channel();
        title.poll_with(Phase::Offline, true, &prefs, move |_, report| {
            let callback = AudioOutput::device_error_callback(report);
            let mut ready = Some((started, callback));
            Ok(Some(move |_| {
                if let Some((tx, callback)) = ready.take() {
                    assert!(tx.send(callback).is_ok());
                }
            }))
        });
        let failure = rodio::cpal::StreamError::DeviceNotAvailable;
        let message = failure.to_string();
        running.recv_timeout(std::time::Duration::from_secs(5))?(failure);
        title.poll(Phase::Offline, true, &prefs);
        assert_eq!(title.status(), Some(message.as_str()));
        std::thread::sleep(std::time::Duration::from_millis(1100));
        title.poll(Phase::Offline, true, &prefs);
        assert!(title.status().is_none());
        Ok(())
    }
}
