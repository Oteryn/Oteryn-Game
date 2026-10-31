use super::VocationPanel;

/// The native shell uses its existing title status surface for this five-key chooser.
pub fn panel_text(panel: &VocationPanel) -> &'static str {
    if panel.pending {
        "Choosing vocation…"
    } else if panel.refused {
        "Choice refused. Choose: 1 Sorcerer · 2 Druid · 3 Paladin · 4 Knight · 5 Monk"
    } else {
        "Choose vocation: 1 Sorcerer · 2 Druid · 3 Paladin · 4 Knight · 5 Monk"
    }
}
