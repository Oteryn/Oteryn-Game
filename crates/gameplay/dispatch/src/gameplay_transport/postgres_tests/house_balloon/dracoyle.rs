//! Actual native677 USE WITH placed inactive statue, retaining House identity.
use super::*;
pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    statue_ref: std::num::NonZeroU32,
    after_key: &str,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|a| a.native_gameplay())
        .ok_or("native")?;
    let gem_ref = native
        .item_index()
        .and_then(|i| i.definition_ref("Item", "oteryn:item.tibia.i677", "definition-r1"))
        .ok_or("gem reference")?;
    let before:Value=sqlx::query_scalar("SELECT jsonb_build_object('item',to_jsonb(i),'house',to_jsonb(h),'provenance',to_jsonb(p)) FROM game_item_instances i JOIN game_item_house_interior_locations h USING(item_instance_id) JOIN game_item_house_reclaim_provenance p USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid")
  .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    let body = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let (next, result, actual) = super::super::rust_lottery::exchange(
        owner,
        admitted,
        1,
        &body,
        Some(gem_ref),
        Some(statue_ref),
    )
    .await?;
    admitted = next;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "actual House677+inactive statue must charge in place"
    );
    let after:Value=sqlx::query_scalar("SELECT jsonb_build_object('item',to_jsonb(i),'house',to_jsonb(h),'provenance',to_jsonb(p)) FROM game_item_instances i JOIN game_item_house_interior_locations h USING(item_instance_id) JOIN game_item_house_reclaim_provenance p USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid")
  .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(after["item"]["definition_production_key"], after_key);
    assert_eq!(
        after["item"]["item_instance_id"],
        before["item"]["item_instance_id"]
    );
    assert_eq!(
        after["item"]["minted_transaction_id"],
        before["item"]["minted_transaction_id"]
    );
    assert_eq!(after["house"], before["house"]);
    assert_eq!(after["provenance"], before["provenance"]);
    let gem: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(72).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(gem, 1, "exactly one of two emeralds spent");
    let (deadline,occurred):(i64,i64)=sqlx::query_as("SELECT coalesce(t.deadline_at,p.expires_at_unix_ms),r.occurred_at FROM game_item_instances i JOIN game_item_consume_transform_records r ON r.transaction_id=i.last_transaction_id LEFT JOIN game_item_timed_states t USING(item_instance_id) LEFT JOIN game_spell_item_temporal_schedules p USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid")
  .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        deadline,
        occurred + 604800000,
        "actual charge original seven-day deadline"
    );
    owner
        .states
        .command_ingress
        .forget(admitted.game_session_id);
    admitted.continuity.next_command_id = 1;
    let (_, replayed, _) =
        super::super::rust_lottery::exchange(owner, admitted, 1, &actual, None, None).await?;
    assert_eq!(replayed, UseDisposition::Committed);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid"
        )
        .bind(id(72).as_slice())
        .fetch_one(&h.pool)
        .await?,
        1,
        "replay cannot spend emerald twice"
    );
    Ok(())
}
