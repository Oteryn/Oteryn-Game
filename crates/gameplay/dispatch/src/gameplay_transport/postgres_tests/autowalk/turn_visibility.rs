//! A TURN must publish the exact caster to both negotiated spatial streams.
use super::super::TestResult;
use super::super::autowalk_fixtures::{at, read};
use super::super::fixture::{debug, id};
use super::super::kill_reward_e2e_fixtures::scenario_thread;
use super::super::target_move::{sees, turn, update_in};
use super::{Scenario, scenario};
use crate::durability::DurabilityRoot;
use crate::foundation::{
    ExactActorRef, GameSessionId, MovementFacing, MovementLocalPosition, NpcPlacement,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_snapshot_with_item_handles;
use oteryn_protocol_oteryn::{MessageType, decode_snapshot_chunk, decode_wire_envelope};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, DuplexStream};

#[test]
fn c18a_turn_publishes_before_receipt_and_reaches_observer_on_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario_thread::run("c18a-turn-visibility", || {
        scenario(Scenario::TurnVisibility)
    })
}

pub(super) async fn qualify(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let caster = admitted.runtime_actor.ok_or("caster")?;
    let observer_session = GameSessionId::decode(&id(77)).map_err(debug)?;
    let cell = at(1, 1);
    let (observer, npc) = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime
            .reserve_fresh_session(observer_session)
            .map_err(debug)?;
        let observer = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(observer, at(2, 1))
            .map_err(debug)?;
        let mut movement = runtime.borrow_movement_position();
        let start = movement.read(caster).map_err(debug)?;
        movement
            .commit_turn(start, MovementFacing::East)
            .map_err(debug)?;
        let npc = runtime
            .place_dynamic_npc(
                "oteryn:npc.cipfried",
                NpcPlacement {
                    floor: -7,
                    x: 2,
                    y: 4,
                    direction: MovementFacing::East,
                },
            )
            .map_err(debug)?;
        (observer, npc)
    };
    let mut peer = AdmittedSession {
        game_session_id: observer_session,
        runtime_actor: Some(observer),
        controller: None,
        item_fence: None,
        ..admitted
    };
    peer.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[6])
            .ok_or("observer selection")?;

    let (mut caster_server, mut caster_client) = tokio::io::duplex(1 << 20);
    let (mut observer_server, mut observer_client) = tokio::io::duplex(1 << 20);
    let driving = async move {
        snapshot(&mut caster_client, caster, cell).await?;
        snapshot(&mut observer_client, caster, cell).await?;
        // These are separate commands with separate commits, even for the second West.
        for command in [1, 2] {
            turn(
                &mut caster_client,
                command,
                caster,
                cell,
                StepDirection::West,
            )
            .await?;
            observe_update(&mut observer_client, caster, cell, StepDirection::West).await?;
        }
        // NPC steps also write facing in the Movement owner. Its exact actor must
        // reach the same visibility stream, including the new cardinal direction.
        let npc_cell = at(2, 3);
        {
            let mut runtime = owner.runtime.lock().await;
            let mut movement = runtime.borrow_movement_position();
            let before = movement.read(npc).map_err(debug)?;
            movement.commit_cardinal(before, npc_cell).map_err(debug)?;
        }
        observe_update(&mut observer_client, npc, npc_cell, StepDirection::North).await?;
        caster_client.shutdown().await?;
        observer_client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (caster_end, peer_end, driving) = tokio::join!(
        DurabilityRoot::with_holder_owner(serve_admitted(
            &mut caster_server,
            admitted,
            owner,
            IDLE_LIVENESS
        )),
        DurabilityRoot::with_holder_owner(serve_admitted(
            &mut observer_server,
            peer,
            owner,
            IDLE_LIVENESS
        )),
        driving,
    );
    driving?;
    for end in [caster_end, peer_end] {
        assert!(
            matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
            "{end:?}"
        );
    }
    eprintln!("POSTGRES_CASE_EXECUTED c18a_turn_publishes_before_receipt_and_reaches_observer");
    Ok(())
}

async fn observe_update(
    client: &mut DuplexStream,
    actor: ExactActorRef,
    cell: MovementLocalPosition,
    facing: StepDirection,
) -> TestResult {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let frame = read(client).await?;
            let envelope = decode_wire_envelope(&frame)?;
            if update_in(&envelope, actor, cell, facing)? {
                return Ok(());
            }
        }
    })
    .await?
}

async fn snapshot(
    client: &mut DuplexStream,
    caster: ExactActorRef,
    cell: MovementLocalPosition,
) -> TestResult {
    let mut saw_caster = false;
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        match envelope.message_type() {
            MessageType::SnapshotChunk => {
                for domain in decode_snapshot_chunk(envelope.payload())?.1 {
                    if domain.domain_id == 1 {
                        let spatial = decode_world_spatial_entities_snapshot_with_item_handles(
                            domain.payload,
                        )
                        .map_err(debug)?;
                        saw_caster |= sees(&spatial.entities, caster, cell, StepDirection::East);
                    }
                }
            }
            MessageType::SnapshotCommit => {
                assert!(
                    saw_caster,
                    "the join snapshot shows the exact caster facing East"
                );
                return Ok(());
            }
            _ => {}
        }
    }
}
