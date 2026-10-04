//! FND-02 optional capability negotiation (CAP-NEG-1, ARCH-BATCH-ITEM-EQUIP-PACKETS §1.9).
//!
//! The server offers the registry's `offered: true` capabilities. Fresh admission selects the
//! client's supported capabilities that the server offers and whose `requires` are all selected;
//! an unknown supported ID is ignored, never selected. The selection is kept per GameSession in
//! its continuity: a resume or channel transfer keeps it unchanged and never widens it, and a
//! resume whose supported set lacks a selected capability is refused as resume-unavailable, so the
//! client falls back to fresh admission. Dispatch and domain emission read the selection: a
//! command or domain whose capability is not selected stays refused or unsent.

use oteryn_protocol_oteryn::achievement_notices::{
    CAPABILITY_ACHIEVEMENT_NOTICES_V1, STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
};
use oteryn_protocol_oteryn::analyser::{CAPABILITY_ANALYSER_V1, STATE_DOMAIN_ACTOR_ANALYSER};
use oteryn_protocol_oteryn::bestiary::STATE_DOMAIN_CHARACTER_BESTIARY;
use oteryn_protocol_oteryn::charm::{
    CAPABILITY_BESTIARY_CHARMS_V1, COMMAND_TYPE_CHARM_ASSIGN_INTENT,
    COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT, STATE_DOMAIN_CHARACTER_CHARMS,
};
use oteryn_protocol_oteryn::chat::{
    CAPABILITY_CHAT_V1, COMMAND_TYPE_CHAT_INTENT, STATE_DOMAIN_CHAT,
};

/// One capability the server offers, with the capabilities that must also be selected for it
/// (the registry entry's `requires`, empty when the entry has none).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OfferedCapability {
    pub(crate) id: u32,
    pub(crate) requires: &'static [u32],
}

/// The production offered set: the registry's `offered: true` entries, ascending by ID. Empty
/// until a packet offers a capability, so production selects nothing. A test keeps it equal to
/// the registry and within `REGISTERED_CAPABILITY_IDS_V1`.
pub(crate) const PRODUCTION_OFFERED_CAPABILITIES: &[OfferedCapability] = &[];

/// The command types and state domains each registered capability owns
/// (`PROTOCOL_OTERYN_V1_REGISTRY.json`; a test keeps them equal). Capability 6
/// `WORLD_SPATIAL_ENTITIES` is not listed: it extends the core domain 1 with payload type 2, which
/// the visibility encoders gate on the selected set themselves.
const GATED: &[(u32, &[u32], &[u32])] = &[
    (
        CAPABILITY_BESTIARY_CHARMS_V1,
        &[
            COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT,
            COMMAND_TYPE_CHARM_ASSIGN_INTENT,
        ],
        &[
            STATE_DOMAIN_CHARACTER_BESTIARY,
            STATE_DOMAIN_CHARACTER_CHARMS,
        ],
    ),
    (
        CAPABILITY_CHAT_V1,
        &[COMMAND_TYPE_CHAT_INTENT],
        &[STATE_DOMAIN_CHAT],
    ),
    (
        CAPABILITY_ACHIEVEMENT_NOTICES_V1,
        &[],
        &[STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES],
    ),
    (CAPABILITY_ANALYSER_V1, &[], &[STATE_DOMAIN_ACTOR_ANALYSER]),
];

/// Bound of one session's selection. A test keeps the production offered set within it.
pub(crate) const SELECTED_CAPACITY: usize = 32;

/// One GameSession's selected capabilities: strictly ascending, each offered by the server at
/// fresh admission. `Copy` so it travels with the session's continuity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectedCapabilities {
    ids: [u32; SELECTED_CAPACITY],
    len: usize,
}

impl SelectedCapabilities {
    pub(crate) const NONE: Self = Self {
        ids: [0; SELECTED_CAPACITY],
        len: 0,
    };

    /// The selection as the `selected_capabilities` wire field: strictly ascending.
    pub(crate) fn as_slice(&self) -> &[u32] {
        &self.ids[..self.len]
    }

    pub(crate) fn contains(&self, capability: u32) -> bool {
        self.as_slice().binary_search(&capability).is_ok()
    }

    /// Fresh admission: the client's `supported` capabilities that `offered` contains and whose
    /// `requires` are all selected, as a closure (a capability whose requirement drops out drops
    /// out too). Unknown and unoffered supported IDs are ignored. `None` when the result would
    /// not fit the bound, which a test rules out for the production set.
    pub(crate) fn select(offered: &[OfferedCapability], supported: &[u32]) -> Option<Self> {
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
    pub(crate) fn resumable_with(&self, supported: &[u32]) -> bool {
        self.as_slice()
            .iter()
            .all(|capability| supported.contains(capability))
    }

    /// Whether a command of `command_type` may be dispatched: a type owned by a capability only
    /// when that capability is selected; a core type always.
    pub(crate) fn command_selected(&self, command_type: u32) -> bool {
        GATED
            .iter()
            .filter(|(_, commands, _)| commands.contains(&command_type))
            .all(|(capability, _, _)| self.contains(*capability))
    }

    /// Whether state `domain` may be sent: a domain owned by a capability only when that
    /// capability is selected; a core domain always.
    pub(crate) fn domain_selected(&self, domain: u32) -> bool {
        GATED
            .iter()
            .filter(|(_, _, domains)| domains.contains(&domain))
            .all(|(capability, _, _)| self.contains(*capability))
    }
}

#[cfg(test)]
pub(super) fn gated_table() -> &'static [(u32, &'static [u32], &'static [u32])] {
    GATED
}

#[cfg(test)]
#[path = "capabilities_tests.rs"]
mod tests;
