//! Read the published combined circuit through the independent job tracks.
//! An absent row inherits the original credit until that job writes its own row.
use std::collections::BTreeMap;

pub const QUEST: &str = "oteryn:quest.authored.venore_daily_tasks_quest";
const PREFIX: &str = "oteryn:quest-progress/chosen/venore-daily/";
pub const STAGE: &str = "oteryn:quest-progress/chosen/venore-daily/stage";
pub const LAST_REPORT: &str = "oteryn:quest-progress/chosen/venore-daily/last-report";

/// Older saves numbered machinery in order, then Scott, Frank, and the final report.
/// Stage 13 was paid out and has the same cooldown for all three independent jobs.
pub fn inherited_value(track: &str, stored: &BTreeMap<String, i64>) -> Option<i64> {
    let stage = *stored.get(STAGE)?;
    if !(1..=13).contains(&stage) {
        return None;
    }
    match track.strip_prefix(PREFIX)? {
        "maintenance/stage" | "pathfinder/stage" | "sabotage/stage" => {
            Some(if stage == 13 { 3 } else { 1 })
        }
        "maintenance/last-report" | "pathfinder/last-report" | "sabotage/last-report" => {
            stored.get(LAST_REPORT).copied()
        }
        "scott" => Some(i64::from(stage >= 11)),
        "frank" => Some(i64::from(stage >= 12)),
        suffix => {
            let number: i64 = suffix.strip_prefix("machine-")?.parse().ok()?;
            (1..=9)
                .contains(&number)
                .then_some(i64::from(stage > number))
        }
    }
}
