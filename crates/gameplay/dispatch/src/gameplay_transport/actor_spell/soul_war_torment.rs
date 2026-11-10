//! Crystal soul_war.lua1398-1450,1545-1579: one canonical HP/counter successor.
use super::*;
use oteryn_gameplay_encounters::bosses::soul_war_torment::Pulse;

impl ChannelSpellStates {
    // PARTIAL W11: native HP/counter tests cover this owner; the shared
    // Symbol/Megalomania Think cohort driver still needs its production caller.
    #[allow(
        dead_code,
        reason = "PARTIAL W11: shared Torment Think/cohort driver is not wired yet"
    )]
    #[allow(clippy::too_many_arguments)]
    pub(in crate::gameplay_transport) fn apply_soul_war_torment_pulse(
        &mut self,
        runtime: &ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
        source: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        now: crate::foundation::owner_timer::SemanticTimeMicros,
    ) -> Option<Pulse> {
        use oteryn_gameplay_encounters::bosses::soul_war_torment::{IconChange, PlayerCounter};
        if !super::super::soul_war_callbacks::qualifies_source(runtime, native)
            || !matches!(
                runtime.binding().runtime_scope(),
                crate::foundation::RuntimeScopeRefV1::Instance { .. }
            )
            || runtime.companion_snapshot(source).ok()?.health <= 0
        {
            return None;
        }
        let key = super::super::soul_war_callbacks::source_actor(runtime, native, source)?;
        let (maximum, center) = match key.as_str() {
            "oteryn:creature.symbol_of_hatred" => (30, (33744, 31599, 14)),
            "oteryn:creature.goshnar_s_megalomania_purple"
            | "oteryn:creature.goshnar_s_megalomania_green"
            | "oteryn:creature.goshnar_s_megalomania_blue" => (36, (33710, 31634, 14)),
            _ => return None,
        };
        let position = runtime.read_actor_position(target).ok()?.position();
        if position.floor != center.2
            || position.x.abs_diff(center.0) > 15
            || position.y.abs_diff(center.1) > 15
        {
            return None;
        }
        let state = self.get(runtime, target, session)?;
        let mut counter = PlayerCounter::from_current(state.soul_war_torment_count())?;
        let pulse = counter.pulse(maximum, runtime.soul_war_megalomania_name_present().ok()?);
        let visible = match pulse.icon {
            IconChange::Keep => state.soul_war_torment_icon_count().is_some(),
            IconChange::Set(_) => true,
            IconChange::Remove => false,
        };
        if self.is_dead(target) {
            if self.has_pending_spell_commit(target, session)
                || runtime.assert_actor_spell_unreserved(target).is_err()
            {
                return None;
            }
            let next = state.stage_soul_war_torment_counter_on_dead_player(
                counter.value(),
                visible,
                (source.placement_identity(), now.get()),
            )?;
            return self.commit(runtime, target, session, next).then_some(pulse);
        }
        self.apply_player_damage_stage(
            runtime,
            target,
            session,
            crate::player_lethal::PlayerHitKind::Death,
            oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Creature(None),
            // The pinned Torment player:addHealth call has a nil caster; source registration is not a killer.
            None,
            |state| {
                state.stage_soul_war_torment_pulse(
                    counter.value(),
                    visible,
                    (source.placement_identity(), now.get()),
                    pulse.damage,
                    now.get(),
                )
            },
        )?;
        Some(pulse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::native_gameplay::test_support::activated_with_loot_tables;
    use crate::foundation::{MovementLocalPosition, NodeId, RuntimeScopeRefV1, WorldId};
    use crate::spell::{Vocation, cast::CharacterCastFacts};
    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn id(tag: u8) -> [u8; 16] {
        let mut value = [tag; 16];
        value[6] = 0x70;
        value[8] = 0x80;
        value
    }
    #[test]
    fn srx3_soul_war_torment_native_player_hp_and_counter_publish_together() -> TestResult {
        exercise_native_pulse(false)
    }
    #[test]
    fn srx3_torment_visible_dead_player_updates_counter_without_reviving_or_second_death()
    -> TestResult {
        exercise_native_pulse(true)
    }
    fn exercise_native_pulse(dead: bool) -> TestResult {
        let world = WorldId::decode(&id(90)).map_err(|e| format!("{e:?}"))?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let native = controller
            .active()
            .and_then(|a| a.native_gameplay())
            .ok_or("native generation")?;
        let scope = RuntimeScopeRefV1::instance(world, id(91)).map_err(|e| format!("{e:?}"))?;
        let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
            scope,
            NodeId::decode(&id(92)).map_err(|e| format!("{e:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin.into_channel_pin(),
        )?;
        runtime.install_companion_policies(native.policies().clone())?;
        let session = GameSessionId::decode(&id(93)).map_err(|e| format!("{e:?}"))?;
        let reservation = runtime.reserve_fresh_session(session)?;
        let player = runtime.commit_fresh_session(reservation)?;
        let position = MovementLocalPosition {
            x: 33711,
            y: 31636,
            floor: 14,
        };
        runtime.initialize_pinned_test_position(player, position)?;
        let source = runtime
            .spawn_arena_opponent(
                "oteryn:creature.goshnar_s_megalomania_purple",
                MovementLocalPosition {
                    y: position.y - 1,
                    ..position
                },
                |_, _| Some(true),
            )
            .map_err(|error| format!("actual source actor admission: {error:?}"))?;
        assert_eq!(
            super::super::super::soul_war_callbacks::source_actor(&runtime, native, source)
                .as_deref(),
            Some("oteryn:creature.goshnar_s_megalomania_purple")
        );
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                player,
                session,
                CharacterCastFacts {
                    vocation: Vocation::Druid,
                    level: 500,
                    magic_level: 0,
                    max_health: 10_000,
                    max_mana: 1_000,
                    max_soul: 100,
                },
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .ok_or("actual current Player state")?;
        let staged = states
            .get(&runtime, player, session)
            .ok_or("current Player")?
            .stage_soul_war_torment_counter(24)
            .ok_or("source prior counter")?;
        assert!(states.commit(&runtime, player, session, staged));
        if dead {
            // Lua increments the counter before addHealth. The latter refuses
            // HP0 in Crystal game.cpp7802-7811, but that refusal does not undo
            // the former (soul_war.lua1421-1436). No dead HP grant is requested.
            states
                .apply_creature_damage_kind(
                    &runtime,
                    player,
                    session,
                    1_000_000,
                    crate::player_lethal::PlayerHitKind::Physical,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(1_000_000),
                    oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Creature(None),
                )
                .ok_or("canonical lethal setup")?;
            assert_eq!(
                states
                    .get(&runtime, player, session)
                    .ok_or("dead state")?
                    .vitals()
                    .health,
                0
            );
            assert!(states.is_dead(player));
            assert_eq!(states.deaths.len(), 1);
        }
        let pulse = states
            .apply_soul_war_torment_pulse(
                &runtime,
                native,
                source,
                player,
                session,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(10_000_000),
            )
            .ok_or("qualified real Mego pulse must publish")?;
        assert_eq!(pulse.damage, 1_400, "damage uses PRE-increment counter24");
        let after = states
            .get(&runtime, player, session)
            .ok_or("published Player")?;
        assert_eq!(after.soul_war_torment_count(), 25);
        assert_eq!(after.soul_war_torment_icon_count(), Some(25));
        if dead {
            assert_eq!(after.vitals().health, 0);
            assert_eq!(states.deaths.len(), 1);
            let before_replay = after.clone();
            assert!(
                states
                    .apply_soul_war_torment_pulse(
                        &runtime,
                        native,
                        source,
                        player,
                        session,
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(10_000_000),
                    )
                    .is_none()
            );
            assert_eq!(states.get(&runtime, player, session), Some(&before_replay));
            return Ok(());
        }
        assert_eq!(after.vitals().health, 8_600);
        assert_eq!(after.vitals().mana, 1_000);
        let before_replay = after.clone();
        assert!(
            states
                .apply_soul_war_torment_pulse(
                    &runtime,
                    native,
                    source,
                    player,
                    session,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(10_000_000)
                )
                .is_none()
        );
        assert_eq!(states.get(&runtime, player, session), Some(&before_replay));
        runtime.remove_encounter_actor(source)?;
        // This owner-level fixture now moves the actual same player to the
        // donor Hatred spectator box; a Symbol at the Mego center would never
        // execute the source GetSpectators reset for this player.
        let goal = MovementLocalPosition {
            x: 33743,
            y: 31599,
            floor: 14,
        };
        loop {
            let current = runtime.read_actor_position(player)?;
            let at = current.position();
            if at == goal {
                break;
            }
            let next = if at.x != goal.x {
                MovementLocalPosition {
                    x: at.x + if at.x < goal.x { 1 } else { -1 },
                    ..at
                }
            } else {
                MovementLocalPosition {
                    y: at.y + if at.y < goal.y { 1 } else { -1 },
                    ..at
                }
            };
            runtime
                .borrow_movement_position()
                .commit_cardinal(current, next)?;
        }
        let symbol = runtime
            .spawn_arena_opponent(
                "oteryn:creature.symbol_of_hatred",
                MovementLocalPosition {
                    x: 33738,
                    y: 31599,
                    floor: 14,
                },
                |_, _| Some(true),
            )
            .map_err(|error| format!("actual source actor admission: {error:?}"))?;
        let pulse = states
            .apply_soul_war_torment_pulse(
                &runtime,
                native,
                symbol,
                player,
                session,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(13_000_000),
            )
            .ok_or("literal Symbol helper must publish its missing-Mego reset")?;
        assert_eq!(
            pulse.damage, 0,
            "the donor ignores Symbol's fifth Hatred argument"
        );
        let reset = states
            .get(&runtime, player, session)
            .ok_or("source reset Player")?;
        assert_eq!(reset.soul_war_torment_count(), 0);
        assert_eq!(reset.soul_war_torment_icon_count(), None);
        assert_eq!(reset.vitals().health, 8_600);
        let before_wrong_session = reset.clone();
        assert!(
            states
                .apply_soul_war_torment_pulse(
                    &runtime,
                    native,
                    symbol,
                    player,
                    GameSessionId::decode(&id(94)).map_err(|e| format!("{e:?}"))?,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(16_000_000),
                )
                .is_none()
        );
        assert_eq!(
            states.get(&runtime, player, session),
            Some(&before_wrong_session)
        );
        Ok(())
    }
    #[test]
    fn srx3_hatred_native_postblock_primary_uses_actual_issuer_and_memoizes_once() -> TestResult {
        use crate::player_lethal::{PlayerHitKind, PlayerLethalVitals};
        let world = WorldId::decode(&id(100)).map_err(|e| format!("{e:?}"))?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let native = controller
            .active()
            .and_then(|a| a.native_gameplay())
            .ok_or("actual native generation")?;
        let scope = RuntimeScopeRefV1::instance(world, id(101)).map_err(|e| format!("{e:?}"))?;
        let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
            scope,
            NodeId::decode(&id(102)).map_err(|e| format!("{e:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin.into_channel_pin(),
        )?;
        runtime.install_companion_policies(native.policies().clone())?;
        let session = GameSessionId::decode(&id(103)).map_err(|e| format!("{e:?}"))?;
        let reserved = runtime.reserve_fresh_session(session)?;
        let target = runtime.commit_fresh_session(reserved)?;
        let at = MovementLocalPosition {
            x: 33743,
            y: 31599,
            floor: 14,
        };
        runtime.initialize_pinned_test_position(target, at)?;
        let boss = runtime
            .spawn_arena_opponent(
                "oteryn:creature.goshnar_s_hatred",
                MovementLocalPosition { x: at.x + 1, ..at },
                |_, _| Some(true),
            )
            .map_err(|error| format!("actual native source: {error:?}"))?;
        assert!(
            runtime
                .soul_war_health_callback(boss)?
                .is_some_and(|proof| proof.is_empowered_boss())
        );
        let boss_before = runtime.companion_snapshot(boss)?;
        runtime.change_soul_war_hatred_percent(
            &boss_before,
            0,
            oteryn_gameplay_entities::carrier::soul_war_hatred::HatredEmpowerment::Add(10),
        )?;
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                target,
                session,
                CharacterCastFacts {
                    vocation: Vocation::Druid,
                    level: 500,
                    magic_level: 0,
                    max_health: 10_000,
                    max_mana: 1_000,
                    max_soul: 100,
                },
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .ok_or("canonical target")?;
        let now = crate::foundation::owner_timer::SemanticTimeMicros::from_micros(1_000_000);
        let receipt = states
            .apply_source_typed_attack_damage(
                &mut runtime,
                boss,
                target,
                session,
                100,
                PlayerHitKind::Physical,
                "source.hatred.primary.postblock",
                now,
            )
            .ok_or("actual post-block source hit")?;
        assert_eq!(
            receipt.applied, 110,
            "source onHealthChange multiplies already-blocked PRIMARY100, not raw pre-armor damage"
        );
        assert_eq!(
            states
                .get(&runtime, target, session)
                .ok_or("target")?
                .vitals()
                .health,
            9_890
        );
        let current_boss = runtime.companion_snapshot(boss)?;
        runtime.change_soul_war_hatred_percent(
            &current_boss,
            10,
            oteryn_gameplay_entities::carrier::soul_war_hatred::HatredEmpowerment::Add(10),
        )?;
        let replay = states
            .apply_source_typed_attack_damage(
                &mut runtime,
                boss,
                target,
                session,
                100,
                PlayerHitKind::Physical,
                "source.hatred.primary.postblock",
                now,
            )
            .ok_or("exact native replay")?;
        assert_eq!(replay, receipt);
        assert_eq!(
            states
                .get(&runtime, target, session)
                .ok_or("target")?
                .vitals()
                .health,
            9_890
        );
        let definition = crate::foundation::ConditionDefinition::new(
            "test.source.hatred.condition",
            1,
            crate::foundation::ConditionValues::TimedStatus {
                kind: crate::foundation::StatusKind::Drunk,
                duration_ms: 1000,
            },
        )
        .ok_or("canonical condition")?;
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([19; 32]);
        let facts = crate::foundation::ApplicationFacts {
            now: now.get(),
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(id(108)),
        };
        let stamp = runtime.issue_owner_work()?;
        let composite = states
            .apply_typed_composite_attack_damage_current(
                &mut runtime,
                target,
                session,
                100,
                PlayerHitKind::Physical,
                "source.hatred.primary.with-condition",
                boss,
                std::slice::from_ref(&definition),
                &facts,
                &[],
                stamp,
            )
            .ok_or("same canonical HP/condition owner")?;
        assert_eq!(
            composite.applied, 120,
            "source PRIMARY after block uses current20%; secondary conditions stay native and unscaled"
        );
        assert!(
            states
                .get(&runtime, target, session)
                .ok_or("actual target")?
                .owned_conditions()
                .has_status(crate::foundation::StatusKind::Drunk, now.get())
        );
        let settled = states
            .get(&runtime, target, session)
            .ok_or("settled composite target")?
            .clone();
        let current_boss = runtime.companion_snapshot(boss)?;
        runtime.change_soul_war_hatred_percent(
            &current_boss,
            20,
            oteryn_gameplay_entities::carrier::soul_war_hatred::HatredEmpowerment::Add(10),
        )?;
        let stamp = runtime.issue_owner_work()?;
        let replay = states
            .apply_typed_composite_attack_damage_current(
                &mut runtime,
                target,
                session,
                100,
                PlayerHitKind::Physical,
                "source.hatred.primary.with-condition",
                boss,
                std::slice::from_ref(&definition),
                &facts,
                &[],
                stamp,
            )
            .ok_or("exact composite replay after source empowerment")?;
        assert_eq!(replay, composite);
        assert_eq!(
            states.get(&runtime, target, session),
            Some(&settled),
            "replay retains the original primary receipt and secondary condition"
        );
        assert!(
            crate::domain::character_name::CharacterName::parse("Goshnar's Megalomania").is_err(),
            "the donor's Player-name Mego branch is unreachable under accepted naming policy1"
        );
        Ok(())
    }
}
