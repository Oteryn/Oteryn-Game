use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BossRewardsPanel {
    pub selected: bool,
    pub open: bool,
    pub death: [u8; 16],
    pub after_ordinal: u16,
    pub selected_ordinal: u16,
    pub browse: Option<boss_rewards::BrowseReply>,
    pub pending: bool,
    pub feedback: Option<&'static str>,
}
