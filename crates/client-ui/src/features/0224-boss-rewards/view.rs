use super::*;
use std::fmt::Write;
#[must_use]
pub fn row_text(row: &boss_rewards::RewardRow) -> String {
    lfmt!(
        "#{}: item {} x{}",
        row.ordinal,
        row.definition_reference,
        row.quantity
    )
}
#[must_use]
pub fn panel_text(panel: &BossRewardsPanel) -> String {
    let mut text = String::from(tr!("Boss rewards\n"));
    if let Some(page) = &panel.browse {
        let _ = lwriteln!(
            text,
            "Death: {}",
            page.death
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        let _ = lwriteln!(text, "Expires: {}", page.expires_at_unix_ms);
        for row in &page.rows {
            let _ = writeln!(text, "{}", row_text(row));
        }
        if page.more {
            text.push_str(tr!("More rewards available\n"));
        }
    } else {
        text.push_str(tr!("Enter the server death id, then browse\n"));
    }
    if let Some(feedback) = panel.feedback {
        let _ = writeln!(text, "{feedback}");
    }
    text
}
