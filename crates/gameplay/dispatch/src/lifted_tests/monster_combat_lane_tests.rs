#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

    use crate::ability::RevisionSet;
    use crate::ai_think::ThinkOccurrence;
    use crate::content::ProjectReferenceRecord;
    use crate::content::ProjectV2AuthoringProfile;
    use crate::content::ProjectV2AuthoringProfileData as Data;
    use crate::content::ProjectV2BehaviorAuthoring;
    use crate::content::ProjectV2DefinitionRef as Ref;
    use crate::content::ProjectV2Family;
    use crate::creature_attack_geometry::Facing;
    use crate::creature_auto_attack::AttackError;
    use crate::creature_auto_attack::CurrentCombatFactsReader;
    use crate::creature_auto_attack::{AttackFacts, CurrentConditionPolicy};
    use crate::creature_damage_spell::SpellCombatFacts;
    use crate::creature_damage_spell::SpellWorldReader;
    use crate::foundation::ApplicationFacts;
    use crate::foundation::ChannelRuntimeV1;
    use crate::foundation::ExactActorRef;
    use crate::foundation::GameSessionId;
    use crate::foundation::RuntimeWorkStamp;
    use crate::foundation::ScopeRuntimeFence;
    use crate::foundation::TickFacts;
    use crate::foundation::owner_timer::OwnerClock;
    use crate::foundation::owner_timer::SemanticTimeMicros;
    use crate::foundation::owner_timer::VirtualOwnerClock;
    use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1, crystal_timer_fixture};
    use crate::gameplay_transport::actor_spell::ChannelSpellStates;
    use crate::gameplay_transport::actor_spell::{
        observe_vitals,
        tests::{FACTS, runtime_with_player},
    };
    use crate::monster_combat_lane::native_invisibility::InvisibleTileCombatPolicy;
    use crate::monster_combat_lane::native_ticks::NativeRegenerationRegistry;
    use crate::monster_combat_lane::*;
    use crate::monster_owner_cycle::CycleError;
    use crate::spell::chain::TilePosition;
    use oteryn_simulation_determinism::DecisionOccurrenceId;
    use oteryn_simulation_determinism::GameplayDecisionRoot;
    use sha2::Digest;
    use sha2::Sha256;
    /// Explicit current-world test policy only; these values are not production defaults.
    struct CurrentWorld {
        tick_inputs: Option<TrustedMonsterConditionTickInputs>,
        movement_scope: Option<crate::content::static_cell_engine::EngineeringStaticCellScope>,
        movement_index: Option<crate::content::static_cell_engine::EngineeringStaticCellIndex>,
        movement_ground: bool,
        target: ExactActorRef,
        session: GameSessionId,
        missing: bool,
    }
    impl CurrentWorld {
        fn facts(
            &self,
            r: &ChannelRuntimeV1,
            i: ExactActorRef,
            t: ExactActorRef,
            s: GameSessionId,
        ) -> Option<AttackFacts> {
            if self.missing || t != self.target || s != self.session {
                return None;
            }
            r.read_actor_position(i).ok()?;
            r.player_control_facts(t, s).ok()?;
            Some(AttackFacts {
                issuer: i,
                target: t,
                session: s,
                revision: 1,
                visible: true,
                issuer_pz: false,
                target_pz: false,
                issuer_protected: false,
                target_protected: false,
                defense: 0,
                armor: 0,
            })
        }
    }
    impl CurrentCombatFactsReader for CurrentWorld {
        fn read_attack(
            &mut self,
            r: &ChannelRuntimeV1,
            i: ExactActorRef,
            t: ExactActorRef,
            s: GameSessionId,
            _: RuntimeWorkStamp,
        ) -> Option<AttackFacts> {
            self.facts(r, i, t, s)
        }
        fn read_condition_policy(
            &mut self,
            r: &ChannelRuntimeV1,
            _: ExactActorRef,
            t: ExactActorRef,
            s: GameSessionId,
            _: RuntimeWorkStamp,
        ) -> Option<CurrentConditionPolicy> {
            r.player_control_facts(t, s).ok()?;
            (!self.missing).then_some(CurrentConditionPolicy {
                immunities: Vec::new(),
                base_speed: 220,
            })
        }
    }
    impl SpellWorldReader for CurrentWorld {
        fn current_players(
            &mut self,
            r: &ChannelRuntimeV1,
            _: RuntimeWorkStamp,
        ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
            r.player_control_facts(self.target, self.session).ok()?;
            (!self.missing).then_some(vec![(self.target, self.session)])
        }
        fn current_facing(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<Facing> {
            None
        }
        fn tile_allowed(
            &mut self,
            r: &ChannelRuntimeV1,
            i: ExactActorRef,
            _: i32,
            _: i32,
            floor: i16,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            let p = r.read_actor_position(i).ok()?.position();
            (!self.missing).then_some(floor == p.floor)
        }
        fn combat(
            &mut self,
            r: &ChannelRuntimeV1,
            i: ExactActorRef,
            t: ExactActorRef,
            s: GameSessionId,
            _: Option<&str>,
            _: RuntimeWorkStamp,
        ) -> Option<SpellCombatFacts> {
            Some(SpellCombatFacts {
                attack: self.facts(r, i, t, s)?,
                multiplier_ppm: 1_000_000,
                immune: false,
                condition_policy: Some(CurrentConditionPolicy {
                    immunities: Vec::new(),
                    base_speed: 220,
                }),
            })
        }
    }

    impl InvisibleTileCombatPolicy for CurrentWorld {
        fn current_tile_allowed(&mut self, r: &ChannelRuntimeV1, c: ExactActorRef) -> Option<bool> {
            r.read_actor_position(c).ok()?;
            (!self.missing).then_some(true)
        }
    }
    impl MonsterCombatReader for CurrentWorld {
        fn source_creature_standing_field(
            &mut self,
            runtime: &ChannelRuntimeV1,
            actor: ExactActorRef,
            _stamp: RuntimeWorkStamp,
        ) -> Option<Option<crate::foundation::DotElement>> {
            if self.missing {
                return None;
            }
            self.tick_inputs.as_ref()?.current(runtime)?;
            runtime.read_actor_position(actor).ok()?;
            Some(None) // TEST_PARAMETERS: immutable qualified component room with no dynamic fields.
        }
        fn native_condition_tick_inputs<'a>(
            &'a mut self,
            runtime: &ChannelRuntimeV1,
            _stamp: RuntimeWorkStamp,
        ) -> Option<(
            &'a crate::content::NativeEntryMovementCells,
            &'a NativeRegenerationRegistry,
        )> {
            if self.missing {
                return None;
            }
            self.tick_inputs.as_ref()?.current(runtime)
        }
        fn regular_creature_movement<'a>(
            &'a mut self,
            r: &ChannelRuntimeV1,
            actor: ExactActorRef,
            _target: Option<(ExactActorRef, GameSessionId)>,
            _creature: &Ref,
            _stamp: RuntimeWorkStamp,
        ) -> Option<RegularCreatureMovementFacts<'a>> {
            r.read_actor_position(actor).ok()?;
            Some(RegularCreatureMovementFacts {
                direction: crate::movement::CardinalStep::East,
                scope: self.movement_scope.as_ref()?,
                index: self.movement_index.as_ref()?,
                ground: self.movement_ground.then_some(
                    &crate::movement::speed::EngineeringGroundSpeed
                        as &dyn crate::movement::speed::GroundSpeedSource,
                ),
                blocking: &[],
                tile_admits: Some(true),
            })
        }

        fn chain_sight(
            &mut self,
            r: &ChannelRuntimeV1,
            _from: TilePosition,
            _to: TilePosition,
            _stamp: RuntimeWorkStamp,
        ) -> Option<bool> {
            r.player_control_facts(self.target, self.session).ok()?;
            (!self.missing).then_some(true)
        }
        fn chain_path(
            &mut self,
            r: &ChannelRuntimeV1,
            _from: TilePosition,
            to: TilePosition,
            _stamp: RuntimeWorkStamp,
        ) -> Option<Vec<TilePosition>> {
            r.player_control_facts(self.target, self.session).ok()?;
            (!self.missing).then_some(vec![to])
        }

        fn self_condition_facts(
            &mut self,
            r: &ChannelRuntimeV1,
            c: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<SelfConditionFacts> {
            r.read_actor_position(c).ok()?;
            (!self.missing).then_some(SelfConditionFacts {
                base_speed: 220,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
            })
        }

        fn selected_player(
            &mut self,
            r: &ChannelRuntimeV1,
            _: RuntimeWorkStamp,
            _: ThinkOccurrence,
            _: &ProjectV2BehaviorAuthoring,
        ) -> Option<Option<(ExactActorRef, GameSessionId)>> {
            r.player_control_facts(self.target, self.session).ok()?;
            (!self.missing).then_some(Some((self.target, self.session)))
        }
        fn condition_tick_facts(
            &mut self,
            r: &ChannelRuntimeV1,
            t: ExactActorRef,
            s: GameSessionId,
            _: RuntimeWorkStamp,
        ) -> Option<TickFacts> {
            r.player_control_facts(t, s).ok()?;
            (!self.missing).then_some(TickFacts {
                in_protection_zone: false,
                standing_on_field: None,
            })
        }
    }
    fn setup() -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        ExactActorRef,
        ScopeRuntimeFence,
        MonsterCombatLane,
        CurrentWorld,
        VirtualOwnerClock,
    ) {
        let (mut r, t, s) = runtime_with_player(0x58);
        r.initialize_movement_test_position(
            t,
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
        )
        .expect("monster_combat_lane.rs:tests:2229: qualified fixture operation must succeed");
        let value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../apps/game-server/src/creature_auto_attack_test_data.json"
        ))
        .expect("monster_combat_lane.rs:tests:2231: qualified fixture operation must succeed");
        let records: Vec<ProjectReferenceRecord> = serde_json::from_value(value["records"].clone())
            .expect("monster_combat_lane.rs:tests:2233: qualified fixture operation must succeed");
        let profiles: Vec<ProjectV2AuthoringProfile> = serde_json::from_value(
            value["authoring_profiles"].clone(),
        )
        .expect("monster_combat_lane.rs:tests:2235: qualified fixture operation must succeed");
        let key = Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.1st_mate_ratticus".into(),
            revision: "definition-r1".into(),
        };
        let hp = profiles
            .iter()
            .find_map(|p| match &p.data {
                Data::Creature(c) if p.target == key => c.health,
                _ => None,
            })
            .expect("monster_combat_lane.rs:tests:2247: qualified fixture operation must succeed");
        let issuer = r
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 101,
                    y: 100,
                    floor: 7,
                },
                &key.key,
                hp as i64,
            )
            .expect("monster_combat_lane.rs:tests:2258: qualified fixture operation must succeed");
        let b = r.binding();
        let (mut fence, _) = crystal_timer_fixture(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .expect("monster_combat_lane.rs:tests:2264: qualified fixture operation must succeed");
        let mut lane = MonsterCombatLane::new(&r, &fence)
            .expect("monster_combat_lane.rs:tests:2265: qualified fixture operation must succeed");
        let first = lane
            .register_native(
                &r,
                &mut fence,
                issuer,
                &key,
                &records,
                &profiles,
                r.content_pin().server_artifact_digest(),
                SemanticTimeMicros::from_micros(0),
            )
            .expect("monster_combat_lane.rs:tests:2277: qualified fixture operation must succeed");
        assert_eq!(first.sequence, 0);
        assert_eq!(lane.actors[0].melee.len(), 1);
        assert_eq!(lane.actors[0].spells.len(), 1);
        assert!(lane.actors[0].unqualified.is_empty());
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &r,
                t,
                s,
                crate::spell::cast::CharacterCastFacts {
                    max_health: 100000,
                    ..FACTS
                },
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .expect("monster_combat_lane.rs:tests:2295: qualified fixture operation must succeed");
        (
            r,
            states,
            issuer,
            fence,
            lane,
            CurrentWorld {
                tick_inputs: None,
                movement_scope: None,
                movement_index: None,
                movement_ground: false,
                target: t,
                session: s,
                missing: false,
            },
            VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0)),
        )
    }
    #[test]
    fn aggregate_source_chain_schedules_and_drains_native_hp_on_same_owner_cycle() {
        let (mut r, mut states, old, mut fence, _, mut world, clock) = setup();
        r.remove_test_actor(old)
            .expect("monster_combat_lane.rs:tests:2317: qualified fixture operation must succeed");
        let value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../apps/game-server/src/creature_auto_attack_test_data.json"
        ))
        .expect("monster_combat_lane.rs:tests:2319: qualified fixture operation must succeed");
        let records: Vec<ProjectReferenceRecord> = serde_json::from_value(value["records"].clone())
            .expect("monster_combat_lane.rs:tests:2321: qualified fixture operation must succeed");
        let profiles: Vec<ProjectV2AuthoringProfile> = serde_json::from_value(
            value["authoring_profiles"].clone(),
        )
        .expect("monster_combat_lane.rs:tests:2323: qualified fixture operation must succeed");
        let key = Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.bony_sea_devil".into(),
            revision: "definition-r1".into(),
        };
        let hp = profiles
            .iter()
            .find_map(|p| match &p.data {
                Data::Creature(c) if p.target == key => c.health,
                _ => None,
            })
            .expect("monster_combat_lane.rs:tests:2335: qualified fixture operation must succeed");
        let actor = r
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 101,
                    y: 100,
                    floor: 7,
                },
                &key.key,
                hp as i64,
            )
            .expect("monster_combat_lane.rs:tests:2346: qualified fixture operation must succeed");
        let mut lane = MonsterCombatLane::new(&r, &fence)
            .expect("monster_combat_lane.rs:tests:2347: qualified fixture operation must succeed");
        lane.register_native(
            &r,
            &mut fence,
            actor,
            &key,
            &records,
            &profiles,
            r.content_pin().server_artifact_digest(),
            SemanticTimeMicros::from_micros(0),
        )
        .expect("monster_combat_lane.rs:tests:2358: qualified fixture operation must succeed");
        let mut observed = false;
        for _ in 0..256 {
            let pulse = lane
                .run(
                    &mut r,
                    &mut states,
                    &mut fence,
                    &clock,
                    &mut world,
                    revisions(),
                )
                .expect(
                    "monster_combat_lane.rs:tests:2370: qualified fixture operation must succeed",
                );
            let steps = pulse.chain_steps.expect(
                "monster_combat_lane.rs:tests:2371: qualified fixture operation must succeed",
            );
            if pulse
                .thinks
                .iter()
                .any(|t| t.chains.iter().any(|(_, r)| r.is_ok()))
            {
                assert!(!steps.is_empty());
                assert!(steps.iter().any(|(_, _, r)| r.as_ref().is_ok_and(|h| {
                    h.targets.iter().any(|(_, t)| {
                        t.as_ref()
                            .is_ok_and(|t| t.damage.is_some_and(|d| d.applied > 0))
                    })
                })));
                observed = true;
                break;
            }
            clock.advance(1_000_000);
        }
        assert!(
            observed,
            "actual source native schedule chance eventually supplies chain; aggregate drains actual HP owner"
        );
    }

    fn revisions() -> RevisionSet {
        RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r1",
            "sim-r1",
        )
        .expect("monster_combat_lane.rs:tests:2403: qualified fixture operation must succeed")
    }
    #[test]
    fn actual_aggregate_native_regular_movement_uses_current_ground_pacing_and_fence() {
        use crate::content::static_cell_engine::*;
        use crate::content::*;
        let (mut runtime, mut states, actor, mut fence, mut lane, mut reader, clock) = setup();
        let before = runtime
            .read_actor_position(actor)
            .expect("monster_combat_lane.rs:tests:2410: qualified fixture operation must succeed");
        // The committed native room compiler supplies valid frame/revision/lock vocabulary.
        // Cells below remain explicitly local engineering movement vectors at102/103,
        // not a claim that those cells belong to the committed entry room.
        let native_room = qualify_native_entry_room(runtime.binding().world_id())
            .expect("monster_combat_lane.rs:tests:2414: qualified fixture operation must succeed");
        let qualified_scope = native_room.movement_cells().scope();
        let scope = EngineeringStaticCellScope {
            world_id: runtime.binding().world_id(),
            coordinate_frame: qualified_scope.coordinate_frame.clone(),
            map_revision: qualified_scope.map_revision.clone(),
            generation_digest: runtime.content_pin().server_artifact_digest(),
            content_lock: qualified_scope.content_lock.clone(),
        };
        let cells = [102, 103]
            .into_iter()
            .map(|x| EngineeringStaticCellClaim {
                scope: scope.clone(),
                cell: LogicalCell { x, y: 100, z: 7 },
                collision: EngineeringCollisionClaim::Qualified(CollisionClass::Walkable),
            })
            .collect();
        reader.movement_index =
            Some(EngineeringStaticCellIndex::from_claims(cells).expect(
                "monster_combat_lane.rs:tests:2431: qualified fixture operation must succeed",
            ));
        reader.movement_scope = Some(scope);
        // Actual selected native profile says100, not the former test/default220.
        let source = lane.actors[0]
            .native_speed
            .as_ref()
            .expect("monster_combat_lane.rs:tests:2434: qualified fixture operation must succeed");
        assert_eq!(
            crate::movement::speed::runtime_creature_speed(
                &runtime,
                actor,
                source,
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(clock.now().get())
            ),
            Some(100)
        );
        let pulse = lane
            .run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions(),
            )
            .expect("monster_combat_lane.rs:tests:2453: qualified fixture operation must succeed");
        assert!(matches!(
            &pulse.movements[0].1,
            Err(crate::movement::speed::CreatureCadenceError::MissingMap)
        ));
        assert_eq!(
            runtime.read_actor_position(actor).expect(
                "monster_combat_lane.rs:tests:2458: qualified fixture operation must succeed"
            ),
            before
        );
        reader.movement_ground = true;
        let pulse = lane
            .run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions(),
            )
            .expect("monster_combat_lane.rs:tests:2469: qualified fixture operation must succeed");
        assert!(matches!(&pulse.movements[0].1, Ok(Some(_))));
        let moved = runtime
            .read_actor_position(actor)
            .expect("monster_combat_lane.rs:tests:2471: qualified fixture operation must succeed");
        assert_eq!(moved.position().x, 102);
        let pulse = lane
            .run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions(),
            )
            .expect("monster_combat_lane.rs:tests:2482: qualified fixture operation must succeed");
        assert!(matches!(&pulse.movements[0].1, Ok(None)));
        assert_eq!(
            runtime.read_actor_position(actor).expect(
                "monster_combat_lane.rs:tests:2484: qualified fixture operation must succeed"
            ),
            moved
        );
        let duration = crate::movement::speed::StepSpeedTable::embedded()
            .expect("monster_combat_lane.rs:tests:2486: qualified fixture operation must succeed")
            .step_duration(100, 150)
            .expect("monster_combat_lane.rs:tests:2488: qualified fixture operation must succeed");
        clock.advance(
            u64::try_from(duration.as_micros()).expect(
                "monster_combat_lane.rs:tests:2489: qualified fixture operation must succeed",
            ) - 1,
        );
        let pulse = lane
            .run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions(),
            )
            .expect("monster_combat_lane.rs:tests:2499: qualified fixture operation must succeed");
        assert!(matches!(&pulse.movements[0].1, Ok(None)));
        assert_eq!(
            runtime.read_actor_position(actor).expect(
                "monster_combat_lane.rs:tests:2501: qualified fixture operation must succeed"
            ),
            moved
        );
        clock.advance(1);
        let pulse = lane
            .run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions(),
            )
            .expect("monster_combat_lane.rs:tests:2512: qualified fixture operation must succeed");
        assert!(matches!(&pulse.movements[0].1, Ok(Some(_))));
        let final_position = runtime
            .read_actor_position(actor)
            .expect("monster_combat_lane.rs:tests:2514: qualified fixture operation must succeed");
        assert_eq!(final_position.position().x, 103);
        fence
            .apply_external_grant(crate::foundation::ScopeOwnershipGeneration::new(2).expect(
                "monster_combat_lane.rs:tests:2517: qualified fixture operation must succeed",
            ))
            .expect("monster_combat_lane.rs:tests:2518: qualified fixture operation must succeed");
        clock.advance(1_000_000);
        assert!(
            lane.run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions()
            )
            .is_err()
        );
        assert_eq!(
            runtime.read_actor_position(actor).expect(
                "monster_combat_lane.rs:tests:2531: qualified fixture operation must succeed"
            ),
            final_position
        );
    }

    #[test]
    fn composed_event_zero_native_melee_and_spell_real_hp_repeat_and_dead_skip() {
        let (mut r, mut states, issuer, mut fence, mut lane, mut world, clock) = setup();
        let first = lane
            .run(
                &mut r,
                &mut states,
                &mut fence,
                &clock,
                &mut world,
                revisions(),
            )
            .expect("monster_combat_lane.rs:tests:2546: qualified fixture operation must succeed");
        assert_eq!(first.thinks.len(), 1);
        assert_eq!(first.thinks[0].occurrence.sequence, 0);
        assert!(first.thinks[0].selection.is_ok());
        assert!(first.melee.is_empty());
        let mut melee_damage = 0;
        let mut spell_damage = 0;
        for _ in 0..128 {
            clock.advance(1_000_000);
            let pulse = lane
                .run(
                    &mut r,
                    &mut states,
                    &mut fence,
                    &clock,
                    &mut world,
                    revisions(),
                )
                .expect(
                    "monster_combat_lane.rs:tests:2564: qualified fixture operation must succeed",
                );
            for (_, hit) in pulse.melee {
                if let Ok(hit) = hit {
                    melee_damage += hit.damage.map_or(0, |d| d.applied)
                }
            }
            for think in pulse.thinks {
                assert!(think.schedule.is_ok());
                for (_, hit) in think.spells {
                    if let Ok(hit) = hit {
                        for (_, target) in hit.targets {
                            if let Ok(target) = target {
                                spell_damage += target.damage.map_or(0, |d| d.applied)
                            }
                        }
                    }
                }
            }
        }
        assert!(melee_damage > 0);
        assert!(spell_damage > 0);
        let before = observe_vitals(&r, &states, world.target, world.session)
            .expect("monster_combat_lane.rs:tests:2585: qualified fixture operation must succeed");
        assert_eq!(before.1.health, 100000 - melee_damage - spell_damage);
        let replay = lane
            .run(
                &mut r,
                &mut states,
                &mut fence,
                &clock,
                &mut world,
                revisions(),
            )
            .expect("monster_combat_lane.rs:tests:2596: qualified fixture operation must succeed");
        assert!(replay.thinks.is_empty());
        assert!(replay.melee.is_empty());
        assert_eq!(
            observe_vitals(&r, &states, world.target, world.session).expect(
                "monster_combat_lane.rs:tests:2600: qualified fixture operation must succeed"
            ),
            before
        );
        r.remove_test_actor(issuer)
            .expect("monster_combat_lane.rs:tests:2603: qualified fixture operation must succeed");
        clock.advance(1_000_000);
        let dead = lane
            .run(
                &mut r,
                &mut states,
                &mut fence,
                &clock,
                &mut world,
                revisions(),
            )
            .expect("monster_combat_lane.rs:tests:2614: qualified fixture operation must succeed");
        assert!(dead.thinks.is_empty());
        assert!(dead.melee.is_empty());
        assert!(lane.actors.is_empty());
    }
    #[test]
    fn composed_missing_current_selection_and_superseded_grant_never_changes_hp() {
        let (mut r, mut states, _, mut fence, mut lane, mut world, clock) = setup();
        let before = observe_vitals(&r, &states, world.target, world.session)
            .expect("monster_combat_lane.rs:tests:2622: qualified fixture operation must succeed");
        world.missing = true;
        let pulse = lane
            .run(
                &mut r,
                &mut states,
                &mut fence,
                &clock,
                &mut world,
                revisions(),
            )
            .expect("monster_combat_lane.rs:tests:2633: qualified fixture operation must succeed");
        assert_eq!(
            pulse.thinks[0].selection,
            Err(AttackError::MissingCombatFacts)
        );
        assert!(pulse.conditions.is_err());
        assert_eq!(
            observe_vitals(&r, &states, world.target, world.session).expect(
                "monster_combat_lane.rs:tests:2640: qualified fixture operation must succeed"
            ),
            before
        );
        fence
            .apply_external_grant(crate::foundation::ScopeOwnershipGeneration::new(2).expect(
                "monster_combat_lane.rs:tests:2644: qualified fixture operation must succeed",
            ))
            .expect("monster_combat_lane.rs:tests:2645: qualified fixture operation must succeed");
        clock.advance(1_000_000);
        assert!(matches!(
            lane.run(
                &mut r,
                &mut states,
                &mut fence,
                &clock,
                &mut world,
                revisions()
            ),
            Err(CombatLaneError::Cycle(CycleError::StaleOwner))
        ));
        assert_eq!(
            observe_vitals(&r, &states, world.target, world.session).expect(
                "monster_combat_lane.rs:tests:2659: qualified fixture operation must succeed"
            ),
            before
        );
    }
    fn source_creature_dot_loaded_room_fixture() -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        ExactActorRef,
        GameSessionId,
        TrustedMonsterConditionTickInputs,
        MovementLocalPosition,
    ) {
        use crate::content::ProjectSnapshot;
        use {
            crate::foundation::NodeId,
            oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
        };
        let (base, _, session) = runtime_with_player(0x59);
        let world = base.binding().world_id();
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/spells.manifest.json");
        // Load exact committed v5 artifact pins, selecting only the local four-cell map.
        // The separately optional real-world addon is not needed by this component fixture.
        // Native compiler independently validates every included artifact SHA below.
        use crate::content::native_gameplay::{
            NativeGameplayInput, NativeGameplayMapProfile, NativeTrainingInput, PinnedGameplayBytes,
        };
        let manifest_value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&manifest).expect(
                "monster_combat_lane.rs:tests:2684: qualified fixture operation must succeed",
            ))
            .expect("monster_combat_lane.rs:tests:2684: qualified fixture operation must succeed");
        let pinned = |name: &str| -> PinnedGameplayBytes {
            let pin = &manifest_value[name];
            let bytes = std::fs::read(
                manifest
                    .parent()
                    .expect("monster_combat_lane.rs:tests:2690: qualified fixture operation must succeed")
                    .join(pin["path"].as_str().expect("monster_combat_lane.rs:tests:2691: qualified fixture operation must succeed")),
            )
            .expect("monster_combat_lane.rs:tests:2693: qualified fixture operation must succeed");
            PinnedGameplayBytes {
                bytes,
                sha256: pin["sha256"].as_str().expect("monster_combat_lane.rs:tests:2696: qualified fixture operation must succeed").to_owned(),
            }
        };
        let native_input = NativeGameplayInput {
            cosmetics: NativeGameplayInput::unavailable_cosmetics(),
            native_map_profile: NativeGameplayMapProfile::SourceQualifiedSpellEntryR2,
            catalog: pinned("catalog"),
            source_selection: pinned("source_selection"),
            creature_profiles: pinned("creature_profiles"),
            presentation_profiles: pinned("presentation_profiles"),
            item_profiles: Some(pinned("item_profiles")),
            spell_appearances: Some(pinned("spell_appearances")),
            build_training: Some(NativeTrainingInput {
                profile: pinned("build_training"),
                content_revision: manifest_value["build_training"]["content_revision"]
                    .as_str()
                    .expect("monster_combat_lane.rs:tests:2711: qualified fixture operation must succeed")
                    .to_owned(),
                magnitude_policy: serde_json::from_value(
                    manifest_value["build_training"]["magnitude_policy"].clone(),
                )
                .expect("monster_combat_lane.rs:tests:2716: qualified fixture operation must succeed"),
            }),
            familiar_config: Some(pinned("familiar_config")),
            familiar_defenses: Some(pinned("familiar_defenses")),
            wheel_profile: Some(pinned("wheel_profile")),
            source_world: None,
            progression: None,
            item_keys: None,
            loot_tables: None,
        };
        let room = crate::content::qualify_selected_native_gameplay_room(world, &native_input)
            .expect("monster_combat_lane.rs:tests:2724: qualified fixture operation must succeed");
        let documents = crate::content::native_spell_entry_room_documents(world)
            .expect("monster_combat_lane.rs:tests:2725: qualified fixture operation must succeed");
        let snapshot = ProjectSnapshot::new(
            documents.documents().clone(),
            crate::content::native_spell_entry_candidate_limits().project,
        )
        .expect("monster_combat_lane.rs:tests:2730: qualified fixture operation must succeed");
        let source = snapshot
            .parse_native_spell_entry()
            .expect("monster_combat_lane.rs:tests:2731: qualified fixture operation must succeed");
        let draft = source.project().migrate_to_v2();
        // Digest the exact canonical native documents actually parsed above.
        let loader_digest: [u8; 32] =
            Sha256::digest(serde_json::to_vec(documents.documents()).expect(
                "monster_combat_lane.rs:tests:2735: qualified fixture operation must succeed",
            ))
            .into();
        let start = room.entry_start();
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (start.x, start.y, start.floor),
        );
        let node = NodeId::decode(&[1, 144, 0, 0, 0, 0x62, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 0x62])
            .expect("monster_combat_lane.rs:tests:2747: qualified fixture operation must succeed");
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            base.binding().channel_id(),
            node,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .expect("monster_combat_lane.rs:tests:2759: qualified fixture operation must succeed");
        let reservation = runtime
            .reserve_fresh_session(session)
            .expect("monster_combat_lane.rs:tests:2760: qualified fixture operation must succeed");
        let target = runtime
            .commit_fresh_session(reservation)
            .expect("monster_combat_lane.rs:tests:2761: qualified fixture operation must succeed");
        runtime
            .initialize_first_entry_position(target)
            .expect("monster_combat_lane.rs:tests:2762: qualified fixture operation must succeed");
        let inputs = TrustedMonsterConditionTickInputs::from_test_native_room(
            &runtime,
            &room,
            &draft,
            loader_digest,
        )
        .expect("monster_combat_lane.rs:tests:2769: qualified fixture operation must succeed");
        assert_eq!(
            inputs.regeneration.len(),
            0,
            "actual qualified native room has no regeneration source profile"
        );
        assert!(
            inputs.current(&base).is_none(),
            "equal World alone cannot authorize another content pin"
        );
        assert!(
            TrustedMonsterConditionTickInputs::from_test_native_room(
                &base,
                &room,
                &draft,
                loader_digest
            )
            .is_none()
        );
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                target,
                session,
                crate::spell::cast::CharacterCastFacts {
                    max_health: 100000,
                    ..FACTS
                },
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .expect("monster_combat_lane.rs:tests:2801: qualified fixture operation must succeed");
        crate::content::qualified_native_gameplay_test_state(room.compiled())
            .expect("monster_combat_lane.rs:tests:2803: qualified fixture operation must succeed")
            .install_companion_policies(&mut runtime)
            .expect("monster_combat_lane.rs:tests:2805: qualified fixture operation must succeed");
        (
            runtime,
            states,
            target,
            session,
            inputs,
            MovementLocalPosition {
                x: start.x,
                y: start.y,
                floor: start.floor,
            },
        )
    }
    #[test]
    fn loaded_actor_roster_drives_real_creature_dot_owner_and_unknown_map_refuses_without_cursor_loss()
     {
        use crate::foundation::{
            ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionValues, DotElement,
        };
        let (mut runtime, mut states, source, session, inputs, start) =
            source_creature_dot_loaded_room_fixture();
        let key = Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.1st_mate_ratticus".into(),
            revision: "definition-r1".into(),
        };
        let actor = runtime
            .admit_source_pinned_lab_creature(start, &key.key, 200000)
            .expect("monster_combat_lane.rs:tests:2834: qualified fixture operation must succeed");
        runtime
            .install_creature_policy(actor, &key.key)
            .expect("monster_combat_lane.rs:tests:2835: qualified fixture operation must succeed");
        let value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../apps/game-server/src/creature_auto_attack_test_data.json"
        ))
        .expect("monster_combat_lane.rs:tests:2837: qualified fixture operation must succeed");
        let records: Vec<ProjectReferenceRecord> = serde_json::from_value(value["records"].clone())
            .expect("monster_combat_lane.rs:tests:2839: qualified fixture operation must succeed");
        let profiles: Vec<ProjectV2AuthoringProfile> = serde_json::from_value(
            value["authoring_profiles"].clone(),
        )
        .expect("monster_combat_lane.rs:tests:2841: qualified fixture operation must succeed");
        let b = runtime.binding();
        let (mut fence, _) = crystal_timer_fixture(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .expect("monster_combat_lane.rs:tests:2847: qualified fixture operation must succeed");
        let mut lane = MonsterCombatLane::new(&runtime, &fence)
            .expect("monster_combat_lane.rs:tests:2848: qualified fixture operation must succeed");
        lane.register_native(
            &runtime,
            &mut fence,
            actor,
            &key,
            &records,
            &profiles,
            runtime.content_pin().server_artifact_digest(),
            SemanticTimeMicros::from_micros(0),
        )
        .expect("monster_combat_lane.rs:tests:2859: qualified fixture operation must succeed");
        let before = runtime
            .companion_snapshot(actor)
            .expect("monster_combat_lane.rs:tests:2860: qualified fixture operation must succeed");
        let mut next = before.state.clone();
        let definition = ConditionDefinition::new(
            "condition.test.loaded-creature-dot",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Fire,
                total_min: 30,
                total_max: 30,
                per_tick: 10,
                interval_ms: 1000,
                delayed: true,
            },
        )
        .expect("monster_combat_lane.rs:tests:2874: qualified fixture operation must succeed");
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let facts = ApplicationFacts {
            now: 0,
            base_speed: 100,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: false,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([127; 16]),
        };
        next.conditions
            .apply(
                &definition,
                Some(source),
                ConditionSourceKind::Player,
                &[],
                &facts,
            )
            .expect("monster_combat_lane.rs:tests:2894: qualified fixture operation must succeed");
        runtime
            .compare_companion_state(&before, next)
            .expect("monster_combat_lane.rs:tests:2895: qualified fixture operation must succeed");
        let mut reader = CurrentWorld {
            tick_inputs: None,
            movement_scope: None,
            movement_index: None,
            movement_ground: false,
            target: source,
            session,
            missing: false,
        };
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(1_000_000));
        let before = runtime
            .companion_snapshot(actor)
            .expect("monster_combat_lane.rs:tests:2906: qualified fixture operation must succeed");
        let refused = lane
            .run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions(),
            )
            .expect("monster_combat_lane.rs:tests:2916: qualified fixture operation must succeed");
        assert!(
            refused
                .creature_conditions
                .iter()
                .any(|(a, r)| *a == actor && r.is_err())
        );
        assert_eq!(
            runtime
                .companion_snapshot(actor)
                .expect(
                    "monster_combat_lane.rs:tests:2924: qualified fixture operation must succeed"
                )
                .state
                .conditions,
            before.state.conditions,
            "missingactualmap cannot consume source cursor"
        );
        reader.tick_inputs = Some(inputs);
        let applied = lane
            .run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions(),
            )
            .expect("monster_combat_lane.rs:tests:2938: qualified fixture operation must succeed");
        let receipt = applied
            .creature_conditions
            .iter()
            .find(|(a, _)| *a == actor)
            .expect("monster_combat_lane.rs:tests:2943: qualified fixture operation must succeed")
            .1
            .as_ref()
            .expect("monster_combat_lane.rs:tests:2946: qualified fixture operation must succeed");
        assert_eq!(
            receipt.ticks.len(),
            1,
            "real registered roster producer reaches native periodic HP owner"
        );
        assert!(receipt.health_after < receipt.health_before);
        let hp = runtime
            .companion_snapshot(actor)
            .expect("monster_combat_lane.rs:tests:2953: qualified fixture operation must succeed")
            .health;
        let retry = lane
            .run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions(),
            )
            .expect("monster_combat_lane.rs:tests:2963: qualified fixture operation must succeed");
        assert!(
            retry
                .creature_conditions
                .iter()
                .find(|(a, _)| *a == actor)
                .expect(
                    "monster_combat_lane.rs:tests:2969: qualified fixture operation must succeed"
                )
                .1
                .as_ref()
                .expect(
                    "monster_combat_lane.rs:tests:2972: qualified fixture operation must succeed"
                )
                .ticks
                .is_empty()
        );
        assert_eq!(
            runtime
                .companion_snapshot(actor)
                .expect(
                    "monster_combat_lane.rs:tests:2977: qualified fixture operation must succeed"
                )
                .health,
            hp,
            "same time cannot replay heal/damage"
        );
    }

    #[test]
    fn aggregate_native_poison_ticks_with_actual_qualified_loader_room_and_refuses_missing_or_stale_inputs()
     {
        use crate::content::ProjectSnapshot;
        use {
            crate::foundation::{ConditionDefinition, ConditionValues, DotElement, NodeId},
            oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
        };
        let (base, _, session) = runtime_with_player(0x59);
        let world = base.binding().world_id();
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/spells.manifest.json");
        // Load exact committed v5 artifact pins, selecting only the local four-cell map.
        // The separately optional real-world addon is not needed by this component fixture.
        // Native compiler independently validates every included artifact SHA below.
        use crate::content::native_gameplay::{
            NativeGameplayInput, NativeGameplayMapProfile, NativeTrainingInput, PinnedGameplayBytes,
        };
        let manifest_value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&manifest).expect(
                "monster_combat_lane.rs:tests:3001: qualified fixture operation must succeed",
            ))
            .expect("monster_combat_lane.rs:tests:3001: qualified fixture operation must succeed");
        let pinned = |name: &str| -> PinnedGameplayBytes {
            let pin = &manifest_value[name];
            let bytes = std::fs::read(
                manifest
                    .parent()
                    .expect("monster_combat_lane.rs:tests:3007: qualified fixture operation must succeed")
                    .join(pin["path"].as_str().expect("monster_combat_lane.rs:tests:3008: qualified fixture operation must succeed")),
            )
            .expect("monster_combat_lane.rs:tests:3010: qualified fixture operation must succeed");
            PinnedGameplayBytes {
                bytes,
                sha256: pin["sha256"].as_str().expect("monster_combat_lane.rs:tests:3013: qualified fixture operation must succeed").to_owned(),
            }
        };
        let native_input = NativeGameplayInput {
            cosmetics: NativeGameplayInput::unavailable_cosmetics(),
            native_map_profile: NativeGameplayMapProfile::SourceQualifiedSpellEntryR2,
            catalog: pinned("catalog"),
            source_selection: pinned("source_selection"),
            creature_profiles: pinned("creature_profiles"),
            presentation_profiles: pinned("presentation_profiles"),
            item_profiles: Some(pinned("item_profiles")),
            spell_appearances: Some(pinned("spell_appearances")),
            build_training: Some(NativeTrainingInput {
                profile: pinned("build_training"),
                content_revision: manifest_value["build_training"]["content_revision"]
                    .as_str()
                    .expect("monster_combat_lane.rs:tests:3028: qualified fixture operation must succeed")
                    .to_owned(),
                magnitude_policy: serde_json::from_value(
                    manifest_value["build_training"]["magnitude_policy"].clone(),
                )
                .expect("monster_combat_lane.rs:tests:3033: qualified fixture operation must succeed"),
            }),
            familiar_config: Some(pinned("familiar_config")),
            familiar_defenses: Some(pinned("familiar_defenses")),
            wheel_profile: Some(pinned("wheel_profile")),
            source_world: None,
            progression: None,
            item_keys: None,
            loot_tables: None,
        };
        let room = crate::content::qualify_selected_native_gameplay_room(world, &native_input)
            .expect("monster_combat_lane.rs:tests:3041: qualified fixture operation must succeed");
        let documents = crate::content::native_spell_entry_room_documents(world)
            .expect("monster_combat_lane.rs:tests:3042: qualified fixture operation must succeed");
        let snapshot = ProjectSnapshot::new(
            documents.documents().clone(),
            crate::content::native_spell_entry_candidate_limits().project,
        )
        .expect("monster_combat_lane.rs:tests:3047: qualified fixture operation must succeed");
        let source = snapshot
            .parse_native_spell_entry()
            .expect("monster_combat_lane.rs:tests:3048: qualified fixture operation must succeed");
        let draft = source.project().migrate_to_v2();
        // Digest the exact canonical native documents actually parsed above.
        let loader_digest: [u8; 32] =
            Sha256::digest(serde_json::to_vec(documents.documents()).expect(
                "monster_combat_lane.rs:tests:3052: qualified fixture operation must succeed",
            ))
            .into();
        let start = room.entry_start();
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (start.x, start.y, start.floor),
        );
        let node = NodeId::decode(&[1, 144, 0, 0, 0, 0x62, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 0x62])
            .expect("monster_combat_lane.rs:tests:3064: qualified fixture operation must succeed");
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            base.binding().channel_id(),
            node,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .expect("monster_combat_lane.rs:tests:3076: qualified fixture operation must succeed");
        let reservation = runtime
            .reserve_fresh_session(session)
            .expect("monster_combat_lane.rs:tests:3077: qualified fixture operation must succeed");
        let target = runtime
            .commit_fresh_session(reservation)
            .expect("monster_combat_lane.rs:tests:3078: qualified fixture operation must succeed");
        runtime
            .initialize_first_entry_position(target)
            .expect("monster_combat_lane.rs:tests:3079: qualified fixture operation must succeed");
        let inputs = TrustedMonsterConditionTickInputs::from_test_native_room(
            &runtime,
            &room,
            &draft,
            loader_digest,
        )
        .expect("monster_combat_lane.rs:tests:3086: qualified fixture operation must succeed");
        assert_eq!(
            inputs.regeneration.len(),
            0,
            "actual qualified native room has no regeneration source profile"
        );
        assert!(
            inputs.current(&base).is_none(),
            "equal World alone cannot authorize another content pin"
        );
        assert!(
            TrustedMonsterConditionTickInputs::from_test_native_room(
                &base,
                &room,
                &draft,
                loader_digest
            )
            .is_none()
        );
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                target,
                session,
                crate::spell::cast::CharacterCastFacts {
                    max_health: 100000,
                    ..FACTS
                },
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .expect("monster_combat_lane.rs:tests:3118: qualified fixture operation must succeed");
        let definition = ConditionDefinition::new(
            "condition.test_qualified_loader_poison",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Poison,
                total_min: 40,
                total_max: 40,
                per_tick: 10,
                interval_ms: 1000,
                delayed: true,
            },
        )
        .expect("monster_combat_lane.rs:tests:3131: qualified fixture operation must succeed");
        let decision =
            GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let facts = ApplicationFacts {
            now: 0,
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &decision,
            occurrence: DecisionOccurrenceId::from_bytes([91; 16]),
        };
        let caster = runtime
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: start.x,
                    y: start.y,
                    floor: start.floor,
                },
                "oteryn:creature.1st_mate_ratticus",
                10000,
            )
            .expect("monster_combat_lane.rs:tests:3154: qualified fixture operation must succeed");
        assert!(states.install_owned_source_condition_fixture(
            &runtime,
            target,
            session,
            caster,
            &[definition],
            &facts
        ));
        runtime
            .remove_test_actor(caster)
            .expect("monster_combat_lane.rs:tests:3163: qualified fixture operation must succeed"); // Frozen source does not require a live caster.
        assert!(
            inputs
                .source_player_tick_spatial_facts(&runtime, target, session, None)
                .is_none()
        );
        assert!(
            !(inputs
                .source_player_tick_spatial_facts(&runtime, target, session, Some(None))
                .expect(
                    "monster_combat_lane.rs:tests:3172: qualified fixture operation must succeed"
                )
                .in_protection_zone)
        );
        let binding = runtime.binding();
        let (mut fence, _) = crystal_timer_fixture(
            RuntimeScopeRefV1::channel(world, binding.channel_id()),
            binding.scope_generation(),
        )
        .expect("monster_combat_lane.rs:tests:3181: qualified fixture operation must succeed");
        let mut lane = MonsterCombatLane::new(&runtime, &fence)
            .expect("monster_combat_lane.rs:tests:3182: qualified fixture operation must succeed");
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(1000000));
        let mut reader = CurrentWorld {
            tick_inputs: None,
            movement_scope: None,
            movement_index: None,
            movement_ground: false,
            target,
            session,
            missing: false,
        };
        let before = observe_vitals(&runtime, &states, target, session)
            .expect("monster_combat_lane.rs:tests:3193: qualified fixture operation must succeed");
        let conditions_before = states
            .owned_source_condition_fixture_snapshot(&runtime, target, session)
            .expect("monster_combat_lane.rs:tests:3196: qualified fixture operation must succeed");
        let refused = lane
            .run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions(),
            )
            .expect("monster_combat_lane.rs:tests:3206: qualified fixture operation must succeed");
        assert!(
            refused.conditions.expect(
                "monster_combat_lane.rs:tests:3207: qualified fixture operation must succeed"
            )[0]
            .1
            .is_none()
        );
        assert_eq!(
            observe_vitals(&runtime, &states, target, session).expect(
                "monster_combat_lane.rs:tests:3209: qualified fixture operation must succeed"
            ),
            before
        );
        assert_eq!(
            states
                .owned_source_condition_fixture_snapshot(&runtime, target, session)
                .expect(
                    "monster_combat_lane.rs:tests:3215: qualified fixture operation must succeed"
                ),
            conditions_before
        );
        reader.tick_inputs = Some(inputs);
        let applied = lane
            .run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions(),
            )
            .expect("monster_combat_lane.rs:tests:3228: qualified fixture operation must succeed");
        let ticks =
            applied.conditions.as_ref().expect(
                "monster_combat_lane.rs:tests:3229: qualified fixture operation must succeed",
            )[0]
            .1
            .as_ref()
            .expect("monster_combat_lane.rs:tests:3229: qualified fixture operation must succeed");
        assert_eq!(ticks.damage, 10);
        assert_eq!(
            observe_vitals(&runtime, &states, target, session)
                .expect(
                    "monster_combat_lane.rs:tests:3233: qualified fixture operation must succeed"
                )
                .1
                .health,
            99990
        );
        let after = observe_vitals(&runtime, &states, target, session)
            .expect("monster_combat_lane.rs:tests:3238: qualified fixture operation must succeed");
        lane.run(
            &mut runtime,
            &mut states,
            &mut fence,
            &clock,
            &mut reader,
            revisions(),
        )
        .expect("monster_combat_lane.rs:tests:3247: qualified fixture operation must succeed");
        assert_eq!(
            observe_vitals(&runtime, &states, target, session).expect(
                "monster_combat_lane.rs:tests:3249: qualified fixture operation must succeed"
            ),
            after
        );
        fence
            .apply_external_grant(crate::foundation::ScopeOwnershipGeneration::new(2).expect(
                "monster_combat_lane.rs:tests:3253: qualified fixture operation must succeed",
            ))
            .expect("monster_combat_lane.rs:tests:3254: qualified fixture operation must succeed");
        assert!(
            lane.run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions()
            )
            .is_err()
        );
        assert_eq!(
            observe_vitals(&runtime, &states, target, session).expect(
                "monster_combat_lane.rs:tests:3267: qualified fixture operation must succeed"
            ),
            after
        );
    }

    #[test]
    fn missing_loader_tick_inputs_refuse_and_native_target_tick_survives_dead_caster() {
        use crate::foundation::{
            ApplicationFacts, ConditionDefinition, ConditionValues, DotElement,
        };
        use crate::player_lethal::PlayerLethalVitals;
        let (mut r, mut states, issuer, mut fence, mut lane, mut world, clock) = setup();
        // Explicit local condition fixture, exercising the actual installed owner store/HP path.
        let definition = ConditionDefinition::new(
            "condition.test_composed_poison",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Poison,
                total_min: 40,
                total_max: 40,
                per_tick: 10,
                interval_ms: 1000,
                delayed: true,
            },
        )
        .expect("monster_combat_lane.rs:tests:3292: qualified fixture operation must succeed");
        let root = GameplayDecisionRoot::from_bytes(r.content_pin().server_artifact_digest());
        let facts = ApplicationFacts {
            now: 0,
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([7; 16]),
        };
        assert!(states.install_owned_source_condition_fixture(
            &r,
            world.target,
            world.session,
            issuer,
            &[definition],
            &facts
        ));
        states
            .apply_attack_damage(
                &mut r,
                world.target,
                world.session,
                1,
                "composed-condition-install",
                crate::foundation::owner_timer::OwnerClock::now(&clock),
            )
            .expect("monster_combat_lane.rs:tests:3321: qualified fixture operation must succeed");
        r.remove_test_actor(issuer)
            .expect("monster_combat_lane.rs:tests:3322: qualified fixture operation must succeed");
        clock.advance(1_000_000);
        world.missing = true;
        let refused = lane
            .run(
                &mut r,
                &mut states,
                &mut fence,
                &clock,
                &mut world,
                revisions(),
            )
            .expect("monster_combat_lane.rs:tests:3334: qualified fixture operation must succeed");
        assert!(refused.thinks.is_empty());
        assert!(refused.conditions.is_err());
        assert_eq!(
            observe_vitals(&r, &states, world.target, world.session)
                .expect(
                    "monster_combat_lane.rs:tests:3339: qualified fixture operation must succeed"
                )
                .1
                .health,
            99999
        );
        world.missing = false;
        let applied = lane
            .run(
                &mut r,
                &mut states,
                &mut fence,
                &clock,
                &mut world,
                revisions(),
            )
            .expect("monster_combat_lane.rs:tests:3354: qualified fixture operation must succeed");
        assert!(applied.thinks.is_empty());
        assert!(applied.melee.is_empty());
        // No trusted native map/registry was installed in this test fixture: the aggregate
        // must refuse publication. Exercise the canonical target owner separately with
        // independently explicit local TickFacts, retaining dead source provenance.
        assert!(
            applied.conditions.as_ref().expect(
                "monster_combat_lane.rs:tests:3360: qualified fixture operation must succeed"
            )[0]
            .1
            .is_none()
        );
        let tick = states
            .tick_source_player_conditions(
                &mut r,
                world.target,
                world.session,
                DecisionOccurrenceId::from_bytes([8; 16]),
                crate::ability::condition::TickFacts::default(),
                clock.now(),
            )
            .expect("monster_combat_lane.rs:tests:3370: qualified fixture operation must succeed");
        assert_eq!(tick.damage, 10);
        assert_eq!(
            observe_vitals(&r, &states, world.target, world.session)
                .expect(
                    "monster_combat_lane.rs:tests:3374: qualified fixture operation must succeed"
                )
                .1
                .health,
            99989
        );
        let revision = observe_vitals(&r, &states, world.target, world.session)
            .expect("monster_combat_lane.rs:tests:3380: qualified fixture operation must succeed")
            .0;
        lane.run(
            &mut r,
            &mut states,
            &mut fence,
            &clock,
            &mut world,
            revisions(),
        )
        .expect("monster_combat_lane.rs:tests:3390: qualified fixture operation must succeed");
        assert_eq!(
            observe_vitals(&r, &states, world.target, world.session)
                .expect(
                    "monster_combat_lane.rs:tests:3393: qualified fixture operation must succeed"
                )
                .0,
            revision
        );
    }
}
