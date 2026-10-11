//! MAP-DOOR-1: one `USE_INTENT` on a door of a bundle World (decision
//! `OTERYN_GAME_WORLD_INTERACTION0_DOORS_LEVERS_FIELDS_AND_WORLD_CLOCK_DECISION` §4).
//!
//! The client names a base entry by the 40-byte map target and the object revision it last saw
//! (`WorldObjectTargetV1`, contract §4). The server resolves the target, reaches it (same floor,
//! Chebyshev distance <= 1 from the door's tile) and toggles the door's state once, refusing to
//! close a door whose tile an actor stands on. The new state reaches every session through the
//! next map view update (the connection loop refreshes it every 250 ms and after its own steps);
//! the overlay stays empty (MAP track packets §1.2), so a commit sends no domain 2 delta.

use super::connection::{EarnedNotice, UseOutcome};
use super::world_map::{MapTargetRefusal, MapViewSource, resolve_map_target};
use super::world_object::{UseDisposition, WorldObjectTarget};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use crate::map::door::DoorUse;
use crate::map::facts::BundleFacts;
use crate::map::overlay::ChannelOverlay;
use crate::map::overlay::TilePos;

fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        committed: None,
        earned: EarnedNotice::NoneEarned,
    }
}

/// `WORLDINT0-RL-02`: the per-actor `world_action` cooldown.
const WORLD_ACTION_COOLDOWN: std::time::Duration = std::time::Duration::from_millis(200);
/// `WORLDINT0-RL-08`: world interactions admitted per Channel per simulation tick.
const WORLD_ACTIONS_PER_TICK: u32 = 512;
/// The owner's simulation tick, the window the per-tick cap counts in.
const SIMULATION_TICK: std::time::Duration = std::time::Duration::from_millis(100);

/// The world-interaction admission gate of one Channel (`WORLD_INTERACTION0` §3.4): the
/// `world_action` cooldown per actor, started at admission and kept after a refused act, and the
/// per-tick cap. A refused act is `EXHAUSTED`, queues nothing and changes nothing.
#[derive(Debug, Default)]
pub(crate) struct WorldActionGate {
    started: Vec<(ExactActorRef, std::time::Duration)>,
    tick: u128,
    admitted: u32,
}

impl WorldActionGate {
    /// Admits one `USE` of `actor` at owner-clock time `now`, or refuses it.
    pub(crate) fn admit(&mut self, actor: ExactActorRef, now: std::time::Duration) -> bool {
        self.started
            .retain(|(_, at)| now.saturating_sub(*at) < WORLD_ACTION_COOLDOWN);
        if self.started.iter().any(|(who, _)| *who == actor) {
            return false;
        }
        let tick = now.as_nanos() / SIMULATION_TICK.as_nanos();
        if tick != self.tick {
            self.tick = tick;
            self.admitted = 0;
        }
        if self.admitted >= WORLD_ACTIONS_PER_TICK {
            return false;
        }
        self.admitted += 1;
        self.started.push((actor, now));
        true
    }
}

/// Whether `actor` can reach a door at `door`: same floor, Chebyshev distance <= 1.
pub(super) fn reachable(actor: TilePos, door: TilePos) -> bool {
    actor.floor == door.floor && actor.x.abs_diff(door.x) <= 1 && actor.y.abs_diff(door.y) <= 1
}

/// Whether a live actor stands on `door`: the acting actor on `actor_tile`, or any other actor
/// the Channel's own collision census counts (Codex 4238971357, 4239177729): committed players,
/// live creatures, placed NPCs, a companion reserved for an unresolved spawn and a pending
/// relocation's destination. A census that cannot read a position fails closed (occupied), as
/// movement does.
pub(super) fn doorway_occupied(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    actor_tile: TilePos,
    door: TilePos,
) -> bool {
    if door == actor_tile {
        return true;
    }
    // The native floor is the runtime's legacy `z` negated.
    let cell = MovementLocalPosition {
        x: i32::from(door.x),
        y: i32::from(door.y),
        floor: i16::from(door.floor).saturating_neg(),
    };
    // The issuer's own unresolved relocation into the doorway (Codex 4240828401): the census
    // above skips the issuer's slot, so a later COMMIT would land it on a solid tile.
    runtime
        .reserved_into_by_own_pending(actor, cell)
        .unwrap_or(true)
        || runtime
            .position_occupied_by_other(actor, cell)
            .unwrap_or(true)
}

/// Decides one `USE` of `target` by an actor on `actor`. `occupied` says whether an actor stands
/// on a tile; it is read under the same lock as the toggle.
pub(super) fn use_door(
    overlay: &ChannelOverlay,
    facts: &BundleFacts,
    content_generation: [u8; 32],
    actor: TilePos,
    occupied: impl FnOnce(TilePos) -> bool,
    target: &WorldObjectTarget,
) -> UseOutcome {
    let source = MapViewSource {
        overlay,
        facts,
        content_generation,
        reset_epoch: 0,
    };
    let resolved = match resolve_map_target(&source, target, |_| None::<()>) {
        Ok(resolved) => resolved,
        Err(MapTargetRefusal::StateRevisionMismatch) => {
            return outcome(UseDisposition::StaleState);
        }
        Err(MapTargetRefusal::Use(disposition)) => return outcome(disposition),
    };
    let doors = facts.doors();
    if !doors.contains(resolved.placement_key) {
        return outcome(UseDisposition::NothingToUse);
    }
    if !reachable(actor, resolved.pos) {
        return outcome(UseDisposition::TooFar);
    }
    let occupied = occupied(resolved.pos);
    match doors.toggle(resolved.placement_key, target.expected_revision, occupied) {
        Some(DoorUse::Toggled { .. }) => outcome(UseDisposition::Committed),
        Some(DoorUse::Stale) => outcome(UseDisposition::StaleState),
        Some(DoorUse::Occupied) => outcome(UseDisposition::Occupied),
        None => outcome(UseDisposition::NothingToUse),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]
    use super::super::item_view::{ItemViewContinuity, SessionItemView};
    use super::super::world_map::{MapUpdate, SessionMapView};
    use super::*;
    use crate::map::boot::BundleWorld;
    use crate::map::boot::tests::{DOOR_AT, door_key, door_world};
    use oteryn_protocol_oteryn::world_map::{
        MapOrigin, MapViewHeader, decode_world_map_delta, decode_world_map_snapshot,
    };
    use oteryn_protocol_oteryn::world_spatial::ActorPosition;

    const GENERATION: [u8; 32] = [9; 32];

    fn target(digest: [u8; 32], key: u64, revision: u64) -> WorldObjectTarget {
        let mut placement = digest.to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        WorldObjectTarget {
            placement,
            expected_revision: revision,
        }
    }

    fn at(x: u16) -> TilePos {
        TilePos { x, y: 0, floor: -7 }
    }

    fn run(
        world: &BundleWorld,
        actor: TilePos,
        others: &[TilePos],
        t: &WorldObjectTarget,
    ) -> UseOutcome {
        use_door(
            world.overlay(),
            world.facts(),
            GENERATION,
            actor,
            |pos| others.contains(&pos),
            t,
        )
    }

    #[test]
    fn reach_is_same_floor_chebyshev_one() {
        assert!(reachable(at(5), DOOR_AT) && reachable(at(6), DOOR_AT));
        assert!(!reachable(at(4), DOOR_AT));
        assert!(!reachable(TilePos { floor: -6, ..at(5) }, DOOR_AT));
        assert!(reachable(TilePos { y: 1, ..at(7) }, DOOR_AT));
    }

    #[test]
    fn use_toggles_a_door_once_per_revision_and_refuses_the_rest() {
        let world = door_world().expect("world");
        let digest = world.base().digest();
        let key = door_key();
        let go = |actor: TilePos, others: &[TilePos], revision: u64| {
            run(&world, actor, others, &target(digest, key, revision)).disposition
        };

        // Out of reach, then in reach.
        assert_eq!(go(at(3), &[], 0), UseDisposition::TooFar);
        assert!(!world.enterable(DOOR_AT));
        assert_eq!(go(at(7), &[at(7)], 0), UseDisposition::Committed);
        assert!(world.enterable(DOOR_AT));
        // The same revision again is stale, and nothing changed.
        assert_eq!(go(at(7), &[at(7)], 0), UseDisposition::StaleState);
        assert!(world.enterable(DOOR_AT));
        // An actor on the open door's tile keeps it open, whoever issues the USE.
        assert_eq!(go(at(7), &[at(7), DOOR_AT], 1), UseDisposition::Occupied);
        assert!(world.enterable(DOOR_AT));
        assert_eq!(go(at(7), &[at(7)], 1), UseDisposition::Committed);
        assert!(!world.enterable(DOOR_AT));
        assert_eq!(world.facts().doors().revision(key), 2);

        // Another bundle, an entry that is no door, a tile with no such entry, a short target.
        let other = run(&world, at(7), &[], &target([7; 32], key, 2));
        assert_eq!(other.disposition, UseDisposition::StaleState);
        let grass = crate::map::view::placement_key(DOOR_AT, 0).expect("key");
        let plain = run(&world, at(7), &[], &target(digest, grass, 0));
        assert_eq!(plain.disposition, UseDisposition::NothingToUse);
        let missing = crate::map::view::placement_key(at(2), 5).expect("key");
        let none = run(&world, at(2), &[], &target(digest, missing, 0));
        assert_eq!(none.disposition, UseDisposition::NothingToUse);
        let short = WorldObjectTarget {
            placement: vec![0; 8],
            expected_revision: 0,
        };
        let bad = run(&world, at(7), &[], &short);
        assert_eq!(bad.disposition, UseDisposition::NothingToUse);
        assert!(bad.committed.is_none());
    }

    /// The client path at the wire level: two sessions join and see the closed door, one reads
    /// the door's ordinal and object revision from domain 17 and sends them back as a map target;
    /// both sessions then receive the open door in their next map delta with revision 1, and the
    /// echo of the revision they saw closes it again.
    #[test]
    fn a_client_uses_the_door_it_saw_and_every_session_sees_the_change() {
        let world = door_world().expect("world");
        let source = MapViewSource {
            overlay: world.overlay(),
            facts: world.facts(),
            content_generation: GENERATION,
            reset_epoch: 0,
        };
        let actor = ActorPosition {
            x: 7,
            y: 0,
            floor: -7,
        };
        let door_tile = ActorPosition {
            x: 6,
            y: 0,
            floor: -7,
        };
        let session = || {
            (
                SessionMapView::default(),
                SessionItemView::resume(ItemViewContinuity::default()).with_map_view(),
            )
        };
        let door_item = |tile: &oteryn_protocol_oteryn::world_map::MapTile| {
            *tile.items.get(1).expect("the door is the second entry")
        };
        let (mut first, mut first_items) = session();
        let (mut second, mut second_items) = session();
        let joined = first
            .snapshot(&mut first_items, &source, actor)
            .expect("snapshot");
        second
            .snapshot(&mut second_items, &source, actor)
            .expect("snapshot");
        let snapshot = decode_world_map_snapshot(&joined.payload).expect("decodes");
        let header: MapViewHeader = snapshot.header;
        let tile = snapshot
            .tiles
            .iter()
            .find(|tile| tile.position == door_tile)
            .expect("the door tile");
        let seen = door_item(tile);
        assert_eq!(seen.appearance_id, 1629);
        let MapOrigin::BaseOrdinal {
            ordinal,
            object_revision,
        } = seen.origin
        else {
            panic!("a door is a base entry without a handle");
        };
        assert_eq!((ordinal, object_revision), (1, 0));

        // The client builds the 40-byte target from what it saw.
        let key = crate::map::view::placement_key(DOOR_AT, ordinal).expect("key");
        let used = run(
            &world,
            at(7),
            &[at(7)],
            &target(header.bundle_digest, key, object_revision),
        );
        assert_eq!(used.disposition, UseDisposition::Committed);

        for (view, items) in [
            (&mut first, &mut first_items),
            (&mut second, &mut second_items),
        ] {
            let Some(MapUpdate::Delta(delta)) = view.update(items, &source, actor).expect("update")
            else {
                panic!("the door change is a map delta");
            };
            let decoded = decode_world_map_delta(&header, &delta.payload).expect("decodes");
            let tile = decoded
                .tiles
                .iter()
                .find(|tile| tile.position == door_tile)
                .expect("the changed tile");
            let open = door_item(tile);
            assert_eq!(open.appearance_id, 1630);
            assert_eq!(
                open.origin,
                MapOrigin::BaseOrdinal {
                    ordinal: 1,
                    object_revision: 1
                }
            );
            // Nothing else about the window changed.
            assert_eq!(decoded.tiles.len(), 1);
        }

        // The stale echo of revision 0 is refused; the revision just seen closes the door.
        let stale = run(&world, at(7), &[], &target(header.bundle_digest, key, 0));
        assert_eq!(stale.disposition, UseDisposition::StaleState);
        let closed = run(&world, at(7), &[], &target(header.bundle_digest, key, 1));
        assert_eq!(closed.disposition, UseDisposition::Committed);
        let Some(MapUpdate::Delta(delta)) = first
            .update(&mut first_items, &source, actor)
            .expect("update")
        else {
            panic!("the door change is a map delta");
        };
        let decoded = decode_world_map_delta(&header, &delta.payload).expect("decodes");
        let back = door_item(&decoded.tiles[0]);
        assert_eq!(back.appearance_id, 1629);
        assert_eq!(
            back.origin,
            MapOrigin::BaseOrdinal {
                ordinal: 1,
                object_revision: 2
            }
        );
    }

    /// A Channel runtime with one player at native `(5, 0, -7)`, beside [`DOOR_AT`], and a rat
    /// companion policy.
    fn runtime_beside_the_door() -> (
        ChannelRuntimeV1,
        ExactActorRef,
        crate::foundation::GameSessionId,
    ) {
        use crate::foundation::{
            ChannelContentPin, ChannelId, CompiledCreaturePolicies, CompiledCreaturePolicy,
            CreatureFlags, GameSessionId, NodeId, WorldId,
        };
        let id = |tag: u8| [1, 0x92, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag];
        let world = WorldId::decode(&id(1)).expect("world");
        let pin = ChannelContentPin::test(world);
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).expect("channel"),
            NodeId::decode(&id(3)).expect("node"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            pin.clone(),
        )
        .expect("runtime");
        let rat = CompiledCreaturePolicy {
            definition_key: "fixture:rat".into(),
            definition_revision: "fixture:1".into(),
            display_name: "fixture Rat".into(),
            maximum_health: 20,
            base_speed: 110,
            outfit_look_type: 21,
            object_look_type: None,
            summonable: false,
            convinceable: false,
            mana_cost: None,
            is_familiar: false,
            condition_immunities: vec![],
            armor: Some(1),
            mitigation: None,
            resistances: vec![],
            damage_immunities: vec![],
            healing_from_damage: vec![],
            preferred_distance: Some(1),
            reward_boss: Some(false),
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
        };
        runtime
            .install_companion_policies(
                CompiledCreaturePolicies::from_active_artifact(
                    pin.server_artifact_digest(),
                    vec![rat],
                )
                .expect("policies"),
            )
            .expect("installed");
        let session = GameSessionId::decode(&id(4)).expect("session");
        let reserved = runtime.reserve_fresh_session(session).expect("reserve");
        let actor = runtime.commit_fresh_session(reserved).expect("commit");
        runtime
            .initialize_pinned_test_position(actor, movement(at(5)))
            .expect("position");
        (runtime, actor, session)
    }

    fn movement(tile: TilePos) -> MovementLocalPosition {
        MovementLocalPosition {
            x: i32::from(tile.x),
            y: i32::from(tile.y),
            floor: -i16::from(tile.floor),
        }
    }

    /// Codex 4239177729: a companion whose spawn is reserved for an unresolved durable COMMIT
    /// holds the doorway like a live actor, so the open door does not close on it; the census
    /// is the carrier's own collision census, so a live creature and the acting player count too.
    #[test]
    fn a_reserved_companion_or_a_creature_keeps_the_open_door_open() {
        let world = door_world().expect("world");
        let digest = world.base().digest();
        let key = door_key();
        let (mut runtime, actor, session) = runtime_beside_the_door();
        let actor_tile = at(5);
        let go = |runtime: &ChannelRuntimeV1, revision: u64| {
            use_door(
                world.overlay(),
                world.facts(),
                GENERATION,
                actor_tile,
                |door| doorway_occupied(runtime, actor, actor_tile, door),
                &target(digest, key, revision),
            )
            .disposition
        };
        assert!(!doorway_occupied(&runtime, actor, actor_tile, DOOR_AT));
        assert!(doorway_occupied(&runtime, actor, DOOR_AT, DOOR_AT));
        assert_eq!(go(&runtime, 0), UseDisposition::Committed);

        // The companion's spawn is reserved on the open doorway: it is not visible yet, but
        // the doorway is occupied and the door refuses to close.
        let mut prepared = runtime
            .prepare_companion_spawn(
                actor,
                session,
                "fixture:rat",
                movement(DOOR_AT),
                None,
                None,
                0,
                0,
                false,
            )
            .expect("prepared");
        runtime
            .reserve_companion_spawn(&mut prepared)
            .expect("reserved");
        assert!(runtime.visible_entities().creatures.is_empty());
        assert!(doorway_occupied(&runtime, actor, actor_tile, DOOR_AT));
        assert_eq!(go(&runtime, 1), UseDisposition::Occupied);
        assert!(world.enterable(DOOR_AT));

        // Once the reservation rolls back, nobody stands there and the door closes.
        runtime
            .rollback_companion_spawn(&prepared)
            .expect("rolled back");
        assert!(!doorway_occupied(&runtime, actor, actor_tile, DOOR_AT));

        // A live creature on the doorway holds it as well.
        runtime
            .realize_native_qualification_spawn(
                runtime.content_pin().clone(),
                "fixture:rat",
                movement(DOOR_AT),
            )
            .expect("rat");
        assert_eq!(go(&runtime, 1), UseDisposition::Occupied);
        assert!(world.enterable(DOOR_AT));
    }

    #[test]
    fn the_world_action_gate_applies_the_cooldown_and_the_tick_cap() {
        use std::time::Duration;
        let (_runtime, actor, _) = runtime_beside_the_door();
        let ms = Duration::from_millis;
        let mut gate = WorldActionGate::default();
        assert!(gate.admit(actor, ms(1_000)));
        // Inside the 200 ms cooldown: refused, and the refusal does not restart it.
        assert!(!gate.admit(actor, ms(1_100)));
        assert!(!gate.admit(actor, ms(1_199)));
        assert!(gate.admit(actor, ms(1_200)));
        // The per-tick cap: the cooldown is per actor, the cap per Channel.
        let mut gate = WorldActionGate {
            admitted: WORLD_ACTIONS_PER_TICK,
            tick: Duration::from_millis(5_000).as_nanos() / SIMULATION_TICK.as_nanos(),
            ..WorldActionGate::default()
        };
        assert!(!gate.admit(actor, ms(5_010)));
        assert!(gate.admit(actor, ms(5_100)));
    }
}
