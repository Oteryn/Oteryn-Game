#![allow(
    clippy::expect_used,
    reason = "accepted native House fixture authority"
)]
use super::*;
use crate::foundation::{
    CharacterId, CharacterLease, HirelingProjection, LocalPosition, MovementFacing, NpcPlacement,
};
use crate::gameplay_transport::actor_spell::tests::runtime_with_capacity;
use crate::gameplay_transport::chat_intent::ChatRuntime;
use oteryn_gameplay_entities::carrier::house_custody::HouseActorState;
use oteryn_protocol_oteryn::hireling::management::Reply;

fn id(seed: u8) -> [u8; 16] {
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
) -> (ExactActorRef, GameSessionId) {
    let session = GameSessionId::decode(&id(seed)).expect("session");
    let source = GameSessionId::decode(&id(seed + 100)).expect("source session");
    let lease =
        CharacterLease::new(CharacterId::decode(&id(seed)).expect("character"), 1).expect("lease");
    let position = LocalPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    let state = HouseActorState {
        house: house.into(),
        scope: RuntimeScopeRefV1::Instance {
            world_id: runtime.binding().world_id(),
            instance_id: id(seed + 50),
        },
        generation: ScopeOwnershipGeneration::new(1).expect("generation"),
        position,
        lease,
        origin_position: LocalPosition {
            x: 0,
            y: 0,
            floor: 7,
        },
    };
    let actor = runtime
        .restore_transfer_actor(source, session, lease, position, Some(state))
        .expect("restore House");
    (actor, session)
}
fn hireling(seed: u8) -> HirelingProjection {
    HirelingProjection {
        key: crate::durability::house_hireling::npc_key(id(seed)),
        name: "Alex Moon".into(),
        placement: NpcPlacement {
            x: 101,
            y: 100,
            floor: -7,
            direction: MovementFacing::West,
        },
        look_type: 1110,
        colors: [1, 2, 3, 4],
    }
}

#[test]
fn scoped_house_snapshot_and_projection_bind_actual_npc_and_exclude_channel_and_other_house()
-> Result<(), &'static str> {
    let (mut runtime, channel_actor, _) = runtime_with_capacity(0x71, 8);
    let (a, sa) = resident(&mut runtime, 40, "house:a");
    let (b, sb) = resident(&mut runtime, 41, "house:b");
    let npc_a = runtime
        .reconcile_house_hirelings(a, sa, &[hireling(1)])
        .expect("A NPC")[0];
    let npc_b = runtime
        .reconcile_house_hirelings(b, sb, &[hireling(2)])
        .expect("B NPC")[0];
    let view = entities(&runtime, a, 1)
        .expect("view authority")
        .expect("House view");
    assert_eq!(view.entities.len(), 2);
    assert!(
        view.entities
            .iter()
            .any(|entry| entry.identity == npc_a.placement_identity()
                && entry.kind == world_spatial::VisibleKind::Hireling
                && entry.direction == StepDirection::West)
    );
    assert!(
        !view
            .entities
            .iter()
            .any(|entry| entry.identity == npc_b.placement_identity()
                || entry.identity == channel_actor.placement_identity())
    );
    let reply = ComposedFreshAdmission::hireling_actor_bindings(&runtime, a, sa).expect("bindings");
    let Reply::Projection(bindings) = &reply else {
        return Err("expected projection");
    };
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].hireling, id(1));
    assert_eq!(
        bindings[0].entity,
        EntityRef {
            identity: npc_a.placement_identity(),
            generation: npc_a.actor_local_generation()
        }
    );
    assert_eq!(Reply::decode(&reply.encode().expect("encode")), Some(reply));
    assert!(ComposedFreshAdmission::hireling_actor_bindings(&runtime, a, sb).is_none());
    let before = runtime.read_house_movement(a, sa).expect("House movement");
    let north = LocalPosition {
        x: 100,
        y: 99,
        floor: 7,
    };
    runtime
        .compare_move_house(&before, north, MovementFacing::North)
        .expect("House step");
    let moved = entities(&runtime, a, 2)
        .expect("view authority")
        .expect("House view");
    let player = moved
        .entities
        .iter()
        .find(|entry| entry.identity == a.placement_identity())
        .expect("current player");
    assert_eq!(player.direction, StepDirection::North);
    assert_eq!(
        player.position,
        ActorPosition {
            x: 100,
            y: 99,
            floor: 7
        }
    );
    assert_eq!(player.revision, before.revision() + 1);
    assert!(
        moved
            .entities
            .iter()
            .any(|entry| entry.identity == npc_a.placement_identity()
                && entry.direction == StepDirection::West)
    );
    Ok(())
}

#[test]
fn hireling_chat_keeps_current_house_customer_and_closes_other_house_or_dismissed_actor() {
    let (mut runtime, _, _) = runtime_with_capacity(0x72, 8);
    let (a, sa) = resident(&mut runtime, 40, "house:a");
    let (b, sb) = resident(&mut runtime, 41, "house:b");
    let npc_a = runtime
        .reconcile_house_hirelings(a, sa, &[hireling(1)])
        .expect("A NPC")[0];
    let npc_b = runtime
        .reconcile_house_hirelings(b, sb, &[hireling(2)])
        .expect("B NPC")[0];
    let chat = ChatRuntime::default();
    chat.admit(
        sa,
        "Aela",
        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1),
    );
    chat.converse(sa, |engaged| *engaged = Some(npc_a))
        .expect("conversation");
    assert!(!chat.close_out_of_range(&runtime, sa));
    assert_eq!(chat.engaged(sa), Some(npc_a));
    chat.converse(sa, |engaged| *engaged = Some(npc_b))
        .expect("other actor");
    assert!(chat.close_out_of_range(&runtime, sa));
    assert_eq!(chat.engaged(sa), None);
    chat.converse(sa, |engaged| *engaged = Some(npc_a))
        .expect("conversation");
    runtime
        .reconcile_house_hirelings(a, sa, &[])
        .expect("dismiss");
    assert!(chat.close_out_of_range(&runtime, sa));
    assert_eq!(chat.engaged(sa), None);
}
