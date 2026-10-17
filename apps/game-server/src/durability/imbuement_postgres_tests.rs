#![allow(clippy::expect_used)]

use super::*;
use crate::domain::imbuement_checkpoint::{CheckpointUsage, MAX_CHECKPOINT_WORK, SlotCheckpoint};
// Existing canonical-root fixtures and actual native equipment/authority APIs.
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

#[path = "imbuement_test_fixtures.rs"]
mod fixtures;
use fixtures::*;

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
    let Some(admin) = native::configured_admin() else {
        eprintln!("IMBUE-STORAGE-PG NOT_RUN: canonical PostgreSQL route is absent");
        return Ok(());
    };
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

// Audited zero-row expiry cases use this same canonical native PostgreSQL harness.
#[path = "../../tests/support/imbuement_expiry_postgres_cases.rs"]
mod expiry;
