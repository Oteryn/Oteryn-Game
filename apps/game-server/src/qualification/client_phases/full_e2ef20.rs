//! E2EF20/E02: native NPC dialogue coin exchange and durable backpack custody.
use super::*;
use super::client_row_e02::{coin_profile, find_exchange_npc};
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef};
use oteryn_dev_client::client_runtime::npc_shop as runtime;
use oteryn_protocol_oteryn::npc_service as wire;
use std::collections::BTreeMap;

const GOLD_ID: u32 = 3031;
const PLATINUM_ID: u32 = 3035;
const CRYSTAL_ID: u32 = 3043;

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

    fn finish(self, result: TestResult) {
        if let Err(error) = &result {
            evidence(&format!("E2EF20_GAP E02 step=journey observed={error}"));
        }
        for (step, passed) in &self.steps {
            if !passed {
                evidence(&format!("E2EF20_GAP E02 step={step} observed=not-reached"));
            }
        }
        evidence(&format!(
            "e2e_all scenario=E02.full status={} seconds={:.3}",
            if result.is_ok() && self.steps.values().all(|passed| *passed) {
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
    let result = tokio::time::timeout(Duration::from_secs(180), journey(&mut row, clients)).await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err("step=deadline observed=180s-timeout".into()),
    });
}

fn coin_refs() -> TestResult<[std::num::NonZeroU32; 3]> {
    let index = provisioning::item_index()?;
    Ok([
        provisioning::item_reference(&index, GOLD_ID)?,
        provisioning::item_reference(&index, PLATINUM_ID)?,
        provisioning::item_reference(&index, CRYSTAL_ID)?,
    ])
}

fn backpack_coins(client: &DevClientSession, refs: [std::num::NonZeroU32; 3]) -> TestResult<[u64; 3]> {
    let inventory = client.inventory().ok_or("step=inventory observed=snapshot-absent")?;
    let mut counts = [0; 3];
    for entry in &inventory.entries {
        if let Some(index) = refs.iter().position(|reference| *reference == entry.item_definition_ref) {
            counts[index] += u64::from(entry.count.get());
        }
    }
    Ok(counts)
}

async fn wait_coins(
    client: &mut DevClientSession,
    refs: [std::num::NonZeroU32; 3],
    expected: [u64; 3],
    step: &'static str,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        let observed = backpack_coins(client, refs)?;
        if observed == expected {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step={step} observed=coin-delta-timeout:{observed:?} expected={expected:?}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

async fn talk(client: &mut DevClientSession, actor: EntityRef, text: &str) -> TestResult<wire::NpcConversation> {
    tokio::time::sleep(Duration::from_millis(260)).await;
    let disposition = runtime::talk(
        client.session_mut(),
        &wire::NpcTalkIntent { npc_actor: actor, text: text.into() },
    )
    .await?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("step=talk-{text} observed=disposition:{disposition:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    wire::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("step=talk-{text} observed=malformed-conversation:{error:?}").into())
}

pub(super) async fn qualification_npc_talk_book(
    client: &mut DevClientSession,
) -> TestResult<crate::gameplay_transport::NpcTalkBook> {
    route_work(client, || {
        let staged = std::path::PathBuf::from(required("NODE_BOOT_NPC_DATA").map_err(|error| error.to_string())?);
        let digest = std::fs::read_to_string(staged.join("PROJECT_SHA256")).map_err(|error| error.to_string())?;
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

async fn exchange(
    client: &mut DevClientSession,
    actor: EntityRef,
    words: &[&str],
) -> TestResult<wire::NpcConversation> {
    let mut last = None;
    for word in words {
        last = Some(talk(client, actor, word).await?);
    }
    last.ok_or_else(|| "step=exchange observed=empty-command-sequence".into())
}

fn exact_delta(before: [u64; 3], after: [u64; 3], delta: [i64; 3]) -> bool {
    before.into_iter().zip(after).zip(delta).all(|((before, after), delta)| {
        i128::from(after) - i128::from(before) == i128::from(delta)
    })
}

async fn refused_gold_exchange(
    client: &mut DevClientSession,
    actor: EntityRef,
    amount: &str,
    database_url: &str,
) -> TestResult<wire::NpcConversation> {
    exchange(client, actor, &["change gold", amount]).await?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let command = client.next_command_id().to_string();
    let reply = talk(client, actor, "yes").await?;
    let mut database = sqlx::PgConnection::connect(database_url).await?;
    let result: i16 = sqlx::query_scalar(
        "SELECT result FROM game_coin_exchange_receipts
         WHERE game_session_id=$1::uuid AND command_id=$2::numeric",
    )
    .bind(session)
    .bind(command)
    .fetch_one(&mut database)
    .await?;
    // The durable exchange result encodes InsufficientCoins as 3.
    if result != 3 || reply.lines != ["Sorry, you do not have enough gold coins."] {
        return Err(format!("step=insufficient-coins observed=code:{result},reply:{:?}", reply.lines).into());
    }
    Ok(reply)
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let refs = coin_refs()?;
    let (characters, mut client) =
        coin_profile(clients, "E02.full", "Coin Exchange Journey", 0xb0, refs).await?;
    let dedicated = SeamClients { characters: &characters, ..*clients };
    let talk_book = qualification_npc_talk_book(&mut client).await?;
    let initial = backpack_coins(&client, refs)?;
    row.pass("legitimate-gold", initial);

    let (actor, npc_key) = find_exchange_npc(&mut client, world, &metadata.npcs, &talk_book).await?;
    row.pass("walk-and-greet", npc_key);

    let reply = exchange(&mut client, actor, &["1", "yes"]).await?;
    let platinum = [initial[0] - 100, initial[1] + 1, initial[2]];
    wait_coins(&mut client, refs, platinum, "gold-to-platinum").await?;
    if !exact_delta(initial, platinum, [-100, 1, 0]) || !reply.lines.iter().any(|line| line == "Here you are.") {
        return Err(format!("step=gold-to-platinum observed=reply:{:?},coins:{platinum:?}", reply.lines).into());
    }
    row.pass("gold-to-platinum", (initial, platinum, reply.lines));

    let reply = exchange(&mut client, actor, &["change platinum", "gold", "1", "yes"]).await?;
    wait_coins(&mut client, refs, initial, "platinum-to-gold").await?;
    if !exact_delta(platinum, initial, [100, -1, 0]) || !reply.lines.iter().any(|line| line == "Here you are.") {
        return Err(format!("step=platinum-to-gold observed=reply:{:?},coins:{:?}", reply.lines, backpack_coins(&client, refs)?).into());
    }
    row.pass("platinum-to-gold", (platinum, initial, reply.lines));

    let bank_before = super::full_row_e06::bank_balance(&dedicated, 1).await?;
    let deposited = bank_before.checked_add(100).ok_or("E02 bank balance overflow")?;
    let reply = refused_gold_exchange(&mut client, actor, "2", clients.url).await?;
    let after_refusal = backpack_coins(&client, refs)?;
    if after_refusal != initial || super::full_row_e06::bank_balance(&dedicated, 1).await? != bank_before {
        return Err(format!("step=insufficient-refusal observed=reply:{:?},coins:{after_refusal:?}", reply.lines).into());
    }
    row.pass("insufficient-refusal", (reply.lines, after_refusal));

    let reply = exchange(&mut client, actor, &["deposit all", "yes"]).await?;
    wait_coins(&mut client, refs, [0, 0, 0], "deposit-all").await?;
    if super::full_row_e06::bank_balance(&dedicated, 1).await? != deposited {
        return Err("step=deposit-all observed=incorrect-bank-credit".into());
    }
    let refusal = refused_gold_exchange(&mut client, actor, "1", clients.url).await?;
    if backpack_coins(&client, refs)? != [0, 0, 0] || super::full_row_e06::bank_balance(&dedicated, 1).await? != deposited {
        return Err(format!("step=bank-only-refusal observed=deposit:{:?},exchange:{:?},coins:{:?}", reply.lines, refusal.lines, backpack_coins(&client, refs)?).into());
    }
    row.pass("bank-only-refusal", (reply.lines, refusal.lines));

    let withdrawal = exchange(&mut client, actor, &["withdraw 100 gold", "yes"]).await?;
    wait_coins(&mut client, refs, platinum, "withdraw-after-refusal").await?;
    if super::full_row_e06::bank_balance(&dedicated, 1).await? != bank_before {
        return Err("step=withdraw-after-refusal observed=incorrect-bank-debit".into());
    }
    let restored = exchange(&mut client, actor, &["change platinum", "gold", "1", "yes"]).await?;
    wait_coins(&mut client, refs, initial, "restore-gold-before-relogin").await?;
    if !restored.lines.iter().any(|line| line == "Here you are.") {
        return Err(format!("step=restore-gold-before-relogin observed={:?}", restored.lines).into());
    }
    let final_conversion = exchange(&mut client, actor, &["change gold", "1", "yes"]).await?;
    wait_coins(&mut client, refs, platinum, "convert-before-relogin").await?;
    if final_conversion.lines != ["Here you are."] {
        return Err(format!("step=convert-before-relogin observed={:?}", final_conversion.lines).into());
    }
    let old_session = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("step=logout observed=refused".into());
    }
    drop(client);
    let mut fresh = connect_world(&dedicated, 0xb1).await?;
    let persisted = backpack_coins(&fresh, refs)?;
    if fresh.join_snapshot().game_session_id == old_session || persisted != platinum
        || super::full_row_e06::bank_balance(&dedicated, 1).await? != bank_before {
        return Err(format!("step=fresh-admission-persistence observed=session-reused:{},coins:{persisted:?}", fresh.join_snapshot().game_session_id == old_session).into());
    }
    row.pass("fresh-admission-persistence", (withdrawal.lines, persisted));
    if fresh.session_mut().logout().await?.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("step=final-logout observed=refused".into());
    }
    Ok(())
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
