use super::*;
use crate::durability::spell_item_transaction::{
    apply_spell_items_in_transaction, apply_spell_items_in_transaction_guarded,
};
use crate::durability::spell_items_abi::{
    SpellFieldClass, SpellFieldOrigin, SpellItemOperation, SpellItemTransactionOutcome,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn admit_cap_2_postgres_non_field_cost_path_does_not_consult_field_capabilities() -> TestResult {
    run("admit_cap_2_control", async |h| {
        let (_runtime, origin) = spell_origin(h)?;
        let request = spell_request(&origin, 700, 160, vec![spell_mint(180)])?;
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let refused = SpellFieldOrigin::Player(Arc::new(move |_, _, _| {
            observed.fetch_add(1, Ordering::SeqCst);
            false
        }));
        let before = spell_footprint(h).await?;
        let mut tx = h.pool.begin().await?;
        let (_permit, authority) = player_authority(&mut tx, h, request.command).await?;
        assert!(matches!(
            apply_spell_items_in_transaction(&mut tx, &authority, &request, &refused)
                .await
                .map_err(debug)?,
            SpellItemTransactionOutcome::Applied(_)
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        tx.rollback().await?;
        assert_eq!(spell_footprint(h).await?, before);
        Ok(())
    })
}

#[test]
fn admit_cap_2_postgres_field_and_wall_refuse_without_any_durable_trace() -> TestResult {
    run("admit_cap_2_refusal", async |h| {
        let (_runtime, origin) = spell_origin(h)?;
        let before = spell_footprint(h).await?;
        for class in [SpellFieldClass::Field, SpellFieldClass::Wall] {
            let mut operation = spell_mint(180);
            let SpellItemOperation::MintGround { definition, .. } = &mut operation else {
                return Err("mint".into());
            };
            definition.field_class = class;
            let mut qualified = spell_definition();
            qualified.field_class = class;
            let variants = [
                operation,
                SpellItemOperation::MintInventory {
                    item_instance_id: id(180),
                    definition: qualified.clone(),
                    quantity: 1,
                    backpack: spell_backpack(),
                    overflow_placement: ground_placement(&target()),
                    allow_ground_overflow: false,
                },
                SpellItemOperation::MintInventory {
                    item_instance_id: id(180),
                    definition: qualified,
                    quantity: 1,
                    backpack: spell_backpack(),
                    overflow_placement: ground_placement(&target()),
                    allow_ground_overflow: true,
                },
            ];
            for operation in variants {
                let request = spell_request(&origin, 700, 160, vec![operation])?;
                for provenance in [
                    SpellFieldOrigin::Player(Arc::new(|_, _, _| false)),
                    SpellFieldOrigin::Monster,
                    SpellFieldOrigin::System,
                ] {
                    let mut tx = h.pool.begin().await?;
                    let (_permit, authority) =
                        player_authority(&mut tx, h, request.command).await?;
                    assert!(matches!(
                        apply_spell_items_in_transaction(
                            &mut tx,
                            &authority,
                            &request,
                            &provenance
                        )
                        .await,
                        Err(SpellItemError::FieldCapabilityRefused)
                    ));
                    let count: i64 =
                        sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts")
                            .fetch_one(&mut *tx)
                            .await?;
                    assert_eq!(count, before[2], "refusal does not even stage a receipt");
                    tx.rollback().await?;
                    assert_eq!(spell_footprint(h).await?, before);
                }
            }
        }
        Ok(())
    })
}

#[test]
fn admit_cap_2_postgres_exact_receipt_replays_before_refusing_new_field_writes() -> TestResult {
    run("admit_cap_2_replay", async |h| {
        let (_runtime, origin) = spell_origin(h)?;
        let mut operation = spell_mint(180);
        let SpellItemOperation::MintGround { definition, .. } = &mut operation else {
            return Err("mint".into());
        };
        definition.field_class = SpellFieldClass::Field;
        let request = spell_request(&origin, 700, 160, vec![operation])?;
        let scope = harness::scope()?;
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let expected_scope = scope;
        let allowed = SpellFieldOrigin::Player(Arc::new(move |scope, generation, policy| {
            observed.fetch_add(1, Ordering::SeqCst);
            scope == expected_scope && generation == 1 && policy == "world-policy-1"
        }));
        let mut tx = h.pool.begin().await?;
        let (_permit, authority) = player_authority(&mut tx, h, request.command).await?;
        let SpellItemTransactionOutcome::Applied(receipt) =
            apply_spell_items_in_transaction(&mut tx, &authority, &request, &allowed)
                .await
                .map_err(debug)?
        else {
            return Err("new receipt".into());
        };
        tx.commit().await?;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let before = spell_footprint(h).await?;
        drop(_permit);
        let mut tx = h.pool.begin().await?;
        let (_permit, authority) = player_authority(&mut tx, h, request.command).await?;
        let refused = SpellFieldOrigin::Player(Arc::new(|_, _, _| false));
        assert!(
            matches!(apply_spell_items_in_transaction_guarded(&mut tx, &authority, &request,
            Err(SpellItemError::Rejected("current owner changed")), &refused).await.map_err(debug)?,
            SpellItemTransactionOutcome::AlreadyCommitted(actual) if actual == receipt)
        );
        tx.rollback().await?;
        assert_eq!(spell_footprint(h).await?, before);
        Ok(())
    })
}
