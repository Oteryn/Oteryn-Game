//! W19 event containers use the existing item-use dispatch and craft receipt.
//! Each case checks committed rows, refusal rollback and restart replay.
#[path = "event_containers/ground.rs"]
mod ground;
#[path = "event_containers/wire.rs"]
mod wire;
use super::fixture::{Harness, debug, id, run};
use super::item_craft::exchange;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_gameplay_items::reward_bag::{reward_bag_conversion, reward_bag_draw};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

#[test]
fn w19_event_containers_present_wire_atomic_replay_and_refusals() -> TestResult {
    run_scenario("oteryn:item.tibia.i6570", 1, 1)
}

#[test]
fn w19_event_containers_christmas_wire_atomic_replay_and_refusals() -> TestResult {
    run_scenario("oteryn:item.tibia.i6506", 1, 7)
}

#[test]
fn w19_event_containers_special_box_wire_atomic_replay_and_refusals() -> TestResult {
    run_scenario("oteryn:item.tibia.i37457", 2, 1)
}

#[test]
fn w19_event_containers_anniversary_wire_atomic_replay_and_refusals() -> TestResult {
    run_scenario("oteryn:item.tibia.i39710", 2, 14)
}

#[test]
fn w19_event_containers_music_box_soft_boots_mint_equip_and_replay() -> TestResult {
    run_scenario("oteryn:item.tibia.i12045", 1, 1)
}

#[test]
fn w19_event_containers_nested_christmas_wire_atomic_replay_and_refusals() -> TestResult {
    run_scenario_source("oteryn:item.tibia.i6506", 1, 7, true)
}

fn run_scenario(bag_key: &'static str, source_quantity: u32, output_count: usize) -> TestResult {
    run_scenario_source(bag_key, source_quantity, output_count, false)
}

fn run_scenario_source(
    bag_key: &'static str,
    source_quantity: u32,
    output_count: usize,
    nested: bool,
) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!(
            "W19 POSTGRES_CASE_COMPLETED mode=SKIPPED_POSTGRES_UNCONFIGURED item={bag_key} nested={nested}"
        );
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            scenario(bag_key, source_quantity, output_count, nested).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "event container e2e panic")??;
    eprintln!("W19 POSTGRES_CASE_COMPLETED mode=POSTGRES item={bag_key} nested={nested}");
    Ok(())
}

fn scenario(bag_key: &str, source_quantity: u32, output_count: usize, nested: bool) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "w19_event_container", true, false).await?;
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let oven = "oteryn:item.tibia.i2535";
        let bag = "oteryn:item.tibia.i2854";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (oven, reference(oven).ok_or("oven")?.get() - 1),
        );
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 6,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                let policy = native.item_policy(key, "definition-r1")?.record();
                let crate::content::ReferenceItemField::Known(physical) =
                    &policy.semantics.physical
                else {
                    return None;
                };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable
                        == crate::content::ReferenceItemField::Known(true),
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = map
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (6, 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            content_pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 6,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        deps.initialize(&owner).await;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        let mut seed_items = vec![(bag_key, i64::from(source_quantity))];
        if nested {
            seed_items.push((bag, 1));
        }
        super::item_look::seed_main_backpack(&harness, &seed_items).await?;
        if nested {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($1,'hex')::uuid)")
                .bind(id(71).as_slice()).bind(id(42).as_slice()).bind(id(72).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            if nested {
                &[4, 6, 11, 12, 14, 15, 18]
            } else {
                &[4, 6, 11, 15, 18]
            },
        )
        .ok_or("reward bag offer selectable")?;

        let bag_definition = crate::gameplay_transport::item_look::typed_definition(
            index,
            reference(bag_key).ok_or("source reference")?,
        )
        .ok_or("source definition")?;
        // Keep the real session's command floor at one. Select a valid fixture
        // item UUID for the source branch instead of skipping unexecuted commands.
        let command_id = if nested { 2 } else { 1 };
        // Crystal CreateItem normalizes the source prize 3549 to base soft boots 6529.
        let source = if bag_key == "oteryn:item.tibia.i12045" {
            (1_u32..=4096)
                .map(|candidate| {
                    let mut source = id(71);
                    source[12..].copy_from_slice(&candidate.to_be_bytes());
                    source
                })
                .find(|source| {
                    let draw =
                        reward_bag_draw(*fence.game_session_id.as_bytes(), command_id, *source);
                    reward_bag_conversion(native, &bag_definition, draw).is_some_and(|plan| {
                        plan.outputs.iter().any(|(definition, count)| {
                            definition.production_key == "oteryn:item.tibia.i6529" && *count == 1
                        })
                    })
                })
                .ok_or("source music-box soft-boots draw")?
        } else {
            id(71)
        };
        if source != id(71) {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE game_item_instances SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid")
                .bind(source.as_slice()).bind(id(71).as_slice()).execute(&mut *tx).await?;
            sqlx::query("UPDATE game_item_container_entries SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid")
                .bind(source.as_slice()).bind(id(71).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        let draw = reward_bag_draw(*fence.game_session_id.as_bytes(), command_id, source);
        let expected =
            reward_bag_conversion(native, &bag_definition, draw).ok_or("event conversion")?;
        assert_eq!(expected.outputs.len(), output_count);
        let mut expected_items: Vec<_> = expected
            .outputs
            .iter()
            .map(|(definition, count)| (definition.production_key.clone(), *count))
            .collect();
        let source_id = bag_key
            .strip_prefix("oteryn:item.tibia.i")
            .ok_or("source id")?;
        assert_eq!(
            expected.recipe,
            format!("oteryn:reward-bag.event_{source_id}")
        );
        // SP04 consumes one unit, including anniversary boxes.
        let remaining_quantity = source_quantity - 1;
        if remaining_quantity > 0 && !nested {
            expected_items.push((bag_key.to_string(), remaining_quantity));
        }
        if nested {
            expected_items.push((bag.to_string(), 1));
        }
        expected_items.sort();
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("source")?),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let craft_count = async || {
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records")
                .fetch_one(&harness.pool)
                .await
        };
        let command = |fence| UseCommand {
            game_session_id: deps.admitted.game_session_id,
            command_id,
            item_fence: Some(fence),
        };
        assert!(
            owner
                .use_reward_bag_intent(
                    deps.actor,
                    command(fence),
                    &intent,
                    Some(ItemKey::Instance(source))
                )
                .await
                .map_err(debug)?
                .is_none(),
            "W19 pre-routing must defer to SP04"
        );
        // Changing only the current connection generation cannot consume the source.
        let mut stale = fence;
        stale.connection_generation =
            crate::foundation::ConnectionGeneration::new(fence.connection_generation.get() + 1)
                .map_err(debug)?;
        let refused = owner
            .use_reward_bag(deps.actor, command(stale), ItemKey::Instance(source))
            .await
            .ok_or("SP04 route")?
            .ok_or("stale result")?;
        assert_eq!(refused.disposition, UseDisposition::Rejected);
        assert_eq!(craft_count().await?, 0);
        let unchanged: (i64, i16) = sqlx::query_as(
            "SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
        ).bind(source.as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(unchanged, (i64::from(source_quantity), 1));

        if nested {
            let before = ground::durable_state(&harness).await?;
            let (next, result, raw) = wire::exchange_nested(
                &owner,
                admitted,
                command_id,
                &intent,
                reference(bag).ok_or("nested bag definition")?,
                reference(bag_key).ok_or("nested reward definition")?,
            )
            .await?;
            assert_eq!(result, UseDisposition::NothingToUse);
            assert_eq!(ground::durable_state(&harness).await?, before);
            let (_, replayed, same) = exchange(&owner, next, command_id, &raw, None).await?;
            assert_eq!(replayed, UseDisposition::NothingToUse);
            assert_eq!(same, raw);
            assert_eq!(ground::durable_state(&harness).await?, before);
            assert_eq!(craft_count().await?, 0);
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }

        // Nineteen real, heavy native items exhaust both slots and capacity. A
        // multi-output recipe refuses slots; the one-output recipe reaches weight.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let mut fillers = Vec::new();
        for ordinal in 100_i64..119 {
            fillers.push(sqlx::query_scalar::<_, String>(
                "WITH i AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item','oteryn:item.tibia.i43872','definition-r1',1,1,game_character_uuid_v7()) RETURNING item_instance_id) INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,game_character_uuid_v7() FROM i RETURNING item_instance_id::text",
            ).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice())
                .bind(ordinal).fetch_one(&mut *tx).await?);
        }
        tx.commit().await?;
        let refused = owner
            .use_reward_bag(deps.actor, command(fence), ItemKey::Instance(source))
            .await
            .ok_or("SP04 route")?
            .ok_or("resource refusal")?;
        assert_eq!(
            refused.disposition,
            if output_count > 1 || remaining_quantity > 0 || nested {
                UseDisposition::NoRoom
            } else {
                UseDisposition::NotPossible
            }
        );
        assert_eq!(craft_count().await?, 0);
        let unchanged: (i64, i16) = sqlx::query_as(
            "SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
        ).bind(source.as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(unchanged, (i64::from(source_quantity), 1));
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id::text=ANY($1)")
            .bind(&fillers)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM game_item_instances WHERE item_instance_id::text=ANY($1)")
            .bind(&fillers)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        if bag_key == "oteryn:item.tibia.i6506" {
            // A fixture has no node driver to replenish retired ready holders.
            // Without maintenance this unknown USE outcome disconnects the wire.
            let mut holders = Vec::new();
            while let Ok(holder) = owner.root.try_acquire_ready() {
                holders.push(holder);
            }
            for holder in holders {
                holder.close().await?;
            }
            assert!(owner.root.has_ready_demand());
            assert!(!owner.root.is_ready());
            assert!(matches!(
                owner
                    .use_reward_bag(deps.actor, command(fence), ItemKey::Instance(source))
                    .await,
                Some(None)
            ));
            assert_eq!(craft_count().await?, 0);
        }
        admitted.continuity.next_command_id = if nested { command_id - 1 } else { command_id };
        let (next, result, payload) = if nested {
            wire::exchange_nested(
                &owner,
                admitted,
                command_id,
                &intent,
                reference(bag).ok_or("nested bag definition")?,
                reference(bag_key).ok_or("nested reward definition")?,
            )
            .await?
        } else {
            exchange(&owner, admitted, command_id, &payload, reference(bag_key)).await?
        };
        oteryn_protocol_oteryn::world_object::decode_use_intent_target(
            next.continuity.selected_capabilities.as_slice(),
            &payload,
        )
        .map_err(debug)?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(craft_count().await?, 1);
        let retired: (i64, i16, i64) = sqlx::query_as(
            "SELECT quantity,lifecycle,(SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid",
        ).bind(source.as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(
            retired,
            if remaining_quantity == 0 {
                (0, 2, 0)
            } else {
                (i64::from(remaining_quantity), 1, i64::from(!nested))
            }
        );
        let nested_entries: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(source.as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(nested_entries, i64::from(nested && remaining_quantity > 0));
        let nested_proof: Option<serde_json::Value> =
            sqlx::query_scalar("SELECT event_source_location FROM game_item_craft_records")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(nested_proof.is_some(), nested);
        let durable = harness
            .root
            .read_character_backpack(&authority, fence.character_id)
            .await?
            .ok_or("backpack read-back")?;
        let mut items: Vec<_> = durable
            .entries
            .iter()
            .map(|entry| {
                (
                    entry.item.definition.production_key.clone(),
                    entry.item.quantity,
                )
            })
            .collect();
        items.sort();
        assert_eq!(items, expected_items);
        let (recipe, outputs, hash_matches): (String, serde_json::Value, bool) = sqlx::query_as(
            "SELECT r.recipe,r.outputs,a.envelope_sha256=sha256(a.envelope) AND a.envelope_sha256=r.envelope_sha256 FROM game_item_craft_records r JOIN game_item_audit_outbox a USING(event_id)",
        ).fetch_one(&harness.pool).await?;
        assert_eq!(recipe, expected.recipe);
        assert_eq!(
            outputs.as_array().ok_or("recorded outputs")?.len(),
            output_count
        );
        assert!(hash_matches);
        if bag_key == "oteryn:item.tibia.i39710" {
            let children: i64 = sqlx::query_scalar(
                "SELECT (SELECT count(*) FROM game_item_nested_entries e JOIN game_item_instances i ON i.item_instance_id=e.parent_item_instance_id WHERE i.definition_production_key='oteryn:item.tibia.i39693') + (SELECT count(*) FROM game_item_container_entries e JOIN game_item_instances i ON i.item_instance_id=e.parent_item_instance_id WHERE i.definition_production_key='oteryn:item.tibia.i39693')",
            ).fetch_one(&harness.pool).await?;
            assert_eq!(children, 0, "the birthday backpack is minted empty");
        }
        // A restarted live owner returns the stored receipt for the source, even
        // if the original stack unit is gone. Rebinding that command is rejected.
        drop(owner);
        let mut restarted = deps.owner(&harness, &authority);
        restarted.active_generation = Some(active);
        restarted.movement_cells = (&cells).into();
        deps.initialize(&restarted).await;
        let replayed = restarted
            .use_reward_bag(deps.actor, command(fence), ItemKey::Instance(source))
            .await
            .ok_or("SP04 route")?
            .ok_or("restart replay without retired view handle")?;
        assert_eq!(replayed.disposition, UseDisposition::Committed);
        let changed = restarted
            .use_reward_bag(deps.actor, command(fence), ItemKey::Instance(id(99)))
            .await
            .ok_or("SP04 route")?
            .ok_or("changed-intent replay")?;
        assert_eq!(changed.disposition, UseDisposition::Rejected);
        let recorded_intent: Vec<u8> =
            sqlx::query_scalar("SELECT intent FROM game_item_craft_records")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(
            recorded_intent,
            [b"oteryn:reward-bag-use:v1".as_slice(), source.as_slice()].concat(),
            "SP04 receipt binds the historical instance tag"
        );
        assert_eq!(craft_count().await?, 1);
        let replay_outputs: serde_json::Value =
            sqlx::query_scalar("SELECT outputs FROM game_item_craft_records")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(replay_outputs, outputs);
        let minted: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_instances i JOIN game_item_craft_records r ON r.transaction_id=i.minted_transaction_id",
        ).fetch_one(&harness.pool).await?;
        assert_eq!(minted, i64::try_from(output_count)?);
        if bag_key == "oteryn:item.tibia.i12045" {
            use crate::gameplay_transport::equipment_move::move_equipment;
            use crate::gameplay_transport::item_move::{EquipmentMove, EquipmentMoveResult};
            use oteryn_protocol_oteryn::item_view::EquipmentSlot;
            let boot = durable
                .entries
                .iter()
                .find(|entry| entry.item.definition.production_key == "oteryn:item.tibia.i6529")
                .ok_or("source soft-boots reward")?;
            let timed_rows: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid",
            ).bind(boot.item.item_instance_id.as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(
                timed_rows, 0,
                "a fresh active-time item starts from its admitted full budget"
            );
            let equip = move_equipment(
                &restarted,
                deps.actor,
                UseCommand {
                    game_session_id: deps.admitted.game_session_id,
                    command_id: command_id + 1,
                    item_fence: Some(fence),
                },
                EquipmentMove::Equip {
                    item: ItemKey::Instance(boot.item.item_instance_id),
                    to: EquipmentSlot::Feet,
                },
            )
            .await;
            assert_eq!(equip, EquipmentMoveResult::Committed);
            let equipment = restarted
                .observe_character_equipment(deps.actor, deps.admitted.game_session_id)
                .await
                .ok_or("reward equipment read-back")?;
            let worn = equipment
                .iter()
                .find(|(slot, _)| *slot == EquipmentSlot::Feet)
                .ok_or("reward in feet slot")?;
            assert_eq!(
                Some(worn.1.item_definition_ref),
                reference("oteryn:item.tibia.i3549"),
                "the existing equipment owner presents the donor activated form"
            );
            restarted.clock_origin -= std::time::Duration::from_secs(2);
            assert!(
                restarted
                    .checkpoint_timed_equipment(deps.actor, deps.admitted.game_session_id, true)
                    .await
            );
            let remaining: i64 = sqlx::query_scalar(
                "SELECT remaining_ms FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid",
            ).bind(boot.item.item_instance_id.as_slice()).fetch_one(&harness.pool).await?;
            assert!(
                (14_390_000..=14_398_000).contains(&remaining),
                "the existing equipment clock spent the fresh reward's budget: {remaining}"
            );
            let replayed = restarted
                .use_reward_bag(deps.actor, command(fence), ItemKey::Instance(source))
                .await
                .ok_or("SP04 route")?
                .ok_or("timed reward replay")?;
            assert_eq!(replayed.disposition, UseDisposition::Committed);
            let after_replay: i64 = sqlx::query_scalar(
                "SELECT remaining_ms FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid",
            ).bind(boot.item.item_instance_id.as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(
                after_replay, remaining,
                "opening replay cannot refill a reward's timed state"
            );
            assert_eq!(craft_count().await?, 1);
        }
        drop(restarted);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
