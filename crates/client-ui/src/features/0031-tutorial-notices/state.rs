use std::collections::VecDeque;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NoticePanel {
    pub open: bool,
    pub events: crate::notifications::NotificationCenter,
    pub tutorial_selected: bool,
    pub save_selected: bool,
    pub tutorial: VecDeque<String>,
    pub save_minutes: Option<u8>,
}
