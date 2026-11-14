//! Writing and mail, projected from the same owner as the Win32 form.
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0115_mail::view::{MAX_TEXT_BYTES, MailBinding};
use bevy::{prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::mail::Intent;
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(u64, MailBinding)>,
    overlay: Option<Entity>,
    pending: Option<(Entity, u64, u64, Option<Intent>)>,
    revision: u64,
}
#[derive(Component)]
struct Field(usize);
#[derive(Component)]
struct Control {
    generation: u64,
    revision: u64,
    intent: Option<Intent>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Update, project)
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
    parents: Query<&ChildOf>,
) {
    let current = owner
        .play
        .as_ref()
        .map(|p| p.link.mail_binding())
        .filter(|p| p.panel.open)
        .map(|p| (owner.generation(), p));
    if state.snapshot == current {
        return;
    }
    if let Some(overlay) = state.overlay.take() {
        let mut focused = focus.get();
        while let Some(entity) = focused {
            if entity == overlay {
                focus.clear();
                break;
            }
            focused = parents.get(entity).ok().map(ChildOf::parent);
        }
        commands.entity(overlay).despawn();
    }
    state.pending = None;
    state.revision = state.revision.saturating_add(1);
    if let Some((generation, binding)) = &current {
        state.overlay = Some(widgets::build(
            &mut commands,
            &skins,
            *generation,
            state.revision,
            &binding.panel,
        ));
    }
    state.snapshot = current;
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if guard.allows(event.entity)
        && !owner.modal_active()
        && let Ok(control) = controls.get(event.entity)
        && control.revision == state.revision
        && state.snapshot.as_ref().is_some_and(|(generation, _)| {
            *generation == control.generation && *generation == owner.generation()
        })
    {
        state.pending = Some((
            event.entity,
            control.generation,
            control.revision,
            control.intent,
        ));
    }
}
fn finish(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
) {
    let Some((source, generation, revision, intent)) = state.pending.take() else {
        return;
    };
    if generation != owner.generation()
        || revision != state.revision
        || owner.modal_active()
        || !guard.allows(source)
    {
        return;
    }
    let Some((_, binding)) = &state.snapshot else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    if intent.is_some()
        && fields.iter().any(|(_, text)| {
            text.pending_paste.is_some()
                || text
                    .pending_edits
                    .iter()
                    .any(bevy::text::TextEdit::is_destructive)
        })
    {
        state.pending = Some((source, generation, revision, intent));
        return;
    }
    let mut values = binding.panel.fields.clone();
    for (field, text) in &fields {
        values[field.0] = text.value().to_string();
    }
    let _ = play.link.mail_dispatch_bound(binding, values, intent);
}

#[cfg(test)]
mod tests;
