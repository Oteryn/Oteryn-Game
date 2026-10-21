#![allow(clippy::expect_used)]
use crate::content::{ProjectV2DefinitionRef as Ref, ProjectV2Draft, ProjectV2Family};
use crate::creature_auto_attack::ProfileAbilityProposal;
use crate::creature_auto_attack::{AttackError, AttackFacts};
use crate::creature_damage_spell::hex;
use crate::creature_damage_spell::*;
use crate::durability::creature_schedule::ScheduleList;
use crate::foundation::ScopeOwnershipGeneration;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::owner_timer::{OwnerClock, VirtualOwnerClock};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, RuntimeScopeRefV1, RuntimeWorkStamp, ScopeRuntimeFence,
};
use crate::foundation::{GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::actor_spell::{
    ChannelSpellStates,
    tests::{FACTS, runtime_with_player},
};
use crate::source_callback_cast::*;

fn draft(r: &ChannelRuntimeV1) -> ProjectV2Draft {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../apps/game-server/src/source_callback_native_fixture.json"
    ))
    .expect("qualified fixture");
    ProjectV2Draft {
        core: crate::content::ProjectDraft {
            project_revision: "g4-npc-wave-a-r9".into(),
            package_key: "oteryn:content.world-project".into(),
            semantic_schema_version: "reference-schema-v1".into(),
            licensing_metadata: "PENDING".into(),
            world_id: hex(r.binding().world_id().as_bytes()),
            coordinate_frame: "global-target-2026-09-27".into(),
            records: serde_json::from_value(v["records"].clone()).expect("qualified fixture"),
            imports: vec![],
            metadata: vec![],
        },
        state: crate::content::ProjectV2State {
            declarations: serde_json::from_value(v["declarations"].clone())
                .expect("qualified fixture"),
            authoring_profiles: serde_json::from_value(v["authoring_profiles"].clone())
                .expect("qualified fixture"),
            source_identity_bindings: serde_json::from_value(v["source_identity_bindings"].clone())
                .expect("qualified fixture"),
            sources: serde_json::from_value(v["sources"].clone()).expect("qualified fixture"),
            ..Default::default()
        },
    }
}
fn fence(r: &ChannelRuntimeV1) -> (ScopeRuntimeFence, RuntimeWorkStamp) {
    let b = r.binding();
    crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
        b.scope_generation(),
    )
    .expect("qualified fixture")
}
fn proposal(a: ExactActorRef, s: &SourceCallbackCast, sequence: u64) -> ProfileAbilityProposal {
    let atom = format!("actor:{}", hex(&a.placement_identity()));
    ProfileAbilityProposal {
        issuer: a,
        target: a,
        ability: s.parent.clone(),
        list: ScheduleList::Attack,
        entry_index: s.index,
        magnitude: s.magnitude,
        range_tiles: s.range,
        occurrence: crate::ability::AbilityOccurrence::new(
            &format!(
                "ai-profile:{}:{sequence}:attack:{}",
                hex(&a.placement_identity()),
                s.index
            ),
            crate::ability::RevisionSet::new(
                "rules-r1",
                "content-r1",
                "policy-r1",
                "definition-r1",
                "sim-r1",
            )
            .expect("qualified fixture"),
        )
        .expect("qualified fixture"),
        intent: crate::ability::AiAbilityAdapter::normalize(&atom, &[&atom])
            .expect("qualified fixture"),
    }
}
struct Reader {
    player: ExactActorRef,
    session: GameSessionId,
    facts: bool,
    pz: bool,
}
impl SpellWorldReader for Reader {
    fn current_players(
        &mut self,
        _: &ChannelRuntimeV1,
        _: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
        Some(vec![(self.player, self.session)])
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
        self.facts.then_some(true)
    }
    fn combat(
        &mut self,
        _: &ChannelRuntimeV1,
        c: ExactActorRef,
        p: ExactActorRef,
        session: GameSessionId,
        _: Option<&str>,
        _: RuntimeWorkStamp,
    ) -> Option<SpellCombatFacts> {
        self.facts.then_some(SpellCombatFacts {
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
            multiplier_ppm: 1000000,
            immune: false,
            condition_policy: None,
        })
    }
}
#[test]
fn nine_source_callbacks_actual_native_damage_delays_stages_and_replay() {
    let w: Witness = serde_json::from_str(include_str!(
        "../../../../../apps/game-server/src/source_callback_cast_fixture.json"
    ))
    .expect("qualified fixture");
    for case in w.cases.values() {
        let (mut r, p, session) = runtime_with_player(0x78);
        let at = if case.caster.ends_with("rootthing_amber_shaper") {
            MovementLocalPosition {
                x: 96,
                y: 100,
                floor: 7,
            }
        } else {
            MovementLocalPosition {
                x: 100,
                y: 99,
                floor: 7,
            }
        };
        r.initialize_movement_test_position(p, at)
            .expect("qualified fixture");
        let actor = r
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                &case.caster,
                300000,
            )
            .expect("qualified fixture");
        let d = draft(&r);
        let creature = Ref {
            family: ProjectV2Family::Creature,
            key: case.caster.clone(),
            revision: "definition-r1".into(),
        };
        let source = SourceCallbackCast::qualify(
            &d,
            &creature,
            case.index,
            r.content_pin().server_artifact_digest(),
        )
        .expect("qualified fixture")
        .expect("qualified fixture");
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &r,
                p,
                session,
                crate::spell::cast::CharacterCastFacts {
                    max_health: 100000,
                    ..FACTS
                },
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .expect("qualified fixture");
        let before =
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session);
        let (mut f, stamp) = fence(&r);
        let mut owner = CallbackCastOwner::new(&r, &f).expect("qualified fixture");
        let mut proposal = proposal(actor, &source, 0);
        proposal.target = p;
        proposal.intent = crate::ability::AiAbilityAdapter::normalize(
            &format!("actor:{}", hex(&actor.placement_identity())),
            &[&format!("actor:{}", hex(&p.placement_identity()))],
        )
        .expect("qualified fixture");
        assert!(
            owner
                .schedule(
                    &r,
                    &f,
                    stamp,
                    &source,
                    &proposal,
                    SemanticTimeMicros::from_micros(0)
                )
                .expect("qualified fixture")
        );
        assert!(
            !owner
                .schedule(
                    &r,
                    &f,
                    stamp,
                    &source,
                    &proposal,
                    SemanticTimeMicros::from_micros(0)
                )
                .expect("qualified fixture")
        );
        let initial_warning = owner.take_source_speech();
        assert_eq!(
            initial_warning.len(),
            usize::from(matches!(
                case.caster.as_str(),
                "oteryn:creature.gaz_haragoth" | "oteryn:creature.lady_tenebris"
            ))
        );
        assert!(owner.take_source_speech().is_empty());
        let mut forged = proposal.clone();
        forged.intent =
            crate::ability::AiAbilityAdapter::normalize("actor:forged", &["actor:forged"])
                .expect("qualified fixture");
        let timers_before = owner.casts.len();
        assert!(matches!(
            owner.schedule(
                &r,
                &f,
                stamp,
                &source,
                &forged,
                SemanticTimeMicros::from_micros(0)
            ),
            Err(AttackError::InvalidPlan)
        ));
        assert_eq!(owner.casts.len(), timers_before);
        let mut forged = proposal.clone();
        forged.target = actor;
        assert!(matches!(
            owner.schedule(
                &r,
                &f,
                stamp,
                &source,
                &forged,
                SemanticTimeMicros::from_micros(0)
            ),
            Err(AttackError::InvalidPlan)
        ));
        assert_eq!(owner.casts.len(), timers_before);
        let mut reader = Reader {
            player: p,
            session,
            facts: true,
            pz: false,
        };
        let damage = DamageSpellOwner::default();
        let mut receipts = 0;
        for delay in [0, 1000, 2000, 3000, 4000, 5000, 7000, 10000] {
            let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(delay * 1000));
            let results = owner
                .drain(
                    &mut r,
                    &mut f,
                    stamp,
                    &mut states,
                    &damage,
                    &mut reader,
                    &clock,
                )
                .expect("qualified fixture");
            receipts += results.len();
            for (_, _, _, result) in results {
                assert!(result.is_ok(), "{} {:?}", case.caster, result);
            }
        }
        let completion_lines = owner.take_source_speech();
        assert_eq!(
            completion_lines.len(),
            usize::from(case.caster == "oteryn:creature.gaz_haragoth")
        );
        assert_eq!(receipts, case.casts.len());
        assert_ne!(
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session),
            before,
            "actual HP unchanged {}",
            case.caster
        );
        let after = crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session);
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(20000000));
        if matches!(
            case.caster.as_str(),
            "oteryn:creature.energy_pulse" | "oteryn:creature.eruption_of_destruction"
        ) {
            assert!(!r.contains_live_creature(actor));
            let removals = owner.take_source_removals();
            assert_eq!(removals.len(), 1);
            assert!(removals[0].1.is_ok());
            assert!(matches!(
                owner.schedule(&r, &f, stamp, &source, &proposal, clock.now()),
                Err(AttackError::StaleIssuer)
            ));
        } else {
            assert!(owner.take_source_removals().is_empty());
            assert!(
                !owner
                    .schedule(&r, &f, stamp, &source, &proposal, clock.now())
                    .expect("qualified fixture")
            );
        }
        assert!(
            owner
                .drain(
                    &mut r,
                    &mut f,
                    stamp,
                    &mut states,
                    &damage,
                    &mut reader,
                    &clock
                )
                .expect("qualified fixture")
                .is_empty()
        );
        assert_eq!(
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session),
            after
        );
    }
}
#[test]
fn every_source_callback_missing_facts_provenance_scope_dead_parent_refuses() {
    let w: Witness = serde_json::from_str(include_str!(
        "../../../../../apps/game-server/src/source_callback_cast_fixture.json"
    ))
    .expect("qualified fixture");
    for case in w.cases.values() {
        let (mut r, p, session) = runtime_with_player(0x79);
        r.initialize_movement_test_position(
            p,
            MovementLocalPosition {
                x: 100,
                y: 99,
                floor: 7,
            },
        )
        .expect("qualified fixture");
        let actor = r
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                &case.caster,
                300000,
            )
            .expect("qualified fixture");
        let mut d = draft(&r);
        let cr = Ref {
            family: ProjectV2Family::Creature,
            key: case.caster.clone(),
            revision: "definition-r1".into(),
        };
        let source = SourceCallbackCast::qualify(
            &d,
            &cr,
            case.index,
            r.content_pin().server_artifact_digest(),
        )
        .expect("qualified fixture")
        .expect("qualified fixture");
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &r,
                p,
                session,
                FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .expect("qualified fixture");
        let before =
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session);
        let (mut f, stamp) = fence(&r);
        let mut owner = CallbackCastOwner::new(&r, &f).expect("qualified fixture");
        let proposal = proposal(actor, &source, 0);
        owner
            .schedule(
                &r,
                &f,
                stamp,
                &source,
                &proposal,
                SemanticTimeMicros::from_micros(0),
            )
            .expect("qualified fixture");
        let mut reader = Reader {
            player: p,
            session,
            facts: false,
            pz: false,
        };
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(10000000));
        let hits = owner
            .drain(
                &mut r,
                &mut f,
                stamp,
                &mut states,
                &DamageSpellOwner::default(),
                &mut reader,
                &clock,
            )
            .expect("qualified fixture");
        assert!(
            hits.iter()
                .all(|v| matches!(v.3, Err(AttackError::MissingCombatFacts)))
        );
        assert_eq!(
            crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, p, session),
            before
        );
        let binding = d
            .state
            .source_identity_bindings
            .iter_mut()
            .find(|b| b.target.key == case.encounter)
            .expect("qualified fixture");
        binding.source_revision = "wrong".into();
        assert!(SourceCallbackCast::qualify(&d, &cr, case.index, source.content).is_err());
        let b = r.binding();
        let (other_f, other_stamp) = crate::foundation::crystal_timer_fixture(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            ScopeOwnershipGeneration::new(2).expect("qualified fixture"),
        )
        .expect("qualified fixture");
        assert!(matches!(
            owner.schedule(&r, &other_f, other_stamp, &source, &proposal, clock.now()),
            Err(AttackError::StaleOwner)
        ));
        r.remove_test_actor(actor).expect("qualified fixture");
        assert!(matches!(
            owner.schedule(&r, &f, stamp, &source, &proposal, clock.now()),
            Err(AttackError::StaleIssuer)
        ));
    }
}
