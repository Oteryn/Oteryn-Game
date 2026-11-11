//! E09 player scenario, derived from the matrix row, re-audit, and the production MAIL owner:
//! 1. Walk to a reachable postal NPC, say `hi` and `trade`, buy two letters, one parcel and one
//!    label, and assert each BUY reply plus its CHARACTER_INVENTORY delta.
//! 2. Send MAIL `WriteText` for the first letter with the admitted recipient character's name and
//!    town, assert `TextWritten`, and assert the refreshed inventory still owns that letter.
//! 3. Walk beside an authored mailbox and send MAIL `Post`; assert `Posted` and the inventory
//!    delta removing the letter, then search its client id and assert one Inbox delivery.
//! 4. Address the second letter to an unknown recipient, post it to the same mailbox, assert the
//!    `UnknownRecipient` refusal, and assert the letter remains in sender custody unchanged.
//! 5. Write the recipient on the parcel label, move that label into the parcel with ITEM_MOVE,
//!    assert the accepted move and inventory delta, then post the parcel and assert `Posted`, its
//!    removal, and an Inbox parcel delivery.
//! 6. Logout normally, obtain a fresh admission for the recipient, and re-run depot searches for
//!    the stamped letter and parcel; both durable Inbox counts must remain present.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind};
use oteryn_protocol_oteryn::{depot_search, item_view, mail, npc_service};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

const ACTIONS: &[&str] = &[
    "player-bought-mail-items",
    "letter-address-written",
    "letter-posted-and-delivered",
    "unknown-recipient-refused",
    "parcel-label-and-delivery",
    "fresh-admission-durable-inbox",
];

struct Row {
    passed: BTreeSet<&'static str>,
    started: std::time::Instant,
    invalid: bool,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: BTreeSet::new(),
            started: std::time::Instant::now(),
            invalid: false,
        }
    }
    fn pass(&mut self, action: &'static str, observed: impl std::fmt::Debug) {
        if !ACTIONS.contains(&action) || !self.passed.insert(action) {
            self.invalid = true;
            return;
        }
        evidence(&format!("E09_CHECK {action} PASS observed={observed:?}"));
    }
    fn finish(self, result: TestResult) {
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("E09_CHECK {action} FAIL assertion-not-completed"));
            }
        }
        if let Err(error) = &result {
            evidence(&format!("E09_GAP step-failed observed={error}"));
        }
        evidence(&format!(
            "e2e_all scenario=E09.full status={} seconds={:.3}",
            if result.is_ok() && !self.invalid && ACTIONS.iter().all(|a| self.passed.contains(a)) {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = row_complete_with_timeout(Duration::from_secs(900), journey(&mut row, clients))
        .await
        .unwrap_or_else(|_| {
            Err("step=journey observed=timeout-900s need=responsive-mail-route".into())
        });
    row.finish(result);
}

fn units(client: &DevClientSession, definition: NonZeroU32) -> u64 {
    client.inventory().map_or(0, |inventory| {
        inventory
            .entries
            .iter()
            .chain(inventory.equipment.iter().map(|e| &e.item))
            .filter(|item| item.item_definition_ref == definition)
            .map(|item| u64::from(item.count.get()))
            .sum()
    })
}

fn item(client: &DevClientSession, definition: NonZeroU32) -> TestResult<item_view::ItemEntry> {
    client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == definition)
        })
        .copied()
        .ok_or_else(|| {
            format!(
                "step=inventory observed=definition-{definition}-absent need=purchased-item-delta"
            )
            .into()
        })
}

async fn wait_units(
    client: &mut DevClientSession,
    definition: NonZeroU32,
    expected: u64,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    while units(client, definition) != expected {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=inventory-delta observed=definition-{definition}-units-{} expected={expected} need=character-inventory-refresh", units(client, definition)).into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    Ok(())
}

async fn mail_command(
    client: &mut DevClientSession,
    intent: mail::MailIntent,
) -> TestResult<(CommandStatus, mail::MailOutcome)> {
    let payload = mail::encode_intent(&intent).map_err(|e| format!("mail encode: {e:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            u32::from(mail::CAPABILITY_MAIL_V1),
            u32::from(mail::COMMAND_TYPE_MAIL),
            &payload,
        )
        .await?;
    let outcome =
        match reply.payload.as_slice() {
            [1] => mail::MailOutcome::Posted,
            [2] => mail::MailOutcome::TextWritten,
            [3] => mail::MailOutcome::NoAddress,
            [4] => mail::MailOutcome::UnknownRecipient,
            [5] => mail::MailOutcome::RecipientCannotReceiveParcels,
            [6] => mail::MailOutcome::NotMailable,
            [7] => mail::MailOutcome::RecipientInboxFull,
            [8] => mail::MailOutcome::Exhausted,
            [9] => mail::MailOutcome::NotWritable,
            [10] => mail::MailOutcome::TooLong,
            [11] => mail::MailOutcome::Rejected,
            other => return Err(format!(
                "step=mail-reply observed=status-{:?}-payload-{other:?} need=single-mail-outcome",
                reply.status
            )
            .into()),
        };
    Ok((reply.status, outcome))
}

async fn write(client: &mut DevClientSession, handle: u64, text: String) -> TestResult {
    let observed = mail_command(
        client,
        mail::MailIntent::WriteText {
            item_handle: handle,
            text,
        },
    )
    .await?;
    if observed != (CommandStatus::Accepted, mail::MailOutcome::TextWritten) {
        return Err(
            format!("step=write-address observed={observed:?} need=accepted-TextWritten").into(),
        );
    }
    Ok(())
}

async fn post(
    client: &mut DevClientSession,
    item_handle: u64,
    mailbox_handle: u64,
) -> TestResult<mail::MailOutcome> {
    let (status, outcome) = mail_command(
        client,
        mail::MailIntent::Post {
            item_handle,
            mailbox_handle,
        },
    )
    .await?;
    if status != CommandStatus::Accepted {
        return Err(format!(
            "step=post observed=status-{status:?}-outcome-{outcome:?} need=accepted-mail-outcome"
        )
        .into());
    }
    Ok(outcome)
}

async fn depot_search(
    client: &mut DevClientSession,
    client_id: u32,
) -> TestResult<depot_search::Reply> {
    let payload = depot_search::encode_request(depot_search::Request::Search { client_id })
        .map_err(|error| format!("depot search input: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            depot_search::CAPABILITY_DEPOT_SEARCH_V1,
            depot_search::COMMAND_TYPE_DEPOT_SEARCH,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(
            format!("step=inbox-search observed={reply:?} need=accepted-depot-search").into(),
        );
    }
    depot_search::decode_reply(&reply.payload)
        .map_err(|e| format!("step=inbox-search observed=decode-{e:?} need=valid-reply").into())
}

fn inbox(reply: &depot_search::Reply) -> u32 {
    reply.results.iter().map(|result| result.inbox).sum()
}

async fn acquire(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    definition: NonZeroU32,
    count: u32,
) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop;
    for npc in &npcs.npcs {
        let Some(path) = route(world, tile(client.world_spatial().actor_position)?, |at| {
            npc.placements
                .iter()
                .any(|p| p.floor == at.floor && p.x.abs_diff(at.x).max(p.y.abs_diff(at.y)) <= 3)
        }) else {
            continue;
        };
        walk(client, world, &path).await?;
        let Some(actor) = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entity| {
                    entity.kind == EntityKind::Npc
                        && npc.placements.iter().any(|p| {
                            entity.position
                                == ActorPosition {
                                    x: i32::from(p.x),
                                    y: i32::from(p.y),
                                    floor: -i16::from(p.floor),
                                }
                        })
                })
            })
            .map(|entity| entity.entity)
        else {
            continue;
        };
        for text in ["hi", "trade"] {
            let disposition = npc_shop::talk(
                client.session_mut(),
                &npc_service::NpcTalkIntent {
                    npc_actor: actor,
                    text: text.into(),
                },
            )
            .await?;
            if disposition != npc_service::NpcIntentDisposition::Ok {
                break;
            }
            let domain = if text == "hi" {
                npc_service::STATE_DOMAIN_NPC_CONVERSATION
            } else {
                npc_service::STATE_DOMAIN_NPC_TRADE_WINDOW
            };
            let delta = client.session_mut().read_gated_delta(domain, 1).await?;
            if text == "hi" {
                continue;
            }
            let window = npc_service::decode_npc_trade_window(&delta.payload)
                .map_err(|e| format!("trade window: {e:?}"))?;
            let Some((index, offer)) = window.offers.iter().enumerate().find(|(_, offer)| {
                offer.side == npc_service::NpcTradeSide::Buy
                    && offer.item_definition_ref == definition.get()
            }) else {
                break;
            };
            if !count.is_multiple_of(offer.count) {
                return Err(format!(
                    "step=buy-mail-items observed=definition-{definition}-offer-count-{}-requested-{count} need=exact-purchased-unit-count",
                    offer.count
                )
                .into());
            }
            let quantity = count / offer.count;
            let before = units(client, definition);
            let observed = npc_shop::buy(
                client.session_mut(),
                &npc_service::NpcTradeIntent {
                    npc_actor: actor,
                    catalogue_revision: window.open.ok_or("trade window closed")?.1,
                    offer_index: u32::try_from(index)?,
                    side: offer.side,
                    quantity,
                    expected_unit_price: offer.unit_price,
                },
            )
            .await?;
            if observed != npc_service::NpcIntentDisposition::Ok {
                return Err(format!("step=buy-mail-items observed=npc-{}-BUY-{observed:?}-price-{}-quantity-{quantity} need=player-earned-gold-or-accepted-trade", npc.key, offer.unit_price).into());
            }
            wait_units(
                client,
                definition,
                before + u64::from(quantity) * u64::from(offer.count),
            )
            .await?;
            return Ok(());
        }
    }
    Err(format!("step=buy-mail-items observed=no-reachable-vendor-offer-definition-{definition} need=reachable-postal-NPC").into())
}

fn mailbox(
    world: &BundleWorld,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    from: TilePos,
) -> TestResult<(TilePos, u64)> {
    use crate::map::view::MapFacts;
    let keys = [
        "oteryn:item.tibia.i3501",
        "oteryn:item.tibia.i3508",
        "oteryn:item.tibia.i23399",
        "oteryn:item.tibia.i23400",
        "oteryn:item.tibia.i23401",
        "oteryn:item.tibia.i23402",
    ];
    for y in from.y.saturating_sub(160)..=from.y.saturating_add(160) {
        for x in from.x.saturating_sub(160)..=from.x.saturating_add(160) {
            let pos = TilePos {
                x,
                y,
                floor: from.floor,
            };
            let Some(tile) = world.base().tile(x, y, from.floor) else {
                continue;
            };
            for (ordinal, id) in tile
                .ids()
                .iter()
                .zip(tile.depths())
                .filter(|(_, d)| **d == 0)
                .map(|(id, _)| id)
                .enumerate()
            {
                let Ok(ordinal) = u8::try_from(ordinal) else {
                    continue;
                };
                let Some(fact) = world.facts().base_entry(pos, ordinal, *id) else {
                    continue;
                };
                let oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference) =
                    fact.definition
                else {
                    continue;
                };
                if index
                    .definition_at(reference)
                    .is_some_and(|(key, _)| keys.contains(&key))
                    && route(world, from, |at| {
                        at.floor == pos.floor && at.x.abs_diff(pos.x).max(at.y.abs_diff(pos.y)) <= 1
                    })
                    .is_some()
                    && let Some(handle) = crate::map::view::placement_key(pos, ordinal)
                {
                    return Ok((pos, handle));
                }
            }
        }
    }
    Err(
        "step=walk-mailbox observed=no-reachable-authored-mailbox need=mailbox-content-route"
            .into(),
    )
}

fn visible_mailbox_handle(
    client: &mut DevClientSession,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    at: TilePos,
) -> TestResult<u64> {
    use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};
    let keys = [
        "oteryn:item.tibia.i3501",
        "oteryn:item.tibia.i3508",
        "oteryn:item.tibia.i23399",
        "oteryn:item.tibia.i23400",
        "oteryn:item.tibia.i23401",
        "oteryn:item.tibia.i23402",
    ];
    let position = ActorPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    };
    client.session_mut().world_map_snapshot()
        .and_then(|map| map.tiles.into_iter().find(|tile| tile.position == position))
        .and_then(|tile| tile.items.into_iter().rev().find_map(|item| {
            let MapDefinition::Item(reference) = item.definition else { return None };
            if !index.definition_at(reference).is_some_and(|(key, _)| keys.contains(&key)) { return None }
            let MapOrigin::Handle(handle) = item.origin else { return None };
            Some(handle.get())
        }))
        .ok_or_else(|| "step=mailbox-handle observed=mailbox-not-handle-bearing-in-map-view need=visible-item-handle".into())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let definitions = provisioning::item_index()?;
    let letter = provisioning::item_reference(&definitions, 3505)?;
    let parcel = provisioning::item_reference(&definitions, 3503)?;
    let label = provisioning::item_reference(&definitions, 3507)?;
    let mut client = row_connect_world(clients, 0xe9).await?;
    let session = client.join_snapshot().game_session_id;
    let entities = client.world_entities().ok_or("own entities absent")?;
    let own_identity = *entities.own_identity();
    let own_name = entities
        .iter()
        .find(|e| e.entity.identity == own_identity)
        .and_then(|e| e.display_name)
        .ok_or("own display name absent")?
        .as_str()
        .map_err(|e| format!("own name: {e:?}"))?
        .to_owned();
    acquire(&mut client, world, &metadata.npcs, letter, 2).await?;
    acquire(&mut client, world, &metadata.npcs, parcel, 1).await?;
    acquire(&mut client, world, &metadata.npcs, label, 1).await?;
    row.pass(
        "player-bought-mail-items",
        (
            units(&client, letter),
            units(&client, parcel),
            units(&client, label),
        ),
    );
    let first = item(&client, letter)?;
    write(
        &mut client,
        first.handle.get(),
        format!("{own_name}\nThais"),
    )
    .await?;
    if item(&client, letter)?.handle != first.handle {
        return Err(
            "step=write-address observed=letter-handle-lost need=inventory-refresh-retaining-item"
                .into(),
        );
    }
    row.pass("letter-address-written", first.handle);
    let (mailbox_at, _) = mailbox(
        world,
        &definitions,
        tile(client.world_spatial().actor_position)?,
    )?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == mailbox_at.floor
            && at.x.abs_diff(mailbox_at.x).max(at.y.abs_diff(mailbox_at.y)) <= 1
    })
    .ok_or("step=walk-mailbox observed=route-disappeared need=reachable-adjacent-tile")?;
    walk(&mut client, world, &path).await?;
    let mailbox_handle = visible_mailbox_handle(&mut client, &definitions, mailbox_at)?;
    let before_letters = units(&client, letter);
    let outcome = post(&mut client, first.handle.get(), mailbox_handle).await?;
    if outcome != mail::MailOutcome::Posted {
        return Err(format!("step=post-letter observed={outcome:?} need=Posted").into());
    }
    wait_units(&mut client, letter, before_letters - 1).await?;
    let delivered = depot_search(&mut client, 3506).await?;
    if inbox(&delivered) == 0 {
        return Err(
            format!("step=letter-inbox observed={delivered:?} need=inbox-count-positive").into(),
        );
    }
    row.pass("letter-posted-and-delivered", delivered);
    let refused = item(&client, letter)?;
    write(
        &mut client,
        refused.handle.get(),
        "Nobody Of That Name\nThais".into(),
    )
    .await?;
    let outcome = post(&mut client, refused.handle.get(), mailbox_handle).await?;
    if outcome != mail::MailOutcome::UnknownRecipient
        || item(&client, letter)?.handle != refused.handle
    {
        return Err(format!("step=unknown-recipient-refusal observed={outcome:?}-custody-{} need=UnknownRecipient-and-retained-item", units(&client, letter)).into());
    }
    row.pass("unknown-recipient-refused", outcome);
    let parcel_item = item(&client, parcel)?;
    let label_item = item(&client, label)?;
    write(&mut client, label_item.handle.get(), own_name.clone()).await?;
    let moved = client
        .move_item(&item_view::ItemMoveIntent {
            source: label_item.handle,
            destination: item_view::ItemMoveDestination::Container(parcel_item.handle),
        })
        .await?;
    if moved.outcome != item_view::ItemMoveOutcome::Moved {
        return Err(format!(
            "step=label-into-parcel observed={moved:?} need=Moved-and-inventory-delta"
        )
        .into());
    }
    wait_units(&mut client, label, 0).await?;
    let outcome = post(&mut client, parcel_item.handle.get(), mailbox_handle).await?;
    if outcome != mail::MailOutcome::Posted {
        return Err(format!("step=post-parcel observed={outcome:?} need=Posted").into());
    }
    wait_units(&mut client, parcel, 0).await?;
    let parcel_delivery = depot_search(&mut client, 3504).await?;
    if inbox(&parcel_delivery) == 0 {
        return Err(format!(
            "step=parcel-inbox observed={parcel_delivery:?} need=inbox-count-positive"
        )
        .into());
    }
    row.pass("parcel-label-and-delivery", parcel_delivery);
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused need=accepted-logout".into());
    }
    drop(client);
    let mut fresh = row_connect_world(clients, 0xea).await?;
    if fresh.join_snapshot().game_session_id == session {
        return Err("step=fresh-admission observed=reused-session need=new-session".into());
    }
    let letter_after = depot_search(&mut fresh, 3506).await?;
    let parcel_after = depot_search(&mut fresh, 3504).await?;
    if inbox(&letter_after) == 0 || inbox(&parcel_after) == 0 {
        return Err(format!("step=durable-inbox observed=letter-{letter_after:?}-parcel-{parcel_after:?} need=persisted-inbox-counts").into());
    }
    row.pass(
        "fresh-admission-durable-inbox",
        (letter_after, parcel_after),
    );
    if fresh.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=final-logout observed=refused need=accepted-logout".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verdict_requires_each_scenario_step_once() {
        let mut row = Row::new();
        for action in ACTIONS {
            row.pass(action, "unit");
        }
        assert!(ACTIONS.iter().all(|action| row.passed.contains(action)));
        row.pass(ACTIONS[0], "duplicate");
        assert!(row.invalid);
    }
    #[test]
    fn inbox_totals_only_inbox_custody() {
        let reply = depot_search::Reply {
            outcome: depot_search::Outcome::Found,
            retrieved: 0,
            results: vec![depot_search::SearchResult {
                client_id: 3506,
                depot: 2,
                stash: 3,
                inbox: 4,
                tiers: vec![],
            }],
        };
        assert_eq!(inbox(&reply), 4);
    }
}
