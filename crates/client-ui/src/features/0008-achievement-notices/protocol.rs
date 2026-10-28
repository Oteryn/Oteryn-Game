//! Capability-8 values arrive validated through `oteryn-session` and `client-runtime`.

/// Encodes an account-achievement page request for command 10.
#[must_use]
pub fn request(page: u32) -> Vec<u8> {
    oteryn_client_runtime::achievement_notices::encode_account_achievements_query(
        oteryn_client_runtime::achievement_notices::AccountAchievementsQuery { page },
    )
}
