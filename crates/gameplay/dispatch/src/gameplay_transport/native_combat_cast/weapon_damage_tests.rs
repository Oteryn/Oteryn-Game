#![allow(clippy::expect_used, clippy::panic)]

use super::*;
use crate::content::native_gameplay::{NativeGameplayInput, compile_native_gameplay, decode};
use crate::durability::character_build::DurableBuildState;
use crate::durability::character_equipment::{CombatMode, EquipmentSnapshot, EquippedItem};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::spell::Vocation;
use crate::spell::cast::CharacterCastFacts;
use crate::spell::harmony::HarmonyMultiplier;
use crate::spell::magnitude_owner::ExactRatio;
use crate::spell::owned_cast_facts::{AccessProjections, CastFactsBinding, CurrentProjection};
use oteryn_protocol_oteryn::damage_element::DamageElement;
use std::sync::OnceLock;

fn content() -> &'static NativeGameplayState {
    static CONTENT: OnceLock<NativeGameplayState> = OnceLock::new();
    CONTENT.get_or_init(|| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/spells.manifest.json");
        let mut input = NativeGameplayInput::from_manifest(&path).expect("committed native inputs");
        input.source_world = None;
        let world = crate::content::native_gameplay_test_world_id().expect("world");
        let room = crate::content::qualify_native_entry_room(world).expect("qualified room");
        let compiled = compile_native_gameplay(room.compiled(), &input).expect("native artifact");
        decode(&compiled.server_artifact)
            .expect("decoded artifact")
            .state
    })
}

fn check_weapon_skill_damage(name: &str, aimed: bool, immune: Option<&str>) {
    let converted = immune.is_some();
    let (runtime, bindings, mut attributes) =
        crate::spell_magnitude_owner_tests::native_weapon_damage_fixture(immune);
    let content = content();
    let spell = content
        .spell_book()
        .spells
        .iter()
        .find(|spell| spell.name == name)
        .expect("committed native spell");
    let mut before = PlayerSpellState::new(
        CharacterCastFacts {
            vocation: Vocation::Knight,
            level: 330,
            magic_level: 0,
            max_health: 5_000,
            max_mana: 1_000,
            max_soul: 100,
        },
        0,
        0,
    )
    .expect("Knight player state");
    if converted {
        before.install_equipped_offence(
            oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence {
                elemental_conversion: Some((crate::content::ProjectV2RangedElement::Ice, 50)),
                ..Default::default()
            },
        );
    }
    attributes.binding.player_revision = before.revision();
    attributes.base_critical_chance_permyriad = Some(0);
    attributes.equipment_critical_chance_permyriad = Some(0);
    attributes.fatal_chance_permyriad = Some(ExactRatio {
        numerator: 0,
        denominator: 1,
    });
    attributes.wheel_flat_damage = Some(0);
    let binding = CastFactsBinding {
        actor: attributes.binding.actor,
        session: attributes.binding.session,
        character: *attributes.binding.character.as_bytes(),
        character_revision: 1,
        lease_generation: attributes.binding.lease_generation,
        connection_generation: 1,
        player_revision: before.revision(),
        content_digest: content.source_digest(),
        equipment_revision: 1,
    };
    let now = SemanticTimeMicros::from_micros(100_000);
    let target = &bindings[1];
    let caster_position = TilePosition {
        x: 10,
        y: 10,
        floor: 7,
    };
    let target_position = TilePosition {
        y: 9,
        ..caster_position
    };
    let operational = OperationalCastFacts {
        caster_position,
        target_position: aimed.then_some(target_position),
        target: aimed.then(|| crate::spell::target::CastTarget {
            caster: bindings[0].source_id,
            creature: target.source_id,
            actor: target.target_atom.clone(),
            master: None,
        }),
        line_of_sight_clear: Some(true),
        direction_available: true,
        wheel_unlocked: Some(true),
        in_protection_zone: false,
        target_tile_solid: Some(false),
        target_tile_creature: Some(aimed),
    };

    // The production preview reads the lower endpoint and the paid phase draws
    // a live value. Exercise both with the real cast and damage owners in
    // isolation from charge, missile and world-presentation admission.
    let mut resolved_damage = Vec::new();
    let mut expected_damage = Vec::new();
    let weapons: &[_] = if immune == Some("physical") {
        &[(3264, 14, 0)]
    } else if converted {
        &[(53213, 60, 54)]
    } else {
        &[(49523, 52, 44), (3318, 33, 0)]
    };
    for &(item, attack, elemental_attack) in weapons {
        let definition = &content
            .item_policy_for_source_id(item)
            .expect("qualified axe")
            .record()
            .production_definition;
        let owned = OwnedCastFacts::from_owner_reads(
            binding.clone(),
            DurableBuildState::new("knight", (0, 0), [(100, 0); 7]).expect("Knight build"),
            330,
            EquipmentSnapshot {
                character: binding.character,
                content_digest: binding.content_digest,
                character_revision: binding.character_revision,
                revision: binding.equipment_revision,
                combat_mode: Some(CombatMode::Attack),
                items: vec![EquippedItem {
                    slot: 5,
                    item_instance_id: [2; 16],
                    state_revision: 1,
                    tier: 0,
                    quantity: 1,
                    definition: TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: definition.production_key.clone(),
                        revision_ref: definition.revision_ref.clone(),
                    },
                }],
            },
            AccessProjections {
                premium: Some(CurrentProjection {
                    binding: binding.clone(),
                    authority_revision: 1,
                    valid_until_micros: 1_000_000,
                    value: true,
                }),
                learned: Some(CurrentProjection {
                    binding: binding.clone(),
                    authority_revision: 1,
                    valid_until_micros: 1_000_000,
                    value: [spell.key.clone()].into_iter().collect(),
                }),
                magnitude: Some(attributes.clone()),
                ..Default::default()
            },
        )
        .expect("owned equipment and magnitude");
        let caster = owned
            .caster(&before, spell, content, HarmonyMultiplier::ONE, now.get())
            .expect("owned skill inputs");
        assert_eq!(caster.attack_value, attack);
        let facts = NativeCombatFacts {
            now_ms: now.get() / 1000,
            stage: u8::from(aimed),
            inputs: FormulaInputs {
                level: caster.level,
                magic_level: caster.magic_level,
                base_power: spell.base_power,
                attack_skill: caster.attack_skill,
                attack_value: caster.attack_value,
                attack_factor: caster.attack_factor,
                shielding_skill: caster.shielding_skill,
                shield_defense: caster.shield_defense,
            },
            maximum_mana: caster.max_mana,
            caster: bindings[0].source_id,
            caster_position,
            area_origin: target_position,
            area_sight_origin: caster_position,
            direction: Direction::North,
            explicit_target: aimed.then_some(target.source_id),
            attacked_target: None,
            targets: vec![NativeTargetFact {
                id: target.source_id,
                position: target_position,
                kind: TargetKind::MasterlessMonster,
                name: "fixture creature".into(),
                reward_boss: false,
                preferred_distance: 4,
                health: 20,
                maximum_health: 20,
                legal: true,
            }],
            clear_sight: [(caster_position, target_position)].into_iter().collect(),
            floor_change_tiles: BTreeSet::new(),
            elemental_stance: ElementalStance::None,
            incoming_hit: None,
        };
        for (phase, draw) in [
            ("preview", (|low, _| low) as fn(i64, i64) -> i64),
            ("paid", |_, high| high),
        ] {
            let mut draw = draw;
            let plan = if super::super::ordinary_combat::applicable(spell) {
                let paid = crate::spell::cast::prepare_ordinary_owner_cast_with_caster(
                    content.spell_book(),
                    &before,
                    spell,
                    &operational,
                    now,
                    &caster,
                    None,
                    None,
                    &mut draw,
                )
                .expect("qualified ordinary cast");
                let [
                    crate::spell::ResolvedEffect::Damage {
                        damage_type,
                        magnitude,
                    },
                ] = paid.resolution.effects.as_slice()
                else {
                    panic!("expected one Berserk damage effect");
                };
                assert_eq!(damage_type, "physical");
                Box::new(NativeCombatPlan::Combat(crate::spell::native_combat::CombatPlan {
                    element: crate::spell::native_combat::Element::Physical,
                    area: None,
                    hits: vec![crate::spell::native_combat::MagnitudePlan {
                        target: target.source_id,
                        magnitude: *magnitude,
                        bonus_percent: 0,
                        side_percent: None,
                        delay_ms: 0,
                    }],
                    cooldown_ms: None,
                    shared_cooldown: None,
                    reduce_all_spell_cooldowns_ms: 0,
                    block_armor: oteryn_gameplay_spell_rules::dispatch_plans::ordinary_combat::block_armor(spell)
                        .expect("Berserk armor binding"),
                    use_weapon_charges: false,
                    weapon_missile: false,
                    dispel_paralysis_targets: Vec::new(),
                    resolve_critical_and_fatal_once: true,
                }))
            } else {
                let (paid, _) = delayed_strikes::paid(
                    &before,
                    spell,
                    &operational,
                    Facts::Combat(&facts),
                    None,
                    now,
                    &mut draw,
                    &caster,
                )
                .expect("qualified native combat plan");
                let Plan::Combat(plan) = paid.plan else {
                    panic!("expected a combat plan");
                };
                plan
            };
            let NativeCombatPlan::Combat(combat) = plan.as_ref() else {
                panic!("expected a weapon-skill hit");
            };
            let [hit] = combat.hits.as_slice() else {
                panic!("expected exactly one hit: {:?}", combat.hits);
            };
            let mut owner = qualify_magnitude(
                &runtime,
                &before,
                &owned,
                (binding.actor, binding.session),
                spell,
                content,
                &plan,
                &bindings,
                &[],
                facts.now_ms,
            )
            .expect("dispatch magnitude qualification");
            owner
                .finish_with_damage_healing(&plan, hit, target, &mut draw)
                .expect("resolve source weapon-skill damage");
            let components = owner
                .damage_components()
                .next()
                .expect("resolved components");
            resolved_damage.push((
                item,
                phase,
                components.secondary_element,
                components.post_block(),
            ));
            // Crystal 201189f8 combat.cpp:2099-2104 splits the shared formula
            // roll by attack points. The fixture blocks 5 or 9 physical damage
            // at the chosen armor endpoint and mitigates each component by 5%.
            let fire_ratio = f64::from(elemental_attack) / f64::from(attack);
            let raw_physical = (hit.magnitude as f64 * (1.0 - fire_ratio)).round() as i64;
            let raw_fire = (hit.magnitude as f64 * fire_ratio).round() as i64;
            let armor = if combat.block_armor { draw(5, 9) } else { 0 };
            if converted {
                let converted_raw = raw_physical / 2;
                expected_damage.push((
                    item,
                    phase,
                    Some(DamageElement::Ice),
                    [
                        if immune == Some("physical") {
                            0
                        } else {
                            (converted_raw - armor).max(0) * 95 / 100
                        },
                        converted_raw * 95 / 100,
                    ],
                ));
                continue;
            }
            expected_damage.push((
                item,
                phase,
                (elemental_attack > 0).then_some(DamageElement::Fire),
                [
                    (raw_physical - armor).max(0) * 95 / 100,
                    raw_fire * 95 / 100,
                ],
            ));
        }
    }
    assert_eq!(resolved_damage, expected_damage, "{name}");
}

#[test]
fn native_weapon_skill_front_sweep_splits_preview_and_paid_damage() {
    check_weapon_skill_damage("Front Sweep", false, None);
}

#[test]
fn native_weapon_skill_executioners_throw_splits_preview_and_paid_damage() {
    check_weapon_skill_damage("Executioner's Throw", true, None);
}

#[test]
fn native_weapon_skill_executioners_throw_replaces_immune_secondary_with_imbuement() {
    check_weapon_skill_damage("Executioner's Throw", true, Some("fire"));
}

#[test]
fn native_weapon_skill_berserk_replaces_immune_secondary_with_imbuement() {
    check_weapon_skill_damage("Berserk", false, Some("fire"));
}

#[test]
fn native_physical_sword_weapon_spells_apply_frost_imbuement() {
    check_weapon_skill_damage("Berserk", false, Some("physical"));
    check_weapon_skill_damage("Executioner's Throw", true, Some("physical"));
}
