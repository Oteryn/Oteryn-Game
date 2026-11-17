//! Crystal actions_bait.lua:37-83. The fenced quest writer supplies the draw.
use super::{
    QuestRefusal, QuestTrackChange,
    world_callbacks::{NativeSelector, Position},
};

// The imported Sea Serpent storage tracks belong to the Tibia Tales aggregate.
pub const QUEST: &str = "oteryn:quest.tibia_tales";
pub const KEY: &str = "oteryn:quest-callback/sea-serpent-telescope";
pub const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_hunt_for_the_sea_serpent/actions_bait.lua:";
pub const SELECTOR: NativeSelector = NativeSelector {
    position: Position {
        x: 31942,
        y: 31045,
        floor: -2,
    },
    ordinal: 2,
    appearance: 938,
};
pub const OUTCOME: &str = "oteryn:quest-progress/native/sea-serpent/telescope-outcome";
pub const TRACKS: [&str; 6] = [
    "oteryn:quest-progress/quest/u8_2/the_hunt_for_the_sea_serpent/quest_line",
    "oteryn:quest-progress/quest/u8_2/the_hunt_for_the_sea_serpent/bait",
    "oteryn:quest-progress/quest/u8_2/the_hunt_for_the_sea_serpent/direction",
    "oteryn:quest-progress/quest/u8_2/the_hunt_for_the_sea_serpent/success_switch",
    "oteryn:quest-progress/quest/u8_2/the_hunt_for_the_sea_serpent/access",
    OUTCOME,
];
pub const RANGES: [(i64, i64); 6] = [(1, 1), (0, 1), (0, 4), (0, 1), (0, 1), (0, 16)];

// Each appearance roll has 24 equally likely combinations of distance,
// direction and bait loss. Only an accurate report adds the winning face.
pub fn draw_count(bait: i64, direction: i64, success_switch: i64) -> Option<i16> {
    (bait == 1 && direction == 0).then_some(if success_switch == 1 { 240 } else { 216 })
}

pub fn apply(mut state: [i64; 6], draw: Option<u8>) -> Result<[i64; 6], QuestRefusal> {
    let [questline, bait, direction, success_switch, _, _] = state;
    if questline != 1 {
        return Err(QuestRefusal::StageMismatch);
    }
    state[5] = if bait == 0 {
        0
    } else if direction > 0 {
        state[2] = 0;
        state[3] = 0;
        1
    } else {
        let count =
            draw_count(bait, direction, success_switch).ok_or(QuestRefusal::NotSupported)?;
        let draw = i64::from(
            draw.filter(|value| i16::from(*value) < count)
                .ok_or(QuestRefusal::NotSupported)?,
        );
        match draw / 24 {
            0..=3 => 2,
            4..=7 => {
                let detail = draw % 24;
                state[2] = 1 + detail / 2 % 3;
                state[1] = 1 - detail % 2;
                3 + (detail / 6) * 3 + state[2] - 1
            }
            8 => {
                state[2] = 4;
                state[1] = 1 - draw % 2;
                15
            }
            9 => {
                state[0] = 2;
                state[4] = 1;
                16
            }
            _ => return Err(QuestRefusal::NotSupported),
        }
    };
    Ok(state)
}

const MESSAGES: [&str; 17] = [
    "You should set a new bait first.",
    "Oops! You lost it.",
    "You see water as far as the eye can reach. No sea serpent in sight.",
    "You see dot under the surface straight ahead of you.",
    "You see dot under the surface. It is to the starboard side.",
    "You see dot under the surface. It is on the larboard side.",
    "You see a dot under the surface straight ahead of you.",
    "You see a dot under the surface. It is to the starboard side.",
    "You see a dot under the surface. It is on the larboard side.",
    "You see a shadow under the surface straight ahead of you.",
    "You see a shadow under the surface. It is to the starboard side.",
    "You see a shadow under the surface. It is on the larboard side.",
    "You see a huge shadow under the surface straight ahead of you.",
    "You see a huge shadow under the surface. It is to the starboard side.",
    "You see a huge shadow under the surface. It is on the larboard side.",
    "It's getting away! You should tell the captain to gain speed!",
    "There are multiple shadows under the surface. This has to be the right location.",
];

pub(super) fn message(changes: &[QuestTrackChange]) -> Option<&'static str> {
    let outcome = changes.iter().find(|change| change.track == OUTCOME)?;
    MESSAGES.get(usize::try_from(outcome.after).ok()?).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sea_serpent_telescope_draws_match_crystal_odds_and_preserve_hunt_state()
    -> Result<(), QuestRefusal> {
        for (success_switch, count) in [(0, 216), (1, 240)] {
            let mut outcomes = [0; 17];
            let mut lost_bait = 0;
            for draw in 0..count {
                let after = apply([1, 1, 0, success_switch, 0, 0], Some(draw))?;
                outcomes[usize::try_from(after[5]).map_err(|_| QuestRefusal::OutOfRange)?] += 1;
                lost_bait += 1 - after[1];
                if after[5] == 16 {
                    assert_eq!((after[0], after[4]), (2, 1));
                } else {
                    assert_eq!((after[0], after[4]), (1, 0));
                }
            }
            assert_eq!(
                outcomes,
                [
                    0,
                    0,
                    96,
                    8,
                    8,
                    8,
                    8,
                    8,
                    8,
                    8,
                    8,
                    8,
                    8,
                    8,
                    8,
                    24,
                    24 * success_switch
                ]
            );
            assert_eq!(lost_bait, 60);
        }
        assert_eq!(apply([1, 1, 3, 1, 0, 9], None)?, [1, 1, 0, 0, 0, 1]);
        assert_eq!(apply([1, 0, 3, 1, 0, 9], None)?, [1, 0, 3, 1, 0, 0]);
        assert_eq!(
            apply([2, 1, 0, 1, 1, 16], None),
            Err(QuestRefusal::StageMismatch)
        );
        Ok(())
    }
}
