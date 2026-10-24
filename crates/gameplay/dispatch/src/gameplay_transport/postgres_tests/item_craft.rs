//! CRAFT-1: admitted USE bytes → real composed owner → atomic PG rows → result/reload.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use tokio::io::{AsyncWriteExt};

#[test]
fn item_craft_cooking_wire_atomic_replay_restart_and_refusals() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "craft e2e panic")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
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
        let dough = "oteryn:item.tibia.i3604";
        let bread = "oteryn:item.tibia.i3600";
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
        owner.movement_cells = &cells;
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
        super::item_look::seed_main_backpack(
            &harness,
            &[(dough, 3), ("oteryn:item.tibia.i3577", 1)],
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
        let used_definition = crate::gameplay_transport::item_look::typed_definition(index, reference(dough).ok_or("dough reference")?).ok_or("used definition")?;
        let station_definition = crate::gameplay_transport::item_look::typed_definition(index, reference(oven).ok_or("oven reference")?).ok_or("station definition")?;
        let plan = oteryn_gameplay_items::item_craft::cooking_conversion(native, &used_definition, &station_definition).ok_or("admitted cooking recipe")?;
        assert_eq!(plan.outputs[0].0.production_key, bread);
        let typed = |key: &str| reference(key).and_then(|r| crate::gameplay_transport::item_look::typed_definition(index, r)).ok_or("cooking Item ref");
        let milled = oteryn_gameplay_items::item_craft::cooking_conversion(native, &typed("oteryn:item.tibia.i3605")?, &typed("oteryn:item.tibia.i1943")?).ok_or("milling recipe")?;
        assert_eq!(milled.outputs[0].0.production_key, "oteryn:item.tibia.i3603");
        let cookies = oteryn_gameplay_items::item_craft::cooking_conversion(native, &typed("oteryn:item.tibia.i8198")?, &station_definition).ok_or("cookies recipe")?;
        assert_eq!(cookies.outputs.len(), 2);
        assert_eq!(cookies.outputs[1].1, 12);
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let (end, result, payload) = exchange(&owner, admitted, 1, &payload, reference(dough)).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records").fetch_one(&harness.pool).await?, 1);
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
                .any(|e| e.item.definition.production_key == bread && e.item.quantity == 1)
        );
        // The immutable receipt and bounded outbox carry the same committed cooking proof.
        let (before, outputs, envelope, hash_matches): (serde_json::Value, serde_json::Value, Vec<u8>, bool) = sqlx::query_as(
            "SELECT r.before_item,r.outputs,a.envelope,a.envelope_sha256=sha256(a.envelope) AND a.envelope_sha256=r.envelope_sha256 FROM game_item_craft_records r JOIN game_item_audit_outbox a USING(event_id)",
        ).fetch_one(&harness.pool).await?;
        assert!(envelope.len() <= 9_216 && hash_matches);
        assert_eq!(before["quantity"], 3);
        assert_eq!(outputs[0]["key"], bread);
        // Reconstruct the owner and replay the original bytes without trusting current handles.
        drop(owner);
        let mut restarted = deps.owner(&harness, &authority);
        restarted.active_generation = Some(active);
        restarted.movement_cells = &cells;
        deps.initialize(&restarted).await;
        let (_, replayed, _) = exchange(&restarted, admitted, 1, &payload, None).await?;
        assert_eq!(replayed, UseDisposition::Committed);
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records").fetch_one(&harness.pool).await?, 1);
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
                .bind(id(42).as_slice()).bind(bread).bind(id(41).as_slice()).bind(id(70).as_slice()).bind(100_i64+n).execute(&mut *tx).await?;
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
        assert_eq!(old_slot, bread);
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

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_source: Option<std::num::NonZeroU32>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        let mut source = None;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) = oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                        let inventory = oteryn_protocol_oteryn::item_view::decode_character_inventory(domain.payload).map_err(debug)?;
                        source = inventory.entries.iter().find(|entry| Some(entry.item_definition_ref) == current_source).map(|entry| entry.handle);
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit { break; }
        }
        let actual = if current_source.is_some() {
            let mut intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(admitted.continuity.selected_capabilities.as_slice(), payload).map_err(debug)?;
            intent.target = UseTarget::Item(source.ok_or("current source handle")?);
            encode_item_use_intent(&intent).map_err(debug)?
        } else { payload.to_vec() };
        let frame = oteryn_protocol_oteryn::encode_client_command(1, &oteryn_protocol_oteryn::ClientCommandValue { command_id, command_type: 2, payload: &actual })?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        client.shutdown().await?;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command_id {
                    let disposition = oteryn_protocol_oteryn::world_object::decode_use_result(result.payload).map_err(debug)?;
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
    tokio::time::timeout(std::time::Duration::from_secs(30), std::future::poll_fn(|context| {
        if end.is_none() && let Poll::Ready(value) = serving.as_mut().poll(context) {
            end = Some(value);
        }
        if result.is_none() && let Poll::Ready(value) = client_task.as_mut().poll(context) {
            result = Some(value);
        }
        if end.is_some() && result.is_some() { Poll::Ready(()) } else { Poll::Pending }
    })).await?;
    let end = end.ok_or("server did not finish")?;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end else { return Err(format!("{end:?}").into()); };
    let (result, actual) = result.ok_or("client did not finish")??;
    Ok((admitted, result, actual))
}
