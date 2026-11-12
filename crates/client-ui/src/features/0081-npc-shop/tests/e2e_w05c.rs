//! W05c replay ownership: `apply_conversation` and `apply_window` in
//! `crates/client-ui/src/features/0081-npc-shop/mod.rs` consume Rashid's real
//! conversation and trade-window domain payloads.
use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (Vec<u8>, Vec<u8>, String);

fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok((
            std::fs::read(root.join("w05c-conversation.bin"))?,
            std::fs::read(root.join("w05c-window.bin"))?,
            std::fs::read_to_string(root.join("w05c-expected.txt"))?,
        ));
    }
    let npc = EntityRef {
        identity: [9; 16],
        generation: 2,
    };
    let conversation = npc_service::NpcConversation {
        open: Some((npc, npc_service::NpcConversationState::Open)),
        lines: vec!["Welcome to my humble store.".into()],
    };
    let window = npc_service::NpcTradeWindow {
        open: Some((npc, 5)),
        offers: vec![
            npc_service::NpcOffer {
                item_definition_ref: 17,
                side: npc_service::NpcTradeSide::Buy,
                count: 1,
                unit_price: 30,
            },
            npc_service::NpcOffer {
                item_definition_ref: 18,
                side: npc_service::NpcTradeSide::Sell,
                count: 1,
                unit_price: 4,
            },
        ],
    };
    Ok((
        npc_service::encode_npc_conversation(&conversation).map_err(|e| format!("{e:?}"))?,
        npc_service::encode_npc_trade_window(&window).map_err(|e| format!("{e:?}"))?,
        "conversation_open=true\nconversation_lines=Welcome to my humble store.\noffers=2\n".into(),
    ))
}

#[test]
fn w05c_rashid_payloads_replay_into_service_panel() -> TestResult {
    let (conversation, window, expected) = fixture()?;
    let mut panel = NpcServicePanel::default();
    assert!(apply_conversation(&mut panel, &conversation));
    assert!(apply_window(&mut panel, &window));
    let shown_conversation = panel.conversation.as_ref().ok_or("conversation absent")?;
    let shown_window = panel.window.as_ref().ok_or("window absent")?;
    let visible = format!(
        "conversation_open={}\nconversation_lines={}\noffers={}\n",
        shown_conversation.open.is_some(),
        shown_conversation.lines.join("|"),
        shown_window.offers.len()
    );
    assert_eq!(visible, expected);
    let (actor, revision) = shown_window.open.ok_or("window closed")?;
    assert_eq!(shown_conversation.open.map(|open| open.0), Some(actor));
    assert!(revision > 0);

    let before = panel.clone();
    assert!(!apply_window(&mut panel, &[0xff, 0x00]));
    assert!(!apply_conversation(&mut panel, &[0xff, 0x00]));
    assert_eq!(panel, before);
    Ok(())
}
