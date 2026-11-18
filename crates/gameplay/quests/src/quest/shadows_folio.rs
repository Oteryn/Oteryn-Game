//! Crystal actions_mission09.lua:6-54, evaluated against the locked quest tracks.
use super::{
    QuestRefusal, QuestTrackChange,
    world_callbacks::{Callback, Position},
};

pub const QUEST: &str = "oteryn:quest.shadows_of_yalahar_quest";
pub const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/shadows_of_yalahar_quest/actions_mission09.lua:";
pub const ACTION: u16 = 4953;
pub const KEY: &str = "oteryn:quest-callback/shadows-of-yalahar-folio";
pub const MISSION: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/mission09";
pub const STREAK: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/mission09_books_success";
pub const HEADACHE: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/mission09_headache_until";
pub const OUTCOME: &str = "oteryn:quest-progress/shadows-of-yalahar/folio-outcome";
pub const TRACKS: [&str; 4] = [MISSION, STREAK, HEADACHE, OUTCOME];
pub const RANGES: [(i64, i64); 4] = [(1, 1), (-1, 8), (-1, i64::MAX), (0, 6)];
pub const RELOCATION: Position = Position {
    x: 33324,
    y: 32173,
    floor: -6,
};

pub fn callback() -> Result<Callback, String> {
    let storage: Vec<_> = TRACKS.into_iter().zip(RANGES).map(|(track, (minimum, maximum))| {
        serde_json::json!({"track":track,"minimum":minimum,"maximum":maximum,"set":0,"preserve":true})
    }).collect();
    super::world_callbacks::parse(
        &serde_json::json!({"schema":1,"callbacks":[{
            "key":KEY,"quest":QUEST,"trigger":"use",
            "selector":{"action":ACTION},
            "lever":null,"reversible":false,"passages":[],"teleport":null,
            "storage":storage,"offerings":[],"item_on_target":null,
            "native_rule":{"kind":"shadows_folio"},
            "return_on_failure":false,"failure_message":null,"sources":[format!("{SOURCE}6-54")]
        }]})
        .to_string(),
    )?
    .pop()
    .ok_or_else(|| "missing Shadows folio callback".into())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Outcome {
    Relocation = 1,
    Headache = 2,
    WarGolem = 3,
    FireDevil = 4,
    EnragedBookworm = 5,
    Success = 6,
}

impl Outcome {
    pub fn from_dice(read: i16, monster: i16) -> Self {
        match (read, monster) {
            (1, _) => Self::Relocation,
            (2..=20, 1..=5) => Self::WarGolem,
            (2..=20, 6..=10) => Self::FireDevil,
            (2..=20, 11..=20) => Self::EnragedBookworm,
            (2..=20, _) => Self::Headache,
            _ => Self::Success,
        }
    }

    fn from_code(code: i64) -> Option<Self> {
        match code {
            1 => Some(Self::Relocation),
            2 => Some(Self::Headache),
            3 => Some(Self::WarGolem),
            4 => Some(Self::FireDevil),
            5 => Some(Self::EnragedBookworm),
            6 => Some(Self::Success),
            _ => None,
        }
    }

    pub fn committed(changes: &[QuestTrackChange]) -> Option<Self> {
        Self::from_code(changes.iter().find(|change| change.track == OUTCOME)?.after)
    }

    pub fn creature(self) -> Option<&'static str> {
        match self {
            Self::WarGolem => Some("oteryn:creature.war_golem"),
            Self::FireDevil => Some("oteryn:creature.fire_devil"),
            Self::EnragedBookworm => Some("oteryn:creature.enraged_bookworm"),
            _ => None,
        }
    }
}

pub fn eligible(mission: i64, streak: i64, headache: i64, now: i64) -> bool {
    mission == 1 && streak < 8 && headache <= now
}

pub(super) fn apply(
    mission: i64,
    streak: i64,
    headache: i64,
    now: i64,
    roll: Option<u8>,
) -> Result<[i64; 4], QuestRefusal> {
    if !eligible(mission, streak, headache, now) {
        return Err(QuestRefusal::StageMismatch);
    }
    let outcome = roll
        .and_then(|roll| Outcome::from_code(i64::from(roll)))
        .ok_or(QuestRefusal::NotSupported)?;
    let (streak, deadline) = match outcome {
        Outcome::Relocation => (0, headache),
        Outcome::Success => (streak.max(0) + 1, headache),
        _ => (0, now.checked_add(60).ok_or(QuestRefusal::OutOfRange)?),
    };
    Ok([mission, streak, deadline, i64::from(outcome as u8)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadows_folio_source_dice_and_deadline() {
        assert_eq!(Outcome::from_dice(1, 1), Outcome::Relocation);
        assert_eq!(Outcome::from_dice(2, 5), Outcome::WarGolem);
        assert_eq!(Outcome::from_dice(20, 6), Outcome::FireDevil);
        assert_eq!(Outcome::from_dice(20, 11), Outcome::EnragedBookworm);
        assert_eq!(Outcome::from_dice(20, 21), Outcome::Headache);
        assert_eq!(Outcome::from_dice(21, 1), Outcome::Success);
        let mut counts = [0; 6];
        for roll in 0..10_000 {
            let outcome = Outcome::from_dice(roll / 100 + 1, roll % 100 + 1);
            counts[usize::from(outcome as u8) - 1] += 1;
        }
        assert_eq!(counts, [100, 1520, 95, 95, 190, 8000]);
        assert_eq!(apply(1, 7, 100, 100, Some(1)), Ok([1, 0, 100, 1]));
        assert_eq!(apply(1, 7, 100, 100, Some(3)), Ok([1, 0, 160, 3]));
        assert_eq!(apply(1, 7, 100, 100, Some(6)), Ok([1, 8, 100, 6]));
        assert_eq!(
            apply(1, 7, 101, 100, None),
            Err(QuestRefusal::StageMismatch)
        );
        assert_eq!(
            apply(1, 8, 100, 100, None),
            Err(QuestRefusal::StageMismatch)
        );
        assert_eq!(
            apply(2, 7, 100, 100, None),
            Err(QuestRefusal::StageMismatch)
        );
    }

    #[test]
    fn shadows_folio_is_registered_as_durable_action_use() -> Result<(), String> {
        let callback = callback()?;
        assert!(callback.requires_durable_commit());
        let loaded = super::super::loader::load_embedded_quest_state("shadows-folio")
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            loaded.catalogue().transition(KEY),
            Some(&super::super::world_callbacks::primary_transition(
                &callback
            )?)
        );
        assert!(super::super::world_callbacks::load_for(loaded.catalogue())?.contains(&callback));
        Ok(())
    }
}
