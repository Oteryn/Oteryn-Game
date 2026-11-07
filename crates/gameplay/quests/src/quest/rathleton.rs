//! Rathleton ballots spend the existing Oramond Quest voting-rights custody.
//! Ballot options/ranks follow the secondary structured Rathleton guide pinned in
//! WCONTRIB evidence: Lavovskii/grimorio@dbeee0138a54b30a74dd690c4a7e6fab514c8db0,
//! quests/data/quests/rathleton-quest.json:1, sha256
//! 12b42e657f56071108198da245b16c5a53b41455a638bc289366f5b6a251c315.
//! This secondary guide defines topics, rank thresholds and fortnightly saves.
//! Fixed one-point casts, rank-zero raids and the unknown election-week phase
//! anchored at Monday 2000-01-03 are owner-authorized conservative policy.
//! Periods consume the active World epoch timestamp, not a node's wall clock.
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};

pub const QUEST: &str = "oteryn:quest.oramond_quest";
pub const POINTS_TRACK: &str = "oteryn:quest-progress/quest/u10_50/oramond_quest/voting_points";
pub const SPENT_TRACK: &str = "oteryn:quest-progress/quest/u10_50/oramond_quest/votes_spent";
pub const RANK_TRACK: &str = "oteryn:quest-progress/quest/u10_50/oramond_quest/citizen_rank";
pub const ELECTION_ANCHOR_MS: i64 = 946_857_600_000; // Monday 2000-01-03 UTC.
pub const ELECTION_PERIOD_MS: i64 = 14 * 24 * 60 * 60 * 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ballot {
    pub identity: &'static str,
    pub category: &'static str,
    pub option: &'static str,
    pub minimum_rank: i64,
    pub transition: &'static str,
}
macro_rules! ballot {
    ($category:literal,$option:literal,$minimum:literal) => {
        Ballot {
            identity: concat!($category, "/", $option),
            category: $category,
            option: $option,
            minimum_rank: $minimum,
            transition: concat!(
                "oteryn:quest-transition/world-contribution/votes/",
                $category,
                "/",
                $option
            ),
        }
    };
}
pub const BALLOTS: [Ballot; 12] = [
    ballot!("shortcut", "minotaur", 0),
    ballot!("shortcut", "fungi", 0),
    ballot!("shortcut", "catacomb", 0),
    ballot!("dungeon", "golems", 1),
    ballot!("dungeon", "minotaurs", 1),
    ballot!("dungeon", "wrathofevil", 1),
    ballot!("boss", "bullwark", 2),
    ballot!("boss", "lisa", 2),
    ballot!("boss", "gloothfairy", 2),
    ballot!("raid", "minotaurs", 0),
    ballot!("raid", "mechanical", 0),
    ballot!("raid", "wildlife", 0),
];
pub fn ballot_for_transition(key: &str) -> Option<Ballot> {
    BALLOTS
        .iter()
        .copied()
        .find(|ballot| ballot.transition == key)
}
pub fn election_period(unix_ms: i64) -> i64 {
    // Quotient subtraction avoids overflow even at extreme timestamps.
    unix_ms.div_euclid(ELECTION_PERIOD_MS)
        - ELECTION_ANCHOR_MS.div_euclid(ELECTION_PERIOD_MS)
        - i64::from(
            unix_ms.rem_euclid(ELECTION_PERIOD_MS)
                < ELECTION_ANCHOR_MS.rem_euclid(ELECTION_PERIOD_MS),
        )
}
pub fn rank_for_spent(spent: i64) -> u8 {
    match spent {
        ..50 => 0,
        50..150 => 1,
        150..300 => 2,
        300..500 => 3,
        _ => 4,
    }
}
/// Admission consumes the promoted durable rank, never merely accumulated votes.
pub fn admits_rank(rank: i64, required: u8) -> bool {
    (0..=4).contains(&rank) && required <= 4 && rank >= i64::from(required)
}
/// Tie retains incumbent; no votes never creates a winner. Counts correspond to
/// the category's three fixed options, so callers cannot invent ballot identity.
pub fn winner<'a>(options: &[(&'a str, u64); 3], incumbent: Option<&'a str>) -> Option<&'a str> {
    let highest = options.iter().map(|(_, count)| *count).max()?;
    if highest == 0 {
        return incumbent;
    }
    let mut leaders = options.iter().filter(|(_, count)| *count == highest);
    let first = leaders.next()?.0;
    if leaders.next().is_some() {
        incumbent
    } else {
        Some(first)
    }
}
pub fn dialogue(
    npc: &str,
    message: &str,
    topic: u16,
) -> Option<super::world_contributions::Dialogue> {
    if npc != "oteryn:npc.marvin" {
        return None;
    }
    let text = message.trim().to_ascii_lowercase();
    if let Some(rest) = text.strip_prefix("vote ") {
        let ballot = BALLOTS
            .iter()
            .position(|ballot| rest == format!("{} {}", ballot.category, ballot.option))?;
        return Some(super::world_contributions::Dialogue {
            topic: 100 + u16::try_from(ballot).ok()?,
            transition: None,
            reply: "Cast one earned voting right on this ballot?",
        });
    }
    if let Some(rank) = text.strip_prefix("rank ") {
        let transition = match rank {
            "commoner" => "oteryn:quest-transition/rathleton/promote/commoner",
            "inhabitant" => "oteryn:quest-transition/rathleton/promote/inhabitant",
            "citizen" => "oteryn:quest-transition/rathleton/promote/citizen",
            "squire" => "oteryn:quest-transition/rathleton/promote/squire",
            _ => return None,
        };
        return Some(super::world_contributions::Dialogue {
            topic: 0,
            transition: Some(transition),
            reply: "Your earned citizen rank has been admitted.",
        });
    }
    if !(100..112).contains(&topic) {
        return None;
    }
    match text.as_str() {
        "yes" => Some(super::world_contributions::Dialogue {
            topic: 0,
            transition: Some(BALLOTS[usize::from(topic - 100)].transition),
            reply: "Your vote was cast for this election period.",
        }),
        "no" => Some(super::world_contributions::Dialogue {
            topic: 0,
            transition: None,
            reply: "You keep your voting right.",
        }),
        _ => None,
    }
}
pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    if !catalogue.tracks.contains_key(POINTS_TRACK) {
        return Err("Oramond voting-right custody missing".into());
    }
    for (key, max) in [(SPENT_TRACK, i64::MAX), (RANK_TRACK, 4)] {
        tracks.push(QuestTrack {
            key: key.into(),
            quest: QUEST.into(),
            initial: 0,
            min: 0,
            max,
        });
    }
    for ballot in BALLOTS {
        transitions.push(QuestTransition {
            key: ballot.transition.into(),
            quest: QUEST.into(),
            completes: false,
            experience: None,
            effects: vec![
                QuestEffect {
                    track: POINTS_TRACK.into(),
                    from: QuestComparison::Ge(1),
                    effect: QuestEffectKind::Add(-1),
                },
                QuestEffect {
                    track: SPENT_TRACK.into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Add(1),
                },
                QuestEffect {
                    track: RANK_TRACK.into(),
                    from: QuestComparison::Ge(ballot.minimum_rank),
                    effect: QuestEffectKind::Add(0),
                },
            ],
        });
    }
    // Rank promotion consumes no points. Rank admission still reads the durable
    // promoted rank, never an NPC display snapshot or an election aggregate.
    for (name, rank, spent) in [
        ("commoner", 1, 50),
        ("inhabitant", 2, 150),
        ("citizen", 3, 300),
        ("squire", 4, 500),
    ] {
        transitions.push(QuestTransition {
            key: format!("oteryn:quest-transition/rathleton/promote/{name}"),
            quest: QUEST.into(),
            completes: false,
            experience: None,
            effects: vec![
                QuestEffect {
                    track: SPENT_TRACK.into(),
                    from: QuestComparison::Ge(spent),
                    effect: QuestEffectKind::Add(0),
                },
                QuestEffect {
                    track: RANK_TRACK.into(),
                    from: QuestComparison::Lt(rank),
                    effect: QuestEffectKind::Set(rank),
                },
            ],
        });
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|error| format!("{error:?}"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rathleton_boundaries_and_ballot_identity() {
        assert_eq!(election_period(ELECTION_ANCHOR_MS - 1), -1);
        assert_eq!(election_period(ELECTION_ANCHOR_MS), 0);
        assert_eq!(election_period(ELECTION_ANCHOR_MS + ELECTION_PERIOD_MS), 1);
        assert_eq!(rank_for_spent(49), 0);
        assert_eq!(rank_for_spent(50), 1);
        assert_eq!(rank_for_spent(149), 1);
        assert_eq!(rank_for_spent(150), 2);
        assert_eq!(rank_for_spent(299), 2);
        assert_eq!(rank_for_spent(300), 3);
        assert_eq!(rank_for_spent(499), 3);
        assert_eq!(rank_for_spent(500), 4);
        for ballot in BALLOTS {
            assert_eq!(ballot_for_transition(ballot.transition), Some(ballot));
        }
        assert_eq!(
            winner(&[("a", 3), ("b", 3), ("c", 0)], Some("c")),
            Some("c")
        );
        assert_eq!(
            winner(&[("a", 3), ("b", 4), ("c", 0)], Some("c")),
            Some("b")
        );
        assert_eq!(winner(&[("a", 0), ("b", 0), ("c", 0)], None), None);
        assert!(dialogue("oteryn:npc.marvin", "vote shortcut fungi", 0).is_some());
        assert!(dialogue("oteryn:npc.junkar_thais", "yes", 101).is_none());
        assert!(dialogue("oteryn:npc.marvin", "yes", 1).is_none());
    }
}

#[cfg(test)]
mod catalogue_tests {
    use super::*;
    use std::collections::BTreeMap;

    fn catalogue() -> Result<QuestStateCatalogue, String> {
        let existing = QuestStateCatalogue::new(
            "rathleton-r1",
            vec![QuestTrack {
                key: POINTS_TRACK.into(),
                quest: QUEST.into(),
                initial: -1,
                min: -1,
                max: i64::MAX,
            }],
            Vec::new(),
        )
        .map_err(|error| format!("{error:?}"))?;
        extend(existing)
    }

    #[test]
    fn rathleton_votes_require_promoted_rank_and_preserve_it() -> Result<(), String> {
        let catalogue = catalogue()?;
        let ballot = BALLOTS[3];
        let transition = catalogue.transition(ballot.transition).ok_or("ballot")?;
        let mut stored = BTreeMap::from([
            (POINTS_TRACK.into(), 2),
            (SPENT_TRACK.into(), 50),
            (RANK_TRACK.into(), 0),
        ]);
        assert_eq!(
            catalogue.evaluate(transition, &stored, 0),
            Err(super::super::QuestRefusal::StageMismatch)
        );
        let promotion = catalogue
            .transition("oteryn:quest-transition/rathleton/promote/commoner")
            .ok_or("promotion")?;
        let promoted = catalogue
            .evaluate(promotion, &stored, 0)
            .map_err(|error| format!("{error:?}"))?;
        assert!(promoted.iter().any(|change| change.track == SPENT_TRACK
            && change.before == 50
            && change.after == 50));
        for change in promoted {
            stored.insert(change.track, change.after);
        }
        let changes = catalogue
            .evaluate(transition, &stored, 0)
            .map_err(|error| format!("{error:?}"))?;
        assert!(
            changes.iter().any(|change| change.track == POINTS_TRACK
                && change.before == 2
                && change.after == 1)
        );
        assert!(changes.iter().any(|change| change.track == SPENT_TRACK
            && change.before == 50
            && change.after == 51));
        assert!(
            changes.iter().any(|change| change.track == RANK_TRACK
                && change.before == 1
                && change.after == 1)
        );
        assert!(!admits_rank(0, 1));
        assert!(admits_rank(1, 1));
        assert!(!admits_rank(1, 2));
        assert!(admits_rank(2, 2));
        Ok(())
    }

    #[test]
    fn rathleton_promotions_have_exact_lifetime_spent_thresholds() -> Result<(), String> {
        let catalogue = catalogue()?;
        for (name, rank, threshold) in [
            ("commoner", 1, 50),
            ("inhabitant", 2, 150),
            ("citizen", 3, 300),
            ("squire", 4, 500),
        ] {
            let key = format!("oteryn:quest-transition/rathleton/promote/{name}");
            let transition = catalogue.transition(&key).ok_or("promotion")?;
            let mut stored = BTreeMap::from([(SPENT_TRACK.into(), threshold - 1)]);
            assert_eq!(
                catalogue.evaluate(transition, &stored, 0),
                Err(super::super::QuestRefusal::StageMismatch)
            );
            stored.insert(SPENT_TRACK.into(), threshold);
            let promoted = catalogue
                .evaluate(transition, &stored, 0)
                .map_err(|error| format!("{error:?}"))?;
            assert!(
                promoted
                    .iter()
                    .any(|change| change.track == RANK_TRACK && change.after == rank)
            );
        }
        Ok(())
    }

    #[test]
    fn rathleton_recipes_have_one_owned_point_and_fixed_ballot_identity() {
        for ballot in BALLOTS {
            let recipe = super::super::world_contributions::recipe(ballot.transition);
            assert!(recipe.is_some_and(|recipe| {
                recipe.kind == super::super::world_contributions::Kind::Votes
                    && recipe.amount == 1
                    && recipe.item_key.is_none()
                    && recipe.points_track == Some(POINTS_TRACK)
            }));
        }
        assert!(
            ballot_for_transition("oteryn:quest-transition/world-contribution/votes/boss/unknown")
                .is_none()
        );
    }
}
