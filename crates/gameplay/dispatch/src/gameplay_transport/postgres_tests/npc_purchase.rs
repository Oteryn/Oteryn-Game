//! NPC-BUY-BACKPACKS-1: named acceptance cases through serve_admitted and real PostgreSQL.
use super::fixture::{self, Harness, debug, id, run};
use super::item_look::seed_main_backpack;
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::{
    NpcDataCatalogue, ProjectV2DefinitionRef, ProjectV2Family, ProjectV2ServiceOffer,
    ProjectV2ServiceOfferDirection, npc_catalogue_preproduction_limits,
};
use crate::foundation::{
    MessageType, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
    decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::npc_talk::{NpcTalkBook, NpcTalkEntry};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::npc_purchase::*;
use oteryn_protocol_oteryn::npc_service::{
    COMMAND_TYPE_NPC_TALK_INTENT, NpcTalkIntent, NpcTradeIntent, NpcTradeSide,
    encode_npc_talk_intent,
};
use oteryn_protocol_oteryn::quick_loot::{
    COMMAND_TYPE_QUICK_LOOT, QuickLootAction, QuickLootCategory, QuickLootRecipient,
    QuickLootRequest, encode_quick_loot_request,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

const TRADER: &str = "oteryn:npc.alice";
const RASHID: &str = "oteryn:npc.rashid";
#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Bags,
    Bank,
    Bundle,
    Currency,
    CurrencyGoldMissing,
    CurrencyTokensMissing,
    Stackable,
    Unfunded,
    Obtain,
    Separate,
    Drop,
    Capacity,
    Compat,
}

#[test]
fn npc_buy_in_backpacks_uses_bank_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(move || scenario(admin, Case::Bank).map_err(|error| error.to_string()))?
        .join().map_err(|_| "bank purchase scenario thread panicked")??;
    Ok(())
}

#[test]
fn npc_buy_non_gold_currency_without_tokens_refuses_atomically() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(move || scenario(admin, Case::CurrencyTokensMissing).map_err(|error| error.to_string()))?
        .join().map_err(|_| "bank purchase scenario thread panicked")??;
    Ok(())
}

#[test]
fn npc_buy_non_gold_currency_without_bag_gold_refuses_atomically() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(move || scenario(admin, Case::CurrencyGoldMissing).map_err(|error| error.to_string()))?
        .join().map_err(|_| "bank purchase scenario thread panicked")??;
    Ok(())
}

#[test]
fn npc_buy_non_gold_currency_and_gold_bag_fee_atomically() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(move || scenario(admin, Case::Currency).map_err(|error| error.to_string()))?
        .join().map_err(|_| "bank purchase scenario thread panicked")??;
    Ok(())
}

#[test]
fn npc_buy_bundled_offer_prices_bundles_and_mints_all_units() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(move || scenario(admin, Case::Bundle).map_err(|error| error.to_string()))?
        .join().map_err(|_| "bank purchase scenario thread panicked")??;
    Ok(())
}

#[test]
fn npc_buy_in_backpacks_charges_bags_and_packs_twenty_per_bag() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin, Case::Bags).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "NPC purchase scenario thread panicked")??;
    Ok(())
}

#[test]
fn npc_buy_in_backpacks_stackable_bag_count() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin, Case::Stackable).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "NPC purchase scenario thread panicked")??;
    Ok(())
}

#[test]
fn npc_buy_in_backpacks_unfunded_for_bags_refuses_atomically() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin, Case::Unfunded).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "NPC purchase scenario thread panicked")??;
    Ok(())
}

#[test]
fn npc_buy_routes_to_obtain_container_then_default_then_main() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin, Case::Obtain).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "NPC purchase scenario thread panicked")??;
    Ok(())
}

#[test]
fn obtain_container_set_clear_is_separate_from_loot_container() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin, Case::Separate).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "NPC purchase scenario thread panicked")??;
    Ok(())
}

#[test]
fn npc_buy_ignore_cap_drops_overflow_under_player() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin, Case::Drop).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "NPC purchase scenario thread panicked")??;
    Ok(())
}

#[test]
fn npc_buy_without_ignore_cap_refuses_over_capacity() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin, Case::Capacity).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "NPC purchase scenario thread panicked")??;
    Ok(())
}

#[test]
fn npc_buy_old_capability_client_keeps_legacy_route() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin, Case::Compat).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "compat scenario thread panicked")??;
    Ok(())
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    greet: Option<EntityRef>,
    command_id: u64,
    kind: u32,
    payload: Vec<u8>,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let mut commands = Vec::new();
    if let Some(npc) = greet {
        // Each reconnect greets twice; respect the existing four-talks-per-second gate.
        tokio::time::sleep(std::time::Duration::from_millis(1050)).await;
        for (talk_id, text) in [(command_id - 2, "hi"), (command_id - 1, "trade")] {
            let talk = NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            };
            commands.push((
                talk_id,
                COMMAND_TYPE_NPC_TALK_INTENT,
                encode_npc_talk_intent(&talk).map_err(debug)?,
            ));
        }
    }
    commands.push((command_id, kind, payload));
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving: std::pin::Pin<Box<dyn std::future::Future<Output = ConnectionEnd> + '_>> =
        Box::pin(async move {
            let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
            drop(server);
            ended
        });
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let mut reply = None;
        for (sent_id, command_type, payload) in &commands {
            write_frame(
                &mut client,
                &encode_client_command(
                    before.continuity.connection_generation,
                    &ClientCommandValue {
                        command_id: *sent_id,
                        command_type: *command_type,
                        payload,
                    },
                )
                .map_err(debug)?,
            )
            .await?;
            loop {
                let frame = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, *sent_id);
                    if *sent_id == command_id {
                        reply = Some((result.status, result.payload.to_vec()));
                    } else {
                        assert_eq!(result.status, CommandStatus::Accepted, "talk {sent_id}");
                        assert_eq!(
                            oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(
                                result.payload
                            )
                            .map_err(debug)?,
                            oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok,
                            "talk {sent_id} must open the current NPC window",
                        );
                    }
                    break;
                }
            }
        }
        let reply = reply.ok_or("shop result")?;
        client.shutdown().await?;
        Ok(reply)
    };
    let (ended, result): (_, TestResult<(CommandStatus, Vec<u8>)>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &ended
    {
        *admitted = *current;
    }
    result.map_err(|error| format!("shop command {command_id} ended {ended:?}: {error}").into())
}

async fn footprint(h: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_npc_trades),(SELECT count(*) FROM game_npc_trade_lines),\
         (SELECT count(*) FROM game_item_audit_outbox),\
         (SELECT count(*) FROM game_item_instances WHERE lifecycle=1)",
    )
    .fetch_one(&h.pool)
    .await?)
}

fn uuid_bytes(value: &str) -> TestResult<[u8; 16]> {
    let value = value.replace('-', "");
    if value.len() != 32 {
        return Err("fixture UUID length".into());
    }
    let mut bytes = [0; 16];
    for (index, part) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        bytes[index] = u8::from_str_radix(std::str::from_utf8(part)?, 16)?;
    }
    Ok(bytes)
}
async fn gold(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity*game_item_coin_worth(i.definition_production_key)),0)::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1").bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
async fn last_parent(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT o.parent_id::text FROM game_npc_purchase_outputs o JOIN game_npc_trades t USING(transaction_id) WHERE NOT o.is_bag ORDER BY t.command_id DESC,o.purchase_index LIMIT 1").fetch_one(&h.pool).await?)
}
async fn set_target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    action: ObtainAction,
    category: Option<QuickLootCategory>,
    container: Option<[u8; 16]>,
) -> TestResult {
    let (status, _) = exchange(
        owner,
        admitted,
        None,
        command,
        COMMAND_TYPE_OBTAIN_CONTAINER,
        encode_obtain_container(&ObtainContainerIntent {
            action,
            category,
            container,
        })
        .map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    Ok(())
}
fn scenario(admin: String, case: Case) -> TestResult {
    run(async move |_environment_admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "npc_purchase_wire", true, false).await?;
        prepare_current_session(&h).await?;
        if case == Case::Bank {
            sqlx::query("INSERT INTO game_type2_audit_activation VALUES (1, now())").execute(&h.pool).await?;
        }
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let active = controller.active().ok_or("active content")?;
        let room = pin.qualified_room().clone();
        let frame_digest = pin.frame_binding().digest();
        let channel_pin = pin.into_channel_pin();
        // One player plus Alice and Rashid each occupy a carrier slot.
        let deps = Dependencies::with_content_and_capacity(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            3,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        // Freeze a two-gold native offer while retaining the normal NPC catalogue and route.
        let sold_item = if matches!(case, Case::Stackable | Case::Bundle) {
            "oteryn:item.tibia.i3446"
        } else {
            "oteryn:item.tibia.i3264"
        };
        let unit_price: u32 = 2;
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        let offer = ProjectV2ServiceOffer {
            item: ProjectV2DefinitionRef {
                family: ProjectV2Family::Item,
                key: sold_item.to_owned(),
                revision: "definition-r1".to_owned(),
            },
            direction: ProjectV2ServiceOfferDirection::SellToPlayer,
            unit_price: u64::from(unit_price),
            currency: matches!(case, Case::Currency | Case::CurrencyGoldMissing | Case::CurrencyTokensMissing).then(|| ProjectV2DefinitionRef { family: ProjectV2Family::Item, key: "oteryn:item.tibia.i3446".into(), revision: "definition-r1".into() }),
            count: (case == Case::Bundle).then_some(100),
            sub_type: None,
            parity_pending: false,
        };
        let book = NpcTalkBook::from_model(&model)
            .with(
                TRADER,
                NpcTalkEntry::new("Alice", Some("Alice trades.")).with_offers(vec![offer.clone()]),
            )
            .with(
                RASHID,
                NpcTalkEntry::new("Rashid", Some("Rashid trades.")).with_offers(vec![offer]),
            );
        let revision = book.catalogue_revision();
        owner.states.chat.install_npc_talk(Arc::new(book));
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let spot = runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            runtime
                .place_npcs(
                    &{
                        let mut table = NpcPlacementTable {
                            npcs: vec![
                                NpcPlacements {
                                    key: TRADER.into(),
                                    placements: vec![NpcPlacement {
                                        floor: -i8::try_from(spot.floor)?,
                                        x: u16::try_from(spot.x + 1)?,
                                        y: u16::try_from(spot.y)?,
                                        direction: MovementFacing::South,
                                    }],
                                },
                                NpcPlacements {
                                    key: RASHID.into(),
                                    placements: vec![NpcPlacement {
                                        floor: -i8::try_from(spot.floor)?,
                                        x: u16::try_from(spot.x + 2)?,
                                        y: u16::try_from(spot.y)?,
                                        direction: MovementFacing::South,
                                    }],
                                },
                            ],
                        };
                        table.npcs.sort_by(|left, right| left.key.cmp(&right.key));
                        table
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime
                .visible_entities()
                .npcs
                .into_iter()
                .find(|npc| runtime.npc_key(npc.actor) == Some(TRADER))
                .ok_or("shop actor absent")?;
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };

        let coins = if matches!(case, Case::Unfunded | Case::Bank) {
            ("oteryn:item.tibia.i3031", 90)
        } else {
            ("oteryn:item.tibia.i3035", 10)
        };
        let currency_case = matches!(case, Case::Currency | Case::CurrencyGoldMissing | Case::CurrencyTokensMissing);
        let coins = if currency_case { ("oteryn:item.tibia.i3031", if case == Case::CurrencyGoldMissing { 10 } else { 20 }) } else { coins };
        let mut purse = vec![coins, ("oteryn:item.tibia.i2854", 1), ("oteryn:item.tibia.i2854", 1)];
        if currency_case { purse.push(("oteryn:item.tibia.i3446", if case == Case::CurrencyTokensMissing { 5 } else { 10 })); }
        seed_main_backpack(&h, &purse).await?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?
                .0;
        let fence = item_fence_of(current).ok_or("item fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT(character_id) DO NOTHING")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(active.identity().server_artifact_digest().as_slice())
            .bind(active.identity().client_artifact_digest().as_slice())
            .bind(frame_digest.as_slice()).execute(&h.pool).await?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            if case == Case::Compat {
                &[3, 4, 6, 98]
            } else {
                &[3, 4, 6, 12, 14, 98, 514, 515]
            },
        )
        .ok_or("bulk purchase capabilities")?;
        assert!(admitted.continuity.selected_capabilities.contains(4));
        assert_eq!(
            admitted.continuity.selected_capabilities.contains(514),
            case != Case::Compat,
        );
        assert_eq!(
            admitted.continuity.selected_capabilities.contains(515),
            case != Case::Compat,
        );
        let inner:Vec<String>=sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid ORDER BY placement_ordinal").bind(id(41).as_slice()).fetch_all(&h.pool).await?;
        let target = uuid_bytes(&inner[1])?;
        let default = uuid_bytes(&inner[2])?;
        {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE game_character_progression_state SET level=$2 WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).bind(if matches!(case,Case::Drop|Case::Capacity){1_i64}else{1000_i64}).execute(&mut *tx).await?;
            if currency_case {
                sqlx::query("WITH removed AS (DELETE FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid AND item_instance_id IN(SELECT item_instance_id FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3446') RETURNING item_instance_id,world_id,placed_transaction_id) INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,world_id,encode($2,'hex')::uuid,1,placed_transaction_id FROM removed").bind(id(41).as_slice()).bind(target.as_slice()).execute(&mut *tx).await?;
            }
            if matches!(case, Case::Drop | Case::Capacity) {
                sqlx::query("WITH minted AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT game_character_uuid_v7(),encode($1,'hex')::uuid,'Item','oteryn:item.tibia.i3264','definition-r1',1,1,game_character_uuid_v7() FROM generate_series(1,20) RETURNING item_instance_id) INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,encode($1,'hex')::uuid,encode($2,'hex')::uuid,row_number() OVER(),item_instance_id FROM minted").bind(id(42).as_slice()).bind(target.as_slice()).execute(&mut *tx).await?;
            }
            if case == Case::Bank {
        sqlx::query("INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,sha256('seed'),encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,1,100000,0,1,sha256('seed'),0,encode($8,'hex')::uuid,encode($9,'hex')::uuid,encode($10,'hex')::uuid)").bind(id(235).as_slice()).bind(id(232).as_slice()).bind(id(234).as_slice()).bind(id(41).as_slice()).bind(id(40).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(id(236).as_slice()).bind(id(237).as_slice()).bind(id(238).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,100000,0,100000,encode($5,'hex')::uuid)").bind(id(233).as_slice()).bind(id(232).as_slice()).bind(id(40).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,100000,encode($3,'hex')::uuid)").bind(id(40).as_slice()).bind(id(42).as_slice()).bind(id(233).as_slice()).execute(&mut *tx).await?;
            }
            tx.commit().await?;
        }
        let make = |amount, in_backpacks, ignore_cap| NpcPurchaseIntent {
            offer: NpcTradeIntent {
                npc_actor: npc,
                catalogue_revision: revision,
                offer_index: 0,
                side: NpcTradeSide::Buy,
                quantity: 1,
                expected_unit_price: unit_price,
            },
            amount,
            in_backpacks,
            ignore_cap,
        };
        let main:String=sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        let mut first_buy = 3;
        if case == Case::Compat {
            let before = footprint(&h).await?;
            let (status, _) = exchange(
                &owner,
                &mut admitted,
                Some(npc),
                3,
                COMMAND_TYPE_NPC_PURCHASE,
                encode_npc_purchase(&make(1, false, true)).map_err(debug)?,
            )
            .await?;
            assert_eq!(status, CommandStatus::Rejected);
            assert_eq!(footprint(&h).await?, before);
            let (status, payload) = exchange(
                &owner,
                &mut admitted,
                Some(npc),
                6,
                oteryn_protocol_oteryn::npc_service::COMMAND_TYPE_NPC_TRADE_INTENT,
                oteryn_protocol_oteryn::npc_service::encode_npc_trade_intent(
                    &make(1, false, true).offer,
                )
                .map_err(debug)?,
            )
            .await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(
                oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&payload)
                    .map_err(debug)?,
                oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok
            );
            let new: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_purchase_details")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(new, 0);
            assert_eq!(gold(&h).await?, 998);
        } else if case == Case::Separate {
            set_target(
                &owner,
                &mut admitted,
                1,
                ObtainAction::Set,
                Some(QuickLootCategory::Equipment),
                Some(target),
            )
            .await?;
            let previous_views = admitted.continuity.item_view.views_revision;
            set_target(
                &owner,
                &mut admitted,
                2,
                ObtainAction::Open,
                Some(QuickLootCategory::Equipment),
                None,
            )
            .await?;
            assert!(
                admitted.continuity.item_view.views_revision >= previous_views + 2,
                "OPEN must publish a container view delta after its snapshot"
            );
            let request = QuickLootRequest {
                action: QuickLootAction::Assign,
                corpse: None,
                category: None,
                recipient: Some(QuickLootRecipient::Container),
                recipient_id: Some(default),
                fallback_to_main: true,
                filter_mode: None,
                filter_definition_ref: None,
            };
            let (status, _) = exchange(
                &owner,
                &mut admitted,
                None,
                3,
                COMMAND_TYPE_QUICK_LOOT,
                encode_quick_loot_request(&request).map_err(debug)?,
            )
            .await?;
            assert_eq!(status, CommandStatus::Accepted);
            set_target(
                &owner,
                &mut admitted,
                4,
                ObtainAction::Clear,
                Some(QuickLootCategory::Equipment),
                None,
            )
            .await?;
            let count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_obtain_containers")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(count, 0);
            let loot:String=sqlx::query_scalar("SELECT unassigned_container_id::text FROM game_character_quick_loot_settings WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(loot, inner[2]);
        } else {
            if case == Case::Obtain {
                set_target(
                    &owner,
                    &mut admitted,
                    1,
                    ObtainAction::Set,
                    None,
                    Some(default),
                )
                .await?;
                set_target(
                    &owner,
                    &mut admitted,
                    2,
                    ObtainAction::Set,
                    Some(QuickLootCategory::Equipment),
                    Some(target),
                )
                .await?;
                first_buy = 5;
            }
            let amount = match case {
                Case::Bags | Case::Unfunded | Case::Bank => 45,
                Case::Stackable => 250,
                Case::Bundle | Case::Currency | Case::CurrencyGoldMissing | Case::CurrencyTokensMissing => 3,
                Case::Drop => 3,
                _ => 1,
            };
            let intent = make(
                amount,
                matches!(case, Case::Bags | Case::Stackable | Case::Unfunded | Case::Bank | Case::Currency | Case::CurrencyGoldMissing | Case::CurrencyTokensMissing),
                case != Case::Capacity,
            );
            let encoded = encode_npc_purchase(&intent).map_err(debug)?;
            let before = footprint(&h).await?;
            let funds = gold(&h).await?;
            let (status, text) = exchange(
                &owner,
                &mut admitted,
                Some(npc),
                first_buy,
                COMMAND_TYPE_NPC_PURCHASE,
                encoded.clone(),
            )
            .await?;
            if matches!(case, Case::Unfunded | Case::Capacity | Case::CurrencyGoldMissing | Case::CurrencyTokensMissing) {
                assert_eq!(
                    status,
                    CommandStatus::Rejected,
                    "{}",
                    String::from_utf8_lossy(&text)
                );
                assert_eq!(footprint(&h).await?, before);
                assert_eq!(gold(&h).await?, funds);
                if case == Case::Capacity {
                    assert_eq!(String::from_utf8(text)?, "You do not have enough capacity.");
                } else {
                    assert_eq!(String::from_utf8(text)?, "You do not have enough money.");
                }
            } else {
                assert_eq!(
                    status,
                    CommandStatus::Accepted,
                    "{}",
                    String::from_utf8_lossy(&text)
                );
                let bags = match case {
                    Case::Bags | Case::Bank => 3,
                    Case::Stackable => 1,
                    _ => 0,
                };
                let cost = if currency_case { 20 } else { i64::from(amount) * 2 + bags * 20 };
                assert_eq!(gold(&h).await?, if case == Case::Bank { 0 } else { funds - cost });
                if case == Case::Bank {
                    let balance: i64 = sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid").bind(id(40).as_slice()).bind(id(42).as_slice()).fetch_one(&h.pool).await?;
                    assert_eq!(balance, 100000 - (cost - funds));
                    let debits: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_bank_entries WHERE kind=5").fetch_one(&h.pool).await?;
                    assert_eq!(debits, 1);
                }
                let after = footprint(&h).await?;
                let (replayed, replay_text) = exchange(
                    &owner,
                    &mut admitted,
                    None,
                    first_buy,
                    COMMAND_TYPE_NPC_PURCHASE,
                    encoded,
                )
                .await?;
                assert_eq!(replayed, CommandStatus::DuplicateReplay);
                assert_eq!(replay_text, text);
                assert_eq!(footprint(&h).await?, after);
                assert_eq!(gold(&h).await?, if case == Case::Bank { 0 } else { funds - cost });
                if case == Case::Bank {
                    let balance: i64 = sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid").bind(id(40).as_slice()).bind(id(42).as_slice()).fetch_one(&h.pool).await?;
                    assert_eq!(balance, 100000 - (cost - funds));
                    let debits: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_bank_entries WHERE kind=5").fetch_one(&h.pool).await?;
                    assert_eq!(debits, 1);
                }
                if matches!(case, Case::Bags | Case::Stackable) {
                    let counts:Vec<i64>=sqlx::query_scalar("SELECT count(e.item_instance_id) FROM game_npc_purchase_outputs o LEFT JOIN game_item_nested_entries e ON e.parent_item_instance_id=o.item_instance_id WHERE o.is_bag GROUP BY o.purchase_index ORDER BY o.purchase_index").fetch_all(&h.pool).await?;
                    assert_eq!(
                        counts,
                        if case == Case::Bags {
                            vec![20, 20, 5]
                        } else {
                            vec![3]
                        }
                    );
                    if case == Case::Stackable {
                        let stacks:Vec<i64>=sqlx::query_scalar("SELECT quantity FROM game_npc_purchase_outputs WHERE NOT is_bag ORDER BY purchase_index").fetch_all(&h.pool).await?;
                        assert_eq!(stacks, vec![100, 100, 50]);
                    }
                    let expected = if case == Case::Bags {
                        "Bought 45x sword and 3 shopping bags for 150 gold coins."
                    } else {
                        "Bought 250x bolt and 1 shopping bag for 520 gold coins."
                    };
                    assert_eq!(std::str::from_utf8(&text)?, expected);
                }
                if case == Case::Currency {
                    let tokens: i64 = sqlx::query_scalar("SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3446' AND lifecycle=1").fetch_one(&h.pool).await?;
                    assert_eq!(tokens, 4);
                    assert_eq!(std::str::from_utf8(&text)?, "Bought 3x sword for 6 bolts and 1x shopping bag for 20 gold coins.");
                }
                if case == Case::Bundle {
                    let stacks: Vec<i64> = sqlx::query_scalar("SELECT quantity FROM game_npc_purchase_outputs ORDER BY purchase_index").fetch_all(&h.pool).await?;
                    assert_eq!(stacks, vec![100,100,100]);
                    assert_eq!(std::str::from_utf8(&text)?, "Bought 300x bolt for 6 gold coins.");
                }
                if case == Case::Drop {
                    assert_eq!(
                        std::str::from_utf8(&text)?,
                        "Bought 3x sword for 6 gold coins."
                    );
                    let positions:Vec<Vec<u8>>=sqlx::query_scalar("SELECT g.spatial_position FROM game_npc_purchase_outputs o JOIN game_item_ground_locations g USING(item_instance_id)").fetch_all(&h.pool).await?;
                    let position = owner
                        .runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position();
                    let cell = [
                        position.x.to_be_bytes().as_slice(),
                        position.y.to_be_bytes().as_slice(),
                        position.floor.to_be_bytes().as_slice(),
                    ]
                    .concat();
                    assert_eq!(positions, vec![cell; 3]);
                    {
                        let mut tx = h.pool.begin().await?;
                        sqlx::query("SET LOCAL session_replication_role=replica")
                            .execute(&mut *tx)
                            .await?;
                        sqlx::query("WITH minted AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT game_character_uuid_v7(),encode($1,'hex')::uuid,'Item','oteryn:item.tibia.i3031','definition-r1',1,1,game_character_uuid_v7() FROM generate_series(1,25) RETURNING item_instance_id) INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context) SELECT m.item_instance_id,g.world_id,g.channel_id,g.runtime_scope_ownership_generation,g.spatial_position,convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8')||uuid_send(m.item_instance_id),g.map_revision,g.content_revision,g.native_room_placement_context FROM minted m CROSS JOIN (SELECT g.* FROM game_npc_purchase_outputs o JOIN game_item_ground_locations g USING(item_instance_id) ORDER BY o.purchase_index LIMIT 1) g").bind(id(42).as_slice()).execute(&mut *tx).await?;
                        tx.commit().await?;
                    }
                    let full = footprint(&h).await?;
                    let money = gold(&h).await?;
                    let (status, _) = exchange(
                        &owner,
                        &mut admitted,
                        Some(npc),
                        6,
                        COMMAND_TYPE_NPC_PURCHASE,
                        encode_npc_purchase(&make(3, false, true)).map_err(debug)?,
                    )
                    .await?;
                    assert_eq!(status, CommandStatus::Rejected);
                    assert_eq!(footprint(&h).await?, full);
                    assert_eq!(gold(&h).await?, money);
                }
                if case == Case::Obtain {
                    assert_eq!(last_parent(&h).await?, inner[1]);
                    set_target(
                        &owner,
                        &mut admitted,
                        6,
                        ObtainAction::Clear,
                        Some(QuickLootCategory::Equipment),
                        None,
                    )
                    .await?;
                    let (status, response) = exchange(
                        &owner,
                        &mut admitted,
                        Some(npc),
                        9,
                        COMMAND_TYPE_NPC_PURCHASE,
                        encode_npc_purchase(&make(1, false, true)).map_err(debug)?,
                    )
                    .await?;
                    assert_eq!(
                        status,
                        CommandStatus::Accepted,
                        "{}",
                        String::from_utf8_lossy(&response)
                    );
                    assert_eq!(last_parent(&h).await?, inner[2]);
                    set_target(&owner, &mut admitted, 10, ObtainAction::Clear, None, None).await?;
                    let (status, response) = exchange(
                        &owner,
                        &mut admitted,
                        Some(npc),
                        13,
                        COMMAND_TYPE_NPC_PURCHASE,
                        encode_npc_purchase(&make(1, false, true)).map_err(debug)?,
                    )
                    .await?;
                    assert_eq!(
                        status,
                        CommandStatus::Accepted,
                        "{}",
                        String::from_utf8_lossy(&response)
                    );
                    assert_eq!(last_parent(&h).await?, main);
                    set_target(
                        &owner,
                        &mut admitted,
                        14,
                        ObtainAction::Set,
                        Some(QuickLootCategory::Equipment),
                        Some(target),
                    )
                    .await?;
                    {
                        let mut tx = h.pool.begin().await?;
                        sqlx::query("SET LOCAL session_replication_role=replica")
                            .execute(&mut *tx)
                            .await?;
                        sqlx::query("WITH minted AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT game_character_uuid_v7(),encode($1,'hex')::uuid,'Item','oteryn:item.tibia.i3446','definition-r1',1,1,game_character_uuid_v7() FROM generate_series(1,19) RETURNING item_instance_id) INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,encode($1,'hex')::uuid,encode($2,'hex')::uuid,1+row_number() OVER(),item_instance_id FROM minted").bind(id(42).as_slice()).bind(target.as_slice()).execute(&mut *tx).await?;
                        tx.commit().await?;
                    }
                    let (status, response) = exchange(
                        &owner,
                        &mut admitted,
                        Some(npc),
                        17,
                        COMMAND_TYPE_NPC_PURCHASE,
                        encode_npc_purchase(&make(1, false, true)).map_err(debug)?,
                    )
                    .await?;
                    assert_eq!(
                        status,
                        CommandStatus::Accepted,
                        "{}",
                        String::from_utf8_lossy(&response)
                    );
                    assert_eq!(last_parent(&h).await?, main);
                    assert_eq!(gold(&h).await?, funds - 8);
                }
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
