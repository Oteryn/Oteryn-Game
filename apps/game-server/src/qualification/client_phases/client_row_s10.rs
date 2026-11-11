//! S10 client replay capture: today's Rashid placement as an ordinary player. The driver walks
//! to the authored Rashid actor, says `hi` then `trade`, asserts the accepted dispositions, and
//! writes the raw conversation and trade-window delta payloads (`Session::read_gated_delta`)
//! that the session replay consumes (`crates/session/src/tests/e2e_s10.rs`). It buys nothing.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef};
use oteryn_protocol_oteryn::npc_service as wire;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

fn identity(npc: EntityRef) -> String {
    let hex: String = npc
        .identity
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("{hex}:{}", npc.generation)
}

async fn talk(
    client: &mut DevClientSession,
    npc: EntityRef,
    text: &str,
    previous: &mut Option<u64>,
) -> TestResult<(String, u64)> {
    tokio::time::sleep(Duration::from_millis(260)).await;
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(|error| format!("step={text} encode: {error:?}"))?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let reply = loop {
        let reply = client
            .session_mut()
            .gated_command(
                wire::CAPABILITY_NPC_SERVICE_V1,
                wire::COMMAND_TYPE_NPC_TALK_INTENT,
                &payload,
            )
            .await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!("step={text} command status {:?}", reply.status).into());
        }
        let disposition = wire::decode_npc_intent_result(&reply.payload)
            .map_err(|error| format!("step={text} result: {error:?}"))?;
        if disposition == wire::NpcIntentDisposition::RateLimited
            && tokio::time::Instant::now() < deadline
        {
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
            continue;
        }
        if disposition != wire::NpcIntentDisposition::Ok {
            return Err(format!("step={text} observed {disposition:?}").into());
        }
        break reply;
    };
    let delta = loop {
        let delta = client
            .session_mut()
            .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
            .await?;
        if delta.server_sequence > reply.server_sequence {
            break delta;
        }
    };
    if previous.is_some_and(|new| new != delta.base_revision) {
        return Err(format!(
            "step={text} conversation revision gap at {}",
            delta.base_revision
        )
        .into());
    }
    *previous = Some(delta.new_revision);
    let conversation = wire::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("step={text} conversation: {error:?}"))?;
    if conversation.open.map(|(actor, _)| actor) != Some(npc) || conversation.lines.is_empty() {
        return Err(format!("step={text} different conversation {conversation:?}").into());
    }
    capture(&format!("s10-{text}-conversation.bin"), &delta.payload)?;
    Ok((
        format!(
            "{text}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            wire::STATE_DOMAIN_NPC_CONVERSATION,
            delta.server_sequence,
            delta.base_revision,
            delta.new_revision,
            identity(npc),
            conversation.lines.join("\u{1f}")
        ),
        reply.server_sequence,
    ))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0x42, 1).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let rashid = metadata
            .npcs
            .npcs
            .iter()
            .find(|npc| npc.key == "oteryn:npc.rashid")
            .ok_or("S10 client capture: today's Rashid placement absent")?;
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
            rashid.placements.iter().any(|place| {
                place.floor == at.floor && at.x.abs_diff(place.x).max(at.y.abs_diff(place.y)) <= 3
            })
        })
        .ok_or("S10 client capture: Rashid unreachable")?;
        walk(&mut client, world, &path).await?;
        let actor = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entity| {
                    entity.kind == EntityKind::Npc
                        && rashid.placements.iter().any(|place| {
                            entity.position.x == i32::from(place.x)
                                && entity.position.y == i32::from(place.y)
                                && entity.position.floor == -i16::from(place.floor)
                        })
                })
            })
            .map(|entity| entity.entity)
            .ok_or("S10 client capture: Rashid not visible")?;
        let mut previous = None;
        let result: TestResult<String> = async {
            let (mut expected, _) = talk(&mut client, actor, "hi", &mut previous).await?;
            let (line, reply_sequence) = talk(&mut client, actor, "trade", &mut previous).await?;
            expected.push_str(&line);
            let window_delta = loop {
                let delta = client
                    .session_mut()
                    .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
                    .await?;
                if delta.server_sequence > reply_sequence {
                    break delta;
                }
            };
            let window = wire::decode_npc_trade_window(&window_delta.payload)
                .map_err(|error| format!("step=trade window: {error:?}"))?;
            let revision = match window.open {
                Some((opened, revision)) if opened == actor => revision,
                other => return Err(format!("step=trade window opened {other:?}").into()),
            };
            if window.offers.is_empty()
                || window
                    .offers
                    .iter()
                    .any(|offer| offer.count == 0 || offer.unit_price == 0)
            {
                return Err(format!("step=trade window offers {:?}", window.offers).into());
            }
            let mut lines = vec![format!("revision={revision}")];
            for offer in &window.offers {
                lines.push(format!(
                    "{:?} {} x{} @{}",
                    offer.side, offer.item_definition_ref, offer.count, offer.unit_price
                ));
            }
            capture("s10-trade-window.bin", &window_delta.payload)?;
            expected.push_str(&format!(
                "trade\t{}\t{}\t{}\t{}\t{}\t{}\n",
                wire::STATE_DOMAIN_NPC_TRADE_WINDOW,
                window_delta.server_sequence,
                window_delta.base_revision,
                window_delta.new_revision,
                identity(actor),
                lines.join("\u{1f}")
            ));
            capture("s10-expected.txt", expected.as_bytes())?;
            Ok(format!("offers={}", window.offers.len()))
        }
        .await;

        result
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=S10.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S10.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=S10.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn entity_identity_preserves_incarnation() {
        assert_eq!(
            identity(EntityRef {
                identity: [0; 16],
                generation: u64::MAX
            }),
            format!("{}:{}", "00".repeat(16), u64::MAX)
        );
    }
}
