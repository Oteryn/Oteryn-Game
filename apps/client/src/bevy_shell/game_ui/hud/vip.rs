//! Retained source-shaped VIP rows over canonical 0043 requests and 0258 presence.
use super::*;
use bevy::{
    input_focus::{
        FocusCause, InputFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    text::{EditableText, TextCursorStyle},
    ui_widgets::{ScrollArea, TextInput},
};
use model::{Contact, Snapshot};
use oteryn_client_ui::vip::Intent;
#[path = "vip/dialog.rs"]
mod dialog;
#[path = "vip/groups.rs"]
mod groups;
#[cfg(test)]
#[path = "vip/groups_tests.rs"]
mod groups_tests;
#[path = "vip/input.rs"]
mod input;
#[path = "vip/model.rs"]
mod model;
#[cfg(test)]
#[path = "vip/tests.rs"]
mod tests;
#[path = "vip/view.rs"]
mod view;
#[derive(Resource, Default)]
struct State {
    generation: Option<u64>,
    serial: u64,
    draft: Option<Draft>,
    pending: Option<Pending>,
}
#[derive(Clone)]
struct Draft {
    generation: u64,
    revision: u64,
    serial: u64,
    contact: Option<Contact>,
    group: Option<groups::Draft>,
    text: String,
    icon: Option<u8>,
    notify: Option<bool>,
    groups: Option<Vec<u8>>,
    catalog: Vec<model::Group>,
    feedback: String,
}
#[derive(Clone)]
struct Pending {
    generation: u64,
    revision: u64,
    serial: u64,
    field: Entity,
}
#[derive(Component, Clone, Copy)]
struct Control {
    generation: u64,
    revision: u64,
    command: Action,
}
#[derive(Clone, Copy)]
enum Action {
    Close,
    Add,
    GroupAdd,
    GroupRename(u8),
    GroupRemove(u8),
    Refresh,
    Next(u64),
    Edit(u64),
    Remove(u64),
    Save(u64),
    Cancel(u64),
    Icon(u64, i8),
    Notify(u64),
    Group(u64, u8),
}
#[derive(Component)]
struct List;
#[derive(Component)]
struct Row {
    generation: u64,
    entry: u64,
}
#[derive(Component)]
struct Message;
#[derive(Component)]
struct Dialog(u64);
#[derive(Component)]
struct Field(u64);
#[derive(Component)]
struct DraftLabel(Label);
#[derive(Component)]
struct Toggle(Label);
#[derive(Component)]
struct ToggleIndicator(Label);
#[derive(Clone, Copy)]
enum Label {
    Icon,
    Notify,
    Group(u8),
    Feedback,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .init_resource::<InputFocus>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(
            Startup,
            view::setup
                .after(super::super::setup)
                .after(crate::bevy_shell::ui::setup),
        )
        .add_systems(
            Update,
            (
                view::project,
                groups::project,
                view::dialog,
                input::keyboard,
            )
                .chain(),
        )
        .add_systems(
            PostUpdate,
            input::finish.after(bevy::text::EditableTextSystems),
        );
}
fn available(owner: &HostOwner) -> bool {
    !owner.modal_active()
        && owner.play.as_ref().is_some_and(|play| {
            !play.view.gameplay_input_blocked(&play.link) && !play.link.server_modal_active()
        })
}
fn current(owner: &HostOwner, draft: &Draft) -> bool {
    if !available(owner) || owner.generation() != draft.generation {
        return false;
    }
    let Some(play) = &owner.play else {
        return false;
    };
    let panel = play.link.vip_panel();
    panel.selected
        && !panel.pending
        && panel.revision == draft.revision
        && groups::current(&panel, draft.group.as_ref())
        && draft.contact.as_ref().is_none_or(|contact| {
            Snapshot::read(&panel)
                .ok()
                .is_some_and(|snapshot| snapshot.entries.iter().any(|entry| entry == contact))
        })
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    control: Control,
    label: &str,
    width: f32,
    id: &str,
) -> Entity {
    let entity = commands
        .spawn((
            source::button(skins.buttons.clone(), width),
            control,
            Name::new(id.to_owned()),
        ))
        .observe(input::activate)
        .id();
    let text = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(text);
    entity
}
