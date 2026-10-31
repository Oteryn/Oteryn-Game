//! Newhaven's NPC tasks share the authored Newhaven quest and its receipt owner.
use std::collections::BTreeMap;

pub const START: &str = "oteryn:quest-progress/crystalserver/quest/u15_12/newhaven_goblin_quest_start";
pub const DONE: &str = "oteryn:quest-progress/crystalserver/quest/u15_12/newhaven_goblin_quest_done";
pub const KILLS: &str = "oteryn:quest-progress/crystalserver/quest/u15_12/newhaven_muglex_kill_count";
pub const BOOK: &str = "oteryn:quest-progress/crystalserver/quest/u15_12/newhaven_viola_book_quest_start";
pub const BEGIN: &str = "oteryn:quest-transition/newhaven/gustavo-start";
pub const KILL: &str = "oteryn:quest-transition/newhaven/gustavo-kill";
pub const FINISH: &str = "oteryn:quest-transition/newhaven/gustavo-finish";
pub const BOOK_BEGIN: &str = "oteryn:quest-transition/newhaven/viola-start";

/// Same principal as Task Board/Bestiary; no additional party credit is introduced.
pub fn kill_credit(tracks: &BTreeMap<String, i64>, race: &str) -> bool {
    matches!(race, "oteryn:creature.muglex_clan_footman" | "oteryn:creature.muglex_clan_assassin")
        && tracks.get(START) == Some(&1)
        && tracks.get(DONE) != Some(&1)
        && tracks.get(KILLS).copied().unwrap_or(0) < 20
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credited_muglex_kills_only_during_task_and_capped_at_twenty() {
        let mut tracks = BTreeMap::new();
        let footman = "oteryn:creature.muglex_clan_footman";
        assert!(!kill_credit(&tracks, footman));
        tracks.insert(START.into(), 1);
        for race in [footman, "oteryn:creature.muglex_clan_assassin"] {
            assert!(kill_credit(&tracks, race));
        }
        for race in ["oteryn:creature.muglex_clan_chief", "oteryn:creature.muglex_clan_scavenger"] {
            assert!(!kill_credit(&tracks, race));
        }
        tracks.insert(KILLS.into(), 20);
        assert!(!kill_credit(&tracks, footman));
        tracks.insert(KILLS.into(), 19);
        tracks.insert(DONE.into(), 1);
        assert!(!kill_credit(&tracks, footman));
    }
}
