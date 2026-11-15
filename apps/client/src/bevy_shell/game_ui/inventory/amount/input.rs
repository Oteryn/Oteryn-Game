use super::*;
use crate::bevy_shell::game_ui::modal::ModalGuard;
use bevy::{input::keyboard::KeyboardInput, input_focus::InputFocus, ui_widgets::ValueChange};
pub(super) fn changed(
    event: On<ValueChange<String>>,
    guard: ModalGuard,
    mut fields: Query<(&Field, &mut EditableText)>,
    owner: NonSend<HostOwner>,
    mut ui: ResMut<InventoryUi>,
) {
    if !guard.allows(event.source) {
        return;
    }
    let Ok((field, mut editor)) = fields.get_mut(event.source) else {
        return;
    };
    if !super::super::actions::prompt_current(field.0, &owner, &ui) {
        return;
    }
    set_draft(&owner, &mut ui, field.0, event.value.clone());
    set_editor(&mut editor, &event.value);
}
/// Bridge upstream completed edits into the same ValueChange observer used by native widget callers.
pub(super) fn edited(
    mut commands: Commands,
    fields: Query<(Entity, &Field, &EditableText), Changed<EditableText>>,
    guard: ModalGuard,
    ui: Res<InventoryUi>,
) {
    for (entity, field, editor) in &fields {
        let raw = editor.value().to_string();
        if ui.prompt == Some(field.0) && raw != value(&ui) && guard.allows(entity) {
            commands.trigger(ValueChange {
                source: entity,
                value: raw,
                is_final: false,
            });
        }
    }
}
pub(in crate::bevy_shell::game_ui::inventory) fn prepare_confirm(
    prompt: Prompt,
    fields: &Query<(&Field, &EditableText)>,
    owner: &HostOwner,
    ui: &mut InventoryUi,
) -> bool {
    let Some((_, editor)) = fields.iter().find(|(field, _)| field.0 == prompt) else {
        return false;
    };
    if editor
        .pending_edits
        .iter()
        .any(bevy::text::TextEdit::is_destructive)
        || editor.pending_paste.is_some()
    {
        ui.pending_amount_confirm = Some(prompt);
        return true;
    }
    set_draft(owner, ui, prompt, editor.value().to_string());
    false
}
pub(super) fn keyboard(
    mut keys: MessageReader<KeyboardInput>,
    guard: ModalGuard,
    focus: Res<InputFocus>,
    fields: Query<&Field>,
    owner: NonSend<HostOwner>,
    mut ui: ResMut<InventoryUi>,
) {
    let field = focus
        .get()
        .and_then(|entity| guard.allows(entity).then_some(entity))
        .and_then(|entity| fields.get(entity).ok());
    let Some(field) = field else {
        keys.clear();
        return;
    };
    if !super::super::actions::prompt_current(field.0, &owner, &ui)
        || owner
            .play
            .as_ref()
            .is_some_and(|play| play.link.server_modal_active())
    {
        keys.clear();
        return;
    }
    for key in keys.read() {
        if key.state != bevy::input::ButtonState::Pressed || key.repeat {
            continue;
        }
        match key.key_code {
            KeyCode::Enter | KeyCode::NumpadEnter => ui.pending_amount_confirm = Some(field.0),
            KeyCode::Escape => {
                ui.pending_amount_confirm = None;
                super::super::actions::choose(
                    Control::Cancel {
                        generation: field.0.generation,
                        prompt: Some(field.0),
                    },
                    &owner,
                    &mut ui,
                );
            }
            _ => {}
        }
    }
}
pub(super) fn finish_confirm(
    guard: ModalGuard,
    fields: Query<(Entity, &Field, &EditableText)>,
    owner: NonSend<HostOwner>,
    mut ui: ResMut<InventoryUi>,
) {
    let Some(prompt) = ui.pending_amount_confirm else {
        return;
    };
    let Some((entity, _, editor)) = fields.iter().find(|(_, field, _)| field.0 == prompt) else {
        ui.pending_amount_confirm = None;
        return;
    };
    if !guard.allows(entity)
        || owner
            .play
            .as_ref()
            .is_some_and(|play| play.link.server_modal_active())
    {
        ui.pending_amount_confirm = None;
        return;
    }
    // Clipboard reads may remain pending across frames; never submit the old displayed value.
    if editor.pending_paste.is_some()
        || editor
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
    {
        return;
    }
    ui.pending_amount_confirm = None;
    set_draft(&owner, &mut ui, prompt, editor.value().to_string());
    confirm(&owner, &mut ui, prompt);
}
