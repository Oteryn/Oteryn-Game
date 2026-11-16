//! CONTAINER-MANUAL-SORT-1 (capability 510): a deposit naming its box slot takes that slot of
//! the box's view order (newest first) through the depot owner, in the same transaction and under
//! a container sort receipt; a replay with the same slot changes nothing and another slot
//! conflicts.
use super::*;
use oteryn_protocol_oteryn::item_view::ItemHandle;

/// Box 17's items, newest first.
async fn order(pool: &sqlx::PgPool) -> TestResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT item_instance_id::text FROM game_item_character_depot_locations \
         WHERE town_id=0 AND box_number=17 ORDER BY ordinal DESC",
    )
    .fetch_all(pool)
    .await?)
}

fn text(item: [u8; 16]) -> String {
    let h: String = item.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    pool: &sqlx::PgPool,
    stacks: &[(ItemHandle, [u8; 16])],
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let fence = admitted.item_fence.ok_or("fence")?;
    let command = |command_id| UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: Some(fence),
    };
    let [(a_handle, a), (b_handle, b), (c_handle, c)] = stacks[..3] else {
        return Err("three carried stacks".into());
    };
    let deposit = |handle| DepotIntent {
        town: Some(0),
        box_number: 17,
        page: 0,
        action: DepotAction::Deposit(handle),
    };
    let outcome = |command_id, handle, item, slot| async move {
        Ok::<_, Box<dyn std::error::Error>>(
            owner
                .depot_at(
                    actor,
                    command(command_id),
                    deposit(handle),
                    slot,
                    Some(ItemKey::Instance(item)),
                )
                .await
                .ok_or("deposit")?
                .reply
                .outcome,
        )
    };
    assert_eq!(outcome(1, a_handle, a, None).await?, DepotOutcome::Moved);
    assert_eq!(order(pool).await?, vec![text(a)]);
    // A slot past the last entry places the deposit last.
    assert_eq!(outcome(2, b_handle, b, Some(5)).await?, DepotOutcome::Moved);
    assert_eq!(order(pool).await?, vec![text(a), text(b)]);
    // The third deposit takes slot 1, between the earlier two.
    assert_eq!(outcome(3, c_handle, c, Some(1)).await?, DepotOutcome::Moved);
    assert_eq!(order(pool).await?, vec![text(a), text(c), text(b)]);
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_container_sort_receipts \
         WHERE permutation @> '[{\"depot\":{\"town\":0,\"box\":17}}]'",
    )
    .fetch_one(pool)
    .await?;
    assert_eq!(receipts, 2, "one depot ordinal receipt per rotated placement");
    // The same command with the same slot replays; with another slot it conflicts.
    assert_eq!(outcome(3, c_handle, c, Some(1)).await?, DepotOutcome::Moved);
    assert_eq!(outcome(3, c_handle, c, Some(0)).await?, DepotOutcome::Rejected);
    assert_eq!(order(pool).await?, vec![text(a), text(c), text(b)]);
    // The receipt triggers admit no ordinal change outside a current depot move.
    let forged = sqlx::query(
        "UPDATE game_item_character_depot_locations SET ordinal=ordinal \
         WHERE item_instance_id=$1::uuid",
    )
    .bind(text(a))
    .execute(pool)
    .await;
    assert!(forged.is_err(), "an ordinal update without its receipt is refused");
    Ok(())
}
