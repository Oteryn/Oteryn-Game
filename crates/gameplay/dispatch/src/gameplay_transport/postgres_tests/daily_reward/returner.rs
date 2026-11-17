use super::*;
#[test]
fn returner_real_dispatch_qualifies_native_history_and_claims_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_mode(false, 2).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "returner scenario panicked")?
        .map_err(Into::into)
}
/// Only historical timestamps are seeded; the current session/fences come from fresh admission.
pub(super) async fn seed_history(h: &Harness) -> TestResult {
    sqlx::raw_sql("ALTER TABLE game_durability_reconnect_sessions DISABLE TRIGGER game_training_session_clock").execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,session_state,training_created_at_us,training_session_sequence) OVERRIDING SYSTEM VALUE VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,0,1,1,1,1,3,extract(epoch FROM TIMESTAMPTZ '2020-01-01 00:00:00+00')::bigint*1000000,0)")
        .bind(id(49).as_slice()).bind(id(40).as_slice()).bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::raw_sql(
        "ALTER TABLE game_durability_reconnect_sessions ENABLE TRIGGER game_training_session_clock",
    )
    .execute(&h.pool)
    .await?;
    Ok(())
}
async fn query(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<wire::returner::Reply> {
    let (end, bytes) = serve(owner, admitted, &[(wire::returner::QUERY, &[])]).await?;
    let values = results(&bytes)?;
    assert_eq!(values.len(), 1, "{end}");
    assert_eq!(
        values[0].1,
        crate::foundation::CommandStatus::Accepted,
        "{values:?}"
    );
    wire::returner::Reply::decode(&values[0].2).map_err(Into::into)
}
pub(super) async fn case(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    use wire::returner::{CLAIM, QUERY};
    assert_eq!(
        query(owner, admitted).await?.state,
        0,
        "missing terminal end evidence grants no gift"
    );
    sqlx::query("INSERT INTO game_character_session_ends(game_session_id,character_id,ended_at_us) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,s.training_created_at_us-30::bigint*86400*1000000 FROM game_durability_reconnect_sessions s WHERE s.game_session_id=encode($3,'hex')::uuid")
        .bind(id(49).as_slice()).bind(id(41).as_slice()).bind(id(50).as_slice()).execute(&h.pool).await?;
    assert_eq!(
        query(owner, admitted).await?.state,
        0,
        "a thirty-day break is not twelve months"
    );
    sqlx::raw_sql("ALTER TABLE game_character_session_ends DISABLE TRIGGER game_character_session_ends_immutable; UPDATE game_character_session_ends SET ended_at_us=extract(epoch FROM TIMESTAMPTZ '2024-01-01 00:00:00+00')::bigint*1000000; ALTER TABLE game_character_session_ends ENABLE TRIGGER game_character_session_ends_immutable").execute(&h.pool).await?;
    let offer = query(owner, admitted).await?;
    assert_eq!(offer.state, 1);
    let offer = offer.id.ok_or("entitlement")?;
    assert_eq!(
        query(owner, admitted).await?.id,
        Some(offer),
        "query must reload the same persistent entitlement"
    );
    let (_, bytes) = serve(owner, admitted, &[(CLAIM, &id(99))]).await?;
    assert_eq!(
        results(&bytes)?[0].1,
        crate::foundation::CommandStatus::Rejected
    );
    sqlx::raw_sql("CREATE FUNCTION returner_fixture_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected returner receipt failure'; END $$; CREATE CONSTRAINT TRIGGER returner_fixture_fail AFTER INSERT ON game_returner_reward_claims DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION returner_fixture_fail()").execute(&h.pool).await?;
    let (_, bytes) = serve(owner, admitted, &[(CLAIM, &offer)]).await?;
    assert_eq!(
        results(&bytes)?[0].1,
        crate::foundation::CommandStatus::Rejected
    );
    let rolled_back:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_returner_reward_claims),(SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=encode($1,'hex')::uuid)").bind(offer.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(rolled_back, (0, 0));
    sqlx::raw_sql("DROP TRIGGER returner_fixture_fail ON game_returner_reward_claims")
        .execute(&h.pool)
        .await?;
    let (_, bytes) = serve(
        owner,
        admitted,
        &[(CLAIM, &offer), (CLAIM, &offer), (QUERY, &[])],
    )
    .await?;
    let values = results(&bytes)?;
    assert_eq!(values.len(), 3);
    assert_eq!(values[0].2[1], 1, "{values:?}");
    assert_eq!(values[1].2, [8, 2]);
    assert_eq!(wire::returner::Reply::decode(&values[2].2)?.state, 2);
    let delivered:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_returner_reward_entitlements),(SELECT count(*) FROM game_returner_reward_claims),(SELECT sum(i.quantity)::bigint FROM game_returner_reward_claims c JOIN game_item_instances i USING(item_instance_id))").fetch_one(&h.pool).await?;
    assert_eq!(delivered, (1, 1, 1));
    let ordinary: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_daily_reward_claims")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(ordinary, 0, "returner claim must not consume a daily claim");
    let mut stale = *admitted;
    let mut fence = stale.item_fence.ok_or("fence")?;
    fence.connection_generation =
        crate::foundation::ConnectionGeneration::new(99).map_err(debug)?;
    stale.item_fence = Some(fence);
    let (_, bytes) = serve(owner, &mut stale, &[(QUERY, &[])]).await?;
    assert!(results(&bytes)?.is_empty());
    Ok(())
}
