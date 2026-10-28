//! F28 native boundary evidence only. The open House container dispatch owner is
//! absent; an interior snapshot below is deliberately NOT called an open container.
//! Reuse the existing isolated House grant fixture, never a new authority model.
use super::*;
use crate::durability::house_interior::load_house_interior_in_transaction;

#[test]
fn f28_native_add_rechecks_acl_at_mutation_boundary() -> TestResult {
    native_acl_boundary(5)
}

#[test]
fn f28_native_remove_rechecks_acl_at_mutation_boundary() -> TestResult {
    native_acl_boundary(6)
}

fn native_acl_boundary(shape: u8) -> TestResult {
    // Independent databases: unchanged R1 control, R2 revocation, and R2 with
    // continued permission (isolates revision checking from role checking).
    for change in 0..3 {
        run_native(
            &format!("house_interior_f28_{shape}_{change}"),
            async |h, backpack, item| {
                sqlx::query("UPDATE test_house_grants SET role='GUEST'")
                    .execute(&h.pool)
                    .await?;
                let request = house_request_for_shape(h, backpack, item, shape).await?;
                let seal = h.recovery.seal_current().map_err(native::debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(native::debug)?;
                let candidate = h
                    .root
                    .freeze_house_item(&authority, &h.node, house_fence()?, request.clone())
                    .await
                    .map_err(native::debug)?;
                let snapshot = {
                    let mut tx = h.pool.begin().await?;
                    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                        .execute(&mut *tx)
                        .await?;
                    let snapshot =
                        load_house_interior_in_transaction(&mut tx, &current_house()?, 1, &h.node)
                            .await
                            .map_err(native::debug)?;
                    tx.commit().await?;
                    snapshot
                };
                assert_eq!(snapshot.items.len(), usize::from(shape == 6));
                if shape == 6 {
                    assert_eq!(snapshot.items[0].item_instance_id, item);
                    assert_eq!(snapshot.items[0].quantity, 1);
                }
                let before = house_value_snapshot(h).await?;
                if change != 0 {
                    let mut tx = h.pool.begin().await?;
                    assert_eq!(
                        sqlx::query("UPDATE test_house_properties SET acl_revision=2")
                            .execute(&mut *tx)
                            .await?
                            .rows_affected(),
                        1
                    );
                    if change == 1 {
                        assert_eq!(
                            sqlx::query("DELETE FROM test_house_grants")
                                .execute(&mut *tx)
                                .await?
                                .rows_affected(),
                            1
                        );
                    }
                    tx.commit().await?;
                }
                let result = h
                    .root
                    .commit_house_item(&authority, &h.node, house_fence()?, &candidate)
                    .await;
                if change == 0 {
                    committed(result.map_err(native::debug)?)?;
                    assert_ne!(house_value_snapshot(h).await?, before);
                } else {
                    assert!(
                        matches!(
                            (&result, change),
                            (Err(HouseWriterError::NoAccess), 1)
                                | (Err(HouseWriterError::AccessChanged), 2)
                        ),
                        "F28 shape {shape}, ACL change {change}: {result:?}"
                    );
                    assert_eq!(house_value_snapshot(h).await?, before);
                    // Retaining the old read and the independently frozen R1
                    // candidate did not grant any new write authority.
                    assert_eq!(snapshot.ownership_generation, 1);
                    if change == 2 {
                        let mut fresh = request;
                        fresh.expected_acl_revision = 2;
                        fresh.command = CommandRef::new(
                            house_fence()?.game_session_id,
                            CommandId::new(902).map_err(native::debug)?,
                        );
                        let fresh = h
                            .root
                            .freeze_house_item(&authority, &h.node, house_fence()?, fresh)
                            .await
                            .map_err(native::debug)?;
                        committed(
                            h.root
                                .commit_house_item(&authority, &h.node, house_fence()?, &fresh)
                                .await
                                .map_err(native::debug)?,
                        )?;
                        assert_ne!(house_value_snapshot(h).await?, before);
                    }
                }
                if change != 1 {
                    let (in_house, in_backpack, quantity): (bool, bool, i64) = sqlx::query_as(
                        "SELECT EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=i.item_instance_id), \
                         EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id AND parent_item_instance_id=encode($2,'hex')::uuid), \
                         i.quantity FROM game_item_instances i WHERE i.item_instance_id=encode($1,'hex')::uuid",
                    )
                    .bind(item.as_slice())
                    .bind(backpack.as_slice())
                    .fetch_one(&h.pool)
                    .await?;
                    assert_eq!(
                        (in_house, in_backpack, quantity),
                        (shape == 5, shape == 6, 1)
                    );
                }
                Ok(())
            },
        )?;
    }
    Ok(())
}
