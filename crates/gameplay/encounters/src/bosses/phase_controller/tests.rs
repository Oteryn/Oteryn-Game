#![allow(
    clippy::unwrap_used,
    reason = "native fixture setup and assertions fail with their exact carrier error"
)]
use super::*;

#[test]
fn cults_requires_every_distinct_guardian_before_essence() {
    let plan = cults::plan();
    let mut progress = Progress::new(plan).unwrap();
    assert!(!progress.ready(10));
    for i in 0..5 {
        assert!(progress.ready(i));
        assert!(!progress.ready(i + 5));
        assert!(progress.record_death(i).unwrap());
        assert!(!progress.record_death(i).unwrap());
        assert!(progress.ready(i + 5));
        assert!(progress.record_death(i + 5).unwrap());
        assert_eq!(progress.ready(10), i == 4);
    }
    assert!(!progress.completed());
    assert!(progress.record_death(10).unwrap());
    assert!(progress.completed());
}

#[test]
fn premature_and_unknown_phase_events_do_not_mutate() {
    let mut progress = Progress::new(cults::plan()).unwrap();
    assert!(progress.record_death(5).is_err());
    assert!(progress.record_death(64).is_err());
    assert_eq!(progress.dead(), 0);
}

fn uuid(tag: u8) -> [u8; 16] {
    [1, 0, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
}

#[test]
fn native_cults_proves_deaths_and_retains_admitted_sessions() {
    use oteryn_gameplay_entities::carrier::ActorState;
    use oteryn_gameplay_entities::*;
    let world = WorldId::decode(&uuid(1)).unwrap();
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&uuid(2)).unwrap(),
        NodeId::decode(&uuid(3)).unwrap(),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        32,
        ChannelContentPin::test(world),
    )
    .unwrap();
    let session = GameSessionId::decode(&uuid(4)).unwrap();
    let reserved = runtime.reserve_fresh_session(session).unwrap();
    let player = runtime.commit_fresh_session(reserved).unwrap();
    let party = [(player, session)];
    let plan = cults::plan();
    let mut owner = NativePhaseOwner::bind(&runtime, plan, &party).unwrap();
    assert_eq!(owner.pending().count(), 5);
    assert!(owner.victors(&runtime).unwrap().is_none());
    for i in (0..5).flat_map(|i| [i, i + 5]).chain([10]) {
        let key = plan.roles[i].creature;
        let actor = ExactActorRef(
            runtime
                .admit_encounter_fixture_actor(
                    ActorState(0),
                    key,
                    100,
                    Some(oteryn_gameplay_entities::carrier::LocalPosition {
                        x: 100,
                        y: 100,
                        floor: 7,
                    }),
                )
                .unwrap(),
        );
        owner.attach(&runtime, i, actor).unwrap();
        assert!(owner.projected_death(&mut runtime, actor).is_err());
        runtime
            .borrow_exact_actor_commit()
            .commit_damage(
                actor,
                OwnerDamageCommand {
                    target: key.as_bytes(),
                    occurrence: b"phase-death",
                    binding: b"phase-death\0lethal",
                    damage: 100,
                },
            )
            .unwrap();
        // Zero HP alone is still not a published native death.
        assert!(owner.projected_death(&mut runtime, actor).is_err());
        let mut death = runtime.borrow_combat_death();
        let receipt = death.committed_lethal_receipt(actor).unwrap();
        death.project_committed_lethal(receipt).unwrap();
        assert!(owner.projected_death(&mut runtime, actor).unwrap());
        assert!(!owner.projected_death(&mut runtime, actor).unwrap());
    }
    assert_eq!(owner.victors(&runtime).unwrap(), Some(party.as_slice()));
    assert_eq!(owner.pending().count(), 0);
}

fn policy(key: &str) -> crate::content::creature_policy::CompiledCreaturePolicy {
    use crate::content::creature_policy::*;
    CompiledCreaturePolicy {
        definition_key: key.into(),
        definition_revision: "test-1".into(),
        display_name: key.into(),
        maximum_health: 100,
        forge_difficulty: None,
        base_speed: 220,
        outfit_look_type: 1,
        object_look_type: None,
        summonable: false,
        convinceable: false,
        mana_cost: None,
        is_familiar: false,
        condition_immunities: vec![],
        defense: Some(0),
        armor: Some(0),
        mitigation: None,
        resistances: vec![],
        damage_immunities: vec![],
        healing_from_damage: vec![],
        flags: CreatureFlags {
            attackable: true,
            illusionable: false,
            health_hidden: false,
        },
        preferred_distance: Some(1),
        reward_boss: Some(false),
        blocked_by_nearby_players: false,
    }
}

#[test]
fn cults_source_hook_replaces_stops_and_retries_refused_native_spawn() {
    use crate::content::creature_policy::CompiledCreaturePolicies;
    use oteryn_gameplay_entities::carrier::{ActorState, LocalPosition};
    use oteryn_gameplay_entities::*;
    let world = WorldId::decode(&uuid(1)).unwrap();
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&uuid(2)).unwrap(),
        NodeId::decode(&uuid(3)).unwrap(),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        64,
        ChannelContentPin::test(world),
    )
    .unwrap();
    let mut policies: Vec<_> = cults::plan()
        .roles
        .iter()
        .map(|role| policy(role.creature))
        .collect();
    policies.push(policy("oteryn:creature.destroyed_pillar"));
    runtime
        .install_companion_policies(
            CompiledCreaturePolicies::from_active_artifact(
                runtime.content_pin().server_artifact_digest(),
                policies,
            )
            .unwrap(),
        )
        .unwrap();
    let session = GameSessionId::decode(&uuid(4)).unwrap();
    let reserved = runtime.reserve_fresh_session(session).unwrap();
    let player = runtime.commit_fresh_session(reserved).unwrap();
    let party = [(player, session)];
    let mut pillars = Vec::new();
    let mut stops = Vec::new();
    let keys = cults::plan();
    for i in 0..5 {
        let x = 100 + i as i32 * 3;
        pillars.push(ExactActorRef(
            runtime
                .admit_encounter_fixture_actor(
                    ActorState(0),
                    keys.roles[i].creature,
                    100,
                    Some(LocalPosition {
                        x,
                        y: 100,
                        floor: 7,
                    }),
                )
                .unwrap(),
        ));
        let key = format!("{}_stop", keys.roles[i + 5].creature);
        stops.push(ExactActorRef(
            runtime
                .admit_encounter_fixture_actor(
                    ActorState(0),
                    &key,
                    100,
                    Some(LocalPosition {
                        x,
                        y: 101,
                        floor: 7,
                    }),
                )
                .unwrap(),
        ));
    }
    let pillars: [ExactActorRef; 5] = pillars.try_into().unwrap();
    let stops: [ExactActorRef; 5] = stops.try_into().unwrap();
    let essence_at = MovementLocalPosition {
        x: 120,
        y: 100,
        floor: 7,
    };
    let mut encounter =
        cults::CultsEncounter::bind(&mut runtime, &party, pillars, stops, essence_at).unwrap();
    let stop_key = format!("{}_stop", keys.roles[5].creature);
    assert!(
        runtime
            .borrow_exact_actor_commit()
            .commit_damage(
                stops[0],
                OwnerDamageCommand {
                    target: stop_key.as_bytes(),
                    occurrence: b"protected-stop",
                    binding: b"protected-stop\0lethal",
                    damage: 100,
                }
            )
            .is_err()
    );
    assert!(runtime.contains_live_creature(stops[0]));
    for i in 0..5 {
        lethal(&mut runtime, pillars[i], keys.roles[i].creature);
        assert!(encounter.projected_death(&mut runtime, pillars[i]).unwrap());
        assert!(encounter.advance(&mut runtime, |_, _| false).is_err());
        assert!(runtime.contains_live_creature(stops[i]));
        if i == 0 {
            // One freed stop slot can hold the guardian but not its decoration. The
            // failed second spawn must restore the stop and keep the phase pending.
            let mut fillers = Vec::new();
            while let Ok(actor) =
                runtime.admit_encounter_fixture_actor(ActorState(0), "oteryn:creature.rat", 1, None)
            {
                fillers.push(ExactActorRef(actor));
            }
            assert!(!fillers.is_empty());
            assert!(encounter.advance(&mut runtime, |_, _| true).is_err());
            assert!(runtime.contains_live_creature(stops[i]));
            for actor in fillers {
                runtime.remove_encounter_actor(actor).unwrap();
            }
        }
        encounter.advance(&mut runtime, |_, _| true).unwrap();
        assert!(!runtime.contains_live_creature(stops[i]));
        let guardian = runtime
            .positioned_melee_census()
            .unwrap()
            .iter()
            .find(|(a, _, _)| {
                runtime.matches_live_creature_identity(*a, keys.roles[i + 5].creature.as_bytes())
            })
            .map(|(a, _, _)| *a)
            .unwrap();
        lethal(&mut runtime, guardian, keys.roles[i + 5].creature);
        assert!(encounter.projected_death(&mut runtime, guardian).unwrap());
        assert!(!encounter.projected_death(&mut runtime, guardian).unwrap());
        encounter.advance(&mut runtime, |_, _| true).unwrap();
        assert!(encounter.victors(&runtime).unwrap().is_none());
        let essence = runtime
            .positioned_melee_census()
            .unwrap()
            .iter()
            .find(|(a, _, _)| {
                runtime.matches_live_creature_identity(*a, keys.roles[10].creature.as_bytes())
            })
            .map(|(a, _, _)| *a);
        assert_eq!(essence.is_some(), i == 4);
        if let Some(essence) = essence {
            lethal(&mut runtime, essence, keys.roles[10].creature);
            encounter.projected_death(&mut runtime, essence).unwrap();
        }
    }
    assert_eq!(encounter.victors(&runtime).unwrap(), Some(party.as_slice()));
}

fn lethal(runtime: &mut ChannelRuntimeV1, actor: ExactActorRef, key: &str) {
    use oteryn_gameplay_entities::OwnerDamageCommand;
    runtime
        .borrow_exact_actor_commit()
        .commit_damage(
            actor,
            OwnerDamageCommand {
                target: key.as_bytes(),
                occurrence: b"phase-death",
                binding: b"phase-death\0lethal",
                damage: 100,
            },
        )
        .unwrap();
    let mut death = runtime.borrow_combat_death();
    let receipt = death.committed_lethal_receipt(actor).unwrap();
    death.project_committed_lethal(receipt).unwrap();
}

#[test]
fn invalid_dependency_graphs_are_refused_before_any_native_binding() {
    const CYCLE: [Role; 1] = [Role {
        creature: "oteryn:creature.rat",
        after: 1,
    }];
    assert!(
        Progress::new(Plan {
            roles: &CYCLE,
            terminal: 1
        })
        .is_err()
    );
    assert!(
        Progress::new(Plan {
            roles: &[],
            terminal: 1
        })
        .is_err()
    );
    assert!(
        Progress::new(Plan {
            roles: cults::plan().roles,
            terminal: 1 << 63
        })
        .is_err()
    );
    const FULL: [Role; 64] = [Role {
        creature: "oteryn:creature.rat",
        after: 0,
    }; 64];
    let mut progress = Progress::new(Plan {
        roles: &FULL,
        terminal: 1 << 63,
    })
    .unwrap();
    assert!(!progress.ready(64));
    assert!(progress.record_death(63).unwrap());
    assert!(progress.completed());
}

#[test]
fn fugue_native_combat_closure_qualifies_every_source_attack_and_heal() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../content/creatures/definitions/spell-native-profiles.json");
    let document: crate::content::native_gameplay::CreatureProfilesDocument =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let row = document
        .records
        .iter()
        .find(|row| row.profile.target.key == "oteryn:creature.fugue")
        .expect("production Fugue body");
    let closure = row.attack_definitions.as_ref().expect("Fugue closure");
    let creature = &row.profile.target;
    crate::creature_auto_attack::MeleeSource::from_native(
        creature,
        0,
        &closure.records,
        &closure.profiles,
        [1; 32],
    )
    .expect("Fugue source melee");
    for index in 1..3 {
        crate::creature_damage_spell::SpellSource::from_native(
            creature,
            index,
            &closure.records,
            &closure.profiles,
            [1; 32],
        )
        .expect("Fugue source spell");
    }
    let heal = crate::self_heal_qualification::QualifiedSelfHealRules::qualify_native_parts(
        &closure.records,
        &closure.profiles,
        &creature.key,
        "oteryn:ability.creature.fugue.defense-1",
    )
    .expect("Fugue source healing");
    assert_eq!(heal.magnitude_range(), (200, 650));
    assert_eq!(heal.source_schedule(), (2000, 200000));
    let mut detached = closure.records.clone();
    detached.retain(|record| {
        !matches!(
            record,
            crate::content::ProjectReferenceRecord::Formula { .. }
        )
    });
    assert!(
        crate::self_heal_qualification::QualifiedSelfHealRules::qualify_native_parts(
            &detached,
            &closure.profiles,
            &creature.key,
            "oteryn:ability.creature.fugue.defense-1"
        )
        .is_err()
    );
}
