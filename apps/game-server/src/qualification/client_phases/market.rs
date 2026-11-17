use super::*;
fn market_cancellation_proven(
    acknowledged: bool,
    listed: bool,
    before: u64,
    after: u64,
    fee: u64,
) -> bool {
    acknowledged && !listed && before.checked_sub(fee) == Some(after)
}

#[test]
fn accepted_market_cancel_requires_unlisted_offer_and_refunded_escrow() {
    assert!(!market_cancellation_proven(true, true, 100, 80, 20));
    assert!(!market_cancellation_proven(true, false, 100, 79, 20));
    assert!(market_cancellation_proven(true, false, 100, 80, 20));
}

pub(super) async fn market(
    client: &mut oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<()> {
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
    // Browse (action 2) names one definition; the opening overview is Wares (action 1).
    let command = ui::request(&mut panel, ui::Intent::Wares).ok_or("market request")?;
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
    let before = market_bank_balance(clients).await?;
    let fee = crate::durability::market::placement_fee(1, 1)
        .map_err(|error| format!("market fee: {error:?}"))?;
    let command = ui::request(&mut panel, ui::Intent::CreateOffer).ok_or("market create")?;
    let reply = market::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply) {
        return client_imbue::mutation_verdict(
            false,
            format!("mechanic=market status=FAIL create={:?}", reply.reply),
        );
    }
    let offer_id = reply
        .reply
        .as_ref()
        .ok_or("market create receipt missing")?
        .offer;
    let definition = panel.definition;
    let created: TestResult = async {
        let command = ui::request(&mut panel, ui::Intent::OwnOffers).ok_or("market own offers")?;
        let reply = market::send(client.session_mut(), &command).await?;
        if !ui::apply_result(&mut panel, &reply)
            || !panel.rows.iter().any(|row| {
                row.offer == offer_id
                    && row.definition == definition
                    && row.side == 1
                    && row.amount == 1
                    && row.price == 1
                    && row.state == 1
            })
        {
            return Err("market buy offer absent from current own offers".into());
        }
        if before.checked_sub(fee + 1) != Some(market_bank_balance(clients).await?) {
            return Err("market creation did not debit exactly the fee and buy escrow".into());
        }
        Ok(())
    }
    .await;

    panel.selected_offer = Some(offer_id);
    let command = ui::request(&mut panel, ui::Intent::CancelOffer)
        .ok_or("market cancel qualification offer")?;
    let reply = market::send(client.session_mut(), &command).await?;
    let acknowledged = ui::apply_result(&mut panel, &reply)
        && reply.reply.is_some_and(|reply| reply.offer == offer_id);
    let command =
        ui::request(&mut panel, ui::Intent::OwnOffers).ok_or("market refresh after cancel")?;
    let reply = market::send(client.session_mut(), &command).await?;
    if !ui::apply_result(&mut panel, &reply)
        || !market_cancellation_proven(
            acknowledged,
            panel.rows.iter().any(|row| row.offer == offer_id),
            before,
            market_bank_balance(clients).await?,
            fee,
        )
    {
        return Err(
            "market cancellation did not remove the buy offer and refund its escrow".into(),
        );
    }
    created?;
    evidence("mechanic=market buy_offer=committed debit=verified cancel=verified escrow=refunded");
    Ok(())
}

async fn market_bank_balance(clients: &SeamClients<'_>) -> TestResult<u64> {
    within_database_deadline(tokio::time::Instant::now() + Duration::from_secs(20), async {
        let mut database = sqlx::PgConnection::connect(clients.url).await?;
        let balance: Option<i64> = sqlx::query_scalar(
            "SELECT balance FROM game_account_bank_balances WHERE account_id = $1::uuid AND world_id = encode($2, 'hex')::uuid"
        ).bind(&clients.accounts[1]).bind(clients.world.as_slice()).fetch_optional(&mut database).await?;
        database.close().await?;
        Ok(u64::try_from(balance.unwrap_or(0))?)
    }).await
}
