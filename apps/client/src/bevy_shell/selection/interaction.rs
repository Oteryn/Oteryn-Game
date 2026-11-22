//! Character picker input: button activation, Escape and row clicks.
use super::*;

pub(super) fn activate(
    event: On<Activate>,
    choices: Query<(&Target, Has<InteractionDisabled>)>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<PickerState>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok((target, disabled)) = choices.get(event.entity) else {
        return;
    };
    if disabled || state.request != Some(target.request) {
        return;
    }
    let Some(request) = owner
        .selection
        .pending()
        .filter(|request| request.id == target.request)
    else {
        return;
    };
    let action = match target.choice {
        Choice::Row(index) => {
            if request.rows.get(index).is_some_and(|row| row.available) {
                if state.selected != Some(index) {
                    state.channel = None;
                }
                state.selected = Some(index);
            }
            None
        }
        Choice::Channel => {
            if let Some(row) = state.selected.and_then(|index| request.rows.get(index)) {
                state.channel = match state.channel {
                    None if !row.channels.is_empty() => Some(0),
                    Some(index) if index + 1 < row.channels.len() => Some(index + 1),
                    _ => None,
                };
            }
            None
        }
        Choice::Play | Choice::Train => state
            .selected
            .and_then(|index| request.rows.get(index))
            .filter(|row| row.available)
            .map(|row| {
                let character_id = row.character.character_id.clone();
                let channel_id = state
                    .channel
                    .and_then(|index| row.channels.get(index))
                    .map(|channel| channel.id.clone());
                if matches!(target.choice, Choice::Train) {
                    return SelectionAction::Train {
                        character_id,
                        channel_id,
                    };
                }
                SelectionAction::Play {
                    character_id,
                    channel_id,
                }
            }),
        Choice::Mark => {
            training::toggle(&mut state, &request.rows);
            None
        }
        Choice::TrainMarked => training::batch(&state, &request.rows),
        Choice::Account => Some(SelectionAction::AccountServices),
        Choice::Create => Some(SelectionAction::Create),
        Choice::Refresh => Some(SelectionAction::Refresh),
        Choice::Cancel => Some(SelectionAction::Cancel),
    };
    if let Some(action) = action
        && (!matches!(action, SelectionAction::Cancel) || owner.prepare_picker_cancel())
    {
        owner.selection.respond(target.request, action);
    }
    state.revision = state.revision.wrapping_add(1);
}

/// Escape replies through the same request identity as the Cancel button.
pub(in crate::bevy_shell) fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    roots: Query<Entity, With<Picker>>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<PickerState>,
) {
    if !roots.iter().any(|entity| guard.allows(entity)) {
        keys.clear();
        return;
    }
    for key in keys
        .read()
        .filter(|key| key.state == bevy::input::ButtonState::Pressed && !key.repeat)
    {
        let Some(request) = owner
            .selection
            .pending()
            .filter(|request| state.request == Some(request.id))
        else {
            continue;
        };
        if key.key_code == KeyCode::Escape {
            let id = request.id;
            if owner.prepare_picker_cancel() {
                owner.selection.respond(id, SelectionAction::Cancel);
            }
            continue;
        }
        let forward = match key.key_code {
            KeyCode::ArrowDown => true,
            KeyCode::ArrowUp => false,
            _ => continue,
        };
        let selected = state
            .selected
            .unwrap_or(if forward { 0 } else { request.rows.len() });
        let next = if forward {
            request
                .rows
                .iter()
                .enumerate()
                .skip(state.selected.map_or(0, |index| index.saturating_add(1)))
                .find_map(|(index, row)| row.available.then_some(index))
        } else {
            request
                .rows
                .iter()
                .enumerate()
                .take(selected)
                .rev()
                .find_map(|(index, row)| row.available.then_some(index))
        };
        if let Some(next) = next {
            state.selected = Some(next);
            state.reveal_selection = true;
            state.channel = None;
            state.last_click = None;
            state.revision = state.revision.wrapping_add(1);
        }
    }
}

pub(super) fn row_click(
    event: On<PointerClick>,
    targets: Query<(&Target, Has<InteractionDisabled>)>,
    guard: ModalGuard,
    settings: Option<Res<bevy::picking::PickingSettings>>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<PickerState>,
) {
    if event.button != bevy::picking::pointer::PointerButton::Primary || !guard.allows(event.entity)
    {
        return;
    }
    let Ok((target, disabled)) = targets.get(event.entity) else {
        return;
    };
    let Choice::Row(index) = target.choice else {
        return;
    };
    if disabled || state.request != Some(target.request) {
        return;
    }
    let Some(request) = owner
        .selection
        .pending()
        .filter(|request| request.id == target.request)
    else {
        return;
    };
    let Some(row) = request.rows.get(index).filter(|row| row.available) else {
        return;
    };
    let interval = settings.as_ref().map_or_else(
        || bevy::picking::PickingSettings::default().multi_click_interval,
        |settings| settings.multi_click_interval,
    );
    let now = std::time::Instant::now();
    let double = state.last_click.is_some_and(|(request, previous, at)| {
        request == target.request && previous == index && now.duration_since(at) <= interval
    });
    state.last_click = Some((target.request, index, now));
    if double || event.count >= 2 {
        let action = SelectionAction::Play {
            character_id: row.character.character_id.clone(),
            channel_id: state
                .channel
                .and_then(|channel| row.channels.get(channel))
                .map(|channel| channel.id.clone()),
        };
        owner.selection.respond(target.request, action);
        state.last_click = None;
    }
}
