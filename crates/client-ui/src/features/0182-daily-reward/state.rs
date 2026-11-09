use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DailyRewardPanel {
    pub details: Option<daily_reward::details::Details>,
    pub details_pending: bool,
    pub picker: Option<daily_reward::picker::Offer>,
    pub picker_pending: Option<u32>,
    pub picker_open: bool,
    pub selected: bool,
    pub open: bool,
    pub wall: Option<daily_reward::DailyRewardWall>,
    pub pending: bool,
    /// Latest confirmed claim observations in this session, not account-wide history.
    pub claim_receipts: [Option<(u8, u32)>; 8],
    pub feedback: Option<&'static str>,
}
