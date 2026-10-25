//! PUSH-ROUTE-1: one admitted player pushes one adjacent creature or player one cell.
//! Canary `Game::playerMoveCreature` order: movability, throw range, adjacency, zones, room.
//! A refusal returns before any position commit, so nothing moves.
use super::*;
use crate::content::ProjectV2AuthoringProfileData;
use crate::foundation::MovementLocalPosition;
use crate::foundation::MovementPositionSnapshot;
use oteryn_protocol_oteryn::creature_push::{CreaturePushDisposition, CreaturePushIntent};

enum Target {
    Creature(MovementLocalPosition),
    Player(GameSessionId, MovementPositionSnapshot),
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn push_creature_now(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        intent: CreaturePushIntent,
    ) -> CreaturePushDisposition {
        use CreaturePushDisposition::{
            DestinationOutOfReach, Moved, NotEnoughRoom, NotMovable, NotPossible, Rejected,
        };
        if !self.ensure_source_map_initialized().await || self.states.newhaven_travel.held(session)
        {
            return Rejected;
        }
        let destination = MovementLocalPosition {
            x: intent.destination.x,
            y: intent.destination.y,
            floor: intent.destination.floor,
        };
        let Some(_permit) = self.spell_lane_permit().await else {
            return Rejected;
        };
        let mut runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        let context = runtime.pinned_movement_context();
        let pusher = match runtime.borrow_movement_position().read(actor) {
            Ok(snapshot)
                if snapshot.context() == context
                    && !states.is_dead(actor)
                    && runtime
                        .player_control_facts(actor, session)
                        .is_ok_and(|facts| facts.control_loss.is_none()) =>
            {
                snapshot
            }
            _ => return Rejected,
        };
        let entities = runtime.visible_entities();
        let matches = |entry: ExactActorRef, generation: u64| {
            entry.placement_identity() == intent.target.identity
                && generation == intent.target.generation
        };
        // Canary game.cpp:1878: an NPC is never pushed into a free cell by a player here.
        if entities
            .npcs
            .iter()
            .any(|npc| matches(npc.actor, npc.generation))
        {
            return NotEnoughRoom;
        }
        let (target, at) = if let Some(creature) = entities
            .creatures
            .iter()
            .find(|entry| matches(entry.actor, entry.generation))
        {
            if states.is_dead(creature.actor) || !self.creature_pushable(&runtime, creature.actor) {
                return NotMovable;
            }
            (creature.actor, Target::Creature(creature.position))
        } else if let Some(player) = entities
            .players
            .iter()
            .find(|entry| matches(entry.actor, entry.generation))
        {
            let census = runtime.positioned_melee_census().unwrap_or_default();
            let Some((_, snapshot, Some(target_session))) = census
                .into_iter()
                .find(|(entry, _, _)| *entry == player.actor)
            else {
                return NotPossible;
            };
            if states.is_dead(player.actor) || snapshot.context() != context {
                return NotPossible;
            }
            (player.actor, Target::Player(target_session, snapshot))
        } else {
            return NotPossible;
        };
        if target == actor {
            return NotPossible;
        }
        let from = match at {
            Target::Creature(position) => position,
            Target::Player(_, snapshot) => snapshot.position(),
        };
        let here = pusher.position();
        // Canary game.cpp:1851: the destination is a neighbour of the pushed creature.
        if destination.floor != from.floor
            || destination == from
            || (destination.x - from.x).abs() > 1
            || (destination.y - from.y).abs() > 1
        {
            return DestinationOutOfReach;
        }
        // Canary game.cpp:1856: the pushed creature stands next to the pusher.
        if here.floor != from.floor || (here.x - from.x).abs() > 1 || (here.y - from.y).abs() > 1 {
            return NotPossible;
        }
        // Canary game.cpp:1865: no push out of a protection or no-PvP zone into a weaker one.
        let (Some(origin), Some(landing)) = (self.push_zones(from), self.push_zones(destination))
        else {
            return NotPossible;
        };
        if (origin.0 && !landing.0) || (origin.1 && !landing.1) {
            return NotPossible;
        }
        // Canary game.cpp:1869: any creature on the destination refuses the push.
        if runtime
            .position_occupied_by_other(target, destination)
            .unwrap_or(true)
        {
            return NotEnoughRoom;
        }
        match at {
            Target::Creature(_) => {
                let (none, bundle_doors, legacy);
                let blocking: &dyn crate::map::floor::CellBlocker = if self
                    .rooms
                    .gate_source_world()
                    .is_some_and(|room| room.source_world().is_some())
                {
                    none = std::collections::BTreeSet::<crate::content::LogicalCell>::new();
                    &none
                } else if let Some(map) = self.bundle_map() {
                    bundle_doors = crate::map::floor::BundleDoors(map.facts.doors());
                    &bundle_doors
                } else {
                    legacy = self.door.lock().await.blocking_cells().clone();
                    &legacy
                };
                // Canary game.cpp:1862: the cell must admit the creature (walkable, no block).
                match crate::movement::relocate::relocate_creature(
                    &mut runtime,
                    target,
                    self.movement_cells,
                    blocking,
                    destination,
                ) {
                    Ok(_) => Moved,
                    Err(_) => NotEnoughRoom,
                }
            }
            Target::Player(target_session, snapshot) => {
                let direction = match (destination.x - from.x, destination.y - from.y) {
                    (0, -1) => StepDirection::North,
                    (1, 0) => StepDirection::East,
                    (0, 1) => StepDirection::South,
                    (-1, 0) => StepDirection::West,
                    // A diagonal push of a player is outside this slice.
                    _ => return NotPossible,
                };
                drop(states);
                drop(runtime);
                drop(_permit);
                // Canary creature.cpp:520: the pushed creature's step clock restarts.
                let holder = Arc::new(());
                if self.begin_actor_step_now(target, &holder).await.is_some() {
                    return NotMovable;
                }
                let (outcome, duration) = self
                    .tutorial_paced_step_from(
                        target,
                        target_session,
                        direction,
                        Some(snapshot),
                        Some((actor, pusher)),
                    )
                    .await;
                let (disposition, due) = match outcome.disposition {
                    StepDisposition::Moved => (
                        Moved,
                        duration.map(|duration| tokio::time::Instant::now() + duration),
                    ),
                    StepDisposition::Blocked => (NotEnoughRoom, None),
                    _ => (NotPossible, None),
                };
                self.record_actor_step_deadline_now(target, &holder, due)
                    .await;
                disposition
            }
        }
    }

    /// Canary monster.cpp:275: a wild creature whose profile is pushable and can walk.
    fn creature_pushable(&self, runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> bool {
        let Ok(snapshot) = runtime.companion_snapshot(actor) else {
            return false;
        };
        if snapshot.state.master.is_some() || snapshot.state.policy.is_familiar {
            return false;
        }
        let Some(content) = self.active_generation.and_then(|a| a.native_gameplay()) else {
            return false;
        };
        content
            .creature_profiles()
            .records
            .iter()
            .find(|record| {
                record.profile.target.key == snapshot.state.policy.definition_key
                    && record.profile.target.revision == snapshot.state.policy.definition_revision
            })
            .and_then(|record| record.behavior.as_ref())
            .is_some_and(|behavior| match &behavior.data {
                ProjectV2AuthoringProfileData::Behavior(behavior) => {
                    behavior.movement.pushable && behavior.movement.can_walk
                }
                _ => false,
            })
    }

    /// (protection zone, no-PvP zone) of one cell; `None` when its tile is unknown.
    fn push_zones(&self, at: MovementLocalPosition) -> Option<(bool, bool)> {
        let cells = self.movement_cells;
        let cell = crate::content::LogicalCell {
            x: at.x,
            y: at.y,
            z: i32::from(at.floor),
        };
        match cells.spell_tiles().lookup(cells.scope(), cell) {
            Ok(tile) => Some((
                tile.flags().protection_zone,
                tile.no_pvp_zone().unwrap_or(false),
            )),
            Err(crate::content::SpellTileLookupError::Unknown)
                if crate::movement::relocate::accepted_entry_movement(cells) =>
            {
                Some((false, false))
            }
            Err(_) => None,
        }
    }
}

macro_rules! authority_methods {
    () => {
        fn push_creature(
            &self,
            _actor: $crate::foundation::ExactActorRef,
            _session: $crate::foundation::GameSessionId,
            _intent: oteryn_protocol_oteryn::creature_push::CreaturePushIntent,
        ) -> impl std::future::Future<
            Output = oteryn_protocol_oteryn::creature_push::CreaturePushDisposition,
        > {
            async { oteryn_protocol_oteryn::creature_push::CreaturePushDisposition::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn push_creature(
            &self,
            actor: $crate::foundation::ExactActorRef,
            session: $crate::foundation::GameSessionId,
            intent: oteryn_protocol_oteryn::creature_push::CreaturePushIntent,
        ) -> oteryn_protocol_oteryn::creature_push::CreaturePushDisposition {
            self.push_creature_now(actor, session, intent).await
        }
    };
}
pub(super) use composed_methods;
