//! IMBUE-WIRE-1: command 85 -> real composed dispatch -> durable item owner read.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::imbuement::*;
use oteryn_protocol_oteryn::item_view::*;
use tokio::io::AsyncWriteExt;

const SWORD: &str = "oteryn:item.tibia.i3264";

#[test]
fn imbuement_wire_dispatch_reads_the_carried_item_and_fails_closed_without_writes() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "imbuement wire e2e panic")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "imbuement_wire", true, false).await?;
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
        let coin = "oteryn:item.tibia.i3031";
        let bag = "oteryn:item.tibia.i2854";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (coin, reference(coin).ok_or("oven")?.get() - 1),
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
        super::item_look::seed_main_backpack(&harness, &[(SWORD, 1), (coin, 100)]).await?;
        seed_imbuement(&harness).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 12, 14, 18, CAPABILITY_IMBUE_V1],
        )
        .ok_or("imbuement capability selectable")?;
        let before = snapshot(&harness).await?;
        // The carried sword is read through the durable owner with its stored imbuement;
        // the qualified native policy declares no imbuement semantics, so the shrine
        // window, apply and clear are all the same NOT_ALLOWED and nothing is written.
        let mut next = admitted;
        for (command, action) in [
            (1, Some(ImbuementAction::Query)),
            (
                2,
                Some(ImbuementAction::Apply {
                    slot: 1,
                    imbuement: 0,
                }),
            ),
            (3, Some(ImbuementAction::Clear { slot: 0 })),
        ] {
            let (end, status, reply) = exchange(&owner, next, command, action, true).await?;
            assert_eq!(status, CommandStatus::Accepted, "command {command}");
            assert_eq!(
                reply,
                Some(ImbuementReply {
                    result: ImbuementResult::NotAllowed,
                    window: None,
                }),
                "command {command}"
            );
            next = end;
        }
        // A handle that names no current item is the same NOT_ALLOWED.
        let (end, status, reply) = exchange(&owner, next, 4, None, true).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            reply.map(|reply| reply.result),
            Some(ImbuementResult::NotAllowed)
        );
        next = end;
        // A malformed payload and a session without capability 68 are rejected.
        let (end, status, _) = exchange(&owner, next, 5, None, false).await?;
        assert_eq!(status, CommandStatus::Rejected);
        let mut legacy = end;
        legacy.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 12, 14, 18])
                .ok_or("legacy selection")?;
        let (_, status, _) =
            exchange(&owner, legacy, 6, Some(ImbuementAction::Query), true).await?;
        assert_eq!(status, CommandStatus::Rejected);
        assert_eq!(snapshot(&harness).await?, before);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Imbuement rows, item quantities and lifecycles: no command may change any of them.
async fn snapshot(harness: &Harness) -> TestResult<(Vec<(i16, i16, i64)>, Vec<(i64, i16)>)> {
    let imbuements = sqlx::query_as(
        "SELECT slot, tier, remaining_ms FROM game_item_imbuements ORDER BY item_instance_id, slot",
    )
    .fetch_all(&harness.pool)
    .await?;
    let items = sqlx::query_as(
        "SELECT quantity, lifecycle FROM game_item_instances ORDER BY item_instance_id",
    )
    .fetch_all(&harness.pool)
    .await?;
    Ok((imbuements, items))
}

/// Only fixture construction bypasses triggers, as in `seed_main_backpack`: one stored
/// imbuement on the sword (entry id 71) in slot 0.
async fn seed_imbuement(harness: &Harness) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_imbuements(item_instance_id,slot,category,imbuement_key,imbuement_revision,tier,remaining_ms,applying_occurrence_id,applying_transaction_id) VALUES(encode($1,'hex')::uuid,0,10,'oteryn:imbuement.fixture','r1',1,1000,game_character_uuid_v7(),game_character_uuid_v7())")
        .bind(id(71).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    action: Option<ImbuementAction>,
    well_formed: bool,
) -> TestResult<(AdmittedSession, CommandStatus, Option<ImbuementReply>)> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        let mut sword = None;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                        let inventory =
                            decode_character_inventory_with_equip_drop(domain.payload, true)
                                .map_err(debug)?;
                        sword = inventory
                            .entries
                            .iter()
                            .find(|entry| entry.count.get() == 1)
                            .map(|entry| entry.handle);
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit {
                break;
            }
        }
        let payload = if well_formed {
            let item = match action {
                Some(_) => sword.ok_or("carried sword")?,
                None => ItemHandle::new(u64::MAX).ok_or("handle")?,
            };
            encode_imbuement_request(&ImbuementRequest {
                item,
                action: action.unwrap_or(ImbuementAction::Query),
            })
            .map_err(debug)?
        } else {
            vec![8, 4, 16, 1]
        };
        send(&mut client, command_id, COMMAND_TYPE_IMBUEMENT, &payload).await?;
        client.shutdown().await?;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command_id {
                    let reply = (result.status == CommandStatus::Accepted)
                        .then(|| decode_imbuement_reply(result.payload))
                        .transpose()
                        .map_err(debug)?;
                    return Ok::<_, Box<dyn std::error::Error>>((result.status, reply));
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
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end.ok_or("server did not finish")?
    else {
        return Err("server ended before admission".into());
    };
    let (status, reply) = result.ok_or("client did not finish")??;
    Ok((admitted, status, reply))
}

async fn send(
    client: &mut tokio::io::DuplexStream,
    command_id: u64,
    command_type: u32,
    payload: &[u8],
) -> TestResult {
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type,
            payload,
        },
    )?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    Ok(())
}
