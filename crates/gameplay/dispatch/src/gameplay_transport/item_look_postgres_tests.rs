//! LOOK-2 through the real composed owner, PostgreSQL admission and `serve_admitted`.
//! Nested under monk-save tests to share their fresh-admission/recovery fixture.

use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::ReferenceItemField;
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, MessageType, NodeId, decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::{
    COMMAND_TYPE_ITEM_LOOK_INTENT, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use crate::map::boot::tests::bundle_with_item_ids;
use crate::map::boot::{BootPins, boot};
use crate::map::facts::ItemDefinition;
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::item_look::{
    ItemLookIntent, ItemLookRefusal, ItemLookResult, ItemLookSelection, ItemLookTarget,
    decode_item_look_result_for, encode_item_look_intent_for,
};
use oteryn_protocol_oteryn::item_view::{
    STATE_DOMAIN_CHARACTER_INVENTORY, decode_character_inventory,
};
use oteryn_protocol_oteryn::world_map::{
    MapOrigin, STATE_DOMAIN_WORLD_MAP_VIEW, decode_world_map_snapshot,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, decode_snapshot_chunk,
    encode_client_command,
};
use std::fmt::Write;
use std::future::Future;
use std::task::Poll;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio::sync::Mutex;

const EXTENDED: ItemLookSelection = ItemLookSelection { extended: true };

async fn read_frame(client: &mut DuplexStream) -> TestResult<Vec<u8>> {
    let length = client.read_u32().await?;
    let mut frame = vec![0; usize::try_from(length)?];
    client.read_exact(&mut frame).await?;
    Ok(frame)
}

async fn client_looks(client: &mut DuplexStream, own: EntityRef) -> TestResult {
    let mut backpack = None;
    let mut map_item = None;
    loop {
        let frame = read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == MessageType::SnapshotChunk {
            let (_, domains) = decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
            for domain in domains {
                if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                    let inventory = decode_character_inventory(domain.payload).map_err(debug)?;
                    backpack = inventory.main_backpack.map(|item| item.handle);
                } else if domain.domain_id == STATE_DOMAIN_WORLD_MAP_VIEW {
                    let map = decode_world_map_snapshot(domain.payload).map_err(debug)?;
                    map_item = map
                        .tiles
                        .iter()
                        .flat_map(|tile| &tile.items)
                        .find_map(|item| {
                            if let MapOrigin::Handle(handle) = item.origin {
                                Some(handle)
                            } else {
                                None
                            }
                        });
                }
            }
        }
        if envelope.message_type() == MessageType::SnapshotCommit {
            break;
        }
    }
    let backpack = backpack.ok_or("domain 9 carries the main backpack handle")?;
    let map_item = map_item.ok_or("domain 17 carries a map item handle")?;
    for (offset, target) in [
        ItemLookTarget::Creature(own),
        ItemLookTarget::Item(backpack),
        ItemLookTarget::Creature(EntityRef {
            identity: id(99),
            generation: 1,
        }),
        ItemLookTarget::Item(map_item),
    ]
    .into_iter()
    .enumerate()
    {
        let command_id = u64::try_from(offset)? + 1;
        let payload =
            encode_item_look_intent_for(ItemLookIntent { target }, EXTENDED).map_err(debug)?;
        let frame = encode_client_command(
            1,
            &ClientCommandValue {
                command_id,
                command_type: COMMAND_TYPE_ITEM_LOOK_INTENT,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        loop {
            let frame = read_frame(client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() != MessageType::CommandResult {
                continue;
            }
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.command_id, command_id);
            assert_eq!(result.status, CommandStatus::Accepted);
            let looked = decode_item_look_result_for(result.payload, EXTENDED).map_err(debug)?;
            match offset {
                0 => assert_eq!(
                    looked,
                    ItemLookResult::Text("You see yourself (Level 50, monk).".into())
                ),
                1 => assert!(
                    matches!(looked, ItemLookResult::Text(text) if text.starts_with("You see "))
                ),
                2 => assert_eq!(looked, ItemLookResult::Refused(ItemLookRefusal::NotVisible)),
                3 => assert!(
                    matches!(looked, ItemLookResult::Text(text) if text.starts_with("You see "))
                ),
                _ => unreachable!(),
            }
            break;
        }
    }
    client.shutdown().await?;
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    Ok(())
}

async fn serve_looks(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    own: EntityRef,
) -> TestResult {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        assert!(
            matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
            "{end:?}"
        );
        drop(server);
    };
    let mut serving = std::pin::pin!(serving);
    let mut looking = std::pin::pin!(client_looks(&mut client, own));
    let mut served = false;
    let mut result = None;
    std::future::poll_fn(|context| {
        if !served && serving.as_mut().poll(context).is_ready() {
            served = true;
        }
        if result.is_none()
            && let Poll::Ready(value) = looking.as_mut().poll(context)
        {
            result = Some(value);
        }
        if served && result.is_some() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
    result.ok_or("LOOK client did not finish")?
}

#[test]
fn item_look_postgres_composed_owner_serves_self_backpack_and_hidden_creature() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "the LOOK PostgreSQL e2e thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let harness = Harness::create_with_admission_seed(admin, "item_look", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let mut admitted = dependencies.admitted;
        let (controller, pin) = activated_with_loot_tables(admitted.world_id, None);
        let active = controller.active().ok_or("activated generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let index = native.item_index().ok_or("item index")?;
        let reference = |key: &str| {
            index
                .definition_ref("Item", key, "definition-r1")
                .ok_or("fixture item belongs to the activated index")
        };
        let (bytes, pins) = bundle_with_item_ids(
            (
                "oteryn:item.tibia.i2854",
                reference("oteryn:item.tibia.i2854")?.get() - 1,
            ),
            (
                "oteryn:item.tibia.i3607",
                reference("oteryn:item.tibia.i3607")?.get() - 1,
            ),
        );
        let mut map_revision = String::from("sha256:");
        for byte in &pins.digest {
            write!(&mut map_revision, "{byte:02x}")?;
        }
        let bundle = boot(
            &bytes,
            &BootPins {
                bundle: pins,
                map_revision,
                start: TilePos {
                    x: 2,
                    y: 0,
                    floor: -7,
                },
            },
            admitted.world_id,
            admitted.channel_id,
            |key| {
                let profile = native.item_policy(key, "definition-r1")?.record();
                let ReferenceItemField::Known(physical) = &profile.semantics.physical else {
                    return None;
                };
                Some(ItemDefinition {
                    reference: reference(key).ok()?,
                    solid: profile.attributes.blocks_movement,
                    blocks_projectile: profile.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable == ReferenceItemField::Known(true),
                    door: false,
                })
            },
        )?;
        let cells = bundle
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let channel_pin = ChannelContentPin::from_activation(
            admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (2, 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            admitted.world_id,
            admitted.channel_id,
            NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            channel_pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(admitted.game_session_id)
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        let player = runtime.visible_entities().players[0];
        let own = EntityRef {
            identity: actor.placement_identity(),
            generation: player.generation,
        };
        let runtime = Mutex::new(runtime);
        let mut owner = dependencies.owner(&harness, &authority);
        owner.runtime = &runtime;
        owner.movement_cells = &cells;
        owner.active_generation = Some(active);
        dependencies.initialize(&owner).await;
        assert!(
            owner
                .offered_capabilities()
                .iter()
                .any(|capability| capability.id == 28)
        );
        admitted.runtime_actor = Some(actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 18, 28])
                .ok_or("4, 6, 18 and 28 are selectable")?;
        let (current, _) = FreshAdmissionStore::from_root(harness.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?;
        admitted.item_fence = item_fence_of(current);

        // Seed the migration-owned fixture as the existing item PostgreSQL harnesses do.
        // All observations below use the real durable owner under its recovery fence.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
            definition_production_key, definition_revision_ref, quantity, lifecycle, minted_transaction_id) \
            VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 'Item', 'oteryn:item.tibia.i2854', \
            'definition-r1', 1, 1, encode($3,'hex')::uuid)")
            .bind(id(70).as_slice()).bind(id(42).as_slice()).bind(id(71).as_slice())
            .execute(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO game_item_container_slots(character_id, item_instance_id, world_id, \
            placed_transaction_id) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, \
            encode($3,'hex')::uuid, encode($4,'hex')::uuid)",
        )
        .bind(id(41).as_slice())
        .bind(id(70).as_slice())
        .bind(id(42).as_slice())
        .bind(id(72).as_slice())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        tokio::time::timeout(
            std::time::Duration::from_secs(30),
            serve_looks(&owner, admitted, own),
        )
        .await??;
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
