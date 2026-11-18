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

fn deposited_ordinal(previous: &BTreeSet<u64>, ordinals: impl Iterator<Item = u64>) -> Option<u64> {
    let mut new = ordinals.filter(|ordinal| !previous.contains(ordinal));
    let ordinal = new.next()?;
    new.next().is_none().then_some(ordinal)
}

#[cfg(test)]
fn stash_units(rows: &[oteryn_protocol_oteryn::stash::StashRow], key: &str, revision: &str) -> u64 {
    rows.iter()
        .filter(|row| row.definition_key == key && row.definition_revision == revision)
        .map(|row| u64::from(row.quantity))
        .sum()
}

async fn depot_entries(
    client: &mut DevClientSession,
    town: u16,
) -> TestResult<Vec<oteryn_protocol_oteryn::depot::DepotEntry>> {
    use oteryn_protocol_oteryn::depot;
    let mut entries = Vec::new();
    let mut total = None;
    let mut ordinals = BTreeSet::new();
    for page in 0..=depot::MAX_DEPOT_PAGE {
        let reply = depot_page(client, Some(town), page, depot::DepotAction::Browse).await?;
        if reply.outcome != depot::DepotOutcome::Opened
            || reply.page != page
            || reply.town != Some(town)
            || reply.box_number != 1
            || *total.get_or_insert(reply.total) != reply.total
            || reply
                .entries
                .iter()
                .any(|entry| !ordinals.insert(entry.ordinal))
        {
            return Err(format!("depot page identity or custody changed: {reply:?}").into());
        }
        let page_len = reply.entries.len();
        entries.extend(reply.entries);
        if entries.len() == reply.total as usize {
            return Ok(entries);
        }
        if page_len == 0 || entries.len() > reply.total as usize {
            break;
        }
        client.take_events();
        progress();
    }
    Err("depot pages ended before the declared total".into())
}

pub(super) fn successful_storage_reply(status: CommandStatus, payload_success: bool) -> bool {
    status == CommandStatus::Accepted && payload_success
}

struct DepotProbe {
    definition: u32,
    count: u32,
    stack_max: Option<u32>,
    has_children: bool,
}

fn depot_probe_index(lots: &[DepotProbe]) -> Option<usize> {
    lots.iter().enumerate().position(|(index, lot)| {
        !lot.has_children
            && lot.stack_max.is_some_and(|max| {
                max == 1
                    || lots.iter().enumerate().all(|(other, candidate)| {
                        index == other
                            || lot.definition != candidate.definition
                            || candidate.count >= max
                    })
            })
    })
}

type DepotInstances = BTreeMap<u64, Vec<u8>>;

async fn depot_instances(clients: &SeamClients<'_>, town: u16) -> TestResult<DepotInstances> {
    use sqlx::Connection as _;
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut connection = sqlx::PgConnection::connect(clients.url).await?;
        let rows: Vec<(String, Vec<u8>)> = sqlx::query_as(
            "SELECT ordinal::text,uuid_send(item_instance_id) FROM game_item_character_depot_locations
             WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid
               AND town_id=$3 AND box_number=1",
        ).bind(clients.characters[1].as_slice()).bind(clients.world.as_slice())
            .bind(i32::from(town)).fetch_all(&mut connection).await?;
        rows.into_iter().map(|(ordinal,id)| Ok((ordinal.parse()?,id))).collect()
    }).await?
}

fn depot_custody_matches(
    before: &[oteryn_protocol_oteryn::depot::DepotEntry],
    after: &[oteryn_protocol_oteryn::depot::DepotEntry],
) -> bool {
    let ordered = |entries: &[oteryn_protocol_oteryn::depot::DepotEntry]| {
        entries
            .iter()
            .map(|entry| (entry.ordinal, (entry.definition, entry.count)))
            .collect::<BTreeMap<_, _>>()
    };
    before.len() == after.len() && ordered(before) == ordered(after)
}

type StashCustody = BTreeMap<(String, String), u32>;

fn stash_custody_matches(expected: &StashCustody, observed: &StashCustody) -> bool {
    expected == observed
}

fn stash_custody(rows: &[oteryn_protocol_oteryn::stash::StashRow]) -> StashCustody {
    rows.iter()
        .map(|row| {
            (
                (row.definition_key.clone(), row.definition_revision.clone()),
                row.quantity,
            )
        })
        .collect()
}

async fn stash_page(
    client: &mut DevClientSession,
    panel: &mut oteryn_dev_client::client_ui::stash::StashPanel,
    page: u32,
) -> TestResult {
    use oteryn_dev_client::{client_runtime::stash as runtime, client_ui::stash as ui};
    panel.page = page;
    let command = ui::request(panel, ui::Intent::Refresh).ok_or("stash browse input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(panel, &reply);
    if !successful_storage_reply(
        reply.status,
        reply.reply.outcome == ui::stash::StashOutcome::Opened,
    ) || panel.page != page
    {
        return Err(format!("stash page {page} refused or mismatched: {:?}", reply.reply).into());
    }
    Ok(())
}

async fn stash_entries(
    client: &mut DevClientSession,
    panel: &mut oteryn_dev_client::client_ui::stash::StashPanel,
) -> TestResult<Vec<(u32, oteryn_protocol_oteryn::stash::StashRow)>> {
    let mut entries = Vec::new();
    let mut definitions = BTreeSet::new();
    for page in 0..=oteryn_protocol_oteryn::stash::MAX_STASH_PAGE {
        stash_page(client, panel, page).await?;
        if panel.rows.iter().any(|row| {
            !definitions.insert((row.definition_key.clone(), row.definition_revision.clone()))
        }) {
            return Err("stash definition repeated across pages".into());
        }
        entries.extend(panel.rows.iter().cloned().map(|row| (page, row)));
        if !panel.has_more {
            return Ok(entries);
        }
        if panel.rows.is_empty() {
            break;
        }
        client.take_events();
        progress();
    }
    Err("stash pages ended while more rows were advertised".into())
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
    let browse = depot_call(client, None, depot::DepotAction::Browse).await?;
    if browse.outcome != depot::DepotOutcome::Opened || browse.box_number != 1 || browse.page != 0 {
        return Err(format!("depot/inbox browse: {browse:?}").into());
    }
    let town = browse.town.ok_or("depot browse named no town")?;
    // Depot custody round trip: deposit a whole carried stack, prove the locker holds exactly it,
    // then withdraw that locker entry by ordinal and prove the backpack is whole again.
    {
        let entries = &client
            .inventory()
            .ok_or("depot input inventory absent")?
            .entries;
        let whole = inventory_units(client)?;
        let originals = backpack_instances(clients, 1, client).await?;
        let profiles: crate::content::native_gameplay::ItemProfilesDocument =
            serde_json::from_slice(
                &native_input()?
                    .item_profiles
                    .as_ref()
                    .ok_or("depot item profiles absent")?
                    .bytes,
            )?;
        let definitions = provisioning::item_index()?;
        use crate::content::{ReferenceItemField::Known, native_gameplay::NativeItemStackClass};
        use sqlx::Connection as _;
        let parents: Vec<Vec<u8>> = tokio::time::timeout(Duration::from_secs(5), async {
            let mut connection = sqlx::PgConnection::connect(clients.url).await?;
            sqlx::query_scalar(
                "SELECT DISTINCT uuid_send(parent_item_instance_id) FROM game_item_nested_entries
                 WHERE parent_item_instance_id IN (SELECT encode(x,'hex')::uuid FROM unnest($1::bytea[]) x)",
            ).bind(&originals).fetch_all(&mut connection).await
        }).await??;
        let lots: Vec<_> = entries
            .iter()
            .zip(&originals)
            .map(|(entry, id)| {
                let stack_max = definitions
                    .definition_at(entry.item_definition_ref)
                    .and_then(|(key, revision)| {
                        let record = profiles.records.iter().find(|record| {
                            record.production_definition.production_key == key
                                && record.production_definition.revision_ref == revision
                                && record.admission.materializable
                        })?;
                        use crate::domain::item_definition::ItemStackClass;
                        let class = match record.admission.stack_class {
                            NativeItemStackClass::NonStackable => ItemStackClass::NonStackable,
                            NativeItemStackClass::StackCapable => ItemStackClass::Stackable {
                                proven_maximum: match &record.semantics.stack {
                                    Known(stack) => match stack.stack_max {
                                        Known(max) => Some(u32::from(max)),
                                        _ => None,
                                    },
                                    _ => None,
                                },
                            },
                        };
                        oteryn_gameplay_items::stacking::stack_maximum(class)
                            .ok()
                            .map(|(maximum, _)| maximum)
                    });
                DepotProbe {
                    definition: entry.item_definition_ref.get(),
                    count: entry.count.get(),
                    stack_max,
                    has_children: parents.contains(id),
                }
            })
            .collect();
        let selected = depot_probe_index(&lots).ok_or("no empty, identity-preserving depot lot")?;
        let leaf = entries[selected];
        let original = &originals[selected];
        let initial = depot_entries(client, town).await?;
        let initial_instances = depot_instances(clients, town).await?;
        if initial_instances.len() != initial.len()
            || initial
                .iter()
                .any(|entry| !initial_instances.contains_key(&entry.ordinal))
        {
            return Err("depot initial wire and durable ordinals differ".into());
        }
        let previous: BTreeSet<_> = initial.iter().map(|entry| entry.ordinal).collect();
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
        let listing = depot_entries(client, town).await?;
        let ordinal = deposited_ordinal(
            &previous,
            listing
                .iter()
                .filter(|entry| {
                    entry.definition == leaf.item_definition_ref.get()
                        && entry.count == leaf.count.get()
                })
                .map(|entry| entry.ordinal),
        )
        .ok_or("unique deposited entry absent from the depot listing")?;
        let other_entries: Vec<_> = listing
            .into_iter()
            .filter(|entry| entry.ordinal != ordinal)
            .collect();
        if !depot_custody_matches(&initial, &other_entries) {
            return Err("depot deposit changed unrelated locker custody".into());
        }
        let mut deposited = depot_instances(clients, town).await?;
        if deposited.remove(&ordinal).as_ref() != Some(original) || deposited != initial_instances {
            return Err("depot deposit changed the offered or unrelated instance identity".into());
        }
        let moved = depot_call(client, Some(town), depot::DepotAction::Withdraw(ordinal)).await?;
        if moved.outcome != depot::DepotOutcome::Moved {
            return Err(format!("depot withdraw refused: {moved:?}").into());
        }
        settle(client, "depot-withdraw", |now| *now == whole).await?;
        let restored = backpack_instances(clients, 1, client).await?;
        if restored.iter().collect::<BTreeSet<_>>() != originals.iter().collect() {
            return Err("depot withdrawal returned a different backpack instance set".into());
        }
        let after = depot_entries(client, town).await?;
        if !depot_custody_matches(&initial, &after)
            || depot_instances(clients, town).await? != initial_instances
        {
            return Err("depot withdrawal changed unrelated locker custody".into());
        }
        row.pass("depot", "deposit-exact-stack-then-withdraw-by-ordinal");
    }
    inbox(row, client, clients, world).await?;
    let mut panel = ui::StashPanel {
        selected: true,
        ..Default::default()
    };
    let original_rows: Vec<_> = stash_entries(client, &mut panel)
        .await?
        .into_iter()
        .map(|(_, row)| row)
        .collect();
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
    panel.carried = client
        .inventory()
        .ok_or("stash input inventory absent")?
        .entries
        .clone();

    let command =
        ui::request(&mut panel, ui::Intent::Stow(item.handle)).ok_or("stash stow input")?;
    let reply = runtime::send(client.session_mut(), &command).await?;
    ui::apply_result(&mut panel, &reply);
    if !successful_storage_reply(
        reply.status,
        reply.reply.outcome == ui::stash::StashOutcome::Committed,
    ) || reply.reply.stowed_quantity != item.count.get()
    {
        return Err(format!("stash stow: {:?}", reply.reply).into());
    }
    let pages_after = stash_entries(client, &mut panel).await?;
    let rows_after: Vec<_> = pages_after.iter().map(|(_, row)| row.clone()).collect();
    let definitions = provisioning::item_index()?;
    let (page, selected_row) = pages_after
        .iter()
        .find(|(_, row)| {
            definitions.definition_ref("Item", &row.definition_key, &row.definition_revision)
                == Some(item.item_definition_ref)
        })
        .ok_or("stowed definition absent from stash pages")?;
    let key = selected_row.definition_key.clone();
    let revision = selected_row.definition_revision.clone();
    stash_page(client, &mut panel, *page).await?;
    let selected = panel
        .rows
        .iter()
        .position(|row| row == selected_row)
        .ok_or("stowed definition changed before withdrawal")?;
    let mut expected_stash = stash_custody(&original_rows);
    let selected_key = (key, revision);
    let count = expected_stash.entry(selected_key.clone()).or_default();
    *count = count
        .checked_add(item.count.get())
        .ok_or("stash quantity overflow")?;
    if !stash_custody_matches(&expected_stash, &stash_custody(&rows_after)) {
        return Err("stash stow changed unrelated custody or credited the wrong quantity".into());
    }
    row.pass("stow", "stash-definition-increased-by-exact-stack");
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
    if !successful_storage_reply(
        reply.status,
        reply.reply.outcome == ui::stash::StashOutcome::Committed,
    ) {
        return Err(format!("withdraw refused: {:?}", reply.reply.outcome).into());
    }
    let withdrawn_rows: Vec<_> = stash_entries(client, &mut panel)
        .await?
        .into_iter()
        .map(|(_, row)| row)
        .collect();
    let count = expected_stash
        .get_mut(&selected_key)
        .ok_or("stowed stash row absent")?;
    *count = count.checked_sub(1).ok_or("stash withdrawal underflow")?;
    if !stash_custody_matches(&expected_stash, &stash_custody(&withdrawn_rows)) {
        return Err(
            "chosen one-unit withdrawal changed unrelated custody or debited the wrong quantity"
                .into(),
        );
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
    relog(client, clients).await?;
    if inventory_units(client)? != expected {
        return Err("stash relog inventory changed".into());
    }
    let stash_after: BTreeMap<_, _> = stash_entries(client, &mut panel)
        .await?
        .into_iter()
        .map(|(_, row)| row)
        .map(|r| {
            (
                (r.definition_key.clone(), r.definition_revision.clone()),
                r.quantity,
            )
        })
        .collect();
    if stash_after != expected_stash {
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

#[cfg(test)]
mod depot_tests {
    use super::*;

    #[test]
    fn depot_withdraws_new_ordinal_even_with_an_identical_existing_lot() {
        assert_eq!(
            deposited_ordinal(&BTreeSet::from([7]), [7, 8].into_iter()),
            Some(8)
        );
        assert_eq!(
            deposited_ordinal(&BTreeSet::from([7]), [7, 8, 9].into_iter()),
            None
        );
    }
}

#[cfg(test)]
mod stash_tests {
    use super::stash_units;
    use oteryn_protocol_oteryn::stash::StashRow;

    #[test]
    fn stash_custody_excludes_other_definitions_and_revisions() {
        let row = |key: &str, revision: &str, quantity| StashRow {
            handle: [1; 16],
            definition_key: key.to_owned(),
            definition_revision: revision.to_owned(),
            quantity,
            migration_required: false,
        };
        let before = [
            row("arrow", "r1", 4),
            row("bolt", "r1", 3),
            row("arrow", "r2", 2),
        ];
        let wrong_stow = [
            row("arrow", "r1", 4),
            row("bolt", "r1", 6),
            row("arrow", "r2", 2),
        ];
        assert_eq!(stash_units(&before, "arrow", "r1"), 4);
        assert_eq!(stash_units(&wrong_stow, "arrow", "r1"), 4);
    }
}

#[cfg(test)]
mod storage_review_tests {
    use super::*;
    use oteryn_protocol_oteryn::depot::DepotEntry;

    #[test]
    fn storage_rejects_success_payload_on_rejected_command() {
        assert!(!successful_storage_reply(CommandStatus::Rejected, true));
        assert!(successful_storage_reply(CommandStatus::Accepted, true));
    }

    #[test]
    fn depot_probe_selects_a_lot_that_cannot_merge_with_a_spare() {
        let lot = |count, has_children| DepotProbe {
            definition: 10,
            count,
            stack_max: Some(100),
            has_children,
        };
        assert_eq!(depot_probe_index(&[lot(50, false), lot(30, false)]), None);
        assert_eq!(depot_probe_index(&[lot(100, true)]), None);
    }

    #[test]
    fn depot_rejects_changed_payload_on_existing_ordinal() {
        let expected = DepotEntry {
            ordinal: 7,
            definition: 10,
            count: 4,
        };
        let changed = DepotEntry {
            count: 3,
            ..expected.clone()
        };
        assert!(!depot_custody_matches(&[expected], &[changed]));
    }

    #[test]
    fn stash_rejects_compensating_changes_to_unrelated_definitions() {
        let key = |name: &str| (name.to_owned(), "first".to_owned());
        let selected = key("arrow");
        let expected = BTreeMap::from([(selected.clone(), 4), (key("b"), 3), (key("c"), 2)]);
        let observed = BTreeMap::from([(selected.clone(), 4), (key("b"), 2), (key("c"), 3)]);
        assert!(!stash_custody_matches(&expected, &observed));
    }
}

#[test]
fn full_duplicate_lots_are_valid_for_depot_probe() {
    // Both stacks are full, so neither can receive a unit from the other.
    let lot = || DepotProbe {
        definition: 10,
        count: 100,
        stack_max: Some(100),
        has_children: false,
    };
    assert_eq!(depot_probe_index(&[lot(), lot()]), Some(0));
}
