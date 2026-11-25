use super::fixture::{Harness, debug, fence, run};
use super::store_fixtures::{FakePlatform, Ledger, purchase_request, uuid};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::SelectedCapabilities,
    connection::{
        AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
    },
    tcp_tls::{read_frame, write_frame},
};
use crate::native_admission_source::store::StoreClient;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, MessageType, decode_command_result, decode_snapshot_chunk,
    decode_state_delta, decode_wire_envelope, encode_client_command, item_view, store as wire,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, DuplexStream};

const SWORD: &str = "store:exercise-sword";
const SELECTED: &[u32] = &[4, 6, 12, wire::CAPABILITY_STORE_V1];

#[test]
fn committed_store_claim_refreshes_live_inventory() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-claim-live-view", || {
        run(async |admin| {
            let harness =
                Harness::create_with_admission_seed(admin, "store_claim_live_view", true, false)
                    .await?;
            prepare_current_session(&harness).await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = fence(1)?.runtime_scope.world_id();
            let (controller, pin) =
                crate::content::native_gameplay::test_support::activated_with_loot_tables(
                    world, None,
                );
            let generation = controller.active().ok_or("active generation")?;
            let room = pin.qualified_room().clone();
            let frame_digest = pin.frame_binding().digest();
            let deps = Dependencies::with_content(
                pin.into_channel_pin(),
                room,
                crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            deps.initialize(&owner).await;
            owner
                .runtime
                .lock()
                .await
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
                harness.root.clone(),
            )
            .current_session_at(deps.admitted.game_session_id)
            .await?
            .0;
            let account: sqlx::types::Uuid =
                sqlx::query_scalar("SELECT account_id FROM game_character_roots")
                    .fetch_one(&harness.pool)
                    .await?;
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.item_fence =
                Some(crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?);
            admitted.controller.as_mut().ok_or("controller")?.account_id = *account.as_bytes();

            let ledger = Arc::new(Mutex::new(Ledger {
                balance: 60,
                world: uuid(admitted.world_id.as_bytes()),
                advertise_claims: true,
                ..Ledger::default()
            }));
            owner.store = Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(
                Box::new(FakePlatform(ledger.clone())),
                "oteryn-game/store-consumer".into(),
            ))));
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
                        .bind(admitted.world_id.as_bytes().as_slice())
                        .bind(admitted.channel_id.as_bytes().as_slice())
                        .execute(&harness.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                        .bind(admitted.world_id.as_bytes().as_slice())
                        .bind(admitted.channel_id.as_bytes().as_slice())
                        .bind(generation.identity().server_artifact_digest().as_slice())
                        .bind(generation.identity().client_artifact_digest().as_slice())
                        .bind(frame_digest.as_slice())
                        .execute(&harness.pool).await?;
            super::item_look_fixtures::seed_main_backpack(&harness, &[]).await?;
            sqlx::query(
                "INSERT INTO game_character_equipment_state(character_id,revision)
                         VALUES($1,1) ON CONFLICT(character_id) DO NOTHING",
            )
            .bind(sqlx::types::Uuid::from_bytes(
                *admitted
                    .item_fence
                    .ok_or("item fence")?
                    .character_id
                    .as_bytes(),
            ))
            .execute(&harness.pool)
            .await?;
            admitted.continuity.selected_capabilities =
                SelectedCapabilities::select(owner.offered_capabilities(), SELECTED)
                    .ok_or("Store and inventory selection")?;
            observe_claim(&owner, admitted).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await
        })
    })
}

async fn observe_claim(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let sword = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i28552", "definition-r1"))
        .ok_or("exercise sword definition")?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
    };
    let driving = async move {
        tokio::time::timeout(Duration::from_secs(4), async {
            let mut saw_empty_backpack = false;
            loop {
                let frame = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                match envelope.message_type() {
                    MessageType::SnapshotChunk => {
                        for domain in decode_snapshot_chunk(envelope.payload()).map_err(debug)?.1 {
                            if domain.domain_id == item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                                let inventory =
                                    item_view::decode_character_inventory_with_equip_drop(
                                        domain.payload,
                                        true,
                                    )
                                    .map_err(debug)?;
                                saw_empty_backpack = inventory.main_backpack.is_some()
                                    && inventory.entries.is_empty();
                            }
                        }
                    }
                    MessageType::SnapshotCommit => {
                        assert!(saw_empty_backpack);
                        return Ok::<_, Box<dyn std::error::Error>>(());
                    }
                    _ => {}
                }
            }
        })
        .await
        .map_err(|_| "initial inventory snapshot missing")??;

        let purchase_id = admitted.continuity.next_command_id;
        let (purchase_type, purchase) = purchase_request(80, SWORD, 25)?;
        send(&mut client, admitted, purchase_id, purchase_type, &purchase).await?;
        let delivery = tokio::time::timeout(Duration::from_secs(4), async {
            loop {
                let frame = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() != MessageType::CommandResult {
                    continue;
                }
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, purchase_id);
                assert_eq!(result.status, CommandStatus::Accepted);
                let purchase = wire::decode_purchase_result(result.payload).map_err(debug)?;
                assert_eq!(purchase.code, wire::StorePurchaseCode::Accepted);
                return Ok::<_, Box<dyn std::error::Error>>(
                    purchase.operation_id.ok_or("delivery")?,
                );
            }
        })
        .await
        .map_err(|_| "Store purchase reply missing")??;
        let claim = wire::encode_inbox_claim_request(&wire::StoreInboxClaimRequest {
            fulfilment_unit_id: uuid(&delivery),
        })
        .map_err(debug)?;
        let claim_id = purchase_id + 1;
        send(
            &mut client,
            admitted,
            claim_id,
            wire::COMMAND_TYPE_STORE_INBOX_CLAIM,
            &claim,
        )
        .await?;
        let mut committed = false;
        let mut saw_item = false;
        let observed = tokio::time::timeout(Duration::from_secs(3), async {
            while !saw_item {
                let frame = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, claim_id);
                    assert_eq!(result.status, CommandStatus::Accepted);
                    let receipt = wire::decode_claim_receipt(result.payload).map_err(debug)?;
                    assert_eq!(receipt.result, wire::StoreClaimCode::Committed);
                    committed = true;
                }
                if envelope.message_type() == MessageType::StateDelta {
                    let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                    if delta.domain_id == item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                        let inventory = item_view::decode_character_inventory_with_equip_drop(
                            delta.payload,
                            true,
                        )
                        .map_err(debug)?;
                        saw_item = inventory
                            .entries
                            .iter()
                            .any(|item| item.item_definition_ref == sword);
                    }
                }
            }
            Ok::<_, Box<dyn std::error::Error>>(())
        })
        .await;
        assert!(committed, "command 186 must commit the real claim");
        assert!(
            observed.is_ok(),
            "committed claim must publish the claimed item to live inventory"
        );
        observed??;
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let ((), result) = tokio::time::timeout(Duration::from_secs(12), async {
        tokio::join!(serving, driving)
    })
    .await?;
    result
}

async fn send(
    client: &mut DuplexStream,
    admitted: AdmittedSession,
    command_id: u64,
    command_type: u32,
    payload: &[u8],
) -> TestResult {
    let frame = encode_client_command(
        admitted.continuity.connection_generation,
        &ClientCommandValue {
            command_id,
            command_type,
            payload,
        },
    )
    .map_err(debug)?;
    write_frame(client, &frame).await?;
    Ok(())
}
