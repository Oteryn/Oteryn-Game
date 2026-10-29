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

use oteryn_protocol_oteryn::capability_tables;
use oteryn_protocol_oteryn::world_weather::{
    CAPABILITY_WORLD_WEATHER_V1, CAPABILITY_WORLD_WEATHER_V1_REQUIRES,
};

/// LOOK-2: capability 28, offered on a bundle World and dispatched by
/// the connection.
pub use oteryn_protocol_oteryn::capability_tables::CAPABILITY_ITEM_LOOK_V1;
#[cfg(test)]
pub use oteryn_protocol_oteryn::capability_tables::COMMAND_TYPE_ITEM_LOOK_INTENT;
use oteryn_protocol_oteryn::capability_tables::{GATED, SUPERSEDED};

/// One capability the server offers, with the capabilities that must also be selected for it
/// (the registry entry's `requires`, empty when the entry has none).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OfferedCapability {
    pub id: u32,
    pub requires: &'static [u32],
}

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

// Materialize the protocol's generated facts at compile time; no allocation or runtime scan.
const fn offers<const N: usize>(entries: &[(u32, &'static [u32])]) -> [OfferedCapability; N] {
    assert!(entries.len() == N);
    let mut result = [OfferedCapability {
        id: 0,
        requires: &[],
    }; N];
    let mut index = 0;
    while index < N {
        result[index] = OfferedCapability {
            id: entries[index].0,
            requires: entries[index].1,
        };
        index += 1;
    }
    result
}

/// Fallback offers for a generation without a non-empty Item key set, ascending by id.
pub const PRODUCTION_OFFERED_CAPABILITIES: &[OfferedCapability] =
    &offers::<{ capability_tables::PRODUCTION_OFFERS_V1.len() }>(
        capability_tables::PRODUCTION_OFFERS_V1,
    );
/// Bundle World offers, ascending by id. Per-connection weather gating still applies.
pub const BUNDLE_WORLD_OFFERED_CAPABILITIES: &[OfferedCapability] =
    &offers::<{ capability_tables::BUNDLE_WORLD_OFFERS_V1.len() }>(
        capability_tables::BUNDLE_WORLD_OFFERS_V1,
    );
/// Offers for a generation pinning a non-empty Item key set, ascending by id.
pub const ITEM_VIEW_OFFERED_CAPABILITIES: &[OfferedCapability] =
    &offers::<{ capability_tables::ITEM_VIEW_OFFERS_V1.len() }>(
        capability_tables::ITEM_VIEW_OFFERS_V1,
    );
const BUNDLE_WORLD_UNTIMED_LEN: usize = BUNDLE_WORLD_OFFERED_CAPABILITIES.len() - 1;
/// [`BUNDLE_WORLD_OFFERED_CAPABILITIES`] without capability 39, filtered at compile time so the
/// bundle offer keeps one source and stays ascending. Fails to compile unless 39 is offered
/// exactly once.
const BUNDLE_WORLD_UNTIMED_ARRAY: [OfferedCapability; BUNDLE_WORLD_UNTIMED_LEN] = {
    let all = BUNDLE_WORLD_OFFERED_CAPABILITIES;
    let mut out = [all[0]; BUNDLE_WORLD_UNTIMED_LEN];
    let (mut from, mut to) = (0, 0);
    while from < all.len() {
        if all[from].id != oteryn_protocol_oteryn::timed_item::CAPABILITY_TIMED_ITEM_V1 {
            out[to] = all[from];
            to += 1;
        }
        from += 1;
    }
    assert!(to == BUNDLE_WORLD_UNTIMED_LEN);
    out
};
/// The same bundle offer before the native timed definitions are available.
pub(super) const BUNDLE_WORLD_UNTIMED_OFFERED_CAPABILITIES: &[OfferedCapability] =
    &BUNDLE_WORLD_UNTIMED_ARRAY;

/// Bound of one session's selection. A test keeps the production offered set within it.
/// CAPCAP-1 raised it from 32 to 64; it stays within the protocol capability count (128).
pub const SELECTED_CAPACITY: usize = 64;
const _: () = assert!(SELECTED_CAPACITY <= oteryn_protocol_oteryn::MAX_CAPABILITY_COUNT);

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
