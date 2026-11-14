//! Complete the original server dialog's editor before queuing its revision-bound reply.
use super::*;
#[derive(Clone, Copy)]
pub(super) struct Pending {
    key: Key,
    button: u8,
    source: Entity,
    field: Entity,
    selected: Option<u8>,
    remember: bool,
}
fn editing(text: &EditableText) -> bool {
    text.pending_paste.is_some()
        || text
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
}
pub(super) fn request(
    owner: &HostOwner,
    state: &mut State,
    key: Key,
    button: u8,
    source: Entity,
    input: &Query<(Entity, &EditableText), With<ValueField>>,
) {
    state.pending = None;
    let cancel = owner
        .play
        .as_ref()
        .and_then(|play| play.link.server_modal_panel())
        .and_then(|panel| panel.panel.active)
        .is_some_and(|dialog| dialog.cancel_button == Some(button));
    // Cancellation does not carry a value and must remain available during a stalled paste.
    if cancel {
        submit(owner, state, key, button, "");
    } else if let Ok((field, text)) = input.single() {
        if editing(text) {
            state.pending = Some(Pending {
                key,
                button,
                source,
                field,
                selected: state.selected,
                remember: state.remember,
            });
        } else {
            submit(owner, state, key, button, &text.value().to_string());
        }
    } else {
        submit(owner, state, key, button, "");
    }
}
pub(super) fn finish(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    input: Query<(Entity, &EditableText), With<ValueField>>,
    guard: ModalGuard,
) {
    let Some(pending) = state.pending.take() else {
        return;
    };
    if !current(&owner, &state, pending.key)
        || !guard.allows(pending.source)
        || !guard.allows(pending.field)
        || state.selected != pending.selected
        || state.remember != pending.remember
    {
        return;
    }
    let Ok((_, text)) = input.get(pending.field) else {
        return;
    };
    if editing(text) {
        state.pending = Some(pending);
        return;
    }
    submit(
        &owner,
        &mut state,
        pending.key,
        pending.button,
        &text.value().to_string(),
    );
}
