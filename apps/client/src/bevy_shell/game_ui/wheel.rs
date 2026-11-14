//! Wheel and Gems are projections of the retained allocation owner.
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0148_wheel_allocation::view::WheelControl as Kind;
use bevy::{
    input_focus::{
        InputFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::{Activate, ScrollArea, TextInput},
};
use oteryn_client_ui::wheel_allocation::{Intent, WheelPanel, diagram};
use oteryn_client_ui_bevy as source;
#[cfg(test)]
mod tests;
#[derive(Resource, Default)]
struct State {
    generation: u64,
    serial: u64,
    snapshot: Option<WheelPanel>,
    root: Option<Entity>,
    name: String,
}
#[derive(Component)]
struct PresetName;
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    serial: u64,
    kind: Kind,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(Update, project);
}
fn summary(p: &WheelPanel) -> String {
    let revision = p
        .confirmed_wheel
        .as_ref()
        .map_or_else(|| "unavailable".into(), |w| w.revision.to_string());
    format!(
        "Revision: {revision}\n{}\n{}",
        oteryn_client_ui::wheel_allocation::panel_text(p),
        diagram::view::text(p)
    )
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    fields: Query<&EditableText, With<PresetName>>,
    mut focus: ResMut<InputFocus>,
    modals: Query<Entity, With<GameModal>>,
) {
    let blocked = modals.iter().any(|e| Some(e) != state.root);
    let panel = owner
        .play
        .as_ref()
        .and_then(|p| p.link.wheel_window_panel());
    if state.generation == owner.generation() && state.snapshot == panel {
        return;
    }
    if let Ok(field) = fields.single() {
        state.name = field.value().to_string();
    }
    if let Some(root) = state.root.take() {
        // The dialog owns its focus; clearing avoids a dangling field after projection.
        if focus.get().is_some() {
            focus.clear();
        }
        commands.entity(root).despawn();
    }
    state.generation = owner.generation();
    state.serial = state.serial.saturating_add(1);
    state.snapshot = panel;
    let Some(p) = state.snapshot.as_ref() else {
        return;
    };
    let root = build(&mut commands, &skins, &state, p);
    if !blocked
        && p.ready()
        && p.diagram.selected
        && diagram::needs_query(p)
        && p.diagram.pending.is_none()
        && !owner.modal_active()
        && let Some(play) = &owner.play
    {
        play.link.wheel_dispatch_bound(p, Kind::Metadata);
    }
    state.root = Some(root);
}
fn perform(owner: &HostOwner, state: &State, control: &Control) -> bool {
    if owner.modal_active()
        || control.generation != owner.generation()
        || control.serial != state.serial
    {
        return false;
    }
    let (Some(play), Some(expected)) = (&owner.play, &state.snapshot) else {
        return false;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return false;
    }
    play.link
        .wheel_dispatch_bound(expected, control.kind.clone())
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    guard: ModalGuard,
    fields: Query<&EditableText, With<PresetName>>,
    modals: Query<Entity, With<GameModal>>,
) {
    if modals.iter().any(|e| Some(e) != state.root) || !guard.allows(event.entity) {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    let mut control = control.clone();
    if let Kind::Preset { name, .. } = &mut control.kind {
        let Ok(field) = fields.single() else {
            return;
        };
        if field.pending_paste.is_some() || !field.pending_edits.is_empty() {
            return;
        }
        *name = field.value().to_string();
    }
    perform(&owner, &state, &control);
}
fn build(commands: &mut Commands, skins: &Skins, state: &State, p: &WheelPanel) -> Entity {
    let root = commands
        .spawn((
            GameModal,
            GlobalZIndex(165),
            TabGroup {
                order: 2,
                modal: true,
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            ScrollArea,
            Node {
                width: px(750.),
                max_width: percent(95.),
                max_height: percent(90.),
                padding: UiRect::all(px(16.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    source::dialog_caption(
        commands,
        panel,
        "Wheel of Destiny and Gems",
        skins.font.clone(),
    );
    text(commands, skins, panel, summary(p));
    for slot in 0..36 {
        let row = row(commands, panel);
        let confirmed = p
            .confirmed_wheel
            .as_ref()
            .map_or_else(|| "unavailable".into(), |w| w.points[slot].to_string());
        text(
            commands,
            skins,
            row,
            format!(
                "{} · allocated {confirmed} · proposal {}",
                diagram::metadata(p)
                    .and_then(|r| r.nodes.get(slot))
                    .map_or_else(
                        || oteryn_client_ui::wheel_allocation::slot_label(p, slot),
                        |node| format!("Slice {}: {}", slot + 1, node.name)
                    ),
                p.points[slot]
            ),
        );
        if let Some(points) = p.points[slot].checked_sub(1) {
            button(
                commands,
                skins,
                row,
                state,
                Kind::Points { slot, points },
                "−",
            );
        }
        if let Some(points) = p.points[slot]
            .checked_add(1)
            .filter(|v| *v <= oteryn_client_ui::wheel_allocation::SLOT_CAPACITIES[slot])
        {
            button(
                commands,
                skins,
                row,
                state,
                Kind::Points { slot, points },
                "+",
            );
        }
    }
    let footer = row(commands, panel);
    for (intent, label) in [
        (Intent::Save, "Save Wheel"),
        (Intent::Refresh, "Refresh"),
        (Intent::Retry, "Retry"),
    ] {
        button(commands, skins, footer, state, Kind::Request(intent), label);
    }
    text(commands, skins, panel, "Gems (confirmed by the server)");
    if let Some(a) = &p.confirmed_atelier {
        text(
            commands,
            skins,
            panel,
            format!("Gem revision: {}", a.revision),
        );
        for gem in &a.gems {
            button(
                commands,
                skins,
                panel,
                state,
                Kind::Gem(gem.id),
                &format!(
                    "{}{}",
                    if p.gem == Some(gem.id) {
                        "Selected: "
                    } else {
                        ""
                    },
                    oteryn_client_ui::wheel_allocation::gem_label(gem)
                ),
            );
        }
    }
    let vessels = row(commands, panel);
    for slot in 0..p.vessel_slot_count() {
        button(
            commands,
            skins,
            vessels,
            state,
            Kind::Vessel(slot),
            &oteryn_client_ui::wheel_allocation::vessel_name(slot),
        );
    }
    let gems = row(commands, panel);
    for (kind, label) in [
        (Kind::Socket, "Equip"),
        (Kind::Clear, "Unequip"),
        (Kind::Request(Intent::Dismantle), "Destroy gem…"),
    ] {
        button(commands, skins, gems, state, kind, label);
    }
    if let Some(review) = &p.gems.review {
        text(commands, skins, panel, &review.text);
        let actions = row(commands, panel);
        button(commands, skins, actions, state, Kind::Confirm, "Confirm");
        button(commands, skins, actions, state, Kind::Cancel, "Cancel");
    }
    text(
        commands,
        skins,
        panel,
        "Presets (points only): enter a saved or new name",
    );
    let field = commands
        .spawn((
            PresetName,
            TextInput,
            EditableText::new(&state.name),
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: px(250.),
                min_height: px(25.),
                padding: UiRect::all(px(4.)),
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(field);
    let presets = row(commands, panel);
    for (action, label) in [(0, "Save named build"), (1, "Load named build")] {
        button(
            commands,
            skins,
            presets,
            state,
            Kind::Preset {
                action,
                name: String::new(),
            },
            label,
        );
    }
    text(
        commands,
        skins,
        panel,
        format!(
            "Saved builds: {}",
            p.preset_book
                .0
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    text(commands, skins, panel, &p.preset_feedback);
    button(commands, skins, panel, state, Kind::Close, "Close");
    root
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let row = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(row);
    row
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn((
            source::text(value, source::font(skins.font.clone(), false)),
            Node {
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    state: &State,
    kind: Kind,
    label: &str,
) {
    let entity = commands
        .spawn((
            source::button(skins.buttons.clone(), 140.),
            Control {
                generation: state.generation,
                serial: state.serial,
                kind,
            },
        ))
        .observe(activate)
        .id();
    let caption = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
