use super::*;
pub(super) async fn npc_shop(_client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    evidence(
        "mechanic=npc_shop skipped=fixture_phase_has_no_authored_shop_target content_id=oteryn:npc.sam source=apps/game-server/src/qualification/client_phases/dev_client.rs:179",
    );
    Ok(())
}

pub(super) async fn transact(
    client: &mut oteryn_dev_client::DevClientSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    clients: &SeamClients<'_>,
) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::npc_shop, client_ui::npc_shop as ui};
    use ui::npc_service as wire;
    for capability in [3, 81] {
        if !client.selected_capabilities().contains(&capability) {
            return Err(format!("shop prerequisite {capability} not selected").into());
        }
    }
    let mut service = ui::NpcServicePanel::default();
    let greeting = wire::NpcTalkIntent {
        npc_actor: npc,
        text: "hi".into(),
    };
    let disposition = npc_shop::talk(client.session_mut(), &greeting).await?;
    if disposition != ui::NpcIntentDisposition::Ok {
        return Err(format!("authored shop greeting: {disposition:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    if !ui::apply_conversation(&mut service, &delta.payload)
        || service
            .conversation
            .as_ref()
            .is_none_or(|conversation| conversation.open.is_none())
    {
        return Err("NPC greeting did not open a conversation".into());
    }
    let disposition = npc_shop::talk(
        client.session_mut(),
        &wire::NpcTalkIntent {
            npc_actor: npc,
            text: "trade".into(),
        },
    )
    .await?;
    if disposition != ui::NpcIntentDisposition::Ok {
        return Err(format!("authored trade dialogue: {disposition:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    if !ui::apply_window(&mut service, &delta.payload) {
        return Err("NPC shop reducer rejected window".into());
    }
    let window = service.window.as_ref().ok_or("NPC window absent")?;
    let index = window
        .offers
        .iter()
        .enumerate()
        .filter(|(_, offer)| offer.side == ui::NpcTradeSide::Buy)
        .min_by_key(|(_, offer)| offer.unit_price)
        .map(|(index, _)| index)
        .ok_or("authored shop has no buy offer")?;
    let request = ui::buy_request(&service, index).ok_or("shop buy request")?;
    let definition = window.offers[index].item_definition_ref;
    let count = window.offers[index].count;
    let mut panel = ui::NpcShopPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(
        &mut panel,
        ui::Intent::Look(npc_shop::npc_shop::NpcShopIntent {
            npc_actor: request.npc_actor,
            catalogue_revision: request.catalogue_revision,
            offer_index: request.offer_index,
            side: request.side,
            action: npc_shop::npc_shop::NpcShopAction::Look,
            expected_unit_price: request.expected_unit_price,
        }),
    )
    .ok_or("shop look")?;
    ui::apply_result(
        &mut panel,
        &npc_shop::send(client.session_mut(), &command).await?,
    );
    if panel.feedback.is_some()
        || panel
            .result
            .as_ref()
            .is_none_or(|result| result.look_text.is_empty())
    {
        return Err("shop look did not describe real offer".into());
    }
    evidence("mechanic=npc_shop conversation=open trade_window=decoded look=real_offer");
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let before_gold = gold(&mut connection, clients).await?;
    if before_gold < i64::from(request.expected_unit_price) {
        evidence(&format!(
            "mechanic=npc_shop skipped=insufficient_gold_for_authored_buy_offer definition_ref={definition} price={} available={before_gold} content_id=oteryn:service.trade.sam source=crates/gameplay/durability/src/durability/npc_trade.rs:780",
            request.expected_unit_price
        ));
        return Ok(());
    }
    let units = |client: &oteryn_dev_client::DevClientSession| -> u64 {
        client.inventory().map_or(0, |inventory| {
            inventory
                .entries
                .iter()
                .filter(|entry| entry.item_definition_ref.get() == definition)
                .map(|entry| u64::from(entry.count.get()))
                .sum()
        })
    };
    let before = units(client);
    let disposition = npc_shop::buy(client.session_mut(), &request).await?;
    if disposition != ui::NpcIntentDisposition::Ok {
        return Err(format!("funded shop buy refused: {disposition:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while units(client) != before + u64::from(count) {
        if tokio::time::Instant::now() >= deadline {
            return Err("shop buy did not update current inventory".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    let after_gold = gold(&mut connection, clients).await?;
    if after_gold != before_gold - i64::from(request.expected_unit_price) {
        return Err("shop buy gold delta disagrees with quoted price".into());
    }
    evidence(
        "mechanic=npc_shop buy=committed inventory_units=increased gold=decreased_by_quoted_price",
    );
    Ok(())
}

async fn gold(connection: &mut sqlx::PgConnection, clients: &SeamClients<'_>) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT sum(i.quantity * CASE i.definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END)::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1),0) + COALESCE((SELECT balance FROM game_account_bank_balances WHERE account_id=$2::uuid AND world_id=encode($3,'hex')::uuid),0)")
        .bind(clients.characters[1].as_slice()).bind(&clients.accounts[1]).bind(clients.world.as_slice()).fetch_one(connection).await?)
}
