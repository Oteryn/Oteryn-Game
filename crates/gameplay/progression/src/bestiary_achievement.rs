//! P11: the Achievement keys a credited Bestiary kill earns.
//!
//! Completion counts follow the Global catalogue texts (TibiaWiki, captured in
//! `imports/tibiawiki/achievements/2026-09-29/tibiawiki-achievements-facts.json`): Hunting Permit
//! (436) "your very first monster", Little Adventure (430) and Little Big Adventure (431) 10 and
//! 100 easy, Contender (432) and Serious Contender (433) 10 and 100 medium, Skilled Hunter (434)
//! and Master Hunter (435) 10 and 100 hard monsters fully unlocked. Crystal and Canary grant none
//! of them. Trail of the Ape God is five terrified elephant kills (Crystal `201189f8`
//! `data/scripts/creaturescripts/monster/terrified_elephant_death.lua:15-25`).

/// The grant request's source kind; the event id is the kill's Bestiary occurrence.
pub const SOURCE_KIND: &str = "oteryn:bestiary-kill";

/// (difficulty, fully unlocked entries, key); `None` counts every difficulty.
const COMPLETIONS: [(Option<&str>, usize, &str); 7] = [
    (None, 1, "oteryn:achievement/hunting_permit"),
    (Some("easy"), 10, "oteryn:achievement/little_adventure"),
    (Some("easy"), 100, "oteryn:achievement/little_big_adventure"),
    (Some("medium"), 10, "oteryn:achievement/contender"),
    (Some("medium"), 100, "oteryn:achievement/serious_contender"),
    (Some("hard"), 10, "oteryn:achievement/skilled_hunter"),
    (Some("hard"), 100, "oteryn:achievement/master_hunter"),
];

/// (race, kills, key).
const KILLS: [(&str, u32, &str); 1] = [(
    "oteryn:creature.terrified_elephant",
    5,
    "oteryn:achievement/trail_of_the_ape_god",
)];

/// Every key a kill can earn, for the catalogue check.
pub fn all_keys() -> impl Iterator<Item = &'static str> {
    COMPLETIONS
        .iter()
        .map(|(_, _, key)| *key)
        .chain(KILLS.iter().map(|(_, _, key)| *key))
}

/// The keys earned by holding `kill_count` kills of `race_key`.
pub fn kill_count_keys(race_key: &str, kill_count: u32) -> impl Iterator<Item = &'static str> {
    KILLS
        .iter()
        .filter(move |(race, kills, _)| *race == race_key && kill_count >= *kills)
        .map(|(_, _, key)| *key)
}

/// The keys earned by the difficulties of every fully unlocked entry.
pub fn completion_keys<'a>(completed: impl IntoIterator<Item = &'a str>) -> Vec<&'static str> {
    let mut counts = [0_usize; COMPLETIONS.len()];
    for difficulty in completed {
        for (count, (wanted, _, _)) in counts.iter_mut().zip(COMPLETIONS) {
            if wanted.is_none_or(|wanted| wanted == difficulty) {
                *count += 1;
            }
        }
    }
    counts
        .iter()
        .zip(COMPLETIONS)
        .filter(|(count, (_, needed, _))| **count >= *needed)
        .map(|(_, (_, _, key))| key)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_completed_entry_of_any_difficulty_earns_hunting_permit() {
        assert!(completion_keys([]).is_empty());
        for difficulty in ["harmless", "trivial", "easy", "challenging"] {
            assert_eq!(
                completion_keys([difficulty]),
                ["oteryn:achievement/hunting_permit"]
            );
        }
    }

    #[test]
    fn ten_and_hundred_entries_of_one_difficulty_earn_its_two_steps() {
        let keys = |difficulty, count| completion_keys(std::iter::repeat_n(difficulty, count));
        assert_eq!(keys("easy", 9), ["oteryn:achievement/hunting_permit"]);
        assert_eq!(
            keys("easy", 10),
            [
                "oteryn:achievement/hunting_permit",
                "oteryn:achievement/little_adventure"
            ]
        );
        assert_eq!(
            keys("medium", 100),
            [
                "oteryn:achievement/hunting_permit",
                "oteryn:achievement/contender",
                "oteryn:achievement/serious_contender"
            ]
        );
        assert_eq!(
            keys("hard", 99),
            [
                "oteryn:achievement/hunting_permit",
                "oteryn:achievement/skilled_hunter"
            ]
        );
        assert_eq!(
            keys("hard", 100).last(),
            Some(&"oteryn:achievement/master_hunter")
        );
        // Trivial and challenging entries count only towards Hunting Permit.
        assert_eq!(
            keys("challenging", 100),
            ["oteryn:achievement/hunting_permit"]
        );
        assert_eq!(keys("trivial", 100), ["oteryn:achievement/hunting_permit"]);
    }

    #[test]
    fn the_fifth_terrified_elephant_earns_trail_of_the_ape_god() {
        let race = "oteryn:creature.terrified_elephant";
        assert_eq!(kill_count_keys(race, 4).count(), 0);
        for kills in [5, 6, 500] {
            assert!(kill_count_keys(race, kills).eq(["oteryn:achievement/trail_of_the_ape_god"]));
        }
        assert_eq!(kill_count_keys("oteryn:creature.elephant", 5).count(), 0);
    }

    #[test]
    fn every_key_is_earnable_in_the_embedded_catalogue()
    -> Result<(), crate::achievement_catalogue::AchievementCatalogueError> {
        let catalogue =
            crate::achievement_catalogue::parse_shards(&crate::achievement_catalogue::SHARDS)?;
        for key in all_keys() {
            assert!(
                catalogue.get(key).is_some_and(|entry| !entry.retired),
                "{key}"
            );
        }
        Ok(())
    }
}
