use super::AchievementsPanel;
use std::fmt::Write;
pub fn panel_text(p: &AchievementsPanel) -> String {
    let mut text = String::from("Account achievements — Ctrl+Shift+A\r\n");
    if let Some(page) = &p.page {
        let _ = writeln!(
            text,
            "{} points — {} earned — Page {}\r",
            page.total_points,
            page.fact_count,
            u64::from(page.page) + 1
        );
        if page.rows.is_empty() {
            text.push_str("No earned achievements on this page\r\n");
        }
        for row in &page.rows {
            let _ = write!(
                text,
                "{} — Grade {} — {} points{}\r\n{}\r\nEarned: {} (server Unix ms)\r\n\r\n",
                row.name,
                row.grade,
                row.points,
                if row.secret { " — Secret" } else { "" },
                row.description,
                row.earned_at
            );
        }
    }
    if p.pending.is_some() {
        text.push_str("Loading achievements…\r\n");
    }
    if let Some(error) = p.error {
        text.push_str(error);
    }
    text
}
