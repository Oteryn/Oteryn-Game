#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
use super::*;
use crate::foundation::{
    CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureExactRatio, CreatureFlags,
    CreatureResistance, MovementLocalPosition,
};
use crate::gameplay_transport::actor_spell::tests::FACTS;
use crate::spell::combat_batch::{AvatarState, OwnerCombatChange};
use crate::spell::native_combat::CombatPlan;

struct Fixture {
    runtime: ChannelRuntimeV1,
    caster: ExactActorRef,
    session: GameSessionId,
    state: PlayerSpellState,
    bindings: Vec<LiveActorBinding>,
    attributes: MagnitudeOwnedAttributes,
}
fn fixture() -> Fixture {
    let uuid = |tag| [1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag];
    let world = crate::foundation::WorldId::decode(&uuid(0x60)).expect("world");
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        crate::foundation::ChannelId::decode(&uuid(0x61)).expect("channel"),
        crate::foundation::NodeId::decode(&uuid(0x62)).expect("node"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        crate::foundation::ChannelContentPin::test(world),
    )
    .expect("bounded actual owner4 slots");
    let session = GameSessionId::decode(&uuid(32)).expect("session");
    let reservation = runtime
        .reserve_fresh_session(session)
        .expect("real reserve");
    let caster = runtime
        .commit_fresh_session(reservation)
        .expect("real actor");
    runtime
        .initialize_first_entry_position(caster)
        .expect("actual pinned player position");
    let mut state = PlayerSpellState::new(FACTS, 0, 0).expect("actual player owner data");
    super::super::actor_conditions::apply_combat_change(
        &mut state,
        &OwnerCombatChange::Avatar(AvatarState {
            expires_ms: 15100,
            outfit_look_type: 1595,
            incoming_reduction_percent: 15,
            critical_chance_percent: 100,
            critical_extra_percentage_points: 15,
        }),
        100,
    )
    .expect("real Avatar condition store");
    let policy = CompiledCreaturePolicy {
        definition_key: "fixture:qualified".into(),
        definition_revision: "fixture:1".into(),
        display_name: "fixture creature".into(),
        maximum_health: 20,
        base_speed: 110,
        outfit_look_type: 1,
        object_look_type: None,
        summonable: false,
        convinceable: false,
        mana_cost: None,
        is_familiar: false,
        preferred_distance: Some(4),
        reward_boss: Some(false),
        condition_immunities: vec![],
        armor: Some(10),
        mitigation: Some(CreatureExactRatio {
            numerator: 5,
            denominator: 1,
        }),
        resistances: vec![CreatureResistance {
            damage_type: "energy".into(),
            percent: CreatureExactRatio {
                numerator: 20,
                denominator: 1,
            },
        }],
        damage_immunities: vec![],
        flags: CreatureFlags {
            attackable: true,
            illusionable: false,
            health_hidden: false,
        },
    };
    runtime
        .install_companion_policies(
            CompiledCreaturePolicies::from_active_artifact(
                runtime.content_pin().server_artifact_digest(),
                vec![policy],
            )
            .expect("qualified table"),
        )
        .expect("matches actual active pin");
    let mut bindings = vec![LiveActorBinding {
        source_id: 1,
        actor: caster,
        target_atom: super::super::combat_execution::actor_atom(caster),
    }];
    for id in [2, 3] {
        let actor = runtime
            .admit_pinned_test_creature(MovementLocalPosition {
                x: 8 + id as i32,
                y: 9,
                floor: 7,
            })
            .expect("real creature slot");
        runtime
            .install_creature_policy(actor, "fixture:qualified")
            .expect("actual source policy binding");
        bindings.push(LiveActorBinding {
            source_id: id,
            actor,
            target_atom: "test:creature".into(),
        });
    }
    let attributes = MagnitudeOwnedAttributes {
        policy: MagnitudePolicy::Strict,
        owner_revisions: None,
        binding: MagnitudeOwnerBinding {
            actor: caster,
            session,
            character: CharacterId::decode(&[
                1, 0x90, 0, 0, 0, 42, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 42,
            ])
            .expect("character"),
            lease_generation: 1,
            player_revision: state.revision(),
            content_digest: runtime.content_pin().server_artifact_digest(),
        },
        base_critical_chance_permyriad: Some(500),
        base_critical_extra_permyriad: Some(5000),
        equipment_critical_chance_permyriad: Some(700),
        equipment_critical_extra_permyriad: Some(0),
        fatal_chance_permyriad: Some(ExactRatio {
            numerator: 3500,
            denominator: 1,
        }),
        armor_penetration_permyriad: Some(0),
        elemental_pierce_permyriad: Some(vec![
            (Element::Energy, 1000),
            (Element::Physical, 0),
            (Element::Fire, 0),
            (Element::Ice, 0),
            (Element::Earth, 0),
            (Element::Death, 0),
        ]),
        wheel_flat_damage: Some(4),
        wheel_flat_healing: Some(3),
        source_damage_multiplier_percent: Some(0),
        source_healing_multiplier_percent: Some(0),
        healing_dealt_percent: Some(100),
        monster_armor_disabled: Some(false),
        inactive_extra_stages: Some(vec![]),
    };
    Fixture {
        runtime,
        caster,
        session,
        state,
        bindings,
        attributes,
    }
}
fn plan() -> NativeCombatPlan {
    NativeCombatPlan::Combat(CombatPlan {
        element: Element::Energy,
        area: None,
        hits: vec![
            MagnitudePlan {
                target: 2,
                magnitude: 101,
                bonus_percent: 10,
                side_percent: Some(25),
                delay_ms: 0,
            },
            MagnitudePlan {
                target: 3,
                magnitude: 101,
                bonus_percent: 10,
                side_percent: Some(25),
                delay_ms: 0,
            },
        ],
        cooldown_ms: None,
        shared_cooldown: None,
        reduce_all_spell_cooldowns_ms: 0,
        block_armor: true,
        use_weapon_charges: false,
        weapon_missile: false,
        dispel_paralysis_targets: vec![],
        resolve_critical_and_fatal_once: true,
    })
}
fn qualify(f: &Fixture, p: &NativeCombatPlan) -> Result<PreparedMagnitudeOwner, Error> {
    PreparedMagnitudeOwner::qualify(
        &f.runtime,
        &f.state,
        f.caster,
        f.session,
        Some(&f.attributes),
        p,
        &f.bindings,
        &[],
        100,
    )
}

#[test]
fn content_test_baseline_omits_optional_stages_preserves_unknowns_and_durable_revisions() {
    let mut f = fixture();
    let p = plan();
    f.attributes.fatal_chance_permyriad = None;
    f.attributes.wheel_flat_damage = None;
    f.attributes.inactive_extra_stages = None;
    assert!(
        qualify(&f, &p).is_err(),
        "strict Content refuses unknown stages"
    );
    f.attributes.policy = MagnitudePolicy::BaselineTest;
    assert!(
        qualify(&f, &p).is_err(),
        "baseline still requires durable observations"
    );
    f.attributes.owner_revisions = Some((5, 3, Some(2)));
    let mut prepared = qualify(&f, &p).expect("explicit test Content policy");
    assert_eq!(prepared.attributes.fatal_chance_permyriad, None);
    assert_eq!(prepared.attributes.inactive_extra_stages, None);
    assert!(
        prepared
            .attributes
            .omitted_modifiers()
            .contains(&"fatal_chance")
    );
    assert!(prepared.attributes.omitted_modifiers().contains(&"charms"));
    let NativeCombatPlan::Combat(combat) = &p else {
        panic!("combat")
    };
    let mut draws = 0;
    let amount = prepared
        .finish(&p, &combat.hits[0], &f.bindings[1], &mut |low, _| {
            draws += 1;
            low
        })
        .expect("real creature magnitude with omitted fatal/Wheel stages");
    assert!(amount > 0);
    assert!(draws > 0);
    assert!(
        prepared
            .validate_current(&f.runtime, &f.state, &f.attributes, &[])
            .is_ok()
    );
    let mut changed = f.attributes.clone();
    changed.owner_revisions = Some((5, 4, Some(2)));
    assert_eq!(
        prepared.validate_current(&f.runtime, &f.state, &changed, &[]),
        Err(Error::SnapshotChanged)
    );
    changed = f.attributes.clone();
    changed.policy = MagnitudePolicy::Strict;
    assert_eq!(
        prepared.validate_current(&f.runtime, &f.state, &changed, &[]),
        Err(Error::SnapshotChanged)
    );
    f.attributes.inactive_extra_stages = Some(vec![ExtraStage::Charms]);
    assert!(
        qualify(&f, &p).is_err(),
        "known active unsupported stage still refuses"
    );
}

#[test]
fn test_baseline_does_not_grant_player_damage_or_missing_target_health_authority() {
    let mut f = fixture();
    f.attributes.policy = MagnitudePolicy::BaselineTest;
    f.attributes.owner_revisions = Some((1, 1, None));
    let mut p = plan();
    let NativeCombatPlan::Combat(combat) = &mut p else {
        panic!("combat")
    };
    combat.hits[0].target = 1;
    combat.hits.truncate(1);
    assert!(
        qualify(&f, &p).is_err(),
        "baseline cannot authorize damage against player caster"
    );
    let NativeCombatPlan::Combat(combat) = &mut p else {
        panic!("combat")
    };
    combat.hits[0].target = 99;
    assert!(
        qualify(&f, &p).is_err(),
        "unowned target remains unavailable"
    );
    f.attributes.binding.player_revision += 1;
    assert!(
        qualify(&f, &plan()).is_err(),
        "stale caster owner remains unavailable"
    );
}

#[test]
fn test_baseline_healing_uses_actual_player_owner_and_requires_current_content() {
    let mut f = fixture();
    f.attributes.policy = MagnitudePolicy::BaselineTest;
    f.attributes.owner_revisions = Some((1, 1, None));
    f.attributes.healing_dealt_percent = None;
    f.attributes.source_healing_multiplier_percent = None;
    let mut p = plan();
    let NativeCombatPlan::Combat(combat) = &mut p else {
        panic!("combat")
    };
    combat.element = Element::Healing;
    combat.hits[0].target = 1;
    combat.hits.truncate(1);
    let mut prepared = qualify(&f, &p).expect("real player owner for healing");
    let NativeCombatPlan::Combat(combat) = &p else {
        panic!("combat")
    };
    assert!(
        prepared
            .finish(&p, &combat.hits[0], &f.bindings[0], &mut |low, _| low)
            .expect("baseline healing magnitude")
            > 0
    );
    assert!(
        prepared
            .validate_current(&f.runtime, &f.state, &f.attributes, &[])
            .is_ok()
    );
    f.attributes.binding.content_digest = [0; 32];
    assert!(
        qualify(&f, &plan()).is_err(),
        "current Content identity remains mandatory"
    );
}

#[test]
fn real_avatar_policy_vector_preserves_source_order_and_one_beam_extension_draw() {
    let f = fixture();
    let p = plan();
    let mut prepared = qualify(&f, &p).expect("complete actual owner fields");
    let NativeCombatPlan::Combat(combat) = &p else {
        panic!("combat")
    };
    let mut calls = Vec::new();
    let mut draw = |low, high| {
        calls.push((low, high));
        low
    };
    for (hit, binding) in combat.hits.iter().zip(&f.bindings[1..]) {
        assert_eq!(prepared.finish(&p, hit, binding, &mut draw), Ok(58));
    }
    // Independent source receipt: 101*1.65 trunc166; fatal +round99.6=266;
    // side round66.5=67; armor -5=62; mitigation trunc58.9=58;
    // resistance20%-pierce10%=round52.2=52; signed Wheel trunc57.2=57;
    // side flat round(4*.25)=1; final58. Armor draws once per real target.
    assert_eq!(calls, vec![(1, 10000), (0, 10000), (5, 9), (5, 9)]);
    prepared
        .validate_current(&f.runtime, &f.state, &f.attributes, &[])
        .expect("actual owner unchanged");
}

#[test]
fn every_unknown_attribute_and_active_source_extra_refuses_before_rng() {
    let f = fixture();
    let p = plan();
    for index in 0..15 {
        let mut a = f.attributes.clone();
        match index {
            0 => a.base_critical_chance_permyriad = None,
            1 => a.base_critical_extra_permyriad = None,
            2 => a.equipment_critical_chance_permyriad = None,
            3 => a.equipment_critical_extra_permyriad = None,
            4 => a.fatal_chance_permyriad = None,
            5 => a.armor_penetration_permyriad = None,
            6 => a.elemental_pierce_permyriad = None,
            7 => a.wheel_flat_damage = None,
            8 => a.wheel_flat_healing = None,
            9 => a.source_damage_multiplier_percent = None,
            10 => a.source_healing_multiplier_percent = None,
            11 => a.healing_dealt_percent = None,
            12 => a.monster_armor_disabled = None,
            13 => a.inactive_extra_stages = None,
            _ => a.inactive_extra_stages = Some(vec![ExtraStage::Charms]),
        }
        assert!(
            PreparedMagnitudeOwner::qualify(
                &f.runtime,
                &f.state,
                f.caster,
                f.session,
                Some(&a),
                &p,
                &f.bindings,
                &[],
                100
            )
            .is_err(),
            "unknown index {index}"
        );
    }
    assert!(
        PreparedMagnitudeOwner::qualify(
            &f.runtime,
            &f.state,
            f.caster,
            f.session,
            None,
            &p,
            &f.bindings,
            &[],
            100
        )
        .is_err()
    );
}

#[test]
fn changed_actual_target_or_caster_condition_refuses_prepared_magnitude() {
    let mut f = fixture();
    let p = plan();
    let prepared = qualify(&f, &p).expect("qualified actual facts");
    f.runtime
        .borrow_exact_actor_commit()
        .commit_damage(
            f.bindings[1].actor,
            crate::foundation::OwnerDamageCommand {
                target: b"test:creature",
                occurrence: b"other",
                binding: b"other\0qualified",
                damage: 1,
            },
        )
        .expect("intervening physical write");
    assert_eq!(
        prepared.validate_current(&f.runtime, &f.state, &f.attributes, &[]),
        Err(Error::SnapshotChanged)
    );
    let f = fixture();
    let prepared = qualify(&f, &p).expect("qualified");
    let mut changed = f.state.clone();
    changed
        .conditions
        .remove_type(crate::ability::condition::ConditionType::Attributes);
    assert_eq!(
        prepared.validate_current(&f.runtime, &changed, &f.attributes, &[]),
        Err(Error::SnapshotChanged)
    );
}

#[test]
fn changed_plan_hit_and_forged_target_binding_refuse_without_drawing() {
    let f = fixture();
    let p = plan();
    let mut prepared = qualify(&f, &p).expect("qualified");
    let NativeCombatPlan::Combat(combat) = &p else {
        panic!("combat")
    };
    let mut changed = combat.hits[0].clone();
    changed.magnitude += 1;
    let mut draws = 0;
    let mut draw = |low, _| {
        draws += 1;
        low
    };
    assert!(
        prepared
            .finish(&p, &changed, &f.bindings[1], &mut draw)
            .is_err()
    );
    let mut forged = f.bindings[1].clone();
    forged.actor = f.bindings[2].actor;
    assert!(
        prepared
            .finish(&p, &combat.hits[0], &forged, &mut draw)
            .is_err()
    );
    assert_eq!(draws, 0);
}

#[test]
fn future_chain_bounce_and_player_damage_need_separate_actual_due_or_pvp_owner() {
    let f = fixture();
    let mut p = plan();
    if let NativeCombatPlan::Combat(combat) = &mut p {
        combat.hits[1].delay_ms = 200;
    }
    assert!(qualify(&f, &p).is_err());
    if let NativeCombatPlan::Combat(combat) = &mut p {
        combat.hits[1].delay_ms = 0;
        combat.hits[1].target = 1;
    }
    assert!(qualify(&f, &p).is_err());
}

#[test]
fn strict_fatal_endpoint_and_invalid_draw_preserve_source_rng_contract() {
    let mut f = fixture();
    f.attributes.fatal_chance_permyriad = Some(ExactRatio {
        numerator: 3500,
        denominator: 1,
    });
    let p = plan();
    let NativeCombatPlan::Combat(combat) = &p else {
        panic!("combat")
    };
    let mut q = qualify(&f, &p).expect("qualified");
    let mut draw = |low, high| if (low, high) == (0, 10000) { 3500 } else { low };
    // Fatal endpoint equality refuses fatal:166→side42→armor37→mit35→res32→Wheel35+1=36.
    assert_eq!(
        q.finish(&p, &combat.hits[0], &f.bindings[1], &mut draw),
        Ok(36)
    );
    let mut q = qualify(&f, &p).expect("qualified");
    assert_eq!(
        q.finish(&p, &combat.hits[0], &f.bindings[1], &mut |_, hi| hi + 1),
        Err(Error::InvalidMagnitude)
    );
}

#[test]
fn all_twenty_canonical_combat_profiles_lower_against_real_slots_and_qualified_magnitudes() {
    use super::super::{
        chain::TilePosition, combat_batch as batch, formula::FormulaInputs, native_combat as native,
    };
    use crate::foundation::{CommandId, CommandRef};
    use std::collections::BTreeSet;
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
    ))
    .expect("canonical profiles");
    let profiles = document["profiles"].as_array().expect("profiles");
    let mut count = 0;
    let mut deferred_refusals = 0;
    for profile in profiles {
        let behavior = &profile["execution"]["native_behavior"];
        let Some(key) = behavior["key"].as_str() else {
            continue;
        };
        if !matches!(
            key,
            "wheel_combat"
                | "avatar_state"
                | "monster_ai_override"
                | "mass_spirit_mend"
                | "mana_shield_capacity"
        ) {
            continue;
        }
        let f = fixture();
        let origin = TilePosition {
            x: 10,
            y: 10,
            floor: 7,
        };
        let mut targets = vec![native::NativeTargetFact {
            id: 1,
            position: origin,
            kind: native::TargetKind::Player,
            name: "actual caster".into(),
            reward_boss: false,
            preferred_distance: 1,
            health: f.state.vitals().health,
            maximum_health: f.state.facts.max_health,
            legal: true,
        }];
        for (index, binding) in f.bindings[1..].iter().enumerate() {
            let actual = f
                .runtime
                .companion_snapshot(binding.actor)
                .expect("same physical owner policy");
            targets.push(native::NativeTargetFact {
                id: binding.source_id,
                position: TilePosition {
                    x: actual.position.x,
                    y: actual.position.y,
                    floor: actual.position.floor,
                },
                kind: native::TargetKind::MasterlessMonster,
                name: actual.state.policy.display_name.clone(),
                reward_boss: false,
                preferred_distance: 4,
                health: actual.health as u32,
                maximum_health: actual.maximum_health as u32,
                legal: true,
            });
            assert_eq!(index + 2, binding.source_id as usize);
        }
        let mut clear_sight = BTreeSet::new();
        for y in -8..29 {
            for x in -8..29 {
                let tile = TilePosition { x, y, floor: 7 };
                clear_sight.insert((origin, tile));
                for target in &targets {
                    clear_sight.insert((target.position, tile));
                }
            }
        }
        // World legality/visibility in this component fixture is explicit. The production
        // resolver must read real permissions/tiles and never infer these values.
        let facts = native::NativeCombatFacts {
            now_ms: 100,
            stage: 1,
            inputs: FormulaInputs {
                level: f.state.facts.level,
                magic_level: f.state.facts.magic_level,
                base_power: None,
                attack_skill: 100,
                attack_value: 50,
                attack_factor: 1.0,
                shielding_skill: 0,
                shield_defense: None,
            },
            maximum_mana: f.state.facts.max_mana,
            caster: 1,
            caster_position: origin,
            area_origin: origin,
            area_sight_origin: origin,
            direction: native::Direction::North,
            explicit_target: Some(2),
            attacked_target: Some(2),
            targets,
            clear_sight,
            floor_change_tiles: BTreeSet::new(),
            elemental_stance: native::ElementalStance::None,
            incoming_hit: None,
        };
        let plan = native::plan(key, &behavior["parameters"], &facts, &mut |low, _| low)
            .expect("qualified canonical planner");
        let qualified = qualify(&f, &plan);
        if profile["name"]
            .as_str()
            .is_some_and(|name| name.eq_ignore_ascii_case("executioner's throw"))
        {
            assert!(
                qualified.is_err(),
                "chain bounce must qualify under due owner, not cast time"
            );
            deferred_refusals += 1;
            count += 1;
            continue;
        }
        let mut qualified = qualified.expect("actual policy/caster qualification");
        let template = batch::OwnerCombatBatch {
            caster: f.caster,
            attacker: f.attributes.binding.character,
            current_lease_generation: 1,
            command: CommandRef::new(f.session, CommandId::new(1).expect("command")),
            occurrence: batch::SpellOccurrenceBinding {
                id: format!("canonical:{}", profile["name"]),
                revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"]
                    .map(str::to_owned),
            },
            binding: serde_json::to_vec(profile).expect("immutable complete source binding"),
            anchor: None,
            now_ms: 100,
            effects: vec![],
            deferred: None,
        };
        let lowered = batch::lower_native(
            &f.runtime,
            template,
            1,
            &plan,
            &f.bindings,
            0,
            &mut |p, h, b| qualified.finish(p, h, b, &mut |low, _| low),
        )
        .expect("real actor binding lowerer");
        let mut ordinals: Vec<_> = lowered
            .batch
            .effects
            .iter()
            .map(|e| e.sub_ordinal)
            .chain(lowered.delayed.iter().map(|e| e.effect.sub_ordinal))
            .collect();
        ordinals.sort_unstable();
        assert!(
            ordinals.windows(2).all(|w| w[0] < w[1]),
            "globally unique effect ordinals"
        );
        qualified
            .validate_current(&f.runtime, &f.state, &f.attributes, &[])
            .expect("read-only lowering");
        count += 1;
    }
    assert_eq!((count, deferred_refusals), (20, 1));
}
