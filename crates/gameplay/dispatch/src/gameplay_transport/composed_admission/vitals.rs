use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn observe_actor_vitals(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        self.load_wheel_gift(actor, game_session_id).await;
        if self
            .runtime
            .lock()
            .await
            .house_actor_state(actor, game_session_id)
            .is_ok()
        {
            let (current, _) = FreshAdmissionStore::from_root(self.root.clone())
                .current_session_at(game_session_id)
                .await
                .ok()?;
            let fence = item_fence_of(current)?;
            let runtime = self.runtime.lock().await;
            return self.states.spell_states.lock().await.house_vitals(
                &runtime,
                actor,
                game_session_id,
                fence,
            );
        }
        self.refresh_wheel_vitals(actor, game_session_id).await;
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

    async fn tick_house_vitals(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        use crate::foundation::condition::TickFacts;
        self.load_wheel_gift(actor, session).await;
        let (current, _) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(session)
            .await
            .ok()?;
        let fence = item_fence_of(current)?;
        let runtime = self.runtime.lock().await;
        let house = runtime.house_actor_state(actor, session).ok()?;
        let scope = self.movement_cells().scope();
        let map_revision: [u8; 32] =
            <sha2::Sha256 as sha2::Digest>::digest(scope.map_revision.as_str().as_bytes()).into();
        if scope.world_id != runtime.binding().world_id()
            || scope.generation_digest != runtime.content_pin().server_artifact_digest()
            || map_revision != runtime.content_pin().map_revision_digest()
            || runtime
                .assert_house_actor_spell_unreserved(actor, session)
                .is_err()
        {
            return None;
        }
        let cell = crate::content::LogicalCell {
            x: house.position.x,
            y: house.position.y,
            z: i32::from(house.position.floor),
        };
        let position = crate::content::bundle_spell_tiles::tile_pos(cell)?;
        let inside = self
            .bundle_map()?
            .overlay
            .base()
            .tile(position.x, position.y, position.floor)?
            .protection_zone();
        let now = self.owner_now();
        let mut states = self.states.spell_states.lock().await;
        if states.has_pending_spell_commit(actor, session) {
            return None;
        }
        let stamina = states.live_stamina(actor, session).unwrap_or(2520);
        let state = states.house_state_mut(&runtime, actor, session, fence)?;
        if state.vitals().health == 0 {
            return None;
        }
        state.expire_owner_premium(now.get()).ok()?;
        state.set_resting_streak(self.states.resting_areas.tier(session, actor, inside));
        if let Some(cycle) =
            crate::spell::actor_conditions::stage_source_owner_cycle_without_damage(
                state,
                now,
                TickFacts {
                    in_protection_zone: inside,
                    standing_on_field: None,
                },
                None,
            )
            .ok()?
        {
            *state = cycle.next;
        }
        let premium = state.hunting_premium_current(now.get());
        let interval =
            super::super::resting_area::soul_interval(state.character_facts().vocation, premium);
        let (stamina_due, soul_due) = self
            .states
            .resting_areas
            .cadence(session, actor, inside, premium, stamina, interval);
        if soul_due {
            *state = state.after_resting_soul()?;
        }
        let projection = (state.revision(), state.vitals());
        if stamina_due {
            states.gain_resting_stamina(actor, session);
        }
        Some(projection)
    }

    pub(in crate::gameplay_transport) async fn tick_actor_vitals(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        if !self.drome_sync(game_session_id, false).await {
            return None;
        }
        // Named House residents also own these persistent native conditions.
        // Checkpoint before unrelated Channel-only cadence may return early.
        if !self
            .reconcile_pending_item_effects(actor, game_session_id)
            .await
            || !super::super::item_use::gourmet::checkpoint(self, actor, game_session_id, false)
                .await
        {
            return None;
        }
        if self
            .runtime
            .lock()
            .await
            .house_actor_state(actor, game_session_id)
            .is_ok()
        {
            if let Some(vitals) = self.tick_house_vitals(actor, game_session_id).await {
                return Some(vitals);
            }
            return self.observe_actor_vitals(actor, game_session_id).await;
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
        self.refresh_attack_weapon(actor, game_session_id).await;
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
        let _ = self.tick_familiar_defenses(actor, game_session_id).await;
        let _ = self.drain_native_ai_timers().await;
        resting.or(result).or(party_vitals)
    }
}
