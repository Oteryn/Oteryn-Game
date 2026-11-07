//! Protocol-neutral comparison of a player's Might with creature Threat.

/// Presentation tier for a creature name. `Boss` deliberately overrides all ratios.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreatTier {
    Trivial,
    Easy,
    Fair,
    Hard,
    Deadly,
    Boss,
}

/// Classifies Threat relative to Might using integer arithmetic at inclusive boundaries.
#[must_use]
pub const fn threat_tier(
    might_or_party_might: u64,
    creature_threat: u64,
    boss: bool,
) -> ThreatTier {
    if boss {
        return ThreatTier::Boss;
    }
    if might_or_party_might == 0 {
        return if creature_threat == 0 {
            ThreatTier::Trivial
        } else {
            ThreatTier::Deadly
        };
    }
    let might = might_or_party_might as u128;
    let threat = creature_threat as u128;
    if threat * 100 <= might * 25 {
        ThreatTier::Trivial
    } else if threat * 100 <= might * 60 {
        ThreatTier::Easy
    } else if threat <= might {
        ThreatTier::Fair
    } else if threat * 100 <= might * 160 {
        ThreatTier::Hard
    } else {
        ThreatTier::Deadly
    }
}

#[cfg(test)]
mod tests {
    use super::{ThreatTier, threat_tier};

    #[test]
    fn every_inclusive_boundary_and_successor_is_classified() {
        assert_eq!(threat_tier(100, 25, false), ThreatTier::Trivial);
        assert_eq!(threat_tier(100, 26, false), ThreatTier::Easy);
        assert_eq!(threat_tier(100, 60, false), ThreatTier::Easy);
        assert_eq!(threat_tier(100, 61, false), ThreatTier::Fair);
        assert_eq!(threat_tier(100, 100, false), ThreatTier::Fair);
        assert_eq!(threat_tier(100, 101, false), ThreatTier::Hard);
        assert_eq!(threat_tier(100, 160, false), ThreatTier::Hard);
        assert_eq!(threat_tier(100, 161, false), ThreatTier::Deadly);
    }

    #[test]
    fn boss_overrides_ratio_and_zero_might_is_safe() {
        assert_eq!(threat_tier(100, 1, true), ThreatTier::Boss);
        assert_eq!(threat_tier(0, 1, false), ThreatTier::Deadly);
        assert_eq!(threat_tier(0, 0, false), ThreatTier::Trivial);
    }
}
