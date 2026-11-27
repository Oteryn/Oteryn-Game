use super::fixture::{Harness, debug, id, run};
use super::item_look_fixtures::seed_main_backpack;
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements, WorldId,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_fence_of;
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::npc_trade_command::NpcTradeStep;
use oteryn_protocol_oteryn::npc_service::{
    NpcConversationState, NpcIntentDisposition, NpcTalkIntent, NpcTradeIntent, NpcTradeSide,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const ALEXANDER: &str = "oteryn:npc.alexander";
const ANIMATE_DEAD_RUNE: &str = "oteryn:item.tibia.i3203";

#[test]
fn alexander_greets_opens_trade_and_sells_animate_dead_rune() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("alexander-shop", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "alexander_shop", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
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
        deps.initialize(&owner).await;

        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        owner
            .states
            .chat
            .install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
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
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: ALEXANDER.into(),
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
            let entities = runtime.visible_entities();
            let placed = entities
                .npcs
                .iter()
                .find(|npc| runtime.npc_key(npc.actor) == Some(ALEXANDER))
                .ok_or("Alexander was not placed")?;
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        seed_main_backpack(&h, &[("oteryn:item.tibia.i3035", 4)]).await?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?
                .0;
        let fence = item_fence_of(current).ok_or("item fence")?;
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence),
        };
        let talk = |command_id, text: &str| {
            owner.npc_talk(
                deps.actor,
                command(command_id),
                NpcTalkIntent {
                    npc_actor: npc,
                    text: text.into(),
                },
            )
        };
        let greeting = talk(1, "hi").await;
        assert_eq!(
            greeting.disposition,
            NpcIntentDisposition::Ok,
            "Alexander: hi"
        );
        let conversation = greeting.conversation.ok_or("Alexander did not greet")?;
        assert_eq!(conversation.open, Some((npc, NpcConversationState::Open)));
        assert!(
            conversation
                .lines
                .iter()
                .any(|line| line.contains("Fixture Hero"))
        );

        let trade = talk(2, "trade").await;
        assert_eq!(
            trade.disposition,
            NpcIntentDisposition::Ok,
            "Alexander: trade"
        );
        let window = trade
            .window
            .ok_or("Alexander did not show a trade window")?;
        let (seller, catalogue_revision) = window.open.ok_or("Alexander's window is closed")?;
        assert_eq!(seller, npc);
        let item_ref = active
            .native_gameplay()
            .and_then(|native| native.item_index())
            .and_then(|index| index.definition_ref("Item", ANIMATE_DEAD_RUNE, "definition-r1"))
            .ok_or("Animate Dead Rune definition")?;
        let (offer_index, offer) = window
            .offers
            .iter()
            .enumerate()
            .find(|(_, offer)| {
                offer.side == NpcTradeSide::Buy && offer.item_definition_ref == item_ref.get()
            })
            .ok_or("Alexander did not offer Animate Dead Rune")?;

        let purchase = owner
            .npc_trade(
                deps.actor,
                command(3),
                NpcTradeIntent {
                    npc_actor: npc,
                    catalogue_revision,
                    offer_index: u32::try_from(offer_index)?,
                    side: offer.side,
                    quantity: 1,
                    expected_unit_price: offer.unit_price,
                },
            )
            .await;
        assert!(matches!(purchase, NpcTradeStep::Traded));
        let quantity: i64 = sqlx::query_scalar(
            "SELECT quantity::bigint FROM game_item_container_entries e
             JOIN game_item_instances i USING (item_instance_id)
             WHERE e.parent_item_instance_id = encode($1,'hex')::uuid
             AND i.definition_production_key = $2 AND i.lifecycle = 1 LIMIT 1",
        )
        .bind(id(70).as_slice())
        .bind(ANIMATE_DEAD_RUNE)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(quantity, 1);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
