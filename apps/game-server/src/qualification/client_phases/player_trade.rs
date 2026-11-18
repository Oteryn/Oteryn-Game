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

use oteryn_protocol_oteryn::item_view::{CharacterInventory, ItemEntry};

fn received_offer(
    before: &CharacterInventory,
    after: &CharacterInventory,
    sent: ItemEntry,
    incoming: ItemEntry,
) -> bool {
    let defined_count = |inventory: &CharacterInventory, definition| {
        inventory
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref == definition)
            .map(|entry| i64::from(entry.count.get()))
            .sum::<i64>()
    };
    !after
        .entries
        .iter()
        .chain(after.equipment.iter().map(|entry| &entry.item))
        .any(|entry| entry.handle == sent.handle)
        && after.entries.iter().any(|entry| {
            *entry
                == ItemEntry {
                    handle: entry.handle,
                    ..incoming
                }
                && before
                    .entries
                    .iter()
                    .chain(before.equipment.iter().map(|entry| &entry.item))
                    .all(|old| old.handle != entry.handle)
        })
        && [sent.item_definition_ref, incoming.item_definition_ref]
            .into_iter()
            .all(|definition| {
                let debit =
                    i64::from(sent.count.get()) * i64::from(sent.item_definition_ref == definition);
                let credit = i64::from(incoming.count.get())
                    * i64::from(incoming.item_definition_ref == definition);
                defined_count(after, definition) - defined_count(before, definition)
                    == credit - debit
            })
}

#[test]
fn trade_requires_each_offered_handle_to_leave_and_a_new_counter_offer() -> TestResult {
    let sent = ItemEntry {
        appearance_id: 4,
        handle: std::num::NonZeroU64::MIN,
        item_definition_ref: std::num::NonZeroU32::MIN,
        count: std::num::NonZeroU32::MIN,
        sub_type: 0,
    };
    let incoming = ItemEntry {
        handle: std::num::NonZeroU64::new(2).ok_or("incoming handle")?,
        ..sent
    };
    let before = CharacterInventory {
        entries: vec![sent, incoming],
        ..Default::default()
    };
    assert!(!received_offer(&before, &before, sent, incoming));
    let new = ItemEntry {
        handle: std::num::NonZeroU64::new(3).ok_or("new handle")?,
        ..incoming
    };
    let after = CharacterInventory {
        entries: vec![incoming, new],
        ..Default::default()
    };
    assert!(received_offer(&before, &after, sent, incoming));
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
        return Err("core trade has no carried offer item".into());
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
        return Err("core trade partner has no carried counter-offer item".into());
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
    let offered_instances = [
        client_world::observed_item_instance(
            *client.join_snapshot().game_session_id.as_bytes(),
            item.handle,
        )
        .await?
        .ok_or("offered handle is not bound to a durable Instance")?,
        client_world::observed_item_instance(
            *partner.join_snapshot().game_session_id.as_bytes(),
            counter.handle,
        )
        .await?
        .ok_or("counter-offer handle is not bound to a durable Instance")?,
    ];
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
        return Err(format!("trade offer: {:?} reply={reply:?}", reply.result).into());
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
        return Err(format!("trade counter: {:?} reply={reply:?}", reply.result).into());
    }
    ui::apply_result(&mut other_panel, &reply);
    let delta = client.session_mut().read_gated_delta(26, 1).await?;
    panel.state = Some(
        ui::player_trade::decode_state(&delta.payload)
            .map_err(|error| format!("trade update: {error:?}"))?,
    );
    let command = ui::request(&mut panel, ui::Intent::Accept).ok_or("trade accept")?;
    let first_accept = client.next_command_id();
    let reply = runtime::send(client.session_mut(), &command).await?;
    if reply.result != ui::player_trade::ResultCode::Accepted {
        return Err(format!("trade accept: {:?} reply={reply:?}", reply.result).into());
    }
    ui::apply_result(&mut panel, &reply);
    let delta = partner.session_mut().read_gated_delta(26, 1).await?;
    other_panel.state = Some(
        ui::player_trade::decode_state(&delta.payload)
            .map_err(|error| format!("trade counter update: {error:?}"))?,
    );
    let command =
        ui::request(&mut other_panel, ui::Intent::Accept).ok_or("trade counter accept")?;
    let second_accept = partner.next_command_id();
    let reply = runtime::send(partner.session_mut(), &command).await?;
    if reply.result != ui::player_trade::ResultCode::Completed {
        return Err(format!("trade complete: {:?} reply={reply:?}", reply.result).into());
    }
    ui::apply_result(&mut other_panel, &reply);
    // The initiator receives the completion state and both receive inventory deltas.
    client.session_mut().read_gated_delta(26, 1).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !client
        .inventory()
        .is_some_and(|after| received_offer(&before, after, item, counter))
        || !partner
            .inventory()
            .is_some_and(|after| received_offer(&counter_before, after, counter, item))
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("completed trade did not transfer both offered stacks".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
        partner.service_liveness(Duration::from_millis(20)).await?;
    }
    let accepts = [
        (
            *client.join_snapshot().game_session_id.as_bytes(),
            first_accept,
            item,
        ),
        (
            *partner.join_snapshot().game_session_id.as_bytes(),
            second_accept,
            counter,
        ),
    ];
    prove_durable_exchange(clients.url, accepts, offered_instances).await?;
    evidence(
        "mechanic=player_trade completion=committed offered_handles=removed peer_stacks=received durable_item_custody=swapped",
    );
    let (result, logout, attempts, busy_timed_out) =
        client_world::logout_retrying_busy(&mut partner).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("trade partner logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    Ok(())
}

async fn prove_durable_exchange(
    url: &str,
    accepts: [([u8; 16], u64, ItemEntry); 2],
    instances: [[u8; 16]; 2],
) -> TestResult {
    let definitions = client_world::provisioning::item_index()?;
    let mut expected = Vec::new();
    for (_, _, item) in accepts {
        expected.push(
            definitions
                .definition_at(item.item_definition_ref)
                .ok_or("trade offer definition is outside the served Item pin")?,
        );
    }
    within_database_deadline(tokio::time::Instant::now() + Duration::from_secs(20), async {
        let mut db = sqlx::PgConnection::connect(url).await?;
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_player_trade_participants a
             JOIN game_player_trade_participants b USING (occurrence_id)
             JOIN game_player_trade_receipts r USING (occurrence_id)
             JOIN game_item_container_entries e ON e.item_instance_id=r.item_instance_id
                AND e.character_id=r.destination_character_id AND e.parent_item_instance_id=r.destination_parent
             JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
             WHERE a.game_session_id=encode($1,'hex')::uuid AND a.command_id=$2::text::numeric
               AND b.game_session_id=encode($3,'hex')::uuid AND b.command_id=$4::text::numeric
               AND i.lifecycle=1
               AND ((r.source_character_id=a.character_id AND r.destination_character_id=b.character_id
                     AND r.item_instance_id=encode($11,'hex')::uuid AND i.definition_production_key=$5 AND i.definition_revision_ref=$6 AND i.quantity=$7::bigint)
                 OR (r.source_character_id=b.character_id AND r.destination_character_id=a.character_id
                     AND r.item_instance_id=encode($12,'hex')::uuid AND i.definition_production_key=$8 AND i.definition_revision_ref=$9 AND i.quantity=$10::bigint))",
        ).bind(accepts[0].0.as_slice()).bind(accepts[0].1.to_string())
            .bind(accepts[1].0.as_slice()).bind(accepts[1].1.to_string())
            .bind(expected[0].0).bind(expected[0].1).bind(i64::from(accepts[0].2.count.get()))
            .bind(expected[1].0).bind(expected[1].1).bind(i64::from(accepts[1].2.count.get()))
            .bind(instances[0].as_slice()).bind(instances[1].as_slice())
            .fetch_one(&mut db).await?;
        db.close().await?;
        if count != 2 {
            return Err("the paired trade receipts did not prove custody of both offered items".into());
        }
        Ok(())
    }).await
}
