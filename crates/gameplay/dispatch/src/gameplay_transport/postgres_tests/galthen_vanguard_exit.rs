use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, UseDisposition, WorldObjectTarget, decode_use_result,
    encode_use_intent,
};

#[test]
fn galthen_vanguard_use_returns_only_clicking_participant_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::m25_ahau_exit::vanguard_scenario()
}

pub(super) async fn exercise(
    h: &Harness,
    origin: &ComposedFreshAdmission<'_, '_, '_>,
    view: &mut ComposedFreshAdmission<'_, '_, '_>,
    mut participant: AdmittedSession,
    source_peer: AdmittedSession,
) -> TestResult {
    let peer = origin.activity_runtimes.current_connection(source_peer);
    assert_ne!(peer.game_session_id, source_peer.game_session_id);
    let scope = participant
        .item_fence
        .ok_or("Vanguard fence")?
        .runtime_scope;
    assert_eq!(peer.item_fence.ok_or("peer fence")?.runtime_scope, scope);
    let actor = participant.runtime_actor.ok_or("Vanguard actor")?;
    let tile = MovementLocalPosition {
        x: 34068,
        y: 31418,
        floor: 11,
    };
    let target = {
        let runtime = view.runtime.lock().await;
        view.wire_observation(&runtime, tile)
            .ok_or("exit position")?
    };
    let map = view.bundle_map().ok_or("published Instance map")?;
    let key =
        oteryn_world_bundle::bundle::placement_key(-11, 34068, 31418, 1).ok_or("exit object")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
    .map_err(debug)?;
    let command_id = participant.continuity.next_command_id;
    let (_, reply, barrier, _) = super::guild_wire::exchange_capture(
        view,
        &mut participant,
        command_id,
        COMMAND_TYPE_USE_INTENT,
        |_| Ok(payload.clone()),
        false,
        &mut Vec::new(),
    )
    .await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::TooFar
    );
    assert!(barrier.is_none());
    assert_eq!(
        participant
            .item_fence
            .ok_or("unchanged fence")?
            .runtime_scope,
        scope
    );
    let mut route = view
        .item_walk_route(
            actor,
            participant.game_session_id,
            crate::gameplay_transport::paced_movement::AutoWalkTarget::Cell(target.actor_position),
        )
        .await
        .map_err(|_| "Vanguard exit route")?
        .ok_or("Vanguard exit reachable")?;
    route.steps.pop_back().ok_or("step onto exit")?;
    for direction in route.steps {
        view.clock_origin = view
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("movement cadence")?;
        let (step, _) = view
            .paced_step(actor, participant.game_session_id, direction)
            .await;
        assert_eq!(
            step.disposition,
            oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
        );
    }
    let source = participant;
    let command_id = participant.continuity.next_command_id;
    let (_, reply, barrier, _) = super::guild_wire::exchange_capture(
        view,
        &mut participant,
        command_id,
        COMMAND_TYPE_USE_INTENT,
        |_| Ok(payload),
        true,
        &mut Vec::new(),
    )
    .await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    assert!(barrier.is_some());
    assert!(matches!(
        participant
            .item_fence
            .ok_or("returned fence")?
            .runtime_scope,
        RuntimeScopeRefV1::Channel { .. }
    ));
    assert_eq!(
        origin
            .activity_runtimes
            .current_connection(peer)
            .game_session_id,
        peer.game_session_id
    );
    let landed = origin
        .runtime
        .lock()
        .await
        .read_actor_position(participant.runtime_actor.ok_or("returned actor")?)?
        .position();
    assert_eq!(
        landed,
        MovementLocalPosition {
            x: 34053,
            y: 31431,
            floor: 11
        }
    );
    let receipt: (i32, i32, i16, i16, Option<String>, bool) = sqlx::query_as(
        "SELECT return_x,return_y,return_floor,reason,command_id::text,native_use_exit
         FROM game_activity_runtime_returns
         WHERE source_game_session_id=encode($1,'hex')::uuid",
    )
    .bind(source.game_session_id.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        receipt,
        (34053, 31431, 11, 1, Some(command_id.to_string()), true)
    );
    let returns: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_runtime_returns")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(returns, 1);
    Ok(())
}
