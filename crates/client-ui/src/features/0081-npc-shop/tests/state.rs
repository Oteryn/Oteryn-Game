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
