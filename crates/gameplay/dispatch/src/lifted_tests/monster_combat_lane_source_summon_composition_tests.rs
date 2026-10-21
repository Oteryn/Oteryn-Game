#[cfg(test)]
#[allow(clippy::expect_used)]
mod source_summon_composition_tests {
    use crate::ai_think::ThinkOccurrence;
    use crate::content::ProjectV2AuthoringProfileData as Data;
    use crate::content::ProjectV2BehaviorAuthoring;
    use crate::content::ProjectV2DefinitionRef as Ref;
    use crate::content::ProjectV2Family;
    use crate::content::*;
    use crate::creature_attack_geometry::Facing;
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
    use crate::gameplay_transport::actor_spell::ChannelSpellStates;
    use crate::monster_combat_lane::native_invisibility::InvisibleTileCombatPolicy;
    use crate::monster_combat_lane::*;
    struct World {
        target: ExactActorRef,
        session: GameSessionId,
    }
    impl CurrentCombatFactsReader for World {
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
    impl SpellWorldReader for World {
        fn current_players(
            &mut self,
            r: &ChannelRuntimeV1,
            _: RuntimeWorkStamp,
        ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
            r.player_control_facts(self.target, self.session).ok()?;
            Some(vec![(self.target, self.session)])
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

    impl InvisibleTileCombatPolicy for World {
        fn current_tile_allowed(&mut self, r: &ChannelRuntimeV1, c: ExactActorRef) -> Option<bool> {
            r.read_actor_position(c).ok()?;
            Some(true)
        }
    }
    impl MonsterCombatReader for World {
        fn selected_player(
            &mut self,
            r: &ChannelRuntimeV1,
            _: RuntimeWorkStamp,
            _: ThinkOccurrence,
            _: &ProjectV2BehaviorAuthoring,
        ) -> Option<Option<(ExactActorRef, GameSessionId)>> {
            r.player_control_facts(self.target, self.session).ok()?;
            Some(Some((self.target, self.session)))
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
        fn summon_target_reachable(
            &mut self,
            r: &ChannelRuntimeV1,
            p: ExactActorRef,
            t: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            r.read_actor_position(p).ok()?;
            r.read_actor_position(t).ok()?;
            Some(t == self.target)
        }
        fn summon_cell_admits(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: &Ref,
            p: crate::foundation::MovementLocalPosition,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            Some((99..=101).contains(&p.x) && (99..=101).contains(&p.y) && p.floor == 7)
        }
        fn summon_inherited_target_eligible(
            &mut self,
            r: &ChannelRuntimeV1,
            c: ExactActorRef,
            t: ExactActorRef,
            s: GameSessionId,
            _: RuntimeWorkStamp,
            _: &ProjectV2BehaviorAuthoring,
        ) -> Option<bool> {
            r.player_control_facts(t, s).ok()?;
            r.read_actor_position(c).ok()?;
            Some(t == self.target && s == self.session)
        }
    }
    #[test]
    #[ignore = "requires retained native11document capture; actual Defense schedule->spawn->native registration"]
    fn captured_defense_summon_registers_two_source_children_and_exact_live_health() {
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
            path.parent().expect("monster_combat_lane.rs:source_summon_composition_tests:4527: qualified fixture operation must succeed"),
            path.file_name().expect("monster_combat_lane.rs:source_summon_composition_tests:4528: qualified fixture operation must succeed"),
            ProjectFilesystemLimits {
                project: limits,
                max_entries_per_directory_scan: 32,
                max_total_directory_entries_scanned: 201,
            },
        )
        .expect("monster_combat_lane.rs:source_summon_composition_tests:4533: qualified fixture operation must succeed");
        let draft = project.migrate_to_v2();
        let (mut runtime, mut fence, old, _, _, _) =
            crate::monster_summon::summon_execution_fixture_for_world(
                project.lower_reference_source().expect("monster_combat_lane.rs:source_summon_composition_tests:4539: qualified fixture operation must succeed").world_id,
            );
        runtime.remove_test_actor(old).expect("monster_combat_lane.rs:source_summon_composition_tests:4541: qualified fixture operation must succeed");
        let creature = Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.white_pale".into(),
            revision: "definition-r1".into(),
        };
        let expected_child = Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.carrion_worm".into(),
            revision: "definition-r1".into(),
        };
        let health = |key: &Ref| {
            match &draft
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target == *key)
            .expect("monster_combat_lane.rs:source_summon_composition_tests:4556: qualified fixture operation must succeed")
            .data { Data::Creature(c) => Some(i64::try_from(c.health.expect("monster_combat_lane.rs:source_summon_composition_tests:4560: qualified fixture operation must succeed")).expect("monster_combat_lane.rs:source_summon_composition_tests:4560: qualified fixture operation must succeed")), _ => None }.expect("actual child source must be Creature profile")
        };
        let parent = runtime
            .admit_source_pinned_lab_creature(
                crate::foundation::MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                &creature.key,
                health(&creature),
            )
            .expect("monster_combat_lane.rs:source_summon_composition_tests:4572: qualified fixture operation must succeed");
        let mut id = [0; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id[15] = 0x66;
        let session = GameSessionId::decode(&id).expect("monster_combat_lane.rs:source_summon_composition_tests:4578: qualified fixture operation must succeed");
        let reservation = runtime.reserve_fresh_session(session).expect("monster_combat_lane.rs:source_summon_composition_tests:4579: qualified fixture operation must succeed");
        let target = runtime.commit_fresh_session(reservation).expect("monster_combat_lane.rs:source_summon_composition_tests:4580: qualified fixture operation must succeed");
        runtime.initialize_first_entry_position(target).expect("monster_combat_lane.rs:source_summon_composition_tests:4581: qualified fixture operation must succeed");
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                target,
                session,
                crate::gameplay_transport::actor_spell::tests::FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .expect("monster_combat_lane.rs:source_summon_composition_tests:4591: qualified fixture operation must succeed");
        let mut reader = World { target, session };
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        let mut lane = MonsterCombatLane::new(&runtime, &fence).expect("monster_combat_lane.rs:source_summon_composition_tests:4595: qualified fixture operation must succeed");
        lane.register_project(
            &runtime,
            &mut fence,
            parent,
            &creature,
            &draft,
            [1; 32],
            clock.now(),
        )
        .expect("monster_combat_lane.rs:source_summon_composition_tests:4603: qualified fixture operation must succeed");
        // Source-qualified sixteen callback cases add fourteen unique child definitions to the captured ordinary/inline set.
        assert_eq!(lane.child_closure.as_ref().expect("monster_combat_lane.rs:source_summon_composition_tests:4607: qualified fixture operation must succeed").children.len(), 178);
        assert_eq!(
            lane.actors
                .iter()
                .find(|a| a.actor == parent)
                .expect("monster_combat_lane.rs:source_summon_composition_tests:4611: qualified fixture operation must succeed")
                .defense_summons
                .len(),
            1
        );
        let revisions = crate::ability::RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r1",
            "sim-r1",
        )
        .expect("monster_combat_lane.rs:source_summon_composition_tests:4622: qualified fixture operation must succeed");
        let mut children = None;
        for _ in 0..1000 {
            let pulse = lane
                .run(
                    &mut runtime,
                    &mut states,
                    &mut fence,
                    &clock,
                    &mut reader,
                    revisions.clone(),
                )
                .expect("monster_combat_lane.rs:source_summon_composition_tests:4635: qualified fixture operation must succeed");
            for thought in &pulse.thinks {
                for (_, outcome) in &thought.defense_summons {
                    if let Ok(batch) = outcome
                        && !batch.children.is_empty()
                    {
                        assert_eq!(batch.requested, 2);
                        assert_eq!(batch.children.len(), 2);
                        assert_eq!(batch.cap_omitted, 0);
                        assert_eq!(batch.refused, None);
                        let refs = batch.children.iter().map(|c| c.child).collect::<Vec<_>>();
                        for child in &refs {
                            let admission = pulse
                                .child_registration
                                .iter()
                                .find(|entry| entry.actor == *child)
                                .expect("each committed child retains registration result");
                            assert!(
                                matches!(&admission.result,Ok(Some(think))if think.actor==*child&&think.sequence==0)
                            );
                            assert_eq!(runtime.native_summon_role(*child), Ok(Some(parent)));
                            assert!(runtime.matches_live_creature_identity(
                                *child,
                                expected_child.key.as_bytes()
                            ));
                            assert_eq!(
                                runtime.crystal_router_fixture_health(*child),
                                health(&expected_child)
                            );
                            assert!(
                                lane.actors
                                    .iter()
                                    .any(|a| a.actor == *child && a.creature == expected_child)
                            );
                        }
                        children = Some(refs);
                    }
                }
            }
            if children.is_some() {
                break;
            }
            clock.advance(1_000_000);
        }
        let children = children.expect("real12% source defense chance produces native children");
        let count = runtime.native_summon_count(parent, None);
        assert_eq!(count, 2);
        let repeat = lane
            .run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions.clone(),
            )
            .expect("monster_combat_lane.rs:source_summon_composition_tests:4691: qualified fixture operation must succeed");
        assert!(
            repeat
                .thinks
                .iter()
                .any(|t| children.contains(&t.occurrence.actor) && t.occurrence.sequence == 0)
        );
        assert_eq!(runtime.native_summon_count(parent, None), count);
        assert!(repeat.child_registration.is_empty());
        fence
            .apply_external_grant(crate::foundation::ScopeOwnershipGeneration::new(2).expect("monster_combat_lane.rs:source_summon_composition_tests:4702: qualified fixture operation must succeed"))
            .expect("monster_combat_lane.rs:source_summon_composition_tests:4702: qualified fixture operation must succeed");
        assert!(
            lane.run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions
            )
            .is_err()
        );
        assert_eq!(runtime.native_summon_count(parent, None), 2);
    }

    #[test]
    #[ignore = "requires retained exact native project source capture; run locally explicitly"]
    fn captured_source_summon_creates_current_child_schedule_and_inherits_actual_parent_target() {
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
            path.parent().expect("monster_combat_lane.rs:source_summon_composition_tests:4739: qualified fixture operation must succeed"),
            path.file_name().expect("monster_combat_lane.rs:source_summon_composition_tests:4740: qualified fixture operation must succeed"),
            ProjectFilesystemLimits {
                project: limits,
                max_entries_per_directory_scan: 32,
                max_total_directory_entries_scanned: 201,
            },
        )
        .expect("monster_combat_lane.rs:source_summon_composition_tests:4745: qualified fixture operation must succeed");
        let draft = project.migrate_to_v2();
        let (mut runtime, mut fence, parent, _, _, _) =
            crate::monster_summon::summon_execution_fixture_for_world(
                project.lower_reference_source().expect("monster_combat_lane.rs:source_summon_composition_tests:4752: qualified fixture operation must succeed").world_id,
            );
        let mut id = [0; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id[15] = 0x65;
        let session = GameSessionId::decode(&id).expect("monster_combat_lane.rs:source_summon_composition_tests:4758: qualified fixture operation must succeed");
        let reservation = runtime.reserve_fresh_session(session).expect("monster_combat_lane.rs:source_summon_composition_tests:4759: qualified fixture operation must succeed");
        let target = runtime.commit_fresh_session(reservation).expect("monster_combat_lane.rs:source_summon_composition_tests:4760: qualified fixture operation must succeed");
        runtime.initialize_first_entry_position(target).expect("monster_combat_lane.rs:source_summon_composition_tests:4761: qualified fixture operation must succeed");
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                target,
                session,
                crate::gameplay_transport::actor_spell::tests::FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .expect("monster_combat_lane.rs:source_summon_composition_tests:4770: qualified fixture operation must succeed");
        let mut reader = World { target, session };
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        let mut lane = MonsterCombatLane::new(&runtime, &fence).expect("monster_combat_lane.rs:source_summon_composition_tests:4775: qualified fixture operation must succeed");
        let creature = Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.orc_shaman".into(),
            revision: "definition-r1".into(),
        };
        lane.register_project(
            &runtime,
            &mut fence,
            parent,
            &creature,
            &draft,
            [1; 32],
            clock.now(),
        )
        .expect("monster_combat_lane.rs:source_summon_composition_tests:4788: qualified fixture operation must succeed");
        // Source-qualified sixteen callback cases add fourteen unique child definitions to the captured ordinary/inline set.
        assert_eq!(lane.child_closure.as_ref().expect("monster_combat_lane.rs:source_summon_composition_tests:4792: qualified fixture operation must succeed").children.len(), 178);
        let revisions = crate::ability::RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r1",
            "sim-r1",
        )
        .expect("monster_combat_lane.rs:source_summon_composition_tests:4798: qualified fixture operation must succeed");
        let mut child = None;
        for _ in 0..128 {
            let pulse = lane
                .run(
                    &mut runtime,
                    &mut states,
                    &mut fence,
                    &clock,
                    &mut reader,
                    revisions.clone(),
                )
                .expect("monster_combat_lane.rs:source_summon_composition_tests:4810: qualified fixture operation must succeed");
            for thought in pulse.thinks {
                for (_, outcome) in thought.summons {
                    if let Ok(receipt) = outcome
                        && receipt.newly_created
                    {
                        child = Some(receipt.child);
                    }
                }
            }
            if child.is_some() {
                break;
            }
            clock.advance(1_000_000);
        }
        let child = child.expect("source20% draws produce real child");
        assert_eq!(runtime.native_summon_role(child), Ok(Some(parent)));
        assert!(lane.actors.iter().any(|a| a.actor == child));
        let pin = lane.child_closure.as_ref().expect("monster_combat_lane.rs:source_summon_composition_tests:4830: qualified fixture operation must succeed").clone();
        let pulse = lane
            .run(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                &mut reader,
                revisions,
            )
            .expect("monster_combat_lane.rs:source_summon_composition_tests:4838: qualified fixture operation must succeed");
        assert!(
            pulse
                .thinks
                .iter()
                .any(|t| t.occurrence.actor == child && t.occurrence.sequence == 0)
        );
        assert_eq!(
            lane.actors
                .iter()
                .find(|a| a.actor == child)
                .expect("monster_combat_lane.rs:source_summon_composition_tests:4850: qualified fixture operation must succeed")
                .selected_target,
            Some((target, session))
        );
        assert!(std::sync::Arc::ptr_eq(
            &pin,
            lane.child_closure.as_ref().expect("monster_combat_lane.rs:source_summon_composition_tests:4857: qualified fixture operation must succeed")
        ));
    }
}
