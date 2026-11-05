use super::{AchievementEarned, AchievementWatermark};
use std::collections::VecDeque;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AchievementNoticesPanel {
    pub selected: bool,
    pub open: bool,
    pub watermark: Option<AchievementWatermark>,
    /// The latest 32 notices, in server order, for this session only.
    pub recent: VecDeque<AchievementEarned>,
}
