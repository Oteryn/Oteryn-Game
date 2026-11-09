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

pub const MAX_ADVANCEMENT_LOG_LINES: usize = 100;

#[must_use]
pub fn advancement_text(name: &str) -> String {
    lfmt!(
        "Congratulations! You earned the achievement \"{name}\".",
        name = name
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchievementPopup {
    pub name: String,
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AchievementNotices {
    pub selected: bool,
    popup: Option<AchievementPopup>,
    advancement_log: VecDeque<String>,
}

impl AchievementNotices {
    #[must_use]
    pub fn popup(&self) -> Option<&AchievementPopup> {
        self.popup.as_ref()
    }

    pub fn dismiss_popup(&mut self) {
        self.popup = None;
    }

    pub fn advancement_log(&self) -> impl ExactSizeIterator<Item = &str> {
        self.advancement_log.iter().map(String::as_str)
    }

    pub(crate) fn push(&mut self, name: String) {
        let text = advancement_text(&name);
        self.popup = Some(AchievementPopup {
            name,
            text: text.clone(),
        });
        if self.advancement_log.len() == MAX_ADVANCEMENT_LOG_LINES {
            self.advancement_log.pop_front();
        }
        self.advancement_log.push_back(text);
    }
}
