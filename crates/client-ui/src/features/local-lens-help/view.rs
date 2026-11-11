/// The identifiers the lens resolves: the HUD panel names, the minimap panel id and the two
/// areas of the game window.
pub const KEYS: [&str; 7] = [
    "resources",
    "combat",
    "conditions",
    "monk",
    "minimap",
    "creature",
    "world",
];

/// The help text of an element, if the catalog has one.
#[must_use]
pub fn help_text(key: &str) -> Option<&'static str> {
    Some(match key {
        "resources" => {
            tr!("Status panel: your health, mana and resources, with the movement and audio state.")
        }
        "combat" => tr!("Combat panel: the state of your current fight and target."),
        "conditions" => {
            tr!("Conditions panel: the conditions on your character, with rest and stamina.")
        }
        "monk" => tr!("Monk panel: the resources of the monk vocation."),
        "minimap" => {
            tr!("Minimap: the explored map of the current floor. Click a spot to walk there.")
        }
        "creature" => tr!("A creature or player: click to select it, right-click for its actions."),
        "world" => tr!(
            "Game window: your character and the world around it. Click a tile to walk there, right-click for object actions."
        ),
        _ => return None,
    })
}

/// The notice that announces the mode switching on or off.
#[must_use]
pub fn mode_text(active: bool) -> &'static str {
    if active {
        tr!("Lens help on: click a part of the game window to read about it")
    } else {
        tr!("Lens help off")
    }
}
