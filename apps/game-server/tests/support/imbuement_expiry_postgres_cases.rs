//! Actual canonical Character authority PG root, nested beside the existing checkpoint cases.
//! Fixture retained rows are raw DATA. These cases prove only audited deletion of stored zero,
//! never eligible elapsed time, Content admission or an application/fee producer.

use super::*;
use crate::durability::imbuement_expiry::{
    IMBUEMENT_EXPIRY_SCHEMA_PROPOSAL, ImbuementExpiryError, ImbuementExpiryOutcome,
    ImbuementExpiryRequest, expire_imbuement_in_transaction,
};
use crate::durability::imbuement_expiry_audit::{
    OneItemImbuementSlotV1, decode_imbuement_expiry_envelope,
};
use crate::durability::item_mint_audit::Type2Transaction;

async fn setup(h: &Harness, zero: bool) -> TestResult<Vec<ItemInstanceId>> {
    let items = storage_setup(h, 1).await?;
    h.pool.execute(IMBUEMENT_EXPIRY_SCHEMA_PROPOSAL).await?;
    // Test owner grants only in this harness database, never a production migration grant.
    h.pool
        .execute(
            "GRANT SELECT,INSERT ON game_imbuement_expiry_receipts TO oteryn_game_runtime; \
        GRANT SELECT ON game_imbuement_expiry_evidence TO oteryn_game_runtime; \
        GRANT DELETE ON game_item_imbuements TO oteryn_game_runtime; \
        GRANT EXECUTE ON FUNCTION game_assert_imbuement_expiry(UUID) TO oteryn_game_runtime;",
        )
        .await?;
    if zero {
        let before = snapshot(h, &items).await?;
        store(h, &before, &decreased(&before, 0))
            .await?
            .map_err(debug)?;
    }
    Ok(items)
}

fn request(row: &SlotRecord, seed: u8) -> ImbuementExpiryRequest {
    let binding = row.binding();
    ImbuementExpiryRequest {
        item_instance_id: *binding.item.as_bytes(),
        before: OneItemImbuementSlotV1 {
            slot: u32::from(binding.slot),
            imbuement_key: binding.key.clone(),
            content_revision: binding.content_revision.clone(),
            category: binding.category.clone(),
            tier: match binding.tier {
                Tier::Basic => 1,
                Tier::Intricate => 2,
                Tier::Powerful => 3,
            },
            remaining_ms: row.remaining_ms(),
            applying_occurrence: binding.applying_occurrence.to_vec(),
            applying_transaction_id: binding.transaction_id.to_vec(),
        },
        expiry_occurrence: uuid(seed),
        transaction_id: uuid(seed + 1),
        event_id: uuid(seed + 2),
        occurred_at_unix_ms: 1_700_000_000_000,
        server_build_id: "native-imbuement-expiry-test".into(),
    }
}

async fn expire_with_recovery(
    h: &Harness,
    recovered: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    digest: [u8; 32],
    request: &ImbuementExpiryRequest,
) -> TestResult<Result<ImbuementExpiryOutcome, ImbuementExpiryError>> {
    let mut raw = h.pool.begin().await?;
    raw.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
    // The native type-2 fence is the first statement after session settings, not a fake
    // selected tuple. No SQL owner or activation checks are replaced by this harness.
    let mut tx = Type2Transaction::open(raw).await.map_err(debug)?;
    let result =
        expire_imbuement_in_transaction(&mut tx, &h.root, recovered, node, &fence, digest, request)
            .await;
    // Even a typed refusal must leave the surrounding transaction committable and unchanged.
    tx.into_inner().commit().await?;
    Ok(result)
}
async fn expire(
    h: &Harness,
    request: &ImbuementExpiryRequest,
) -> TestResult<Result<ImbuementExpiryOutcome, ImbuementExpiryError>> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovered = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    expire_with_recovery(
        h,
        &recovered,
        &h.node,
        native::fence()?,
        STORAGE_DIGEST,
        request,
    )
    .await
}

async fn footprint(
    h: &Harness,
    items: &[ItemInstanceId],
) -> TestResult<(CheckpointSnapshot, String, i64, i64, i64)> {
    let snapshot = snapshot(h, items).await?;
    let item: String = sqlx::query_scalar("SELECT to_jsonb(i)::text FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(items[0].as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let receipts = sqlx::query_scalar("SELECT count(*) FROM game_imbuement_expiry_receipts")
        .fetch_one(&h.pool)
        .await?;
    let evidence = sqlx::query_scalar("SELECT count(*) FROM game_imbuement_expiry_evidence")
        .fetch_one(&h.pool)
        .await?;
    let audit = sqlx::query_scalar("SELECT count(*) FROM game_item_audit_outbox")
        .fetch_one(&h.pool)
        .await?;
    Ok((snapshot, item, receipts, evidence, audit))
}

#[test]
fn native_expiry_removes_only_zero_slot_and_preserves_item_character_and_other_bindings()
-> TestResult {
    run_storage("imbue_expiry_commit", async |h| {
        let items = setup(h, true).await?;
        let before = footprint(h, &items).await?;
        let request = request(&before.0.rows[0], 150);
        assert_eq!(
            expire(h, &request).await?.map_err(debug)?,
            ImbuementExpiryOutcome::Committed
        );
        let after = footprint(h, &items).await?;
        assert_eq!(after.0.rows, before.0.rows[1..]);
        assert_eq!(after.0.equipment, before.0.equipment);
        assert_eq!(after.1, before.1); // Every native ItemInstance column, including its state revision.
        assert_eq!(
            (after.2, after.3, after.4),
            (before.2 + 1, before.3 + 1, before.4 + 1)
        );
        let wire: Vec<u8> = sqlx::query_scalar(
            "SELECT envelope FROM game_item_audit_outbox WHERE event_id=encode($1,'hex')::uuid",
        )
        .bind(request.event_id.as_slice())
        .fetch_one(&h.pool)
        .await?;
        let (envelope, event) = decode_imbuement_expiry_envelope(&wire).map_err(debug)?;
        assert_eq!(event.row_before.as_ref(), Some(&request.before));
        assert_eq!(
            event.character_revision,
            before.0.equipment.character_revision
        );
        assert_eq!(event.expiry_occurrence, request.expiry_occurrence);
        assert_eq!(
            envelope
                .transaction_event
                .as_ref()
                .map(|value| value.transaction_id.as_slice()),
            Some(request.transaction_id.as_slice())
        );
        assert!(envelope.game_session_id.is_none() && envelope.command_id.is_none());
        Ok(())
    })
}

#[test]
fn native_expiry_refuses_forged_zero_and_every_retained_binding_substitution_without_writes()
-> TestResult {
    run_storage("imbue_expiry_binding", async |h| {
        let items = setup(h, false).await?;
        let initial = footprint(h, &items).await?;
        let mut forged_zero = request(&initial.0.rows[0], 150);
        forged_zero.before.remaining_ms = 0;
        assert!(matches!(
            expire(h, &forged_zero).await?,
            Err(ImbuementExpiryError::Stale)
        ));
        assert_eq!(footprint(h, &items).await?, initial);
        store(h, &initial.0, &decreased(&initial.0, 0))
            .await?
            .map_err(debug)?;
        let before = footprint(h, &items).await?;
        let original = request(&before.0.rows[0], 150);
        let mutations: &[fn(&mut ImbuementExpiryRequest)] = &[
            |v| v.before.imbuement_key.push_str("-other"),
            |v| v.before.content_revision.push_str("-other"),
            |v| v.before.category.push_str("-other"),
            |v| v.before.tier = 2,
            |v| v.before.applying_occurrence = uuid(180).to_vec(),
            |v| v.before.applying_transaction_id = uuid(181).to_vec(),
            |v| v.before.slot = 1,
            |v| v.item_instance_id = uuid(182),
            |v| v.before.remaining_ms = 1,
        ];
        for mutation in mutations {
            let mut changed = original.clone();
            mutation(&mut changed);
            assert!(matches!(
                expire(h, &changed).await?,
                Err(ImbuementExpiryError::Stale
                    | ImbuementExpiryError::NotEquipped
                    | ImbuementExpiryError::InvalidInput)
            ));
            assert_eq!(footprint(h, &items).await?, before);
        }
        Ok(())
    })
}

#[test]
fn native_expiry_rolls_back_deleted_row_receipt_and_evidence_after_audit_error_even_on_outer_commit()
-> TestResult {
    run_storage("imbue_expiry_rollback", async |h| {
        let items = setup(h, true).await?;
        let before = footprint(h, &items).await?;
        let mut request = request(&before.0.rows[0], 150);
        request.event_id = sqlx::query_scalar::<_, Vec<u8>>(
            "SELECT uuid_send(event_id) FROM game_item_audit_outbox LIMIT 1",
        )
        .fetch_one(&h.pool)
        .await?
        .try_into()
        .map_err(|_| "native event UUID length")?;
        assert!(matches!(
            expire(h, &request).await?,
            Err(ImbuementExpiryError::Database(_))
        ));
        assert_eq!(footprint(h, &items).await?, before);
        Ok(())
    })
}

#[test]
fn native_expiry_revalidates_actual_recovery_root_session_node_scope_content_and_equipment()
-> TestResult {
    run_storage("imbue_expiry_authority", async |h| {
        let items = setup(h, true).await?;
        let before = footprint(h, &items).await?;
        let request = request(&before.0.rows[0], 150);
        let wrong_node = native::register(&h.root, 2).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovered = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // A real independently opened Root issues a capability for itself. The same database
        // and sealed recovery record must not make that capability portable to h.root.
        let foreign_root =
            crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(foreign_root.maintain_ready_once().await?);
        let foreign_recovered = foreign_root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(matches!(
            expire_with_recovery(
                h,
                &foreign_recovered,
                &h.node,
                native::fence()?,
                STORAGE_DIGEST,
                &request
            )
            .await?,
            Err(ImbuementExpiryError::AuthorityRejected)
        ));
        assert_eq!(footprint(h, &items).await?, before);
        for mutation in 0..9 {
            let mut fence = native::fence()?;
            let mut digest = STORAGE_DIGEST;
            match mutation {
                0 => fence.connection_generation = ConnectionGeneration::new(2)?,
                1 => fence.character_lease_generation = 2,
                2 => fence.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
                3 => digest = [9; 32],
                4 => {}
                5 => fence.game_session_id = GameSessionId::decode(&uuid(201))?,
                6 => fence.character_id = crate::domain::CharacterId::from_bytes(uuid(202))?,
                7 => {
                    fence.runtime_scope = RuntimeScopeRefV1::channel(
                        WorldId::decode(&uuid(203))?,
                        ChannelId::decode(&native::id(native::CHANNEL))?,
                    )
                }
                _ => {
                    fence.runtime_scope = RuntimeScopeRefV1::channel(
                        WorldId::decode(&native::id(native::WORLD))?,
                        ChannelId::decode(&uuid(204))?,
                    )
                }
            }
            assert!(matches!(
                expire_with_recovery(
                    h,
                    &recovered,
                    if mutation == 4 { &wrong_node } else { &h.node },
                    fence,
                    digest,
                    &request
                )
                .await?,
                Err(ImbuementExpiryError::AuthorityRejected)
            ));
            assert_eq!(footprint(h, &items).await?, before);
        }
        assert!(
            h.root
                .commit_equipment_move(
                    &recovered,
                    &h.node,
                    &native::fence()?,
                    native::command(250)?,
                    &StorageProfile,
                    EquipmentMove {
                        item: *items[0].as_bytes(),
                        from_slot: Some(1),
                        to_slot: None
                    }
                )
                .await
                .map_err(debug)?
                .is_ok()
        );
        assert!(matches!(
            expire_with_recovery(
                h,
                &recovered,
                &h.node,
                native::fence()?,
                STORAGE_DIGEST,
                &request
            )
            .await?,
            Err(ImbuementExpiryError::NotEquipped)
        ));
        let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_imbuements")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(remaining, 3);
        Ok(())
    })
}

#[test]
fn native_expiry_exact_replay_survives_fence_loss_but_changed_occurrence_or_binding_conflicts()
-> TestResult {
    run_storage("imbue_expiry_replay", async |h| {
        let items = setup(h, true).await?;
        let before = snapshot(h, &items).await?;
        let original = request(&before.rows[0], 150);
        assert_eq!(
            expire(h, &original).await?.map_err(debug)?,
            ImbuementExpiryOutcome::Committed
        );
        let committed = footprint(h, &items).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovered = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut stale = native::fence()?;
        stale.connection_generation = ConnectionGeneration::new(2)?;
        assert_eq!(
            expire_with_recovery(h, &recovered, &h.node, stale, [9; 32], &original)
                .await?
                .map_err(debug)?,
            ImbuementExpiryOutcome::AlreadyCommitted
        );
        for mutation in 0..5 {
            let mut changed = original.clone();
            match mutation {
                0 => changed.expiry_occurrence = uuid(181),
                1 => changed.before.content_revision.push_str("-changed"),
                2 => changed.occurred_at_unix_ms += 1,
                3 => changed.server_build_id.push('x'),
                _ => changed.event_id = uuid(182),
            }
            assert!(matches!(
                expire(h, &changed).await?,
                Err(ImbuementExpiryError::ConflictingOccurrence)
            ));
            assert_eq!(footprint(h, &items).await?, committed);
        }
        let mut other_character = native::fence()?;
        other_character.character_id = crate::domain::CharacterId::from_bytes(uuid(202))?;
        assert!(matches!(
            expire_with_recovery(
                h,
                &recovered,
                &h.node,
                other_character,
                STORAGE_DIGEST,
                &original
            )
            .await?,
            Err(ImbuementExpiryError::ConflictingOccurrence)
        ));
        assert_eq!(footprint(h, &items).await?, committed);
        Ok(())
    })
}

#[test]
fn native_expiry_database_guards_reject_bare_delete_forged_witness_and_historical_receipt()
-> TestResult {
    run_storage("imbue_expiry_guards", async |h| {
        let items = setup(h, true).await?;
        let before = footprint(h, &items).await?;
        for statement in [
            "DELETE FROM game_item_imbuements WHERE slot=0",
            "INSERT INTO game_imbuement_expiry_evidence(transaction_id,slot_before) VALUES('00000000-0000-7000-8000-000000000001','{}')",
            "TRUNCATE game_imbuement_expiry_receipts",
        ] {
            let mut tx = h.pool.begin().await?;
            tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
            assert!(tx.execute(statement).await.is_err());
            tx.rollback().await?;
            assert_eq!(footprint(h, &items).await?, before);
        }
        let original = request(&before.0.rows[0], 150);
        expire(h, &original).await?.map_err(debug)?;
        // A new privileged fixture row is DATA, not a real apply or timer. Reusing its old
        // binding cannot borrow the committed receipt's physical transaction as authority.
        sqlx::query("INSERT INTO game_item_imbuements SELECT (jsonb_populate_record(NULL::game_item_imbuements, slot_before)).* FROM game_imbuement_expiry_receipts WHERE transaction_id=encode($1,'hex')::uuid")
            .bind(original.transaction_id.as_slice()).execute(&h.pool).await?;
        let installed = footprint(h, &items).await?;
        let mut tx = h.pool.begin().await?;
        tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
        assert!(
            tx.execute("DELETE FROM game_item_imbuements WHERE slot=0")
                .await
                .is_err()
        );
        tx.rollback().await?;
        assert_eq!(footprint(h, &items).await?, installed);
        Ok(())
    })
}

#[test]
fn native_expiry_serializes_actual_concurrent_replay_and_conflicting_occurrences() -> TestResult {
    run_storage("imbue_expiry_race", async |h| {
        let items = setup(h, true).await?;
        let before = footprint(h, &items).await?;
        let request_a = request(&before.0.rows[0], 150);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovered = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (a, b) = native::join_two(
            expire_with_recovery(
                h,
                &recovered,
                &h.node,
                native::fence()?,
                STORAGE_DIGEST,
                &request_a,
            ),
            expire_with_recovery(
                h,
                &recovered,
                &h.node,
                native::fence()?,
                STORAGE_DIGEST,
                &request_a,
            ),
        )
        .await;
        let outcomes = [a?.map_err(debug)?, b?.map_err(debug)?];
        assert_eq!(
            outcomes
                .iter()
                .filter(|v| **v == ImbuementExpiryOutcome::Committed)
                .count(),
            1
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|v| **v == ImbuementExpiryOutcome::AlreadyCommitted)
                .count(),
            1
        );
        let first = footprint(h, &items).await?;
        assert_eq!(
            (first.2, first.3, first.4),
            (before.2 + 1, before.3 + 1, before.4 + 1)
        );
        let request_b = request(&first.0.rows[0], 160);
        let mut request_c = request_b.clone();
        request_c.expiry_occurrence = uuid(170);
        request_c.transaction_id = uuid(171);
        request_c.event_id = uuid(172);
        let (b, c) = native::join_two(
            expire_with_recovery(
                h,
                &recovered,
                &h.node,
                native::fence()?,
                STORAGE_DIGEST,
                &request_b,
            ),
            expire_with_recovery(
                h,
                &recovered,
                &h.node,
                native::fence()?,
                STORAGE_DIGEST,
                &request_c,
            ),
        )
        .await;
        let results = [b?, c?];
        assert_eq!(
            results
                .iter()
                .filter(|v| matches!(v, Ok(ImbuementExpiryOutcome::Committed)))
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|v| matches!(v, Err(ImbuementExpiryError::ConflictingOccurrence)))
                .count(),
            1
        );
        let after = footprint(h, &items).await?;
        assert_eq!(after.0.rows.len(), 1);
        assert_eq!(after.0.equipment, before.0.equipment);
        assert_eq!(after.1, before.1);
        assert_eq!(
            (after.2, after.3, after.4),
            (before.2 + 2, before.3 + 2, before.4 + 2)
        );
        Ok(())
    })
}

#[test]
fn native_expiry_one_slot_per_physical_transaction_bound_preserves_first_success_on_second_refusal()
-> TestResult {
    run_storage("imbue_expiry_bound", async |h| {
        let items = setup(h, true).await?;
        let before = footprint(h, &items).await?;
        let first = request(&before.0.rows[0], 150);
        let second = request(&before.0.rows[1], 160);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovered = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut raw = h.pool.begin().await?;
        raw.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
        let mut tx = Type2Transaction::open(raw).await.map_err(debug)?;
        assert_eq!(
            expire_imbuement_in_transaction(
                &mut tx,
                &h.root,
                &recovered,
                &h.node,
                &native::fence()?,
                STORAGE_DIGEST,
                &first
            )
            .await
            .map_err(debug)?,
            ImbuementExpiryOutcome::Committed
        );
        assert!(matches!(
            expire_imbuement_in_transaction(
                &mut tx,
                &h.root,
                &recovered,
                &h.node,
                &native::fence()?,
                STORAGE_DIGEST,
                &second
            )
            .await,
            Err(ImbuementExpiryError::CapacityExceeded)
        ));
        tx.into_inner().commit().await?;
        let after = footprint(h, &items).await?;
        assert_eq!(after.0.rows, before.0.rows[1..]);
        assert_eq!(after.0.equipment, before.0.equipment);
        assert_eq!(after.1, before.1);
        assert_eq!(
            (after.2, after.3, after.4),
            (before.2 + 1, before.3 + 1, before.4 + 1)
        );
        Ok(())
    })
}
