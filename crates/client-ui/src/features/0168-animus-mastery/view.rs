use super::*;
use std::fmt::Write;

pub fn panel_text(panel: &AnimusPanel) -> String {
    if !panel.selected {
        return "Animus Mastery is unavailable in this session".into();
    }
    let mut text = String::from("Animus Mastery\r\n");
    if panel.loaded {
        let _ = writeln!(text, "Completed species: {}\r", panel.total);
        if panel.total == 0 {
            text.push_str(
                "No completed species yet. Complete a Soulpit encounter to earn mastery.\r\n",
            );
        } else if panel.keys.is_empty() {
            text.push_str("No species on this page. Refresh the list.\r\n");
        } else {
            let _ = writeln!(
                text,
                "Species {}-{} of {}\r",
                usize::from(panel.offset) + 1,
                usize::from(panel.offset) + panel.keys.len(),
                panel.total
            );
            text.push_str("The Animus XP bonus applies to mastered species.\r\n\r\n");
            for key in &panel.keys {
                let label = key
                    .strip_prefix("oteryn:creature.")
                    .unwrap_or(key)
                    .replace('_', " ");
                let _ = writeln!(text, "{label}\r");
            }
        }
    }
    if panel.pending.is_some() {
        text.push_str("Loading...\r\n");
    }
    if let Some(error) = panel.error {
        text.push_str(error);
    }
    text
}
