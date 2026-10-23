use crate::durability::ground_cell_limit::{
    GroundCellLimitError, check_ground_cell_root_admission,
};
use crate::durability::item_mint::GroundItemInstance;
use crate::durability::item_mint::{GroundPlacement, TypedDefinitionRef};
use crate::durability::spell_items_abi::SpellGroundTarget;
use crate::foundation::{ChannelId, WorldId};

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

#[test]
fn only_the_typed_room_limit_maps_to_native_mint_capacity() {
    use crate::durability::ground_cell_limit::GroundCellAdmissionError;
    use crate::durability::item_mint::ItemMintError;
    use crate::durability::map_item_mint::MapItemMintError;
    use crate::durability::spell_item_transaction::SpellItemError;
    assert!(matches!(
        GroundCellAdmissionError::Limit(GroundCellLimitError::NotEnoughRoom).into_item_mint_error(),
        ItemMintError::CapacityExceeded
    ));
    assert!(matches!(
        GroundCellAdmissionError::Limit(GroundCellLimitError::NotEnoughRoom)
            .into_map_item_mint_error(),
        MapItemMintError::CapacityExceeded
    ));
    for reason in [
        "unreconciled opaque or incompatible Ground placement",
        "tile item snapshot exceeds 500 visited items",
        "tile read requires same physical owner transaction",
        "not enough room",
    ] {
        assert!(matches!(
            GroundCellAdmissionError::Source(SpellItemError::Rejected(reason))
                .into_item_mint_error(),
            ItemMintError::Unavailable(_)
        ));
        assert!(matches!(
            GroundCellAdmissionError::Source(SpellItemError::Rejected(reason))
                .into_map_item_mint_error(),
            MapItemMintError::Unavailable(_)
        ));
    }
}

#[test]
fn current_scope_issuer_failure_stays_authority_and_native_source_error_stays_source() {
    use crate::durability::ground_cell_limit::GroundCellAdmissionError;
    use crate::durability::item_mint::ItemMintError;
    use crate::durability::map_item_mint::MapItemMintError;
    use crate::durability::spell_item_transaction::SpellItemError;
    assert!(matches!(
        GroundCellAdmissionError::Source(SpellItemError::Rejected(
            "current timer scope/node authority"
        ))
        .into_item_mint_error(),
        ItemMintError::AuthorityRejected
    ));
    assert!(matches!(
        GroundCellAdmissionError::Source(SpellItemError::Rejected(
            "current timer scope/node authority"
        ))
        .into_map_item_mint_error(),
        MapItemMintError::AuthorityRejected
    ));
    assert!(matches!(
        GroundCellAdmissionError::Source(SpellItemError::Durability(
            crate::durability::DurabilityError::RootUnavailable
        ))
        .into_spell_error(),
        SpellItemError::Durability(crate::durability::DurabilityError::RootUnavailable)
    ));
}

/// PostgreSQL cases use real native source MINT/TRANSFER writers and current
/// Root/Node/recovery/scope predicates. Prospective insertions remain DATA.
mod postgres {
    use crate::bestiary_postgres_harness as harness;
    use crate::durability::ground_cell_limit::{
        GroundCellAdmissionError, GroundCellLimitError,
        check_ground_cell_root_admission_in_transaction,
    };
    use crate::durability::item_mint::{GroundItemInstance, GroundPlacement};
    use crate::durability::item_mint::{
        ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
    };
    use crate::durability::item_transfer::{
        ItemDefinitionFacts, ItemStackClass, ItemTransferDestination, ItemTransferRequest,
    };
    use crate::durability::item_weight_lookup::TEST_WEIGHTS;
    use crate::durability::spell_item_transaction::SpellItemError;
    use crate::durability::spell_item_transaction::{
        SpellItemScopeAuthority, assert_spell_item_scope_in_transaction,
    };
    use crate::durability::spell_items_abi::SpellGroundTarget;
    use crate::durability::spell_owner_commit::SpellLanePermit;
    use crate::foundation::{ChannelId, WorldId};
    use crate::foundation::{CommandId, CommandRef, GameSessionId, ScopeOwnershipGeneration};
    use harness::{CHANNEL, Harness, TestResult, WORLD, debug, id};
    use sqlx::{Postgres, Transaction};

    fn run<F>(tag: &'static str, body: F) -> TestResult
    where
        F: AsyncFnOnce(&Harness) -> TestResult,
    {
        let Some(admin) = harness::configured_admin() else {
            return Ok(());
        };
        harness::runtime()?.block_on(async move {
            let h = Harness::create(admin, tag, true).await?;
            let version: String = sqlx::query_scalar("SHOW server_version_num")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(version, crate::CANONICAL_POSTGRES_SERVER_VERSION_NUM);
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
        mint_quantity_with_definition(h, first, count, at, quantity, definition()).await
    }

    async fn mint_quantity_with_definition(
        h: &Harness,
        first: u32,
        count: u32,
        at: &SpellGroundTarget,
        quantity: u32,
        item: TypedDefinitionRef,
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
                        item: item.clone(),
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

    async fn frozen_mint(
        h: &Harness,
        actor: u32,
        at: &SpellGroundTarget,
        purpose: &str,
    ) -> TestResult<crate::durability::item_mint::ItemMintCandidate> {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        h.root
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
                        purpose.into(),
                        0,
                    ),
                    item: definition(),
                    quantity: 1,
                    ground: GroundPlacement {
                        spatial_position: at.spatial_position.clone(),
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
            .map_err(|error| debug(error).into())
    }

    async fn mint_footprint(h: &Harness) -> TestResult<[i64; 4]> {
        Ok([
            h.count("game_item_instances").await?,
            h.count("game_item_ground_locations").await?,
            h.count("game_item_mint_receipts").await?,
            h.count("game_item_audit_outbox").await?,
        ])
    }

    fn command(number: u64) -> TestResult<CommandRef> {
        Ok(CommandRef::new(
            GameSessionId::decode(&id(harness::SESSION)).map_err(debug)?,
            CommandId::new(number).map_err(debug)?,
        ))
    }

    async fn player_authority(
        tx: &mut Transaction<'_, Postgres>,
        h: &Harness,
        command: CommandRef,
    ) -> TestResult<(
        SpellLanePermit,
        crate::durability::spell_item_transaction::SpellItemAuthority,
    )> {
        let permit = SpellLanePermit::of_fresh_scope(harness::scope()?).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let authority =
            crate::durability::spell_item_transaction::assert_spell_item_authority_in_transaction(
                tx,
                &permit,
                &h.root,
                &recovery,
                &h.node,
                &harness::item_fence()?,
                command,
                [7; 32],
            )
            .await
            .map_err(debug)?;
        Ok((permit, authority))
    }

    fn ground_placement(at: &SpellGroundTarget) -> GroundPlacement {
        GroundPlacement {
            spatial_position: at.spatial_position.clone(),
            corpse_ref: id(3).to_vec(),
            map_revision: at.map_revision.clone(),
            content_revision: at.content_revision.clone(),
            native_room_placement_context: at.placement_context.clone(),
        }
    }

    fn spell_definition() -> crate::durability::spell_items_abi::QualifiedItemDefinition {
        // Explicit native SQL-writer definition input fixture, not an assertion
        // that a runtime cast or compatible Content producer was admitted.
        crate::durability::spell_items_abi::QualifiedItemDefinition {
            definition: TypedDefinitionRef {
                production_key: "oteryn:item.tibia.i2148".into(),
                ..definition()
            },
            content_generation_digest: [7; 32],
            materializable: true,
            movable: true,
            stack_maximum: 100,
            container_capacity: None,
            inventory_destination: true,
            ground_destination: true,
            decay: None,
            decay_chain: Vec::new(),
        }
    }

    fn spell_origin(
        h: &Harness,
    ) -> TestResult<(
        crate::foundation::ChannelRuntimeV1,
        crate::durability::spell_items_abi::SourceCasterOrigin,
    )> {
        use {
            crate::foundation::ChannelRuntimeV1,
            oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
        };
        let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
        let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
        // The pin is explicit native runtime test DATA, not Content activation
        // acceptance. The exact actor comes only from the real runtime factory;
        // DB current recovery/node/scope/lease authority is independently issued.
        let mut owner = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            h.node.fact().node_id(),
            h.node.fact().registration_revision(),
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            ChannelContentPin::test(world),
        )
        .map_err(debug)?;
        let reservation = owner
            .reserve_fresh_session(GameSessionId::decode(&id(harness::SESSION)).map_err(debug)?)
            .map_err(debug)?;
        let actor = owner.commit_fresh_session(reservation).map_err(debug)?;
        let origin = crate::durability::spell_items_abi::SourceCasterOrigin {
            actor,
            character_lease_generation: harness::item_fence()?.character_lease_generation,
        };
        Ok((owner, origin))
    }

    fn spell_backpack() -> crate::durability::spell_items_abi::QualifiedItemDefinition {
        let mut backpack = spell_definition();
        backpack.definition = definition();
        backpack.stack_maximum = 1;
        backpack.container_capacity = Some(20);
        backpack
    }

    fn spell_mint(number: u8) -> crate::durability::spell_items_abi::SpellItemOperation {
        crate::durability::spell_items_abi::SpellItemOperation::MintGround {
            item_instance_id: id(number),
            definition: spell_definition(),
            quantity: 1,
            placement: ground_placement(&target()),
            lifetime_millis: None,
            blocks_movement: false,
            blocks_projectile: false,
            description: None,
        }
    }

    fn spell_request(
        origin: &crate::durability::spell_items_abi::SourceCasterOrigin,
        number: u64,
        transaction: u8,
        operations: Vec<crate::durability::spell_items_abi::SpellItemOperation>,
    ) -> TestResult<crate::durability::spell_items_abi::SpellItemTransactionRequest> {
        use crate::durability::spell_items_abi::{CastCostBinding, SpellItemTransactionRequest};
        Ok(SpellItemTransactionRequest {
            command: command(number)?,
            spell: TypedDefinitionRef {
                family: "Spell".into(),
                production_key: "fixture:ground.spell".into(),
                revision_ref: "definition-r1".into(),
            },
            catalog_digest: [7; 32],
            transaction_id: id(transaction),
            event_id: id(transaction + 1),
            cost: CastCostBinding {
                vitals_revision_before: 1,
                vitals_revision_after: 2,
                mana_before: 200,
                mana_after: 180,
                soul_before: 100,
                soul_after: 100,
                cooldowns_before: Vec::new(),
                cooldowns_after: vec![("spell:fixture_ground".into(), 100_000)],
                caster_digest_before: [11; 32],
                caster_digest_after: [12; 32],
            },
            caster_origin: Some(origin.clone()),
            operations,
            companion: None,
            direct_companion: None,
        })
    }

    async fn spell_footprint(h: &Harness) -> TestResult<[i64; 7]> {
        Ok([
            h.count("game_item_instances").await?,
            h.count("game_item_ground_locations").await?,
            h.count("game_spell_item_receipts").await?,
            h.count("game_spell_item_lines").await?,
            h.count("game_spell_item_audit_outbox").await?,
            h.count("game_spell_item_source_descriptions").await?,
            h.count("game_spell_field_temporal_schedules").await?,
        ])
    }

    async fn readiness_footprint(h: &Harness) -> TestResult<[i64; 5]> {
        Ok([
            h.count("game_item_instances").await?,
            h.count("game_item_ground_locations").await?,
            h.count("game_item_mint_receipts").await?,
            h.count("game_item_audit_outbox").await?,
            h.count("game_map_item_mint_receipts").await?,
        ])
    }

    #[test]
    fn genuine_not_ready_publication_refuses_fresh_mint_corpse_and_map_without_room_classification()
    -> TestResult {
        run("ground_not_ready", async |h| {
            use crate::durability::admission_authority_guards::{
                AdmissionGuardStore, GuardPublicationDisposition,
            };
            use crate::durability::item_mint::{
                CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintError,
                ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
            };
            use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};
            use crate::durability::map_item_mint::{MapItemMintError, MapItemMintRequest};
            use crate::durability::map_item_mint_audit::{
                map_item_spatial_position, map_revision_of,
            };
            use crate::durability::spell_owner_commit::SpellLanePermit;
            use crate::foundation::admission_authority_publication::{
                AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
                AdmissionPublicationPreconditionV1,
            };
            use crate::foundation::{
                ChannelId, CommandId, CommandRef, GameSessionId, ScopeOwnershipGeneration, WorldId,
            };

            let digest = [0xd1; 32];
            let definition = TypedDefinitionRef {
                family: "Item".into(),
                production_key: "fixture:b3.stone".into(),
                revision_ref: "definition-r1".into(),
            };
            let ground = GroundPlacement {
                spatial_position: map_item_spatial_position(120, 5, -7),
                corpse_ref: id(3).to_vec(),
                map_revision: map_revision_of(&digest),
                content_revision: "content-1".into(),
                native_room_placement_context: digest.to_vec(),
            };
            let request = |actor, purpose: &str| -> TestResult<ItemMintRequest> {
                Ok(ItemMintRequest {
                    cause: ItemMintCause::for_test(
                        WorldId::decode(&id(harness::WORLD)).map_err(debug)?,
                        ChannelId::decode(&id(harness::CHANNEL)).map_err(debug)?,
                        ScopeOwnershipGeneration::new(1).map_err(debug)?,
                        actor,
                        1,
                        TypedDefinitionRef {
                            family: "LootTable".into(),
                            production_key: "fixture:ground.loot".into(),
                            revision_ref: "definition-r1".into(),
                        },
                        purpose.into(),
                        0,
                    ),
                    item: definition.clone(),
                    quantity: 1,
                    ground: ground.clone(),
                    content_revision: "content-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                })
            };
            let seal = h.recovery.seal_current().map_err(debug)?;
            let recovery = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            for actor in 1..=64 {
                let mut candidate = h
                    .root
                    .freeze_item_mint(&recovery, &h.node, request(actor, "fixture:ground.seed")?)
                    .await
                    .map_err(debug)?;
                let permit = candidate.fresh_death_lane_permit().await;
                assert!(matches!(
                    h.root
                        .commit_item_mint(&permit, &recovery, &h.node, &mut candidate)
                        .await
                        .map_err(debug)?,
                    ItemMintOutcome::Committed(_)
                ));
            }
            let mut ordinary = h
                .root
                .freeze_item_mint(&recovery, &h.node, request(100, "fixture:ground.new")?)
                .await
                .map_err(debug)?;
            let mut corpse = h
                .root
                .freeze_item_mint(
                    &recovery,
                    &h.node,
                    request(101, CORPSE_MATERIALIZATION_PURPOSE_KEY)?,
                )
                .await
                .map_err(debug)?;
            let fence = harness::item_fence()?;
            let mut map = h
                .root
                .freeze_map_item_mint(
                    &recovery,
                    &h.node,
                    fence,
                    MapItemMintRequest {
                        command: CommandRef::new(
                            GameSessionId::decode(&id(harness::SESSION)).map_err(debug)?,
                            CommandId::new(900).map_err(debug)?,
                        ),
                        base_bundle_digest: digest,
                        placement_key: oteryn_world_bundle::bundle::placement_key(-7, 120, 5, 1)
                            .ok_or("native placement key")?,
                        reset_epoch: 1,
                        item: ItemDefinitionFacts {
                            weight: None,
                            definition,
                            stack: ItemStackClass::NonStackable,
                            container_capacity: None,
                            container_slot_equip_pattern: false,
                        },
                        quantity: 1,
                        content_revision: "content-1".into(),
                        ruleset_revision: "ruleset-1".into(),
                        sim_revision: "sim-1".into(),
                    },
                )
                .await
                .map_err(debug)?;
            let store = AdmissionGuardStore::from_root(h.root.clone());
            let mut changes = store
                .load(&[AdmissionAuthorityGuardKeyV1::Runtime(harness::scope()?)])
                .await
                .map_err(debug)?;
            let mut change = changes
                .pop()
                .flatten()
                .ok_or("actual current Runtime publication absent")?;
            let AdmissionAuthorityGuardStateV1::Runtime {
                ready,
                ownership_generation,
                ..
            } = &mut change.state
            else {
                return Err("actual runtime guard has wrong type".into());
            };
            assert!(*ready);
            assert_eq!(*ownership_generation, 1);
            *ready = false;
            change.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                expected_publication_revision: change.publication_revision,
            };
            change.publication_revision += 1;
            change.source.source_revision += 1;
            change.source.decision_identity = "ground-native-readiness-false".into();
            let now: i64 = sqlx::query_scalar(
                "SELECT floor(extract(epoch FROM statement_timestamp()))::bigint",
            )
            .fetch_one(&h.pool)
            .await?;
            change.source.source_observed_at = now.max(change.source.source_observed_at);
            let publication = harness::ground_native_readiness_enforcement::prepare(change, now)?;
            assert_eq!(
                h.root
                    .publish_runtime_readiness(&h.node, &publication)
                    .await
                    .map_err(debug)?,
                GuardPublicationDisposition::Applied
            );
            let before = readiness_footprint(h).await?;
            assert_eq!(before[1], 64);
            let permit = SpellLanePermit::of_fresh_scope(harness::scope()?).await?;
            assert!(matches!(
                h.root
                    .commit_item_mint(&permit, &recovery, &h.node, &mut ordinary)
                    .await,
                Err(ItemMintError::AuthorityRejected)
            ));
            assert!(matches!(
                h.root
                    .commit_corpse_mint(
                        &permit,
                        &recovery,
                        &h.node,
                        &mut corpse,
                        id(harness::CHARACTER)
                    )
                    .await,
                Err(ItemMintError::AuthorityRejected)
            ));
            assert!(matches!(
                h.root
                    .commit_map_item_mint(&permit, &recovery, &h.node, fence, &mut map)
                    .await,
                Err(MapItemMintError::AuthorityRejected)
            ));
            assert_eq!(readiness_footprint(h).await?, before);
            Ok(())
        })
    }

    #[test]
    fn spell_batch_capacity_failure_rolls_back_earlier_root_and_all_source_rows() -> TestResult {
        run("ground_spell_batch", async |h| {
            let (_runtime, origin) = spell_origin(h)?;
            use crate::durability::spell_item_transaction::apply_spell_items_in_transaction;
            mint(h, 1, 63, &target()).await?;
            let before = spell_footprint(h).await?;
            let request = spell_request(&origin, 700, 160, vec![spell_mint(180), spell_mint(181)])?;
            let mut tx = h.pool.begin().await?;
            let (_permit, authority) = player_authority(&mut tx, h, request.command).await?;
            let result = apply_spell_items_in_transaction(&mut tx, &authority, &request).await;
            assert!(
                matches!(result, Err(SpellItemError::Rejected("not enough room"))),
                "{result:?}"
            );
            let earlier_root: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(180).as_slice()).fetch_one(&mut *tx).await?;
            assert_eq!(
                earlier_root, 1,
                "first native root must precede second-root refusal"
            );
            tx.rollback().await?;
            assert_eq!(spell_footprint(h).await?, before);
            assert_eq!(h.root_revision().await?, "1");
            Ok(())
        })
    }

    #[test]
    fn spell_actual_removal_frees_room_and_committed_replay_does_not_charge_it_again() -> TestResult
    {
        run("ground_spell_replace", async |h| {
            let (_runtime, origin) = spell_origin(h)?;
            use crate::durability::spell_item_transaction::{
                apply_spell_items_in_transaction, prepare_ground_removal_in_transaction,
            };
            use crate::durability::spell_items_abi::{
                SpellItemOperation, SpellItemTransactionOutcome,
            };
            // RemoveGround validates its actual stored Item identity too. Seed
            // this source through native MINT with the same qualified identity;
            // do not retag a reservation or weaken the native operation check.
            let ids = mint_quantity_with_definition(
                h,
                1,
                64,
                &target(),
                1,
                spell_definition().definition,
            )
            .await?;
            let mut tx = h.pool.begin().await?;
            let (_permit, authority) = player_authority(&mut tx, h, command(701)?).await?;
            let removed =
                prepare_ground_removal_in_transaction(&mut tx, &authority, &target(), ids[0])
                    .await
                    .map_err(debug)?;
            assert_eq!(removed.definition, spell_definition().definition);
            let request = spell_request(
                &origin,
                701,
                162,
                vec![SpellItemOperation::RemoveGround(removed), spell_mint(180)],
            )?;
            let outcome = apply_spell_items_in_transaction(&mut tx, &authority, &request)
                .await
                .map_err(debug)?;
            let SpellItemTransactionOutcome::Applied(receipt) = outcome else {
                return Err("fresh replacement did not apply".into());
            };
            tx.commit().await?;
            let before = spell_footprint(h).await?;
            assert_eq!(before[1], 64);
            let retired: i16 = sqlx::query_scalar("SELECT lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(ids[0].as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(retired, 2);
            let mut replay = h.pool.begin().await?;
            let (_permit, authority) = player_authority(&mut replay, h, request.command).await?;
            assert!(
                matches!(apply_spell_items_in_transaction(&mut replay, &authority, &request).await.map_err(debug)?, SpellItemTransactionOutcome::AlreadyCommitted(value) if value == receipt)
            );
            replay.commit().await?;
            assert_eq!(spell_footprint(h).await?, before);
            Ok(())
        })
    }

    #[test]
    fn actual_inventory_overflow_is_a_ground_root_and_refuses_without_partial_writes() -> TestResult
    {
        run("ground_spell_overflow", async |h| {
            let (_runtime, origin) = spell_origin(h)?;
            use crate::durability::spell_item_transaction::apply_spell_items_in_transaction;
            use crate::durability::spell_items_abi::SpellItemOperation;
            mint(h, 1, 64, &target()).await?;
            let before = spell_footprint(h).await?;
            let operation = SpellItemOperation::MintInventory {
                item_instance_id: id(180),
                definition: spell_definition(),
                quantity: 1,
                backpack: spell_backpack(),
                overflow_placement: ground_placement(&target()),
                allow_ground_overflow: true,
            };
            let request = spell_request(&origin, 702, 164, vec![operation])?;
            let mut tx = h.pool.begin().await?;
            let (_permit, authority) = player_authority(&mut tx, h, request.command).await?;
            let result = apply_spell_items_in_transaction(&mut tx, &authority, &request).await;
            assert!(
                matches!(result, Err(SpellItemError::Rejected("not enough room"))),
                "{result:?}"
            );
            tx.rollback().await?;
            assert_eq!(spell_footprint(h).await?, before);
            Ok(())
        })
    }

    #[test]
    fn actual_inventory_destination_and_existing_stack_merge_do_not_charge_full_ground()
    -> TestResult {
        run("ground_inventory_zero", async |h| {
            let (_runtime, origin) = spell_origin(h)?;
            use crate::durability::spell_item_transaction::{
                apply_spell_items_in_transaction, prepare_inventory_merge_in_transaction,
            };
            use crate::durability::spell_items_abi::SpellItemOperation;
            let backpack_id = mint_quantity(h, 1, 1, &target(), 1).await?[0];
            let seal = h.recovery.seal_current().map_err(debug)?;
            let recovery = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let fence = harness::item_fence()?;
            let mut transfer = h
                .root
                .freeze_item_transfer(
                    &recovery,
                    &h.node,
                    fence,
                    ItemTransferRequest {
                        weight_generation_digest: [0xD1; 32],
                        command: command(800)?,
                        source_item_instance_id: backpack_id,
                        destination: ItemTransferDestination::ContainerSlot,
                        item: ItemDefinitionFacts {
                            weight: Some(1),
                            definition: definition(),
                            stack: ItemStackClass::NonStackable,
                            container_capacity: Some(20),
                            container_slot_equip_pattern: true,
                        },
                        backpack: None,
                        content_revision: "content-1".into(),
                        ruleset_revision: "ruleset-1".into(),
                        sim_revision: "sim-1".into(),
                    },
                    &TEST_WEIGHTS,
                )
                .await
                .map_err(debug)?;
            let permit = SpellLanePermit::of_fresh_scope(harness::scope()?).await?;
            h.root
                .commit_item_transfer(
                    &permit,
                    &recovery,
                    &h.node,
                    fence,
                    &mut transfer,
                    &TEST_WEIGHTS,
                )
                .await
                .map_err(debug)?;
            drop(permit);
            mint(h, 2, 64, &target()).await?;
            let qualified = spell_definition();
            let backpack = spell_backpack();
            let request = spell_request(
                &origin,
                801,
                166,
                vec![SpellItemOperation::MintInventory {
                    item_instance_id: id(180),
                    definition: qualified.clone(),
                    quantity: 1,
                    backpack: backpack.clone(),
                    overflow_placement: ground_placement(&target()),
                    allow_ground_overflow: true,
                }],
            )?;
            let mut tx = h.pool.begin().await?;
            let (_permit, authority) = player_authority(&mut tx, h, request.command).await?;
            apply_spell_items_in_transaction(&mut tx, &authority, &request)
                .await
                .map_err(debug)?;
            tx.commit().await?;
            assert_eq!(h.count("game_item_ground_locations").await?, 64);
            assert_eq!(h.count("game_item_container_entries").await?, 1);
            let mut tx = h.pool.begin().await?;
            let (_permit, authority) = player_authority(&mut tx, h, command(802)?).await?;
            let reservation = prepare_inventory_merge_in_transaction(
                &mut tx,
                &authority,
                &qualified,
                1,
                &backpack,
                &ground_placement(&target()),
            )
            .await
            .map_err(debug)?
            .ok_or("real compatible inventory stack absent")?;
            let request = spell_request(
                &origin,
                802,
                168,
                vec![SpellItemOperation::MergeInventory(reservation)],
            )?;
            apply_spell_items_in_transaction(&mut tx, &authority, &request)
                .await
                .map_err(debug)?;
            tx.commit().await?;
            assert_eq!(h.count("game_item_ground_locations").await?, 64);
            assert_eq!(h.count("game_item_container_entries").await?, 1);
            let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(180).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(quantity, 2);
            Ok(())
        })
    }

    #[test]
    fn map_mint_writer_counts_actual_roots_and_replays_before_full_tile_admission() -> TestResult {
        run("ground_map_mint", async |h| {
            use crate::durability::map_item_mint::{
                MapItemMintError, MapItemMintOutcome, MapItemMintRequest,
            };
            use crate::durability::map_item_mint_audit::{
                map_item_spatial_position, map_revision_of,
            };
            let digest = [0xd1; 32];
            let at = SpellGroundTarget {
                spatial_position: map_item_spatial_position(120, 5, -7),
                map_revision: map_revision_of(&digest),
                content_revision: "content-1".into(),
                placement_context: digest.to_vec(),
            };
            mint(h, 1, 63, &at).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let recovery = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let fence = harness::item_fence()?;
            let request = |number, ordinal| -> TestResult<MapItemMintRequest> {
                Ok(MapItemMintRequest {
                    command: command(number)?,
                    base_bundle_digest: digest,
                    placement_key: oteryn_world_bundle::bundle::placement_key(-7, 120, 5, ordinal)
                        .ok_or("native placement key")?,
                    reset_epoch: 1,
                    item: ItemDefinitionFacts {
                        weight: None,
                        definition: definition(),
                        stack: ItemStackClass::NonStackable,
                        container_capacity: None,
                        container_slot_equip_pattern: false,
                    },
                    quantity: 1,
                    content_revision: "content-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                })
            };
            let mut first = h
                .root
                .freeze_map_item_mint(&recovery, &h.node, fence, request(400, 1)?)
                .await
                .map_err(debug)?;
            let mut second = h
                .root
                .freeze_map_item_mint(&recovery, &h.node, fence, request(401, 2)?)
                .await
                .map_err(debug)?;
            let permit = SpellLanePermit::of_fresh_scope(harness::scope()?).await?;
            let committed = h
                .root
                .commit_map_item_mint(&permit, &recovery, &h.node, fence, &mut first)
                .await
                .map_err(debug)?;
            let MapItemMintOutcome::Committed(committed) = committed else {
                return Err("fresh map MINT did not commit".into());
            };
            let before = mint_footprint(h).await?;
            let receipts = h.count("game_map_item_mint_receipts").await?;
            assert_eq!(before[1], 64);
            assert!(matches!(
                h.root
                    .commit_map_item_mint(&permit, &recovery, &h.node, fence, &mut second)
                    .await,
                Err(MapItemMintError::CapacityExceeded)
            ));
            assert_eq!(mint_footprint(h).await?, before);
            assert_eq!(h.count("game_map_item_mint_receipts").await?, receipts);
            assert!(
                matches!(h.root.commit_map_item_mint(&permit, &recovery, &h.node, fence, &mut first).await.map_err(debug)?, MapItemMintOutcome::AlreadyCommitted(value) if value == committed)
            );
            assert_eq!(mint_footprint(h).await?, before);
            Ok(())
        })
    }

    // Tokio's configured native runtime has no macros feature. Poll all three
    // real futures on the current task, retaining each result until all finish.
    async fn join_three<A: std::future::Future, B: std::future::Future, C: std::future::Future>(
        a: A,
        b: B,
        c: C,
    ) -> (A::Output, B::Output, C::Output) {
        use std::task::Poll;
        let (mut a, mut b, mut c) = (std::pin::pin!(a), std::pin::pin!(b), std::pin::pin!(c));
        let (mut left, mut right, mut observer) = (None, None, None);
        std::future::poll_fn(|context| {
            if left.is_none()
                && let Poll::Ready(output) = a.as_mut().poll(context)
            {
                left = Some(output);
            }
            if right.is_none()
                && let Poll::Ready(output) = b.as_mut().poll(context)
            {
                right = Some(output);
            }
            if observer.is_none()
                && let Poll::Ready(output) = c.as_mut().poll(context)
            {
                observer = Some(output);
            }
            match (left.take(), right.take(), observer.take()) {
                (Some(l), Some(r), Some(o)) => Poll::Ready((l, r, o)),
                (l, r, o) => {
                    (left, right, observer) = (l, r, o);
                    Poll::Pending
                }
            }
        })
        .await
    }

    #[test]
    fn concurrent_native_mints_serialize_the_last_physical_root() -> TestResult {
        run("ground_race", async |h| {
            use crate::durability::item_mint::ItemMintError;
            mint(h, 1, 63, &target()).await?;
            let mut first = frozen_mint(h, 100, &target(), "fixture:ground.drop").await?;
            let mut second = frozen_mint(h, 101, &target(), "fixture:ground.drop").await?;
            // Each genuine Root retains its unchanged one-connection pool. Two
            // independent Roots are required for overlapping physical passes.
            let left_root = crate::durability::DurabilityRoot::connect_test_runtime(&format!(
                "{}?application_name=ground_race_left",
                h.database.url
            ))?;
            let right_root = crate::durability::DurabilityRoot::connect_test_runtime(&format!(
                "{}?application_name=ground_race_right",
                h.database.url
            ))?;
            assert!(left_root.maintain_ready_once().await?);
            assert!(right_root.maintain_ready_once().await?);
            let seal = h.recovery.seal_current().map_err(debug)?;
            let left_authority = left_root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let right_authority = right_root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let one = first.fresh_death_lane_permit().await;
            let two = second.fresh_death_lane_permit().await;
            // An observer holds ONLY the real native Channel key33. It writes no
            // custody or authority. Both independent native passes must overlap
            // and wait before it releases the lock; timing alone is no witness.
            let mut blocker = h.pool.begin().await?;
            let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                .fetch_one(&mut *blocker)
                .await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'), 33))")
                .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice())
                .execute(&mut *blocker).await?;
            let (left, right, overlap) = join_three(
                left_root.commit_item_mint(&one, &left_authority, &h.node, &mut first),
                right_root.commit_item_mint(&two, &right_authority, &h.node, &mut second),
                async {
                    tokio::time::timeout(std::time::Duration::from_millis(1_500), async {
                        loop {
                            let observed: bool = sqlx::query_scalar("SELECT count(*)=2 AND coalesce(bool_or($1=ANY(pg_blocking_pids(pid))),false) FROM pg_stat_activity WHERE datname=current_database() AND application_name IN ('ground_race_left','ground_race_right') AND wait_event_type='Lock' AND cardinality(pg_blocking_pids(pid))>0")
                                .bind(blocker_pid).fetch_one(&h.pool).await?;
                            if observed { break; }
                            tokio::task::yield_now().await;
                        }
                        Ok::<(), sqlx::Error>(())
                    }).await.map_err(|_| "native passes did not overlap at the held Channel key33")??;
                    blocker.commit().await?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                }
            ).await;
            overlap?;
            assert!(matches!(
                (&left, &right),
                (
                    Ok(ItemMintOutcome::Committed(_)),
                    Err(ItemMintError::CapacityExceeded)
                ) | (
                    Err(ItemMintError::CapacityExceeded),
                    Ok(ItemMintOutcome::Committed(_))
                )
            ));
            assert_eq!(h.count("game_item_ground_locations").await?, 64);
            assert_eq!(h.count("game_item_mint_receipts").await?, 64);
            assert_eq!(h.count("game_item_audit_outbox").await?, 64);
            // Reload from a different connection, not an in-memory capacity receipt.
            let mut reloaded = h.pool.begin().await?;
            let (_permit, scope) = authority(&mut reloaded, h).await?;
            let rows = check_ground_cell_root_admission_in_transaction(
                &mut reloaded,
                &scope,
                &target(),
                &[],
            )
            .await
            .map_err(debug)?;
            assert_eq!(rows.len(), 64);
            reloaded.rollback().await?;
            Ok(())
        })
    }

    #[test]
    fn ordinary_and_corpse_mint_boundaries_enforce_capacity_and_replay_full_tile() -> TestResult {
        run("ground_mint_writers", async |h| {
            use crate::durability::item_mint::{CORPSE_MATERIALIZATION_PURPOSE_KEY, ItemMintError};
            mint(h, 1, 63, &target()).await?;
            let mut corpse =
                frozen_mint(h, 100, &target(), CORPSE_MATERIALIZATION_PURPOSE_KEY).await?;
            let mut ordinary = frozen_mint(h, 101, &target(), "fixture:ground.drop").await?;
            let mut another_corpse =
                frozen_mint(h, 102, &target(), CORPSE_MATERIALIZATION_PURPOSE_KEY).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let recovery = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let permit = corpse.fresh_death_lane_permit().await;
            let committed = h
                .root
                .commit_corpse_mint(
                    &permit,
                    &recovery,
                    &h.node,
                    &mut corpse,
                    id(harness::CHARACTER),
                )
                .await
                .map_err(debug)?;
            let ItemMintOutcome::Committed(committed) = committed else {
                return Err("fresh corpse did not commit".into());
            };
            let before = mint_footprint(h).await?;
            assert_eq!(before[1], 64);
            assert!(matches!(
                h.root
                    .commit_item_mint(&permit, &recovery, &h.node, &mut ordinary)
                    .await,
                Err(ItemMintError::CapacityExceeded)
            ));
            assert!(matches!(
                h.root
                    .commit_corpse_mint(
                        &permit,
                        &recovery,
                        &h.node,
                        &mut another_corpse,
                        id(harness::CHARACTER)
                    )
                    .await,
                Err(ItemMintError::CapacityExceeded)
            ));
            assert_eq!(mint_footprint(h).await?, before);
            assert!(matches!(
                h.root.commit_corpse_mint(&permit, &recovery, &h.node, &mut corpse, id(harness::CHARACTER)).await.map_err(debug)?,
                ItemMintOutcome::AlreadyCommitted(value) if value == committed
            ));
            assert_eq!(mint_footprint(h).await?, before);
            Ok(())
        })
    }

    #[test]
    fn native_locked_census_enforces_64_and_preserves_zero_admission_truth() -> TestResult {
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
            assert!(mint(h, 65, 1, &target()).await.is_err());
            let mut tx = h.pool.begin().await?;
            let (_permit, scope) = authority(&mut tx, h).await?;
            // The real writer now prevents creating the65th root. A zero-root
            // inspection still changes no existing custody or quantity.
            check_ground_cell_root_admission_in_transaction(&mut tx, &scope, &target(), &[])
                .await
                .map_err(debug)?;
            tx.commit().await?;
            assert_eq!(h.count("game_item_ground_locations").await?, 64);
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
                weight_generation_digest: [0xD1; 32],
                command: CommandRef::new(
                    GameSessionId::decode(&id(harness::SESSION)).map_err(debug)?,
                    CommandId::new(1).map_err(debug)?,
                ),
                source_item_instance_id: ids[0],
                destination: ItemTransferDestination::ContainerSlot,
                item: ItemDefinitionFacts {
                    weight: Some(1),
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
                .freeze_item_transfer(&recovery, &h.node, fence, request, &TEST_WEIGHTS)
                .await
                .map_err(debug)?;
            let permit = SpellLanePermit::of_fresh_scope(harness::scope()?).await?;
            h.root
                .commit_item_transfer(
                    &permit,
                    &recovery,
                    &h.node,
                    fence,
                    &mut candidate,
                    &TEST_WEIGHTS,
                )
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
    fn native_writer_refuses_opaque_targets_and_empty_target_is_not_content_authority() -> TestResult
    {
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
            let before = mint_footprint(h).await?;
            assert!(mint(h, 1, 1, &opaque).await.is_err());
            assert_eq!(mint_footprint(h).await?, before);
            let mut tx = h.pool.begin().await?;
            let (_permit, scope) = authority(&mut tx, h).await?;
            assert!(matches!(
                check_ground_cell_root_admission_in_transaction(
                    &mut tx,
                    &scope,
                    &opaque,
                    &[prospective(200, &opaque)?]
                )
                .await,
                Err(GroundCellAdmissionError::Limit(
                    GroundCellLimitError::IncompatibleInput
                ))
            ));
            tx.rollback().await?;
            Ok(())
        })
    }
    #[test]
    fn native_writer_refuses_65th_without_creating_unread_occupancy() -> TestResult {
        run("ground_read_bound", async |h| {
            // Previously this fixture created501 roots through unguarded MINT.
            // Enforced native writers must not bypass admission to seed that state.
            mint_quantity(h, 1, 64, &target(), 1).await?;
            let before = mint_footprint(h).await?;
            assert!(mint_quantity(h, 65, 1, &target(), 1).await.is_err());
            assert_eq!(mint_footprint(h).await?, before);
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
            assert_eq!(h.count("game_item_ground_locations").await?, 64);
            Ok(())
        })
    }
}
