//! MP04 market admission diagnostics. Refused prerequisites fail dependent cases.
use super::*;

pub(super) async fn run(core: &SeamClients<'_>, packets: &SeamClients<'_>) -> TestResult {
    use oteryn_dev_client::{client_runtime::market, client_ui::market as ui};
    let (mut participants, setup) =
        bots::connect_bots(&[(core, 0, None), (packets, 0, None)]).await;
    let result: TestResult = async {
        setup?;
        for (index, bot) in participants.iter_mut().enumerate() {
            let client = bot.client()?;
            let mut panel = ui::MarketPanel {
                selected: client.selected_capabilities().contains(&63),
                ..Default::default()
            };
            let command = ui::request(&mut panel, ui::Intent::Wares)
                .ok_or("market capability 63 absent")?;
            let response = market::send(client.session_mut(), &command).await?;
            let detail = format!("bot={index} reply={response:?} source=crates/gameplay/dispatch/src/gameplay_transport/market.rs:150");
            let accepted = ui::apply_result(&mut panel, &response);
            evidence(&format!("MP_CHECK MP04 open_{index} {} {detail}", if accepted { "PASS" } else { "FAIL" }));
            if !accepted {
                return Err(detail.into());
            }
        }
        Err("market transaction journey not established by opening overview".into())
    }.await;
    if let Err(error) = &result {
        for case in [
            "sell_offer",
            "partial_purchase",
            "settlement",
            "cancel_remainder",
            "buy_offer",
            "sell_into_buy",
            "history_both_sides",
            "excess_items_refused",
            "insufficient_gold_refused",
            "offer_cap_refused",
            "relog_persistence",
        ] {
            evidence(&format!("MP_CHECK MP04 {case} FAIL prerequisite={error}"));
        }
    }
    let cleanup = bots::release_all(&mut participants, core, packets).await;
    evidence(&format!(
        "MP_CHECK MP04 cleanup {} reply={cleanup:?}",
        if cleanup.is_ok() { "PASS" } else { "FAIL" }
    ));
    result.and(cleanup)
}
