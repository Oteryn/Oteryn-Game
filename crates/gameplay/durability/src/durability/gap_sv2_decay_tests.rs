// GAP-SV2/I05: real mint -> stage -> custody moves -> next stage, with replay.
use super::postgres::{
    authority, command, definition, ground_placement, mint_quantity, player_authority, run,
    spell_backpack, spell_definition, spell_mint, spell_origin, spell_request, target,
};
use crate::bestiary_postgres_harness::{self as harness, CHANNEL, Harness, TestResult, WORLD, debug, id};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_weight_lookup::TEST_WEIGHTS;
use crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::WorldId;
use crate::durability::item_loop_move::{
    ItemLoopContent, LoopDestination, LoopGround, LoopMoveRequest,
};
use crate::durability::item_transfer::{
    ItemDefinitionFacts, ItemStackClass, ItemTransferDestination, ItemTransferRequest,
    ItemWeightLookup, ItemWeightRefused,
};
use crate::durability::spell_item_transaction::apply_spell_items_in_transaction;
use crate::durability::spell_items_abi::{
    QualifiedItemDecayStage, SpellFieldOrigin, SpellItemOperation,
};

struct Content;

#[test]
fn gap_sv2_i05_permanent_stack_grants_keep_existing_rules() -> TestResult {
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let (controller, _) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("native Content")?;
    let policy = native
        .item_policy("oteryn:item.tibia.i3031", "definition-r1")
        .ok_or("permanent coin")?;
    let item =
        crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)?;
    assert!(item.decay.is_none());
    assert_eq!(item.stack_maximum, 100);
    let result =
        crate::durability::native_world_items::qualify_source_item_chain(native, item.clone())
            .map_err(debug)?;
    assert_eq!(result, item);
    Ok(())
}

#[test]
fn gap_sv2_i05_activated_ordinary_chain_keeps_permanent_terminal_form() -> TestResult {
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let (controller, _) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("native Content")?;
    let policy = native
        .item_policy("oteryn:item.tibia.i324", "definition-r1")
        .ok_or("ordinary plant")?;
    let item =
        crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)?;
    let chain = crate::durability::native_world_items::qualify_source_item_chain(native, item)
        .map_err(debug)?;
    assert_eq!(chain.decay_chain.len(), 2);
    assert_eq!(
        chain.decay_chain[0].definition.production_key,
        "oteryn:item.tibia.i324"
    );
    assert_eq!(
        chain.decay_chain[1].definition.production_key,
        "oteryn:item.tibia.i330"
    );
    assert_eq!(
        chain.decay_chain[1]
            .target
            .as_ref()
            .ok_or("permanent terminal")?
            .production_key,
        "oteryn:item.tibia.i306"
    );
    Ok(())
}
impl ItemWeightLookup for Content {
    fn generation_digest(&self) -> [u8; 32] {
        [7; 32]
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        self.loop_facts(definition)
            .and_then(|facts| facts.weight)
            .ok_or(ItemWeightRefused::Unknown)
    }
}
impl ItemLoopContent for Content {
    fn loop_content_digest(&self) -> [u8; 32] {
        [7; 32]
    }
    fn loop_facts(&self, item: &TypedDefinitionRef) -> Option<ItemDefinitionFacts> {
        if item.family != "Item" || item.revision_ref != "definition-r1" {
            return None;
        }
        let bag = item.production_key == definition().production_key;
        if !bag
            && !matches!(
                item.production_key.as_str(),
                "oteryn:item.tibia.i2923"
                    | "oteryn:item.tibia.i901001"
                    | "oteryn:item.tibia.i901002"
                    | "oteryn:item.tibia.i901003"
            )
        {
            return None;
        }
        Some(ItemDefinitionFacts {
            definition: item.clone(),
            weight: Some(1),
            stack: ItemStackClass::NonStackable,
            container_capacity: bag.then_some(20),
            container_slot_equip_pattern: bag,
        })
    }
}

#[test]
fn gap_sv2_i05_postgres_chain_custody_handoff_and_replay() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario(ChainCustody::Carried).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "I05 worker panicked")?
        .map_err(Into::into)
}

#[test]
fn srx1_i05_depot_chain_pending_withdrawal_catchup_and_replay_postgres() -> TestResult {
    std::thread::Builder::new().stack_size(64 << 20)
        .spawn(|| scenario(ChainCustody::Depot).map_err(|error|error.to_string()))?
        .join().map_err(|_|"I05 depot worker panicked")?.map_err(Into::into)
}

#[test]
fn xdecay_postgres_ground_rooted_container_advances_without_character_custody() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario(ChainCustody::Ground).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "ground nested decay worker panicked")?
        .map_err(Into::into)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ChainCustody {
    Carried,
    Depot,
    Ground,
}

fn scenario(custody: ChainCustody) -> TestResult {
    run("gap_sv2_i05", async |h| {
        let depot = custody == ChainCustody::Depot;
        let started = std::time::Instant::now();
        let (_runtime, origin) = spell_origin(h)?;
        let bag = mint_quantity(h, 1, 1, &target(), 1).await?[0];
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
                    source_item_instance_id: bag,
                    destination: ItemTransferDestination::ContainerSlot,
                    item: ItemDefinitionFacts {
                        definition: definition(),
                        weight: Some(1),
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
        // Independently activated fixture Content for the actual movement writer.
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).bind([7_u8;32].as_slice())
            .bind([8_u8;32].as_slice()).bind([9_u8;32].as_slice()).execute(&h.pool).await?;
        let reference = |key: &str| TypedDefinitionRef {
            production_key: key.into(),
            ..definition()
        };
        let mut item = spell_definition();
        item.definition = reference("oteryn:item.tibia.i901001");
        item.stack_maximum = 1;
        item.decay_chain = vec![
            QualifiedItemDecayStage {
                definition: item.definition.clone(),
                duration_millis: 1,
                target: Some(reference("oteryn:item.tibia.i901002")),
                blocks_movement: false,
                blocks_projectile: false,
                immovable_block_solid: false,
            },
            QualifiedItemDecayStage {
                definition: reference("oteryn:item.tibia.i901002"),
                duration_millis: 1,
                target: Some(reference("oteryn:item.tibia.i901003")),
                blocks_movement: false,
                blocks_projectile: false,
                immovable_block_solid: false,
            },
        ];
        if custody == ChainCustody::Ground {
            let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
            let (controller, _) =
                oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
                    world, None,
                );
            let native = controller.active().and_then(|active| active.native_gameplay())
                .ok_or("native Content")?;
            item = xdecay_chain(native, 2921, false)?;
            assert_eq!(item.decay_chain.len(), 4);
            item.content_generation_digest = [7; 32];
            for stage in &mut item.decay_chain {
                stage.duration_millis = 1;
            }
        }
        let request = spell_request(
            &origin,
            801,
            166,
            vec![SpellItemOperation::MintInventory {
                item_instance_id: id(180),
                definition: item,
                quantity: 1,
                backpack: spell_backpack(),
                overflow_placement: ground_placement(&target()),
                allow_ground_overflow: false,
            }],
        )?;
        let mut tx = h.pool.begin().await?;
        let (permit, current) = player_authority(&mut tx, h, request.command).await?;
        apply_spell_items_in_transaction(&mut tx, &current, &request, &SpellFieldOrigin::System)
            .await
            .map_err(debug)?;
        tx.commit().await?;
        drop(permit);
        assert_eq!(
            h.count("game_spell_field_temporal_schedules").await?,
            if custody == ChainCustody::Ground { 4 } else { 2 }
        );
        // Wait for the original terminal deadline. Each drain opens one due stage.
        let final_stage = if custody == ChainCustody::Ground { 4_i32 } else { 2_i32 };
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let due: bool = sqlx::query_scalar("SELECT expires_at_unix_ms<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_spell_field_temporal_schedules WHERE source_stage=$1")
                    .bind(final_stage).fetch_one(&h.pool).await?;
                if due { break Ok::<(), sqlx::Error>(()); }
                tokio::task::yield_now().await;
            }
        }).await??;
        assert_eq!(drain(h).await?, 1);
        // A pre-existing carried bag is fixture data; every move of the decaying
        // item and every stage below still uses its real production owner.
        let mut fixture = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *fixture)
            .await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($4,'hex')::uuid)")
            .bind(id(190).as_slice()).bind(id(WORLD).as_slice()).bind(definition().production_key).bind(id(191).as_slice()).execute(&mut *fixture).await?;
        sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,9,encode($5,'hex')::uuid)")
            .bind(id(190).as_slice()).bind(id(WORLD).as_slice()).bind(id(harness::CHARACTER).as_slice())
            .bind(bag.as_slice()).bind(id(192).as_slice()).execute(&mut *fixture).await?;
        fixture.commit().await?;
        let at = target();
        let ground = LoopGround {
            spatial_position: at.spatial_position.clone(),
            map_revision: at.map_revision,
            content_revision: at.content_revision,
            placement_context: at.placement_context,
        };
        let permit = SpellLanePermit::of_fresh_scope(harness::scope()?).await?;
        for (number, pickup_cell, destination) in [
            (802, None, LoopDestination::Ground(ground.clone())),
            (
                803,
                Some(ground.spatial_position.clone()),
                LoopDestination::Container(id(190)),
            ),
        ] {
            let moved = h
                .root
                .commit_item_loop_move(
                    &permit,
                    &recovery,
                    &h.node,
                    &fence,
                    command(number)?,
                    &Content,
                    LoopMoveRequest {
                        item: id(180),
                        count: None,
                        pickup_cell,
                        destination,
                    },
                )
                .await
                .map_err(debug)?;
            assert!(moved.is_ok(), "custody move refused: {moved:?}");
        }
        if custody == ChainCustody::Ground {
            let moved = h
                .root
                .commit_item_loop_move(
                    &permit,
                    &recovery,
                    &h.node,
                    &fence,
                    command(804)?,
                    &Content,
                    LoopMoveRequest {
                        item: id(190),
                        count: None,
                        pickup_cell: None,
                        destination: LoopDestination::Ground(ground),
                    },
                )
                .await
                .map_err(debug)?;
            assert!(moved.is_ok(), "root backpack drop refused: {moved:?}");
        }
        drop(permit);
        if depot {
            let schedules:serde_json::Value=sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(p) ORDER BY source_stage) FROM game_spell_field_temporal_schedules p").fetch_one(&h.pool).await?;
            for (number,deposit) in [(804,true),(805,false)] {
                let permit=SpellLanePermit::of_fresh_scope(harness::scope()?).await?;
                let access=crate::durability::item_loop_move::DepotMove{town:1,box_number:1,deposit,
                    request_digest:[if deposit {4} else {5};32],premium:crate::domain::premium::PremiumGate::CLOSED};
                for _ in 0..2 {
                    let moved=h.root.commit_depot_move(&permit,&recovery,&h.node,&fence,command(number)?,&Content,access,id(180)).await.map_err(debug)?;
                    assert!(moved.is_ok(),"ordinary timed item depot movement/replay: {moved:?}");
                }
                drop(permit);
                if deposit {assert_eq!(drain(h).await?,0,"inactive storage retains its pending source deadline");}
                assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT jsonb_agg(to_jsonb(p) ORDER BY source_stage) FROM game_spell_field_temporal_schedules p").fetch_one(&h.pool).await?,schedules);
            }
        }
        assert_eq!(drain(h).await?, 1);
        let stored: (String, String, i64) = sqlx::query_as("SELECT definition_production_key,state_revision::text,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(180).as_slice()).fetch_one(&h.pool).await?;
        let successor = if custody == ChainCustody::Ground {
            "oteryn:item.tibia.i2925"
        } else {
            "oteryn:item.tibia.i901003"
        };
        assert_eq!(stored, (successor.into(), if depot {"7"} else {"5"}.into(), 1));
        if custody == ChainCustody::Ground {
            let receipt: (i16, bool, bool) = sqlx::query_as(
                "SELECT custody_kind, character_id IS NULL, game_session_id IS NULL
                 FROM game_spell_field_temporal_receipts WHERE source_stage=2",
            ).fetch_one(&h.pool).await?;
            assert_eq!(receipt, (2, true, true));
        }
        assert_eq!(h.count("game_spell_field_temporal_receipts").await?, 2);
        assert_eq!(h.count("game_spell_field_temporal_audit").await?, 2);
        assert_eq!(h.count("game_item_decay_handoffs").await?, if depot {4} else {2});
        if custody == ChainCustody::Ground {
            assert_eq!(drain(h).await?, 1, "the third stage leaves a burnt-down torch");
            assert_eq!(drain(h).await?, 1, "the fourth stage retires the nested torch");
            let retired: (i16, i64, bool) = sqlx::query_as(
                "SELECT i.lifecycle,i.quantity,NOT EXISTS(
                    SELECT 1 FROM game_item_nested_entries n WHERE n.item_instance_id=i.item_instance_id)
                 FROM game_item_instances i WHERE i.item_instance_id=encode($1,'hex')::uuid",
            ).bind(id(180).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(retired, (2, 0, true));
        }
        assert_eq!(drain(h).await?, 0);
        let mut denied = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *denied)
            .await?;
        let forged = sqlx::query("INSERT INTO game_item_decay_handoffs SELECT item_instance_id,result_revision+10,custody_transaction_id,source_stage,stage_transaction_id,created_xact_id FROM game_item_decay_handoffs LIMIT 1")
            .execute(&mut *denied).await.err().ok_or("runtime forged custody handoff")?;
        assert_eq!(
            forged
                .as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some("42501")
        );
        denied.rollback().await?;
        eprintln!(
            "I05 PostgreSQL two stages, Ground/nested custody, depot={depot}, replay PASS in {:?}",
            started.elapsed()
        );
        Ok(())
    })
}

async fn drain(h: &Harness) -> TestResult<usize> {
    let mut tx = h.pool.begin().await?;
    let (permit, current) = authority(&mut tx, h).await?;
    let count = crate::durability::spell_item_temporal::drain_due_field_chains_in_transaction(
        &mut tx, &current, 1,
    )
    .await
    .map_err(debug)?;
    tx.commit().await?;
    drop(permit);
    Ok(count)
}

#[test]
fn gap_sv2_i05_postgres_ground_chain_survives_scope_takeover() -> TestResult {
    run("gap_sv2_takeover", async |h| {
        let started = std::time::Instant::now();
        let (_runtime, origin) = spell_origin(h)?;
        let mut operation = spell_mint(180);
        let SpellItemOperation::MintGround { definition, .. } = &mut operation else {
            return Err("ground fixture".into());
        };
        definition.stack_maximum = 1;
        definition.decay_chain = vec![QualifiedItemDecayStage {
            definition: definition.definition.clone(),
            duration_millis: 1,
            target: Some(TypedDefinitionRef {
                production_key: "oteryn:item.tibia.i901003".into(),
                ..definition.definition.clone()
            }),
            blocks_movement: false,
            blocks_projectile: false,
            immovable_block_solid: false,
        }];
        let request = spell_request(&origin, 801, 166, vec![operation])?;
        let mut tx = h.pool.begin().await?;
        let (permit, current) = player_authority(&mut tx, h, request.command).await?;
        apply_spell_items_in_transaction(&mut tx, &current, &request, &SpellFieldOrigin::System)
            .await.map_err(debug)?;
        tx.commit().await?;
        drop(permit);
        // A takeover assignment is independent fixture setup; production expiry
        // below must freshly attest this exact current generation and node.
        let mut fixture = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *fixture).await?;
        sqlx::query("UPDATE game_runtime_scope_assignments SET ownership_generation=2")
            .execute(&mut *fixture).await?;
        sqlx::query("UPDATE game_runtime_scope_assignment_receipts SET ownership_generation=2")
            .execute(&mut *fixture).await?;
        sqlx::query("UPDATE game_durability_admission_runtime_guards SET ownership_generation=2")
            .execute(&mut *fixture).await?;
        fixture.commit().await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let recovery = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let permit = SpellLanePermit::of_fresh_scope(harness::scope()?).await?;
        let mut tx = h.pool.begin().await?;
        let current = assert_spell_item_scope_in_transaction(
            &mut tx, &permit, &h.root, &recovery, &h.node, harness::scope()?, 2,
        ).await.map_err(debug)?;
        assert_eq!(crate::durability::field_removal::retire_restarted_fields(&mut tx, &current, 1).await.map_err(debug)?, 0);
        assert_eq!(crate::durability::spell_item_temporal::drain_due_field_chains_in_transaction(&mut tx, &current, 1).await.map_err(debug)?, 1);
        tx.commit().await?;
        let stored: (String, i16, i64) = sqlx::query_as("SELECT definition_production_key,lifecycle,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(180).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(stored, ("oteryn:item.tibia.i901003".into(), 1, 1));
        assert_eq!(h.count("game_field_restart_retirements").await?, 0);
        eprintln!("GAP-SV2 I05 PostgreSQL ordinary ground chain takeover PASS in {:?}", started.elapsed());
        Ok(())
    })
}

fn xdecay_chain(
    native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
    id: u32,
    field: bool,
) -> TestResult<crate::durability::spell_items_abi::QualifiedItemDefinition> {
    use crate::durability::native_world_items::{qualify_source_field_chain, qualify_source_item_chain};
    use crate::durability::spell_items_abi::QualifiedItemDefinition;
    let key = format!("oteryn:item.tibia.i{id}");
    let revision = native
        .item_index()
        .and_then(|index| index.revision(&key))
        .ok_or_else(|| format!("missing native decay reference {id}"))?;
    let policy = native
        .item_policy(&key, revision)
        .ok_or_else(|| format!("missing qualified native decay source {id}"))?;
    let item = QualifiedItemDefinition::from_native_policy(policy)?;
    let chain = if field {
        qualify_source_field_chain(native, item)
    } else {
        qualify_source_item_chain(native, item)
    }
    .map_err(debug)?;
    assert!(!chain.decay_chain.is_empty(), "empty source chain for {id}");
    Ok(chain)
}

#[test]
fn xdecay_inventory_qualifies_every_reported_world_chain() -> TestResult {
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let (controller, _) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            world, None,
        );
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("native Content")?;
    let inventory: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tools/content-migration/samples/crystal-item-decay-inventory.json"
    ))?;
    let rows = inventory["records"].as_array().ok_or("Item decay census")?;
    for row in rows {
        let kind = row["kind"].as_str().ok_or("Item decay kind")?;
        if row["after"] != "qualified and runnable"
            || matches!(kind, "corpses" | "rings and amulets with duration" | "other worn items")
        {
            continue;
        }
        let source = u32::try_from(row["id"].as_u64().ok_or("Item decay ID")?)?;
        xdecay_chain(native, source, matches!(kind, "fields" | "magic walls and similar"))?;
    }
    Ok(())
}

#[test]
fn xdecay_postgres_fire_field_advances_a_source_stage() -> TestResult {
    xdecay_postgres_stage(2131, 120_000, Some(2132), true)
}

#[test]
fn xdecay_postgres_lit_torch_advances_a_source_stage() -> TestResult {
    xdecay_postgres_stage(2921, 600_000, Some(2923), false)
}

#[test]
fn xdecay_postgres_ice_cream_retires_at_its_deadline() -> TestResult {
    xdecay_postgres_stage(229, 120_000, None, false)
}

#[test]
fn xdecay_postgres_magic_wall_retires_at_its_deadline() -> TestResult {
    xdecay_postgres_stage(2128, 20_000, None, true)
}

#[test]
fn xdecay_postgres_energy_net_retires_at_its_deadline() -> TestResult {
    xdecay_postgres_stage(9150, 60_000, None, false)
}

fn xdecay_postgres_stage(
    source: u32,
    duration: u32,
    target_id: Option<u32>,
    field: bool,
) -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run("xdecay_source_stage", async |h| {
                let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
                let (controller, _) =
                    oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
                        world, None,
                    );
                let native = controller
                    .active()
                    .and_then(|active| active.native_gameplay())
                    .ok_or("native Content")?;
                let mut definition = xdecay_chain(native, source, field)?;
                let first = definition.decay_chain.first_mut().ok_or("source stage")?;
                let successor = target_id.map(|id| format!("oteryn:item.tibia.i{id}"));
                assert_eq!(first.duration_millis, duration);
                assert_eq!(
                    first.target.as_ref().map(|reference| &reference.production_key),
                    successor.as_ref()
                );
                let blocks_movement = first.blocks_movement;
                let blocks_projectile = first.blocks_projectile;
                // Compress the first interval only after checking the exact source
                // policy. Both scheduling and advancement still use the production DB owner.
                first.duration_millis = 1;
                definition.content_generation_digest = [7; 32];
                let (_runtime, origin) = spell_origin(h)?;
                let request = spell_request(
                    &origin,
                    801,
                    166,
                    vec![SpellItemOperation::MintGround {
                        item_instance_id: id(180),
                        definition,
                        quantity: 1,
                        placement: ground_placement(&target()),
                        lifetime_millis: None,
                        blocks_movement,
                        blocks_projectile,
                        description: None,
                    }],
                )?;
                let mut tx = h.pool.begin().await?;
                let (permit, current) = player_authority(&mut tx, h, request.command).await?;
                apply_spell_items_in_transaction(&mut tx, &current, &request, &SpellFieldOrigin::System)
                    .await.map_err(debug)?;
                tx.commit().await?;
                drop(permit);
                tokio::time::timeout(std::time::Duration::from_secs(5), async {
                    loop {
                        if drain(h).await? == 1 {
                            return Ok::<(), Box<dyn std::error::Error>>(());
                        }
                        tokio::task::yield_now().await;
                    }
                }).await??;
                let stored: (String, i16, i64) = sqlx::query_as(
                    "SELECT definition_production_key,lifecycle,quantity FROM game_item_instances
                     WHERE item_instance_id=encode($1,'hex')::uuid",
                ).bind(id(180).as_slice()).fetch_one(&h.pool).await?;
                match successor {
                    Some(key) => assert_eq!(stored, (key, 1, 1)),
                    None => assert_eq!(stored, (format!("oteryn:item.tibia.i{source}"), 2, 0)),
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "source decay worker panicked")?
        .map_err(Into::into)
}
