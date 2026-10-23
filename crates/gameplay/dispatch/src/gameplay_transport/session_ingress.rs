//! One command stream per GameSession, retained by the Channel across transport loss.
//!
//! Family writers still own their atomic durable effects. An uncertain pending command
//! is never re-executed here: without terminal truth that session cannot resume.
use super::connection::AdmittedSession;
use crate::foundation::{
    CommandId, CommandIngress, CommandSemanticIdentity, DuplicateDisposition,
    FoundationProtocolError, GameSessionId, IngressDecision, NormalizedSemanticIntentIdentity,
    RetainedBindingIdentity, TerminalSemanticOutcome,
};
use base64::Engine;
use oteryn_protocol_oteryn::command_replay::{MAX_RETAINED_RESULT_BYTES, encode_duplicate_replay};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct SessionIngress {
    ingress: CommandIngress,
    retained: Option<Vec<u8>>,
    result_sequence_floor: u64,
    active: bool,
}

/// Owned alongside the Channel's admitted sessions, forgotten only at retirement.
#[derive(Default)]
pub struct SessionCommands(Mutex<HashMap<GameSessionId, Arc<Mutex<SessionIngress>>>>);

impl SessionCommands {
    pub(super) fn admit(&self, session: GameSessionId) {
        if let Ok(mut commands) = self.0.lock() {
            commands.entry(session).or_default();
        }
    }
    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut commands) = self.0.lock() {
            commands.remove(&session);
        }
    }

    pub(super) fn resumable(&self, admitted: &AdmittedSession) -> bool {
        self.0.lock().is_ok_and(|commands| {
            commands
                .get(&admitted.game_session_id)
                .is_some_and(|state| {
                    state.lock().is_ok_and(|state| {
                        !state.active
                            && state.ingress.outstanding() == 0
                            && state.ingress.next_command_id().map(CommandId::get)
                                == Some(admitted.continuity.next_command_id)
                            && state.result_sequence_floor <= admitted.continuity.server_sequence
                            && (state.ingress.retained_count() == 0 || state.retained.is_some())
                    })
                })
        })
    }
}

pub(super) enum CommandAction {
    Execute,
    Pending,
    Replay(Vec<u8>),
}

/// The connection borrows its session's stream; cancellation releases the active claim
/// while preserving reservations and terminal truth in the owning Channel.
pub(super) struct SessionCommandStream {
    state: Option<Arc<Mutex<SessionIngress>>>,
    next: u64,
}

fn semantic(command: CommandId) -> Option<CommandSemanticIdentity> {
    // FND-02 §13.4: the reserved CommandId is the operation identity. Replacement
    // bytes are never decoded or compared with a new interpretation of that operation.
    Some(CommandSemanticIdentity::new(
        NormalizedSemanticIntentIdentity::new("session", command.get(), "command", 0).ok()?,
        RetainedBindingIdentity::new("session-owned", "reserved-command").ok()?,
    ))
}

impl SessionCommandStream {
    pub(super) fn open(
        owner: Option<&SessionCommands>,
        admitted: &AdmittedSession,
    ) -> Option<Self> {
        let session = admitted.game_session_id;
        let next = admitted.continuity.next_command_id;
        if owner.is_none() && !cfg!(test) {
            return None;
        }
        let state = if let Some(owner) = owner {
            let mut commands = owner.0.lock().ok()?;
            // Missing high-water/terminal truth must never be reconstructed from a counter.
            if next != 1 && !commands.contains_key(&session) {
                return None;
            }
            let entry = Arc::clone(commands.entry(session).or_default());
            let mut state = entry.lock().ok()?;
            if state.active || state.ingress.outstanding() != 0 {
                return None;
            }
            if state.ingress.next_command_id().map(CommandId::get) != Some(next)
                || state.result_sequence_floor > admitted.continuity.server_sequence
                || (state.ingress.retained_count() != 0 && state.retained.is_none())
            {
                return None;
            }
            // If recovery ever detects corrupt truth, keep the slot unavailable.
            state.active = true;
            state.ingress =
                CommandIngress::recover(std::mem::take(&mut state.ingress).into_recovery_state())
                    .ok()?;
            drop(state);
            Some(entry)
        } else {
            None
        };
        Some(Self { state, next })
    }

    pub(super) const fn next_command_id(&self) -> u64 {
        self.next
    }

    pub(super) fn begin(
        &mut self,
        command: u64,
        generation: u64,
        sequence: u64,
        buffered_original: bool,
    ) -> Result<CommandAction, (FoundationProtocolError, u64)> {
        use FoundationProtocolError::{CommandOutcomeExpired, CommandSequenceGap};
        let unavailable = (CommandOutcomeExpired, 0);
        sequence.checked_add(1).ok_or(unavailable)?;
        let Some(state) = &self.state else {
            // Transport-only injected authorities have no owning session directory. Keep
            // their historical non-resumable counter semantics; production always supplies it.
            return if command == self.next {
                Ok(CommandAction::Execute)
            } else if command < self.next {
                Err(unavailable)
            } else {
                Err((CommandSequenceGap, self.next))
            };
        };
        let mut state = state.lock().map_err(|_| unavailable)?;
        let id = CommandId::new(command).map_err(|_| unavailable)?;
        let identity = semantic(id).ok_or(unavailable)?;
        match state.ingress.classify_duplicate(id, &identity) {
            DuplicateDisposition::PendingOriginal => {
                return Ok(if buffered_original {
                    CommandAction::Execute
                } else {
                    CommandAction::Pending
                });
            }
            DuplicateDisposition::ReplayRetainedOutcome(_) => {
                let frame = encode_duplicate_replay(
                    generation,
                    sequence.checked_add(1).ok_or(unavailable)?,
                    state.retained.as_deref().ok_or(unavailable)?,
                )
                .map_err(|_| unavailable)?;
                state.result_sequence_floor = sequence + 1;
                return Ok(CommandAction::Replay(frame));
            }
            DuplicateDisposition::ConflictChangedInput | DuplicateDisposition::OutcomeExpired => {
                return Err(unavailable);
            }
            DuplicateDisposition::NotDuplicate => {}
        }
        match state.ingress.reserve(id, identity) {
            IngressDecision::Reserved(_) => {
                // Exhaustion is refused before domain execution; the namespace never wraps.
                self.next = state.ingress.next_command_id().ok_or(unavailable)?.get();
                Ok(CommandAction::Execute)
            }
            IngressDecision::SequenceGap { expected } => Err((CommandSequenceGap, expected.get())),
            IngressDecision::TooManyOutstanding { expected } => Err((
                FoundationProtocolError::TooManyOutstandingCommands,
                expected.get(),
            )),
            IngressDecision::AlreadyReserved(_) | IngressDecision::CommandSpaceExhausted => {
                Err(unavailable)
            }
        }
    }

    pub(super) fn terminalize(&mut self, command: u64, result: &[u8]) -> Option<()> {
        if let Some(state) = &self.state {
            if result.len() > MAX_RETAINED_RESULT_BYTES {
                return None;
            }
            let envelope = oteryn_protocol_oteryn::decode_wire_envelope(result).ok()?;
            let digest = base64::engine::general_purpose::STANDARD.encode(Sha256::digest(result));
            let outcome =
                TerminalSemanticOutcome::new("command-result", &digest, envelope.server_sequence())
                    .ok()?;
            let mut state = state.lock().ok()?;
            let SessionIngress {
                ingress,
                retained,
                result_sequence_floor,
                ..
            } = &mut *state;
            ingress
                .terminalize(CommandId::new(command).ok()?, outcome, || {
                    *retained = Some(result.to_vec());
                    *result_sequence_floor = envelope.server_sequence();
                })
                .ok()?;
        } else {
            self.next = self.next.checked_add(1)?;
        }
        Some(())
    }
}

impl Drop for SessionCommandStream {
    fn drop(&mut self) {
        if let Some(state) = &self.state
            && let Ok(mut state) = state.lock()
        {
            state.active = false;
        }
    }
}

#[cfg(test)]
mod tests;
