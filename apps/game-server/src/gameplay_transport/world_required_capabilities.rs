//! ADMIT0 §3–4 capability-set kernel, not an admission authority or a runtime adapter.
//!
//! CP must supply registered IDs, the offered graph and the build's mandatory floor
//! from trusted build facts, not operator/client claims. No production floor, World
//! profile or transport-profile adapter is chosen here. The owner must bind the boot
//! result to the current Channel generation/policy revision and check it only after
//! authentication, including final atomic revalidation. Support is not authority.

use super::capabilities::{OfferedCapability, SelectedCapabilities};
use oteryn_protocol_oteryn::MAX_CAPABILITY_COUNT;
use std::collections::BTreeSet;

/// ADMIT0-RL-01 (§3.1); registry/configuration integration remains slot-only.
pub(crate) const MAX_DECLARED_REQUIRED_CAPABILITIES: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BootCapabilityError {
    InvalidDeclaredList,
    DeclaredLimitExceeded,
    InvalidRegisteredList,
    InvalidFloorList,
    DuplicateOffered(u32),
    Unregistered(u32),
    Unoffered(u32),
    Cycle(u32),
    ClosureLimitExceeded,
    MissingFloor(Vec<u32>),
    /// The existing negotiated-selection representation cannot hold the requirements.
    SelectionUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionCapabilityError {
    InvalidSupportList,
    InvalidProfileList,
    InvalidProfile(BootCapabilityError),
    RequiredLimitExceeded,
    MissingSupport(Vec<u32>),
    SelectionUnavailable,
}

/// Immutable boot result, borrowing the same build catalogue for every session check.
/// This carries no World/Channel identity, owner fence, authenticated session or nonce.
#[derive(Debug)]
pub(crate) struct RequiredCapabilities<'a> {
    effective: Vec<u32>,
    registered: &'a [u32],
    offered: &'a [OfferedCapability],
}

fn canonical(ids: &[u32]) -> bool {
    ids.first().is_none_or(|id| *id != 0) && ids.windows(2).all(|pair| pair[0] < pair[1])
}

fn offered_node<'a>(
    id: u32,
    registered: &[u32],
    offered: &'a [OfferedCapability],
) -> Result<&'a OfferedCapability, BootCapabilityError> {
    if registered.binary_search(&id).is_err() {
        return Err(BootCapabilityError::Unregistered(id));
    }
    offered
        .iter()
        .find(|node| node.id == id)
        .ok_or(BootCapabilityError::Unoffered(id))
}

fn visit(
    id: u32,
    registered: &[u32],
    offered: &[OfferedCapability],
    active: &mut BTreeSet<u32>,
    complete: &mut BTreeSet<u32>,
) -> Result<(), BootCapabilityError> {
    if complete.contains(&id) {
        return Ok(());
    }
    if active.contains(&id) {
        return Err(BootCapabilityError::Cycle(id));
    }
    let node = offered_node(id, registered, offered)?;
    active.insert(id);
    // Bounds both the closure and recursion depth before visiting another dependency.
    if active.len() + complete.len() > MAX_CAPABILITY_COUNT {
        return Err(BootCapabilityError::ClosureLimitExceeded);
    }
    for dependency in node.requires {
        visit(*dependency, registered, offered, active, complete)?;
    }
    active.remove(&id);
    complete.insert(id);
    Ok(())
}

fn closure(
    roots: &[u32],
    registered: &[u32],
    offered: &[OfferedCapability],
) -> Result<Vec<u32>, BootCapabilityError> {
    let mut active = BTreeSet::new();
    let mut complete = BTreeSet::new();
    for id in roots {
        visit(*id, registered, offered, &mut active, &mut complete)?;
    }
    Ok(complete.into_iter().collect())
}

impl<'a> RequiredCapabilities<'a> {
    /// All three catalogue/floor arguments require a trusted build adapter at the CP slot.
    /// An explicit empty floor is valid only when that build offers no floor member.
    pub(crate) fn for_boot(
        declared: &[u32],
        registered: &'a [u32],
        offered: &'a [OfferedCapability],
        build_floor: &[u32],
    ) -> Result<Self, BootCapabilityError> {
        if declared.len() > MAX_DECLARED_REQUIRED_CAPABILITIES {
            return Err(BootCapabilityError::DeclaredLimitExceeded);
        }
        if !canonical(declared) {
            return Err(BootCapabilityError::InvalidDeclaredList);
        }
        if !canonical(registered) {
            return Err(BootCapabilityError::InvalidRegisteredList);
        }
        if !canonical(build_floor) || build_floor.len() > MAX_CAPABILITY_COUNT {
            return Err(BootCapabilityError::InvalidFloorList);
        }
        let mut offered_ids = BTreeSet::new();
        for node in offered {
            if registered.binary_search(&node.id).is_err() {
                return Err(BootCapabilityError::Unregistered(node.id));
            }
            if !offered_ids.insert(node.id) {
                return Err(BootCapabilityError::DuplicateOffered(node.id));
            }
        }
        let effective = closure(declared, registered, offered)?;
        for id in build_floor {
            offered_node(*id, registered, offered)?;
        }
        let missing: Vec<_> = build_floor
            .iter()
            .copied()
            .filter(|id| effective.binary_search(id).is_err())
            .collect();
        if !missing.is_empty() {
            return Err(BootCapabilityError::MissingFloor(missing));
        }
        // Reuse CAP-NEG-1 rather than duplicate negotiation or raise its capacity.
        if SelectedCapabilities::select(offered, &effective)
            .is_none_or(|selected| effective.iter().any(|id| !selected.contains(*id)))
        {
            return Err(BootCapabilityError::SelectionUnavailable);
        }
        Ok(Self {
            effective,
            registered,
            offered,
        })
    }

    pub(crate) fn as_slice(&self) -> &[u32] {
        &self.effective
    }

    /// The future authenticated caller supplies the trusted transport-profile roots.
    /// Profile requirements do not change the channel's boot set. Missing IDs are
    /// internal results and must never be correlated to a client before authentication.
    pub(crate) fn select_for_session(
        &self,
        supported: &[u32],
        profile_required: &[u32],
    ) -> Result<SelectedCapabilities, SessionCapabilityError> {
        if !canonical(supported) || supported.len() > MAX_CAPABILITY_COUNT {
            return Err(SessionCapabilityError::InvalidSupportList);
        }
        if !canonical(profile_required) || profile_required.len() > MAX_CAPABILITY_COUNT {
            return Err(SessionCapabilityError::InvalidProfileList);
        }
        let profile = closure(profile_required, self.registered, self.offered)
            .map_err(SessionCapabilityError::InvalidProfile)?;
        let required: BTreeSet<_> = self.effective.iter().copied().chain(profile).collect();
        if required.len() > MAX_CAPABILITY_COUNT {
            return Err(SessionCapabilityError::RequiredLimitExceeded);
        }
        let missing: Vec<_> = required
            .iter()
            .copied()
            .filter(|id| supported.binary_search(id).is_err())
            .collect();
        if !missing.is_empty() {
            return Err(SessionCapabilityError::MissingSupport(missing));
        }
        let selected = SelectedCapabilities::select(self.offered, supported)
            .ok_or(SessionCapabilityError::SelectionUnavailable)?;
        if required.iter().any(|id| !selected.contains(*id)) {
            return Err(SessionCapabilityError::SelectionUnavailable);
        }
        Ok(selected)
    }
}

#[cfg(test)]
#[path = "world_required_capabilities_tests.rs"]
mod tests;
