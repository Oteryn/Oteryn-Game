use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DailyRewardPanel {
    pub selected: bool,
    pub open: bool,
    pub wall: Option<daily_reward::DailyRewardWall>,
    pub pending: bool,
    pub feedback: Option<&'static str>,
}
