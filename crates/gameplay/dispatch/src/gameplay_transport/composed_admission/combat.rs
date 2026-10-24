use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn select_attack_target(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        command_id: u64,
        target: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
    ) -> oteryn_protocol_oteryn::attack::AttackIntentDisposition {
        let permit = if target.is_some() {
            let Some(permit) = self.spell_lane_permit().await else {
                return oteryn_protocol_oteryn::attack::AttackIntentDisposition::Rejected;
            };
            Some(permit)
        } else {
            None
        };
        let now = self.owner_now();
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        let Some(state) = states.get(&runtime, actor, game_session_id) else {
            return oteryn_protocol_oteryn::attack::AttackIntentDisposition::Rejected;
        };
        // CONDITIONS-0 §3.5 / ATTACK-0 §3: a `pacified` held in the player store or the runtime
        // slot refuses a new target first; the typed gate stays in the operator log (§7) and
        // `set_target` rechecks the slot. Clearing (`None`) stays open.
        if target.is_some() {
            let gate = state
                .owned_conditions()
                .attack_target_gate(now.get())
                .err()
                .or_else(|| {
                    runtime
                        .actor_conditions(actor, Some(game_session_id))
                        .ok()
                        .and_then(|store| store.attack_target_gate(now.get()).err())
                });
            if let Some(gate) = gate {
                operator_event(&format!(
                    "event=attack_target_condition_gated gate={}",
                    gate.key()
                ));
                return oteryn_protocol_oteryn::attack::AttackIntentDisposition::Rejected;
            }
            if states.is_dead(actor) {
                return oteryn_protocol_oteryn::attack::AttackIntentDisposition::Rejected;
            }
        }
        let room = self.rooms.gate_attack_target();
        if let Some(permit) = permit.as_ref()
            && !self
                .pvp_selection_allowed(
                    permit,
                    &runtime,
                    &states,
                    room,
                    actor,
                    game_session_id,
                    command_id,
                    target,
                )
                .await
        {
            return oteryn_protocol_oteryn::attack::AttackIntentDisposition::Rejected;
        }
        self.states.attack.lock().await.set_target(
            &runtime,
            room,
            actor,
            game_session_id,
            now,
            target,
            command_id,
        )
    }

    pub(in crate::gameplay_transport) async fn set_fight_modes(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        modes: oteryn_protocol_oteryn::attack::FightModes,
    ) -> oteryn_protocol_oteryn::attack::AttackIntentDisposition {
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        if states.get(&runtime, actor, game_session_id).is_some()
            && self
                .states
                .attack
                .lock()
                .await
                .set_modes(&runtime, actor, game_session_id, modes)
        {
            oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
        } else {
            oteryn_protocol_oteryn::attack::AttackIntentDisposition::Rejected
        }
    }

    pub(in crate::gameplay_transport) async fn observe_actor_combat_state(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::attack::ActorCombatState> {
        let now = self.owner_now();
        // ATTACK-1b lock order: runtime, spell states, attack.
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        let mut attack = self.states.attack.lock().await;
        attack.settle_pacified_target(&runtime, &states, actor, game_session_id);
        let mut state = attack.combat_state(actor, game_session_id, now);
        state.in_fight |= pvp_live::logout_blocked(&states, actor, game_session_id, now);
        Some(state)
    }

    pub(in crate::gameplay_transport) async fn tick_actor_combat(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::attack::ActorCombatState> {
        if !self.ensure_source_map_initialized().await {
            return None;
        }
        self.refresh_attack_weapon(actor, game_session_id).await;
        self.drain_auto_attacks().await;
        let _ = self.drain_kill_rewards(game_session_id).await;
        self.observe_combat_state(actor, game_session_id).await
    }
}
