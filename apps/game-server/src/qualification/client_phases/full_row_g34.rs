//! G34: ENGNPC NPC/quest receipts (#3050). A quest NPC offer commits its track write and the
//! quest receipt atomically (`crates/gameplay/dispatch/src/gameplay_transport/npc_command/
//! reward_callbacks.rs`).
//! 1. Pick, from the committed offers (`content/quests/shared-reward-offers.json` and
//!    `npc-offers/*.json`), the nearest NPC whose first-step offer (topic 0, plain keyword,
//!    track write) a fresh character can trigger, reachable by walking.
//! 2. Walk to it, say "hi" and the offer keyword through the real NPC talk command; require
//!    `Ok` replies and the NPC's reply text in the chat log.
//! 3. Read the track before the talk (it must differ from the offer's `set`), then require the
//!    durable track (`game_character_quest_tracks`, read-only SQL) to hold `set` and a receipt
//!    that wrote that track from another value at the track's own last-write revision.
//! 4. Log out, freshly admit and require the track value and that receipt again.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::chat::ChatLine;
use sqlx::Connection;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Offer {
    npc: String,
    keyword: String,
    reply: String,
    track: String,
    set: i64,
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients))).await;
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

/// First-step offers of one offers file: topic 0, only the plain fields, a first storage
/// condition a fresh (absent or zero) track satisfies and a non-preserving write of `set >= 0`.
fn parse_offers(file: &serde_json::Value) -> Vec<Offer> {
    let Some(offers) = file["offers"].as_array() else {
        return Vec::new();
    };
    offers
        .iter()
        .filter_map(|offer| {
            let object = offer.as_object()?;
            if !object.keys().all(|key| {
                matches!(
                    key.as_str(),
                    "npc" | "keywords" | "topic" | "next_topic" | "storage" | "reply"
                )
            }) || offer["topic"].as_i64()? != 0
            {
                return None;
            }
            let storage = offer["storage"].as_array()?;
            if storage.is_empty()
                || !storage
                    .iter()
                    .all(|entry| entry["minimum"].as_i64().is_some_and(|m| m <= 0))
            {
                return None;
            }
            let first = storage.first()?;
            let set = first["set"].as_i64().filter(|set| *set >= 0)?;
            if first["preserve"].as_bool().unwrap_or(false) {
                return None;
            }
            let keyword = offer["keywords"].as_array()?.first()?.as_str()?.to_owned();
            Some(Offer {
                npc: offer["npc"].as_str()?.to_owned(),
                keyword,
                reply: offer["reply"].as_array()?.first()?.as_str()?.to_owned(),
                track: first["track"].as_str()?.to_owned(),
                set,
            })
        })
        .collect()
}

fn load_offers() -> TestResult<Vec<Offer>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/quests");
    let mut offers = parse_offers(&serde_json::from_slice(&std::fs::read(
        root.join("shared-reward-offers.json"),
    )?)?);
    for entry in std::fs::read_dir(root.join("npc-offers"))? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            offers.extend(parse_offers(&serde_json::from_slice(&std::fs::read(
                path,
            )?)?));
        }
    }
    Ok(offers)
}

/// Whether the chat log holds a line that starts with the reply's first words.
fn reply_seen(client: &DevClientSession, reply: &str) -> bool {
    let head: String = reply.chars().take(24).collect();
    client.chat_log().is_some_and(|log| {
        log.lines().any(|line| match line {
            ChatLine::Local { text, .. } | ChatLine::Private { text, .. } => text.contains(&head),
            _ => false,
        })
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

/// Receipts of the character that wrote `track` from another value to `set` at the same
/// character revision the track row records as its last write (read-only SQL).
async fn bound_receipts(clients: &SeamClients<'_>, track: &str, set: i64) -> TestResult<i64> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts r \
         JOIN game_character_quest_tracks t ON t.character_id = r.character_id AND t.track_key = $2 \
         WHERE r.character_id = encode($1, 'hex')::uuid \
           AND r.committed_character_revision = t.committed_character_revision \
           AND t.quest_key = r.quest_key AND t.value = $3 \
           AND r.values_after[array_position(r.track_keys, $2)] = $3 \
           AND r.values_before[array_position(r.track_keys, $2)] IS DISTINCT FROM $3",
    )
    .bind(clients.characters[1].as_slice())
    .bind(track)
    .bind(set)
    .fetch_one(&mut database)
    .await?;
    database.close().await?;
    Ok(count)
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
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
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
    let offers = load_offers()?;
    let mut client = connect_world(clients, 0xe4).await?;
    let here = tile(client.world_spatial().actor_position)?;
    let (offer, npc, path) = offers
        .iter()
        .filter_map(|offer| {
            let npc = metadata.npcs.npcs.iter().find(|npc| npc.key == offer.npc)?;
            let path = route(world, here, |at| {
                npc.placements
                    .iter()
                    .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
            })?;
            Some((offer, npc, path))
        })
        .min_by_key(|(_, _, path)| path.len())
        .ok_or("step=select;needed=walkable-quest-npc-with-first-step-offer")?;
    let track_before = track_value(clients, &offer.track).await?;
    if track_before == Some(offer.set) {
        let _ = logout(&mut client).await;
        return Err(format!(
            "step=initial;track={};observed=already:{track_before:?};needed=track-not-yet-set",
            offer.track
        )
        .into());
    }
    walk(&mut client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|p| {
                        entity.position.x == i32::from(p.x)
                            && entity.position.y == i32::from(p.y)
                            && entity.position.floor == -i16::from(p.floor)
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or("step=walk;observed=npc-not-visible")?;
    for text in ["hi", offer.keyword.as_str()] {
        let reply = runtime::talk(
            client.session_mut(),
            &wire::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        if reply != wire::NpcIntentDisposition::Ok {
            return Err(format!("step=talk;text={text};observed={reply:?}").into());
        }
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !reply_seen(&client, &offer.reply) {
        if tokio::time::Instant::now() >= deadline {
            let _ = logout(&mut client).await;
            return Err(format!(
                "step=reply;npc={};keyword={};observed=no-npc-reply-line",
                offer.npc, offer.keyword
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    let value = track_value(clients, &offer.track).await?;
    let bound = bound_receipts(clients, &offer.track, offer.set).await?;
    logout(&mut client).await?;
    drop(client);
    if value != Some(offer.set) || bound == 0 {
        return Err(format!(
            "step=receipt;track={};observed=value:{track_before:?}->{value:?},bound-receipts:{bound};needed=change-to-set:{}-with-receipt-of-that-revision",
            offer.track, offer.set
        )
        .into());
    }
    let mut fresh = connect_world(clients, 0xe5).await?;
    let durable = track_value(clients, &offer.track).await;
    let durable_bound = bound_receipts(clients, &offer.track, offer.set).await;
    logout(&mut fresh).await?;
    if durable? != Some(offer.set) || durable_bound? == 0 {
        return Err("step=fresh;observed=track-or-bound-receipt-not-durable".into());
    }
    Ok(format!("npc={};track-set={}", offer.npc, offer.set))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_offers_keeps_only_fresh_first_step_offers() {
        let file = serde_json::json!({"offers": [
            {"npc":"oteryn:npc.a","keywords":["task"],"topic":0,"next_topic":1,
             "storage":[{"track":"oteryn:quest-progress/x","minimum":-1,"maximum":0,"set":1}],
             "reply":["Help me."]},
            {"npc":"oteryn:npc.b","keywords":["task"],"topic":0,"next_topic":1,
             "storage":[{"track":"oteryn:quest-progress/y","minimum":2,"maximum":2,"set":3}],
             "reply":["No."]},
            {"npc":"oteryn:npc.c","keywords":["task"],"topic":0,"next_topic":0,
             "storage":[{"track":"oteryn:quest-progress/z","minimum":0,"maximum":0,"set":0,"preserve":true}],
             "reply":["No."]},
            {"npc":"oteryn:npc.d","keywords":["task"],"topic":0,"next_topic":0,"items":[1],
             "storage":[{"track":"oteryn:quest-progress/w","minimum":0,"maximum":0,"set":1}],
             "reply":["No."]}
        ]});
        let offers = parse_offers(&file);
        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].npc, "oteryn:npc.a");
        assert_eq!(offers[0].set, 1);
        assert_eq!(token("a  b"), "a-b");
    }
}
