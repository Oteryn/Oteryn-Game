//! Crystal 201189f8, bounac_cathedral.lua:18-46,65-129 and order_of_the_stag.lua:52-58.
use super::world_callbacks::Position;

pub const SOURCE: Position = Position {
    x: 32402,
    y: 32467,
    floor: -4,
};
pub const DESTINATION: Position = Position {
    floor: -5,
    ..SOURCE
};
pub const STAND: Position = Position { y: 32465, ..SOURCE };
pub const WARLOCK_SPAWN: Position = Position {
    x: 32403,
    y: 32470,
    floor: -5,
};
pub const WARLOCK: &str = "oteryn:creature.ancient_lion_warlock";
pub const CLOSED: u16 = 417;
pub const OPEN: u16 = 52654;
pub const PICK: &str = "oteryn:item.tibia.i3456";
pub const STARLIGHT: &str = "oteryn:item.tibia.i3249";
pub const OPEN_US: u64 = 300_000_000;
