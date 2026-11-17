use super::*;
use oteryn_dev_client::{CharacterInventory, ItemEntry};
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
    client.take_events();
    let mut partner =
        with_trade_peer(client, client_world::connect_character(clients, 0x74, 0)).await??;
    let Some(counter) = partner
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref != item.item_definition_ref)
                .or_else(|| inventory.entries.first())
        })
        .copied()
    else {
        let (result, logout, attempts, busy_timed_out) =
            with_trade_peer(client, client_world::logout_retrying_busy(&mut partner)).await??;
        if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("trade partner logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
        }
        evidence(
            "mechanic=player_trade skipped=second_platform_created_character_has_no_carried_offer_item source=crates/gameplay/dispatch/src/gameplay_transport/item_ref_admission.rs:373",
        );
        return Ok(());
    };
    if item.item_definition_ref == counter.item_definition_ref {
        return Err("trade custody fixture requires distinct offered definitions".into());
    }
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
    let offered_revision = trade_revision(&mut partner);
    let reply =
        with_trade_peer(&mut partner, runtime::send(client.session_mut(), &command)).await??;
    if reply.result != ui::player_trade::ResultCode::Offered {
        return Err(format!("trade offer: {:?} reply={reply:?}", reply.result).into());
    }
    ui::apply_result(&mut panel, &reply);
    // Each peer consumes the other writer's unsolicited state before sending its next command.
    with_trade_peer(
        client,
        trade_state(&mut partner, offered_revision, |state| state.phase == 1),
    )
    .await??;
    let command = ui::request(
        &mut other_panel,
        ui::Intent::Offer {
            partner: own,
            item: counter.handle.get(),
        },
    )
    .ok_or("trade counter offer")?;
    let ready_revision = trade_revision(client);
    let reply = with_trade_peer(client, runtime::send(partner.session_mut(), &command)).await??;
    if reply.result != ui::player_trade::ResultCode::Offered {
        return Err(format!("trade counter: {:?} reply={reply:?}", reply.result).into());
    }
    ui::apply_result(&mut other_panel, &reply);
    panel.state = Some(
        with_trade_peer(
            &mut partner,
            trade_state(client, ready_revision, |state| state.phase == 2),
        )
        .await??,
    );
    let command = ui::request(&mut panel, ui::Intent::Accept).ok_or("trade accept")?;
    let accepted_revision = trade_revision(&mut partner);
    let reply =
        with_trade_peer(&mut partner, runtime::send(client.session_mut(), &command)).await??;
    if reply.result != ui::player_trade::ResultCode::Accepted {
        return Err(format!("trade accept: {:?} reply={reply:?}", reply.result).into());
    }
    ui::apply_result(&mut panel, &reply);
    other_panel.state = Some(
        with_trade_peer(
            client,
            trade_state(&mut partner, accepted_revision, |state| state.accepted[1]),
        )
        .await??,
    );
    let command =
        ui::request(&mut other_panel, ui::Intent::Accept).ok_or("trade counter accept")?;
    let completed_revision = trade_revision(client);
    let reply = with_trade_peer(client, runtime::send(partner.session_mut(), &command)).await??;
    if reply.result != ui::player_trade::ResultCode::Completed {
        return Err(format!("trade complete: {:?} reply={reply:?}", reply.result).into());
    }
    ui::apply_result(&mut other_panel, &reply);
    // The initiator receives the completion state and both receive inventory deltas.
    with_trade_peer(
        &mut partner,
        trade_state(client, completed_revision, |state| {
            state.phase == 4 && state.reason == 1
        }),
    )
    .await??;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !client
        .inventory()
        .is_some_and(|after| offers_exchanged(&before, after, item, counter))
        || !partner
            .inventory()
            .is_some_and(|after| offers_exchanged(&counter_before, after, counter, item))
    {
        if tokio::time::Instant::now() >= deadline {
            return Err(
                "completed trade did not exchange both offers while retaining unrelated inventory"
                    .into(),
            );
        }
        tokio::try_join!(
            client.service_liveness(Duration::from_millis(20)),
            partner.service_liveness(Duration::from_millis(20)),
        )?;
        client.take_events();
        partner.take_events();
    }
    evidence(
        "mechanic=player_trade completion=committed offered_handles=departed received_items=matched unrelated_custody=retained",
    );
    let (result, logout, attempts, busy_timed_out) =
        with_trade_peer(client, client_world::logout_retrying_busy(&mut partner)).await??;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("trade partner logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    Ok(())
}

async fn with_trade_peer<T>(
    peer: &mut oteryn_dev_client::DevClientSession,
    action: impl std::future::Future<Output = T>,
) -> TestResult<T> {
    client_world::with_peer_service(action, async || {
        peer.service_liveness(Duration::from_millis(20)).await?;
        peer.take_events();
        Ok(())
    })
    .await
}

fn trade_revision(client: &mut oteryn_dev_client::DevClientSession) -> u64 {
    client
        .session_mut()
        .gated_snapshot(oteryn_protocol_oteryn::player_trade::STATE_DOMAIN_PLAYER_TRADE)
        .map_or(0, |snapshot| snapshot.revision)
}

async fn trade_state(
    client: &mut oteryn_dev_client::DevClientSession,
    prior_revision: u64,
    matches: impl Fn(&oteryn_protocol_oteryn::player_trade::State) -> bool,
) -> TestResult<oteryn_protocol_oteryn::player_trade::State> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if trade_revision(client) > prior_revision
                && let Some(update) = client.session_mut().player_trade_state()
                && matches(&update)
            {
                return Ok(update);
            }
            client.service_liveness(Duration::from_millis(20)).await?;
            client.take_events();
        }
    })
    .await
    .map_err(|_| "trade state update deadline")?
}

fn offers_exchanged(
    before: &CharacterInventory,
    after: &CharacterInventory,
    offered: ItemEntry,
    incoming: ItemEntry,
) -> bool {
    offered.item_definition_ref != incoming.item_definition_ref
        && before.main_backpack == after.main_backpack
        && before.equipment == after.equipment
        && before.entries.len() == after.entries.len()
        && !after
            .entries
            .iter()
            .any(|entry| entry.handle == offered.handle)
        && before
            .entries
            .iter()
            .filter(|entry| entry.handle != offered.handle)
            .all(|entry| after.entries.contains(entry))
        && after.entries.iter().any(|entry| {
            *entry
                == ItemEntry {
                    handle: entry.handle,
                    ..incoming
                }
                && before.entries.iter().all(|old| old.handle != entry.handle)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrelated_inventory_change_cannot_prove_trade_custody() -> TestResult {
        fn item(handle: u64) -> TestResult<ItemEntry> {
            Ok(ItemEntry {
                handle: std::num::NonZeroU64::new(handle).ok_or("zero handle")?,
                item_definition_ref: std::num::NonZeroU32::MIN,
                count: std::num::NonZeroU32::MIN,
                appearance_id: 0,
                sub_type: 0,
            })
        }
        let offered = item(1)?;
        let retained = ItemEntry {
            item_definition_ref: std::num::NonZeroU32::new(2).ok_or("zero definition")?,
            ..item(2)?
        };
        let incoming = ItemEntry {
            handle: item(5)?.handle,
            ..retained
        };
        let before = CharacterInventory {
            entries: vec![offered, retained],
            ..Default::default()
        };
        let unrelated = CharacterInventory {
            entries: vec![
                offered,
                ItemEntry {
                    handle: item(3)?.handle,
                    ..retained
                },
            ],
            ..Default::default()
        };
        assert!(!offers_exchanged(&before, &unrelated, offered, incoming));

        let reissued = CharacterInventory {
            entries: vec![
                item(3)?,
                ItemEntry {
                    handle: item(4)?.handle,
                    ..retained
                },
            ],
            ..Default::default()
        };
        assert!(!offers_exchanged(&before, &reissued, offered, incoming));

        let same_definition_reissued = CharacterInventory {
            entries: vec![retained, item(3)?],
            ..Default::default()
        };
        assert!(!offers_exchanged(
            &before,
            &same_definition_reissued,
            offered,
            item(5)?
        ));

        let swapped = CharacterInventory {
            entries: vec![
                retained,
                ItemEntry {
                    handle: item(3)?.handle,
                    ..incoming
                },
            ],
            ..Default::default()
        };
        assert!(offers_exchanged(&before, &swapped, offered, incoming));
        Ok(())
    }
}
