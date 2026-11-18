//! Crystal 201189f dreamers_challenge_quest/actions_stone_teleport.lua:44-67.
use super::*;
use crate::map::facts::{QuestCallbackProjection, QuestSceneMutation};

pub(super) fn prepare(
    map: &BundleMap,
    portal: &world_callbacks::OfferingPortal,
) -> Result<Option<QuestCallbackProjection>, UseDisposition> {
    let (key, current) =
        super::board_portal::base_slot(map, tile(portal.position), &[portal.sealed, portal.opened])
            .filter(|(key, _)| !map.facts.base_hidden(*key))
            .ok_or(UseDisposition::Rejected)?;
    if current.appearance_id == portal.opened {
        return Ok(None);
    }
    let facts = map
        .facts
        .quest_callback_replacement(key, portal.opened)
        .filter(|entry| !entry.pickupable)
        .ok_or(UseDisposition::Rejected)?;
    Ok(Some(QuestCallbackProjection::Scene {
        mutations: vec![QuestSceneMutation::Replace {
            key,
            revision: map.facts.object_revision(key),
            facts,
            passable: true,
        }],
        remove_self: false,
    }))
}
