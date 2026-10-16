use super::*;
use crate::durability::item_mint::{GroundPlacement, TypedDefinitionRef};

fn uuid(n: u8) -> [u8; 16] {
    let mut value = [0; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value[15] = n;
    value
}

fn world(n: u8) -> WorldId {
    match WorldId::decode(&uuid(n)) {
        Ok(value) => value,
        Err(error) => unreachable!("valid fixture WorldId: {error:?}"),
    }
}

fn channel(n: u8) -> ChannelId {
    match ChannelId::decode(&uuid(n)) {
        Ok(value) => value,
        Err(error) => unreachable!("valid fixture ChannelId: {error:?}"),
    }
}

fn target() -> SpellGroundTarget {
    SpellGroundTarget {
        spatial_position: [
            120_i32.to_be_bytes().as_slice(),
            (-5_i32).to_be_bytes().as_slice(),
            (-1_i16).to_be_bytes().as_slice(),
        ]
        .concat(),
        map_revision: "map-a".into(),
        content_revision: "content-a".into(),
        placement_context: vec![9],
    }
}

fn root(n: u8) -> GroundItemInstance {
    let target = target();
    GroundItemInstance {
        item_instance_id: uuid(n),
        world_id: world(1),
        channel_id: channel(2),
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: "stack".into(),
            revision_ref: "revision-a".into(),
        },
        quantity: 100,
        runtime_scope_ownership_generation: 7,
        ground: GroundPlacement {
            spatial_position: target.spatial_position,
            corpse_ref: Vec::new(),
            map_revision: target.map_revision,
            content_revision: target.content_revision,
            native_room_placement_context: target.placement_context,
        },
        minted_transaction_id: uuid(n),
    }
}

fn roots(count: u8) -> Vec<GroundItemInstance> {
    (0..count).map(root).collect()
}

fn check(
    remaining: &[GroundItemInstance],
    inserted: &[GroundItemInstance],
) -> Result<(), GroundCellLimitError> {
    check_ground_cell_root_admission(world(1), channel(2), 7, &target(), remaining, inserted)
}

#[test]
fn accepts_exactly_64_physical_roots_and_refuses_the_65th() {
    let existing = roots(63);
    assert_eq!(check(&existing, &[root(63)]), Ok(()));
    assert_eq!(
        check(&roots(64), &[root(64)]),
        Err(GroundCellLimitError::NotEnoughRoom)
    );
    assert_eq!(check(&[], &roots(64)), Ok(()));
    assert_eq!(
        check(&[], &roots(65)),
        Err(GroundCellLimitError::NotEnoughRoom)
    );
    assert_eq!(
        GroundCellLimitError::NotEnoughRoom.user_message(),
        Some("not enough room")
    );
    assert_eq!(GroundCellLimitError::IncompatibleInput.user_message(), None);
}

#[test]
fn no_new_roots_preserves_preexisting_overfull_ground() {
    assert_eq!(check(&roots(65), &[]), Ok(()));
    assert_eq!(check(&roots(255), &[]), Ok(()));
    assert_eq!(
        check(&roots(65), &[root(65)]),
        Err(GroundCellLimitError::NotEnoughRoom)
    );
}

#[test]
fn overfull_no_new_root_still_checks_identity_scope_and_placement() {
    for mutate in [
        |item: &mut GroundItemInstance| item.item_instance_id = uuid(0),
        |item: &mut GroundItemInstance| item.world_id = world(3),
        |item: &mut GroundItemInstance| item.channel_id = channel(3),
        |item: &mut GroundItemInstance| item.runtime_scope_ownership_generation += 1,
        |item: &mut GroundItemInstance| item.ground.spatial_position[0] ^= 1,
        |item: &mut GroundItemInstance| item.ground.map_revision.push('b'),
        |item: &mut GroundItemInstance| item.ground.content_revision.push('b'),
        |item: &mut GroundItemInstance| item.ground.native_room_placement_context.push(1),
    ] {
        let mut existing = roots(65);
        mutate(&mut existing[64]);
        assert_eq!(
            check(&existing, &[]),
            Err(GroundCellLimitError::IncompatibleInput)
        );
    }
}

#[test]
fn a_stack_counts_once_and_native_merge_with_no_new_root_needs_no_room() {
    let mut existing = roots(64);
    existing[0].quantity = u32::MAX;
    assert_eq!(check(&existing, &[]), Ok(()));
    // Native merging changes quantity, not the number of Ground item instances.
    existing[1].quantity = 1;
    assert_eq!(check(&existing, &[]), Ok(()));
}

#[test]
fn actual_root_removal_leaves_room_for_atomic_replacement() {
    let mut remaining = roots(64);
    remaining.remove(0);
    assert_eq!(check(&remaining, &[root(64)]), Ok(()));
    assert_eq!(
        check(&remaining, &[root(64), root(65)]),
        Err(GroundCellLimitError::NotEnoughRoom)
    );
}

#[test]
fn refuses_duplicate_identity_within_or_across_native_row_sets() {
    assert_eq!(
        check(&[root(1), root(1)], &[]),
        Err(GroundCellLimitError::IncompatibleInput)
    );
    assert_eq!(
        check(&[root(1)], &[root(1)]),
        Err(GroundCellLimitError::IncompatibleInput)
    );
    assert_eq!(
        check(&[], &[root(1), root(1)]),
        Err(GroundCellLimitError::IncompatibleInput)
    );
}

#[test]
fn supplied_scope_and_generation_are_compared_independently() {
    let existing = roots(1);
    for (world, channel, generation) in [
        (world(3), channel(2), 7),
        (world(1), channel(3), 7),
        (world(1), channel(2), 8),
    ] {
        assert_eq!(
            check_ground_cell_root_admission(world, channel, generation, &target(), &existing, &[]),
            Err(GroundCellLimitError::IncompatibleInput)
        );
    }
    let mut inserted = root(2);
    inserted.channel_id = channel(3);
    assert_eq!(
        check(&existing, &[inserted]),
        Err(GroundCellLimitError::IncompatibleInput)
    );
}

#[test]
fn incompatible_tile_metadata_is_not_silently_filtered_out() {
    for mutate in [
        |item: &mut GroundItemInstance| item.ground.spatial_position[0] ^= 1,
        |item: &mut GroundItemInstance| item.ground.map_revision.push('b'),
        |item: &mut GroundItemInstance| item.ground.content_revision.push('b'),
        |item: &mut GroundItemInstance| item.ground.native_room_placement_context.push(1),
    ] {
        let mut item = root(0);
        mutate(&mut item);
        assert_eq!(
            check(&[item.clone()], &[]),
            Err(GroundCellLimitError::IncompatibleInput)
        );
        assert_eq!(
            check(&[], &[item]),
            Err(GroundCellLimitError::IncompatibleInput)
        );
    }
}

#[test]
fn opaque_or_truncated_target_is_never_an_empty_cell() {
    for length in [0, 9, 11] {
        let mut target = target();
        target.spatial_position.resize(length, 0);
        assert_eq!(
            check_ground_cell_root_admission(world(1), channel(2), 7, &target, &[], &[]),
            Err(GroundCellLimitError::IncompatibleInput)
        );
    }
}

#[test]
fn repeated_reload_evaluation_has_no_memory_or_mutation() {
    let existing = roots(63);
    let inserted = [root(63)];
    let original = (existing.clone(), inserted.clone());
    assert_eq!(check(&existing, &inserted), Ok(()));
    assert_eq!(check(&existing.clone(), &inserted.clone()), Ok(()));
    assert_eq!((existing, inserted), original);
    // A later independently read occupancy must be checked again.
    assert_eq!(
        check(&roots(64), &[root(64)]),
        Err(GroundCellLimitError::NotEnoughRoom)
    );
}

/// PostgreSQL cases use real native source MINT/TRANSFER writers and current
/// Root/Node/recovery/scope predicates. Prospective insertions remain DATA.
mod postgres {
    use super::super::*;
    use crate::bestiary_postgres_harness as harness;
    use crate::durability::item_mint::{
        ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
    };
    use crate::durability::item_transfer::{
        ItemDefinitionFacts, ItemStackClass, ItemTransferDestination, ItemTransferRequest,
    };
    use crate::durability::spell_item_transaction::{
        SpellItemScopeAuthority, assert_spell_item_scope_in_transaction,
    };
    use crate::durability::spell_owner_commit::SpellLanePermit;
    use crate::foundation::{CommandId, CommandRef, GameSessionId, ScopeOwnershipGeneration};
    use harness::{CHANNEL, Harness, TestResult, WORLD, debug, id};

    fn run<F>(tag: &'static str, body: F) -> TestResult
    where
        F: AsyncFnOnce(&Harness) -> TestResult,
    {
        let admin = harness::configured_admin().ok_or("configured PostgreSQL17.6 is required")?;
        harness::runtime()?.block_on(async move {
            let h = Harness::create(admin, tag, true).await?;
            let version: String = sqlx::query_scalar("SHOW server_version_num")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(version, "170006");
            eprintln!("GROUND-CAP-NATIVE-PG server_version_num={version} fixture={tag}");
            let result = body(&h).await;
            h.cleanup().await?;
            result
        })
    }
    fn target() -> SpellGroundTarget {
        SpellGroundTarget {
            spatial_position: [
                120_i32.to_be_bytes().as_slice(),
                5_i32.to_be_bytes().as_slice(),
                (-1_i16).to_be_bytes().as_slice(),
            ]
            .concat(),
            map_revision: "map-1".into(),
            content_revision: "content-1".into(),
            placement_context: id(6).to_vec(),
        }
    }
    fn definition() -> TypedDefinitionRef {
        TypedDefinitionRef {
            family: "Item".into(),
            production_key: "fixture:ground.root".into(),
            revision_ref: "definition-r1".into(),
        }
    }
    fn prospective(number: u8, target: &SpellGroundTarget) -> TestResult<GroundItemInstance> {
        Ok(GroundItemInstance {
            item_instance_id: id(number),
            world_id: WorldId::decode(&id(WORLD)).map_err(debug)?,
            channel_id: ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
            definition: definition(),
            quantity: 100,
            runtime_scope_ownership_generation: 1,
            ground: GroundPlacement {
                spatial_position: target.spatial_position.clone(),
                corpse_ref: vec![],
                map_revision: target.map_revision.clone(),
                content_revision: target.content_revision.clone(),
                native_room_placement_context: target.placement_context.clone(),
            },
            minted_transaction_id: id(number),
        })
    }
    async fn authority(
        tx: &mut Transaction<'_, Postgres>,
        h: &Harness,
    ) -> TestResult<(SpellLanePermit, SpellItemScopeAuthority)> {
        let permit = SpellLanePermit::of_fresh_scope(harness::scope()?).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let scope = assert_spell_item_scope_in_transaction(
            tx,
            &permit,
            &h.root,
            &recovery,
            &h.node,
            harness::scope()?,
            1,
        )
        .await
        .map_err(debug)?;
        Ok((permit, scope))
    }
    async fn mint(
        h: &Harness,
        first: u32,
        count: u32,
        at: &SpellGroundTarget,
    ) -> TestResult<Vec<[u8; 16]>> {
        mint_quantity(h, first, count, at, 100).await
    }
    async fn mint_quantity(
        h: &Harness,
        first: u32,
        count: u32,
        at: &SpellGroundTarget,
        quantity: u32,
    ) -> TestResult<Vec<[u8; 16]>> {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut ids = Vec::new();
        for actor in first..first + count {
            // Existing native MINT fixture cause: no claim of a gameplay-produced kill.
            // Full native migration, custody, audit, Node/recovery and source guards stay ON.
            let mut candidate = h
                .root
                .freeze_item_mint(
                    &recovery,
                    &h.node,
                    ItemMintRequest {
                        cause: ItemMintCause::for_test(
                            WorldId::decode(&id(WORLD)).map_err(debug)?,
                            ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
                            ScopeOwnershipGeneration::new(1).map_err(debug)?,
                            actor,
                            1,
                            TypedDefinitionRef {
                                family: "LootTable".into(),
                                production_key: "fixture:ground.loot".into(),
                                revision_ref: "definition-r1".into(),
                            },
                            "fixture:ground.drop".into(),
                            0,
                        ),
                        item: definition(),
                        quantity,
                        ground: GroundPlacement {
                            spatial_position: at.spatial_position.clone(),
                            // Native MINT technical bytes require a nonempty reference.
                            corpse_ref: id(3).to_vec(),
                            map_revision: at.map_revision.clone(),
                            content_revision: at.content_revision.clone(),
                            native_room_placement_context: at.placement_context.clone(),
                        },
                        content_revision: "content-1".into(),
                        ruleset_revision: "ruleset-1".into(),
                        sim_revision: "sim-1".into(),
                    },
                )
                .await
                .map_err(debug)?;
            let permit = candidate.fresh_death_lane_permit().await;
            let outcome = h
                .root
                .commit_item_mint(&permit, &recovery, &h.node, &mut candidate)
                .await
                .map_err(debug)?;
            let ItemMintOutcome::Committed(value) = outcome else {
                return Err("fixture MINT replayed unexpectedly".into());
            };
            ids.push(value.item_instance_id);
        }
        Ok(ids)
    }

    #[test]
    fn native_locked_census_enforces_64_and_preserves_zero_admission_overfull_truth() -> TestResult
    {
        run("ground_cap", async |h| {
            mint(h, 1, 63, &target()).await?;
            let mut tx = h.pool.begin().await?;
            let (_permit, scope) = authority(&mut tx, h).await?;
            check_ground_cell_root_admission_in_transaction(
                &mut tx,
                &scope,
                &target(),
                &[prospective(200, &target())?],
            )
            .await
            .map_err(debug)?;
            tx.rollback().await?;
            mint(h, 64, 1, &target()).await?;
            let mut tx = h.pool.begin().await?;
            let (_permit, scope) = authority(&mut tx, h).await?;
            assert!(matches!(
                check_ground_cell_root_admission_in_transaction(
                    &mut tx,
                    &scope,
                    &target(),
                    &[prospective(200, &target())?]
                )
                .await,
                Err(GroundCellAdmissionError::Limit(
                    GroundCellLimitError::NotEnoughRoom
                ))
            ));
            tx.rollback().await?;
            mint(h, 65, 1, &target()).await?;
            let mut tx = h.pool.begin().await?;
            let (_permit, scope) = authority(&mut tx, h).await?;
            // A pre-existing65-root tile is retained. No new root => no capacity charge,
            // not permission to insert, remove, expire or reconstruct any item.
            check_ground_cell_root_admission_in_transaction(&mut tx, &scope, &target(), &[])
                .await
                .map_err(debug)?;
            tx.commit().await?;
            assert_eq!(h.count("game_item_ground_locations").await?, 65);
            assert_eq!(h.root_revision().await?, "1");
            Ok(())
        })
    }

    #[test]
    fn native_source_removal_is_observed_without_a_remaining_rows_dto() -> TestResult {
        run("ground_removal", async |h| {
            let ids = mint_quantity(h, 1, 64, &target(), 1).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let recovery = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            // Genuine current-fenced Ground→container-slot transfer removes one root.
            // The definition's equipment facts are explicit fixture Content DATA.
            let request = ItemTransferRequest {
                command: CommandRef::new(
                    GameSessionId::decode(&id(harness::SESSION)).map_err(debug)?,
                    CommandId::new(1).map_err(debug)?,
                ),
                source_item_instance_id: ids[0],
                destination: ItemTransferDestination::ContainerSlot,
                item: ItemDefinitionFacts {
                    definition: definition(),
                    stack: ItemStackClass::NonStackable,
                    container_capacity: Some(20),
                    container_slot_equip_pattern: true,
                },
                backpack: None,
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            };
            let fence = harness::item_fence()?;
            let mut candidate = h
                .root
                .freeze_item_transfer(&recovery, &h.node, fence, request)
                .await
                .map_err(debug)?;
            let permit = SpellLanePermit::of_fresh_scope(harness::scope()?).await?;
            h.root
                .commit_item_transfer(&permit, &recovery, &h.node, fence, &mut candidate)
                .await
                .map_err(debug)?;
            drop(permit);
            assert_eq!(h.count("game_item_ground_locations").await?, 63);
            let transferred: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND item_instance_id=encode($3,'hex')::uuid)")
                .bind(id(harness::CHARACTER).as_slice()).bind(id(WORLD).as_slice()).bind(ids[0].as_slice()).fetch_one(&h.pool).await?;
            assert!(transferred);
            let mut tx = h.pool.begin().await?;
            let (_permit, scope) = authority(&mut tx, h).await?;
            check_ground_cell_root_admission_in_transaction(
                &mut tx,
                &scope,
                &target(),
                &[prospective(200, &target())?],
            )
            .await
            .map_err(debug)?;
            tx.rollback().await?;
            assert_eq!(h.count("game_item_instances").await?, 64);
            assert_eq!(h.count("game_item_ground_locations").await?, 63);
            // This qualifies reading a genuine committed removal, not a composed
            // removal+insertion writer or an assumed caller-supplied remaining set.
            Ok(())
        })
    }

    #[test]
    fn native_scope_token_cannot_cross_physical_transactions() -> TestResult {
        run("ground_xid", async |h| {
            let mut first = h.pool.begin().await?;
            let (_permit, scope) = authority(&mut first, h).await?;
            let mut second = h.pool.begin().await?;
            assert!(matches!(
                check_ground_cell_root_admission_in_transaction(
                    &mut second,
                    &scope,
                    &target(),
                    &[]
                )
                .await,
                Err(GroundCellAdmissionError::Source(SpellItemError::Rejected(
                    "tile read requires same physical owner transaction"
                )))
            ));
            second.rollback().await?;
            check_ground_cell_root_admission_in_transaction(
                &mut first,
                &scope,
                &target(),
                &[prospective(200, &target())?],
            )
            .await
            .map_err(debug)?;
            first.rollback().await?;
            assert_eq!(h.count("game_item_ground_locations").await?, 0);
            Ok(())
        })
    }

    #[test]
    fn actual_stored_placement_and_prospective_identity_must_match_without_empty_fallback()
    -> TestResult {
        run("ground_binding", async |h| {
            let ids = mint(h, 1, 1, &target()).await?;
            let mut tx = h.pool.begin().await?;
            let (_permit, scope) = authority(&mut tx, h).await?;
            for mutate in [
                |t: &mut SpellGroundTarget| t.map_revision.push('x'),
                |t: &mut SpellGroundTarget| t.content_revision.push('x'),
                |t: &mut SpellGroundTarget| t.placement_context.push(1),
            ] {
                let mut wrong = target();
                mutate(&mut wrong);
                assert!(matches!(
                    check_ground_cell_root_admission_in_transaction(
                        &mut tx,
                        &scope,
                        &wrong,
                        &[prospective(200, &wrong)?]
                    )
                    .await,
                    Err(GroundCellAdmissionError::Source(SpellItemError::Rejected(
                        "unreconciled opaque or incompatible Ground placement"
                    )))
                ));
            }
            let mut duplicate = prospective(200, &target())?;
            duplicate.item_instance_id = ids[0];
            assert!(matches!(
                check_ground_cell_root_admission_in_transaction(
                    &mut tx,
                    &scope,
                    &target(),
                    &[duplicate]
                )
                .await,
                Err(GroundCellAdmissionError::Limit(
                    GroundCellLimitError::IncompatibleInput
                ))
            ));
            let mut wrong = prospective(200, &target())?;
            wrong.world_id = WorldId::decode(&id(99)).map_err(debug)?;
            assert!(matches!(
                check_ground_cell_root_admission_in_transaction(
                    &mut tx,
                    &scope,
                    &target(),
                    &[wrong]
                )
                .await,
                Err(GroundCellAdmissionError::Limit(
                    GroundCellLimitError::IncompatibleInput
                ))
            ));
            check_ground_cell_root_admission_in_transaction(
                &mut tx,
                &scope,
                &target(),
                &[prospective(200, &target())?],
            )
            .await
            .map_err(debug)?;
            tx.rollback().await?;
            Ok(())
        })
    }

    #[test]
    fn real_scope_constructor_rejects_stale_generation_and_other_current_node() -> TestResult {
        run("ground_authority", async |h| {
            let other = harness::register(&h.root, 2).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let recovery = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let permit = SpellLanePermit::of_fresh_scope(harness::scope()?).await?;
            let mut tx = h.pool.begin().await?;
            assert!(matches!(
                assert_spell_item_scope_in_transaction(
                    &mut tx,
                    &permit,
                    &h.root,
                    &recovery,
                    &h.node,
                    harness::scope()?,
                    2
                )
                .await,
                Err(SpellItemError::Rejected(
                    "current timer scope/node authority"
                ))
            ));
            assert!(matches!(
                assert_spell_item_scope_in_transaction(
                    &mut tx,
                    &permit,
                    &h.root,
                    &recovery,
                    &other,
                    harness::scope()?,
                    1
                )
                .await,
                Err(SpellItemError::Rejected(
                    "current timer scope/node authority"
                ))
            ));
            let scope = assert_spell_item_scope_in_transaction(
                &mut tx,
                &permit,
                &h.root,
                &recovery,
                &h.node,
                harness::scope()?,
                1,
            )
            .await
            .map_err(debug)?;
            check_ground_cell_root_admission_in_transaction(&mut tx, &scope, &target(), &[])
                .await
                .map_err(debug)?;
            tx.rollback().await?;
            Ok(())
        })
    }

    #[test]
    fn native_item_read_locks_last_until_rollback_and_do_not_write_source_state() -> TestResult {
        run("ground_lock", async |h| {
            let ids = mint(h, 1, 1, &target()).await?;
            let mut tx = h.pool.begin().await?;
            let (_permit, scope) = authority(&mut tx, h).await?;
            check_ground_cell_root_admission_in_transaction(
                &mut tx,
                &scope,
                &target(),
                &[prospective(200, &target())?],
            )
            .await
            .map_err(debug)?;
            let mut concurrent = h.pool.begin().await?;
            sqlx::query("SET LOCAL lock_timeout='40ms'")
                .execute(&mut *concurrent)
                .await?;
            let error=sqlx::query("SELECT item_instance_id FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE").bind(ids[0].as_slice()).fetch_one(&mut *concurrent).await.err().ok_or("native row lock not held")?;
            assert_eq!(
                error.as_database_error().and_then(|e| e.code()).as_deref(),
                Some("55P03")
            );
            concurrent.rollback().await?;
            tx.rollback().await?;
            let mut after = h.pool.begin().await?;
            sqlx::query("SELECT item_instance_id FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE NOWAIT").bind(ids[0].as_slice()).fetch_one(&mut *after).await?;
            after.rollback().await?;
            assert_eq!(h.count("game_item_instances").await?, 1);
            assert_eq!(h.count("game_item_ground_locations").await?, 1);
            assert_eq!(h.root_revision().await?, "1");
            Ok(())
        })
    }

    #[test]
    fn complete_native_reader_refuses_opaque_sources_and_empty_target_is_not_content_authority()
    -> TestResult {
        run("ground_opaque", async |h| {
            let mut unknown = target();
            unknown.map_revision = "unadmitted-data".into();
            let mut tx = h.pool.begin().await?;
            let (_permit, scope) = authority(&mut tx, h).await?;
            // Empty matching is capacity-only and cannot attest current Content.
            check_ground_cell_root_admission_in_transaction(
                &mut tx,
                &scope,
                &unknown,
                &[prospective(200, &unknown)?],
            )
            .await
            .map_err(debug)?;
            tx.rollback().await?;
            let mut opaque = target();
            opaque.spatial_position = vec![1, 2, 3];
            mint(h, 1, 1, &opaque).await?;
            let mut tx = h.pool.begin().await?;
            let (_permit, scope) = authority(&mut tx, h).await?;
            assert!(matches!(
                check_ground_cell_root_admission_in_transaction(
                    &mut tx,
                    &scope,
                    &target(),
                    &[prospective(200, &target())?]
                )
                .await,
                Err(GroundCellAdmissionError::Source(SpellItemError::Rejected(
                    "unreconciled opaque or incompatible Ground placement"
                )))
            ));
            tx.rollback().await?;
            Ok(())
        })
    }
    #[test]
    fn native_complete_reader_work_bound_never_falls_back_to_empty_occupancy() -> TestResult {
        run("ground_read_bound", async |h| {
            // The existing MINT writer has not integrated the new admission guard;
            // these native fixture MINTs can establish an overfull durable tile.
            mint_quantity(h, 1, 501, &target(), 1).await?;
            let mut tx = h.pool.begin().await?;
            let (_permit, scope) = authority(&mut tx, h).await?;
            assert!(matches!(
                check_ground_cell_root_admission_in_transaction(
                    &mut tx,
                    &scope,
                    &target(),
                    &[prospective(200, &target())?]
                )
                .await,
                Err(GroundCellAdmissionError::Source(SpellItemError::Rejected(
                    "tile item snapshot exceeds 500 visited items"
                )))
            ));
            tx.rollback().await?;
            assert_eq!(h.count("game_item_ground_locations").await?, 501);
            Ok(())
        })
    }
}
