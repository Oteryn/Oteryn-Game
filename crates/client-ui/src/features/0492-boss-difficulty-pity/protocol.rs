//! Capability 492 `BOSS_DIFFICULTY_PITY_V1`: a progress window of a pity boss carries the
//! character's first-drop tracker. Without the capability the legacy reply decoder applies.
use oteryn_client_runtime::boss_difficulty::{self, PityTracker, Reply};

/// Decodes a command 431 reply, taking the tracker only when the session selected 492.
#[must_use]
pub fn decode(selected: bool, bytes: &[u8]) -> Option<(Reply, Option<PityTracker>)> {
    if selected {
        boss_difficulty::decode_reply_with_pity(bytes).ok()
    } else {
        boss_difficulty::decode_reply(bytes)
            .ok()
            .map(|reply| (reply, None))
    }
}

/// The bonus over the base chance as `+X.X%`, from permille of base.
#[must_use]
pub fn bonus_percent(bonus_permille: u16) -> String {
    format!("+{}.{}%", bonus_permille / 10, bonus_permille % 10)
}

/// The tracker lines of the difficulty window.
#[must_use]
pub fn tracker_text(pity: &PityTracker) -> String {
    if pity.tracker_active {
        lfmt!(
            "Bad luck bonus: {bonus}\nKills without drop: {kills}",
            bonus = bonus_percent(pity.bonus_permille),
            kills = pity.kills_without_drop
        )
    } else {
        tr!("Bad luck tracker: inactive").into()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
