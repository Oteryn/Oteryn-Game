//! GP-001: actual owner qualification feeds each affected cast preparation.

use super::*;
use crate::ability::{AbilityOccurrence, RevisionSet};
use crate::content::native_gameplay::{
    NativeGameplayInput, NativeGameplayMapProfile, NativeGameplayState, NativeTrainingInput,
    PinnedGameplayBytes, compile_native_gameplay, decode,
};
use crate::durability::character_build::DurableBuildState;
use crate::durability::character_equipment::EquipmentSnapshot;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::spell::cast::CharacterCastFacts;
use crate::spell::chain::TilePosition;
use crate::spell::owned_cast_facts::{
    AccessProjections, CastFactsBinding, CurrentProjection, OwnedCastFacts, OwnedFactsError,
};
use crate::spell::{OperationalCastFacts, Vocation};
use oteryn_protocol_oteryn::{ChannelId, WorldId};
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
        item_profiles: Some(pinned(
            br#"{"schema":"OTERYN_NATIVE_ITEM_PROFILES/v1","records":[]}"#,
        )),
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
    let (vocation, key) = if spell.name == "Heal Friend" {
        (Vocation::Druid, "druid")
    } else {
        (Vocation::Paladin, "paladin")
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
        combat_mode: None,
        items: vec![],
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
