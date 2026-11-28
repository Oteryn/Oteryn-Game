use tokio::io::AsyncWriteExt;
use super::fixture::{self, Harness, debug, id, run};

use super::item_look_fixtures::seed_main_backpack;

use super::{Dependencies, TestResult, prepare_current_session, reset};

use crate::content::native_gameplay::test_support::activated_with_loot_tables;

use crate::content::{
    NpcDataCatalogue, ProjectV2DefinitionRef, ProjectV2Family, ProjectV2ServiceOffer,
    ProjectV2ServiceOfferDirection, npc_catalogue_preproduction_limits,
};

use crate::foundation::{
    ChannelContentPin, MessageType, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
    decode_wire_envelope,
};

use crate::gameplay_transport::capabilities::SelectedCapabilities;

use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    UseCommand, serve_admitted,
};

use crate::gameplay_transport::npc_shop::NpcShopStep;

use crate::gameplay_transport::npc_talk::{NpcTalkBook, NpcTalkEntry};

use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};

use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};

use oteryn_protocol_oteryn::npc_service::{
    CAPABILITY_NPC_SERVICE_V1, COMMAND_TYPE_NPC_TALK_INTENT, NpcIntentDisposition, NpcTalkIntent,
    NpcTradeSide, encode_npc_talk_intent,
};

use oteryn_protocol_oteryn::npc_shop::{
    CAPABILITY_NPC_SHOP_V1, COMMAND_TYPE_NPC_SHOP, NpcShopAction, NpcShopIntent, NpcShopResult,
    decode_npc_shop_result, encode_npc_shop_intent,
};

use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};

use std::sync::Arc;

const MEAT: &str = "oteryn:item.tibia.i3577";

const FISH: &str = "oteryn:item.tibia.i3578";

const TRADER: &str = "oteryn:npc.alice";

const RASHID: &str = "oteryn:npc.rashid";

/// One connection: when `greet` names two CommandIds, the real NPC_TALK "hi" and "trade" open
/// the conversation and its window first, as every connection starts without one; then the
/// command 97 under `command_id`. Answers that command's result.
async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    greet: Option<(u64, u64)>,
    command_id: u64,
    intent: &NpcShopIntent,
) -> TestResult<(CommandStatus, NpcShopResult)> {
    exchange_with_scope(owner, admitted, greet, command_id, intent, false).await
}

async fn exchange_with_scope(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    greet: Option<(u64, u64)>,
    command_id: u64,
    intent: &NpcShopIntent,
    include_equipped: bool,
) -> TestResult<(CommandStatus, NpcShopResult)> {
    let mut commands = Vec::new();
    if let Some((hi, trade)) = greet {
        for (talk_id, text) in [(hi, "hi"), (trade, "trade")] {
            let talk = NpcTalkIntent {
                npc_actor: intent.npc_actor,
                text: text.into(),
            };
            commands.push((
                talk_id,
                COMMAND_TYPE_NPC_TALK_INTENT,
                encode_npc_talk_intent(&talk).map_err(debug)?,
            ));
        }
    }
    commands.push((
        command_id,
        COMMAND_TYPE_NPC_SHOP,
        {
            let mut payload = encode_npc_shop_intent(intent).map_err(debug)?;
            if include_equipped {
                payload.extend([56, 1]);
            }
            payload
        },
    ));
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async move {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
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
                if envelope.message_type() == MessageType::StateDelta {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                        .map_err(debug)?;
                    if delta.domain_id
                        == oteryn_protocol_oteryn::npc_service::STATE_DOMAIN_NPC_TRADE_WINDOW
                    {
                        let window =
                            oteryn_protocol_oteryn::npc_service::decode_npc_trade_window_for(
                                delta.payload,
                                true,
                            )
                            .map_err(debug)?;
                        assert_eq!(window.details.len(), window.offers.len());
                        for detail in &window.details {
                            assert!(!detail.name.is_empty());
                            assert!(!detail.name.contains("Unknown"));
                            assert!(detail.maximum_quantity > 0);
                        }
                    }
                }
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, *sent_id);
                    if *command_type == COMMAND_TYPE_NPC_SHOP {
                        reply = Some((
                            result.status,
                            decode_npc_shop_result(result.payload).map_err(debug)?,
                        ));
                    } else {
                        assert_eq!(result.status, CommandStatus::Accepted, "talk {sent_id}");
                        if include_equipped {
                            let disposition =
                                oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(
                                    result.payload,
                                )
                                .map_err(debug)?;
                            assert_eq!(disposition, NpcIntentDisposition::Ok, "talk {sent_id}");
                        }
                    }
                    break;
                }
            }
        }
        let reply = reply.ok_or("shop result")?;
        client.shutdown().await?;
        Ok(reply)
    };
    let (ended, result): (_, TestResult<(CommandStatus, NpcShopResult)>) = tokio::time::timeout(
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

/// Row counts of the trade record, its lines, the outbox and the live item instances.
async fn footprint(h: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_npc_trades),(SELECT count(*) FROM game_npc_trade_lines),\
         (SELECT count(*) FROM game_item_audit_outbox),\
         (SELECT count(*) FROM game_item_instances WHERE lifecycle=1)",
    )
    .fetch_one(&h.pool)
    .await?)
}

/// Live backpack entries as (key, quantity), lowest ordinal first.
async fn carried(h: &Harness) -> TestResult<Vec<(String, i64)>> {
    Ok(sqlx::query_as(
        "SELECT i.definition_production_key,i.quantity FROM game_item_container_entries e \
         JOIN game_item_instances i USING(item_instance_id) \
         WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY e.placement_ordinal",
    )
    .bind(id(70).as_slice())
    .fetch_all(&h.pool)
    .await?)
}

pub(super) fn run_shop_scenario(wandering_rashid: bool) -> TestResult {
    scenario_for(if wandering_rashid {
        ShopScenario::Rashid
    } else {
        ShopScenario::Sell
    })
}

pub(super) enum ShopScenario {
    Sell,
    Rashid,
    Bulk,
    WeightLimit,
    MissingProfile,
    HeldTradeWindow,
    PackedBuy,
    EquippedSale,
}

pub(super) fn scenario_for(case: ShopScenario) -> TestResult {
    let wandering_rashid = matches!(case, ShopScenario::Rashid);
    let packed = matches!(case, ShopScenario::PackedBuy);
    let equipped = matches!(case, ShopScenario::EquippedSale);
    let bulk = matches!(case, ShopScenario::Bulk) || packed;
    let weight_limit = matches!(case, ShopScenario::WeightLimit);
    run(async move |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "npc_shop_wire", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        if equipped {
            super::tutorial_vitals_fixture::publish_native_activation(&h, &pin).await?;
        }
        let active = controller.active().ok_or("active content")?;
        let entry = if wandering_rashid {
            let now = i64::try_from(crate::world_clock::now_utc_ms().ok_or("World clock")?)?;
            let destination = oteryn_gameplay_entities::rashid_schedule::destination_at(now).placement;
            (
                i32::from(destination.x),
                i32::from(destination.y),
                -i16::from(destination.floor),
            )
        } else {
            (2, 0, 7)
        };
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            entry,
        );
        // One player plus Alice and Rashid each occupy a carrier slot.
        let deps = Dependencies::with_content_and_capacity(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            3,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        if matches!(case, ShopScenario::MissingProfile) {
            use oteryn_protocol_oteryn::npc_service::{NpcOffer, NpcTradeWindow};
            let native = active.native_gameplay().ok_or("native gameplay")?;
            let index = native.item_index().ok_or("item index")?;
            let (reference, key, revision) = (1..=u32::MAX)
                .filter_map(std::num::NonZeroU32::new)
                .map_while(|reference| {
                    let (key, revision) = index.definition_at(reference)?;
                    Some((reference, key, revision))
                })
                .find(|(_, key, revision)| native.item_policy(key, revision).is_none())
                .ok_or("fixture requires an indexed item without a native item policy")?;
            assert!(native.item_policy(key, revision).is_none());
            let window = NpcTradeWindow {
                open: Some((EntityRef { identity: [7; 16], generation: 1 }, 1)),
                offers: vec![NpcOffer {
                    item_definition_ref: reference.get(),
                    side: NpcTradeSide::Buy,
                    count: 1,
                    unit_price: 10,
                }],
                details: Vec::new(),
            };
            let quoted = owner.npc_quote_window(deps.admitted.game_session_id, &window)
                .ok_or("missing profile closed the quote window")?;
            assert_eq!(quoted.offers, window.offers);
            oteryn_protocol_oteryn::npc_service::encode_npc_trade_window_for(&quoted, true).map_err(debug)?;
            return Ok(());
        }
        // Rashid's captured source buys white cloth at 150 gold, after the quest.
        // Alice retains the synthetic four-gold meat offer for the generic case.
        let sold_item = if wandering_rashid {
            "oteryn:item.tibia.i5917"
        } else if equipped {
            "oteryn:item.tibia.i3447"
        } else {
            MEAT
        };
        let unit_price: u32 = if wandering_rashid { 150 } else { 4 };
        let total_gold = 8 * u64::from(unit_price);
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
            direction: ProjectV2ServiceOfferDirection::BuyFromPlayer,
            unit_price: u64::from(unit_price),
            currency: None,
            count: None,
            sub_type: None,
            parity_pending: false,
        };
        let offers = if weight_limit {
            let mut buy = offer.clone();
            buy.item.key = "oteryn:item.tibia.i3447".into();
            buy.direction = ProjectV2ServiceOfferDirection::SellToPlayer;
            buy.unit_price = 1;
            vec![buy]
        } else if bulk {
            let mut buy = offer.clone();
            buy.direction = ProjectV2ServiceOfferDirection::SellToPlayer;
            buy.unit_price = 1;
            let mut sell = offer.clone();
            sell.unit_price = 5000;
            vec![buy, sell]
        } else {
            vec![offer.clone()]
        };
        let book = NpcTalkBook::from_model(&model)
            .with(
                TRADER,
                NpcTalkEntry::new("Alice", Some("Alice trades.")).with_offers(offers),
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
                                    key: if wandering_rashid { RASHID } else { TRADER }.into(),
                                    placements: vec![NpcPlacement {
                                        floor: -i8::try_from(spot.floor)?,
                                        x: u16::try_from(spot.x + 1)?,
                                        y: u16::try_from(spot.y)?,
                                        direction: MovementFacing::South,
                                    }],
                                },
                                NpcPlacements {
                                    key: if wandering_rashid { TRADER } else { RASHID }.into(),
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
                .find(|npc| {
                    runtime.npc_key(npc.actor)
                        == Some(if wandering_rashid { RASHID } else { TRADER })
                })
                .ok_or("shop actor absent")?;
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        if bulk || weight_limit {
            // Exercise the weight boundary separately from the bulk side and quantity limits.
            // The bulk case buys 150 meat, so give that fixture room for the full purchase.
            let level = if bulk { 200_i64 } else { 1_i64 };
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE game_character_progression_state SET level=$2 WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).bind(level).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        if weight_limit {
            // Level-one capacity is 580 oz. The backpack, four plate armors and
            // one meat weigh 511 oz, leaving room for 98 arrows but not 101.
            let plate = "oteryn:item.tibia.i3357";
            seed_main_backpack(
                &h,
                &[(plate, 1), (plate, 1), (plate, 1), (plate, 1), (MEAT, 1)],
            )
            .await?;
            // Bank settlement is available only in the activated type-2 audit tuple.
            sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
                .execute(&h.pool)
                .await?;
            super::guild_bank_fixtures::bank_seed::balance(&h, id(40), id(41), 1000, 180).await?;
        } else if bulk {
            seed_main_backpack(
                &h,
                &[
                    ("oteryn:item.tibia.i3031", 100),
                    ("oteryn:item.tibia.i3031", 100),
                ],
            )
            .await?;
        } else {
            seed_main_backpack(&h, &[(sold_item, 5), (FISH, 2), (sold_item, 3)]).await?;
        }
        if equipped {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT(character_id) DO NOTHING")
                .bind(id(41).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',3,1,encode($1,'hex')::uuid)")
                .bind(id(220).as_slice()).bind(id(42).as_slice()).bind(sold_item).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,10,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
                .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(220).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?
                .0;
        let fence = item_fence_of(current).ok_or("item fence")?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            // Capabilities 3 and 81 are offered by the bundle World only.
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[
                CAPABILITY_NPC_SERVICE_V1,
                4,
                6,
                CAPABILITY_NPC_SHOP_V1,
                oteryn_protocol_oteryn::trade_details::CAPABILITY_TRADE_DETAILS_V1,
                589,
            ],
        )
        .ok_or("npc shop selection")?;
        if matches!(case, ShopScenario::HeldTradeWindow) {
            use crate::durability::DurabilityRoot;
            use crate::gameplay_transport::npc_command::npc_bank_talk;
            use oteryn_simulation_determinism::SemanticTimeMicros;

            let first = owner.owner_now();
            let greeted = npc_bank_talk(
                &owner,
                deps.actor,
                UseCommand { game_session_id: session, command_id: 1, item_fence: Some(fence) },
                NpcTalkIntent { npc_actor: npc, text: "hi".into() },
                first,
            ).await;
            assert_eq!(greeted.disposition, NpcIntentDisposition::Ok);
            assert!(!greeted.unknown);

            let held = h.root.try_issue_background_semantic_pass()?;
            let mut trade = std::pin::pin!(DurabilityRoot::with_holder_owner(npc_bank_talk(
                &owner,
                deps.actor,
                UseCommand { game_session_id: session, command_id: 2, item_fence: Some(fence) },
                NpcTalkIntent { npc_actor: npc, text: "trade".into() },
                SemanticTimeMicros::from_micros(first.get() + 1_000_000),
            )));
            let polled = std::future::poll_fn(|cx| {
                std::task::Poll::Ready(std::future::Future::poll(trade.as_mut(), cx))
            }).await;
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            assert!(polled.is_pending(), "the trade dialogue must wait for the receipt read");
            let reply = trade.await;
            assert!(!reply.unknown);
            assert_eq!(reply.disposition, NpcIntentDisposition::Ok);
            assert!(reply.window.is_some(), "ordinary trade must open a window");
            return Ok(());
        }
        if weight_limit {
            let before = footprint(&h).await?;
            let inventory = carried(&h).await?;
            let (_, inventory_delta) = bulk_exchange(
                &owner,
                &mut admitted,
                npc,
                1,
                NpcTradeSide::Buy,
                (101, false),
                NpcIntentDisposition::Rejected,
            )
            .await?;
            assert!(inventory_delta.is_none());
            assert_eq!(footprint(&h).await?, before);
            assert_eq!(carried(&h).await?, inventory);
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT balance FROM game_account_bank_balances")
                    .fetch_one(&h.pool)
                    .await?,
                1000
            );
            let (_, accepted) = bulk_exchange(
                &owner,
                &mut admitted,
                npc,
                4,
                NpcTradeSide::Buy,
                (98, false),
                NpcIntentDisposition::Ok,
            )
            .await?;
            assert!(accepted.is_some());
            let after = carried(&h).await?;
            assert_eq!(after.last(), Some(&("oteryn:item.tibia.i3447".into(), 98)));
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT balance FROM game_account_bank_balances")
                    .fetch_one(&h.pool)
                    .await?,
                902
            );
            drop(owner);
            drop(authority);
            drop(seal);
            return Ok(());
        }
        if bulk {
            let (window, bought) = bulk_exchange(
                &owner,
                &mut admitted,
                npc,
                1,
                NpcTradeSide::Buy,
                (150, packed),
                NpcIntentDisposition::Ok,
            )
            .await?;
            let bought = bought.ok_or("buy inventory")?;
            if packed {
                assert_eq!(carried(&h).await?, vec![
                    ("oteryn:item.tibia.i3031".to_owned(), 30),
                    ("oteryn:item.tibia.i21411".to_owned(), 1),
                ]);
                let children: Vec<i64> = sqlx::query_scalar(
                    "SELECT i.quantity FROM game_npc_trades t JOIN game_item_nested_entries n
                      ON n.parent_item_instance_id=t.shopping_bag_item_id
                      JOIN game_item_instances i ON i.item_instance_id=n.item_instance_id
                      WHERE i.definition_production_key=$1 ORDER BY i.quantity"
                ).bind(MEAT).fetch_all(&h.pool).await?;
                assert_eq!(children, vec![50, 100]);
                let total: i64 = sqlx::query_scalar("SELECT total_gold_units FROM game_npc_trades")
                    .fetch_one(&h.pool).await?;
                assert_eq!(total, 170);
                drop(owner);
                drop(authority);
                drop(seal);
                h.cleanup().await?;
                return Ok(());
            }
            assert!(window.details.iter().all(|detail| detail.name == "meat"));
            let mut limits: Vec<_> = window
                .offers
                .iter()
                .zip(&window.details)
                .map(|(offer, detail)| (offer.side as u32, detail.maximum_quantity))
                .collect();
            limits.sort_unstable();
            assert_eq!(limits, vec![(1, 2000), (2, 201)]);
            let item_ref = active
                .native_gameplay()
                .and_then(|native| native.item_index())
                .and_then(|index| index.definition_ref("Item", MEAT, "definition-r1"))
                .ok_or("meat definition")?;
            let mut counts: Vec<_> = bought
                .entries
                .iter()
                .filter(|entry| entry.item_definition_ref == item_ref)
                .map(|entry| entry.count.get())
                .collect();
            counts.sort_unstable();
            assert_eq!(counts, vec![50, 100]);
            let (_, sold) = bulk_exchange(
                &owner,
                &mut admitted,
                npc,
                4,
                NpcTradeSide::Sell,
                (125, false),
                NpcIntentDisposition::Ok,
            )
            .await?;
            let sold = sold.ok_or("sale inventory")?;
            assert_eq!(
                sold.entries
                    .iter()
                    .filter(|entry| entry.item_definition_ref == item_ref)
                    .map(|entry| entry.count.get())
                    .sum::<u32>(),
                25
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_npc_trades")
                    .fetch_one(&h.pool)
                    .await?,
                2
            );
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await?;
            return Ok(());
        }

        let intent = |action| NpcShopIntent {
            npc_actor: npc,
            catalogue_revision: revision,
            offer_index: 0,
            side: NpcTradeSide::Sell,
            action,
            expected_unit_price: unit_price,
        };
        let seeded = vec![
            (sold_item.to_owned(), 5),
            (FISH.to_owned(), 2),
            (sold_item.to_owned(), 3),
        ];
        assert_eq!(carried(&h).await?, seeded);
        let empty = footprint(&h).await?;

        // LOOK answers the server content description and writes nothing.
        let (status, looked) = exchange(
            &owner,
            &mut admitted,
            Some((1, 2)),
            3,
            &intent(NpcShopAction::Look),
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(looked.disposition, NpcIntentDisposition::Ok);
        assert!(
            looked.look_text.starts_with("You see "),
            "{}",
            looked.look_text
        );
        assert_eq!((looked.sold_units, looked.total_gold_units), (0, 0));
        assert_eq!(footprint(&h).await?, empty);
        assert_eq!(carried(&h).await?, seeded);

        if wandering_rashid {
            use crate::durability::quest_state::{
                QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
            };
            use oteryn_gameplay_quests::quest::{
                QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
                QuestTransition,
            };
            let quest = "oteryn:quest.the_travelling_trader_quest";
            let track = "oteryn:quest-progress/chosen/travelling-trader/stage";
            let key = "oteryn:quest-transition/fixture/rashid-shop-access";
            let refused = owner
                .npc_shop(
                    deps.actor,
                    UseCommand {
                        game_session_id: session,
                        command_id: 899,
                        item_fence: Some(fence),
                    },
                    intent(NpcShopAction::SellAll),
                )
                .await;
            assert_eq!(refused.result().disposition, NpcIntentDisposition::Rejected);
            assert_eq!(footprint(&h).await?, empty);
            assert_eq!(carried(&h).await?, seeded);
            // Only the prior quest is seeded; shop authority, payment and replay
            // still run through their production dispatch and locked item owner.
            let catalogue = Arc::new(
                QuestStateCatalogue::new(
                    "content-1",
                    vec![QuestTrack {
                        key: track.into(),
                        quest: quest.into(),
                        initial: -1,
                        min: -1,
                        max: 13,
                    }],
                    vec![QuestTransition {
                        world_change: None,
                        key: key.into(),
                        quest: quest.into(),
                        completes: true,
                        experience: None,
                        effects: vec![QuestEffect {
                            track: track.into(),
                            from: QuestComparison::Eq(-1),
                            effect: QuestEffectKind::Set(13),
                        }],
                    }],
                )
                .map_err(debug)?,
            );
            let quest_fence = owner
                .current_quest_fence(session)
                .await
                .map_err(|()| "quest fence")?
                .ok_or("quest fence")?;
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let completed = slot
                .commit_quest_transition(
                    owner.root,
                    owner.character,
                    owner.holder,
                    quest_fence,
                    QuestTransitionRequest {
                        transition_key: key.into(),
                        cause: QuestCause::Command(crate::foundation::CommandRef::new(
                            session,
                            crate::foundation::CommandId::new(900).map_err(debug)?,
                        )),
                    },
                    catalogue,
                )
                .await
                .map_err(debug)?;
            assert!(
                matches!(completed, QuestTransitionOutcome::Committed(_)),
                "{completed:?}"
            );
        }

        // SELL_ALL sells both stacks, eight units in one durable trade.
        let sell_all = intent(NpcShopAction::SellAll);
        let wall_ms = || -> TestResult<i64> {
            let elapsed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
            Ok(i64::try_from(elapsed.as_millis())?)
        };
        let wall_before = wall_ms()?;
        let (status, sold) = exchange(&owner, &mut admitted, Some((4, 5)), 6, &sell_all).await?;
        let wall_after = wall_ms()?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            (sold.disposition, sold.sold_units, sold.total_gold_units),
            (NpcIntentDisposition::Ok, 8, total_gold)
        );
        assert_eq!(
            carried(&h).await?,
            vec![
                (FISH.to_owned(), 2),
                (
                    if wandering_rashid {
                        crate::domain::currency::Coin::Platinum
                    } else {
                        crate::domain::currency::Coin::Gold
                    }
                    .production_key()
                    .to_owned(),
                    if wandering_rashid { 12 } else { 32 }
                ),
            ]
        );
        let after = footprint(&h).await?;
        assert_eq!((after.0, after.1, after.2), (1, 3, 1));
        let (units, gold, flag): (i64, String, bool) = sqlx::query_as(
            "SELECT quantity::bigint,(quantity*unit_price)::text,sell_all FROM game_npc_trades",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!((units, gold, flag), (8, total_gold.to_string(), true));
        if wandering_rashid {
            let occurred_at: i64 = sqlx::query_scalar("SELECT occurred_at FROM game_npc_trades")
                .fetch_one(&h.pool)
                .await?;
            assert!(
                (wall_before..=wall_after).contains(&occurred_at),
                "Rashid's trade receipt timestamp {occurred_at} is outside wall time {wall_before}..={wall_after}",
            );
        }

        if equipped {
            let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_slots WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(220).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(before, 1);
            // The two earlier exchanges used all four talk intents in this session's window.
            tokio::time::sleep(std::time::Duration::from_micros(
                oteryn_gameplay_npc::npc_command::RATE_WINDOW_MICROS,
            ))
            .await;
            let (status, included) = exchange_with_scope(
                &owner, &mut admitted, Some((7, 8)), 9, &sell_all, true,
            ).await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!((included.disposition, included.sold_units, included.total_gold_units),
                (NpcIntentDisposition::Ok, 3, 12));
            let after: (i64, i64) = sqlx::query_as("SELECT i.quantity,s.revision::bigint
                FROM game_item_instances i CROSS JOIN game_character_equipment_state s
                WHERE i.item_instance_id=encode($1,'hex')::uuid AND s.character_id=encode($2,'hex')::uuid
                AND NOT EXISTS (SELECT 1 FROM game_character_equipment_slots e WHERE e.item_instance_id=i.item_instance_id)")
                .bind(id(220).as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(after, (0, 2));
            let (status, replayed) = exchange_with_scope(&owner, &mut admitted, None, 9, &sell_all, true).await?;
            assert_eq!(status, CommandStatus::DuplicateReplay);
            assert_eq!(replayed, included);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await?;
            return Ok(());
        }
        // The resend of command 6 replays the retained result; the durable receipt answers the
        // same command again with its first outcome and writes nothing.
        let (status, replayed) = exchange(&owner, &mut admitted, None, 6, &sell_all).await?;
        assert_eq!(status, CommandStatus::DuplicateReplay);
        assert_eq!(replayed, sold);
        let again = owner
            .npc_shop(
                deps.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 6,
                    item_fence: Some(fence),
                },
                sell_all,
            )
            .await;
        assert!(matches!(again, NpcShopStep::Sold { units: 8, gold } if gold == total_gold));
        assert_eq!(footprint(&h).await?, after);

        // A new sell-all finds no meat: refused, nothing written.
        let (status, nothing) = exchange(&owner, &mut admitted, Some((7, 8)), 9, &sell_all).await?;
        assert_ne!(nothing.disposition, NpcIntentDisposition::Ok, "{status:?}");
        assert_eq!(footprint(&h).await?, after);

        if wandering_rashid {
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }

        // A visible Rashid outside today's server-owned Canary destination cannot trade. This
        // traverses the production command decoder and dispatcher, rather than calling the
        // schedule predicate directly, and leaves durable state untouched.
        let placed_rashid = owner.runtime.lock().await.visible_entities().npcs[1];
        let rashid_intent = NpcShopIntent {
            npc_actor: EntityRef {
                identity: placed_rashid.actor.placement_identity(),
                generation: placed_rashid.generation,
            },
            catalogue_revision: revision,
            offer_index: 0,
            side: NpcTradeSide::Sell,
            action: NpcShopAction::SellAll,
            expected_unit_price: 4,
        };
        let (_, unavailable) =
            exchange(&owner, &mut admitted, Some((10, 11)), 12, &rashid_intent).await?;
        assert_ne!(unavailable.disposition, NpcIntentDisposition::Ok);
        assert_eq!(footprint(&h).await?, after);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn bulk_exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    first: u64,
    side: NpcTradeSide,
    (quantity, packed): (u32, bool),
    expected: NpcIntentDisposition,
) -> TestResult<(
    oteryn_protocol_oteryn::npc_service::NpcTradeWindow,
    Option<oteryn_protocol_oteryn::item_view::CharacterInventory>,
)> {
    use oteryn_protocol_oteryn::npc_service as wire;
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async { serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let mut shown = None;
        for (command_id, text) in [(first, "hi"), (first + 1, "trade")] {
            if text == "trade" {
                shown = None;
            }
            let payload = wire::encode_npc_talk_intent(&NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            })
            .map_err(debug)?;
            write_frame(
                &mut client,
                &encode_client_command(
                    before.continuity.connection_generation,
                    &ClientCommandValue {
                        command_id,
                        command_type: COMMAND_TYPE_NPC_TALK_INTENT,
                        payload: &payload,
                    },
                )
                .map_err(debug)?,
            )
            .await?;
            let mut result = false;
            while !result || text == "trade" && shown.is_none() {
                let frame = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let reply = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(reply.command_id, command_id);
                    assert_eq!(
                        wire::decode_npc_intent_result(reply.payload).map_err(debug)?,
                        NpcIntentDisposition::Ok
                    );
                    result = true;
                } else if envelope.message_type() == MessageType::StateDelta {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                        .map_err(debug)?;
                    if delta.domain_id == wire::STATE_DOMAIN_NPC_TRADE_WINDOW {
                        shown = Some(
                            wire::decode_npc_trade_window_for(delta.payload, true)
                                .map_err(debug)?,
                        );
                    }
                }
            }
        }
        let window = shown.ok_or("shop window")?;
        let (offer_index, offer) = window
            .offers
            .iter()
            .enumerate()
            .find(|(_, offer)| offer.side == side)
            .ok_or("side offer")?;
        assert!(!window.details[offer_index].name.is_empty());
        assert!(quantity <= window.details[offer_index].maximum_quantity);
        let mut shop = oteryn_dev_client::client_ui::npc_shop::LiveShop {
            selected: true,
            ..Default::default()
        };
        oteryn_dev_client::client_ui::trade_details::apply_window(&mut shop, window.clone());
        let quote = shop
            .quotes()
            .into_iter()
            .find(|quote| quote.side == side)
            .ok_or("decoded UI quote")?;
        assert!(
            shop.quote_text(quote)
                .contains(&window.details[offer_index].name)
        );
        assert_eq!(
            shop.maximum_amount(quote),
            window.details[offer_index].maximum_quantity
        );
        assert!(!shop.valid_amount(quote, shop.maximum_amount(quote) + 1));
        let intent = shop
            .trade(quote, quantity)
            .ok_or("UI trade within authoritative bound")?;
        assert_eq!(intent.expected_unit_price, offer.unit_price);
        let mut payload = wire::encode_npc_trade_intent(&intent).map_err(debug)?;
        if packed {
            payload.extend([56, 1]);
        }
        write_frame(
            &mut client,
            &encode_client_command(
                before.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: first + 2,
                    command_type: wire::COMMAND_TYPE_NPC_TRADE_INTENT,
                    payload: &payload,
                },
            )
            .map_err(debug)?,
        )
        .await?;
        let mut result = false;
        let mut inventory = None;
        while !result || (expected == NpcIntentDisposition::Ok && inventory.is_none()) {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let reply = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(reply.command_id, first + 2);
                assert_eq!(
                    wire::decode_npc_intent_result(reply.payload).map_err(debug)?,
                    expected
                );
                result = true;
            } else if envelope.message_type() == MessageType::StateDelta {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                    .map_err(debug)?;
                if delta.domain_id == 9 {
                    inventory = Some(
                        oteryn_protocol_oteryn::item_view::decode_character_inventory(
                            delta.payload,
                        )
                        .map_err(debug)?,
                    );
                }
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>((window, inventory))
    };
    let (ended, result) = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        fixture::join_two(serving, driving),
    )
    .await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = ended
    {
        *admitted = current;
    }
    result
}
