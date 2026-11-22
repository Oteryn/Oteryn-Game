//! Crystal 201189f Dreamer's Challenge physical puzzle controls.
use super::world_callbacks::{Callback, Position, Selector, Trigger};

const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dreamers_challenge_quest/movement_riddle.lua:1-59";
const ROW: &str = "oteryn:quest-callback/dreamers-challenge/colour-row";
const COLUMN: &str = "oteryn:quest-callback/dreamers-challenge/colour-column";
pub const COLOUR_EXIT: &str = "oteryn:quest-callback/dreamers-challenge/colour-exit";
pub const WALL_EXIT: &str = "oteryn:quest-callback/dreamers-challenge/wall-exit";
pub const BOARD_EXIT: &str = "oteryn:quest-callback/dreamers-challenge/board-exit";

pub const PILLOWS: [u16; 5] = [2394, 2395, 2396, 2397, 900];
pub const WALL_CONTROLS: [(u16, u16); 4] = [
    (32763, 32291),
    (32761, 32291),
    (32763, 32289),
    (32761, 32289),
];
pub const TOKENS: [(u16, u16, u16); 9] = [
    (32845, 32264, 3548),
    (32843, 32266, 3548),
    (32843, 32268, 3548),
    (32845, 32268, 3548),
    (32844, 32267, 3548),
    (32840, 32269, 3548),
    (32841, 32269, 3547),
    (32840, 32268, 3547),
    (32842, 32267, 3547),
];

pub(crate) fn colour_controls() -> impl Iterator<Item = Callback> {
    [(ROW, 2245), (COLUMN, 2246)]
        .into_iter()
        .flat_map(|(key, action)| {
            [Trigger::StepIn, Trigger::StepOut]
                .into_iter()
                .map(move |trigger| {
                    let mut callback = super::source_world_objects::uneasy_crystal();
                    callback.key = if trigger == Trigger::StepOut {
                        format!("{key}-release")
                    } else {
                        key.into()
                    };
                    callback.quest = super::dreamer_fire_bug::QUEST.into();
                    callback.trigger = trigger;
                    callback.selector = Selector {
                        action: Some(action),
                        position_aliases: Vec::new(),
                        ordinal: None,
                        native: None,
                        position: None,
                        unique: None,
                        appearance: None,
                    };
                    callback.storage.clear();
                    callback.teleport = None;
                    callback.effects = Default::default();
                    callback.effects.transform =
                        Some(if trigger == Trigger::StepIn { 430 } else { 431 });
                    callback.return_on_failure = false;
                    callback.sources = vec![SOURCE.into()];
                    callback
                })
        })
}

pub fn shift_line(callback: &Callback, control: Position) -> Option<[Position; 6]> {
    let row = match callback.key.as_str() {
        ROW if control.x == 32816 && (32334..=32339).contains(&control.y) => true,
        COLUMN if control.y == 32332 && (32818..=32823).contains(&control.x) => false,
        _ => return None,
    };
    (control.floor == -9).then(|| {
        [2, 3, 4, 5, 6, 7].map(|offset| Position {
            x: control.x + if row { offset } else { 0 },
            y: control.y + if row { 0 } else { offset },
            floor: control.floor,
        })
    })
}

pub fn board_cell(x: u16, y: u16) -> Position {
    Position {
        x: 32818 + x,
        y: 32334 + y,
        floor: -9,
    }
}

pub fn solved_colour(x: u16, y: u16, appearance: u16) -> bool {
    match (x >= 3, y >= 3) {
        (false, false) => appearance == 2394,
        (true, false) => appearance == 2396,
        (false, true) => appearance == 2395,
        (true, true) => appearance == 2397 || appearance == 900,
    }
}

pub(crate) fn token_requirements(callback: &Callback) -> bool {
    callback.key == BOARD_EXIT
        && callback.quest == super::dreamer_fire_bug::QUEST
        && callback.trigger == Trigger::StepIn
        && callback.world_requirements.len() == TOKENS.len()
        && TOKENS.iter().all(|&(x, y, id)| {
            callback.world_requirements.iter().any(|requirement| {
                requirement.position == (Position { x, y, floor: -14 })
                    && requirement.ground_stack.as_deref().is_some_and(|stack| {
                        let [entry] = stack else { return false };
                        entry.count == 1 && entry.item == format!("oteryn:item.tibia.i{id}")
                    })
            })
        })
}
