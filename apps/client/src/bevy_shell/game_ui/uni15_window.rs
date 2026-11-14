//! Writing and mail, projected from the same owner as the Win32 form.
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0115_mail::view::MailBinding;
use bevy::{prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::mail::Intent;
#[derive(Resource, Default)]
struct State {
    snapshot: Option<(u64, MailBinding)>,
    overlay: Option<Entity>,
    pending: Option<(u64, Option<Intent>)>,
}
#[derive(Component)]
struct Field(usize);
#[derive(Component)]
struct Control {
    generation: u64,
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
        focus.clear();
        commands.entity(overlay).despawn();
    }
    state.pending = None;
    if let Some((generation, binding)) = &current {
        state.overlay = Some(widgets::build(
            &mut commands,
            &skins,
            *generation,
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
        && state.snapshot.as_ref().is_some_and(|(generation, _)| {
            *generation == control.generation && *generation == owner.generation()
        })
    {
        state.pending = Some((control.generation, control.intent));
    }
}
fn finish(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<(&Field, &EditableText)>,
) {
    let Some((generation, intent)) = state.pending.take() else {
        return;
    };
    if generation != owner.generation() || owner.modal_active() {
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
    let mut values = binding.panel.fields.clone();
    for (field, text) in &fields {
        values[field.0] = text.text.clone();
    }
    let _ = play.link.mail_dispatch_bound(binding, values, intent);
}
