//! Pure bounded equipment data ABI. Parsing authoring semantics belongs to the actual
//! qualified Content producer; no Content, Gameplay or compiler dependency enters Durability.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::character_equipment::EquipmentError;
#[allow(
    unused_imports,
    reason = "compatibility exports for existing equipment consumers"
)]
pub use crate::foundation::equipment_policy_abi::{
    EquipmentBaseVocation, EquipmentClaims, EquipmentPolicyLookup, QualifiedEquipmentPolicy,
    policy_registration,
};
pub fn claims(
    policy: &QualifiedEquipmentPolicy,
    slot: u8,
) -> Result<EquipmentClaims, EquipmentError> {
    let value = policy
        .claims()
        .iter()
        .find(|(s, _)| *s == slot)
        .ok_or(EquipmentError::Rejected("equipment slot"))?;
    value.1.clone().map_err(EquipmentError::Rejected)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::foundation::equipment_policy_abi::SLOTS;
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
