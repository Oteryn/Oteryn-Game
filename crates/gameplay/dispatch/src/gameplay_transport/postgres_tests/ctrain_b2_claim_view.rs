use super::fixture::{Harness, debug, fence, run};
use super::store_fixtures::{
    FakePlatform, Ledger, purchase_request, uuid, wait_for_advisory_waiter,
};
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
    claim_view_case(ClaimViewCase::Regular)
}

#[test]
fn store_inbox_can_claim_a_legacy_twenty_first_delivery() -> TestResult {
    claim_view_case(ClaimViewCase::LegacyPage)
}

#[test]
fn committed_store_claim_survives_contention_after_commit() -> TestResult {
    claim_view_case(ClaimViewCase::Contended)
}

#[derive(Clone, Copy)]
enum ClaimViewCase {
    Regular,
    LegacyPage,
    Contended,
}

fn claim_view_case(case: ClaimViewCase) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-claim-live-view", move || {
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
            observe_claim(&owner, admitted, &harness, case).await?;
            eprintln!("STORE_CLAIM_VIEW POSTGRES_CASE_COMPLETED");
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
    harness: &Harness,
    case: ClaimViewCase,
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
        let mut claim_id = purchase_id + 1;
        if matches!(case, ClaimViewCase::LegacyPage) {
            claim_id =
                page_legacy_inbox(&mut client, admitted, harness, delivery, claim_id).await?;
        }
        let barrier = if matches!(case, ClaimViewCase::Contended) {
            Some(block_claim_commit(harness).await?)
        } else {
            None
        };
        let claim = wire::encode_inbox_claim_request(&wire::StoreInboxClaimRequest {
            fulfilment_unit_id: uuid(&delivery),
        })
        .map_err(debug)?;
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
        let (receipt_seen, release) = tokio::sync::oneshot::channel();
        let mut receipt_seen = Some(receipt_seen);
        let receive = async {
            while !saw_item {
                let frame = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, claim_id);
                    if let Some(receipt_seen) = receipt_seen.take() {
                        let _ = receipt_seen.send(());
                    }
                    assert_eq!(
                        result.status,
                        CommandStatus::Accepted,
                        "the committed claim must survive a busy proof holder"
                    );
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
        };
        let contender = async {
            let Some((barrier, pid)) = barrier else {
                return Ok::<_, Box<dyn std::error::Error>>(());
            };
            crate::durability::DurabilityRoot::with_holder_owner(async {
                wait_for_advisory_waiter(&harness.pool, pid).await?;
                let ticket = harness.root.holder_waiter().ok_or("proof holder ticket")?;
                barrier.rollback().await?;
                let now = tokio::time::Instant::now();
                assert!(
                    harness
                        .root
                        .wait_holder_turn(now, now + Duration::from_secs(2))
                        .await
                );
                let pass = harness.root.try_issue_semantic_pass()?;
                drop(ticket);
                pass.run(move |_, _| {
                    Box::pin(async move {
                        let _ = release.await;
                        Ok(())
                    })
                })
                .await?;
                Ok(())
            })
            .await
        };
        let observed = tokio::time::timeout(Duration::from_secs(4), async {
            tokio::try_join!(receive, contender).map(|_| ())
        })
        .await;
        assert!(committed, "command 186 must commit the real claim");
        assert!(
            observed.is_ok(),
            "committed claim must publish the claimed item to live inventory"
        );
        observed??;
        if matches!(case, ClaimViewCase::LegacyPage) {
            let remaining: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_item_character_inbox_locations")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(remaining, 20, "the first page must remain unclaimed");
        }
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

async fn block_claim_commit(
    harness: &Harness,
) -> TestResult<(sqlx::Transaction<'_, sqlx::Postgres>, i32)> {
    sqlx::raw_sql(
        "CREATE FUNCTION test_hold_store_claim_commit() RETURNS trigger LANGUAGE plpgsql AS
         'BEGIN PERFORM pg_advisory_xact_lock(1863291); RETURN NEW; END';
         CREATE CONSTRAINT TRIGGER test_hold_store_claim_commit
         AFTER INSERT ON game_depot_search_retrieve_receipts DEFERRABLE INITIALLY DEFERRED
         FOR EACH ROW EXECUTE FUNCTION test_hold_store_claim_commit()",
    )
    .execute(&harness.pool)
    .await?;
    let mut barrier = harness.pool.begin().await?;
    let pid = sqlx::query_scalar("SELECT pg_backend_pid() FROM pg_advisory_xact_lock(1863291)")
        .fetch_one(&mut *barrier)
        .await?;
    Ok((barrier, pid))
}

async fn page_legacy_inbox(
    client: &mut DuplexStream,
    admitted: AdmittedSession,
    harness: &Harness,
    delivery: [u8; 16],
    first_id: u64,
) -> TestResult<u64> {
    // The pre-cap Store writer permitted a character to retain more than 20 deliveries.
    // Seed only the unaffected rows. The delivery we claim has a real purchase and mint proof.
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    for n in 1..=20 {
        let id = sqlx::types::Uuid::parse_str(&format!("00000000-0000-7000-8000-{n:012x}"))?;
        sqlx::query(
            "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,
               definition_revision_ref,quantity,lifecycle,minted_transaction_id)
             SELECT $1, world_id, 'Item', 'oteryn:item.tibia.i28552', 'definition-r1', 1, 1, $1
             FROM store_delivery_receipts WHERE operation_id=$2",
        ).bind(id).bind(sqlx::types::Uuid::from_bytes(delivery)).execute(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO game_item_character_inbox_locations(item_instance_id,world_id,character_id,ordinal)
             SELECT $1,world_id,character_id,$3 FROM store_delivery_receipts WHERE operation_id=$2",
        ).bind(id).bind(sqlx::types::Uuid::from_bytes(delivery)).bind(1000+n).execute(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO store_delivery_receipts(operation_id,account_id,character_id,world_id,
               fulfilment_unit_id,result,recovery_generation,decided_at,item_instance_id,receipt)
             SELECT $1,account_id,character_id,world_id,$1::uuid::text,result,recovery_generation,
               decided_at,$1,receipt FROM store_delivery_receipts WHERE operation_id=$2",
        )
        .bind(id)
        .bind(sqlx::types::Uuid::from_bytes(delivery))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    let kind = wire::COMMAND_TYPE_STORE_INBOX_CLAIM;
    send(client, admitted, first_id, kind, wire::STORE_INBOX_QUERY).await?;
    let (status, first) = inbox_result(client, first_id).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let first = wire::decode_inbox_entries(&first).map_err(debug)?;
    assert_eq!(first.len(), 20);
    assert!(
        !first
            .iter()
            .any(|entry| entry.fulfilment_unit_id == delivery)
    );
    let last = first.last().ok_or("first page")?;
    let mut next = vec![16, 2, 26, 16];
    next.extend(last.fulfilment_unit_id);
    send(client, admitted, first_id + 1, kind, &next).await?;
    let (status, page) = inbox_result(client, first_id + 1).await?;
    assert_eq!(
        status,
        CommandStatus::Accepted,
        "the 21st delivery needs a selectable Store claim page"
    );
    let second = wire::decode_inbox_entries(&page).map_err(debug)?;
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].fulfilment_unit_id, delivery);
    Ok(first_id + 2)
}

async fn inbox_result(
    client: &mut DuplexStream,
    command_id: u64,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let frame = read_frame(client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, command_id);
                return Ok((result.status, result.payload.to_vec()));
            }
        }
    })
    .await?
}
