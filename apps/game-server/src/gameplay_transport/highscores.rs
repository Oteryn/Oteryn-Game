//! RANK-1 (HS-WIRE-1): the gameplay transport's command type 14 handler is
//! [`crate::durability::highscore_snapshot::reply`]; this module binds the category 10 point
//! owner to the activated Achievement catalogue.

pub(crate) use crate::durability::highscore_snapshot::reply::{HighscoresReply, answer_highscores};

/// HIGHSCORES0 §3 category 10: the activated Achievement catalogue owns the point values; a
/// retired entry counts 0 and an unknown key or an overflow is `None` (the account is refused,
/// never ranked on a guess).
impl crate::durability::highscore_snapshot::AchievementPoints
    for crate::achievement_catalogue::AchievementCatalogue
{
    fn total_points(
        &self,
        earned: &[crate::durability::account_achievement::EarnedAchievement],
    ) -> Option<u32> {
        // A page past the end carries no rows; `total_points` always sums every fact.
        self.account_achievements_page(earned, u32::MAX)
            .ok()
            .map(|page| page.total_points)
    }
}

#[cfg(test)]
#[path = "highscores_tests.rs"]
mod tests;
