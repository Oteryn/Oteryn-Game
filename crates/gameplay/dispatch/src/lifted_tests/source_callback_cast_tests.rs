#![allow(clippy::expect_used)]
use crate::content::{ProjectV2DefinitionRef as Ref, ProjectV2Draft, ProjectV2Family};
use crate::creature_auto_attack::ProfileAbilityProposal;
use crate::creature_auto_attack::{AttackError, AttackFacts};
use crate::creature_damage_spell::hex;
use crate::creature_damage_spell::*;
use crate::foundation::ScopeOwnershipGeneration;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::owner_timer::{OwnerClock, VirtualOwnerClock};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, RuntimeWorkStamp, ScopeRuntimeFence};
use crate::foundation::{GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::actor_spell::{
    ChannelSpellStates,
    tests::{FACTS, runtime_with_player},
};
use crate::source_callback_cast::*;
use oteryn_gameplay_foundation::foundation::profile_schedule::ScheduleList;

pub(crate) fn draft(r: &ChannelRuntimeV1) -> ProjectV2Draft {
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
pub(crate) fn fence(r: &ChannelRuntimeV1) -> (ScopeRuntimeFence, RuntimeWorkStamp) {
    let b = r.binding();
    crate::foundation::crystal_timer_fixture(b.runtime_scope(), b.scope_generation())
        .expect("qualified fixture")
}
pub(crate) fn proposal(
    a: ExactActorRef,
    s: &SourceCallbackCast,
    sequence: u64,
) -> ProfileAbilityProposal {
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
    ) -> Option<oteryn_gameplay_content::content::creature_attack_geometry::Facing> {
        Some(oteryn_gameplay_content::content::creature_attack_geometry::Facing::North)
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
                player_level: 1,
                reference_level_party_lowest: None,
            },
            multiplier_ppm: 1000000,
            equipment_protection: None,
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
            b.runtime_scope(),
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

// Canary47dfd51f foam_splash.lua:39-51 retains the original `var` in all
// three addEvent closures. Crystal201189f spells.cpp:1293-1328 constructs a
// VARIANT_POSITION for this needTarget=false spell; combat_functions.cpp:198-215
// executes that saved position rather than the caster's later position.
#[test]
fn srx3_foam_splash_delayed_first_ring_keeps_original_cast_position() {
    let (mut r, player, session) = runtime_with_player(0x7a);
    r.initialize_movement_test_position(
        player,
        MovementLocalPosition {
            x: 100,
            y: 99,
            floor: 7,
        },
    )
    .expect("player position");
    let actor = r
        .admit_monster_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            "oteryn:creature.foam_stalker",
            300000,
        )
        .expect("source creature");
    let draft = draft(&r);
    let creature = Ref {
        family: ProjectV2Family::Creature,
        key: "oteryn:creature.foam_stalker".into(),
        revision: "definition-r1".into(),
    };
    let source = SourceCallbackCast::qualify(
        &draft,
        &creature,
        4,
        r.content_pin().server_artifact_digest(),
    )
    .expect("source proof")
    .expect("source callback");
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &r,
            player,
            session,
            crate::spell::cast::CharacterCastFacts {
                max_health: 100000,
                ..FACTS
            },
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("player vitals");
    let before =
        crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, player, session);
    let (mut fence, stamp) = fence(&r);
    let mut owner = CallbackCastOwner::new(&r, &fence).expect("callback owner");
    let mut proposal = proposal(actor, &source, 0);
    proposal.target = player;
    proposal.intent = crate::ability::AiAbilityAdapter::normalize(
        &format!("actor:{}", hex(&actor.placement_identity())),
        &[&format!("actor:{}", hex(&player.placement_identity()))],
    )
    .expect("original intent");
    owner
        .schedule(
            &r,
            &fence,
            stamp,
            &source,
            &proposal,
            SemanticTimeMicros::from_micros(0),
        )
        .expect("source schedule");
    let snapshot = r
        .read_actor_position(actor)
        .expect("current source position");
    r.borrow_movement_position()
        .commit_creature_relocation(
            actor,
            snapshot,
            MovementLocalPosition {
                x: 105,
                y: 100,
                floor: 7,
            },
        )
        .expect("actual position commit");
    let mut reader = Reader {
        player,
        session,
        facts: true,
        pz: false,
    };
    let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(1_000_000));
    let results = owner
        .drain(
            &mut r,
            &mut fence,
            stamp,
            &mut states,
            &DamageSpellOwner::default(),
            &mut reader,
            &clock,
        )
        .expect("first source pulse");
    assert_eq!(results.len(), 1);
    assert!(results[0].3.is_ok());
    assert_ne!(
        crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, player, session),
        before,
        "first ring must hit original cast neighbor after caster movement"
    );
}

// Canary47dfd51f plagirath_bog.lua:33-40,57-58 retains targetID VARIANT_NUMBER.
#[test]
fn srx3_plagirath_bog_delayed_damage_tracks_original_target_position() {
    let (mut r, player, session) = runtime_with_player(0x7a);
    r.initialize_movement_test_position(
        player,
        MovementLocalPosition {
            x: 100,
            y: 99,
            floor: 7,
        },
    )
    .expect("player position");
    let actor = r
        .admit_monster_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            "oteryn:creature.plagirath",
            300000,
        )
        .expect("source creature");
    let draft = draft(&r);
    let creature = Ref {
        family: ProjectV2Family::Creature,
        key: "oteryn:creature.plagirath".into(),
        revision: "definition-r1".into(),
    };
    let source = SourceCallbackCast::qualify(
        &draft,
        &creature,
        6,
        r.content_pin().server_artifact_digest(),
    )
    .expect("source proof")
    .expect("source callback");
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &r,
            player,
            session,
            crate::spell::cast::CharacterCastFacts {
                max_health: 100000,
                ..FACTS
            },
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("player vitals");
    let before =
        crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, player, session);
    let (mut fence, stamp) = fence(&r);
    let mut owner = CallbackCastOwner::new(&r, &fence).expect("callback owner");
    let mut proposal = proposal(actor, &source, 0);
    proposal.target = player;
    proposal.intent = crate::ability::AiAbilityAdapter::normalize(
        &format!("actor:{}", hex(&actor.placement_identity())),
        &[&format!("actor:{}", hex(&player.placement_identity()))],
    )
    .expect("original intent");
    owner
        .schedule(
            &r,
            &fence,
            stamp,
            &source,
            &proposal,
            SemanticTimeMicros::from_micros(0),
        )
        .expect("source schedule");
    for x in 101..=118 {
        let snapshot = r
            .read_actor_position(player)
            .expect("current target position");
        r.borrow_movement_position()
            .commit_cardinal(snapshot, MovementLocalPosition { x, y: 99, floor: 7 })
            .expect("actual target position commit");
    }
    let mut reader = Reader {
        player,
        session,
        facts: true,
        pz: false,
    };
    let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(10_000_000));
    let results = owner
        .drain(
            &mut r,
            &mut fence,
            stamp,
            &mut states,
            &DamageSpellOwner::default(),
            &mut reader,
            &clock,
        )
        .expect("first source pulse");
    assert_eq!(results.len(), 1);
    assert!(results[0].3.is_ok());
    assert_ne!(
        crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, player, session),
        before,
        "delayed poison must follow original target within source distance20"
    );
}

#[test]
fn srx3_native_source_component_damage_hooks_are_available() {
    let _definition =
        crate::ability::creature_bite::CreatureBiteDefinition::from_native_source_with_roll;
    let _bite =
        <dyn crate::ability::creature_bite::CreatureBiteVitals>::source_combat_component_damage;
    let _spell = <dyn SpellWorldReader>::source_combat_component_damage;
}

// Canary47dfd51f plagirath_bog.lua:39 uses strictdistance<20.
#[test]
fn srx3_plagirath_bog_delayed_damage_refuses_source_distance_twenty() {
    let (mut r, player, session) = runtime_with_player(0x7c);
    r.initialize_movement_test_position(
        player,
        MovementLocalPosition {
            x: 100,
            y: 99,
            floor: 7,
        },
    )
    .expect("player position");
    let actor = r
        .admit_monster_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            "oteryn:creature.plagirath",
            300000,
        )
        .expect("source creature");
    let draft = draft(&r);
    let creature = Ref {
        family: ProjectV2Family::Creature,
        key: "oteryn:creature.plagirath".into(),
        revision: "definition-r1".into(),
    };
    let source = SourceCallbackCast::qualify(
        &draft,
        &creature,
        6,
        r.content_pin().server_artifact_digest(),
    )
    .expect("source proof")
    .expect("source callback");
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &r,
            player,
            session,
            crate::spell::cast::CharacterCastFacts {
                max_health: 100000,
                ..FACTS
            },
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("player vitals");
    let before =
        crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, player, session);
    let (mut fence, stamp) = fence(&r);
    let mut owner = CallbackCastOwner::new(&r, &fence).expect("callback owner");
    let mut proposal = proposal(actor, &source, 0);
    proposal.target = player;
    proposal.intent = crate::ability::AiAbilityAdapter::normalize(
        &format!("actor:{}", hex(&actor.placement_identity())),
        &[&format!("actor:{}", hex(&player.placement_identity()))],
    )
    .expect("original intent");
    owner
        .schedule(
            &r,
            &fence,
            stamp,
            &source,
            &proposal,
            SemanticTimeMicros::from_micros(0),
        )
        .expect("source schedule");
    for x in 101..=120 {
        let snapshot = r
            .read_actor_position(player)
            .expect("current target position");
        r.borrow_movement_position()
            .commit_cardinal(snapshot, MovementLocalPosition { x, y: 99, floor: 7 })
            .expect("actual target position commit");
    }
    let mut reader = Reader {
        player,
        session,
        facts: true,
        pz: false,
    };
    let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(10_000_000));
    let results = owner
        .drain(
            &mut r,
            &mut fence,
            stamp,
            &mut states,
            &DamageSpellOwner::default(),
            &mut reader,
            &clock,
        )
        .expect("first source pulse");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].3,
        Err(crate::creature_auto_attack::AttackError::OutOfRange)
    );
    assert_eq!(
        crate::gameplay_transport::actor_spell::observe_vitals(&r, &states, player, session),
        before,
        "source distance20 must cancel the delayed poison"
    );
}

// Canary47dfd51f plagirath_bog.lua:6-20 is the explicitdiamond matrix,
// not AreaCombat's numeric radius4kernel (which has different edges).
#[test]
fn srx3_plagirath_bog_preserves_exact_source_matrix_edges() {
    let (runtime, _, _) = runtime_with_player(0x7d);
    let definitions = draft(&runtime);
    let creature = Ref {
        family: ProjectV2Family::Creature,
        key: "oteryn:creature.plagirath".into(),
        revision: "definition-r1".into(),
    };
    let source = SourceCallbackCast::qualify(
        &definitions,
        &creature,
        6,
        runtime.content_pin().server_artifact_digest(),
    )
    .expect("source proof")
    .expect("source callback");
    let stages = source.stages();
    assert_eq!(stages.len(), 1);
    let area = stages[0].1.area.as_ref().expect("source poison matrix");
    let cells = oteryn_gameplay_content::content::creature_attack_geometry::offsets(
        area,
        oteryn_gameplay_content::content::creature_attack_geometry::Facing::North,
    )
    .expect("source matrix geometry");
    assert_eq!(cells.len(), 41);
    assert!(cells.contains(&(4, 0)) && cells.contains(&(0, 4)) && cells.contains(&(3, 1)));
    assert!(!cells.contains(&(3, 2)) && !cells.contains(&(4, 1)));
}

#[test]
fn srx3_four_current_foam_stalkers_keep_independent_source_casts() {
    let (mut runtime, target, _) =
        crate::gameplay_transport::actor_spell::tests::runtime_with_capacity(0x7e, 8);
    runtime
        .initialize_movement_test_position(
            target,
            MovementLocalPosition {
                x: 100,
                y: 99,
                floor: 7,
            },
        )
        .expect("actual target position");
    let definitions = draft(&runtime);
    let creature = Ref {
        family: ProjectV2Family::Creature,
        key: "oteryn:creature.foam_stalker".into(),
        revision: "definition-r1".into(),
    };
    let source = SourceCallbackCast::qualify(
        &definitions,
        &creature,
        4,
        runtime.content_pin().server_artifact_digest(),
    )
    .expect("source proof")
    .expect("source callback");
    let (fence, stamp) = fence(&runtime);
    let mut owner = CallbackCastOwner::new(&runtime, &fence).expect("current owner");
    for ordinal in 0..4 {
        let actor = runtime
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 100 + ordinal,
                    y: 100,
                    floor: 7,
                },
                &creature.key,
                300000,
            )
            .expect("independent real actor slot");
        let mut proposal = proposal(actor, &source, 0);
        proposal.target = target;
        proposal.intent = crate::ability::AiAbilityAdapter::normalize(
            &format!("actor:{}", hex(&actor.placement_identity())),
            &[&format!("actor:{}", hex(&target.placement_identity()))],
        )
        .expect("actual target intent");
        assert_eq!(
            owner.schedule(
                &runtime,
                &fence,
                stamp,
                &source,
                &proposal,
                SemanticTimeMicros::from_micros(0)
            ),
            Ok(true),
            "source has independent addEvent closures percurrent caster {ordinal}"
        );
    }
    assert_eq!(owner.casts.len(), 4);
}

#[test]
fn srx3_source_callback_missing_facts_retries_original_due_stage_once() {
    let (mut runtime, player, session) = runtime_with_player(0x7f);
    runtime
        .initialize_movement_test_position(
            player,
            MovementLocalPosition {
                x: 100,
                y: 99,
                floor: 7,
            },
        )
        .expect("actual player position");
    let actor = runtime
        .admit_monster_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            "oteryn:creature.foam_stalker",
            300000,
        )
        .expect("actual current caster");
    let source = SourceCallbackCast::qualify(
        &draft(&runtime),
        &Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.foam_stalker".into(),
            revision: "definition-r1".into(),
        },
        4,
        runtime.content_pin().server_artifact_digest(),
    )
    .expect("source qualification")
    .expect("actual source callback");
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &runtime,
            player,
            session,
            crate::spell::cast::CharacterCastFacts {
                max_health: 100000,
                ..FACTS
            },
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("canonical player vitals");
    let before =
        crate::gameplay_transport::actor_spell::observe_vitals(&runtime, &states, player, session);
    let (mut fence, stamp) = fence(&runtime);
    let mut owner = CallbackCastOwner::new(&runtime, &fence).expect("current callback owner");
    let proposal = proposal(actor, &source, 0);
    owner
        .schedule(
            &runtime,
            &fence,
            stamp,
            &source,
            &proposal,
            SemanticTimeMicros::from_micros(0),
        )
        .expect("original source callback schedule");
    let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(1_000_000));
    let mut reader = Reader {
        player,
        session,
        facts: false,
        pz: false,
    };
    let first = owner
        .drain(
            &mut runtime,
            &mut fence,
            stamp,
            &mut states,
            &DamageSpellOwner::default(),
            &mut reader,
            &clock,
        )
        .expect("missing DATA pass");
    assert_eq!(first.len(), 1);
    assert!(matches!(first[0].3, Err(AttackError::MissingCombatFacts)));
    assert_eq!(
        crate::gameplay_transport::actor_spell::observe_vitals(&runtime, &states, player, session),
        before
    );
    reader.facts = true;
    let retried = owner
        .drain(
            &mut runtime,
            &mut fence,
            stamp,
            &mut states,
            &DamageSpellOwner::default(),
            &mut reader,
            &clock,
        )
        .expect("same due callback with current DATA");
    assert_eq!(
        retried.len(),
        1,
        "unavailable DATA must retain the original source callback, not silently consume it"
    );
    assert!(retried[0].3.is_ok());
    let after =
        crate::gameplay_transport::actor_spell::observe_vitals(&runtime, &states, player, session);
    assert_ne!(after, before);
    assert!(
        owner
            .drain(
                &mut runtime,
                &mut fence,
                stamp,
                &mut states,
                &DamageSpellOwner::default(),
                &mut reader,
                &clock
            )
            .expect("replay pass")
            .is_empty()
    );
    assert_eq!(
        crate::gameplay_transport::actor_spell::observe_vitals(&runtime, &states, player, session),
        after
    );
}

#[test]
fn srx3_target_tile_casts_retain_exact_four_base_vocation_selector() {
    use oteryn_gameplay_content::content::spell_data::Vocation;
    let mut wrongly_allowed = Vec::new();
    for (slug, index) in [
        ("gaz_haragoth", 10),
        ("eruption_of_destruction", 1),
        ("lady_tenebris", 4),
    ] {
        let (mut runtime, _, _) = runtime_with_player(0x6e);
        let key = format!("oteryn:creature.{slug}");
        let caster = runtime
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                &key,
                300000,
            )
            .expect("real caster slot");
        let source = SourceCallbackCast::qualify(
            &draft(&runtime),
            &Ref {
                family: ProjectV2Family::Creature,
                key,
                revision: "definition-r1".into(),
            },
            index,
            runtime.content_pin().server_artifact_digest(),
        )
        .expect("complete source closure")
        .expect("actual source callback");
        let (fence, stamp) = fence(&runtime);
        let mut owner = CallbackCastOwner::new(&runtime, &fence).expect("actual callback owner");
        owner
            .schedule(
                &runtime,
                &fence,
                stamp,
                &source,
                &proposal(caster, &source, 0),
                SemanticTimeMicros::from_micros(0),
            )
            .expect("original source occurrence");
        let plans = owner
            .due_stages(
                &runtime,
                &fence,
                stamp,
                SemanticTimeMicros::from_micros(10_000_000),
            )
            .expect("actual due source stage");
        assert_eq!(plans.len(), 1);
        for vocation in [Vocation::None, Vocation::Monk, Vocation::ExaltedMonk] {
            if plans[0].permits_player_vocation(vocation) {
                wrongly_allowed.push((slug, vocation));
            }
        }
        for vocation in [
            Vocation::Sorcerer,
            Vocation::MasterSorcerer,
            Vocation::Druid,
            Vocation::ElderDruid,
            Vocation::Paladin,
            Vocation::RoyalPaladin,
            Vocation::Knight,
            Vocation::EliteKnight,
        ] {
            assert!(plans[0].permits_player_vocation(vocation));
        }
    }
    assert!(
        wrongly_allowed.is_empty(),
        "pinned target-tile Lua lists exactly four base IDs: {wrongly_allowed:?}"
    );
}

#[test]
fn srx3_source_cast_stages_retain_literal_donor_impact_bindings() {
    for (slug, index, expected) in [
        ("energy_pulse", 0, vec!["energyarea"]),
        (
            "foam_stalker",
            4,
            vec!["watersplash", "watersplash", "watersplash"],
        ),
        ("charging_outburst", 5, vec!["purpleenergy"]),
        ("eruption_of_destruction", 1, vec!["firearea"]),
        ("gaz_haragoth", 10, vec!["purpleenergy"]),
        ("lady_tenebris", 4, vec!["mortarea"]),
        ("plagirath", 6, vec!["poison"]),
        (
            "rootthing_amber_shaper",
            4,
            vec!["smallplants", "fireattack"],
        ),
        ("rootthing_nutshell", 3, vec!["holydamage", "groundshaker"]),
    ] {
        let (mut runtime, _, _) = runtime_with_player(0x6f);
        let key = format!("oteryn:creature.{slug}");
        let caster = runtime
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                &key,
                300000,
            )
            .expect("real source caster");
        let source = SourceCallbackCast::qualify(
            &draft(&runtime),
            &Ref {
                family: ProjectV2Family::Creature,
                key,
                revision: "definition-r1".into(),
            },
            index,
            runtime.content_pin().server_artifact_digest(),
        )
        .expect("frozen source closure")
        .expect("actual callback");
        let (fence, stamp) = fence(&runtime);
        let mut owner = CallbackCastOwner::new(&runtime, &fence).expect("real owner");
        owner
            .schedule(
                &runtime,
                &fence,
                stamp,
                &source,
                &proposal(caster, &source, 0),
                SemanticTimeMicros::from_micros(0),
            )
            .expect("actual parent occurrence");
        let plans = owner
            .due_stages(
                &runtime,
                &fence,
                stamp,
                SemanticTimeMicros::from_micros(10_000_000),
            )
            .expect("exact retained due stages");
        assert_eq!(plans.len(), expected.len());
        for (plan, expected) in plans.iter().zip(expected) {
            assert_eq!(
                plan.source_impact_binding(),
                Some(format!("canary.appearance:effect/{expected}").as_str()),
                "{slug} loses the donor stage visual even when no player takes damage"
            );
        }
    }
}

#[test]
fn srx3_source_large_callback_areas_keep_literal_lua_matrix_edges() {
    let (runtime, _, _) = runtime_with_player(0x73);
    let definitions = draft(&runtime);
    for (slug, index, count, included, excluded) in [
        ("charging_outburst", 5, 101, (2, -5), (3, -5)),
        ("eruption_of_destruction", 1, 101, (2, -5), (3, -5)),
        ("gaz_haragoth", 10, 101, (2, -5), (3, -5)),
        ("lady_tenebris", 4, 113, (1, -6), (2, -6)),
    ] {
        let source = SourceCallbackCast::qualify(
            &definitions,
            &Ref {
                family: ProjectV2Family::Creature,
                key: format!("oteryn:creature.{slug}"),
                revision: "definition-r1".into(),
            },
            index,
            runtime.content_pin().server_artifact_digest(),
        )
        .expect("source closure")
        .expect("actual callback");
        let cells = oteryn_gameplay_content::content::creature_attack_geometry::offsets(
            source.stages()[0]
                .1
                .area
                .as_ref()
                .expect("literal source area"),
            oteryn_gameplay_content::content::creature_attack_geometry::Facing::North,
        )
        .expect("actual geometry owner");
        assert_eq!(cells.len(), count, "{slug}");
        assert!(cells.contains(&included), "{slug}");
        assert!(!cells.contains(&excluded), "{slug}");
    }
}
