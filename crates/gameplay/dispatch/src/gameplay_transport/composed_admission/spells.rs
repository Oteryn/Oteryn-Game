use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn cast_actor_spell(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        command_id: u64,
        intent: actor_spell::SpellCastIntent,
    ) -> actor_spell::SpellCastOutcome {
        if self.staff_muted(game_session_id).await {
            return actor_spell::SpellCastOutcome::rejected();
        }
        if let Some(outcome) = self
            .cast_durable_stance(actor, game_session_id, command_id, &intent)
            .await
        {
            return outcome;
        }
        let now = self.owner_now();
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        actor_spell::cast_in_channel(
            &runtime,
            &mut states,
            self.spells,
            actor,
            game_session_id,
            command_id,
            &intent,
            now,
        )
    }

    pub(in crate::gameplay_transport) async fn complete_actor_spell(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        command_id: u64,
        intent: actor_spell::SpellCastIntent,
    ) -> Option<actor_spell::SpellCastOutcome> {
        let outcome = self
            .cast_spell_dispatch(actor, game_session_id, command_id, intent)
            .await;
        if outcome.is_some() {
            self.settle_pacified_target(actor, game_session_id).await;
        }
        outcome
    }

    pub(in crate::gameplay_transport) async fn cast_spell_dispatch(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        command_id: u64,
        intent: actor_spell::SpellCastIntent,
    ) -> Option<actor_spell::SpellCastOutcome> {
        if self.staff_muted(game_session_id).await {
            return Some(actor_spell::SpellCastOutcome::rejected());
        }
        // CHAT-0 §6: a muted character cannot cast any spell.
        if let Some(outcome) = self
            .states
            .chat
            .muted_cast(game_session_id, self.owner_now())
        {
            return Some(outcome);
        }
        // CONDITIONS-0 §3.5: the condition gates answer next, ahead of every dispatcher.
        let gated = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            actor_spell::condition_gated_cast(
                &runtime,
                &states,
                self.spells,
                actor,
                game_session_id,
                &intent,
                self.owner_now(),
            )
        };
        if let Some(gate) = gated {
            // §7: the typed gate stays in the operator log; the wire answer is `REJECTED`.
            operator_event(&format!(
                "event=spell_cast_condition_gated gate={}",
                gate.key()
            ));
            return Some(actor_spell::SpellCastOutcome::rejected());
        }
        if !self.ensure_source_map_initialized().await {
            return None;
        }
        if self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .is_some()
        {
            let _ = self
                .refresh_source_party_presence(actor, game_session_id)
                .await;
        }
        // This explicit unavailable producer grants no commercial, Wheel or
        // control benefit. A configured authenticated owner replaces this port.
        let access = self
            .refresh_and_apply_spell_access(actor, game_session_id)
            .await;
        match self
            .cast_native_familiar(actor, game_session_id, command_id, &intent, &access)
            .await
        {
            actor_spell::NativeCastDispatch::Outcome(outcome) => return Some(outcome),
            actor_spell::NativeCastDispatch::Pending => return None,
            actor_spell::NativeCastDispatch::RuneRefused(_) => {
                return Some(actor_spell::SpellCastOutcome::rejected());
            }
            actor_spell::NativeCastDispatch::NotApplicable => (),
        }
        match self
            .cast_world_items(actor, game_session_id, command_id, &intent, &access)
            .await
        {
            actor_spell::NativeCastDispatch::Outcome(outcome) => return Some(outcome),
            actor_spell::NativeCastDispatch::Pending => return None,
            actor_spell::NativeCastDispatch::RuneRefused(_) => {
                return Some(actor_spell::SpellCastOutcome::rejected());
            }
            actor_spell::NativeCastDispatch::NotApplicable => (),
        }
        match self
            .cast_native_combat(actor, game_session_id, command_id, &intent, &access)
            .await
        {
            actor_spell::NativeCastDispatch::Outcome(outcome) => {
                let _ = self.drain_kill_rewards(game_session_id).await;
                self.party_spell_feedback(game_session_id, &intent, &outcome);
                Some(outcome)
            }
            actor_spell::NativeCastDispatch::Pending => None,
            actor_spell::NativeCastDispatch::RuneRefused(_) => {
                Some(actor_spell::SpellCastOutcome::rejected())
            }
            actor_spell::NativeCastDispatch::NotApplicable => {
                let outcome = self
                    .cast_spell(actor, game_session_id, command_id, intent)
                    .await;
                if self
                    .states
                    .spell_states
                    .lock()
                    .await
                    .has_pending_spell_commit(actor, game_session_id)
                {
                    None
                } else {
                    Some(outcome)
                }
            }
        }
    }

    /// ATTACK-0 §4 (as amended by #2106): a cast that applied `pacified` to its caster clears
    /// the caster's held target before the intent's completion is answered.
    pub(in crate::gameplay_transport) async fn settle_pacified_target(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) {
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        self.states.attack.lock().await.settle_pacified_target(
            &runtime,
            &states,
            actor,
            game_session_id,
        );
    }
}
