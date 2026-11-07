use super::*;

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    mut admitted: AdmittedSession,
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3081','definition-r1',1,1,encode($3,'hex')::uuid)")
        .bind(id(120).as_slice()).bind(id(42).as_slice()).bind(id(121).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,5,encode($5,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(120).as_slice()).bind(id(42).as_slice()).bind(id(122).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    // Existing ITEM_USE_V1 sessions also reach the timed writer through ordinary USE.
    let offered = [
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 4,
            requires: &[],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 15,
            requires: &[4],
        },
    ];
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &[4, 15])
            .ok_or("use selection")?;
    let payload = oteryn_protocol_oteryn::world_object::encode_use_item_intent(
        std::num::NonZeroU64::new(2).ok_or("handle")?,
    );
    sqlx::raw_sql("CREATE FUNCTION timed_use_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test timed use commit failure' USING ERRCODE='23514'; END $$; CREATE CONSTRAINT TRIGGER timed_use_fail AFTER INSERT ON game_item_timed_commands DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION timed_use_fail();").execute(&h.pool).await?;
    assert!(
        exchange_kind(
            owner,
            admitted,
            oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT,
            &[(30, payload.clone())]
        )
        .await?
        .is_empty(),
        "unknown ordinary USE cannot consume its command id"
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(120).as_slice()).fetch_one(&h.pool).await?, 0, "failed USE rolls back its charge");
    sqlx::raw_sql(
        "DROP TRIGGER timed_use_fail ON game_item_timed_commands; DROP FUNCTION timed_use_fail();",
    )
    .execute(&h.pool)
    .await?;
    let results = exchange_kind(
        owner,
        admitted,
        oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT,
        &[(30, payload.clone())],
    )
    .await?;
    assert_eq!(
        results,
        [oteryn_protocol_oteryn::world_object::encode_use_result(
            oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        )
        .map_err(debug)?]
    );
    assert_eq!(
        exchange_kind(
            owner,
            admitted,
            oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT,
            &[(30, payload)]
        )
        .await?,
        results,
        "ordinary USE receipt replays exactly"
    );
    let charges: Option<i32> = sqlx::query_scalar(
        "SELECT charges FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(120).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(charges, Some(4));
    Ok(())
}
