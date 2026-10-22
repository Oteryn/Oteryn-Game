// MAP-PICKUP-1: a base-map pickup is a MINT into Ground followed by the carried-tree loop
// move. Mounted once from map_item_mint_postgres_cases.rs, so both PostgreSQL targets run it.

use super::*;

/// MAP-PICKUP-1: the loop content seam uses the same pinned fixture definitions as MINT.
struct MapPickupContent;

impl crate::durability::item_transfer::ItemWeightLookup for MapPickupContent {
    fn generation_digest(&self) -> [u8; 32] {
        [0xd1; 32]
    }

    fn weight_of(
        &self,
        definition: &TypedDefinitionRef,
    ) -> Result<u32, crate::durability::item_transfer::ItemWeightRefused> {
        crate::durability::item_transfer::ItemWeightLookup::weight_of(&TEST_WEIGHTS, definition)
    }
}

impl crate::durability::item_loop_move::ItemLoopContent for MapPickupContent {
    fn loop_content_digest(&self) -> [u8; 32] {
        [1; 32]
    }

    fn loop_facts(&self, definition: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        let facts = match definition.production_key.as_str() {
            BACKPACK => backpack_facts(),
            COIN => facts(COIN, STACKABLE),
            STONE => facts(STONE, ItemStackClass::NonStackable),
            _ => return None,
        };
        (facts.definition == *definition).then_some(facts)
    }
}

#[test]
fn map_item_mint_loop_pickup_recovers_after_mint_and_replays_once() -> TestResult {
    use crate::durability::item_loop_move::{LoopDestination, LoopMoveReplay, LoopMoveRequest};
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "loop_pickup").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        map_pickup_initialize(&harness, &authority).await?;
        let backpack = harness.equip_backpack(&authority, 1).await?;
        let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
        let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
        let request = pickup(command(20)?, entry(300, 200, 2)?, 0, 5);
        let minted = harness.taken(&authority, fence()?, request.clone()).await?;
        // Crash boundary: the MINT committed; no loop move exists yet. Rebuild from SQL,
        // recreate the candidate and reconcile its original receipt before pickup.
        let ground = harness.root.read_item_loop_ground(world, channel).await?;
        assert_eq!(ground.len(), 1);
        assert_eq!(ground[0].item.item_instance_id, minted.item_instance_id);
        assert!(ground[0].stack_ordinal > 0);
        assert!(harness.ground_of(minted.item_instance_id).await?.is_some());
        assert!(!harness.in_backpack(minted.item_instance_id).await?);
        let mut replay = harness
            .root
            .freeze_map_item_mint(&authority, &harness.node, fence()?, request)
            .await
            .map_err(debug)?;
        assert_eq!(
            harness
                .root
                .reconcile_map_item_mint(&authority, &mut replay)
                .await
                .map_err(debug)?,
            Some(minted.clone())
        );
        let permit = SpellLanePermit::of_fresh_scope(fence()?.runtime_scope).await?;
        let moving = LoopMoveRequest {
            item: minted.item_instance_id,
            count: None,
            pickup_cell: Some(audit::map_item_spatial_position(300, 200, FLOOR)),
            destination: LoopDestination::Container(backpack),
        };
        let receipt = harness
            .root
            .commit_item_loop_move(
                &permit,
                &authority,
                &harness.node,
                &fence()?,
                command(20)?,
                &MapPickupContent,
                moving.clone(),
            )
            .await?
            .map_err(debug)?;
        assert!(!receipt.replayed);
        assert_eq!(receipt.operation, 5);
        assert!(harness.ground_of(minted.item_instance_id).await?.is_none());
        assert!(harness.in_backpack(minted.item_instance_id).await?);
        assert_eq!(
            harness
                .root
                .read_item_loop_move_receipt(&authority, fence()?.character_id, command(20)?)
                .await?,
            LoopMoveReplay::Moved
        );
        let before = harness.footprint().await?;
        assert!(
            harness
                .root
                .commit_item_loop_move(
                    &permit,
                    &authority,
                    &harness.node,
                    &fence()?,
                    command(20)?,
                    &MapPickupContent,
                    moving
                )
                .await?
                .map_err(debug)?
                .replayed
        );
        assert_eq!(harness.footprint().await?, before);
        assert_eq!(harness.count("game_map_item_mint_receipts").await?, 1);
        assert_eq!(harness.count("game_item_loop_move_receipts").await?, 1);
        drop(permit);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn map_pickup_initialize(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    use crate::durability::character_equipment::{
        assert_equipment_authority_in_transaction, initialize_equipment_in_transaction,
    };
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).execute(&harness.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
        .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).bind([1_u8;32].as_slice())
        .bind([2_u8;32].as_slice()).bind([3_u8;32].as_slice()).execute(&harness.pool).await?;
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let equipment = assert_equipment_authority_in_transaction(
        &mut tx,
        &harness.root,
        authority,
        &harness.node,
        &fence()?,
        command(90)?,
        [1; 32],
    )
    .await
    .map_err(debug)?;
    initialize_equipment_in_transaction(&mut tx, &equipment)
        .await
        .map_err(debug)?;
    tx.commit().await?;
    Ok(())
}

#[test]
fn map_item_mint_loop_admission_preserves_every_ground_refusal() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "loop_admission").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness.root.open_character_authority(&seal).await.map_err(debug)?;
        let backpack = harness.equip_backpack(&authority, 1).await?;
        let minted = harness.taken(&authority, fence()?, pickup(command(20)?, entry(300, 200, 2)?, 0, 5)).await?;
        let before = harness.footprint().await?;
        // Each negative mutates one invariant of an otherwise admissible source. Setup-only
        // corruption is transaction-local; triggers are restored before exercising admission,
        // and every probe rolls back. The full API case above covers deferred commit proofs.
        let source = "item loop pickup source is not a loop Ground item of this Channel";
        let current = "item loop line must name the exact current item";
        let temporal = "item loop timed item not hosted";
        let cases = [
            ("valid", "", WORLD, CHANNEL, 3_i16, 5_i64, "1", None),
            ("world", "", SECOND_CHANNEL, CHANNEL, 3, 5, "1", Some(current)),
            ("channel", "", WORLD, SECOND_CHANNEL, 3, 5, "1", Some(source)),
            ("non-Ground", "", WORLD, CHANNEL, 1, 5, "1", Some("item loop source is not in the Character tree")),
            ("stale quantity", "", WORLD, CHANNEL, 3, 4, "1", Some(current)),
            ("stale revision", "", WORLD, CHANNEL, 3, 5, "2", Some(current)),
            ("ordinary Ground", "UPDATE game_item_ground_locations SET corpse_ref = decode('01','hex') WHERE item_instance_id = encode($1,'hex')::uuid", WORLD, CHANNEL, 3, 5, "1", Some(source)),
            ("missing Ground", "DELETE FROM game_item_ground_locations WHERE item_instance_id = encode($1,'hex')::uuid", WORLD, CHANNEL, 3, 5, "1", Some(source)),
            ("marker without receipt", "DELETE FROM game_map_item_mint_receipts WHERE item_instance_id = encode($1,'hex')::uuid", WORLD, CHANNEL, 3, 5, "1", Some(source)),
            ("changed MINT quantity", "UPDATE game_item_instances SET quantity = 4 WHERE item_instance_id = encode($1,'hex')::uuid", WORLD, CHANNEL, 3, 4, "1", Some(source)),
            ("changed MINT revision", "UPDATE game_item_instances SET state_revision = 2 WHERE item_instance_id = encode($1,'hex')::uuid", WORLD, CHANNEL, 3, 5, "2", Some(source)),
            ("Ground channel", "UPDATE game_item_ground_locations SET channel_id = '2c020304-0506-7008-800a-0b0c0d0e0f2c' WHERE item_instance_id = encode($1,'hex')::uuid", WORLD, CHANNEL, 3, 5, "1", Some(source)),
            ("Ground world", "UPDATE game_item_ground_locations SET world_id = '2c020304-0506-7008-800a-0b0c0d0e0f2c' WHERE item_instance_id = encode($1,'hex')::uuid", WORLD, CHANNEL, 3, 5, "1", Some(source)),
            ("Ground scope", "UPDATE game_item_ground_locations SET runtime_scope_ownership_generation = 2 WHERE item_instance_id = encode($1,'hex')::uuid", WORLD, CHANNEL, 3, 5, "1", Some(source)),
            ("Ground position", "UPDATE game_item_ground_locations SET spatial_position = decode('0000012d000000c8fff9','hex') WHERE item_instance_id = encode($1,'hex')::uuid", WORLD, CHANNEL, 3, 5, "1", Some(source)),
            ("Ground content", "UPDATE game_item_ground_locations SET content_revision = 'other-content' WHERE item_instance_id = encode($1,'hex')::uuid", WORLD, CHANNEL, 3, 5, "1", Some(source)),
            ("map revision", "UPDATE game_item_ground_locations SET map_revision = 'other-map' WHERE item_instance_id = encode($1,'hex')::uuid", WORLD, CHANNEL, 3, 5, "1", Some(source)),
            ("placement context", "UPDATE game_item_ground_locations SET native_room_placement_context = decode('01','hex') WHERE item_instance_id = encode($1,'hex')::uuid", WORLD, CHANNEL, 3, 5, "1", Some(source)),
            ("missing ordinal", "UPDATE game_item_ground_locations SET stack_ordinal = NULL WHERE item_instance_id = encode($1,'hex')::uuid", WORLD, CHANNEL, 3, 5, "1", Some(source)),
            ("timed", "INSERT INTO game_item_timed_states(item_instance_id, charges, state_revision) VALUES (encode($1,'hex')::uuid, 1, 1)", WORLD, CHANNEL, 3, 5, "1", Some(temporal)),
            ("temporal item", "INSERT INTO game_spell_item_temporal_schedules(item_instance_id, source_transaction_id, source_ordinal, duration_millis, expires_at_unix_ms, definition_family, definition_production_key, definition_revision, content_digest) SELECT item_instance_id, minted_transaction_id, 1, 1000, 999999, definition_family, definition_production_key, definition_revision_ref, decode(repeat('01',32),'hex') FROM game_item_instances WHERE item_instance_id = encode($1,'hex')::uuid", WORLD, CHANNEL, 3, 5, "1", Some(temporal)),
            ("temporal field", "INSERT INTO game_spell_field_temporal_schedules(item_instance_id, source_stage, blocks_movement, blocks_projectile, immovable_block_solid, source_transaction_id, source_ordinal, duration_millis, expires_at_unix_ms, definition_family, definition_production_key, definition_revision, content_digest) SELECT item_instance_id, 1, false, false, false, minted_transaction_id, 1, 1000, 999999, definition_family, definition_production_key, definition_revision_ref, decode(repeat('01',32),'hex') FROM game_item_instances WHERE item_instance_id = encode($1,'hex')::uuid", WORLD, CHANNEL, 3, 5, "1", Some(temporal)),
        ];
        for (label, setup, world, channel, from, quantity, revision, expected) in cases {
            let mut tx = harness.pool.begin().await?;
            if !setup.is_empty() {
                sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
                sqlx::query(sqlx::AssertSqlSafe(setup.to_owned())).bind(minted.item_instance_id.as_slice()).execute(&mut *tx).await?;
                sqlx::query("SET LOCAL session_replication_role = origin").execute(&mut *tx).await?;
            }
            sqlx::query("INSERT INTO game_item_loop_move_receipts(transaction_id,event_id,character_id,game_session_id,command_id,connection_generation,lease_generation,world_id,channel_id,scope_generation,content_digest,operation,line_count,intent,intent_sha256) VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,99,1,1,encode($5,'hex')::uuid,encode($6,'hex')::uuid,1,$7,5,1,$8,sha256($8))")
                .bind(id(91).as_slice()).bind(id(92).as_slice()).bind(id(CHARACTER).as_slice()).bind(id(SESSION).as_slice()).bind(id(world).as_slice()).bind(id(channel).as_slice()).bind([1_u8;32].as_slice()).bind(b"probe".as_slice()).execute(&mut *tx).await?;
            let result = sqlx::query("INSERT INTO game_item_loop_move_lines(transaction_id,line_ordinal,item_instance_id,definition_family,definition_production_key,definition_revision_ref,from_kind,to_kind,to_parent_item_instance_id,to_capacity,quantity_before,quantity_after,state_revision_before) VALUES (encode($1,'hex')::uuid,1,encode($2,'hex')::uuid,'Item',$3,'definition-r1',$4,1,encode($5,'hex')::uuid,20,$6,5,$7::text::numeric)")
                .bind(id(91).as_slice()).bind(minted.item_instance_id.as_slice()).bind(COIN).bind(from).bind(backpack.as_slice()).bind(quantity).bind(revision).execute(&mut *tx).await;
            match expected {
                None => { assert!(result.is_ok(), "{label}: {result:?}"); }
                Some(expected) => {
                    let error = result.expect_err(label);
                    let database = error.as_database_error().ok_or("not a database refusal")?;
                    assert_eq!(database.code().as_deref(), Some("23514"), "{label}: {error}");
                    assert_eq!(database.message(), expected, "{label}");
                }
            }
            tx.rollback().await?;
            assert_eq!(harness.footprint().await?, before, "{label}");
        }
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn map_item_mint_loop_preflight_and_overlay_acceptance_batch() -> TestResult {
    use crate::durability::item_loop_move::{LoopDestination, LoopMoveRefusal, LoopMoveRequest};
    use production_server::map::overlay::pickup::{
        hide_origin_at_freeze, rehide_taken_origins, within_reach,
    };
    use production_server::map::overlay::{ChannelOverlay, TilePos};
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "loop_acceptance").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        map_pickup_initialize(&harness, &authority).await?;
        harness.equip_backpack(&authority, 1).await?;
        let base = map_overlay_pickup::base()?;
        let key = map_overlay_pickup::coin_key()?;
        let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
        let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
        let pos = TilePos {
            x: 2,
            y: 1,
            floor: FLOOR,
        };
        let mut overlay = ChannelOverlay::new(base.clone(), world, channel);
        let request = MapItemMintRequest {
            base_bundle_digest: base.digest(),
            ..pickup(command(20)?, key, 0, 5)
        };
        let before = harness.footprint().await?;
        assert_eq!(
            harness
                .root
                .preflight_map_item_loop_pickup(
                    &authority,
                    &harness.node,
                    &fence()?,
                    &MapPickupContent,
                    &request,
                    &LoopDestination::MainBackpack
                )
                .await?,
            Ok(())
        );
        assert_eq!(harness.footprint().await?, before);
        hide_origin_at_freeze(&mut overlay, key)?;
        let minted = harness.taken(&authority, fence()?, request).await?;
        // MINT is complete on its own: the origin is hidden and the one item is on Ground.
        assert!(overlay.tile(pos).ok_or("hidden tile")?.is_hidden(1));
        assert!(harness.ground_of(minted.item_instance_id).await?.is_some());
        assert!(!harness.in_backpack(minted.item_instance_id).await?);
        let permit = SpellLanePermit::of_fresh_scope(fence()?.runtime_scope).await?;
        harness
            .root
            .commit_item_loop_move(
                &permit,
                &authority,
                &harness.node,
                &fence()?,
                command(20)?,
                &MapPickupContent,
                LoopMoveRequest {
                    item: minted.item_instance_id,
                    count: None,
                    pickup_cell: Some(audit::map_item_spatial_position(pos.x, pos.y, pos.floor)),
                    destination: LoopDestination::MainBackpack,
                },
            )
            .await?
            .map_err(debug)?;
        drop(permit);
        assert!(harness.ground_of(minted.item_instance_id).await?.is_none());
        assert!(harness.in_backpack(minted.item_instance_id).await?);
        let taken = harness
            .root
            .read_map_item_mint_placements(world, channel, base.digest(), 0, base.entry_count())
            .await
            .map_err(debug)?;
        let mut rebuilt = ChannelOverlay::new(base.clone(), world, channel);
        rehide_taken_origins(&mut rebuilt, &taken).map_err(debug)?;
        assert!(rebuilt.tile(pos).ok_or("rebuilt hide")?.is_hidden(1));
        let before = harness.footprint().await?;
        refused(
            harness
                .take(
                    &authority,
                    fence()?,
                    MapItemMintRequest {
                        base_bundle_digest: base.digest(),
                        ..pickup(command(21)?, key, 0, 5)
                    },
                )
                .await,
            MapItemMintRefusal::AlreadyTaken,
        )?;
        assert_eq!(harness.footprint().await?, before);
        // Fill the remaining nineteen real slots through the same MINT and loop owners.
        for ordinal in 0..19 {
            let request = MapItemMintRequest {
                base_bundle_digest: base.digest(),
                item: facts(STONE, ItemStackClass::NonStackable),
                ..pickup(
                    command(100 + u64::from(ordinal))?,
                    entry(1, 1, ordinal)?,
                    0,
                    1,
                )
            };
            let minted = harness.taken(&authority, fence()?, request.clone()).await?;
            let permit = SpellLanePermit::of_fresh_scope(fence()?.runtime_scope).await?;
            harness
                .root
                .commit_item_loop_move(
                    &permit,
                    &authority,
                    &harness.node,
                    &fence()?,
                    request.command,
                    &MapPickupContent,
                    LoopMoveRequest {
                        item: minted.item_instance_id,
                        count: None,
                        pickup_cell: Some(audit::map_item_spatial_position(1, 1, FLOOR)),
                        destination: LoopDestination::MainBackpack,
                    },
                )
                .await?
                .map_err(debug)?;
        }
        let request = MapItemMintRequest {
            base_bundle_digest: base.digest(),
            ..pickup(command(200)?, entry(1, 1, 20)?, 0, 100)
        };
        let before = harness.footprint().await?;
        assert_eq!(
            harness
                .root
                .preflight_map_item_loop_pickup(
                    &authority,
                    &harness.node,
                    &fence()?,
                    &MapPickupContent,
                    &request,
                    &LoopDestination::MainBackpack
                )
                .await?,
            Err(LoopMoveRefusal::NoRoom)
        );
        assert!(!within_reach(TilePos { x: 4, ..pos }, pos));
        assert_eq!(harness.footprint().await?, before);
        assert!(
            rebuilt
                .tile(TilePos {
                    x: 1,
                    y: 1,
                    floor: FLOOR
                })
                .is_none()
        );
        for (digest, epoch) in [([9; 32], 0), (base.digest(), 1)] {
            assert!(
                harness
                    .root
                    .read_map_item_mint_placements(
                        world,
                        channel,
                        digest,
                        epoch,
                        base.entry_count()
                    )
                    .await
                    .map_err(debug)?
                    .is_empty()
            );
        }
        // A second Channel has its own hide and may take the identical origin once.
        harness.move_to_channel(SECOND_CHANNEL).await?;
        let other_channel = ChannelId::decode(&id(SECOND_CHANNEL)).map_err(debug)?;
        let other_fence = fence_in(SECOND_SESSION, SECOND_CHANNEL)?;
        let mut other_overlay = ChannelOverlay::new(base.clone(), world, other_channel);
        assert!(other_overlay.tile(pos).is_none());
        let other = harness
            .taken(
                &authority,
                other_fence,
                MapItemMintRequest {
                    base_bundle_digest: base.digest(),
                    ..pickup(command_in(SECOND_SESSION, 1)?, key, 0, 5)
                },
            )
            .await?;
        assert_ne!(other.item_instance_id, minted.item_instance_id);
        let taken = harness
            .root
            .read_map_item_mint_placements(
                world,
                other_channel,
                base.digest(),
                0,
                base.entry_count(),
            )
            .await
            .map_err(debug)?;
        assert_eq!(taken, vec![key]);
        rehide_taken_origins(&mut other_overlay, &taken).map_err(debug)?;
        assert!(other_overlay.tile(pos).ok_or("other hide")?.is_hidden(1));
        let before = harness.footprint().await?;
        refused(
            harness
                .take(
                    &authority,
                    other_fence,
                    MapItemMintRequest {
                        base_bundle_digest: base.digest(),
                        ..pickup(command_in(SECOND_SESSION, 2)?, key, 0, 5)
                    },
                )
                .await,
            MapItemMintRefusal::AlreadyTaken,
        )?;
        assert_eq!(harness.footprint().await?, before);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
