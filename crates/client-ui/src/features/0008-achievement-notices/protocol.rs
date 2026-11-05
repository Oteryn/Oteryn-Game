use super::AchievementNotices;

pub fn apply(
    notices: &mut AchievementNotices,
    earned: oteryn_client_runtime::achievement_notices::AchievementEarned,
) {
    notices.push(earned.name);
}
