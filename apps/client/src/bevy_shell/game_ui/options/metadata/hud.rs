use super::*;

pub(super) const HUD: [Row; 47] = [
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
    check!(
        "hud.status_bar_axe",
        "Status Bar: Axe",
        status_bar_skills[4]
    ),
    check!(
        "hud.status_bar_bottom",
        "Place Status Bar Below Map",
        status_bar_bottom
    ),
    check!(
        "hud.status_bar_club",
        "Status Bar: Club",
        status_bar_skills[2]
    ),
    check!(
        "hud.status_bar_distance",
        "Status Bar: Distance",
        status_bar_skills[5]
    ),
    check!(
        "hud.status_bar_experience",
        "Status Bar: Level and Experience",
        status_bar_experience
    ),
    check!(
        "hud.status_bar_fishing",
        "Status Bar: Fishing",
        status_bar_skills[7]
    ),
    check!(
        "hud.status_bar_fist",
        "Status Bar: Fist",
        status_bar_skills[1]
    ),
    check!(
        "hud.status_bar_magic",
        "Status Bar: Magic",
        status_bar_skills[0]
    ),
    check!(
        "hud.status_bar_shielding",
        "Status Bar: Shielding",
        status_bar_skills[6]
    ),
    Row {
        id: "hud.status_bar_style",
        label: "Status Bar Style",
        editor: Editor::Choice {
            values: &["Default", "Compact", "Large", "Parallel"],
            read: |p| usize::from(p.status_bar_style.min(3)),
            write: |p, value| p.status_bar_style = value as u8,
        },
    },
    check!(
        "hud.status_bar_sword",
        "Status Bar: Sword",
        status_bar_skills[3]
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
    check!("hud.experience_arc", "Show Experience Arc", arcs.experience),
    check!("hud.skill_arc", "Show Skill Arc", arcs.skill),
    Row {
        id: "hud.tracked_skill",
        label: "Tracked Skill",
        editor: Editor::Choice {
            values: &oteryn_client_ui::skill_progress::SKILL_LABELS,
            read: |p| usize::from(p.arcs.tracked_skill),
            write: |p, i| p.arcs.tracked_skill = i as u8,
        },
    },
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
