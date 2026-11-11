use super::*;
use oteryn_dev_client::{DevClientSession, client_runtime::npc_shop};
use oteryn_protocol_oteryn::{chat::ChatLine, npc_service as npc};
use sqlx::Connection;

const LEGS: &[(&str, &str, &[&str], &str)] = &[
    (
        "albinius",
        "quest-rules-supplemental/questpc2-albinius.json",
        &["temple", "yes"],
        "/contribute",
    ),
    (
        "gerimor",
        "quest-rules-supplemental/questpc2-gerimor.json",
        &["mission", "minotaurs", "yes"],
        "/accept-minotaurs-first",
    ),
    (
        "gnomus",
        "quest-rules-supplemental/questpc2-gnomus.json",
        &["ordnance", "yes"],
        "/accept-first",
    ),
    (
        "henricus",
        "quest-rules-supplemental/questpc2-henricus.json",
        &["join", "yes"],
        "/join-accept",
    ),
    (
        "oldrak",
        "quest-rules-supplemental/questpc2-oldrak.json",
        &["mission", "yes"],
        "/mission-accept",
    ),
    (
        "mystery",
        "quest-rules/quest-authored-mystery-of-the-valley.json",
        &["statue"],
        "/statue",
    ),
    (
        "father",
        "quest-rules/quest-batch3-a-fathers_burden.json",
        &["mission", "yes"],
        "/mission-accept",
    ),
];

fn token(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join("-")
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(900), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", detail),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "needed:journey-completes-within-900s".into()),
    };
    evidence(&format!(
        "e2e_all scenario=G40.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn document(path: &str) -> TestResult<serde_json::Value> {
    Ok(serde_json::from_slice(&std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/dialogues")
            .join(path),
    )?)?)
}

fn rule<'a>(document: &'a serde_json::Value, suffix: &str) -> TestResult<&'a serde_json::Value> {
    document["rules"]
        .as_array()
        .and_then(|rules| {
            rules.iter().find(|rule| {
                rule["key"]
                    .as_str()
                    .is_some_and(|key| key.ends_with(suffix))
            })
        })
        .ok_or_else(|| format!("needed:authored-rule:{suffix}").into())
}

fn writes(rule: &serde_json::Value) -> TestResult<Vec<(String, i64)>> {
    let writes: Vec<_> = rule["effects"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|effect| effect["kind"] == "set")
        .filter_map(|effect| {
            Some((
                effect["track"].as_str()?.to_owned(),
                effect["value"].as_i64()?,
            ))
        })
        .collect();
    if writes.is_empty() {
        return Err("needed:durable-first-step-track-write".into());
    }
    Ok(writes)
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("needed:accepted-final-logout".into());
    }
    Ok(())
}

async fn talk(
    client: &mut DevClientSession,
    actor: oteryn_dev_client::EntityRef,
    text: &str,
) -> TestResult {
    let reply = npc_shop::talk(
        client.session_mut(),
        &npc::NpcTalkIntent {
            npc_actor: actor,
            text: text.into(),
        },
    )
    .await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!("needed:npc-talk:{text};reply:{reply:?}").into());
    }
    Ok(())
}

async fn approach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    key: &str,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let npc = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == key)
        .ok_or_else(|| format!("needed:placed-npc:{key}"))?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        npc.placements
            .iter()
            .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
    })
    .ok_or_else(|| format!("needed:player-travel-to:{key}"))?;
    walk(client, world, &path).await?;
    client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|p| {
                        entry.position.x == i32::from(p.x)
                            && entry.position.y == i32::from(p.y)
                            && entry.position.floor == -i16::from(p.floor)
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or_else(|| format!("needed:visible-npc:{key}").into())
}

async fn track(clients: &SeamClients<'_>, key: &str) -> TestResult<Option<i64>> {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let value = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(clients.characters[1].as_slice()).bind(key).fetch_optional(&mut db).await?;
    db.close().await?;
    Ok(value)
}

// Bind each changed track to this exact transition, character revision and before/after value.
async fn receipt(
    clients: &SeamClients<'_>,
    rule: &serde_json::Value,
    key: &str,
    before: Option<i64>,
    after: i64,
) -> TestResult<i64> {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let count = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts r JOIN game_character_quest_tracks t ON t.character_id=r.character_id AND t.track_key=$2 WHERE r.character_id=encode($1,'hex')::uuid AND r.transition_key=$3 AND r.quest_key=t.quest_key AND r.committed_character_revision=t.committed_character_revision AND t.value=$4 AND r.values_after[array_position(r.track_keys,$2)]=$4 AND r.values_before[array_position(r.track_keys,$2)] IS NOT DISTINCT FROM $5")
        .bind(clients.characters[1].as_slice()).bind(key).bind(rule["key"].as_str().ok_or("rule-key")?).bind(after).bind(before).fetch_one(&mut db).await?;
    db.close().await?;
    Ok(count)
}

async fn quest(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    file: &str,
    words: &[&str],
    suffix: &str,
) -> TestResult<Vec<(String, i64, Option<i64>)>> {
    let document = document(file)?;
    let rule = rule(&document, suffix)?;
    let expected = writes(rule)?;
    let actor = approach(client, world, npcs, rule["npc"].as_str().ok_or("rule-npc")?).await?;
    let mut proof = Vec::new();
    for (key, after) in expected {
        let before = track(clients, &key).await?.or_else(|| {
            document["tracks"]
                .as_array()?
                .iter()
                .find(|track| track["key"] == key)?["initial"]
                .as_i64()
        });
        proof.push((key, after, before));
    }
    if proof
        .iter()
        .all(|(_, after, before)| *before == Some(*after))
    {
        return Err("needed:unstarted-quest-step".into());
    }
    talk(client, actor, "hi").await?;
    for word in words {
        talk(client, actor, word).await?;
    }
    let reply = rule["reply"]
        .as_array()
        .and_then(|replies| replies.first())
        .and_then(|reply| reply.as_str())
        .ok_or("rule-reply")?;
    let head: String = reply.chars().take(32).collect();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
    loop {
        if client.chat_log().is_some_and(|log| log.lines().any(|line| matches!(line, ChatLine::Local { text, .. } | ChatLine::Private { text, .. } if text.contains(&head)))) { break; }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "needed:quest-guards-and-reply;rule:{};guards:{}",
                rule["key"],
                token(&rule["guards"].to_string())
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    for (key, after, before) in &proof {
        if track(clients, key).await? != Some(*after)
            || (*before != Some(*after) && receipt(clients, rule, key, *before, *after).await? == 0)
        {
            return Err(format!("needed:state-and-bound-receipt:{key}:{before:?}->{after}").into());
        }
    }
    Ok(proof)
}

fn carried(client: &DevClientSession, definition: std::num::NonZeroU32) -> u64 {
    client.inventory().map_or(0, |inventory| {
        inventory
            .entries
            .iter()
            .filter(|item| item.item_definition_ref == definition)
            .map(|item| u64::from(item.count.get()))
            .sum()
    })
}

async fn trade_receipts(clients: &SeamClients<'_>, npc: &str) -> TestResult<i64> {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let count = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades WHERE character_id=encode($1,'hex')::uuid AND npc_key=$2 AND side=3")
        .bind(clients.characters[1].as_slice()).bind(npc).fetch_one(&mut db).await?;
    db.close().await?;
    Ok(count)
}

async fn exchange(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    seymour: bool,
) -> TestResult<(u32, u64, i64)> {
    let (key, word, input, output) = if seymour {
        ("oteryn:npc.seymour", "box", 2856, 3374)
    } else {
        ("oteryn:npc.al_dee", "pick", 3462, 3456)
    };
    let actor = approach(client, world, npcs, key).await?;
    let index = provisioning::item_index()?;
    let input_ref = provisioning::item_reference(&index, input)?;
    let output_ref = provisioning::item_reference(&index, output)?;
    let before = (
        carried(client, input_ref),
        carried(client, output_ref),
        trade_receipts(clients, key).await?,
    );
    talk(client, actor, "hi").await?;
    talk(client, actor, word).await?;
    talk(client, actor, "yes").await?;
    if before.0 == 0 {
        return Err(format!("needed:player-acquired-exchange-item:{input}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
    while carried(client, input_ref) != before.0 - 1 || carried(client, output_ref) != before.1 + 1
    {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("needed:exchange-inventory-delta:{input}->{output}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    let count = trade_receipts(clients, key).await?;
    if count != before.2 + 1 {
        return Err("needed:one-new-exchange-receipt".into());
    }
    Ok((output, before.1 + 1, count))
}

fn inventory_summary(client: &DevClientSession) -> Vec<(u32, u32, u32)> {
    let mut items: Vec<_> = client
        .inventory()
        .into_iter()
        .flat_map(|inventory| {
            inventory
                .entries
                .iter()
                .chain(inventory.main_backpack.iter())
                .chain(inventory.equipment.iter().map(|entry| &entry.item))
        })
        .map(|entry| {
            (
                entry.item_definition_ref.get(),
                entry.count.get(),
                entry.sub_type,
            )
        })
        .collect();
    items.sort_unstable();
    items
}

async fn forge_receipt(
    clients: &SeamClients<'_>,
    occurrence: [u8; 16],
) -> TestResult<serde_json::Value> {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let reply: Option<serde_json::Value> = sqlx::query_scalar("SELECT reply FROM game_forge_operations WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid AND operation=1 AND applied AND jsonb_array_length(inputs)>0")
        .bind(clients.characters[1].as_slice()).bind(occurrence.as_slice()).fetch_optional(&mut db).await?;
    db.close().await?;
    reply.ok_or_else(|| "needed:applied-forge-receipt-with-input-consumption".into())
}

async fn forge(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<([u8; 16], serde_json::Value)> {
    use oteryn_dev_client::client_runtime::forge_preview as wire;
    use oteryn_protocol_oteryn::item_forge as mutation;
    let request = wire::Request {
        nonce: 1,
        selection: wire::Selection::Browse { after: 0 },
    };
    let payload = request.encode().ok_or("forge-preview-encode")?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_FORGE_PREVIEW_V1,
            wire::COMMAND_TYPE_FORGE_PREVIEW,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("needed:forge-preview-admission;reply:{:?}", reply.status).into());
    }
    let decoded = wire::Reply::decode(&reply.payload).ok_or("forge-preview-decode")?;
    let wire::Payload::Page(page) = decoded.payload else {
        return Err("needed:forge-candidate-page".into());
    };
    let policy = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("migrations/1305_questpc8_forge_profile_admission.sql"),
    )?;
    let index = provisioning::item_index()?;
    let (first, second) = page
        .items
        .iter()
        .enumerate()
        .find_map(|(i, first)| {
            let reference = std::num::NonZeroU32::new(first.definition_ref)?;
            let (key, _) = index.definition_at(reference)?;
            if first.container
                || first.tier >= first.maximum_tier
                || !policy.contains(&format!("'{key}'"))
            {
                return None;
            }
            page.items
                .iter()
                .skip(i + 1)
                .find(|second| {
                    second.definition_ref == first.definition_ref
                        && second.tier == first.tier
                        && !second.container
                })
                .map(|second| (first, second))
        })
        .ok_or("needed:player-acquired-matching-1305-profile-item-pair")?;
    let selection = wire::Selection::Pair {
        operation: mutation::Operation::Fusion,
        first: first.handle,
        second: second.handle,
        boost: false,
        protect: false,
    };
    let payload = wire::Request {
        nonce: 2,
        selection,
    }
    .encode()
    .ok_or("forge-pair-encode")?;
    let quote = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_FORGE_PREVIEW_V1,
            wire::COMMAND_TYPE_FORGE_PREVIEW,
            &payload,
        )
        .await?;
    if quote.status != CommandStatus::Accepted {
        return Err("needed:admitted-forge-pair".into());
    }
    let decoded = wire::Reply::decode(&quote.payload).ok_or("forge-quote-decode")?;
    let wire::Payload::Quote(quote) = decoded.payload else {
        return Err("needed:eligible-forge-pair-and-resources".into());
    };
    if quote.resources.dust < quote.cost.dust
        || quote
            .resources
            .carried_gold
            .saturating_add(if quote.resources.bank_spendable {
                quote.resources.bank_gold
            } else {
                0
            })
            < quote.cost.gold
    {
        return Err("needed:player-earned-forge-dust-and-gold".into());
    }
    let mut occurrence = *client.join_snapshot().game_session_id.as_bytes();
    occurrence[15] ^= 0x40;
    let request = selection
        .canonical(occurrence)
        .ok_or("forge-canonical-pair")?;
    let before = inventory_summary(client);
    let reply = client
        .session_mut()
        .gated_command(
            mutation::CAPABILITY_ITEM_FORGE_V1,
            mutation::COMMAND_TYPE_ITEM_FORGE,
            &request.encode().ok_or("forge-encode")?,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err("needed:accepted-forge-mutation".into());
    }
    let decoded: serde_json::Value = serde_json::from_slice(&reply.payload)?;
    if decoded["applied"] != true {
        return Err(format!("needed:applied-forge-transaction;observed:{decoded}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
    while inventory_summary(client) == before {
        if tokio::time::Instant::now() >= deadline {
            return Err("needed:forge-inventory-delta".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    if forge_receipt(clients, occurrence).await? != decoded {
        return Err("needed:forge-receipt-matches-server-reply".into());
    }
    Ok((occurrence, decoded))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut failures = Vec::new();
    for (name, file, words, suffix) in LEGS {
        let mut client = connect_world(clients, 0xf0).await?;
        let result = tokio::time::timeout(
            Duration::from_secs(90),
            quest(
                &mut client,
                clients,
                world,
                &metadata.npcs,
                file,
                words,
                suffix,
            ),
        )
        .await;
        let ended = logout(&mut client).await;
        drop(client);
        let result: TestResult = async {
            ended?;
            let proof = result.map_err(|_| "needed:quest-leg-completes-within-90s")??;
            let document = document(file)?;
            let rule = rule(&document, suffix)?;
            let mut fresh = connect_world(clients, 0xf1).await?;
            let checked: TestResult = async {
                for (key, after, before) in &proof {
                    if track(clients, key).await? != Some(*after)
                        || (*before != Some(*after)
                            && receipt(clients, rule, key, *before, *after).await? == 0)
                    {
                        return Err(format!("needed:relog-state-and-receipt:{key}").into());
                    }
                }
                Ok(())
            }
            .await;
            logout(&mut fresh).await?;
            checked
        }
        .await;
        record(name, result, &mut failures);
    }
    for seymour in [false, true] {
        let name = if seymour { "seymour" } else { "al-dee" };
        let key = if seymour {
            "oteryn:npc.seymour"
        } else {
            "oteryn:npc.al_dee"
        };
        let mut client = connect_world(clients, 0xf2).await?;
        let result = tokio::time::timeout(
            Duration::from_secs(90),
            exchange(&mut client, clients, world, &metadata.npcs, seymour),
        )
        .await;
        let ended = logout(&mut client).await;
        drop(client);
        let result: TestResult = async {
            ended?;
            let (output, count, receipts) =
                result.map_err(|_| "needed:exchange-leg-completes-within-90s")??;
            let mut fresh = connect_world(clients, 0xf3).await?;
            let checked: TestResult = async {
                let reference = provisioning::item_reference(&provisioning::item_index()?, output)?;
                if carried(&fresh, reference) != count
                    || trade_receipts(clients, key).await? != receipts
                {
                    return Err("needed:relog-exchange-state-and-receipt".into());
                }
                Ok(())
            }
            .await;
            logout(&mut fresh).await?;
            checked
        }
        .await;
        record(name, result, &mut failures);
    }
    let mut client = connect_world(clients, 0xf4).await?;
    let result = forge(&mut client, clients).await;
    let state = inventory_summary(&client);
    let ended = logout(&mut client).await;
    drop(client);
    let result: TestResult = async {
        ended?;
        let (occurrence, reply) = result?;
        let mut fresh = connect_world(clients, 0xf7).await?;
        let checked: TestResult = async {
            if inventory_summary(&fresh) != state
                || forge_receipt(clients, occurrence).await? != reply
            {
                return Err("needed:relog-forge-state-and-receipt".into());
            }
            Ok(())
        }
        .await;
        logout(&mut fresh).await?;
        checked
    }
    .await;
    record("forge-profile", result, &mut failures);
    if failures.is_empty() {
        Ok("all-ten-legs-state-receipt-relog".into())
    } else {
        Err(failures.join(";").into())
    }
}

fn record(name: &str, result: TestResult, failures: &mut Vec<String>) {
    match result {
        Ok(()) => evidence(&format!(
            "G40_CHECK leg={name} status=PASS evidence=state-receipt-relog"
        )),
        Err(error) => {
            let detail = token(&error.to_string());
            evidence(&format!(
                "G40_CHECK leg={name} status=FAIL evidence={detail}"
            ));
            failures.push(format!("{name}:{detail}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracts_only_durable_set_effects() -> TestResult {
        assert_eq!(
            writes(
                &serde_json::json!({"effects":[{"kind":"set_topic","value":2},{"kind":"set","track":"x","value":1}]})
            )?,
            vec![("x".into(), 1)]
        );
        assert!(writes(&serde_json::json!({"effects":[]})).is_err());
        assert_eq!(token("needed:a b\nc"), "needed:a-b-c");
        Ok(())
    }
    #[test]
    fn authored_first_step_rules_exist() -> TestResult {
        for (_, file, _, suffix) in LEGS {
            writes(rule(&document(file)?, suffix)?)?;
        }
        Ok(())
    }
}
