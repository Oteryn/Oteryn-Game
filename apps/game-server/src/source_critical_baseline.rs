//! Closed four-actor SOURCE-DEFAULT baseline. Not global critical parity or dynamic SoulPit support.
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
pub(crate) const CRITICAL_BASELINE_QUALIFICATION: &str =
    "PROJECT_SOURCE_DEFAULT_CRITICAL_BONUS_ZERO(GlobalUnverifiedDynamicCritical)";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceCriticalBaseline {
    chance_ppm: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceCriticalEvent {
    pub(crate) critical: bool,
    pub(crate) bonus_basis_points: u32,
    pub(crate) qualification: &'static str,
}
impl SourceCriticalBaseline {
    /// Native source profiles must originate from the current owning artifact loader. Closed
    /// keys/chances qualify the cached pinned source default, not arbitrary Lua setters.
    pub(crate) fn qualify(key: &str, chance_ppm: u32) -> Result<Option<Self>, ()> {
        if chance_ppm == 0 {
            return Ok(None);
        }
        let expected = match key {
            "oteryn:creature.doctor_marrow" | "oteryn:creature.the_monster" => 100_000,
            "oteryn:creature.mitmah_scout" | "oteryn:creature.mitmah_seer" => 30_000,
            _ => return Err(()),
        };
        if chance_ppm != expected {
            return Err(());
        }
        Ok(Some(Self { chance_ppm }))
    }
    pub(crate) fn fingerprint_parts(self) -> (u32, &'static str) {
        (self.chance_ppm, CRITICAL_BASELINE_QUALIFICATION)
    }
    /// Distinct deterministic decision-purpose keyed by the ORIGINAL owner-issued cast/swing
    /// occurrence. Native memoized outcomes suppress replay; no extra attack/target occurrence.
    /// Source uniform_random(1,10000)<=chance*100: exact source chance granularity retained.
    pub(crate) fn draw(
        self,
        root: &GameplayDecisionRoot,
        occurrence: DecisionOccurrenceId,
        index: u64,
    ) -> Result<SourceCriticalEvent, ()> {
        let draw = deterministic_decision_u64(root, occurrence, "source_critical_draw", index)
            .map_err(|_| ())?;
        Ok(SourceCriticalEvent {
            critical: draw % 10_000 < u64::from(self.chance_ppm / 100),
            bonus_basis_points: 0,
            qualification: CRITICAL_BASELINE_QUALIFICATION,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_default_is_closed_and_deterministic_without_invented_damage_bonus() {
        let policy = SourceCriticalBaseline::qualify("oteryn:creature.doctor_marrow", 100_000)
            .unwrap()
            .unwrap();
        assert!(SourceCriticalBaseline::qualify("oteryn:creature.doctor_marrow", 500_000).is_err());
        assert!(SourceCriticalBaseline::qualify("oteryn:creature.cat", 100_000).is_err());
        assert_eq!(
            SourceCriticalBaseline::qualify("oteryn:creature.cat", 0).unwrap(),
            None
        );
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let mut passed = 0;
        for sequence in 1u64..=1000 {
            let mut bytes = [0; 16];
            bytes[..8].copy_from_slice(&sequence.to_be_bytes());
            let id = DecisionOccurrenceId::from_bytes(bytes);
            let first = policy.draw(&root, id, 0).unwrap();
            assert_eq!(first, policy.draw(&root, id, 0).unwrap());
            assert_eq!(first.bonus_basis_points, 0);
            assert_eq!(first.qualification, CRITICAL_BASELINE_QUALIFICATION);
            passed += usize::from(first.critical);
        }
        assert!((50..150).contains(&passed));
    }
}
