use super::DailyBoostPanel;
pub fn panel_text(p: &DailyBoostPanel) -> String {
    if !p.selected {
        return "Unavailable".into();
    }
    if let Some(f) = p.feedback {
        return f.into();
    }
    if p.pending {
        return "Loading daily boost...".into();
    }
    p.boost.as_ref().map_or_else(
        || "Ready".into(),
        |v| {
            format!(
                "Daily boost\nCreature: {}\nBoss: {}\nReset epoch: {}",
                v.creature_key, v.boss_key, v.reset_epoch
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loading_hides_previous_daily_selection() {
        let panel = DailyBoostPanel {
            selected: true,
            pending: true,
            boost: Some(oteryn_client_runtime::cui3::daily_boost::DailyBoostView {
                reset_epoch: 9,
                creature_key: "rat".into(),
                boss_key: "boss".into(),
            }),
            ..Default::default()
        };
        assert_eq!(panel_text(&panel), "Loading daily boost...");
    }
}
