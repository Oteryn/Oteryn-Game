//! Replay-bound Barbarian Test booze outcomes, Crystal action_mead.lua:6-29.
use super::{
    QuestRefusal,
    world_callbacks::{Callback, NativeSelector, Position},
};

pub const KEY: &str = "oteryn:quest-callback/barbarian-test-mead";
pub const QUEST: &str = "oteryn:quest.the_ice_islands_quest";
pub const TRACKS: [&str; 4] = [
    "oteryn:quest-progress/quest/u8_0/barbarian_test/questline",
    "oteryn:quest-progress/quest/u8_0/barbarian_test/mission01",
    "oteryn:quest-progress/quest/u8_0/barbarian_test/mead_success_sips",
    "oteryn:quest-progress/quest/u8_0/barbarian_test/mead_total_sips",
];
/// Internal receipt evidence, separate from the four source storage counters.
pub const DRAW: &str = "oteryn:quest-progress/native/barbarian_test/mead_draw";
pub const NATIVE_TRACKS: [&str; 5] = [TRACKS[0], TRACKS[1], TRACKS[2], TRACKS[3], DRAW];
pub const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/barbarian_test/action_mead.lua:";
/// Retained B3 source stack: floor408, wall2321, mead barrel7142.
pub const NATIVE: NativeSelector = NativeSelector {
    position: Position {
        x: 32201,
        y: 31154,
        floor: -7,
    },
    ordinal: 2,
    appearance: 7142,
};

pub fn callback() -> Result<Callback, String> {
    let storage: Vec<_> = NATIVE_TRACKS.into_iter().zip(RANGES).map(|(track,(minimum,maximum))|
        serde_json::json!({"track":track,"minimum":minimum,"maximum":maximum,"set":0,"preserve":true})
    ).collect();
    super::world_callbacks::parse(&serde_json::json!({"schema":1,"callbacks":[{
        "key":KEY,"quest":QUEST,"trigger":"use",
        "selector":{"position":NATIVE.position,"unique":null,"action":null,"appearance":null,"native":NATIVE},
        "lever":null,"reversible":false,"passages":[],"teleport":null,
        "storage":storage,"offerings":[],"item_on_target":null,
        "native_rule":{"kind":"barbarian_mead"},
        "return_on_failure":false,"failure_message":null,"sources":[format!("{SOURCE}1-35")]
    }]}).to_string())?.pop().ok_or_else(|| "missing mead callback".into())
}

pub const RANGES: [(i64, i64); 5] = [
    (-1, 8),
    (-1, 3),
    (-1, i32::MAX as i64),
    (0, i32::MAX as i64),
    (0, 5),
];

pub fn message(changes: &[super::QuestTrackChange]) -> Option<&'static str> {
    let line = changes.iter().find(|c| c.track == TRACKS[0])?;
    let draw = changes.iter().find(|c| c.track == DRAW)?;
    if line.after == 1 {
        Some("Ask Sven for another round.")
    } else if line.before >= 3 && line.after == line.before {
        Some("You already passed the test, no need to torture yourself anymore.")
    } else if line.after == 3 {
        Some("10 sips in a row. Yeah!")
    } else if draw.after > 1 {
        Some("The world seems to spin but you manage to stay on your feet.")
    } else {
        Some("The mead was too strong. You passed out for a moment.")
    }
}

/// Source randomness is needed only for an eligible sip, not round resets or passed tests.
pub fn needs_draw(questline: i64, total_sips: i64) -> bool {
    questline == 2 && total_sips <= 20
}

/// Locked source snapshot: questline, mission01, successful sips, total sips.
/// Roll is supplied by the durable owner, never by a gameplay client.
pub fn apply(mut state: [i64; 4], roll: u8) -> Result<[i64; 4], QuestRefusal> {
    if !(1..=5).contains(&roll) {
        return Err(QuestRefusal::NotSupported);
    }
    if needs_draw(state[0], state[3]) {
        state[2] = if roll > 1 {
            state[2].checked_add(1).ok_or(QuestRefusal::OutOfRange)?
        } else {
            0
        };
        if state[2] == 9 {
            state[0] = 3;
            state[1] = 3;
            return Ok(state);
        }
        state[3] = state[3].checked_add(1).ok_or(QuestRefusal::OutOfRange)?;
    } else if state[3] > 20 {
        state[0] = 1;
        state[1] = 1;
    } else if state[0] < 3 {
        return Err(QuestRefusal::StageMismatch);
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn m47_native_barrel_selector_is_closed() -> Result<(), String> {
        let callback = callback()?;
        let native = callback
            .selector
            .native
            .clone()
            .ok_or("missing native barrel selector")?;
        assert_eq!(
            (
                native.position.x,
                native.position.y,
                native.position.floor,
                native.ordinal,
                native.appearance
            ),
            (32201, 31154, -7, 2, 7142)
        );
        for (ordinal, appearance) in [(0, 7142), (1, 7142), (2, 111)] {
            let mut invalid = callback.clone();
            let binding = invalid.selector.native.as_mut().ok_or("native barrel")?;
            binding.ordinal = ordinal;
            binding.appearance = appearance;
            assert!(
                super::super::world_callbacks::parse(
                    &serde_json::json!({"schema":1,"callbacks":[invalid]}).to_string()
                )
                .is_err()
            );
        }
        Ok(())
    }
    #[test]
    fn m47_completed_mead_is_a_silent_progress_noop_with_source_feedback() -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("m47-passed")
            .map_err(|e| format!("{e:?}"))?;
        let catalogue = lowered.catalogue();
        let transition = catalogue.transition(KEY).ok_or("transition")?;
        let stored = NATIVE_TRACKS
            .into_iter()
            .zip([3, 3, 9, 17, 4])
            .map(|(key, value)| (key.to_owned(), value))
            .collect();
        let changes = catalogue
            .evaluate_with_roll(transition, &stored, 0, None)
            .map_err(|e| format!("{e:?}"))?;
        assert!(
            changes
                .iter()
                .filter(|c| c.track != DRAW)
                .all(|c| c.before == c.after)
        );
        assert_eq!(
            changes.iter().find(|c| c.track == DRAW).map(|c| c.after),
            Some(0)
        );
        assert_eq!(
            message(&changes),
            Some("You already passed the test, no need to torture yourself anymore.")
        );
        Ok(())
    }

    #[test]
    fn m47_first_sip_preserves_absence_and_committed_draw() -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("m47-first")
            .map_err(|e| format!("{e:?}"))?;
        let catalogue = lowered.catalogue();
        assert_eq!(catalogue.track(TRACKS[2]).ok_or("streak")?.initial, -1);
        let transition = catalogue.transition(KEY).ok_or("transition")?;
        let stored = TRACKS
            .into_iter()
            .zip([2, 2, -1, 0])
            .map(|(key, value)| (key.to_owned(), value))
            .collect();
        for roll in 1..=5 {
            let changes = catalogue
                .evaluate_with_roll(transition, &stored, 0, Some(roll))
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(
                changes
                    .iter()
                    .find(|c| c.track == "oteryn:quest-progress/native/barbarian_test/mead_draw")
                    .map(|c| c.after),
                Some(i64::from(roll))
            );
            assert_eq!(
                changes
                    .iter()
                    .find(|c| c.track == TRACKS[2])
                    .map(|c| c.after),
                Some(0)
            );
            assert_eq!(
                message(&changes),
                Some(if roll == 1 {
                    "The mead was too strong. You passed out for a moment."
                } else {
                    "The world seems to spin but you manage to stay on your feet."
                })
            );
        }
        Ok(())
    }

    #[test]
    fn m47_loaded_callback_requires_writer_roll_and_binds_all_tracks() -> Result<(), String> {
        let catalogue =
            super::super::loader::load_embedded_quest_state("m47").map_err(|e| format!("{e:?}"))?;
        let catalogue = catalogue.catalogue();
        let callback = super::super::world_callbacks::load()?
            .into_iter()
            .find(|c| c.key == KEY)
            .ok_or("M47 callback")?;
        let transition = catalogue.transition(KEY).ok_or("M47 transition")?;
        let stored = TRACKS
            .into_iter()
            .zip([2, 2, 8, 20])
            .map(|(k, v)| (k.to_owned(), v))
            .collect();
        assert_eq!(
            catalogue.evaluate(transition, &stored, 0),
            Err(QuestRefusal::NotSupported)
        );
        let mut invalid = callback.clone();
        invalid.selector.position.as_mut().ok_or("position")?.x += 1;
        assert!(
            super::super::world_callbacks::parse(
                &serde_json::json!({"schema":1,"callbacks":[invalid]}).to_string()
            )
            .is_err()
        );
        for roll in 1..=5 {
            let changes = catalogue
                .evaluate_with_roll(transition, &stored, 0, Some(roll))
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(changes.len(), 5);
            assert!(callback.committed_message(&changes).is_some());
            assert_eq!(changes[0].after, if roll == 1 { 2 } else { 3 });
            assert_eq!(changes[3].after, if roll == 1 { 21 } else { 20 });
        }
        Ok(())
    }

    #[test]
    fn m47_mead_source_boundaries() {
        assert_eq!(apply([2, 2, 8, 20], 5), Ok([3, 3, 9, 20]));
        assert_eq!(apply([2, 2, 8, 20], 1), Ok([2, 2, 0, 21]));
        assert_eq!(apply([2, 2, 0, 21], 5), Ok([1, 1, 0, 21]));
        for roll in 2..=5 {
            assert_eq!(apply([2, 2, -1, -1], roll), Ok([2, 2, 0, 0]));
        }
        assert!(apply([1, 1, 0, 0], 5).is_err());
        assert!(apply([2, 2, 0, 0], 0).is_err());
        assert!(apply([2, 2, 0, 0], 6).is_err());
    }
}
