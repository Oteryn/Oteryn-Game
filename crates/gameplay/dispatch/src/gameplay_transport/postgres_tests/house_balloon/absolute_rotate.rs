use super::*;

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    source: [u8; 16],
    before_ref: std::num::NonZeroU32,
    after_ref: std::num::NonZeroU32,
    history: bool,
) -> TestResult {
    let original:Value=sqlx::query_scalar("SELECT to_jsonb(p) FROM game_spell_item_temporal_schedules p WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        original["definition_production_key"],
        "oteryn:item.tibia.i9034"
    );
    let capture = std::sync::Arc::new(std::sync::Mutex::new(super::wire::HouseWireCapture {
        move_destination: Some(
            oteryn_protocol_oteryn::item_view::ItemMoveDestination::Ground(
                oteryn_protocol_oteryn::item_view::WorldTilePosition {
                    x: POSITION[0],
                    y: POSITION[1],
                    floor: -i16::try_from(POSITION[2])?,
                },
            ),
        ),
        ..Default::default()
    }));
    let (admitted, placed, _, _) = exchange(
        owner,
        admitted,
        1,
        &[],
        HouseExchange {
            rotate: false,
            current: Some(before_ref),
            after: None,
            nested: None,
            foreign_after_snapshot: None,
            capture: Some(std::sync::Arc::clone(&capture)),
        },
    )
    .await?;
    assert_eq!(placed, UseDisposition::Committed);
    assert_eq!(
        capture
            .lock()
            .map_err(|_| "charged House move capture")?
            .move_outcome,
        Some(oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved)
    );
    assert_eq!(sqlx::query_scalar::<_,Value>("SELECT to_jsonb(p) FROM game_spell_item_temporal_schedules p WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&h.pool).await?,original,"real HousePlace preserves original absolute deadline");
    let body = oteryn_protocol_oteryn::world_rotate::encode_rotate_intent(
        &UseTarget::Item(std::num::NonZeroU64::MIN),
        Some(before_ref),
    )
    .map_err(debug)?;
    let (admitted, result, _, _) = exchange(
        owner,
        admitted,
        2,
        &body,
        HouseExchange {
            rotate: true,
            current: Some(before_ref),
            after: Some(after_ref),
            nested: None,
            foreign_after_snapshot: None,
            capture: None,
        },
    )
    .await?;
    assert_eq!(result, UseDisposition::Committed);
    let rotated:Value=sqlx::query_scalar("SELECT to_jsonb(p) FROM game_spell_item_temporal_schedules p WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(source.as_slice()).fetch_one(&h.pool).await?;
    let mut expected = original.clone();
    expected["definition_production_key"] = serde_json::json!("oteryn:item.tibia.i9039");
    expected["target_production_key"] = serde_json::json!("oteryn:item.tibia.i9040");
    assert_eq!(
        rotated, expected,
        "House Rotate preserves the actual enchantment clock and source"
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(source.as_slice()).fetch_one(&h.pool).await?,0,"no parallel House timer");
    // Boundary: first prove the real produced deadline unchanged; then make
    // that same schedule due in this isolated fixture. Seven days do not elapse.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_spell_item_temporal_schedules SET expires_at_unix_ms=floor(extract(epoch FROM statement_timestamp())*1000)::bigint-1 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(source.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let actor = admitted.runtime_actor.ok_or("House absolute actor")?;
    owner
        .expire_house_furniture_on_tick(actor, admitted.game_session_id)
        .await
        .ok_or("House absolute tick")?;
    let after:Value=sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(source.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        after["definition_production_key"],
        "oteryn:item.tibia.i9040"
    );
    assert_eq!(
        after["minted_transaction_id"], original["source_transaction_id"],
        "expiry preserves genuine enchantment mint lineage"
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_furniture_absolute_expiries WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(source.as_slice()).fetch_one(&h.pool).await?,1);
    owner
        .expire_house_furniture_on_tick(actor, admitted.game_session_id)
        .await
        .ok_or("House repeat tick")?;
    assert_eq!(sqlx::query_scalar::<_,Value>("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(source.as_slice()).fetch_one(&h.pool).await?,after,"absolute House expiry executes once");
    if history {
        return super::absolute_history::exercise(h, owner, admitted, source, original).await;
    }
    Ok(())
}
