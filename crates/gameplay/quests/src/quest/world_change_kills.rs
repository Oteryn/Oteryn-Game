//! Bounded lifetime counters owned by the existing Quest writer.
//!
//! Spider Nest: retained wiki page 76262/rev887154, line 6, body SHA256
//! c238c55a16f6c740250e13a288ffe4226aea1f227338b197bb189626f68c9e75.
//! No reset is stated: the conservative rule is three lifetime kills, then no
//! further entry. Return portals remain independent of this admission counter.
//! Bored: Crystal 201189f, giant_spider_wyda_death.lua:1-12, SHA256
//! 4b78cf145d150264e8630ec73381e66b0fa4e1dca855292b93505805acacf0c2.
//! Down the Drain: retained wiki page76219/rev887142, line8, body SHA256
//! c3b3859f63be056d74f41a6edf13dc7a75368b24659144cd6cca2e01737aa355.

use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreditedKill {
    pub creature: &'static str,
    pub change: &'static str,
    pub quest: &'static str,
    pub track: &'static str,
    pub transition: &'static str,
    pub maximum: i64,
    pub achievement: &'static str,
    pub notice: Option<&'static str>,
}

pub const SPIDER_NEST: CreditedKill = CreditedKill {
    creature: "oteryn:creature.mamma_longlegs",
    change: "spider_nest",
    quest: "oteryn:quest.world_change_spider_nest",
    track: "oteryn:quest-progress/world-change/spider-nest/credited-kills",
    transition: "oteryn:quest-transition/world-change/spider-nest/credited-kill",
    maximum: 3,
    achievement: "oteryn:achievement/nestling",
    notice: None,
};
pub const BORED: CreditedKill = CreditedKill {
    creature: "oteryn:creature.giant_spider_wyda",
    change: "bored",
    quest: "oteryn:quest.world_change_bored",
    track: "oteryn:quest-progress/world-change/bored/credited-kills",
    transition: "oteryn:quest-transition/world-change/bored/credited-kill",
    maximum: 1,
    achievement: "oteryn:achievement/someone_s_bored",
    notice: Some("It seems this was just an illusion."),
};
pub const DOWN_THE_DRAIN: CreditedKill = CreditedKill {
    creature: "oteryn:creature.water_elemental",
    change: "down_the_drain",
    quest: "oteryn:quest.world_change_down_the_drain",
    track: "oteryn:quest-progress/world-change/down-the-drain/credited-kills",
    transition: "oteryn:quest-transition/world-change/down-the-drain/credited-kill",
    maximum: 50,
    achievement: "oteryn:achievement/down_the_drain",
    notice: None,
};
// Warpath spoiler page76270/rev944151, source body SHA256
//25a6dec3aceaee5d95e56659c96ff4673ae2db757a8105dcd380fd1da1c40e9d:
//https://tibia.fandom.com/index.php?curid=76270&oldid=944151.
pub const BIBBY: CreditedKill = CreditedKill {
    creature: "oteryn:creature.bibby_bloodbath",
    change: "warpath",
    quest: "oteryn:quest.world_change_warpath",
    track: "oteryn:quest-progress/world-change/warpath/credited-kills",
    transition: "oteryn:quest-transition/world-change/warpath/credited-kill",
    maximum: 1,
    achievement: "oteryn:achievement/bibby_s_bloodbath",
    notice: None,
};
pub const DEFINITIONS: [CreditedKill; 4] = [SPIDER_NEST, BORED, DOWN_THE_DRAIN, BIBBY];

pub fn for_creature(creature: &str) -> Option<CreditedKill> {
    DEFINITIONS
        .into_iter()
        .find(|rule| rule.creature == creature)
}
pub fn for_transition(transition: &str) -> Option<CreditedKill> {
    DEFINITIONS
        .into_iter()
        .find(|rule| rule.transition == transition)
}

/// Adds a fixed number of bounded tracks. Neither reset epochs nor occurrences
/// create new track names; idempotency belongs to the existing Quest receipts.
pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for rule in DEFINITIONS {
        tracks.push(QuestTrack {
            key: rule.track.into(),
            quest: rule.quest.into(),
            initial: 0,
            min: 0,
            max: rule.maximum,
        });
        transitions.push(QuestTransition {
            key: rule.transition.into(),
            quest: rule.quest.into(),
            effects: vec![QuestEffect {
                track: rule.track.into(),
                from: QuestComparison::Between(0, rule.maximum - 1),
                effect: QuestEffectKind::Add(1),
            }],
            completes: false,
            experience: None,
        });
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn counters_reach_exact_source_limit_then_refuse() -> Result<(), String> {
        let catalogue = extend(QuestStateCatalogue::empty("test").ok_or("empty catalogue")?)?;
        for rule in DEFINITIONS {
            let transition = catalogue.transition(rule.transition).ok_or("transition")?;
            let mut stored = BTreeMap::new();
            for expected in 1..=rule.maximum {
                let changes = catalogue
                    .evaluate(transition, &stored, 0)
                    .map_err(|error| format!("{error:?}"))?;
                assert_eq!(changes.len(), 1);
                assert_eq!(changes[0].after, expected);
                stored.insert(rule.track.to_owned(), expected);
            }
            assert_eq!(
                catalogue.evaluate(transition, &stored, 0),
                Err(super::super::QuestRefusal::StageMismatch)
            );
        }
        Ok(())
    }

    #[test]
    fn native_identity_is_exact_and_does_not_match_ordinary_spiders() {
        assert_eq!(
            for_creature("oteryn:creature.mamma_longlegs"),
            Some(SPIDER_NEST)
        );
        assert_eq!(
            for_creature("oteryn:creature.giant_spider_wyda"),
            Some(BORED)
        );
        for key in [
            "Mamma Longlegs",
            "oteryn:creature.giant_spider",
            "oteryn:creature.GIANT_SPIDER_WYDA",
        ] {
            assert_eq!(for_creature(key), None);
        }
    }
}
