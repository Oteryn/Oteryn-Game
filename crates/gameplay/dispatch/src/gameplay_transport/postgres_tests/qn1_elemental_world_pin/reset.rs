//! The actual reset owner drains a soil-created field; creation history grants no authority.
use super::super::fixture::{Harness, debug};
use super::super::{ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::AdmittedSession;
use std::time::Duration;

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
    item: [u8; 16],
) -> TestResult {
    let pin = owner.runtime.lock().await.content_pin().clone();
    let map = crate::map::overlay::map_revision(
        owner.bundle_map().ok_or("energy reset map")?.overlay.base(),
    );
    // The fixture Control publishes through the real SQL seam. Expected pins
    // are current qualified content; the Root independently resolves the job,
    // current registration, assignment, closed readiness and terminal sessions.
    assert!(sqlx::query_scalar::<_, bool>("SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$2,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(pin.server_artifact_digest().as_slice())
        .bind(&map).bind(pin.client_artifact_digest().as_slice()).bind(pin.frame_binding_digest().as_slice())
        .fetch_one(&h.pool).await?);
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT game_world_reset_publish(encode($1,'hex')::uuid,0,$2,$3,$2,$4,$5,'[]'::jsonb,1)"
        )
        .bind(owner.world_id.as_bytes().as_slice())
        .bind(pin.server_artifact_digest().as_slice())
        .bind(&map)
        .bind(pin.client_artifact_digest().as_slice())
        .bind(pin.frame_binding_digest().as_slice())
        .fetch_one(&h.pool)
        .await?
    );
    let reset = owner
        .root
        .retiring_world_reset(owner.world_id)
        .await?
        .ok_or("energy retiring reset")?;
    eprintln!("QN1_ENERGY_FIELD_RESET_PG_READY");
    let live: (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(live, (1, 1));
    tokio::time::timeout(
        Duration::from_secs(65),
        crate::gameplay_transport::listener::control_loss_lifecycle(
            owner,
            crate::gameplay_transport::listener::LostControl {
                session: admitted,
                wait: Duration::ZERO,
                reset: Some(reset),
            },
            &oteryn_foundation::CancellationToken::new(),
        ),
    )
    .await?;
    let store = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone());
    assert_eq!(
        store
            .current_session(admitted.game_session_id)
            .await?
            .session_state(),
        crate::foundation::GameSessionState::Terminal
    );
    assert!(
        !owner
            .root
            .retire_world_reset_ground(owner.character, owner.holder, &reset)
            .await
            .map_err(debug)?,
        "terminal sessions alone cannot replace the required fresh assignment"
    );
    super::super::planned_world_reset::reset_fixture::replace(
        h,
        owner.world_id,
        owner.channel_id,
        owner.holder,
    )
    .await?;
    for _ in 0..2 {
        assert!(
            owner
                .root
                .retire_world_reset_ground(owner.character, owner.holder, &reset)
                .await
                .map_err(debug)?
        );
    }
    let retired: (i64, i16, i64, i64, i64) = sqlx::query_as("SELECT i.quantity,i.lifecycle,(SELECT count(*) FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id),(SELECT count(*) FROM game_item_decay_retire_receipts r WHERE r.item_instance_id=i.item_instance_id AND r.cause_kind='WORLD_RESET'),(SELECT count(*) FROM game_item_audit_outbox a WHERE a.item_instance_id=i.item_instance_id AND a.transaction_id=i.last_transaction_id) FROM game_item_instances i WHERE i.item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        retired,
        (0, 2, 0, 1, 1),
        "one conserved reset retirement, including retry"
    );
    let (source, count): (bool, i64) = sqlx::query_as("SELECT game_elemental_energy_field_source(encode($1,'hex')::uuid),game_live_channel_field_count(encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::bytea[])")
        .bind(item.as_slice()).bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice())
        .bind(Vec::<Vec<u8>>::new()).fetch_one(&h.pool).await?;
    assert!(
        !source,
        "immutable creation history cannot revive a retired field"
    );
    assert_eq!(count, 0);
    assert!(
        !owner
            .root
            .read_item_loop_ground(owner.world_id, owner.channel_id)
            .await
            .map_err(debug)?
            .iter()
            .any(|row| row.item.item_instance_id == item)
    );
    eprintln!("QN1_ENERGY_FIELD_RESET_PG_EXECUTED");
    Ok(())
}
