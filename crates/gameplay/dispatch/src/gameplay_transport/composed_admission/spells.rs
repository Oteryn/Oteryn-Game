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
        self.cast_spell_dispatch_with_parameter(actor, game_session_id, command_id, intent, None)
            .await
    }

    async fn cast_spell_dispatch_with_parameter(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        command_id: u64,
        intent: actor_spell::SpellCastIntent,
        parameter: Option<&oteryn_protocol_oteryn::actor_spell_v2::ParameterSpellCastIntent>,
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
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if self
                .spells
                .source_indexed(intent.spell)
                .is_some_and(|(spell, _)| spell.aggressive)
                && states
                    .get(&runtime, actor, game_session_id)
                    .is_some_and(|state| {
                        state
                            .cooldowns
                            .group_ready_at(
                                oteryn_gameplay_spell_rules::cast::STAIRHOP_COOLDOWN_KEY,
                            )
                            .is_some_and(|until| self.owner_now() < until)
                    })
            {
                return Some(actor_spell::SpellCastOutcome {
                    disposition:
                        oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::CoolingDown,
                    vitals: None,
                });
            }
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
        if let Some(parameter) = parameter {
            let dispatch = self
                .cast_parameters(actor, game_session_id, command_id, parameter, &access)
                .await;
            let outcome = match dispatch.cast {
                actor_spell::NativeCastDispatch::Outcome(outcome) => outcome,
                actor_spell::NativeCastDispatch::Pending => return None,
                actor_spell::NativeCastDispatch::NotApplicable => actor_spell::SpellCastOutcome {
                    disposition: actor_spell::SpellCastDisposition::NotAvailable,
                    vitals: None,
                },
                actor_spell::NativeCastDispatch::RuneRefused(_) => {
                    actor_spell::SpellCastOutcome::rejected()
                }
            };
            if let Some(feedback) = dispatch.result.and_then(|result| result.feedback)
                && !feedback.text.is_empty()
            {
                self.states
                    .chat
                    .push_party_spell_feedback(game_session_id, &feedback.text);
            }
            return Some(outcome);
        }
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

    /// PARAM-CAST-WIRE-1: speech is another input to the common cast dispatcher. The
    /// admitted CHAT_V1 route carries its private feedback without allocating a v2 command.
    pub(in crate::gameplay_transport) async fn complete_chat_intent(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        intent: &oteryn_protocol_oteryn::chat::ChatIntent,
    ) -> Option<(
        oteryn_protocol_oteryn::chat::ChatIntentResult,
        Option<actor_spell::SpellCastOutcome>,
    )> {
        use oteryn_protocol_oteryn::chat::{
            ChatDisposition, ChatIntent, ChatIntentResult, ChatSpeechMode,
        };
        let spoken = match intent {
            ChatIntent::Say {
                mode: ChatSpeechMode::Say,
                text,
            } => self.spells.spoken(text),
            _ => None,
        };
        let Some(spoken) = spoken else {
            return Some((
                self.chat_speak(actor, session, command_id, intent).await,
                None,
            ));
        };
        if let Some(refused) = self.states.chat.pre_gate(session, intent, self.owner_now()) {
            return Some((refused, None));
        }
        let index = (1..=self.spells.source_len())
            .filter_map(|index| {
                u32::try_from(index)
                    .ok()
                    .and_then(std::num::NonZeroU32::new)
            })
            .find(|index| {
                self.spells
                    .indexed(*index)
                    .is_some_and(|spell| spell.key == spoken.spell.key)
            })?;
        let cast = actor_spell::SpellCastIntent {
            spell: index,
            target: actor_spell::SpellTarget::None,
            aim_at_target: false,
        };
        let parameter = matches!(
            spoken.spell.carrier,
            crate::spell::Carrier::Instant {
                takes_parameter: true,
                ..
            }
        )
        .then_some(
            oteryn_protocol_oteryn::actor_spell_v2::ParameterSpellCastIntent {
                intent: cast,
                parameter: spoken.parameter,
            },
        );
        let outcome = self
            .cast_spell_dispatch_with_parameter(
                actor,
                session,
                command_id,
                cast,
                parameter.as_ref(),
            )
            .await?;
        self.settle_pacified_target(actor, session).await;
        let disposition = if outcome.disposition == actor_spell::SpellCastDisposition::Cast {
            ChatDisposition::Ok
        } else {
            ChatDisposition::Rejected
        };
        Some((
            ChatIntentResult {
                disposition,
                wait_seconds: 0,
            },
            Some(outcome),
        ))
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
