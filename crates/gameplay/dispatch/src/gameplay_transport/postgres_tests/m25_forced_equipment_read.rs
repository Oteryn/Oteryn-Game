//! A live physical PUSH witness qualifies the grace participant's source-ground equipment read.
use super::TestResult;
use super::fixture::debug;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, ControlLossResult},
};

#[test]
fn forced_reconnectable_push_reads_equipment_on_qualified_ground() -> TestResult {
    super::m25_ahau_exit::push_equipment_scenario()
}

pub(super) async fn exercise(
    origin: &ComposedFreshAdmission<'_, '_, '_>,
    view: &mut ComposedFreshAdmission<'_, '_, '_>,
    pusher: AdmittedSession,
    source_peer: AdmittedSession,
) -> TestResult {
    let peer = origin.activity_runtimes.current_connection(source_peer);
    super::m25_ahau_push::walk(
        view,
        peer,
        MovementLocalPosition {
            x: 34002,
            y: 31705,
            floor: 10,
        },
    )
    .await?;
    super::m25_ahau_push::walk(
        view,
        pusher,
        MovementLocalPosition {
            x: 34002,
            y: 31704,
            floor: 10,
        },
    )
    .await?;
    let target = peer.runtime_actor.ok_or("actual pushed actor")?;
    let pusher_actor = pusher.runtime_actor.ok_or("actual pusher actor")?;
    assert!(
        view.read_movement_equipment(target, peer.game_session_id)
            .await
            .is_some(),
        "the controlled source-ground participant reads its persisted equipment"
    );
    let controller = peer.controller.ok_or("actual peer controller")?;
    assert_eq!(
        view.commit_control_loss(peer.game_session_id, target, controller)
            .await,
        ControlLossResult::Recorded
    );
    assert!(
        view.read_movement_equipment(target, peer.game_session_id)
            .await
            .is_none(),
        "a grace session without a live push witness reads no equipment"
    );
    let push = {
        let mut runtime = view.runtime.lock().await;
        let pusher_at = runtime.read_actor_position(pusher_actor).map_err(debug)?;
        let target_at = runtime.read_actor_position(target).map_err(debug)?;
        runtime
            .prepare_player_push(
                (pusher_actor, pusher.game_session_id, pusher_at),
                (target, peer.game_session_id, target_at),
                MovementLocalPosition {
                    x: 34002,
                    y: 31706,
                    floor: 10,
                },
            )
            .map_err(debug)?
    };
    let forced = view
        .read_movement_equipment(target, peer.game_session_id)
        .await;
    drop(push);
    assert!(
        forced.is_some(),
        "the pushed Reconnectable participant reads equipment on qualified source ground"
    );
    assert!(
        view.read_movement_equipment(target, peer.game_session_id)
            .await
            .is_none(),
        "the retired push witness grants no standing read"
    );
    eprintln!("M25_FORCED_RECONNECTABLE_EQUIPMENT_READ_EXECUTED");
    Ok(())
}
