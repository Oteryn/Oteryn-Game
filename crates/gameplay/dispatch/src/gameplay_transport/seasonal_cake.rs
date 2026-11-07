//! Source-specific A Piece of Cake owner composition on the existing Channel cadence.
mod bonus;
pub(in crate::gameplay_transport) mod food;
use std::time::{SystemTime, UNIX_EPOCH};

pub(in crate::gameplay_transport) fn now_seconds() -> i64 {
    #[cfg(test)]
    if let Some(now) = test_clock::get() {
        return now;
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .unwrap_or(0)
}

#[cfg(test)]
pub(super) mod test_clock {
    use std::cell::Cell;
    thread_local! { static NOW: Cell<Option<i64>> = const { Cell::new(None) }; }
    pub(in crate::gameplay_transport) struct Guard(Option<i64>);
    impl Drop for Guard {
        fn drop(&mut self) {
            NOW.with(|now| now.set(self.0));
        }
    }
    pub(super) fn get() -> Option<i64> {
        NOW.with(Cell::get)
    }
    pub(in crate::gameplay_transport) fn set(now: i64) -> Guard {
        Guard(NOW.with(|cell| cell.replace(Some(now))))
    }
}

use super::ComposedFreshAdmission;
use crate::durability::seasonal_cake::character::CakeCity;
use crate::foundation::MovementLocalPosition;
use crate::map::overlay::TilePos;

const GOLEM: &str = "oteryn:creature.cake_golem";

/// Crystal 201189f8 startup/tables/tile.lua:1114-1152; each machine has two source pads.
pub(in crate::gameplay_transport) fn machine_city(at: MovementLocalPosition) -> Option<CakeCity> {
    if at.floor != 7 {
        return None;
    }
    match (at.x, at.y) {
        (32346, 31822 | 31823) => Some(CakeCity::Carlin),
        (32350, 32275 | 32276) => Some(CakeCity::Thais),
        (33251, 31802 | 31803) => Some(CakeCity::Edron),
        (33191, 32397 | 32398) => Some(CakeCity::Darashia),
        (32377, 32801 | 32802) => Some(CakeCity::LibertyBay),
        _ => None,
    }
}

mod cadence;
mod golems;
mod player_step;
