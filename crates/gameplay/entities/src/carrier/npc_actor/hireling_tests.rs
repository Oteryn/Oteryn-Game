#![allow(clippy::expect_used)] // Existing carrier fixture helpers construct accepted authority.
use super::*;
use crate::carrier::house_custody::HouseActorState;
use crate::carrier::tests::{runtime, session};
use crate::carrier::{CharacterId, CharacterLease};

fn uuid(seed: u8) -> [u8; 16] {
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = seed;
    id
}
fn resident(
    runtime: &mut ChannelRuntimeV1,
    seed: u8,
    house: &str,
    house_id: u8,
    floor: i16,
) -> (ExactActorRef, GameSessionId) {
    let session = session(u64::from(seed));
    let lease = CharacterLease::new(CharacterId::decode(&uuid(seed)).expect("character"), 1)
        .expect("lease");
    let position = LocalPosition {
        x: 100,
        y: 100,
        floor,
    };
    let state = HouseActorState {
        house: house.into(),
        scope: RuntimeScopeRefV1::Instance {
            world_id: runtime.binding.world_id,
            instance_id: uuid(house_id),
        },
        generation: ScopeOwnershipGeneration::new(1).expect("generation"),
        position,
        lease,
        origin_position: LocalPosition {
            x: 0,
            y: 0,
            floor: 0,
        },
    };
    let actor = runtime
        .restore_transfer_actor(
            crate::carrier::tests::session(100 + u64::from(seed)),
            session,
            lease,
            position,
            Some(state),
        )
        .expect("restored current House custody");
    (actor, session)
}
fn projection(seed: u8, floor: i8) -> HirelingProjection {
    let hex: String = uuid(seed).iter().map(|b| format!("{b:02x}")).collect();
    HirelingProjection {
        key: format!("oteryn:npc.hireling:{hex}"),
        name: "Alex Moon".into(),
        placement: NpcPlacement {
            x: 101,
            y: 100,
            floor,
            direction: MovementFacing::South,
        },
        look_type: 1110,
        colors: [1, 2, 3, 4],
    }
}

#[test]
fn hireling_projection_is_scoped_to_current_house_and_native_floor() {
    let mut runtime = runtime(12);
    let (first, session_a) = resident(&mut runtime, 40, "house:a", 80, 7);
    let (other, session_b) = resident(&mut runtime, 41, "house:b", 81, 7);
    let (upstairs, session_up) = resident(&mut runtime, 42, "house:a", 80, 8);
    let value = projection(1, -7);
    let actor = runtime
        .reconcile_house_hirelings(first, session_a, std::slice::from_ref(&value))
        .expect("reconcile")[0];
    let entities = runtime
        .house_hireling_entities(first, session_a)
        .expect("House entities");
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].position.floor, 7);
    assert_eq!(
        (
            entities[0].name.as_str(),
            entities[0].look_type,
            entities[0].colors
        ),
        ("Alex Moon", 1110, [1, 2, 3, 4])
    );
    assert_eq!(
        runtime.house_hireling_key(first, session_a, actor),
        Some(value.key.as_str())
    );
    assert_eq!(runtime.house_hireling_key(other, session_b, actor), None);
    assert_eq!(
        runtime.house_hireling_key(upstairs, session_up, actor),
        None
    );
    assert_eq!(runtime.house_hireling_key(first, session_b, actor), None);
    assert!(
        runtime
            .house_hireling_entities(other, session_b)
            .expect("other House")
            .is_empty()
    );
    assert!(
        runtime
            .house_hireling_entities(upstairs, session_up)
            .expect("other floor")
            .is_empty()
    );
    assert!(runtime.visible_entities().npcs.is_empty());
    assert_eq!(runtime.npc_key(actor), None);
    assert!(runtime.borrow_movement_position().read(actor).is_err());
    assert!(!runtime.carrier.cell_occupied(
        runtime.pinned_position_context(),
        LocalPosition {
            x: 101,
            y: 100,
            floor: 7
        }
    ));
    assert_eq!(
        runtime.house_hireling_cell_occupied(
            first,
            session_a,
            LocalPosition {
                x: 101,
                y: 100,
                floor: 7
            }
        ),
        Ok(true)
    );
    assert_eq!(
        runtime.house_hireling_cell_occupied(
            other,
            session_b,
            LocalPosition {
                x: 101,
                y: 100,
                floor: 7
            }
        ),
        Ok(false)
    );
}

#[test]
fn hireling_projection_replay_preserves_actor_and_outfit_update_advances_once() {
    let mut runtime = runtime(6);
    let (observer, session) = resident(&mut runtime, 43, "house:a", 80, 7);
    let mut value = projection(1, -7);
    let first = runtime
        .reconcile_house_hirelings(observer, session, std::slice::from_ref(&value))
        .expect("first");
    let before = runtime.carrier.slots.clone();
    let replay = runtime
        .reconcile_house_hirelings(observer, session, std::slice::from_ref(&value))
        .expect("replay");
    assert_eq!(replay, first);
    assert_eq!(runtime.carrier.slots, before);
    value.name = "Morgan".into();
    value.look_type = 1126;
    value.colors = [5, 6, 7, 8];
    assert_eq!(
        runtime
            .reconcile_house_hirelings(observer, session, std::slice::from_ref(&value))
            .expect("update"),
        first
    );
    let view = runtime
        .house_hireling_entities(observer, session)
        .expect("view");
    assert_eq!(
        (
            view[0].name.as_str(),
            view[0].look_type,
            view[0].colors,
            view[0].revision
        ),
        ("Morgan", 1126, [5, 6, 7, 8], 2)
    );
    runtime
        .reconcile_house_hirelings(observer, session, std::slice::from_ref(&value))
        .expect("update replay");
    assert_eq!(
        runtime
            .house_hireling_entities(observer, session)
            .expect("view"),
        view
    );
}

#[test]
fn equal_display_names_keep_distinct_house_actor_identity() {
    let mut runtime = runtime(4);
    let (observer, session) = resident(&mut runtime, 47, "house:a", 80, 7);
    let first = projection(1, -7);
    let mut second = projection(2, -7);
    second.placement.x += 1;
    let actors = runtime
        .reconcile_house_hirelings(observer, session, &[first.clone(), second.clone()])
        .expect("same names are valid");
    assert_ne!(actors[0], actors[1]);
    let view = runtime
        .house_hireling_entities(observer, session)
        .expect("view");
    assert_eq!(view.len(), 2);
    assert_eq!(view[0].name, view[1].name);
    assert_ne!(view[0].key, view[1].key);
    assert_eq!(
        runtime.house_hireling_key(observer, session, actors[0]),
        Some(first.key.as_str())
    );
    assert_eq!(
        runtime.house_hireling_key(observer, session, actors[1]),
        Some(second.key.as_str())
    );
    runtime
        .reconcile_house_hirelings(observer, session, std::slice::from_ref(&second))
        .expect("dismiss by durable identity");
    assert_eq!(
        runtime.house_hireling_key(observer, session, actors[0]),
        None
    );
    assert_eq!(
        runtime.house_hireling_key(observer, session, actors[1]),
        Some(second.key.as_str())
    );
}

#[test]
fn hireling_dismiss_preserves_other_house_and_static_npcs_and_invalidates_old_handle() {
    let mut runtime = runtime(8);
    let static_actor = runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: "oteryn:npc.static".into(),
                    placements: vec![NpcPlacement {
                        x: 101,
                        y: 100,
                        floor: -7,
                        direction: MovementFacing::North,
                    }],
                }],
            },
            |_| true,
            true,
        )
        .expect("static")
        .actors[0];
    let (a, sa) = resident(&mut runtime, 44, "house:a", 80, 7);
    let (b, sb) = resident(&mut runtime, 45, "house:b", 81, 7);
    let value = projection(1, -7);
    let other = projection(2, -7);
    let actor = runtime
        .reconcile_house_hirelings(a, sa, std::slice::from_ref(&value))
        .expect("A")[0];
    let actor_b = runtime
        .reconcile_house_hirelings(b, sb, std::slice::from_ref(&other))
        .expect("B")[0];
    assert!(runtime.remove_dynamic_npc(&value.key).is_err());
    runtime
        .reconcile_house_hirelings(a, sa, &[])
        .expect("dismiss");
    runtime
        .reconcile_house_hirelings(a, sa, &[])
        .expect("dismiss replay");
    assert_eq!(runtime.house_hireling_key(a, sa, actor), None);
    assert_eq!(
        runtime.house_hireling_key(b, sb, actor_b),
        Some(other.key.as_str())
    );
    assert_eq!(runtime.npc_key(static_actor), Some("oteryn:npc.static"));
    assert_eq!(runtime.visible_entities().npcs.len(), 1);
    let replacement = runtime
        .reconcile_house_hirelings(a, sa, std::slice::from_ref(&value))
        .expect("restore")[0];
    assert_ne!(replacement, actor);
    assert_eq!(runtime.house_hireling_key(a, sa, actor), None);
}

#[test]
fn hireling_invalid_floor_duplicate_tile_and_capacity_leave_projection_unchanged() {
    let mut runtime = runtime(2);
    let (observer, session) = resident(&mut runtime, 46, "house:a", 80, 7);
    let value = projection(1, -7);
    runtime
        .reconcile_house_hirelings(observer, session, std::slice::from_ref(&value))
        .expect("first");
    let before = runtime.carrier.slots.clone();
    let mut bad = value.clone();
    bad.placement.floor = 1;
    assert!(
        runtime
            .reconcile_house_hirelings(observer, session, &[bad])
            .is_err()
    );
    assert_eq!(runtime.carrier.slots, before);
    assert!(
        runtime
            .reconcile_house_hirelings(observer, session, &[value.clone(), projection(2, -7)])
            .is_err()
    );
    assert_eq!(runtime.carrier.slots, before);
    let mut second = projection(2, -7);
    second.placement.x += 1;
    assert!(
        runtime
            .reconcile_house_hirelings(observer, session, &[value, second])
            .is_err()
    );
    assert_eq!(runtime.carrier.slots, before);
}

#[test]
fn last_committed_house_exit_retires_only_that_projection_and_reuses_capacity() {
    let mut runtime = runtime(6);
    let static_actor = runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: "oteryn:npc.static".into(),
                    placements: vec![NpcPlacement {
                        x: 101,
                        y: 100,
                        floor: -7,
                        direction: MovementFacing::North,
                    }],
                }],
            },
            |_| true,
            true,
        )
        .expect("static")
        .actors[0];
    let (a, sa) = resident(&mut runtime, 50, "house:a", 80, 7);
    let (roommate, sr) = resident(&mut runtime, 51, "house:a", 80, 7);
    let (b, sb) = resident(&mut runtime, 52, "house:b", 81, 7);
    let first = projection(1, -7);
    let other = projection(2, -7);
    let old = runtime
        .reconcile_house_hirelings(a, sa, std::slice::from_ref(&first))
        .expect("House A")[0];
    let kept = runtime
        .reconcile_house_hirelings(b, sb, std::slice::from_ref(&other))
        .expect("House B")[0];
    assert_eq!(
        runtime.carrier.occupied.len(),
        6,
        "fixture reaches the fixed carrier capacity"
    );
    let exit = |runtime: &mut ChannelRuntimeV1, actor, source, destination| {
        let state = runtime
            .house_actor_state(actor, source)
            .expect("current custody");
        let lease = CharacterLease::new(state.lease.character_id(), state.lease.generation() + 1)
            .expect("successor lease");
        runtime
            .commit_house_custody(
                actor,
                source,
                destination,
                lease,
                &state.house,
                state.scope,
                state.generation,
                LocalPosition {
                    x: 0,
                    y: 0,
                    floor: 7,
                },
                true,
            )
            .expect("committed exit");
    };
    let next_a = session(150);
    exit(&mut runtime, a, sa, next_a);
    assert_eq!(
        runtime.house_hireling_key(roommate, sr, old),
        Some(first.key.as_str()),
        "another current resident retains the same projected actor"
    );
    exit(&mut runtime, roommate, sr, session(151));
    assert_eq!(runtime.carrier.occupied.len(), 5);
    assert_eq!(
        runtime.house_hireling_key(b, sb, kept),
        Some(other.key.as_str())
    );
    assert_eq!(runtime.npc_key(static_actor), Some("oteryn:npc.static"));
    assert_eq!(runtime.visible_entities().npcs.len(), 1);
    // Re-entering another House does not reserve a second player slot. Its newly read NPC can
    // now consume the released projection slot, with a new generation invalidating the old ref.
    let next = session(152);
    let lease =
        CharacterLease::new(CharacterId::decode(&uuid(50)).expect("character"), 3).expect("lease");
    runtime
        .commit_house_custody(
            a,
            next_a,
            next,
            lease,
            "house:c",
            RuntimeScopeRefV1::Instance {
                world_id: runtime.binding.world_id,
                instance_id: uuid(82),
            },
            ScopeOwnershipGeneration::new(1).expect("scope"),
            LocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            false,
        )
        .expect("new House");
    let replacement = runtime
        .reconcile_house_hirelings(a, next, &[projection(3, -7)])
        .expect("capacity was released")[0];
    assert_ne!(replacement, old);
    assert_eq!(runtime.house_hireling_key(a, next, old), None);
    assert_eq!(
        runtime.house_hireling_key(b, sb, kept),
        Some(other.key.as_str())
    );
}
