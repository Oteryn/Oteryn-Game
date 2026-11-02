use super::PlayerTypingPanel;
use std::fmt::Write;

pub fn panel_text(panel: &PlayerTypingPanel) -> String {
    let mut text = format!(
        "Player typing: {}\nVisible typing players: {}",
        if panel.own_typing { "on" } else { "off" },
        panel.indicators.len()
    );
    for indicator in &panel.indicators {
        let _ = write!(
            text,
            "\n{:02x?} generation {}",
            indicator.identity, indicator.generation
        );
    }
    if let Some(feedback) = panel.feedback {
        let _ = write!(text, "\n{feedback}");
    }
    text
}
