//! E09 client replay capture: buy two letters at a postal NPC, address and post one to the
//! admitted character through an authored mailbox, then post the other to an unknown recipient.
//! Client owner: the Mail panel in `apps/client/src/features/0115-mail/protocol.rs`.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind};
use oteryn_protocol_oteryn::{item_view, mail, npc_service};
use std::num::NonZeroU32;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

/// Sends one MAIL command and returns the status and the single raw outcome byte payload.
async fn mail_command(
    client: &mut DevClientSession,
    intent: mail::MailIntent,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let payload = mail::encode_intent(&intent).map_err(|e| format!("mail encode: {e:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            u32::from(mail::CAPABILITY_MAIL_V1),
            u32::from(mail::COMMAND_TYPE_MAIL),
            &payload,
        )
        .await?;
    Ok((reply.status, reply.payload))
}

fn units(client: &DevClientSession, definition: NonZeroU32) -> u64 {
    units_from(client.inventory(), definition)
}

fn units_from(
    input: Option<&oteryn_dev_client::CharacterInventory>,
    definition: NonZeroU32,
) -> u64 {
    input.map_or(0, |inventory| {
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

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(300);
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
    let mut client = connect_world(clients, 0x3a).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
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
    let first = item(&client, letter)?;
    let address = format!("{own_name}\nThais");
    let (status, written) = mail_command(
        &mut client,
        mail::MailIntent::WriteText {
            item_handle: first.handle.get(),
            text: address.clone(),
        },
    )
    .await?;
    if (status, written.as_slice()) != (CommandStatus::Accepted, &[2][..]) {
        return Err(format!("write reply {status:?} {written:?}").into());
    }
    let (mailbox_at, _) = mailbox(
        world,
        &definitions,
        tile(client.world_spatial().actor_position)?,
    )?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == mailbox_at.floor
            && at.x.abs_diff(mailbox_at.x).max(at.y.abs_diff(mailbox_at.y)) <= 1
    })
    .ok_or("mailbox route disappeared")?;
    walk(&mut client, world, &path).await?;
    let mailbox_handle = visible_mailbox_handle(&mut client, &definitions, mailbox_at)?;
    let before = units(&client, letter);
    let (status, posted) = mail_command(
        &mut client,
        mail::MailIntent::Post {
            item_handle: first.handle.get(),
            mailbox_handle,
        },
    )
    .await?;
    if (status, posted.as_slice()) != (CommandStatus::Accepted, &[1][..]) {
        return Err(format!("post reply {status:?} {posted:?}").into());
    }
    wait_units(&mut client, letter, before - 1).await?;
    let second = item(&client, letter)?;
    let (status, text) = mail_command(
        &mut client,
        mail::MailIntent::WriteText {
            item_handle: second.handle.get(),
            text: "Nobody Of That Name\nThais".into(),
        },
    )
    .await?;
    if (status, text.as_slice()) != (CommandStatus::Accepted, &[2][..]) {
        return Err(format!("second write reply {status:?} {text:?}").into());
    }
    let (status, unknown) = mail_command(
        &mut client,
        mail::MailIntent::Post {
            item_handle: second.handle.get(),
            mailbox_handle,
        },
    )
    .await?;
    if (status, unknown.as_slice()) != (CommandStatus::Accepted, &[4][..])
        || item(&client, letter)?.handle != second.handle
    {
        return Err(format!("unknown-recipient reply {status:?} {unknown:?}").into());
    }
    let expected = format!(
        "write_payload={}\npost_payload={}\nunknown_payload={}\nwrite=Item text saved\npost=Mail posted\nunknown=Unknown recipient\n",
        written[0], posted[0], unknown[0]
    );
    capture("e09-write.bin", &written)?;
    capture("e09-post.bin", &posted)?;
    capture("e09-unknown.bin", &unknown)?;
    capture("e09-expected.txt", expected.as_bytes())?;

    Ok("write=2 post=1 unknown=4".into())

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=E09.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E09.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=E09.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_projection_cannot_supply_evidence() {
        assert_eq!(units_from(None, std::num::NonZeroU32::MIN), 0);
    }
}
