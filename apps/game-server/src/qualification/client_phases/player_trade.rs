use super::*;
pub(super) async fn player_trade(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime::player_trade, client_ui::player_trade as ui};
    if !client.selected_capabilities().contains(&60) {
        evidence(
            "mechanic=player_trade skipped=capability_60_not_selected source=crates/gameplay/dispatch/src/gameplay_transport/world_required_capabilities.rs:126",
        );
        return Ok(());
    }
    let mut panel = ui::PlayerTradePanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Cancel).ok_or("player trade request")?;
    let reply = player_trade::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if panel.pending || panel.state.is_none() {
        return Err("player trade reducer did not load state".into());
    }
    evidence(
        "mechanic=player_trade cancel=decoded skipped=fixture_phase_has_no_second_client source=apps/game-server/src/qualification/client_phases/dev_client.rs:177",
    );
    Ok(())
}

pub(super) async fn pair(
    client: &mut oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<()> {
    use oteryn_dev_client::{
        client_runtime::player_trade as runtime, client_ui::player_trade as ui,
    };
    let Some(item) = client
        .inventory()
        .and_then(|inventory| inventory.entries.first())
        .copied()
    else {
        evidence(
            "mechanic=player_trade skipped=no_carried_offer_item_in_platform_created_character source=crates/gameplay/dispatch/src/gameplay_transport/item_ref_admission.rs:373",
        );
        return Ok(());
    };
    let mut partner = client_world::connect_character(clients, 0x74, 0).await?;
    let Some(counter) = partner
        .inventory()
        .and_then(|inventory| inventory.entries.first())
        .copied()
    else {
        let (result, logout, attempts, busy_timed_out) =
            client_world::logout_retrying_busy(&mut partner).await?;
        if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("trade partner logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
        }
        evidence(
            "mechanic=player_trade skipped=second_platform_created_character_has_no_carried_offer_item source=crates/gameplay/dispatch/src/gameplay_transport/item_ref_admission.rs:373",
        );
        return Ok(());
    };
    let own = client
        .world_entities()
        .and_then(|entities| {
            entities
                .iter()
                .find(|entity| &entity.entity.identity == entities.own_identity())
        })
        .map(|entity| entity.entity)
        .ok_or("trade own actor")?;
    let other = partner
        .world_entities()
        .and_then(|entities| {
            entities
                .iter()
                .find(|entity| &entity.entity.identity == entities.own_identity())
        })
        .map(|entity| entity.entity)
        .ok_or("trade partner actor")?;
    let before = client.inventory().cloned().ok_or("trade own inventory")?;
    let counter_before = partner
        .inventory()
        .cloned()
        .ok_or("trade counter inventory")?;
    let mut panel = ui::PlayerTradePanel {
        selected: true,
        ..Default::default()
    };
    let mut other_panel = ui::PlayerTradePanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(
        &mut panel,
        ui::Intent::Offer {
            partner: other,
            item: item.handle.get(),
        },
    )
    .ok_or("trade offer")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if reply.result != ui::player_trade::ResultCode::Offered {
        return Err(format!("trade offer: {:?}", reply.result).into());
    }
    ui::apply_result(&mut panel, &reply);
    // Each peer consumes the other writer's unsolicited state before sending its next command.
    partner.session_mut().read_gated_delta(26, 1).await?;
    let command = ui::request(
        &mut other_panel,
        ui::Intent::Offer {
            partner: own,
            item: counter.handle.get(),
        },
    )
    .ok_or("trade counter offer")?;
    let reply = runtime::send(partner.session_mut(), &command).await?;
    if reply.result != ui::player_trade::ResultCode::Offered {
        return Err(format!("trade counter: {:?}", reply.result).into());
    }
    ui::apply_result(&mut other_panel, &reply);
    let delta = client.session_mut().read_gated_delta(26, 1).await?;
    panel.state = Some(
        ui::player_trade::decode_state(&delta.payload)
            .map_err(|error| format!("trade update: {error:?}"))?,
    );
    let command = ui::request(&mut panel, ui::Intent::Accept).ok_or("trade accept")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    if reply.result != ui::player_trade::ResultCode::Accepted {
        return Err(format!("trade accept: {:?}", reply.result).into());
    }
    ui::apply_result(&mut panel, &reply);
    let delta = partner.session_mut().read_gated_delta(26, 1).await?;
    other_panel.state = Some(
        ui::player_trade::decode_state(&delta.payload)
            .map_err(|error| format!("trade counter update: {error:?}"))?,
    );
    let command =
        ui::request(&mut other_panel, ui::Intent::Accept).ok_or("trade counter accept")?;
    let reply = runtime::send(partner.session_mut(), &command).await?;
    if reply.result != ui::player_trade::ResultCode::Completed {
        return Err(format!("trade complete: {:?}", reply.result).into());
    }
    ui::apply_result(&mut other_panel, &reply);
    // The initiator receives the completion state and both receive inventory deltas.
    client.session_mut().read_gated_delta(26, 1).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while client.inventory() == Some(&before) || partner.inventory() == Some(&counter_before) {
        if tokio::time::Instant::now() >= deadline {
            return Err("completed trade did not change both inventories".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
        partner.service_liveness(Duration::from_millis(20)).await?;
    }
    if client.inventory().is_none_or(|inventory| {
        !inventory
            .entries
            .iter()
            .any(|entry| entry.item_definition_ref == counter.item_definition_ref)
    }) || partner.inventory().is_none_or(|inventory| {
        !inventory
            .entries
            .iter()
            .any(|entry| entry.item_definition_ref == item.item_definition_ref)
    }) {
        return Err("completed trade inventories lack exchanged definitions".into());
    }
    evidence(
        "mechanic=player_trade completion=committed both_inventories=changed exchanged_definitions=present",
    );
    let (result, logout, attempts, busy_timed_out) =
        client_world::logout_retrying_busy(&mut partner).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("trade partner logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    Ok(())
}
