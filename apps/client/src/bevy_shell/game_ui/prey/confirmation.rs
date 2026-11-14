//! Prey confirmation state: slot editor drafts and the cancel control.
use super::{Control, State};
use crate::bevy_shell::{game_ui::modal::ModalGuard, owner::HostOwner};
use bevy::{prelude::*, ui_widgets::Activate};
pub(super) fn editor_drafts(
    editors: &Query<(&CreatureField, &bevy::text::EditableText)>,
) -> Vec<(u8, String)> {
    let mut drafts: Vec<_> = editors
        .iter()
        .map(|(field, editor)| (field.0, editor.value().to_string()))
        .collect();
    drafts.sort_by_key(|(slot, _)| *slot);
    drafts
}
pub(super) fn editing(editors: &Query<(&CreatureField, &bevy::text::EditableText)>) -> bool {
    editors.iter().any(|(_, editor)| {
        editor.pending_paste.is_some()
            || editor
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
    })
}
pub(super) fn invalidate_confirmation(
    mut commands: Commands,
    editors: Query<(&CreatureField, &bevy::text::EditableText)>,
    mut state: ResMut<State>,
) {
    if state.confirmation.is_some()
        && (editing(&editors) || state.confirmation_draft != editor_drafts(&editors))
    {
        clear_confirmation(&mut state, &mut commands);
    }
}
pub(super) fn clear_confirmation(state: &mut State, commands: &mut Commands) {
    state.confirmation = None;
    state.confirmation_draft.clear();
    if let Some(root) = state.confirmation_root.take() {
        commands.entity(root).despawn();
    }
}
#[derive(Component)]
pub(super) struct CreatureField(pub(super) u8);
#[derive(Component)]
pub(super) struct CancelConfirmation(pub(super) Control);
pub(super) fn cancel_confirmation(
    event: On<Activate>,
    controls: Query<&CancelConfirmation>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    mut commands: Commands,
) {
    if !guard.allows(event.entity) || owner.modal_active() {
        return;
    }
    let Ok(CancelConfirmation(control)) = controls.get(event.entity) else {
        return;
    };
    if owner.generation() != control.generation
        || owner
            .play
            .as_ref()
            .is_none_or(|p| p.link.prey_panel() != control.expected)
    {
        return;
    }
    clear_confirmation(&mut state, &mut commands);
}
