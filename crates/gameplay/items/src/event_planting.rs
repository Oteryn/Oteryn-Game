//! W19b source rules. Crystal 201189f owns the seed, flower and bush actions.
use crate::item_craft::CookingConversion;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;

pub const ICE_FLOWER_SEEDS: u32 = 13_844;
pub const EMPTY_POT: u32 = 306;

/// Stable identity for a boot-created world-change object. The upper two bits are
/// distinct from ordinary callback objects and volatile moved/player-grown items.
/// This is an internal map provenance key, never a client-selected handle.
pub fn world_change_item_key(appearance: u16, x: u16, y: u16, z: u8) -> Option<u64> {
    (appearance != 0 && z <= 15).then_some(
        (3_u64 << 62)
            | (u64::from(appearance) << 36)
            | (u64::from(z) << 32)
            | (u64::from(x) << 16)
            | u64::from(y),
    )
}

/// Decode only a canonical world-change provenance key; ordinary callback IDs fail closed.
pub fn world_change_item_source(key: u64) -> Option<(u16, u16, u16, u8)> {
    if (key & !((1_u64 << 52) - 1)) != (3_u64 << 62) {
        return None;
    }
    let appearance = (key >> 36 & 0xffff) as u16;
    (appearance != 0).then_some((
        appearance,
        (key >> 16 & 0xffff) as u16,
        (key & 0xffff) as u16,
        (key >> 32 & 15) as u8,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecayStep {
    pub target: u32,
    pub after_ms: i64,
}

/// Exact source durations, never caller-selected. Delays are from the previous deadline.
pub fn decay_steps(item: u32) -> &'static [DecayStep] {
    match item {
        // data/items/items.xml:18237-18242; party_trumpet.lua:3-6.
        6573 => &[DecayStep {
            target: 6572,
            after_ms: 4_000,
        }],
        // data/items/items.xml:11687-11691.
        3700 => &[DecayStep {
            target: 3699,
            after_ms: 300_000,
        }],
        // data/items/items.xml:31744-31747.
        13_843 => &[DecayStep {
            target: 13_842,
            after_ms: 14_400_000,
        }],
        // data/items/items.xml:32372-32384.
        14_031 => &[
            DecayStep {
                target: 14_032,
                after_ms: 72_000_000,
            },
            DecayStep {
                target: 306,
                after_ms: 72_000_000,
            },
        ],
        14_032 => &[DecayStep {
            target: 306,
            after_ms: 72_000_000,
        }],
        // data/items/items.xml:573-578, 613-618.
        324 => &[
            DecayStep {
                target: 330,
                after_ms: 72_000_000,
            },
            DecayStep {
                target: 306,
                after_ms: 72_000_000,
            },
        ],
        330 => &[DecayStep {
            target: 306,
            after_ms: 72_000_000,
        }],
        _ => &[],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedTrigger {
    Use,
    AddedToPot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedAction {
    Plant { target: u32 },
    Preserve,
}

/// ice_flower_seeds.lua:3-17; movements/seeds.lua:3-17.
/// Ordinary seeds only trigger on addition to the pot; they have no USE action.
pub fn seed_action(seed: u32, target: u32, trigger: SeedTrigger) -> Option<SeedAction> {
    match (seed, target, trigger) {
        (ICE_FLOWER_SEEDS, EMPTY_POT, _) => Some(SeedAction::Plant { target: 14_031 }),
        (ICE_FLOWER_SEEDS, 14_029, SeedTrigger::Use) => Some(SeedAction::Preserve),
        (647, EMPTY_POT, SeedTrigger::AddedToPot) => Some(SeedAction::Plant { target: 324 }),
        _ => None,
    }
}

/// The secondary operand is the seeds; the source operand is the pot, retaining its slot.
/// Existing consume/transform custody and audit remain the transaction owner.
pub fn seed_conversion(
    native: &NativeGameplayState,
    seeds: &TypedDefinitionRef,
    pot: &TypedDefinitionRef,
) -> Option<CookingConversion> {
    let id = |definition: &TypedDefinitionRef| {
        if definition.family != "Item"
            || native
                .item_policy(&definition.production_key, &definition.revision_ref)
                .is_none()
        {
            return None;
        }
        definition
            .production_key
            .strip_prefix("oteryn:item.tibia.i")?
            .parse::<u32>()
            .ok()
    };
    let SeedAction::Plant { target } = seed_action(id(seeds)?, id(pot)?, SeedTrigger::Use)? else {
        return None;
    };
    let record = native.item_policy_for_source_id(target)?.record();
    Some(CookingConversion {
        recipe: "oteryn:planting.ice-flower".into(),
        outputs: vec![(
            TypedDefinitionRef {
                family: record.production_definition.family.clone(),
                production_key: record.production_definition.production_key.clone(),
                revision_ref: record.production_definition.revision_ref.clone(),
            },
            1,
        )],
    })
}

/// Crystal201189f data/scripts/actions/items/ice_flower_seeds.lua:4-14.
/// The carried seed and the target transition share the existing gathering transaction.
/// Target14029 consumes the seed and awards Preservationist without changing its appearance.
/// Ordinary seed647 is a DROP callback (movements/seeds.lua), so USE never admits it here.
pub fn planting_attempt(seed: u16, target: u16) -> Option<crate::fishing::GatheringAttempt> {
    let after = match (seed, target) {
        (13844, 14029) => 14029,
        (13844, 306) => 14031,
        _ => return None,
    };
    Some(crate::fishing::GatheringAttempt {
        after: Some(after),
        yield_item: None,
        consumes_bait: true,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MapHarvest {
    pub after: u16,
    pub yield_item: Option<(u16, u16)>,
    pub source_ground: bool,
    pub decay: Option<(u32, i64)>,
}

/// Crystal201189f actions/items/blueberry_bush.lua:4-7, ice_flower.lua:4-13.
pub fn map_harvest(source: u16, roll: u32) -> Option<MapHarvest> {
    if !(1..=100_000).contains(&roll) {
        return None;
    }
    Some(match source {
        3699 => MapHarvest {
            after: 3700,
            yield_item: Some((3588, 3)),
            source_ground: true,
            decay: Some((3699, 300_000)),
        },
        13842 => MapHarvest {
            after: 13843,
            yield_item: (roll <= 25_000).then_some((13844, 1)),
            // Game::transformItem starts decay at game.cpp:3144 even without item:decay().
            // The pinned items.xml defines harvested ice flower as 14,400s -> 13842.
            source_ground: false,
            decay: Some((13842, 14_400_000)),
        },
        _ => return None,
    })
}

/// A flower has one draw for its real physical revision, independent of which player or
/// handle reaches it first. A failed/retried command cannot reroll that same flower.
pub fn map_harvest_roll(
    content: [u8; 32],
    world: [u8; 16],
    channel: [u8; 16],
    placement: &[u8],
    epoch: i64,
    revision: i64,
) -> Option<u32> {
    use sha2::{Digest, Sha256};
    let zone = u64::MAX - u64::MAX % 100_000;
    for counter in 0_u32..32 {
        let hash = Sha256::new()
            .chain_update(b"oteryn:map-harvest:v1")
            .chain_update(content)
            .chain_update(world)
            .chain_update(channel)
            .chain_update(placement)
            .chain_update(epoch.to_be_bytes())
            .chain_update(revision.to_be_bytes())
            .chain_update(counter.to_be_bytes())
            .finalize();
        let draw = u64::from_be_bytes(hash[..8].try_into().ok()?);
        if draw < zone {
            return u32::try_from(draw % 100_000 + 1).ok();
        }
    }
    None
}

/// Every subsequent source XML stage, measured from the previous deadline.
pub fn planting_decays(after: u16) -> Vec<(u32, i64)> {
    decay_steps(u32::from(after))
        .iter()
        .map(|step| (step.target, step.after_ms))
        .collect()
}

/// Crystal201189f dryad_garden.lua:19 and utils_definitions.hpp:109.
/// The source teleports first, then emits CONST_ME_WATERSPLASH at the destination.
pub const DRYAD_GARDEN_EFFECT_ID: u16 = 54;
/// Captured only: Crystal201189f ice_flower_seeds.lua:6,13 uses CONST_ME_POFF.
pub const SEED_POFF_EFFECT_ID: u16 = 3;

/// Crystal ice_flower_seeds.lua:13: POFF belongs to the physical pot position.
/// An inventory position is not a world tile and therefore emits no world effect.
pub fn seed_effects(ground: Option<crate::novelty_items::GroundEffect>) -> serde_json::Value {
    let mut effects = serde_json::json!({
        "kind": "transform", "source_item_id": ICE_FLOWER_SEEDS,
        "donor": "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/scripts/actions/items/ice_flower_seeds.lua",
        "cues": if ground.is_some() {
            vec![serde_json::json!({"binding": "appearance:effect/poff", "effect_id": SEED_POFF_EFFECT_ID})]
        } else { Vec::new() },
        "private_speech": false, "require_head": false, "require_player_target": false,
        "achievement_progress": [],
    });
    if let Some(ground) = ground {
        effects["ground"] = serde_json::json!(ground);
    }
    effects
}

/// Crystal ice_flower_seeds.lua:4-8: the full plant remains and the account grants once.
pub fn preservation_effects(
    ground: Option<crate::novelty_items::GroundEffect>,
) -> serde_json::Value {
    let mut effects = seed_effects(ground);
    effects["achievement"] = serde_json::json!("oteryn:achievement/preservationist");
    effects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worldchg_thawing_stable_source_key_excludes_ordinary_and_noncanonical_ids()
    -> Result<(), &'static str> {
        let key = world_change_item_key(13842, 32255, 31079, 5).ok_or("source key")?;
        assert_eq!(
            world_change_item_source(key),
            Some((13842, 32255, 31079, 5))
        );
        assert_eq!(world_change_item_key(13842, 32255, 31079, 5), Some(key));
        assert_eq!(world_change_item_source((1_u64 << 63) | 1), None);
        assert_eq!(world_change_item_source(1), None);
        assert_eq!(world_change_item_source(key | (1_u64 << 60)), None);
        assert_eq!(world_change_item_key(13842, 32255, 31079, 16), None);
        assert_eq!(world_change_item_key(0, 32255, 31079, 5), None);
        Ok(())
    }

    #[test]
    fn event_planting_use_and_add_item_have_distinct_source_actions() {
        assert_eq!(
            seed_action(13_844, 306, SeedTrigger::Use),
            Some(SeedAction::Plant { target: 14_031 })
        );
        assert_eq!(
            seed_action(13_844, 306, SeedTrigger::AddedToPot),
            Some(SeedAction::Plant { target: 14_031 })
        );
        assert_eq!(
            seed_action(647, 306, SeedTrigger::AddedToPot),
            Some(SeedAction::Plant { target: 324 })
        );
        assert_eq!(seed_action(647, 306, SeedTrigger::Use), None);
        assert_eq!(
            seed_action(13_844, 14_029, SeedTrigger::Use),
            Some(SeedAction::Preserve)
        );
        assert_eq!(seed_action(13_844, 14_029, SeedTrigger::AddedToPot), None);
        for (seed, target) in [(1, 306), (13_844, 1), (647, 14_029), (13_844, 14_031)] {
            assert_eq!(seed_action(seed, target, SeedTrigger::Use), None);
        }
    }

    #[test]
    fn event_planting_ice_flower_has_exact_twenty_five_percent_boundary() {
        assert_eq!(
            (1..=100_000)
                .filter(|roll| map_harvest(13_842, *roll)
                    .is_some_and(|harvest| harvest.yield_item.is_some()))
                .count(),
            25_000
        );
        assert_eq!(
            map_harvest(13_842, 25_000).and_then(|harvest| harvest.yield_item),
            Some((13_844, 1))
        );
        assert_eq!(
            map_harvest(13_842, 25_001).and_then(|harvest| harvest.yield_item),
            None
        );
        for roll in [0, 100_001, u32::MAX] {
            assert_eq!(map_harvest(13_842, roll), None);
        }
        assert_eq!(map_harvest(13_843, 1), None);
        assert_eq!(map_harvest(3700, 1), None);
        assert_eq!(
            map_harvest(3699, 1),
            Some(MapHarvest {
                after: 3700,
                yield_item: Some((3588, 3)),
                source_ground: true,
                decay: Some((3699, 300_000)),
            })
        );
    }

    #[test]
    fn event_planting_decay_is_source_timed_and_restores_empty_pot() {
        assert_eq!(
            decay_steps(14_031),
            [
                DecayStep {
                    target: 14_032,
                    after_ms: 72_000_000
                },
                DecayStep {
                    target: 306,
                    after_ms: 72_000_000
                },
            ]
        );
        assert_eq!(
            decay_steps(3700),
            [DecayStep {
                target: 3699,
                after_ms: 300_000
            }]
        );
        assert_eq!(
            decay_steps(13_843),
            [DecayStep {
                target: 13_842,
                after_ms: 14_400_000
            }]
        );
        assert!(decay_steps(306).is_empty());
        assert!(decay_steps(14_029).is_empty());
    }

    #[test]
    fn event_planting_captures_exact_effect_metadata_without_queueing() {
        let ground = crate::novelty_items::GroundEffect {
            x: 10,
            y: 20,
            floor: 7,
        };
        let effects = seed_effects(Some(ground));
        assert_eq!(effects["source_item_id"], 13844);
        assert_eq!(
            effects["cues"],
            serde_json::json!([{"binding":"appearance:effect/poff","effect_id":3}])
        );
        assert_eq!(
            effects["donor"],
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/scripts/actions/items/ice_flower_seeds.lua"
        );
        assert_eq!(seed_effects(None)["cues"], serde_json::json!([]));
        assert_eq!(
            preservation_effects(None)["achievement"],
            "oteryn:achievement/preservationist"
        );
        assert_eq!(DRYAD_GARDEN_EFFECT_ID, 54);
    }
}
