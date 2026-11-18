//! Crystal 201189f8, quests/roshamuul_quest/actions_resonance_chamber.lua.

use super::{
    CatalogueSpawnMode, RaidArea, RaidDefinition, RaidEndPolicy, RaidError, RaidPolicy,
    RaidPosition, RaidRandom, SelectedRaid, Wave, WaveEntry,
};
use oteryn_gameplay_content::content::ProductionKey;

pub const RAID_KEY: &str = "raid:roshamuul.silencer-plateau";
pub const CHAMBER_KEY: &str = "oteryn:item.tibia.i20201";
pub const COOLDOWN_MS: u32 = 30 * 60_000;
const WAVE_INTERVAL_MS: u32 = 60_000;

pub const SOURCE_SPAWN_AREA: [[i32; 3]; 2] = [[33637, 32516, 5], [33664, 32537, 5]];
pub const SOURCE_SPECTATOR_AREA: [[i32; 3]; 2] = [[33637, 32516, 5], [33663, 32538, 5]];

/// Bind the donor's inclusive draw to the persisted command occurrence.
pub fn successful_use(seed: [u8; 32], occurrence: [u8; 16]) -> Result<bool, RaidError> {
    Ok(RaidRandom::new(seed, occurrence).amount(4, 0, 0, 10_000)? >= 7_000)
}

pub fn select(
    seed: [u8; 32],
    firing: [u8; 16],
    from: RaidPosition,
    to: RaidPosition,
) -> Result<SelectedRaid, RaidError> {
    if from.floor != to.floor || from.x > to.x || from.y > to.y {
        return Err(RaidError::InvalidDefinition);
    }
    let anchor = ProductionKey::new(RAID_KEY).map_err(|_| RaidError::InvalidDefinition)?;
    let random = RaidRandom::new(seed, firing);
    let mut waves = Vec::new();
    let mut spawn_modes = std::collections::BTreeMap::new();
    for (index, (species, min, max)) in [
        ("oteryn:creature.silencer", 8, 15),
        ("oteryn:creature.silencer", 11, 18),
        ("oteryn:creature.silencer", 8, 15),
        ("oteryn:creature.sight_of_surrender", 3, 8),
    ]
    .into_iter()
    .enumerate()
    {
        let ordinal = u16::try_from(index).map_err(|_| RaidError::Exhausted)?;
        waves.push(Wave {
            delay_ms: (u32::from(ordinal) + 1) * WAVE_INTERVAL_MS,
            entries: vec![WaveEntry {
                creature: ProductionKey::new(species).map_err(|_| RaidError::InvalidDefinition)?,
                count: random.amount(ordinal, 0, min, max)?,
                area_anchor: anchor.clone(),
            }],
        });
        spawn_modes.insert((ordinal, 0), CatalogueSpawnMode::Area);
    }
    // The runner adds this duration after the last wave. Crystal cleans up
    // thirty minutes from the successful chamber use.
    let despawn_after_ms = COOLDOWN_MS - 4 * WAVE_INTERVAL_MS;
    let definition = RaidDefinition {
        policy: RaidPolicy::SourceScheduled,
        despawn_after_ms: Some(despawn_after_ms),
        announcements: Vec::new(),
        waves,
    };
    definition.validate()?;
    Ok(SelectedRaid {
        definition,
        variant_index: 0,
        areas: [(anchor, RaidArea { from, to })].into(),
        boss_entries: Default::default(),
        spawn_modes,
        end_policy: RaidEndPolicy::DespawnAfter {
            after_last_wave_ms: despawn_after_ms,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silencer_plateau_waves_end_thirty_minutes_after_use() -> Result<(), RaidError> {
        let from = RaidPosition {
            x: 10,
            y: 20,
            floor: -5,
        };
        let to = RaidPosition {
            x: 37,
            y: 41,
            floor: -5,
        };
        let raid = select([1; 32], [2; 16], from, to)?;
        for (wave, (delay, range)) in raid.definition.waves.iter().zip([
            (60_000, 8..=15),
            (120_000, 11..=18),
            (180_000, 8..=15),
            (240_000, 3..=8),
        ]) {
            assert_eq!(wave.delay_ms, delay);
            assert!(range.contains(&wave.entries[0].count));
        }
        assert_eq!(raid.definition.deadline(10)?, 1_800_010);
        Ok(())
    }
}
