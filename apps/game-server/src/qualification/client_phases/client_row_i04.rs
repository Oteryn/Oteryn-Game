//! I04 client replay capture: real MAIL write replies for a write-once and a rewritable item.
//!
//! The replies are replayed through the production reducer `apply` in
//! `apps/client/src/features/0115-mail/protocol.rs` (wired by `Outbox::properties_result`).
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind};
use oteryn_protocol_oteryn::{mail, npc_service};
use std::num::NonZeroU32;

const WRITE_ONCE: u32 = 2818;
const WRITTEN: u32 = 2834;
const REWRITABLE: u32 = 2817;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = row_complete(journey(clients)).await;
    match result.unwrap_or_else(|_| Err("I04 client capture timed out".into())) {
        Ok(()) => evidence(
            "e2e_all scenario=I04.client status=PASS evidence=mail-write-once-too-long-and-rewrite",
        ),
        Err(error) => evidence(&format!(
            "e2e_all scenario=I04.client status=FAIL evidence={error}"
        )),
    }
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
            .filter(|item| item.item_definition_ref == definition)
            .map(|item| u64::from(item.count.get()))
            .sum()
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
            return Err(
                format!("inventory definition {definition} did not reach {expected}").into(),
            );
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
            if npc_shop::talk(
                client.session_mut(),
                &npc_service::NpcTalkIntent {
                    npc_actor: actor,
                    text: text.into(),
                },
            )
            .await?
                != npc_service::NpcIntentDisposition::Ok
            {
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
            let before = units(client, definition);
            let disposition = npc_shop::buy(
                client.session_mut(),
                &npc_service::NpcTradeIntent {
                    npc_actor: actor,
                    catalogue_revision: window.open.ok_or("closed trade window")?.1,
                    offer_index: u32::try_from(index)?,
                    side: offer.side,
                    quantity: 1,
                    expected_unit_price: offer.unit_price,
                },
            )
            .await?;
            if disposition != npc_service::NpcIntentDisposition::Ok {
                return Err(format!("buy {definition} refused: {disposition:?}").into());
            }
            wait_units(client, definition, before + u64::from(offer.count)).await?;
            return Ok(());
        }
    }
    Err(format!("no reachable vendor for {definition}").into())
}

async fn write(client: &mut DevClientSession, handle: u64, text: String) -> TestResult<Vec<u8>> {
    let payload = mail::encode_intent(&mail::MailIntent::WriteText {
        item_handle: handle,
        text,
    })
    .map_err(|e| format!("mail encode: {e:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            u32::from(mail::CAPABILITY_MAIL_V1),
            u32::from(mail::COMMAND_TYPE_MAIL),
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("mail command status: {:?}", reply.status).into());
    }
    Ok(reply.payload)
}

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&path).join(name), bytes)?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(180);
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
    let once = provisioning::item_reference(&definitions, WRITE_ONCE)?;
    let written = provisioning::item_reference(&definitions, WRITTEN)?;
    let rewrite = provisioning::item_reference(&definitions, REWRITABLE)?;
    let mut client = connect_world(clients, 0x04).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    acquire(&mut client, world, &metadata.npcs, once).await?;
    acquire(&mut client, world, &metadata.npcs, rewrite).await?;
    let once_item = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == once)
        })
        .copied()
        .ok_or("write-once item absent")?;
    let first = write(
        &mut client,
        once_item.handle.get(),
        "One inscription: żółw 🐢".into(),
    )
    .await?;
    wait_units(&mut client, written, 1).await?;
    let refused = write(&mut client, once_item.handle.get(), "second".into()).await?;
    let rewrite_item = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == rewrite)
        })
        .copied()
        .ok_or("rewritable item absent")?;
    let too_long = write(&mut client, rewrite_item.handle.get(), "x".repeat(7_996)).await?;
    let rewrite_first = write(&mut client, rewrite_item.handle.get(), "first".into()).await?;
    let rewrite_second = write(&mut client, rewrite_item.handle.get(), "second".into()).await?;
    if first != [2] || too_long != [10] || rewrite_first != [2] || refused == [2] {
        return Err(format!(
            "I04 unexpected mail replies: first={first:?} refused={refused:?} too_long={too_long:?} rewrite_first={rewrite_first:?}"
        )
        .into());
    }
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let expected = format!(
        "write_once={}\nrewrite_refusal={}\ntoo_long={}\nrewrite_first={}\nrewrite_second={}\n",
        hex(&first),
        hex(&refused),
        hex(&too_long),
        hex(&rewrite_first),
        hex(&rewrite_second)
    );
    for (name, payload) in [
        ("i04-write-once.bin", &first),
        ("i04-write-once-refused.bin", &refused),
        ("i04-too-long.bin", &too_long),
        ("i04-rewrite-first.bin", &rewrite_first),
        ("i04-rewrite-second.bin", &rewrite_second),
    ] {
        capture(name, payload)?;
    }
    capture("i04-expected.txt", expected.as_bytes())?;
    Ok(())

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_projection_cannot_supply_evidence() {
        assert_eq!(units_from(None, std::num::NonZeroU32::MIN), 0);
    }
}
