//! Offline training for several characters: rows are marked in the picker and sent as one batch
//! through the existing selector callback.
use super::{Choice, PickerState, Skin, add_button};
use crate::character_selection::{CharacterRow, SelectionAction, TrainingSelection};
use bevy::prelude::*;

/// The row label suffix of a character marked for training.
pub(super) fn suffix(state: &PickerState, index: usize) -> &'static str {
    if state.marked.iter().any(|(row, _)| *row == index) {
        " (training)"
    } else {
        ""
    }
}

/// The Mark and Train marked buttons for the current selection and batch.
pub(super) fn buttons(
    commands: &mut Commands,
    skin: &Skin,
    request: u64,
    state: &PickerState,
    rows: &[CharacterRow],
) -> [Entity; 2] {
    let selected = state.selected.and_then(|index| rows.get(index));
    let marked = state
        .selected
        .is_some_and(|index| !suffix(state, index).is_empty());
    let mark = add_button(
        commands,
        skin,
        request,
        Choice::Mark,
        if marked {
            "Unmark training"
        } else {
            "Mark for training"
        },
        130.,
        selected.is_none_or(|row| !row.available),
    );
    let batch = add_button(
        commands,
        skin,
        request,
        Choice::TrainMarked,
        &format!("Train marked ({})", state.marked.len()),
        140.,
        state.marked.is_empty(),
    );
    [mark, batch]
}

/// Adds the selected available row with its chosen channel to the batch, or removes it.
pub(super) fn toggle(state: &mut PickerState, rows: &[CharacterRow]) {
    let Some(index) = state
        .selected
        .filter(|index| rows.get(*index).is_some_and(|row| row.available))
    else {
        return;
    };
    if let Some(at) = state.marked.iter().position(|(row, _)| *row == index) {
        state.marked.remove(at);
    } else {
        let channel = state.channel;
        state.marked.push((index, channel));
    }
}

/// The marked available characters in marking order; nothing when none remain.
pub(super) fn batch(state: &PickerState, rows: &[CharacterRow]) -> Option<SelectionAction> {
    let characters: Vec<_> = state
        .marked
        .iter()
        .filter_map(|(index, channel)| {
            let row = rows.get(*index).filter(|row| row.available)?;
            Some(TrainingSelection {
                character_id: row.character.character_id.clone(),
                channel_id: channel
                    .and_then(|channel| row.channels.get(channel))
                    .map(|channel| channel.id.clone()),
            })
        })
        .collect();
    (!characters.is_empty()).then_some(SelectionAction::TrainMany { characters })
}
