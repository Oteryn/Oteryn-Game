//! Yasir's committed catalogue through NPC_TALK/NPC_SHOP and the existing trade receipt.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::{
    NpcDataCatalogue, ProjectV2ServiceOfferDirection, npc_catalogue_preproduction_limits,
};
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
use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcTalkIntent, NpcTradeSide, decode_npc_intent_result,
    encode_npc_talk_intent,
};
use oteryn_protocol_oteryn::npc_shop::{
    NpcShopAction, NpcShopIntent, decode_npc_shop_result, encode_npc_shop_intent,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

const YASIR: &str = "oteryn:npc.yasir";
const ACHIEVEMENT: &str = "oteryn:achievement/si_ariki";

type Replies = Vec<(CommandStatus, Vec<u8>)>;

#[test]
fn oriental_trader_catalogue_ariki_sale_and_achievement_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "oriental trader thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED Oriental Trader catalogue/ariki/sale/replay elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

async fn wire(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    commands: &[(u64, u32, Vec<u8>)],
) -> TestResult<Replies> {
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let driving = async {
        loop {
            let bytes = read_frame(&mut client).await?;
            if decode_wire_envelope(&bytes).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let mut results = Vec::new();
        for (command_id, command_type, payload) in commands {
            let bytes = encode_client_command(
                before.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: *command_id,
                    command_type: *command_type,
                    payload,
                },
            )
            .map_err(debug)?;
            write_frame(&mut client, &bytes).await?;
            loop {
                let bytes = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, *command_id);
                    results.push((result.status, result.payload.to_vec()));
                    break;
                }
            }
        }
        client.shutdown().await?;
        Ok(results)
    };
    let (ended, results): (_, TestResult<Replies>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, driving),
    )
    .await?;
    match ended {
        ConnectionEnd::AdmittedThenDisconnected(current)
        | ConnectionEnd::AdmittedThenClosed(current, _) => *admitted = current,
        other => return Err(format!("oriental trader wire: {other:?}").into()),
    }
    results
}

async fn achievement(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT \
        (SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1), \
        (SELECT count(*) FROM game_account_achievement_grant_requests WHERE achievement_key=$1)",
    )
    .bind(ACHIEVEMENT)
    .fetch_one(&h.pool)
    .await?)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "oriental_trader", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        // The trade owner requires today's committed ship selection and its
        // actual source placement, even when the conversation was already open.
        let epoch = (1..1000)
            .find(|epoch| crate::durability::world_event::changes::roll(world, *epoch, 0)
                .is_ok_and(|day| day.oriental_trader.is_some()))
            .ok_or("active Oriental Trader epoch")?;
        super::worldchg_trader::day(&h, world, epoch).await?;
        let (yasir_x, yasir_y, yasir_z) = crate::durability::world_event::changes::roll(world, epoch, 0)?
            .oriental_trader.ok_or("selected Yasir town")?.yasir_position();
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        let native = active.native_gameplay().ok_or("native content")?;
        let index = native.item_index().ok_or("item index")?;
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
        deps.initialize(&owner).await;
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        let book = NpcTalkBook::from_model(&model);
        let entry = book
            .get(YASIR)
            .ok_or("committed Yasir dialogue is not admitted")?;
        let (offer_index, offer) = entry.offers().iter().enumerate().find(|(_, offer)| {
            offer.direction == ProjectV2ServiceOfferDirection::BuyFromPlayer
                && offer.currency.is_none() && offer.sub_type.is_none()
                && offer.unit_price > 0 && offer.unit_price < 100
                && offer.count.is_none_or(|count| count == 1)
                && index.definition_ref("Item", &offer.item.key, &offer.item.revision).is_some()
                && native.item_policy(&offer.item.key, &offer.item.revision).is_some_and(|policy| {
                    let admission = &policy.record().admission;
                    admission.materializable && admission.stack_class == crate::content::native_gameplay::NativeItemStackClass::StackCapable
                })
        }).ok_or("native creature-product offer")?;
        let item_key = offer.item.key.clone();
        let price = offer.unit_price;
        let offer_index = u32::try_from(
            entry.offers()[..offer_index]
                .iter()
                .filter(|offer| {
                    index
                        .definition_ref("Item", &offer.item.key, &offer.item.revision)
                        .is_some()
                })
                .count(),
        )?;
        let revision = book.catalogue_revision();
        owner.states.chat.install_npc_talk(Arc::new(book));
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_pinned_test_position(deps.actor, crate::foundation::MovementLocalPosition {
                    x: i32::from(yasir_x) - 1,
                    y: i32::from(yasir_y),
                    floor: i16::from(yasir_z),
                })
                .map_err(debug)?;
            let spot = runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: YASIR.into(),
                            placements: vec![NpcPlacement {
                                x: u16::try_from(spot.x + 1)?,
                                y: u16::try_from(spot.y)?,
                                floor: -i8::try_from(spot.floor)?,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime.visible_entities().npcs[0];
            oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        super::item_look::seed_main_backpack(&h, &[(&item_key, 2)]).await?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence =
            Some(crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[3, 81],
        )
        .ok_or("NPC capabilities")?;
        let talk = |text: &str| {
            encode_npc_talk_intent(&NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            })
            .map_err(debug)
        };
        let sell = encode_npc_shop_intent(&NpcShopIntent {
            npc_actor: npc,
            catalogue_revision: revision,
            offer_index,
            side: NpcTradeSide::Sell,
            action: NpcShopAction::SellAll,
            expected_unit_price: u32::try_from(price)?,
        })
        .map_err(debug)?;
        assert_eq!(achievement(&h).await?, (0, 0));
        let replies = wire(
            &owner,
            &mut admitted,
            &[(1, 7, talk("hi")?), (2, 7, talk("ariki")?)],
        )
        .await?;
        assert!(
            replies
                .iter()
                .all(|reply| reply.0 == CommandStatus::Accepted)
        );
        assert_eq!(
            decode_npc_intent_result(&replies[0].1).map_err(debug)?,
            NpcIntentDisposition::Ok
        );
        // Oteryn's accepted P11 boundary grants on the committed sale, never on opening UI.
        assert_eq!(achievement(&h).await?, (0, 0));
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let replies = wire(
            &owner,
            &mut admitted,
            &[
                (3, 7, talk("hi")?),
                (4, 7, talk("ariki")?),
                (5, 97, sell.clone()),
                (5, 97, sell.clone()),
            ],
        )
        .await?;
        let sale = decode_npc_shop_result(&replies[2].1).map_err(debug)?;
        assert_eq!(
            (
                replies[2].0,
                sale.disposition,
                sale.sold_units,
                sale.total_gold_units
            ),
            (
                CommandStatus::Accepted,
                NpcIntentDisposition::Ok,
                2,
                2 * price
            )
        );
        assert_eq!(
            replies[3],
            (CommandStatus::DuplicateReplay, replies[2].1.clone())
        );
        assert_eq!(achievement(&h).await?, (1, 1));
        let trades: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_npc_trades WHERE npc_key=$1")
                .bind(YASIR)
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(trades, 1);
        let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
            .bind(&item_key).fetch_one(&h.pool).await?;
        assert_eq!(remaining, 0);
        // Reconnect/replay retains the grant; a fresh empty sale cannot mint coins again.
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let replies = wire(
            &owner,
            &mut admitted,
            &[(6, 7, talk("hi")?), (7, 7, talk("trade")?), (8, 97, sell)],
        )
        .await?;
        assert_ne!(
            decode_npc_shop_result(&replies[2].1)
                .map_err(debug)?
                .disposition,
            NpcIntentDisposition::Ok
        );
        assert_eq!(achievement(&h).await?, (1, 1));
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
