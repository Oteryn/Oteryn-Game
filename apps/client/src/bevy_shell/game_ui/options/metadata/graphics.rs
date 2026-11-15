use super::*;
pub(super) const MAIN: [Row; 7] = [
    check!(
        "graphics.fullscreen",
        "Show in Fullscreen",
        graphics.fullscreen
    ),
    check!(
        "graphics.vsync",
        "Vertical Synchronisation (V-Sync)",
        graphics.vsync
    ),
    check!(
        "graphics.integer_scaling",
        "Scale Using Only Integral Multiples",
        graphics.integer_scaling
    ),
    number!(
        "graphics.world_zoom",
        "Game Window Scale (%)",
        80,
        180,
        u16,
        ui_scale
    ),
    number!(
        "graphics.ambient",
        "Minimum Ambient Brightness (%)",
        0,
        100,
        u8,
        graphics.ambient
    ),
    number!(
        "graphics.indoor",
        "Indoor Dimming (%)",
        0,
        100,
        u8,
        graphics.indoor
    ),
    number!(
        "graphics.separator",
        "Lower-Floor Dimming (%)",
        0,
        100,
        u8,
        graphics.separator
    ),
];
pub(super) const EFFECTS: [Row; 8] = [
    check!("effects.enabled", "Show Effects", show_effects),
    check!("effects.lights", "Show Lights", show_lights),
    check!("effects.harmony", "Show Harmony Effects", show_harmony),
    check!("effects.serene", "Show Serene Effects", show_serene),
    number!(
        "effects.own_opacity",
        "Own Effects Opacity (%)",
        0,
        100,
        u8,
        effect_opacity[0]
    ),
    number!(
        "effects.other_opacity",
        "Other Players' Effects Opacity (%)",
        0,
        100,
        u8,
        effect_opacity[1]
    ),
    number!(
        "effects.creature_opacity",
        "Creature Effects Opacity (%)",
        0,
        100,
        u8,
        effect_opacity[2]
    ),
    number!(
        "effects.clouds",
        "Cloud Dimming (%)",
        0,
        100,
        u8,
        graphics.cloud
    ),
];

impl Row {
    /// Graphics-page help also follows the shared controls into Basic Options.
    pub fn help(self) -> Option<&'static str> {
        Some(match self.id {
            "graphics.fullscreen" => {
                "Use a borderless fullscreen window on the current monitor. Turn this off to return to a window. Choose Apply or OK to change the window mode."
            }
            "graphics.vsync" => {
                "Synchronise presentation with the display to reduce tearing. The frame rate depends on your display and graphics backend. Turning this off allows unsynchronised presentation. Choose Apply or OK to change it."
            }
            "graphics.integer_scaling" => {
                "Round the game world's tile size to whole sprite-size multiples, keeping pixel edges consistent. The result is limited by the available game window. This does not change interface text size. Choose Apply or OK to change it."
            }
            "graphics.world_zoom" => {
                "Change the size of tiles and creatures in the game window. The scale is capped to fit the visible map; integral scaling can round it further. This does not change interface text size. Choose Apply or OK to change it."
            }
            "graphics.ambient" => {
                "Raise the minimum ambient brightness used for world lighting. Higher values brighten dark surroundings before indoor dimming is applied. Show Lights on the Effects page must be enabled. Choose Apply or OK to change it."
            }
            "graphics.indoor" => {
                "Reduce ambient brightness underground or beneath a roof. Zero adds no indoor dimming; higher values darken the surroundings. Show Lights on the Effects page must be enabled. Choose Apply or OK to change it."
            }
            "graphics.separator" => {
                "Darken visible floors below your character to make floor boundaries easier to distinguish. Zero adds no lower-floor dimming. Show Lights on the Effects page must be enabled. Choose Apply or OK to change it."
            }
            _ => return None,
        })
    }
}
