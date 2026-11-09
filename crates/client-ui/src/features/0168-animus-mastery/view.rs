use super::*;
use std::fmt::Write;

pub fn panel_text(panel: &AnimusPanel) -> String {
    if !panel.selected {
        return tr!("Animus Mastery is unavailable in this session").into();
    }
    let mut text = String::from(tr!("Animus Mastery\r\n"));
    if panel.loaded {
        let _ = lwriteln!(text, "Completed species: {}\r", panel.total);
        if panel.total == 0 {
            text.push_str(tr!(
                "No completed species yet. Complete a Soulpit encounter to earn mastery.\r\n"
            ));
        } else if panel.keys.is_empty() {
            text.push_str(tr!("No species on this page. Refresh the list.\r\n"));
        } else {
            let _ = lwriteln!(
                text,
                "Species {}-{} of {}\r",
                usize::from(panel.offset) + 1,
                usize::from(panel.offset) + panel.keys.len(),
                panel.total
            );
            text.push_str(tr!(
                "The Animus XP bonus applies to mastered species.\r\n\r\n"
            ));
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
        text.push_str(tr!("Loading...\r\n"));
    }
    if let Some(error) = panel.error {
        text.push_str(error);
    }
    text
}
