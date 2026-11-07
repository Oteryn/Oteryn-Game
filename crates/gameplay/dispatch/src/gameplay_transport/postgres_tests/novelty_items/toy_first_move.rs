//! The actual individually moved native gift starts its existing absolute clock.
use super::*;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    source: [u8; 16],
    case: Case,
) -> TestResult {
    let target = match case {
        Case::ToyDragonMove => "oteryn:item.tibia.i5791",
        Case::ToySantaMove => "oteryn:item.tibia.i6511",
        Case::ToyPandaMove => "oteryn:item.tibia.i5080",
        _ => return Err("toy case".into()),
    };
    let index = owner
        .active_generation
        .and_then(|a| a.native_gameplay())
        .and_then(|n| n.item_index())
        .ok_or("native toy index")?;
    let reference = index
        .definition_ref("Item", case.key(), "definition-r1")
        .ok_or("toy ref")?;
    let clocks:i64=sqlx::query_scalar("SELECT (SELECT count(*) FROM game_spell_item_temporal_schedules WHERE item_instance_id=encode($1,'hex')::uuid)+(SELECT count(*) FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid)").bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        clocks, 0,
        "new source gift in an unopened bag has default duration only"
    );
    let (next, result, payload) = super::super::map_seed_planting::drop_exchange(
        owner, admitted, 1, None, reference, 1, false,
    )
    .await?;
    assert_eq!(
        result,
        Some(oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved)
    );
    let timer:Option<(String,i64,i64)>=sqlx::query_as("SELECT target_production_key,duration_millis,expires_at_unix_ms FROM game_spell_item_temporal_schedules WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_optional(&h.pool).await?;
    let timer = timer.ok_or(
        "first actual native MOVE must start the individual source toy three-second clock",
    )?;
    assert_eq!((&timer.0, timer.1), (&target.to_string(), 3000));
    admitted = next;
    owner
        .states
        .command_ingress
        .forget(admitted.game_session_id);
    admitted.continuity.next_command_id = 1;
    let (after_replay, replayed, _) = super::super::map_seed_planting::drop_exchange(
        owner,
        admitted,
        1,
        Some(payload),
        reference,
        1,
        false,
    )
    .await?;
    assert_eq!(
        replayed,
        Some(oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved)
    );
    let deadline:i64=sqlx::query_scalar("SELECT expires_at_unix_ms FROM game_spell_item_temporal_schedules WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        deadline, timer.2,
        "native historical MOVE replay cannot restart duration"
    );
    tokio::time::timeout(std::time::Duration::from_secs(10),async {
        loop {
            if owner.root.has_ready_demand() {owner.root.maintain_ready_once().await.map_err(debug)?;}
            owner.drain_source_item_deadlines().await;
            let key:String=sqlx::query_scalar("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
            if key==target { break; }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        Ok::<_,Box<dyn std::error::Error>>(())
    }).await??;
    let retired:i64=sqlx::query_scalar("SELECT count(*) FROM game_spell_item_temporal_receipts WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        retired, 1,
        "existing actual Channel clock retires the individually moved toy once"
    );
    let base_ref = index
        .definition_ref("Item", target, "definition-r1")
        .ok_or("expired source base native reference")?;
    let body = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let (_, used, _) =
        super::super::rust_lottery::exchange(owner, after_replay, 2, &body, Some(base_ref), None)
            .await?;
    assert_eq!(
        used,
        UseDisposition::Committed,
        "the next native source snapshot decodes and USEs the expired base toy"
    );
    Ok(())
}
