use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    let preview = match (panel.fields[1].trim(), panel.fields[2].trim()) {
        (kind @ ("1" | "2" | "3"), direction @ ("0" | "1" | "2" | "3"))
            if !panel.fields[3].trim().is_empty() =>
        {
            let kind = match kind {
                "1" => "outfit",
                "2" => "mount",
                _ => "creature",
            };
            let direction = match direction {
                "0" => "north",
                "1" => "east",
                "2" => "south",
                _ => "west",
            };
            format!(
                "Preview: {kind} {} facing {direction} (unsaved)",
                panel.fields[3].trim()
            )
        }
        _ => "Preview: choose a display, direction and content key".into(),
    };
    format!(
        "House podium\r\n{preview}\r\n{}\r\n{}",
        panel.summary,
        if panel.pending {
            "Waiting for the server"
        } else if !panel.selected && panel.feedback.is_empty() {
            "Unavailable in this session"
        } else {
            &panel.feedback
        }
    )
}
