use super::*;
pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    before_ref: std::num::NonZeroU32,
    after_ref: std::num::NonZeroU32,
) -> TestResult {
    let body = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let mut exit = vec![8, 2, 18, HOUSE.len() as u8];
    exit.extend(HOUSE.as_bytes());
    let capture = std::sync::Arc::new(std::sync::Mutex::new(super::wire::HouseWireCapture {
        exit_after_death: Some(exit),
        exit_after_commit: true,
        ..Default::default()
    }));
    let (admitted, result, _, _) = exchange(
        owner,
        admitted,
        1,
        &body,
        HouseExchange {
            rotate: false,
            current: Some(before_ref),
            after: Some(after_ref),
            nested: None,
            foreign_after_snapshot: None,
            capture: Some(std::sync::Arc::clone(&capture)),
        },
    )
    .await?;
    assert_eq!(
        result,
        UseDisposition::Committed,
        "ordinary carried candle2914 USE inside House lights2915"
    );
    let original:Value=sqlx::query_scalar("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid")
  .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert!(original["deadline_at"].as_i64().is_some());
    {
        let captured = capture.lock().map_err(|_| "candle live Exit capture")?;
        assert_eq!(
            captured.exit_status,
            Some(CommandStatus::Accepted),
            "live native command102 leaves House after committed candle USE"
        );
        assert!(
            captured
                .exit_ack
                .as_ref()
                .is_some_and(|ack| !ack.is_empty())
        );
    }
    let channel = admitted;
    let unchanged:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_timed_states t JOIN game_item_instances i USING(item_instance_id) JOIN game_item_timed_state_writes w ON w.transaction_id=i.last_transaction_id WHERE t.item_instance_id=encode($1,'hex')::uuid AND t.deadline_at=w.deadline_after AND t.remaining_ms=w.remaining_ms_after AND t.state_revision=w.expected_revision+1)").bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert!(
        unchanged,
        "real scope exit preserves the exact original USE timer write"
    );
    assert!(matches!(
        channel.item_fence.ok_or("Channel fence")?.runtime_scope,
        crate::foundation::RuntimeScopeRefV1::Channel { .. }
    ));
    assert_eq!(sqlx::query_scalar::<_,Value>("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid")
  .bind(id(71).as_slice()).fetch_one(&h.pool).await?,original,"scope exit preserves running candle budget");
    // The real source action supplied this exact clock. Make only that clock due
    // in the isolated fixture; three real minutes are not claimed to elapse.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_timed_states SET deadline_at=floor(extract(epoch FROM statement_timestamp())*1000)::bigint-1,remaining_ms=1 WHERE item_instance_id=encode($1,'hex')::uuid")
  .bind(id(71).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    owner.tick_timed_items().await;
    let after:Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
  .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        after["definition_production_key"], "oteryn:item.tibia.i2916",
        "current Channel owner must expire a carried House-lit candle"
    );
    let settled = after.clone();
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    owner.tick_timed_items().await;
    assert_eq!(sqlx::query_scalar::<_,Value>("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
  .bind(id(71).as_slice()).fetch_one(&h.pool).await?,settled,"carried expiry is once-only");
    Ok(())
}
