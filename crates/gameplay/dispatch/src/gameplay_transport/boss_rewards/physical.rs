use super::*;
use crate::content::item_ref::ItemDefinitionIndex;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_map::MapDefinition;

/// Crystal utils_definitions.hpp:646 ITEM_REWARD_CHEST=19250. The same physical
/// chest opens the Character's personal store in a boss room and at a depot.
pub(super) const REWARD_CHEST_KEY: &str = "oteryn:item.tibia.i19250";

pub(super) async fn current_session(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterItemFence,
) -> bool {
    super::super::FreshAdmissionStore::from_root(owner.root.clone())
        .current_session_at(fence.game_session_id)
        .await
        .is_ok_and(|(current, _)| {
            current.session_state() == super::super::GameSessionState::Active
                && super::super::item_fence_of(current) == Some(fence)
        })
}

pub(super) fn reward_chest(
    at: &super::super::bundle_map_position::BundleMapPosition<'_>,
    index: &ItemDefinitionIndex,
) -> Option<u64> {
    let mut found = Vec::new();
    for dy in -1..=1 {
        for dx in -1..=1 {
            let Some(x) = at
                .at
                .x
                .checked_add(dx)
                .and_then(|value| u16::try_from(value).ok())
            else {
                continue;
            };
            let Some(y) = at
                .at
                .y
                .checked_add(dy)
                .and_then(|value| u16::try_from(value).ok())
            else {
                continue;
            };
            let pos = TilePos {
                x,
                y,
                floor: i8::try_from(at.at.floor).ok()?,
            };
            let Some(tile) = at.map.overlay.base().tile(x, y, pos.floor) else {
                continue;
            };
            let overlay = at.map.overlay.tile(pos);
            for (ordinal, (id, _)) in tile
                .ids()
                .iter()
                .zip(tile.depths())
                .filter(|(_, depth)| **depth == 0)
                .enumerate()
            {
                let Ok(ordinal) = u8::try_from(ordinal) else {
                    continue;
                };
                if overlay
                    .as_ref()
                    .is_some_and(|value| value.is_hidden(ordinal))
                {
                    continue;
                }
                let Some(base) = at.map.facts.base_entry(pos, ordinal, *id) else {
                    continue;
                };
                let MapDefinition::Item(reference) = base.definition else {
                    continue;
                };
                let Some(definition) = super::super::item_look::typed_definition(index, reference)
                else {
                    continue;
                };
                if definition.production_key == REWARD_CHEST_KEY
                    && let Some(key) = crate::map::view::placement_key(pos, ordinal)
                {
                    found.push((dx.abs().max(dy.abs()), x, y, ordinal, key));
                }
            }
        }
    }
    found.sort_unstable();
    found.first().map(|entry| entry.4)
}
