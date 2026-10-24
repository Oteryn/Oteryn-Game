use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn observe_actor_vitals(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        let restored = pvp_live::tracked(
            &*self.states.spell_states.lock().await,
            actor,
            game_session_id,
        );
        if !restored && !self.refresh_pvp_locks(actor, game_session_id).await {
            return None;
        }
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        actor_spell::observe_vitals(&runtime, &states, actor, game_session_id)
    }

    pub(in crate::gameplay_transport) async fn tick_actor_vitals(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        if !self.ensure_source_map_initialized().await {
            return None;
        }
        self.drain_monster_melee().await;
        self.refresh_attack_weapon(actor, game_session_id).await;
        self.drain_auto_attacks().await;
        let _ = self.drain_kill_rewards(game_session_id).await;
        self.drain_source_item_deadlines().await;
        self.drain_source_party_deadlines_bounded().await;
        // DEATH-2 §4.5: a dead player's cadence tick settles its death and respawns it.
        let death = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            states.player_death(&runtime, actor, game_session_id)
        };
        if let Some(death) = death {
            // #2199 Codex P1 4303261095 (FAMILIARS0 §7): the owner's death ends its live
            // familiar (remaining 0) through the fenced lifecycle writer before the respawn; a
            // failed write keeps the actor dead and the familiar present for the next tick.
            if self
                .end_familiars_at_owner_death(actor, game_session_id, death.occurrence)
                .await
                .is_err()
            {
                return None;
            }
            return self
                .respawn_after_death(actor, game_session_id, death)
                .await;
        }
        self.refresh_level_up_vitals(actor, game_session_id).await;
        if self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .is_some()
        {
            let _ = self
                .refresh_source_party_presence(actor, game_session_id)
                .await;
        }
        let _ = self
            .refresh_and_apply_spell_access(actor, game_session_id)
            .await;
        let party_vitals = self
            .refresh_source_party_serene(actor, game_session_id)
            .await;
        let result = if self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .is_some()
        {
            self.tick_spell_periodic(actor, game_session_id).await
        } else {
            let now = self.owner_now();
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            states.tick_in_environment(
                &mut runtime,
                self.movement_cells,
                actor,
                game_session_id,
                now,
            )
        };
        let _ = self
            .save_spell_training(actor, game_session_id, false)
            .await;
        let cleanup_access = self.refresh_spell_access(actor, game_session_id).await;
        let _ = self
            .drain_familiar_deaths(actor, game_session_id, &cleanup_access)
            .await;
        // #2199 Codex P2 4302041461: a familiar Login stored for lack of a free tile returns
        // on this cadence once a tile frees.
        let _ = self
            .retry_waiting_familiar_return(actor, game_session_id, &cleanup_access)
            .await;
        let _ = self.tick_familiar_defenses(actor, game_session_id).await;
        let _ = self.drain_native_ai_timers().await;
        result.or(party_vitals)
    }
}
