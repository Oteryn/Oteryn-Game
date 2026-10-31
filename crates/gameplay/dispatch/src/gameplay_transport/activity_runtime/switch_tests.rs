//! The committed native destination is recoverable before any scope-barrier byte.
#![allow(clippy::expect_used)]
use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{
    AuthenticatedTransportRefV1, ChannelId, ConnectionGeneration, NodeId, ScopeOwnershipGeneration,
    WorldId,
};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    ControllerBinding, FirstEntryOutcome, SessionContinuity,
};
use oteryn_protocol_oteryn::activity_scope::{
    ActivityScope, ActivityScopeSnapshot, encode_activity_scope,
};

fn id(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

fn native_switch() -> ActivityConnectionSwitch {
    let world = WorldId::decode(&id(1)).expect("world");
    let channel = ChannelId::decode(&id(2)).expect("channel");
    let node = NodeId::decode(&id(4)).expect("node");
    let source_scope = RuntimeScopeRefV1::channel(world, channel);
    let destination_scope = RuntimeScopeRefV1::instance(world, id(3)).expect("Instance");
    let source_session = GameSessionId::decode(&id(6)).expect("source session");
    let destination_session = GameSessionId::decode(&id(7)).expect("destination session");
    let (_controller, activated) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let pin = activated.into_channel_pin();
    let mut source_runtime = ChannelRuntimeV1::from_committed_scope_assignment(
        source_scope,
        node,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        1,
        pin.clone(),
    )
    .expect("source owner");
    let mut destination_runtime = ChannelRuntimeV1::from_committed_scope_assignment(
        destination_scope,
        node,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        1,
        pin,
    )
    .expect("independent Instance owner");
    let source_reservation = source_runtime
        .reserve_fresh_session(source_session)
        .expect("source reservation");
    let source_actor = source_runtime
        .commit_fresh_session(source_reservation)
        .expect("source actor");
    let destination_reservation = destination_runtime
        .reserve_fresh_session(destination_session)
        .expect("destination reservation");
    let destination_actor = destination_runtime
        .commit_fresh_session(destination_reservation)
        .expect("destination actor");
    let fence = CurrentCharacterItemFence {
        character_id: crate::domain::CharacterId::from_bytes(id(5)).expect("character"),
        game_session_id: source_session,
        connection_generation: ConnectionGeneration::new(1).expect("connection generation"),
        character_lease_generation: 2,
        runtime_scope: source_scope,
        scope_ownership_generation: ScopeOwnershipGeneration::new(1).expect("scope generation"),
    };
    let selected = SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[117])
        .expect("boss selection");
    let source = AdmittedSession {
        game_session_id: source_session,
        world_id: world,
        channel_id: channel,
        runtime_actor: Some(source_actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(ControllerBinding {
            transport: AuthenticatedTransportRefV1::decode(&id(8)).expect("source transport"),
            account_id: id(5),
        }),
        item_fence: Some(fence),
        continuity: SessionContinuity {
            selected_capabilities: selected,
            ..SessionContinuity::FRESH
        },
    };
    let destination = AdmittedSession {
        game_session_id: destination_session,
        runtime_actor: Some(destination_actor),
        controller: Some(ControllerBinding {
            transport: AuthenticatedTransportRefV1::decode(&id(9))
                .expect("fresh destination transport"),
            account_id: id(5),
        }),
        item_fence: Some(CurrentCharacterItemFence {
            game_session_id: destination_session,
            runtime_scope: destination_scope,
            character_lease_generation: 3,
            ..fence
        }),
        ..source
    };
    let payload = encode_activity_scope(&ActivityScopeSnapshot {
        handoff: id(10),
        source_session: *source_session.as_bytes(),
        session: *destination_session.as_bytes(),
        scope: ActivityScope::Instance {
            world: *world.as_bytes(),
            instance: id(3),
        },
        scope_generation: 1,
        connection_generation: 1,
        lease_generation: 3,
        selection: selected.as_slice().to_vec(),
    })
    .expect("committed scope projection");
    ActivityConnectionSwitch {
        source,
        destination,
        payload,
    }
}

#[test]
fn committed_destination_is_visible_before_barrier_without_resurrecting_old_control() {
    let registry = ActivityRuntimeRegistry::default();
    let switch = native_switch();
    registry.remember(switch.source).expect("source connection");
    registry
        .stage_switch(switch.clone())
        .expect("committed native publication");
    // No finish_switch has occurred: the peer may have received zero bytes.
    assert_eq!(
        registry.connection(switch.destination.game_session_id),
        Some(switch.destination),
        "verified recovery and autonomous lifecycle can locate the actual destination"
    );
    assert_eq!(
        registry.connection(switch.source.game_session_id),
        Some(switch.source),
        "the source stream still owns its pending barrier"
    );
    assert!(
        registry
            .pending_switch(switch.source)
            .expect("pending barrier")
            .is_some()
    );
    registry
        .finish_switch(switch.source, switch.destination)
        .expect("wire barrier completed");
    assert!(registry.connection(switch.source.game_session_id).is_none());

    let mut recovered = switch.destination;
    recovered.continuity.connection_generation = 2;
    recovered
        .item_fence
        .as_mut()
        .expect("fence")
        .connection_generation = ConnectionGeneration::new(2).expect("recovered generation");
    recovered.controller.as_mut().expect("controller").transport =
        AuthenticatedTransportRefV1::decode(&id(11)).expect("recovered transport");
    registry
        .remember(recovered)
        .expect("later verified controller");
    registry
        .stage_switch(switch.clone())
        .expect("historical publication retry");
    assert_eq!(
        registry.connection(recovered.game_session_id),
        Some(recovered),
        "idempotent publication cannot replace a later connection with generation one"
    );
    assert!(
        registry
            .pending_switch(switch.source)
            .expect("completed barrier")
            .is_none()
    );

    registry.forget_terminal_connection(
        recovered.game_session_id,
        recovered.runtime_actor.expect("actor"),
    );
    registry
        .stage_switch(switch)
        .expect("historical retry after terminal release");
    assert!(
        registry.connection(recovered.game_session_id).is_none(),
        "a committed historical receipt cannot resurrect terminal control"
    );
}
