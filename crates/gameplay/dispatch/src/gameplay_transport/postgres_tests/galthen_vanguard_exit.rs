//! Crystal's arena USE returns through the native Instance handoff.
use super::TestResult;
use super::fixture::{Harness, debug};
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, UseCommand},
};
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, UseDisposition, WorldObjectTarget, decode_use_result,
    encode_use_intent,
};

const TRIGGER: MovementLocalPosition = MovementLocalPosition {
    x: 34068,
    y: 31418,
    floor: 11,
};
const LANDING: MovementLocalPosition = MovementLocalPosition {
    x: 34053,
    y: 31431,
    floor: 11,
};
const TIMED_EXIT: [i32; 3] = [34060, 31432, 11];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Case {
    SoloExit,
    Reward,
    PartyExit,
}

#[test]
fn galthen_vanguard_native_use_exit_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::m25_ahau_exit::vanguard_scenario(Case::SoloExit)
}

#[test]
fn galthen_vanguard_use_returns_only_clicking_participant_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::m25_ahau_exit::vanguard_scenario(Case::PartyExit)
}

pub(super) async fn exercise(
    h: &Harness,
    origin: &mut ComposedFreshAdmission<'_, '_, '_>,
    view: &mut ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    source_peer: Option<AdmittedSession>,
) -> TestResult {
    let peer = if let Some(source) = source_peer {
        let peer = origin.activity_runtimes.current_connection(source);
        assert_ne!(peer.game_session_id, source.game_session_id);
        let scope = admitted.item_fence.ok_or("Vanguard fence")?.runtime_scope;
        assert_eq!(peer.item_fence.ok_or("peer fence")?.runtime_scope, scope);
        Some(peer)
    } else {
        None
    };
    let actor = admitted.runtime_actor.ok_or("current Instance actor")?;
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: admitted.continuity.next_command_id,
        item_fence: admitted.item_fence,
    };
    let before = view.runtime.lock().await.read_actor_position(actor)?;
    // The transport walks on TooFar and retries. Check the owner's reach gate
    // before driving the adjacent USE over the wire below.
    let outcome = view.use_world_object(actor, command, exit_target(view)?).await;
    assert_eq!(outcome.disposition, UseDisposition::TooFar);
    assert_eq!(view.runtime.lock().await.read_actor_position(actor)?, before);
    let returns: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_runtime_returns")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(returns, 0);
    exercise_exit(h, origin, view, &mut admitted).await?;
    if let Some(mut peer) = peer {
        let returns: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_runtime_returns")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(returns, 1, "each USE returns just its issuer");
        // The shared roster entered together, but one USE returns only its issuer.
        assert_eq!(
            origin
                .activity_runtimes
                .current_connection(peer)
                .game_session_id,
            peer.game_session_id
        );
        assert_eq!(
            view.runtime
                .lock()
                .await
                .positioned_actor_census()
                .map_err(debug)?
                .into_iter()
                .filter(|(_, _, session)| *session == Some(peer.game_session_id))
                .count(),
            1
        );
        super::m25_ahau_push::walk(
            origin,
            admitted,
            MovementLocalPosition {
                x: LANDING.x - 1,
                ..LANDING
            },
        )
        .await?;
        exercise_exit(h, origin, view, &mut peer).await?;
        let returns: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_runtime_returns")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(
            returns, 2,
            "both participants have independent return receipts"
        );
    }
    Ok(())
}

fn exit_target(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("Vanguard Instance map")?;
    let object = oteryn_world_bundle::bundle::placement_key(
        i8::try_from(-TRIGGER.floor)?,
        u16::try_from(TRIGGER.x)?,
        u16::try_from(TRIGGER.y)?,
        1,
    )
    .ok_or("Vanguard arena object")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&object.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(object),
    })
}

async fn exercise_exit(
    h: &Harness,
    origin: &mut ComposedFreshAdmission<'_, '_, '_>,
    view: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let source_cell = MovementLocalPosition {
        y: TRIGGER.y - 1,
        ..TRIGGER
    };
    super::m25_ahau_push::walk(view, *admitted, source_cell).await?;
    let map = view.bundle_map().ok_or("Vanguard Instance map")?;
    let first_effect = map
        .facts
        .movement_effects_since(0)
        .last()
        .map_or(0, |row| row.0);
    let source = admitted.game_session_id;
    let command = admitted.continuity.next_command_id;
    let target = exit_target(view)?;
    let (_, reply, barrier, _) = super::guild_wire::exchange_capture(
        view,
        admitted,
        command,
        COMMAND_TYPE_USE_INTENT,
        |_| {
            encode_use_intent(&target)
                .map_err(debug)
                .map_err(Into::into)
        },
        true,
        &mut Vec::new(),
    )
    .await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    let effects = map.facts.movement_effects_since(first_effect);
    assert!(effects.iter().any(|(_, at, effect)| *effect == 11
        && i32::from(at.x) == source_cell.x
        && i32::from(at.y) == source_cell.y
        && -i16::from(at.floor) == source_cell.floor));
    assert!(barrier.is_some());
    assert_ne!(source, admitted.game_session_id);
    assert!(matches!(
        admitted.item_fence.ok_or("returned fence")?.runtime_scope,
        RuntimeScopeRefV1::Channel { .. }
    ));
    let actor = admitted.runtime_actor.ok_or("returned actor")?;
    assert_eq!(
        origin
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        LANDING
    );
    let row: (i16, String, i32, i32, i16, bool, i32, i32, i16, Vec<i32>) = sqlx::query_as(
        "SELECT r.reason,r.command_id::text,r.return_x,r.return_y,r.return_floor,r.native_use_exit,
            p.exit_x,p.exit_y,p.exit_floor,p.use_exit_cell
         FROM game_activity_runtime_returns r JOIN game_activity_runtime_placements p USING(handoff_id,character_id)
         WHERE source_game_session_id=encode($1,'hex')::uuid AND destination_game_session_id=encode($2,'hex')::uuid",
    ).bind(source.as_bytes().as_slice()).bind(admitted.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((row.0, row.1), (1, command.to_string()));
    let landing = [LANDING.x, LANDING.y, i32::from(LANDING.floor)];
    assert_eq!([row.2, row.3, i32::from(row.4)], landing);
    assert!(row.5);
    assert_eq!([row.6, row.7, i32::from(row.8)], TIMED_EXIT);
    assert_eq!(row.9, landing);
    Ok(())
}
