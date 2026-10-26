//! NPC-TRADE-SELLALL-1: command 97 bytes through `serve_admitted`, the composed NPC trade owner
//! and runtime-role PostgreSQL: LOOK writes nothing, SELL_ALL sells every carried stack once.
use super::fixture::{self, Harness, debug, id, run};
use super::item_look::seed_main_backpack;
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{
    NpcDataCatalogue, ProjectV2DefinitionRef, ProjectV2Family, ProjectV2ServiceOffer,
    ProjectV2ServiceOfferDirection, npc_catalogue_preproduction_limits,
};
use crate::foundation::{
    ChannelContentPin, MessageType, MovementFacing, NpcPlacement, NpcPlacementTable,
    NpcPlacements, decode_wire_envelope,
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
use tokio::io::AsyncWriteExt;

const MEAT: &str = "oteryn:item.tibia.i3577";
const FISH: &str = "oteryn:item.tibia.i3578";
const TRADER: &str = "oteryn:npc.alice";

#[test]
fn npc_shop_postgres_real_wire_look_sell_all_replay_and_nothing_left() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "npc shop PostgreSQL thread panicked")??;
    Ok(())
}

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
        encode_npc_shop_intent(intent).map_err(debug)?,
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

fn scenario() -> TestResult {
    run(async |admin| {
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
        let active = controller.active().ok_or("active content")?;
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (2, 0, 7),
        );
        let deps = Dependencies::with_content(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        // The production catalogue's revision, plus one trader buying meat at 4 gold each.
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
                key: MEAT.to_owned(),
                revision: "definition-r1".to_owned(),
            },
            direction: ProjectV2ServiceOfferDirection::BuyFromPlayer,
            unit_price: 4,
            currency: None,
            count: None,
            sub_type: None,
            parity_pending: false,
        };
        let book = NpcTalkBook::from_model(&model).with(
            TRADER,
            NpcTalkEntry::new("Alice", Some("Alice trades.")).with_offers(vec![offer]),
        );
        let revision = book.catalogue_revision();
        owner.states.chat.install_npc_talk(Arc::new(book));
        let session = deps.admitted.game_session_id;
        owner.states.chat.admit(session, "Fixture Hero", owner.owner_now());
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
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: TRADER.into(),
                            placements: vec![NpcPlacement {
                                floor: -i8::try_from(spot.floor)?,
                                x: u16::try_from(spot.x + 1)?,
                                y: u16::try_from(spot.y)?,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime.visible_entities().npcs[0];
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        // Two meat stacks and a fish the trader does not buy.
        seed_main_backpack(&h, &[(MEAT, 5), (FISH, 2), (MEAT, 3)]).await?;
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
            &[CAPABILITY_NPC_SERVICE_V1, CAPABILITY_NPC_SHOP_V1],
        )
        .ok_or("npc shop selection")?;
        let intent = |action| NpcShopIntent {
            npc_actor: npc,
            catalogue_revision: revision,
            offer_index: 0,
            side: NpcTradeSide::Sell,
            action,
            expected_unit_price: 4,
        };
        let seeded = vec![
            (MEAT.to_owned(), 5),
            (FISH.to_owned(), 2),
            (MEAT.to_owned(), 3),
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
        ).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(looked.disposition, NpcIntentDisposition::Ok);
        assert!(looked.look_text.starts_with("You see "), "{}", looked.look_text);
        assert_eq!((looked.sold_units, looked.total_gold_units), (0, 0));
        assert_eq!(footprint(&h).await?, empty);
        assert_eq!(carried(&h).await?, seeded);

        // SELL_ALL sells both meat stacks, 8 units for 32 gold, in one durable trade.
        let sell_all = intent(NpcShopAction::SellAll);
        let (status, sold) = exchange(&owner, &mut admitted, Some((4, 5)), 6, &sell_all).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            (sold.disposition, sold.sold_units, sold.total_gold_units),
            (NpcIntentDisposition::Ok, 8, 32)
        );
        assert_eq!(
            carried(&h).await?,
            vec![
                (FISH.to_owned(), 2),
                (crate::domain::currency::Coin::Gold.production_key().to_owned(), 32),
            ]
        );
        let after = footprint(&h).await?;
        assert_eq!((after.0, after.1, after.2), (1, 3, 1));
        let (units, gold, flag): (i64, String, bool) = sqlx::query_as(
            "SELECT quantity::bigint,(quantity*unit_price)::text,sell_all FROM game_npc_trades",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!((units, gold.as_str(), flag), (8, "32", true));

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
        assert!(matches!(again, NpcShopStep::Sold { units: 8, gold: 32 }));
        assert_eq!(footprint(&h).await?, after);

        // A new sell-all finds no meat: refused, nothing written.
        let (status, nothing) = exchange(&owner, &mut admitted, Some((7, 8)), 9, &sell_all).await?;
        assert_ne!(nothing.disposition, NpcIntentDisposition::Ok, "{status:?}");
        assert_eq!(footprint(&h).await?, after);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
