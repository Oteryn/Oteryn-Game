//! Real native source-owner regressions for the accepted charm incoming order.
//! This file is auto-mounted; the cases are in-memory owner tests, not DB coverage.
#![allow(
    clippy::expect_used,
    reason = "test fixture invariants use named assertions"
)]
use crate::ai_think::{
    ThinkSequenceTracker,
    profile_schedule::{AttackTarget, ProfileScheduleState},
};
use crate::content::{
    ProjectReferenceRecord, ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Family,
};
use crate::creature_auto_attack::{AttackFacts, ProfileAbilityProposal};
use crate::creature_damage_spell::*;
use crate::foundation::MovementLocalPosition;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeWorkStamp, ScopeRuntimeFence,
};
use crate::gameplay_transport::actor_spell::{ChannelSpellStates, tests::FACTS};
use oteryn_gameplay_content::content::creature_attack_geometry::Facing;
use oteryn_simulation_determinism::GameplayDecisionRoot;

struct CurrentWorld {
    facts: SpellCombatFacts,
    missing: bool,
    blocked: bool,
    facing: Option<Facing>,
}
impl SpellWorldReader for CurrentWorld {
    fn current_players(
        &mut self,
        r: &ChannelRuntimeV1,
        _: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
        r.player_control_facts(self.facts.attack.target, self.facts.attack.session)
            .ok()?;
        Some(vec![(self.facts.attack.target, self.facts.attack.session)])
    }
    fn current_facing(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<Facing> {
        self.facing
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
        (!self.missing).then_some(!self.blocked)
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
        r.read_actor_position(i).ok()?;
        r.player_control_facts(t, s).ok()?;
        (!self.missing
            && self.facts.attack.issuer == i
            && self.facts.attack.target == t
            && self.facts.attack.session == s)
            .then(|| self.facts.clone())
    }
}
fn test_uuid(tag: u8) -> [u8; 16] {
    [
        0x01, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag,
    ]
}
fn runtime_three_slots() -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
    use {
        crate::foundation::{ChannelId, NodeId, WorldId},
        oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
    };
    let w = WorldId::decode(&test_uuid(0x60)).expect("qualified fixture");
    let mut r = ChannelRuntimeV1::from_committed_assignment(
        w,
        ChannelId::decode(&test_uuid(0x61)).expect("qualified fixture"),
        NodeId::decode(&test_uuid(0x62)).expect("qualified fixture"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        3,
        ChannelContentPin::test(w),
    )
    .expect("qualified fixture");
    let s = GameSessionId::decode(&test_uuid(0x71)).expect("qualified fixture");
    let reservation = r.reserve_fresh_session(s).expect("qualified fixture");
    let a = r
        .commit_fresh_session(reservation)
        .expect("qualified fixture");
    (r, a, s)
}
fn native() -> (Vec<ProjectReferenceRecord>, Vec<ProjectV2AuthoringProfile>) {
    let v: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server/src/creature_auto_attack_test_data.json"
    )))
    .expect("qualified fixture");
    (
        serde_json::from_value(v["records"].clone()).expect("qualified fixture"),
        serde_json::from_value(v["authoring_profiles"].clone()).expect("qualified fixture"),
    )
}
fn setup_named_at(
    name: &str,
    index: usize,
    at: MovementLocalPosition,
) -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    SpellSource,
    ProfileAbilityProposal,
    CurrentWorld,
    ScopeRuntimeFence,
    RuntimeWorkStamp,
) {
    let (mut r, t, s) = runtime_three_slots();
    r.initialize_source_pinned_lab_player_position(t, s, at)
        .expect("qualified fixture");
    let (records, profiles) = if matches!(
        name,
        "radiant_acolyte"
            | "radiant_paragon"
            | "radiant_templar"
            | "phosphorus_final"
            | "cake_golem"
    ) {
        let document: crate::content::native_gameplay::CreatureProfilesDocument =
            serde_json::from_slice(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/creatures/definitions/spell-native-profiles.json"
            )))
            .expect("actual native profiles");
        let definitions = document
            .records
            .into_iter()
            .find(|record| record.profile.target.key == format!("oteryn:creature.{name}"))
            .and_then(|record| record.attack_definitions)
            .expect("actual Summer closure");
        (definitions.records, definitions.profiles)
    } else {
        native()
    };
    let cref = Ref {
        family: ProjectV2Family::Creature,
        key: format!("oteryn:creature.{name}"),
        revision: "definition-r1".into(),
    };
    let health = profiles
        .iter()
        .find_map(|p| match &p.data {
            Data::Creature(c) if p.target == cref => c.health,
            _ => None,
        })
        .expect("qualified fixture");
    let issuer = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition { x: at.x + 1, ..at },
            &cref.key,
            health as i64,
        )
        .expect("qualified fixture");
    let source = SpellSource::from_native(
        &cref,
        index,
        &records,
        &profiles,
        r.content_pin().server_artifact_digest(),
    )
    .expect("qualified fixture");
    let abilities = profiles
        .iter()
        .filter_map(|p| match &p.data {
            Data::Ability(a) => Some((p.target.clone(), a.clone())),
            _ => None,
        })
        .collect();
    let behavior = profiles
        .iter()
        .find_map(|p| match &p.data {
            Data::Behavior(b) if p.target.key == format!("oteryn:behavior.creature.{name}") => {
                Some(b)
            }
            _ => None,
        })
        .expect("qualified fixture");
    let root = GameplayDecisionRoot::from_bytes(r.content_pin().server_artifact_digest());
    let revisions = crate::ability::RevisionSet::new(
        "rules-r1",
        "content-r1",
        "policy-r1",
        "definition-r1",
        "sim-r1",
    )
    .expect("qualified fixture");
    let mut schedule = ProfileScheduleState::new(issuer);
    let mut tracker = ThinkSequenceTracker::new();
    let mut found = None;
    for _ in 0..256 {
        let occurrence = tracker.next_occurrence(issuer);
        let summon_facts = behavior.summons.as_ref().map(|summons| {
            crate::ai_think::profile_schedule::MonsterSummonFacts {
                occurrence,
                is_summon: r
                    .native_summon_role(issuer)
                    .expect("qualified fixture")
                    .is_some(),
                target_with_path: None,
                total_count: r.native_summon_count(issuer, None),
                entry_counts: summons
                    .entries
                    .iter()
                    .map(
                        |entry| crate::ai_think::profile_schedule::ObservedSummonCount {
                            creature: entry.creature.clone(),
                            count: r.native_summon_count(issuer, Some(&entry.creature.key)),
                        },
                    )
                    .collect(),
            }
        });
        let plan = schedule
            .prepare_with_summons(
                crate::ai_think::profile_schedule::ProfileScheduleInput {
                    occurrence,
                    behavior,
                    abilities: &abilities,
                    target: Some(AttackTarget {
                        actor: t,
                        same_floor_distance: Some(1),
                    }),
                    revisions: &revisions,
                    root: &root,
                },
                summon_facts.as_ref(),
            )
            .expect("native source schedule with independently current carrier summon counts");
        if let Some(p) = plan
            .proposals
            .into_iter()
            .find(|p| p.ability == source.ability)
        {
            found = Some(p);
            break;
        }
    }
    let proposal = found.expect("actual source chance/timing eventually prepares native spell");
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &r,
            t,
            s,
            crate::spell::cast::CharacterCastFacts {
                max_health: 10_000,
                max_mana: 10_000,
                ..FACTS
            },
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture");
    let facts = SpellCombatFacts {
        attack: AttackFacts {
            issuer,
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
            player_level: 1,
            reference_level_party_lowest: None,
        },
        multiplier_ppm: 1_000_000,
        equipment_protection: None,
        immune: false,
        condition_policy: None,
    };
    let b = r.binding();
    let (fence, stamp) =
        crate::foundation::crystal_timer_fixture(b.runtime_scope(), b.scope_generation())
            .expect("qualified fixture");
    (
        r,
        states,
        source,
        proposal,
        CurrentWorld {
            facts,
            missing: false,
            blocked: false,
            facing: None,
        },
        fence,
        stamp,
    )
}

fn qualify_hazard_monster(runtime: &mut ChannelRuntimeV1, actor: ExactActorRef, key: &str) {
    let policies = crate::content::native_gameplay::callback_source_native_test_policies(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/creatures/definitions/spell-native-profiles.json"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/presentations/definitions/spell-native-profiles.json"
        )),
        runtime.content_pin().server_artifact_digest(),
    )
    .expect("actual native Creature policies");
    runtime
        .install_companion_policies(policies)
        .expect("pinned policy table");
    runtime
        .install_creature_policy(actor, key)
        .expect("actual wild Creature policy");
    assert!(
        runtime
            .companion_snapshot_including_dead(actor)
            .expect("current monster")
            .state
            .master
            .is_none()
    );
}

#[test]
fn incoming_order_parry_uses_original_native_roll() {
    std::thread::Builder::new()
        .name("incoming-order-parry".into())
        .stack_size(64 << 20)
        .spawn(|| {
            let mut reflected = Vec::new();
            for multiplier in [1_000_000, 500_000] {
                let (mut runtime, mut states, source, proposal, mut world, fence, stamp) =
                    setup_named_at(
                        "1st_mate_ratticus",
                        1,
                        MovementLocalPosition {
                            x: 100,
                            y: 100,
                            floor: 7,
                        },
                    );
                let session = world.facts.attack.session;
                qualify_hazard_monster(
                    &mut runtime,
                    proposal.issuer,
                    "oteryn:creature.1st_mate_ratticus",
                );
                runtime
                    .bind_attacker_lease(
                        proposal.target,
                        session,
                        crate::foundation::CharacterLease::new(
                            crate::foundation::CharacterId::decode(&test_uuid(0x72))
                                .expect("character"),
                            1,
                        )
                        .expect("lease"),
                    )
                    .expect("principal");
                world.facts.multiplier_ppm = multiplier;
                let before = runtime
                    .companion_snapshot(proposal.issuer)
                    .expect("source")
                    .health;
                let mut owner = DamageSpellOwner::default();
                let result = states
                    .with_source_charm_test(
                        &mut runtime,
                        proposal.issuer,
                        proposal.target,
                        session,
                        "parry",
                        |runtime, vitals| {
                            owner.execute(
                                runtime,
                                &fence,
                                stamp,
                                vitals,
                                &source,
                                &proposal,
                                &mut world,
                                SemanticTimeMicros::from_micros(2_000_000),
                            )
                        },
                    )
                    .expect("real native cast");
                assert!(result.targets[0].1.as_ref().expect("hit").damage.is_some());
                let after = runtime
                    .companion_snapshot_including_dead(proposal.issuer)
                    .expect("source")
                    .health;
                reflected.push(before - after);
            }
            assert!(
                reflected[0] > 0,
                "Parry must reflect the admitted native hit"
            );
            assert_eq!(
                reflected[0], reflected[1],
                "player resistance cannot reduce Parry's original rolled basis"
            );
        })
        .expect("thread")
        .join()
        .expect("scenario");
}

struct CertainCharms(Vec<oteryn_gameplay_progression::charm_effects::CharmDefinition>);
impl oteryn_gameplay_progression::charm_effects::CharmCatalogueRead for CertainCharms {
    fn charm(
        &self,
        key: &str,
    ) -> Option<&oteryn_gameplay_progression::charm_effects::CharmDefinition> {
        self.0.iter().find(|definition| definition.key() == key)
    }
}
fn assign_charms(
    states: &mut ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    source: ExactActorRef,
    target: ExactActorRef,
    session: GameSessionId,
    names: &[&str],
) -> CertainCharms {
    use crate::domain::charm::{BestiaryRaceKey, CharmKey, CharmStage};
    use crate::durability::charm_state::{CharacterCharmState, CombatCharmState};
    use oteryn_gameplay_progression::charm_effects::{
        CharmCatalogueRead, CharmDefinition, CharmPercent,
    };
    let canonical = crate::content::charm_source::CanonicalCharmCatalogue::embedded()
        .expect("canonical charms");
    let race = runtime
        .companion_snapshot_including_dead(source)
        .expect("source")
        .state
        .policy
        .definition_key
        .clone();
    let mut certain = Vec::new();
    let mut state = CharacterCharmState {
        unlocks: Default::default(),
        assignments: Default::default(),
    };
    for name in names {
        let key = CharmKey::new(format!("oteryn:charm.{name}")).expect("key");
        let definition = canonical.charm(key.as_str()).expect("definition");
        certain.push(
            CharmDefinition::new(
                key.as_str(),
                definition.category(),
                definition.effect().stage_value(),
                [100, 5000, 10000]
                    .map(|chance| CharmPercent::from_hundredths(chance).expect("chance")),
                definition.effect(),
            )
            .expect("certain"),
        );
        state
            .unlocks
            .insert(key.clone(), CharmStage::new(3).expect("stage"));
        state
            .assignments
            .insert(key, BestiaryRaceKey::new(race.clone()).expect("race"));
    }
    let character = *runtime
        .bound_attacker_lease(target, session)
        .expect("principal")
        .character_id()
        .as_bytes();
    assert!(states.install_charm_state(
        runtime,
        target,
        session,
        CombatCharmState::new(
            crate::domain::CharacterId::from_bytes(character).expect("character"),
            state
        )
    ));
    CertainCharms(certain)
}

#[test]
fn incoming_order_native_hit_gate_shield_lethal_source_and_player_minor() {
    std::thread::Builder::new().name("incoming-order-native".into()).stack_size(64 << 20).spawn(|| {
        use crate::player_lethal::PlayerLethalVitals;
        use crate::foundation::condition::{ApplicationFacts, ConditionDefinition};
        use oteryn_gameplay_domain::domain::condition::ConditionValues;
        for mode in ["shield", "blocked", "source-lethal-numb", "source-lethal-haste", "source-lethal-cleanse", "player-lethal"] {
            let (mut runtime, mut states, source, proposal, mut world, fence, stamp) = setup_named_at("1st_mate_ratticus", 1,
                MovementLocalPosition { x: 100, y: 100, floor: 7 });
            let target = proposal.target;
            let issuer = proposal.issuer;
            let session = world.facts.attack.session;
            qualify_hazard_monster(&mut runtime, issuer, "oteryn:creature.1st_mate_ratticus");
            runtime.bind_attacker_lease(target, session, crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(&test_uuid(0x72)).expect("character"), 1).expect("lease")).expect("principal");
            let names: &[&str] = match mode { "source-lethal-numb" => &["parry", "numb"],
                "source-lethal-haste" => &["parry", "adrenaline_burst"],
                "source-lethal-cleanse" => &["parry", "cleanse"], _ => &["parry"] };
            let catalogue = assign_charms(&mut states, &runtime, issuer, target, session, names);
            if mode.starts_with("source-lethal") {
                let health = runtime.companion_snapshot(issuer).expect("source").health;
                runtime.commit_source_damage(issuer, target, session, b"fixture:wound-source", 1,
                    u32::try_from(health - 1).expect("HP"), 0).expect("source wound");
            }
            if mode == "player-lethal" {
                states.apply_typed_attack_damage(&mut runtime, target, session, 9_999, crate::player_lethal::PlayerHitKind::Undefined,
                    "fixture:wound-player", SemanticTimeMicros::from_micros(0)).expect("player wound");
            }
            if mode == "blocked" { world.facts.immune = true; }
            let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
            if mode == "source-lethal-cleanse" {
                let definition = ConditionDefinition::new_damage_schedule("fixture:cleanse-poison", 1,
                    oteryn_gameplay_domain::domain::condition::DotElement::Poison,
                    crate::foundation::condition::DamageSchedule::Fixed { delayed: true, segments: vec![crate::foundation::condition::DamageSegment { count: 3, interval_ms: 1000, amount: 10 }] }).expect("poison");
                let facts = ApplicationFacts { now: 0, base_speed: 220, mana_shield_capacity: 0,
                    target_reentry_protected: false, source_reentry_protected: false, target_is_player: true,
                    decision_root: &root, occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([25; 16]) };
                assert!(states.install_owned_source_condition_fixture(&runtime, target, session, issuer, &[definition], &facts));
            }
            if mode == "shield" {
                let definition = ConditionDefinition::new("fixture:incoming-shield", 1, ConditionValues::ManaShield { duration_ms: 60_000 }).expect("shield");
                let facts = ApplicationFacts { now: 0, base_speed: 220, mana_shield_capacity: 10_000,
                    target_reentry_protected: false, source_reentry_protected: false, target_is_player: true,
                    decision_root: &root, occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([23; 16]) };
                assert!(states.install_owned_source_condition_fixture(&runtime, target, session, issuer, &[definition], &facts));
            }
            let before = states.get(&runtime, target, session).expect("player").vitals();
            let source_before = runtime.companion_snapshot(issuer).expect("source").health;
            let mut attack = crate::gameplay_transport::attack::ChannelAttackStates::default();
            let mut owner = DamageSpellOwner::default();
            let mut vitals = crate::gameplay_transport::attack::charm_combat::source_vitals::CharmSourceVitals {
                states: &mut states, attack: &mut attack, catalogue: &catalogue, root };
            let result = owner.execute(&mut runtime, &fence, stamp, &mut vitals, &source, &proposal, &mut world,
                SemanticTimeMicros::from_micros(2_000_000)).expect("cast");
            assert!(result.targets[0].1.is_ok(), "{mode}: {result:?}");
            let source_after = runtime.companion_snapshot_including_dead(issuer).expect("source").health;
            if mode == "blocked" {
                assert_eq!(source_after, source_before);
                assert_eq!(states.get(&runtime, target, session).expect("player").vitals(), before);
            } else {
                assert!(source_after < source_before, "{mode}: admitted hit reflects");
                if mode.starts_with("source-lethal") { assert_eq!(source_after, 0); }
                if mode == "shield" {
                    let after = states.get(&runtime, target, session).expect("player").vitals();
                    assert_eq!(after.health, before.health);
                    assert!(after.mana < before.mana);
                }
                if mode == "source-lethal-haste" {
                    assert!(states.get(&runtime, target, session).expect("player").owned_conditions().speed_delta_at(2_000_000) > 0);
                }
                if mode == "source-lethal-numb" {
                    assert_eq!(runtime.companion_snapshot_including_dead(issuer).expect("source").state.conditions.speed_delta_at(2_000_000), 0);
                }
                if mode == "source-lethal-cleanse" {
                    assert_eq!(states.get(&runtime, target, session).expect("player").owned_conditions().cleanse_candidate_count(2_000_000), 0);
                }
                if mode == "player-lethal" { assert!(states.is_dead(target)); }
            }
            let after = states.get(&runtime, target, session).expect("player").vitals();
            let catalogue = assign_charms(&mut states, &runtime, issuer, target, session, &["dodge"]);
            let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
            let replay = owner.execute(&mut runtime, &fence, stamp,
                &mut crate::gameplay_transport::attack::charm_combat::source_vitals::CharmSourceVitals { states: &mut states, attack: &mut attack, catalogue: &catalogue, root },
                &source, &proposal, &mut world, SemanticTimeMicros::from_micros(9_000_000)).expect("retained native occurrence after selection change");
            assert_eq!(replay, result, "{mode}: no reroll after assignment change");
            assert_eq!(states.get(&runtime, target, session).expect("player").vitals(), after);
            assert_eq!(runtime.companion_snapshot_including_dead(issuer).expect("source").health, source_after);

        }
    }).expect("thread").join().expect("scenario");
}

#[test]
fn incoming_order_native_condition_ticks_dodge_parry_source_liveness_and_agony() {
    std::thread::Builder::new()
        .name("incoming-order-ticks".into())
        .stack_size(64 << 20)
        .spawn(|| {
            use crate::foundation::condition::{
                ApplicationFacts, ConditionDefinition, DamageSchedule, DamageSegment, TickFacts,
            };
            use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
            for mode in ["dodge", "parry", "dead", "retired", "agony", "immune"] {
                let (mut runtime, mut states, _, proposal, world, _, _) = setup_named_at(
                    "1st_mate_ratticus",
                    1,
                    MovementLocalPosition {
                        x: 100,
                        y: 100,
                        floor: 7,
                    },
                );
                let target = proposal.target;
                let issuer = proposal.issuer;
                let session = world.facts.attack.session;
                qualify_hazard_monster(&mut runtime, issuer, "oteryn:creature.1st_mate_ratticus");
                runtime
                    .bind_attacker_lease(
                        target,
                        session,
                        crate::foundation::CharacterLease::new(
                            crate::foundation::CharacterId::decode(&test_uuid(0x72))
                                .expect("character"),
                            1,
                        )
                        .expect("lease"),
                    )
                    .expect("principal");
                let names: &[&str] = if mode == "parry" {
                    &["parry", "adrenaline_burst"]
                } else {
                    &["dodge"]
                };
                let catalogue =
                    assign_charms(&mut states, &runtime, issuer, target, session, names);
                let definition = if mode == "agony" {
                    ConditionDefinition::new(
                        "fixture:incoming-agony",
                        1,
                        ConditionValues::Agony {
                            total_min: 300,
                            total_max: 300,
                            per_tick: 100,
                            interval_ms: 1000,
                            delayed: true,
                        },
                    )
                    .expect("agony")
                } else {
                    ConditionDefinition::new_damage_schedule(
                        "fixture:incoming-poison",
                        1,
                        DotElement::Poison,
                        DamageSchedule::Fixed {
                            delayed: true,
                            segments: vec![DamageSegment {
                                count: 3,
                                interval_ms: 1000,
                                amount: 100,
                            }],
                        },
                    )
                    .expect("poison")
                };
                let root = GameplayDecisionRoot::from_bytes(
                    runtime.content_pin().server_artifact_digest(),
                );
                let facts = ApplicationFacts {
                    now: 0,
                    base_speed: 220,
                    mana_shield_capacity: 0,
                    target_reentry_protected: false,
                    source_reentry_protected: false,
                    target_is_player: true,
                    decision_root: &root,
                    occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(
                        [24; 16],
                    ),
                };
                assert!(states.install_owned_source_condition_fixture(
                    &runtime,
                    target,
                    session,
                    issuer,
                    &[definition],
                    &facts
                ));
                let mut replacement = None;
                if mode == "dead" {
                    let health = runtime.companion_snapshot(issuer).expect("source").health;
                    runtime
                        .commit_source_damage(
                            issuer,
                            target,
                            session,
                            b"fixture:kill-source",
                            1,
                            u32::try_from(health).expect("HP"),
                            0,
                        )
                        .expect("kill source");
                }
                if mode == "retired" {
                    runtime.remove_test_actor(issuer).expect("retire");
                    let actor = runtime
                        .admit_source_pinned_lab_creature(
                            MovementLocalPosition {
                                x: 101,
                                y: 100,
                                floor: 7,
                            },
                            "oteryn:creature.1st_mate_ratticus",
                            200_000,
                        )
                        .expect("replacement");
                    qualify_hazard_monster(
                        &mut runtime,
                        actor,
                        "oteryn:creature.1st_mate_ratticus",
                    );
                    replacement = Some(actor);
                }
                let before = states
                    .get(&runtime, target, session)
                    .expect("player")
                    .vitals();
                let source_before = runtime
                    .companion_snapshot_including_dead(issuer)
                    .ok()
                    .map(|source| source.health);
                let tick_facts = TickFacts {
                    in_protection_zone: mode == "immune",
                    standing_on_field: None,
                };
                let first = states
                    .tick_source_player_conditions_with_charms(
                        &mut runtime,
                        target,
                        session,
                        tick_facts,
                        SemanticTimeMicros::from_micros(1_000_000),
                        Some(&catalogue),
                    )
                    .expect("real periodic owner");
                assert_eq!(first.ticks.len(), 1, "{mode}");
                if mode == "dodge" || mode == "immune" {
                    assert_eq!(first.damage, 0);
                    assert_eq!(
                        states
                            .get(&runtime, target, session)
                            .expect("player")
                            .vitals(),
                        before
                    );
                } else {
                    assert!(
                        first.damage > 0,
                        "{mode}: no living source and Agony must bypass Dodge"
                    );
                }
                if mode == "parry" {
                    assert!(
                        runtime
                            .companion_snapshot_including_dead(issuer)
                            .expect("source")
                            .health
                            < source_before.expect("source")
                    );
                    assert!(
                        states
                            .get(&runtime, target, session)
                            .expect("player")
                            .owned_conditions()
                            .speed_delta_at(1_000_000)
                            > 0
                    );
                }
                if let Some(actor) = replacement {
                    assert_eq!(
                        runtime
                            .companion_snapshot(actor)
                            .expect("replacement")
                            .health,
                        200_000
                    );
                }
                let retained = states
                    .owned_source_condition_fixture_snapshot(&runtime, target, session)
                    .expect("condition store");
                assert!(
                    states
                        .tick_source_player_conditions_with_charms(
                            &mut runtime,
                            target,
                            session,
                            tick_facts,
                            SemanticTimeMicros::from_micros(500_000),
                            Some(&catalogue)
                        )
                        .is_none()
                );
                assert_eq!(
                    states
                        .owned_source_condition_fixture_snapshot(&runtime, target, session)
                        .expect("store after refusal"),
                    retained
                );
                let duplicate = states
                    .tick_source_player_conditions_with_charms(
                        &mut runtime,
                        target,
                        session,
                        tick_facts,
                        SemanticTimeMicros::from_micros(1_000_000),
                        Some(&catalogue),
                    )
                    .expect("duplicate periodic turn");
                assert!(duplicate.ticks.is_empty());
                assert_eq!(duplicate.damage, 0);
                let second = states
                    .tick_source_player_conditions_with_charms(
                        &mut runtime,
                        target,
                        session,
                        tick_facts,
                        SemanticTimeMicros::from_micros(2_000_000),
                        Some(&catalogue),
                    )
                    .expect("next scheduled tick retained");
                assert_eq!(second.ticks.len(), 1, "{mode}: next tick survives a Dodge");
            }
        })
        .expect("thread")
        .join()
        .expect("scenario");
}

#[test]
fn incoming_order_explicit_native_mana_drain_skips_health_hook_and_retains_replay() {
    std::thread::Builder::new().name("incoming-mana-only".into()).stack_size(64 << 20).spawn(|| {
        let (mut runtime, mut states, source, proposal, mut world, fence, stamp) = setup_named_at("an_astral_glyph", 3,
            MovementLocalPosition { x: 100, y: 100, floor: 7 });
        let target = proposal.target;
        let issuer = proposal.issuer;
        let session = world.facts.attack.session;
        assert_eq!(source.element(), Some("mana_drain"));
        qualify_hazard_monster(&mut runtime, issuer, "oteryn:creature.an_astral_glyph");
        runtime.bind_attacker_lease(target, session, crate::foundation::CharacterLease::new(
            crate::foundation::CharacterId::decode(&test_uuid(0x72)).expect("character"), 1).expect("lease")).expect("principal");
        let catalogue = assign_charms(&mut states, &runtime, issuer, target, session, &["parry", "adrenaline_burst"]);
        let source_before = runtime.companion_snapshot(issuer).expect("source");
        let before = states.get(&runtime, target, session).expect("player").vitals();
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let mut attack = crate::gameplay_transport::attack::ChannelAttackStates::default();
        let mut owner = DamageSpellOwner::default();
        let result = owner.execute(&mut runtime, &fence, stamp,
            &mut crate::gameplay_transport::attack::charm_combat::source_vitals::CharmSourceVitals { states: &mut states, attack: &mut attack, catalogue: &catalogue, root },
            &source, &proposal, &mut world, SemanticTimeMicros::from_micros(2_000_000)).expect("native mana cast");
        let after = states.get(&runtime, target, session).expect("player").vitals();
        assert_eq!(after.health, before.health);
        assert!(after.mana < before.mana);
        assert_eq!(states.get(&runtime, target, session).expect("player").owned_conditions().speed_delta_at(2_000_000), 0);
        assert_eq!(runtime.companion_snapshot(issuer).expect("source"), source_before);
        let replay = states.with_source_charm_test(&mut runtime, issuer, target, session, "dodge", |runtime, vitals| {
            owner.execute(runtime, &fence, stamp, vitals, &source, &proposal, &mut world, SemanticTimeMicros::from_micros(9_000_000))
        }).expect("retained mana cast");
        assert_eq!(replay, result);
        assert_eq!(states.get(&runtime, target, session).expect("player").vitals(), after);
    }).expect("thread").join().expect("scenario");
}
