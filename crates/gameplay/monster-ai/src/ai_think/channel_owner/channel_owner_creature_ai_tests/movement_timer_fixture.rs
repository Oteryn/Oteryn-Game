//! Reuses the existing Channel harness and World fixtures; no second runtime or movement loop.
use super::*;

impl Channel {
    pub(super) fn advance_movement(&mut self, now: u64, walkable: bool) {
        use crate::movement::speed::{EngineeringGroundSpeed, creature_speed_fixture::Native};
        let blocking: BTreeSet<_> = if walkable {
            BTreeSet::new()
        } else {
            (0..128)
                .flat_map(|x| (0..128).map(move |y| crate::content::LogicalCell { x, y, z: 7 }))
                .collect()
        };
        let sources: Vec<_> = self
            .runtime
            .visible_entities()
            .creatures
            .iter()
            .map(|creature| Native {
                actor: creature.actor,
                base: 67,
                pin: self.runtime.content_pin().server_artifact_digest(),
            })
            .collect();
        let navigation = self.runtime.content_pin().map_revision_digest();
        let mut time = self.movement_time;
        loop {
            self.table.drive_movement(
                &mut self.runtime,
                &self.cells,
                &blocking,
                navigation,
                &sources,
                &EngineeringGroundSpeed,
                &self.root,
                time,
                &mut self.path_budget,
                false,
            );
            if time >= now {
                break;
            }
            time = time.saturating_add(50_000).min(now);
        }
        self.movement_time = now;
    }
}
