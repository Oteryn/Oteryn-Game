//! Crystal 201189f8, creaturescripts_putrefactory-x.lua:1-38,55-68,216.

pub const CHARGE_TRACK: &str =
    "oteryn:quest-progress/crystalserver/quest/u13_20/rotten_blood/putrefactory_task";

/// Source icon is present in the Putrefactory, its lever room and vestibule.
/// The charge track can outlive the icon when the player leaves these rooms.
#[must_use]
pub fn shows_charge_icon(x: i32, y: i32, z: i16) -> bool {
    let putrefactory =
        (34002..=34115).contains(&x) && (31658..=31770).contains(&y) && (14..=15).contains(&z);
    let lever_room = (32969..=32983).contains(&x) && (32328..=32338).contains(&y) && z == 15;
    let vestibule =
        (34095..=34105).contains(&x) && (31675..=31684).contains(&y) && (13..=14).contains(&z);
    putrefactory || lever_room || vestibule
}
