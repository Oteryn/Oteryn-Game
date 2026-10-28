use super::*;
fn uuid(seed: u8) -> [u8; 16] {
    let mut v = [seed; 16];
    v[6] = (v[6] & 15) | 0x70;
    v[8] = (v[8] & 63) | 0x80;
    v
}
#[test]
fn purchase_binding_is_stable_and_complete() {
    let r = PurchaseRequest {
        purchase_request_id: uuid(1),
        account_id: uuid(2),
        character_id: uuid(3),
        binding: b"binding".to_vec(),
    };
    assert!(r.validate().is_ok());
    let mut c = r.clone();
    c.binding.push(0);
    assert_ne!(r.binding_digest(), c.binding_digest());
}
#[test]
fn delivery_plan_validation_is_closed() -> TestResult {
    let mut p = plan(80)?;
    assert!(p.validate().is_ok());
    p.mint.outputs[0].quantity = 0;
    assert_eq!(p.validate(), Err(StoreValidationError::InvalidQuantity));
    p.mint.outputs[0].quantity = 1;
    p.fulfilment_unit_id = "x".repeat(513);
    assert_eq!(p.validate(), Err(StoreValidationError::InvalidText));
    Ok(())
}
#[test]
fn receipt_codec_round_trips() -> TestResult {
    let r = DeliveryReceipt {
        operation_id: "o".into(),
        account_id: "a".into(),
        character_id: "c".into(),
        world_id: "w".into(),
        fulfilment_unit_id: "u".into(),
        result: "COMMITTED".into(),
        recovery_generation: "2".into(),
        decided_at: "3".into(),
    };
    let b = super::codec::encode_receipt(&r).map_err(debug)?;
    assert_eq!(serde_json::from_slice::<DeliveryReceipt>(&b)?, r);
    Ok(())
}

use crate::bestiary_postgres_harness as harness;
use harness::{Harness, TestResult, debug, id};

fn plan(seed: u8) -> TestResult<DeliveryPlan> {
    use crate::durability::character_inbox::{
        RewardClaimInboxMintOutput, RewardClaimInboxMintRequest,
    };
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};
    use crate::foundation::{CommandId, CommandRef};
    Ok(DeliveryPlan {
        account_id: id(40),
        character_id: id(harness::CHARACTER),
        world_id: id(harness::WORLD),
        fulfilment_unit_id: format!("unit-{seed}"),
        item_instance_id: id(seed),
        command: CommandRef::new(
            harness::fence(1)?.game_session_id,
            CommandId::new(u64::from(seed)).map_err(debug)?,
        ),
        mint: RewardClaimInboxMintRequest {
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: "oteryn:store".into(),
                revision_ref: "store-r1".into(),
            },
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
            outputs: vec![RewardClaimInboxMintOutput {
                source_line: 0,
                quantity: 1,
                key_number: None,
                item: ItemDefinitionFacts {
                    definition: TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:store.fixture".into(),
                        revision_ref: "item-r1".into(),
                    },
                    weight: Some(1),
                    stack: ItemStackClass::NonStackable,
                    container_capacity: None,
                    container_slot_equip_pattern: false,
                },
            }],
        },
    })
}

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let Some(admin) = harness::configured_admin() else {
        return Ok(());
    };
    harness::runtime()?.block_on(async move {
        let h = Harness::create(admin, tag, true).await?;
        let result = async {
            runtime_role(&h.root).await?;
            body(&h).await
        }
        .await;
        h.cleanup().await?;
        result
    })
}

async fn apply(
    h: &Harness,
    operation: u8,
    plan: DeliveryPlan,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
) -> Result<DeliveryReceipt, StoreDurabilityError> {
    let seal = h
        .recovery
        .seal_current()
        .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(|_| StoreDurabilityError::AuthorityRejected)?;
    h.root
        .apply_store_delivery(&authority, &h.node, id(operation), plan, fence)
        .await
}

#[test]
fn store_postgres_delivery_replay_conflicts_outbox_and_restart() -> TestResult {
    run("store_replay", async |h| {
        let p = plan(80)?;
        let first = apply(h, 90, p.clone(), harness::fence(1)?)
            .await
            .map_err(debug)?;
        let bytes: Vec<u8> = sqlx::query_scalar(
            "SELECT receipt FROM store_delivery_receipts WHERE operation_id=encode($1,'hex')::uuid",
        )
        .bind(id(90).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(super::codec::encode_receipt(&first).map_err(debug)?, bytes);
        let mut stale = harness::fence(1)?;
        stale.character_lease_generation += 1;
        let replay = apply(h, 90, p.clone(), stale).await.map_err(debug)?;
        assert_eq!(first, replay);
        assert_eq!(super::codec::encode_receipt(&replay).map_err(debug)?, bytes);
        for field in 0..4 {
            let mut conflict = p.clone();
            match field {
                0 => conflict.account_id = id(99),
                1 => conflict.character_id = id(99),
                2 => conflict.world_id = id(99),
                _ => conflict.fulfilment_unit_id.push('x'),
            }
            assert!(matches!(
                apply(h, 90, conflict, harness::fence(1)?).await,
                Err(StoreDurabilityError::ConflictingOperation)
            ));
        }
        for table in [
            "game_item_instances",
            "game_item_character_inbox_locations",
            "store_delivery_receipts",
            "store_receipt_outbox",
        ] {
            assert_eq!(h.count(table).await?, 1, "{table}");
        }
        let counters: (String, String) = sqlx::query_as("SELECT committed::text,next_ordinal::text FROM game_character_inbox_counters WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(harness::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(counters, ("1".into(), "2".into()));
        let reopened = new_runtime_root(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = reopened
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            reopened
                .apply_store_delivery(&authority, &h.node, id(90), p, stale)
                .await
                .map_err(debug)?,
            first
        );
        assert_eq!(
            reopened
                .list_unacknowledged_store_receipts(1)
                .await
                .map_err(debug)?,
            vec![first.clone()]
        );
        let second = apply(h, 91, plan(81)?, harness::fence(1)?)
            .await
            .map_err(debug)?;
        assert_eq!(
            reopened
                .list_unacknowledged_store_receipts(1)
                .await
                .map_err(debug)?,
            vec![first.clone()]
        );
        assert_eq!(
            reopened
                .list_unacknowledged_store_receipts(2)
                .await
                .map_err(debug)?,
            vec![first, second.clone()]
        );
        assert!(
            reopened
                .mark_store_receipt_acknowledged(id(90))
                .await
                .map_err(debug)?
        );
        assert!(
            !reopened
                .mark_store_receipt_acknowledged(id(90))
                .await
                .map_err(debug)?
        );
        assert_eq!(
            reopened
                .list_unacknowledged_store_receipts(256)
                .await
                .map_err(debug)?,
            vec![second]
        );
        assert!(
            reopened
                .mark_store_receipt_acknowledged(id(91))
                .await
                .map_err(debug)?
        );
        assert!(
            reopened
                .list_unacknowledged_store_receipts(256)
                .await
                .map_err(debug)?
                .is_empty()
        );
        Ok(())
    })
}

#[test]
fn store_postgres_wrong_account_world_and_stale_fence_write_nothing() -> TestResult {
    run("store_refusal", async |h| {
        for variant in 0..5 {
            let mut p = plan(80)?;
            let mut fence = harness::fence(1)?;
            match variant {
                0 => p.account_id = id(99),
                1 => p.world_id = id(99),
                2 => {
                    fence.connection_generation =
                        crate::foundation::ConnectionGeneration::new(2).map_err(debug)?
                }
                3 => fence.character_lease_generation += 1,
                _ => {
                    fence.expected_character_revision =
                        crate::domain::CharacterRevision::new(2).map_err(debug)?
                }
            }
            assert!(matches!(
                apply(h, 90, p, fence).await,
                Err(StoreDurabilityError::AuthorityRejected)
            ));
        }
        for table in [
            "game_item_instances",
            "game_character_inbox_deliveries",
            "store_delivery_receipts",
            "store_receipt_outbox",
            "game_reward_claim_inbox_mint_reservations",
        ] {
            assert_eq!(h.count(table).await?, 0, "{table}");
        }
        Ok(())
    })
}

#[test]
fn store_postgres_concurrent_duplicate_delivery() -> TestResult {
    run("store_race", async |h| {
        let p = plan(80)?;
        let other = new_runtime_root(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let second_authority = other.open_character_authority(&seal).await.map_err(debug)?;
        let fence = harness::fence(1)?;
        let (a, b) = concurrent(
            h.root
                .apply_store_delivery(&authority, &h.node, id(90), p.clone(), fence),
            other.apply_store_delivery(&second_authority, &h.node, id(90), p, fence),
        )
        .await;
        assert_eq!(a.map_err(debug)?, b.map_err(debug)?);
        for table in [
            "game_item_instances",
            "store_delivery_receipts",
            "store_receipt_outbox",
        ] {
            assert_eq!(h.count(table).await?, 1, "{table}");
        }
        Ok(())
    })
}

#[test]
fn store_postgres_purchase_retry_binding_and_answer_once() -> TestResult {
    run("store_purchase", async |h| {
        let r = PurchaseRequest {
            purchase_request_id: id(90),
            account_id: id(40),
            character_id: id(harness::CHARACTER),
            binding: b"binding".to_vec(),
        };
        assert_eq!(
            h.root
                .record_purchase_request(r.clone())
                .await
                .map_err(debug)?,
            PurchaseRequestState::Sent
        );
        assert_eq!(
            h.root
                .record_purchase_request(r.clone())
                .await
                .map_err(debug)?,
            PurchaseRequestState::Sent
        );
        for field in 0..3 {
            let mut conflict = r.clone();
            match field {
                0 => conflict.account_id = id(99),
                1 => conflict.character_id = id(99),
                _ => conflict.binding.push(0),
            }
            assert!(matches!(
                h.root.record_purchase_request(conflict).await,
                Err(StoreDurabilityError::ConflictingBinding)
            ));
        }
        let other = new_runtime_root(h).await?;
        let (a, b) = concurrent(
            h.root.record_store_answer(r.clone(), b"yes".to_vec()),
            other.record_store_answer(r.clone(), b"yes".to_vec()),
        )
        .await;
        assert_eq!(a.map_err(debug)?, b"yes");
        assert_eq!(b.map_err(debug)?, b"yes");
        assert_eq!(
            h.root
                .record_purchase_request(r.clone())
                .await
                .map_err(debug)?,
            PurchaseRequestState::Answered(b"yes".to_vec())
        );
        assert!(matches!(
            h.root.record_store_answer(r, b"different".to_vec()).await,
            Err(StoreDurabilityError::ConflictingBinding)
        ));
        let mut wrong_binding = PurchaseRequest {
            purchase_request_id: id(90),
            account_id: id(40),
            character_id: id(harness::CHARACTER),
            binding: b"wrong".to_vec(),
        };
        assert!(matches!(
            other
                .record_store_answer(wrong_binding.clone(), b"yes".to_vec())
                .await,
            Err(StoreDurabilityError::ConflictingBinding)
        ));
        wrong_binding.purchase_request_id = id(91);
        for answer in [Vec::new(), vec![0; super::codec::MAX_ANSWER_BYTES + 1]] {
            assert!(matches!(
                h.root
                    .record_store_answer(wrong_binding.clone(), answer)
                    .await,
                Err(StoreDurabilityError::Invalid(
                    StoreValidationError::InvalidBytes
                ))
            ));
        }
        for limit in [0, 257] {
            assert!(matches!(
                h.root.list_unacknowledged_store_receipts(limit).await,
                Err(StoreDurabilityError::Invalid(
                    StoreValidationError::InvalidQuantity
                ))
            ));
        }
        assert!(
            !h.root
                .mark_store_receipt_acknowledged(id(99))
                .await
                .map_err(debug)?
        );
        assert_eq!(h.count("store_purchase_requests").await?, 1);
        Ok(())
    })
}

// Independent Root tasks: a single Root deliberately rejects overlapping passes.
async fn concurrent<A: std::future::Future, B: std::future::Future>(
    left: A,
    right: B,
) -> (A::Output, B::Output) {
    use std::task::Poll;
    let mut left = std::pin::pin!(left);
    let mut right = std::pin::pin!(right);
    let (mut a, mut b) = (None, None);
    std::future::poll_fn(|cx| {
        if a.is_none()
            && let Poll::Ready(value) = left.as_mut().poll(cx)
        {
            a = Some(value);
        }
        if b.is_none()
            && let Poll::Ready(value) = right.as_mut().poll(cx)
        {
            b = Some(value);
        }
        match (a.take(), b.take()) {
            (Some(left), Some(right)) => Poll::Ready((left, right)),
            (left, right) => {
                a = left;
                b = right;
                Poll::Pending
            }
        }
    })
    .await
}

#[test]
fn store_postgres_outbox_failure_rolls_back_mint_and_receipt() -> TestResult {
    run("store_atomic", async |h| {
        sqlx::query(
            "ALTER TABLE store_receipt_outbox ADD CONSTRAINT injected_outbox_failure CHECK(false)",
        )
        .execute(&h.pool)
        .await?;
        assert!(apply(h, 90, plan(80)?, harness::fence(1)?).await.is_err());
        for table in [
            "game_item_instances",
            "game_character_inbox_deliveries",
            "store_delivery_receipts",
            "store_receipt_outbox",
            "game_reward_claim_inbox_mint_receipts",
        ] {
            assert_eq!(h.count(table).await?, 0, "{table}");
        }
        sqlx::query("ALTER TABLE store_receipt_outbox DROP CONSTRAINT injected_outbox_failure")
            .execute(&h.pool)
            .await?;
        apply(h, 90, plan(80)?, harness::fence(1)?)
            .await
            .map_err(debug)?;
        assert_eq!(h.count("game_item_instances").await?, 1);
        // Re-running this migration preserves retained receipts and triggers.
        let migration = crate::GAME_MIGRATOR
            .iter()
            .find(|m| m.version == 345)
            .ok_or("missing 0345")?;
        sqlx::raw_sql(migration.sql.clone())
            .execute(&h.pool)
            .await?;
        assert_eq!(h.count("store_delivery_receipts").await?, 1);
        Ok(())
    })
}

#[test]
fn store_validation_rejects_invalid_identifiers_and_bounds() -> TestResult {
    let base = PurchaseRequest {
        purchase_request_id: id(90),
        account_id: id(40),
        character_id: id(harness::CHARACTER),
        binding: vec![1; 4096],
    };
    assert_eq!(base.validate(), Ok(()));
    for field in 0..5 {
        let mut request = base.clone();
        match field {
            0 => request.purchase_request_id = [0; 16],
            1 => request.account_id = [0; 16],
            2 => request.character_id = [0; 16],
            3 => request.binding.clear(),
            _ => request.binding.push(0),
        }
        assert!(request.validate().is_err());
    }
    for field in 0..7 {
        let mut delivery = plan(80)?;
        match field {
            0 => delivery.account_id = [0; 16],
            1 => delivery.character_id = [0; 16],
            2 => delivery.world_id = [0; 16],
            3 => delivery.item_instance_id = [0; 16],
            4 => delivery.fulfilment_unit_id.clear(),
            5 => delivery.fulfilment_unit_id.push('\n'),
            _ => delivery.mint.outputs.clear(),
        }
        assert!(delivery.validate().is_err());
    }
    Ok(())
}

async fn runtime_role(root: &crate::durability::DurabilityRoot) -> TestResult {
    // Match the existing native Inbox PG convention: one physical Root holder.
    let mut connection = root.try_acquire_ready().map_err(debug)?;
    sqlx::query("SET ROLE oteryn_game_runtime")
        .execute(&mut *connection)
        .await?;
    let role: String = sqlx::query_scalar("SELECT current_user")
        .fetch_one(&mut *connection)
        .await?;
    assert_eq!(role, "oteryn_game_runtime");
    let disposition = connection
        .return_to_pool_observed_until(
            std::time::Instant::now() + std::time::Duration::from_secs(5),
        )
        .await;
    if !matches!(
        disposition,
        sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle
    ) {
        return Err(format!("runtime holder was not returned: {disposition:?}").into());
    }
    Ok(())
}

async fn new_runtime_root(h: &Harness) -> TestResult<crate::durability::DurabilityRoot> {
    let root = crate::durability::DurabilityRoot::connect_test_runtime(&h.database.url)?;
    assert!(root.maintain_ready_once().await?);
    runtime_role(&root).await?;
    Ok(root)
}
