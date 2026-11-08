//! Children of the Revolution: Crystal 9f5a72c actions_mission4_levers_puzzle.lua:1-70.
//! OTS_HYPOTHESIS_ONLY; exact four-cell state, five switches and completion branches.
use super::world_callbacks::{Callback, Position, Storage};

pub const CELLS: [Position; 4] = [
    Position {
        x: 33352,
        y: 31126,
        floor: -5,
    },
    Position {
        x: 33353,
        y: 31126,
        floor: -5,
    },
    Position {
        x: 33354,
        y: 31126,
        floor: -5,
    },
    Position {
        x: 33355,
        y: 31126,
        floor: -5,
    },
];
pub const PAIRS: [[u16; 2]; 4] = [[9935, 9933], [9936, 9937], [9939, 9934], [9929, 9938]];
pub const SOLVED: [u16; 4] = [9933, 9936, 9939, 9938];
pub const SPEECH: &str =
    "After a cracking noise a deep humming suddenly starts from somewhere below.";
const MASKS: [u8; 5] = [0b1011, 0b1011, 0b1100, 0b0110, 0b0101];

pub fn binding(key: &str) -> Option<(usize, bool)> {
    let (index, branch) = key
        .strip_prefix("oteryn:quest-callback/qn1-children-lever-")?
        .split_once('/')?;
    let index = index.parse::<usize>().ok().filter(|index| *index < 5)?;
    Some((
        index,
        match branch {
            "ordinary" => false,
            "solved" => true,
            _ => return None,
        },
    ))
}

pub fn toggle(index: usize, mut state: [u16; 4]) -> Option<[u16; 4]> {
    let mask = *MASKS.get(index)?;
    for (cell, pair) in PAIRS.iter().enumerate() {
        let side = pair.iter().position(|id| *id == state[cell])?;
        if mask & (1 << cell) != 0 {
            state[cell] = pair[1 - side];
        }
    }
    Some(state)
}

pub(crate) fn callbacks() -> Vec<Callback> {
    let mut callbacks = Vec::new();
    for index in 0..5_u16 {
        for solved in [false, true] {
            let mut callback = super::source_world_objects::uneasy_crystal();
            callback.key = format!(
                "oteryn:quest-callback/qn1-children-lever-{index}/{}",
                if solved { "solved" } else { "ordinary" }
            );
            callback.quest = "oteryn:quest.children_of_the_revolution_quest".into();
            callback.selector.native = None;
            callback.selector.action = None;
            callback.selector.appearance = Some(9125);
            callback.selector.position = Some(Position {
                x: 33349,
                y: 31123 + index,
                floor: -5,
            });
            callback.lever = Some([9125, 9126]);
            callback.reversible = true;
            callback.effects.transform = None;
            callback.effects.reset_after_ms = Some(5000);
            callback.storage = vec![Storage {
                track: "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/questline"
                    .into(),
                minimum: 14,
                maximum: 14,
                set: if solved { 17 } else { 14 },
                preserve: !solved,
                increment: false,
                at_least: false,
            }];
            if solved {
                callback.effects.player_speech = Some(SPEECH.into());
                callback.storage.push(Storage {
                    track: "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/mission04"
                        .into(),
                    minimum: -1,
                    maximum: 5,
                    set: 5,
                    preserve: false,
                    increment: false,
                    at_least: false,
                });
            }
            callback.sources = vec!["crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/scripts/quests/children_of_the_revolution/actions_mission4_levers_puzzle.lua:1-70".into()];
            callbacks.push(callback);
        }
    }
    callbacks
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn children_all_five_levers_toggle_exact_cells_and_are_involutions() -> Result<(), String> {
        for (index, mask) in MASKS.iter().enumerate() {
            for bits in 0..16_u8 {
                let state =
                    std::array::from_fn(|cell| PAIRS[cell][usize::from(bits & (1 << cell) != 0)]);
                let after = toggle(index, state).ok_or("toggle")?;
                for cell in 0..4 {
                    assert_eq!(after[cell] != state[cell], *mask & (1 << cell) != 0);
                }
                assert_eq!(toggle(index, after), Some(state));
            }
        }
        assert_eq!(toggle(0, [0; 4]), None);
        assert_eq!(toggle(5, SOLVED), None);
        Ok(())
    }
}

#[cfg(test)]
mod catalogue_tests {
    use super::*;
    #[test]
    fn children_completion_commits_both_source_tracks_only_for_solved_branch() -> Result<(), String>
    {
        let loaded =
            crate::quest::loader::load_embedded_quest_state("qn1").map_err(|e| format!("{e:?}"))?;
        let catalogue = loaded.catalogue();
        for callback in callbacks() {
            let (_, solved) = binding(&callback.key).ok_or("binding")?;
            let transition = catalogue.transition(&callback.key).ok_or("transition")?;
            let stored = std::collections::BTreeMap::from([
                (callback.storage[0].track.clone(), 14),
                (
                    "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/mission04".into(),
                    4,
                ),
            ]);
            let changes = catalogue
                .evaluate(transition, &stored, 0)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(changes[0].after, if solved { 17 } else { 14 });
            if solved {
                assert_eq!(changes[1].after, 5);
            }
        }
        Ok(())
    }
}
