use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    format!(
        "Bestiary and Charms\r\n{}\r\n{}",
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

pub fn bestiary_text(panel: &Panel) -> String {
    use std::fmt::Write;
    let mut out = format!("Bestiary — {:?}\r\n", panel.race_filter);
    if !panel.selected {
        return "Bestiary unavailable in this session".into();
    }
    if panel.reply.is_none() {
        return "Query to load Bestiary progress".into();
    }
    for race in panel.race_cards() {
        let _ = write!(
            out,
            "{}Race {}: {} kills\r\n",
            if panel.fields[1] == race.race.to_string() {
                "> "
            } else {
                ""
            },
            race.race,
            race.kill_count
        );
        for (index, goal) in race.kill_thresholds.iter().enumerate() {
            let _ = write!(
                out,
                "  Stage {}: {}/{} {}\r\n",
                index + 1,
                race.kill_count.min(*goal),
                goal,
                if race.kill_count >= *goal {
                    "complete"
                } else {
                    "in progress"
                }
            );
        }
    }
    out.push_str("Creature names, illustrations, combat statistics, resistances and loot are not supplied by this progress snapshot.\r\n");
    out
}
