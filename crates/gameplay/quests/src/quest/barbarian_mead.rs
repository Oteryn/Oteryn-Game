//! Replay-bound Barbarian Test booze outcomes, Crystal action_mead.lua:6-29.
use super::{QuestRefusal, world_callbacks::Callback};

pub const KEY: &str = "oteryn:quest-callback/barbarian-test-mead";
pub const QUEST: &str = "oteryn:quest.the_ice_islands_quest";
pub const TRACKS: [&str; 4] = [
    "oteryn:quest-progress/quest/u8_0/barbarian_test/questline",
    "oteryn:quest-progress/quest/u8_0/barbarian_test/mission01",
    "oteryn:quest-progress/quest/u8_0/barbarian_test/mead_success_sips",
    "oteryn:quest-progress/quest/u8_0/barbarian_test/mead_total_sips",
];
pub const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/barbarian_test/action_mead.lua:";

pub fn callback() -> Result<Callback, String> {
    let storage: Vec<_> = TRACKS.into_iter().zip(RANGES).map(|(track,(minimum,maximum))|
        serde_json::json!({"track":track,"minimum":minimum,"maximum":maximum,"set":0,"preserve":true})
    ).collect();
    super::world_callbacks::parse(&serde_json::json!({"schema":1,"callbacks":[{
        "key":KEY,"quest":QUEST,"trigger":"use",
        "selector":{"position":{"x":32201,"y":31154,"floor":-7},"unique":null,"action":null,"appearance":null},
        "lever":null,"reversible":false,"passages":[],"teleport":null,
        "storage":storage,"offerings":[],"item_on_target":null,
        "native_rule":{"kind":"barbarian_mead"},
        "return_on_failure":false,"failure_message":null,"sources":[format!("{SOURCE}1-35")]
    }]}).to_string())?.pop().ok_or_else(|| "missing mead callback".into())
}

pub const RANGES: [(i64, i64); 4] = [(-1, 8), (-1, 3), (0, i32::MAX as i64), (0, i32::MAX as i64)];

pub fn message(changes: &[super::QuestTrackChange]) -> Option<&'static str> {
    let line = changes.iter().find(|c| c.track == TRACKS[0])?;
    let sips = changes.iter().find(|c| c.track == TRACKS[2])?;
    if line.after == 1 {
        Some("Ask Sven for another round.")
    } else if line.after == 3 {
        Some("10 sips in a row. Yeah!")
    } else if sips.after > sips.before {
        Some("The world seems to spin but you manage to stay on your feet.")
    } else {
        Some("The mead was too strong. You passed out for a moment.")
    }
}

/// Locked source snapshot: questline, mission01, successful sips, total sips.
/// Roll is supplied by the durable owner, never by a gameplay client.
pub fn apply(mut state: [i64; 4], roll: u8) -> Result<[i64; 4], QuestRefusal> {
    if !(1..=5).contains(&roll) {
        return Err(QuestRefusal::NotSupported);
    }
    if state[0] == 2 && state[3] <= 20 {
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
    } else {
        return Err(QuestRefusal::StageMismatch);
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
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
            assert_eq!(changes.len(), 4);
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
