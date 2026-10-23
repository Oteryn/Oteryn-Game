//! FND-02 optional capability negotiation (CAP-NEG-1, ARCH-BATCH-ITEM-EQUIP-PACKETS §1.9).
//!
//! The server offers the registry's `offered: true` capabilities. Fresh admission selects the
//! client's supported capabilities that the server offers and whose `requires` are all selected;
//! an unknown supported ID is ignored, never selected. The selection is kept per GameSession in
//! its continuity: a resume or channel transfer keeps it unchanged and never widens it, and a
//! resume whose supported set lacks a selected capability is refused as resume-unavailable, so the
//! client falls back to fresh admission. Dispatch and domain emission read the selection: a
//! command or domain whose capability is not selected stays refused or unsent. A core domain that
//! a selected capability supersedes stays unsent too ([`SUPERSEDED`]).

use oteryn_protocol_oteryn::attack::CAPABILITY_ATTACK_V1;
use oteryn_protocol_oteryn::chat::CAPABILITY_CHAT_V1;
use oteryn_protocol_oteryn::container_tree::CAPABILITY_CONTAINER_TREE_V1;
use oteryn_protocol_oteryn::highscores::CAPABILITY_HIGHSCORES_V1;
use oteryn_protocol_oteryn::item_view::{
    CAPABILITY_ITEM_EQUIP_DROP_V1, CAPABILITY_ITEM_VIEW_MOVE_V1,
};
use oteryn_protocol_oteryn::logout::CAPABILITY_LOGOUT_V1;
use oteryn_protocol_oteryn::npc_service::CAPABILITY_NPC_SERVICE_V1;
use oteryn_protocol_oteryn::quest_log::CAPABILITY_QUEST_LOG_V1;
use oteryn_protocol_oteryn::world_clock::CAPABILITY_WORLD_LIGHT_V1;
use oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1;
use oteryn_protocol_oteryn::world_spatial::CAPABILITY_PACED_MOVEMENT_V1;
use oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES;
use oteryn_protocol_oteryn::world_weather::{
    CAPABILITY_WORLD_WEATHER_V1, CAPABILITY_WORLD_WEATHER_V1_REQUIRES,
};

/// LOOK-2: capability 28 and its command type 30, offered on a bundle World and dispatched by
/// the connection.
pub use oteryn_protocol_oteryn::capability_tables::{
    CAPABILITY_ITEM_LOOK_V1, COMMAND_TYPE_ITEM_LOOK_INTENT,
};
use oteryn_protocol_oteryn::capability_tables::{GATED, SUPERSEDED};

/// One capability the server offers, with the capabilities that must also be selected for it
/// (the registry entry's `requires`, empty when the entry has none).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OfferedCapability {
    pub id: u32,
    pub requires: &'static [u32],
}

/// CLOCK-1: capability 19 `WORLD_LIGHT_V1` (domain 18 and the VIS-2 `light` of domain 1), which
/// requires 6. All offered sets include it before capability 21.
const WORLD_LIGHT: OfferedCapability = OfferedCapability {
    id: CAPABILITY_WORLD_LIGHT_V1,
    requires: &[CAPABILITY_WORLD_SPATIAL_ENTITIES],
};

/// WEATHER-1: capability 26 `WORLD_WEATHER_V1` (domain 22), which requires 6. Offered sets
/// stay ascending, so a higher ID follows it (D1027 signed off). A connection keeps it only when
/// its authority exposes a weather source (the shipped Oteryn-designed data in production, CP
/// D1448), see [`with_weather_offer`].
pub const WORLD_WEATHER: OfferedCapability = OfferedCapability {
    id: CAPABILITY_WORLD_WEATHER_V1,
    requires: &CAPABILITY_WORLD_WEATHER_V1_REQUIRES,
};

/// The offered set of one connection: `base` without capability 26 unless `weather_bound`, plus
/// 26 at its ascending position when `weather_bound` (no source means no offer: fail closed).
pub fn with_weather_offer(
    base: &[OfferedCapability],
    weather_bound: bool,
) -> Vec<OfferedCapability> {
    let mut offered: Vec<OfferedCapability> = base
        .iter()
        .copied()
        .filter(|capability| capability.id != CAPABILITY_WORLD_WEATHER_V1)
        .collect();
    if weather_bound {
        let at = offered.partition_point(|capability| capability.id < CAPABILITY_WORLD_WEATHER_V1);
        offered.insert(at, WORLD_WEATHER);
    }
    offered
}

/// LOGOUT-WIRE-1: capability 21 `LOGOUT_V1` (command type 23), which requires nothing. Only
/// capabilities 26 and 28 are higher: every offered set ends with 21 then 26, and the bundle set
/// then offers 28 (LOOK-2).
const TUTORIAL_NOTICES: OfferedCapability = OfferedCapability {
    id: oteryn_protocol_oteryn::tutorial_notices::CAPABILITY_TUTORIAL_NOTICES_V1,
    requires: &[],
};

const LOGOUT: OfferedCapability = OfferedCapability {
    id: CAPABILITY_LOGOUT_V1,
    requires: &[],
};

/// RANK-1 (HS-WIRE-1, CP D1155): capability 9 `HIGHSCORES_V1` (command type 14), which requires
/// nothing. It reads persisted World snapshots only, so every offered set offers it.
const HIGHSCORES: OfferedCapability = OfferedCapability {
    id: CAPABILITY_HIGHSCORES_V1,
    requires: &[],
};

const QUEST_LOG: OfferedCapability = OfferedCapability {
    id: CAPABILITY_QUEST_LOG_V1,
    requires: &[],
};

/// The production offered set for a generation without a non-empty Item key set: the registry's
/// `offered: true` entries except capability 4, ascending by ID. VIS-3 offers capability 6
/// `WORLD_SPATIAL_ENTITIES`, SPEED-1 capability 13 `PACED_MOVEMENT_V1` and ATTACK-1b capability
/// 17 `ATTACK_V1`, which requires 6; CHAT-WIRE-1 offers capability 7 `CHAT_V1`; RANK-1 offers
/// capability 9 `HIGHSCORES_V1`; QUEST-RT-1 offers 16 after journal composition; CLOCK-1
/// offers capability 19 and LOGOUT-WIRE-1 capability 21.
/// A test keeps it equal to the registry and within `REGISTERED_CAPABILITY_IDS_V1`.
pub const PRODUCTION_OFFERED_CAPABILITIES: &[OfferedCapability] = &[
    OfferedCapability {
        id: CAPABILITY_WORLD_SPATIAL_ENTITIES,
        requires: &[],
    },
    OfferedCapability {
        id: CAPABILITY_CHAT_V1,
        requires: &[],
    },
    HIGHSCORES,
    OfferedCapability {
        id: CAPABILITY_PACED_MOVEMENT_V1,
        requires: &[],
    },
    QUEST_LOG,
    OfferedCapability {
        id: CAPABILITY_ATTACK_V1,
        requires: &[CAPABILITY_WORLD_SPATIAL_ENTITIES],
    },
    WORLD_LIGHT,
    LOGOUT,
    WORLD_WEATHER,
    TUTORIAL_NOTICES,
    super::guild_chat::OFFERED,
    super::party_command::OFFERED,
];

/// The offered set of a World booted from a bundle (MAP-CUTOVER-1b, NPC-CMD-1a, ITEM-LOOP-1),
/// ascending by ID: capability 3 `NPC_SERVICE_V1` (a bundle World places the NPCs), 4
/// `ITEM_VIEW_MOVE_V1`, 6, 7, 9 `HIGHSCORES_V1`, 12 `ITEM_EQUIP_DROP_V1` (requires 4), 13, 14
/// `CONTAINER_TREE_V1` (requires 4 and 12), 16 `QUEST_LOG_V1`, 17, 18 `WORLD_MAP_VIEW_V1`
/// (requires 4 and 6), 19,
/// 21, 26 and 28. A bundle World boots only against a generation whose Item key set names every
/// bundle Item. ITEM-LOOP-1 (CP D1438) offers 12 and 14 here because only a bundle World
/// composes their command type 9 equipment, ground and container destinations (equip, drop,
/// pickup and the container tree) and the command type 21 container views. Only a bundle World
/// returns it; the registry entries of 3, 12, 14, 18 and 28 stay `offered: false` and their
/// `offer_gate` names this offer. A test keeps it equal to the Item view set plus 3, 12, 14, 18
/// and 28.
pub const BUNDLE_WORLD_OFFERED_CAPABILITIES: &[OfferedCapability] = &[
    OfferedCapability {
        id: CAPABILITY_NPC_SERVICE_V1,
        requires: &[],
    },
    OfferedCapability {
        id: CAPABILITY_ITEM_VIEW_MOVE_V1,
        requires: &[CAPABILITY_WORLD_SPATIAL_ENTITIES],
    },
    OfferedCapability {
        id: CAPABILITY_WORLD_SPATIAL_ENTITIES,
        requires: &[],
    },
    OfferedCapability {
        id: CAPABILITY_CHAT_V1,
        requires: &[],
    },
    HIGHSCORES,
    OfferedCapability {
        id: CAPABILITY_ITEM_EQUIP_DROP_V1,
        requires: &[CAPABILITY_ITEM_VIEW_MOVE_V1],
    },
    OfferedCapability {
        id: CAPABILITY_PACED_MOVEMENT_V1,
        requires: &[],
    },
    OfferedCapability {
        id: CAPABILITY_CONTAINER_TREE_V1,
        requires: &[CAPABILITY_ITEM_VIEW_MOVE_V1, CAPABILITY_ITEM_EQUIP_DROP_V1],
    },
    QUEST_LOG,
    OfferedCapability {
        id: CAPABILITY_ATTACK_V1,
        requires: &[CAPABILITY_WORLD_SPATIAL_ENTITIES],
    },
    OfferedCapability {
        id: CAPABILITY_WORLD_MAP_VIEW_V1,
        requires: &[
            CAPABILITY_ITEM_VIEW_MOVE_V1,
            CAPABILITY_WORLD_SPATIAL_ENTITIES,
        ],
    },
    WORLD_LIGHT,
    LOGOUT,
    WORLD_WEATHER,
    OfferedCapability {
        id: CAPABILITY_ITEM_LOOK_V1,
        requires: &[],
    },
    TUTORIAL_NOTICES,
    super::guild_chat::OFFERED,
    OfferedCapability {
        id: oteryn_protocol_oteryn::depot::CAPABILITY_DEPOT_V1,
        requires: &[CAPABILITY_ITEM_VIEW_MOVE_V1],
    },
    super::party_command::OFFERED,
];

/// MAP-ITEM-REF-1: the production offered set with capability 4 `ITEM_VIEW_MOVE_V1`, which
/// requires 6, for a generation that pins a non-empty Item key set: the registry's whole
/// `offered: true` set. Part B offers it together with the composed corpse observation, item
/// target and corpse take.
pub const ITEM_VIEW_OFFERED_CAPABILITIES: &[OfferedCapability] = &[
    OfferedCapability {
        id: CAPABILITY_ITEM_VIEW_MOVE_V1,
        requires: &[CAPABILITY_WORLD_SPATIAL_ENTITIES],
    },
    OfferedCapability {
        id: CAPABILITY_WORLD_SPATIAL_ENTITIES,
        requires: &[],
    },
    OfferedCapability {
        id: CAPABILITY_CHAT_V1,
        requires: &[],
    },
    HIGHSCORES,
    OfferedCapability {
        id: CAPABILITY_PACED_MOVEMENT_V1,
        requires: &[],
    },
    QUEST_LOG,
    OfferedCapability {
        id: CAPABILITY_ATTACK_V1,
        requires: &[CAPABILITY_WORLD_SPATIAL_ENTITIES],
    },
    WORLD_LIGHT,
    LOGOUT,
    WORLD_WEATHER,
    TUTORIAL_NOTICES,
    super::guild_chat::OFFERED,
    super::party_command::OFFERED,
];

/// Bound of one session's selection. A test keeps the production offered set within it.
pub const SELECTED_CAPACITY: usize = 32;

/// One GameSession's selected capabilities: strictly ascending, each offered by the server at
/// fresh admission. `Copy` so it travels with the session's continuity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedCapabilities {
    ids: [u32; SELECTED_CAPACITY],
    len: usize,
}

impl SelectedCapabilities {
    pub const NONE: Self = Self {
        ids: [0; SELECTED_CAPACITY],
        len: 0,
    };

    /// The selection as the `selected_capabilities` wire field: strictly ascending.
    pub fn as_slice(&self) -> &[u32] {
        &self.ids[..self.len]
    }

    pub fn contains(&self, capability: u32) -> bool {
        self.as_slice().binary_search(&capability).is_ok()
    }

    /// Fresh admission: the client's `supported` capabilities that `offered` contains and whose
    /// `requires` are all selected, as a closure (a capability whose requirement drops out drops
    /// out too). Unknown and unoffered supported IDs are ignored. `None` when the result would
    /// not fit the bound, which a test rules out for the production set.
    pub fn select(offered: &[OfferedCapability], supported: &[u32]) -> Option<Self> {
        let mut candidates: Vec<OfferedCapability> = offered
            .iter()
            .copied()
            .filter(|capability| supported.contains(&capability.id))
            .collect();
        loop {
            let before = candidates.len();
            let ids: Vec<u32> = candidates.iter().map(|capability| capability.id).collect();
            candidates.retain(|capability| {
                capability
                    .requires
                    .iter()
                    .all(|required| ids.contains(required))
            });
            if candidates.len() == before {
                break;
            }
        }
        let mut ids: Vec<u32> = candidates.iter().map(|capability| capability.id).collect();
        ids.sort_unstable();
        ids.dedup();
        if ids.len() > SELECTED_CAPACITY {
            return None;
        }
        let mut selected = Self::NONE;
        selected.ids[..ids.len()].copy_from_slice(&ids);
        selected.len = ids.len();
        Some(selected)
    }

    /// Resume: the session keeps exactly its original selection. A resume whose `supported` set
    /// lacks a selected capability cannot keep it and is refused (`false`); it never widens.
    pub fn resumable_with(&self, supported: &[u32]) -> bool {
        self.as_slice()
            .iter()
            .all(|capability| supported.contains(capability))
    }

    /// Whether a command of `command_type` may be dispatched: a type owned by a capability only
    /// when that capability is selected; a core type always.
    pub fn command_selected(&self, command_type: u32) -> bool {
        GATED
            .iter()
            .filter(|(_, commands, _)| commands.contains(&command_type))
            .all(|(capability, _, _)| self.contains(*capability))
    }

    /// Whether state `domain` may be sent: a domain owned by a capability only when that
    /// capability is selected; a core domain unless a selected capability supersedes it
    /// ([`SUPERSEDED`]).
    pub fn domain_selected(&self, domain: u32) -> bool {
        GATED
            .iter()
            .filter(|(_, _, domains)| domains.contains(&domain))
            .all(|(capability, _, _)| self.contains(*capability))
            && !SUPERSEDED.iter().any(|(capability, domains)| {
                domains.contains(&domain) && self.contains(*capability)
            })
    }
}

#[cfg(test)]
#[path = "capabilities_tests.rs"]
mod tests;

#[cfg(test)]
pub(super) use tests::{RUNE_USE_OFFERED, command as test_command};
