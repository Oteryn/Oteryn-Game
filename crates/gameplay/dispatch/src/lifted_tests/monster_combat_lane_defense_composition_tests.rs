#[cfg(test)]
#[allow(clippy::expect_used)]
mod defense_composition_tests {
    use crate::ai_think::ThinkOccurrence;
    use crate::ai_think::profile_schedule::MonsterSummonFacts;
    use crate::content::ProjectReferenceRecord;
    use crate::content::ProjectV2AuthoringProfile;
    use crate::content::ProjectV2AuthoringProfileData as Data;
    use crate::content::ProjectV2BehaviorAuthoring;
    use crate::content::ProjectV2DefinitionRef as Ref;
    use crate::content::ProjectV2Family;
    use crate::content::*;
    use crate::creature_auto_attack::AttackFacts;
    use crate::creature_auto_attack::CurrentCombatFactsReader;
    use crate::creature_damage_spell::SpellCombatFacts;
    use crate::creature_damage_spell::SpellWorldReader;
    use crate::foundation::ChannelRuntimeV1;
    use crate::foundation::ExactActorRef;
    use crate::foundation::GameSessionId;
    use crate::foundation::RuntimeWorkStamp;
    use crate::foundation::TickFacts;
    use crate::foundation::owner_timer::OwnerClock;
    use crate::foundation::owner_timer::SemanticTimeMicros;
    use crate::foundation::owner_timer::VirtualOwnerClock;
    use crate::foundation::{
        ChannelContentPin, ChannelId, MovementLocalPosition, NodeId, RuntimeScopeRefV1,
        crystal_timer_fixture,
    };
    use crate::foundation::{ConditionValues, StatusKind};
    use crate::gameplay_transport::actor_spell::ChannelSpellStates;
    use crate::gameplay_transport::actor_spell::CreatureVision;
    use crate::monster_combat_lane::native_invisibility::InvisibleTileCombatPolicy;
    use crate::monster_combat_lane::native_invisibility::SelfSpeedSource;
    use crate::monster_combat_lane::*;
    use oteryn_gameplay_content::content::creature_attack_geometry::Facing;
    /// Explicit no-target test owner. Unknown attack/condition facts never grant admission.
    struct DefenseWorld;
    impl CurrentCombatFactsReader for DefenseWorld {
        fn read_attack(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: ExactActorRef,
            _: GameSessionId,
            _: RuntimeWorkStamp,
        ) -> Option<AttackFacts> {
            None
        }
    }
    impl SpellWorldReader for DefenseWorld {
        fn current_players(
            &mut self,
            _: &ChannelRuntimeV1,
            _: RuntimeWorkStamp,
        ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
            None
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
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: i32,
            _: i32,
            _: i16,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            None
        }
        fn combat(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: ExactActorRef,
            _: GameSessionId,
            _: Option<&str>,
            _: RuntimeWorkStamp,
        ) -> Option<SpellCombatFacts> {
            None
        }
    }

    impl InvisibleTileCombatPolicy for DefenseWorld {
        fn current_tile_allowed(&mut self, r: &ChannelRuntimeV1, c: ExactActorRef) -> Option<bool> {
            let pos = r.read_actor_position(c).ok()?;
            (pos.context() == r.pinned_movement_context()).then_some(true)
        }
    }
    impl MonsterCombatReader for DefenseWorld {
        // Resolve the actual carrier parent/child ledger under the same owner turn.
        // selected_player above proves no current target in this fixture, so no
        // target/path capability is manufactured and summon admission remains absent.
        fn summon_facts(
            &mut self,
            runtime: &ChannelRuntimeV1,
            _: RuntimeWorkStamp,
            occurrence: ThinkOccurrence,
            behavior: &ProjectV2BehaviorAuthoring,
        ) -> Option<MonsterSummonFacts> {
            let summons = behavior.summons.as_ref()?;
            let master = runtime.native_summon_role(occurrence.actor).ok()?;
            runtime.read_actor_position(occurrence.actor).ok()?;
            Some(MonsterSummonFacts {
                occurrence,
                is_summon: master.is_some(),
                target_with_path: None,
                total_count: runtime.native_summon_count(occurrence.actor, None),
                entry_counts: summons
                    .entries
                    .iter()
                    .map(
                        |entry| crate::ai_think::profile_schedule::ObservedSummonCount {
                            creature: entry.creature.clone(),
                            count: runtime
                                .native_summon_count(occurrence.actor, Some(&entry.creature.key)),
                        },
                    )
                    .collect(),
            })
        }
        fn selected_player(
            &mut self,
            r: &ChannelRuntimeV1,
            _: RuntimeWorkStamp,
            o: ThinkOccurrence,
            _: &ProjectV2BehaviorAuthoring,
        ) -> Option<Option<(ExactActorRef, GameSessionId)>> {
            r.contains_live_creature(o.actor).then_some(None)
        }
        fn condition_tick_facts(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: GameSessionId,
            _: RuntimeWorkStamp,
        ) -> Option<TickFacts> {
            None
        }
        fn self_condition_facts(
            &mut self,
            r: &ChannelRuntimeV1,
            a: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<SelfConditionFacts> {
            r.read_actor_position(a).ok()?;
            Some(SelfConditionFacts {
                base_speed: 220,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
            })
        }
    }
    #[test]
    #[ignore = "requires retained frozen eleven-document native project capture; run explicitly locally"]
    fn captured_native_defense_proposals_heal_actual_creature_and_invisible_expires_in_perception()
    {
        let retained_native_capture_path = std::env::var_os("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT")
            .filter(|value| !value.is_empty())
            .map(std::path::PathBuf::from)
            .expect("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT must explicitly name the current final native eleven-document capture");
        let path = retained_native_capture_path.as_path();
        let limits = ProjectEvidenceLimits {
            max_documents: 11,
            max_document_bytes: 96_000_000,
            max_total_bytes: 160_000_000,
            max_json_depth: 24,
            max_decoded_fields: 2_400_000,
            max_string_bytes: 43_000_000,
            max_locator_bytes: 160,
            max_locator_segments: 8,
            max_reference_records: 70000,
            max_import_records: 29,
            max_reimport_states: 404,
        };
        let project = capture_world_project(
            path.parent().expect("monster_combat_lane.rs:defense_composition_tests:3562: qualified fixture operation must succeed"),
            path.file_name().expect("monster_combat_lane.rs:defense_composition_tests:3563: qualified fixture operation must succeed"),
            ProjectFilesystemLimits {
                project: limits,
                max_entries_per_directory_scan: 32,
                max_total_directory_entries_scanned: 201,
            },
        )
        .expect("actual retained canonical project");
        let draft = project.migrate_to_v2();
        let source = project.lower_reference_source().expect("monster_combat_lane.rs:defense_composition_tests:3572: qualified fixture operation must succeed");
        let world = source.world_id;
        // Actual typed source membership is proven by capture+qualification above. The current
        // Item-only Reference artifact compiler cannot encode Creature closure; this independent
        // local carrier fixture pin is NOT claimed to be a compiled Creature artifact digest.
        let digest = [1; 32];
        let id = |tag: u8| {
            let mut b = [0; 16];
            b[6] = 0x70;
            b[8] = 0x80;
            b[15] = tag;
            b
        };
        // Frame/map are explicit local test position owners, not a Global map activation claim.
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            digest,
            [2; 32],
            [3; 32],
            [4; 32],
            (100, 100, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).expect("monster_combat_lane.rs:defense_composition_tests:3597: qualified fixture operation must succeed"),
            NodeId::decode(&id(3)).expect("monster_combat_lane.rs:defense_composition_tests:3598: qualified fixture operation must succeed"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            128,
            pin,
        )
        .expect("monster_combat_lane.rs:defense_composition_tests:3604: qualified fixture operation must succeed");
        let reference = |key: &str| Ref {
            family: ProjectV2Family::Creature,
            key: key.into(),
            revision: "definition-r1".into(),
        };
        let creature = reference("oteryn:creature.abyssador");
        let actor = runtime
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                &creature.key,
                100,
            )
            .expect("monster_combat_lane.rs:defense_composition_tests:3622: qualified fixture operation must succeed");
        let rat = runtime
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 104,
                    y: 100,
                    floor: 7,
                },
                "oteryn:creature.rat",
                20,
            )
            .expect("monster_combat_lane.rs:defense_composition_tests:3633: qualified fixture operation must succeed");
        let demon = runtime
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 105,
                    y: 100,
                    floor: 7,
                },
                "oteryn:creature.demon",
                8200,
            )
            .expect("monster_combat_lane.rs:defense_composition_tests:3644: qualified fixture operation must succeed");
        let b = runtime.binding();
        let (mut fence, _) = crystal_timer_fixture(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .expect("monster_combat_lane.rs:defense_composition_tests:3649: qualified fixture operation must succeed");
        let mut lane = MonsterCombatLane::new(&runtime, &fence).expect("monster_combat_lane.rs:defense_composition_tests:3652: qualified fixture operation must succeed");
        let first = lane
            .register_project(
                &runtime,
                &mut fence,
                actor,
                &creature,
                &draft,
                digest,
                SemanticTimeMicros::from_micros(0),
            )
            .expect("monster_combat_lane.rs:defense_composition_tests:3662: qualified fixture operation must succeed");
        assert_eq!(first.sequence, 0);
        assert!(!lane.actors[0].heals.is_empty());
        assert_eq!(lane.actors[0].invisible.len(), 1);
        let rat_vision = CreatureVision::from_native(
            &runtime,
            &reference("oteryn:creature.rat"),
            &draft.core.records,
            &draft.state.authoring_profiles,
            digest,
        )
        .expect("monster_combat_lane.rs:defense_composition_tests:3672: qualified fixture operation must succeed");
        let demon_vision = CreatureVision::from_native(
            &runtime,
            &reference("oteryn:creature.demon"),
            &draft.core.records,
            &draft.state.authoring_profiles,
            digest,
        )
        .expect("monster_combat_lane.rs:defense_composition_tests:3680: qualified fixture operation must succeed");
        let entry = &lane.actors[0].behavior.defenses[2];
        let ability = draft
            .state
            .authoring_profiles
            .iter()
            .find_map(|p| match &p.data {
                Data::Ability(a) if p.target == entry.ability => Some(a),
                _ => None,
            })
            .expect("monster_combat_lane.rs:defense_composition_tests:3691: qualified fixture operation must succeed");
        let inline = match &ability.details.as_ref().expect("monster_combat_lane.rs:defense_composition_tests:3693: qualified fixture operation must succeed").effects[0] { ProjectV2AbilityEffect::Inline(inline) => Some(inline), _ => None }.expect("native invisible source must have Inline effect");
        let definition =
            crate::creature_condition_content::lower_condition_definition(inline, 1, None).expect("monster_combat_lane.rs:defense_composition_tests:3698: qualified fixture operation must succeed");
        let duration_ms = match definition.values() {
            ConditionValues::TimedStatus {
                kind: StatusKind::Invisible,
                duration_ms,
            } => Some(duration_ms),
            _ => None,
        }
        .expect("source invisible must lower to exact native timed status");
        let mut states = ChannelSpellStates::default();
        let mut reader = DefenseWorld;
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        let revisions = crate::ability::RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r1",
            "sim-r1",
        )
        .expect("monster_combat_lane.rs:defense_composition_tests:3714: qualified fixture operation must succeed");
        let mut healed = false;
        let mut invisible_at = None;
        for _ in 0..256 {
            let pulse = lane
                .run(
                    &mut runtime,
                    &mut states,
                    &mut fence,
                    &clock,
                    &mut reader,
                    revisions.clone(),
                )
                .expect("monster_combat_lane.rs:defense_composition_tests:3728: qualified fixture operation must succeed");
            for think in pulse.thinks {
                assert!(think.schedule.is_ok());
                for (_, effect) in think.defenses {
                    let effect = effect.expect("monster_combat_lane.rs:defense_composition_tests:3733: qualified fixture operation must succeed");
                    assert!(
                        !matches!(&effect, DefenseOutcome::Appearance(_)),
                        "Demon fixture has no native appearance defense"
                    );
                    match effect {
                        DefenseOutcome::SelfHeal(h) => {
                            if h.health_after > h.health_before {
                                healed = true;
                            }
                        }
                        DefenseOutcome::Invisible(true) => {
                            invisible_at = Some(clock.now().get());
                        }
                        DefenseOutcome::Invisible(false) => {}
                        DefenseOutcome::Speed(_) => {}
                        DefenseOutcome::AreaHeal(_) => {}
                        DefenseOutcome::Icicle(_) => {}
                        DefenseOutcome::ThresholdHeal(_) => {}
                        DefenseOutcome::Presentation(_) => {}
                        DefenseOutcome::Appearance(_) => {}
                    }
                }
            }
            if healed && invisible_at.is_some() {
                break;
            }
            clock.advance(1_000_000);
        }
        assert!(
            healed,
            "real source self-heal proposal committed physical creature HP"
        );
        let applied = invisible_at.expect("native chance prepared and admitted invisibility");
        assert!(!(states
                .native_creature_visible(
                    &runtime,
                    rat,
                    &rat_vision,
                    actor,
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(applied)
                )
                .expect("monster_combat_lane.rs:defense_composition_tests:3772: qualified fixture operation must succeed")));
        assert!(states
                .native_creature_visible(
                    &runtime,
                    demon,
                    &demon_vision,
                    actor,
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(applied)
                )
                .expect("monster_combat_lane.rs:defense_composition_tests:3784: qualified fixture operation must succeed"));
        let expiry = applied + u64::from(duration_ms) * 1000;
        assert!(
            states
                .native_actor_invisible(
                    &runtime,
                    actor,
                    None,
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(expiry - 1)
                )
                .expect("monster_combat_lane.rs:defense_composition_tests:3796: qualified fixture operation must succeed")
        );
        assert!(
            !states
                .native_actor_invisible(
                    &runtime,
                    actor,
                    None,
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(expiry)
                )
                .expect("monster_combat_lane.rs:defense_composition_tests:3806: qualified fixture operation must succeed")
        );
        let retry = lane
            .run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions,
            )
            .expect("monster_combat_lane.rs:defense_composition_tests:3817: qualified fixture operation must succeed");
        assert!(retry.thinks.is_empty());
    }
    #[test]
    fn real_profile_defense_schedule_installs_native_creature_speed_once_without_player_base_default()
     {
        let packet:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/conditions/creature-speed/native-fixtures.json"))).expect("monster_combat_lane.rs:defense_composition_tests:3824: qualified fixture operation must succeed");
        let records: Vec<ProjectReferenceRecord> =
            serde_json::from_value(packet["records"].clone()).expect("monster_combat_lane.rs:defense_composition_tests:3826: qualified fixture operation must succeed");
        let profiles: Vec<ProjectV2AuthoringProfile> =
            serde_json::from_value(packet["authoring_profiles"].clone()).expect("monster_combat_lane.rs:defense_composition_tests:3828: qualified fixture operation must succeed");
        let (mut runtime, _, _) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(0x5a);
        let creature = Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.demon".into(),
            revision: "definition-r1".into(),
        };
        let actor = runtime
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 10,
                    y: 10,
                    floor: 7,
                },
                &creature.key,
                100,
            )
            .expect("monster_combat_lane.rs:defense_composition_tests:3845: qualified fixture operation must succeed");
        let binding = runtime.binding();
        let (mut fence, _) = crystal_timer_fixture(
            RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
            binding.scope_generation(),
        )
        .expect("monster_combat_lane.rs:defense_composition_tests:3850: qualified fixture operation must succeed");
        let mut lane = MonsterCombatLane::new(&runtime, &fence).expect("monster_combat_lane.rs:defense_composition_tests:3853: qualified fixture operation must succeed");
        let first = lane
            .register_native(
                &runtime,
                &mut fence,
                actor,
                &creature,
                &records,
                &profiles,
                [1; 32],
                SemanticTimeMicros::from_micros(0),
            )
            .expect("monster_combat_lane.rs:defense_composition_tests:3864: qualified fixture operation must succeed");
        assert_eq!(first.sequence, 0);
        assert_eq!(lane.actors[0].speed.len(), 1);
        let source =
            SelfSpeedSource::from_native(&runtime, &creature, 1, &records, &profiles, [1; 32])
                .expect("monster_combat_lane.rs:defense_composition_tests:3869: qualified fixture operation must succeed");
        assert_eq!(source.base_speed(), 128);
        let mut states = ChannelSpellStates::default();
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        let mut reader = DefenseWorld;
        // Existing reader intentionally supplies220 (player default). Aggregate speed must
        // use the exact current native Creature profile128, not silently copy that scalar.
        let revisions = crate::ability::RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r1",
            "sim-r1",
        )
        .expect("monster_combat_lane.rs:defense_composition_tests:3882: qualified fixture operation must succeed");
        let mut applied = None;
        for _ in 0..64 {
            let pulse = lane
                .run(
                    &mut runtime,
                    &mut states,
                    &mut fence,
                    &clock,
                    &mut reader,
                    revisions.clone(),
                )
                .expect("monster_combat_lane.rs:defense_composition_tests:3895: qualified fixture operation must succeed");
            for think in pulse.thinks {
                assert!(
                    think.schedule.is_ok(),
                    "exact native Demon schedule refused: {:?}",
                    think.schedule
                );
                for (ability, effect) in think.defenses {
                    if ability.key == "oteryn:ability.creature.demon.defense-2" && let DefenseOutcome::Speed(true) = effect.expect("monster_combat_lane.rs:defense_composition_tests:3905: qualified fixture operation must succeed") {
                            applied = Some(clock.now().get());
                        }
                }
            }
            if applied.is_some() {
                break;
            }
            clock.advance(1_000_000);
        }
        let time = applied.expect("actual existing chance schedule must eventually admit speed");
        let speed = crate::movement::speed::runtime_creature_speed(
            &runtime,
            actor,
            &source,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(time),
        )
        .expect("monster_combat_lane.rs:defense_composition_tests:3921: qualified fixture operation must succeed");
        assert!((98..=156).contains(&speed));
        let retry = lane
            .run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions,
            )
            .expect("monster_combat_lane.rs:defense_composition_tests:3933: qualified fixture operation must succeed");
        assert!(retry.thinks.is_empty());
        assert_eq!(
            crate::movement::speed::runtime_creature_speed(
                &runtime,
                actor,
                &source,
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(time)
            ),
            Some(speed)
        );
        assert_eq!(runtime.read_actor_position(actor).expect("monster_combat_lane.rs:defense_composition_tests:3945: qualified fixture operation must succeed").position().x, 10);
    }
}
