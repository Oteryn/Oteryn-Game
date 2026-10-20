#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use crate::ai_think::profile_schedule::ProfileAbilityProposal;
    use crate::ai_think::profile_schedule::ScheduleList;
    use crate::content::ProjectReferenceRecord as Record;
    use crate::content::ProjectV2DefinitionRef as Ref;
    use crate::content::ProjectV2Draft;
    use crate::content::ProjectV2Family as Family;
    use crate::creature_auto_attack::AttackError;
    use crate::creature_auto_attack::AttackFacts;
    use crate::creature_damage_spell::SpellCombatFacts;
    use crate::creature_damage_spell::SpellWorldReader;
    use crate::foundation::ChannelRuntimeV1;
    use crate::foundation::ExactActorRef;
    use crate::foundation::RuntimeScopeRefV1;
    use crate::foundation::RuntimeWorkStamp;
    use crate::foundation::ScopeRuntimeFence;
    use crate::foundation::owner_timer::OwnerClock;
    use crate::foundation::owner_timer::SemanticTimeMicros;
    use crate::foundation::owner_timer::VirtualOwnerClock;
    use crate::foundation::{GameSessionId, MovementLocalPosition};
    use crate::gameplay_transport::actor_spell::{
        ChannelSpellStates,
        tests::{FACTS, runtime_with_player},
    };
    use crate::smelly_encounter_dispatch::*;
    fn setup() -> (
        ChannelRuntimeV1,
        ExactActorRef,
        GameSessionId,
        ExactActorRef,
        ChannelSpellStates,
        ProjectV2Draft,
        SourceSmelly,
    ) {
        let (mut r, p, session) = runtime_with_player(0x75);
        r.initialize_movement_test_position(
            p,
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
        )
        .expect("smelly_encounter_dispatch.rs:tests:485: qualified fixture operation must succeed");
        let c = r
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                CREATURE,
                5000,
            )
            .expect(
                "smelly_encounter_dispatch.rs:tests:496: qualified fixture operation must succeed",
            );
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &r,
                p,
                session,
                crate::spell::cast::CharacterCastFacts {
                    max_health: 2000,
                    ..FACTS
                },
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .expect(
                "smelly_encounter_dispatch.rs:tests:510: qualified fixture operation must succeed",
            );
        let v: serde_json::Value = serde_json::from_str(include_str!(
            "../smelly_encounter_native_fixture.json"
        ))
        .expect("smelly_encounter_dispatch.rs:tests:512: qualified fixture operation must succeed");
        let d = ProjectV2Draft {
            core: crate::content::ProjectDraft {
                project_revision: "g4-npc-wave-a-r9".into(),
                package_key: "oteryn:content.world-project".into(),
                semantic_schema_version: "reference-schema-v1".into(),
                licensing_metadata: "PENDING".into(),
                world_id: hex(*r.binding().world_id().as_bytes()),
                coordinate_frame: "global-target-2026-09-27".into(),
                records: serde_json::from_value(v["records"].clone()).expect("smelly_encounter_dispatch.rs:tests:521: qualified fixture operation must succeed"),
                imports: vec![],
                metadata: vec![],
            },
            state: crate::content::ProjectV2State {
                declarations: serde_json::from_value(v["declarations"].clone()).expect("smelly_encounter_dispatch.rs:tests:526: qualified fixture operation must succeed"),
                authoring_profiles: serde_json::from_value(v["authoring_profiles"].clone())
                    .expect("smelly_encounter_dispatch.rs:tests:528: qualified fixture operation must succeed"),
                source_identity_bindings: serde_json::from_value(
                    v["source_identity_bindings"].clone(),
                )
                .expect("smelly_encounter_dispatch.rs:tests:532: qualified fixture operation must succeed"),
                sources: serde_json::from_value(v["sources"].clone()).expect("smelly_encounter_dispatch.rs:tests:533: qualified fixture operation must succeed"),
                ..Default::default()
            },
        };
        let s = SourceSmelly::qualify(
            &d,
            &Ref {
                family: Family::Creature,
                key: CREATURE.into(),
                revision: "definition-r1".into(),
            },
            r.content_pin().server_artifact_digest(),
        )
        .expect("smelly_encounter_dispatch.rs:tests:546: qualified fixture operation must succeed");
        (r, p, session, c, states, d, s)
    }
    fn proposal(c: ExactActorRef, s: &SourceSmelly, seq: u64) -> ProfileAbilityProposal {
        let atom = format!("actor:{}", hex(c.placement_identity()));
        let revisions = crate::ability::RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r1",
            "sim-r1",
        )
        .expect("smelly_encounter_dispatch.rs:tests:558: qualified fixture operation must succeed");
        ProfileAbilityProposal {
            issuer: c,
            target: c,
            ability: s.ability.clone(),
            list: ScheduleList::Attack,
            entry_index: 0,
            magnitude: None,
            range_tiles: 0,
            occurrence: crate::ability::AbilityOccurrence::new(
                &format!("ai-profile:{}:{seq}:attack:0", hex(c.placement_identity())),
                revisions,
            )
            .expect(
                "smelly_encounter_dispatch.rs:tests:571: qualified fixture operation must succeed",
            ),
            intent: crate::ability::AiAbilityAdapter::normalize(&atom, &[&atom]).expect(
                "smelly_encounter_dispatch.rs:tests:572: qualified fixture operation must succeed",
            ),
        }
    }
    struct Reader {
        p: ExactActorRef,
        session: GameSessionId,
        available: bool,
        pz: bool,
        multiplier: u32,
    }
    impl SpellWorldReader for Reader {
        fn current_players(
            &mut self,
            _: &ChannelRuntimeV1,
            _: RuntimeWorkStamp,
        ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
            Some(vec![(self.p, self.session)])
        }
        fn current_facing(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: RuntimeWorkStamp,
        ) -> Option<crate::creature_attack_geometry::Facing> {
            Some(crate::creature_attack_geometry::Facing::North)
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
            self.available.then_some(true)
        }
        fn combat(
            &mut self,
            _: &ChannelRuntimeV1,
            c: ExactActorRef,
            p: ExactActorRef,
            session: GameSessionId,
            element: Option<&str>,
            _: RuntimeWorkStamp,
        ) -> Option<SpellCombatFacts> {
            assert_eq!(element, Some("earth"));
            self.available.then_some(SpellCombatFacts {
                attack: AttackFacts {
                    issuer: c,
                    target: p,
                    session,
                    revision: 1,
                    visible: true,
                    issuer_pz: false,
                    target_pz: self.pz,
                    issuer_protected: false,
                    target_protected: false,
                    defense: 0,
                    armor: 0,
                },
                multiplier_ppm: self.multiplier,
                immune: false,
                condition_policy: None,
            })
        }
    }
    fn fence(r: &ChannelRuntimeV1) -> (ScopeRuntimeFence, RuntimeWorkStamp) {
        let b = r.binding();
        crate::foundation::crystal_timer_fixture(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .expect("smelly_encounter_dispatch.rs:tests:645: qualified fixture operation must succeed")
    }
    #[test]
    fn actual_source_zero_occurrence_damage500_earth_policy_and_presentation700() {
        let (mut r, p, session, c, mut states, _, s) = setup();
        let (f, stamp) = fence(&r);
        let mut o = SmellyEncounterOwner::default();
        let proposal = proposal(c, &s, 0);
        let mut reader = Reader {
            p,
            session,
            available: true,
            pz: false,
            multiplier: 500000,
        };
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        assert!(
            o.schedule(&r, &f, stamp, &s, &proposal, clock.now())
                .expect("smelly_encounter_dispatch.rs:tests:663: qualified fixture operation must succeed")
        );
        assert!(
            !o.schedule(&r, &f, stamp, &s, &proposal, clock.now())
                .expect("smelly_encounter_dispatch.rs:tests:667: qualified fixture operation must succeed")
        );
        assert!(
            o.drain(&mut r, &f, stamp, &mut states, &mut reader, &clock)
                .expect("smelly_encounter_dispatch.rs:tests:671: qualified fixture operation must succeed")
                .is_empty()
        );
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(500000));
        let hits = o
            .drain(&mut r, &f, stamp, &mut states, &mut reader, &clock)
            .expect(
                "smelly_encounter_dispatch.rs:tests:677: qualified fixture operation must succeed",
            );
        let damage = match hits[0].1.as_ref().expect(
            "smelly_encounter_dispatch.rs:tests:678: qualified fixture operation must succeed",
        ) {
            SmellyEffect::Damage(damage) => Some(damage),
            _ => None,
        }
        .expect("source500ms pulse must be Damage");
        assert_eq!(damage.len(), 1);
        let receipt = damage[0].1.as_ref().expect(
            "smelly_encounter_dispatch.rs:tests:682: qualified fixture operation must succeed",
        );
        assert!((200..=400).contains(&receipt.applied));
        assert!(
            !o.schedule(&r, &f, stamp, &s, &proposal, clock.now())
                .expect("smelly_encounter_dispatch.rs:tests:686: qualified fixture operation must succeed")
        );
        assert!(
            o.drain(&mut r, &f, stamp, &mut states, &mut reader, &clock)
                .expect("smelly_encounter_dispatch.rs:tests:690: qualified fixture operation must succeed")
                .is_empty()
        );
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(700000));
        let hits = o
            .drain(&mut r, &f, stamp, &mut states, &mut reader, &clock)
            .expect(
                "smelly_encounter_dispatch.rs:tests:696: qualified fixture operation must succeed",
            );
        assert!(matches!(&hits[0].1,Ok(SmellyEffect::Presentation(event))if event.tiles.len()==9));
    }
    #[test]
    fn source_substitution_missing_map_pz_dead_caster_refuse_actual_hp() {
        let (mut r, p, session, c, mut states, mut d, s) = setup();
        let (f, stamp) = fence(&r);
        let mut o = SmellyEncounterOwner::default();
        let mut reader = Reader {
            p,
            session,
            available: false,
            pz: false,
            multiplier: 1000000,
        };
        let now = SemanticTimeMicros::from_micros(0);
        let before =
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session);
        o.schedule(&r, &f, stamp, &s, &proposal(c, &s, 0), now)
            .expect(
                "smelly_encounter_dispatch.rs:tests:715: qualified fixture operation must succeed",
            );
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(500000));
        let hits = o
            .drain(&mut r, &f, stamp, &mut states, &mut reader, &clock)
            .expect(
                "smelly_encounter_dispatch.rs:tests:719: qualified fixture operation must succeed",
            );
        assert!(matches!(hits[0].1, Err(AttackError::MissingCombatFacts)));
        d.state.source_identity_bindings[0].source_revision = "wrong-pin".into();
        assert!(
            SourceSmelly::qualify(
                &d,
                &Ref {
                    family: Family::Creature,
                    key: CREATURE.into(),
                    revision: "definition-r1".into()
                },
                s.content
            )
            .is_err()
        );
        reader.available = true;
        reader.pz = true;
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(700000));
        o.drain(&mut r, &f, stamp, &mut states, &mut reader, &clock)
            .expect(
                "smelly_encounter_dispatch.rs:tests:738: qualified fixture operation must succeed",
            );
        o.schedule(
            &r,
            &f,
            stamp,
            &s,
            &proposal(c, &s, 1),
            SemanticTimeMicros::from_micros(2000000),
        )
        .expect("smelly_encounter_dispatch.rs:tests:747: qualified fixture operation must succeed");
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(2500000));
        let hits = o
            .drain(&mut r, &f, stamp, &mut states, &mut reader, &clock)
            .expect(
                "smelly_encounter_dispatch.rs:tests:751: qualified fixture operation must succeed",
            );
        assert!(
            matches!(&hits.iter().find(|(_,v)|matches!(v,Ok(SmellyEffect::Damage(_)))).expect("smelly_encounter_dispatch.rs:tests:753: qualified fixture operation must succeed").1,Ok(SmellyEffect::Damage(v))if v.is_empty())
        );
        assert_eq!(
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session),
            before
        );
        r.remove_test_actor(c).expect(
            "smelly_encounter_dispatch.rs:tests:759: qualified fixture operation must succeed",
        );
        assert!(
            o.schedule(
                &r,
                &f,
                stamp,
                &s,
                &proposal(c, &s, 2),
                SemanticTimeMicros::from_micros(4000000)
            )
            .is_err()
        );
    }
    #[test]
    fn smelly_duplicate_source_identity_and_profiles_refuse_qualification() {
        let (r, _, _, _, _, d, s) = setup();
        let caster = Ref {
            family: Family::Creature,
            key: CREATURE.into(),
            revision: "definition-r1".into(),
        };
        let mut duplicate = d.clone();
        let b = duplicate
            .state
            .source_identity_bindings
            .iter()
            .find(|b| b.target.key == CREATURE)
            .expect(
                "smelly_encounter_dispatch.rs:tests:786: qualified fixture operation must succeed",
            )
            .clone();
        duplicate.state.source_identity_bindings.push(b);
        assert!(matches!(
            SourceSmelly::qualify(&duplicate, &caster, s.content),
            Err(AttackError::InvalidSource)
        ));
        let mut duplicate = d.clone();
        let p = duplicate
            .state
            .authoring_profiles
            .iter()
            .find(|p| p.target.key == PARENT)
            .expect(
                "smelly_encounter_dispatch.rs:tests:799: qualified fixture operation must succeed",
            )
            .clone();
        duplicate.state.authoring_profiles.push(p);
        assert!(matches!(
            SourceSmelly::qualify(&duplicate, &caster, s.content),
            Err(AttackError::InvalidSource)
        ));
        let mut duplicate = d.clone();
        let record = duplicate
            .core
            .records
            .iter()
            .find(|r| matches!(r,Record::Ability{identity,..}if identity.key==PARENT))
            .expect(
                "smelly_encounter_dispatch.rs:tests:812: qualified fixture operation must succeed",
            )
            .clone();
        duplicate.core.records.push(record);
        assert!(matches!(
            SourceSmelly::qualify(&duplicate, &caster, s.content),
            Err(AttackError::InvalidSource)
        ));
        for family in [Family::Creature, Family::Encounter] {
            let mut forged = d.clone();
            let b = forged
                .state
                .source_identity_bindings
                .iter_mut()
                .find(|b| b.target.family == family)
                .expect("smelly_encounter_dispatch.rs:tests:826: qualified fixture operation must succeed");
            b.identity_namespace = "crystalserver/forged".into();
            assert!(matches!(
                SourceSmelly::qualify(&forged, &caster, s.content),
                Err(AttackError::InvalidSource)
            ));
            let mut forged = d.clone();
            let b = forged
                .state
                .source_identity_bindings
                .iter_mut()
                .find(|b| b.target.family == family)
                .expect("smelly_encounter_dispatch.rs:tests:838: qualified fixture operation must succeed");
            b.external_id = "foreign-source-file-or-encounter".into();
            assert!(matches!(
                SourceSmelly::qualify(&forged, &caster, s.content),
                Err(AttackError::InvalidSource)
            ));
        }
        assert!(
            SourceSmelly::qualify(&d, &caster, r.content_pin().server_artifact_digest()).is_ok()
        );
    }
    #[test]
    fn smelly_foreign_owner_and_forged_proposal_do_not_mutate_or_consume_real_cast() {
        let (mut r, p, session, c, mut states, _, s) = setup();
        let (f, stamp) = fence(&r);
        let b = r.binding();
        let mut world = *b.world_id().as_bytes();
        world[15] ^= 1;
        let (foreign, _) = crate::foundation::crystal_timer_fixture(
            RuntimeScopeRefV1::channel(
                crate::foundation::WorldId::decode(&world).expect("smelly_encounter_dispatch.rs:tests:858: qualified fixture operation must succeed"),
                b.channel_id(),
            ),
            b.scope_generation(),
        )
        .expect("smelly_encounter_dispatch.rs:tests:863: qualified fixture operation must succeed");
        let mut o = SmellyEncounterOwner::default();
        let now = SemanticTimeMicros::from_micros(0);
        assert!(matches!(
            o.schedule(&r, &foreign, stamp, &s, &proposal(c, &s, 0), now),
            Err(AttackError::StaleOwner)
        ));
        assert!(o.entries.is_empty());
        let mut forged = proposal(c, &s, 0);
        forged.target = p;
        assert!(matches!(
            o.schedule(&r, &f, stamp, &s, &forged, now),
            Err(AttackError::InvalidPlan)
        ));
        assert!(o.entries.is_empty());
        let mut forged = proposal(c, &s, 0);
        let other = format!("actor:{}", hex(p.placement_identity()));
        forged.intent = crate::ability::AiAbilityAdapter::normalize(&other, &[&other]).expect(
            "smelly_encounter_dispatch.rs:tests:880: qualified fixture operation must succeed",
        );
        assert!(matches!(
            o.schedule(&r, &f, stamp, &s, &forged, now),
            Err(AttackError::InvalidPlan)
        ));
        assert!(o.entries.is_empty());
        let mut forged = proposal(c, &s, 0);
        let atom = format!("actor:{}", hex(c.placement_identity()));
        forged.intent = crate::ability::AiAbilityAdapter::normalize(&atom, &[&other]).expect(
            "smelly_encounter_dispatch.rs:tests:888: qualified fixture operation must succeed",
        );
        assert!(matches!(
            o.schedule(&r, &f, stamp, &s, &forged, now),
            Err(AttackError::InvalidPlan)
        ));
        assert!(o.entries.is_empty());
        assert!(
            o.schedule(&r, &f, stamp, &s, &proposal(c, &s, 0), now)
                .expect("smelly_encounter_dispatch.rs:tests:896: qualified fixture operation must succeed")
        );
        assert_eq!(o.entries.len(), 1);
        let before =
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session);
        let mut reader = Reader {
            p,
            session,
            available: true,
            pz: false,
            multiplier: 1000000,
        };
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(500000));
        let hits = o
            .drain(&mut r, &f, stamp, &mut states, &mut reader, &clock)
            .expect(
                "smelly_encounter_dispatch.rs:tests:911: qualified fixture operation must succeed",
            );
        assert!(matches!(&hits[0].1,Ok(SmellyEffect::Damage(v))if v.len()==1&&v[0].1.is_ok()));
        assert_ne!(
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session),
            before
        );
        assert!(
            !o.schedule(&r, &f, stamp, &s, &proposal(c, &s, 0), now)
                .expect("smelly_encounter_dispatch.rs:tests:919: qualified fixture operation must succeed")
        );
    }
}
