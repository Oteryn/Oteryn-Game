//! Pure bounded equipment data ABI. Parsing authoring semantics belongs to the actual
//! qualified Content producer; no Content, Gameplay or compiler dependency enters Durability.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
pub const SLOTS: [u8; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 10];
pub mod policy_registration {
    pub trait Registered {}
}
pub trait EquipmentPolicyLookup: policy_registration::Registered {
    fn source_digest(&self) -> [u8; 32];
    fn equipment_policy(&self, key: &str, revision: &str) -> Option<QualifiedEquipmentPolicy>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentBaseVocation {
    Druid,
    Knight,
    Monk,
    Paladin,
    Sorcerer,
}
#[derive(Debug, Clone)]
pub struct EquipmentClaims {
    pub slots: Vec<u8>,
    pub groups: Vec<String>,
    pub level: u16,
    pub vocations: Vec<EquipmentBaseVocation>,
}
pub struct QualifiedEquipmentPolicy {
    digest: [u8; 32],
    claims: Vec<(u8, Result<EquipmentClaims, &'static str>)>,
    container_capacity: Option<u16>,
}
impl QualifiedEquipmentPolicy {
    /// Sole audited producer: active NativeGameplayState member lookup. Data only; the writer
    /// independently checks current Content/session/custody within its actual transaction.
    pub fn from_qualified_content(
        digest: [u8; 32],
        claims: Vec<(u8, Result<EquipmentClaims, &'static str>)>,
        container_capacity: Option<u16>,
    ) -> Option<Self> {
        if digest == [0; 32] || claims.len() != SLOTS.len() {
            return None;
        }
        for (index, (slot, value)) in claims.iter().enumerate() {
            if *slot != SLOTS[index] {
                return None;
            }
            if let Ok(value) = value
                && (value.slots.is_empty()
                    || value.slots.len() > SLOTS.len()
                    || !value.slots.contains(slot)
                    || value.slots.iter().any(|s| !SLOTS.contains(s))
                    || value.slots.windows(2).any(|w| w[0] >= w[1])
                    || value.groups.len() > 32
                    || value.groups.iter().any(|g| g.is_empty() || g.len() > 128)
                    || value.vocations.len() > 5)
            {
                return None;
            }
        }
        Some(Self {
            digest,
            claims,
            container_capacity,
        })
    }
    pub fn source_digest(&self) -> [u8; 32] {
        self.digest
    }
    pub fn container_capacity(&self) -> Option<u16> {
        self.container_capacity
    }
    /// The qualified per-slot claims, read-only: the field stays private so no crate can
    /// replace them after `from_qualified_content` validated them.
    ///
    /// ```compile_fail
    /// use oteryn_gameplay_domain::domain::equipment_policy_abi::QualifiedEquipmentPolicy;
    /// fn forge(policy: &mut QualifiedEquipmentPolicy) { policy.claims.clear(); }
    /// ```
    pub fn claims(&self) -> &[(u8, Result<EquipmentClaims, &'static str>)] {
        &self.claims
    }
}
