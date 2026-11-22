//! The baited lookout telescope from Crystal actions_bait.lua, lines 20–83.
use super::QuestRefusal;
use super::world_callbacks::{Callback, NativeSelector, Position};

pub const KEY: &str = "oteryn:quest-callback/sea-serpent-telescope";
// The imported Captain Haba and crane storage belongs to the Tibia Tales group.
pub const QUEST: &str = "oteryn:quest.tibia_tales";
pub const DRAW: &str = "oteryn:quest-progress/native/the_hunt_for_the_sea_serpent/telescope_draw";
pub const TRACKS: [&str; 6] = [
    "oteryn:quest-progress/quest/u8_2/the_hunt_for_the_sea_serpent/quest_line",
    "oteryn:quest-progress/quest/u8_2/the_hunt_for_the_sea_serpent/bait",
    "oteryn:quest-progress/quest/u8_2/the_hunt_for_the_sea_serpent/success_switch",
    "oteryn:quest-progress/quest/u8_2/the_hunt_for_the_sea_serpent/direction",
    "oteryn:quest-progress/quest/u8_2/the_hunt_for_the_sea_serpent/access",
    DRAW,
];
pub const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_hunt_for_the_sea_serpent/actions_bait.lua:";
pub const NATIVE: NativeSelector = NativeSelector {
    position: Position {
        x: 31942,
        y: 31045,
        floor: -2,
    },
    ordinal: 2,
    appearance: 938,
};
pub const RANGES: [(i64, i64); 6] = [
    (0, 2),
    (0, 1),
    (0, 1),
    (0, i32::MAX as i64),
    (0, 1),
    (0, 240),
];

const APPEARANCE_FACES: u8 = 10;
const BAIT_FACES: u8 = 2;
const PHASE_FACES: u8 = 4;
const DIRECTION_FACES: u8 = 3;

pub fn callback() -> Result<Callback, String> {
    let storage: Vec<_> = TRACKS.into_iter().zip(RANGES).map(|(track, (minimum, maximum))|
        serde_json::json!({"track":track,"minimum":minimum,"maximum":maximum,"set":0,"preserve":true})
    ).collect();
    super::world_callbacks::parse(&serde_json::json!({"schema":1,"callbacks":[{
        "key":KEY,"quest":QUEST,"trigger":"use",
        "selector":{"position":NATIVE.position,"unique":null,"action":null,"appearance":null,"native":NATIVE},
        "lever":null,"reversible":false,"passages":[],"teleport":null,
        "storage":storage,"offerings":[],"item_on_target":null,
        "native_rule":{"kind":"sea_serpent_telescope"},
        "return_on_failure":false,"failure_message":null,"sources":[format!("{SOURCE}20-83")]
    }]}).to_string())?.pop().ok_or_else(|| "missing telescope callback".into())
}

fn appearance_faces(switch: i64) -> u8 {
    APPEARANCE_FACES - u8::from(switch != 1)
}

pub(super) fn draw_faces(quest_line: i64, bait: i64, switch: i64, direction: i64) -> Option<u8> {
    (quest_line == 1 && bait == 1 && direction <= 0)
        .then(|| appearance_faces(switch) * BAIT_FACES * PHASE_FACES * DIRECTION_FACES)
}

struct Sighting {
    appearance: u8,
    lost_bait: bool,
    phase: usize,
    direction: usize,
}

impl Sighting {
    fn from_roll(roll: u8, switch: i64) -> Option<Self> {
        let appearance_faces = appearance_faces(switch);
        let max = appearance_faces * BAIT_FACES * PHASE_FACES * DIRECTION_FACES;
        let index = roll.checked_sub(1).filter(|index| *index < max)?;
        let detail = index / appearance_faces;
        Some(Self {
            appearance: 1 + index % appearance_faces,
            lost_bait: detail % BAIT_FACES == 1,
            phase: usize::from(detail / BAIT_FACES % PHASE_FACES),
            direction: usize::from(detail / (BAIT_FACES * PHASE_FACES)),
        })
    }
}

pub(super) fn apply(mut state: [i64; 6], roll: Option<u8>) -> Result<[i64; 6], QuestRefusal> {
    let [quest_line, bait, switch, direction, _, _] = state;
    if quest_line != 1 {
        return Err(QuestRefusal::StageMismatch);
    }
    state[5] = 0;
    if bait < 1 {
        return Ok(state);
    }
    if direction > 0 {
        state[2] = 0;
        state[3] = 0;
        return Ok(state);
    }
    let roll = roll.ok_or(QuestRefusal::NotSupported)?;
    let sighting = Sighting::from_roll(roll, switch).ok_or(QuestRefusal::NotSupported)?;
    state[5] = i64::from(roll);
    match sighting.appearance {
        5..=9 => {
            state[3] = if sighting.appearance == 9 {
                4
            } else {
                sighting.direction as i64 + 1
            };
            if sighting.lost_bait {
                state[1] = 0;
            }
        }
        10 => {
            state[0] = 2;
            state[4] = 1;
        }
        _ => {}
    }
    Ok(state)
}

pub(super) fn message(changes: &[super::QuestTrackChange]) -> Option<&'static str> {
    let before = |key| changes.iter().find(|c| c.track == key).map(|c| c.before);
    if before(TRACKS[1])? < 1 {
        return Some("You should set a new bait first.");
    }
    if before(TRACKS[3])? > 0 {
        return Some("Oops! You lost it.");
    }
    let roll = u8::try_from(changes.iter().find(|c| c.track == TRACKS[5])?.after).ok()?;
    let sighting = Sighting::from_roll(roll, before(TRACKS[2])?)?;
    Some(match sighting.appearance {
        1..=4 => "You see water as far as the eye can reach. No sea serpent in sight.",
        9 => "It's getting away! You should tell the captain to gain speed!",
        10 => "There are multiple shadows under the surface. This has to be the right location.",
        _ => [
            [
                "You see dot under the surface straight ahead of you.",
                "You see dot under the surface. It is to the starboard side.",
                "You see dot under the surface. It is on the larboard side.",
            ],
            [
                "You see a dot under the surface straight ahead of you.",
                "You see a dot under the surface. It is to the starboard side.",
                "You see a dot under the surface. It is on the larboard side.",
            ],
            [
                "You see a shadow under the surface straight ahead of you.",
                "You see a shadow under the surface. It is to the starboard side.",
                "You see a shadow under the surface. It is on the larboard side.",
            ],
            [
                "You see a huge shadow under the surface straight ahead of you.",
                "You see a huge shadow under the surface. It is to the starboard side.",
                "You see a huge shadow under the surface. It is on the larboard side.",
            ],
        ][sighting.phase][sighting.direction],
    })
}
