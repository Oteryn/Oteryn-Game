#![allow(clippy::expect_used)]

use super::*;
use crate::domain::imbuement::{
    ApplyRoute, MAX_REMAINING_MS, OutstandingAccess, SlotBinding, StateError, Tier,
    check_target_slot, outstanding_access,
};
use crate::domain::premium::{PremiumGate, PremiumRefusal};

// These are unchanged actual native fixtures, mounted at the crate root ONLY in the
// disposable qualification overlay. Their future shared test registration is a CP seam.
use crate::durability::character_equipment::{
    EquipmentMove, EquipmentMoveContent, assert_equipment_authority_in_transaction,
    initialize_equipment_in_transaction,
};
use crate::durability::equipment_policy_abi::{
    EquipmentClaims, EquipmentPolicyLookup, QualifiedEquipmentPolicy, policy_registration,
};
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass, ItemTransferOutcome};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{
    ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    WorldId,
};
use crate::item_transfer_postgres_cases as native;
use native::{Harness, TestResult, debug};
use sqlx::Executor;

const STORAGE_DIGEST: [u8; 32] = [1; 32];
const WEAR_SLOTS: [u8; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 10];

/// Test-only item equipment classification, following the actual equipment owner's fixture.
/// These facts do not admit Imbuement content or elapsed time. Only ordinary untimed items
/// are equipped through the unchanged native mint/transfer/equipment owners.
struct StorageProfile;
impl policy_registration::Registered for StorageProfile {}
impl EquipmentPolicyLookup for StorageProfile {
    fn source_digest(&self) -> [u8; 32] {
        STORAGE_DIGEST
    }
    fn equipment_policy(&self, key: &str, revision: &str) -> Option<QualifiedEquipmentPolicy> {
        if revision != "definition-r1" {
            return None;
        }
        let selected = if key == native::BACKPACK {
            None
        } else {
            Some(
                key.strip_prefix("fixture:checkpoint.wear-")?
                    .parse::<u8>()
                    .ok()?,
            )
        };
        let claims = WEAR_SLOTS
            .into_iter()
            .map(|slot| {
                (
                    slot,
                    if Some(slot) == selected {
                        Ok(EquipmentClaims {
                            slots: vec![slot],
                            groups: Vec::new(),
                            level: 0,
                            vocations: Vec::new(),
                        })
                    } else {
                        Err("fixture equipment slot mismatch")
                    },
                )
            })
            .collect();
        QualifiedEquipmentPolicy::from_qualified_content(
            STORAGE_DIGEST,
            claims,
            (key == native::BACKPACK).then_some(3),
        )
    }
}
impl EquipmentMoveContent for StorageProfile {
    fn is_plain_item(&self, _: &str, _: &str) -> bool {
        true
    }
    fn is_timed_item(&self, _: &str, _: &str) -> bool {
        false
    }
}

async fn equipment_authority(
    h: &Harness,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    f: crate::durability::item_transfer::CurrentCharacterItemFence,
    node: &NodeIncarnationProof,
    digest: [u8; 32],
) -> TestResult<super::EquipmentAuthority> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovered = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    Ok(assert_equipment_authority_in_transaction(
        tx,
        &h.root,
        &recovered,
        node,
        &f,
        native::command(900)?,
        digest,
    )
    .await
    .map_err(debug)?)
}

async fn storage_setup(h: &Harness, count: usize) -> TestResult<Vec<ItemInstanceId>> {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(version, "170006");
    eprintln!("IMBUE-STORAGE-PG server_version_num={version}");
    h.pool.execute(IMBUEMENT_CHECKPOINT_SCHEMA_PROPOSAL).await?;
    // Isolated migration-owner fixture privilege, never proposed production elapsed authority.
    h.pool
        .execute("GRANT SELECT,UPDATE(remaining_ms) ON game_item_imbuements TO oteryn_game_runtime")
        .await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovered = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let backpack = h.mint(&recovered, native::BACKPACK, 1).await?;
    assert!(matches!(
        h.transfer(
            &recovered,
            native::fence()?,
            native::to_slot(native::command(1)?, backpack, native::backpack_facts())
        )
        .await
        .map_err(debug)?,
        ItemTransferOutcome::Committed(_)
    ));
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice())
        .bind(STORAGE_DIGEST.as_slice()).bind([2_u8;32].as_slice()).bind([3_u8;32].as_slice()).execute(&h.pool).await?;
    let mut tx = h.pool.begin().await?;
    tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
    let authority = assert_equipment_authority_in_transaction(
        &mut tx,
        &h.root,
        &recovered,
        &h.node,
        &native::fence()?,
        native::command(90)?,
        STORAGE_DIGEST,
    )
    .await
    .map_err(debug)?;
    initialize_equipment_in_transaction(&mut tx, &authority)
        .await
        .map_err(debug)?;
    tx.commit().await?;
    let mut items = Vec::new();
    for (index, slot) in WEAR_SLOTS.into_iter().take(count).enumerate() {
        let key = format!("fixture:checkpoint.wear-{slot}");
        let id = h.mint(&recovered, &key, 1).await?;
        let facts = ItemDefinitionFacts {
            definition: crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: key,
                revision_ref: "definition-r1".into(),
            },
            stack: ItemStackClass::NonStackable,
            container_capacity: None,
            container_slot_equip_pattern: false,
        };
        assert!(matches!(
            h.transfer(
                &recovered,
                native::fence()?,
                native::to_backpack(native::command(10 + index as u64)?, id, facts)
            )
            .await
            .map_err(debug)?,
            ItemTransferOutcome::Committed(_)
        ));
        let moved = h
            .root
            .commit_equipment_move(
                &recovered,
                &h.node,
                &native::fence()?,
                native::command(30 + index as u64)?,
                &StorageProfile,
                EquipmentMove {
                    item: id,
                    from_slot: None,
                    to_slot: Some(slot),
                },
            )
            .await
            .map_err(debug)?;
        assert!(moved.is_ok(), "native equip refused: {moved:?}");
        items.push(ItemInstanceId::from_bytes(id).map_err(debug)?);
        for slot in 0..3_i16 {
            // Raw synthetic retained slot DATA; never an apply/elapsed/content admission.
            let mut occurrence = id;
            occurrence[0] ^= 0x20;
            occurrence[15] = slot as u8;
            let mut transaction = id;
            transaction[0] ^= 0x40;
            transaction[15] = slot as u8;
            sqlx::query("INSERT INTO game_item_imbuements VALUES(encode($1,'hex')::uuid,$2,$3,'raw-imbue-r1',$4,1,1000,encode($5,'hex')::uuid,encode($6,'hex')::uuid)")
                .bind(id.as_slice()).bind(slot).bind(format!("raw:imbue-{slot}")).bind(format!("raw-category-{slot}"))
                .bind(occurrence.as_slice()).bind(transaction.as_slice()).execute(&h.pool).await?;
        }
    }
    Ok(items)
}

async fn snapshot(h: &Harness, items: &[ItemInstanceId]) -> TestResult<CheckpointSnapshot> {
    let mut tx = h.pool.begin().await?;
    tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
    let authority =
        equipment_authority(h, &mut tx, native::fence()?, &h.node, STORAGE_DIGEST).await?;
    let value = load_checkpoint_in_transaction(&mut tx, &authority, items)
        .await
        .map_err(debug)?;
    tx.commit().await?;
    Ok(value)
}
fn decreased(before: &CheckpointSnapshot, remaining: u64) -> CheckpointBatch {
    CheckpointBatch::plan(
        before
            .rows()
            .iter()
            .map(|old| SlotCheckpoint {
                before: old.clone(),
                after: row(old.binding().clone(), remaining),
            })
            .collect(),
    )
    .expect("synthetic downward DATA batch")
}
async fn store(
    h: &Harness,
    before: &CheckpointSnapshot,
    batch: &CheckpointBatch,
) -> TestResult<Result<(), CheckpointStorageError>> {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovered = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    store_with_recovery(h, &recovered, before, batch).await
}
async fn store_with_recovery(
    h: &Harness,
    recovered: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    before: &CheckpointSnapshot,
    batch: &CheckpointBatch,
) -> TestResult<Result<(), CheckpointStorageError>> {
    let mut tx = h.pool.begin().await?;
    tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
    let authority = assert_equipment_authority_in_transaction(
        &mut tx,
        &h.root,
        recovered,
        &h.node,
        &native::fence()?,
        native::command(900)?,
        STORAGE_DIGEST,
    )
    .await
    .map_err(debug)?;
    let result = store_checkpoint_in_transaction(&mut tx, &authority, before, batch).await;
    // Commit even on typed storage refusal: savepoint must have preserved outer atomicity.
    tx.commit().await?;
    Ok(result)
}
fn run_storage<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let admin = native::configured_admin()
        .ok_or("canonical PostgreSQL route required; no skipped qualification")?;
    native::runtime()?.block_on(async {
        let h = Harness::create(admin, tag).await?;
        let result = body(&h).await;
        h.cleanup().await?;
        result
    })
}

#[test]
fn private_native_storage_decreases_to_zero_and_retains_complete_binding_after_reload() -> TestResult
{
    run_storage("imbue_zero", async |h| {
        let items = storage_setup(h, 1).await?;
        let before = snapshot(h, &items).await?;
        let revision = before.equipment.character_revision;
        store(h, &before, &decreased(&before, 0))
            .await?
            .map_err(debug)?;
        let after = snapshot(h, &items).await?;
        assert_eq!(after.rows.len(), 3);
        assert!(after.rows.iter().all(|r| r.remaining_ms() == 0));
        for (old, new) in before.rows.iter().zip(&after.rows) {
            assert_eq!(old.binding(), new.binding());
        }
        assert_eq!(after.equipment.character_revision, revision);
        assert_eq!(before.equipment, after.equipment);
        assert!(matches!(
            store(h, &before, &decreased(&before, 0)).await?,
            Err(CheckpointStorageError::Stale)
        ));
        assert_eq!(snapshot(h, &items).await?, after);
        Ok(())
    })
}

#[test]
fn private_native_storage_accepts_mixed_and_entirely_unchanged_checkpoint_snapshots() -> TestResult
{
    run_storage("imbue_noop", async |h| {
        let items = storage_setup(h, 2).await?;
        let before = snapshot(h, &items).await?;
        let changed_item = before.items[0];
        let mixed = CheckpointBatch::plan(
            before
                .rows()
                .iter()
                .map(|old| SlotCheckpoint {
                    before: old.clone(),
                    after: row(
                        old.binding().clone(),
                        if old.binding().item == changed_item {
                            900
                        } else {
                            1000
                        },
                    ),
                })
                .collect(),
        )
        .map_err(debug)?;
        assert_eq!(
            mixed.usage,
            CheckpointUsage {
                items: 1,
                participants: 1,
                work: 4
            }
        );
        store(h, &before, &mixed).await?.map_err(debug)?;
        let after = snapshot(h, &items).await?;
        assert_eq!(before.equipment, after.equipment);
        for (old, new) in before.rows().iter().zip(after.rows()) {
            assert_eq!(old.binding(), new.binding());
            assert_eq!(
                new.remaining_ms(),
                if old.binding().item == changed_item {
                    900
                } else {
                    1000
                }
            );
        }
        let noop = CheckpointBatch::plan(
            after
                .rows()
                .iter()
                .map(|old| SlotCheckpoint {
                    before: old.clone(),
                    after: old.clone(),
                })
                .collect(),
        )
        .map_err(debug)?;
        assert!(noop.items.is_empty());
        store(h, &after, &noop).await?.map_err(debug)?;
        assert_eq!(snapshot(h, &items).await?, after);
        Ok(())
    })
}

#[test]
fn private_native_storage_rechecks_omitted_items_before_any_checkpoint_write() -> TestResult {
    run_storage("imbue_omitted", async |h| {
        let items = storage_setup(h, 2).await?;
        let before = snapshot(h, &items).await?;
        let changed_item = before.items[0];
        let omitted_item = before.items[1];
        let batch = CheckpointBatch::plan(
            before
                .rows()
                .iter()
                .filter(|old| old.binding().item == changed_item)
                .map(|old| SlotCheckpoint {
                    before: old.clone(),
                    after: row(old.binding().clone(), 900),
                })
                .collect(),
        )
        .map_err(debug)?;
        // The complete snapshot remains binding even for an item omitted from the write set.
        // Neither substituted snapshot DATA nor a real concurrent checkpoint may disappear
        // when the store accepts a strict subset of the originally read items.
        let mut forged = before.clone();
        let unchanged = forged
            .rows
            .iter_mut()
            .find(|r| r.binding().item == omitted_item)
            .ok_or("fixture omitted item row missing")?;
        let mut rebound = unchanged.binding().clone();
        rebound.content_revision = "substituted-omitted-revision".into();
        *unchanged = row(rebound, unchanged.remaining_ms());
        assert!(matches!(
            store(h, &forged, &batch).await?,
            Err(CheckpointStorageError::Stale)
        ));
        assert_eq!(snapshot(h, &items).await?, before);

        let unselected = CheckpointBatch::plan(vec![change(220, 0, 1000, 900)]).map_err(debug)?;
        assert!(matches!(
            store(h, &before, &unselected).await?,
            Err(CheckpointStorageError::InvalidBatch)
        ));
        assert_eq!(snapshot(h, &items).await?, before);

        let other = snapshot(h, &[omitted_item]).await?;
        store(h, &other, &decreased(&other, 800))
            .await?
            .map_err(debug)?;
        let committed = snapshot(h, &items).await?;
        assert!(matches!(
            store(h, &before, &batch).await?,
            Err(CheckpointStorageError::Stale)
        ));
        assert_eq!(snapshot(h, &items).await?, committed);
        assert!(
            committed
                .rows()
                .iter()
                .filter(|r| r.binding().item == changed_item)
                .all(|r| r.remaining_ms() == 1000)
        );
        Ok(())
    })
}

#[test]
fn private_native_storage_rejects_each_complete_before_binding_substitution() -> TestResult {
    run_storage("imbue_binding", async |h| {
        let items = storage_setup(h, 1).await?;
        let before = snapshot(h, &items).await?;
        for mutation in 0..10 {
            let mut wrong = before.clone();
            let mut binding = wrong.rows[0].binding().clone();
            match mutation {
                0 => binding.item = item(2),
                1 => binding.slot = 2,
                2 => binding.key = "raw:other".into(),
                3 => binding.content_revision = "raw-imbue-r2".into(),
                4 => binding.category = "raw-other".into(),
                5 => binding.tier = Tier::Intricate,
                6 => binding.applying_occurrence = uuid(102),
                7 => binding.transaction_id = uuid(103),
                8 => {}
                _ => wrong.equipment.items[0].state_revision += 1,
            }
            wrong.rows[0] = row(binding, if mutation == 8 { 999 } else { 1000 });
            let batch = if mutation == 0 || mutation == 1 {
                decreased(&before, 900)
            } else {
                decreased(&wrong, 900)
            };
            assert!(matches!(
                store(h, &wrong, &batch).await?,
                Err(CheckpointStorageError::Stale)
            ));
            assert_eq!(snapshot(h, &items).await?, before);
        }
        store(h, &before, &decreased(&before, 900))
            .await?
            .map_err(debug)?;
        Ok(())
    })
}

#[test]
fn private_native_storage_rolls_back_earlier_rows_even_when_outer_transaction_commits() -> TestResult
{
    run_storage("imbue_rollback", async |h| {
        let items = storage_setup(h, 1).await?;
        let before = snapshot(h, &items).await?;
        h.pool.execute("CREATE FUNCTION imbue_fixture_failure() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN IF NEW.slot=1 THEN RAISE EXCEPTION 'injected second-row failure' USING ERRCODE='23514';END IF;RETURN NEW;END$$;CREATE TRIGGER imbue_fixture_failure AFTER UPDATE ON game_item_imbuements FOR EACH ROW EXECUTE FUNCTION imbue_fixture_failure();").await?;
        assert!(matches!(
            store(h, &before, &decreased(&before, 800)).await?,
            Err(CheckpointStorageError::Database(_))
        ));
        assert_eq!(snapshot(h, &items).await?, before);
        h.pool.execute("DROP TRIGGER imbue_fixture_failure ON game_item_imbuements;DROP FUNCTION imbue_fixture_failure()").await?;
        store(h, &before, &decreased(&before, 800))
            .await?
            .map_err(debug)?;
        Ok(())
    })
}

#[test]
fn private_native_storage_guards_reject_rebind_increase_delete_and_truncate() -> TestResult {
    run_storage("imbue_guards", async |h| {
        let items = storage_setup(h, 1).await?;
        let before = snapshot(h, &items).await?;
        for sql in [
            "UPDATE game_item_imbuements SET tier=2",
            "UPDATE game_item_imbuements SET imbuement_key='raw:other'",
            "UPDATE game_item_imbuements SET remaining_ms=1001",
            "DELETE FROM game_item_imbuements",
            "TRUNCATE game_item_imbuements",
        ] {
            let error = h
                .pool
                .execute(sql)
                .await
                .expect_err("private binding guard");
            assert_eq!(
                error.as_database_error().and_then(|e| e.code()).as_deref(),
                Some("23514")
            );
            assert_eq!(snapshot(h, &items).await?, before);
        }
        Ok(())
    })
}

#[test]
fn private_native_storage_uses_actual_session_node_content_and_transaction_identity() -> TestResult
{
    run_storage("imbue_authority", async |h| {
        let items = storage_setup(h, 1).await?;
        let before = snapshot(h, &items).await?;
        let wrong_node = native::register(&h.root, 2).await?;
        for mutation in 0..8 {
            let mut f = native::fence()?;
            let mut digest = STORAGE_DIGEST;
            match mutation {
                0 => f.connection_generation = ConnectionGeneration::new(2)?,
                1 => f.character_lease_generation = 2,
                2 => f.scope_ownership_generation = ScopeOwnershipGeneration::new(2)?,
                3 => digest = [9; 32],
                4 => {}
                5 => f.game_session_id = GameSessionId::decode(&uuid(201))?,
                6 => f.character_id = crate::domain::CharacterId::from_bytes(uuid(202))?,
                _ => {
                    f.runtime_scope = RuntimeScopeRefV1::channel(
                        WorldId::decode(&uuid(203))?,
                        ChannelId::decode(&native::id(native::CHANNEL))?,
                    )
                }
            }
            let mut tx = h.pool.begin().await?;
            tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
            assert!(
                equipment_authority(
                    h,
                    &mut tx,
                    f,
                    if mutation == 4 { &wrong_node } else { &h.node },
                    digest
                )
                .await
                .is_err()
            );
            tx.rollback().await?;
            assert_eq!(snapshot(h, &items).await?, before);
        }
        let mut first = h.pool.begin().await?;
        first.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
        let native_authority =
            equipment_authority(h, &mut first, native::fence()?, &h.node, STORAGE_DIGEST).await?;
        first.commit().await?;
        let mut second = h.pool.begin().await?;
        second.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
        assert!(matches!(
            store_checkpoint_in_transaction(
                &mut second,
                &native_authority,
                &before,
                &decreased(&before, 900)
            )
            .await,
            Err(CheckpointStorageError::Equipment(_))
        ));
        second.commit().await?;
        assert_eq!(snapshot(h, &items).await?, before);
        Ok(())
    })
}

#[test]
fn private_native_storage_rejects_genuine_changed_equipment_and_forged_batch_usage() -> TestResult {
    run_storage("imbue_equipment", async |h| {
        let items = storage_setup(h, 1).await?;
        let before = snapshot(h, &items).await?;
        let batch = decreased(&before, 900);
        let mut forged = batch.clone();
        forged.usage.work = 0;
        assert!(matches!(
            store(h, &before, &forged).await?,
            Err(CheckpointStorageError::InvalidBatch)
        ));
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovered = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        for (n, from, to) in [(200, Some(1), None), (201, None, Some(1))] {
            assert!(
                h.root
                    .commit_equipment_move(
                        &recovered,
                        &h.node,
                        &native::fence()?,
                        native::command(n)?,
                        &StorageProfile,
                        EquipmentMove {
                            item: *items[0].as_bytes(),
                            from_slot: from,
                            to_slot: to
                        }
                    )
                    .await
                    .map_err(debug)?
                    .is_ok()
            );
        }
        assert!(matches!(
            store(h, &before, &batch).await?,
            Err(CheckpointStorageError::Stale)
        ));
        let current = snapshot(h, &items).await?;
        assert_eq!(current.rows, before.rows);
        assert_ne!(current.equipment.revision, before.equipment.revision);
        store(h, &current, &decreased(&current, 900))
            .await?
            .map_err(debug)?;
        Ok(())
    })
}

#[test]
fn private_native_storage_serializes_real_concurrent_before_snapshots() -> TestResult {
    run_storage("imbue_race", async |h| {
        let items = storage_setup(h, 1).await?;
        let before = snapshot(h, &items).await?;
        let batch = decreased(&before, 700);
        // Native Root has one fail-fast semantic pass. Reconcile genuinely before joining;
        // each independent SQL transaction still obtains/checks its OWN opaque equipment
        // authority/current recovery/session/node/content under actual native predicates.
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovered = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (a, b) = native::join_two(
            store_with_recovery(h, &recovered, &before, &batch),
            store_with_recovery(h, &recovered, &before, &batch),
        )
        .await;
        let results = [a?, b?];
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|r| matches!(r, Err(CheckpointStorageError::Stale)))
                .count(),
            1
        );
        let after = snapshot(h, &items).await?;
        assert!(after.rows.iter().all(|r| r.remaining_ms() == 700));
        Ok(())
    })
}

#[test]
fn private_native_storage_current_owner_maximum_is_nine_equipped_items_not_a_fabricated_tenth()
-> TestResult {
    run_storage("imbue_nine", async |h| {
        let items = storage_setup(h, 9).await?;
        let before = snapshot(h, &items).await?;
        let batch = decreased(&before, 600);
        assert_eq!(batch.usage.items, 9);
        assert_eq!(batch.usage.work, 36);
        store(h, &before, &batch).await?.map_err(debug)?;
        assert_eq!(snapshot(h, &items).await?.rows.len(), 27);
        assert_eq!(MAX_CHECKPOINT_ITEMS, 10);
        assert_eq!(MAX_CHECKPOINT_WORK, 40);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovered = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // A genuine live native MINT in Ground custody, not a nonexistent asset ID.
        let foreign = ItemInstanceId::from_bytes(
            h.mint(&recovered, "fixture:checkpoint.not-equipped", 1)
                .await?,
        )
        .map_err(debug)?;
        let mut tx = h.pool.begin().await?;
        tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
        let authority =
            equipment_authority(h, &mut tx, native::fence()?, &h.node, STORAGE_DIGEST).await?;
        assert!(matches!(
            load_checkpoint_in_transaction(&mut tx, &authority, &[foreign]).await,
            Err(CheckpointStorageError::NotEquipped)
        ));
        let mut over_budget = items.clone();
        over_budget.extend([foreign, item(221)]);
        assert!(matches!(
            load_checkpoint_in_transaction(&mut tx, &authority, &over_budget).await,
            Err(CheckpointStorageError::InvalidBatch)
        ));
        tx.rollback().await?;
        Ok(())
    })
}

fn uuid(tag: u8) -> [u8; 16] {
    let mut value = [0; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value[15] = tag;
    value
}

fn item(tag: u8) -> ItemInstanceId {
    ItemInstanceId::from_bytes(uuid(tag)).expect("native UUIDv7 constructor")
}

fn binding(tag: u8, slot: u8) -> SlotBinding {
    // Synthetic raw row bindings, not admitted definitions or current authority.
    let mut applying_occurrence = uuid(100);
    applying_occurrence[0] = tag;
    applying_occurrence[1] = slot;
    let mut transaction_id = uuid(101);
    transaction_id[0] = tag;
    transaction_id[1] = slot;
    SlotBinding {
        item: item(tag),
        slot,
        key: format!("fixture:imbue-{slot}"),
        content_revision: "definition-r1".into(),
        category: format!("fixture-category-{slot}"),
        tier: Tier::Basic,
        applying_occurrence,
        transaction_id,
    }
}

fn row(binding: SlotBinding, remaining: u64) -> SlotRecord {
    SlotRecord::new(binding, remaining).expect("structural row fixture")
}

fn change(tag: u8, slot: u8, before: u64, after: u64) -> SlotCheckpoint {
    SlotCheckpoint {
        before: row(binding(tag, slot), before),
        after: row(binding(tag, slot), after),
    }
}

#[test]
fn structural_rows_accept_exact_bounds_and_reject_max_plus_one_or_invalid_bindings() {
    assert!(SlotRecord::new(binding(1, 2), 0).is_ok());
    assert!(SlotRecord::new(binding(1, 2), MAX_REMAINING_MS).is_ok());
    assert_eq!(
        SlotRecord::new(binding(1, 3), 1).err(),
        Some(StateError::InvalidBinding)
    );
    assert_eq!(
        SlotRecord::new(binding(1, 0), MAX_REMAINING_MS + 1).err(),
        Some(StateError::InvalidRemaining)
    );
    assert_eq!(
        SlotRecord::new(binding(1, 0), u64::MAX).err(),
        Some(StateError::InvalidRemaining)
    );
    for mutation in 0..5 {
        let mut raw = binding(1, 0);
        match mutation {
            0 => raw.key.clear(),
            1 => raw.content_revision = " ".into(),
            2 => raw.category.push('\n'),
            3 => raw.applying_occurrence = [0; 16],
            _ => raw.transaction_id[6] = 0,
        }
        assert_eq!(
            SlotRecord::new(raw, 1).err(),
            Some(StateError::InvalidBinding)
        );
    }
}

#[test]
fn native_premium_is_only_a_subcheck_and_no_result_grants_quest_or_content_access() {
    // Classification data for a pure check, not a current fence fixture for a mutation.
    let open = PremiumGate::from_fence_read(true);
    for tier in [Tier::Basic, Tier::Intricate, Tier::Powerful] {
        assert_eq!(
            outstanding_access(ApplyRoute::ScrollApply, tier, PremiumGate::CLOSED),
            Ok(OutstandingAccess::ItemAndContentBindings)
        );
        assert_eq!(
            outstanding_access(ApplyRoute::ScrollCreate, tier, PremiumGate::CLOSED),
            Err(PremiumRefusal::NotCurrent)
        );
        assert_eq!(
            outstanding_access(ApplyRoute::ScrollCreate, tier, open),
            Ok(OutstandingAccess::QuestAndContentBindings)
        );
        assert_eq!(
            outstanding_access(ApplyRoute::Shrine, tier, open),
            Ok(OutstandingAccess::QuestAndContentBindings)
        );
        assert_eq!(
            outstanding_access(ApplyRoute::Shrine, tier, PremiumGate::default()),
            if tier == Tier::Basic {
                Ok(OutstandingAccess::QuestAndContentBindings)
            } else {
                Err(PremiumRefusal::NotCurrent)
            }
        );
    }
    assert_eq!(
        outstanding_access(
            ApplyRoute::Shrine,
            Tier::Powerful,
            PremiumGate::from_fence_read(false)
        ),
        Err(PremiumRefusal::NotCurrent)
    );
}

#[test]
fn missing_or_invalid_slot_rules_fail_closed() {
    assert_eq!(
        check_target_slot(item(1), &[], None, 0, "category"),
        Err(StateError::UnresolvedItemRules)
    );
    assert_eq!(
        check_target_slot(item(1), &[], Some(0), 0, "category"),
        Err(StateError::SlotUnavailable)
    );
    assert_eq!(
        check_target_slot(item(1), &[], Some(4), 0, "category"),
        Err(StateError::InvalidItemRules)
    );
    for category in ["", "   ", "category\n"] {
        assert_eq!(
            check_target_slot(item(1), &[], Some(3), 0, category),
            Err(StateError::InvalidItemRules)
        );
    }
}

#[test]
fn every_tier_fits_a_slot_bearing_item_while_shrine_premium_stays_independent() {
    // Native pure flow helpers, not current session/quest/content authority for a mutation.
    let premium = PremiumGate::from_fence_read(true);
    let free = PremiumGate::from_fence_read(false);
    for tier in [Tier::Basic, Tier::Intricate, Tier::Powerful] {
        let mut proposed = binding(1, 0);
        proposed.tier = tier;
        let candidate = row(proposed, 1_000);
        let proposed = candidate.binding();
        assert_eq!(
            check_target_slot(
                proposed.item,
                &[],
                Some(1),
                proposed.slot,
                &proposed.category
            ),
            Ok(())
        );
        assert_eq!(
            outstanding_access(ApplyRoute::Shrine, proposed.tier, premium),
            Ok(OutstandingAccess::QuestAndContentBindings)
        );
        assert_eq!(
            outstanding_access(ApplyRoute::Shrine, proposed.tier, free),
            if tier == Tier::Basic {
                Ok(OutstandingAccess::QuestAndContentBindings)
            } else {
                Err(PremiumRefusal::NotCurrent)
            }
        );
    }
}

#[test]
fn zero_time_rows_still_occupy_their_slot_and_category_until_audited_expiry() {
    let expired = row(binding(1, 0), 0);
    assert_eq!(
        check_target_slot(item(1), std::slice::from_ref(&expired), Some(3), 0, "other"),
        Err(StateError::SlotUnavailable)
    );
    assert_eq!(
        check_target_slot(
            item(1),
            std::slice::from_ref(&expired),
            Some(3),
            1,
            &expired.binding().category,
        ),
        Err(StateError::CategoryOccupied)
    );
    assert_eq!(
        check_target_slot(item(1), &[expired], Some(3), 1, "other"),
        Ok(())
    );
}

#[test]
fn wrong_item_duplicate_slots_and_duplicate_categories_are_invalid_snapshots() {
    let first = row(binding(1, 0), 100);
    let mut same_category = binding(1, 1);
    same_category.category = first.binding().category.clone();
    let mut same_slot = binding(1, 1);
    same_slot.slot = 0;
    for rows in [
        vec![row(binding(2, 0), 100)],
        vec![first.clone(), row(same_slot, 100)],
        vec![first.clone(), row(same_category, 100)],
    ] {
        assert_eq!(
            check_target_slot(item(1), &rows, Some(3), 2, "other"),
            Err(StateError::InvalidSnapshot)
        );
    }
}

#[test]
fn each_immutable_binding_field_is_compared_before_planning_a_checkpoint() {
    let before = row(binding(1, 0), 1_000);
    for mutation in 0..8 {
        let mut changed = before.binding().clone();
        match mutation {
            0 => changed.item = item(2),
            1 => changed.slot = 1,
            2 => changed.key = "fixture:other".into(),
            3 => changed.content_revision = "definition-r2".into(),
            4 => changed.category = "other".into(),
            5 => changed.tier = Tier::Intricate,
            6 => changed.applying_occurrence = uuid(102),
            _ => changed.transaction_id = uuid(103),
        }
        let candidate = SlotCheckpoint {
            before: before.clone(),
            after: row(changed, 999),
        };
        assert_eq!(
            CheckpointBatch::plan(vec![candidate]).err(),
            Some(CheckpointError::BindingChanged)
        );
    }
}

#[test]
fn checkpoints_never_increase_time_and_noop_snapshots_produce_no_mutation() {
    assert_eq!(
        CheckpointBatch::plan(vec![change(1, 0, 100, 101)]).err(),
        Some(CheckpointError::TimeIncreased)
    );
    let empty = CheckpointBatch::plan(vec![change(1, 0, 100, 100)]).expect("no change");
    assert!(empty.items.is_empty());
    assert_eq!(
        empty.usage,
        CheckpointUsage {
            items: 0,
            participants: 0,
            work: 0
        }
    );
    let zero =
        CheckpointBatch::plan(vec![change(1, 0, 1, 0)]).expect("zero stays a row, no delete");
    assert_eq!(zero.items[0].changes[0].after.remaining_ms(), 0);
    assert_eq!(zero.usage.work, 2);
}

#[test]
fn duplicate_checkpoint_slots_or_categories_cannot_mutate_a_row_twice() {
    let first = change(1, 0, 100, 99);
    assert_eq!(
        CheckpointBatch::plan(vec![first.clone(), first.clone()]).err(),
        Some(CheckpointError::DuplicateSlot)
    );
    let mut other_binding = binding(1, 1);
    other_binding.category = first.before.binding().category.clone();
    let second = SlotCheckpoint {
        before: row(other_binding.clone(), 100),
        after: row(other_binding, 99),
    };
    assert_eq!(
        CheckpointBatch::plan(vec![first, second]).err(),
        Some(CheckpointError::DuplicateCategory)
    );
}

#[test]
fn ten_items_thirty_rows_forty_work_are_accepted_in_deterministic_item_slot_order() {
    let changes: Vec<_> = (1..=10)
        .flat_map(|tag| (0..3).map(move |slot| change(tag, slot, 72_000_000, 71_940_000)))
        .collect();
    let expected = CheckpointBatch::plan(changes.clone()).expect("registered maxima");
    let mut reversed = changes;
    reversed.reverse();
    assert_eq!(
        CheckpointBatch::plan(reversed).expect("same batch order"),
        expected
    );
    assert_eq!(
        expected.usage,
        CheckpointUsage {
            items: 10,
            participants: 10,
            work: 40
        }
    );
    assert_eq!(
        expected.items.first().map(|group| group.item),
        Some(item(1))
    );
    assert_eq!(
        expected.items.last().map(|group| group.item),
        Some(item(10))
    );
    assert!(expected.items.iter().all(|group| {
        group
            .changes
            .iter()
            .map(|delta| delta.before.binding().slot)
            .eq([0, 1, 2])
    }));
}

#[test]
fn eleven_items_or_thirty_one_input_rows_are_refused_before_any_output() {
    for count in [11, 31] {
        let changes = (1..=count).map(|tag| change(tag, 0, 100, 99)).collect();
        assert_eq!(
            CheckpointBatch::plan(changes).err(),
            Some(CheckpointError::BudgetExceeded)
        );
    }
}

#[test]
fn each_numeric_ceiling_refuses_maximum_plus_one_and_extreme_counts() {
    assert_eq!(
        CheckpointUsage {
            items: 10,
            participants: 10,
            work: 40
        }
        .check(),
        Ok(())
    );
    for usage in [
        CheckpointUsage {
            items: 11,
            participants: 10,
            work: 40,
        },
        CheckpointUsage {
            items: 10,
            participants: 11,
            work: 40,
        },
        CheckpointUsage {
            items: 10,
            participants: 10,
            work: 41,
        },
        CheckpointUsage {
            items: usize::MAX,
            participants: 10,
            work: 40,
        },
    ] {
        assert_eq!(usage.check(), Err(CheckpointError::BudgetExceeded));
    }
}

#[test]
fn checkpoint_does_not_rebind_old_applications_to_a_global_active_revision_or_restart_time() {
    let first = change(1, 0, 100, 80);
    let mut raw = binding(2, 0);
    raw.content_revision = "older-definition-revision".into();
    raw.applying_occurrence = uuid(104);
    let second = SlotCheckpoint {
        before: row(raw.clone(), 9),
        after: row(raw, 8),
    };
    let batch =
        CheckpointBatch::plan(vec![second, first]).expect("each existing row binds nothing new");
    assert_eq!(batch.items[0].changes[0].after.remaining_ms(), 80);
    assert_eq!(
        batch.items[1].changes[0].after.binding().content_revision,
        "older-definition-revision"
    );
    assert_eq!(batch.items[1].changes[0].after.remaining_ms(), 8);
}
