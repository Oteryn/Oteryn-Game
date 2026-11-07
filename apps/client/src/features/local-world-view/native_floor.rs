//! Presentation coordinates for runtime actors on an already decoded native map.
use crate::play::PlayView;

fn native_floor(floor: i16) -> i16 {
    if (1..=15).contains(&floor) {
        -floor
    } else {
        floor
    }
}

impl PlayView {
    pub(in crate::play) fn presentation_floor(&self, floor: i16) -> i16 {
        if self.live_map && self.map_floor.is_some_and(|map_floor| map_floor <= 0) {
            native_floor(floor)
        } else {
            floor
        }
    }

    pub(super) fn project_native_positions(&mut self) {
        if !self.live_map || self.map_floor.is_none_or(|floor| floor > 0) {
            return;
        }
        self.floor = native_floor(self.floor);
        for entity in &mut self.entities {
            entity.position.floor = native_floor(entity.position.floor);
        }
        for appearance in &mut self.appearances {
            appearance.entity.position.floor = native_floor(appearance.entity.position.floor);
        }
    }
}
