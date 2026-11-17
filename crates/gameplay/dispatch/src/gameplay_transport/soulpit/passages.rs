//! Source Soulpit entrances and USE-only arena exit.
//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/scripts/actions/soulpit/soulpit_entrance.lua:1-57;
//! data-global/scripts/actions/soulpit/soulpit_arena_exit.lua:8-16.
use super::super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
    paced_movement::underwater_drown::underwater,
};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{
    ExactActorRef, GameSessionId, MovementLocalPosition, MovementPositionSnapshot,
};
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_gameplay_domain::domain::soulpit::{PassageTrigger, SoulpitPassage, soulpit_passage};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

fn native(position: MovementLocalPosition) -> Option<TilePos> {
    Some(TilePos {
        x: position.x.try_into().ok()?,
        y: position.y.try_into().ok()?,
        floor: position.floor.checked_neg()?.try_into().ok()?,
    })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Only the committed movement owner calls this, while retaining its spell-lane permit.
    /// The three lobby passages have no quest state, reward or inventory effect.
    pub(in crate::gameplay_transport) async fn soulpit_lobby_step(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) -> MovementPositionSnapshot {
        let Some(position) = native(entered.position()) else {
            return entered;
        };
        let Some(SoulpitPassage::Lobby(destination)) =
            soulpit_passage(PassageTrigger::StepIn, position)
        else {
            return entered;
        };
        let Some(map) = self.bundle_map() else {
            return entered;
        };
        let optional_pvp = self.special_movement_policy(permit, actor, session).await;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.read_actor_position(actor) != Ok(entered)
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, session)
        {
            return entered;
        }
        let destination = MovementLocalPosition {
            x: i32::from(destination.x),
            y: i32::from(destination.y),
            floor: -i16::from(destination.floor),
        };
        let blocked = super::super::pvp_live::pz_blocked(&states, actor, session, self.owner_now());
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport_facing(
            &runtime,
            self.movement_cells(),
            actor,
            session,
            entered,
            destination,
            blocked,
            &closed,
            None,
        ) else {
            return entered;
        };
        let Ok(snapshot) = runtime.commit_source_step(proof) else {
            return entered;
        };
        states.stage_special_movement_step(
            &runtime,
            map,
            actor,
            session,
            self.owner_now().get(),
            Some(entered.position()),
            snapshot.position(),
            optional_pvp,
        );
        states.stage_underwater_step(
            &runtime,
            actor,
            session,
            self.owner_now().get(),
            underwater(map, entered.position()),
            underwater(map, snapshot.position()),
        );
        snapshot
    }

    /// Resolve the authenticated map target before invoking the existing Instance return
    /// owner. A source coordinate alone never proves actor, session, scope or membership.
    pub(in crate::gameplay_transport) async fn use_soulpit_exit(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        let map = self.bundle_map()?;
        let generation = self.active_generation?;
        let source = super::super::world_map::MapViewSource {
            overlay: &map.overlay,
            facts: &map.facts,
            content_generation: generation.identity().client_artifact_digest(),
            reset_epoch: self.reset_epoch,
        };
        let resolved =
            super::super::world_map::resolve_map_target(&source, target, |_| None::<()>).ok()?;
        if soulpit_passage(PassageTrigger::Use, resolved.pos) != Some(SoulpitPassage::ArenaExit) {
            return None;
        }
        let Some(permit) = self.spell_lane_permit().await else {
            return Some(UseOutcome::rejected());
        };
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            let binding = runtime.binding();
            if runtime.owner_fence().is_err()
                || runtime.content_pin().client_artifact_digest()
                    != generation.identity().client_artifact_digest()
                || runtime.content_pin().server_artifact_digest()
                    != generation.identity().server_artifact_digest()
                || !command.item_fence.is_some_and(|fence| {
                    fence.game_session_id == command.game_session_id
                        && fence.runtime_scope == binding.runtime_scope()
                        && fence.scope_ownership_generation == binding.scope_generation()
                })
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
                // Acquiring the lane may have awaited while the map target was
                // removed or hidden. Re-prove the complete target under the owner lock.
                || super::super::world_map::resolve_map_target(&source, target, |_| None::<()>).is_err()
                || map.facts.object_revision(resolved.placement_key) != target.expected_revision
            {
                return Some(UseOutcome::rejected());
            }
            let Some(position) = runtime
                .read_actor_position(actor)
                .ok()
                .and_then(|p| native(p.position()))
            else {
                return Some(UseOutcome::rejected());
            };
            if !super::super::map_door::reachable(position, resolved.pos) {
                return Some(super::outcome(UseDisposition::TooFar));
            }
            if !self
                .states
                .world_gate
                .lock()
                .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
            {
                return Some(super::outcome(UseDisposition::Exhausted));
            }
        }
        // The handoff owner takes its complete admission lane and revalidates the
        // current Instance membership/session. Never retain this lane across it.
        drop(permit);
        let Some(owner) = self.states.boss_activity_handoff.get() else {
            return Some(UseOutcome::rejected());
        };
        Some(
            match owner
                .return_from_soulpit(self, actor, command.game_session_id, command.command_id)
                .await
            {
                Ok(()) => super::outcome(UseDisposition::Committed),
                Err(()) => UseOutcome::rejected(),
            },
        )
    }
}
