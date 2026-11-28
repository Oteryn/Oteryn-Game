use crate::play::ReferencePreferences;
use serde_json::{Value, json};

pub(super) fn read(p: &ReferencePreferences) -> Value {
    let audio_device_id = (!p.audio_device.is_empty())
        .then(|| oteryn_client_assets::text_sha256(p.audio_device.as_bytes()));
    let profiles = &p.hotkey_contexts;
    let selected_profile = profiles
        .presets
        .keys()
        .position(|key| key == &profiles.selected);

    // Allowlist the applied Options fields. In particular, the store also contains
    // screenshot paths, conversation names, profile names and arbitrary action text.
    let mut settings = json!({
        "action_bar": p.action_bar,
        "arcs": p.arcs,
        "audio": p.audio,
        "audio_device_id": audio_device_id,
        "bar_layout": p.bar_layout,
        "console": {
            "auto_private_tabs": p.console.auto_private_tabs,
            "dropped": p.console.dropped,
            "events": p.console.events,
            "feedback": p.console.feedback,
            "information": p.console.information,
            "join_leave": p.console.join_leave,
            "levels": p.console.levels,
            "line_limit": p.console.line_limit,
            "mode_labels": p.console.mode_labels,
            "seconds": p.console.seconds,
            "status": p.console.status,
            "timestamps": p.console.timestamps,
            "visible": p.console.visible,
            "font_px": p.console_font_px,
        },
        "controls": p.controls,
        "diagnostics": p.diagnostics,
        "effects": {
            "opacity": p.effect_opacity,
            "show_effects": p.show_effects,
            "show_harmony": p.show_harmony,
            "show_lights": p.show_lights,
            "show_serene": p.show_serene,
        },
        "game_window": p.game_window,
        "gameplay": p.gameplay,
        "graphics": p.graphics,
        "hotkeys": {
            "action_keys": p.action_keys,
            "chat_on": profiles.chat_on,
            "primary": p.keybindings,
            "profile_count": profiles.presets.len(),
            "selected_profile": selected_profile,
            "secondary": p.secondary_keybindings,
        },
        "hud": {
            "boss_area_opacity": p.boss_area_opacity,
            "compact_status": p.compact_status,
            "condition_hud_hidden": p.condition_hud_hidden,
            "condition_status_hidden": p.condition_status_hidden,
            "conditions_bar_enabled": p.conditions_bar_enabled,
            "conditions_hud_enabled": p.conditions_hud_enabled,
            "harmony_left": p.harmony_left,
            "health_circle": p.health_circle,
            "hud_bars": p.hud_bars,
            "hud_health": p.hud_health,
            "hud_mana": p.hud_mana,
            "hud_marks_other": p.hud_marks_other,
            "hud_marks_own": p.hud_marks_own,
            "hud_names": p.hud_names,
            "hud_npc_icons": p.hud_npc_icons,
            "ordered_condition_keys": p.ordered_condition_keys(),
            "hud_other_health": p.hud_other_health,
            "other_hud_enabled": p.other_hud_enabled,
            "player_hud_enabled": p.player_hud_enabled,
            "resource_hud": p.resource_hud,
            "skill_order": p.skill_order,
            "status_bar_bottom": p.status_bar_bottom,
            "status_bar_experience": p.status_bar_experience,
            "status_bar_skills": p.status_bar_skills,
            "status_bar_style": p.status_bar_style,
            "status_bar_visible": p.status_bar_visible,
            "status_panel_visible": p.status_panel_visible,
        },
        "interface": {
            "animated_cursor": p.animated_cursor,
            "chat_drag_locked": p.chat_drag_locked,
            "confirm_copy_link": p.confirm_copy_link,
            "cursor_crosshair": p.cursor_crosshair,
            "expiry_containers": p.expiry_containers,
            "expiry_inventory": p.expiry_inventory,
            "expiry_unused": p.expiry_unused,
            "highlight_mouse_target": p.highlight_mouse_target,
            "interface_scale": p.interface_scale,
            "large_cursor": p.large_cursor,
            "system_cursor": p.system_cursor,
            "tooltip_delay_ms": p.tooltip_delay_ms,
            "tooltips_enabled": p.tooltips_enabled,
            "top_menu_visible": p.top_menu_visible,
            "ui_scale": p.ui_scale,
        },
        "language": p.language().code(),
        "miscellaneous": p.miscellaneous,
        "optimise_connection": p.optimise_connection,
        "performance": p.render_performance(),
        "screenshots": {
            "automatic_disabled": p.screenshots.automatic_disabled,
            "backlog_frames": p.screenshots.backlog_frames,
            "backlog_seconds": p.screenshots.backlog_seconds,
            "game_only": p.screenshots.game_only,
            "on_achievement": p.screenshots.on_achievement,
            "on_bestiary_complete": p.screenshots.on_bestiary_complete,
            "on_bestiary_unlock": p.screenshots.on_bestiary_unlock,
            "on_boss_defeat": p.screenshots.on_boss_defeat,
            "on_death": p.screenshots.on_death,
            "on_level": p.screenshots.on_level,
            "on_low_health": p.screenshots.on_low_health,
            "on_player_attacking": p.screenshots.on_player_attacking,
            "on_record_damage": p.screenshots.on_record_damage,
            "on_record_healing": p.screenshots.on_record_healing,
            "on_skill": p.screenshots.on_skill,
            "on_valuable_loot": p.screenshots.on_valuable_loot,
            "reward_chest": p.screenshots.reward_chest,
        },
    });
    settings["confirm_store_purchase"] = p.confirm_store_purchase.into();
    settings
}
