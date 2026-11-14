//! Catalogue presentation over the retained inspection owner.
mod widgets;
use super::super::{owner::HostOwner, ui::Skins};
use super::{GameModal, modal::ModalGuard};
use crate::play::feature_0132_inspection::view::ItemCatalogueSnapshot;
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};

#[derive(Resource, Default)]
struct State {
    snapshot: Option<(u64, ItemCatalogueSnapshot)>,
    widgets: Option<widgets::Widgets>,
}
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    expected: ItemCatalogueSnapshot,
    action: Option<usize>,
}
#[derive(Component)]
struct Field(usize);
#[derive(Component)]
struct CatalogueText;
const ACTIONS: [&str; 10] = [
    "Browse",
    "Next page",
    "Previous category",
    "Next category",
    "Previous item",
    "Next item",
    "Item details",
    "Drop sources",
    "More drop sources",
    "Market offers",
];
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn open(owner: &HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
    {
        play.link.item_catalogue_open_bound();
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
) {
    let current = owner
        .play
        .as_ref()
        .map(|play| play.link.item_catalogue_snapshot());
    let current = current.filter(|snapshot| snapshot.panel.catalogue_open);
    if current.as_ref().is_some_and(|snapshot| {
        state
            .snapshot
            .as_ref()
            .is_some_and(|(generation, previous)| {
                *generation == owner.generation() && previous == snapshot
            })
    }) {
        return;
    }
    if let Some(widgets) = state.widgets.take() {
        let mut entity = focus.get();
        while let Some(current) = entity {
            if current == widgets.overlay {
                focus.clear();
                break;
            }
            entity = parents.get(current).ok().map(ChildOf::parent);
        }
        commands.entity(widgets.overlay).despawn();
    }
    state.snapshot = current.map(|snapshot| (owner.generation(), snapshot));
    if let Some((generation, snapshot)) = &state.snapshot {
        state.widgets = Some(widgets::build(&mut commands, &skin, *generation, snapshot));
    }
}
fn perform(owner: &HostOwner, control: &Control, fields: [String; 3]) -> bool {
    if control.generation != owner.generation()
        || (control.action.is_some() && owner.modal_active())
    {
        return false;
    }
    owner.play.as_ref().is_some_and(|play| {
        play.link
            .item_catalogue_dispatch_bound(&control.expected, fields, control.action)
    })
}
fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    owner: NonSend<HostOwner>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if control.action.is_some() && !guard.allows(event.entity) {
        return;
    }
    let mut values = control.expected.fields();
    for (field, text) in &fields {
        values[field.0] = text.editor.text().to_string();
    }
    perform(&owner, control, values);
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    for event in events.read() {
        if event.repeat
            || event.state != bevy::input::ButtonState::Pressed
            || event.key_code != KeyCode::Escape
        {
            continue;
        }
        if let Some((generation, expected)) = &state.snapshot {
            perform(
                &owner,
                &Control {
                    generation: *generation,
                    expected: expected.clone(),
                    action: None,
                },
                expected.fields(),
            );
        }
    }
}
#[cfg(test)]
mod tests;
