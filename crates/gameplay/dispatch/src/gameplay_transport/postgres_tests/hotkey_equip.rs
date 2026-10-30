//! Real command routing, fenced custody, replay and durable equipment reload.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, item_look, prepare_current_session};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_protocol_oteryn::{self as protocol, item_view::ItemMoveOutcome};
use tokio::io::AsyncWriteExt;

#[test]
fn hotkey_equip_postgres_dispatch_toggle_replay_and_durable_reload() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-equip", || scenario(false))
}

#[test]
fn hotkey_equip_postgres_worn_item_toggles_with_spare_carried() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-equip-spare", || scenario(true))
}

fn scenario(spare: bool) -> TestResult {
    scenario_options(spare, false)
}

#[test]
fn hotkey_equip_postgres_feared_refuses_without_mutation() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("hotkey-equip-feared", || {
        scenario_options(false, true)
    })
}

fn scenario_options(spare: bool, feared: bool) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "hotkey_equip", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("active")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let reference = native
            .item_index()
            .ok_or("index")?
            .definition_ref("Item", "oteryn:item.tibia.i3264", "definition-r1")
            .ok_or("sword ref")?;
        let room = pin.qualified_room().clone();
        let frame_digest = pin.frame_binding().digest();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;
        if feared {
            crate::gameplay_transport::hotkey_equip::tests::apply_status(
                &mut *deps.runtime.lock().await,
                deps.actor,
                deps.admitted.game_session_id,
                crate::foundation::StatusKind::Feared,
                owner.owner_now(),
            )?;
        }
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(frame_digest.as_slice()).execute(&harness.pool).await?;
        let mut entries = vec![("oteryn:item.tibia.i3264", 1)];
        if spare {
            entries.push(("oteryn:item.tibia.i3264", 1));
        }
        item_look::seed_main_backpack(&harness, &entries).await?;
        let mut admitted = deps.admitted;
        admitted.item_fence = Some(fence);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, 12, 14, 179],
        )
        .ok_or("capabilities")?;
        assert!(admitted.continuity.selected_capabilities.contains(179));
        let request = protocol::hotkey_equip::Request {
            definition: reference,
        }
        .encode();
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let serve = async {
            crate::gameplay_transport::connection::serve_admitted(
                &mut server,
                admitted,
                &owner,
                crate::gameplay_transport::connection::IDLE_LIVENESS,
            )
            .await
        };
        let pool = &harness.pool;
        let exercise = async move {
            loop {
                let frame = item_look::read_frame(&mut client).await?;
                let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == protocol::MessageType::SnapshotCommit {
                    break;
                }
            }
            if feared {
                assert_eq!(
                    send(&mut client, 1, &request, protocol::CommandStatus::Accepted).await?,
                    ItemMoveOutcome::Rejected
                );
                assert_eq!(equipped_count(pool).await?, 0);
                let receipts: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_receipts")
                        .fetch_one(pool)
                        .await?;
                assert_eq!(receipts, 0);
                client.shutdown().await?;
                return Ok(());
            }
            assert_eq!(
                send(
                    &mut client,
                    1,
                    &[0, 0, 0, 0],
                    protocol::CommandStatus::Accepted
                )
                .await?,
                ItemMoveOutcome::Rejected
            );
            assert_eq!(
                send(&mut client, 2, &request, protocol::CommandStatus::Accepted).await?,
                ItemMoveOutcome::Moved
            );
            assert_eq!(equipped_count(pool).await?, 1);
            // Foundation replay must not select the now-worn item and toggle it back.
            assert_eq!(
                send(
                    &mut client,
                    2,
                    &request,
                    protocol::CommandStatus::DuplicateReplay
                )
                .await?,
                ItemMoveOutcome::Moved
            );
            assert_eq!(equipped_count(pool).await?, 1);
            assert_eq!(
                send(&mut client, 3, &request, protocol::CommandStatus::Accepted).await?,
                ItemMoveOutcome::Moved
            );
            assert_eq!(equipped_count(pool).await?, 0);
            assert_eq!(
                send(&mut client, 4, &request, protocol::CommandStatus::Accepted).await?,
                ItemMoveOutcome::Moved
            );
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(pool).await?;
            assert_eq!(receipts, 3);
            client.shutdown().await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        };
        tokio::time::timeout(std::time::Duration::from_secs(45), drive(serve, exercise)).await??;
        // A newly composed owner reads the stored item by identity, independent of the wire view.
        let mut reloaded = deps.owner(&harness, &authority);
        reloaded.active_generation = Some(generation);
        let equipment = crate::gameplay_transport::equipment_move::read_current_equipment(
            &reloaded,
            admitted.game_session_id,
        )
        .await
        .ok_or("reloaded equipment")?;
        if feared {
            assert!(equipment.items.is_empty());
        } else {
            assert_eq!(equipment.items.len(), 1);
            assert_eq!(
                equipment.items[0].item_instance_id,
                if spare { id(72) } else { id(71) }
            );
            assert_eq!(equipment.items[0].slot, 5);
        }
        let quantity: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3264' AND lifecycle=1").fetch_one(&harness.pool).await?;
        assert_eq!(quantity, if spare { 2 } else { 1 });
        if spare {
            let untouched: String = sqlx::query_scalar("SELECT state_revision::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(untouched, "1");
            let receipts_for_worn: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_receipts WHERE item_instance_id=encode($1,'hex')::uuid AND swap_item_instance_id IS NULL")
                .bind(id(72).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(receipts_for_worn, 3);
        }
        drop(reloaded);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn equipped_count(pool: &sqlx::PgPool) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(pool).await?)
}

async fn send(
    client: &mut tokio::io::DuplexStream,
    id: u64,
    payload: &[u8],
    expected_status: protocol::CommandStatus,
) -> TestResult<ItemMoveOutcome> {
    let frame = protocol::encode_client_command(
        1,
        &protocol::ClientCommandValue {
            command_id: id,
            command_type: protocol::hotkey_equip::COMMAND_TYPE_HOTKEY_EQUIP,
            payload,
        },
    )
    .map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    loop {
        let frame = item_look::read_frame(client).await?;
        let e = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        if e.message_type() == protocol::MessageType::CommandResult {
            let r = protocol::decode_command_result(e.payload()).map_err(debug)?;
            let outcome =
                protocol::item_view::decode_item_move_result_with_equip_drop(r.payload, true)
                    .map_err(debug)?;
            assert_eq!(r.status, expected_status);
            return Ok(outcome);
        }
    }
}

async fn drive<A, B>(
    serve: A,
    exercise: B,
) -> TestResult<crate::gameplay_transport::connection::ConnectionEnd>
where
    A: std::future::Future<Output = crate::gameplay_transport::connection::ConnectionEnd>,
    B: std::future::Future<Output = TestResult>,
{
    use std::task::Poll;
    let mut serve = std::pin::pin!(serve);
    let mut exercise = std::pin::pin!(exercise);
    let mut result = None;
    std::future::poll_fn(|cx| {
        if result.is_none()
            && let Poll::Ready(value) = exercise.as_mut().poll(cx)
        {
            if let Err(error) = value {
                return Poll::Ready(Err(error));
            }
            result = Some(());
        }
        if let Poll::Ready(end) = serve.as_mut().poll(cx) {
            return Poll::Ready(if result.is_some() {
                Ok(end)
            } else {
                Err("connection ended before hotkey exercise".into())
            });
        }
        Poll::Pending
    })
    .await
}
