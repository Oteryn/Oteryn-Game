//! The server-owned seven-day one-click daily reward catalogue (Winter Update 2026, WU26-07).
use super::native_gameplay::NativeGameplayState;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DailyRewardContent {
    pub schema: String,
    pub revision: String,
    pub entry_shrine: DailyRewardShrine,
    pub shrine_source_ids: Vec<u32>,
    pub days: Vec<DailyRewardDay>,
    #[serde(default)]
    premium_day_seven_boost_windows: Vec<DailyRewardBoostWindow>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DailyRewardBoostWindow {
    first_reset_date: i32,
    last_reset_date: i32,
    source: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DailyRewardShrine {
    pub placement: String,
    pub coordinate_frame: String,
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DailyRewardDay {
    pub day: u8,
    pub free: DailyRewardTier,
    pub premium: DailyRewardTier,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DailyRewardTier {
    pub cosmetic_tokens: u8,
    pub prey_wildcards: u8,
    pub temple_teleport_scrolls: u8,
    pub xp_boost_percent: u8,
    pub xp_boost_minutes: u8,
    pub bounty_reroll_tokens: u8,
}
impl DailyRewardContent {
    /// The authored table is the runtime reward authority.
    pub fn catalogue() -> Option<&'static Self> {
        static CONTENT: std::sync::OnceLock<Option<DailyRewardContent>> =
            std::sync::OnceLock::new();
        CONTENT.get_or_init(Self::parse).as_ref()
    }

    fn parse() -> Option<Self> {
        let content: Self = serde_json::from_str(include_str!(
            "../../../../../content/interactions/daily-rewards.json"
        ))
        .ok()?;
        (content.schema == "OTERYN_DAILY_REWARDS/v2"
            && content
                .premium_day_seven_boost_windows
                .iter()
                .all(|window| {
                    !window.source.is_empty() && window.first_reset_date <= window.last_reset_date
                })
            && content.days.len() == 7
            && content.days.iter().enumerate().all(|(index, day)| {
                day.day == u8::try_from(index + 1).unwrap_or_default()
                    && [&day.free, &day.premium].iter().all(|tier| {
                        tier.bounty_reroll_tokens == 1
                            && tier.cosmetic_tokens <= 10
                            && tier.prey_wildcards <= 2
                            && tier.temple_teleport_scrolls <= 1
                            && matches!(tier.xp_boost_percent, 0 | 10 | 20 | 25 | 50)
                            && matches!(tier.xp_boost_minutes, 0 | 30 | 45 | 60)
                            && (tier.xp_boost_percent == 0) == (tier.xp_boost_minutes == 0)
                    })
            }))
        .then_some(content)
    }

    pub fn load(_native: &NativeGameplayState) -> Option<&'static Self> {
        Self::catalogue()
    }

    pub fn reward(&self, day: u8, premium: bool) -> Option<DailyRewardTier> {
        let row = self.days.get(usize::from(day.checked_sub(1)?))?;
        Some(if premium { row.premium } else { row.free })
    }

    /// Resolve the advertised tier for a committed server-save date (YYYYMMDD).
    pub fn reward_on_reset(
        &self,
        day: u8,
        premium: bool,
        reset_date: i32,
    ) -> Option<DailyRewardTier> {
        let mut tier = self.reward(day, premium)?;
        // The Fandom event archive confirms a 50% one-hour Premium day-seven boost.
        // Its other entries describe a different reward catalogue.
        if premium
            && day == 7
            && (tier.xp_boost_percent, tier.xp_boost_minutes) == (50, 30)
            && self.premium_day_seven_boost_windows.iter().any(|window| {
                (window.first_reset_date..=window.last_reset_date).contains(&reset_date)
            })
        {
            tier.xp_boost_minutes = 60;
        }
        Some(tier)
    }
}
