//! Exact public control names only; widget values and arbitrary Name prefixes stay private.
use super::Bounds;
use bevy::{ecs::system::SystemParam, input_focus::InputFocus, prelude::*};
use std::collections::BTreeMap;

// This fixed ancestry tuple reads visibility and disabled state, never widget contents.
#[allow(clippy::type_complexity)]
type Hierarchy<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static Name>,
        Option<&'static Node>,
        Option<&'static Visibility>,
        Option<&'static InheritedVisibility>,
        Option<&'static ChildOf>,
        Has<bevy::ui::InteractionDisabled>,
    ),
>;
#[derive(SystemParam)]
pub(in crate::bevy_shell) struct NamedControls<'w, 's> {
    nodes: Query<
        'w,
        's,
        (
            Entity,
            &'static Name,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
    hierarchy: Hierarchy<'w, 's>,
    focus: Option<Res<'w, InputFocus>>,
    full_options: Option<Res<'w, crate::bevy_shell::game_ui::options::Installed>>,
}
impl NamedControls<'_, '_> {
    pub(super) fn focus(&self) -> Option<Entity> {
        self.focus.as_ref().and_then(|focus| focus.get())
    }
    pub(super) fn full_options(&self) -> bool {
        self.full_options.is_some()
    }
    pub(super) fn bounds(&self, scale: f32) -> Vec<Bounds> {
        let mut controls = BTreeMap::new();
        for (entity, name, node, transform) in &self.nodes {
            let Some(id) = IDS.iter().copied().find(|id| *id == name.as_str()) else {
                continue;
            };
            if node.size().min_element() <= 0.
                || !node.size().is_finite()
                || !transform.translation.is_finite()
            {
                continue;
            }
            let scope = if id.starts_with("exit.") {
                "exit.dialog"
            } else {
                "options.dialog"
            };
            let Some(enabled) = self.shown(entity, scope) else {
                continue;
            };
            let min = transform.translation - node.size() / 2.;
            let physical = [min.x, min.y, node.size().x, node.size().y];
            controls.entry(id).or_insert_with(|| Bounds {
                id: id.into(),
                bounds_physical: physical,
                bounds_logical: physical.map(|value| value / scale),
                enabled,
                text_metrics: None,
            });
        }
        controls.into_values().collect()
    }
    fn shown(&self, entity: Entity, scope: &str) -> Option<bool> {
        let mut current = Some(entity);
        let mut scoped = false;
        let mut enabled = true;
        while let Some(candidate) = current {
            let (name, node, visibility, inherited, parent, disabled) =
                self.hierarchy.get(candidate).ok()?;
            if candidate == entity && (node.is_none() || inherited.is_none()) {
                return None;
            }
            if node.is_some_and(|node| node.display == Display::None)
                || visibility.is_some_and(|visibility| *visibility == Visibility::Hidden)
                || inherited.is_some_and(|visibility| !visibility.get())
            {
                return None;
            }
            scoped |= name.is_some_and(|name| name.as_str() == scope);
            enabled &= !disabled;
            current = parent.map(ChildOf::parent);
        }
        scoped.then_some(enabled)
    }
}

// Deliberate whitelist: static Options metadata, its bounded choice rows, and exit controls.
const IDS: &[&str] = &[
    "action_bars.amounts",
    "action_bars.auto_insert",
    "action_bars.cooldown_bar",
    "action_bars.cooldowns",
    "action_bars.labels",
    "action_bars.three_rows",
    "action_bars.tooltips",
    "action_bars.visible",
    "battle_sound.creature_attacks",
    "battle_sound.creature_healing",
    "battle_sound.creature_support",
    "battle_sound.creature_volume",
    "battle_sound.other_attacks",
    "battle_sound.other_healing",
    "battle_sound.other_support",
    "battle_sound.other_volume",
    "battle_sound.own_attacks",
    "battle_sound.own_healing",
    "battle_sound.own_support",
    "battle_sound.own_volume",
    "console.dropped",
    "console.feedback",
    "console.guild",
    "console.lines",
    "console.local",
    "console.mode_labels",
    "console.party",
    "console.private",
    "console.public",
    "controls.keyboard_delay",
    "controls.mouse",
    "controls.mouse.0",
    "controls.mouse.1",
    "controls.movement",
    "controls.movement.0",
    "controls.movement.1",
    "controls.rotation",
    "controls.rotation.0",
    "controls.rotation.1",
    "controls.rotation.2",
    "effects.clouds",
    "effects.creature_opacity",
    "effects.enabled",
    "effects.harmony",
    "effects.lights",
    "effects.other_opacity",
    "effects.own_opacity",
    "effects.serene",
    "exit.dialog",
    "exit.no",
    "exit.panel",
    "exit.yes",
    "game_window.banner",
    "game_window.loot",
    "game_window.loot_highlight",
    "game_window.other_spells",
    "game_window.own_spells",
    "game_window.pvp",
    "game_window.target",
    "game_window.values",
    "gameplay.chase",
    "gameplay.inspect",
    "gameplay.secondary_loot",
    "graphics.ambient",
    "graphics.fullscreen",
    "graphics.indoor",
    "graphics.integer_scaling",
    "graphics.separator",
    "graphics.vsync",
    "graphics.world_zoom",
    "hud.arc_distance",
    "hud.arc_opacity",
    "hud.arc_size",
    "hud.boss_opacity",
    "hud.creature_names",
    "hud.health",
    "hud.health_arc",
    "hud.mana",
    "hud.mana_arc",
    "hud.npc_icons",
    "hud.other_marks",
    "hud.own_marks",
    "hud.own_name",
    "hud.player_names",
    "hud.status_compact",
    "hud.status_visible",
    "interface.crosshair",
    "interface.scale",
    "interface.tooltip_delay",
    "interface.tooltips",
    "misc.connection",
    "misc.exit",
    "misc.logout",
    "options.advanced",
    "options.apply",
    "options.cancel",
    "options.dialog",
    "options.navigation.frame",
    "options.ok",
    "options.page.ActionBars",
    "options.page.Basic",
    "options.page.BattleSounds",
    "options.page.Console",
    "options.page.Controls",
    "options.page.Effects",
    "options.page.GameWindow",
    "options.page.Gameplay",
    "options.page.Graphics",
    "options.page.Hud",
    "options.page.Interface",
    "options.page.Misc",
    "options.page.Sound",
    "options.page.UiSounds",
    "options.reset",
    "sound.ambient",
    "sound.enabled",
    "sound.event",
    "sound.item",
    "sound.master",
    "sound.music",
    "sound.ui",
    "ui_sound.chat",
    "ui_sound.interaction",
    "ui_sound.party",
    "ui_sound.vip",
];
