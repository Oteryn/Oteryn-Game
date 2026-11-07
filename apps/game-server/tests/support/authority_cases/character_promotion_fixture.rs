//! Existing Wheel fixtures acquire permanent promotion through the real purchase participant.
use crate::bestiary_postgres_harness::{Harness, TestResult, debug, fence, id};
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::item_fee_burn::test_support::{item, request};
use crate::durability::npc_purchase::{NpcPurchaseRequest, PurchaseOutcome, PurchaseService};
use crate::foundation::{CommandId, CommandRef};

pub(crate) async fn prepare(h: &Harness) -> TestResult<NpcPurchaseRequest> {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "{}{}",
        item(210, "oteryn:item.tibia.i2854", "rev-1", 1),
        item(212, "oteryn:item.tibia.i3043", "rev-1", 2)
    )))
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(210).as_slice()).bind(id(42).as_slice()).bind(id(214).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(210).as_slice()).bind(id(212).as_slice()).bind(id(42).as_slice()).bind(id(215).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let fence = fence(1)?;
    Ok(NpcPurchaseRequest {
        command: CommandRef::new(fence.game_session_id, CommandId::new(99).map_err(debug)?),
        occurrence: id(200),
        transaction_id: id(201),
        event_id: id(202),
        account: id(40),
        npc_key: "oteryn:npc.eremo".into(),
        npc_identity: id(203),
        npc_generation: 1,
        service: PurchaseService::Promotion("oteryn:service.promotion.eremo".into()),
        coins: request(1, 20_000)?.change,
        expected_backpack: Some(id(210)),
        occurred_at_unix_ms: 1_790_000_000_000,
    })
}

pub(crate) async fn purchase(h: &Harness, revision: u64) -> TestResult {
    let request = prepare(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let fence = fence(revision)?;
    let coordinator =
        crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
            Default::default(),
            None,
            None,
            Some(h.root.clone()),
        );
    let sequencer = CharacterRevisionSequencer::new();
    let outcome = sequencer
        .acquire(fence.character_id)
        .await
        .commit_npc_purchase(&h.root, &authority, &h.node, fence, request, &coordinator)
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, PurchaseOutcome::Committed(_)),
        "{outcome:?}"
    );
    Ok(())
}
