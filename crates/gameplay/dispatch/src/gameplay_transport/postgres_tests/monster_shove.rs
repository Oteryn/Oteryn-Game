use super::fixture::{Harness, debug, id};
use super::loop_monster_fixtures::fixture::{cell, dependencies, next_pass, run_case};
use super::{TestResult, prepare_current_session};
use crate::durability::item_loop_move::monster_push::{MonsterPushOccurrence, push_ground_blocker};
use crate::durability::spell_item_transaction::{
    assert_spell_item_scope_in_transaction, read_creature_scope_tile_in_transaction,
};
use crate::durability::spell_items_abi::{SpellGroundTarget, decode_ground_cell};
use crate::foundation::MovementLocalPosition;

#[test]
fn monster_shove_commits_ground_receipt_and_steps_through_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_case(async |harness| {
        prepare_current_session(harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_monster_shove(world);
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("active native content")?;
        let (deps, monster) = dependencies(active, &pin)?;
        let target = SpellGroundTarget::for_native_tile_read(
            &deps.room,
            &*deps.runtime.lock().await,
            cell(3),
        )
        .map_err(debug)?;
        let key = "oteryn:item.tibia.i2904";
        let revision = native
            .item_index()
            .and_then(|index| index.revision(key))
            .ok_or("amphora revision")?;
        let item = id(180);
        let digest = native.source_digest();
        let (ordinal, mint) =
            seed_blocker(harness, &target, (&item, key, revision), digest).await?;

        let mut owner = deps.owner(harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = deps.room.movement_cells().into();
        owner.clock_origin -= std::time::Duration::from_secs(10);
        let permit = owner.spell_lane_permit().await.ok_or("Item lane")?;
        let binding = deps.runtime.lock().await.binding();
        let scope = binding.runtime_scope();
        let generation = binding.scope_generation().get();
        let mut tx = harness.pool.begin().await?;
        let scope_authority = assert_spell_item_scope_in_transaction(
            &mut tx,
            &permit,
            owner.root,
            owner.character,
            owner.holder,
            scope,
            generation,
        )
        .await
        .map_err(debug)?;
        let mut source =
            read_creature_scope_tile_in_transaction(&mut tx, &scope_authority, &target)
                .await
                .map_err(debug)?
                .items;
        let original = source
            .pop()
            .ok_or("blocking item in the original collision read")?;
        assert!(source.is_empty() && original.blocks_movement && original.item_instance_id == item);
        tx.rollback().await?;
        drop(permit);

        owner.drain_monster_melee().await;
        next_pass(&mut owner).await?;
        let position: Vec<u8> = sqlx::query_scalar(
            "SELECT spatial_position FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid",
        ).bind(item.as_slice()).fetch_one(&harness.pool).await?;
        assert_ne!(
            position, target.spatial_position,
            "the movement deadline must displace the blocking item"
        );

        let receipt: (i64, i64, bool, bool, String) = sqlx::query_as(
            "SELECT r.source_ordinal,g.stack_ordinal,r.ground_after=to_jsonb(g),
                    i.quantity=1 AND i.state_revision=1 AND i.minted_transaction_id=encode($2,'hex')::uuid, r.step_due::text
             FROM game_monster_item_push_receipts r
             JOIN game_item_ground_locations g USING(item_instance_id)
             JOIN game_item_instances i USING(item_instance_id)
             WHERE i.item_instance_id=encode($1,'hex')::uuid",
        ).bind(item.as_slice()).bind(mint.as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(receipt.0, ordinal);
        assert!(receipt.1 > ordinal && receipt.2 && receipt.3);
        // A fresh map/owner pass must see the new custody instead of the cached blocker.
        next_pass(&mut owner).await?;
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(monster)
                .map_err(debug)?
                .position(),
            cell(3)
        );
        let landing = decode_ground_cell(&position).map_err(debug)?;
        let runtime = deps.runtime.lock().await;
        let landing_target = SpellGroundTarget::for_native_tile_read(
            &deps.room,
            &runtime,
            MovementLocalPosition {
                x: landing.x,
                y: landing.y,
                floor: i16::try_from(landing.z)?,
            },
        )
        .map_err(debug)?;
        drop(runtime);
        let permit = owner.spell_lane_permit().await.ok_or("Item lane")?;
        let mut replay_tx = harness.pool.begin().await?;
        let scope_authority = assert_spell_item_scope_in_transaction(
            &mut replay_tx,
            &permit,
            owner.root,
            owner.character,
            owner.holder,
            scope,
            generation,
        )
        .await
        .map_err(debug)?;
        let moved = read_creature_scope_tile_in_transaction(
            &mut replay_tx,
            &scope_authority,
            &landing_target,
        )
        .await
        .map_err(debug)?
        .items;
        assert_eq!(moved.len(), 1);
        assert_eq!(moved[0].item_instance_id, item);
        assert!(moved[0].blocks_movement);
        assert_eq!(moved[0].top_down_ordinal, u64::try_from(receipt.1)?);
        let mut occurrence = MonsterPushOccurrence {
            actor: monster,
            due: receipt.4.parse()?,
            node: owner.holder,
            reset_epoch: owner.reset_epoch,
        };
        push_ground_blocker(
            &mut replay_tx,
            &scope_authority,
            &occurrence,
            &target,
            &landing_target,
            &original,
        )
        .await
        .map_err(debug)?;
        occurrence.due += 1;
        assert!(
            push_ground_blocker(
                &mut replay_tx,
                &scope_authority,
                &occurrence,
                &target,
                &landing_target,
                &original
            )
            .await
            .is_err(),
            "a different step must not reuse the shove receipt"
        );
        replay_tx.commit().await?;
        drop(permit);
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_monster_item_push_receipts WHERE source_ordinal=$1",
        )
        .bind(ordinal)
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(count, 1);
        let projected = harness
            .root
            .read_item_loop_ground(world, deps.admitted.channel_id)
            .await
            .map_err(debug)?;
        assert!(
            projected
                .iter()
                .any(|entry| entry.item.item_instance_id == item
                    && entry.item.ground.spatial_position == position
                    && entry.source_native_ground)
        );
        eprintln!("POSTGRES_CASE_COMPLETED monster_shove_commits_ground_receipt_and_steps_through");
        Ok(())
    })
}

#[test]
fn monster_shove_displaces_pushable_creature_and_enters_its_cell_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_case(async |harness| {
        prepare_current_session(harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_monster_shove(world);
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native content")?;
        let (deps, monster) = dependencies(active, &pin)?;
        let mut owner = deps.owner(harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = deps.room.movement_cells().into();
        owner.clock_origin -= std::time::Duration::from_secs(10);
        let blocker = {
            let mut runtime = deps.runtime.lock().await;
            let policy = runtime
                .companion_snapshot(monster)
                .map_err(debug)?
                .state
                .policy;
            let blocker = runtime
                .admit_encounter_actor(&policy.definition_key, policy.maximum_health, cell(3))
                .map_err(debug)?;
            runtime
                .install_creature_policy(blocker, &policy.definition_key)
                .map_err(debug)?;
            let reference = crate::content::ProjectV2DefinitionRef {
                family: crate::content::ProjectV2Family::Creature,
                key: policy.definition_key.clone(),
                revision: policy.definition_revision.clone(),
            };
            owner
                .states
                .spell_states
                .lock()
                .await
                .monster_melee
                .creature_ai
                .admit_native_creature(
                    &runtime,
                    native,
                    blocker,
                    &reference,
                    owner.owner_now().get(),
                )
                .map_err(debug)?;
            blocker
        };
        owner.drain_monster_melee().await;
        next_pass(&mut owner).await?;
        let runtime = deps.runtime.lock().await;
        assert_ne!(
            runtime
                .read_actor_position(blocker)
                .map_err(debug)?
                .position(),
            cell(3)
        );
        assert_eq!(
            runtime
                .read_actor_position(monster)
                .map_err(debug)?
                .position(),
            cell(3)
        );
        eprintln!(
            "POSTGRES_CASE_COMPLETED monster_shove_displaces_pushable_creature_and_enters_its_cell"
        );
        Ok(())
    })
}

async fn seed_blocker(
    harness: &Harness,
    target: &SpellGroundTarget,
    (item, key, revision): (&[u8; 16], &str, &str),
    digest: [u8; 32],
) -> TestResult<(i64, [u8; 16])> {
    // Arrange a qualified, live ordinary Ground blocker. Restore trigger enforcement
    // before calling the production movement and custody owners.
    let mint = id(181);
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$4,$5,1,1,encode($3,'hex')::uuid)")
        .bind(item.as_slice()).bind(id(42).as_slice()).bind(mint.as_slice()).bind(key).bind(revision).execute(&mut *tx).await?;
    let ordinal = sqlx::query_scalar("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,$7,$8,nextval('game_item_ground_stack_ordinal_seq')) RETURNING stack_ordinal")
        .bind(item.as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(&target.spatial_position).bind(id(182).as_slice()).bind(&target.map_revision).bind(&target.content_revision).bind(&target.placement_context).fetch_one(&mut *tx).await?;
    let cost = serde_json::to_vec(&serde_json::json!({
        "version": 1, "before_revision": 0, "after_revision": 1,
        "caster_before": vec![1; 32], "caster_after": vec![2; 32],
        "mana_before": 100, "mana_after": 50, "soul_before": 5, "soul_after": 4,
        "cooldowns_before": [], "cooldowns_after": [["spell:fixture", 1_000_000]],
    }))?;
    sqlx::query("INSERT INTO game_spell_item_receipts(transaction_id,event_id,game_session_id,command_id,character_id,world_id,channel_id,ownership_generation,spell_family,spell_production_key,spell_revision,catalog_digest,binding,intent,cost,cost_binding,operation_count,occurred_at_unix_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,999,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,1,'Spell','fixture:monster_shove_blocker','r1',$7,sha256($8),$8,$9,sha256($9),1,1)")
        .bind(mint.as_slice()).bind(id(183).as_slice()).bind(id(50).as_slice()).bind(id(41).as_slice())
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(digest.as_slice()).bind(b"monster-shove-blocker".as_slice()).bind(cost)
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_spell_item_lines(transaction_id,ordinal,operation_kind,item_instance_id,definition_family,definition_production_key,definition_revision,quantity_before,quantity_after,state_revision_before,world_id,channel_id,spatial_position,map_revision,content_revision,placement_context,content_generation_digest,blocks_movement,blocks_projectile,immovable_block_solid) SELECT i.minted_transaction_id,1,1,i.item_instance_id,i.definition_family,i.definition_production_key,i.definition_revision_ref,0,1,0,g.world_id,g.channel_id,g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context,$2,true,false,false FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).bind(digest.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok((ordinal, mint))
}
