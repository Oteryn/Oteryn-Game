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
pub(super) const HUD: [Row; 33] = [
    check!(
        "hud.conditions_bar_all",
        "Show Conditions in Status Bar",
        conditions_bar_enabled
    ),
    check!(
        "hud.conditions_hud_all",
        "Show Conditions in HUD",
        conditions_hud_enabled
    ),
    check!("hud.status_bars", "Show Status Bars", status_bar_visible),
    check!(
        "hud.other_health",
        "Show Other Creatures' Health Bars",
        hud_other_health
    ),
    check!("hud.player_bars", "Show Player HUD Bars", hud_bars),
    check!("hud.player_enabled", "Show Player HUD", player_hud_enabled),
    check!(
        "hud.other_enabled",
        "Show Other Creature HUDs",
        other_hud_enabled
    ),
    check!(
        "hud.harmony_left",
        "Place Harmony Indicator on the Left",
        harmony_left
    ),
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
    Row {
        id: "hud.arc_width",
        label: "Arc Width",
        editor: Editor::Number {
            min: 20,
            max: 120,
            read: |p| p.arcs.width.unwrap_or(p.arcs.size),
            write: |p, v| p.arcs.width = Some(v),
        },
    },
    Row {
        id: "hud.arc_height",
        label: "Arc Height",
        editor: Editor::Number {
            min: 20,
            max: 120,
            read: |p| p.arcs.height.unwrap_or(p.arcs.size),
            write: |p, v| p.arcs.height = Some(v),
        },
    },
    Row {
        id: "hud.arc_size",
        label: "Arc Size",
        editor: Editor::Number {
            min: 20,
            max: 120,
            read: |p| p.arcs.size,
            write: |p, size| {
                p.arcs.size = size;
                p.arcs.width = None;
                p.arcs.height = None;
            },
        },
    },
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
pub(super) const GAME_WINDOW: [Row; 20] = [
    check!(
        "game_window.own_spells",
        "Show Own Spells",
        game_window.own_spells
    ),
    check!(
        "game_window.other_spells",
        "Show Other Players' Spells",
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
    check!(
        "game_window.messages",
        "Show Messages in Game Window",
        game_window.messages
    ),
    check!(
        "game_window.boosted_messages",
        "Show Boosted Creature Messages",
        game_window.boosted_messages
    ),
    check!(
        "game_window.hotkey_messages",
        "Show Hotkey Usage Messages",
        game_window.hotkey_messages
    ),
    check!(
        "game_window.private_messages",
        "Show Private Messages",
        game_window.private_messages
    ),
    check!(
        "game_window.store_messages",
        "Show Store Messages",
        game_window.store_messages
    ),
    check!(
        "game_window.attack_animation",
        "Show Attack Hit Animation",
        game_window.attack_animation
    ),
    check!(
        "game_window.combat_frames",
        "Show Combat Frames",
        game_window.combat_frames
    ),
    check!(
        "game_window.target_highlight",
        "Highlight Attack Target",
        game_window.target_highlight
    ),
    check!(
        "game_window.fit_viewport",
        "Fit Map to Game Window",
        graphics.fit_viewport
    ),
    number!(
        "game_window.upper_pane_height",
        "Upper Pane Height (0 = automatic)",
        0,
        600,
        u16,
        game_window.upper_pane_height
    ),
    Row {
        id: "game_window.loot_style",
        label: "Colourise Loot Value",
        editor: Editor::Choice {
            read: |p| usize::from(p.game_window.loot_style),
            write: |p, v| p.game_window.loot_style = v as u8,
            values: &["None", "Frames", "Corners"],
        },
    },
    check!(
        "game_window.offline_training",
        "Show Offline Training Messages",
        game_window.offline_training_messages
    ),
    check!("game_window.banner", "Show Banner", game_window.banner),
];
macro_rules! side_bars {
    ($id:literal,$label:literal,$($row:literal),+) => {
        Row {
            id: $id,
            label: $label,
            editor: Editor::Check {
                read: |p| [$($row),+].iter().all(|&i| p.bar_layout.rows[i].enabled),
                write: |p, v| {
                    for i in [$($row),+] {
                        p.bar_layout.rows[i].enabled = v;
                    }
                },
            },
        }
    };
}
pub(super) const ACTION_BARS: [Row; 20] = [
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
    side_bars!(
        "action_bars.bottom_all",
        "Show All Bottom Action Bars",
        0,
        3,
        4
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
    side_bars!("action_bars.left_all", "Show All Left Action Bars", 1, 5, 6),
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
    side_bars!(
        "action_bars.right_all",
        "Show All Right Action Bars",
        2,
        7,
        8
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
