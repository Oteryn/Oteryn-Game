use super::*;
use npc_shop::{NpcShopAction, NpcShopIntent, NpcShopResult};
fn value() -> NpcShopIntent {
    NpcShopIntent {
        npc_actor: EntityRef {
            identity: [7; 16],
            generation: 3,
        },
        catalogue_revision: 4,
        offer_index: 2,
        side: NpcTradeSide::Sell,
        action: NpcShopAction::Look,
        expected_unit_price: 12,
    }
}
#[test]
fn request_selects_every_wire_action_and_guards_pending() {
    let mut panel = NpcShopPanel {
        selected: true,
        ..Default::default()
    };
    assert_eq!(
        request(&mut panel, Intent::SellAll(value())).map(|v| v.0.action),
        Some(NpcShopAction::SellAll)
    );
    assert!(request(&mut panel, Intent::Look(value())).is_none());
}
#[test]
fn replies_render_state_and_refusals() {
    let mut panel = NpcShopPanel {
        selected: true,
        pending: true,
        ..Default::default()
    };
    apply_result(
        &mut panel,
        &NpcShopResponse {
            status: CommandStatus::Accepted,
            result: Some(NpcShopResult {
                disposition: NpcIntentDisposition::Ok,
                look_text: "A fine item".into(),
                sold_units: 0,
                total_gold_units: 0,
            }),
        },
    );
    assert_eq!(panel_text(&panel), "A fine item");
    apply_result(
        &mut panel,
        &NpcShopResponse {
            status: CommandStatus::Accepted,
            result: Some(NpcShopResult::refused(NpcIntentDisposition::Stale)),
        },
    );
    assert!(panel_text(&panel).contains("stale"));
}

#[test]
fn npc_prerequisite_reducer_uses_live_quote_and_clears_closed_window() -> Result<(), String> {
    let npc = EntityRef {
        identity: [1; 16],
        generation: 9,
    };
    let mut panel = NpcServicePanel::default();
    let window = npc_service::NpcTradeWindow {
        open: Some((npc, 42)),
        offers: vec![npc_service::NpcOffer {
            item_definition_ref: 8,
            side: NpcTradeSide::Buy,
            count: 1,
            unit_price: 17,
        }],
    };
    assert!(apply_window(
        &mut panel,
        &npc_service::encode_npc_trade_window(&window).map_err(|e| format!("{e:?}"))?
    ));
    let request = buy_request(&panel, 0).ok_or("buy")?;
    assert_eq!(
        (
            request.npc_actor,
            request.catalogue_revision,
            request.expected_unit_price
        ),
        (npc, 42, 17)
    );
    assert!(buy_request(&panel, 1).is_none());
    assert!(!apply_window(&mut panel, &[255]));
    assert_eq!(panel.window.as_ref(), Some(&window));
    assert!(apply_conversation(
        &mut panel,
        &npc_service::encode_npc_conversation(&Default::default()).map_err(|e| format!("{e:?}"))?
    ));
    assert!(panel.window.is_none());
    assert!(buy_request(&panel, 0).is_none());
    Ok(())
}
