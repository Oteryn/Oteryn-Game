use super::*;
pub(super) const MAIN: [Row; 13] = [
    Row {
        id: "interface.animated_cursor",
        label: "Animate Game Cursor",
        editor: Editor::Check {
            read: |p| p.animated_cursor,
            write: |p, v| {
                p.animated_cursor = v;
                if v {
                    p.system_cursor = false;
                }
            },
        },
    },
    check!(
        "interface.confirm_copy_link",
        "Confirm Before Copying Links",
        confirm_copy_link
    ),
    check!(
        "interface.expiry_containers",
        "Show Expiry in Containers",
        expiry_containers
    ),
    check!(
        "interface.expiry_inventory",
        "Show Expiry on Equipped Items",
        expiry_inventory
    ),
    check!(
        "interface.expiry_unused",
        "Show Expiry on Unused Items",
        expiry_unused
    ),
    check!(
        "interface.highlight_mouse_target",
        "Highlight Object Under Pointer",
        highlight_mouse_target
    ),
    Row {
        id: "interface.large_cursor",
        label: "Large Game Cursor",
        editor: Editor::Check {
            read: |p| p.large_cursor,
            write: |p, v| {
                p.large_cursor = v;
                if v {
                    p.system_cursor = false;
                }
            },
        },
    },
    Row {
        id: "interface.system_cursor",
        label: "Use Operating System Cursor",
        editor: Editor::Check {
            read: |p| p.system_cursor,
            write: |p, v| {
                p.system_cursor = v;
                if v {
                    p.animated_cursor = false;
                    p.large_cursor = false;
                }
            },
        },
    },
    check!("interface.top_menu", "Show Game Menu", top_menu_visible),
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
pub(super) const CONSOLE: [Row; 19] = [
    check!(
        "console.auto_private_tabs",
        "Open Incoming Private Messages in New Tabs",
        console.auto_private_tabs
    ),
    check!("console.events", "Show Event Messages", console.events),
    check!(
        "console.information",
        "Show Information Messages",
        console.information
    ),
    check!(
        "console.join_leave",
        "Show Join and Leave Messages",
        console.join_leave
    ),
    check!("console.levels", "Show Speaker Levels", console.levels),
    check!("console.seconds", "Show Timestamp Seconds", console.seconds),
    check!("console.status", "Show Own Status Messages", console.status),
    check!("console.timestamps", "Show Timestamps", console.timestamps),
    check!(
        "console.drag_locked",
        "Lock Floating Chat Position",
        chat_drag_locked
    ),
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
macro_rules! lock_side {
    ($id:literal, $label:literal, $dock:ident) => {
        Row {
            id: $id,
            label: $label,
            editor: Editor::Check {
                read: |p| {
                    p.bar_layout
                        .rows
                        .iter()
                        .filter(|row| row.dock == crate::play::BarDock::$dock)
                        .all(|row| row.locked)
                },
                write: |p, locked| {
                    for row in &mut p.bar_layout.rows {
                        if row.dock == crate::play::BarDock::$dock {
                            row.locked = locked;
                        }
                    }
                },
            },
        }
    };
}
pub(super) const ACTION_BARS: [Row; 27] = [
    lock_side!("action_bars.bottom_lock", "Lock all bottom bars", Bottom),
    check!(
        "action_bars.cooldown_numbers",
        "Show numeric cooldowns",
        action_bar.cooldown_numbers
    ),
    check!(
        "action_bars.graphical_cooldown",
        "Show graphical cooldowns",
        action_bar.graphical_cooldown
    ),
    lock_side!("action_bars.left_lock", "Lock all left bars", Left),
    lock_side!("action_bars.right_lock", "Lock all right bars", Right),
    check!(
        "action_bars.show_hotkey",
        "Show assigned hotkeys",
        action_bar.show_hotkey
    ),
    check!(
        "action_bars.show_spell_parameters",
        "Show assigned spell parameters",
        action_bar.show_spell_parameters
    ),
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
