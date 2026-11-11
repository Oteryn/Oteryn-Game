//! Once-only committed entry and its opaque source-qualified XML activation plan.
use crate::content::bosses::activity_template::QualifiedAmbientXmlSpawns;
use crate::foundation::ChannelRuntimeV1;
use oteryn_gameplay_encounters::bosses::goshnars_greed::GoshnarsGreedRoom;

#[derive(Debug, Default)]
pub(crate) struct EntryState {
    entered: bool,
    ambient: Option<QualifiedAmbientXmlSpawns>,
}
impl EntryState {
    pub(super) fn prepare_ambient(&mut self, policy: QualifiedAmbientXmlSpawns) -> Result<(), ()> {
        if self.entered || self.ambient.as_ref().is_some_and(|old| old != &policy) {
            return Err(());
        }
        self.ambient = Some(policy);
        Ok(())
    }
    pub(super) fn has_ambient(&self) -> bool {
        self.ambient.is_some()
    }
    pub(super) fn claim_with_ambient(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        now_us: u64,
        elapsed_ms: u64,
    ) -> Result<bool, ()> {
        if self.entered {
            return Ok(false);
        }
        if let Some(policy) = &self.ambient {
            // The actual handoff clock, not prewarming or a later participant,
            // anchors the existing ten-second respawn schedule. A refused
            // current owner leaves this activation available for its retry.
            let activated = now_us
                .checked_sub(elapsed_ms.checked_mul(1_000).ok_or(())?)
                .ok_or(())?;
            runtime
                .register_empty_ambient_xml_spawns(policy, activated)
                .map_err(|_| ())?;
        }
        Ok(self.claim())
    }
    pub(crate) fn claim(&mut self) -> bool {
        !std::mem::replace(&mut self.entered, true)
    }
}

pub(crate) fn greed_at_birth() -> GoshnarsGreedRoom {
    GoshnarsGreedRoom::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_committed_soul_war_entry_is_claimed_once() {
        let mut entry = EntryState::default();
        assert!(entry.claim());
        assert!(
            !entry.claim(),
            "second participant and recovered transfer cannot restart timers"
        );
    }
    #[test]
    fn srx3_greed_prewarmed_birth_does_not_start_source_lever_timers() {
        let mut clock = greed_at_birth();
        let _ = clock.boss_spawned();
        assert_eq!(clock.next_due_ms(), None, "onSpawn is not onUseExtra");
        assert!(
            clock.advance(60_000).is_empty(),
            "empty prewarm emits no warnings/spawns"
        );
    }

    #[test]
    fn srx3_source_entry_activates_exact_empty_xml_points_once_after_current_handoff()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        std::thread::Builder::new()
            .name("srx3-source-entry-xml".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(exercise_source_entry)?
            .join()
            .map_err(|_| "source entry regression panicked")?
    }

    fn exercise_source_entry() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Room;
        use crate::content::native_gameplay::test_support::activated_with_loot_tables;
        use crate::foundation::{ChannelContentPin, ChannelId, NodeId, RuntimeScopeRefV1, WorldId};

        fn id(tag: u8) -> [u8; 16] {
            let mut value = [tag; 16];
            value[6] = 0x70;
            value[8] = 0x80;
            value
        }
        let world = WorldId::decode(&id(81)).map_err(|error| format!("{error:?}"))?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("actual activation")?;
        let native = active.native_gameplay().ok_or("actual native policies")?;
        for (tag, role, count) in [
            (82, Room::GoshnarsSpite, 3),
            (83, Room::GoshnarsHatred, 2),
            (84, Room::GoshnarsCruelty, 1),
            (85, Room::GoshnarsMegalomania, 3),
        ] {
            let room = active
                .qualify_activity_source_room(pin.qualified_room(), role)
                .map_err(|error| format!("actual source room: {error:?}"))?;
            let policy = active
                .qualify_ambient_xml_spawn_policy(&room)
                .map_err(|error| format!("actual XML policy: {error:?}"))?;
            let [server, client, frame, map] = policy.pins();
            let start = room.entry_start();
            let content = ChannelContentPin::from_activation(
                world,
                policy.activation_sequence(),
                server,
                client,
                frame,
                map,
                (start.x, start.y, start.floor),
            );
            let scope = RuntimeScopeRefV1::instance(world, id(tag))
                .map_err(|error| format!("{error:?}"))?;
            let node = NodeId::decode(&id(86)).map_err(|error| format!("{error:?}"))?;
            let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
                scope,
                node,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                32,
                content.clone(),
            )?;
            runtime.install_companion_policies(native.policies().clone())?;
            let mut callbacks = super::super::SoulWarCallbacks::default();
            callbacks
                .prepare_ambient_entry(active, &room)
                .map_err(|_| "exact ambient preparation")?;
            assert!(callbacks.requires_committed_entry());
            assert!(
                runtime.spawn_point_creatures().is_empty(),
                "prewarm has no XML timers or actors"
            );

            let channel = ChannelId::decode(&id(87)).map_err(|error| format!("{error:?}"))?;
            let mut wrong_scope = ChannelRuntimeV1::from_committed_scope_assignment(
                RuntimeScopeRefV1::channel(world, channel),
                node,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                32,
                content,
            )?;
            assert!(
                callbacks
                    .committed_entry(&mut wrong_scope, 5_000_000, 2_000)
                    .is_err()
            );
            assert!(wrong_scope.spawn_point_creatures().is_empty());
            assert!(
                callbacks
                    .committed_entry(&mut runtime, 1_000, 2_000)
                    .is_err(),
                "an unrepresentable original clock cannot consume the once-only claim"
            );
            assert!(runtime.spawn_point_creatures().is_empty());

            assert!(
                callbacks
                    .committed_entry(&mut runtime, 5_000_000, 2_000)
                    .map_err(|_| "current committed source entry")?
            );
            let points = runtime.spawn_point_creatures();
            assert_eq!(points.len(), count);
            assert!(
                points.iter().all(|(_, _, actor)| actor.is_none()),
                "clear leaves empty points, not injected startup monsters"
            );
            let delay = oteryn_gameplay_foundation::scheduled_events::scale_respawn_delay_micros(
                10_000_000,
                oteryn_gameplay_foundation::scheduled_events::current_rates().spawn,
            );
            let deadlines: Vec<_> = points
                .iter()
                .map(|(point, _, _)| {
                    runtime
                        .pending_respawn(*point)
                        .map(|pending| pending.due_micros)
                })
                .collect();
            assert_eq!(
                deadlines,
                vec![Some(3_000_000 + delay); count],
                "the source handoff at3s anchors the existing source10s delay"
            );
            assert!(
                !callbacks
                    .committed_entry(&mut runtime, 9_000_000, 6_000)
                    .map_err(|_| "same handoff replay")?
            );
            assert_eq!(runtime.spawn_point_creatures(), points);
            assert_eq!(
                runtime
                    .spawn_point_creatures()
                    .iter()
                    .map(|(point, _, _)| {
                        runtime
                            .pending_respawn(*point)
                            .map(|pending| pending.due_micros)
                    })
                    .collect::<Vec<_>>(),
                deadlines,
                "replay never replaces the original schedule"
            );
        }
        Ok(())
    }
}
