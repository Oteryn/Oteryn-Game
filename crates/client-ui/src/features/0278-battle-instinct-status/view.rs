use super::BattleInstinctStatusPanel;

pub fn panel_text(panel: &BattleInstinctStatusPanel) -> String {
    if !panel.selected {
        return "Battle Instinct unavailable in this session".into();
    }
    if let Some(error) = panel.error {
        return error.into();
    }
    let Some(snapshot) = panel.snapshot else {
        return "Loading Battle Instinct…".into();
    };
    format!(
        "Battle Instinct: {}\r\nMelee: +{}\r\nShielding: +{}\r\n",
        if snapshot.active {
            "Active"
        } else {
            "Inactive"
        },
        snapshot.melee,
        snapshot.shielding
    )
}
