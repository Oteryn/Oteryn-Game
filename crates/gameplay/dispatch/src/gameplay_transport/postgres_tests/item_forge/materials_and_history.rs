//! Current native stash, recursive carried custody and the shared bounded history.
use super::*;

#[test]
fn forge_postgres_conversion_consumes_carried_and_stashed_slivers_once() -> TestResult {
    super::super::kill_reward_e2e::scenario_thread::run("forge-stash", || {
        support::scenario(Case::Stash)
    })
}

#[test]
fn forge_postgres_conversion_consumes_materials_in_recursive_carried_custody() -> TestResult {
    super::super::kill_reward_e2e::scenario_thread::run("forge-nested", || {
        support::scenario(Case::Nested)
    })
}

#[test]
fn forge_postgres_history_keeps_shared_thousand_entries_and_fifty_entry_pages() -> TestResult {
    super::super::kill_reward_e2e::scenario_thread::run("forge-history-bound", || {
        support::scenario(Case::HistoryBound)
    })
}

pub(super) async fn stash(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    seed_items(h, &[(80, FORGE_SLIVER_KEY, 3, 0)]).await?;
    // Bootstrap an already completed native stow in this isolated fixture. The
    // Forge debit below uses the normal stash chain and its production SQL guards.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_stash_operations(transaction_id,game_session_id,command_id,character_id,world_id,intent,action,outcome,event_id,occurred_at,envelope_sha256) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1000000,encode($3,'hex')::uuid,encode($4,'hex')::uuid,sha256('forge-stash-fixture'),2,2,encode($5,'hex')::uuid,1,sha256('forge-stash-fixture'))")
        .bind(id(235).as_slice()).bind(admitted.game_session_id.as_bytes().as_slice()).bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(237).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_stash_entries(entry_id,transaction_id,character_id,world_id,definition_key,definition_revision,amount,quantity_before,quantity_after) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,'definition-r1',47,0,47)")
        .bind(id(236).as_slice()).bind(id(235).as_slice()).bind(id(41).as_slice()).bind(id(42).as_slice()).bind(FORGE_SLIVER_KEY).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_stash_balances(character_id,world_id,definition_key,definition_revision,quantity,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'definition-r1',47,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(FORGE_SLIVER_KEY).bind(id(236).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let action = request(Operation::SliversToCores, 160);
    let (status, result) = exchange(owner, admitted, action).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(result["applied"], true);
    assert_eq!(result["result"]["slivers_spent"], 50);
    assert_eq!(balances(h).await?, (100, 100, 0, 1));
    let chain: (i64, i64, i64, bool) = sqlx::query_as("SELECT b.quantity,e.amount,e.quantity_before,e.previous_entry_id=encode($2,'hex')::uuid FROM game_character_stash_balances b JOIN game_character_stash_entries e ON e.entry_id=b.last_entry_id WHERE b.character_id=encode($1,'hex')::uuid AND b.definition_key=$3")
        .bind(id(41).as_slice()).bind(id(236).as_slice()).bind(FORGE_SLIVER_KEY).fetch_one(&h.pool).await?;
    assert_eq!(chain, (0, 47, 47, true));
    let common: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_stash_operations s JOIN game_forge_operations f USING(transaction_id,event_id,envelope_sha256,character_id,game_session_id,command_id) WHERE s.action=4 AND f.occurrence_id=encode($1,'hex')::uuid")
        .bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        common, 1,
        "one native stash debit shares the Forge receipt and event"
    );
    assert_eq!(
        exchange(owner, admitted, action).await?,
        (CommandStatus::Accepted, result)
    );
    let entries: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_stash_entries")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        entries, 2,
        "the fixture stow and one debit, even after replay"
    );
    assert_eq!(balances(h).await?, (100, 100, 0, 1));
    Ok(())
}

pub(super) async fn nested(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    seed_items(
        h,
        &[
            (80, "oteryn:item.tibia.i2854", 1, 0),
            (81, "oteryn:item.tibia.i2854", 1, 0),
            (82, FORGE_SLIVER_KEY, 50, 0),
        ],
    )
    .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (item, parent) in [(81, 80), (82, 81)] {
        sqlx::query("WITH e AS (DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid RETURNING *) INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,world_id,encode($2,'hex')::uuid,1,placed_transaction_id FROM e")
            .bind(id(item).as_slice()).bind(id(parent).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let action = request(Operation::SliversToCores, 170);
    let (status, result) = exchange(owner, admitted, action).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(result["applied"], true);
    assert_eq!(balances(h).await?, (100, 100, 0, 1));
    let consumed: (i64, i16, bool) = sqlx::query_as("SELECT quantity,lifecycle,EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(82).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(consumed, (0, 2, false));
    let preserved: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid AND parent_item_instance_id=encode($2,'hex')::uuid")
        .bind(id(81).as_slice()).bind(id(80).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        preserved, 1,
        "consuming a nested material preserves its enclosing bags"
    );
    let proof: Value = sqlx::query_scalar("SELECT inputs->0->'location' FROM game_forge_operations WHERE occurrence_id=encode($1,'hex')::uuid")
        .bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(proof["kind"], 2);
    assert_eq!(
        exchange(owner, admitted, action).await?,
        (CommandStatus::Accepted, result)
    );
    assert_eq!(balances(h).await?, (100, 100, 0, 1));
    Ok(())
}

pub(super) async fn history_bound(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let first = request(Operation::DustLimit, 180);
    let (status, limit_receipt) = exchange(owner, admitted, first).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(limit_receipt["succeeded"], true);
    // Fill the retained-history fixture with the actual native receipt shape.
    // The 1001st record is produced by the guarded command, which must evict only
    // the oldest history row and must keep that occurrence's replay receipt.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_forge_history(character_id,occurrence_id,history_ordinal,reply) SELECT encode($1,'hex')::uuid,game_character_uuid_v7(),n,$2 FROM generate_series(2,1000) n")
        .bind(id(41).as_slice()).bind(&limit_receipt).execute(&mut *tx).await?;
    tx.commit().await?;
    cooldown_elapsed(h).await?;
    let (status, newest) =
        exchange(owner, admitted, request(Operation::DustToSlivers, 181)).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(newest["applied"], true);
    let bounds: (i64, i64, i64) = sqlx::query_as("SELECT count(*),min(history_ordinal),max(history_ordinal) FROM game_forge_history WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(bounds, (1000, 2, 1001));
    cooldown_elapsed(h).await?;
    let (status, page) = exchange(owner, admitted, request(Operation::QueryHistory, 0)).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let rows = page.as_array().ok_or("history page")?;
    assert_eq!(rows.len(), 50);
    assert_eq!(rows[0]["receipt"], newest);
    assert_eq!(rows[1]["receipt"], limit_receipt);
    assert_eq!(rows[0]["ordinal"], 1001);
    assert_eq!(rows[49]["ordinal"], 952);
    assert!(serde_json::to_vec(&page)?.len() <= 32_768);
    assert_eq!(
        exchange(owner, admitted, first).await?,
        (CommandStatus::Accepted, limit_receipt)
    );
    assert_eq!(balances(h).await?, (15, 120, 3, 0));
    let mut too_large = request(Operation::QueryHistory, 0)
        .encode()
        .ok_or("history request")?;
    too_large[50] = 51;
    assert_eq!(
        support::exchange_raw(owner, admitted, &too_large).await?,
        (CommandStatus::Rejected, Vec::new())
    );
    Ok(())
}
