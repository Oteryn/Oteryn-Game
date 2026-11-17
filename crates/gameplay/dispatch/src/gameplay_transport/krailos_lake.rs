//! Project the durable World lake into each Channel's source-qualified terrain.
use super::ComposedFreshAdmission;
use crate::map::{
    overlay::TilePos,
    view::{self, MapFacts},
};
use oteryn_gameplay_quests::quest::krailos_lake::Phase;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct LakeTile {
    x: u16,
    y: u16,
    dry: u16,
}

#[derive(Deserialize)]
struct LakeTerrain {
    floor: i8,
    dirty_water: u16,
    shallow_water: u16,
    tiles: Vec<LakeTile>,
}

fn terrain() -> Option<&'static LakeTerrain> {
    static TERRAIN: OnceLock<Option<LakeTerrain>> = OnceLock::new();
    TERRAIN
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../../../../../content/quests/krailos-lake.json"
            ))
            .ok()
        })
        .as_ref()
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn refresh_krailos_lake(&self) {
        let (Some(map), Some(source)) = (self.bundle_map(), terrain()) else {
            return;
        };
        if !source.tiles.iter().any(|tile| {
            map.overlay
                .base()
                .tile(tile.x, tile.y, source.floor)
                .is_some()
        }) {
            return;
        }
        // Order snapshots with the Channel's terrain USE and publication turns.
        let Some(_permit) = self.spell_lane_permit().await else {
            return;
        };
        let Ok(Some(phase)) = self
            .root
            .krailos_lake_phase(self.world_id, self.reset_epoch)
            .await
        else {
            return;
        };
        let runtime = self.runtime.lock().await;
        if runtime.binding().world_id() != self.world_id
            || runtime.binding().channel_id() != Some(self.channel_id)
            || runtime.owner_fence().is_err()
        {
            return;
        }
        let base = map.facts.base_only();
        let live = map.facts.pinned();
        for tile in &source.tiles {
            let pos = TilePos {
                x: tile.x,
                y: tile.y,
                floor: source.floor,
            };
            let Some(key) = view::placement_key(pos, 0) else {
                continue;
            };
            let Some(id) = map
                .overlay
                .base()
                .tile(pos.x, pos.y, pos.floor)
                .and_then(|tile| tile.ids().first().copied())
            else {
                continue;
            };
            let Some(before) = base
                .base_entry(pos, 0, id)
                .filter(|facts| facts.appearance_id == tile.dry)
            else {
                continue;
            };
            let appearance = match phase {
                Phase::Dry => tile.dry,
                Phase::DirtyWater => source.dirty_water,
                Phase::ShallowWater => source.shallow_water,
            };
            if live.base_hidden(key)
                || !live.base_entry(pos, 0, id).is_some_and(|current| {
                    current.appearance_id != appearance
                        && [tile.dry, source.dirty_water, source.shallow_water]
                            .contains(&current.appearance_id)
                })
            {
                continue;
            }
            let after = if phase == Phase::Dry {
                Some(before)
            } else {
                map.facts
                    .quest_callback_replacement(key, appearance)
                    .map(|water| view::EntryFacts {
                        definition: water.definition,
                        appearance_id: water.appearance_id,
                        ..before
                    })
            };
            if let Some(after) = after
                && let Some(revision) = live.object_revision(key).checked_add(1)
            {
                map.facts
                    .install_gathering_state(key, revision, before, Some(after));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_gameplay_quests::quest::{krailos_lake, loader, world_callbacks};

    #[test]
    fn krailos_water_content_binds_the_production_npc_and_map_tools() -> Result<(), String> {
        let terrain = terrain().ok_or("Krailos lake terrain")?;
        let loaded =
            loader::load_embedded_quest_state("content-1").map_err(|e| format!("{e:?}"))?;
        let catalogue = loaded.catalogue();
        for key in [
            krailos_lake::START,
            krailos_lake::CAPTURE,
            krailos_lake::POUR,
            krailos_lake::PURIFY,
        ] {
            assert!(catalogue.transition(key).is_some(), "{key}");
        }
        let callbacks = world_callbacks::load_for(catalogue)?;
        for key in [krailos_lake::POUR, krailos_lake::PURIFY] {
            let binding = callbacks
                .iter()
                .find(|callback| callback.key == key)
                .and_then(|callback| callback.selector.native.as_ref())
                .ok_or_else(|| format!("{key} native selector"))?;
            assert!(
                terrain
                    .tiles
                    .iter()
                    .any(|tile| (tile.x, tile.y, terrain.floor, tile.dry)
                        == (
                            binding.position.x,
                            binding.position.y,
                            binding.position.floor,
                            binding.appearance
                        ))
            );
        }
        Ok(())
    }
}
