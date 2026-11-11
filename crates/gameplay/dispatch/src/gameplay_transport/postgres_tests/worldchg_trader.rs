//! Real native Yasir offers through NPC talk, trade and the fenced PostgreSQL owner.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::durability::world_event::changes;
use crate::foundation::{
    ChannelContentPin, ConnectionGeneration, MovementFacing, NpcPlacement, NpcPlacementTable,
    NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::{NpcTalkBook, NpcTalkEntry};
use crate::gameplay_transport::npc_trade_command::NpcTradeStep;
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcTalkIntent, NpcTradeIntent, NpcTradeSide,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;
const YASIR: &str = "oteryn:npc.yasir";
const SAM: &str = "oteryn:npc.sam";
const LEATHER: &str = "oteryn:item.tibia.i5878";
const SWORD: &str = "oteryn:item.tibia.i3264";
#[test]
fn worldchg_trader_postgres_yasir_current_day_ship_and_receipt_gate() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("worldchg-trader", scenario)
}
pub(super) async fn day(h: &Harness, world: crate::foundation::WorldId, epoch: u64) -> TestResult {
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01') ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind([7u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active(world_id,reset_epoch) VALUES(encode($1,'hex')::uuid,$2::text::numeric) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&h.pool).await?;
    // Isolate the daily callback from unrelated live phase controllers. Legacy immutable
    // daily decisions have world_counters=false; all actual selection draws stay unchanged.
    let mut expected = changes::roll(world, epoch, 0)?;
    expected.world_counters = false;
    sqlx::query("INSERT INTO game_world_change_days(world_id,reset_epoch,decision) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3) ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(serde_json::to_value(expected)?).execute(&h.pool).await?;
    let loaded = h
        .root
        .world_changes(world, epoch)
        .await?
        .ok_or("daily snapshot")?;
    assert_eq!(
        (loaded.reset_epoch, loaded.oriental_trader),
        (epoch, expected.oriental_trader)
    );
    Ok(())
}
async fn footprint(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_npc_trades),(SELECT count(*) FROM game_npc_trade_lines),(SELECT count(*) FROM game_item_audit_outbox)").fetch_one(&h.pool).await?)
}
async fn intent(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    npc: EntityRef,
    key: &str,
    item: &str,
) -> TestResult<NpcTradeIntent> {
    let book = owner.states.chat.npc_talk().ok_or("NPC book")?;
    let entry = book.get(key).ok_or("native merchant")?;
    let runtime = owner.runtime.lock().await;
    let index = crate::gameplay_transport::item_ref_admission::pinned_index(
        &runtime,
        owner.active_generation,
    )
    .ok_or("native item index")?;
    let (offer_index, (shown, _)) =
        oteryn_gameplay_npc::npc_command::projected(entry.offers(), index)
            .enumerate()
            .find(|(_, (_, source))| {
                source.item.key == item
                    && source.direction
                        == crate::content::ProjectV2ServiceOfferDirection::BuyFromPlayer
            })
            .ok_or("native sell offer")?;
    assert_eq!(shown.side, NpcTradeSide::Sell);
    Ok(NpcTradeIntent {
        npc_actor: npc,
        catalogue_revision: book.catalogue_revision(),
        offer_index: offer_index.try_into()?,
        side: shown.side,
        quantity: 1,
        expected_unit_price: shown.unit_price,
    })
}
async fn open(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    command: UseCommand,
    npc: EntityRef,
    key: &str,
) -> TestResult {
    // Match the existing native Grizzly PG fixture: each conversation batch starts
    // beyond the four-utterance window, without clearing NPC state or weakening ingress.
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    for (offset, text) in [(0, "hi"), (1, if key == YASIR { "ariki" } else { "trade" })] {
        let reply = owner
            .npc_talk(
                actor,
                UseCommand {
                    command_id: command.command_id + offset,
                    ..command
                },
                NpcTalkIntent {
                    npc_actor: npc,
                    text: text.into(),
                },
            )
            .await;
        assert_eq!(
            reply.disposition,
            NpcIntentDisposition::Ok,
            "native merchant conversation: {text}"
        );
    }
    Ok(())
}
fn scenario() -> TestResult {
    run(async |admin| {
        let start = std::time::Instant::now();
        reset();
        let h = Harness::create_with_admission_seed(admin, "worldchg_yasir", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let active_epoch = (1..1000)
            .find(|e| changes::roll(world, *e, 0).is_ok_and(|d| d.oriental_trader.is_some()))
            .ok_or("active epoch")?;
        let selected = changes::roll(world, active_epoch, 0)?
            .oriental_trader
            .ok_or("town")?;
        let inactive_epoch = (1..1000)
            .find(|e| changes::roll(world, *e, 0).is_ok_and(|d| d.oriental_trader.is_none()))
            .ok_or("inactive epoch")?;
        let other_epoch = (1..1000)
            .find(|e| {
                changes::roll(world, *e, 0)
                    .is_ok_and(|d| d.oriental_trader.is_some_and(|t| t != selected))
            })
            .ok_or("other town epoch")?;
        let (x, y, z) = selected.yasir_position();
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("native content")?;
        // Explicit source-coordinate fixture. Existing daily-map tests prove ship terrain;
        // this case proves actual NPC placement, native offers and trade authority.
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (i32::from(x) - 1, i32::from(y), i16::from(z)),
        );
        let deps = Dependencies::with_content_and_capacity(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            3,
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
        let mut book = NpcTalkBook::from_model(&model);
        assert!(
            book.get(YASIR).is_some(),
            "production native Yasir must be admitted"
        );
        // CONTROL ONLY: Sam's native Dialogue consumer is outside this title. His explicit
        // fixture uses the canonical Sam greet and unchanged admitted Sam service offers.
        let (_, sam_source) = model
            .npcs()
            .iter()
            .find(|(key, _)| key.key == SAM)
            .ok_or("canonical Sam control")?;
        let sam_offers = sam_source
            .services
            .iter()
            .filter_map(|key| model.services().get(key))
            .flat_map(|service| service.offers.iter().cloned())
            .collect();
        book=book.with(SAM,NpcTalkEntry::new(&sam_source.name,None)
            .with_source_greeting("Welcome to my shop, adventurer |PLAYERNAME|! I {trade} with weapons and armor.")
            .with_offers(sam_offers));
        owner.states.chat.install_npc_talk(Arc::new(book));
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let (yasir, sam) = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![
                            NpcPlacements {
                                key: SAM.into(),
                                placements: vec![NpcPlacement {
                                    floor: -z,
                                    x: x - 2,
                                    y,
                                    direction: MovementFacing::South,
                                }],
                            },
                            NpcPlacements {
                                key: YASIR.into(),
                                placements: vec![NpcPlacement {
                                    floor: -z,
                                    x,
                                    y,
                                    direction: MovementFacing::South,
                                }],
                            },
                        ],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let lookup = |key| {
                runtime
                    .visible_entities()
                    .npcs
                    .iter()
                    .find(|npc| runtime.npc_key(npc.actor) == Some(key))
                    .map(|npc| EntityRef {
                        identity: npc.actor.placement_identity(),
                        generation: npc.generation,
                    })
            };
            (lookup(YASIR).ok_or("Yasir")?, lookup(SAM).ok_or("Sam")?)
        };
        super::item_look::seed_main_backpack(&h, &[(LEATHER, 3), (SWORD, 1)]).await?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?
                .0;
        let fence = item_fence_of(current).ok_or("item fence")?;
        let command = |id| UseCommand {
            game_session_id: session,
            command_id: id,
            item_fence: Some(fence),
        };
        let yasir_intent = intent(&owner, yasir, YASIR, LEATHER).await?;
        assert_eq!(
            yasir_intent.expected_unit_price, 80,
            "authored native Yasir offer"
        );
        day(&h, world, inactive_epoch).await?;
        open(&mut owner, deps.actor, command(1), yasir, YASIR).await?;
        let before = footprint(&h).await?;
        assert!(
            matches!(
                owner.npc_trade(deps.actor, command(3), yasir_intent).await,
                NpcTradeStep::Refused(_)
            ),
            "inactive world change must refuse an otherwise valid native Yasir trade"
        );
        assert_eq!(
            footprint(&h).await?,
            before,
            "inactive refusal burns/mints nothing"
        );
        day(&h, world, active_epoch).await?;
        assert!(
            matches!(
                owner.npc_trade(deps.actor, command(4), yasir_intent).await,
                NpcTradeStep::Traded
            ),
            "selected daily ship permits native Yasir trade"
        );
        let after = footprint(&h).await?;
        assert_eq!(after.0, before.0 + 1);
        day(&h, world, inactive_epoch).await?;
        assert!(
            matches!(
                owner.npc_trade(deps.actor, command(4), yasir_intent).await,
                NpcTradeStep::Traded
            ),
            "historical receipt survives later inactive day"
        );
        assert_eq!(footprint(&h).await?, after);
        day(&h, world, other_epoch).await?;
        assert!(
            matches!(
                owner.npc_trade(deps.actor, command(5), yasir_intent).await,
                NpcTradeStep::Refused(_)
            ),
            "different selected ship refuses old Yasir placement"
        );
        assert_eq!(footprint(&h).await?, after);
        day(&h, world, active_epoch).await?;
        let stale = UseCommand {
            item_fence: Some(
                crate::durability::item_transfer::CurrentCharacterItemFence {
                    connection_generation: ConnectionGeneration::new(2).map_err(debug)?,
                    ..fence
                },
            ),
            ..command(6)
        };
        assert!(
            matches!(
                owner.npc_trade(deps.actor, stale, yasir_intent).await,
                NpcTradeStep::Refused(_)
            ),
            "stale Character connection cannot trade"
        );
        assert_eq!(footprint(&h).await?, after);
        day(&h, world, inactive_epoch).await?;
        let _ = owner
            .npc_talk(
                deps.actor,
                command(7),
                NpcTalkIntent {
                    npc_actor: yasir,
                    text: "bye".into(),
                },
            )
            .await;
        let sam_intent = intent(&owner, sam, SAM, SWORD).await?;
        open(&mut owner, deps.actor, command(8), sam, SAM).await?;
        assert!(
            matches!(
                owner.npc_trade(deps.actor, command(10), sam_intent).await,
                NpcTradeStep::Traded
            ),
            "ordinary native merchant is independent of Oriental Trader"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert!(
            start.elapsed().as_secs_f64() > 1.0,
            "real PG execution required"
        );
        println!(
            "WORLDCHG-Q PG EXECUTED: native Yasir active/inactive/selected ship, stale fence, receipt replay, ordinary merchant; elapsed={:?}",
            start.elapsed()
        );
        Ok(())
    })
}
