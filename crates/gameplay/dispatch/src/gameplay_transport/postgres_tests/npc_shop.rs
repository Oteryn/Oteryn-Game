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
const RASHID: &str = "oteryn:npc.rashid";

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
    run_shop_scenario(false)
}

pub(super) fn run_shop_scenario(wandering_rashid: bool) -> TestResult {
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
        let active = controller.active().ok_or("active content")?;
        let destination = oteryn_gameplay_entities::rashid_schedule::destination_at(
            crate::gameplay_transport::npc_trade_command::unix_ms_now(),
        ).placement;
        let entry = if wandering_rashid {
            (i32::from(destination.x), i32::from(destination.y), -i16::from(destination.floor))
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
        // Rashid's captured source buys white cloth at 150 gold, after the quest.
        // Alice retains the synthetic four-gold meat offer for the generic case.
        let sold_item = if wandering_rashid { "oteryn:item.tibia.i5917" } else { MEAT };
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
            let placed = runtime.visible_entities().npcs.into_iter().find(|npc| {
                runtime.npc_key(npc.actor) == Some(if wandering_rashid { RASHID } else { TRADER })
            }).ok_or("shop actor absent")?;
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        // Two meat stacks and a fish the trader does not buy.
        seed_main_backpack(&h, &[(sold_item, 5), (FISH, 2), (sold_item, 3)]).await?;
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
        ).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(looked.disposition, NpcIntentDisposition::Ok);
        assert!(looked.look_text.starts_with("You see "), "{}", looked.look_text);
        assert_eq!((looked.sold_units, looked.total_gold_units), (0, 0));
        assert_eq!(footprint(&h).await?, empty);
        assert_eq!(carried(&h).await?, seeded);

        if wandering_rashid {
            use oteryn_gameplay_quests::quest::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition};
            use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
            let quest = "oteryn:quest.the_travelling_trader_quest";
            let track = "oteryn:quest-progress/chosen/travelling-trader/stage";
            let key = "oteryn:quest-transition/fixture/rashid-shop-access";
            let refused = owner.npc_shop(deps.actor, UseCommand { game_session_id: session, command_id: 899, item_fence: Some(fence) }, intent(NpcShopAction::SellAll)).await;
            assert_eq!(refused.result().disposition, NpcIntentDisposition::Rejected);
            assert_eq!(footprint(&h).await?, empty);
            assert_eq!(carried(&h).await?, seeded);
            // Only the prior quest is seeded; shop authority, payment and replay
            // still run through their production dispatch and locked item owner.
            let catalogue = Arc::new(QuestStateCatalogue::new("content-1", vec![QuestTrack {
                key: track.into(), quest: quest.into(), initial: -1, min: -1, max: 13,
            }], vec![QuestTransition {
                world_change: None, key: key.into(), quest: quest.into(), completes: true, experience: None,
                effects: vec![QuestEffect { track: track.into(), from: QuestComparison::Eq(-1), effect: QuestEffectKind::Set(13) }],
            }]).map_err(debug)?);
            let quest_fence = owner.current_quest_fence(session).await.map_err(|()| "quest fence")?.ok_or("quest fence")?;
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let completed = slot.commit_quest_transition(owner.root, owner.character, owner.holder, quest_fence,
                QuestTransitionRequest { transition_key: key.into(), cause: QuestCause::Command(crate::foundation::CommandRef::new(session, crate::foundation::CommandId::new(900).map_err(debug)?)) }, catalogue).await.map_err(debug)?;
            assert!(matches!(completed, QuestTransitionOutcome::Committed(_)), "{completed:?}");
        }

        // SELL_ALL sells both stacks, eight units in one durable trade.
        let sell_all = intent(NpcShopAction::SellAll);
        let (status, sold) = exchange(&owner, &mut admitted, Some((4, 5)), 6, &sell_all).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            (sold.disposition, sold.sold_units, sold.total_gold_units),
            (NpcIntentDisposition::Ok, 8, total_gold)
        );
        assert_eq!(
            carried(&h).await?,
            vec![
                (FISH.to_owned(), 2),
                (if wandering_rashid { crate::domain::currency::Coin::Platinum } else { crate::domain::currency::Coin::Gold }.production_key().to_owned(), if wandering_rashid { 12 } else { 32 }),
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
