use super::*;
use bevy::picking::hover::Hovered;

#[derive(bevy::ecs::system::SystemParam)]
// Disjoint text/node filters are required for Bevy to prove the mutable queries do not alias.
#[allow(clippy::type_complexity)]
pub(super) struct Projection<'w, 's> {
    rows: Query<
        'w,
        's,
        (&'static BarRow, &'static mut Node, &'static ComputedNode),
        Without<ActionControl>,
    >,
    controls: Query<
        'w,
        's,
        (
            &'static ActionControl,
            &'static mut Generation,
            &'static mut Node,
        ),
        Without<BarRow>,
    >,
    labels: Query<
        'w,
        's,
        (&'static SlotCaption, &'static mut Text),
        (
            Without<ToolCaption>,
            Without<CooldownLine>,
            Without<ActionHint>,
        ),
    >,
    tools: Query<
        'w,
        's,
        (&'static ToolCaption, &'static mut Text),
        (
            Without<SlotCaption>,
            Without<CooldownLine>,
            Without<ActionHint>,
        ),
    >,
    cooldown: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<CooldownLine>,
            Without<ActionHint>,
            Without<SlotCaption>,
            Without<ToolCaption>,
        ),
    >,
    hints: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<ActionHint>,
            Without<CooldownLine>,
            Without<SlotCaption>,
            Without<ToolCaption>,
        ),
    >,
    hovered: Query<'w, 's, (&'static Hovered, &'static ActionControl)>,
}

pub(super) fn project(owner: NonSend<HostOwner>, mut ui: Projection) {
    let Some(play) = &owner.play else {
        return;
    };
    let layout = play.link.bar_layout();
    let labels = play.link.spell_slot_labels();
    let tips = play.link.spell_slot_tooltips();
    let mut capacities = [12; 3];
    for (bar, mut node, computed) in &mut ui.rows {
        let row = &layout.rows[bar.0];
        let available = computed.size().x * computed.inverse_scale_factor;
        if available > 0. {
            capacities[bar.0] =
                ((available - 38.) / (SLOT_SIZE + SLOT_GAP)).floor().max(1.) as usize;
        }
        let display = if row.enabled && owner.settings.preferences.action_bar.visible {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    for (control, mut generation, mut node) in &mut ui.controls {
        generation.0 = owner.generation();
        if let ActionControl::Slot(slot) = control {
            let row = &layout.rows[slot / ROW_SLOTS];
            let visible = row.visible_slots(capacities[slot / ROW_SLOTS]);
            let display = if row.enabled && visible.contains(&(slot % ROW_SLOTS)) {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
    }
    for (slot, mut text) in &mut ui.labels {
        let value = labels.get(slot.0).map(String::as_str).unwrap_or("");
        let object = matches!(
            owner.settings.preferences.bar_actions.0.get(&slot.0),
            Some(crate::play::BarAction::Use { .. } | crate::play::BarAction::Equip { .. })
        );
        let value = if object { "Object" } else { value };
        let compact = value
            .lines()
            .map(|line| line.chars().take(6).collect::<String>())
            .take(3)
            .collect::<Vec<_>>()
            .join("\n");
        if text.0 != compact {
            text.0 = compact;
        }
    }
    for (tool, mut text) in &mut ui.tools {
        let caption = match tool.0 {
            ActionControl::Toggle(bar) => format!(
                "{} {}",
                bar + 1,
                if layout.rows[bar].enabled {
                    "on"
                } else {
                    "off"
                }
            ),
            ActionControl::Lock(bar) => if layout.rows[bar].locked {
                "Locked"
            } else {
                "Edit"
            }
            .into(),
            _ => continue,
        };
        if text.0 != caption {
            text.0 = caption;
        }
    }
    let cooldown = play
        .link
        .cooldown_bar_icons()
        .into_iter()
        .filter(|icon| icon.remaining_ms > 0)
        .map(|icon| format!("{} {:.1}s", icon.label, icon.remaining_ms as f64 / 1000.))
        .collect::<Vec<_>>()
        .join("  ");
    for mut text in &mut ui.cooldown {
        if text.0 != cooldown {
            text.0.clone_from(&cooldown);
        }
    }
    let hint = ui
        .hovered
        .iter()
        .find_map(|(interaction, control)| match control {
            ActionControl::Slot(slot) if interaction.get() => {
                Some(match owner.settings.preferences.bar_actions.0.get(slot) {
                    Some(
                        crate::play::BarAction::Use { .. } | crate::play::BarAction::Equip { .. },
                    ) => "Object appearance unavailable.",
                    _ => tips.get(*slot).map(String::as_str).unwrap_or(""),
                })
            }
            _ => None,
        })
        .map_or(String::new(), |tip| tip.replace('\n', "  "));
    for mut text in &mut ui.hints {
        if text.0 != hint {
            text.0.clone_from(&hint);
        }
    }
}
