use super::*;
use bevy::{input::ButtonState, input::keyboard::KeyboardInput};
pub(super) fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    controls: Query<&Control>,
    fields: Query<(Entity, &Field)>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<State>,
) {
    if !guard.allows(event.entity) || !available(&owner) {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if control.generation != owner.generation() {
        return;
    }
    let Some(panel) = owner.play.as_ref().map(|play| play.link.vip_panel()) else {
        return;
    };
    if panel.revision != control.revision {
        return;
    }
    if matches!(control.command, Action::Close) {
        if owner.settings.preferences.panel_visible("vip") {
            let mut preferences = owner.settings.preferences.clone();
            preferences.visible_panels.retain(|panel| panel != "vip");
            owner.settings.apply(preferences);
        }
        if let Some(play) = &owner.play {
            play.link.vip_close();
        }
        state.draft = None;
        state.pending = None;
        return;
    }
    if !panel.selected || panel.pending {
        return;
    }
    let Some(play) = &owner.play else { return };
    match control.command {
        Action::Close => {}
        Action::Refresh => play.link.vip_request(Intent::Query(0)),
        Action::Next(cursor) => {
            if Snapshot::read(&panel)
                .ok()
                .and_then(|snapshot| snapshot.next_entry)
                == Some(cursor)
                && cursor != 0
            {
                play.link.vip_request(Intent::Query(cursor));
            }
        }
        Action::Add | Action::Edit(_) => {
            let Ok(snapshot) = Snapshot::read(&panel) else {
                return;
            };
            let contact = match control.command {
                Action::Edit(entry) => {
                    let Some(contact) = snapshot.entries.iter().find(|row| row.entry == entry)
                    else {
                        return;
                    };
                    Some(contact.clone())
                }
                _ => None,
            };
            state.serial = state.serial.saturating_add(1);
            state.pending = None;
            state.draft = Some(Draft {
                generation: owner.generation(),
                revision: panel.revision,
                serial: state.serial,
                text: contact
                    .as_ref()
                    .and_then(|entry| entry.description.clone())
                    .unwrap_or_default(),
                icon: contact.as_ref().and_then(|entry| entry.icon),
                notify: contact.as_ref().and_then(|entry| entry.notify),
                groups: contact.as_ref().and_then(|entry| entry.groups.clone()),
                contact,
                catalog: snapshot.groups,
                feedback: String::new(),
            });
        }
        Action::Cancel(serial) => {
            if state
                .draft
                .as_ref()
                .is_some_and(|draft| draft.serial == serial)
            {
                state.draft = None;
                state.pending = None;
            }
        }
        Action::Save(serial) => prepare(&mut state, &fields, serial),
        Action::Remove(entry) => {
            if state.draft.as_ref().is_some_and(|draft| {
                current(&owner, draft)
                    && draft
                        .contact
                        .as_ref()
                        .is_some_and(|contact| contact.entry == entry)
            }) {
                play.link.vip_request(Intent::Remove(entry));
                if play.link.vip_panel().pending {
                    state.draft = None;
                    state.pending = None;
                }
            }
        }
        Action::Icon(serial, delta) => {
            if let Some(draft) = state
                .draft
                .as_mut()
                .filter(|draft| draft.serial == serial && current(&owner, draft))
                && let Some(icon) = draft.icon.as_mut()
            {
                *icon = icon.saturating_add_signed(delta).min(19);
            }
        }
        Action::Notify(serial) => {
            if let Some(draft) = state
                .draft
                .as_mut()
                .filter(|draft| draft.serial == serial && current(&owner, draft))
                && let Some(notify) = draft.notify.as_mut()
            {
                *notify = !*notify;
            }
        }
        Action::Group(serial, group) => {
            if let Some(draft) = state
                .draft
                .as_mut()
                .filter(|draft| draft.serial == serial && current(&owner, draft))
                && draft.catalog.iter().any(|row| row.group == group)
                && let Some(groups) = draft.groups.as_mut()
            {
                if groups.contains(&group) {
                    groups.retain(|entry| *entry != group);
                } else if groups.len() < 8 {
                    groups.push(group);
                }
            }
        }
    }
}
fn prepare(state: &mut State, fields: &Query<(Entity, &Field)>, serial: u64) {
    let Some(draft) = state.draft.as_ref().filter(|draft| draft.serial == serial) else {
        return;
    };
    let Some((field, _)) = fields.iter().find(|(_, field)| field.0 == serial) else {
        return;
    };
    state.pending = Some(Pending {
        generation: draft.generation,
        revision: draft.revision,
        serial,
        field,
    });
}
pub(super) fn keyboard(
    mut keys: MessageReader<KeyboardInput>,
    focus: Res<InputFocus>,
    guard: ModalGuard,
    fields: Query<(Entity, &Field)>,
    dialogs: Query<(Entity, &Dialog)>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Some(draft) = &state.draft else {
        keys.clear();
        return;
    };
    let serial = draft.serial;
    if !current(&owner, draft)
        || !dialogs
            .iter()
            .any(|(entity, dialog)| dialog.0 == serial && guard.allows(entity))
    {
        keys.clear();
        return;
    }
    for key in keys.read() {
        if key.state != ButtonState::Pressed || key.repeat {
            continue;
        }
        match key.key_code {
            KeyCode::Escape => {
                state.draft = None;
                state.pending = None;
            }
            KeyCode::Enter | KeyCode::NumpadEnter
                if focus.get().is_some_and(|entity| fields.get(entity).is_ok()) =>
            {
                prepare(&mut state, &fields, serial);
            }
            _ => {}
        }
    }
}
pub(super) fn finish(
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    fields: Query<(&Field, &EditableText)>,
    mut state: ResMut<State>,
) {
    let Some(pending) = state.pending.take() else {
        return;
    };
    let Some(draft) = state.draft.as_mut().filter(|draft| {
        draft.serial == pending.serial
            && draft.generation == pending.generation
            && draft.revision == pending.revision
            && current(&owner, draft)
    }) else {
        return;
    };
    let Ok((field, editor)) = fields.get(pending.field) else {
        return;
    };
    if field.0 != draft.serial || !guard.allows(pending.field) {
        return;
    }
    if editor.pending_paste.is_some()
        || editor
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
    {
        state.pending = Some(pending);
        return;
    }
    draft.text = editor.value().to_string();
    if draft.text.chars().count() > 128 {
        draft.feedback = "The maximum length is 128 characters.".into();
        return;
    }
    let intent = if let Some(contact) = &draft.contact {
        if !contact.editable() {
            return;
        }
        let (Some(icon), Some(notify), Some(groups)) =
            (draft.icon, draft.notify, draft.groups.clone())
        else {
            return;
        };
        Intent::Edit {
            number: contact.entry,
            text: draft.text.clone(),
            icon,
            notify,
            groups,
        }
    } else {
        Intent::Add(draft.text.clone())
    };
    let Some(play) = &owner.play else { return };
    play.link.vip_request(intent);
    let accepted = play.link.vip_panel();
    if accepted.pending {
        state.draft = None;
    } else if let Some(draft) = state.draft.as_mut() {
        draft.feedback = accepted
            .feedback
            .unwrap_or_else(|| "VIP request was not queued.".into());
    }
}

pub(super) fn toggle(
    event: On<bevy::ui_widgets::ValueChange<bool>>,
    guard: ModalGuard,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if !guard.allows(event.source) || !available(&owner) {
        return;
    }
    let Ok(control) = controls.get(event.source) else {
        return;
    };
    if control.generation != owner.generation() {
        return;
    }
    let Some(draft) = state
        .draft
        .as_mut()
        .filter(|draft| current(&owner, draft) && draft.revision == control.revision)
    else {
        return;
    };
    match control.command {
        Action::Notify(serial) if serial == draft.serial => {
            if let Some(notify) = draft.notify.as_mut() {
                *notify = event.value;
            }
        }
        Action::Group(serial, group)
            if serial == draft.serial && draft.catalog.iter().any(|row| row.group == group) =>
        {
            if let Some(groups) = draft.groups.as_mut() {
                if !event.value {
                    groups.retain(|entry| *entry != group);
                } else if !groups.contains(&group) && groups.len() < 8 {
                    groups.push(group);
                }
            }
        }
        _ => {}
    }
}
