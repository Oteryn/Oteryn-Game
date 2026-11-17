//! Official T174 owner unlock list (2026-11-09), read from the existing quest owner.
use crate::durability::quest_state::QuestStateCopy;

pub(super) fn powerful(family: u8, copy: &QuestStateCopy) -> bool {
    let complete = |quest: &str| {
        copy.states()
            .get(quest)
            .is_some_and(|state| state.completed)
    };
    let track = |key: &str| copy.tracks().get(key).copied().unwrap_or(0) > 0;
    let boss = |name: &str| {
        track(&format!(
            "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/{name}_killed"
        ))
    };
    let heart = complete("oteryn:quest.heart_of_destruction_quest");
    match family {
        1 | 18 => boss("last_lore") || heart,
        3 | 25 => boss("lady_tenebris"),
        11 => boss("lady_tenebris") || heart,
        12 => boss("dragonking") || heart,
        5 | 10 | 17 | 24 => boss("lloyd"),
        4 | 14 | 15 | 20 | 22 => boss("thorn_knight"),
        6 | 21 => boss("dragonking"),
        8 | 19 | 23 => boss("horror"),
        7 | 16 => boss("time_guardian"),
        9 => {
            complete("oteryn:quest.dangerous_depths_quest")
                || track("oteryn:quest-progress/quest/u11_50/dangerous_depths/questline")
        }
        13 | 26 => {
            complete("oteryn:quest.the_dream_courts_quest")
                || track("oteryn:quest-progress/quest/u12_00/the_dream_courts/main/questline")
        }
        _ => false,
    }
}
