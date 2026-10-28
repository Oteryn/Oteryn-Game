//! Presentation state for capability 8 achievement advancement notices.

use oteryn_client_runtime::achievement_notices::AchievementEarned;
use std::collections::VecDeque;

mod protocol;
pub use protocol::request;

/// Maximum advancement lines retained for the current client session.
pub const MAX_ADVANCEMENT_LOG_LINES: usize = 100;

/// The Tibia-compatible advancement line used by both the pop-up and the log.
#[must_use]
pub fn advancement_text(name: &str) -> String {
    format!("Congratulations! You earned the achievement \"{name}\".")
}

/// View data for the transient achievement pop-up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchievementPopup {
    pub name: String,
    pub text: String,
}

/// Client-owned presentation state. The newest grant replaces the pop-up and is appended once
/// to the bounded advancement log; reconnect snapshots only carry a watermark and add no line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AchievementNotices {
    popup: Option<AchievementPopup>,
    advancement_log: VecDeque<String>,
}

/// Registration entry for the advancement log panel.
#[must_use]
pub fn advancement_log_panel() -> AchievementNotices {
    AchievementNotices::default()
}

/// Registration entry for the transient pop-up view.
#[must_use]
pub fn advancement_popup(notices: &AchievementNotices) -> Option<&AchievementPopup> {
    notices.popup()
}

impl AchievementNotices {
    pub fn apply(&mut self, earned: AchievementEarned) {
        let text = advancement_text(&earned.name);
        self.popup = Some(AchievementPopup {
            name: earned.name,
            text: text.clone(),
        });
        if self.advancement_log.len() == MAX_ADVANCEMENT_LOG_LINES {
            self.advancement_log.pop_front();
        }
        self.advancement_log.push_back(text);
    }

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_client_runtime::achievement_notices::AchievementWatermark;

    fn earned(name: &str, facts: u32) -> AchievementEarned {
        AchievementEarned {
            key: "oteryn:achievement/allow_cookies".into(),
            name: name.into(),
            watermark: AchievementWatermark {
                fact_count: facts,
                total_points: facts,
            },
        }
    }

    #[test]
    fn grant_presents_popup_and_one_advancement_line() {
        let mut notices = AchievementNotices::default();
        notices.apply(earned("Allow Cookies?!", 1));
        assert_eq!(
            notices.popup().map(|popup| popup.text.as_str()),
            Some("Congratulations! You earned the achievement \"Allow Cookies?!\".")
        );
        assert_eq!(notices.advancement_log().count(), 1);
        notices.dismiss_popup();
        assert!(notices.popup().is_none());
    }

    #[test]
    fn log_is_bounded_and_preserves_newest_advancements() {
        let mut notices = AchievementNotices::default();
        for index in 0..=MAX_ADVANCEMENT_LOG_LINES {
            notices.apply(earned(&format!("Achievement {index}"), index as u32 + 1));
        }
        assert_eq!(notices.advancement_log().len(), MAX_ADVANCEMENT_LOG_LINES);
        assert_eq!(
            notices.advancement_log().next(),
            Some("Congratulations! You earned the achievement \"Achievement 1\".")
        );
    }
}
