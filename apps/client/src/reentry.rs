//! Window-independent native entry lifecycle.

use crate::play::{PlayLink, PlayView};
use crate::{AdmittedSession, ClientBootstrap, GameplayEntryError};
use oteryn_input_actions::{ButtonState, NormalizedInputEvent};
use oteryn_platform_client::native_login::PublicClass;
use std::sync::mpsc::{self, Receiver, TryRecvError};

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
    client: ClientBootstrap,
    state: EntryState,
    pending: Option<Receiver<Result<AdmittedSession, GameplayEntryError>>>,
}

impl Reentry {
    pub fn new(client: ClientBootstrap) -> Self {
        Self {
            client,
            state: EntryState::Offline,
            pending: None,
        }
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
        // Dropping the sender on a stopped runtime is handled as a public refusal by poll.
        let _task = self.client.runtime.spawn(async move {
            let result = match config {
                Some(config) => crate::native_entry::enter(&config, cancellation).await,
                None => Err(GameplayEntryError::NativeProtocolUnavailable),
            };
            let _ = sender.send(result);
        });
        true
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
        self.pending = None;
        let result = result.and_then(|admitted| self.client.start_play(admitted));
        self.state.finish(result.is_ok());
        Some(result)
    }

    pub fn leave_play(&mut self) {
        if self.state == EntryState::Playing {
            self.state = EntryState::Offline;
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
}
