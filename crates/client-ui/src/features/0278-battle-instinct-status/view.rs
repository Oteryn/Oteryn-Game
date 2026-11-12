use super::BattleInstinctStatusPanel;

pub fn panel_text(panel: &BattleInstinctStatusPanel) -> String {
    if !panel.selected {
        return tr!("Battle Instinct unavailable in this session").into();
    }
    if let Some(error) = panel.error {
        return error.into();
    }
    let Some(snapshot) = panel.snapshot else {
        return tr!("Loading Battle Instinct…").into();
    };
    lfmt!(
        "Battle Instinct: {}\r\nMelee: +{}\r\nShielding: +{}\r\n",
        // Crystal player_wheel.cpp:2981-3004,3397-3418: Wheel eligibility
        // and the adjacent-monster census determine separate states.
        if !snapshot.active {
            tr!("Inactive")
        } else if snapshot.melee == 0 && snapshot.shielding == 0 {
            tr!("Eligible")
        } else {
            tr!("Active")
        },
        snapshot.melee,
        snapshot.shielding
    )
}
