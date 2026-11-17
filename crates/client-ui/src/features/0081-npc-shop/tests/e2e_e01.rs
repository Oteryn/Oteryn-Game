//! E01 replay ownership:
//! - `apply_conversation` and `apply_window` in
//!   `crates/client-ui/src/features/0081-npc-shop/mod.rs` consume domain payloads;
//! - `apply_result` in that file consumes decoded NPC shop command replies;
//! - `DialoguePanel::apply` in `crates/client-ui/src/features/0003-npc-service/mod.rs`
//!   projects bank deposit/withdraw/transfer conversation replies.
use super::*;
use crate::npc_dialogue::DialoguePanel;

#[test]
fn e01_real_server_payloads_replay_into_native_panels() -> Result<(), Box<dyn std::error::Error>> {
    let (conversation, window, look, expected) = if let Some(root) =
        std::env::var_os("E2E_ALL_CAPTURE_DIR")
    {
        let root = std::path::Path::new(&root);
        (
            std::fs::read(root.join("e01-conversation.bin"))?,
            std::fs::read(root.join("e01-window.bin"))?,
            std::fs::read(root.join("e01-look.bin"))?,
            std::fs::read_to_string(root.join("e01-expected.txt"))?,
        )
    } else {
        let npc = EntityRef {
            identity: [7; 16],
            generation: 3,
        };
        let conversation = npc_service::NpcConversation {
            open: Some((npc, npc_service::NpcConversationState::Open)),
            lines: vec!["Welcome. {trade}, {deposit}, {withdraw}, or {transfer}?".into()],
        };
        let window = npc_service::NpcTradeWindow {
            open: Some((npc, 11)),
            offers: vec![npc_service::NpcOffer {
                item_definition_ref: 42,
                side: NpcTradeSide::Buy,
                count: 1,
                unit_price: 9,
            }],
        };
        let result = npc_shop::NpcShopResult {
            disposition: NpcIntentDisposition::Ok,
            look_text: "A sturdy backpack.".into(),
            sold_units: 0,
            total_gold_units: 0,
        };
        (
                npc_service::encode_npc_conversation(&conversation)
                    .map_err(|error| format!("{error:?}"))?,
                npc_service::encode_npc_trade_window(&window)
                    .map_err(|error| format!("{error:?}"))?,
                npc_shop::encode_npc_shop_result(&result)
                    .map_err(|error| format!("{error:?}"))?,
                "conversation_open=true\nconversation_lines=Welcome. {trade}, {deposit}, {withdraw}, or {transfer}?\noffers=1\nlook=A sturdy backpack.\n".into(),
            )
    };

    let mut service = NpcServicePanel::default();
    assert!(apply_conversation(&mut service, &conversation));
    assert!(apply_window(&mut service, &window));
    let decoded = npc_shop::decode_npc_shop_result(&look).map_err(|error| format!("{error:?}"))?;
    let mut shop = NpcShopPanel {
        selected: true,
        ..NpcShopPanel::default()
    };
    apply_result(
        &mut shop,
        &NpcShopResponse {
            status: CommandStatus::Accepted,
            result: Some(decoded),
        },
    );
    let conversation = service.conversation.as_ref().ok_or("conversation absent")?;
    let window = service.window.as_ref().ok_or("window absent")?;
    let visible = format!(
        "conversation_open={}\nconversation_lines={}\noffers={}\nlook={}\n",
        conversation.open.is_some(),
        conversation.lines.join("|"),
        window.offers.len(),
        panel_text(&shop)
    );
    assert_eq!(visible, expected);

    let decoded_conversation =
        npc_service::decode_npc_conversation(&conversation_payload(&service)?)
            .map_err(|error| format!("{error:?}"))?;
    let mut dialogue = DialoguePanel::default();
    dialogue.apply(decoded_conversation);
    assert_eq!(dialogue.text(), conversation.lines.join("\n"));

    let before = service.clone();
    assert!(!apply_window(&mut service, &[0xff, 0x00]));
    assert_eq!(service, before);
    Ok(())
}

fn conversation_payload(panel: &NpcServicePanel) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    npc_service::encode_npc_conversation(panel.conversation.as_ref().ok_or("conversation absent")?)
        .map_err(|error| format!("{error:?}").into())
}
