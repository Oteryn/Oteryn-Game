//! Imbuing and tracker projections over the retained owner.
#[cfg(test)]
mod tests;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0068_imbue::view::{ImbueControl as Kind, ImbueSnapshot};
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::imbue::{ImbuePanel, family_name, slot_label, tracker_text};
use oteryn_client_ui_bevy as source;

#[derive(Component, Clone)]
struct Control {
    generation: u64,
    snapshot: ImbueSnapshot,
    action: Action,
}
#[derive(Clone)]
enum Action {
    Owner(Kind),
    Confirm(bool),
    Cancel,
}
#[derive(Resource, Default)]
struct State {
    shown: Option<(u64, ImbueSnapshot)>,
    root: Option<Entity>,
    confirmation: Option<(u64, ImbueSnapshot, bool)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
fn offer_text(panel: &ImbuePanel, index: usize) -> String {
    let Some(offer) = panel.window.as_ref().and_then(|w| w.offers.get(index)) else {
        return String::new();
    };
    let quote = panel
        .details
        .as_ref()
        .filter(|d| Some(d.item) == panel.item)
        .and_then(|d| {
            d.quotes
                .iter()
                .find(|q| q.family == offer.family && q.tier == offer.tier && q.gold == offer.gold)
        });
    let chance = panel
        .success_selected
        .then(|| panel.success.get(index))
        .flatten()
        .copied()
        .or_else(|| quote.map(|q| q.success_percent));
    let mut text = format!(
        "{} — tier {}, {} gold; success: {}{}",
        family_name(offer.family),
        offer.tier,
        offer.gold,
        chance.map_or_else(|| "unavailable".into(), |p| format!("{p}%")),
        if offer.requires_premium {
            "; premium required"
        } else {
            ""
        }
    );
    if let Some(q) = quote {
        for material in &q.materials {
            text.push_str(&format!(
                "\n{} × {}",
                material.quantity, material.definition_key
            ));
        }
        if !q.recipe_available {
            text.push_str("\nRecipe unavailable");
        }
    } else {
        text.push_str("\nMaterial requirements unavailable");
    }
    text
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, text: &str) {
    let child = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(child);
}
fn button(commands: &mut Commands, skins: &Skins, parent: Entity, control: Control, text: &str) {
    let child = commands
        .spawn((source::button(skins.buttons.clone(), 300.), control))
        .observe(activate)
        .id();
    label(commands, skins, child, text);
    commands.entity(parent).add_child(child);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
) {
    let current = owner
        .play
        .as_ref()
        .map(|p| (owner.generation(), p.link.imbue_snapshot()));
    if state.shown == current {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    if state
        .confirmation
        .as_ref()
        .is_some_and(|(g, s, _)| current.as_ref().is_none_or(|(cg, cs)| g != cg || s != cs))
    {
        state.confirmation = None;
    }
    state.shown = current.clone();
    let Some((generation, snapshot)) = current else {
        return;
    };
    let panel = &snapshot.panel;
    if !panel.open && !panel.tracker_open {
        return;
    }
    let root = commands
        .spawn((
            GameModal,
            GlobalZIndex(160),
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            bevy::ui_widgets::ScrollArea,
            Node {
                position_type: PositionType::Absolute,
                left: percent(20.),
                top: percent(8.),
                width: percent(60.),
                max_height: percent(84.),
                padding: UiRect::all(px(14.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    state.root = Some(root);
    let control = |action| Control {
        generation,
        snapshot: snapshot.clone(),
        action,
    };
    if let Some((_, _, apply)) = &state.confirmation {
        let Some(cost) = panel.window.as_ref().and_then(|w| {
            if *apply {
                w.offers.get(panel.offer).map(|o| o.gold)
            } else {
                Some(w.clear_gold)
            }
        }) else {
            button(
                &mut commands,
                &skins,
                root,
                control(Action::Cancel),
                "Cancel",
            );
            return;
        };
        label(
            &mut commands,
            &skins,
            root,
            &format!(
                "Confirm {}: item {:?}, slot {}, {} gold",
                if *apply { "apply" } else { "clear" },
                panel.item,
                panel.slot + 1,
                cost
            ),
        );
        if *apply {
            label(&mut commands, &skins, root, &offer_text(panel, panel.offer));
        }
        button(
            &mut commands,
            &skins,
            root,
            control(Action::Owner(if *apply {
                Kind::Apply
            } else {
                Kind::Clear
            })),
            "Confirm cost",
        );
        button(
            &mut commands,
            &skins,
            root,
            control(Action::Cancel),
            "Cancel",
        );
        return;
    }
    if panel.open {
        label(&mut commands, &skins, root, "Imbuement Shrine");
        for item in snapshot
            .items
            .inventory
            .main_backpack
            .iter()
            .chain(&snapshot.items.inventory.entries)
            .chain(snapshot.items.inventory.equipment.iter().map(|e| &e.item))
            .chain(&snapshot.items.views)
        {
            button(
                &mut commands,
                &skins,
                root,
                control(Action::Owner(Kind::Item(item.handle))),
                &format!(
                    "Item {} (definition {})",
                    item.handle, item.item_definition_ref
                ),
            );
        }
        if let Some(w) = &panel.window {
            for slot in 0..w.slot_count {
                button(
                    &mut commands,
                    &skins,
                    root,
                    control(Action::Owner(Kind::Slot(slot))),
                    &slot_label(panel, slot),
                );
            }
            for index in 0..w.offers.len() {
                button(
                    &mut commands,
                    &skins,
                    root,
                    control(Action::Owner(Kind::Offer(index))),
                    &offer_text(panel, index),
                );
            }
        }
        label(
            &mut commands,
            &skins,
            root,
            &format!(
                "Selected item {:?}, slot {}, offer {}",
                panel.item,
                panel.slot + 1,
                panel.offer + 1
            ),
        );
        if let Some(feedback) = panel.feedback {
            label(&mut commands, &skins, root, feedback);
        }
        for (kind, text) in [
            (Kind::Refresh, "Refresh item"),
            (Kind::Details, "Load material requirements"),
            (Kind::Close, "Close imbuing"),
        ] {
            button(
                &mut commands,
                &skins,
                root,
                control(Action::Owner(kind)),
                text,
            );
        }
        if panel.can_apply() {
            button(
                &mut commands,
                &skins,
                root,
                control(Action::Confirm(true)),
                "Apply…",
            );
        }
        if panel.can_clear() {
            button(
                &mut commands,
                &skins,
                root,
                control(Action::Confirm(false)),
                "Clear…",
            );
        }
    }
    if panel.tracker_open {
        label(&mut commands, &skins, root, &tracker_text(panel));
        button(
            &mut commands,
            &skins,
            root,
            control(Action::Owner(Kind::TrackerRefresh)),
            "Refresh equipment",
        );
        button(
            &mut commands,
            &skins,
            root,
            control(Action::Owner(Kind::CloseTracker)),
            "Close tracker",
        );
    }
}
fn perform(owner: &HostOwner, state: &mut State, control: &Control) {
    if control.generation != owner.generation() || owner.modal_active() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link)
        || play.link.imbue_snapshot() != control.snapshot
    {
        return;
    }
    match &control.action {
        Action::Confirm(apply) => {
            if control.snapshot.panel.pending || !control.snapshot.panel.open {
                return;
            }
            state.confirmation = Some((control.generation, control.snapshot.clone(), *apply));
            state.shown = None;
        }
        Action::Cancel => {
            state.confirmation = None;
            state.shown = None;
        }
        Action::Owner(kind) => {
            if matches!(kind, Kind::Apply | Kind::Clear)
                && !state.confirmation.as_ref().is_some_and(|(g, s, a)| {
                    *g == control.generation
                        && s == &control.snapshot
                        && *a == (kind == &Kind::Apply)
                })
            {
                return;
            }
            if play
                .link
                .imbue_dispatch_bound(&control.snapshot, kind.clone())
            {
                state.confirmation = None;
            }
        }
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        perform(&owner, &mut state, control);
    }
}

fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Some(root) = state.root else {
        events.clear();
        return;
    };
    if !guard.allows(root) {
        events.clear();
        return;
    }
    if events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    }) && let Some((generation, snapshot)) = state.shown.clone()
    {
        let action = if state.confirmation.is_some() {
            Action::Cancel
        } else {
            Action::Owner(if snapshot.panel.open {
                Kind::Close
            } else {
                Kind::CloseTracker
            })
        };
        perform(
            &owner,
            &mut state,
            &Control {
                generation,
                snapshot,
                action,
            },
        );
    }
}
