use super::{CATEGORIES, CATEGORY_IDS, HighscoresPanel, VOCATIONS};
use std::fmt::Write;
pub fn panel_text(p: &HighscoresPanel) -> String {
    let mut text = lfmt!(
        "Highscores — {} / {}",
        CATEGORY_IDS
            .iter()
            .position(|id| *id == p.category)
            .map(|index| crate::localization::tr(CATEGORIES[index]))
            .unwrap_or(tr!("Unavailable")),
        crate::localization::tr(VOCATIONS[p.vocation as usize])
    );
    if !p.selected {
        text.push_str(tr!("\nHighscores unavailable in this session"));
    }
    if p.pending {
        text.push_str(tr!("\nLoading…"));
    }
    if let Some(reply) = &p.reply {
        let _ = lwrite!(
            text,
            "\nPage {} — snapshot {} (Unix milliseconds)\nRank | Name | Vocation | Level | Value",
            reply.page,
            reply.computed_at
        );
        for row in &reply.rows {
            let _ = write!(
                text,
                "\n{} | {} | {} | {} | {}",
                row.rank, row.name, row.vocation, row.level, row.value
            );
        }
        if reply.rows.is_empty() {
            text.push_str(tr!("\nNo rankings on this page"));
        }
        if let Some(row) = &reply.own_row {
            let _ = lwrite!(
                text,
                "\nYour rank: {} — {} | {} | level {} | {}",
                row.rank,
                row.name,
                row.vocation,
                row.level,
                row.value
            );
        } else {
            text.push_str(tr!("\nYour character is not ranked in this list"));
        }
    }
    if let Some(feedback) = &p.feedback {
        let _ = write!(text, "\n{feedback}");
    }
    text
}
