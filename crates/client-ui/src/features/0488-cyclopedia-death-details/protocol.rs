//! Capability 488 `CYCLOPEDIA_DEATH_DETAILS_V1`: Recent Deaths rows carry the level at death, the
//! final-blow killer, the killer that dealt the most damage and whether the death was unjustified.
use crate::cyclopedia_character::HistoryRow;
use oteryn_client_runtime::cyclopedia_character::KillStatus;

/// The column heading of the history table; `detailed` once the rows carry death details.
#[must_use]
pub fn history_columns(detailed: bool) -> &'static str {
    if detailed {
        "Time (server Unix ms)     Result     Level     Killer     Most damage     Unjustified     Description"
    } else {
        "Time (server Unix ms)     Result     Description"
    }
}

/// The tab-separated table row of one history entry. A Recent Deaths row with details shows its
/// level, killer, most-damage dealer (an unknown one as `unknown`) and justification before the
/// cause; any other row keeps the time, result and description columns.
#[must_use]
pub fn history_label(row: &HistoryRow) -> String {
    let kind = match row.status {
        Some(KillStatus::Unjustified) => "Unjustified",
        Some(KillStatus::GuildWar) => "Guild war",
        None => "Death",
    };
    let Some(death) = &row.death else {
        return format!("{}\t{}\t{}", row.at, kind, row.description);
    };
    let name = |name: &str| {
        if name.is_empty() {
            "unknown".to_owned()
        } else {
            name.to_owned()
        }
    };
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}",
        row.at,
        kind,
        death.level,
        name(&death.killer),
        name(&death.most_damage),
        if death.unjustified { "Yes" } else { "No" },
        row.description
    )
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
