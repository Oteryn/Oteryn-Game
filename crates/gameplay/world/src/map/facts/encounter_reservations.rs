//! A bounded room lease bridges fenced quest IO and the native owner turn.
use super::{BundleFacts, CallbackStates, TilePos};
use crate::map::view::MapFacts;
use std::collections::BTreeSet;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone)]
pub struct EncounterRoomReservation(Arc<Reservation>);
struct Reservation {
    callbacks: Arc<Mutex<Arc<CallbackStates>>>,
    id: u64,
    released: AtomicBool,
}
impl Reservation {
    fn release(&self) {
        if self.released.swap(true, Ordering::AcqRel) {
            return;
        }
        if let Ok(mut states) = self.callbacks.lock() {
            Arc::make_mut(&mut states).encounter_rooms.remove(&self.id);
        }
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        self.release();
    }
}
impl EncounterRoomReservation {
    /// Release immediately before the synchronous native commit, or on refusal.
    pub fn release(&self) {
        self.0.release();
    }
}
impl BundleFacts {
    /// Reserve a public native room; overlapping admissions fail before payment.
    /// The last lease drop releases it, including cancellation and refusal paths.
    pub fn reserve_encounter_room(
        &self,
        base: &super::super::WorldBase,
        min: TilePos,
        max: TilePos,
    ) -> Option<EncounterRoomReservation> {
        if min.floor != max.floor || min.x > max.x || min.y > max.y {
            return None;
        }
        let width = u32::from(max.x) - u32::from(min.x) + 1;
        let height = u32::from(max.y) - u32::from(min.y) + 1;
        if width.checked_mul(height)? > 4096 {
            return None;
        }
        let mut cells = BTreeSet::new();
        for x in min.x..=max.x {
            for y in min.y..=max.y {
                let at = TilePos {
                    x,
                    y,
                    floor: min.floor,
                };
                if self.house_tile(at) || base.tile(x, y, min.floor).is_none() {
                    return None;
                }
                cells.insert(at);
            }
        }
        let mut states = self.callbacks.lock().ok()?;
        if states.encounter_rooms.len() >= 32
            || !states.has_capacity(cells.len())
            || states
                .encounter_rooms
                .values()
                .any(|room| !room.is_disjoint(&cells))
        {
            return None;
        }
        let id = states
            .encounter_rooms
            .keys()
            .next_back()
            .copied()
            .unwrap_or(0)
            .checked_add(1)?;
        Arc::make_mut(&mut states).encounter_rooms.insert(id, cells);
        Some(EncounterRoomReservation(Arc::new(Reservation {
            callbacks: Arc::clone(&self.callbacks),
            id,
            released: AtomicBool::new(false),
        })))
    }
}
