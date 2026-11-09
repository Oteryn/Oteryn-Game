use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    let mut text = format!(
        "Party shields and skulls\r\n{}\r\n{}",
        panel.summary,
        if panel.pending {
            "Waiting for the server"
        } else if !panel.selected && panel.feedback.is_empty() {
            "Unavailable in this session"
        } else {
            &panel.feedback
        }
    );
    if let Some(timer) = panel.skull_timer {
        text.push_str(&format!(
            "\r\n{}",
            timer.text_at(panel.status_received_at.map_or(0, |at| {
                u64::try_from(at.elapsed().as_millis()).unwrap_or(u64::MAX)
            }))
        ));
    }
    if let Some(snapshot) = &panel.unjustified {
        text.push_str(&format!("\r\n{}", snapshot.text()));
    }
    if let Some([daily, weekly, monthly]) = panel.black_limits_milli {
        text.push_str(&format!(
            "\r\nBlack skull limits (daily / weekly / monthly): {}.{:03} / {}.{:03} / {}.{:03}",
            daily / 1000,
            daily % 1000,
            weekly / 1000,
            weekly % 1000,
            monthly / 1000,
            monthly % 1000
        ));
    }
    text
}
