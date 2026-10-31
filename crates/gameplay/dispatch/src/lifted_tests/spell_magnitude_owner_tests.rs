#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
use crate::foundation::{ChannelRuntimeV1, CharacterId, ExactActorRef, GameSessionId};
use crate::gameplay_transport::actor_spell::tests::FACTS;
use crate::spell::cast::PlayerSpellState;
use crate::spell::combat_batch::{AvatarState, OwnerCombatChange};
use crate::spell::combat_batch::{Error, LiveActorBinding};
use crate::spell::magnitude_owner::*;
use crate::spell::native_combat::CombatPlan;
use crate::spell::native_combat::{Element, MagnitudePlan, NativeCombatPlan};
use {
    crate::foundation::MovementLocalPosition,
    oteryn_gameplay_content::content::creature_policy::{
        CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureExactRatio, CreatureFlags,
        CreatureResistance,
    },
};

#[path = "spell_magnitude_owner_tests/soulpit_hex.rs"]
mod soulpit_hex;

struct Fixture {
    runtime: ChannelRuntimeV1,
    caster: ExactActorRef,
    session: GameSessionId,
    state: PlayerSpellState,
    bindings: Vec<LiveActorBinding>,
    attributes: MagnitudeOwnedAttributes,
}
fn fixture() -> Fixture {
    fixture_with_damage_healing(0)
}
fn fixture_with_damage_healing(source_percent: i64) -> Fixture {
    fixture_with_capacity(source_percent, false)
}
fn fixture_with_capacity(source_percent: i64, extra_slots: bool) -> Fixture {
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
        if extra_slots { 16 } else { 4 },
        oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::test(world),
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
    crate::spell::actor_conditions::apply_combat_change(
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
        forge_difficulty: None,
        base_speed: 110,
        outfit_look_type: 1,
        object_look_type: None,
        summonable: false,
        convinceable: false,
        mana_cost: None,
        is_familiar: false,
        preferred_distance: Some(4),
        reward_boss: Some(false),
        blocked_by_nearby_players: false,
        condition_immunities: vec![],
        defense: Some(1),
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
        damage_immunities: if source_percent > 0 {
            vec!["energy".into()]
        } else {
            vec![]
        },
        healing_from_damage: if source_percent > 0 {
            vec![CreatureResistance {
                damage_type: "energy".into(),
                percent: CreatureExactRatio {
                    numerator: source_percent,
                    denominator: 1,
                },
            }]
        } else {
            vec![]
        },
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
        target_atom: crate::spell::combat_execution::actor_atom(caster),
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
        item_augments: Vec::new(),
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
        // Wheel/Atelier ownership is supplied separately by runtime effect tests.
        wheel_effects: None,
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
fn wheel_magnitude_known_fields_require_real_observed_wheel_revision_in_baseline() {
    let mut f = fixture();
    f.attributes.policy = MagnitudePolicy::BaselineTest;
    f.attributes.owner_revisions = Some((5, 3, None));
    assert!(
        qualify(&f, &plan()).is_err(),
        "known Wheel fields cannot use missing observation"
    );
    f.attributes.owner_revisions = Some((5, 3, Some(0)));
    assert!(
        qualify(&f, &plan()).is_err(),
        "revision zero is not a current proof"
    );
    f.attributes.owner_revisions = Some((5, 3, Some(2)));
    assert!(qualify(&f, &plan()).is_ok());
}

#[test]
fn wheel_magnitude_observed_flat_reaches_both_actual_damage_and_healing_finishers() {
    for element in [Element::Physical, Element::Healing] {
        let mut f = fixture();
        let mut p = plan();
        let NativeCombatPlan::Combat(combat) = &mut p else {
            panic!("combat")
        };
        combat.element = element;
        combat.hits.truncate(1);
        combat.hits[0].side_percent = None;
        if element == Element::Healing {
            combat.hits[0].target = 1;
        }
        let index = if element == Element::Healing { 0 } else { 1 };
        f.attributes.wheel_flat_damage = Some(0);
        f.attributes.wheel_flat_healing = Some(0);
        let mut base = qualify(&f, &p).expect("qualified baseline numeric owner");
        let NativeCombatPlan::Combat(combat) = &p else {
            panic!("combat")
        };
        let base_amount = base
            .finish(&p, &combat.hits[0], &f.bindings[index], &mut |_, high| high)
            .expect("base finish");
        f.attributes.wheel_flat_damage = Some(33);
        f.attributes.wheel_flat_healing = Some(33);
        let mut with_wheel = qualify(&f, &p).expect("current source Wheel numeric owner");
        let wheel_amount = with_wheel
            .finish(&p, &combat.hits[0], &f.bindings[index], &mut |_, high| high)
            .expect("Wheel finish");
        assert_eq!(
            wheel_amount - base_amount,
            33,
            "source flat effect {element:?}"
        );
        assert_eq!(
            base.validate_current(&f.runtime, &f.state, &f.attributes, &[]),
            Err(Error::SnapshotChanged)
        );
    }
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
    f.attributes.wheel_flat_damage = None;
    f.attributes.wheel_flat_healing = None;
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
    f.attributes.wheel_flat_damage = None;
    f.attributes.wheel_flat_healing = None;
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
        .remove_type(oteryn_gameplay_domain::domain::condition::ConditionType::Attributes);
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
    use crate::foundation::{CommandId, CommandRef};
    use crate::spell::{
        chain::TilePosition, combat_batch as batch, formula::FormulaInputs, native_combat as native,
    };
    use std::collections::BTreeSet;
    let document: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
    )))
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
            now_us: None,
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
            &mut |p, h, b| qualified.finish_with_damage_healing(p, h, b, &mut |low, _| low),
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
#[test]
fn source_damage_healing_100_300_precedes_immunity_caps_and_replays_on_real_owner() {
    use crate::foundation::{CommandId, CommandRef};
    use crate::spell::combat_batch::{OwnerCombatBatch, OwnerCombatEffect, SpellOccurrenceBinding};
    for percent in [100, 300] {
        let mut f = fixture_with_damage_healing(percent);
        let target = f.bindings[1].actor;
        let make_batch = |command, effects| OwnerCombatBatch {
            caster: f.caster,
            attacker: f.attributes.binding.character,
            current_lease_generation: 1,
            command: CommandRef::new(f.session, CommandId::new(command).expect("test command")),
            occurrence: SpellOccurrenceBinding {
                id: format!("source-healing:{percent}:{command}"),
                revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"]
                    .map(str::to_owned),
            },
            binding: format!("source-map:{percent}:{command}").into_bytes(),
            anchor: None,
            now_us: None,
            now_ms: 100,
            effects,
            deferred: None,
        };
        let wound = make_batch(
            1,
            vec![OwnerCombatEffect {
                target,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: f.bindings[1].target_atom.clone(),
                    magnitude: 15,
                },
            }],
        );
        let staged = f
            .runtime
            .stage_spell_batch(&wound)
            .expect("actual wound preflight");
        f.runtime
            .commit_spell_batch(staged)
            .expect("actual HP wound");
        let mut p = plan();
        let NativeCombatPlan::Combat(combat) = &mut p else {
            panic!("test combat");
        };
        combat.hits.truncate(1);
        combat.hits[0].magnitude = 20;
        combat.hits[0].side_percent = None;
        combat.block_armor = false;
        let mut q = qualify(&f, &p).expect("actual current native policy pin");
        let lowered = crate::spell::combat_batch::lower_native(
            &f.runtime,
            make_batch(2, vec![]),
            1,
            &p,
            &f.bindings,
            0,
            &mut |p, h, b| q.finish_with_damage_healing(p, h, b, &mut |low, _| low),
        )
        .expect("source map lowering");
        let [effect] = lowered.batch.effects.as_slice() else {
            panic!("one source occurrence");
        };
        let OwnerCombatChange::DamageWithHealing {
            damage, healing, ..
        } = effect.change
        else {
            panic!("typed composite");
        };
        assert_eq!(
            damage, 0,
            "source damage immunity still blocks residual damage"
        );
        assert!(
            healing >= 20 * percent / 100,
            "map is computed before immunity from source hit"
        );
        let staged = f
            .runtime
            .stage_spell_batch(&lowered.batch)
            .expect("real stage");
        let receipt = f
            .runtime
            .commit_spell_batch(staged)
            .expect("real composite commit");
        let hp = receipt.effects[0]
            .health
            .expect("physical owner HP receipt");
        assert_eq!(
            (hp.health_before, hp.health_after),
            (5, 20),
            "source-qualified maxHP clamp"
        );
        let replay = f
            .runtime
            .stage_spell_batch(&lowered.batch)
            .expect("exact receipt retry");
        assert!(
            !f.runtime
                .commit_spell_batch(replay)
                .expect("replay")
                .applied
        );
        assert_eq!(
            f.runtime
                .companion_snapshot(target)
                .expect("current actual target")
                .health,
            20
        );
        let mut forged = lowered.batch.clone();
        if let OwnerCombatChange::DamageWithHealing { healing, .. } = &mut forged.effects[0].change
        {
            *healing += 1;
        }
        assert!(
            f.runtime.stage_spell_batch(&forged).is_err(),
            "same occurrence changed map is not replay"
        );
    }
}

#[test]
fn source_damage_healing_uses_current_policy_and_stale_hp_fails_before_any_heal() {
    let mut f = fixture_with_damage_healing(100);
    let p = plan();
    let q = qualify(&f, &p).expect("source snapshot");
    let actor = f.bindings[1].actor;
    let before = f.runtime.companion_snapshot(actor).expect("current actor");
    let mut next = before.state.clone();
    next.party_protection = !next.party_protection;
    f.runtime
        .compare_companion_state(&before, next)
        .expect("actual independent owner change");
    assert!(
        q.validate_current(&f.runtime, &f.state, &f.attributes, &[])
            .is_err(),
        "source scope/policy/body snapshot remains fenced"
    );
    assert_eq!(
        f.runtime
            .companion_snapshot(actor)
            .expect("HP unchanged")
            .health,
        20
    );
    let policy = f
        .runtime
        .companion_snapshot(actor)
        .expect("policy")
        .state
        .policy;
    assert_eq!(policy.damage_healing("energy", 1), Ok(1));
    assert_eq!(
        policy.damage_healing("fire", 1),
        Ok(0),
        "unknown element has no fabricated response"
    );
    assert!(policy.damage_healing("energy", -1).is_err());
}

#[test]
fn item_spell_augments_named_base_healing_critical_and_unchanged_owner_binding() {
    use oteryn_gameplay_content::content::item_spell_augments::{
        ItemSpellAugment, ItemSpellAugmentKind as K,
    };
    let mut f = fixture();
    f.state = PlayerSpellState::new(FACTS, 0, 0).expect("player");
    f.attributes.binding.player_revision = f.state.revision();
    f.attributes.base_critical_chance_permyriad = Some(0);
    f.attributes.base_critical_extra_permyriad = Some(0);
    f.attributes.equipment_critical_chance_permyriad = Some(0);
    f.attributes.equipment_critical_extra_permyriad = Some(0);
    f.attributes.fatal_chance_permyriad = Some(ExactRatio {
        numerator: 0,
        denominator: 1,
    });
    f.attributes.item_augments = vec![
        ItemSpellAugment {
            spell: "light healing".into(),
            kind: K::BaseHealing,
            value: 400,
        },
        ItemSpellAugment {
            spell: "other spell".into(),
            kind: K::BaseHealing,
            value: 100000,
        },
        ItemSpellAugment {
            spell: "light healing".into(),
            kind: K::BaseDamage,
            value: 400,
        },
        ItemSpellAugment {
            spell: "light healing".into(),
            kind: K::CriticalHitChance,
            value: 10000,
        },
        ItemSpellAugment {
            spell: "light healing".into(),
            kind: K::CriticalExtraDamage,
            value: 5000,
        },
    ];
    let catalog: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
    ))
    .expect("catalog");
    let row = catalog["bundles"]
        .as_array()
        .expect("bundles")
        .iter()
        .find(|row| row["bundle"]["spell"]["identity"]["key"] == "candidate:spell/light_healing")
        .expect("source spell");
    let mut spell = oteryn_gameplay_content::content::spell_data::authoring::spell_from_bundle(
        &row["bundle"],
        &row["dependencies"],
    )
    .expect("spell");
    let mut p = plan();
    let NativeCombatPlan::Combat(combat) = &mut p else {
        panic!("combat");
    };
    combat.element = Element::Healing;
    combat.block_armor = false;
    combat.hits.truncate(1);
    combat.hits[0].bonus_percent = 0;
    combat.hits[0].side_percent = None;
    let hit = combat.hits[0].clone();
    let mut owner = qualify(&f, &p).expect("current owner");
    owner.select_item_spell(&spell).expect("selected spell");
    assert_eq!(
        owner.finish(&p, &hit, &f.bindings[1], &mut |_, _| panic!(
            "healing has no critical roll"
        )),
        Ok(108)
    );
    owner
        .validate_current(&f.runtime, &f.state, &f.attributes, &[])
        .expect("unchanged original facts");
    spell.name = "unrelated spell".into();
    owner.select_item_spell(&spell).expect("different name");
    assert_eq!(
        owner.finish(&p, &hit, &f.bindings[1], &mut |_, _| panic!("no roll")),
        Ok(104)
    );
    let NativeCombatPlan::Combat(combat) = &mut p else {
        panic!("combat");
    };
    combat.element = Element::Physical;
    let mut damage_owner = qualify(&f, &p).expect("damage owner");
    spell.name = "Light Healing".into();
    damage_owner
        .select_item_spell(&spell)
        .expect("exact case-folded match");
    assert_eq!(
        damage_owner.finish(&p, &hit, &f.bindings[1], &mut |_, max| max),
        Ok(153)
    );
    // 101 + trunc(4%) = 105; critical +50% = 157; mitigation 5% = 149; Wheel flat +4.
    let mut changed = f.attributes.clone();
    changed.item_augments.clear();
    assert!(
        owner
            .validate_current(&f.runtime, &f.state, &changed, &[])
            .is_err()
    );
}

fn wheel_healing_spell() -> crate::spell::SpellDefinition {
    let catalog: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
    ))
    .expect("source spell catalogue");
    let row = catalog["bundles"]
        .as_array()
        .expect("bundles")
        .iter()
        .find(|row| row["bundle"]["spell"]["identity"]["key"] == "candidate:spell/light_healing")
        .expect("light healing");
    oteryn_gameplay_content::content::spell_data::authoring::spell_from_bundle(
        &row["bundle"],
        &row["dependencies"],
    )
    .expect("qualified spell")
}

#[test]
fn wheel_battle_healing_uses_real_shield_and_only_selected_spell_healing() {
    use crate::spell::cast::{WheelEquipment, WheelHand};
    let mut f = fixture();
    let mut effects = oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects::default();
    effects.active_unique_perks.insert("battle_healing".into());
    f.attributes.wheel_effects = Some(effects);
    let mut p = plan();
    let NativeCombatPlan::Combat(combat) = &mut p else {
        panic!("combat");
    };
    combat.element = Element::Healing;
    combat.block_armor = false;
    combat.hits.truncate(1);
    combat.hits[0].target = 1;
    combat.hits[0].bonus_percent = 0;
    combat.hits[0].side_percent = None;
    let hit = combat.hits[0].clone();
    let spell = wheel_healing_spell();
    for (real_shield, expected) in [(false, 114), (true, 135)] {
        f.state.install_wheel_equipment(Some(WheelEquipment {
            shield_hand: Some(WheelHand {
                real_shield,
                secondary_shield: !real_shield,
                ..Default::default()
            }),
            ..Default::default()
        }));
        let mut owner = qualify(&f, &p).expect("actual owner snapshot");
        assert_eq!(
            owner.finish(&p, &hit, &f.bindings[0], &mut |_, _| panic!(
                "no critical roll"
            )),
            Ok(104),
            "unnamed heal is excluded"
        );
        owner.select_item_spell(&spell).expect("spell origin");
        assert_eq!(
            owner.finish(&p, &hit, &f.bindings[0], &mut |_, _| panic!(
                "no critical roll"
            )),
            Ok(expected)
        );
    }
    f.state.install_wheel_equipment(None);
    let mut owner = qualify(&f, &p).expect("after removal");
    owner.select_item_spell(&spell).expect("spell origin");
    assert_eq!(
        owner.finish(&p, &hit, &f.bindings[0], &mut |_, _| panic!(
            "no critical roll"
        )),
        Ok(114)
    );
}

#[test]
fn wheel_grove_healing_critical_is_after_flat_and_never_rolls_fatal() {
    use oteryn_gameplay_progression::wheel::{WheelAllocation, WheelSlots};
    use oteryn_gameplay_progression::wheel_gem_data::WheelGemData;
    let data = WheelGemData::embedded().expect("source Wheel");
    let mut points = [0; 36];
    let mut remaining = 500_u16;
    for (index, slot) in data.wheel()["topology"]
        .as_array()
        .expect("topology")
        .iter()
        .enumerate()
    {
        if slot["domain"] == "red" {
            let amount = u16::try_from(slot["capacity"].as_u64().expect("capacity"))
                .expect("bounded")
                .min(remaining);
            points[index] = amount;
            remaining -= amount;
        }
    }
    assert_eq!(remaining, 0);
    let allocation = WheelAllocation {
        wheel_ruleset_revision: Some(data.revision().into()),
        wheel_revision: 1,
        slots: WheelSlots::new(points).expect("bounded slots"),
        current: true,
    };
    let effects = oteryn_gameplay_progression::wheel_effects::derive_runtime_effects(
        data.wheel(),
        &allocation,
        "druid",
        550,
        true,
        true,
    )
    .expect("actual stage two numeric projection");
    assert_eq!(
        effects.revelation_value_milli("blessing_of_the_grove", "healing_bonus"),
        7_500
    );
    let mut f = fixture();
    f.state.conditions.clear_on_death();
    f.attributes.wheel_flat_healing =
        Some(i32::try_from(effects.flat_healing_bonus().expect("flat")).expect("bounded"));
    f.attributes.wheel_effects = Some(effects);
    let mut p = plan();
    let NativeCombatPlan::Combat(combat) = &mut p else {
        panic!("combat");
    };
    combat.element = Element::Healing;
    combat.block_armor = false;
    combat.hits.truncate(1);
    combat.hits[0].target = 1;
    combat.hits[0].magnitude = 100;
    combat.hits[0].bonus_percent = 0;
    combat.hits[0].side_percent = None;
    let hit = combat.hits[0].clone();
    for (roll, expected) in [(12, 163), (13, 109)] {
        let mut owner = qualify(&f, &p).expect("current owner");
        owner
            .select_item_spell(&wheel_healing_spell())
            .expect("real spell");
        let mut draws = 0;
        assert_eq!(
            owner.finish(&p, &hit, &f.bindings[0], &mut |low, high| {
                assert_eq!((low, high), (1, 100));
                draws += 1;
                roll
            }),
            Ok(expected)
        );
        assert_eq!(draws, 1, "Grove rolls critical only, per target");
    }
}

#[test]
fn wheel_healing_link_uses_uncapped_parent_heal_and_one_atomic_echo() {
    use crate::foundation::{CommandId, CommandRef};
    use crate::spell::combat_batch as batch;
    let mut f = fixture();
    let mut effects = oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects::default();
    effects.active_unique_perks.insert("healing_link".into());
    f.attributes.wheel_effects = Some(effects);
    let mut p = plan();
    let NativeCombatPlan::Combat(combat) = &mut p else {
        panic!("combat");
    };
    combat.element = Element::Healing;
    combat.block_armor = false;
    combat.hits.truncate(1);
    combat.hits[0].bonus_percent = 0;
    combat.hits[0].side_percent = None;
    let mut spell = wheel_healing_spell();
    spell.name = "Heal Friend".into();
    let mut owner = qualify(&f, &p).expect("current actual owner");
    owner
        .select_item_spell(&spell)
        .expect("source-selected name");
    let template = batch::OwnerCombatBatch {
        caster: f.caster,
        attacker: f.attributes.binding.character,
        current_lease_generation: 1,
        command: CommandRef::new(f.session, CommandId::new(7001).expect("command")),
        occurrence: batch::SpellOccurrenceBinding {
            id: "wheel-healing-link".into(),
            revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"].map(str::to_owned),
        },
        binding: vec![1],
        anchor: None,
        now_us: None,
        now_ms: 100,
        effects: vec![],
        deferred: None,
    };
    let mut lowered = batch::lower_native(
        &f.runtime,
        template,
        1,
        &p,
        &f.bindings,
        0,
        &mut |plan, hit, binding| {
            owner.finish_with_damage_healing(plan, hit, binding, &mut |_, _| panic!("no critical"))
        },
    )
    .expect("same owner batch");
    owner
        .append_wheel_healing_echo(&mut lowered)
        .expect("one echo");
    assert_eq!(lowered.batch.effects.len(), 2);
    let batch::OwnerCombatChange::Heal { magnitude, .. } = lowered.batch.effects[0].change else {
        panic!("parent heal");
    };
    assert_eq!(
        magnitude, 104,
        "raw101 + flat3; target maximum20 does not change the link basis"
    );
    let echo = &lowered.batch.effects[1];
    assert_eq!(echo.target, f.caster);
    let batch::OwnerCombatChange::Heal { magnitude, .. } = echo.change else {
        panic!("caster echo");
    };
    assert_eq!(
        magnitude, 29,
        "floor(104*25%) + flat3 on the recursive unnamed heal"
    );
    owner
        .append_wheel_healing_echo(&mut lowered)
        .expect("already drained");
    assert_eq!(
        lowered.batch.effects.len(),
        2,
        "lowering a retained result cannot append it twice"
    );
    owner
        .validate_current(&f.runtime, &f.state, &f.attributes, &[])
        .expect("all predecessor facts remain intact");
}

#[test]
fn positional_formula_census_is_revalidated_before_the_same_owner_commits() {
    let mut f = fixture();
    let mut effects = oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects::default();
    effects
        .active_unique_perks
        .insert("positional_tactics".into());
    f.attributes.wheel_effects = Some(effects);
    let prepared = qualify(&f, &plan()).expect("current census");
    prepared
        .validate_current(&f.runtime, &f.state, &f.attributes, &[])
        .expect("unchanged census");
    let session =
        GameSessionId::decode(&[1, 0x90, 0, 0, 0, 91, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 91])
            .expect("session");
    let reservation = f
        .runtime
        .reserve_fresh_session(session)
        .expect("free fourth actor slot");
    let actor = f
        .runtime
        .commit_fresh_session(reservation)
        .expect("new actor");
    f.runtime
        .initialize_first_entry_position(actor)
        .expect("positioned actor");
    assert_eq!(
        prepared.validate_current(&f.runtime, &f.state, &f.attributes, &[]),
        Err(Error::SnapshotChanged)
    );
}

#[test]
fn runic_uses_current_projection_and_same_name_conjurer_without_resource_admission() {
    use crate::spell::cast::{CharacterCastFacts, prepare_ordinary_owner_cast_with_wheel};
    use crate::spell::formula::{Expression, Formula, Input};
    use crate::spell::harmony::HarmonyMultiplier;
    use crate::spell::owned_cast_facts::{
        AccessProjections, CastFactsBinding, CurrentProjection, OwnedCastFacts,
    };
    use crate::spell::wheel_formula::WheelFormulaMagic;
    use crate::spell::{
        Carrier, CasterPremium, CasterState, Execution, ManaCost, ResolvedEffect, SpellBook,
        Vocation,
    };
    use oteryn_simulation_determinism::SemanticTimeMicros;
    let f = fixture();
    let mut state = PlayerSpellState::new(
        CharacterCastFacts {
            vocation: Vocation::Druid,
            level: 500,
            magic_level: 101,
            max_health: 1000,
            max_mana: 1000,
            max_soul: 200,
        },
        0,
        0,
    )
    .expect("caster");
    state
        .apply_owner_promotion_transition(true, true, Some(10_000_000))
        .expect("premium");
    let mut effects = oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects::default();
    effects.active_unique_perks.insert("runic_mastery".into());
    state.install_wheel_effects(effects.clone());
    let binding = CastFactsBinding {
        actor: f.caster,
        session: f.session,
        character: [7; 16],
        character_revision: 4,
        lease_generation: 1,
        connection_generation: 1,
        player_revision: state.revision(),
        content_digest: f.runtime.content_pin().server_artifact_digest(),
        equipment_revision: 2,
    };
    let equipment = crate::durability::character_equipment::EquipmentSnapshot {
        character: binding.character,
        content_digest: binding.content_digest,
        character_revision: 4,
        revision: 2,
        combat_mode: None,
        items: vec![],
    };
    let mut owned = OwnedCastFacts::from_owner_reads(
        binding.clone(),
        crate::durability::character_build::DurableBuildState::new("druid", (101, 0), [(10, 0); 7])
            .expect("build"),
        500,
        equipment,
        AccessProjections {
            premium: Some(CurrentProjection {
                binding: binding.clone(),
                authority_revision: 1,
                valid_until_micros: 10_000_000,
                value: true,
            }),
            wheel_promoted: true,
            wheel_effects: Some(CurrentProjection {
                binding: binding.clone(),
                authority_revision: 4,
                valid_until_micros: 10_000_000,
                value: effects,
            }),
            ..Default::default()
        },
    )
    .expect("fenced read");
    let caster = CasterState {
        vocation: Vocation::Druid,
        level: 500,
        magic_level: 101,
        premium: CasterPremium::from_current(true),
        mana: 1000,
        max_mana: 1000,
        soul: 200,
        learned: Default::default(),
        attack_skill: 0,
        attack_value: 0,
        attack_factor: 0.0,
        shielding_skill: 0,
        melee_weapon: false,
        shield_defense: None,
        harmony_multiplier: HarmonyMultiplier::ONE,
    };
    let mut rune = wheel_healing_spell();
    rune.authored = None;
    rune.key = "candidate:spell/source_test_rune".into();
    rune.name = "Source Test Rune".into();
    rune.carrier = Carrier::Rune {
        item: 3155,
        charges: 1,
        magic_level: 0,
    };
    rune.level = 1;
    rune.mana = ManaCost::Fixed(0);
    rune.soul = 0;
    rune.vocations = [Vocation::Druid].into_iter().collect();
    rune.learning_required = false;
    rune.execution = Execution::Effects(vec![crate::spell::SpellEffect::Heal {
        formula: Formula {
            minimum: Expression::Var(Input::MagicLevel),
            maximum: Expression::Var(Input::MagicLevel),
        },
    }]);
    let mut conjurer = rune.clone();
    conjurer.key = "candidate:spell/source_test_conjurer".into();
    conjurer.name = rune.name.to_ascii_lowercase(); // Crystal's getInstantSpellByName uses strcasecmp.
    conjurer.carrier = Carrier::Instant {
        words: "source test".into(),
        takes_parameter: false,
    };
    conjurer.level = 9999;
    conjurer.mana = ManaCost::Fixed(u32::MAX);
    conjurer.premium = true;
    let operational = crate::spell::OperationalCastFacts {
        caster_position: crate::spell::chain::TilePosition {
            x: 1,
            y: 1,
            floor: 7,
        },
        target_position: None,
        target: None,
        line_of_sight_clear: None,
        direction_available: false,
        wheel_unlocked: None,
        in_protection_zone: false,
        target_tile_solid: None,
        target_tile_creature: None,
    };
    for (can_conjure, expected) in [(true, 121), (false, 111)] {
        conjurer.vocations = [if can_conjure {
            Vocation::Druid
        } else {
            Vocation::Sorcerer
        }]
        .into_iter()
        .collect();
        let book =
            SpellBook::canonical(vec![rune.clone(), conjurer.clone()]).expect("distinct carriers");
        let wheel =
            WheelFormulaMagic::from_owners(&owned, &f.runtime, &rune, &book, &caster, &f.state, 1)
                .expect("actual current owner");
        let paid = prepare_ordinary_owner_cast_with_wheel(
            &book,
            &state,
            &rune,
            &operational,
            SemanticTimeMicros::from_micros(1),
            &caster,
            None,
            None,
            &mut |low, _| low,
            wheel,
        )
        .expect("paid rune");
        assert_eq!(
            paid.resolution.effects,
            vec![ResolvedEffect::Heal {
                magnitude: expected
            }]
        );
        assert_eq!(state.character_facts().magic_level, 101);
    }
    let no_conjurer = SpellBook::canonical(vec![rune.clone()]).expect("one rune");
    assert_eq!(
        WheelFormulaMagic::from_owners(
            &owned,
            &f.runtime,
            &rune,
            &no_conjurer,
            &caster,
            &f.state,
            1
        )
        .expect("known absence"),
        WheelFormulaMagic::default()
    );
    let book = SpellBook::canonical(vec![rune.clone(), conjurer]).expect("book");
    assert_eq!(
        WheelFormulaMagic::from_owners(
            &owned, &f.runtime, &rune, &book, &caster, &f.state, 10_000_000
        )
        .expect("expired"),
        WheelFormulaMagic::default()
    );
    owned.access.wheel_promoted = false;
    assert_eq!(
        WheelFormulaMagic::from_owners(&owned, &f.runtime, &rune, &book, &caster, &f.state, 1)
            .expect("unpromoted"),
        WheelFormulaMagic::default()
    );
}

#[test]
fn battle_instinct_reads_current_incoming_actor_census_and_closes_on_expiry() {
    use crate::spell::{Vocation, cast::CharacterCastFacts};
    let mut f = fixture_with_capacity(0, true);
    let mut state = PlayerSpellState::new(
        CharacterCastFacts {
            vocation: Vocation::Knight,
            level: 500,
            magic_level: 20,
            max_health: 1000,
            max_mana: 500,
            max_soul: 200,
        },
        0,
        0,
    )
    .expect("player");
    let mut effects = oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects::default();
    effects.active_unique_perks.insert("battle_instinct".into());
    state.install_wheel_effects(effects);
    state
        .apply_owner_promotion_transition(true, true, Some(100))
        .expect("current owner Premium");
    let at = f
        .runtime
        .read_actor_position(f.caster)
        .expect("position")
        .position();
    let initial =
        crate::spell::cast::wheel_adjacent_monsters(&f.runtime, f.caster).expect("census");
    assert!(initial < 5);
    assert_eq!(
        state
            .wheel_battle_instinct(&f.runtime, f.caster, 99)
            .expect("read")
            .shielding,
        0
    );
    let destination = MovementLocalPosition {
        x: at.x + 20,
        y: at.y,
        floor: at.floor,
    };
    for index in 0..5 {
        let actor = f
            .runtime
            .admit_pinned_test_creature(MovementLocalPosition {
                x: destination.x + index % 3 - 1,
                y: destination.y + index / 3 - 1,
                floor: destination.floor,
            })
            .expect("destination creature");
        f.runtime
            .install_creature_policy(actor, "fixture:qualified")
            .expect("destination source policy");
    }
    assert_eq!(
        state
            .wheel_battle_instinct_at(&f.runtime, f.caster, destination, 99)
            .expect("qualified field destination")
            .shielding,
        6
    );
    assert_eq!(
        state
            .wheel_battle_instinct(&f.runtime, f.caster, 99)
            .expect("actor has not moved")
            .shielding,
        0
    );
    assert_eq!(
        f.runtime
            .read_actor_position(f.caster)
            .expect("same owner position")
            .position(),
        at
    );
    for index in initial..5 {
        let actor = f
            .runtime
            .admit_pinned_test_creature(MovementLocalPosition {
                x: at.x + (index % 3) as i32 - 1,
                y: at.y + (index / 3) as i32 - 1,
                floor: at.floor,
            })
            .expect("adjacent live creature");
        f.runtime
            .install_creature_policy(actor, "fixture:qualified")
            .expect("source policy");
    }
    let bonus = state
        .wheel_battle_instinct(&f.runtime, f.caster, 99)
        .expect("current census");
    assert_eq!((bonus.melee, bonus.shielding), (1, 6));
    assert_eq!(
        state
            .wheel_battle_instinct(&f.runtime, f.caster, 100)
            .expect("expired owner")
            .shielding,
        0
    );
    state.install_wheel_effects(Default::default());
    assert_eq!(
        state
            .wheel_battle_instinct(&f.runtime, f.caster, 99)
            .expect("removed perk")
            .shielding,
        0
    );
}

#[test]
fn active_gem_values_keep_global_and_named_spell_critical_and_damage_separate() {
    let mut f = fixture();
    f.state.conditions.clear_on_death();
    f.attributes.base_critical_chance_permyriad = Some(10_000);
    f.attributes.equipment_critical_chance_permyriad = Some(0);
    f.attributes.base_critical_extra_permyriad = Some(0);
    f.attributes.equipment_critical_extra_permyriad = Some(0);
    f.attributes.fatal_chance_permyriad = Some(ExactRatio {
        numerator: 0,
        denominator: 1,
    });
    f.attributes.wheel_flat_damage = Some(0);
    let mut spell = wheel_healing_spell();
    spell.name = "Executioner's Throw".into();
    let mut effects = oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects::default();
    effects.sockets.gem_values_milli.extend([
        (
            (
                "critical_extra_damage".into(),
                "Critical Extra Damage".into(),
            ),
            2_200,
        ),
        (("critical_extra_damage".into(), spell.name.clone()), 12_000),
        (("base_damage_bonus".into(), spell.name.clone()), 10_000),
        (
            ("critical_extra_damage".into(), "Unrelated Spell".into()),
            80_000,
        ),
    ]);
    f.attributes.wheel_effects = Some(effects);
    let mut p = plan();
    let NativeCombatPlan::Combat(combat) = &mut p else {
        panic!("combat");
    };
    combat.element = Element::Physical;
    combat.block_armor = false;
    combat.hits.truncate(1);
    combat.hits[0].magnitude = 100;
    combat.hits[0].bonus_percent = 0;
    combat.hits[0].side_percent = None;
    let hit = combat.hits[0].clone();
    let mut owner = qualify(&f, &p).expect("source owner");
    owner.select_item_spell(&spell).expect("named cast");
    assert_eq!(
        owner.finish(&p, &hit, &f.bindings[1], &mut |low, high| {
            assert_eq!((low, high), (1, 10_000));
            low
        }),
        Ok(118),
        "14.2% critical then 5% creature mitigation then 10% named Wheel bonus"
    );
    spell.name = "Another Spell".into();
    owner
        .select_item_spell(&spell)
        .expect("fresh named selection");
    assert_eq!(
        owner.finish(&p, &hit, &f.bindings[1], &mut |low, _| low),
        Ok(96),
        "only the global2.2% survives a different spell"
    );
}

#[test]
fn active_gem_healing_is_named_and_precedes_the_wheel_flat_stage() {
    let mut f = fixture();
    f.state.conditions.clear_on_death();
    let mut spell = wheel_healing_spell();
    spell.name = "Heal Friend".into();
    let mut effects = oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects::default();
    effects
        .sockets
        .gem_values_milli
        .insert(("base_healing_bonus".into(), spell.name.clone()), 5_000);
    f.attributes.wheel_effects = Some(effects);
    let mut p = plan();
    let NativeCombatPlan::Combat(combat) = &mut p else {
        panic!("combat");
    };
    combat.element = Element::Healing;
    combat.block_armor = false;
    combat.hits.truncate(1);
    combat.hits[0].target = 1;
    combat.hits[0].magnitude = 100;
    combat.hits[0].bonus_percent = 0;
    combat.hits[0].side_percent = None;
    let hit = combat.hits[0].clone();
    let mut owner = qualify(&f, &p).expect("source owner");
    owner.select_item_spell(&spell).expect("named cast");
    assert_eq!(
        owner.finish(&p, &hit, &f.bindings[0], &mut |_, _| panic!(
            "ordinary heal cannot crit"
        )),
        Ok(108)
    );
    spell.name = "Another Heal".into();
    owner
        .select_item_spell(&spell)
        .expect("new selection resets bonuses");
    assert_eq!(
        owner.finish(&p, &hit, &f.bindings[0], &mut |_, _| panic!(
            "ordinary heal cannot crit"
        )),
        Ok(103)
    );
}

#[test]
fn delayed_snapshot_selection_keeps_current_gems_and_focus_exactly_once() {
    for echo in [false, true] {
        let mut f = fixture();
        f.state = PlayerSpellState::new(
            crate::spell::cast::CharacterCastFacts {
                vocation: crate::spell::Vocation::Sorcerer,
                level: 500,
                magic_level: 10,
                max_health: 1_000,
                max_mana: 500,
                max_soul: 100,
            },
            0,
            0,
        )
        .expect("current caster");
        f.state
            .apply_owner_promotion_transition(true, true, Some(20_000_000))
            .expect("Premium");
        let mut spell = wheel_healing_spell();
        spell.name = if echo { "Death Echo" } else { "Divine Grenade" }.into();
        let mut at_cast =
            oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects::default();
        at_cast.active_unique_perks.insert("focus_mastery".into());
        at_cast
            .sockets
            .gem_values_milli
            .insert(("base_damage_bonus".into(), spell.name.clone()), 80_000);
        f.state.install_wheel_effects(at_cast.clone());
        f.state
            .complete_wheel_focus_cast(true, 0)
            .expect("armed Focus");
        f.attributes.binding.player_revision = f.state.revision();
        f.attributes.wheel_effects = Some(at_cast);
        f.attributes.base_critical_chance_permyriad = Some(0);
        f.attributes.equipment_critical_chance_permyriad = Some(0);
        f.attributes.base_critical_extra_permyriad = Some(0);
        f.attributes.equipment_critical_extra_permyriad = Some(0);
        f.attributes.fatal_chance_permyriad = Some(ExactRatio {
            numerator: 0,
            denominator: 1,
        });
        f.attributes.wheel_flat_damage = Some(50);
        f.attributes.source_damage_multiplier_percent = Some(10);
        let planted = StrikeCastModifiers::capture(&f.attributes, &f.state, &spell.name, 0)
            .expect("established cast snapshot");
        let stance = f
            .attributes
            .delayed_stance_snapshot(crate::spell::native_delayed::Element::Death, 1, 310)
            .expect("established stance snapshot");

        // The original snapshot saw 80%, but the actual due Combat sees 20%.
        let mut current = f.attributes.wheel_effects.clone().expect("Wheel");
        current
            .sockets
            .gem_values_milli
            .insert(("base_damage_bonus".into(), spell.name.clone()), 20_000);
        f.attributes.wheel_effects = Some(current.clone());
        f.state.install_wheel_effects(current);
        f.attributes.wheel_flat_damage = Some(7);
        f.attributes.source_damage_multiplier_percent = Some(90);
        let mut p = plan();
        let NativeCombatPlan::Combat(combat) = &mut p else {
            panic!("combat");
        };
        combat.element = Element::Physical;
        combat.block_armor = false;
        combat.hits.truncate(1);
        combat.hits[0].magnitude = 100;
        combat.hits[0].bonus_percent = 0;
        combat.hits[0].side_percent = None;
        let hit = combat.hits[0].clone();
        for (focus, expected) in [(true, 163), (false, 130)] {
            if !focus {
                f.state
                    .complete_wheel_focus_cast(false, 100_000)
                    .expect("other Combat consumed Focus");
            }
            let mut owner = qualify(&f, &p).expect("current numerical owner");
            owner.select_item_spell(&spell).expect("selected source");
            if echo {
                owner.select_echo_stance(&stance).expect("echo snapshot");
            } else {
                owner
                    .select_planted_strike(&planted)
                    .expect("planted snapshot");
            }
            assert_eq!(
                owner.finish(&p, &hit, &f.bindings[1], &mut |_, _| panic!(
                    "no critical roll"
                )),
                Ok(expected)
            );
            // 100 minus source creature mitigation5%; then retained10% plus
            // current gem20% and current Focus35% (or zero after consumption),
            // then the current flat7 rather than the cast-time flat50.
            owner
                .validate_current(&f.runtime, &f.state, &f.attributes, &[])
                .expect("fresh owner binding remains intact");
        }
        // Legacy stance critical facts remain selected, while current global
        // and named gem extra damage still join the due critical multiplier.
        let mut old_critical = f.attributes.clone();
        old_critical.base_critical_chance_permyriad = Some(10_000);
        old_critical.source_damage_multiplier_percent = Some(10);
        let critical_stance = old_critical
            .delayed_stance_snapshot(crate::spell::native_delayed::Element::Death, 1, 310)
            .expect("critical stance snapshot");
        f.attributes.base_critical_chance_permyriad = Some(10_000);
        let mut current = f.attributes.wheel_effects.clone().expect("current Wheel");
        current.sockets.gem_values_milli.extend([
            (
                (
                    "critical_extra_damage".into(),
                    "Critical Extra Damage".into(),
                ),
                2_200,
            ),
            (("critical_extra_damage".into(), spell.name.clone()), 12_000),
        ]);
        f.attributes.wheel_effects = Some(current.clone());
        f.state.install_wheel_effects(current);
        let mut owner = qualify(&f, &p).expect("current critical owner");
        owner.select_item_spell(&spell).expect("due source");
        if echo {
            owner
                .select_echo_stance(&critical_stance)
                .expect("legacy critical stance");
        } else {
            owner
                .select_planted_strike(&planted)
                .expect("legacy raw snapshot");
        }
        assert_eq!(
            owner.finish(&p, &hit, &f.bindings[1], &mut |low, high| {
                assert_eq!((low, high), (1, 10_000));
                low
            }),
            Ok(147),
            "current gem critical14.2%, target mitigation5%, source30%, current flat7"
        );
    }
}

#[test]
fn pvp_target_block_precedes_attacker_wheel_and_requires_exact_numerical_owner_proof() {
    use crate::foundation::{CommandId, CommandRef};
    use crate::gameplay_transport::actor_spell::{
        ChannelSpellStates, stage_player_batch, stage_player_batch_with_magnitude,
    };
    use crate::spell::cast::{CharacterCastFacts, WheelEquipment};
    use crate::spell::combat_batch as batch;
    use oteryn_simulation_determinism::SemanticTimeMicros;
    let mut f = fixture();
    f.state.conditions.clear_on_death();
    f.attributes.base_critical_chance_permyriad = Some(0);
    f.attributes.equipment_critical_chance_permyriad = Some(0);
    f.attributes.fatal_chance_permyriad = Some(ExactRatio {
        numerator: 0,
        denominator: 1,
    });
    f.attributes.wheel_flat_damage = Some(3);
    let session =
        GameSessionId::decode(&[1, 0x90, 0, 0, 0, 92, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 92])
            .expect("target session");
    let reservation = f
        .runtime
        .reserve_fresh_session(session)
        .expect("fourth slot");
    let target = f
        .runtime
        .commit_fresh_session(reservation)
        .expect("target actor");
    f.runtime
        .initialize_first_entry_position(target)
        .expect("target position");
    let facts = CharacterCastFacts {
        vocation: crate::spell::Vocation::Knight,
        level: 500,
        magic_level: 20,
        max_health: 1000,
        max_mana: 500,
        max_soul: 200,
    };
    let mut target_state = PlayerSpellState::new(facts, 0, 0).expect("target state");
    target_state
        .load_owned_build(
            &crate::durability::character_build::DurableBuildState::new(
                "knight",
                (20, 0),
                [(100, 0); 7],
            )
            .expect("durable build"),
        )
        .expect("qualified shielding");
    target_state.install_wheel_equipment(Some(WheelEquipment::default()));
    target_state
        .apply_owner_promotion_transition(true, true, Some(1_000_000))
        .expect("Premium");
    let mut effects = oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects::default();
    effects
        .sockets
        .gem_values_milli
        .insert(("Physical Resistance".into(), "".into()), 1_010);
    target_state.install_wheel_effects(effects);
    crate::spell::actor_conditions::apply_combat_change(
        &mut target_state,
        &OwnerCombatChange::Avatar(AvatarState {
            expires_ms: 1000,
            outfit_look_type: 1595,
            incoming_reduction_percent: 15,
            critical_chance_percent: 100,
            critical_extra_percentage_points: 15,
        }),
        0,
    )
    .expect("real Avatar");
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &f.runtime,
            f.caster,
            f.session,
            FACTS,
            (0, 0),
            SemanticTimeMicros::from_micros(0),
        )
        .expect("caster owner");
    assert!(states.replace_owned_player_state_test_fixture(
        &f.runtime,
        f.caster,
        f.session,
        f.state.clone()
    ));
    states
        .initialize(
            &f.runtime,
            target,
            session,
            facts,
            (0, 0),
            SemanticTimeMicros::from_micros(0),
        )
        .expect("target owner");
    assert!(states.replace_owned_player_state_test_fixture(
        &f.runtime,
        target,
        session,
        target_state.clone()
    ));
    f.bindings.push(LiveActorBinding {
        source_id: 4,
        actor: target,
        target_atom: crate::spell::combat_execution::actor_atom(target),
    });
    let mut p = plan();
    let NativeCombatPlan::Combat(combat) = &mut p else {
        panic!("combat")
    };
    combat.element = Element::Physical;
    combat.block_armor = false;
    combat.hits.truncate(1);
    combat.hits[0].target = 4;
    combat.hits[0].magnitude = 200;
    combat.hits[0].bonus_percent = 0;
    combat.hits[0].side_percent = None;
    let mut owner = PreparedMagnitudeOwner::qualify_pvp(
        &f.runtime,
        &f.state,
        f.caster,
        f.session,
        Some(&f.attributes),
        &p,
        &f.bindings,
        &[(target, &target_state)],
        100,
        &[(target, false)],
    )
    .expect("real target numerical owner");
    let template = batch::OwnerCombatBatch {
        caster: f.caster,
        attacker: f.attributes.binding.character,
        current_lease_generation: 1,
        command: CommandRef::new(f.session, CommandId::new(7002).expect("command")),
        occurrence: batch::SpellOccurrenceBinding {
            id: "wheel-player-block".into(),
            revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"].map(str::to_owned),
        },
        binding: vec![1],
        anchor: None,
        now_us: Some(SemanticTimeMicros::from_micros(100_000)),
        now_ms: 100,
        effects: vec![],
        deferred: None,
    };
    let lowered = batch::lower_native(
        &f.runtime,
        template,
        1,
        &p,
        &f.bindings,
        0,
        &mut |plan, hit, binding| {
            owner.finish_with_damage_healing(plan, hit, binding, &mut |_, _| {
                panic!("no random stage")
            })
        },
    )
    .expect("real lowering");
    let OwnerCombatChange::Damage { magnitude, .. } = lowered.batch.effects[0].change else {
        panic!("damage")
    };
    assert_eq!(
        magnitude, 85,
        "200/2=100; baseline1.3%=98; gem1.01%=97; Avatar15%=82; attackerflat3=85"
    );
    let proof =
        stage_player_batch_with_magnitude(&f.runtime, &states, &lowered.batch, None, Some(&owner))
            .expect("qualified final HP stage");
    let receipt = crate::foundation::runtime_actor_spell_types::PlayerBatchProof::health_receipt(
        &proof, target, 0,
    )
    .expect("same hit receipt");
    assert_eq!(
        receipt.health_after, 915,
        "baseline and Avatar are applied once before attacker flat"
    );
    let plain = stage_player_batch(&f.runtime, &states, &lowered.batch, None)
        .expect("legacy input still has full block stage");
    let receipt = crate::foundation::runtime_actor_spell_types::PlayerBatchProof::health_receipt(
        &plain, target, 0,
    )
    .expect("legacy receipt");
    assert_eq!(
        receipt.health_after, 930,
        "a naked magnitude never requests the preprocessed path"
    );
    let mut forged = lowered.batch.clone();
    if let OwnerCombatChange::Damage { magnitude, .. } = &mut forged.effects[0].change {
        *magnitude += 1;
    }
    assert!(
        stage_player_batch_with_magnitude(&f.runtime, &states, &forged, None, Some(&owner))
            .is_err()
    );
    let attack = crate::spell::cast::PreparedWheelAttackHit::prepare(
        &f.runtime,
        crate::spell::cast::WheelAttackActor {
            actor: f.caster,
            session: f.session,
            state: &f.state,
        },
        crate::spell::cast::WheelAttackActor {
            actor: target,
            session,
            state: &target_state,
        },
        crate::spell::cast::WheelAttackComponents {
            primary: (100, crate::player_lethal::PlayerHitKind::Physical),
            secondary: (0, crate::player_lethal::PlayerHitKind::Fire),
        },
        0,
        100_000,
        &mut |magnitude, _| Some(magnitude),
    )
    .expect("current autoattack observations");
    assert_eq!(
        attack.magnitude(),
        82,
        "same incoming block stage without a caster flat grant"
    );
    let mut attack_batch = lowered.batch.clone();
    if let OwnerCombatChange::Damage { magnitude, .. } = &mut attack_batch.effects[0].change {
        *magnitude = i64::from(attack.magnitude());
    }
    let attack_proof = crate::gameplay_transport::actor_spell::stage_player_batch_with_attack(
        &f.runtime,
        &states,
        &attack_batch,
        &attack,
    )
    .expect("existing autoattack joined owner");
    let receipt = crate::foundation::runtime_actor_spell_types::PlayerBatchProof::health_receipt(
        &attack_proof,
        target,
        0,
    )
    .expect("attack receipt");
    assert_eq!(receipt.health_after, 918);
    let mut equipment_inputs = Vec::new();
    let protected = crate::spell::cast::PreparedWheelAttackHit::prepare(
        &f.runtime,
        crate::spell::cast::WheelAttackActor {
            actor: f.caster,
            session: f.session,
            state: &f.state,
        },
        crate::spell::cast::WheelAttackActor {
            actor: target,
            session,
            state: &target_state,
        },
        crate::spell::cast::WheelAttackComponents {
            primary: (101, crate::player_lethal::PlayerHitKind::Physical),
            secondary: (0, crate::player_lethal::PlayerHitKind::Fire),
        },
        0,
        100_000,
        &mut |magnitude, _| {
            equipment_inputs.push(magnitude);
            let protected = oteryn_gameplay_items::equipment_effects::apply_protection(
                i64::from(magnitude),
                crate::content::ReferenceRationalPercent {
                    numerator: 10,
                    denominator: 1,
                },
            )?;
            u32::try_from(protected).ok()
        },
    )
    .expect("same existing equipment owner after baseline");
    assert_eq!(
        equipment_inputs,
        [99, 0],
        "baseline mitigation precedes the equipment read's protection formula"
    );
    assert_eq!(
        protected.magnitude(),
        75,
        "101→baseline99→existing equipment90→gem89→Avatar75"
    );
    let mut component_target = target_state.clone();
    component_target.health = 500;
    let mut component_effects = component_target
        .wheel_effects_at(100_000)
        .expect("current effects")
        .clone();
    component_effects
        .revelation_stages
        .insert("combat_mastery".into(), 3);
    component_effects
        .sockets
        .gem_values_milli
        .insert(("Fire Resistance".into(), "".into()), 1_010);
    component_target.install_wheel_effects(component_effects);
    component_target
        .install_virtue_party_bonuses(
            crate::spell::harmony::VirtuePartyBonuses {
                damage_reduction: 3,
                ..Default::default()
            },
            0,
        )
        .expect("current party observation");
    assert!(states.replace_owned_player_state_test_fixture(
        &f.runtime,
        target,
        session,
        component_target.clone()
    ));
    let components = crate::spell::cast::PreparedWheelAttackHit::prepare(
        &f.runtime,
        crate::spell::cast::WheelAttackActor {
            actor: f.caster,
            session: f.session,
            state: &f.state,
        },
        crate::spell::cast::WheelAttackActor {
            actor: target,
            session,
            state: &component_target,
        },
        crate::spell::cast::WheelAttackComponents {
            primary: (27, crate::player_lethal::PlayerHitKind::Physical),
            secondary: (27, crate::player_lethal::PlayerHitKind::Fire),
        },
        0,
        100_001,
        &mut |magnitude, _| Some(magnitude),
    )
    .expect("two real component types");
    assert_eq!(
        components.magnitude(),
        42,
        "each27→baseline26→gem25→Avatar21"
    );
    let mut component_batch = attack_batch.clone();
    component_batch.now_us = Some(SemanticTimeMicros::from_micros(100_001));
    // The proof and final HP owner must retain the semantic remainder below one millisecond.
    if let OwnerCombatChange::Damage { magnitude, .. } = &mut component_batch.effects[0].change {
        *magnitude = 42;
    }
    let proof = crate::gameplay_transport::actor_spell::stage_player_batch_with_attack(
        &f.runtime,
        &states,
        &component_batch,
        &components,
    )
    .expect("one joined HP occurrence for both components");
    let receipt = crate::foundation::runtime_actor_spell_types::PlayerBatchProof::health_receipt(
        &proof, target, 0,
    )
    .expect("component receipt");
    assert_eq!(
        receipt.health_after, 464,
        "each21→CM19→Virtue18, so36 total instead of37 after premature aggregation"
    );
    assert!(states.replace_owned_player_state_test_fixture(
        &f.runtime,
        target,
        session,
        target_state.clone()
    ));
    let mut caster_with_changed_equipment = f.state.clone();
    caster_with_changed_equipment.install_wheel_equipment(Some(WheelEquipment::default()));
    assert!(states.replace_owned_player_state_test_fixture(
        &f.runtime,
        f.caster,
        f.session,
        caster_with_changed_equipment
    ));
    assert!(
        attack_proof.validate_current(&f.runtime, &states).is_err(),
        "pending attack also compares the original caster equipment"
    );
    assert!(
        crate::gameplay_transport::actor_spell::stage_player_batch_with_attack(
            &f.runtime,
            &states,
            &attack_batch,
            &attack
        )
        .is_err()
    );
    assert!(states.replace_owned_player_state_test_fixture(
        &f.runtime,
        f.caster,
        f.session,
        f.state.clone()
    ));
    let mut target_with_removed_grant = target_state.clone();
    target_with_removed_grant.install_wheel_effects(Default::default());
    assert!(states.replace_owned_player_state_test_fixture(
        &f.runtime,
        target,
        session,
        target_with_removed_grant
    ));
    assert!(
        stage_player_batch_with_magnitude(&f.runtime, &states, &lowered.batch, None, Some(&owner))
            .is_err(),
        "current target grant was removed"
    );
    assert!(states.replace_owned_player_state_test_fixture(
        &f.runtime,
        target,
        session,
        target_state.clone()
    ));
    f.runtime
        .record_control_loss(
            target,
            session,
            crate::foundation::ControlLossMark {
                epoch: 1,
                grace_deadline: 500,
            },
        )
        .expect("target remains physically present during loss");
    let prepare_attack = |runtime: &ChannelRuntimeV1| {
        crate::spell::cast::PreparedWheelAttackHit::prepare(
            runtime,
            crate::spell::cast::WheelAttackActor {
                actor: f.caster,
                session: f.session,
                state: &f.state,
            },
            crate::spell::cast::WheelAttackActor {
                actor: target,
                session,
                state: &target_state,
            },
            crate::spell::cast::WheelAttackComponents {
                primary: (100, crate::player_lethal::PlayerHitKind::Physical),
                secondary: (0, crate::player_lethal::PlayerHitKind::Fire),
            },
            0,
            100_000,
            &mut |magnitude, _| Some(magnitude),
        )
    };
    let detached_target =
        prepare_attack(&f.runtime).expect("control loss grants no target invulnerability");
    assert!(
        crate::gameplay_transport::actor_spell::stage_player_batch_with_attack(
            &f.runtime,
            &states,
            &attack_batch,
            &detached_target
        )
        .is_ok()
    );
    f.runtime
        .record_control_loss(
            f.caster,
            f.session,
            crate::foundation::ControlLossMark {
                epoch: 1,
                grace_deadline: 500,
            },
        )
        .expect("attacker loss");
    assert!(
        prepare_attack(&f.runtime).is_none(),
        "a new attack still requires current caster control"
    );
}

#[test]
fn ascetic_spell_and_rune_damage_use_own_mantra_after_flat_before_sanctuary() {
    use crate::spell::harmony::VirtuePartyBonuses;
    use crate::spell::{Carrier, Vocation, cast::CharacterCastFacts};
    use oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects;
    for vocation in [Vocation::Monk, Vocation::ExaltedMonk] {
        for rune in [false, true] {
            for (own_mantra, expected) in [(3, 139), (0, 129)] {
                // This genuine creature policy also supplies Energy immunity, so
                // the final source additions cannot revive a blocked-zero hit.
                let mut f = fixture_with_damage_healing(100);
                f.state = PlayerSpellState::new(
                    CharacterCastFacts {
                        vocation,
                        level: 500,
                        magic_level: 20,
                        max_health: 1000,
                        max_mana: 500,
                        max_soul: 200,
                    },
                    5,
                    0,
                )
                .expect("current Monk with five Harmony");
                f.state
                    .apply_owner_promotion_transition(true, true, Some(20_000_000))
                    .expect("current Premium and promotion");
                f.state.install_equipped_mantra(own_mantra);
                f.state
                    .install_virtue_party_bonuses(
                        VirtuePartyBonuses {
                            mantra: 17,
                            ..Default::default()
                        },
                        0,
                    )
                    .expect("another member's current Mantra observation");
                let mut effects = WheelRuntimeEffects::default();
                effects.revelation_stages.insert("ascetic".into(), 3);
                effects.active_unique_perks.insert("sanctuary".into());
                f.state.install_wheel_effects(effects.clone());
                f.attributes.wheel_effects = Some(effects);
                f.attributes.binding.player_revision = f.state.revision();
                f.attributes.base_critical_chance_permyriad = Some(0);
                f.attributes.equipment_critical_chance_permyriad = Some(0);
                f.attributes.base_critical_extra_permyriad = Some(0);
                f.attributes.equipment_critical_extra_permyriad = Some(0);
                f.attributes.fatal_chance_permyriad = Some(ExactRatio {
                    numerator: 0,
                    denominator: 1,
                });
                f.attributes.wheel_flat_damage = Some(3);
                f.attributes.source_damage_multiplier_percent = Some(20);
                let mut spell = wheel_healing_spell();
                spell.name = "Tiger Clash".into();
                if rune {
                    spell.carrier = Carrier::Rune {
                        item: 3155,
                        charges: 1,
                        magic_level: 0,
                    };
                } else {
                    assert!(matches!(&spell.carrier, Carrier::Instant { .. }));
                }
                let target = &f.bindings[2];
                assert_eq!(target.source_id, 3);
                let source_position = f
                    .runtime
                    .read_actor_position(f.caster)
                    .expect("current caster position")
                    .position();
                let target_position = f
                    .runtime
                    .read_actor_position(target.actor)
                    .expect("current target position")
                    .position();
                assert_eq!((target_position.x, target_position.y), (11, 9));
                assert!(
                    source_position.floor != target_position.floor
                        || source_position.x.abs_diff(target_position.x) > 1
                        || source_position.y.abs_diff(target_position.y) > 1,
                    "the real target must not add Sanctuary's separate adjacency bonus"
                );
                for element in [Element::Physical, Element::Fire, Element::Energy] {
                    let mut p = plan();
                    let NativeCombatPlan::Combat(combat) = &mut p else {
                        panic!("combat");
                    };
                    combat.element = element;
                    combat.block_armor = false;
                    combat.hits.truncate(1);
                    combat.hits[0].target = target.source_id;
                    combat.hits[0].magnitude = 100;
                    combat.hits[0].bonus_percent = 0;
                    combat.hits[0].side_percent = None;
                    let hit = combat.hits[0].clone();
                    let mut owner = qualify(&f, &p).expect("actual current numerical owner");
                    owner
                        .select_item_spell(&spell)
                        .expect("selected Instant or Rune source");
                    assert_eq!(
                        owner.finish(&p, &hit, target, &mut |_, _| {
                            panic!("zero critical and fatal chances cannot draw")
                        }),
                        Ok(if element == Element::Energy {
                            0
                        } else {
                            expected
                        }),
                        "{vocation:?}, rune={rune}, own Mantra={own_mantra}, {element:?}"
                    );
                    owner
                        .validate_current(&f.runtime, &f.state, &f.attributes, &[])
                        .expect("the same current numerical proof remains valid");
                }
                // Crystal: raw 100 -> target mitigation 95 -> source 20% gives 114 -> flat 3 gives 117
                // -> own Ascetic 3 x 3 gives 126 -> Sanctuary Harmony 5 rounds 138.6 to 139.
                // Own 0 keeps 117 and rounds 128.7 to 129 despite the party's Mantra 17.
            }
        }
    }
}
