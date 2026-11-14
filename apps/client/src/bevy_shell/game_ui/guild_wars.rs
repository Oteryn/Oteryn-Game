//! Guild war presentation over the retained owner; confirmation never invents a result.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0103_guild_war::view::GuildWarsBinding;
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::guild_war::{Intent, Panel};
#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct Field(usize);
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Propose(Intent),
    Confirm,
    Cancel,
    Close,
}
#[derive(Component, Clone, Copy)]
struct Control {
    generation: u64,
    revision: u64,
    kind: Kind,
}
#[derive(Clone)]
struct Confirmation {
    binding: GuildWarsBinding,
    fields: [String; 3],
    intent: Intent,
}
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(u64, Panel)>,
    binding: Option<GuildWarsBinding>,
    revision: u64,
    overlay: Option<Entity>,
    confirmation: Option<Confirmation>,
    redraw: bool,
    draft: [String; 3],
    pending: Option<(Entity, Control)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
pub(super) fn open(owner: &HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
    {
        play.link.guild_war_open();
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
) {
    let binding = owner
        .play
        .as_ref()
        .map(|play| play.link.guild_wars_binding());
    let snapshot = binding
        .as_ref()
        .filter(|binding| binding.panel.open)
        .map(|binding| (owner.generation(), binding.panel.clone()));
    if state.snapshot == snapshot && !state.redraw {
        return;
    }
    let changed = state.snapshot != snapshot;
    if let Some(overlay) = state.overlay.take() {
        if inside(focus.get(), overlay, &parents) {
            focus.clear();
        }
        commands.entity(overlay).despawn();
    }
    if changed {
        state.confirmation = None;
        state.pending = None;
        state.draft = snapshot
            .as_ref()
            .map(|(_, panel)| panel.fields.clone())
            .unwrap_or_default();
    }
    state.redraw = false;
    state.snapshot = snapshot;
    state.binding = binding;
    let Some((generation, panel)) = state.snapshot.clone() else {
        return;
    };
    let Some(revision) = state.revision.checked_add(1) else {
        return;
    };
    state.revision = revision;
    state.overlay = Some(widgets::build(
        &mut commands,
        &skins,
        generation,
        revision,
        &panel,
        &state.draft,
        state.confirmation.as_ref(),
    ));
}
fn inside(mut entity: Option<Entity>, ancestor: Entity, parents: &Query<&ChildOf>) -> bool {
    while let Some(current) = entity {
        if current == ancestor {
            return true;
        }
        entity = parents.get(current).ok().map(ChildOf::parent);
    }
    false
}
fn current(owner: &HostOwner, state: &State, control: &Control) -> bool {
    !owner.modal_active()
        && owner.generation() == control.generation
        && control.revision == state.revision
        && state.snapshot.as_ref().is_some_and(|(generation, panel)| {
            *generation == owner.generation()
                && panel.open
                && owner
                    .play
                    .as_ref()
                    .is_some_and(|play| play.link.guild_war_panel() == *panel)
        })
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity) || !current(&owner, &state, control) {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if control.kind != Kind::Close && play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    match control.kind {
        Kind::Close => {
            if let Some(binding) = &state.binding {
                play.link.guild_wars_close_bound(binding);
            }
        }
        Kind::Cancel => {
            state.confirmation = None;
            state.pending = None;
            state.redraw = true;
        }
        Kind::Propose(_) => {
            if fields.iter().any(|(_, text)| editing(text)) {
                state.pending = Some((event.entity, *control));
            } else {
                propose(&mut state, control.kind, &fields);
            }
        }
        Kind::Confirm => {
            if let Some(confirmation) = state.confirmation.take() {
                play.link.guild_wars_submit_bound(
                    &confirmation.binding,
                    &confirmation.fields,
                    confirmation.intent,
                );
                state.redraw = true;
            }
        }
    }
}
fn editing(text: &EditableText) -> bool {
    text.pending_paste.is_some()
        || text
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
}
fn propose(state: &mut State, kind: Kind, fields: &Query<(&Field, &EditableText)>) {
    let Kind::Propose(intent) = kind else {
        return;
    };
    let Some(binding) = state.binding.clone() else {
        return;
    };
    if binding.panel.pending || !binding.panel.selected || state.confirmation.is_some() {
        return;
    }
    let mut values = binding.panel.fields.clone();
    for (field, text) in fields {
        values[field.0] = text.value().to_string();
    }
    state.draft = values.clone();
    state.confirmation = Some(Confirmation {
        binding,
        fields: values,
        intent,
    });
    state.redraw = true;
}
// TextInput applies queued edits in PostUpdate; never confirm an older editor buffer.
fn finish(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
) {
    let Some((source, control)) = state.pending.take() else {
        return;
    };
    if !current(&owner, &state, &control) || !guard.allows(source) {
        return;
    }
    if fields.iter().any(|(_, text)| editing(text)) {
        state.pending = Some((source, control));
    } else {
        propose(&mut state, control.kind, &fields);
    }
}
fn shortcut(event: &bevy::input::keyboard::KeyboardInput, keys: &ButtonInput<KeyCode>) -> bool {
    event.key_code == KeyCode::KeyW
        && event.state == bevy::input::ButtonState::Pressed
        && !event.repeat
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
        && ![
            KeyCode::ShiftLeft,
            KeyCode::ShiftRight,
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
        ]
        .iter()
        .any(|key| keys.pressed(*key))
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    focus: Res<InputFocus>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for event in events.read() {
        if shortcut(event, &keys) && guard.allows_focus(focus.get()) {
            open(&owner);
        }
        if event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
            && let Some(overlay) = state.overlay
            && guard.allows(overlay)
            && let Some(binding) = &state.binding
            && let Some(play) = &owner.play
            && state
                .snapshot
                .as_ref()
                .is_some_and(|(generation, _)| *generation == owner.generation())
        {
            play.link.guild_wars_close_bound(binding);
        }
    }
}
