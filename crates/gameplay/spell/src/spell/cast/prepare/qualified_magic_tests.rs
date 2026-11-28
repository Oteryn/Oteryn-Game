//! GP-001: actual owner qualification feeds each affected cast preparation.

use super::*;
use crate::ability::{AbilityOccurrence, RevisionSet};
use crate::content::native_gameplay::{
    NativeGameplayInput, NativeGameplayMapProfile, NativeGameplayState, NativeTrainingInput,
    PinnedGameplayBytes, compile_native_gameplay, decode,
};
use crate::durability::character_build::DurableBuildState;
use crate::durability::character_equipment::{CombatMode, EquipmentSnapshot, EquippedItem};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::spell::cast::CharacterCastFacts;
use crate::spell::chain::TilePosition;
use crate::spell::owned_cast_facts::{
    AccessProjections, CastFactsBinding, CurrentProjection, OwnedCastFacts, OwnedFactsError,
};
use crate::spell::{OperationalCastFacts, Vocation};
use oteryn_protocol_oteryn::{ChannelId, WorldId};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const NOW: u64 = 1_000;
const PATHS: [&str; 5] = [
    "Light",
    "Food",
    "Find Person",
    "Conjure Arrow",
    "Heal Friend",
];

fn id(last: u8) -> [u8; 16] {
    [1, 144, 0, 0, 0, 8, 112, 0, 128, 0, 0, 0, 0, 0, 0, last]
}

fn content() -> Result<NativeGameplayState, String> {
    content_with_item_profiles(br#"{"schema":"OTERYN_NATIVE_ITEM_PROFILES/v1","records":[]}"#)
}

fn content_with_item_profiles(item_profiles: &[u8]) -> Result<NativeGameplayState, String> {
    let pinned = |bytes: &[u8]| PinnedGameplayBytes {
        bytes: bytes.to_vec(),
        sha256: Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    };
    macro_rules! sample {
        ($path:literal) => {
            pinned(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../tools/content-schema/",
                $path
            )))
        };
    }
    let input = NativeGameplayInput {
        cosmetics: NativeGameplayInput::unavailable_cosmetics(),
        native_map_profile: NativeGameplayMapProfile::AcceptedEntryR1,
        catalog: sample!("spell-authoring/samples/executable-spell-catalog.json"),
        source_selection: sample!("spell-authoring/samples/executable-spell-source-selection.json"),
        creature_profiles: sample!("native-gameplay/creature_profiles.json"),
        presentation_profiles: sample!("native-gameplay/presentation_profiles.json"),
        item_profiles: Some(pinned(item_profiles)),
        spell_appearances: Some(sample!("native-gameplay/spell_appearances.json")),
        build_training: Some(NativeTrainingInput {
            profile: sample!("native-gameplay/build-training.json"),
            content_revision: "build-content-r1".into(),
            magnitude_policy:
                oteryn_gameplay_domain::domain::magnitude_policy::MagnitudePolicy::Strict,
        }),
        familiar_config: None,
        familiar_defenses: None,
        wheel_profile: None,
        source_world: None,
        progression: None,
        item_keys: None,
        loot_tables: None,
    };
    let world = WorldId::decode(&id(1)).map_err(|e| format!("{e:?}"))?;
    let room = crate::content::qualify_native_entry_room(world).map_err(|e| format!("{e:?}"))?;
    let compiled =
        compile_native_gameplay(room.compiled(), &input).map_err(|e| format!("{e:?}"))?;
    decode(&compiled.server_artifact)
        .map(|decoded| decoded.state)
        .map_err(|e| format!("{e:?}"))
}

fn spell(content: &NativeGameplayState, name: &str) -> Result<SpellDefinition, String> {
    let entry = content
        .catalog()
        .entries
        .iter()
        .find(|entry| entry.header.name == name)
        .ok_or_else(|| format!("missing source spell {name}"))?;
    entry
        .definition
        .clone()
        .map_err(|error| format!("invalid source spell {name}: {error:?}"))
}

fn owner(
    content: &NativeGameplayState,
    spell: &SpellDefinition,
    loyalty: u32,
    proficiency: u32,
) -> Result<(PlayerSpellState, OwnedCastFacts), String> {
    owner_with_equipment(content, spell, loyalty, proficiency, None, Vec::new())
}

fn owner_with_equipment(
    content: &NativeGameplayState,
    spell: &SpellDefinition,
    loyalty: u32,
    proficiency: u32,
    combat_mode: Option<CombatMode>,
    items: Vec<EquippedItem>,
) -> Result<(PlayerSpellState, OwnedCastFacts), String> {
    let (vocation, key) = match spell.name.as_str() {
        "Food" | "Heal Friend" | "Druid familiar" => (Vocation::Druid, "druid"),
        _ if spell.vocations.len() == 1 && spell.vocations.contains(&Vocation::Monk) => {
            (Vocation::Monk, "monk")
        }
        "Annihilation" => (Vocation::Knight, "knight"),
        _ => (Vocation::Paladin, "paladin"),
    };
    let mut state = PlayerSpellState::new(
        CharacterCastFacts {
            vocation,
            level: 400,
            magic_level: 30,
            max_health: 1000,
            max_mana: 5000,
            max_soul: 100,
        },
        0,
        0,
    )
    .ok_or("invalid player fixture")?;
    state
        .apply_owner_premium_transition(true, Some(1_000_000))
        .map_err(|e| format!("{e:?}"))?;
    state
        .make_playable(SemanticTimeMicros::from_micros(0))
        .map_err(|e| format!("{e:?}"))?;
    let binding = CastFactsBinding {
        actor: ExactActorRef::transport_fixture(
            WorldId::decode(&id(1)).map_err(|e| format!("{e:?}"))?,
            ChannelId::decode(&id(2)).map_err(|e| format!("{e:?}"))?,
        ),
        session: GameSessionId::decode(&id(3)).map_err(|e| format!("{e:?}"))?,
        character: id(4),
        character_revision: 1,
        lease_generation: 1,
        connection_generation: 1,
        player_revision: state.revision(),
        content_digest: content.source_digest(),
        equipment_revision: 1,
    };
    let equipment = EquipmentSnapshot {
        character: binding.character,
        content_digest: binding.content_digest,
        character_revision: binding.character_revision,
        revision: binding.equipment_revision,
        combat_mode,
        items,
    };
    let mut access = AccessProjections {
        loyalty_points: loyalty,
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
            value: std::collections::BTreeSet::from([spell.key.clone()]),
        }),
        ..AccessProjections::default()
    };
    access.proficiency_skills[7] = proficiency;
    let owned = OwnedCastFacts::from_owner_reads(
        binding,
        DurableBuildState::new(key, (30, 0), [(10, 0); 7]).map_err(|e| format!("{e:?}"))?,
        400,
        equipment,
        access,
    )
    .map_err(|e| format!("{e:?}"))?;
    Ok((state, owned))
}

fn prepare(
    state: &PlayerSpellState,
    spell: &SpellDefinition,
    caster: &CasterState,
) -> Result<(PlayerSpellState, crate::spell::combat_batch::SpellAnchor), SpellCastDisposition> {
    let now = SemanticTimeMicros::from_micros(NOW);
    let operational = OperationalCastFacts {
        caster_position: TilePosition {
            x: 0,
            y: 0,
            floor: 7,
        },
        target_position: None,
        target: None,
        line_of_sight_clear: Some(true),
        direction_available: true,
        wheel_unlocked: None,
        in_protection_zone: false,
        target_tile_solid: None,
        target_tile_creature: None,
    };
    match spell.name.as_str() {
        "Light" => {
            let book = SpellBook::canonical(vec![spell.clone()])
                .map_err(|_| SpellCastDisposition::Rejected)?;
            let revisions = RevisionSet::new(
                "ruleset:r1",
                "content:r1",
                "world:r1",
                "formula:r1",
                "simulation:r1",
            )
            .map_err(|_| SpellCastDisposition::Rejected)?;
            let paid = prepare_source_self_cast_with_caster(
                state,
                spell,
                &book,
                &operational,
                caster,
                CastContext {
                    caster: "actor:gp001",
                    owner_scope: "channel-owner",
                    occurrence: AbilityOccurrence::new("spell-cast:gp001", revisions)
                        .map_err(|_| SpellCastDisposition::Rejected)?,
                    now,
                    draw: &mut |minimum, _| minimum,
                },
            )?;
            Ok((paid.next, paid.anchor))
        }
        "Food" => {
            let paid = prepare_native_owner_cast_with_caster(
                state,
                spell,
                &operational,
                crate::spell::native::Facts::Food,
                now,
                &mut |minimum, _| minimum,
                caster,
            )?;
            assert!(matches!(paid.plan, crate::spell::native::Plan::Food { .. }));
            Ok((paid.next, paid.anchor))
        }
        "Find Person" => {
            let paid = prepare_native_callback_owner_cast_with_caster(
                state,
                spell,
                &operational,
                now,
                caster,
            )?;
            Ok((paid.next, paid.anchor))
        }
        "Conjure Arrow" => {
            let paid =
                prepare_conjure_owner_cast_with_caster(state, spell, &operational, now, caster)?;
            Ok((paid.next, paid.anchor))
        }
        "Heal Friend" => {
            let paid =
                prepare_named_player_failure_owner_cast_with_caster(state, spell, now, caster)?;
            Ok((paid.next, paid.anchor))
        }
        _ => Err(SpellCastDisposition::Rejected),
    }
}

#[test]
fn gp001_qualified_owner_magic_reaches_all_affected_preparations() -> Result<(), String> {
    let content = content()?;
    for name in PATHS {
        let spell = spell(&content, name)?;
        for (loyalty, proficiency) in [(0, 0), (3600, 0), (0, 2), (3600, 2)] {
            let (state, owned) = owner(&content, &spell, loyalty, proficiency)?;
            let caster = owned
                .caster(&state, &spell, &content, HarmonyMultiplier::ONE, NOW)
                .map_err(|e| format!("{e:?}"))?;
            if loyalty != 0 || proficiency != 0 {
                assert!(
                    caster.magic_level > state.character_facts().magic_level,
                    "{name}"
                );
            } else {
                assert_eq!(caster.magic_level, state.character_facts().magic_level);
            }
            let (next, anchor) = prepare(&state, &spell, &caster).map_err(|e| {
                format!("{name}: loyalty={loyalty}, proficiency={proficiency}: {e:?}")
            })?;
            assert_eq!(next.revision(), state.revision() + 1, "{name}");
            assert_eq!(next.character_facts().magic_level, 30, "{name}");
            let expected_mana = if name == "Heal Friend" {
                0
            } else {
                crate::spell::mana_cost(&spell, &caster)
            };
            let expected_soul = if name == "Heal Friend" { 0 } else { spell.soul };
            assert_eq!(
                (anchor.paid_mana, anchor.paid_soul),
                (expected_mana, expected_soul),
                "{name}"
            );
            assert_eq!(next.mana, state.mana - expected_mana, "{name}");
            assert_eq!(next.soul, state.soul - expected_soul, "{name}");
            assert!(
                next.cooldowns
                    .spell_ready_at(&spell.key)
                    .is_some_and(|ready| ready.get() > NOW),
                "{name}"
            );
        }
    }
    Ok(())
}

#[test]
fn gp001_qualified_magic_keeps_raw_build_binding_and_live_state_checks() -> Result<(), String> {
    let content = content()?;
    for name in PATHS {
        let spell = spell(&content, name)?;
        let (state, owned) = owner(&content, &spell, 3600, 2)?;
        let mut mismatched_raw = state.clone();
        mismatched_raw.facts.magic_level += 1;
        assert!(
            matches!(
                owned.caster(
                    &mismatched_raw,
                    &spell,
                    &content,
                    HarmonyMultiplier::ONE,
                    NOW
                ),
                Err(OwnedFactsError::BuildMismatch)
            ),
            "{name}"
        );
        let mut changed_owner = state.clone();
        changed_owner.revision += 1;
        assert!(
            matches!(
                owned.caster(
                    &changed_owner,
                    &spell,
                    &content,
                    HarmonyMultiplier::ONE,
                    NOW
                ),
                Err(OwnedFactsError::StaleBinding)
            ),
            "{name}"
        );
        let caster = owned
            .caster(&state, &spell, &content, HarmonyMultiplier::ONE, NOW)
            .map_err(|e| format!("{e:?}"))?;
        for field in 0..5 {
            let mut stale = caster.clone();
            match field {
                0 => stale.level += 1,
                1 => stale.mana += 1,
                2 => stale.max_mana += 1,
                3 => stale.soul += 1,
                _ => stale.vocation = Vocation::Knight,
            }
            assert_eq!(
                prepare(&state, &spell, &stale).err(),
                Some(SpellCastDisposition::Rejected),
                "{name}, field={field}"
            );
        }
    }
    Ok(())
}

#[test]
fn qualified_unarmed_skill_casts_receive_source_attack_inputs() -> Result<(), String> {
    let content = content()?;
    for name in [
        "Lesser Ethereal Spear",
        "Ethereal Spear",
        "Strong Ethereal Spear",
        "Ethereal Barrage",
        "Swift Jab",
        "Double Jab",
        "Forceful Uppercut",
        "Tiger Clash",
        "Greater Tiger Clash",
        "Devastating Knockout",
        "Greater Flurry of Blows",
        "Mystic Repulse",
        "Thousand Fist Blows",
        "Chained Penance",
        "Spiritual Outburst",
        "Flurry of Blows",
    ] {
        let mut spell = spell(&content, name)?;
        let (state, owned) = owner(&content, &spell, 0, 0)?;
        let caster = owned
            .caster(&state, &spell, &content, HarmonyMultiplier::ONE, NOW)
            .map_err(|error| format!("{name}: {error:?}"))?;
        assert_eq!(
            (
                caster.attack_skill,
                caster.attack_value,
                caster.attack_factor,
                caster.melee_weapon
            ),
            (0, 7, 0.0, false),
            "{name}"
        );
        assert!(
            owned.intrinsic_weapon_for_spell(&spell, &content).is_none(),
            "{name}"
        );
        spell.needs_weapon = true;
        assert!(matches!(
            owned.caster(&state, &spell, &content, HarmonyMultiplier::ONE, NOW),
            Err(OwnedFactsError::UnavailableEquipment)
        ));
    }
    Ok(())
}

#[test]
fn source_skill_formulas_without_a_weapon_use_unarmed_inputs() -> Result<(), String> {
    let content = content()?;
    for name in ["Chained Penance", "Ethereal Spear"] {
        let spell = spell(&content, name)?;
        assert!(!spell.needs_weapon, "{name}");
        let (state, owned) = owner(&content, &spell, 0, 0)?;
        let caster = owned
            .caster(&state, &spell, &content, HarmonyMultiplier::ONE, NOW)
            .map_err(|error| format!("{name}: {error:?}"))?;
        assert_eq!((caster.attack_skill, caster.attack_value), (0, 7), "{name}");
        assert_eq!(caster.attack_factor, 0.0, "{name}");
        assert!(!caster.melee_weapon, "{name}");
    }
    Ok(())
}

#[test]
fn missing_equipment_reaches_the_ordered_cast_rules_without_payment() -> Result<(), String> {
    use crate::spell::CastRejection;
    let content = content()?;
    let mut spell = spell(&content, "Annihilation")?;
    let (state, owned) = owner(&content, &spell, 0, 0)?;
    assert!(spell.needs_weapon);
    let caster = owned
        .caster_for_admission(&state, &spell, &content, HarmonyMultiplier::ONE, NOW)
        .map_err(|error| format!("qualified caster without equipment: {error:?}"))?;
    assert!(!caster.melee_weapon);
    let empty_cooldowns = crate::spell::Cooldowns::default();
    let operational = OperationalCastFacts {
        caster_position: TilePosition {
            x: 1,
            y: 1,
            floor: 7,
        },
        target_position: Some(TilePosition {
            x: 2,
            y: 1,
            floor: 7,
        }),
        target: Some(crate::spell::target::CastTarget {
            caster: 1,
            creature: 2,
            actor: "creature:target".into(),
            master: None,
        }),
        line_of_sight_clear: Some(true),
        direction_available: true,
        wheel_unlocked: None,
        in_protection_zone: false,
        target_tile_solid: Some(false),
        target_tile_creature: Some(true),
    };
    let draws = std::cell::Cell::new(0);
    let resolve = |spell: &SpellDefinition, caster: &CasterState| {
        crate::spell::resolve_cast_with_operational_facts(
            spell,
            caster,
            &empty_cooldowns,
            SemanticTimeMicros::from_micros(NOW),
            &operational,
            &mut |minimum, _| {
                draws.set(draws.get() + 1);
                minimum
            },
        )
        .err()
    };
    assert_eq!(
        resolve(&spell, &caster),
        Some(CastRejection::WeaponRequired)
    );
    spell.level = caster.level + 1;
    assert_eq!(
        resolve(&spell, &caster),
        Some(CastRejection::LevelTooLow {
            required: spell.level
        }),
    );
    spell.level = caster.level;
    spell.needs_weapon = false;
    spell.needs_shield = true;
    let caster = owned
        .caster_for_admission(&state, &spell, &content, HarmonyMultiplier::ONE, NOW)
        .map_err(|error| format!("qualified caster without shield: {error:?}"))?;
    assert_eq!(caster.shield_defense, None);
    assert_eq!(
        resolve(&spell, &caster),
        Some(CastRejection::ShieldRequired)
    );
    assert_eq!(
        draws.get(),
        0,
        "equipment admission must precede the damage draw"
    );
    Ok(())
}

#[test]
fn familiar_engine_admission_reports_cooldown_before_summon() -> Result<(), String> {
    let content = content()?;
    let spell = spell(&content, "Druid familiar")?;
    let (mut state, owned) = owner(&content, &spell, 0, 0)?;
    let now = SemanticTimeMicros::from_micros(NOW);
    let caster = owned
        .caster(&state, &spell, &content, HarmonyMultiplier::ONE, now.get())
        .map_err(|error| format!("{error:?}"))?;
    let operational = OperationalCastFacts {
        caster_position: TilePosition {
            x: 0,
            y: 0,
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
    let ready_at = SemanticTimeMicros::from_micros(NOW + 10);
    state.cooldowns.spells.insert(spell.key.clone(), ready_at);
    let check = |time| {
        crate::spell::resolve_native_callback_common(
            &spell,
            &caster,
            &state.cooldowns,
            time,
            &operational,
        )
    };
    assert!(
        matches!(
            check(now),
            Err(crate::spell::CastRejection::SpellCooling { ready_at: due }) if due == ready_at
        ),
        "familiar cooldown result: {:?}",
        check(now)
    );
    assert_eq!(
        check(ready_at)
            .map_err(|error| format!("{error:?}"))?
            .mana_spent,
        3000
    );
    Ok(())
}

fn equipped_spell_fixture() -> Result<(NativeGameplayState, Vec<EquippedItem>), String> {
    let record = |document: &[u8], key: &str| -> Result<Value, String> {
        let profiles: Value =
            serde_json::from_slice(document).map_err(|error| error.to_string())?;
        profiles["records"]
            .as_array()
            .and_then(|records| {
                records
                    .iter()
                    .find(|record| record["production_definition"]["production_key"] == key)
            })
            .cloned()
            .ok_or_else(|| format!("missing pinned item profile {key}"))
    };
    let hatchet = "oteryn:item.tibia.i3276";
    let spellbook = "oteryn:item.tibia.i21400";
    let weapon = record(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/items/definitions/spell-native-profiles.json"
        )),
        hatchet,
    )?;
    let mut offhand = record(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/items/definitions/starting-loadout-profiles.json"
        )),
        spellbook,
    )?;
    // Hold the offhand observation unknown even as the shipped spellbook gains facts.
    offhand["semantics"]["weapon"] = json!({"state": "UNKNOWN"});
    assert_eq!(offhand["semantics"]["weapon"]["state"], "UNKNOWN");
    let profiles = serde_json::to_vec(&json!({
        "schema": "OTERYN_NATIVE_ITEM_PROFILES/v1",
        "records": [offhand, weapon],
    }))
    .map_err(|error| error.to_string())?;
    let items = [(5, hatchet), (6, spellbook)]
        .map(|(slot, key)| EquippedItem {
            tier: 0,
            slot,
            item_instance_id: id(slot),
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: "definition-r1".into(),
            },
            quantity: 1,
            state_revision: 1,
        })
        .to_vec();
    Ok((content_with_item_profiles(&profiles)?, items))
}

#[test]
fn attack_spell_reads_the_weapon_without_qualifying_unneeded_shield_facts() -> Result<(), String> {
    let (content, items) = equipped_spell_fixture()?;
    let mut spell = spell(&content, "Chained Penance")?;
    let (state, owned) =
        owner_with_equipment(&content, &spell, 0, 0, Some(CombatMode::Attack), items)?;
    let caster = owned
        .caster(&state, &spell, &content, HarmonyMultiplier::ONE, NOW)
        .map_err(|error| format!("attack-only caster: {error:?}"))?;
    assert_eq!(caster.attack_value, 15);
    assert_eq!(caster.attack_factor, 1.0);
    spell.needs_shield = true;
    assert!(matches!(
        owned.caster(&state, &spell, &content, HarmonyMultiplier::ONE, NOW),
        Err(OwnedFactsError::UnavailableEquipment)
    ));
    Ok(())
}

#[test]
fn attack_spell_uses_initial_attack_mode_until_the_player_selects_one() -> Result<(), String> {
    let (content, items) = equipped_spell_fixture()?;
    let spell = spell(&content, "Chained Penance")?;
    for (mode, factor) in [
        (None, 1.0),
        (Some(CombatMode::Balanced), 0.75),
        (Some(CombatMode::Defense), 0.5),
    ] {
        let (state, owned) = owner_with_equipment(&content, &spell, 0, 0, mode, items.clone())?;
        let caster = owned
            .caster(&state, &spell, &content, HarmonyMultiplier::ONE, NOW)
            .map_err(|error| format!("caster with mode {mode:?}: {error:?}"))?;
        assert_eq!(caster.attack_value, 15);
        assert_eq!(caster.attack_factor, factor);
    }
    Ok(())
}

#[test]
fn elemental_stance_cast_uses_base_element_with_no_selected_stance() -> Result<(), String> {
    use crate::spell::native_combat::ElementalStance;
    let content = content()?;
    let spell = spell(&content, "Energy Beam")?;
    let (_, mut owned) = owner(&content, &spell, 0, 0)?;
    assert_eq!(
        owned.elemental_stance_for_cast(NOW),
        Ok(ElementalStance::None)
    );
    owned.access.elemental_stance = Some(CurrentProjection {
        binding: owned.binding().clone(),
        authority_revision: 1,
        valid_until_micros: NOW + 1,
        value: ElementalStance::Flames,
    });
    assert_eq!(
        owned.elemental_stance_for_cast(NOW),
        Ok(ElementalStance::Flames)
    );
    assert_eq!(
        owned.elemental_stance_for_cast(NOW + 1),
        Err(OwnedFactsError::StaleBinding)
    );
    Ok(())
}
