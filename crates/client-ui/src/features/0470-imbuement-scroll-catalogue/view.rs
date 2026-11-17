/// Canonical imbuement names are shared by all client languages.
#[must_use]
pub fn family_name(family: u8) -> &'static str {
    match family {
        1 => "Strike",
        2 => "Elemental Damage",
        3 => "Lich Shroud",
        4 => "Snake Skin",
        5 => "Cloud Fabric",
        6 => "Dragon Hide",
        7 => "Demon Presence",
        8 => "Quara Scale",
        9 => "Featherweight",
        10 => "Swiftness",
        11 => "Vampirism",
        12 => "Void",
        13 | 26 => "Vibrancy",
        14 => "Chop",
        15 => "Bash",
        16 => "Precision",
        17 => "Punch",
        18 => "Epiphany",
        19 => "Blockade",
        20 => "Slash",
        21 => "Scorch",
        22 => "Venom",
        23 => "Frost",
        24 => "Electrify",
        25 => "Reap",
        _ => "",
    }
}

/// The catalogue extension uses the existing shrine window and its slot controls.
#[must_use]
pub fn panel_text(panel: &super::ImbuePanel) -> String {
    super::view::panel_text(panel)
}
