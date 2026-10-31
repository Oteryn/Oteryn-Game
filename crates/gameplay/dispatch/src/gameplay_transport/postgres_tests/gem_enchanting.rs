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
fn gemx_shrine_stack_wire_atomic_replay_and_refusals() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "craft e2e panic")??;
    Ok(())
}

#[test]
fn gemx_weapon_and_ammunition_wire_atomic_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
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
        owner.movement_cells = &cells;
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
        super::item_look::seed_main_backpack(
            &harness,
            &[
                (dough, 3),
                ("oteryn:item.tibia.i3577", 1),
                ("oteryn:item.tibia.i676", 3),
                ("oteryn:item.tibia.i3271", 1),
                ("oteryn:item.tibia.i3447", 73),
                ("oteryn:item.tibia.i8077", 1),
            ],
        )
        .await?;
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
        if weapon {
            for (command, target, result_key, count, charges) in [
                (
                    1,
                    "oteryn:item.tibia.i3271",
                    "oteryn:item.tibia.i660",
                    1_i64,
                    Some(1000_i32),
                ),
                (
                    2,
                    "oteryn:item.tibia.i3447",
                    "oteryn:item.tibia.i763",
                    73,
                    None,
                ),
                (
                    3,
                    "oteryn:item.tibia.i8077",
                    "oteryn:item.tibia.i8078",
                    1,
                    None,
                ),
            ] {
                let intent = UseIntent {
                    use_with_map_item: Some(MapUseTarget::Item(
                        std::num::NonZeroU64::new(1).ok_or("target")?,
                    )),
                    ..intent.clone()
                };
                let payload = encode_item_use_intent(&intent).map_err(debug)?;
                let before = live().await?;
                let (next, result, actual) = exchange(
                    &owner,
                    admitted,
                    command,
                    &payload,
                    reference("oteryn:item.tibia.i676"),
                    reference(target),
                )
                .await?;
                admitted = next;
                assert_eq!(result, UseDisposition::Committed);
                assert_eq!(live().await?, before);
                let row: (i64, Option<i32>) = sqlx::query_as("SELECT i.quantity,t.charges FROM game_item_instances i LEFT JOIN game_item_timed_states t USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1").bind(result_key).fetch_one(&harness.pool).await?;
                assert_eq!(row, (count, charges));
                // The retired input handles are deliberately retained: replay must use the receipt.
                let (next, replay, _) =
                    exchange(&owner, admitted, command, &actual, None, None).await?;
                admitted = next;
                assert_eq!(replay, UseDisposition::Committed);
                replay_record(&owner, deps.actor, admitted, command, &actual).await?;
                assert_eq!(live().await?, before);
                assert_eq!(
                    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records")
                        .fetch_one(&harness.pool)
                        .await?,
                    i64::try_from(command)?
                );
            }
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT quantity FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i676'").fetch_one(&harness.pool).await?, 0);
            eprintln!("GEMX_WEAPON_AMMO_EXECUTED");
        } else {
            // A wrong item and wrong shrine spend no resources and create no receipt.
            let before = live().await?;
            let (next, wrong, _) = exchange(
                &owner,
                admitted,
                1,
                &payload,
                reference("oteryn:item.tibia.i3577"),
                None,
            )
            .await?;
            admitted = next;
            assert_ne!(wrong, UseDisposition::Committed);
            assert_eq!(live().await?, before);
            let wrong = UseIntent {
                use_with_map_item: Some(MapUseTarget::Item(
                    std::num::NonZeroU64::new(1).ok_or("target")?,
                )),
                ..intent.clone()
            };
            let (next, wrong, _) = exchange(
                &owner,
                admitted,
                2,
                &encode_item_use_intent(&wrong).map_err(debug)?,
                reference(dough),
                reference("oteryn:item.tibia.i3271"),
            )
            .await?;
            admitted = next;
            assert_eq!(wrong, UseDisposition::NothingToUse);
            assert_eq!(live().await?, before);
            // Insufficient mana, then insufficient soul. Each refusal leaves the stack intact.
            for (command, mana, soul) in [(3, 899, 100), (4, 3000, 5)] {
                {
                    let runtime = deps.runtime.lock().await;
                    let mut states = owner.states.spell_states.lock().await;
                    let state = states
                        .get_mut(&runtime, deps.actor, admitted.game_session_id)
                        .ok_or("state")?;
                    state.restore_saved_vitals(185, mana);
                    state
                        .restore_bed_rest([soul, 0, 0, 0], owner.owner_now().get())
                        .ok_or("soul")?;
                }
                let before = live().await?;
                let (next, refused, _) =
                    exchange(&owner, admitted, command, &payload, reference(dough), None).await?;
                admitted = next;
                assert_eq!(refused, UseDisposition::NotPossible);
                assert_eq!(live().await?, before);
            }
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records")
                    .fetch_one(&harness.pool)
                    .await?,
                0
            );
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT quantity FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3030'").fetch_one(&harness.pool).await?, 3);
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
            let (next, result, actual) =
                exchange(&owner, admitted, 5, &payload, reference(dough), None).await?;
            admitted = next;
            assert_eq!(result, UseDisposition::Committed);
            let after = live().await?;
            assert_eq!((after.mana, after.soul), (2100, 94));
            let stored: (i32, i32) =
                sqlx::query_as("SELECT mana,rest[1] FROM game_character_last_positions")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(stored, (2100, 94));
            let outputs: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i676' AND minted_transaction_id IN (SELECT transaction_id FROM game_item_craft_records)").fetch_one(&harness.pool).await?;
            assert_eq!(outputs, 3);
            let (_, replay, _) = exchange(&owner, admitted, 5, &actual, None, None).await?;

            assert_eq!(replay, UseDisposition::Committed);
            replay_record(&owner, deps.actor, admitted, 5, &actual).await?;
            assert_eq!(live().await?, after);
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records")
                    .fetch_one(&harness.pool)
                    .await?,
                1
            );
            eprintln!("GEMX_SHRINE_EXECUTED");
        }
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
    current_target: Option<std::num::NonZeroU32>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        let mut source = None;
        let mut target = None;
        loop {
            let raw = super::item_look::read_frame(&mut client)
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
        loop {
            let raw = super::item_look::read_frame(&mut client)
                .await
                .map_err(|error| format!("result {command_id}: {error}"))?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command_id {
                    let disposition =
                        oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                            .map_err(debug)?;
                    return Ok::<_, Box<dyn std::error::Error>>((disposition, actual));
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
    let weapons = [
        3271, 7383, 7384, 7406, 7402, 3317, 3318, 7389, 7380, 3342, 3311, 3333, 7415, 7392, 3279,
        3447, 8077,
    ];
    for gem in oteryn_gameplay_items::item_craft::enchanting::ENCHANTED {
        for weapon in weapons {
            let rule = oteryn_gameplay_items::item_craft::enchanting::rule(gem, weapon, 3, 1)
                .ok_or("rule")?;
            let output = oteryn_gameplay_items::item_craft::enchanting::output(native, rule)
                .ok_or("output admission")?;
            let record = native
                .item_policy(&output.production_key, &output.revision_ref)
                .ok_or("policy")?
                .record();
            let expected = donor["records"]
                .as_array()
                .ok_or("donor rows")?
                .iter()
                .find(|row| {
                    row["production_definition"]["production_key"].as_str()
                        == Some(&output.production_key)
                })
                .ok_or("donor output")?;
            assert_eq!(
                serde_json::to_value(&record.semantics.weapon)?,
                expected["semantics"]["weapon"]
            );
            if rule.charges.is_some() {
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
