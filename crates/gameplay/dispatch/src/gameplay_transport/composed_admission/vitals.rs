use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn observe_actor_vitals(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        if self
            .runtime
            .lock()
            .await
            .house_actor_state(actor, game_session_id)
            .is_ok()
        {
            let fence = self
                .root
                .observe_current_gameplay_fence(self.character, game_session_id, true)
                .await
                .ok()??
                .item_fence();
            let runtime = self.runtime.lock().await;
            return self.states.spell_states.lock().await.house_vitals(
                &runtime,
                actor,
                game_session_id,
                fence,
            );
        }
        if !self
            .prepare_channel_vitals(actor, game_session_id, None)
            .await
        {
            return None;
        }
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        actor_spell::observe_vitals(&runtime, &states, actor, game_session_id)
    }

    pub(in crate::gameplay_transport) async fn observe_actor_join(
        &self,
        actor: ExactActorRef,
        admitted: crate::gameplay_transport::connection::AdmittedSession,
        house: bool,
    ) -> crate::gameplay_transport::spell_cooldowns::JoinVitals {
        let session = admitted.game_session_id;
        let generation = admitted.continuity.connection_generation;
        if !self
            .states
            .spell_states
            .lock()
            .await
            .current_cooldown_transport(actor, session, generation)
        {
            return (None, None);
        }
        let ready = house
            || self
                .prepare_channel_vitals(actor, session, Some(generation))
                .await;
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if !states.current_cooldown_transport(
            actor,
            session,
            admitted.continuity.connection_generation,
        ) {
            return (None, None);
        }
        let cooldowns = states.subscribe_cooldowns(&runtime, actor, admitted, self.spells);
        let vitals = if house {
            admitted
                .item_fence
                .filter(|fence| {
                    fence.connection_generation.get() == admitted.continuity.connection_generation
                })
                .and_then(|fence| states.house_vitals(&runtime, actor, session, fence))
        } else if ready {
            actor_spell::observe_vitals(&runtime, &states, actor, session)
        } else {
            None
        };
        (vitals, cooldowns)
    }

    async fn prepare_channel_vitals(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        generation: Option<u64>,
    ) -> bool {
        self.load_wheel_gift_for_transport(actor, game_session_id, generation)
            .await;
        self.refresh_wheel_vitals_for_transport(actor, game_session_id, generation)
            .await;
        let restored = pvp_live::tracked(
            &*self.states.spell_states.lock().await,
            actor,
            game_session_id,
        );
        restored
            || self
                .refresh_pvp_locks_for_transport(actor, game_session_id, generation)
                .await
    }

    async fn prepare_actor_vitals(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        // Named House residents also own these persistent native conditions.
        self.drome_sync(session, false).await
            && self.reconcile_pending_item_effects(actor, session).await
            && super::super::item_use::gourmet::checkpoint(self, actor, session, false).await
    }

    pub(in crate::gameplay_transport) async fn tick_house_actor_vitals(
        &self,
        actor: ExactActorRef,
        admitted: AdmittedSession,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        let session = admitted.game_session_id;
        let fence = admitted.item_fence?;
        let generation = admitted.continuity.connection_generation;
        if fence.game_session_id != session
            || fence.connection_generation.get() != generation
            || !self
                .states
                .spell_states
                .lock()
                .await
                .current_cooldown_transport(actor, session, generation)
        {
            return None;
        }
        self.expire_house_furniture_on_tick(actor, session, Some(fence))
            .await;
        let ready = self.reconcile_pending_item_effects(actor, session).await
            && super::super::item_use::gourmet::checkpoint_for_house_transport(self, actor, fence)
                .await;
        if ready {
            self.load_wheel_gift_for_transport(actor, session, Some(generation))
                .await;
        }
        let deadline = self.cake_regeneration_deadline();
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if !states.current_cooldown_transport(actor, session, fence.connection_generation.get()) {
            return None;
        }
        if ready
            && self
                .active_generation
                .and_then(|active| active.native_gameplay())
                .is_some()
            && !states.has_pending_spell_commit(actor, session)
            && runtime
                .assert_house_actor_spell_unreserved(actor, session)
                .is_ok()
            && runtime.house_player_writes_fenced(actor, session) == Ok(false)
            && let Some(state) = states.house_state_mut(&runtime, actor, session, fence)
        {
            super::super::cake_event::tick_house_regeneration(
                state,
                deadline,
                self.owner_now().get(),
                runtime.content_pin().server_artifact_digest(),
                session,
            );
        }
        states.house_vitals(&runtime, actor, session, fence)
    }

    pub(in crate::gameplay_transport) async fn tick_actor_vitals(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        if !self.prepare_actor_vitals(actor, game_session_id).await {
            return None;
        }
        if self
            .runtime
            .lock()
            .await
            .house_actor_state(actor, game_session_id)
            .is_ok()
        {
            self.load_wheel_gift(actor, game_session_id).await;
            // The House service projects vitals from its admitted item fence.
            // Here the cadence only needs to advance the condition owners.
            return None;
        }
        if !self.ensure_source_map_initialized().await {
            return None;
        }
        // I10a: the real one-second Channel cadence owns splash expiry, independently of
        // another fluid use or a map/object read.
        let _ = self
            .root
            .expire_item_fluid_splashes(
                self.character,
                *self.world_id.as_bytes(),
                *self.channel_id.as_bytes(),
            )
            .await;
        self.load_wheel_gift(actor, game_session_id).await;
        // Creature work belongs to the shared source-owner cycle. A connection's
        // vitals refresh must not census or qualify the world's monster population.
        let _ = self.save_wheel_gift(actor, game_session_id, false).await;
        self.refresh_swing_weapon(actor, game_session_id).await;
        self.drain_auto_attacks().await;
        let _ = self.drain_kill_rewards(game_session_id).await;
        self.apply_soul_kills(actor, game_session_id).await;
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
            // SUMMON-OWNER-END-1 (CREATURE-AI-0 §8.4): its other summons are removed with no
            // death; a refused removal keeps the actor dead for the next tick, as above.
            if self
                .runtime
                .lock()
                .await
                .remove_owned_summons_except_familiars(actor, game_session_id)
                .is_err()
            {
                return None;
            }
            return self
                .respawn_after_death(actor, game_session_id, death)
                .await;
        }
        self.refresh_level_up_vitals(actor, game_session_id).await;
        self.refresh_marked_titles(actor, game_session_id).await;
        self.load_wheel_gift(actor, game_session_id).await;
        self.refresh_wheel_vitals(actor, game_session_id).await;
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
        // PKG-P3/P10 UNVERIFIED: map evidence currently identifies PZ tiles, not
        // Global's narrower resting-area subset. This preserves the existing tile gate.
        {
            let mut runtime = self.runtime.lock().await;
            let inside =
                actor_spell::qualified_protection_zone(&mut runtime, self.movement_cells(), actor)
                    == Some(true);
            // Winter Update 2026: the area's tier no longer depends on the streak.
            let streak = self
                .states
                .resting_areas
                .tier(game_session_id, actor, inside);
            if let Some(state) =
                self.states
                    .spell_states
                    .lock()
                    .await
                    .get_mut(&runtime, actor, game_session_id)
            {
                state.set_resting_streak(streak);
            }
        }
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
                self.movement_cells(),
                actor,
                game_session_id,
                now,
            )
        };
        let fifth_taint = self.tick_soul_war_fifth_taint(actor, game_session_id).await;
        let resting = {
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let inside =
                actor_spell::qualified_protection_zone(&mut runtime, self.movement_cells(), actor)
                    == Some(true);
            let stamina = states.live_stamina(actor, game_session_id).unwrap_or(2520);
            let premium = states
                .get(&runtime, actor, game_session_id)
                .is_some_and(|state| state.hunting_premium_current(self.owner_now().get()));
            let soul_interval = states
                .get(&runtime, actor, game_session_id)
                .map_or(120, |state| {
                    super::super::resting_area::soul_interval(
                        state.character_facts().vocation,
                        premium,
                    )
                });
            let (stamina_due, soul_due) = self.states.resting_areas.cadence(
                game_session_id,
                actor,
                inside,
                premium,
                stamina,
                soul_interval,
            );
            if stamina_due {
                states.gain_resting_stamina(actor, game_session_id);
            }
            if inside {
                states.apply_resting_soul(&runtime, actor, game_session_id, soul_due)
            } else {
                None
            }
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
        self.tick_familiar_defenses(actor, game_session_id);
        let _ = self.drain_native_ai_timers().await;
        resting.or(fifth_taint).or(result).or(party_vitals)
    }
}
