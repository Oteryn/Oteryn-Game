//! The actor-end save of monk Harmony and the remaining forced Serene time
//! (`OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1` §8.2, SPELL-D8 save point 1).
//!
//! Before the Character lease is released, the Channel owner writes the actor's live values with
//! `commit_character_monk_state_save`, fenced by the current durable GameSession that owns the
//! actor. The write must commit or be fenced out before the release; while its outcome is unknown
//! the release waits. It advances the CharacterRevision, so it runs in the Character's revision
//! slot (CHAR-REV-SEQ-1); the runtime lock is taken only briefly, inside the slot, to read the
//! values, and never across the slot wait or the durable write.

use oteryn_simulation_determinism::SemanticTimeMicros;

use super::connection::AdmittedSession;
use super::{ComposedFreshAdmission, RECONCILE_ATTEMPTS, RECONCILE_BACKOFF, SecureIdentifiers};
use crate::domain::CharacterId;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveOutcome, MonkStateSaveRequest,
};
use crate::foundation::{ExactActorRef, GameSessionState};

/// The result of the actor-end save; only `Unknown` holds the lease release back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MonkSave {
    /// Nothing to save: the actor has no spell state or is not a monk.
    NotApplicable,
    /// The values are durable: committed, already committed, or already stored.
    Saved,
    /// The current authority refused the write, which wrote nothing (§8.2: a write whose fence
    /// is stale writes nothing), or a death's respawn is pending and its zeros stand.
    FencedOut,
    /// The outcome could not be proven; the release must wait.
    Unknown,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The owner time of the Channel runtime (spell cast §2): process-local and monotonic.
    pub(super) fn owner_now(&self) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(
            u64::try_from(self.clock_origin.elapsed().as_micros()).unwrap_or(u64::MAX),
        )
    }

    /// The durable monk values a new runtime actor of `character_id` loads (§8.2). `None` when
    /// they cannot be read or are corrupt: the actor then fails closed.
    pub(super) async fn load_monk_state(&self, character_id: CharacterId) -> Option<(u8, u64)> {
        let state = self
            .root
            .read_character_monk_state(self.character, character_id)
            .await
            .ok()?;
        Some((state.harmony(), state.serene_forced_remaining_micros()))
    }

    /// Save point 1 of §8.2 for the ending actor of `admitted`. Each attempt reads the current
    /// fence from the durable GameSession and the Character root and issues a fresh occurrence.
    /// A lost response is reconciled by its occurrence before another attempt, so a committed
    /// save is never mistaken for a failed one.
    pub(super) async fn save_monk_state(
        &self,
        admitted: &AdmittedSession,
        actor: ExactActorRef,
    ) -> MonkSave {
        for attempt in 0..RECONCILE_ATTEMPTS {
            if attempt > 0 {
                tokio::time::sleep(RECONCILE_BACKOFF).await;
            }
            let fence = match self.current_monk_fence(admitted).await {
                Ok(Some(fence)) => fence,
                // The session no longer owns the actor: any write would be fenced out.
                Ok(None) => return MonkSave::FencedOut,
                Err(()) => continue,
            };
            let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
            let values = {
                let runtime = self.runtime.lock().await;
                self.spell_states.lock().await.monk_save_values(
                    &runtime,
                    actor,
                    admitted.game_session_id,
                    self.owner_now(),
                )
            };
            let Some((harmony, micros)) = values else {
                return MonkSave::NotApplicable;
            };
            let Ok(state) = DurableMonkState::new(harmony, micros) else {
                return MonkSave::Unknown;
            };
            let Some(occurrence) = SecureIdentifiers::random::<16>().and_then(|mut bytes| {
                bytes[6] = 0x70 | (bytes[6] & 0x0f);
                bytes[8] = 0x80 | (bytes[8] & 0x3f);
                MonkStateSaveOccurrence::from_bytes(bytes).ok()
            }) else {
                continue;
            };
            let request = MonkStateSaveRequest { occurrence, state };
            match slot
                .commit_monk_state_save(self.root, self.character, self.holder, fence, request)
                .await
            {
                Ok(
                    MonkStateSaveOutcome::Committed(_)
                    | MonkStateSaveOutcome::AlreadyCommitted(_)
                    | MonkStateSaveOutcome::Unchanged,
                ) => return MonkSave::Saved,
                Err(
                    CharacterProgressionError::AuthorityRejected
                    | CharacterProgressionError::RespawnPending,
                ) => return MonkSave::FencedOut,
                // Under the slot only a writer that bypassed the sequencer moves the revision:
                // the save's binding includes it, so it fails closed (the slot reports the
                // defect) and is not retried at another revision.
                Err(CharacterProgressionError::CharacterRevisionMismatch) => {
                    return MonkSave::Unknown;
                }
                Err(CharacterProgressionError::Unavailable(_)) => {
                    if let Ok(Some(_)) = self
                        .root
                        .reconcile_character_monk_state_save(self.character, occurrence)
                        .await
                    {
                        return MonkSave::Saved;
                    }
                }
                Err(_) => return MonkSave::Unknown,
            }
        }
        MonkSave::Unknown
    }

    /// The gameplay fence of the session that owns the actor, from current durable reads. Its
    /// expected revision is replaced by the revision slot's cursor at commit.
    /// `Ok(None)` when that session is terminal; `Err` when a read failed.
    pub(super) async fn current_monk_fence(
        &self,
        admitted: &AdmittedSession,
    ) -> Result<Option<CurrentCharacterGameplayFence>, ()> {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let (session, _) = store
            .current_session_at(admitted.game_session_id)
            .await
            .map_err(|_| ())?;
        if session.session_state() == GameSessionState::Terminal {
            return Ok(None);
        }
        let character_id =
            CharacterId::from_bytes(*session.commit().character_id().as_bytes()).map_err(|_| ())?;
        let record = self
            .root
            .read_current_character(self.character, character_id)
            .await
            .map_err(|_| ())?;
        Ok(Some(CurrentCharacterGameplayFence {
            character_id,
            game_session_id: admitted.game_session_id,
            connection_generation: session.current_connection_generation(),
            character_lease_generation: session.current_character_lease().generation(),
            runtime_scope: session.current_runtime_scope(),
            scope_ownership_generation: session.current_scope_generation(),
            expected_character_revision: record.revision,
        }))
    }
}
