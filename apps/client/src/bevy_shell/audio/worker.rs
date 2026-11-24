//! A stream and its pinned asset reader live on one playback thread.
use super::{AudioOutput, AudioSettings, Output, SoundCue, UiSound};
use std::sync::{Arc, Mutex, mpsc};

#[derive(Default)]
struct Pending {
    settings: Option<AudioSettings>,
    cues: Vec<SoundCue>,
    ui: Vec<UiSound>,
    stop: bool,
    closed: bool,
}

pub(super) struct WorkerOutput {
    pending: Arc<Mutex<Pending>>,
    wake: mpsc::SyncSender<()>,
    failures: mpsc::Receiver<String>,
    status: mpsc::Receiver<String>,
    error: Option<String>,
    settings: Option<AudioSettings>,
}

impl WorkerOutput {
    pub(super) fn open(device: &str) -> Result<Self, String> {
        Self::start(device.to_owned(), AudioOutput::open_reporting)
    }

    pub(super) fn start<O: Output>(
        device: String,
        open: impl FnOnce(&str, mpsc::SyncSender<String>) -> Result<O, String> + Send + 'static,
    ) -> Result<Self, String> {
        let pending = Arc::new(Mutex::new(Pending::default()));
        let shared = Arc::clone(&pending);
        let (wake, receive) = mpsc::sync_channel(1);
        let (report, failures) = mpsc::sync_channel(1);
        let (status_tx, status) = mpsc::channel();
        std::thread::Builder::new()
            .name("client-game-audio".into())
            .spawn(move || {
                let mut output = match open(&device, report.clone()) {
                    Ok(output) => output,
                    Err(error) => {
                        let _ = report.try_send(error);
                        return;
                    }
                };
                let mut last_error = None;
                while receive.recv().is_ok() {
                    let batch = {
                        let mut pending = shared
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        if pending.closed {
                            break;
                        }
                        std::mem::take(&mut *pending)
                    };
                    if batch.stop {
                        output.stop();
                    }
                    if let Some(settings) = batch.settings {
                        output.update(settings, batch.cues, batch.ui);
                        if output.status() != last_error.as_deref() {
                            last_error = output.status().map(str::to_owned);
                            if let Some(error) = &last_error {
                                let _ = status_tx.send(error.clone());
                            }
                        }
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            pending,
            wake,
            failures,
            status,
            error: None,
            settings: None,
        })
    }
}

impl Output for WorkerOutput {
    fn update(&mut self, settings: AudioSettings, cues: Vec<SoundCue>, ui: Vec<UiSound>) {
        if let Ok(error) = self.status.try_recv() {
            self.error = Some(error);
        }
        if self.settings == Some(settings) && cues.is_empty() && ui.is_empty() {
            return;
        }
        self.settings = Some(settings);
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        pending.settings = Some(settings);
        for cue in cues {
            if cue.ambient {
                pending.cues.retain(|old| !old.ambient);
            }
            if pending.cues.len() == 32
                && let Some(old) = pending.cues.iter().position(|old| !old.ambient)
            {
                pending.cues.remove(old);
            }
            pending.cues.push(cue);
        }
        let room = 32usize.saturating_sub(pending.ui.len());
        pending.ui.extend(ui.into_iter().take(room));
        let _ = self.wake.try_send(());
    }
    fn stop(&mut self) {
        self.settings = None;
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *pending = Pending {
            stop: true,
            ..Default::default()
        };
        let _ = self.wake.try_send(());
    }
    fn status(&self) -> Option<&str> {
        self.error.as_deref()
    }
    fn failure(&mut self) -> Option<String> {
        self.failures.try_recv().ok()
    }
}

impl Drop for WorkerOutput {
    fn drop(&mut self) {
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .closed = true;
        let _ = self.wake.try_send(());
    }
}
