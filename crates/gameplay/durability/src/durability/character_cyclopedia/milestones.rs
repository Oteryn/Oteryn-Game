//! Crystal 201189f data/scripts/discovery_system.lua:527-562.
use super::{MapRequest, storage::CanonicalProgress};
use crate::domain::CharacterId;
use crate::durability::DurabilityError;
use crate::durability::account_achievement::grant_missing;
use crate::durability::character_appearance::quest_grants::{
    grant_cosmetics_outfit, outfit_matches_cosmetics,
};
use oteryn_gameplay_quests::quest::reward_callbacks::OutfitReward;
use oteryn_world_bundle::subarea::{MAX_AREAS, Table};
use std::collections::BTreeMap;

const WIDELY_TRAVELLED: &str = "oteryn:achievement/widely_travelled";
const MEASURING_THE_WORLD: &str = "oteryn:achievement/measuring_the_world";
const DISCOVERER: &str = "oteryn:outfit.discoverer";

/// Main areas and their canonical children taken from the admitted World bundle.
#[derive(Debug, Clone)]
pub struct DiscoveryAreas {
    mains: BTreeMap<u32, Vec<u32>>,
}

impl DiscoveryAreas {
    pub fn from_table(table: &Table) -> Option<Self> {
        let mut mains: BTreeMap<_, Vec<_>> = table
            .areas
            .iter()
            .filter(|area| area.parent.is_none())
            .map(|area| (area.id, Vec::new()))
            .collect();
        if mains.is_empty() || table.areas.len() > MAX_AREAS {
            return None;
        }
        for area in &table.areas {
            if let Some(parent) = area.parent {
                mains.get_mut(&parent)?.push(area.id);
            }
        }
        Some(Self { mains })
    }

    pub(super) fn subarea_count(&self) -> usize {
        self.mains.values().map(Vec::len).sum()
    }

    pub(super) fn subareas(&self, main: u32) -> &[u32] {
        self.mains.get(&main).map_or(&[], Vec::as_slice)
    }

    pub(super) fn all_subareas(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.mains
            .iter()
            .flat_map(|(&main, ids)| ids.iter().map(move |&id| (main, id)))
    }

    /// Every main area contributes to the denominator. Only a nonempty main
    /// area with all its canonical children discovered contributes completion.
    pub(in crate::durability) fn completion(
        &self,
        is_discovered: impl Fn(u32) -> bool,
    ) -> (usize, u8) {
        let done = self
            .mains
            .values()
            .filter(|ids| !ids.is_empty() && ids.iter().copied().all(&is_discovered))
            .count();
        (done, (done * 100 / self.mains.len()) as u8)
    }
}

pub(super) async fn grant(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &MapRequest,
    progress: &CanonicalProgress,
) -> Result<(), DurabilityError> {
    let (done, percent) = request
        .areas
        .completion(|id| progress.subareas.get(&id).is_some_and(|state| state.discovered));
    if done == 0 {
        return Ok(());
    }

    let (achievements, addons): (&[&str], u8) = match percent {
        100 => (&[WIDELY_TRAVELLED, MEASURING_THE_WORLD], 3),
        50..=99 => (&[WIDELY_TRAVELLED], 1),
        _ => (&[WIDELY_TRAVELLED], 0),
    };
    if let Some(cosmetics) = &request.cosmetics {
        let reward = OutfitReward {
            outfit: DISCOVERER.into(),
            addons,
            sex: None,
        };
        if outfit_matches_cosmetics(cosmetics, &reward) {
            grant_cosmetics_outfit(tx, character, &request.occurrence, &reward).await?;
        }
    }
    grant_missing(
        tx,
        character,
        achievements,
        "oteryn:cyclopedia-discovery",
        &request.occurrence,
    )
    .await
}
