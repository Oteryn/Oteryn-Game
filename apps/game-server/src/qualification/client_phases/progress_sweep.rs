//! Opt-in content census and real-wire progression journeys.
//! Select with NODE_BOOT_E2E_ONLY=progress_sweep and OTERYN_E2E_PROGRESS_SWEEP_BATCH=start:len.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::logout::LogoutResult;
use serde_json::Value;
use std::num::NonZeroU32;

const BATCH: &str = "OTERYN_E2E_PROGRESS_SWEEP_BATCH";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    Achievement,
    Charm,
    Craft,
    Proficiency,
}

impl Family {
    fn steps(self) -> &'static [&'static str] {
        match self {
            Self::Achievement => &["trigger", "grant", "once"],
            Self::Charm => &["unlock", "assign", "effect"],
            Self::Craft => &["use", "inputs", "output"],
            Self::Proficiency => &["progress", "perks", "effect"],
        }
    }
}

struct Entry {
    family: Family,
    key: String,
    content: Value,
    source: String,
}

fn batch(value: Option<&str>) -> TestResult<(usize, usize)> {
    let Some(value) = value else {
        return Ok((0, 3));
    };
    let parse = |part: &str| {
        if part.bytes().all(|byte| byte.is_ascii_digit()) {
            part.parse::<usize>().ok()
        } else {
            None
        }
    };
    value
        .split_once(':')
        .and_then(|(start, len)| Some((parse(start)?, parse(len)?)))
        .filter(|(_, len)| *len > 0)
        .ok_or_else(|| format!("invalid {BATCH}={value:?}: expected start:len with len > 0").into())
}

fn family_batch(setting: Option<&str>) -> TestResult<(Option<Family>, (usize, usize))> {
    let (family, range) = match setting.and_then(|value| value.split_once(':')) {
        Some(("achievements", range)) => (Some(Family::Achievement), Some(range)),
        Some(("charms", range)) => (Some(Family::Charm), Some(range)),
        Some(("craft", range)) => (Some(Family::Craft), Some(range)),
        Some(("proficiencies", range)) => (Some(Family::Proficiency), Some(range)),
        _ => (None, setting),
    };
    Ok((family, batch(range)?))
}

fn document(path: &Path) -> TestResult<Value> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

fn census(root: &Path) -> TestResult<Vec<Entry>> {
    let mut entries = Vec::new();
    for (family, directory, prefix) in [
        (Family::Achievement, "achievements", "achievements-"),
        (Family::Charm, "charms", "charms-"),
        (Family::Craft, "craft", "recipes.json"),
        (Family::Proficiency, "proficiencies", "proficiencies-"),
    ] {
        let mut shards = std::fs::read_dir(root.join(directory))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?;
        shards.sort();
        let mut ordinal = 0;
        for shard in shards.iter().filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".json"))
        }) {
            let bytes = std::fs::read(shard)?;
            let source: Value = serde_json::from_slice(&bytes)?;
            for record in list(&source["records"])? {
                let content = record.get("definition").unwrap_or(record);
                let key = string(&content["identity"]["key"])?;
                entries.push(Entry {
                    family,
                    key: key.to_owned(),
                    content: content.clone(),
                    source: format!(
                        "content/{}:{}",
                        shard.strip_prefix(root)?.display(),
                        bytes
                            .split(|byte| *byte == b'\n')
                            .position(|line| line
                                .windows(key.len())
                                .any(|part| part == key.as_bytes()))
                            .ok_or("content key has no source line")?
                            + 1
                    ),
                });
                ordinal += 1;
            }
        }
        if ordinal == 0 {
            return Err(format!("empty progress sweep family {directory}").into());
        }
        evidence(&format!(
            "progress_sweep census family={directory} entries={ordinal}"
        ));
    }
    entries.sort_unstable_by(|a, b| a.key.cmp(&b.key));
    if entries.windows(2).any(|pair| pair[0].key == pair[1].key) {
        return Err("duplicate progress sweep key".into());
    }
    Ok(entries)
}

fn string(value: &Value) -> TestResult<&str> {
    value
        .as_str()
        .ok_or_else(|| "missing content string".into())
}

fn number(value: &Value) -> TestResult<u32> {
    Ok(u32::try_from(
        value.as_u64().ok_or("missing content integer")?,
    )?)
}

fn list(value: &Value) -> TestResult<&[Value]> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| "missing content array".into())
}

fn token(value: &str) -> String {
    value
        .chars()
        .take(300)
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._:/-".contains(c) {
                c
            } else {
                '-'
            }
        })
        .collect()
}

struct Rows<'a> {
    entry: &'a Entry,
    passed: usize,
    started: std::time::Instant,
}

impl Rows<'_> {
    fn emit(&self, step: &str, status: &str, detail: &str) {
        evidence(&format!(
            "e2e_all scenario=PS.{}.{step}.full status={status} seconds={:.3} mode=seeded evidence={}",
            self.entry
                .key
                .strip_prefix("oteryn:")
                .unwrap_or(&self.entry.key)
                .replace('/', "."),
            self.started.elapsed().as_secs_f64(),
            token(detail),
        ));
    }

    fn pass(&mut self, detail: &str) {
        self.emit(self.entry.family.steps()[self.passed], "PASS", detail);
        self.passed += 1;
    }
}

pub(super) async fn run(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult {
    let setting = match std::env::var(BATCH) {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => return Err(error.into()),
    };
    let (family, (start, len)) = family_batch(setting.as_deref())?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let mut entries = census(&root)?;
    entries.retain(|entry| family.is_none_or(|family| entry.family == family));
    let end = start.saturating_add(len).min(entries.len());
    if start >= end {
        return Err(format!("{BATCH} start {start} outside {} entries", entries.len()).into());
    }
    evidence(&format!(
        "progress_sweep batch total={} start={start} len={len} selected={}",
        entries.len(),
        end - start
    ));

    // Keep the selected slice intact. Exercise major stage-one rows before
    // minor setup, so a prerequisite purchase can't pre-unlock a tested row.
    entries[start..end].sort_by_key(|entry| {
        entry.family == Family::Charm && entry.content["cost_currency"] == "minor_charm_echoes"
    });
    let mut pass = 0;
    let mut fail = 0;
    let mut session = None;
    let mut unsupported_mastery = std::collections::BTreeSet::new();
    for entry in &entries[start..end] {
        verdict::Harness::global().progress("progress-sweep-entry");
        let mut rows = Rows {
            entry,
            passed: 0,
            started: std::time::Instant::now(),
        };
        let outcome = verdict::indexed_timeout(
            Duration::from_secs(150),
            Box::pin(async {
                if session.is_none() {
                    session = Some(connect_character(clients, 0x70, 1).await?);
                }
                let client = session.as_mut().ok_or("fixture admission missing")?;
                match entry.family {
                    Family::Achievement => achievement(client, world, &mut rows).await,
                    Family::Charm => {
                        charm(client, world, &mut rows, &mut unsupported_mastery).await
                    }
                    Family::Craft => craft(client, clients, world, &mut rows).await,
                    Family::Proficiency => proficiency(client, world, &root, &mut rows).await,
                }
            }),
        )
        .await;
        let reason = match outcome {
            Ok(Ok(())) => "missing-step-evidence".to_owned(),
            Ok(Err(error)) => {
                // A failed exchange can poison the connection. Preserve this
                // row's error and let the next entry acquire a clean admission.
                if let Some(client) = session.as_mut()
                    && client
                        .session_mut()
                        .service_liveness_discarding_events(Duration::ZERO)
                        .await
                        .is_err()
                {
                    drop(session.take());
                }
                error.to_string()
            }
            Err(_) => {
                drop(session.take());
                "entry-deadline".to_owned()
            }
        };
        let remaining = &entry.family.steps()[rows.passed..];
        for (index, step) in remaining.iter().enumerate() {
            let blocked = format!("blocked-by-{}", remaining[0]);
            rows.emit(step, "FAIL", if index == 0 { &reason } else { &blocked });
        }
        pass += rows.passed;
        fail += remaining.len();
    }
    let mut release_failed = false;
    if let Some(mut client) = session {
        match logout_retrying_busy(&mut client).await {
            Ok((Some(LogoutResult::Accepted), _, _, _)) => {}
            _ => {
                release_failed = true;
                evidence("progress_sweep release_failed");
            }
        }
    }
    evidence(&format!(
        "progress_sweep summary total={} entries={} rows_pass={pass} rows_fail={fail} mode=seeded",
        entries.len(),
        end - start
    ));
    if fail != 0 || release_failed {
        return Err(
            format!("progress sweep: {fail} FAIL rows, release_failed={release_failed}").into(),
        );
    }
    Ok(())
}

// Bound authored reward chests provide a real trigger when the selected world serves one.
// crates/gameplay/dispatch/src/gameplay_transport/bundle_chest.rs:332
async fn achievement(
    client: &mut DevClientSession,
    world: &BundleWorld,
    rows: &mut Rows<'_>,
) -> TestResult {
    let trigger = achievement_trigger(client, world, rows.entry)
        .await
        .map_err(|error| format!("{error}-{}", rows.entry.source))?;
    let points = number(&rows.entry.content["points"])?;
    let (before, initial) = achievement_facts(client, &rows.entry.key).await?;
    if !initial.is_empty() {
        return Err("harness-achievement-already-earned".into());
    }
    trigger
        .fire(client, world, false)
        .await
        .map_err(|error| format!("{error}-{}", rows.entry.source))?;
    rows.pass(trigger.label());
    let (after, facts) = achievement_facts(client, &rows.entry.key).await?;
    // A kill can grant another achievement in the same transaction. The target fact
    // still must appear exactly once and carry the authored point value.
    let grant_matches = if matches!(trigger, AchievementTrigger::Kill(..)) {
        facts == [points] && after.0 > before.0 && after.1 >= before.1.saturating_add(points)
    } else {
        granted_once(before, after, points, &facts)
    };
    if !grant_matches {
        return Err(format!(
            "server-achievement-grant-mismatch-expected-once-{points}-actual-{facts:?}-owner-{}-Crystal-201189f8-{}",
            trigger.owner(), trigger.donor()
        )
        .into());
    }
    rows.pass(&format!("points-{points}"));
    trigger.fire(client, world, true).await?;
    if achievement_facts(client, &rows.entry.key).await?.1 != facts {
        return Err(format!(
            "server-repeated-trigger-changed-achievement-facts-owner-{}",
            trigger.owner()
        )
        .into());
    }
    rows.pass("repeated-trigger-preserved-single-grant");
    Ok(())
}

enum AchievementTrigger {
    Kill(&'static str, u32),
    ConsumeItem {
        item: u32,
        donor: &'static str,
    },
    NpcTalk {
        npc: &'static str,
        keyword: &'static str,
        reward: u32,
    },
    RewardChest(Vec<u8>),
}

impl AchievementTrigger {
    fn label(&self) -> &'static str {
        match self {
            Self::Kill(..) => "kill-count",
            Self::ConsumeItem { .. } => "item-use",
            Self::NpcTalk { .. } => "npc-talk",
            Self::RewardChest(_) => "quest-reward",
        }
    }

    fn owner(&self) -> &'static str {
        match self {
            Self::Kill(..) => "crates/gameplay/durability/src/durability/bestiary_progress.rs:776",
            Self::ConsumeItem { .. } => {
                "crates/gameplay/durability/src/durability/consume_transform.rs:1150"
            }
            Self::NpcTalk { .. } => {
                "crates/gameplay/durability/src/durability/quest_state/npc_program/achievements.rs:42"
            }
            Self::RewardChest(_) => {
                "crates/gameplay/dispatch/src/gameplay_transport/bundle_chest.rs:332"
            }
        }
    }

    fn donor(&self) -> &'static str {
        match self {
            Self::Kill(..) => {
                "data/scripts/creaturescripts/monster/terrified_elephant_death.lua:15"
            }
            Self::ConsumeItem { donor, .. } => donor,
            _ => "unverified",
        }
    }

    async fn fire(
        &self,
        client: &mut DevClientSession,
        world: &BundleWorld,
        replay: bool,
    ) -> TestResult {
        use oteryn_protocol_oteryn::world_object::UseDisposition;
        match self {
            Self::Kill(species, count) => {
                let bytes = world_bytes()?;
                let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
                for _ in 0..*count {
                    full_row_p05::hunt_for_race(client, world, &metadata.spawns, species).await?;
                }
            }
            Self::ConsumeItem { item, .. } => {
                let index = provisioning::item_index()?;
                let reference = provisioning::item_reference(&index, *item)?;
                if quantity(&inventory(client)?, reference) == 0 {
                    full_row_i14::acquire(client, world, *item, 1)
                        .await
                        .map_err(|error| {
                            format!("harness-achievement-item-prerequisite-{item}-{error}")
                        })?;
                }
                let reply = client.use_item(handle(client, reference)?).await?;
                if reply.disposition != UseDisposition::Committed {
                    return Err(format!(
                        "harness-achievement-item-use-refused-{:?}",
                        reply.disposition
                    )
                    .into());
                }
            }
            Self::NpcTalk {
                npc,
                keyword,
                reward,
            } => {
                use oteryn_dev_client::client_runtime::npc_shop;
                use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
                let bytes = world_bytes()?;
                let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
                let npc = metadata
                    .npcs
                    .npcs
                    .iter()
                    .find(|row| row.key == *npc)
                    .ok_or_else(|| format!("harness-npc-placement-missing-{npc}"))?;
                let actor = full_row_s10::approach(client, world, npc)
                    .await
                    .map_err(|error| format!("harness-npc-approach-{error}"))?;
                let index = provisioning::item_index()?;
                let reference = provisioning::item_reference(&index, *reward)?;
                let before = quantity(&inventory(client)?, reference);
                for text in ["hi", *keyword] {
                    let reply = npc_shop::talk(
                        client.session_mut(),
                        &NpcTalkIntent {
                            npc_actor: actor,
                            text: text.into(),
                        },
                    )
                    .await?;
                    if reply != NpcIntentDisposition::Ok {
                        return Err(format!("harness-npc-talk-{text}-refused-{reply:?}").into());
                    }
                }
                if !replay {
                    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                    while quantity(&inventory(client)?, reference) <= before {
                        if tokio::time::Instant::now() >= deadline {
                            return Err(
                                "harness-npc-mission-prerequisites-or-reward-unverified".into()
                            );
                        }
                        client.service_liveness(Duration::from_millis(50)).await?;
                    }
                }
            }
            Self::RewardChest(placement) => {
                let response = client.session_mut().use_object(placement, 0).await?;
                if !replay && response.disposition != UseDisposition::Committed {
                    return Err(format!(
                        "harness-quest-trigger-refused-{:?}",
                        response.disposition
                    )
                    .into());
                }
            }
        }
        Ok(())
    }
}

async fn achievement_trigger(
    client: &mut DevClientSession,
    world: &BundleWorld,
    entry: &Entry,
) -> TestResult<AchievementTrigger> {
    use oteryn_gameplay_progression::{
        achievement_granters, bestiary_achievement, item_use_achievement,
    };
    let key = entry.key.as_str();
    if bestiary_achievement::kill_count_keys("oteryn:creature.terrified_elephant", 5)
        .any(|rule| rule == key)
    {
        return Ok(AchievementTrigger::Kill(
            "oteryn:creature.terrified_elephant",
            5,
        ));
    }
    if key == item_use_achievement::POTION_ADDICT.key {
        // The counter owner requires 100,000 successful uses. A seeded fixture
        // must establish N-1 before this one real potion use can be meaningful.
        return Err("harness-item-use-counter-seed-missing".into());
    }
    if bestiary_achievement::all_keys().any(|rule| rule == key) {
        return Err("harness-bestiary-completion-seed-missing".into());
    }
    use oteryn_gameplay_items::novelty_items::{self, NoveltyKind};
    if let Some(rule) = novelty_items::rules()
        .into_iter()
        .flatten()
        .find(|rule| rule.achievement.as_deref() == Some(key))
    {
        if rule.kind == NoveltyKind::ConsumeEffect
            && let Some(&item) = rule.item_ids.first()
        {
            return Ok(AchievementTrigger::ConsumeItem {
                item,
                donor: &rule.donor,
            });
        }
        return Err("harness-targeted-item-use-fixture-missing".into());
    }
    let known_gap = match key {
        "oteryn:achievement/a_reliable_friend" | "oteryn:achievement/backpack_tourist" => {
            Some("quest-mission")
        }
        "oteryn:achievement/bastard" => {
            // Same final conversation and reward as the server's M41 grant fixture.
            return Ok(AchievementTrigger::NpcTalk {
                npc: "oteryn:npc.eldoran_the_ambitious",
                keyword: "news",
                reward: 52824,
            });
        }
        key if key == achievement_granters::SHIPS_KOBOLD.key => Some("paid-travel"),
        key if [
            achievement_granters::ASHES_TO_DUST.key,
            achievement_granters::SKIN_DEEP.key,
            achievement_granters::HERE_FISHY_FISHY.key,
            achievement_granters::EXQUISITE_TASTE.key,
        ]
        .contains(&key) =>
        {
            Some("collection")
        }
        key if [
            achievement_granters::POLISHER.key,
            item_use_achievement::PETROLOGIST.key,
        ]
        .contains(&key) =>
        {
            Some("item-use")
        }
        _ => None,
    };
    if let Some(kind) = known_gap {
        return Err(format!("harness-{kind}-trigger-fixture-missing").into());
    }
    let index = provisioning::item_index()?;
    let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
        world.base().as_ref(),
        &|item| {
            item_id(item.key().as_str())
                .is_ok_and(|id| provisioning::item_reference(&index, id).is_ok())
        },
    )?;
    let here = tile(client.world_spatial().actor_position)?;
    let placement = report
        .served
        .into_iter()
        .flat_map(|claim| claim.placements)
        .filter(|placement| placement.placement.achievement.as_deref() == Some(key))
        .filter_map(|placement| {
            let cell = placement.cell;
            let target = TilePos {
                x: cell.x,
                y: cell.y,
                floor: cell.floor,
            };
            route(world, here, |at| adjacent(at, target))
                .map(|path| (placement.bundle_placement_key, path))
        })
        .min_by_key(|(_, path)| path.len());
    let Some((placement, path)) = placement else {
        let reason = if matches!(key, "oteryn:achievement/annihilator") {
            "harness-quest-reward-placement-unreachable"
        } else {
            "harness-no-trigger-path-in-game-identified"
        };
        return Err(reason.into());
    };
    walk(client, world, &path).await?;
    Ok(AchievementTrigger::RewardChest(placement_bytes(
        world, placement,
    )))
}

fn granted_once(before: (u32, u32), after: (u32, u32), points: u32, facts: &[u32]) -> bool {
    facts == [points]
        && before.0.checked_add(1) == Some(after.0)
        && before.1.checked_add(points) == Some(after.1)
}

async fn achievement_facts(
    client: &mut DevClientSession,
    key: &str,
) -> TestResult<((u32, u32), Vec<u32>)> {
    use oteryn_dev_client::client_runtime::account_achievements::Response;
    let mut facts = Vec::new();
    let mut totals = None;
    let max_pages = 128;
    for page in 0..max_pages {
        let Response::Page(result) = client.session_mut().account_achievements(page).await? else {
            return Err("achievement-page-refused".into());
        };
        let observed = (result.fact_count, result.total_points);
        if result.page != page
            || totals
                .replace(observed)
                .is_some_and(|previous| previous != observed)
        {
            return Err("achievement-pagination-inconsistent".into());
        }
        facts.extend(
            result
                .rows
                .into_iter()
                .filter(|row| row.key == key)
                .map(|row| row.points),
        );
        if !result.has_more {
            return Ok((observed, facts));
        }
    }
    Err("achievement-page-limit".into())
}

fn adjacent(at: TilePos, target: TilePos) -> bool {
    at.floor == target.floor && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= 1
}

fn placement_bytes(world: &BundleWorld, key: u64) -> Vec<u8> {
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    placement
}

fn item_id(key: &str) -> TestResult<u32> {
    Ok(key
        .strip_prefix("oteryn:item.tibia.i")
        .ok_or("non-native-item-key")?
        .parse()?)
}

async fn accepted(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    bytes: &[u8],
) -> TestResult<Vec<u8>> {
    let reply = client
        .session_mut()
        .gated_command(capability, command, bytes)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("command-{command}-refused-{:?}", reply.status).into());
    }
    Ok(reply.payload)
}

async fn charm_command(
    client: &mut DevClientSession,
    action: oteryn_protocol_oteryn::charm_progression::CharmProgressionAction,
) -> TestResult<oteryn_protocol_oteryn::charm_progression::CharmProgressionReply> {
    use oteryn_protocol_oteryn::charm_progression as wire;
    let request = wire::encode_charm_progression_request(&action).map_err(|e| format!("{e:?}"))?;
    let reply = accepted(
        client,
        wire::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
        wire::COMMAND_TYPE_CHARM_PROGRESSION,
        &request,
    )
    .await?;
    wire::decode_charm_progression_reply(&reply).map_err(|e| format!("{e:?}").into())
}

fn charm_balance(view: &oteryn_protocol_oteryn::charm::CharmView, minor: bool) -> u32 {
    if minor {
        view.minor_charm_echoes_available
    } else {
        view.charm_points_available
    }
}

fn charm_unlock_balances_match(
    before: &oteryn_protocol_oteryn::charm::CharmView,
    after: &oteryn_protocol_oteryn::charm::CharmView,
    minor: bool,
    cost: u32,
) -> bool {
    use crate::domain::charm::MAJOR_STAGE_ECHOES;

    let other_after = if minor {
        Some(u64::from(charm_balance(before, false)))
    } else {
        u64::from(charm_balance(before, true)).checked_add(MAJOR_STAGE_ECHOES[0])
    };
    charm_balance(before, minor).checked_sub(cost) == Some(charm_balance(after, minor))
        && other_after == Some(u64::from(charm_balance(after, !minor)))
}

async fn charm_fixture(
    client: &mut DevClientSession,
    profile: String,
) -> TestResult<Result<String, String>> {
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let setup = tokio::spawn(request_fixture_control(session, "--profile", profile));
    // Let each idle read finish so a ready control reply never interrupts frame handling.
    while !setup.is_finished() {
        client
            .session_mut()
            .service_liveness_discarding_events(Duration::from_millis(100))
            .await?;
    }
    Ok(setup.await?)
}

async fn fund_major_points(
    client: &mut DevClientSession,
    view: &mut oteryn_protocol_oteryn::charm_progression::CharmProgressionReply,
    cost: u32,
) -> TestResult {
    use oteryn_protocol_oteryn::charm_progression::CharmProgressionAction as Action;
    for _ in 0..3 {
        let balance = charm_balance(&view.charms, false);
        if balance >= cost {
            break;
        }
        let setup = charm_fixture(client, "progress-charms".into()).await?;
        *view = charm_command(client, Action::Query).await?;
        if charm_balance(&view.charms, false) <= balance {
            return Err(format!("harness-charm-funding-no-currency-gain-{setup:?}").into());
        }
    }
    Ok(())
}

async fn fund_minor_echoes(
    client: &mut DevClientSession,
    catalogue: &crate::domain::charm::CharmCatalogue,
    view: &mut oteryn_protocol_oteryn::charm_progression::CharmProgressionReply,
    cost: u32,
) -> TestResult {
    use crate::domain::charm::{CharmCategory, MAJOR_STAGE_ECHOES};
    use oteryn_protocol_oteryn::charm::CharmUnlockDisposition;
    use oteryn_protocol_oteryn::charm_progression::{
        CharmProgressionAction as Action, CharmProgressionOutcome as Outcome,
    };
    for _ in 0..MAJOR_STAGE_ECHOES.len() {
        let echoes = charm_balance(&view.charms, true);
        if echoes >= cost {
            break;
        }
        let mut candidates = Vec::new();
        for (ordinal, definition) in catalogue.definitions().enumerate() {
            if definition.category != CharmCategory::Major {
                continue;
            }
            let id = NonZeroU32::new(u32::try_from(ordinal + 1)?).ok_or("zero-charm-id")?;
            let state = charm_state(view, id)?;
            let stage = usize::from(state.unlocked_stage);
            if let Some(&price) = definition.stage_costs.get(stage) {
                if state.next_stage_cost != price {
                    return Err(format!("charm-echo-prerequisite-price-mismatch-{id}").into());
                }
                candidates.push((stage == 0, price, id, state.unlocked_stage));
            }
        }
        let (_, price, id, stage) = candidates
            .into_iter()
            .min()
            .ok_or("harness-charm-no-major-echo-prerequisite")?;
        fund_major_points(client, view, price).await?;
        let expected_points = charm_balance(&view.charms, false)
            .checked_sub(price)
            .ok_or("harness-charm-echo-prerequisite-major-funds-insufficient")?;
        let expected_echoes =
            u64::from(charm_balance(&view.charms, true)) + MAJOR_STAGE_ECHOES[usize::from(stage)];
        let unlocked = charm_command(
            client,
            Action::Unlock {
                charm: id,
                expected_stage: stage,
            },
        )
        .await?;
        if unlocked.outcome != Outcome::Unlock(CharmUnlockDisposition::Unlocked)
            || charm_state(&unlocked, id)?.unlocked_stage != stage + 1
            || charm_balance(&unlocked.charms, false) != expected_points
            || u64::from(charm_balance(&unlocked.charms, true)) != expected_echoes
        {
            return Err(format!(
                "charm-echo-prerequisite-unlock-mismatch-{id}-{:?}",
                unlocked.outcome
            )
            .into());
        }
        *view = unlocked;
    }
    Ok(())
}

fn charm_state(
    reply: &oteryn_protocol_oteryn::charm_progression::CharmProgressionReply,
    id: NonZeroU32,
) -> TestResult<&oteryn_protocol_oteryn::charm::CharmState> {
    reply
        .charms
        .charms
        .iter()
        .find(|state| state.charm == id)
        .ok_or_else(|| format!("charm-{id}-missing-from-wire-view").into())
}

fn world_bytes() -> TestResult<Vec<u8>> {
    Ok(std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?)
}

// The content adapter preserves stage costs and assigns IDs in source order
// (crates/gameplay/content/src/content/charm_source.rs:181).
async fn charm(
    client: &mut DevClientSession,
    world: &BundleWorld,
    rows: &mut Rows<'_>,
    unsupported_mastery: &mut std::collections::BTreeSet<String>,
) -> TestResult {
    use oteryn_protocol_oteryn::charm::{
        CharmAssignDisposition, CharmKind, CharmUnlockDisposition,
    };
    use oteryn_protocol_oteryn::charm_progression::{
        CharmProgressionAction as Action, CharmProgressionOutcome as Outcome,
        CharmRemoveDisposition,
    };

    let content = &rows.entry.content;
    let minor = match string(&content["cost_currency"])? {
        "minor_charm_echoes" => true,
        "charm_points" => false,
        other => return Err(format!("unknown-charm-currency-{other}").into()),
    };
    let cost = number(&content["stages"][0]["cost"])?;
    let catalogue = crate::content::charm_source::CanonicalCharmCatalogue::embedded()?;
    let ordinal = catalogue
        .catalogue()
        .definitions()
        .position(|definition| definition.key.as_str() == rows.entry.key)
        .ok_or("charm-key-unavailable-in-owner-catalogue")?;
    let id = NonZeroU32::new(u32::try_from(ordinal + 1)?).ok_or("zero-charm-id")?;
    let mut before = charm_command(client, Action::Query).await?;
    let state = charm_state(&before, id)?;
    let kind = if minor {
        CharmKind::Minor
    } else {
        CharmKind::Major
    };
    if before.outcome != Outcome::Query
        || state.kind != kind
        || state.unlocked_stage != 0
        || state.next_stage_cost != cost
    {
        return Err(format!(
            "charm-stage-cost-mismatch-expected-{kind:?}-{cost}-observed-{state:?}"
        )
        .into());
    }
    let bytes = world_bytes()?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let races = full_row_p05::canonical_races()?;
    let mut hunt_budget = 24;
    if charm_balance(&before.charms, minor) < cost {
        let mut plan = Vec::new();
        for (ordinal, (definition, points)) in races.iter().enumerate() {
            let race = NonZeroU32::new(u32::try_from(ordinal + 1)?).ok_or("zero-race-id")?;
            let kills = before
                .bestiary
                .iter()
                .find(|row| row.race == race)
                .map_or(0, |row| row.kill_count);
            let remaining = definition.final_kill_threshold().saturating_sub(kills);
            if remaining != 0 && remaining <= hunt_budget && *points != 0 {
                plan.push((remaining, race, definition.key()));
            }
        }
        plan.sort_unstable();
        for (remaining, race, key) in plan {
            if charm_balance(&before.charms, minor) >= cost || remaining > hunt_budget {
                break;
            }
            let here = tile(client.world_spatial().actor_position)?;
            if !full_row_p05::has_reachable_spawn(world, here, &metadata.spawns, key) {
                continue;
            }
            for _ in 0..remaining {
                let kill =
                    full_row_p05::hunt_for_race(client, world, &metadata.spawns, key).await?;
                if kill.race != race {
                    return Err("charm-funding-kill-credited-to-wrong-race".into());
                }
                hunt_budget -= 1;
            }
            before = charm_command(client, Action::Query).await?;
        }
    }
    if minor {
        fund_minor_echoes(client, catalogue.catalogue(), &mut before, cost).await?;
    } else {
        fund_major_points(client, &mut before, cost).await?;
    }
    let balance = charm_balance(&before.charms, minor);
    if balance < cost {
        return Err(format!("charm-currency-prerequisite-needed-{cost}-observed-{balance}").into());
    }
    let mut unlocked = charm_command(
        client,
        Action::Unlock {
            charm: id,
            expected_stage: 0,
        },
    )
    .await?;
    if unlocked.outcome != Outcome::Unlock(CharmUnlockDisposition::Unlocked)
        || charm_state(&unlocked, id)?.unlocked_stage != 1
        || !charm_unlock_balances_match(&before.charms, &unlocked.charms, minor, cost)
    {
        return Err(format!("charm-unlock-mismatch-{:?}", unlocked.outcome).into());
    }
    rows.pass(&format!("stage-one-debit-{cost}"));

    // Free the prior probe's race and slot over the same command path a player uses.
    if let Some(prior) = unlocked
        .charms
        .charms
        .iter()
        .find(|state| state.assigned_race.is_some())
    {
        let prior = prior.charm;
        let removed = charm_command(client, Action::Remove { charm: prior }).await?;
        unlocked = if removed.outcome == Outcome::Remove(CharmRemoveDisposition::NotEnoughGold) {
            charm_fixture(client, "progress-charm-fee".into())
                .await?
                .map_err(|reason| format!("harness-charm-fee-fixture-refused-{reason}"))?;
            charm_command(client, Action::Remove { charm: prior }).await?
        } else {
            removed
        };
        if unlocked.outcome != Outcome::Remove(CharmRemoveDisposition::Removed)
            || unlocked.fee_gold == 0
            || charm_state(&unlocked, prior)?.assigned_race.is_some()
        {
            return Err(format!(
                "harness-charm-cleanup-mismatch-{prior}-{:?}",
                unlocked.outcome
            )
            .into());
        }
    }
    let here = tile(client.world_spatial().actor_position)?;
    let mut candidates = Vec::new();
    for (ordinal, (definition, _)) in races.iter().enumerate() {
        let race = NonZeroU32::new(u32::try_from(ordinal + 1)?).ok_or("zero-race-id")?;
        if unlocked
            .charms
            .charms
            .iter()
            .any(|state| state.assigned_race == Some(race))
        {
            continue;
        }
        let kills = unlocked
            .bestiary
            .iter()
            .find(|row| row.race == race)
            .map_or(0, |row| row.kill_count);
        let threshold = definition.final_kill_threshold();
        candidates.push((
            threshold.saturating_sub(kills),
            race,
            definition.key(),
            threshold,
        ));
    }
    candidates.sort_unstable();
    let mut target = None;
    'targets: for (remaining, race, species, threshold) in candidates {
        if unsupported_mastery.contains(species)
            || !full_row_p05::has_reachable_spawn(world, here, &metadata.spawns, species)
        {
            continue;
        }
        if remaining != 0 {
            let mut kills = threshold - remaining;
            for _ in 0..6 {
                let setup = charm_fixture(client, format!("progress-mastery:{species}")).await?;
                if setup
                    .as_ref()
                    .is_err_and(|reason| reason == "mastery_target_unavailable")
                {
                    unsupported_mastery.insert(species.to_owned());
                    continue 'targets;
                }
                let progress = charm_command(client, Action::Query).await?;
                let observed = progress
                    .bestiary
                    .iter()
                    .find(|row| row.race == race)
                    .map_or(0, |row| row.kill_count);
                if observed >= threshold {
                    kills = observed;
                    break;
                }
                if observed <= kills {
                    return Err(
                        format!("harness-charm-mastery-no-progress-{species}-{setup:?}").into(),
                    );
                }
                kills = observed;
            }
            if kills < threshold {
                return Err(format!(
                    "harness-charm-mastery-incomplete-{species}-{kills}-of-{threshold}"
                )
                .into());
            }
        }
        target = Some((remaining, race, species));
        break;
    }
    let (remaining, race, species) = target.ok_or("charm-no-reachable-bestiary-creature")?;
    let assigned = charm_command(client, Action::Assign { charm: id, race }).await?;
    let observed = charm_state(&assigned, id)?;
    if assigned.outcome != Outcome::Assign(CharmAssignDisposition::Assigned)
        || observed.assigned_race != Some(race)
    {
        return Err(format!("charm-assignment-mismatch-{:?}", assigned.outcome).into());
    }
    rows.pass(&format!(
        "assigned-{species}-mastery-{}",
        if remaining == 0 { "existing" } else { "seeded" }
    ));
    let effect = string(&rows.entry.content["effect"]["type"])?;
    if !observed.effect_active {
        return Err(format!("charm-content-effect-inactive-{effect}").into());
    }
    if effect == "kill_area_damage" {
        let (before, after) = carnage_probe(client, world, &metadata.spawns, species, race).await?;
        rows.pass(&format!(
            "untargeted-cardinal-neighbour-health-{before}-to-{after}"
        ));
        return Ok(());
    }
    use oteryn_protocol_oteryn::damage_element::DamageElement;
    let element = match (effect, content["effect"]["element"].as_str()) {
        ("attack_proc_damage", Some("fire")) => DamageElement::Fire,
        ("attack_proc_damage", Some("earth")) => DamageElement::Earth,
        ("attack_proc_damage", Some("energy")) => DamageElement::Energy,
        ("attack_proc_damage", Some("ice")) => DamageElement::Ice,
        ("attack_proc_damage", Some("holy")) => DamageElement::Holy,
        ("attack_proc_damage", Some("death")) => DamageElement::Death,
        _ => return Err(format!("harness-charm-wire-effect-probe-missing-{effect}").into()),
    };
    // An unarmed hunt supplies physical damage as the baseline.
    if client.inventory().is_none_or(|inventory| {
        inventory.equipment.iter().any(|item| {
            use oteryn_protocol_oteryn::item_view::EquipmentSlot;
            matches!(
                item.slot,
                EquipmentSlot::LeftHand | EquipmentSlot::RightHand
            )
        })
    }) {
        return Err("harness-charm-effect-probe-needs-unarmed-baseline".into());
    }
    for _ in 0..12 {
        if full_row_p05::hunt_for_race(client, world, &metadata.spawns, species)
            .await?
            .damage_elements
            & (1 << element.wire())
            != 0
        {
            rows.pass(&format!("observed-{element:?}-proc"));
            return Ok(());
        }
    }
    Err(format!("harness-charm-{element:?}-proc-unobserved-within-hunt-budget").into())
}

fn charm_creature_view(
    client: &DevClientSession,
    entity: &oteryn_dev_client::EntityRef,
) -> Option<(TilePos, u8)> {
    use oteryn_dev_client::{EntityDetail, EntityKind};
    let row = client.world_entities()?.get(entity)?;
    if row.kind != EntityKind::Creature {
        return None;
    }
    match row.detail {
        EntityDetail::Actor { health_percent, .. }
        | EntityDetail::ActorWithIcons { health_percent, .. } => {
            Some((tile(row.position).ok()?, health_percent))
        }
        _ => None,
    }
}

async fn carnage_probe(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    species: &str,
    race: NonZeroU32,
) -> TestResult<(u8, u8)> {
    use oteryn_dev_client::SessionEvent;
    use oteryn_protocol_oteryn::{analyser::AnalyserFact, attack::AttackIntentDisposition};
    let first = full_row_p05::wait_for_spawn(client, world, spawns, species)
        .await?
        .entity;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let mut creatures = vec![first];
    for _ in 0..12 {
        creatures.retain(|id| charm_creature_view(client, id).is_some());
        let mut pair = None;
        for spawn_attempt in 0..=3 {
            pair = creatures.iter().find_map(|primary| {
                let (at, _) = charm_creature_view(client, primary)?;
                creatures.iter().find_map(|neighbour| {
                    let (other, health) = charm_creature_view(client, neighbour)?;
                    (at.floor == other.floor
                        && u32::from(at.x.abs_diff(other.x)) + u32::from(at.y.abs_diff(other.y))
                            == 1)
                        .then_some((*primary, *neighbour, health))
                })
            });
            if pair.is_some() || spawn_attempt == 3 {
                break;
            }
            let spawned =
                parse_spawned_entity(&request_spawn(session.clone(), species.into()).await?)?;
            let until = tokio::time::Instant::now() + Duration::from_secs(4);
            while charm_creature_view(client, &spawned).is_none() {
                if tokio::time::Instant::now() >= until {
                    return Err("harness-carnage-spawn-not-visible".into());
                }
                client
                    .session_mut()
                    .service_liveness_discarding_events(Duration::from_millis(100))
                    .await?;
            }
            creatures.push(spawned);
        }
        let (primary, neighbour, health) = pair.ok_or("harness-carnage-no-cardinal-pair")?;
        client.take_events();
        let reply = client.session_mut().attack_target(Some(&primary)).await?;
        if reply.disposition != AttackIntentDisposition::Ok {
            return Err(format!("harness-carnage-attack-refused-{:?}", reply.disposition).into());
        }
        let until = tokio::time::Instant::now() + Duration::from_secs(12);
        let mut kill_seen = false;
        while tokio::time::Instant::now() < until {
            client.service_liveness(Duration::from_millis(100)).await?;
            for event in client.take_events() {
                let kills_race =
                    |fact| matches!(fact, AnalyserFact::Kill { race: Some(seen) } if seen == race);
                kill_seen |= match event {
                    SessionEvent::AnalyserFacts(delta) => delta.value.into_iter().any(kills_race),
                    SessionEvent::AnalyserAttributedFacts(delta) => {
                        delta.value.into_iter().any(|fact| kills_race(fact.fact))
                    }
                    _ => false,
                };
            }
            if kill_seen && charm_creature_view(client, &primary).is_none() {
                break;
            }
        }
        client.session_mut().attack_target(None).await?;
        if !kill_seen || charm_creature_view(client, &primary).is_some() {
            return Err("harness-carnage-primary-kill-not-observed".into());
        }
        if let Some((_, after)) = charm_creature_view(client, &neighbour)
            && after < health
        {
            return Ok((health, after));
        }
    }
    Err("harness-carnage-area-damage-unobserved-within-hunt-budget".into())
}

type Quantities = BTreeMap<NonZeroU32, i64>;
type ItemIndex = crate::content::item_ref::ItemDefinitionIndex;

fn reference(index: &ItemIndex, item: &Value) -> TestResult<NonZeroU32> {
    provisioning::item_reference(index, item_id(string(&item["key"])?)?)
}

fn inventory(client: &DevClientSession) -> TestResult<Quantities> {
    let mut counts = BTreeMap::new();
    let view = client.inventory().ok_or("inventory-snapshot-missing")?;
    for item in view
        .entries
        .iter()
        .chain(view.equipment.iter().map(|slot| &slot.item))
    {
        *counts.entry(item.item_definition_ref).or_default() += i64::from(item.count.get());
    }
    Ok(counts)
}

fn quantity(counts: &Quantities, item: NonZeroU32) -> i64 {
    counts.get(&item).copied().unwrap_or_default()
}

fn handle(
    client: &DevClientSession,
    item: NonZeroU32,
) -> TestResult<oteryn_protocol_oteryn::item_view::ItemHandle> {
    client
        .inventory()
        .and_then(|view| {
            view.entries
                .iter()
                .find(|entry| entry.item_definition_ref == item)
        })
        .map(|entry| entry.handle)
        .ok_or_else(|| format!("no-carried-item-handle-{item}").into())
}

// The craft owner resolves the carried source and map/carried target from wire USE,
// then applies the operands through the item authority
// (crates/gameplay/dispatch/src/gameplay_transport/item_craft.rs:26,154).
#[derive(Default)]
struct CraftPlan {
    inputs: Quantities,
    outputs: Vec<Quantities>,
    map_inputs: Quantities,
    map_outputs: Quantities,
}

fn map_operand(recipe: &Value, operand: &Value) -> bool {
    operand["role"] == "TARGET" && recipe["target"]["kind"] != "carried"
}

fn craft_oracle(recipe: &Value, index: &ItemIndex) -> TestResult<CraftPlan> {
    let mut plan = CraftPlan::default();
    let mut transformed = BTreeMap::new();
    for operand in list(&recipe["operands"])? {
        let (inputs, transformed) = if map_operand(recipe, operand) {
            (&mut plan.map_inputs, &mut plan.map_outputs)
        } else {
            (&mut plan.inputs, &mut transformed)
        };
        let item = reference(index, &operand["item_id"])?;
        let count = i64::from(number(&operand["count"])?);
        match string(&operand["effect"])? {
            "KEEP" => {
                inputs.entry(item).or_insert(0);
            }
            "TRANSFORM_TO" | "CONSUME" => {
                *inputs.entry(item).or_default() -= count;
                if operand["effect"] == "TRANSFORM_TO" {
                    *transformed
                        .entry(reference(index, &operand["output_item_id"])?)
                        .or_default() += count;
                }
            }
            other => return Err(format!("unknown-craft-operand-effect-{other}").into()),
        }
    }
    let outputs = &recipe["outputs"];
    let alternatives = match string(&outputs["kind"])? {
        "fixed" => std::slice::from_ref(outputs),
        "weighted" => list(&outputs["alternatives"])?,
        other => return Err(format!("unknown-craft-output-kind-{other}").into()),
    };
    plan.outputs = alternatives
        .iter()
        .map(|alternative| {
            let mut delta = transformed.clone();
            for output in list(&alternative["entries"])? {
                *delta
                    .entry(reference(index, &output["item_id"])?)
                    .or_default() += i64::from(number(&output["count"])?);
            }
            Ok(delta)
        })
        .collect::<TestResult<Vec<_>>>()?;
    Ok(plan)
}

fn output_matches(
    before: &Quantities,
    after: &Quantities,
    inputs: &Quantities,
    outputs: &[Quantities],
) -> bool {
    outputs.iter().any(|choice| {
        before
            .keys()
            .chain(after.keys())
            .chain(inputs.keys())
            .chain(choice.keys())
            .all(|&item| {
                quantity(after, item) - quantity(before, item)
                    == quantity(inputs, item) + quantity(choice, item)
            })
    })
}

async fn seed_baking_dough(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
    let socket = required("NODE_BOOT_CONTROL_SOCKET")?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let provision = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/qualification/node_boot/provision.py");
    let status = tokio::task::spawn_blocking(move || {
        crate::qualification::provision_python()
            .arg(provision)
            .args([
                "--socket",
                &socket,
                "--session",
                &session,
                "--profile",
                "progress-baking",
            ])
            .status()
    })
    .await??;
    if !status.success() {
        return Err("harness-baking-dough-fixture-refused".into());
    }
    let (result, ..) = logout_retrying_busy(client).await?;
    if result != Some(LogoutResult::Accepted) {
        return Err("harness-baking-fixture-logout-refused".into());
    }
    *client = connect_character(clients, 0x70, 1).await?;
    Ok(())
}

async fn craft(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    rows: &mut Rows<'_>,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{
        self as wire, MapUseTarget, UseIntent, UseTarget, WorldObjectTarget,
    };
    let recipe = &rows.entry.content["recipe"];
    let index = provisioning::item_index()?;
    let plan = craft_oracle(recipe, &index)?;
    let source = reference(&index, &recipe["input_item"])?;
    for operand in list(&recipe["operands"])? {
        if map_operand(recipe, operand) {
            continue;
        }
        let item = reference(&index, &operand["item_id"])?;
        let count = number(&operand["count"])?;
        let existing = quantity(&inventory(client)?, item);
        if existing < i64::from(count) {
            let id = item_id(string(&operand["item_id"]["key"])?)?;
            let missing = u32::try_from(i64::from(count) - existing)?;
            if id == 3604 && missing == 1 {
                seed_baking_dough(client, clients).await?;
                if quantity(&inventory(client)?, item) < i64::from(count) {
                    return Err("harness-seeded-dough-missing-after-admission".into());
                }
            } else {
                full_row_i14::acquire(client, world, id, missing).await?;
            }
        }
    }
    let target = &recipe["target"];
    let mut map_before = None;
    let map_item = match string(&target["kind"])? {
        "carried" => list(&recipe["operands"])?
            .iter()
            .find(|operand| operand["role"] == "TARGET")
            .map(|operand| -> TestResult<_> {
                Ok(MapUseTarget::Item(handle(
                    client,
                    reference(&index, &operand["item_id"])?,
                )?))
            })
            .transpose()?,
        "map_item" | "tile_kind" => {
            let (at, ordinal, path) =
                station(world, client, string(&target["definition"]["key"])?)?;
            walk(client, world, &path).await?;
            let key = crate::map::view::placement_key(at, ordinal)
                .ok_or("craft-station-placement-missing")?;
            let (tile, counts) = map_tile(client, at)?;
            let expected = reference(&index, &target["definition"])?;
            let revision = tile
                .items
                .iter()
                .find_map(|item| match item.origin {
                    oteryn_protocol_oteryn::world_map::MapOrigin::BaseOrdinal {
                        ordinal: seen,
                        object_revision,
                    } if seen == ordinal
                        && item.definition
                            == oteryn_protocol_oteryn::world_map::MapDefinition::Item(expected) =>
                    {
                        Some(object_revision)
                    }
                    _ => None,
                })
                .ok_or("craft-station-source-ordinal-missing")?;
            if !plan.map_inputs.is_empty() {
                map_before = Some((at, counts));
            }
            Some(MapUseTarget::Object(WorldObjectTarget {
                placement: placement_bytes(world, key),
                expected_revision: revision,
            }))
        }
        other => return Err(format!("unknown-craft-target-{other}").into()),
    };
    let before = inventory(client)?;
    let payload = wire::encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(handle(client, source)?),
        use_with: None,
        use_with_map_item: map_item,
        use_with_position: None,
    })
    .map_err(|e| format!("craft-intent-encode-{e:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition =
        wire::decode_use_result(&reply.payload).map_err(|e| format!("craft-reply-decode-{e:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != wire::UseDisposition::Committed {
        return Err(format!("craft-use-{:?}-{disposition:?}", reply.status).into());
    }
    rows.pass("content-use-committed");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let (after, map_after) = loop {
        let observed = inventory(client)?;
        let map_after = map_before
            .as_ref()
            .map(|(at, _)| map_tile(client, *at).map(|(_, counts)| counts))
            .transpose()?;
        let map_matches =
            map_before
                .as_ref()
                .zip(map_after.as_ref())
                .is_none_or(|((_, before), after)| {
                    output_matches(
                        before,
                        after,
                        &plan.map_inputs,
                        std::slice::from_ref(&plan.map_outputs),
                    )
                });
        if (map_matches && output_matches(&before, &observed, &plan.inputs, &plan.outputs))
            || tokio::time::Instant::now() >= deadline
        {
            break (observed, map_after);
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    };
    if plan.inputs.iter().any(|(&item, &delta)| {
        !plan.outputs.iter().any(|choice| {
            quantity(&after, item) - quantity(&before, item) == delta + quantity(choice, item)
        })
    }) {
        return Err("craft-content-input-delta-mismatch".into());
    }
    if let Some(((_, map_before), map_after)) = map_before.as_ref().zip(map_after.as_ref())
        && plan.map_inputs.iter().any(|(&item, &delta)| {
            quantity(map_after, item) - quantity(map_before, item)
                != delta + quantity(&plan.map_outputs, item)
        })
    {
        return Err("craft-map-input-delta-mismatch".into());
    }
    rows.pass("content-input-deltas");
    if !output_matches(&before, &after, &plan.inputs, &plan.outputs) {
        return Err("craft-content-output-delta-mismatch".into());
    }
    if let Some(((_, map_before), map_after)) = map_before.as_ref().zip(map_after.as_ref())
        && !output_matches(
            map_before,
            map_after,
            &plan.map_inputs,
            std::slice::from_ref(&plan.map_outputs),
        )
    {
        return Err("craft-map-output-delta-mismatch".into());
    }
    rows.pass("exact-content-output-deltas");
    Ok(())
}

fn map_tile(
    client: &mut DevClientSession,
    at: TilePos,
) -> TestResult<(oteryn_protocol_oteryn::world_map::MapTile, Quantities)> {
    use oteryn_protocol_oteryn::world_map::MapDefinition;
    let view = client
        .session_mut()
        .world_map_snapshot()
        .ok_or("craft-map-snapshot-missing")?;
    let station = view
        .tiles
        .into_iter()
        .find(|entry| tile(entry.position).ok() == Some(at))
        .ok_or("craft-station-not-in-wire-map")?;
    if station.more {
        return Err("craft-station-stack-truncated-on-wire".into());
    }
    let mut counts = Quantities::new();
    for item in &station.items {
        if let MapDefinition::Item(reference) = item.definition {
            *counts.entry(reference).or_default() += i64::from(item.count);
        }
    }
    Ok((station, counts))
}

fn station(
    world: &BundleWorld,
    client: &DevClientSession,
    key: &str,
) -> TestResult<(TilePos, u8, Vec<StepDirection>)> {
    let here = tile(client.world_spatial().actor_position)?;
    let mut stations = BTreeMap::new();
    oteryn_world_bundle::bundle::visit(&world_bytes()?, |manifest, sector| {
        for tile in sector.tiles {
            if here.x.abs_diff(tile.x) > 161 || here.y.abs_diff(tile.y) > 161 {
                continue;
            }
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                if manifest.palette[item.palette as usize].key == key
                    && let Ok(ordinal) = u8::try_from(ordinal)
                {
                    stations
                        .entry(TilePos {
                            x: tile.x,
                            y: tile.y,
                            floor: sector.floor,
                        })
                        .or_insert(ordinal);
                }
            }
        }
        Ok(())
    })?;
    let (at, path) = route(world, here, |at| {
        stations.keys().any(|&target| adjacent(at, target))
    })
    .and_then(|path| {
        let end = path
            .iter()
            .try_fold(here, |at, &step| destination(world, at, step))?;
        stations
            .keys()
            .find(|&&target| adjacent(end, target))
            .map(|&target| (target, path))
    })
    .ok_or_else(|| format!("no-reachable-craft-station-{key}"))?;
    Ok((at, stations[&at], path))
}

async fn weapon_entry(
    client: &mut DevClientSession,
    key: &str,
    offset: u32,
) -> TestResult<oteryn_protocol_oteryn::weapon_proficiency::ProficiencyCatalogueEntry> {
    use oteryn_protocol_oteryn::weapon_proficiency as wire;
    let reply = accepted(
        client,
        wire::CAPABILITY_WEAPON_PROFICIENCY_V1,
        wire::COMMAND_TYPE_PROFICIENCY_CATALOGUE,
        &offset.to_be_bytes(),
    )
    .await?;
    let page =
        wire::ProficiencyCataloguePage::decode(&reply).ok_or("proficiency-catalogue-decode")?;
    if page.offset != offset {
        return Err("proficiency-catalogue-offset-mismatch".into());
    }
    page.entries
        .into_iter()
        .find(|entry| entry.weapon == key)
        .ok_or_else(|| format!("proficiency-weapon-not-served-{key}").into())
}

// The wire catalogue formats exact source ratios as numerator/denominator
// (crates/gameplay/dispatch/src/gameplay_transport/weapon_proficiency.rs:472).
fn perk_label(value: &Value) -> String {
    match value {
        Value::Object(fields) => {
            if let (Some(numerator), Some(denominator)) =
                (fields.get("numerator"), fields.get("denominator"))
            {
                return format!("{numerator}/{denominator}");
            }
            let details = fields
                .iter()
                .filter(|(key, _)| key.as_str() != "kind")
                .map(|(key, value)| format!("{}: {}", key.replace('_', " "), perk_label(value)))
                .collect::<Vec<_>>()
                .join(", ");
            if let Some(kind) = fields.get("kind").and_then(Value::as_str) {
                format!("{} ({details})", kind.replace('_', " "))
            } else {
                details
            }
        }
        Value::String(text) => text.replace('_', " "),
        other => other.to_string(),
    }
}

// The death owner varies credit with forge influence and the daily double-XP draw
// (crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs:4472,4482).
fn kill_credit_matches(
    shape: crate::domain::weapon_proficiency::ProficiencyTrackShape,
    difficulty: crate::domain::weapon_proficiency::CreatureDifficulty,
    before: u32,
    after: u32,
) -> bool {
    use crate::domain::weapon_proficiency::{CreatureInfluence, creature_points};
    (0..=5)
        .map(CreatureInfluence::Stacks)
        .chain([CreatureInfluence::Fiendish])
        .any(|influence| {
            creature_points(difficulty, influence).is_ok_and(|points| {
                after > before
                    && [points, points * 2]
                        .into_iter()
                        .any(|credit| shape.credited(before, credit) == after)
            })
        })
}

// Catalogue enumeration, progress and selections use the bound weapon's committed track
// (crates/gameplay/dispatch/src/gameplay_transport/weapon_proficiency.rs:235).
async fn proficiency(
    client: &mut DevClientSession,
    world: &BundleWorld,
    root: &Path,
    rows: &mut Rows<'_>,
) -> TestResult {
    use crate::content::{ImportedWeaponProficiencyData, import_committed_weapon_proficiency_data};
    use crate::domain::weapon_proficiency::{
        CreatureDifficulty, ProficiencyThresholdClass, ProficiencyTrackShape,
    };
    use oteryn_protocol_oteryn::weapon_proficiency as wire;
    static SOURCE: std::sync::LazyLock<Result<ImportedWeaponProficiencyData, String>> =
        std::sync::LazyLock::new(|| {
            import_committed_weapon_proficiency_data().map_err(|e| e.to_string())
        });
    let source = SOURCE.as_ref().map_err(String::as_str)?;
    let definition = source
        .definitions()
        .get(&rows.entry.key)
        .ok_or("proficiency-profile-not-imported")?;
    let (offset, (key, weapon)) = source
        .bindings()
        .iter()
        .enumerate()
        .find(|(_, (_, binding))| binding.profile_binding.key == rows.entry.key)
        .ok_or("proficiency-profile-has-no-content-weapon-binding")?;
    let offset = u32::try_from(offset)?;
    let class_key = serde_json::to_value(weapon.threshold_class)?;
    let class = ProficiencyThresholdClass::ALL
        .into_iter()
        .find(|class| class_key == class.as_str())
        .ok_or("proficiency-threshold-class-unknown")?;
    let shape = ProficiencyTrackShape::new(class, u8::try_from(definition.levels.len())?)?;
    let index = provisioning::item_index()?;
    let reference = provisioning::item_reference(&index, item_id(key)?)?;
    let before = weapon_entry(client, key, offset).await?;
    if before.definition != reference.get()
        || before.name != definition.name
        || before.thresholds != *class.thresholds()
    {
        return Err("proficiency-content-binding-mismatch".into());
    }
    if quantity(&inventory(client)?, reference) == 0 {
        full_row_i14::acquire(client, world, item_id(key)?, 1).await?;
    }
    if !client.inventory().is_some_and(|inventory| {
        inventory
            .equipment
            .iter()
            .any(|slot| slot.item.item_definition_ref == reference)
    }) {
        let equipped = client
            .session_mut()
            .equip_item_definition(reference)
            .await?;
        if equipped.status != CommandStatus::Accepted
            || equipped.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
        {
            return Err("proficiency-content-weapon-equip-refused".into());
        }
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !client.inventory().is_some_and(|inventory| {
        inventory
            .equipment
            .iter()
            .any(|slot| slot.item.item_definition_ref == reference)
    }) {
        if tokio::time::Instant::now() >= deadline {
            return Err("proficiency-content-weapon-equip-delta-missing".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    let bytes = world_bytes()?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let hunt =
        full_row_p05::hunt_for_race(client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let rats = document(&root.join("creatures/definitions/creatures-01000-01499.json"))?;
    let rat = list(&rats["records"])?
        .iter()
        .find(|record| record["definition"]["identity"]["key"] == "oteryn:creature.rat")
        .ok_or("proficiency-rat-content-missing")?;
    let difficulty_key = string(&rat["authoring"]["profile"]["bestiary"]["difficulty"])?;
    let difficulty = CreatureDifficulty::ALL
        .into_iter()
        .find(|difficulty| difficulty.as_str() == difficulty_key)
        .ok_or("proficiency-rat-difficulty-unknown")?;
    let after = weapon_entry(client, key, offset).await?;
    if hunt.raw_xp == 0
        || !kill_credit_matches(shape, difficulty, before.experience, after.experience)
    {
        return Err(format!(
            "proficiency-kill-credit-mismatch-{}-to-{}",
            before.experience, after.experience
        )
        .into());
    }
    rows.pass(&format!(
        "weapon-experience-{}-to-{}",
        before.experience, after.experience
    ));

    if after.levels.len() != definition.levels.len() {
        return Err("proficiency-content-level-count-mismatch".into());
    }
    for (ordinal, level) in definition.levels.iter().enumerate() {
        let level_number = level.level;
        let labels = level
            .perks
            .iter()
            .map(|perk| serde_json::to_value(perk).map(|value| perk_label(&value)))
            .collect::<Result<Vec<_>, _>>()?;
        if after.levels[ordinal].perks != labels {
            return Err(format!("proficiency-content-perks-mismatch-level-{level_number}").into());
        }
        if level_number > shape.unlocked_perk_levels(after.experience) {
            return Err(format!(
                "proficiency-perk-level-{level_number}-needs-{}-has-{}",
                class.thresholds()[ordinal],
                after.experience
            )
            .into());
        }
        for (perk, _) in level.perks.iter().enumerate() {
            let perk = u8::try_from(perk)?;
            let request = wire::SelectPerkRequest {
                definition: reference,
                level: u8::try_from(ordinal)?,
                perk: Some(perk),
            };
            let reply = accepted(
                client,
                wire::CAPABILITY_WEAPON_PROFICIENCY_V1,
                wire::COMMAND_TYPE_PROFICIENCY_SELECT_PERK,
                &request.encode(),
            )
            .await?;
            let reply =
                wire::SelectPerkReply::decode(&reply).ok_or("proficiency-perk-reply-decode")?;
            if reply.result != wire::SelectPerkResult::Accepted
                || reply.selections.get(ordinal) != Some(&Some(perk))
            {
                return Err(format!(
                    "proficiency-perk-selection-level-{level_number}-perk-{perk}-{:?}",
                    reply.result
                )
                .into());
            }
            let selected = weapon_entry(client, key, offset).await?;
            if selected.levels.get(ordinal).and_then(|slot| slot.selected) != Some(perk) {
                return Err(format!(
                    "proficiency-perk-selection-not-visible-level-{level_number}-perk-{perk}"
                )
                .into());
            }
        }
    }
    rows.pass("catalogue-and-perk-selections-verified");
    Err("harness-proficiency-selected-perk-runtime-effect-unverified".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_sweep_batch_defaults_and_rejects_invalid_slices() -> TestResult {
        assert_eq!(batch(None)?, (0, 3));
        assert_eq!(batch(Some("20:5"))?, (20, 5));
        for invalid in [
            "",
            "1",
            "0:0",
            "0:-1",
            "1:2:3",
            " 1:2",
            "+1:2",
            "1:999999999999999999999999",
        ] {
            assert!(batch(Some(invalid)).is_err(), "{invalid}");
        }
        Ok(())
    }

    #[test]
    fn progress_sweep_family_batches_keep_offsets_local() -> TestResult {
        for (name, family, size) in [
            ("achievements", Family::Achievement, 40),
            ("charms", Family::Charm, 25),
            ("craft", Family::Craft, 40),
            ("proficiencies", Family::Proficiency, 40),
        ] {
            assert_eq!(
                family_batch(Some(&format!("{name}:0:{size}")))?,
                (Some(family), (0, size))
            );
        }
        assert_eq!(family_batch(Some("571:25"))?, (None, (571, 25)));
        assert!(family_batch(Some("craft:0:0")).is_err());
        assert!(family_batch(Some("weapons:0:40")).is_err());
        Ok(())
    }

    #[test]
    fn progress_sweep_content_census_is_strictly_sorted_and_includes_every_family() -> TestResult {
        let entries = census(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))?;
        assert!(entries.windows(2).all(|pair| pair[0].key < pair[1].key));
        for prefix in ["achievement/", "charm.", "item-conversion.", "proficiency."] {
            assert!(
                entries
                    .iter()
                    .any(|entry| entry.key.starts_with(&format!("oteryn:{prefix}")))
            );
        }
        Ok(())
    }

    #[test]
    fn progress_sweep_charm_unlock_checks_both_derived_currencies() {
        use oteryn_protocol_oteryn::charm::CharmView;

        let view = |points, echoes| CharmView {
            charms: Vec::new(),
            charm_points_available: points,
            minor_charm_echoes_available: echoes,
            assignment_slot_limit: None,
        };
        assert!(charm_unlock_balances_match(
            &view(300, 100),
            &view(60, 150),
            false,
            240
        ));
        assert!(!charm_unlock_balances_match(
            &view(300, 100),
            &view(60, 100),
            false,
            240
        ));
        assert!(charm_unlock_balances_match(
            &view(300, 100),
            &view(300, 0),
            true,
            100
        ));
        assert!(!charm_unlock_balances_match(
            &view(300, 100),
            &view(299, 0),
            true,
            100
        ));
    }

    #[test]
    fn progress_sweep_grant_requires_one_fact_and_exact_authored_points() {
        assert!(granted_once((2, 9), (3, 16), 7, &[7]));
        assert!(!granted_once((2, 9), (3, 15), 7, &[7]));
        assert!(!granted_once((2, 9), (3, 16), 7, &[7, 7]));
        assert!(!granted_once((2, 9), (4, 16), 7, &[7]));
    }

    #[test]
    fn progress_sweep_kill_credit_uses_owner_table_and_saturation() -> TestResult {
        use crate::domain::weapon_proficiency::{
            CreatureDifficulty, ProficiencyThresholdClass, ProficiencyTrackShape,
        };
        let shape = ProficiencyTrackShape::new(ProficiencyThresholdClass::Standard, 7)?;
        assert!(kill_credit_matches(
            shape,
            CreatureDifficulty::Trivial,
            10,
            40
        ));
        assert!(kill_credit_matches(
            shape,
            CreatureDifficulty::Trivial,
            10,
            160
        ));
        assert!(!kill_credit_matches(
            shape,
            CreatureDifficulty::Trivial,
            10,
            41
        ));
        let cap = shape.final_progress();
        assert!(kill_credit_matches(
            shape,
            CreatureDifficulty::Trivial,
            cap - 1,
            cap
        ));
        assert!(!kill_credit_matches(
            shape,
            CreatureDifficulty::Trivial,
            cap - 1,
            cap + 1
        ));
        Ok(())
    }

    #[test]
    fn progress_sweep_craft_requires_exact_inputs_and_one_complete_output_choice() -> TestResult {
        let input = NonZeroU32::new(1).ok_or("input")?;
        let a = NonZeroU32::new(2).ok_or("output a")?;
        let b = NonZeroU32::new(3).ok_or("output b")?;
        let before = BTreeMap::from([(input, 3), (a, 1)]);
        let inputs = BTreeMap::from([(input, -2)]);
        let outputs = [BTreeMap::from([(a, 1)]), BTreeMap::from([(b, 1)])];
        assert!(output_matches(
            &before,
            &BTreeMap::from([(input, 1), (a, 2)]),
            &inputs,
            &outputs
        ));
        assert!(output_matches(
            &before,
            &BTreeMap::from([(input, 1), (a, 1), (b, 1)]),
            &inputs,
            &outputs
        ));
        assert!(!output_matches(
            &before,
            &BTreeMap::from([(input, 1), (a, 2), (b, 1)]),
            &inputs,
            &outputs
        ));
        assert!(!output_matches(
            &before,
            &BTreeMap::from([(input, 2), (a, 2)]),
            &inputs,
            &outputs
        ));
        Ok(())
    }
}
