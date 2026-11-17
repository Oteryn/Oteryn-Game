//! S10 client replay: Rashid's weekday shop conversation and trade window reach the client as
//! `STATE_DOMAIN_NPC_CONVERSATION` / `STATE_DOMAIN_NPC_TRADE_WINDOW` deltas, consumed by
//! `Session::apply_state_delta` (`domain_dispatch.rs`) and exposed as
//! `SessionEvent::NpcConversation` / `SessionEvent::NpcTradeWindow`.
use super::*;
use crate::tests::npc_conversation_replay::{built, built_window, captured, replay};
use oteryn_protocol_oteryn::npc_service;

const NPC: [u8; 16] = [0x10; 16];

#[test]
fn e2e_s10_rashid_shop_conversation_and_window_replay() -> Result<(), BoxError> {
    let frames = if let Some(dir) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        captured("s10", std::path::Path::new(&dir))?
    } else {
        let open = Some((NPC, 2));
        let window = npc_service::NpcTradeWindow {
            open: Some((
                EntityRef {
                    identity: NPC,
                    generation: 2,
                },
                9,
            )),
            offers: vec![npc_service::NpcOffer {
                item_definition_ref: 7,
                side: NpcTradeSide::Buy,
                count: 1,
                unit_price: 15,
            }],
        };
        vec![
            built("hi", (21, 0), open, &["Welcome. Ask to {trade}."])?,
            built("trade", (23, 1), open, &["Here are my wares."])?,
            built_window("trade", (24, 0), (NPC, 2), &window)?,
        ]
    };
    assert!(
        frames
            .iter()
            .any(|frame| frame.domain == npc_service::STATE_DOMAIN_NPC_TRADE_WINDOW),
        "S10 replay lacks the shop window frame"
    );
    replay("s10", frames)
}
