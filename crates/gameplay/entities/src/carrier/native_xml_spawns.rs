//! Exact XML placement is an extension of the existing spawn owner, not a second
//! population or respawn clock. Ordinary D115 occupancy policy stays separate.
use super::*;
use oteryn_gameplay_content::content::bosses::activity_template::QualifiedAmbientXmlSpawns;

impl ChannelRuntimeV1 {
    pub(super) fn current_xml_spawn(&self, point: spawn::SpawnPoint) -> Result<bool, CarrierError> {
        let definition = &self
            .carrier
            .spawns
            .get(usize::from(point.source))
            .ok_or(CarrierError::InvalidSpawnDefinition)?
            .definition;
        let Some(policy) = &definition.xml_source else {
            return Ok(false);
        };
        let pin = self.content_pin();
        if policy.world() != pin.world_id()
            || policy.activation_sequence() != pin.activation_sequence()
            || policy.pins()
                != [
                    pin.server_artifact_digest(),
                    pin.client_artifact_digest(),
                    pin.frame_binding_digest(),
                    pin.map_revision_digest(),
                ]
        {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        Ok(true)
    }
    /// A lever clears its private room before its committed activation. Register
    /// those empty XML points against that activation, without startup monsters.
    pub fn register_empty_ambient_xml_spawns(
        &mut self,
        policy: &QualifiedAmbientXmlSpawns,
        activated_at_micros: u64,
    ) -> Result<usize, CarrierError> {
        self.owner_fence()?;
        let pin = self.content_pin();
        if policy.world() != pin.world_id()
            || policy.activation_sequence() != pin.activation_sequence()
            || policy.pins()
                != [
                    pin.server_artifact_digest(),
                    pin.client_artifact_digest(),
                    pin.frame_binding_digest(),
                    pin.map_revision_digest(),
                ]
        {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        if !matches!(
            self.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) {
            return Err(CarrierError::WrongScope);
        }
        let definitions = policy
            .sources()
            .iter()
            .cloned()
            .map(SpawnDefinition::from_activation)
            .collect::<Result<Vec<_>, _>>()?;
        let total = self.check_spawn_activation(&definitions)?;
        let mut schedule = spawn::RespawnSchedule::default();
        let mut deadlines = BTreeSet::new();
        let mut realized = Vec::with_capacity(definitions.len());
        for (source, mut definition) in definitions.into_iter().enumerate() {
            definition.xml_source = Some(policy.clone());
            let source = u16::try_from(source).map_err(|_| CarrierError::CapacityExceeded)?;
            let mut cells = Vec::with_capacity(definition.placement_cells.len());
            for ordinal in 0..definition.placement_cells.len() {
                let point = spawn::SpawnPoint {
                    source,
                    ordinal: u8::try_from(ordinal).map_err(|_| CarrierError::CapacityExceeded)?,
                };
                let mut timing = definition.timing(ordinal)?;
                timing.respawn_delay_micros =
                    oteryn_gameplay_foundation::scheduled_events::scale_respawn_delay_micros(
                        timing.respawn_delay_micros,
                        oteryn_gameplay_foundation::scheduled_events::current_rates().spawn,
                    );
                let due = schedule
                    .creature_gone(point, timing, activated_at_micros)
                    .map_err(|_| CarrierError::CapacityExceeded)?;
                deadlines.insert((due, point));
                cells.push(spawn_realization::SpawnCellState {
                    live: None,
                    attempt: 0,
                    successor: 0,
                });
            }
            realized.push(SpawnRealization {
                source: SpawnSourceId(source),
                definition,
                cells,
            });
        }
        // Publish only after every definition, ceiling and deadline was checked.
        self.carrier.spawns = realized;
        self.spawn_schedule = schedule;
        self.spawn_deadlines = deadlines;
        self.spawn_scan_cursor = spawn::SpawnPoint {
            source: 0,
            ordinal: 0,
        };
        self.spawn_drive_window = None;
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_gameplay_content::content::bosses::activity_template::PinnedActivitySourceRoom;
    use oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables;
    fn id(n: u8) -> [u8; 16] {
        let mut v = [n; 16];
        v[6] = 0x70;
        v[8] = 0x80;
        v
    }
    #[test]
    fn srx3_native_xml_empty_instance_registration_uses_existing_spawn_owner()
    -> Result<(), Box<dyn std::error::Error>> {
        exercise_empty_instance(false)
    }
    #[test]
    fn srx3_native_xml_three_source_warnings_share_existing_respawn_deadline()
    -> Result<(), Box<dyn std::error::Error>> {
        exercise_empty_instance(true)
    }
    fn exercise_empty_instance(
        check_warning_phases: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let world = WorldId::decode(&id(81)).map_err(|e| format!("{e:?}"))?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("actual Content activation")?;
        let room = active
            .qualify_activity_source_room(
                pin.qualified_room(),
                PinnedActivitySourceRoom::GoshnarsSpite,
            )
            .map_err(|e| format!("actual Spite source map: {e:?}"))?;
        let policy = active
            .qualify_ambient_xml_spawn_policy(&room)
            .map_err(|e| format!("actual XML policy: {e:?}"))?;
        assert_eq!(policy.sources().len(), 3);
        let [server, client, frame, map] = policy.pins();
        let current_pin = ChannelContentPin::from_activation(
            world,
            policy.activation_sequence(),
            server,
            client,
            frame,
            map,
            (33742, 31639, 14),
        );
        let scope = RuntimeScopeRefV1::instance(world, id(82)).map_err(|e| format!("{e:?}"))?;
        let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
            scope,
            NodeId::decode(&id(83)).map_err(|e| format!("{e:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            32,
            current_pin,
        )?;
        runtime.install_companion_policies(
            active
                .native_gameplay()
                .ok_or("native closure")?
                .policies()
                .clone(),
        )?;
        assert!(runtime.spawn_point_creatures().is_empty());
        let registered = runtime
            .register_empty_ambient_xml_spawns(&policy, 5_000_000)
            .map_err(|e| format!("REACHED exact XML empty-point registration: {e:?}"))?;
        assert_eq!(registered, 3);
        let points = runtime.spawn_point_creatures();
        assert_eq!(points.len(), 3);
        assert!(points.iter().all(|(_, _, actor)| actor.is_none()));
        // The existing owner clock owns the first ten-second source deadline.
        let before = runtime.drive_spawn_respawns(14_999_999, |_, _| false)?;
        assert!(before.admitted.is_empty() && before.warnings.is_empty());
        let due = runtime.drive_spawn_respawns(15_000_000, |_, _| false)?;
        assert_eq!(due.warnings.len(), 3);
        assert!(due.admitted.is_empty());
        if !check_warning_phases {
            return Ok(());
        }
        // Crystal spawn_monster.cpp:382,393-400 + spawn_monster.hpp:110:
        // the SAME pending nonblockable respawn emits three 1.4s warnings.
        let second = runtime.drive_spawn_respawns(16_400_000, |_, _| false)?;
        assert_eq!(second.warnings.len(), 3, "second XML teleport warning");
        assert!(second.admitted.is_empty());
        let third = runtime.drive_spawn_respawns(17_800_000, |_, _| false)?;
        assert_eq!(third.warnings.len(), 3, "third XML teleport warning");
        assert!(third.admitted.is_empty());
        let born = runtime.drive_spawn_respawns(19_200_000, |_, _| false)?;
        assert_eq!(born.admitted.len(), 3);
        Ok(())
    }
}
