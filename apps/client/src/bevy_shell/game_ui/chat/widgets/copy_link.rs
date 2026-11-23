//! Copying a URL from received text requires a second player action when enabled.
use super::*;

#[derive(Event)]
pub(super) struct Request(pub(super) String);

#[derive(Component)]
struct Pending {
    generation: u64,
    text: String,
}

#[derive(Resource, Default)]
struct QueuedCopies(Vec<(Entity, Vec<bevy::text::TextEdit>)>);

pub(in crate::bevy_shell::game_ui::chat) fn install(app: &mut App) {
    app.init_resource::<QueuedCopies>()
        .add_observer(request)
        .add_systems(
            PostUpdate,
            (
                queue_copy.before(bevy::text::EditableTextSystems),
                finish_copy.after(bevy::text::EditableTextSystems),
            ),
        );
}
fn contains_link(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("https://") || lower.contains("http://") || lower.contains("www.")
}
type ChatText = Or<(With<Transcript>, With<Field>)>;
fn queue_copy(
    owner: NonSend<HostOwner>,
    mut rows: Query<(Entity, &mut EditableText), ChatText>,
    mut queued: ResMut<QueuedCopies>,
) {
    if !owner.settings.preferences.confirm_copy_link {
        return;
    }
    for (entity, mut field) in &mut rows {
        if let Some(index) = field
            .pending_edits
            .iter()
            .position(|edit| matches!(edit, bevy::text::TextEdit::Copy))
        {
            queued
                .0
                .push((entity, field.pending_edits.split_off(index)));
        }
    }
}
fn finish_copy(
    mut rows: Query<&mut EditableText, ChatText>,
    mut queued: ResMut<QueuedCopies>,
    mut commands: Commands,
) {
    for (entity, mut edits) in queued.0.drain(..) {
        let Ok(mut field) = rows.get_mut(entity) else {
            continue;
        };
        // A pending asynchronous paste can defer the selection preceding this copy.
        if field.pending_paste.is_none() && field.pending_edits.is_empty() {
            edits.remove(0);
            if let Some(text) = field.editor().selected_text() {
                commands.trigger(Request(text.to_owned()));
            }
        }
        field.pending_edits.append(&mut edits);
    }
}
fn request(
    event: On<Request>,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<ChatState>,
    mut commands: Commands,
) {
    if owner.play.is_none() || owner.modal_active() {
        return;
    }
    if !owner.settings.preferences.confirm_copy_link || !contains_link(&event.0) {
        commands.trigger(CopyText(event.0.clone()));
        return;
    }
    if let Some(old) = state.menu.take() {
        commands.entity(old).try_despawn();
    }
    let look = popup::Look {
        font: skin.font.clone(),
        frame: skin.frame.clone(),
        checked: None,
        unchecked: None,
    };
    let (menu, layer) = crate::bevy_shell::game_ui::hud_panes::context_menu::spawn(
        &mut commands,
        &look,
        Vec2::splat(32.),
        None,
        &[
            popup::Item::entry("Copy link to clipboard"),
            popup::Item::entry("Cancel"),
        ],
    );
    commands
        .entity(menu.scrim)
        .observe(messages::message_dismiss);
    commands
        .entity(menu.entries[0])
        .insert(Pending {
            generation: owner.generation(),
            text: event.0.clone(),
        })
        .observe(confirm);
    commands.entity(menu.entries[1]).observe(cancel);
    state.menu = Some(layer);
}
fn cancel(_: On<bevy::ui_widgets::Activate>, mut state: ResMut<ChatState>, mut commands: Commands) {
    if let Some(menu) = state.menu.take() {
        commands.entity(menu).try_despawn();
    }
}
fn confirm(
    event: On<bevy::ui_widgets::Activate>,
    pending: Query<&Pending>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    mut state: ResMut<ChatState>,
    mut commands: Commands,
) {
    if guard.allows(event.entity)
        && !owner.modal_active()
        && let Ok(value) = pending.get(event.entity)
        && value.generation == owner.generation()
        && let Some(menu) = state.menu.take()
    {
        commands.trigger(CopyText(value.text.clone()));
        commands.entity(menu).try_despawn();
    }
}
