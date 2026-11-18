//! Crystal 201189f8 actions_flower_puzzle_lever.lua and movements_shulgrax_lever.lua.
use super::*;
use crate::map::facts::{QuestCallbackProjection, QuestTimedPassage};
use crate::map::view::compose_tile;
use std::collections::HashMap;

// The retained native map has the lever at top-level ordinal 2 and omits AID 34300.
const LEVER: TilePos = garden(33476, 32700);
const PORTAL: TilePos = garden(33477, 32698);
const WINDOW_US: u64 = 30_000_000;
const FLOWERS: [(TilePos, u16); 9] = [
    (garden(33455, 32708), 3676),
    (garden(33459, 32707), 3676),
    (garden(33460, 32708), 3676),
    (garden(33456, 32692), 3678),
    (garden(33458, 32692), 3678),
    (garden(33460, 32693), 3678),
    (garden(33457, 32686), 3677),
    (garden(33459, 32686), 3677),
    (garden(33456, 32687), 3677),
];
const GATES: [(TilePos, u16, u16); 4] = [
    (garden(33475, 32698), 1635, 7144),
    (garden(33476, 32698), 7144, 1635),
    (garden(33478, 32698), 7144, 1634),
    (garden(33479, 32698), 1634, 7144),
];

const fn garden(x: u16, y: u16) -> TilePos {
    TilePos { x, y, floor: -14 }
}

#[derive(Default)]
pub(super) struct FlowerPuzzle {
    shared_until: u64,
    accepted: HashMap<GameSessionId, u64>,
}

impl FlowerPuzzle {
    pub(super) fn destination(
        &self,
        at: TilePos,
        session: GameSessionId,
        now: u64,
    ) -> Option<TilePos> {
        if at != PORTAL || now >= self.shared_until {
            return None;
        }
        Some(
            if self.accepted.get(&session).is_some_and(|due| now < *due) {
                TilePos {
                    x: 33436,
                    y: 32800,
                    floor: -13,
                }
            } else {
                garden(PORTAL.x, PORTAL.y + 1)
            },
        )
    }
}

fn has_item(map: &BundleMap, at: TilePos, appearance: u16) -> bool {
    compose_tile(&map.overlay, &map.facts, at)
        .ok()
        .flatten()
        .is_some_and(|tile| {
            tile.entries
                .iter()
                .any(|entry| entry.facts.appearance_id == appearance)
        })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_flower_lever(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        revision: u64,
        resolved: &super::super::world_map::MapObjectTarget<()>,
    ) -> Option<UseOutcome> {
        if resolved.pos != LEVER || resolved.ordinal != 2 {
            return None;
        }
        let result = self
            .activate_flower_lever(actor, command, map, resolved.placement_key, revision)
            .await;
        Some(UseOutcome {
            disposition: result.unwrap_or(UseDisposition::Rejected),
            ..UseOutcome::rejected()
        })
    }

    async fn activate_flower_lever(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        key: u64,
        revision: u64,
    ) -> Option<UseDisposition> {
        let _permit = self.spell_lane_permit().await?;
        let runtime = self.runtime.lock().await;
        let session = command.game_session_id;
        if !runtime
            .player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
            || map.facts.object_revision(key) != revision
            || map.facts.base_hidden(key)
        {
            return None;
        }
        let at = native(runtime.read_actor_position(actor).ok()?.position())?;
        if !super::super::map_door::reachable(at, LEVER) {
            return Some(UseDisposition::TooFar);
        }
        let lever = compose_tile(&map.overlay, &map.facts, LEVER).ok()??.entries.into_iter().find(|entry|
            matches!(entry.source, crate::map::view::EntrySource::Base { placement_key, .. } if placement_key == key)
        )?.facts;
        let appearance = match lever.appearance_id {
            9110 => 9111,
            9111 => 9110,
            _ => return None,
        };
        let after = map.facts.quest_callback_replacement(key, appearance)?;
        let mut puzzle = self.states.quest_callbacks.flower_puzzle.lock().ok()?;
        let now = self.owner_now().get();
        puzzle.accepted.retain(|_, due| now < *due);
        if appearance == 9111 && !FLOWERS.iter().all(|&(at, item)| has_item(map, at, item)) {
            self.states
                .chat
                .push_quest_feedback(session, "The order of the flowers in the garden are wrong.");
            return Some(UseDisposition::Rejected);
        }
        let newly_accepted = appearance == 9111 && !puzzle.accepted.contains_key(&session);
        let due = now.checked_add(WINDOW_US)?;
        let starts_window = newly_accepted && now >= puzzle.shared_until;
        let projection = if starts_window {
            let mut entries = Vec::new();
            for (position, from, to) in GATES {
                let tile = compose_tile(&map.overlay, &map.facts, position).ok()??;
                for entry in tile.entries {
                    if let crate::map::view::EntrySource::Base { placement_key, .. } = entry.source
                        && entry.facts.appearance_id == from
                    {
                        entries.push(QuestTimedPassage {
                            key: placement_key,
                            before: entry.facts,
                            expected_revision: map.facts.object_revision(placement_key),
                            after: Some(map.facts.quest_callback_replacement(placement_key, to)?),
                            passable: to != 7144,
                        });
                        break;
                    }
                }
            }
            let presence = if has_item(map, PORTAL, 5068) {
                None
            } else {
                Some((PORTAL, map.facts.movement_item_facts(5068)?))
            };
            Some(QuestCallbackProjection::TimedPassages {
                due,
                entries,
                presence,
                source_reset: None,
            })
        } else {
            None
        };
        if !map.facts.apply_quest_callback(
            key,
            revision,
            after,
            &[],
            &[],
            false,
            projection.as_ref(),
        ) {
            return None;
        }
        if starts_window {
            puzzle.shared_until = due;
            for x in 33475..=33479 {
                map.facts.emit_movement_effect(garden(x, 32698), 3);
            }
        }
        if newly_accepted {
            puzzle.accepted.insert(session, due);
            self.states.chat.push_quest_feedback(session, "A portal forms as two beams of a strange construction dividing this room move towards each other.");
        } else if appearance == 9111 {
            self.states
                .chat
                .push_quest_feedback(session, "The portal still activated.");
        }
        Some(UseDisposition::Committed)
    }
}
