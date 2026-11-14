//! Private-channel management projected from the existing chat owner.
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0080_chat_private_channel::view::PrivateChannelsBound as Bound;
use bevy::{
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::{Activate, TextInput},
};
use oteryn_client_ui::chat_private_channel::{Intent, leave_label, panel_text};
use oteryn_client_ui_bevy as source;

#[derive(Resource, Default)]
struct State {
    session: Option<(u64, Bound)>,
    shown: Option<(u64, Bound)>,
    root: Option<Entity>,
    confirmation: Option<(Intent, [String; 2])>,
    dirty: bool,
}
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Action {
    Select(u16),
    Request(Intent),
    Confirm(Intent),
    Cancel,
    Close,
}
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    bound: Bound,
    action: Action,
}
#[derive(Component)]
struct Field(usize);

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(
            PostUpdate,
            invalidate_confirmation.after(bevy::text::EditableTextSystems),
        );
}
/// Opening is queued as presentation state, fenced again against the actual owner session.
pub(in crate::bevy_shell) fn open(owner: &HostOwner, commands: &mut Commands) {
    let Some(play) = &owner.play else {
        return;
    };
    let bound = play.link.chat_private_bound();
    if owner.modal_active()
        || play.view.gameplay_input_blocked(&play.link)
        || !play.link.chat_private_admit_bound(&bound, false)
    {
        return;
    }
    let generation = owner.generation();
    commands.queue(move |world: &mut World| {
        let owner = world.non_send::<HostOwner>();
        if owner.generation() == generation
            && !owner.modal_active()
            && owner.play.as_ref().is_some_and(|play| {
                !play.view.gameplay_input_blocked(&play.link)
                    && play.link.chat_private_admit_bound(&bound, false)
            })
        {
            let mut state = world.resource_mut::<State>();
            state.session = Some((generation, bound));
            state.dirty = true;
        }
    });
}
/// The chooser entry uses the same modal admission and owner binding as the toolbar.
pub(in crate::bevy_shell) fn entry(
    event: On<Activate>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut commands: Commands,
) {
    if guard.allows(event.entity) {
        open(&owner, &mut commands);
    }
}
fn label(commands: &mut Commands, skins: &Skins, root: Entity, text: &str) {
    let child = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(root).add_child(child);
}
fn button(commands: &mut Commands, skins: &Skins, root: Entity, label: &str, control: Control) {
    let child = commands
        .spawn((source::button(skins.buttons.clone(), 230.), control))
        .observe(activate)
        .id();
    let caption = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(child).add_child(caption);
    commands.entity(root).add_child(child);
}
fn drafts(fields: &Query<(&Field, &EditableText)>) -> Option<[String; 2]> {
    let mut values: [Option<String>; 2] = [None, None];
    for (field, text) in fields {
        if text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
        {
            return None;
        }
        values[field.0] = Some(text.value().to_string());
    }
    let [Some(name), Some(message)] = values else {
        return None;
    };
    Some([name, message])
}
fn invalidate_confirmation(mut state: ResMut<State>, fields: Query<(&Field, &EditableText)>) {
    if state
        .confirmation
        .as_ref()
        .is_some_and(|(_, frozen)| drafts(&fields).as_ref() != Some(frozen))
    {
        state.confirmation = None;
        state.dirty = true;
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    fields: Query<(&Field, &EditableText)>,
) {
    let next = state.session.as_ref().and_then(|(generation, session)| {
        owner
            .play
            .as_ref()
            .filter(|play| {
                *generation == owner.generation()
                    && play.link.chat_private_admit_bound(session, true)
            })
            .map(|play| (*generation, play.link.chat_private_bound()))
    });
    if state.shown == next && !state.dirty {
        return;
    }
    let values = drafts(&fields);
    if state.shown != next {
        state.confirmation = None;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.shown = next.clone();
    state.dirty = false;
    let Some((generation, bound)) = next else {
        state.session = None;
        state.confirmation = None;
        return;
    };
    let panel = &bound.panel;
    let values = values.unwrap_or_else(|| [panel.name.clone(), panel.text.clone()]);
    let root = commands
        .spawn((
            GameModal,
            Name::new("private_channels.window"),
            GlobalZIndex(165),
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            bevy::ui_widgets::ScrollArea,
            bevy::input_focus::tab_navigation::TabGroup::new(16),
            Node {
                position_type: PositionType::Absolute,
                left: percent(15.),
                top: percent(10.),
                width: percent(70.),
                max_height: percent(80.),
                overflow: Overflow::scroll_y(),
                padding: UiRect::all(px(24.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    state.root = Some(root);
    source::dialog_caption(&mut commands, root, "Private channels", skins.font.clone());
    label(&mut commands, &skins, root, &panel_text(panel));
    let control = |action| Control {
        generation,
        bound: bound.clone(),
        action,
    };
    if let Some(list) = &panel.list {
        if let Some(id) = list.own_channel {
            button(
                &mut commands,
                &skins,
                root,
                &format!("Own channel ({id})"),
                control(Action::Select(id)),
            );
        }
        for invite in &list.invited {
            button(
                &mut commands,
                &skins,
                root,
                &format!("{} ({})", invite.owner_name, invite.channel_id),
                control(Action::Select(invite.channel_id)),
            );
        }
    }
    for (index, caption) in ["Player name", "Message"].iter().enumerate() {
        label(&mut commands, &skins, root, caption);
        let mut editable = EditableText::new(values[index].clone());
        editable.max_characters = Some(if index == 0 { 120 } else { 1_020 });
        editable.allow_newlines = false;
        let field = commands
            .spawn((
                Field(index),
                TextInput,
                editable,
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::default(),
                TextCursorStyle::default(),
                bevy::input_focus::tab_navigation::TabIndex(0),
                Node {
                    min_height: px(26.),
                    ..default()
                },
            ))
            .id();
        commands.entity(root).add_child(field);
    }
    for (intent, caption) in [
        (Intent::Create, "Create/Open own"),
        (Intent::List, "Refresh"),
        (Intent::Join, "Join"),
        (Intent::Leave, leave_label(panel)),
        (Intent::Invite, "Invite player"),
        (Intent::Exclude, "Exclude player"),
        (Intent::Send, "Send"),
    ] {
        button(
            &mut commands,
            &skins,
            root,
            caption,
            control(Action::Request(intent)),
        );
    }
    if let Some((intent, frozen)) = &state.confirmation {
        let summary = match intent {
            Intent::Exclude => match panel.list.as_ref().and_then(|list| list.own_channel) {
                Some(id) => format!("Exclude {} from own channel {id}?", frozen[0]),
                None => format!("Exclude {} from your own channel?", frozen[0]),
            },
            _ => match panel.channel {
                Some(id) => format!("{} {id}?", leave_label(panel)),
                None => format!("{}?", leave_label(panel)),
            },
        };
        label(&mut commands, &skins, root, &summary);
        button(
            &mut commands,
            &skins,
            root,
            "Confirm",
            control(Action::Confirm(*intent)),
        );
        button(
            &mut commands,
            &skins,
            root,
            "Cancel",
            control(Action::Cancel),
        );
    }
    button(&mut commands, &skins, root, "Close", control(Action::Close));
}
fn activate(
    event: On<Activate>,
    queries: (Query<&Control>, Query<(&Field, &EditableText)>),
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let (controls, fields) = queries;
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if owner.generation() != control.generation
        || state.session.as_ref().is_none_or(|(generation, session)| {
            *generation != control.generation || !play.link.chat_private_admit_bound(session, true)
        })
    {
        return;
    }
    // Closing never consumes or clears an in-flight owner operation.
    if control.action == Action::Close {
        if play.link.chat_private_admit_bound(&control.bound, true) {
            close(&mut state);
        }
        return;
    }
    if !guard.allows(event.entity)
        || owner.modal_active()
        || play.view.gameplay_input_blocked(&play.link)
        || !play.link.chat_private_admit_bound(&control.bound, false)
    {
        return;
    }
    let Some(values) = drafts(&fields) else {
        return;
    };
    if state
        .confirmation
        .as_ref()
        .is_some_and(|(_, frozen)| frozen != &values)
    {
        state.confirmation = None;
        state.dirty = true;
        return;
    }
    match control.action {
        Action::Close => close(&mut state),
        Action::Cancel => {
            state.confirmation = None;
            state.dirty = true;
        }
        Action::Select(id) => {
            if state.confirmation.is_none() {
                play.link.chat_private_select_bound(&control.bound, id);
            }
        }
        Action::Request(intent) | Action::Confirm(intent) => {
            if control.bound.panel.pending || !control.bound.panel.selected {
                return;
            }
            let destructive = matches!(intent, Intent::Leave | Intent::Exclude);
            if control.action == Action::Request(intent) && destructive {
                if state.confirmation.is_none() {
                    state.confirmation = Some((intent, values));
                    state.dirty = true;
                }
                return;
            }
            let confirmed = state.confirmation.as_ref() == Some(&(intent, values.clone()));
            if (destructive && !confirmed) || (!destructive && state.confirmation.is_some()) {
                return;
            }
            if play.link.chat_private_dispatch_bound(
                &control.bound,
                intent,
                &values[0],
                &values[1],
                confirmed,
            ) {
                state.confirmation = None;
            }
        }
    }
}
fn close(state: &mut State) {
    state.session = None;
    state.confirmation = None;
    state.dirty = true;
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    }) && let Some((generation, bound)) = &state.session
        && *generation == owner.generation()
        && owner
            .play
            .as_ref()
            .is_some_and(|play| play.link.chat_private_admit_bound(bound, true))
    {
        close(&mut state);
    }
}
#[cfg(test)]
mod tests;
