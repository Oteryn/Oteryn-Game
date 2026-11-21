use super::*;
use bevy::picking::hover::Hovered;
use std::time::Duration;

#[derive(bevy::ecs::system::SystemParam)]
// Disjoint text/node filters are required for Bevy to prove the mutable queries do not alias.
#[allow(clippy::type_complexity)]
pub(super) struct Projection<'w, 's> {
    rows: Query<
        'w,
        's,
        (
            Entity,
            &'static BarRow,
            &'static mut Node,
            &'static ComputedNode,
            &'static ChildOf,
        ),
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
            Without<CooldownLine>,
            Without<ActionHint>,
            Without<SlotHotkey>,
        ),
    >,
    hotkeys: Query<
        'w,
        's,
        (&'static SlotHotkey, &'static mut Text),
        (
            Without<CooldownLine>,
            Without<ActionHint>,
            Without<SlotCaption>,
        ),
    >,
    locks: Query<'w, 's, (&'static ChildOf, &'static mut art::ArtIcon)>,
    hints: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<ActionHint>,
            Without<CooldownLine>,
            Without<SlotCaption>,
            Without<SlotHotkey>,
        ),
    >,
    scale: Option<Res<'w, ModuleScale>>,
    /// The registered binding of each action-bar slot, by slot index.
    bindings: Local<'s, Option<Vec<Option<crate::play::PreferenceBinding>>>>,
    hovered: Query<'w, 's, (&'static Hovered, &'static ActionControl)>,
    hover_since: Local<'s, Option<(u64, usize, Duration)>>,
    time: Res<'w, Time<Real>>,
}

pub(super) fn project(
    mut commands: Commands,
    anchors: Query<(Entity, &DockAnchor)>,
    owner: NonSend<HostOwner>,
    mut ui: Projection,
    mut page_sizes: ResMut<BarCapacities>,
) {
    let Some(play) = &owner.play else {
        *ui.hover_since = None;
        for mut hint in &mut ui.hints {
            if !hint.0.is_empty() {
                hint.0.clear();
            }
        }
        return;
    };
    let layout = play.link.bar_layout();
    let labels = play.link.spell_slot_labels();
    let tips = play.link.spell_slot_tooltips();
    let zoom = ui
        .scale
        .as_ref()
        .map_or(1., |scale| scale.get(Module::ActionBars));
    let slot_size = SLOT_SIZE * zoom;
    let scroll_size = SCROLL_SIZE * zoom;
    let mut capacities = [12; 9];
    for (entity, bar, mut node, computed, parent) in &mut ui.rows {
        let row = &layout.rows[bar.0];
        let side = row.dock != crate::play::BarDock::Bottom;
        if let Some((anchor, _)) = anchors.iter().find(|(_, anchor)| anchor.0 == row.dock)
            && parent.parent() != anchor
        {
            commands.entity(anchor).add_child(entity);
        }
        node.flex_direction = if side {
            FlexDirection::Column
        } else {
            FlexDirection::Row
        };
        node.width = if side { px(slot_size) } else { percent(100.) };
        node.height = if side { percent(100.) } else { px(slot_size) };
        node.min_height = if side { px(0.) } else { px(slot_size) };
        let available = if side {
            computed.size().y
        } else {
            computed.size().x
        } * computed.inverse_scale_factor;
        if available > 0. {
            let lock_size = if side { LOCK_SIZE.y } else { LOCK_SIZE.x } * zoom;
            capacities[bar.0] = ((available - lock_size - 2. * (scroll_size + SLOT_GAP))
                / (slot_size + SLOT_GAP))
                .floor()
                .max(0.) as usize;
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
    page_sizes.0 = capacities;
    for (control, mut generation, mut node) in &mut ui.controls {
        generation.0 = owner.generation();
        if let ActionControl::Scroll(bar, _) = control {
            let side = layout.rows[*bar].dock != crate::play::BarDock::Bottom;
            node.width = px(if side { slot_size } else { scroll_size });
            node.height = px(if side { scroll_size } else { slot_size });
            node.min_height = node.height;
            node.flex_shrink = 0.;
        }
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
    let bindings = ui.bindings.get_or_insert_with(|| {
        let mut slots = vec![None; ROW_SLOTS * 9];
        for binding in crate::play::preference_bindings().unwrap_or_default() {
            let slot = binding
                .id
                .strip_prefix("local-spell-action-bar-spell-")
                .and_then(|slot| slot.parse::<usize>().ok())
                .and_then(|slot| slot.checked_sub(1));
            if let Some(entry) = slot.and_then(|slot| slots.get_mut(slot)) {
                *entry = Some(binding);
            }
        }
        slots
    });
    let chat_on = play.link.preference_chat_open();
    for (hotkey, mut text) in &mut ui.hotkeys {
        let chord = bindings
            .get(hotkey.0)
            .and_then(Option::as_ref)
            .map_or("", |binding| {
                editor::slot_chord(&owner.settings.preferences, binding, hotkey.0, chat_on)
            });
        let label = hotkey_label(chord);
        if text.0 != label {
            text.0 = label;
        }
    }
    for (parent, mut icon) in &mut ui.locks {
        let Ok((ActionControl::Lock(bar), _, _)) = ui.controls.get(parent.parent()) else {
            continue;
        };
        let slot = if layout.rows[*bar].locked {
            ArtSlot::ActionBarLocked
        } else {
            ArtSlot::ActionBarUnlocked
        };
        if icon.0 != slot {
            icon.0 = slot;
        }
    }
    let prefs = &owner.settings.preferences;
    let target = if prefs.tooltips_enabled && prefs.action_bar.tooltips {
        ui.hovered
            .iter()
            .find_map(|(hovered, control)| match control {
                ActionControl::Slot(slot) if hovered.get() => Some((owner.generation(), *slot)),
                _ => None,
            })
    } else {
        None
    };
    let now = ui.time.elapsed();
    if ui
        .hover_since
        .map(|(generation, slot, _)| (generation, slot))
        != target
    {
        *ui.hover_since = target.map(|(generation, slot)| (generation, slot, now));
    }
    let hint = ui
        .hover_since
        .filter(|(_, _, since)| {
            now.saturating_sub(*since) >= Duration::from_millis(prefs.tooltip_delay_ms.into())
        })
        .map_or_else(String::new, |(_, slot, _)| {
            let tip = match prefs.bar_actions.0.get(&slot) {
                Some(crate::play::BarAction::Use { .. } | crate::play::BarAction::Equip { .. }) => {
                    "Object appearance unavailable."
                }
                _ => tips.get(slot).map_or("", String::as_str),
            };
            tip.replace('\n', "  ")
        });
    for mut text in &mut ui.hints {
        if text.0 != hint {
            text.0.clone_from(&hint);
        }
    }
}

/// The corner label of a slot chord: "F1", "C+F1" for Ctrl+F1, "S+A" for Shift+A.
pub(super) fn hotkey_label(chord: &str) -> String {
    let mut parts: Vec<&str> = chord.split('+').filter(|part| !part.is_empty()).collect();
    let Some(key) = parts.pop() else {
        return String::new();
    };
    let mut label = String::new();
    for modifier in parts {
        label.push_str(match modifier {
            "CTRL" | "CONTROL" => "C+",
            "SHIFT" => "S+",
            "ALT" => "A+",
            _ => "M+",
        });
    }
    label.push_str(key);
    label
}
