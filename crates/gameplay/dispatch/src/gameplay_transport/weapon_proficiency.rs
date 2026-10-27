//! PROF-EFFECT-0 native attack/death dispatch owner.
//!
//! The caller freezes the equipped weapon and contributor timestamps with the lethal commit.
//! This owner deduplicates death occurrences, accrues the existing durable track shape, and
//! applies selected attack perks before damage is handed to the combat owner. Checkpoints
//! serialize this state through the existing Character proficiency writer.

use std::collections::{BTreeMap, BTreeSet};

use oteryn_gameplay_domain::domain::weapon_proficiency::{
    ProficiencyKillContributor, ProficiencySelectionShape, proficiency_kill_credit,
};

/// Exact aggregate of selected perks read by the existing combat/stat stages.
/// Fractions use one denominator and are truncated once after summation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProficiencyAttackEffects {
    pub flat_attack: i64,
    pub damage_numerator: i64,
    pub denominator: i64,
    pub alpha_numerator: i64,
    pub omega_numerator: i64,
}

impl ProficiencyAttackEffects {
    fn valid(self) -> bool {
        self.denominator > 0
            && self.damage_numerator >= 0
            && self.alpha_numerator >= 0
            && self.omega_numerator >= 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveProficiencyTrack {
    pub shape: ProficiencySelectionShape,
    pub progress: u32,
    pub selections: Vec<Option<u8>>,
    pub effects: ProficiencyAttackEffects,
}

/// Session-local state. `seen_deaths` implements PROF-EFFECT0-RL-01; both it and
/// uncheckpointed progress disappear on restart by design.
#[derive(Debug, Default)]
pub struct WeaponProficiencyDispatch {
    tracks: BTreeMap<String, LiveProficiencyTrack>,
    seen_deaths: BTreeSet<[u8; 16]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KillDispatch<'a> {
    pub death_occurrence: [u8; 16],
    pub death_at_ms: u64,
    pub creature_is_summon: bool,
    pub points: Option<u32>,
    pub contributor: ProficiencyKillContributor<'a>,
}

impl WeaponProficiencyDispatch {
    /// Install a track loaded by the existing durable proficiency reader.
    pub fn install_track(&mut self, item_key: impl Into<String>, track: LiveProficiencyTrack) {
        self.tracks.insert(item_key.into(), track);
    }

    pub fn track(&self, item_key: &str) -> Option<&LiveProficiencyTrack> {
        self.tracks.get(item_key)
    }

    /// Death-dispatch entry point. Duplicate occurrences and absent tracks are no-ops.
    pub fn dispatch_kill(&mut self, input: KillDispatch<'_>) -> Option<u32> {
        if self.seen_deaths.contains(&input.death_occurrence) {
            return None;
        }
        let credit = proficiency_kill_credit(
            input.death_at_ms,
            input.creature_is_summon,
            input.points,
            input.contributor,
        )?;
        let track = self.tracks.get_mut(credit.weapon_item_key)?;
        self.seen_deaths.insert(input.death_occurrence);
        track.progress = track
            .shape
            .track_shape()
            .credited(track.progress, credit.points);
        Some(track.progress)
    }

    /// Auto-attack dispatch entry point. Alpha is strictly above 95% health and Omega is
    /// strictly below 30%; percentage contributions are summed and truncated once.
    pub fn dispatch_attack(
        &self,
        equipped_weapon_item_key: &str,
        base_damage: i64,
        target_health: u64,
        target_max_health: u64,
    ) -> i64 {
        let Some(track) = self.tracks.get(equipped_weapon_item_key) else {
            return base_damage;
        };
        let effects = track.effects;
        let has_active_selection = track
            .shape
            .active_selections(track.progress, &track.selections)
            .is_ok_and(|selections| !selections.is_empty());
        if !has_active_selection || !effects.valid() || base_damage <= 0 || target_max_health == 0 {
            return base_damage;
        }
        let mut numerator = effects.damage_numerator;
        if u128::from(target_health) * 100 > u128::from(target_max_health) * 95 {
            numerator = numerator.saturating_add(effects.alpha_numerator);
        }
        if u128::from(target_health) * 100 < u128::from(target_max_health) * 30 {
            numerator = numerator.saturating_add(effects.omega_numerator);
        }
        base_damage
            .saturating_add(effects.flat_attack)
            .saturating_add(base_damage.saturating_mul(numerator) / effects.denominator)
            .max(0)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use oteryn_gameplay_domain::domain::weapon_proficiency::ProficiencyThresholdClass;

    fn track() -> LiveProficiencyTrack {
        LiveProficiencyTrack {
            shape: ProficiencySelectionShape::new(
                ProficiencyThresholdClass::Standard,
                &[1, 1, 1, 1, 1, 1, 1],
            )
            .expect("shape"),
            progress: 1_750,
            selections: vec![Some(0), None, None, None, None, None, None],
            effects: ProficiencyAttackEffects {
                flat_attack: 2,
                damage_numerator: 5,
                denominator: 100,
                alpha_numerator: 10,
                omega_numerator: 20,
            },
        }
    }

    #[test]
    fn real_kill_dispatch_credits_every_recent_contributor_full_points_once() {
        let mut first = WeaponProficiencyDispatch::default();
        let mut party_peer = WeaponProficiencyDispatch::default();
        first.install_track("weapon", track());
        party_peer.install_track("weapon", track());
        let input = |last_damage_at_ms| KillDispatch {
            death_occurrence: [7; 16],
            death_at_ms: 400_000,
            creature_is_summon: false,
            points: Some(100),
            contributor: ProficiencyKillContributor {
                character_in_channel: true,
                last_damage_at_ms,
                equipped_weapon_item_key: Some("weapon"),
                weapon_has_promoted_proficiency: true,
            },
        };
        assert_eq!(first.dispatch_kill(input(399_999)), Some(1_850));
        assert_eq!(party_peer.dispatch_kill(input(100_000)), Some(1_850));
        assert_eq!(first.dispatch_kill(input(399_999)), None);
        assert_eq!(first.track("weapon").expect("track").progress, 1_850);
    }

    #[test]
    fn real_attack_dispatch_applies_selected_threshold_effects_once() {
        let mut owner = WeaponProficiencyDispatch::default();
        owner.install_track("weapon", track());
        assert_eq!(owner.dispatch_attack("weapon", 100, 96, 100), 117);
        assert_eq!(owner.dispatch_attack("weapon", 100, 95, 100), 107);
        assert_eq!(owner.dispatch_attack("weapon", 100, 29, 100), 127);
        assert_eq!(owner.dispatch_attack("weapon", 100, 30, 100), 107);
    }
}
