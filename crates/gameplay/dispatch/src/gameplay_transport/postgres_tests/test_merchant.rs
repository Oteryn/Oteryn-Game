//! TESTNPC: the opt-in pack, real command 7 categories and a command 8 gem purchase.
use super::fixture::{self, Harness, debug, id, run};
use super::item_look::seed_main_backpack;
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::npc_catalogue::test_merchant::{self, NPC};
use crate::foundation::{
    ChannelContentPin, MessageType, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
    decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::npc_service::{
    CAPABILITY_NPC_SERVICE_V1, COMMAND_TYPE_NPC_TALK_INTENT, COMMAND_TYPE_NPC_TRADE_INTENT,
    NpcIntentDisposition, NpcTalkIntent, NpcTradeIntent, NpcTradeSide, decode_npc_intent_result,
    encode_npc_talk_intent, encode_npc_trade_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
const GEM: &str = "oteryn:item.tibia.i44604";

#[test]
fn test_merchant_command_8_buys_greater_gem_for_one_gold() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "test merchant PostgreSQL thread panicked")??;
    Ok(())
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    category: &str,
    command_id: u64,
    intent: &NpcTradeIntent,
) -> TestResult<(CommandStatus, NpcIntentDisposition)> {
    let mut commands = Vec::new();
    for (talk_id, text) in [(1, "hi"), (2, "trade"), (3, category)] {
        commands.push((
            talk_id,
            COMMAND_TYPE_NPC_TALK_INTENT,
            encode_npc_talk_intent(&NpcTalkIntent {
                npc_actor: intent.npc_actor,
                text: text.into(),
            })
            .map_err(debug)?,
        ));
    }
    commands.push((
        command_id,
        COMMAND_TYPE_NPC_TRADE_INTENT,
        encode_npc_trade_intent(intent).map_err(debug)?,
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
                    if *command_type == COMMAND_TYPE_NPC_TRADE_INTENT {
                        reply = Some((
                            result.status,
                            decode_npc_intent_result(result.payload).map_err(debug)?,
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
    let (ended, result): (_, TestResult<(CommandStatus, NpcIntentDisposition)>) =
        tokio::time::timeout(
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

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "test_merchant_wire", true, false).await?;
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
        let deps = Dependencies::with_content_and_capacity(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            2,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        let book = NpcTalkBook::default()
            .with_test_merchant(true)
            .map_err(debug)?;
        let entry = book.get(NPC).ok_or("test merchant absent")?;
        let category = test_merchant::pack()
            .map_err(debug)?
            .categories
            .iter()
            .find(|category| {
                entry.offers().iter().any(|offer| {
                    offer.item.key == GEM && test_merchant::contains(Some(&category.keyword), offer)
                })
            })
            .ok_or("greater gem category")?;
        let revision = book.trade_revision(NPC, Some(&category.keyword));
        let offers = entry
            .offers()
            .iter()
            .filter(|offer| test_merchant::contains(Some(&category.keyword), offer))
            .cloned()
            .collect::<Vec<_>>();
        owner.states.chat.install_npc_talk(Arc::new(book));
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let (npc, offer_index) = {
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
                            key: NPC.into(),
                            placements: vec![NpcPlacement {
                                floor: -i8::try_from(spot.floor)?,
                                x: u16::try_from(spot.x + 1)?,
                                y: u16::try_from(spot.y)?,
                                direction: MovementFacing::North,
                            }],
                        }],
                    },
                    |key| key == NPC,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime
                .visible_entities()
                .npcs
                .into_iter()
                .find(|placed| runtime.npc_key(placed.actor) == Some(NPC))
                .ok_or("merchant actor")?;
            let index =
                crate::gameplay_transport::item_ref_admission::pinned_index(&runtime, Some(active))
                    .ok_or("item index")?;
            let (offer_index, (shown, _)) =
                oteryn_gameplay_npc::npc_command::projected(&offers, index)
                    .enumerate()
                    .find(|(_, (_, source))| source.item.key == GEM)
                    .ok_or("projected gem")?;
            assert_eq!(shown.side, NpcTradeSide::Buy);
            assert_eq!(shown.unit_price, 1);
            (
                EntityRef {
                    identity: placed.actor.placement_identity(),
                    generation: placed.generation,
                },
                u32::try_from(offer_index)?,
            )
        };
        seed_main_backpack(&h, &[("oteryn:item.tibia.i3031", 1)]).await?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(item_fence_of(current).ok_or("item fence")?);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[CAPABILITY_NPC_SERVICE_V1],
        )
        .ok_or("NPC capability")?;
        let intent = NpcTradeIntent {
            npc_actor: npc,
            catalogue_revision: revision,
            offer_index,
            side: NpcTradeSide::Buy,
            quantity: 1,
            expected_unit_price: 1,
        };
        assert_eq!(
            exchange(&owner, &mut admitted, &category.keyword, 4, &intent).await?,
            (CommandStatus::Accepted, NpcIntentDisposition::Ok)
        );
        let (trades, cost, gems): (i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_npc_trades), (SELECT sum(quantity*unit_price)::bigint FROM game_npc_trades), (SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i44604' AND lifecycle=1)")
            .fetch_one(&h.pool).await?;
        assert_eq!((trades, cost, gems), (1, 1, 1));
        let gold: i64 = sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3031' AND lifecycle=1").fetch_one(&h.pool).await?;
        assert_eq!(gold, 0, "the only gold coin was spent");
        Ok(())
    })
}
