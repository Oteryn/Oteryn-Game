//! Current World projections use the existing home-spawn owner and its budgets.
use super::*;

/// A source-owner projection, never client-selected. None suppresses the point.
#[derive(Debug, Clone)]
pub struct WorldSpawnProjection {
    pub change_key: String,
    pub generation: u64,
    pub creature: Option<(String, i64, bool)>,
}
impl ChannelRuntimeV1 {
    pub(in crate::carrier) fn spawn_point_enabled(
        &self,
        point: spawn::SpawnPoint,
    ) -> Result<bool, CarrierError> {
        let index = self
            .carrier
            .spawn_index(SpawnSourceId(point.source))
            .ok_or(CarrierError::UnknownSpawnSource)?;
        Ok(self.carrier.spawns[index]
            .definition
            .point(usize::from(point.ordinal))?
            .enabled)
    }
    /// The actual actor's source cohort survives a delayed global phase observation.
    /// A newer cache cannot relabel a still-reserved actor from the preceding cohort.
    pub fn world_spawn_cohort(
        &self,
        actor: ExactActorRef,
    ) -> Option<(&str, u64, MovementLocalPosition)> {
        let (source, ordinal) = self.carrier.locate_spawn_cell(actor)?;
        let source = &self.carrier.spawns[self.carrier.spawn_index(source)?];
        let (key, generation) = source
            .definition
            .points
            .get(ordinal)?
            .world_change
            .as_ref()?;
        let cell = source.definition.placement_cells.get(ordinal)?;
        Some((
            key,
            *generation,
            MovementLocalPosition {
                x: cell.x,
                y: cell.y,
                floor: cell.floor,
            },
        ))
    }
    /// Exact base identity and home cell define membership; None leaves a point alone.
    /// Administrative retirement produces no combat death, loot or XP. Source/ordinal,
    /// placement, normal timing and per-window budgets remain owned by Spawn.
    /// Partial projection after a reserved-actor refusal is idempotently retried.
    pub fn bind_world_spawn_source(
        &mut self,
        source: u16,
        key: &str,
        generation: u64,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if key.is_empty() || key.len() > 128 || generation == 0 {
            return Err(CarrierError::InvalidSpawnDefinition);
        }
        let index = self
            .carrier
            .spawn_index(SpawnSourceId(source))
            .ok_or(CarrierError::UnknownSpawnSource)?;
        if self.carrier.spawns[index]
            .definition
            .points
            .iter()
            .any(|point| point.world_change.is_some())
        {
            return Err(CarrierError::PlanConflict);
        }
        for point in &mut self.carrier.spawns[index].definition.points {
            point.world_change = Some((key.to_owned(), generation));
        }
        Ok(())
    }
    pub fn project_world_spawn_points(
        &mut self,
        mut project: impl FnMut(&str, MovementLocalPosition) -> Option<WorldSpawnProjection>,
    ) -> Result<usize, CarrierError> {
        self.project_world_spawn_cohorts(|base, at, _| project(base, at))
    }
    pub fn project_world_spawn_cohorts(
        &mut self,
        mut project: impl FnMut(
            &str,
            MovementLocalPosition,
            Option<(&str, u64)>,
        ) -> Option<WorldSpawnProjection>,
    ) -> Result<usize, CarrierError> {
        self.owner_fence()?;
        if self.spawn_activation.is_some() {
            return Err(CarrierError::PlanConflict);
        }
        let mut updates = Vec::new();
        for (index, source) in self.carrier.spawns.iter().enumerate() {
            for (ordinal, point) in source.definition.points.iter().enumerate() {
                let cell = source.definition.placement_cells[ordinal];
                let Some(next) = project(
                    &point.base_creature,
                    MovementLocalPosition {
                        x: cell.x,
                        y: cell.y,
                        floor: cell.floor,
                    },
                    point
                        .world_change
                        .as_ref()
                        .map(|(key, generation)| (key.as_str(), *generation)),
                ) else {
                    continue;
                };
                if next.change_key.is_empty()
                    || next.change_key.len() > 128
                    || next.generation == 0
                    || next.creature.as_ref().is_some_and(|(key, health, _)| {
                        key.is_empty() || key.len() > 128 || *health <= 0
                    })
                {
                    return Err(CarrierError::InvalidSpawnDefinition);
                }
                let target_changed = match &next.creature {
                    None => point.enabled,
                    Some((key, health, blockable)) => {
                        !point.enabled
                            || point.creature_target_identity != *key
                            || point.creature_initial_health != *health
                            || point.blockable != *blockable
                    }
                };
                let cohort_changed =
                    point
                        .world_change
                        .as_ref()
                        .is_some_and(|(key, generation)| {
                            *key != next.change_key || *generation != next.generation
                        });
                let metadata_changed =
                    point.world_change.as_ref().is_none_or(|(key, generation)| {
                        *key != next.change_key || *generation != next.generation
                    });
                if target_changed || metadata_changed {
                    updates.push((index, ordinal, next, target_changed || cohort_changed));
                }
            }
        }
        let mut changed = 0;
        for (index, ordinal, next, rearm) in updates {
            if rearm {
                let live = self.carrier.spawns[index].cells[ordinal].live;
                if let Some(actor) =
                    live.filter(|actor| self.carrier.creature_alive(&self.continuity, *actor))
                {
                    self.carrier.remove(&self.continuity, actor.0)?;
                    self.carrier.spawns[index].cells[ordinal].live = None;
                }
            }
            let point = spawn::SpawnPoint {
                source: self.carrier.spawns[index].source.0,
                ordinal: u8::try_from(ordinal).map_err(|_| CarrierError::CapacityExceeded)?,
            };
            if rearm && let Some(pending) = self.spawn_schedule.cancel(point) {
                self.spawn_deadlines.remove(&(pending.due_micros, point));
            }
            let definition = &mut self.carrier.spawns[index].definition.points[ordinal];
            definition.world_change = Some((next.change_key, next.generation));
            definition.enabled = next.creature.is_some();
            if let Some((key, health, blockable)) = next.creature {
                definition.creature_target_identity = key;
                definition.creature_initial_health = health;
                definition.blockable = blockable;
                if rearm {
                    let timing = self.spawn_timing(point)?;
                    let now = self.spawn_drive_window.as_ref().map_or(0, |window| {
                        window.window.saturating_mul(spawn::OWNER_WINDOW_MICROS)
                    });
                    let immediate = spawn::PointTiming {
                        respawn_delay_micros: 0,
                        ..timing
                    };
                    let due = self
                        .spawn_schedule
                        .creature_gone(point, immediate, now)
                        .map_err(|_| CarrierError::CapacityExceeded)?;
                    self.spawn_deadlines.insert((due, point));
                }
            }
            changed += 1;
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn runtime() -> Result<ChannelRuntimeV1, CarrierError> {
        let id = |seed: u8| {
            let mut bytes = [seed; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes
        };
        let world = WorldId::decode(&id(1)).map_err(|_| CarrierError::InvalidAssignmentBinding)?;
        let pin = ChannelContentPin::from_activation(
            world,
            1,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
            (100, 100, 7),
        );
        ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).map_err(|_| CarrierError::InvalidAssignmentBinding)?,
            NodeId::decode(&id(3)).map_err(|_| CarrierError::InvalidAssignmentBinding)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin,
        )
    }
    fn projection(generation: u64, creature: Option<&str>) -> WorldSpawnProjection {
        WorldSpawnProjection {
            change_key: "fixture_world_change".into(),
            generation,
            creature: creature.map(|key| (key.to_owned(), 20, false)),
        }
    }
    #[test]
    fn worldchg_phased_spawns_suppress_rearm_and_preserve_source_membership()
    -> Result<(), CarrierError> {
        let mut runtime = runtime()?;
        runtime.realize_activation_spawns(vec![
            (
                vec![(101, 100, 7)],
                60_000,
                5_000,
                "oteryn:creature.wolf".into(),
                20,
            ),
            (
                vec![(103, 100, 7)],
                60_000,
                5_000,
                "oteryn:creature.wolf".into(),
                20,
            ),
            (
                vec![(102, 100, 7)],
                60_000,
                5_000,
                "oteryn:creature.rat".into(),
                20,
            ),
        ])?;
        let before = runtime.spawn_point_creatures();
        assert_eq!(
            runtime
                .project_world_spawn_points(|_, at| (at.x == 101).then(|| projection(1, None)))?,
            1
        );
        assert!(runtime.spawn_point_creatures()[0].2.is_none());
        assert_eq!(runtime.spawn_point_creatures()[1..], before[1..]);
        assert!(
            runtime.carrier.corpse_projections.is_empty(),
            "phase retirement is not a kill"
        );
        assert!(
            runtime
                .drive_spawn_respawns(300_000_000, |_, _| false)?
                .admitted
                .is_empty()
        );
        assert!(
            runtime.spawn_schedule.is_empty(),
            "suppression cannot enqueue a respawn"
        );
        assert_eq!(
            runtime.project_world_spawn_points(|base, at| (base == "oteryn:creature.wolf"
                && at.x == 101)
                .then(|| projection(2, Some("oteryn:creature.wolf"))))?,
            1
        );
        assert!(
            runtime
                .drive_spawn_respawns(300_000_000, |_, _| false)?
                .admitted
                .is_empty()
        );
        let admitted = runtime
            .drive_spawn_respawns(304_200_000, |_, _| false)?
            .admitted;
        assert_eq!(admitted.len(), 1);
        let actor = admitted[0];
        assert_eq!(
            runtime
                .world_spawn_cohort(actor)
                .map(|(key, generation, at)| (key.to_owned(), generation, at.x)),
            Some(("fixture_world_change".into(), 2, 101))
        );
        assert_eq!(
            runtime.project_world_spawn_points(
                |_, at| (at.x == 101).then(|| projection(2, Some("oteryn:creature.wolf")))
            )?,
            0,
            "same phase is idempotent"
        );
        assert_eq!(
            runtime.project_world_spawn_points(
                |_, at| (at.x == 101).then(|| projection(3, Some("oteryn:creature.wolf")))
            )?,
            1,
            "same species in a new cohort is a new spawn"
        );
        assert!(runtime.spawn_point_creatures()[0].2.is_none());
        assert!(runtime.carrier.corpse_projections.is_empty());
        assert_eq!(runtime.spawn_point_creatures()[1..], before[1..]);
        runtime.retire_owner_cycle();
        assert!(
            runtime
                .project_world_spawn_points(|_, _| Some(projection(4, None)))
                .is_err()
        );
        Ok(())
    }
}
