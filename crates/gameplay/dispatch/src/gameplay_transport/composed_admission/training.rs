use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// DEATH-ML-LOSS-1 (A13 §4.6, CP D1433, #2193 Codex P1 4304911161): the death's loss is
    /// priced from a build that holds the actor's trained progress, so a new death first
    /// force-saves the pending training. A death the retained receipts already hold is not new:
    /// its build was priced when it committed and its pending respawn refuses build writes, so
    /// the flush is skipped. `false` (an unknown save or a failed reconcile) keeps the actor
    /// dead for the next attempt; see [`death_flush_permits_settle`] for a fenced-out save.
    pub(in crate::gameplay_transport) async fn flush_training_before_death(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        death: actor_spell::PlayerDeath,
        releasing: bool,
    ) -> bool {
        if !self.stop_timed_actor(actor, session).await {
            return false;
        }
        match self
            .root
            .reconcile_character_death(self.character, death.occurrence)
            .await
        {
            Ok(Some(_)) => true,
            Ok(None) => {
                let mut save = self.save_spell_training(actor, session, true).await;
                // #2193 Codex P1 4306765047, 4307011952: a flush whose retained checkpoint
                // committed under a since-moved session fence replays that fence forever; its
                // durable receipt is applied first, on the live and the release path, and the
                // post-checkpoint training saves under the current fence.
                if save == actor_spell::TrainingSave::FencedOut
                    && self
                        .adopt_committed_training_checkpoint(actor, session)
                        .await
                {
                    save = self.save_spell_training(actor, session, true).await;
                }
                // #2193 Codex P1 4307011952: a fenced-out flush proves nothing committed (a
                // write the store refused as `AuthorityRejected` keeps its checkpoint and the
                // pending training only in memory), so whether the actor still holds training is
                // read back before the release may settle from the durable build.
                let training_left = save == actor_spell::TrainingSave::FencedOut && {
                    let runtime = self.runtime.lock().await;
                    self.states
                        .spell_states
                        .lock()
                        .await
                        .has_training_to_flush(&runtime, actor, session)
                };
                death_flush_permits_settle(save, releasing, training_left)
            }
            Err(error) => {
                operator_event(&format!("player_death_reconcile_failed reason={error}"));
                false
            }
        }
    }

    /// DEATH-ML-LOSS-1 (#2193 Codex P1 4306765047): a forced training save whose retained
    /// checkpoint committed while the session's connection, lease or scope generation moved
    /// returns `FencedOut` without applying the receipt, and every replay fails the same
    /// immutable retained fence. When the checkpoint's receipt is durable and the live actor is
    /// still this session's under the current session, scope and owner authority, the receipt is
    /// applied to the live actor: the checkpoint retires, the receipt's build is the actor's
    /// durable training base, and its post-checkpoint training stays pending for the next save,
    /// which prepares under the current fence. Nothing is written. `false` (no retained
    /// checkpoint, no durable receipt, a terminal or moved session, a failed read or a refused
    /// receipt) changes nothing.
    pub(in crate::gameplay_transport) async fn adopt_committed_training_checkpoint(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let Ok((current, _)) = store.current_session_at(session).await else {
            return false;
        };
        if current.session_state() == GameSessionState::Terminal {
            return false;
        }
        let Ok(character) =
            domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes())
        else {
            return false;
        };
        // The Character's revision writer, as the training save holds it, before the runtime lock.
        let _slot = self.revision_sequencer.acquire(character).await;
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let binding = runtime.binding();
        if current.current_runtime_scope()
            != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
            || current.current_scope_generation() != binding.scope_generation()
            || runtime.player_control_facts(actor, session).is_err()
            || runtime.owner_fence().is_err()
        {
            return false;
        }
        let Some(state) = states.get_mut(&runtime, actor, session) else {
            return false;
        };
        let Some(retained) = state.pending_training_checkpoint().cloned() else {
            return false;
        };
        let Ok(Some(receipt)) = self
            .root
            .reconcile_character_build(self.character, retained.prepared.request.occurrence)
            .await
        else {
            return false;
        };
        state
            .commit_training_checkpoint(retained.prepared, &receipt)
            .is_ok()
    }
}
