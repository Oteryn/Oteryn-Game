//! Gem rolls use the existing corpse and personal boss loot owners.
use crate::combat::{
    LootDefinitionRef, LootTableDefinition, LootTableEntry, check_corpse_container_capacity,
};
use crate::content::creature_reward::CreatureRewardTable;
use crate::domain::forge_creature::ForgeCreature;
use crate::domain::loot_data::LootEntryDetails;
use crate::domain::uniform_draw::uniform_draw;
use oteryn_gameplay_items::loot_plan::{LootPlanDeathKey, plan_creature_loot};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
pub(super) enum Source {
    Influenced,
    Fiendish,
    Archfoe,
}

impl From<ForgeCreature> for Source {
    fn from(creature: ForgeCreature) -> Self {
        match creature {
            ForgeCreature::Influenced(_) => Self::Influenced,
            ForgeCreature::Fiendish => Self::Fiendish,
        }
    }
}

// Guardian, Marksman, Sage, Mystic and Spiritualist, in Crystal's draw order.
const IDS: [[u32; 5]; 3] = [
    [44_602, 44_605, 44_608, 44_611, 49_371],
    [44_603, 44_606, 44_609, 44_612, 49_372],
    [44_604, 44_607, 44_610, 44_613, 49_373],
];

pub(super) fn roll(
    table: &CreatureRewardTable,
    digest: [u8; 32],
    death: [u8; 16],
    recipient: [u8; 16],
    source: Source,
) -> Option<Vec<(LootDefinitionRef, u32)>> {
    let root = GameplayDecisionRoot::from_bytes(
        Sha256::new()
            .chain_update(digest)
            .chain_update(recipient)
            .finalize()
            .into(),
    );
    let occurrence = DecisionOccurrenceId::from_bytes(death);
    let draw = |purpose: &str, index: u64, high: i64| {
        let seed = deterministic_decision_u64(&root, occurrence, purpose, index).ok()?;
        usize::try_from(uniform_draw(seed, 1, high)).ok()
    };
    roll_with(source, draw, |id| table.admitted_source_item(id).cloned())
}

fn roll_with<T: Eq>(
    source: Source,
    draw: impl Fn(&str, u64, i64) -> Option<usize>,
    admitted_item: impl Fn(u32) -> Option<T>,
) -> Option<Vec<(T, u32)>> {
    // Crystal, Global silent on exact chances and repeats.
    // 201189f8 data/libs/functions/gematelier.lua:10-45,68-86.
    let chances = match source {
        Source::Influenced => [9_000, 0, 0],
        Source::Fiendish => [3_000, 3_000, 9_000],
        Source::Archfoe => [0, 9_000, 3_000],
    };
    let mut items: Vec<(T, u32)> = Vec::new();
    for (tier, ((ids, chance), attempts)) in IDS.iter().zip(chances).zip([2_u64, 2, 1]).enumerate()
    {
        if chance == 0 {
            continue;
        }
        for attempt in 0..attempts {
            let index = tier as u64 * 2 + attempt;
            if draw("GEM_ATELIER_DROP_CHANCE", index, 100_000)? > chance {
                break;
            }
            let item_index =
                draw("GEM_ATELIER_DROP_VOCATION", index, ids.len() as i64)?.checked_sub(1)?;
            let Some(item) = admitted_item(*ids.get(item_index)?) else {
                break;
            };
            if let Some((_, count)) = items.iter_mut().find(|(key, _)| *key == item) {
                *count += 1;
            } else {
                items.push((item, 1));
            }
        }
    }
    Some(items)
}

// Add gems after ordinary loot modifiers. Donor corpse:addLoot runs after ordinary
// loot and can only fill available slots; a gem must never refuse the existing corpse.
pub(super) fn append_corpse_gems(
    table: &mut LootTableDefinition,
    death: LootPlanDeathKey,
    reference: &LootDefinitionRef,
    gems: &[(LootDefinitionRef, u32)],
) {
    for (item, count) in gems {
        table.entries.push(LootTableEntry {
            item: item.clone(),
            min_count: *count,
            max_count: *count,
            probability_ppm: Some(1_000_000),
            details: LootEntryDetails {
                // The callback already rolled; guarantee success even at minimum jitter.
                rated_probability_ppm: Some(2_000_000),
                stackable: true,
                ..Default::default()
            },
        });
        let fits = plan_creature_loot(death, reference, table).is_ok_and(|plan| {
            check_corpse_container_capacity(
                plan.entries
                    .iter()
                    .filter(|entry| entry.parent.is_none())
                    .count(),
            )
            .is_ok()
        });
        if !fits {
            table.entries.pop();
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rates_and_repeat_limits_follow_the_donor_table() {
        for (source, chances) in [
            (Source::Influenced, [9_000, 0, 0]),
            (Source::Fiendish, [3_000, 3_000, 9_000]),
            (Source::Archfoe, [0, 9_000, 3_000]),
        ] {
            let at_threshold = |purpose: &str, index: u64, _: i64| {
                if purpose == "GEM_ATELIER_DROP_CHANCE" {
                    Some(chances[index as usize / 2])
                } else {
                    Some(1)
                }
            };
            let expected: Vec<_> = IDS
                .iter()
                .zip(chances)
                .zip([2, 2, 1])
                .filter_map(|((ids, chance), limit)| (chance != 0).then_some((ids[0], limit)))
                .collect();
            assert_eq!(roll_with(source, at_threshold, Some), Some(expected));
            let miss = |purpose: &str, index: u64, upper: i64| {
                assert_eq!(purpose, "GEM_ATELIER_DROP_CHANCE");
                assert_eq!(index % 2, 0, "a failed draw ends that tier");
                at_threshold(purpose, index, upper).map(|threshold| threshold + 1)
            };
            assert_eq!(roll_with(source, miss, Some), Some(Vec::new()));
        }
    }

    #[test]
    fn corpse_gem_fills_the_last_slot_without_refusing_existing_loot() {
        use crate::combat::LootSelectionAlgorithm;
        use crate::domain::loot_data::GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX;

        let reference = LootDefinitionRef::new("Loot", "oteryn:loot.test", "definition-r1");
        let ordinary = LootTableEntry {
            item: LootDefinitionRef::new("Item", "oteryn:item.tibia.i3031", "definition-r1"),
            min_count: 1,
            max_count: 1,
            probability_ppm: Some(1_000_000),
            details: LootEntryDetails {
                stackable: true,
                rated_probability_ppm: Some(2_000_000),
                ..Default::default()
            },
        };
        let mut table = LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: vec![ordinary; GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX - 1],
        };
        let death = LootPlanDeathKey::new([1; 16], [2; 16], 1, 1, 1);
        let gem = |id| {
            LootDefinitionRef::new("Item", format!("oteryn:item.tibia.i{id}"), "definition-r1")
        };
        let lesser = gem(IDS[0][0]);
        let regular = gem(IDS[1][0]);
        append_corpse_gems(
            &mut table,
            death,
            &reference,
            &[(lesser.clone(), 2), (regular, 1)],
        );
        assert_eq!(table.entries.len(), GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX);
        assert_eq!(
            table
                .entries
                .last()
                .map(|entry| (&entry.item, entry.max_count)),
            Some((&lesser, 2))
        );
    }
}
