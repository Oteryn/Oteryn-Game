//! Window-independent native entry lifecycle.
mod device;

use crate::play::{PlayLink, PlayView};
use crate::{AdmittedSession, ClientBootstrap, GameplayEntryError};
use oteryn_client_runtime::connection::AdmissionWait;
use oteryn_client_ui::login_queue::QueueWait;
use oteryn_foundation::{Cancellable, CancellationToken, cancellable};
use oteryn_input_actions::{ButtonState, NormalizedInputEvent};
use oteryn_platform_client::native_login::PublicClass;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryState {
    Offline,
    Entering,
    Playing,
}

impl EntryState {
    fn begin(&mut self) -> bool {
        if *self != Self::Offline {
            return false;
        }
        *self = Self::Entering;
        true
    }

    fn finish(&mut self, admitted: bool) {
        *self = if admitted {
            Self::Playing
        } else {
            Self::Offline
        };
    }
}

/// One attempt at a time on the existing runtime. Every request starts native entry afresh;
/// no credential, ticket, grant or admitted stream is retained for the next attempt.
pub struct Reentry {
    bulletin: Option<String>,
    client: ClientBootstrap,
    state: EntryState,
    attempt_cancellation: Option<CancellationToken>,
    progress: Option<tokio::sync::watch::Receiver<Option<AdmissionWait>>>,
    waiting: Option<(AdmissionWait, QueueWait)>,
    last_wait_text: Option<String>,
    pending: Option<Receiver<Result<AdmittedSession, GameplayEntryError>>>,
    training_completion: Option<Box<dyn Fn() -> bool + Send>>,
}

impl Reentry {
    /// Whether native admission is pending, including a queued training character.
    pub fn is_entering(&self) -> bool {
        self.state == EntryState::Entering
    }

    pub fn take_bulletin(&mut self) -> Option<String> {
        self.bulletin.take()
    }
    pub fn new(client: ClientBootstrap) -> Self {
        Self {
            bulletin: None,
            client,
            state: EntryState::Offline,
            attempt_cancellation: None,
            progress: None,
            waiting: None,
            last_wait_text: None,
            pending: None,
            training_completion: None,
        }
    }

    /// Applies edited settings only while offline; the runtime is reused.
    pub fn configure(&mut self, mut config: crate::NativeLoginConfig) -> bool {
        if self.state != EntryState::Offline {
            return false;
        }
        if let Some(previous) = &self.client.native_login
            && previous.platform_url == config.platform_url
            && previous.oauth_client_id == config.oauth_client_id
        {
            config.device_sessions = previous.device_sessions.clone();
        }
        self.client.native_login = Some(config);
        true
    }

    /// Starts a user-requested attempt without blocking the window. Existing native entry
    /// owns bounded admission retries; there is no automatic outer retry loop.
    pub fn request(&mut self) -> bool {
        if !self.state.begin() {
            return false;
        }
        let (sender, receiver) = mpsc::channel();
        self.pending = Some(receiver);
        let config = self.client.native_login.clone();
        let cancellation = self.client.runtime.cancellation();
        let attempt_cancellation = CancellationToken::new();
        self.attempt_cancellation = Some(attempt_cancellation.clone());
        let (progress, updates) = tokio::sync::watch::channel(None);
        self.progress = Some(updates);
        self.waiting = None;
        self.last_wait_text = None;
        // Dropping the sender on a stopped runtime is handled as a public refusal by poll.
        let _task = self.client.runtime.spawn(async move {
            let work = async {
                match config {
                    Some(config) => {
                        crate::native_entry::enter_observed(
                            &config,
                            attempt_cancellation.clone(),
                            Some(progress),
                        )
                        .await
                    }
                    None => Err(GameplayEntryError::NativeProtocolUnavailable),
                }
            };
            let result = match cancellable(
                cancellation,
                cancellable(attempt_cancellation.clone(), work),
            )
            .await
            {
                Cancellable::Completed(Cancellable::Completed(result)) => result,
                _ => Err(GameplayEntryError::Rejected(
                    PublicClass::SessionUnavailable,
                )),
            };
            let _ = sender.send(result);
        });
        true
    }

    /// Emits only changed countdown text, so a slow form cannot accumulate frame-rate updates.
    pub fn poll_wait(&mut self, now: Instant) -> Option<String> {
        if self.state != EntryState::Entering {
            return None;
        }
        let text = if self
            .attempt_cancellation
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
        {
            tr!("Cancelling sign-in…").to_owned()
        } else {
            let wait = (*self.progress.as_mut()?.borrow_and_update())?;
            if self
                .waiting
                .as_ref()
                .is_none_or(|(held, _)| held.started != wait.started)
            {
                self.waiting = Some((
                    wait,
                    QueueWait::new(wait.queue_position, wait.started, wait.retry_in),
                ));
            }
            let (_, queue) = self.waiting.as_ref()?;
            if wait.code == 1115 {
                queue.text(now)
            } else {
                lfmt!(
                    "Admission not ready. Retrying in {}s.",
                    queue.remaining_seconds(now)
                )
            }
        };
        if self.last_wait_text.as_ref() == Some(&text) {
            return None;
        }
        self.last_wait_text = Some(text.clone());
        Some(text)
    }

    /// Both initial admission and subsequent admissions start play through this same path.
    pub fn poll(&mut self) -> Option<Result<(PlayView, PlayLink), GameplayEntryError>> {
        let result = match self.pending.as_ref()?.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err(GameplayEntryError::Rejected(
                PublicClass::SessionUnavailable,
            )),
        };
        // Cancel and poll are serialized by the UI owner. A completed worker may have
        // queued its result just before cancellation; it must not start play afterward.
        let result = if self
            .attempt_cancellation
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
        {
            Err(GameplayEntryError::Rejected(
                PublicClass::SessionUnavailable,
            ))
        } else {
            result
        };
        self.pending = None;
        self.attempt_cancellation = None;
        self.progress = None;
        self.waiting = None;
        self.last_wait_text = None;
        let result = result.and_then(|admitted| {
            let bulletin = admitted.bulletin().map(str::to_owned);
            let play = self.client.start_play(admitted)?;
            self.bulletin = bulletin;
            Ok(play)
        });
        self.training_completion = result
            .as_ref()
            .ok()
            .map(|(_, link)| Box::new(link.training_completion()) as Box<dyn Fn() -> bool + Send>);
        if result.is_err()
            && let Some(config) = &self.client.native_login
        {
            config.training_queue.clear();
        }
        self.state.finish(result.is_ok());
        Some(result)
    }

    pub fn leave_play(&mut self) {
        if self.state == EntryState::Playing {
            self.state = EntryState::Offline;
            let confirmed = self
                .training_completion
                .take()
                .is_some_and(|completed| completed());
            if self
                .client
                .native_login
                .as_ref()
                .is_some_and(|config| config.training_queue.finish_current(confirmed))
            {
                self.request();
            }
        }
    }

    pub fn shutdown(self) {
        self.client.shutdown();
    }
}

/// Enter and keypad Enter, pressed once without modifiers, in the offline scene.
pub fn login_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| {
        matches!(event,
        NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if matches!(code.get(), 40 | 88) && *modifiers == oteryn_input_actions::Modifiers::NONE)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_input_actions::{KeyCode, Modifier, Modifiers};

    #[test]
    fn offline_entering_admitted_end_and_repeat() {
        let mut state = EntryState::Offline;
        for _ in 0..2 {
            assert!(state.begin());
            assert_eq!(state, EntryState::Entering);
            assert!(!state.begin());
            state.finish(true);
            assert_eq!(state, EntryState::Playing);
            assert!(!state.begin());
            state = EntryState::Offline;
        }
    }

    #[test]
    fn refusal_returns_offline_and_allows_another_attempt() {
        let mut state = EntryState::Offline;
        for _ in 0..2 {
            assert!(state.begin());
            state.finish(false);
            assert_eq!(state, EntryState::Offline);
        }
    }

    #[test]
    fn login_requires_an_unmodified_fresh_enter_press()
    -> Result<(), oteryn_input_actions::InputError> {
        for code in [40, 88] {
            let event = NormalizedInputEvent::Key {
                code: KeyCode::new(code)?,
                state: ButtonState::Pressed,
                modifiers: Modifiers::NONE,
                repeat: false,
            };
            assert!(login_key(std::slice::from_ref(&event)));
            for changed in [
                NormalizedInputEvent::Key {
                    code: KeyCode::new(code)?,
                    state: ButtonState::Released,
                    modifiers: Modifiers::NONE,
                    repeat: false,
                },
                NormalizedInputEvent::Key {
                    code: KeyCode::new(code)?,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::NONE,
                    repeat: true,
                },
                NormalizedInputEvent::Key {
                    code: KeyCode::new(code)?,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::one(Modifier::Control),
                    repeat: false,
                },
            ] {
                assert!(!login_key(&[changed]));
            }
        }
        Ok(())
    }
    #[test]
    fn admission_progress_counts_down_once_and_cancels_only_this_attempt()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut entry = Reentry::new(ClientBootstrap::new()?);
        assert!(!entry.cancel_entry());
        entry.state = EntryState::Entering;
        let token = CancellationToken::new();
        entry.attempt_cancellation = Some(token.clone());
        let (progress, updates) = tokio::sync::watch::channel(None);
        entry.progress = Some(updates);
        let now = Instant::now();
        progress.send_replace(Some(AdmissionWait {
            code: 1115,
            queue_position: None,
            started: now,
            retry_in: std::time::Duration::from_secs(2),
        }));
        assert_eq!(
            entry.poll_wait(now).as_deref(),
            Some("Waiting for capacity. Retrying in 2s.")
        );
        assert!(entry.poll_wait(now).is_none());
        assert_eq!(
            entry
                .poll_wait(now + std::time::Duration::from_secs(1))
                .as_deref(),
            Some("Waiting for capacity. Retrying in 1s.")
        );
        assert!(entry.cancel_entry());
        assert!(token.is_cancelled());
        assert!(!entry.client.runtime.cancellation().is_cancelled());
        assert!(!entry.cancel_entry());
        entry.shutdown();
        Ok(())
    }
    #[test]
    fn cancel_before_poll_overrides_an_already_queued_admission_result()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut entry = Reentry::new(ClientBootstrap::new()?);
        entry.state = EntryState::Entering;
        entry.attempt_cancellation = Some(CancellationToken::new());
        let (sender, receiver) = mpsc::channel();
        entry.pending = Some(receiver);
        assert!(
            sender
                .send(Err(GameplayEntryError::Rejected(
                    PublicClass::TemporarilyUnavailable
                )))
                .is_ok()
        );
        assert!(entry.cancel_entry());
        let result = entry.poll().ok_or("queued result missing")?;
        assert_eq!(
            result.as_ref().err().copied(),
            Some(GameplayEntryError::Rejected(
                PublicClass::SessionUnavailable
            ))
        );
        entry.shutdown();
        Ok(())
    }
}

#[cfg(test)]
#[path = "features/0054-training/tests/reentry.rs"]
mod training_batch_tests;
