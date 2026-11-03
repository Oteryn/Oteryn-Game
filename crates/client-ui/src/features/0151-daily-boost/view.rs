use super::DailyBoostPanel;
pub fn panel_text(p: &DailyBoostPanel) -> String {
    if !p.selected {
        return "Unavailable".into();
    }
    if let Some(f) = p.feedback {
        return f.into();
    }
    p.boost.as_ref().map_or_else(
        || "Ready".into(),
        |v| {
            format!(
                "Daily boost\nCreature: {}\nBoss: {}\nResets: {}",
                v.creature_key, v.boss_key, v.reset_epoch
            )
        },
    )
}
