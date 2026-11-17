//! E07's player-to-player parcel: buy, address, pack, post and retrieve through native commands.
use super::*;
use oteryn_dev_client::{ItemMoveDestination, ItemMoveIntent};
use oteryn_protocol_oteryn::{depot_search, item_view, mail};

/// Like MP's with_liveness, poll the step without cancelling a partially read frame.
/// The idle recipient must keep answering the server while the sender hunts and walks.
async fn with_recipient<T>(
    recipient: &mut DevClientSession,
    step: impl std::future::Future<Output = TestResult<T>>,
) -> TestResult<T> {
    let mut step = std::pin::pin!(step);
    let mut tick = tokio::time::interval(Duration::from_millis(250));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            biased;
            result = &mut step => return result,
            _ = tick.tick() => {
                verdict::Harness::global().progress("E07-parcel");
                recipient.service_liveness(Duration::from_millis(20)).await?;
                recipient.take_events();
            }
        }
    }
}

async fn search(
    client: &mut DevClientSession,
    request: depot_search::Request,
) -> TestResult<depot_search::Reply> {
    let payload = depot_search::encode_request(request)
        .map_err(|error| format!("E07 inbox request: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            depot_search::CAPABILITY_DEPOT_SEARCH_V1,
            depot_search::COMMAND_TYPE_DEPOT_SEARCH,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("E07 inbox command refused: {:?}", reply.status).into());
    }
    depot_search::decode_reply(&reply.payload)
        .map_err(|error| format!("E07 inbox reply: {error:?}").into())
}

async fn inbox_count(client: &mut DevClientSession, id: u32, exclusive: bool) -> TestResult<u32> {
    let reply = search(client, depot_search::Request::Search { client_id: id }).await?;
    match reply.outcome {
        depot_search::Outcome::NotFound if reply.results.is_empty() => Ok(0),
        depot_search::Outcome::Found => {
            let entry = reply
                .results
                .iter()
                .find(|entry| entry.client_id == id)
                .ok_or("E07 search omitted the requested definition")?;
            // Retrieve searches both depot and inbox. Prove the parcel comes from the inbox.
            if exclusive && (entry.depot != 0 || entry.stash != 0) {
                return Err(
                    format!("E07 mail definition already in other storage: {entry:?}").into(),
                );
            }
            Ok(entry.inbox)
        }
        _ => Err(format!("E07 inbox search refused: {reply:?}").into()),
    }
}

fn carried(client: &DevClientSession, id: u32) -> TestResult<item_view::ItemEntry> {
    let reference = provisioning::item_reference(&provisioning::item_index()?, id)?;
    client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == reference && item.count.get() == 1)
        })
        .copied()
        .ok_or_else(|| format!("E07 carried mail item absent: {id}").into())
}

fn remove(units: &mut BTreeMap<u32, u64>, definition: u32) -> TestResult {
    let quantity = units
        .get_mut(&definition)
        .ok_or("E07 mail custody absent")?;
    *quantity = quantity
        .checked_sub(1)
        .ok_or("E07 mail custody underflow")?;
    if *quantity == 0 {
        units.remove(&definition);
    }
    Ok(())
}

async fn send_parcel(sender: &mut DevClientSession, world: &BundleWorld, name: &str) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let mut mailboxes = BTreeSet::new();
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |manifest, sector| {
        for cell in sector.tiles {
            if cell.items.iter().any(|item| {
                item.depth == 0
                    && [3501, 3508, 23399, 23400, 23401, 23402].iter().any(|id| {
                        manifest.palette[item.palette as usize].key
                            == format!("oteryn:item.tibia.i{id}")
                    })
            }) {
                mailboxes.insert(TilePos {
                    x: cell.x,
                    y: cell.y,
                    floor: sector.floor,
                });
            }
        }
        Ok(())
    })?;
    let arrow_price = full_gold::price(3447, 2)?;
    let needed = full_gold::price(3503, 1)?
        .checked_add(full_gold::price(3507, 1)?)
        .and_then(|price| price.checked_add(arrow_price))
        .ok_or("E07 parcel price overflow")?;
    full_gold::earn(sender, world, &metadata.spawns, "E07", needed).await?;
    full_gold::buy(sender, world, &metadata.npcs, 3503, 1).await?;
    full_gold::buy(sender, world, &metadata.npcs, 3507, 1).await?;
    full_gold::buy(sender, world, &metadata.npcs, 3447, 2).await?;
    let parcel = carried(sender, 3503)?;
    let label = carried(sender, 3507)?;
    let payload = mail::encode_intent(&mail::MailIntent::WriteText {
        item_handle: label.handle.get(),
        text: name.to_owned(),
    })
    .map_err(|error| format!("E07 address encoding: {error:?}"))?;
    let reply = sender
        .session_mut()
        .gated_command(
            u32::from(mail::CAPABILITY_MAIL_V1),
            u32::from(mail::COMMAND_TYPE_MAIL),
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || reply.payload != [mail::MailOutcome::TextWritten as u8]
    {
        return Err(format!("E07 label writing refused: {reply:?}").into());
    }
    let mut expected = inventory_units(sender)?;
    let moved = sender
        .move_item(&ItemMoveIntent {
            source: label.handle,
            destination: ItemMoveDestination::Container(parcel.handle),
        })
        .await?;
    if moved.outcome != item_view::ItemMoveOutcome::Moved {
        return Err(format!("E07 packing label refused: {moved:?}").into());
    }
    remove(&mut expected, label.item_definition_ref.get())?;
    wait_inventory(sender, &expected).await?;
    let arrow_ref = provisioning::item_reference(&provisioning::item_index()?, 3447)?;
    let arrows = sender
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == arrow_ref && item.count.get() >= 2)
        })
        .copied()
        .ok_or("E07 purchased arrow stack absent")?;
    // The ordinary split command places exactly two units into the parcel, preserving any
    // arrows the sender carried before this journey.
    let moved = oteryn_dev_client::client_runtime::inventory::send(
        sender.session_mut(),
        oteryn_dev_client::client_runtime::inventory::MoveRequest {
            source: arrows.handle,
            destination: ItemMoveDestination::Container(parcel.handle),
            count: Some(2),
        },
    )
    .await?
    .ok_or("E07 counted arrow movement unavailable")?;
    if moved.outcome != item_view::ItemMoveOutcome::Moved {
        return Err(format!("E07 packing arrows refused: {moved:?}").into());
    }
    remove(&mut expected, arrow_ref.get())?;
    remove(&mut expected, arrow_ref.get())?;
    wait_inventory(sender, &expected).await?;
    let path = route(world, tile(sender.world_spatial().actor_position)?, |at| {
        mailboxes.iter().any(|mailbox| {
            at.floor == mailbox.floor && at.x.abs_diff(mailbox.x).max(at.y.abs_diff(mailbox.y)) <= 1
        })
    })
    .ok_or("E07 no reachable authored mailbox")?;
    walk(sender, world, &path).await?;
    let at = tile(sender.world_spatial().actor_position)?;
    let mailbox = mailboxes
        .iter()
        .find(|mailbox| {
            at.floor == mailbox.floor && at.x.abs_diff(mailbox.x).max(at.y.abs_diff(mailbox.y)) <= 1
        })
        .ok_or("E07 mailbox absent after walking")?;
    // The normal player drag onto a fixed mailbox uses command 9. Its mail::post_moved
    // owner posts the parcel; no invented handle for the immovable map mailbox is needed.
    let parcel = carried(sender, 3503)?;
    let posted = sender
        .move_item(&ItemMoveIntent {
            source: parcel.handle,
            destination: ItemMoveDestination::Ground(item_view::WorldTilePosition {
                x: i32::from(mailbox.x),
                y: i32::from(mailbox.y),
                floor: i16::from(mailbox.floor),
            }),
        })
        .await?;
    if posted.outcome != item_view::ItemMoveOutcome::Moved {
        return Err(format!("E07 posting parcel refused: {posted:?}").into());
    }
    remove(&mut expected, parcel.item_definition_ref.get())?;
    wait_inventory(sender, &expected).await
}

async fn logout_sender(sender: &mut DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    tokio::time::timeout_at(deadline, async {
        loop {
            verdict::Harness::global().progress("E07-sender-logout");
            let (outcome, text, attempts, busy_timed_out) =
                super::super::logout_retrying_busy(sender).await?;
            match outcome {
                Some(LogoutResult::Accepted) => return Ok(()),
                Some(LogoutResult::InFight { .. }) => {
                    // Hunting's combat block expires on server time. Keep playing the
                    // admitted connection until a real logout succeeds.
                    sender.service_liveness(Duration::from_millis(50)).await?;
                    sender.take_events();
                }
                _ => return Err(format!("E07 sender logout refused: {text}; attempts={attempts}; busy_timed_out={busy_timed_out}").into()),
            }
        }
    }).await.map_err(|_| "E07 sender combat logout deadline")?
}

async fn inbox(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    for id in [3504, 3507, 3447] {
        if inbox_count(client, id, true).await? != 0 {
            return Err(format!("E07 inbox already contains mail definition {id}").into());
        }
    }
    let name = own_name(client)?;
    let mut sender = with_recipient(client, connect_character(clients, 0x89, 0)).await?;
    let sent = with_recipient(client, send_parcel(&mut sender, world, &name)).await;
    let cleanup = with_recipient(client, logout_sender(&mut sender)).await;
    if let Err(error) = cleanup {
        return Err(format!("E07 sender cleanup: {error}; send={sent:?}").into());
    }
    sent?;
    let mut expected = inventory_units(client)?;
    // Posting stamps parcel 3503 into 3504. Its nested label must be searchable too.
    for (id, quantity) in [(3504, 1), (3507, 1), (3447, 2)] {
        if inbox_count(client, id, true).await? != quantity {
            return Err(
                format!("E07 posted mail definition {id} has the wrong inbox quantity").into(),
            );
        }
    }
    // Retrieve the contents first, then the stamped parcel, through depot search.
    for (id, quantity) in [(3447, 2), (3507, 1), (3504, 1)] {
        let reply = search(
            client,
            depot_search::Request::Retrieve {
                client_id: id,
                all: true,
            },
        )
        .await?;
        if reply.outcome != depot_search::Outcome::Retrieved || reply.retrieved != quantity {
            return Err(format!("E07 inbox retrieval {id}: {reply:?}").into());
        }
        let reference = provisioning::item_reference(&provisioning::item_index()?, id)?;
        *expected.entry(reference.get()).or_default() += u64::from(quantity);
        wait_inventory(client, &expected).await?;
        if inbox_count(client, id, true).await? != 0 {
            return Err(format!("E07 retrieved definition {id} remains in inbox").into());
        }
    }
    row.pass(
        "inbox",
        "second-player-parcel-label-and-two-arrows-retrieved-exactly-once",
    );
    Ok(())
}

pub(super) async fn storage(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let mut journey = std::pin::pin!(storage_journey(row, client, clients, world));
    let mut heartbeat = tokio::time::interval(Duration::from_secs(10));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            result = &mut journey => return result,
            _ = heartbeat.tick() => verdict::Harness::global().progress("E07-wait"),
        }
    }
}

async fn storage_journey(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    use oteryn_dev_client::{client_runtime::stash as runtime, client_ui::stash as ui};
    use oteryn_protocol_oteryn::depot;
    locker(client, world).await?;
    for box_number in [1] {
        let payload = depot::encode_depot_intent(depot::DepotIntent {
            town: None,
            box_number,
            page: 0,
            action: depot::DepotAction::Browse,
        })
        .map_err(|e| format!("depot intent: {e:?}"))?;
        let reply = client
            .session_mut()
            .gated_command(
                depot::CAPABILITY_DEPOT_V1,
                depot::COMMAND_TYPE_DEPOT_INTENT,
                &payload,
            )
            .await?;
        let reply =
            depot::decode_depot_reply(&reply.payload).map_err(|e| format!("depot reply: {e:?}"))?;
        if reply.outcome != depot::DepotOutcome::Opened
            || reply.box_number != box_number
            || reply.town.is_none()
        {
            return Err(format!("depot/inbox browse: {reply:?}").into());
        }
        // Open views alone do not prove depot/inbox custody.
    }
    // Depot custody round trip: deposit a whole carried stack, prove the locker holds exactly it,
    // then withdraw that locker entry by ordinal and prove the backpack is whole again.
    {
        let leaf = client
            .inventory()
            .and_then(|i| i.entries.first())
            .copied()
            .ok_or("no direct-backpack leaf to deposit into the depot")?;
        let whole = inventory_units(client)?;
        let town = {
            let browse = depot_call(client, None, depot::DepotAction::Browse).await?;
            browse.town.ok_or("depot browse named no town")?
        };
        let held_before = depot_call(client, Some(town), depot::DepotAction::Browse)
            .await?
            .entries
            .iter()
            .filter(|e| e.definition == leaf.item_definition_ref.get())
            .map(|e| u64::from(e.count))
            .sum::<u64>();
        let moved =
            depot_call(client, Some(town), depot::DepotAction::Deposit(leaf.handle)).await?;
        if moved.outcome != depot::DepotOutcome::Moved {
            return Err(format!("depot deposit refused: {moved:?}").into());
        }
        let mut expected = whole.clone();
        let units = expected
            .get_mut(&leaf.item_definition_ref.get())
            .ok_or("deposited definition absent")?;
        *units -= u64::from(leaf.count.get());
        if *units == 0 {
            expected.remove(&leaf.item_definition_ref.get());
        }
        wait_inventory(client, &expected).await?;
        let listing = depot_call(client, Some(town), depot::DepotAction::Browse).await?;
        let held_after = listing
            .entries
            .iter()
            .filter(|e| e.definition == leaf.item_definition_ref.get())
            .map(|e| u64::from(e.count))
            .sum::<u64>();
        if held_after != held_before + u64::from(leaf.count.get()) {
            return Err(format!(
                "depot holds {held_after} after depositing {} on top of {held_before}",
                leaf.count.get()
            )
            .into());
        }
        let ordinal = listing
            .entries
            .iter()
            .find(|e| e.definition == leaf.item_definition_ref.get())
            .map(|e| e.ordinal)
            .ok_or("deposited entry absent from the depot listing")?;
        let moved = depot_call(client, Some(town), depot::DepotAction::Withdraw(ordinal)).await?;
        if moved.outcome != depot::DepotOutcome::Moved {
            return Err(format!("depot withdraw refused: {moved:?}").into());
        }
        settle(client, "depot-withdraw", |now| *now == whole).await?;
        let after = depot_call(client, Some(town), depot::DepotAction::Browse).await?;
        let held_end = after
            .entries
            .iter()
            .filter(|e| e.definition == leaf.item_definition_ref.get())
            .map(|e| u64::from(e.count))
            .sum::<u64>();
        if held_end != held_before {
            return Err(format!(
                "depot still holds {held_end} after the withdraw; started with {held_before}"
            )
            .into());
        }
        row.pass("depot", "deposit-exact-stack-then-withdraw-by-ordinal");
    }
    inbox(row, client, clients, world).await?;
    let mut panel = ui::StashPanel {
        selected: true,
        ..Default::default()
    };
    let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("stash browse input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if reply.reply.outcome != ui::stash::StashOutcome::Opened {
        return Err(format!("stash open: {:?}", reply.reply.outcome).into());
    }
    let arrow = provisioning::item_reference(&provisioning::item_index()?, 3447)?;
    let item = client
        .inventory()
        .and_then(|i| {
            i.entries
                .iter()
                .find(|i| i.item_definition_ref == arrow && i.count.get() >= 2)
        })
        .copied()
        .ok_or("no carried stack of at least two units for chosen-count withdrawal")?;
    let carried = inventory_units(client)?;
    let before: u64 = panel.rows.iter().map(|r| u64::from(r.quantity)).sum();
    let command =
        ui::request(&mut panel, ui::Intent::Stow(item.handle)).ok_or("stash stow input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if reply.reply.outcome != ui::stash::StashOutcome::Committed
        || reply.reply.stowed_quantity != item.count.get()
    {
        return Err(format!("stash stow: {:?}", reply.reply).into());
    }
    let command = ui::request(&mut panel, ui::Intent::Refresh).ok_or("stash refresh after stow")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    let after: u64 = panel.rows.iter().map(|r| u64::from(r.quantity)).sum();
    if after != before + u64::from(item.count.get()) {
        return Err("stash count delta differs from stowed quantity".into());
    }
    row.pass("stow", "stash-total-increased-by-exact-stack");
    let definitions = provisioning::item_index()?;
    let selected = panel
        .rows
        .iter()
        .position(|r| {
            definitions.definition_ref("Item", &r.definition_key, &r.definition_revision)
                == Some(item.item_definition_ref)
        })
        .ok_or("stowed definition absent from stash page")?;
    let command = ui::request(
        &mut panel,
        ui::Intent::Withdraw {
            row: selected,
            amount: 1,
        },
    )
    .ok_or("chosen-count withdrawal input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if reply.reply.outcome != ui::stash::StashOutcome::Committed {
        return Err(format!("withdraw refused: {:?}", reply.reply.outcome).into());
    }
    let command =
        ui::request(&mut panel, ui::Intent::Refresh).ok_or("stash refresh after withdraw")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if panel
        .rows
        .iter()
        .map(|r| u64::from(r.quantity))
        .sum::<u64>()
        != after - 1
    {
        return Err("chosen one-unit withdrawal changed the wrong stash quantity".into());
    }
    let mut expected = carried;
    let units = expected
        .get_mut(&item.item_definition_ref.get())
        .ok_or("stowed inventory definition absent")?;
    *units = units
        .checked_sub(u64::from(item.count.get()) - 1)
        .ok_or("stow inventory underflow")?;
    wait_inventory(client, &expected).await?;
    row.pass(
        "chosen-withdraw",
        "one-unit-inventory-increase-and-one-unit-stash-decrease",
    );
    let stash_before: BTreeMap<_, _> = panel
        .rows
        .iter()
        .map(|r| (r.definition_key.clone(), r.quantity))
        .collect();
    relog(client, clients).await?;
    if inventory_units(client)? != expected {
        return Err("stash relog inventory changed".into());
    }
    let command =
        ui::request(&mut panel, ui::Intent::Refresh).ok_or("fresh-session stash browse")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    let stash_after: BTreeMap<_, _> = panel
        .rows
        .iter()
        .map(|r| (r.definition_key.clone(), r.quantity))
        .collect();
    if reply.reply.outcome != ui::stash::StashOutcome::Opened || stash_after != stash_before {
        return Err("fresh-session stash differs from committed withdrawal".into());
    }
    for id in [3504, 3507, 3447] {
        if inbox_count(client, id, false).await? != 0 {
            return Err(format!("E07 mail definition {id} reappeared in inbox after relog").into());
        }
    }
    row.pass(
        "relog-custody",
        "fresh-admission-inventory-and-stash-quantities-retained",
    );
    Ok(())
}
