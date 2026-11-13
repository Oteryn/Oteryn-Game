//! A submitted editor waits for Bevy's queued edits without changing its destination.
use super::*;
use oteryn_client_ui::chat::{ChannelDialog, ChannelTab};

#[derive(Clone)]
pub(super) struct Pending {
    entity: Entity,
    generation: u64,
    field: Field,
    mode: Mode,
    tab: ConsoleTab,
    channel: Option<ChannelTab>,
    dialog: ChannelDialog,
    focus_message: bool,
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
    state: &mut ChatState,
    entity: Entity,
    field: Field,
    text: &mut EditableText,
    focus_message: bool,
) -> bool {
    let Some(play) = &owner.play else {
        return false;
    };
    if state.pending.is_some() {
        return false;
    }
    let panel = play.link.chat_panel();
    let pending = Pending {
        entity,
        generation: owner.generation(),
        field,
        mode: state.mode,
        tab: panel.console.tab,
        channel: panel.console.current,
        dialog: panel.console.dialog,
        focus_message,
    };
    if editing(text) {
        state.pending = Some(pending);
        false
    } else {
        perform(owner, &pending, text)
    }
}
fn perform(owner: &HostOwner, pending: &Pending, text: &mut EditableText) -> bool {
    let Some(play) = &owner.play else {
        return false;
    };
    match pending.field {
        Field::Message => {
            if commands::submit(
                &play.link,
                pending.generation,
                owner.generation(),
                pending.mode,
                &text.value().to_string(),
            ) {
                set_text(text, "");
                true
            } else {
                false
            }
        }
        Field::Recipient => {
            play.link
                .chat_channel_dialog_name(&text.value().to_string());
            play.link.chat_channel_dialog_confirm()
        }
    }
}
pub(super) fn finish(
    owner: NonSend<HostOwner>,
    mut state: ResMut<ChatState>,
    mut fields: Query<(Entity, &Field, &mut EditableText)>,
    mut focus: ResMut<InputFocus>,
    guard: ModalGuard,
) {
    let Some(pending) = state.pending.take() else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.chat_panel();
    if pending.generation != owner.generation()
        || state.generation != Some(pending.generation)
        || owner.modal_active()
        || !guard.allows(pending.entity)
        || state.mode != pending.mode
        || panel.console.tab != pending.tab
        || panel.console.current != pending.channel
        || (pending.field == Field::Recipient && panel.console.dialog != pending.dialog)
    {
        return;
    }
    let Ok((_, field, mut text)) = fields.get_mut(pending.entity) else {
        return;
    };
    if *field != pending.field {
        return;
    }
    if editing(&text) {
        state.pending = Some(pending);
        return;
    }
    if perform(&owner, &pending, &mut text) && pending.focus_message {
        for (entity, field, _) in &mut fields {
            if *field == Field::Message {
                focus.set(entity, FocusCause::Navigated);
                break;
            }
        }
    }
}
