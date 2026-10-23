//! ADMIT0 §3–4 capability sets bound to the booted Channel generation (D1081–D1084).
//! The build registry and actual scope offer determine the floor; configuration and
//! clients cannot weaken it. Authentication and independently current authority stay
//! with the admission owners. An immutable boot result is never current authority.

use super::capabilities::{OfferedCapability, SelectedCapabilities};
use oteryn_protocol_oteryn::MAX_CAPABILITY_COUNT;
use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use crate::foundation::RuntimeScopeRefV1;

/// ADMIT0-RL-01 (§3.1), registered in RESOURCE_LIMITS_REGISTRY.json.
pub const MAX_DECLARED_REQUIRED_CAPABILITIES: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootCapabilityError {
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
    InvalidBuildRegistry,
    OfferRegistryMismatch(u32),
    InvalidScopeBinding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCapabilityError {
    InvalidSupportList,
    InvalidProfileList,
    InvalidProfile(BootCapabilityError),
    RequiredLimitExceeded,
    MissingSupport(Vec<u32>),
    SelectionUnavailable,
    StaleScopeBinding,
}

#[derive(serde::Deserialize)]
struct RegistryCapability {
    id: u32,
    name: String,
    #[serde(default)]
    requires: Vec<u32>,
}

#[derive(serde::Deserialize)]
struct RegistryDocument {
    capabilities: Vec<RegistryCapability>,
}

struct BuildRegistry {
    ids: Vec<u32>,
    rows: Vec<RegistryCapability>,
}

fn build_registry() -> Result<&'static BuildRegistry, BootCapabilityError> {
    static REGISTRY: OnceLock<Result<BuildRegistry, BootCapabilityError>> = OnceLock::new();
    REGISTRY
        .get_or_init(|| {
            let document: RegistryDocument =
                serde_json::from_str(oteryn_protocol_oteryn::PROTOCOL_OTERYN_V1_REGISTRY_JSON)
                    .map_err(|_| BootCapabilityError::InvalidBuildRegistry)?;
            let ids: Vec<_> = document.capabilities.iter().map(|row| row.id).collect();
            let names: BTreeSet<_> = document.capabilities.iter().map(|row| &row.name).collect();
            if !canonical(&ids)
                || names.len() != ids.len()
                || document.capabilities.iter().any(|row| {
                    !canonical(&row.requires)
                        || row.requires.iter().any(|id| ids.binary_search(id).is_err())
                })
            {
                return Err(BootCapabilityError::InvalidBuildRegistry);
            }
            Ok(BuildRegistry {
                ids,
                rows: document.capabilities,
            })
        })
        .as_ref()
        .map_err(Clone::clone)
}

/// Shape/registration only; offer gates and the mandatory floor are checked at boot.
pub fn validate_declared(ids: &[u32]) -> Result<(), BootCapabilityError> {
    if ids.len() > MAX_DECLARED_REQUIRED_CAPABILITIES {
        return Err(BootCapabilityError::DeclaredLimitExceeded);
    }
    if !canonical(ids) {
        return Err(BootCapabilityError::InvalidDeclaredList);
    }
    let registry = build_registry()?;
    for id in ids {
        if registry.ids.binary_search(id).is_err() {
            return Err(BootCapabilityError::Unregistered(*id));
        }
    }
    Ok(())
}

/// Shared by boot and the session offer; neither reads the registry's descriptive
/// `offered` flag as a substitute for the real bundle/Item offer gate. WEATHER-1: capability 26
/// is part of the offer only while the shipped weather data is bound (fail closed).
pub fn offered_for_scope(
    bundle: bool,
    active: Option<&crate::content::ActiveGeneration>,
) -> &'static [OfferedCapability] {
    offered_for_scope_with(bundle, active, crate::world_weather::shipped().is_some())
}

/// [`offered_for_scope`] with the weather binding given: without a bound source the set has no
/// capability 26, so a boot that requires 26 fails closed (`Unoffered(26)`) instead of
/// publishing readiness for sessions that can never receive it.
pub fn offered_for_scope_with(
    bundle: bool,
    active: Option<&crate::content::ActiveGeneration>,
    weather_bound: bool,
) -> &'static [OfferedCapability] {
    let base = if bundle {
        super::mining::offer::offered(active)
    } else {
        super::item_ref_admission::offered_capabilities(active)
    };
    if weather_bound {
        base
    } else {
        without_weather(base)
    }
}

/// `base` without capability 26 as a `'static` slice, built once per distinct base set.
fn without_weather(base: &'static [OfferedCapability]) -> &'static [OfferedCapability] {
    type Cached = (&'static [OfferedCapability], &'static [OfferedCapability]);
    static CACHE: std::sync::Mutex<Vec<Cached>> = std::sync::Mutex::new(Vec::new());
    let mut cache = CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((_, set)) = cache.iter().find(|(key, _)| *key == base) {
        return set;
    }
    let set: &'static [OfferedCapability] =
        Box::leak(super::capabilities::with_weather_offer(base, false).into_boxed_slice());
    cache.push((base, set));
    set
}

/// ADMIT0 §3.2 mandatory floor, by registry name. D1083: 18 WORLD_MAP_VIEW_V1 is the accepted
/// map successor (29 MAP_STATE_V1 is registered by D1442 only so its name is never reused, and
/// is never offered). D1442 registers 30 WORLD_SPATIAL_FIELDS.
pub const MANDATORY_FLOOR_NAMES: [&str; 3] = [
    "WORLD_MAP_VIEW_V1",
    "WORLD_SPATIAL_FIELDS",
    "WORLD_INTERACTION_V1",
];

fn offered_named(
    rows: &[RegistryCapability],
    offered: &[OfferedCapability],
    names: &[&str],
) -> Vec<u32> {
    rows.iter()
        .filter(|row| {
            names.contains(&row.name.as_str()) && offered.iter().any(|node| node.id == row.id)
        })
        .map(|row| row.id)
        .collect()
}

/// Build-floor adapter (D1442): the floor is every mandatory name that is registered AND
/// offered by this scope's real offer, so a closed offer gate contributes nothing yet and its
/// opening adds it automatically. A floor name absent from the build registry fails boot
/// closed: the floor is never silently shrunk by a renamed or missing registry row.
fn build_floor(
    rows: &[RegistryCapability],
    offered: &[OfferedCapability],
) -> Result<Vec<u32>, BootCapabilityError> {
    if MANDATORY_FLOOR_NAMES
        .iter()
        .any(|name| !rows.iter().any(|row| row.name == *name))
    {
        return Err(BootCapabilityError::InvalidFloorList);
    }
    Ok(offered_named(rows, offered, &MANDATORY_FLOOR_NAMES))
}

/// The immutable requirements actually validated before this generation publishes readiness.
#[derive(Debug)]
pub struct BoundRequiredCapabilities {
    required: RequiredCapabilities<'static>,
    scope: RuntimeScopeRefV1,
    generation: u64,
    world_policy_revision: String,
    native_profile: Vec<u32>,
}

impl BoundRequiredCapabilities {
    /// The boot-validated, sorted dependency closure for this Channel binding.
    /// This excludes optional session selection and client-profile additions.
    /// Callers must independently resolve current scope/generation/policy and use
    /// `matches_current`; this immutable snapshot grants no current authority.
    pub fn effective(&self) -> &[u32] {
        &self.required.effective
    }

    pub fn matches_current(&self, scope: RuntimeScopeRefV1, generation: u64, policy: &str) -> bool {
        self.scope == scope && self.generation == generation && self.world_policy_revision == policy
    }

    pub fn for_channel_boot(
        declared: &[u32],
        scope: RuntimeScopeRefV1,
        generation: u64,
        world_policy_revision: &str,
        offered: &'static [OfferedCapability],
    ) -> Result<Arc<Self>, BootCapabilityError> {
        validate_declared(declared)?;
        if !matches!(scope, RuntimeScopeRefV1::Channel { .. })
            || generation == 0
            || world_policy_revision.is_empty()
        {
            return Err(BootCapabilityError::InvalidScopeBinding);
        }
        let registry = build_registry()?;
        for node in offered {
            let Some(row) = registry.rows.iter().find(|row| row.id == node.id) else {
                return Err(BootCapabilityError::Unregistered(node.id));
            };
            if row.requires != node.requires {
                return Err(BootCapabilityError::OfferRegistryMismatch(node.id));
            }
        }
        let floor = build_floor(&registry.rows, offered)?;
        let native_profile = offered_named(
            &registry.rows,
            offered,
            &["CLIENT_INTEGRITY_V1", "INPUT_TELEMETRY_V1"],
        );
        let required = RequiredCapabilities::for_boot(declared, &registry.ids, offered, &floor)?;
        Ok(Arc::new(Self {
            required,
            scope,
            generation,
            world_policy_revision: world_policy_revision.into(),
            native_profile,
        }))
    }

    /// Call only after grant authentication and safe current ownership/world classification.
    pub fn authenticate_support(
        self: &Arc<Self>,
        supported: &[u32],
    ) -> Result<AuthenticatedSessionCapabilities, SessionCapabilityError> {
        let selected = self
            .required
            .select_for_session(supported, &self.native_profile)?;
        Ok(AuthenticatedSessionCapabilities {
            requirements: self.clone(),
            supported: supported.to_vec(),
            selected,
        })
    }

    /// A resume forces the current requirements but retains optional selection
    /// semantics: newly advertised optional capabilities do not widen the session.
    /// Missing predecessor support still follows the legacy owner path, separately.
    pub fn authenticate_resume_support(
        self: &Arc<Self>,
        supported: &[u32],
        predecessor: SelectedCapabilities,
    ) -> Result<AuthenticatedSessionCapabilities, SessionCapabilityError> {
        // Validate the full wire support before filtering it (duplicates/oversize
        // evidence must not become valid merely because an optional ID is dropped).
        self.required
            .select_for_session(supported, &self.native_profile)?;
        let profile = closure(
            &self.native_profile,
            self.required.registered,
            self.required.offered,
        )
        .map_err(SessionCapabilityError::InvalidProfile)?;
        let forced: BTreeSet<_> = self.effective().iter().copied().chain(profile).collect();
        let supported: Vec<_> = supported
            .iter()
            .copied()
            .filter(|id| predecessor.contains(*id) || forced.contains(id))
            .collect();
        self.authenticate_support(&supported)
    }
}

/// Support and selection are data. Every mutation must additionally match the
/// independently resolved current scope/generation/policy through `revalidate`.
#[derive(Clone)]
pub struct AuthenticatedSessionCapabilities {
    requirements: Arc<BoundRequiredCapabilities>,
    supported: Vec<u32>,
    selected: SelectedCapabilities,
}

impl AuthenticatedSessionCapabilities {
    pub fn selected(&self) -> SelectedCapabilities {
        self.selected
    }

    pub fn revalidate(
        &self,
        scope: RuntimeScopeRefV1,
        generation: u64,
        world_policy_revision: &str,
    ) -> Result<SelectedCapabilities, SessionCapabilityError> {
        let bound = &self.requirements;
        if !bound.matches_current(scope, generation, world_policy_revision) {
            return Err(SessionCapabilityError::StaleScopeBinding);
        }
        bound
            .required
            .select_for_session(&self.supported, &bound.native_profile)
    }

    pub fn current_check(&self) -> crate::durability::fresh_admission::CurrentCapabilityCheck {
        let support = self.clone();
        Arc::new(move |scope, generation, policy| {
            support.revalidate(scope, generation, policy).is_ok()
        })
    }
}

/// Immutable boot result, borrowing the same build catalogue for every session check.
/// This carries no World/Channel identity, owner fence, authenticated session or nonce.
#[derive(Debug)]
pub struct RequiredCapabilities<'a> {
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
    pub fn for_boot(
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

    #[cfg(test)]
    pub fn as_slice(&self) -> &[u32] {
        &self.effective
    }

    /// The future authenticated caller supplies the trusted transport-profile roots.
    /// Profile requirements do not change the channel's boot set. Missing IDs are
    /// internal results and must never be correlated to a client before authentication.
    pub fn select_for_session(
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
