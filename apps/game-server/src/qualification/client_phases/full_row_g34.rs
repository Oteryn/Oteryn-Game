//! G34: ENGNPC NPC/quest receipts (#3050). A quest NPC offer commits its track write and the
//! quest receipt atomically (`crates/gameplay/dispatch/src/gameplay_transport/npc_command/
//! reward_callbacks.rs`).
//! Start Hanna's citizen-hat mission through the authored prompt and confirmation.
//! Require the NPC conversation reply, then verify each track and its bound receipt
//! before logout and after fresh admission.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::CommandStatus;
use sqlx::Connection;

const NPC: &str = "oteryn:npc.hanna";
// Crystal Hanna walks within two tiles of her placement.
const HANNA_WANDER_RADIUS: u32 = 2;
const TRANSITION: &str = "oteryn:quest-reward/qb-citizen-hat-start";

struct Offer {
    keyword: String,
    prompt: String,
    reply: String,
    tracks: Vec<(String, i64)>,
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", detail),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G34.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

fn load_offer() -> TestResult<Offer> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/quests");
    let file: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("shared-reward-callbacks.json"))?)?;
    let callback = file["callbacks"]
        .as_array()
        .and_then(|callbacks| {
            callbacks
                .iter()
                .find(|entry| entry["key"] == TRANSITION && entry["npc"] == NPC)
        })
        .ok_or("step=select;needed=authored-citizen-hat-start-callback")?;
    let keyword = callback["keywords"][0]
        .as_str()
        .ok_or("callback has no keyword")?;
    let reply = callback["reply"][0]
        .as_str()
        .ok_or("callback has no reply")?;
    // Global Citizen Outfits Quest transcript: Hanna names all three materials after "yes".
    if !["a legion helmet", "100 chicken feathers", "50 honeycombs"]
        .iter()
        .all(|material| reply.contains(material))
    {
        return Err(
            "Hanna confirmation must name all three hat materials and their quantities".into(),
        );
    }
    let tracks = callback["storage"]
        .as_array()
        .ok_or("callback has no storage")?
        .iter()
        .map(|entry| {
            let key = entry["track"]
                .as_str()
                .ok_or("callback track key missing")?;
            let set = entry["set"].as_i64().ok_or("callback track set missing")?;
            Ok((key.to_owned(), set))
        })
        .collect::<TestResult<Vec<_>>>()?;
    if tracks.is_empty() {
        return Err("callback does not write a track".into());
    }
    let offers: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("shared-reward-offers.json"))?)?;
    let prompt = offers["offers"]
        .as_array()
        .and_then(|offers| {
            offers.iter().find(|entry| {
                entry["npc"] == NPC
                    && entry["keywords"][0] == "addon"
                    && entry["topic"] == 0
                    && entry["next_topic"] == callback["topic"]
                    && entry["storage"] == callback["storage"]
            })
        })
        .and_then(|entry| entry["reply"][0].as_str())
        .ok_or("missing authored Hanna addon prompt")?;
    Ok(Offer {
        prompt: prompt.into(),
        keyword: keyword.into(),
        reply: reply.into(),
        tracks,
    })
}

async fn track_value(clients: &SeamClients<'_>, track: &str) -> TestResult<Option<i64>> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let value: Option<i64> = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE character_id = encode($1, 'hex')::uuid AND track_key = $2",
    )
    .bind(clients.characters[1].as_slice())
    .bind(track)
    .fetch_optional(&mut database)
    .await?;
    database.close().await?;
    Ok(value)
}

/// Identify the single command receipt bound to the track’s exact before and after values.
async fn bound_receipt(
    clients: &SeamClients<'_>,
    track: &str,
    before: i64,
    set: i64,
) -> TestResult<String> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let receipts: Vec<String> = sqlx::query_scalar(
        "SELECT concat_ws(':', r.cause_id, r.cause_ordinal, r.committed_character_revision) \
         FROM game_character_quest_receipts r \
         JOIN game_character_quest_tracks t ON t.character_id = r.character_id AND t.track_key = $2 \
         WHERE r.character_id = encode($1, 'hex')::uuid \
           AND r.committed_character_revision = t.committed_character_revision \
           AND t.quest_key = r.quest_key AND t.value = $3 \
           AND r.values_after[array_position(r.track_keys, $2)] = $3 \
           AND r.values_before[array_position(r.track_keys, $2)] = $4 \
           AND r.transition_key = $5 AND r.cause_kind = 'command' LIMIT 2",
    )
    .bind(clients.characters[1].as_slice())
    .bind(track)
    .bind(set)
    .bind(before)
    .bind(TRANSITION)
    .fetch_all(&mut database)
    .await?;
    database.close().await?;
    match receipts.as_slice() {
        [receipt] => Ok(receipt.clone()),
        _ => Err(format!(
            "track={track};expected=one-bound-receipt;observed={}",
            receipts.len()
        )
        .into()),
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout;observed=refused".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    use oteryn_protocol_oteryn::npc_service as wire;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let offer = load_offer()?;
    let npc = metadata
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == NPC)
        .ok_or("step=select;needed=hanna-placement")?;
    let mut client = connect_world(clients, 0xe4).await?;
    let here = tile(client.world_spatial().actor_position)?;
    let path = route(world, here, |at| {
        npc.placements.iter().any(|p| {
            p.floor == at.floor
                && u32::from(at.x.abs_diff(p.x).max(at.y.abs_diff(p.y))) <= wire::TALK_RANGE_TILES
        })
    })
    .ok_or("step=select;needed=walkable-hanna-approach")?;
    if path.is_empty() {
        return Err("step=select;observed=already-at-approach".into());
    }
    let mut before = Vec::new();
    for (track, set) in &offer.tracks {
        let value = track_value(clients, track).await?;
        if value.unwrap_or(0) != 0 || value == Some(*set) {
            let _ = logout(&mut client).await;
            return Err(format!("step=initial;track={track};observed=already:{value:?}").into());
        }
        before.push(value);
    }
    let mut wait_ms = 0;
    for direction in path {
        if wait_ms != 0 {
            client
                .service_liveness(Duration::from_millis(u64::from(wait_ms)))
                .await?;
        }
        let from = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, from, direction)
            .ok_or("step=walk;observed=missing-planned-landing")?;
        let outcome = client.step(direction).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if outcome.status != CommandStatus::Accepted
            || outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.as_ref().is_none_or(|delta| {
                delta.server_sequence <= outcome.result_server_sequence
                    || tile(delta.value.actor_position).ok() != Some(expected)
            })
            || actual != expected
            || actual == from
        {
            return Err(format!(
                "step=walk;from={from:?};direction={direction:?};expected={expected:?};actual={actual:?};disposition={:?}",
                outcome.disposition
            )
            .into());
        }
        wait_ms = outcome.step_wait_ms;
        verdict::Harness::global().progress("walk");
        client.take_events();
    }
    let actor = {
        let player = client.world_spatial().actor_position;
        let mut visible_hanna = client
            .world_entities()
            .ok_or("step=walk;observed=missing-entity-view")?
            .iter()
            .filter(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Npc
                    && entity
                        .display_name
                        .is_some_and(|name| name.as_str() == Ok("Hanna"))
                    && entity.position.floor == player.floor
                    && entity
                        .position
                        .x
                        .abs_diff(player.x)
                        .max(entity.position.y.abs_diff(player.y))
                        <= wire::TALK_RANGE_TILES
                    && npc.placements.iter().any(|p| {
                        entity.position.floor == -i16::from(p.floor)
                            && entity
                                .position
                                .x
                                .abs_diff(i32::from(p.x))
                                .max(entity.position.y.abs_diff(i32::from(p.y)))
                                <= HANNA_WANDER_RADIUS
                    })
            });
        let actor = visible_hanna
            .next()
            .ok_or("step=walk;observed=npc-not-visible")?
            .entity;
        if visible_hanna.next().is_some() {
            return Err("step=walk;observed=ambiguous-hanna".into());
        }
        actor
    };
    for (text, expected_line) in [
        ("hi", None),
        ("addon", Some(offer.prompt.as_str())),
        (offer.keyword.as_str(), Some(offer.reply.as_str())),
    ] {
        if text == offer.keyword {
            for ((track, _), prior) in offer.tracks.iter().zip(&before) {
                if track_value(clients, track).await? != *prior {
                    return Err(format!(
                        "step=pre-confirmation;track={track};observed=premature-write"
                    )
                    .into());
                }
            }
        }
        client.take_events();
        let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
            npc_actor: actor,
            text: text.into(),
        })
        .map_err(|error| format!("step=talk;encode={error:?}"))?;
        let reply = client
            .session_mut()
            .gated_command(
                wire::CAPABILITY_NPC_SERVICE_V1,
                wire::COMMAND_TYPE_NPC_TALK_INTENT,
                &payload,
            )
            .await?;
        if reply.status != CommandStatus::Accepted
            || wire::decode_npc_intent_result(&reply.payload).ok()
                != Some(wire::NpcIntentDisposition::Ok)
        {
            return Err(format!("step=talk;text={text};observed={reply:?}").into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            if client.take_events().into_iter().any(|event| {
                matches!(event, SessionEvent::NpcConversation(delta)
                    if delta.server_sequence > reply.server_sequence
                        && delta.value.open == Some((actor, wire::NpcConversationState::Open))
                        && expected_line.is_none_or(|expected| delta.value.lines.iter().any(|line| line == expected)))
            }) {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("step=reply;npc={NPC};keyword={text};observed=expected-conversation-effect-missing").into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
        }
    }
    let mut committed_receipts = Vec::new();
    for ((track, set), prior) in offer.tracks.iter().zip(&before) {
        let value = track_value(clients, track).await?;
        if value != Some(*set) {
            return Err(
                format!("step=receipt;track={track};observed={value:?};expected={set}").into(),
            );
        }
        committed_receipts.push(bound_receipt(clients, track, prior.unwrap_or(0), *set).await?);
    }
    if committed_receipts.windows(2).any(|pair| pair[0] != pair[1]) {
        return Err("step=receipt;observed=tracks-committed-by-different-commands".into());
    }
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xe5).await?;
    for (((track, set), prior), committed) in
        offer.tracks.iter().zip(before).zip(committed_receipts)
    {
        let value = track_value(clients, track).await?;
        let receipt = bound_receipt(clients, track, prior.unwrap_or(0), *set).await?;
        if value != Some(*set) || receipt != committed {
            return Err(format!(
                "step=fresh;track={track};observed={value:?};expected={set};same-commit={}",
                receipt == committed
            )
            .into());
        }
    }
    logout(&mut fresh).await?;
    Ok(format!(
        "npc={NPC};transition={TRANSITION};durable-tracks={}",
        offer.tracks.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_confirmation_writes_tracks() -> TestResult {
        let offer = load_offer()?;
        assert_eq!(offer.keyword, "yes");
        assert!(!offer.reply.is_empty());
        assert_eq!(offer.tracks.len(), 2);
        assert!(offer.tracks.iter().all(|(_, set)| *set == 1));
        assert_eq!(token("a  b"), "a-b");
        Ok(())
    }
}
