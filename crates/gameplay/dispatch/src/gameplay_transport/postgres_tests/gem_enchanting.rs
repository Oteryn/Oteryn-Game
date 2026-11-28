//! I19: admitted USE bytes → real composed owner → atomic PG rows → result/reload.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use tokio::io::AsyncWriteExt;

#[test]
fn gemx_legacy_shrine_refuses_without_inventory_or_vitals_change() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "craft e2e panic")??;
    Ok(())
}

#[test]
fn gemx_legacy_weapons_refuse_and_live_gem_uses_commit_and_replay() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "enchanting e2e panic")??;
    Ok(())
}

fn scenario(weapon: bool) -> TestResult {
    run(async |admin| {
        let harness = Harness::create_with_admission_seed(admin, "gemx", true, false).await?;
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
        let oven = "oteryn:item.tibia.i146";
        let bag = "oteryn:item.tibia.i2854";
        let dough = "oteryn:item.tibia.i3030";
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
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: matches!(&policy.semantics.physical,crate::content::ReferenceItemField::Known(physical) if physical.pickupable == crate::content::ReferenceItemField::Known(true)),
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
        {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            *states
                .get_mut(&runtime, deps.actor, deps.admitted.game_session_id)
                .ok_or("state")? = crate::spell::cast::PlayerSpellState::new(
                crate::spell::cast::CharacterCastFacts {
                    vocation: crate::spell::Vocation::Monk,
                    max_mana: 3000,
                    ..crate::gameplay_transport::actor_spell::tests::FACTS
                },
                0,
                0,
            )
            .ok_or("state")?;
        }
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
        let mut entries = vec![
            (dough, 3),
            ("oteryn:item.tibia.i3577", 1),
            ("oteryn:item.tibia.i676", 3),
            ("oteryn:item.tibia.i3271", 1),
            ("oteryn:item.tibia.i3447", 73),
            ("oteryn:item.tibia.i8077", 1),
            ("oteryn:item.tibia.i9020", 1),
        ];
        if weapon {
            // Existing custody can outlive its content qualification. The
            // ground-only fixture has no qualified carried weight and is not a recipe input.
            entries.push((oven, 1));
        }
        super::item_look_fixtures::seed_main_backpack(&harness, &entries).await?;
        // The expiry owner leaves an empty timed row on the worn boots.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO game_item_timed_states(item_instance_id,state_revision)
                     SELECT item_instance_id,3 FROM game_item_instances
                     WHERE definition_production_key='oteryn:item.tibia.i9020'",
        )
        .execute(&mut *tx)
        .await?;
        if weapon {
            sqlx::query("UPDATE game_character_progression_state SET level=250 WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,committed_character_revision,last_build_occurrence_id) VALUES(encode($1,'hex')::uuid,'knight',10,0,10,0,10,0,10,0,10,0,10,0,10,0,10,0,2,encode($2,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET vocation='knight'")
                .bind(id(41).as_slice()).bind(id(30).as_slice()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
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
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let session = admitted.game_session_id;
        let live = async || {
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            states
                .get(&runtime, deps.actor, session)
                .map(|s| s.vitals())
                .ok_or("live")
        };
        let inventory = async || {
            sqlx::query_scalar::<_, serde_json::Value>(
                "SELECT jsonb_agg(jsonb_build_array(to_jsonb(i),to_jsonb(e),to_jsonb(t)) ORDER BY i.item_instance_id)
                 FROM game_item_instances i LEFT JOIN game_item_container_entries e USING(item_instance_id)
                 LEFT JOIN game_item_timed_states t USING(item_instance_id)",
            )
            .fetch_one(&harness.pool)
            .await
        };
        let before_inventory = inventory().await?;
        let mut command = 1;
        if weapon {
            let item_use = UseIntent {
                use_with_map_item: Some(MapUseTarget::Item(
                    std::num::NonZeroU64::new(1).ok_or("target")?,
                )),
                ..intent
            };
            let payload = encode_item_use_intent(&item_use).map_err(debug)?;
            let ruby = reference("oteryn:item.tibia.i676").ok_or("ruby")?;
            let before = live().await?;
            for target in ["oteryn:item.tibia.i3271", "oteryn:item.tibia.i3447"] {
                let (next, result, _) = exchange(
                    &owner,
                    admitted,
                    command,
                    &payload,
                    Some(ruby),
                    reference(target),
                    None,
                )
                .await?;
                admitted = next;
                command += 1;
                assert_eq!(
                    result,
                    UseDisposition::NothingToUse,
                    "legacy target {target}"
                );
                assert_eq!(live().await?, before);
                assert_eq!(inventory().await?, before_inventory);
            }
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records")
                    .fetch_one(&harness.pool)
                    .await?,
                0
            );
            for (target, output, remaining) in [
                ("oteryn:item.tibia.i8077", "oteryn:item.tibia.i8078", 2_i64),
                ("oteryn:item.tibia.i9020", "oteryn:item.tibia.i9019", 1),
            ] {
                let (next, result, actual) = exchange(
                    &owner,
                    admitted,
                    command,
                    &payload,
                    Some(ruby),
                    reference(target),
                    Some(reference(output).ok_or("craft output definition")?),
                )
                .await?;
                admitted = next;
                assert_eq!(result, UseDisposition::Committed, "live target {target}");
                assert_eq!(live().await?, before);
                let rows: Vec<(String, i64, i16)> = sqlx::query_as(
                    "SELECT definition_production_key,quantity,lifecycle FROM game_item_instances
                     WHERE definition_production_key=ANY($1) ORDER BY definition_production_key",
                )
                .bind(["oteryn:item.tibia.i676", target, output].as_slice())
                .fetch_all(&harness.pool)
                .await?;
                assert_eq!(rows.len(), 3);
                for (key, quantity, lifecycle) in [
                    ("oteryn:item.tibia.i676", remaining, 1),
                    (target, 0, 2),
                    (output, 1, 1),
                ] {
                    assert!(
                        rows.iter()
                            .any(|row| row.0 == key && (row.1, row.2) == (quantity, lifecycle)),
                        "{key}: {rows:?}"
                    );
                }
                let committed_inventory = inventory().await?;
                let (next, replay, _) =
                    exchange(&owner, admitted, command, &actual, None, None, None).await?;
                admitted = next;
                assert_eq!(replay, UseDisposition::Committed);
                replay_record(&owner, deps.actor, admitted, command, &actual).await?;
                assert_eq!(live().await?, before);
                assert_eq!(inventory().await?, committed_inventory);
                command += 1;
            }
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records")
                    .fetch_one(&harness.pool)
                    .await?,
                2
            );
            // The SQL proof independently checks the same outputs as the gameplay rule.
            use oteryn_gameplay_items::item_craft::enchanting::{ENCHANTED, GEMS, rule};
            for source in ENCHANTED.into_iter().chain(GEMS) {
                for target in [146, 150, 154, 158, 3271, 3447, 8077, 9020] {
                    let persisted_rule: Option<i32> =
                        sqlx::query_scalar("SELECT game_item_enchanting_result($1,$2)")
                            .bind(i32::from(source))
                            .bind(i32::from(target))
                            .fetch_one(&harness.pool)
                            .await?;
                    assert_eq!(
                        persisted_rule,
                        rule(source, target, 3, 1).map(|rule| i32::from(rule.output)),
                        "SQL rule for {source} -> {target}"
                    );
                }
            }
            equip_and_expire_rainbow_shield(&harness, &owner, admitted, command).await?;
            eprintln!("GEMX_LIVE_USES_EXECUTED");
        } else {
            {
                let runtime = deps.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                let state = states
                    .get_mut(&runtime, deps.actor, admitted.game_session_id)
                    .ok_or("state")?;
                state.restore_saved_vitals(185, 3000);
                state
                    .restore_bed_rest([100, 0, 0, 0], owner.owner_now().get())
                    .ok_or("soul")?;
            }
            let before = live().await?;
            let bare = encode_item_use_intent(&UseIntent {
                use_with_map_item: None,
                ..intent.clone()
            })
            .map_err(debug)?;
            // These definite refusals must succeed even while an unrelated writer holds the lane.
            let busy_lane = owner.spell_lane.acquire().await.map_err(|_| "busy fixture lane")?;
            for (name, payload) in [("legacy shrine", &payload), ("untargeted ruby", &bare)] {
                let (next, result, _) = exchange(
                    &owner, admitted, command, payload, reference(dough), None, None,
                )
                .await?;
                admitted = next;
                command += 1;
                assert_eq!(result, UseDisposition::NothingToUse, "{name}");
                assert_eq!(live().await?, before, "{name} changed vitals");
                assert_eq!(inventory().await?, before_inventory, "{name} changed inventory");
            }
            drop(busy_lane);
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records")
                    .fetch_one(&harness.pool)
                    .await?,
                0
            );

            let (next, result, _) = exchange(
                &owner,
                admitted,
                command,
                &payload,
                reference("oteryn:item.tibia.i3577"),
                None,
                None,
            )
            .await?;
            assert_ne!(result, UseDisposition::Committed);
            let wrong_target = UseIntent {
                use_with_map_item: Some(MapUseTarget::Item(
                    std::num::NonZeroU64::new(1).ok_or("target")?,
                )),
                ..intent
            };
            let (_, result, _) = exchange(
                &owner,
                next,
                command + 1,
                &encode_item_use_intent(&wrong_target).map_err(debug)?,
                reference(dough),
                reference("oteryn:item.tibia.i3271"),
                None,
            )
            .await?;
            assert_eq!(result, UseDisposition::NothingToUse);
            assert_eq!(live().await?, before);
            assert_eq!(inventory().await?, before_inventory);
            eprintln!("GEMX_LEGACY_SHRINE_EXECUTED");
        }
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn equip_and_expire_rainbow_shield(
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
) -> TestResult {
    use crate::gameplay_transport::connection::UseCommand;
    use crate::gameplay_transport::item_move::{EquipmentMove, EquipmentMoveResult};
    use crate::gameplay_transport::item_view::ItemKey;
    use oteryn_protocol_oteryn::item_view::EquipmentSlot;

    let shield: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(item_instance_id) FROM game_item_instances
         WHERE definition_production_key='oteryn:item.tibia.i8078' AND lifecycle=1",
    )
    .fetch_one(&harness.pool)
    .await?;
    let actor = admitted.runtime_actor.ok_or("shield owner")?;
    let result = crate::gameplay_transport::equipment_move::move_equipment(
        owner,
        actor,
        UseCommand {
            game_session_id: admitted.game_session_id,
            command_id,
            item_fence: admitted.item_fence,
        },
        EquipmentMove::Equip {
            item: ItemKey::Instance(shield.as_slice().try_into()?),
            to: EquipmentSlot::LeftHand,
        },
    )
    .await;
    assert_eq!(
        result,
        EquipmentMoveResult::Committed,
        "equip the crafted shield"
    );
    let deadline: (i64, i64, String) = sqlx::query_as(
        "SELECT s.duration_millis,s.expires_at_unix_ms-c.occurred_at,s.target_production_key
         FROM game_spell_item_temporal_schedules s
         JOIN game_item_craft_records c ON c.transaction_id=s.source_transaction_id
         WHERE uuid_send(s.item_instance_id)=$1",
    )
    .bind(&shield)
    .fetch_one(&harness.pool)
    .await?;
    assert!(deadline.0 > 0);
    assert_eq!(deadline.0, deadline.1);
    assert_eq!(deadline.2, "oteryn:item.tibia.i8077");

    // Advance the fixture's persisted deadline, then drive the normal absolute owner.
    let mut fixture = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    sqlx::query(
        "UPDATE game_spell_item_temporal_schedules
         SET expires_at_unix_ms=floor(extract(epoch FROM clock_timestamp())*1000)::bigint-1
         WHERE uuid_send(item_instance_id)=$1",
    )
    .bind(&shield)
    .execute(&mut *fixture)
    .await?;
    fixture.commit().await?;
    owner
        .states
        .spell_states
        .lock()
        .await
        .next_item_deadline_pass_us = 0;
    owner.drain_source_item_deadlines().await;
    let expired: (String, i16, i16) = sqlx::query_as(
        "SELECT i.definition_production_key,e.slot,r.custody_kind
         FROM game_item_instances i JOIN game_spell_item_temporal_receipts r USING(item_instance_id)
         JOIN game_character_equipment_slots e USING(item_instance_id)
         WHERE uuid_send(i.item_instance_id)=$1 AND i.lifecycle=1",
    )
    .bind(&shield)
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(expired, ("oteryn:item.tibia.i8077".into(), 5, 3));
    Ok(())
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
    current_target: Option<std::num::NonZeroU32>,
    expected_output: Option<std::num::NonZeroU32>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    use oteryn_protocol_oteryn::item_view::{
        STATE_DOMAIN_CHARACTER_INVENTORY, decode_character_inventory,
    };
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        let mut source = None;
        let mut target = None;
        loop {
            let raw = super::item_look_fixtures::read_frame(&mut client)
                .await
                .map_err(|error| format!("snapshot: {error}"))?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id
                        == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY
                    {
                        let inventory =
                            oteryn_protocol_oteryn::item_view::decode_character_inventory(
                                domain.payload,
                            )
                            .map_err(debug)?;
                        target = inventory
                            .entries
                            .iter()
                            .find(|entry| Some(entry.item_definition_ref) == current_target)
                            .map(|entry| entry.handle);
                        source = inventory
                            .entries
                            .iter()
                            .find(|entry| Some(entry.item_definition_ref) == current_source)
                            .map(|entry| entry.handle);
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit {
                break;
            }
        }
        let actual = if current_source.is_some() {
            let mut intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                admitted.continuity.selected_capabilities.as_slice(),
                payload,
            )
            .map_err(debug)?;
            intent.target = UseTarget::Item(source.ok_or("current source handle")?);
            if current_target.is_some() {
                intent.use_with_map_item =
                    Some(MapUseTarget::Item(target.ok_or("current target handle")?));
            }
            encode_item_use_intent(&intent).map_err(debug)?
        } else {
            payload.to_vec()
        };
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: 2,
                payload: &actual,
            },
        )?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        client.shutdown().await?;
        let mut reply = None;
        loop {
            let raw = super::item_look_fixtures::read_frame(&mut client)
                .await
                .map_err(|error| format!("USE {command_id}: result={reply:?}, expected output={expected_output:?}: {error}"))?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command_id {
                    let disposition =
                        oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                            .map_err(debug)?;
                    if current_source.is_some() && disposition != UseDisposition::Rejected {
                        assert_eq!(result.status, oteryn_protocol_oteryn::CommandStatus::Accepted);
                        assert_eq!(result.error_code, 0);
                    }
                    if expected_output.is_none() {
                        return Ok::<_, Box<dyn std::error::Error>>((disposition, actual));
                    }
                    assert_eq!(disposition, UseDisposition::Committed, "USE {command_id}");
                    reply = Some(disposition);
                }
            }
            if let (Some(disposition), Some(expected)) = (reply, expected_output)
                && envelope.message_type() == crate::foundation::MessageType::StateDelta
            {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                if delta.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                    let inventory = decode_character_inventory(delta.payload).map_err(debug)?;
                    assert!(inventory.entries.iter().any(|entry| entry.item_definition_ref == expected),
                        "USE {command_id} inventory delta is missing output {expected}");
                    return Ok((disposition, actual));
                }
            }
        }
    };
    use std::future::Future;
    use std::task::Poll;
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let mut serving = std::pin::pin!(serving);
    let mut client_task = std::pin::pin!(client_task);
    let mut end = None;
    let mut result = None;
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        std::future::poll_fn(|context| {
            if end.is_none()
                && let Poll::Ready(value) = serving.as_mut().poll(context)
            {
                end = Some(value);
            }
            if result.is_none()
                && let Poll::Ready(value) = client_task.as_mut().poll(context)
            {
                result = Some(value);
            }
            if end.is_some() && result.is_some() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }),
    )
    .await?;
    let end = end.ok_or("server did not finish")?;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end else {
        return Err(format!("{end:?}").into());
    };
    let (result, actual) = result.ok_or("client did not finish")??;
    Ok((admitted, result, actual))
}

#[test]
fn gemx_native_output_closure_includes_stacks_elements_and_charges() -> TestResult {
    use crate::content::ReferenceItemField as F;
    let (controller, _) = crate::content::native_gameplay::test_support::activated_with_loot_tables(
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        None,
    );
    let active = controller.active().ok_or("active")?;
    let native = active.native_gameplay().ok_or("native")?;
    let donor: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/enchanting-item-profiles.json"
    ))?;
    let global: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../../tools/content-schema/native-gameplay/enchanting-global-weapon-facts.json"
    ))?;
    for weapon in donor["rules"]["weapons"].as_array().ok_or("weapons")? {
        let target = weapon[0].as_u64().ok_or("target")?;
        for output in weapon[1].as_array().ok_or("outputs")? {
            let key = format!("oteryn:item.tibia.i{}", output.as_u64().ok_or("output")?);
            let record = native
                .item_policy(&key, "definition-r1")
                .ok_or("policy")?
                .record();
            assert!(record.admission.materializable, "{key}");
            let expected = donor["records"]
                .as_array()
                .ok_or("donor rows")?
                .iter()
                .find(|row| {
                    row["production_definition"]["production_key"].as_str() == Some(key.as_str())
                })
                .ok_or("donor output")?;
            let expected_weapon = global.get(&key).unwrap_or(&expected["semantics"]["weapon"]);
            assert_eq!(
                &serde_json::to_value(&record.semantics.weapon)?,
                expected_weapon,
                "{key}"
            );
            if !matches!(target, 3447 | 8077) {
                assert!(
                    matches!(&record.semantics.charges,F::Known(charges) if charges.count == F::Known(1000))
                );
            }
        }
    }
    Ok(())
}

// Enter the real mechanic owner with no current handles: this cannot use the transport's
// retained result and must answer from the immutable PostgreSQL craft record.
async fn replay_record(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    admitted: AdmittedSession,
    command_id: u64,
    bytes: &[u8],
) -> TestResult {
    let intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
        admitted.continuity.selected_capabilities.as_slice(),
        bytes,
    )
    .map_err(debug)?;
    let command = crate::gameplay_transport::connection::UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: admitted.item_fence,
    };
    let outcome = owner
        .use_craft(actor, command, intent.clone(), None, None)
        .await
        .ok_or("receipt replay")?;
    assert_eq!(outcome.disposition, UseDisposition::Committed);
    let changed = UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MAX),
        ..intent
    };
    let refused = owner
        .use_craft(actor, command, changed, None, None)
        .await
        .ok_or("changed replay")?;
    assert_eq!(refused.disposition, UseDisposition::Rejected);
    Ok(())
}
