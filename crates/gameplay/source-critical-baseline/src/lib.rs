//! Source critical owner: Global 1% monster auto-attack floor and closed source overrides.
//! Global monster extra damage and dynamic SoulPit boosts remain unqualified.
#![allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
pub const CRITICAL_BASELINE_QUALIFICATION: &str =
    "PROJECT_SOURCE_CRITICAL_BONUS_ZERO(GlobalAutoAttackChanceQualifiedDynamicCriticalUnverified)";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceCriticalBaseline {
    chance_ppm: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceCriticalEvent {
    pub critical: bool,
    pub bonus_basis_points: u32,
    pub qualification: &'static str,
}
impl SourceCriticalBaseline {
    /// Native source profiles must originate from the current owning artifact loader. Closed
    /// keys/chances qualify source overrides; a zero source chance gets the Global 1%
    /// default only for auto-attacks. Spell/callback source defaults remain unchanged.
    #[allow(
        clippy::result_unit_err,
        reason = "mechanical crate extraction preserves the existing unit-error API"
    )]
    pub fn qualify(key: &str, chance_ppm: u32, auto_attack: bool) -> Result<Option<Self>, ()> {
        if chance_ppm == 0 {
            return Ok(auto_attack.then_some(Self { chance_ppm: 10_000 }));
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
    pub fn fingerprint_parts(self) -> (u32, &'static str) {
        (self.chance_ppm, CRITICAL_BASELINE_QUALIFICATION)
    }
    /// Distinct deterministic decision-purpose keyed by the ORIGINAL owner-issued cast/swing
    /// occurrence. Native memoized outcomes suppress replay; no extra attack/target occurrence.
    /// Source uniform_random(1,10000)<=chance*100: exact source chance granularity retained.
    #[allow(
        clippy::result_unit_err,
        reason = "mechanical crate extraction preserves the existing unit-error API"
    )]
    pub fn draw(
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
#[allow(clippy::expect_used)]
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_default_is_closed_and_deterministic_without_invented_damage_bonus() {
        let policy =
            SourceCriticalBaseline::qualify("oteryn:creature.doctor_marrow", 100_000, false)
                .expect("qualified fixture")
                .expect("qualified fixture");
        assert!(
            SourceCriticalBaseline::qualify("oteryn:creature.doctor_marrow", 500_000, false)
                .is_err()
        );
        assert!(SourceCriticalBaseline::qualify("oteryn:creature.cat", 100_000, false).is_err());
        assert_eq!(
            SourceCriticalBaseline::qualify("oteryn:creature.cat", 0, false)
                .expect("qualified fixture"),
            None
        );
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let mut passed = 0;
        for sequence in 1u64..=1000 {
            let mut bytes = [0; 16];
            bytes[..8].copy_from_slice(&sequence.to_be_bytes());
            let id = DecisionOccurrenceId::from_bytes(bytes);
            let first = policy.draw(&root, id, 0).expect("qualified fixture");
            assert_eq!(first, policy.draw(&root, id, 0).expect("qualified fixture"));
            assert_eq!(first.bonus_basis_points, 0);
            assert_eq!(first.qualification, CRITICAL_BASELINE_QUALIFICATION);
            passed += usize::from(first.critical);
        }
        assert!((50..150).contains(&passed));
    }

    #[test]
    fn global_monster_auto_attack_has_one_percent_without_changing_source_overrides() {
        let base = SourceCriticalBaseline::qualify("oteryn:creature.cat", 0, true)
            .expect("global default")
            .expect("auto attack baseline");
        assert_eq!(base.fingerprint_parts().0, 10_000);
        let special = SourceCriticalBaseline::qualify("oteryn:creature.mitmah_scout", 30_000, true)
            .expect("source override")
            .expect("source critical");
        assert_eq!(special.fingerprint_parts().0, 30_000);
        let root = GameplayDecisionRoot::from_bytes([1; 32]);
        let mut hits = 0;
        for sequence in 1u64..=10_000 {
            let mut bytes = [0; 16];
            bytes[..8].copy_from_slice(&sequence.to_be_bytes());
            let id = DecisionOccurrenceId::from_bytes(bytes);
            let event = base.draw(&root, id, 0).expect("deterministic draw");
            assert_eq!(event, base.draw(&root, id, 0).expect("replay"));
            assert_eq!(event.bonus_basis_points, 0);
            hits += usize::from(event.critical);
        }
        assert!((50..150).contains(&hits));
    }
}
