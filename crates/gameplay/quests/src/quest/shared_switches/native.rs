//! Authored exact counterparts for native tomb levers whose legacy action ids were omitted.
use super::recipe;
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
pub fn bindings(key: &str) -> &'static [NativeSwitchBinding] {
    match recipe(key).map(|r| r.action) {
        Some(12121) => BINDINGS_12121,
        Some(12122) => BINDINGS_12122,
        Some(12123) => BINDINGS_12123,
        _ => &[],
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_counterparts_do_not_share_a_placement_between_recipe_owners() {
        let mut seen = std::collections::BTreeSet::new();
        for recipe in super::super::RECIPES {
            for b in bindings(recipe.key) {
                assert!(
                    seen.insert((b.position.floor, b.position.x, b.position.y, b.ordinal)),
                    "native counter ownership is unambiguous"
                );
            }
        }
    }
}
