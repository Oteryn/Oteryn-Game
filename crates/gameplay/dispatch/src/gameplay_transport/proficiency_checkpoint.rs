//! PROFICIENCY0 §4.3: one retained training receipt for all dirty weapon tracks.
use std::collections::BTreeMap;
use std::sync::Arc;

use super::ComposedFreshAdmission;
use super::actor_spell::TrainingSave;
use super::weapon_proficiency::WeaponProficiencyDispatch;
use crate::durability::character_proficiency::{
    CommittedProficiencyChange, DurableProficiencyState, ProficiencyCause,
    ProficiencyChangeRequest, ProficiencyCommitOutcome, ProficiencyDefinitions,
    ProficiencyLineCandidate, ProficiencyOccurrence, StoredProficiencyTrack,
};
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::foundation::{ExactActorRef, GameSessionId};

#[derive(Debug, Clone)]
pub(super) struct ProficiencyCheckpoint {
    fence: CurrentCharacterGameplayFence,
    request: ProficiencyChangeRequest,
    deltas: BTreeMap<String, u32>,
}

impl WeaponProficiencyDispatch {
    fn acknowledge_checkpoint(
        &mut self,
        retained: ProficiencyCheckpoint,
        receipt: CommittedProficiencyChange,
        catalogue: &super::weapon_proficiency::ProductionProficiencyCatalogue,
    ) -> bool {
        if !self
            .checkpoint
            .as_ref()
            .is_some_and(|current| current.request == retained.request)
        {
            return false;
        }
        for (key, delta) in retained.deltas {
            if let Some(pending) = self.pending_progress.get_mut(&key) {
                *pending = pending.saturating_sub(delta);
            }
        }
        self.pending_progress.retain(|_, delta| *delta != 0);
        self.checkpoint = None;
        if self.pending_progress.is_empty() {
            self.dirty_since = None;
        }
        for line in receipt.lines {
            let stored = StoredProficiencyTrack {
                state: line.after().clone(),
                committed_character_revision: receipt.committed_character_revision,
                last_proficiency_occurrence_id: receipt.occurrence_id,
            };
            if let Some(track) = catalogue.live_track(&stored) {
                self.install_track(stored.state.item_key(), track);
            }
        }
        true
    }

    pub(super) fn checkpoint_due(&self, now: u64, force: bool) -> bool {
        self.checkpoint.is_some()
            || (!self.pending_progress.is_empty()
                && (force
                    || self
                        .dirty_since
                        .is_some_and(|at| now.saturating_sub(at) >= 60_000_000)
                    || self.pending_progress.iter().any(|(key, delta)| {
                        self.tracks.get(key).is_some_and(|track| {
                            let shape = track.shape.track_shape();
                            shape.level(track.progress)
                                > shape.level(track.progress.saturating_sub(*delta))
                        })
                    })))
    }

    fn prepare_checkpoint(
        &mut self,
        fence: CurrentCharacterGameplayFence,
        stored: &[StoredProficiencyTrack],
        definitions: &dyn ProficiencyDefinitions,
    ) -> Option<ProficiencyCheckpoint> {
        let mut lines = Vec::new();
        for (key, delta) in &self.pending_progress.clone() {
            let resolved = definitions.resolve_weapon(key)?;
            let before = match stored.iter().find(|track| track.state.item_key() == key) {
                Some(track) => track.state.clone(),
                None => DurableProficiencyState::new(
                    key.clone(),
                    resolved.definition_key,
                    resolved.definition_revision,
                    0,
                    vec![None; usize::from(resolved.shape.track_shape().perk_levels())],
                )
                .ok()?,
            };
            let progress = resolved
                .shape
                .track_shape()
                .credited(u32::try_from(before.progress()).ok()?, *delta);
            if u64::from(progress) == before.progress() {
                self.pending_progress.remove(key);
                if let Some(track) = self.tracks.get_mut(key) {
                    track.progress = progress;
                }
                continue;
            }
            let after = DurableProficiencyState::new(
                key.clone(),
                before.definition_key(),
                before.definition_revision(),
                u64::from(progress),
                before.selections().to_vec(),
            )
            .ok()?;
            lines.push(
                ProficiencyLineCandidate::new(ProficiencyCause::Training, before, after).ok()?,
            );
        }
        if lines.is_empty() {
            return None;
        }
        let mut bytes = super::SecureIdentifiers::random::<16>()?;
        bytes[6] = (bytes[6] & 15) | 0x70;
        bytes[8] = (bytes[8] & 63) | 0x80;
        Some(ProficiencyCheckpoint {
            fence,
            request: ProficiencyChangeRequest::new(
                ProficiencyOccurrence::from_bytes(bytes).ok()?,
                ProficiencyCause::Training,
                lines,
                None,
                definitions.digest(),
            )
            .ok()?,
            deltas: self.pending_progress.clone(),
        })
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn save_weapon_proficiency(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        force: bool,
    ) -> TrainingSave {
        if force && self.proficiency_catalogue.is_some() {
            let queue = self.states.attack.lock().await.kills().clone();
            for principal in queue.proficiency_principals(session) {
                // A settled level-up can call this saver with force=false. Boxing this drain
                // keeps that async call cycle finite without holding a runtime/revision lock.
                if Box::pin(self.drain_kill_rewards(principal)).await
                    != super::kill_reward::DrainOutcome::Drained
                {
                    return TrainingSave::Unknown;
                }
            }
            if !queue.proficiency_principals(session).is_empty() {
                return TrainingSave::Unknown;
            }
        }
        let now = self.owner_now().get();
        let due = {
            let Ok(states) = self.states.proficiencies.lock() else {
                return TrainingSave::Unknown;
            };
            states
                .get(&session)
                .is_some_and(|state| state.checkpoint_due(now, force))
        };
        if !due {
            return TrainingSave::NotApplicable;
        }
        let Some(catalogue) = &self.proficiency_catalogue else {
            return TrainingSave::Unknown;
        };
        let Some(binding) = self.progression_binding(session) else {
            return TrainingSave::Unknown;
        };
        let definitions =
            catalogue.for_content(&binding.context.content) as Arc<dyn ProficiencyDefinitions>;
        let Some(current) = super::character_gameplay_fence::current(self, session, true).await
        else {
            return TrainingSave::FencedOut;
        };
        let mut slot = self.revision_sequencer.acquire(current.character_id).await;
        let Ok(root) = self
            .root
            .read_current_character(self.character, current.character_id)
            .await
        else {
            return TrainingSave::Unknown;
        };
        let fence = CurrentCharacterGameplayFence {
            expected_character_revision: root.revision,
            ..current
        };
        let Ok((stored, _)) = self
            .root
            .read_character_proficiency_with_definitions(
                self.character,
                fence.character_id,
                None,
                Arc::clone(&definitions),
            )
            .await
        else {
            return TrainingSave::Unknown;
        };
        let retained = {
            let runtime = self.runtime.lock().await;
            if runtime.player_control_facts(actor, session).is_err()
                || runtime.owner_fence().is_err()
                || fence.runtime_scope
                    != crate::foundation::RuntimeScopeRefV1::channel(
                        runtime.binding().world_id(),
                        runtime.binding().channel_id(),
                    )
                || fence.scope_ownership_generation != runtime.binding().scope_generation()
            {
                return TrainingSave::FencedOut;
            }
            let Ok(mut states) = self.states.proficiencies.lock() else {
                return TrainingSave::Unknown;
            };
            let Some(state) = states.get_mut(&session) else {
                return TrainingSave::NotApplicable;
            };
            if !state.checkpoint_due(now, force) {
                return TrainingSave::NotApplicable;
            }
            if state.checkpoint.is_none() {
                let Some(checkpoint) =
                    state.prepare_checkpoint(fence, &stored, definitions.as_ref())
                else {
                    if state.pending_progress.is_empty() {
                        state.dirty_since = None;
                        return TrainingSave::NotApplicable;
                    }
                    return TrainingSave::Unknown;
                };
                state.checkpoint = Some(checkpoint);
            }
            state.checkpoint.clone()
        };
        let Some(retained) = retained else {
            return TrainingSave::Unknown;
        };
        let outcome = slot
            .commit_proficiency(
                self.root,
                self.character,
                self.holder,
                retained.fence,
                retained.request.clone(),
                Arc::clone(&definitions),
                Some(retained.fence.expected_character_revision),
            )
            .await;
        let receipt = match outcome {
            Ok(
                ProficiencyCommitOutcome::Committed(receipt)
                | ProficiencyCommitOutcome::AlreadyCommitted(receipt),
            ) => receipt,
            other => match self
                .root
                .reconcile_character_proficiency_with_definitions(
                    self.character,
                    retained.fence.character_id,
                    retained.fence.expected_character_revision,
                    retained.request.clone(),
                    Some(definitions),
                )
                .await
            {
                Ok(Some(receipt)) => receipt,
                Ok(None) => {
                    // A known refusal/rollback loses the candidate, never its per-track delta.
                    if matches!(
                        other,
                        Ok(ProficiencyCommitOutcome::Refused(_))
                            | Err(CharacterProgressionError::CharacterRevisionMismatch
                                | CharacterProgressionError::AuthorityRejected)
                    ) && let Ok(mut states) = self.states.proficiencies.lock()
                        && let Some(state) = states.get_mut(&session)
                    {
                        state.checkpoint = None;
                    }
                    return TrainingSave::Unknown;
                }
                Err(_) => return TrainingSave::Unknown,
            },
        };
        let Some(fresh) = super::character_gameplay_fence::current(self, session, true).await
        else {
            return TrainingSave::FencedOut;
        };
        if fresh.character_id != retained.fence.character_id
            || fresh.connection_generation != current.connection_generation
            || fresh.character_lease_generation != current.character_lease_generation
            || fresh.runtime_scope != current.runtime_scope
            || fresh.scope_ownership_generation != current.scope_ownership_generation
        {
            return TrainingSave::FencedOut;
        }
        let runtime = self.runtime.lock().await;
        if runtime.player_control_facts(actor, session).is_err() || runtime.owner_fence().is_err() {
            return TrainingSave::FencedOut;
        }
        let Ok(mut states) = self.states.proficiencies.lock() else {
            return TrainingSave::Unknown;
        };
        let Some(state) = states.get_mut(&session) else {
            return TrainingSave::FencedOut;
        };
        if !state.acknowledge_checkpoint(retained, receipt, catalogue)
            || state.checkpoint_due(now, force)
        {
            TrainingSave::Unknown
        } else {
            TrainingSave::Saved
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "unit fixtures construct valid durable proficiency states"
)]
#[path = "proficiency_checkpoint_tests.rs"]
mod tests;
