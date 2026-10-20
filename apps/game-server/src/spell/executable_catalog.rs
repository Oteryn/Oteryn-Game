//! Compatibility path for Content-owned executable_catalog.
pub(crate) use crate::content::spell_data::executable_catalog::*;

#[cfg(test)]
mod tests {
    #[test]
    fn cooldown_preflight_snapshot_keeps_namespaces_and_microsecond_units() {
        let mut cooldowns = crate::spell::Cooldowns::default();
        cooldowns.spells.insert(
            "same".into(),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(12_345),
        );
        cooldowns.groups.insert(
            "same".into(),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(67_890),
        );
        assert_eq!(
            cooldowns.canonical_deadlines(),
            vec![("group:same".into(), 67_890), ("spell:same".into(), 12_345)]
        );
    }
}
