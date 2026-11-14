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
