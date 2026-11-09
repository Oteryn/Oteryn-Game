use super::{CATEGORIES, CATEGORY_IDS, HighscoresPanel, VOCATIONS};
use std::fmt::Write;
pub fn panel_text(p: &HighscoresPanel) -> String {
    let mut text = format!(
        "Highscores — {} / {}",
        CATEGORY_IDS
            .iter()
            .position(|id| *id == p.category)
            .map(|index| CATEGORIES[index])
            .unwrap_or("Unavailable"),
        VOCATIONS[p.vocation as usize]
    );
    let _ = write!(
        text,
        " / World: {}",
        p.world.map(world_text).unwrap_or_else(|| "current".into())
    );
    if !p.selected {
        text.push_str("\nHighscores unavailable in this session");
    }
    if p.pending {
        text.push_str("\nLoading…");
    }
    if let Some(reply) = &p.reply {
        let _ = write!(
            text,
            "\nPage {} — snapshot {} (Unix milliseconds)\nRank | Name | Vocation | Level | Value",
            reply.page, reply.computed_at
        );
        for row in &reply.rows {
            let _ = write!(
                text,
                "\n{} | {} | {} | {} | {}",
                row.rank, row.name, row.vocation, row.level, row.value
            );
        }
        if reply.rows.is_empty() {
            text.push_str("\nNo rankings on this page");
        }
        if let Some(row) = &reply.own_row {
            let _ = write!(
                text,
                "\nYour rank: {} — {} | {} | level {} | {}",
                row.rank, row.name, row.vocation, row.level, row.value
            );
        } else {
            text.push_str("\nYour character is not ranked in this list");
        }
    }
    if let Some(feedback) = &p.feedback {
        let _ = write!(text, "\n{feedback}");
    }
    text
}

/// Canonical typed World identity; never accepts a Channel selector.
pub fn parse_world(text: &str) -> Option<Option<oteryn_client_runtime::highscores::wire::WorldId>> {
    let text = text.trim();
    if text.is_empty() {
        return Some(None);
    }
    let hex: String = text.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 || !hex.is_ascii() {
        return None;
    }
    let mut bytes = [0; 16];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    oteryn_client_runtime::highscores::wire::WorldId::decode(&bytes)
        .ok()
        .map(Some)
}
pub fn world_text(world: oteryn_client_runtime::highscores::wire::WorldId) -> String {
    let mut text = String::with_capacity(36);
    for (i, byte) in world.as_bytes().iter().enumerate() {
        if [4, 6, 8, 10].contains(&i) {
            text.push('-');
        }
        let _ = write!(text, "{byte:02x}");
    }
    text
}
