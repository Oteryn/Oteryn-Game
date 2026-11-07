//! Authored exact counterparts for native tomb switches and threshold transports.
use super::{gate, recipe};
use crate::quest::world_callbacks::Position;
#[derive(Clone, Copy, Debug)]
pub struct NativeSwitchBinding {
    pub position: Position,
    pub ordinal: u8,
    pub appearance: u16,
}
const fn at(x: u16, y: u16, floor: i8, ordinal: u8) -> NativeSwitchBinding {
    NativeSwitchBinding {
        position: Position { x, y, floor },
        ordinal,
        appearance: 2772,
    }
}
const BINDINGS_12121: &[NativeSwitchBinding] = &[
    at(33320, 32682, -14, 1),
    at(33338, 32702, -14, 1),
    at(33349, 32680, -14, 1),
    at(33358, 32701, -14, 1),
    at(33305, 32734, -14, 1),
    at(33357, 32749, -14, 1),
    at(33368, 32763, -14, 1),
    at(33382, 32786, -14, 1),
];
const BINDINGS_12122: &[NativeSwitchBinding] = &[
    at(33006, 32563, -13, 1),
    at(33055, 32487, -13, 1),
    at(33036, 32507, -13, 1),
    at(33077, 32507, -13, 1),
    at(33130, 32489, -13, 1),
    at(33027, 32530, -13, 1),
    at(33089, 32514, -13, 1),
    at(33104, 32514, -13, 1),
    at(33147, 32524, -13, 1),
    at(33028, 32588, -13, 1),
    at(33123, 32599, -13, 1),
];
const BINDINGS_12123: &[NativeSwitchBinding] = &[
    at(33176, 32880, -11, 2),
    at(33182, 32880, -11, 2),
    at(33175, 32884, -11, 2),
    at(33183, 32884, -11, 2),
    at(33176, 32889, -11, 2),
    at(33181, 32889, -11, 2),
];
const GATE_50135: &[NativeSwitchBinding] = &[NativeSwitchBinding {
    position: Position {
        x: 33381,
        y: 32786,
        floor: -14,
    },
    ordinal: 0,
    appearance: 415,
}];
const GATE_50137: &[NativeSwitchBinding] = &[NativeSwitchBinding {
    position: Position {
        x: 33088,
        y: 32514,
        floor: -13,
    },
    ordinal: 0,
    appearance: 415,
}];
const GATE_50138: &[NativeSwitchBinding] = &[NativeSwitchBinding {
    position: Position {
        x: 33182,
        y: 32884,
        floor: -11,
    },
    ordinal: 0,
    appearance: 417,
}];
const GATE_40083: &[NativeSwitchBinding] = &[NativeSwitchBinding {
    position: Position {
        x: 33093,
        y: 32589,
        floor: -15,
    },
    ordinal: 0,
    appearance: 423,
}];
pub fn bindings(key: &str) -> &'static [NativeSwitchBinding] {
    match recipe(key).map(|r| r.action) {
        Some(12121) => BINDINGS_12121,
        Some(12122) => BINDINGS_12122,
        Some(12123) => BINDINGS_12123,
        _ => match gate(key).map(|g| g.unique) {
            Some(40083) => GATE_40083,
            Some(50135) => GATE_50135,
            Some(50137) => GATE_50137,
            Some(50138) => GATE_50138,
            _ => &[],
        },
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_counterparts_do_not_share_a_placement_between_recipe_owners() {
        let mut seen = std::collections::BTreeSet::new();
        for key in super::super::RECIPES
            .iter()
            .map(|r| r.key)
            .chain(super::super::GATES.iter().map(|g| g.key))
        {
            for b in bindings(key) {
                assert!(
                    seen.insert((b.position.floor, b.position.x, b.position.y, b.ordinal)),
                    "native counter ownership is unambiguous"
                );
            }
        }
    }
}
