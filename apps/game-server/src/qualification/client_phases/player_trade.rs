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
    let Some(item) = client
        .inventory()
        .and_then(|inventory| inventory.entries.first())
        .copied()
    else {
        return Err("core trade has no carried offer item".into());
    };
    let partner =
        with_trade_peer(client, client_world::connect_character(clients, 0x74, 0)).await??;
    with_trade_partner(
        client,
        partner,
        async |client, partner| exchange_pair(client, partner, clients, item).await,
        client_world::logout_trade_character,
        service_trade_peer,
        client_world::wait_character_released(clients, 0),
    )
    .await
}

async fn exchange_pair(
    client: &mut oteryn_dev_client::DevClientSession,
    partner: &mut oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
    item: ItemEntry,
) -> TestResult {
    use oteryn_dev_client::{
        client_runtime::player_trade as runtime, client_ui::player_trade as ui,
    };
    client.take_events();
    let counter = partner
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref != item.item_definition_ref)
                .or_else(|| inventory.entries.first())
        })
        .copied()
        .ok_or("core trade partner has no carried counter-offer item")?;
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
    let keys = client_world::native_input()?
        .item_keys
        .as_ref()
        .ok_or("qualification Item keys missing")?;
    let index = crate::content::item_ref::ItemDefinitionIndex::decode(&keys.bytes)?;
    let durable_instances = with_trade_peer(
        client,
        with_trade_peer(
            partner,
            offered_trade_instances(clients, &index, [item, counter]),
        ),
    )
    .await???;
    let [session, partner_session] = [
        *client.join_snapshot().game_session_id.as_bytes(),
        *partner.join_snapshot().game_session_id.as_bytes(),
    ];
    let offered_instances: TestResult<[[u8; 16]; 2]> = with_trade_peer(
        client,
        with_trade_peer(partner, async {
            Ok([
                client_world::observed_item_instance(session, item.handle)
                    .await?
                    .ok_or("offered handle is not bound to a durable Instance")?,
                client_world::observed_item_instance(partner_session, counter.handle)
                    .await?
                    .ok_or("counter-offer handle is not bound to a durable Instance")?,
            ])
        }),
    )
    .await??;
    let offered_instances = offered_instances?;
    if durable_instances != offered_instances {
        return Err("trade wire handles and durable offer identities disagree".into());
    }
    if item.item_definition_ref == counter.item_definition_ref {
        return Err("trade custody fixture requires distinct offered definitions".into());
    }
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
    let offered_revision = trade_revision(partner);
    let reply = with_trade_peer(partner, runtime::send(client.session_mut(), &command)).await??;
    if reply.result != ui::player_trade::ResultCode::Offered {
        return Err(format!("trade offer: {:?} reply={reply:?}", reply.result).into());
    }
    ui::apply_result(&mut panel, &reply);
    // Each peer consumes the other writer's unsolicited state before sending its next command.
    with_trade_peer(
        client,
        trade_state(partner, offered_revision, |state| state.phase == 1),
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
            partner,
            trade_state(client, ready_revision, |state| state.phase == 2),
        )
        .await??,
    );
    let command = ui::request(&mut panel, ui::Intent::Accept).ok_or("trade accept")?;
    let first_accept = client.next_command_id();
    let accepted_revision = trade_revision(partner);
    let reply = with_trade_peer(partner, runtime::send(client.session_mut(), &command)).await??;
    if reply.result != ui::player_trade::ResultCode::Accepted {
        return Err(format!("trade accept: {:?} reply={reply:?}", reply.result).into());
    }
    ui::apply_result(&mut panel, &reply);
    other_panel.state = Some(
        with_trade_peer(
            client,
            trade_state(partner, accepted_revision, |state| state.accepted[1]),
        )
        .await??,
    );
    let command =
        ui::request(&mut other_panel, ui::Intent::Accept).ok_or("trade counter accept")?;
    let completion_session = *partner.join_snapshot().game_session_id.as_bytes();
    let completion_command = partner.session_mut().next_command_id();
    let second_accept = completion_command;
    let completed_revision = trade_revision(client);
    let reply = with_trade_peer(client, runtime::send(partner.session_mut(), &command)).await??;
    if reply.result != ui::player_trade::ResultCode::Completed {
        return Err(format!("trade complete: {:?} reply={reply:?}", reply.result).into());
    }
    ui::apply_result(&mut other_panel, &reply);
    // The initiator receives the completion state and both receive inventory deltas.
    with_trade_peer(
        partner,
        trade_state(client, completed_revision, |state| {
            state.phase == 4 && state.reason == 1
        }),
    )
    .await??;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !client.inventory().is_some_and(|after| {
        received_offer(&before, after, item, counter)
            && offers_exchanged(&before, after, item, counter)
    }) || !partner.inventory().is_some_and(|after| {
        received_offer(&counter_before, after, counter, item)
            && offers_exchanged(&counter_before, after, counter, item)
    }) {
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
    with_trade_peer(
        client,
        with_trade_peer(
            partner,
            prove_durable_exchange(clients.url, accepts, offered_instances),
        ),
    )
    .await???;
    let received_instances = with_trade_peer(
        client,
        with_trade_peer(
            partner,
            received_trade_instances(clients, &completion_session, completion_command),
        ),
    )
    .await???;
    if !durable_offers_exchanged(offered_instances, received_instances) {
        return Err("completed trade did not move the exact offered item instances".into());
    }
    evidence(
        "mechanic=player_trade completion=committed durable_instances=exchanged offered_handles=departed received_items=matched unrelated_custody=retained",
    );
    Ok(())
}

// The fixture requires two distinct offers. Resolve each wire offer to a unique
// direct entry in its owner's durable main backpack before the swap.
async fn offered_trade_instances(
    clients: &SeamClients<'_>,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    entries: [ItemEntry; 2],
) -> TestResult<[[u8; 16]; 2]> {
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut connection = sqlx::PgConnection::connect(clients.url).await?;
        let mut instances = [[0; 16]; 2];
        for (side, (entry, character)) in entries.into_iter().zip([1, 0]).enumerate() {
            let rows = sqlx::query_as::<_, (Vec<u8>, String, String, String, i64)>(
                "SELECT uuid_send(i.item_instance_id), i.definition_family, \
                        i.definition_production_key, i.definition_revision_ref, i.quantity::bigint \
                 FROM game_item_container_slots s \
                 JOIN game_item_container_entries e \
                   ON e.parent_item_instance_id=s.item_instance_id AND e.character_id=s.character_id \
                 JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id AND i.world_id=e.world_id \
                 WHERE s.character_id=encode($1,'hex')::uuid \
                    AND s.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 LIMIT 21",
            )
            .bind(clients.characters[character].as_slice())
            .bind(clients.world.as_slice())
            .fetch_all(&mut connection)
            .await?;
            if rows.len() > 20 {
                return Err("trade fixture backpack exceeds the supported capacity".into());
            }
            let mut matching = rows.into_iter().filter(|(_, family, key, revision, quantity)| {
                *quantity == i64::from(entry.count.get())
                    && index.definition_ref(family, key, revision) == Some(entry.item_definition_ref)
            });
            let (id, ..) = matching.next().ok_or("wire trade item absent from durable backpack")?;
            if matching.next().is_some() {
                return Err("wire trade item is ambiguous in the durable backpack".into());
            }
            instances[side] = id.try_into().map_err(|_| "malformed durable trade item ID")?;
        }
        Ok(instances)
    })
    .await
    .map_err(|_| "durable trade custody observation deadline")?
}

async fn received_trade_instances(
    clients: &SeamClients<'_>,
    session: &[u8; 16],
    command: u64,
) -> TestResult<[[u8; 16]; 2]> {
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut connection = sqlx::PgConnection::connect(clients.url).await?;
        let rows = sqlx::query_as::<_, (Vec<u8>, Vec<u8>, Vec<u8>)>(
            "SELECT uuid_send(r.item_instance_id), uuid_send(r.source_character_id), \
                    uuid_send(r.destination_character_id) \
             FROM game_player_trade_participants p \
             JOIN game_player_trade_receipts r USING(occurrence_id) \
             JOIN game_item_container_entries e ON e.item_instance_id=r.item_instance_id \
                 AND e.character_id=r.destination_character_id \
                 AND e.parent_item_instance_id=r.destination_parent \
                 AND e.placement_ordinal=r.destination_ordinal \
             JOIN game_item_container_slots s ON s.item_instance_id=e.parent_item_instance_id \
                 AND s.character_id=e.character_id AND s.world_id=e.world_id \
             WHERE p.game_session_id=encode($1,'hex')::uuid AND p.command_id=$2::text::numeric \
                 AND p.character_id=encode($3,'hex')::uuid AND e.world_id=encode($4,'hex')::uuid",
        )
        .bind(session.as_slice())
        .bind(command.to_string())
        .bind(clients.characters[0].as_slice())
        .bind(clients.world.as_slice())
        .fetch_all(&mut connection)
        .await?;
        let mut received = [None; 2];
        for (id, source, destination) in rows {
            let side = match (source.as_slice(), destination.as_slice()) {
                (source, destination)
                    if source == clients.characters[0] && destination == clients.characters[1] =>
                {
                    0
                }
                (source, destination)
                    if source == clients.characters[1] && destination == clients.characters[0] =>
                {
                    1
                }
                _ => return Err("trade receipt names an unexpected custody transfer".into()),
            };
            let id = id
                .try_into()
                .map_err(|_| "malformed received trade item ID")?;
            if received[side].replace(id).is_some() {
                return Err("trade receipt repeats a custody transfer".into());
            }
        }
        match received {
            [Some(first), Some(second)] => Ok([first, second]),
            _ => Err("trade completion has no reciprocal durable custody receipt".into()),
        }
    })
    .await
    .map_err(|_| "durable trade receipt observation deadline")?
}

fn durable_offers_exchanged(offered: [[u8; 16]; 2], received: [[u8; 16]; 2]) -> bool {
    received == [offered[1], offered[0]]
}

async fn with_trade_peer<T>(
    peer: &mut oteryn_dev_client::DevClientSession,
    action: impl std::future::Future<Output = T>,
) -> TestResult<T> {
    client_world::with_peer_service(action, async || service_trade_peer(peer).await).await
}

async fn service_trade_peer(peer: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    peer.service_liveness(Duration::from_millis(20)).await?;
    peer.take_events();
    Ok(())
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
    fn identical_replacement_items_do_not_prove_durable_trade_custody() {
        let offered = [[1; 16], [2; 16]];
        assert!(!durable_offers_exchanged(offered, [[4; 16], [3; 16]]));
        assert!(!durable_offers_exchanged(offered, offered));
        assert!(durable_offers_exchanged(offered, [[2; 16], [1; 16]]));
    }

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

async fn with_trade_partner<P, C>(
    primary: &mut P,
    mut partner: C,
    body: impl AsyncFnOnce(&mut P, &mut C) -> TestResult,
    logout: impl AsyncFnOnce(&mut C) -> TestResult,
    mut service: impl AsyncFnMut(&mut P) -> TestResult,
    release: impl Future<Output = TestResult>,
) -> TestResult {
    let trade = body(primary, &mut partner).await;
    let mut service_error = None;
    let mut tick = async || {
        if service_error.is_none() {
            service_error = service(primary).await.err();
        }
        if service_error.is_some() {
            // Release still has to finish if the primary socket has already failed.
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        Ok(())
    };
    let logged_out = client_world::with_peer_service(logout(&mut partner), &mut tick).await?;
    drop(partner);
    let released = client_world::with_peer_service(release, tick).await?;
    trade
        .and(logged_out)
        .and(released)
        .and(service_error.map_or(Ok(()), Err))
}

#[tokio::test(start_paused = true)]
async fn failed_trade_services_primary_until_partner_release() -> TestResult {
    let serviced = std::cell::Cell::new(false);
    let result = with_trade_partner(
        &mut (),
        (),
        async |_, _| Err("trade refused".into()),
        async |_| {
            tokio::time::sleep(Duration::from_secs(30)).await;
            assert!(
                serviced.get(),
                "primary probe went unanswered during partner logout"
            );
            serviced.set(false);
            Ok(())
        },
        async |_| {
            tokio::time::sleep(Duration::from_millis(20)).await;
            serviced.set(true);
            Ok(())
        },
        async {
            tokio::time::sleep(Duration::from_secs(30)).await;
            assert!(
                serviced.get(),
                "primary probe went unanswered during partner release"
            );
            Ok(())
        },
    )
    .await;
    assert_eq!(
        result.err().as_deref().map(ToString::to_string).as_deref(),
        Some("trade refused")
    );
    Ok(())
}

#[tokio::test]
async fn failed_trade_observation_still_releases_partner() {
    struct Partner<'a>(&'a std::cell::Cell<bool>);
    impl Drop for Partner<'_> {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }
    let dropped = std::cell::Cell::new(false);
    let logged_out = std::cell::Cell::new(false);
    let released = std::cell::Cell::new(false);
    let result = with_trade_partner(
        &mut (),
        Partner(&dropped),
        async |_, _| Err("item observation error".into()),
        async |_| {
            logged_out.set(true);
            Err("invalid frame".into())
        },
        async |_| Ok(()),
        async {
            assert!(dropped.get());
            released.set(true);
            Ok(())
        },
    )
    .await;
    assert!(logged_out.get() && released.get());
    assert_eq!(
        result.err().map(|error| error.to_string()).as_deref(),
        Some("item observation error")
    );
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
