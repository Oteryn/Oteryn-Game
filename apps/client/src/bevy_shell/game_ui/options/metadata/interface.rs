use super::*;
pub(super) const MAIN: [Row; 4] = [
    check!("interface.tooltips", "Show Tooltips", tooltips_enabled),
    number!(
        "interface.tooltip_delay",
        "Tooltip Delay (ms)",
        0,
        3000,
        u16,
        tooltip_delay_ms
    ),
    number!(
        "interface.scale",
        "Interface Scale (%)",
        80,
        200,
        u16,
        interface_scale
    ),
    check!(
        "interface.crosshair",
        "Show Crosshair Cursor",
        cursor_crosshair
    ),
];
pub(super) const HUD: [Row; 23] = [
    check!("hud.health", "Show Health", hud_health),
    check!("hud.mana", "Show Mana", hud_mana),
    check!("hud.own_name", "Show Own Name", hud_names[0]),
    check!(
        "hud.player_names",
        "Show Other Players' Names",
        hud_names[1]
    ),
    check!("hud.creature_names", "Show Creature Names", hud_names[2]),
    check!("hud.own_marks", "Show Own Marks", hud_marks_own),
    check!(
        "hud.other_marks",
        "Show Other Creatures' Marks",
        hud_marks_other
    ),
    check!("hud.npc_icons", "Show NPC Icons", hud_npc_icons),
    check!(
        "hud.status_visible",
        "Show Status Panel",
        status_panel_visible
    ),
    check!("hud.status_compact", "Compact Status Panel", compact_status),
    check!("hud.health_arc", "Show Health Arc", arcs.health),
    check!(
        "hud.health_circle",
        "Show Health/Mana Circles",
        health_circle
    ),
    check!("hud.mana_arc", "Show Mana Arc", arcs.mana),
    number!(
        "hud.arc_distance",
        "Arc Distance",
        0,
        120,
        u16,
        arcs.distance
    ),
    number!("hud.arc_size", "Arc Size", 20, 120, u16, arcs.size),
    number!(
        "hud.arc_opacity",
        "Arc Opacity (%)",
        0,
        100,
        u8,
        arcs.opacity
    ),
    number!(
        "hud.boss_opacity",
        "Boss Area Opacity (%)",
        0,
        100,
        u8,
        boss_area_opacity
    ),
    check!(
        "hud.resource_values",
        "Show Hit Point and Mana Values",
        resource_hud.values
    ),
    check!("hud.resource_soul", "Show Soul Points", resource_hud.soul),
    check!("hud.resource_stamina", "Show Stamina", resource_hud.stamina),
    check!(
        "hud.resource_conditions",
        "Show Condition Icons",
        resource_hud.conditions
    ),
    check!("hud.resource_rest", "Show Rest Icons", resource_hud.rest),
    check!(
        "hud.resource_combo",
        "Show Harmony Combo",
        resource_hud.combo
    ),
];
pub(super) const CONSOLE: [Row; 10] = [
    check!("console.local", "Show Local Speech", console.visible[0]),
    check!(
        "console.private",
        "Show Private Messages and Channels",
        console.visible[1]
    ),
    check!("console.guild", "Show Guild Messages", console.visible[2]),
    check!(
        "console.public",
        "Show Public Channel Messages",
        console.visible[3]
    ),
    check!("console.party", "Show Party Messages", console.visible[4]),
    check!(
        "console.mode_labels",
        "Show Message Mode Labels",
        console.mode_labels
    ),
    check!(
        "console.dropped",
        "Show Dropped-Line Notices",
        console.dropped
    ),
    check!("console.feedback", "Show Server Feedback", console.feedback),
    number!(
        "console.font_size",
        "Console Font Size (px)",
        10,
        32,
        u16,
        console_font_px
    ),
    number!(
        "console.lines",
        "Recent Received Lines",
        1,
        1000,
        u16,
        console.line_limit
    ),
];
pub(super) const GAME_WINDOW: [Row; 8] = [
    check!(
        "game_window.own_spells",
        "Show Own Spells",
        game_window.own_spells
    ),
    check!(
        "game_window.other_spells",
        "Show Other Creatures' Spells",
        game_window.other_spells
    ),
    check!(
        "game_window.values",
        "Show Value Text",
        game_window.value_text
    ),
    check!(
        "game_window.loot",
        "Show Loot Messages",
        game_window.loot_messages
    ),
    check!(
        "game_window.target",
        "Show Target Frame",
        game_window.target_frame
    ),
    check!("game_window.pvp", "Show PvP Frames", game_window.pvp_frame),
    check!(
        "game_window.loot_highlight",
        "Highlight Loot",
        game_window.loot_highlight
    ),
    check!("game_window.banner", "Show Banner", game_window.banner),
];
pub(super) const ACTION_BARS: [Row; 17] = [
    check!(
        "action_bars.visible",
        "Show Action Bars",
        action_bar.visible
    ),
    check!(
        "action_bars.three_rows",
        "Show Three Action Bar Rows",
        action_bar.three_rows
    ),
    check!(
        "action_bars.cooldown_bar",
        "Show Cooldown Bar",
        action_bar.cooldown_bar_enabled
    ),
    check!(
        "action_bars.labels",
        "Show Action Labels",
        action_bar.labels
    ),
    check!(
        "action_bars.amounts",
        "Show Item Amounts",
        action_bar.amounts
    ),
    check!(
        "action_bars.cooldowns",
        "Show Cooldowns",
        action_bar.cooldowns
    ),
    check!(
        "action_bars.tooltips",
        "Show Action Tooltips",
        action_bar.tooltips
    ),
    check!(
        "action_bars.auto_insert",
        "Automatically Insert New Actions",
        action_bar.auto_insert
    ),
    check!(
        "action_bars.bottom_1",
        "Show Bottom Action Bar 1",
        bar_layout.rows[0].enabled
    ),
    check!(
        "action_bars.bottom_2",
        "Show Bottom Action Bar 2",
        bar_layout.rows[3].enabled
    ),
    check!(
        "action_bars.bottom_3",
        "Show Bottom Action Bar 3",
        bar_layout.rows[4].enabled
    ),
    check!(
        "action_bars.left_1",
        "Show Left Action Bar 1",
        bar_layout.rows[1].enabled
    ),
    check!(
        "action_bars.left_2",
        "Show Left Action Bar 2",
        bar_layout.rows[5].enabled
    ),
    check!(
        "action_bars.left_3",
        "Show Left Action Bar 3",
        bar_layout.rows[6].enabled
    ),
    check!(
        "action_bars.right_1",
        "Show Right Action Bar 1",
        bar_layout.rows[2].enabled
    ),
    check!(
        "action_bars.right_2",
        "Show Right Action Bar 2",
        bar_layout.rows[7].enabled
    ),
    check!(
        "action_bars.right_3",
        "Show Right Action Bar 3",
        bar_layout.rows[8].enabled
    ),
];
