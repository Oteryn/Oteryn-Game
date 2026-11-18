//! Spatial admission for dialogue shared by multiple placements of one NPC.
use crate::quest::world_callbacks::Position;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DialoguePlacement {
    pub npc_floor_minimum: i8,
    pub npc_floor_maximum: i8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_area: Option<DialogueArea>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DialogueArea {
    pub minimum: Position,
    pub maximum: Position,
}

impl DialoguePlacement {
    pub fn valid(&self) -> bool {
        (-15..=0).contains(&self.npc_floor_minimum)
            && (self.npc_floor_minimum..=0).contains(&self.npc_floor_maximum)
            && self.player_area.as_ref().is_none_or(|area| {
                (self.npc_floor_minimum..=self.npc_floor_maximum).contains(&area.minimum.floor)
                    && area.minimum.floor == area.maximum.floor
                    && area.minimum.x <= area.maximum.x
                    && area.minimum.y <= area.maximum.y
            })
    }

    pub fn admits(&self, npc_floor: i8, player: Position) -> bool {
        (self.npc_floor_minimum..=self.npc_floor_maximum).contains(&npc_floor)
            && self.player_area.as_ref().is_none_or(|area| {
                (area.minimum.x..=area.maximum.x).contains(&player.x)
                    && (area.minimum.y..=area.maximum.y).contains(&player.y)
                    && area.minimum.floor == player.floor
            })
    }
}
