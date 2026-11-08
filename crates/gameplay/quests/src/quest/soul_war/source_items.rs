//! Literal persistent Character KV rules; Crystal201189f mechanics.lua672-765,869-986.
pub const SPITE_FIRE_COOLDOWN_SECONDS: u64 = 56;
pub const SOURCE_USE_COOLDOWN_SECONDS: u64 = 30;
pub const TRANSITION_PREFIX: &str = "oteryn:quest-transition/soul-war/source-items/";
pub const QUEST: &str = "oteryn:quest/soul-war/source-items";
pub const REMORSE: &str = "oteryn:quest-progress/soul-war/condensed-remorse";
pub const SPITE_FIRE: &str = "oteryn:quest-progress/soul-war/goshnar-spite-fire";
pub const MAW: &str = "oteryn:quest-progress/soul-war/furious-crater/greedy-maw-action";
pub const SANITY: &str = "oteryn:quest-progress/soul-war/furious-crater/cleansed-sanity-action";
// The donor's furius/furious distinction is intentional source data.
pub const ENERGY_TRACKS: [&str; 4] = [
    "oteryn:quest-progress/soul-war/furius-crater/pulsating-energy/access-counter",
    "oteryn:quest-progress/soul-war/furius-crater/pulsating-energy/first-floor-access",
    "oteryn:quest-progress/soul-war/furius-crater/pulsating-energy/second-floor-access",
    "oteryn:quest-progress/soul-war/furius-crater/pulsating-energy/third-floor-access",
];
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SourceItemState {
    pub spite_fire_second: u64,
    pub remorse: u8,
    pub energy: i64,
    pub floors: u8,
    pub maw_second: u64,
    pub sanity_second: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    SpiteFire,
    Remorse,
    Energy,
    GreedyMaw,
    CleansedSanity,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    pub after: SourceItemState,
    pub consume: bool,
    pub reset_torment: bool,
    pub cooldown_remaining: u64,
    pub granted_floors: u8,
    pub target_accepted: bool,
}
impl SourceItemState {
    pub fn apply(self, kind: Kind, now_second: u64, correct_target: bool) -> Option<Plan> {
        if self.remorse > 1 || self.energy < 0 || self.floors > 7 {
            return None;
        }
        let mut plan = Plan {
            after: self,
            consume: false,
            reset_torment: false,
            cooldown_remaining: 0,
            granted_floors: 0,
            target_accepted: correct_target,
        };
        match kind {
            Kind::Remorse => {
                plan.consume = true;
                plan.after.remorse = 1 - self.remorse;
                plan.reset_torment = self.remorse == 1;
            }
            Kind::Energy => {
                let count = self.energy.checked_add(1)?;
                plan.after.energy = count;
                if self.floors & 4 != 0 {
                    return Some(plan);
                }
                for (threshold, bit) in [(40, 1), (55, 2), (70, 4)] {
                    if count >= threshold && self.floors & bit == 0 {
                        plan.after.energy = 0;
                        plan.after.floors |= bit;
                        plan.granted_floors |= bit;
                    }
                }
                plan.consume = true;
            }
            Kind::SpiteFire | Kind::GreedyMaw if !correct_target => {}
            Kind::SpiteFire | Kind::GreedyMaw | Kind::CleansedSanity => {
                let (before, seconds) = match kind {
                    Kind::SpiteFire => (self.spite_fire_second, SPITE_FIRE_COOLDOWN_SECONDS),
                    Kind::GreedyMaw => (self.maw_second, SOURCE_USE_COOLDOWN_SECONDS),
                    _ => (self.sanity_second, SOURCE_USE_COOLDOWN_SECONDS),
                };
                let ready = before.checked_add(seconds)?;
                if now_second < ready {
                    plan.cooldown_remaining = ready - now_second;
                    return Some(plan);
                }
                match kind {
                    Kind::SpiteFire => plan.after.spite_fire_second = now_second,
                    Kind::GreedyMaw => plan.after.maw_second = now_second,
                    _ => plan.after.sanity_second = now_second,
                }
                plan.consume = correct_target;
            }
        }
        Some(plan)
    }
}
pub(crate) fn extend(
    catalogue: crate::quest::QuestStateCatalogue,
) -> Result<crate::quest::QuestStateCatalogue, String> {
    use crate::quest::native_rules::NativeRule;
    use crate::quest::{
        QuestComparison as C, QuestEffect, QuestEffectKind as E, QuestStateCatalogue, QuestTrack,
        QuestTransition,
    };
    let mut tracks: Vec<_> = catalogue.tracks.into_values().collect();
    let mut transitions: Vec<_> = catalogue.transitions.into_values().collect();
    for key in [REMORSE, SPITE_FIRE, MAW, SANITY]
        .into_iter()
        .chain(ENERGY_TRACKS)
    {
        if tracks.iter().any(|track| track.key == key) {
            return Err(format!("duplicate Soul War source track {key}"));
        }
        let bounded = key == REMORSE || ENERGY_TRACKS[1..].contains(&key);
        tracks.push(QuestTrack {
            key: key.into(),
            quest: QUEST.into(),
            initial: 0,
            min: 0,
            max: if bounded { 1 } else { i64::MAX },
        });
    }
    for (name, rule) in [
        ("remorse", NativeRule::SoulWarRemorse),
        ("energy", NativeRule::SoulWarEnergy),
    ] {
        transitions.push(QuestTransition {
            world_change: None,
            key: format!("{TRANSITION_PREFIX}{name}"),
            quest: QUEST.into(),
            effects: rule
                .tracks()
                .into_iter()
                .map(|track| QuestEffect {
                    track: track.into(),
                    from: C::Between(
                        0,
                        if track == REMORSE || ENERGY_TRACKS[1..].contains(&track) {
                            1
                        } else {
                            i64::MAX
                        },
                    ),
                    effect: E::Native(rule),
                })
                .collect(),
            completes: false,
            experience: None,
        });
    }
    for (name, track, seconds) in [
        ("spite-fire", SPITE_FIRE, SPITE_FIRE_COOLDOWN_SECONDS),
        ("greedy-maw", MAW, SOURCE_USE_COOLDOWN_SECONDS),
        ("cleansed-sanity", SANITY, SOURCE_USE_COOLDOWN_SECONDS),
    ] {
        transitions.push(QuestTransition {
            world_change: None,
            key: format!("{TRANSITION_PREFIX}{name}"),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: track.into(),
                from: C::ElapsedAtLeast(seconds as i64),
                effect: E::SetNow,
            }],
            completes: false,
            experience: None,
        });
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|error| format!("{error:?}"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_source_items_remorse_is_persistent_two_pickup_counter() -> Result<(), &'static str> {
        let first = SourceItemState::default()
            .apply(Kind::Remorse, 1000, true)
            .ok_or("first")?;
        assert_eq!(
            (first.after.remorse, first.consume, first.reset_torment),
            (1, true, false)
        );
        let second = first
            .after
            .apply(Kind::Remorse, 2000, true)
            .ok_or("second after login")?;
        assert_eq!(
            (second.after.remorse, second.consume, second.reset_torment),
            (0, true, true)
        );
        Ok(())
    }
    #[test]
    fn srx3_source_items_energy_thresholds_use_old_local_count_for_all_three_checks()
    -> Result<(), &'static str> {
        for (count, flags, granted) in [(39, 0, 1), (54, 1, 2), (69, 3, 4), (69, 0, 7)] {
            let p = SourceItemState {
                energy: count,
                floors: flags,
                ..Default::default()
            }
            .apply(Kind::Energy, 0, true)
            .ok_or("energy")?;
            assert_eq!(
                (p.after.energy, p.after.floors, p.granted_floors, p.consume),
                (0, flags | granted, granted, true)
            );
        }
        let p = SourceItemState {
            energy: 1000,
            floors: 7,
            ..Default::default()
        }
        .apply(Kind::Energy, 0, true)
        .ok_or("completed")?;
        assert_eq!((p.after.energy, p.consume), (1001, false));
        let third_only = SourceItemState {
            energy: 1000,
            floors: 4,
            ..Default::default()
        }
        .apply(Kind::Energy, 0, true)
        .ok_or("third flag alone")?;
        assert_eq!(
            (
                third_only.after.energy,
                third_only.after.floors,
                third_only.consume
            ),
            (1001, 4, false)
        );
        assert!(
            SourceItemState {
                energy: i64::MAX,
                ..Default::default()
            }
            .apply(Kind::Energy, 0, true)
            .is_none()
        );
        Ok(())
    }
    #[test]
    fn srx3_source_items_maw_wrong_target_does_not_charge_but_sanity_does()
    -> Result<(), &'static str> {
        let state = SourceItemState::default();
        let maw = state.apply(Kind::GreedyMaw, 1000, false).ok_or("maw")?;
        assert_eq!(
            (maw.after, maw.consume, maw.target_accepted),
            (state, false, false)
        );
        let sanity = state
            .apply(Kind::CleansedSanity, 1000, false)
            .ok_or("sanity")?;
        assert_eq!(
            (
                sanity.after.sanity_second,
                sanity.consume,
                sanity.target_accepted
            ),
            (1000, false, false)
        );
        let repeated = sanity
            .after
            .apply(Kind::CleansedSanity, 1029, true)
            .ok_or("cooldown")?;
        assert_eq!((repeated.cooldown_remaining, repeated.consume), (1, false));
        let ready = sanity
            .after
            .apply(Kind::CleansedSanity, 1030, true)
            .ok_or("boundary")?;
        assert_eq!((ready.after.sanity_second, ready.consume), (1030, true));
        Ok(())
    }
    #[test]
    fn srx3_source_items_spite_cooldown_equality_and_default_zero_are_literal()
    -> Result<(), &'static str> {
        let state = SourceItemState::default();
        let missing = state
            .apply(Kind::SpiteFire, 1000, false)
            .ok_or("missing actual fire")?;
        assert_eq!((missing.after, missing.consume), (state, false));
        let early = state
            .apply(Kind::SpiteFire, 55, true)
            .ok_or("initial wall seconds")?;
        assert_eq!((early.consume, early.cooldown_remaining), (false, 1));
        let ready = state.apply(Kind::SpiteFire, 56, true).ok_or("boundary")?;
        assert_eq!((ready.consume, ready.after.spite_fire_second), (true, 56));
        Ok(())
    }
    #[test]
    fn srx3_source_items_catalogue_has_stable_character_tracks_and_database_clock_rules()
    -> Result<(), String> {
        let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        let catalogue = loaded.catalogue();
        for key in [REMORSE, SPITE_FIRE, MAW, SANITY]
            .into_iter()
            .chain(ENERGY_TRACKS)
        {
            assert!(
                catalogue.track(key).is_some(),
                "missing source Character track {key}"
            );
        }
        let transition = catalogue
            .transition("oteryn:quest-transition/soul-war/source-items/spite-fire")
            .ok_or("missing source transition")?;
        assert_eq!(
            transition.effects[0].from,
            crate::quest::QuestComparison::ElapsedAtLeast(56)
        );
        assert_eq!(
            transition.effects[0].effect,
            crate::quest::QuestEffectKind::SetNow
        );
        Ok(())
    }
}
