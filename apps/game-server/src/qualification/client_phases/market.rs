use super::*;
pub(super) async fn market(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::market, client_ui::market as ui};
    if !client.selected_capabilities().contains(&63) {
        evidence(
            "mechanic=market skipped=capability_63_not_selected source=crates/gameplay/dispatch/src/gameplay_transport/world_required_capabilities.rs:126",
        );
        return Ok(());
    }
    let mut panel = ui::MarketPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Browse).ok_or("market request")?;
    let reply = market::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) {
        return Err("market reducer rejected browse".into());
    }
    evidence("mechanic=market client_codec=real reducer=real state=loaded");
    let Some(item) = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .first()
                .or(inventory.main_backpack.as_ref())
        })
        .copied()
    else {
        evidence(
            "mechanic=market skipped=no_item_definition_in_current_inventory source=crates/gameplay/dispatch/src/gameplay_transport/item_ref_admission.rs:373",
        );
        return Ok(());
    };
    panel.definition = item.item_definition_ref.get();
    panel.sell = false;
    let command = ui::request(&mut panel, ui::Intent::CreateOffer).ok_or("market create")?;
    let reply = market::send(client.session_mut(), &command).await?;
    let applied = ui::apply_result(&mut panel, &reply);
    if !applied {
        let code = reply
            .reply
            .as_ref()
            .ok_or("market create returned no decoded reply")?
            .code;
        let reason = match code {
            2 => "insufficient_gold_for_buy_offer_and_fee",
            11 => "no_reachable_locker",
            12 => "premium_required",
            13 => "character_is_junior",
            14 => "inventory_definition_is_not_marketable",
            _ => return Err(format!("unexpected market create refusal: {code}").into()),
        };
        evidence(&format!(
            "mechanic=market skipped={reason} code={code} definition_ref={} source=crates/client-ui/src/features/0063-market/protocol.rs:132",
            panel.definition
        ));
        return Ok(());
    }
    let offer_id = reply
        .reply
        .as_ref()
        .ok_or("market create receipt missing")?
        .offer;
    let command = ui::request(&mut panel, ui::Intent::OwnOffers).ok_or("market own offers")?;
    let reply = market::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply)
        || !panel.rows.iter().any(|row| {
            row.offer == offer_id
                && row.definition == panel.definition
                && row.amount == 1
                && row.price == 1
        })
    {
        return Err("market created offer absent from current own offers".into());
    }
    evidence("mechanic=market buy_offer=committed current_own_offers=contains_created_offer");
    panel.selected_offer = Some(offer_id);
    let command = ui::request(&mut panel, ui::Intent::CancelOffer)
        .ok_or("market cancel qualification offer")?;
    if !ui::apply_result(
        &mut panel,
        &market::send(client.session_mut(), &command).await?,
    ) {
        return Err("market cleanup cancellation refused".into());
    }
    Ok(())
}
