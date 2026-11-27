//! CRAFT-1: admitted USE bytes → real composed owner → atomic PG rows → result/reload.
use super::achievement_granters::{assert_progress, seed_progress};
use super::item_craft_fixtures::exchange;

use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};

use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority,
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[test]
fn item_craft_cooking_wire_atomic_replay_restart_and_refusals() -> TestResult {
    run_scenario(None)
}

#[test]
fn cach1_baking_cookies_counts_one_tray() -> TestResult {
    run_scenario(Some((
        "oteryn:item.tibia.i8020",
        "oteryn:item.tibia.i3464",
        "oteryn:achievement/cookie_monster",
        20,
    )))
}

#[test]
fn cach1_baking_plain_cake_counts_one_cake() -> TestResult {
    run_scenario(Some((
        "oteryn:item.tibia.i6276",
        "oteryn:item.tibia.i6277",
        "oteryn:achievement/the_cake_s_the_truth",
        30,
    )))
}

type BakingAward = (&'static str, &'static str, &'static str, i64);

fn run_scenario(award: Option<BakingAward>) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(award, None).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "craft e2e panic")??;
    Ok(())
}

#[test]
fn slice_two_plain_cake_counts_at_thirty_without_crediting_chocolate() -> TestResult {
    cake_case("oteryn:item.tibia.i6276", "oteryn:item.tibia.i6277")
}

#[test]
fn slice_two_chocolate_cake_counts_at_ten_without_crediting_plain_cake() -> TestResult {
    cake_case("oteryn:item.tibia.i8018", "oteryn:item.tibia.i8019")
}

fn cake_case(dough: &'static str, cake: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("slice-two-baking", move || {
        scenario(None, Some((dough, cake)))
    })
}

fn scenario(award: Option<BakingAward>, cake: Option<(&'static str, &'static str)>) -> TestResult {
    run(async move |admin| {
        let harness = Harness::create_with_admission_seed(admin, "item_craft", true, false).await?;
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
        let (dough, baked_output) = award.map_or(
            cake.unwrap_or(("oteryn:item.tibia.i3604", "oteryn:item.tibia.i3600")),
            |(dough, baked, _, _)| (dough, baked),
        );
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
                let crate::content::ReferenceItemField::Known(physical) = &policy.semantics.physical else { return None; };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable == crate::content::ReferenceItemField::Known(true),
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
        super::item_look_fixtures::seed_main_backpack(
            &harness,
            &[(dough, if award.is_some() { 1 } else { 3 }), ("oteryn:item.tibia.i3577", 1)],
        )
        .await?;
        if let Some((_, _, key, total)) = award {
            super::achievement_granters::seed_progress(&harness, key, total - 1).await?;
        }
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("craft offer selectable")?;
        let mut placement = map.base().digest().to_vec();
        let station_pos = crate::map::overlay::TilePos {
            x: 7,
            y: 0,
            floor: -7,
        };
        placement.extend_from_slice(
            &crate::map::view::placement_key(station_pos, 1)
                .ok_or("station placement")?
                .to_be_bytes(),
        );
        let station = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        // Domain 9 uses newest-first order: bag=1, unknown food=2, dough=3.
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("source")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(station.clone())),
            use_with_position: None,
        };
        let used_definition = crate::gameplay_transport::item_look::typed_definition(index, reference(dough).ok_or("dough reference")?).ok_or("used definition")?;
        let station_definition = crate::gameplay_transport::item_look::typed_definition(index, reference(oven).ok_or("oven reference")?).ok_or("station definition")?;
        let plan = oteryn_gameplay_items::item_craft::cooking_conversion(native, &used_definition, &station_definition).ok_or("admitted cooking recipe")?;
        assert_eq!(plan.outputs[0].0.production_key, baked_output);
        let typed = |key: &str| reference(key).and_then(|r| crate::gameplay_transport::item_look::typed_definition(index, r)).ok_or("cooking Item ref");
        let milled = oteryn_gameplay_items::item_craft::cooking_conversion(native, &typed("oteryn:item.tibia.i3605")?, &typed("oteryn:item.tibia.i1943")?).ok_or("milling recipe")?;
        assert_eq!(milled.outputs[0].0.production_key, "oteryn:item.tibia.i3603");
        let cookies = oteryn_gameplay_items::item_craft::cooking_conversion(native, &typed("oteryn:item.tibia.i8198")?, &station_definition).ok_or("cookies recipe")?;
        assert_eq!(cookies.outputs.len(), 2);
        assert_eq!(cookies.outputs[1].1, 12);
        let tray = oteryn_gameplay_items::item_craft::cooking_conversion(native, &typed("oteryn:item.tibia.i8020")?, &station_definition).ok_or("cookie tray recipe")?;
        assert_eq!(tray.outputs.len(), 2);
        assert_eq!(tray.outputs[0].0.production_key, "oteryn:item.tibia.i3464");
        assert_eq!((tray.outputs[1].0.production_key.as_str(), tray.outputs[1].1), ("oteryn:item.tibia.i3598", 12));
        let cake_achievements = [
            ("oteryn:achievement/the_cake_s_the_truth", 30, "oteryn:item.tibia.i6277"),
            ("oteryn:achievement/sweet_tooth", 10, "oteryn:item.tibia.i8019"),
        ];
        if cake.is_some() {
            for (key, target, _) in cake_achievements {
                seed_progress(&harness, key, target - 1).await?;
            }
        }
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        // Hold the connection only after the earlier USE probes have completed.
        let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
        let probing = crate::gameplay_transport::item_craft::CRAFT_RECEIPT_BARRIER.scope(
            barrier.clone(),
            owner.use_craft(
                deps.actor,
                crate::gameplay_transport::connection::UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 1,
                    item_fence: Some(fence),
                },
                intent.clone(),
                Some(crate::gameplay_transport::item_view::ItemKey::Instance(id(71))),
                None,
            ),
        );
        let holding = async {
            barrier.wait().await;
            let holder = harness.root.try_acquire_ready();
            barrier.wait().await;
            holder
        };
        let (probe, holder) = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            super::fixture::join_two(probing, holding),
        )
        .await?;
        let mut holder = holder?;
        holder
            .return_to_pool_observed_until(
                std::time::Instant::now() + crate::durability::DB_PASS_DEADLINE,
            )
            .await;
        assert!(
            probe.is_none(),
            "a failed craft receipt read must stay pending"
        );

        let (end, result, payload) = exchange(&owner, admitted, 1, &payload, reference(dough)).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records").fetch_one(&harness.pool).await?, 1);
        if let Some((_, _, key, total)) = award {
            let recorded: (String, serde_json::Value) =
                sqlx::query_as("SELECT event_id::text,outputs FROM game_item_craft_records")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(recorded.1[0]["key"], baked_output);
            let granted = super::achievement_granters::assert_progress(
                &harness, key, total, true, "oteryn:item-craft",
            )
            .await?;
            assert_eq!(
                granted.as_ref().map(|(_, event, _)| event.as_slice()),
                Some(recorded.0.as_bytes())
            );
            drop(owner);
            let mut restarted = deps.owner(&harness, &authority);
            restarted.active_generation = Some(active);
            restarted.movement_cells = (&cells).into();
            deps.initialize(&restarted).await;
            let (_, replayed, _) = exchange(&restarted, admitted, 1, &payload, None).await?;
            assert_eq!(replayed, UseDisposition::Committed);
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records")
                    .fetch_one(&harness.pool)
                    .await?,
                1
            );
            assert_eq!(
                super::achievement_granters::assert_progress(
                    &harness, key, total, true, "oteryn:item-craft",
                )
                .await?,
                granted
            );
            drop(restarted);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        let quantity:i64=sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(quantity, 2);
        let durable = harness
            .root
            .read_character_backpack(&authority, fence.character_id)
            .await?
            .ok_or("durable backpack")?;
        assert!(
            durable
                .entries
                .iter()
                .any(|e| e.item.definition.production_key == baked_output && e.item.quantity == 1)
        );
        // The immutable receipt and bounded outbox carry the same committed cooking proof.
        let (before, outputs, envelope, hash_matches): (serde_json::Value, serde_json::Value, Vec<u8>, bool) = sqlx::query_as(
            "SELECT r.before_item,r.outputs,a.envelope,a.envelope_sha256=sha256(a.envelope) AND a.envelope_sha256=r.envelope_sha256 FROM game_item_craft_records r JOIN game_item_audit_outbox a USING(event_id)",
        ).fetch_one(&harness.pool).await?;
        assert!(envelope.len() <= 9_216 && hash_matches);
        assert_eq!(before["quantity"], 3);
        assert_eq!(outputs[0]["key"], baked_output);
        // Reconstruct the owner and replay the original bytes without trusting current handles.
        drop(owner);
        let mut restarted = deps.owner(&harness, &authority);
        restarted.active_generation = Some(active);
        restarted.movement_cells = (&cells).into();
        deps.initialize(&restarted).await;
        let mut awarded = Vec::new();
        if cake.is_some() {
            for (key, target, output) in cake_achievements {
                let earned = baked_output == output;
                awarded.push(
                    assert_progress(
                        &harness, key, target - i64::from(!earned), earned, "oteryn:item-craft",
                    )
                    .await?,
                );
            }
        }
        let (_, replayed, _) = exchange(&restarted, admitted, 1, &payload, None).await?;
        assert_eq!(replayed, UseDisposition::Committed);
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records").fetch_one(&harness.pool).await?, 1);
        if cake.is_some() {
            for ((key, target, output), fact) in cake_achievements.into_iter().zip(awarded) {
                let earned = baked_output == output;
                assert_eq!(
                    assert_progress(
                        &harness, key, target - i64::from(!earned), earned, "oteryn:item-craft",
                    )
                    .await?,
                    fact
                );
            }
            drop(restarted);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        let mut next = end;
        next.continuity.next_command_id = 2;
        // Unknown pair: a food item on the oven never consumes or mints anything.
        let unknown = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(2).ok_or("food")?),
            ..intent.clone()
        };
        let (next, result, _) = exchange(
            &restarted,
            next,
            2,
            &encode_item_use_intent(&unknown).map_err(debug)?,
            reference("oteryn:item.tibia.i3577"),
        )
        .await?;
        assert_eq!(result, UseDisposition::NothingToUse);
        // A full backpack rejects a split conversion before any decrement or audit.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        for n in 0..17 {
            sqlx::query("WITH i AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',$2,'definition-r1',1,1,game_character_uuid_v7()) RETURNING item_instance_id) INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,encode($1,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,game_character_uuid_v7() FROM i")
                .bind(id(42).as_slice()).bind(baked_output).bind(id(41).as_slice()).bind(id(70).as_slice()).bind(100_i64+n).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        let (next, result, _) = exchange(&restarted, next, 3, &payload, reference(dough)).await?;
        assert_eq!(result, UseDisposition::NoRoom);
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records").fetch_one(&harness.pool).await?, 1);
        // A whole final unit frees exactly its own slot, even in the full backpack.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_item_instances SET quantity=1 WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let (next, result, _) = exchange(&restarted, next, 4, &payload, reference(dough)).await?;
        assert_eq!(result, UseDisposition::Committed);
        let retired: (i64,i16,i64) = sqlx::query_as("SELECT quantity,lifecycle,(SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(retired, (0,2,0));
        let old_slot: String = sqlx::query_scalar("SELECT i.definition_production_key FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND e.placement_ordinal=1").bind(id(70).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(old_slot, baked_output);
        let stale = UseIntent {target: UseTarget::Item(std::num::NonZeroU64::MAX), ..intent};
        let (_, result, _) = exchange(&restarted, next, 5, &encode_item_use_intent(&stale).map_err(debug)?, None).await?;
        assert_eq!(result, UseDisposition::StaleState);
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records").fetch_one(&harness.pool).await?, 2);
        drop(restarted);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
