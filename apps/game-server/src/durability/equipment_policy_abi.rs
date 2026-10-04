//! Pure bounded equipment data ABI. Parsing authoring semantics belongs to the actual
//! qualified Content producer; no Content, Gameplay or compiler dependency enters Durability.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::character_equipment::EquipmentError;
const SLOTS: [u8; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 10];
pub(crate) mod policy_registration {
    pub(crate) trait Registered {}
}
pub(crate) trait EquipmentPolicyLookup: policy_registration::Registered {
    fn source_digest(&self) -> [u8; 32];
    fn equipment_policy(&self, key: &str, revision: &str) -> Option<QualifiedEquipmentPolicy>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EquipmentBaseVocation {
    Druid,
    Knight,
    Monk,
    Paladin,
    Sorcerer,
}
#[derive(Debug, Clone)]
pub(crate) struct EquipmentClaims {
    pub(crate) slots: Vec<u8>,
    pub(crate) groups: Vec<String>,
    pub(crate) level: u16,
    pub(crate) vocations: Vec<EquipmentBaseVocation>,
}
pub(crate) struct QualifiedEquipmentPolicy {
    digest: [u8; 32],
    claims: Vec<(u8, Result<EquipmentClaims, &'static str>)>,
    container_capacity: Option<u16>,
}
impl QualifiedEquipmentPolicy {
    /// Sole audited producer: active NativeGameplayState member lookup. Data only; the writer
    /// independently checks current Content/session/custody within its actual transaction.
    pub(crate) fn from_qualified_content(
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
    pub(crate) fn source_digest(&self) -> [u8; 32] {
        self.digest
    }
    pub(crate) fn container_capacity(&self) -> Option<u16> {
        self.container_capacity
    }
}
pub(crate) fn claims(
    policy: &QualifiedEquipmentPolicy,
    slot: u8,
) -> Result<EquipmentClaims, EquipmentError> {
    let value = policy
        .claims
        .iter()
        .find(|(s, _)| *s == slot)
        .ok_or(EquipmentError::Rejected("equipment slot"))?;
    value.1.clone().map_err(EquipmentError::Rejected)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    fn unknown_slots() -> Vec<(u8, Result<EquipmentClaims, &'static str>)> {
        SLOTS
            .into_iter()
            .map(|slot| (slot, Err("unknown equipment pattern")))
            .collect()
    }
    #[test]
    fn neutral_unknown_claims_never_become_zero_requirements_or_known_capacity() {
        let policy =
            QualifiedEquipmentPolicy::from_qualified_content([1; 32], unknown_slots(), None)
                .expect("data with unknown fields");
        assert_eq!(policy.source_digest(), [1; 32]);
        assert_eq!(policy.container_capacity(), None);
        assert!(matches!(
            claims(&policy, 5),
            Err(EquipmentError::Rejected("unknown equipment pattern"))
        ));
        assert!(
            QualifiedEquipmentPolicy::from_qualified_content([0; 32], unknown_slots(), None)
                .is_none()
        );
        let mut missing = unknown_slots();
        missing.pop();
        assert!(QualifiedEquipmentPolicy::from_qualified_content([1; 32], missing, None).is_none());
    }
    #[test]
    fn neutral_policy_bounds_refuse_invalid_reserved_slots_and_unbounded_groups() {
        for (slots, groups) in [
            (vec![5, 9], vec![]),
            (vec![5, 5], vec![]),
            (vec![5], vec!["g".to_owned(); 33]),
        ] {
            let mut data = unknown_slots();
            data[4].1 = Ok(EquipmentClaims {
                slots,
                groups,
                level: 0,
                vocations: vec![],
            });
            assert!(
                QualifiedEquipmentPolicy::from_qualified_content([1; 32], data, None).is_none()
            );
        }
    }
}
