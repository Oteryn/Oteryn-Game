//! C16: admitted NPC shop, bounded trades, balances, stale quotes and Sell all.
use oteryn_client::play::{PlayLink, play_channel, run_session};
#[path = "native_npc_shop/peer.rs"]
mod server;
use oteryn_protocol_oteryn::{
    ChannelId, CharacterId, CommandStatus, DomainSnapshot, GameSessionId, ServerAcceptedValue,
    WorldId, decode_wire_envelope, encode_command_result, encode_server_accepted,
    encode_single_chunk_snapshot, encode_state_delta,
    npc_service::{self, NpcConversation, NpcConversationState, NpcIntentDisposition},
    world_spatial::{ActorPosition, StepDirection},
    world_spatial_entities::{
        EntityDetail, EntityKind, EntityRef, WorldSpatialEntitiesSnapshot, WorldSpatialEntity,
        encode_world_spatial_entities_snapshot,
    },
};
use oteryn_protocol_oteryn::{
    item_view::{self, CharacterInventory, ItemEntry},
    npc_service::{NpcOffer, NpcTradeSide, NpcTradeWindow},
};
use oteryn_session::{Admission, CLIENT_SUPPORTED_CAPABILITIES, Session, read_frame, write_frame};
use server::peer;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio::sync::oneshot;

type Error = Box<dyn std::error::Error + Send + Sync>;
const WAIT: Duration = Duration::from_secs(5);

#[test]
fn native_npc_shop_scenario() -> Result<(), Error> {
    run_case(false)
}
#[test]
fn native_npc_shop_balances_scenario() -> Result<(), Error> {
    run_case(true)
}
fn run_case(gold: bool) -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(scenario(gold))
        })?
        .join()
        .map_err(|_| std::io::Error::other("shop scenario panicked"))?
}
async fn pump(link: &PlayLink, ready: impl Fn() -> bool) -> Result<(), Error> {
    tokio::time::timeout(WAIT, async {
        loop {
            while link.poll().is_some() {}
            if ready() {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
async fn scenario(gold: bool) -> Result<(), Error> {
    let (client, server) = tokio::io::duplex(64 * 1024);
    let (finish, finished) = oneshot::channel();
    let peer = tokio::spawn(peer(server, finished, gold));
    let session = Session::admit(
        client,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: CharacterId::decode(&identity(4))?,
            admission_material: b"grant",
            client_build_id: "c16-native-shop",
            supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
            deadline: WAIT,
        },
    )
    .await?;
    let (link, commands, outbox) = play_channel();
    let task = tokio::spawn(run_session(session, commands, outbox));
    pump(&link, || link.npc_shop_panel().live.selected).await?;
    link.talk_to_npc(npc().entity, "trade");
    pump(&link, || {
        let shop = link.npc_shop_panel().live;
        shop.quotes().len() == 2
            && shop.free_slots() == Some(7)
            && (!gold || shop.balances.is_some())
    })
    .await?;
    let panel = link.npc_shop_panel();
    assert!(panel.open);
    assert_eq!(panel.live.owned(42), 7);
    assert_eq!(
        panel
            .live
            .text()
            .contains("unavailable (capability not selected)"),
        !gold
    );
    if gold {
        assert!(
            panel
                .live
                .text()
                .contains("Carried gold: 100; Bank gold: 5000")
        );
    }
    let buy = panel.live.quotes()[0];
    link.npc_trade_request(buy, 101);
    assert!(!link.npc_shop_panel().live.pending);
    link.npc_trade_request(buy, 100);
    wait_inventory(&link, 107, gold, 1).await?;
    // Successful trades retain the catalogue and do not emit a talk command.
    assert_eq!(
        link.npc_shop_panel().live.window.open,
        Some((npc().entity, 10))
    );
    assert_eq!(
        link.npc_dialogue_panel().lines,
        vec!["Ask for {trade}.".to_string()]
    );
    let sell = link.npc_shop_panel().live.quotes()[1];
    link.npc_trade_request(sell, 100);
    wait_inventory(&link, 7, gold, 2).await?;
    link.npc_trade_request(sell, 3);
    pump(&link, || {
        link.npc_shop_panel().live.feedback.contains("Rejected")
            && (!gold
                || link
                    .npc_shop_panel()
                    .live
                    .balances
                    .is_some_and(|b| b.carried_gold == 103))
    })
    .await?;
    link.npc_trade_request(sell, 1);
    pump(&link, || {
        link.npc_shop_panel().live.window.open == Some((npc().entity, 20))
            && (!gold
                || link
                    .npc_shop_panel()
                    .live
                    .balances
                    .is_some_and(|b| b.carried_gold == 104))
    })
    .await?;
    link.npc_trade_request(sell, 1);
    assert!(!link.npc_shop_panel().live.pending);
    let sell = link.npc_shop_panel().live.quotes()[0];
    link.npc_trade_request(sell, 3);
    wait_inventory(&link, 4, gold, 5).await?;
    link.npc_sell_all_request(sell);
    wait_inventory(&link, 0, gold, 6).await?;
    assert!(
        oteryn_client_ui::npc_shop::panel_text(&link.npc_shop_panel()).contains("Sold 4 units")
    );
    finish.send(()).map_err(|_| "peer ended")?;
    pump(&link, || !link.npc_shop_panel().open).await?;
    tokio::time::timeout(WAIT, task).await??;
    tokio::time::timeout(WAIT, peer).await???;
    Ok(())
}
async fn wait_inventory(link: &PlayLink, count: u64, gold: bool, query: u64) -> Result<(), Error> {
    pump(link, || {
        let shop = link.npc_shop_panel();
        !shop.live.pending
            && !shop.pending
            && shop.live.owned(42) == count
            && (!gold
                || shop
                    .live
                    .balances
                    .is_some_and(|b| b.carried_gold == 100 + query))
    })
    .await
}
fn position(x: i32) -> ActorPosition {
    ActorPosition {
        x,
        y: 200,
        floor: -2,
    }
}
fn actor(x: i32) -> WorldSpatialEntity {
    WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Player,
        entity: EntityRef {
            identity: [7; 16],
            generation: 1,
        },
        position: position(x),
        detail: EntityDetail::Actor {
            direction: StepDirection::East,
            appearance_ref: 1,
            health_percent: 100,
        },
    }
}
fn identity(marker: u8) -> [u8; 16] {
    [
        1, 0x93, 0x4f, 0x10, 0x7c, 0, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}

fn npc() -> WorldSpatialEntity {
    WorldSpatialEntity {
        kind: EntityKind::Npc,
        entity: EntityRef {
            identity: [9; 16],
            generation: 2,
        },
        ..actor(101)
    }
}
