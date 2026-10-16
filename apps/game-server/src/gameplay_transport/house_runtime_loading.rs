//! Activation is closed before touching content or durability.
//!
//! The base has no generation-qualified HouseInterior loader or native House
//! carrier/active-bundle adapter. In particular, generic BundleFacts house tiles
//! and a HouseId do not qualify an interior or its arrival tile. There is no
//! runtime SELECT, synthetic empty load, caller-supplied readiness or open flag.
//! Adding the real loader later must preserve this gate until the independently
//! reviewed ADMIT0 §3.2 transfer conditions are explicitly lifted.

use crate::house_scope::{HouseEntryRefusal, HouseId};

/// Identity is accepted only to route the refusal; it grants no load authority.
pub(super) fn activation_refusal(_house: &HouseId) -> HouseEntryRefusal {
    HouseEntryRefusal::HouseClosed
}
