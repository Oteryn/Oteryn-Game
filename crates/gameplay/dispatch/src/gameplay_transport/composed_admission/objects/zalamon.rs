//! Crystal 201189f8, actions_zalamon_door.lua:5-17 and startup/tables/item.lua:4372-4375.
use super::*;
use crate::map::{boot::BundleMap, door::DoorUse};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

const FRONTIER: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission08";
const WRATH: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/mission11";

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_zalamon_door(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        use crate::gameplay_transport::world_map::{
            MapTargetRefusal, MapViewSource, resolve_map_target,
        };

        let raw: &[u8; 40] = target.placement.as_slice().try_into().ok()?;
        let key = u64::from_be_bytes(raw[32..].try_into().ok()?);
        let (at, ordinal) = crate::map::view::placement_of(key)?;
        let base = map.overlay.base().tile(at.x, at.y, at.floor)?;
        let (id, _) = base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .nth(usize::from(ordinal))?;
        if !map.facts.zalamon_door_source(key, *id) {
            return None;
        }
        let outcome = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return outcome(UseDisposition::Rejected);
        };
        let optional_pvp = self
            .special_movement_policy(&permit, actor, command.game_session_id)
            .await;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let session = command.game_session_id;
        if states.is_dead(actor)
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return outcome(UseDisposition::Rejected);
        }
        let source = MapViewSource {
            overlay: &map.overlay,
            facts: &map.facts,
            content_generation: runtime.content_pin().client_artifact_digest(),
            reset_epoch: self.reset_epoch,
        };
        match resolve_map_target(&source, target, |_| None::<()>) {
            Ok(_) => {}
            Err(MapTargetRefusal::Use(disposition)) => return outcome(disposition),
            Err(MapTargetRefusal::StateRevisionMismatch) => {
                return outcome(UseDisposition::StaleState);
            }
        }
        let Ok(expected) = runtime.read_actor_position(actor) else {
            return outcome(UseDisposition::Rejected);
        };
        let before = expected.position();
        if before.floor != -i16::from(at.floor)
            || before.x.abs_diff(i32::from(at.x)) > 1
            || before.y.abs_diff(i32::from(at.y)) > 1
        {
            return outcome(UseDisposition::TooFar);
        }
        if !self
            .states
            .world_gate
            .lock()
            .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
        {
            return outcome(UseDisposition::Exhausted);
        }
        let allowed = self.states.quest_sessions.lock().is_ok_and(|sessions| {
            sessions
                .get(&session)
                .and_then(|quest| quest.copy.as_ref())
                .is_some_and(|copy| {
                    let tracks = copy.tracks();
                    tracks.get(FRONTIER).is_some_and(|value| *value >= 2)
                        && tracks.get(WRATH).is_none_or(|value| *value < 2)
                })
        });
        if !allowed {
            self.states.chat.push_quest_feedback(
                session,
                "The door seems to be sealed against unwanted intruders.",
            );
            return outcome(UseDisposition::Locked);
        }
        let doors = map.facts.doors();
        if !doors.blocks(at) {
            return outcome(UseDisposition::NothingToUse);
        }
        let blocked = crate::gameplay_transport::pvp_live::pz_blocked(
            &states,
            actor,
            session,
            self.owner_now(),
        );
        let Ok(proof) = crate::map::floor::prepare_quest_door_opening(
            &runtime,
            self.movement_cells(),
            map,
            actor,
            session,
            expected,
            key,
            blocked,
        ) else {
            return outcome(UseDisposition::NotPossible);
        };
        let Ok(snapshot) = runtime.commit_source_step_and_project(proof, || {
            matches!(
                doors.toggle(key, target.expected_revision, false, 0, true),
                Some(DoorUse::Toggled { open: true, .. })
            )
        }) else {
            return outcome(UseDisposition::StaleState);
        };
        let now = self.owner_now().get();
        let after = snapshot.position();
        states.stage_special_movement_step(
            &runtime,
            map,
            actor,
            session,
            now,
            Some(before),
            after,
            optional_pvp,
        );
        use crate::gameplay_transport::paced_movement::underwater_drown::underwater;
        states.stage_underwater_step(
            &runtime,
            actor,
            session,
            now,
            underwater(map, before),
            underwater(map, after),
        );
        outcome(UseDisposition::Committed)
    }
}
