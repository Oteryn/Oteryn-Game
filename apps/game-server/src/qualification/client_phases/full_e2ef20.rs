//! E2EF20/E02: native NPC dialogue coin exchange and durable backpack custody.
use super::client_row_e02::{
    COMPLETE, ExchangeProof, GOLD_PROMPT, PLATINUM_DESTINATION, coin_refs, find_exchange_npc,
    gold_quote, greet, seed_coin_profile,
};
use super::*;
use oteryn_dev_client::client_runtime::npc_shop as runtime;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef};
use oteryn_protocol_oteryn::npc_service as wire;
use std::collections::BTreeMap;

struct Row {
    started: std::time::Instant,
    steps: BTreeMap<&'static str, bool>,
}

impl Row {
    fn new() -> Self {
        Self {
            started: std::time::Instant::now(),
            steps: [
                "legitimate-gold",
                "walk-and-greet",
                "gold-to-platinum",
                "platinum-to-gold",
                "crystal-to-platinum",
                "platinum-to-crystal",
                "insufficient-refusal",
                "bank-only-refusal",
                "fresh-admission-persistence",
            ]
            .into_iter()
            .map(|step| (step, false))
            .collect(),
        }
    }

    fn pass(&mut self, step: &'static str, observed: impl std::fmt::Debug) {
        if let Some(done) = self.steps.get_mut(step) {
            *done = true;
            evidence(&format!(
                "E2EF20_CHECK E02 {step} PASS observed={observed:?}"
            ));
        }
    }

    fn finish(self, result: TestResult<String>) {
        if let Err(error) = &result {
            evidence(&format!("E2EF20_GAP E02 step=journey observed={error}"));
        }
        for (step, passed) in &self.steps {
            if !passed {
                evidence(&format!("E2EF20_GAP E02 step={step} observed=not-reached"));
            }
        }
        match result {
            Ok(binding) if self.steps.values().all(|passed| *passed) => evidence(&format!(
                "e2e_all scenario=E02.full status=CAPTURED seconds={:.3} {binding}",
                self.started.elapsed().as_secs_f64()
            )),
            _ => evidence(&format!(
                "e2e_all scenario=E02.full status=FAIL seconds={:.3}",
                self.started.elapsed().as_secs_f64()
            )),
        }
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = tokio::time::timeout(Duration::from_secs(180), journey(&mut row, clients)).await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err("step=deadline observed=180s-timeout".into()),
    });
}

async fn talk(
    client: &mut DevClientSession,
    actor: EntityRef,
    text: &str,
) -> TestResult<wire::NpcConversation> {
    tokio::time::sleep(Duration::from_millis(260)).await;
    let disposition = runtime::talk(
        client.session_mut(),
        &wire::NpcTalkIntent {
            npc_actor: actor,
            text: text.into(),
        },
    )
    .await?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("step=talk-{text} observed=disposition:{disposition:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    wire::decode_npc_conversation(&delta.payload).map_err(|error| {
        format!("step=talk-{text} observed=malformed-conversation:{error:?}").into()
    })
}

pub(super) async fn qualification_npc_talk_book(
    client: &mut DevClientSession,
) -> TestResult<crate::gameplay_transport::NpcTalkBook> {
    route_work(client, || {
        let staged = std::path::PathBuf::from(
            required("NODE_BOOT_NPC_DATA").map_err(|error| error.to_string())?,
        );
        let digest = std::fs::read_to_string(staged.join("PROJECT_SHA256"))
            .map_err(|error| error.to_string())?;
        let catalogue = crate::content::load_data_only_npc_catalogue(
            &staged.join("content/world"),
            digest.trim(),
        )
        .map_err(|error| error.to_string())?;
        let model = catalogue
            .service_model_for_project()
            .map_err(|error| error.to_string())?;
        Ok(crate::gameplay_transport::NpcTalkBook::from_model(&model))
    })
    .await
}

#[cfg(test)]
fn exact_delta(before: [u64; 3], after: [u64; 3], delta: [i64; 3]) -> bool {
    before
        .into_iter()
        .zip(after)
        .zip(delta)
        .all(|((before, after), delta)| i128::from(after) - i128::from(before) == i128::from(delta))
}

async fn refused_gold_exchange(
    proof: &mut ExchangeProof,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    actor: EntityRef,
    amount: u64,
    words: &str,
) -> TestResult {
    proof
        .prompts(
            client,
            clients,
            actor,
            &[("change gold", GOLD_PROMPT), (words, &gold_quote(amount))],
        )
        .await?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let command = client.next_command_id().to_string();
    proof
        .confirm(
            client,
            clients,
            actor,
            "Sorry, you do not have enough gold coins.",
            proof.coins,
            proof.bank,
        )
        .await?;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let result: i16 = sqlx::query_scalar(
        "SELECT result FROM game_coin_exchange_receipts
         WHERE game_session_id=$1::uuid AND command_id=$2::numeric",
    )
    .bind(session)
    .bind(command)
    .fetch_one(&mut database)
    .await?;
    if result != 3 {
        return Err(format!("step=insufficient-coins observed=code:{result}").into());
    }
    Ok(())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult<String> {
    use oteryn_dev_client::native_session::test_support::capture_server_frames;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let refs = coin_refs()?;
    let characters =
        seed_coin_profile(clients, "E02.full", "Coin Exchange Journey", 0xb0, refs).await?;
    let dedicated = SeamClients {
        characters: &characters,
        ..*clients
    };
    let (result, frames) = capture_server_frames(async {
        let mut client = connect_world(&dedicated, 0xb0).await?;
        let initial = [100, 0, 1];
        let platinum = [0, 1, 1];
        let mut proof = ExchangeProof::start(&client, &dedicated, refs, initial).await?;
        let bank_before = proof.bank;
        row.pass("legitimate-gold", initial);
        let talk_book = qualification_npc_talk_book(&mut client).await?;
        let (actor, npc_key) = find_exchange_npc(&mut client, world, &metadata.npcs, &talk_book).await?;
        greet(&mut proof, &mut client, &dedicated, actor, "Coin Exchange Journey").await?;
        row.pass("walk-and-greet", npc_key);

        let gold_to_platinum = [("change gold", GOLD_PROMPT), ("1 platinum", "So you would like me to change 100 of your gold coins into 1 platinum coins?")];
        let platinum_to_gold = [
            ("change platinum", PLATINUM_DESTINATION),
            ("gold", "How many platinum coins would you like to change into gold?"),
            ("1", "So you would like me to change 1 of your platinum coins into 100 gold coins for you?"),
        ];
        proof.prompts(&mut client, &dedicated, actor, &gold_to_platinum).await?;
        proof.confirm(&mut client, &dedicated, actor, COMPLETE, platinum, bank_before).await?;
        row.pass("gold-to-platinum", platinum);

        proof.prompts(&mut client, &dedicated, actor, &platinum_to_gold).await?;
        proof.confirm(&mut client, &dedicated, actor, COMPLETE, initial, bank_before).await?;
        row.pass("platinum-to-gold", initial);

        proof.prompts(&mut client, &dedicated, actor, &[
            ("change crystal", "How many crystal coins would you like to change into platinum?"),
            ("1", "So you would like me to change 1 of your crystal coins into 100 platinum coins for you?"),
        ]).await?;
        proof.confirm(&mut client, &dedicated, actor, COMPLETE, [100, 100, 0], bank_before).await?;
        row.pass("crystal-to-platinum", proof.coins);

        proof.prompts(&mut client, &dedicated, actor, &[
            ("change platinum", PLATINUM_DESTINATION),
            ("crystal", "How many crystal coins would you like to get?"),
            ("1", "So you would like me to change 100 of your platinum coins into 1 crystal coins for you?"),
        ]).await?;
        proof.confirm(&mut client, &dedicated, actor, COMPLETE, initial, bank_before).await?;
        row.pass("platinum-to-crystal", initial);

        refused_gold_exchange(&mut proof, &mut client, &dedicated, actor, 2, "2 platinum").await?;
        row.pass("insufficient-refusal", proof.coins);

        let deposited = bank_before.checked_add(10_100).ok_or("E02 bank balance overflow")?;
        proof.prompts(&mut client, &dedicated, actor, &[("deposit all", "Do you want to deposit 10100 gold?")]).await?;
        proof.confirm(&mut client, &dedicated, actor, "You have deposited 10100 gold.", [0, 0, 0], deposited).await?;
        refused_gold_exchange(&mut proof, &mut client, &dedicated, actor, 1, "1 platinum").await?;
        row.pass("bank-only-refusal", (proof.coins, proof.bank));

        proof.prompts(&mut client, &dedicated, actor, &[("withdraw 100 gold", "Do you want to withdraw 100 gold?")]).await?;
        proof.confirm(&mut client, &dedicated, actor, "You have withdrawn 100 gold.", [0, 1, 0], deposited - 100).await?;
        proof.prompts(&mut client, &dedicated, actor, &[("withdraw 10000 gold", "Do you want to withdraw 10000 gold?")]).await?;
        proof.confirm(&mut client, &dedicated, actor, "You have withdrawn 10000 gold.", platinum, bank_before).await?;
        proof.prompts(&mut client, &dedicated, actor, &platinum_to_gold).await?;
        proof.confirm(&mut client, &dedicated, actor, COMPLETE, initial, bank_before).await?;
        proof.prompts(&mut client, &dedicated, actor, &gold_to_platinum).await?;
        proof.confirm(&mut client, &dedicated, actor, COMPLETE, platinum, bank_before).await?;
        TestResult::Ok((client, proof))
    }).await;
    let (mut client, proof) = result?;
    let old_session = client.join_snapshot().game_session_id;
    let sequence = client.last_server_sequence();
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    drop(client);
    let mut fresh = connect_world(&dedicated, 0xb1).await?;
    proof.check(&fresh, &dedicated, true).await?;
    if fresh.join_snapshot().game_session_id == old_session {
        return Err("step=fresh-admission-persistence observed=session-reused".into());
    }
    row.pass("fresh-admission-persistence", (proof.coins, proof.bank));
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=final-logout observed=refused".into());
    }
    proof.write_capture("full", frames, uuid_text(old_session.as_bytes()), sequence)
}

pub(super) async fn greet_ten(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) {
    let mut row = verdict::Row::new("NPC", "NPC-GREET-10", &["greet-ten", "logout"]);
    let result = row
        .watch()
        .bound(
            Duration::from_secs(180),
            "NPC-GREET-10 deadline",
            greet_ten_journey(&mut row, clients, world, npcs),
        )
        .await;
    row.finish(&result);
}

async fn greet_ten_journey(
    row: &mut verdict::Row,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    mod hunt_route {
        use super::*;
        include!("qualification_hunt_route.rs");
    }
    character_prep::declare_seeded(
        "NPC-GREET-10",
        "dedicated-character-birth",
        "isolate dialogue state",
    );
    let mut characters = clients.characters.to_vec();
    characters[1] =
        hunt_route::qualification_character(clients, 1, 0xb9, "Town Greeter", "Knight").await?;
    let clients = SeamClients {
        characters: &characters,
        ..*clients
    };
    let mut client = connect_world(&clients, 0xb9).await?;
    let result: TestResult = async {
        let talk_book = qualification_npc_talk_book(&mut client).await?;
        // The placement table also contains silent actors. Choose greetings
        // from the authored service book and verify each one over the wire.
        let mut remaining: BTreeMap<TilePos, &str> = npcs
            .npcs
            .iter()
            .filter(|npc| talk_book.get(&npc.key).is_some())
            .flat_map(|npc| {
                npc.placements.iter().map(|placement| {
                    (
                        TilePos {
                            x: placement.x,
                            y: placement.y,
                            floor: placement.floor,
                        },
                        npc.key.as_str(),
                    )
                })
            })
            .collect();
        let mut greeted = Vec::new();
        for _ in 0..10 {
            let start = tile(client.world_spatial().actor_position)?;
            let candidates: Vec<_> = remaining.keys().copied().collect();
            let snapshot = world.clone();
            let (path, target) = route_work(&mut client, move || {
                let mut approaches = BTreeMap::new();
                for target in candidates {
                    for (dx, dy) in [
                        (-1, -1),
                        (-1, 0),
                        (-1, 1),
                        (0, -1),
                        (0, 1),
                        (1, -1),
                        (1, 0),
                        (1, 1),
                    ] {
                        if let (Some(x), Some(y)) = (
                            target.x.checked_add_signed(dx),
                            target.y.checked_add_signed(dy),
                        ) {
                            approaches
                                .entry(TilePos {
                                    x,
                                    y,
                                    floor: target.floor,
                                })
                                .or_insert(target);
                        }
                    }
                }
                let path = route(&snapshot, start, |at| approaches.contains_key(&at))
                    .ok_or("no route to another ungreeted NPC placement")?;
                let end = path
                    .iter()
                    .try_fold(start, |at, &direction| {
                        destination(&snapshot, at, direction)
                    })
                    .ok_or("NPC route endpoint disappeared")?;
                let target = *approaches
                    .get(&end)
                    .ok_or("NPC route ended without an adjacent placement")?;
                Ok((path, target))
            })
            .await?;
            walk(&mut client, world, &path).await?;
            let key = remaining
                .remove(&target)
                .ok_or("NPC selection repeated a placement")?;
            let actor = client
                .world_entities()
                .and_then(|view| {
                    view.iter().find(|entity| {
                        entity.kind == EntityKind::Npc && tile(entity.position).ok() == Some(target)
                    })
                })
                .ok_or_else(|| format!("served NPC missing at {target:?}: {key}"))?
                .entity;
            if greeted.contains(&actor) {
                return Err(format!("NPC actor already greeted: {key}").into());
            }
            let hello = talk(&mut client, actor, "hi")
                .await
                .map_err(|error| format!("NPC greeting at {target:?} for {key}: {error}"))?;
            if hello.open.map(|(npc, _)| npc) != Some(actor) || hello.lines.is_empty() {
                return Err(
                    format!("NPC greeting did not open a dialogue with {key}: {hello:?}").into(),
                );
            }
            greeted.push(actor);
            evidence(&format!(
                "NPC_GREET row=NPC-GREET-10 npc={key} placement={target:?} count={}",
                greeted.len()
            ));
            let goodbye = talk(&mut client, actor, "bye").await?;
            if goodbye.open.is_some() {
                return Err(format!("NPC farewell left the conversation open with {key}").into());
            }
        }
        row.pass(
            "greet-ten",
            format_args!("distinct_actors={}", greeted.len()),
        );
        Ok(())
    }
    .await;
    let release = logout_retrying_busy(&mut client)
        .await
        .and_then(|(status, diagnostic, _, _)| {
            if status != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
                return Err(format!("Town Greeter logout refused: {diagnostic}").into());
            }
            row.pass("logout", "accepted");
            Ok(())
        });
    result.and(release)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_coin_delta_rejects_value_preserving_wrong_denominations() {
        assert!(exact_delta([100, 0, 0], [0, 1, 0], [-100, 1, 0]));
        assert!(!exact_delta([100, 0, 0], [0, 0, 1], [-100, 1, 0]));
        assert!(exact_delta([0, 1, 0], [100, 0, 0], [100, -1, 0]));
    }
}
