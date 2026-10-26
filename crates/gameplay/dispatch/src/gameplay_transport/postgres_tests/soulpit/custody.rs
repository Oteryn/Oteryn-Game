//! USE bytes and observed handles exercise each supported atomic input custody.
use super::*;

pub(super) async fn run(
    harness: &Harness,
    deps: &Dependencies,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult {
    super::super::item_look::seed_main_backpack(harness, &[]).await?;
    let mut admitted = deps.admitted;
    admitted.runtime_actor = Some(deps.actor);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = Some(fence);
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[4, 6, 11, 12, 14, 15, 18, 114],
    )
    .ok_or("custody capabilities")?;
    let index = native.item_index().ok_or("index")?;
    let orc = index
        .definition_ref("Item", "oteryn:item.tibia.i47384", "definition-r1")
        .ok_or("orc")?;
    let rat = index
        .definition_ref("Item", "oteryn:item.tibia.i47398", "definition-r1")
        .ok_or("rat")?;
    let revision = {
        let mut runtime = owner.runtime.lock().await;
        let at = crate::gameplay_transport::bundle_map_position(
            &mut runtime,
            owner.movement_cells,
            deps.actor,
        )
        .ok_or("map position")?;
        crate::map::overlay::map_revision(at.map.overlay.base())
    };
    for (left, right, units) in [
        ("nested", "main", 1),
        ("nested", "nested", 1),
        ("equipment", "main", 1),
        ("main", "equipment", 1),
        ("ground", "main", 1),
        ("main", "ground", 1),
        ("ground", "ground", 1),
        ("nested", "same", 2),
        ("equipment", "same", 2),
        ("ground", "same", 2),
    ] {
        seed(harness, left, right, units, &revision).await?;
        owner.states.loop_ground.invalidate();
        let (next, result) = exchange_pair(
            owner,
            admitted,
            orc,
            if right == "same" { orc } else { rat },
        )
        .await
        .map_err(|error| format!("custody {left}+{right}: {error}"))?;
        assert_eq!(result, UseDisposition::Committed, "{left} + {right}");
        admitted = next;
        let command = admitted.continuity.next_command_id - 1;
        let (source, secondary, outputs, intent): (serde_json::Value, Option<serde_json::Value>, serde_json::Value, Vec<u8>) = sqlx::query_as("SELECT source_custody,secondary_custody,outputs,intent FROM game_item_craft_records WHERE command_id=$1::text::numeric")
            .bind(command.to_string()).fetch_one(&harness.pool).await?;
        assert_eq!(source["kind"], left);
        assert_eq!(
            secondary.as_ref().map(|c| c["kind"].as_str()),
            (right != "same").then_some(Some(right))
        );
        let retired: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE item_instance_id IN (encode($1,'hex')::uuid,encode($2,'hex')::uuid) AND lifecycle=2 AND quantity=0")
            .bind(id(71).as_slice()).bind(id(if right == "same" { 71 } else { 72 }).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(retired, if right == "same" { 1 } else { 2 });
        let bag = harness
            .root
            .read_character_backpack(authority, fence.character_id)
            .await?
            .ok_or("bag")?;
        assert_eq!(bag.entries.len(), 3, "two empty bags and one output");
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_audit_outbox WHERE transaction_id=(SELECT transaction_id FROM game_item_craft_records WHERE command_id=$1::text::numeric)")
            .bind(command.to_string()).fetch_one(&harness.pool).await?;
        assert_eq!(count, 1);
        let mut restarted = deps.owner(harness, authority);
        restarted.active_generation = owner.active_generation;
        restarted.movement_cells = owner.movement_cells;
        deps.initialize(&restarted).await;
        let payload = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
            admitted.continuity.selected_capabilities.as_slice(),
            &intent,
        )
        .map_err(debug)?;
        let replay = restarted
            .use_craft(
                deps.actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command,
                    item_fence: Some(fence),
                },
                payload,
                None,
                None,
            )
            .await
            .ok_or("replay")?;
        assert_eq!(replay.disposition, UseDisposition::Committed);
        let retained: serde_json::Value = sqlx::query_scalar(
            "SELECT outputs FROM game_item_craft_records WHERE command_id=$1::text::numeric",
        )
        .bind(command.to_string())
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(outputs, retained);
    }
    Ok(())
}

async fn seed(
    harness: &Harness,
    left: &str,
    right: &str,
    units: i64,
    revision: &str,
) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for sql in [
        "DELETE FROM game_item_nested_entries WHERE item_instance_id IN (encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
        "DELETE FROM game_character_equipment_slots WHERE item_instance_id IN (encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
        "DELETE FROM game_item_ground_locations WHERE item_instance_id IN (encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
    ] {
        sqlx::query(sql)
            .bind(id(71).as_slice())
            .bind(id(72).as_slice())
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query(
        "DELETE FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .execute(&mut *tx)
    .await?;
    for (seed, key, quantity) in [
        (73, "oteryn:item.tibia.i2854", 1),
        (74, "oteryn:item.tibia.i2854", 1),
        (71, "oteryn:item.tibia.i47384", units),
        (72, "oteryn:item.tibia.i47398", 1),
    ] {
        if seed == 72 && right == "same" {
            continue;
        }
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',$4,1,game_character_uuid_v7()) ON CONFLICT(item_instance_id) DO UPDATE SET quantity=excluded.quantity,lifecycle=1,last_transaction_id=NULL")
            .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).bind(quantity).execute(&mut *tx).await?;
    }
    for (seed, ordinal) in [(73, 1_i64), (74, 2)] {
        sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,game_character_uuid_v7())")
            .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice()).bind(ordinal).execute(&mut *tx).await?;
    }
    for (item, location, bag, ordinal) in [(71, left, 73, 3_i64), (72, right, 74, 4)] {
        match location {
            "same" => {}
            "main" => {
                sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,game_character_uuid_v7())")
                    .bind(id(item).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice()).bind(ordinal).execute(&mut *tx).await?;
            }
            "nested" => {
                sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,game_character_uuid_v7())")
                    .bind(id(item).as_slice()).bind(id(42).as_slice()).bind(id(bag).as_slice()).execute(&mut *tx).await?;
            }
            "equipment" => {
                sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,5,encode($2,'hex')::uuid,encode($3,'hex')::uuid,game_character_uuid_v7())")
                    .bind(id(41).as_slice()).bind(id(item).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
            }
            "ground" => {
                let mut cell = Vec::new();
                cell.extend_from_slice(&6_i32.to_be_bytes());
                cell.extend_from_slice(&0_i32.to_be_bytes());
                cell.extend_from_slice(&(-7_i16).to_be_bytes());
                sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$7,'content-r1',decode('01','hex'),$6)")
                    .bind(id(item).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(cell).bind([crate::durability::item_loop_move::ITEM_LOOP_DROP_REF_PREFIX, id(item).as_slice()].concat()).bind(ordinal).bind(revision).execute(&mut *tx).await?;
            }
            _ => return Err("unknown custody fixture".into()),
        }
    }
    tx.commit().await?;
    Ok(())
}
