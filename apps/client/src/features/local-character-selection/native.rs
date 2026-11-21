//! The selector owns its HWNDs on the blocking form thread.
// Native controls require Win32 FFI, confined to this module.
#![allow(unsafe_code)]

// Win32 list-box messages, kept at the native control seam for headless tests.
fn set_preselection(index: usize, mut send: impl FnMut(u32, usize, isize)) {
    send(0x0185, 1, index as isize); // LB_SETSEL
    send(0x019e, index, 0); // LB_SETCARETINDEX
    send(0x0197, index, 0); // LB_SETTOPINDEX
}

fn training_selections(
    rows: &[super::CharacterRow],
    mut selected: impl FnMut(usize) -> bool,
) -> Vec<super::TrainingSelection> {
    rows.iter()
        .enumerate()
        .filter(|(index, row)| row.available && selected(*index))
        .map(|(_, row)| super::TrainingSelection {
            character_id: row.character.character_id.clone(),
            channel_id: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preselection_sets_selection_caret_and_scroll_to_the_same_row() {
        let mut messages = Vec::new();
        set_preselection(4, |message, w, l| messages.push((message, w, l)));
        assert_eq!(messages, [(0x0185, 1, 4), (0x019e, 4, 0), (0x0197, 4, 0)]);
    }
}

#[cfg(test)]
mod training_tests {
    use super::training_selections;
    use crate::character_selection::{CharacterRow, TrainingQueue};
    use oteryn_platform_client::native_login::{CharacterAvailability, OwnerCharacter};

    #[test]
    fn mixed_training_selection_starts_only_available_selected_characters() -> Result<(), String> {
        let rows: Vec<_> = [true, false, true]
            .into_iter()
            .enumerate()
            .map(|(index, available)| CharacterRow {
                character: OwnerCharacter {
                    character_id: format!("character-{index}"),
                    world_id: "world".into(),
                    name: format!("Character {index}"),
                    availability: if available {
                        CharacterAvailability::Available
                    } else {
                        CharacterAvailability::Unavailable
                    },
                },
                world_name: "World".into(),
                channels: Vec::new(),
                available,
            })
            .collect();
        let selections = training_selections(&rows, |index| index < 2);
        let queue = TrainingQueue::default();
        let first = queue
            .start(&rows, selections.clone())
            .ok_or("mixed selection rejected")?;
        assert_eq!(first.character_id, "character-0");
        assert_eq!(selections, [first]);
        assert!(training_selections(&rows, |index| index == 1).is_empty());
        Ok(())
    }
}
