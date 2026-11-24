//! Opt-in quest census and real-wire journey checks.
//! Quest state evidence comes from the receipt owner (0056_character_quest_state.sql:28).
//! Track deltas use the checked transition evaluator (crates/gameplay/quests/src/quest/mod.rs:775).
//! Journal expectations use the same projection as the server (crates/gameplay/quests/src/quest/log.rs:346).
//! Fixed chest reward placement and counts come from world_reward_claims.rs:119 and
//! the once-only claim owner (0012_reward_claim_backpack_mint.sql:18).
use super::*;
use crate::content::world_reward_claims::{self, BoundPlacement};
use oteryn_dev_client::{DevClientSession, client_runtime::npc_shop};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, QuestTransition,
    completion_sets::CompletionSet,
    loader::load_embedded_quest_state,
    log::{QuestLogContent, QuestLogFacts, QuestLogState},
};
use oteryn_protocol_oteryn::{npc_service, world_object::UseDisposition};
use serde::Deserialize;
use serde_json::Value;

const BATCH_VARIABLE: &str = "OTERYN_E2E_QUEST_SWEEP_BATCH";
const BATCH_DEFAULT: (usize, usize) = (0, 3);
const CHARACTER: usize = 1;
const SEED_LEVEL: u32 = 100;

fn quest_batch(value: Option<&str>) -> TestResult<(usize, usize)> {
    let Some(text) = value else {
        return Ok(BATCH_DEFAULT);
    };
    let number = |part: &str| {
        (!part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| part.parse::<usize>().ok())
            .flatten()
    };
    match text
        .split_once(':')
        .and_then(|(start, len)| Some((number(start)?, number(len)?)))
    {
        Some((start, len)) if len > 0 => Ok((start, len)),
        _ => Err(format!("invalid {BATCH_VARIABLE}: expected start:len with len > 0").into()),
    }
}

#[derive(Default)]
struct Tally {
    pass: usize,
    fail: usize,
    blocked: usize,
}

impl Tally {
    fn blocked(&mut self, key: &str, step: &str, root: &str) {
        self.blocked += 1;
        evidence(&format!(
            "quest_sweep blocked scenario=QS.{key}.{step} root=QS.{key}.{root} mode=seeded"
        ));
    }

    fn row(&mut self, key: &str, step: &str, result: Result<(), &str>, seconds: f64) {
        let (status, detail) = match result {
            Ok(()) => {
                self.pass += 1;
                ("PASS", "wire-and-owner-oracles")
            }
            Err(cause) => {
                self.fail += 1;
                ("FAIL", cause)
            }
        };
        let cause = result
            .err()
            .map_or_else(String::new, |cause| format!(" cause={cause}"));
        evidence(&format!(
            "e2e_all scenario=QS.{key}.{step} status={status} seconds={seconds:.3} evidence=mode=seeded,{detail}{cause}"
        ));
    }
}

#[derive(Default)]
struct Quest {
    claims: Vec<String>,
    stages: Vec<Stage>,
    recipe_stages: Vec<String>,
}

enum Entry {
    Quest(Quest),
    ClassicReply {
        npc: String,
        branch: String,
        reply: Vec<String>,
    },
}

#[derive(Deserialize)]
struct Stage {
    key: String,
    completes: bool,
    trigger: Trigger,
}

#[derive(Deserialize)]
struct Trigger {
    kind: String,
    callback: Option<Value>,
    usage: Option<Value>,
    #[serde(default)]
    alternatives: Vec<Value>,
}

fn shards(root: &Path, prefix: &str) -> TestResult<Vec<Value>> {
    let mut paths = std::fs::read_dir(root)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    paths
        .into_iter()
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with(prefix))
        })
        .map(|path| Ok(serde_json::from_slice(&std::fs::read(path)?)?))
        .collect()
}

fn text(value: &Value) -> TestResult<&str> {
    value
        .as_str()
        .ok_or_else(|| "missing content string".into())
}

fn array(value: &Value) -> TestResult<&[Value]> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| "missing content array".into())
}

fn census(root: &Path) -> TestResult<BTreeMap<String, Entry>> {
    let mut quests = BTreeMap::<String, Quest>::new();
    for shard in shards(&root.join("quests/definitions"), "quests-")? {
        for record in array(&shard["records"])? {
            let definition = &record["definition"];
            let key = text(&definition["identity"]["key"])?;
            let mut quest = Quest::default();
            for claim in definition["claims"].as_array().into_iter().flatten() {
                quest.claims.push(text(&claim["key"])?.to_owned());
            }
            for stage in definition["oteryn_recipe"]["payload"]["recipe"]["stages"]
                .as_array()
                .into_iter()
                .flatten()
            {
                quest.recipe_stages.push(text(&stage["key"])?.to_owned());
            }
            if quests.insert(key.to_owned(), quest).is_some() {
                return Err(format!("duplicate quest definition {key}").into());
            }
        }
    }
    for shard in shards(&root.join("quests/authored-bindings"), "")? {
        for binding in array(&shard["quests"])? {
            let quest = quests
                .entry(text(&binding["quest"])?.to_owned())
                .or_default();
            let stages: Vec<Stage> = serde_json::from_value(binding["stages"].clone())?;
            quest.stages.extend(stages);
        }
    }
    let sets: BTreeMap<String, CompletionSet> =
        serde_json::from_slice(&std::fs::read(root.join("quests/claim-sets.json"))?)?;
    for (quest, set) in sets {
        quests.entry(quest).or_default().claims.extend(
            set.groups
                .iter()
                .flatten()
                .map(|marker| format!("oteryn:reward-claim.{}", marker.replace('/', "."))),
        );
    }
    let mut entries: BTreeMap<_, _> = quests
        .into_iter()
        .map(|(key, mut quest)| {
            quest.claims.sort();
            quest.claims.dedup();
            (key, Entry::Quest(quest))
        })
        .collect();
    let classic: Value = serde_json::from_slice(&std::fs::read(
        root.join("dialogues/classic-quest-branches.json"),
    )?)?;
    for npc in array(&classic["npcs"])? {
        let npc_key = text(&npc["npc"])?;
        for branch in array(&npc["branches"])? {
            let branch_key = text(&branch["key"])?;
            let key = format!(
                "oteryn:reply.{}.{}",
                npc_key.trim_start_matches("oteryn:npc."),
                branch_key
            );
            let reply = Entry::ClassicReply {
                npc: npc_key.to_owned(),
                branch: branch_key.to_owned(),
                reply: serde_json::from_value(branch["reply"].clone())?,
            };
            if entries.insert(key.clone(), reply).is_some() {
                return Err(format!("duplicate classic dialogue branch {key}").into());
            }
        }
    }
    Ok(entries)
}

#[derive(Deserialize)]
struct Offers {
    offers: Vec<Offer>,
}

#[derive(Deserialize)]
struct Offer {
    npc: String,
    keywords: Vec<String>,
    topic: u16,
    next_topic: u16,
}

fn load_offers(root: &Path) -> TestResult<Vec<Offer>> {
    let mut documents = shards(&root.join("quests/npc-offers"), "")?;
    for name in ["chosen-reward-offers.json", "shared-reward-offers.json"] {
        documents.push(serde_json::from_slice(&std::fs::read(
            root.join("quests").join(name),
        )?)?);
    }
    let mut offers = Vec::new();
    for document in documents {
        let document: Offers = serde_json::from_value(document)?;
        offers.extend(document.offers);
    }
    Ok(offers)
}

// The conversation owner matches an offer's current topic before advancing it
// (dispatch/gameplay_transport/npc_command/reward_callbacks.rs).
fn offer_route<'a>(offers: &'a [Offer], npc: &str, target: u16) -> Option<Vec<&'a str>> {
    let mut queue = VecDeque::from([(0, Vec::new())]);
    let mut visited = std::collections::BTreeSet::from([0]);
    while let Some((topic, messages)) = queue.pop_front() {
        if topic == target {
            return Some(messages);
        }
        for offer in offers
            .iter()
            .filter(|offer| offer.npc == npc && offer.topic == topic)
        {
            if let Some(keyword) = offer.keywords.first()
                && visited.insert(offer.next_topic)
            {
                let mut next = messages.clone();
                next.push(keyword.as_str());
                queue.push_back((offer.next_topic, next));
            }
        }
    }
    None
}

#[derive(Default)]
struct Facts {
    state: Option<(String, bool)>,
    tracks: BTreeMap<String, i64>,
}

impl QuestLogFacts for Facts {
    fn track_value(&self, track: &str) -> Option<i64> {
        self.tracks.get(track).copied()
    }

    fn quest_state(&self, _: &str) -> Option<QuestLogState<'_>> {
        self.state
            .as_ref()
            .map(|(revision, completed)| QuestLogState {
                pinned_content_revision: revision,
                completed: *completed,
            })
    }
}

struct Journey<'a, 'scope> {
    clients: &'a SeamClients<'scope>,
    world: &'a BundleWorld,
    npcs: &'a oteryn_world_bundle::npc::Table,
    offers: &'a [Offer],
    db: &'a mut sqlx::PgConnection,
    character: &'a [u8],
    log: &'a QuestLogContent,
    catalogue: &'a QuestStateCatalogue,
    items: &'a crate::content::item_ref::ItemDefinitionIndex,
    client: &'a mut DevClientSession,
    reconnect: bool,
}

type Receipt = (bool, Vec<String>, Vec<i64>, Vec<i64>, i64);

impl Journey<'_, '_> {
    async fn receipts(&mut self, quest: &str, transition: &str) -> TestResult<Vec<Receipt>> {
        Ok(sqlx::query_as(
            "SELECT completes,track_keys,values_before,values_after,committed_at FROM game_character_quest_receipts
             WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND transition_key=$3
             ORDER BY committed_character_revision DESC LIMIT 2",
        )
        .bind(self.character)
        .bind(quest)
        .bind(transition)
        .fetch_all(&mut *self.db)
        .await?)
    }

    async fn tracks(&mut self, quest: &str) -> TestResult<BTreeMap<String, i64>> {
        Ok(sqlx::query_as::<_, (String, i64)>(
            "SELECT track_key,value FROM game_character_quest_tracks
             WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 ORDER BY track_key LIMIT 512",
        )
        .bind(self.character)
        .bind(quest)
        .fetch_all(&mut *self.db)
        .await?
        .into_iter()
        .collect())
    }

    async fn journal(
        &mut self,
    ) -> TestResult<Vec<oteryn_protocol_oteryn::quest_log::QuestListEntry>> {
        use oteryn_dev_client::client_runtime::quest_log as wire;
        let command = oteryn_protocol_oteryn::quest_log::encode_quest_log_query(
            &wire::QuestLogQuery::ListOnly,
        )
        .map_err(|error| format!("quest-list-command-{error:?}"))?;
        let reply = wire::send(self.client.session_mut(), &command).await?;
        if reply.status != CommandStatus::Accepted {
            return Err("quest-list-refused".into());
        }
        match wire::decode_quest_log(&reply.payload)
            .map_err(|error| format!("quest-list-payload-{error:?}"))?
            .view
        {
            Some(wire::QuestLogView::List(list)) => Ok(list),
            _ => Err("quest-list-view-missing".into()),
        }
    }

    async fn check_log(&mut self, quest: &str) -> TestResult {
        let Some(index) = self
            .log
            .active()
            .quests()
            .iter()
            .position(|entry| entry.key == quest)
        else {
            self.journal().await?;
            return Ok(());
        };
        let entry = &self.log.active().quests()[index];
        let state = sqlx::query_as(
            "SELECT pinned_content_revision,completed_character_revision IS NOT NULL
             FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2",
        )
        .bind(self.character)
        .bind(quest)
        .fetch_optional(&mut *self.db)
        .await?;
        let mut facts = Facts {
            state,
            tracks: BTreeMap::new(),
        };
        if let Some((track, _)) = &entry.start {
            let value: Option<i64> = sqlx::query_scalar(
                "SELECT value FROM game_character_quest_tracks
                 WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
            )
            .bind(self.character)
            .bind(track)
            .fetch_optional(&mut *self.db)
            .await?;
            facts
                .tracks
                .extend(value.map(|value| (track.clone(), value)));
        }
        let index = u32::try_from(index)?;
        let expected = self
            .log
            .list(&facts)
            .into_iter()
            .find(|entry| entry.index == index)
            .map(|entry| entry.completed);
        let actual = self
            .journal()
            .await?
            .into_iter()
            .find(|entry| entry.quest_index == index)
            .map(|entry| entry.completed);
        if actual != expected {
            return Err("quest-log-state-mismatch".into());
        }
        Ok(())
    }

    async fn approach(&mut self, target: TilePos, distance: u16) -> TestResult {
        let from = tile(self.client.world_spatial().actor_position)?;
        let path = route(self.world, from, |at| {
            at.floor == target.floor
                && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= distance
        })
        .ok_or("no-wire-route-in-served-bundle")?;
        walk(self.client, self.world, &path).await
    }

    async fn use_object(&mut self, placement: u64) -> TestResult<UseDisposition> {
        let mut key = self.world.base().digest().to_vec();
        key.extend_from_slice(&placement.to_be_bytes());
        Ok(self
            .client
            .session_mut()
            .use_object(&key, 0)
            .await?
            .disposition)
    }

    async fn use_terrain_tool(&mut self, placement: u64, tools: &[Value]) -> TestResult {
        use oteryn_protocol_oteryn::world_object as wire;
        let inventory = self
            .client
            .inventory()
            .ok_or("missing-inventory-snapshot")?;
        let mut handle = None;
        for tool in tools {
            let reference =
                provisioning::item_reference(self.items, item_id(text(&tool["key"])?)?)?;
            handle = inventory
                .entries
                .iter()
                .chain(inventory.equipment.iter().map(|entry| &entry.item))
                .find(|item| item.item_definition_ref == reference)
                .map(|item| item.handle);
            if handle.is_some() {
                break;
            }
        }
        let mut key = self.world.base().digest().to_vec();
        key.extend_from_slice(&placement.to_be_bytes());
        let intent = wire::UseIntent {
            target: wire::UseTarget::Item(handle.ok_or("fixture-terrain-tool-missing")?),
            use_with: None,
            use_with_map_item: Some(wire::MapUseTarget::Object(wire::WorldObjectTarget {
                placement: key,
                expected_revision: 0,
            })),
            use_with_position: None,
        };
        let payload =
            wire::encode_item_use_intent(&intent).map_err(|error| format!("{error:?}"))?;
        let result = self
            .client
            .session_mut()
            .world_context_use(&payload)
            .await?;
        let disposition =
            wire::decode_use_result(&result.payload).map_err(|error| format!("{error:?}"))?;
        if result.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
            return Err(format!("terrain-tool-refused-{:?}-{disposition:?}", result.status).into());
        }
        Ok(())
    }

    async fn npc_actor(&mut self, npc_key: &str) -> TestResult<oteryn_dev_client::EntityRef> {
        let npc = self
            .npcs
            .npcs
            .iter()
            .find(|npc| npc.key == npc_key)
            .ok_or("npc-not-in-served-bundle")?;
        let from = tile(self.client.world_spatial().actor_position)?;
        let placement = npc
            .placements
            .iter()
            .min_by_key(|p| {
                (
                    i16::from(p.floor).abs_diff(i16::from(from.floor)),
                    p.x.abs_diff(from.x).max(p.y.abs_diff(from.y)),
                )
            })
            .ok_or("npc-has-no-placement")?;
        let target = TilePos {
            x: placement.x,
            y: placement.y,
            floor: placement.floor,
        };
        self.approach(target, 3).await?;
        self.client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entity| {
                    entity.kind == oteryn_dev_client::EntityKind::Npc
                        && entity.position.x == i32::from(target.x)
                        && entity.position.y == i32::from(target.y)
                        && entity.position.floor == -i16::from(target.floor)
                })
            })
            .map(|entity| entity.entity)
            .ok_or_else(|| "npc-not-visible".into())
    }

    async fn talk(
        &mut self,
        actor: oteryn_dev_client::EntityRef,
        message: &str,
    ) -> TestResult<Vec<String>> {
        let result = npc_shop::talk(
            self.client.session_mut(),
            &npc_service::NpcTalkIntent {
                npc_actor: actor,
                text: message.to_owned(),
            },
        )
        .await?;
        if result != npc_service::NpcIntentDisposition::Ok {
            return Err("npc-talk-refused".into());
        }
        let delta = self
            .client
            .session_mut()
            .read_gated_delta(npc_service::STATE_DOMAIN_NPC_CONVERSATION, 1)
            .await?;
        let conversation = npc_service::decode_npc_conversation(&delta.payload)
            .map_err(|error| format!("npc-conversation-{error:?}"))?;
        Ok(conversation.lines)
    }

    async fn classic(&mut self, npc: &str, branch: &str, expected: &[String]) -> TestResult {
        let messages = classic_input(npc, branch).ok_or("classic-branch-input-unmapped")?;
        let actor = self.npc_actor(npc).await?;
        self.talk(actor, "hi").await?;
        let mut lines = Vec::new();
        for message in messages {
            lines = self.talk(actor, message).await?;
        }
        if expected.is_empty()
            || !lines
                .windows(expected.len())
                .any(|window| window == expected)
        {
            return Err("classic-reply-mismatch".into());
        }
        Ok(())
    }

    async fn outfit_receipts(&mut self, keys: &[String]) -> TestResult<BTreeMap<String, i64>> {
        if keys.is_empty() {
            return Ok(BTreeMap::new());
        }
        Ok(sqlx::query_as::<_, (String, i64)>(
            "SELECT r.unlock_key, count(*) FROM game_account_appearance_unlock_receipts r
             JOIN game_account_appearance_unlocks u ON u.account_id = r.account_id
                 AND u.unlock_key = r.unlock_key
             WHERE r.account_id = (SELECT account_id FROM game_character_roots
                 WHERE character_id = encode($1, 'hex')::uuid)
                 AND r.unlock_key = ANY($2) AND r.source_kind = 'quest'
             GROUP BY r.unlock_key",
        )
        .bind(self.character)
        .bind(keys)
        .fetch_all(&mut *self.db)
        .await?
        .into_iter()
        .collect())
    }

    async fn stage(&mut self, quest: &str, stage: &Stage, callback: &Value) -> TestResult {
        let transition = text(&callback["key"])?;
        let definition = self
            .catalogue
            .transition(transition)
            .ok_or("transition-not-admitted")?;
        if definition.quest != quest || definition.completes != stage.completes {
            return Err("authored-transition-mismatch".into());
        }
        let before = self.receipts(quest, transition).await?;
        if !before.is_empty() {
            return Err("stage-already-driven".into());
        }
        let before_tracks = self.tracks(quest).await?;
        let item_delta = stage_item_delta(self.items, callback);
        let outfit_keys = outfit_grant_keys(callback)?;
        let before_outfits = self.outfit_receipts(&outfit_keys).await?;
        if !before_outfits.is_empty() {
            return Err("fixture-outfit-already-granted".into());
        }
        let before_items = inventory(self.client)?;
        let before_xp = if definition.experience.is_some() {
            Some(experience(self.client).await?)
        } else {
            None
        };
        let replay = match stage.trigger.kind.as_str() {
            "npc_dialogue" => {
                let npc = text(&callback["npc"])?;
                let actor = self.npc_actor(npc).await?;
                let topic = u16::try_from(callback["topic"].as_u64().ok_or("missing-npc-topic")?)?;
                let route =
                    offer_route(self.offers, npc, topic).ok_or("npc-topic-route-missing")?;
                let keyword = array(&callback["keywords"])?
                    .first()
                    .ok_or("missing-npc-keyword")?;
                self.talk(actor, "hi").await?;
                for message in route {
                    self.talk(actor, message).await?;
                }
                let keyword = text(keyword)?;
                let observed = self.talk(actor, keyword).await?;
                let expected: Vec<String> = serde_json::from_value(callback["reply"].clone())?;
                if expected.is_empty()
                    || !observed
                        .windows(expected.len())
                        .any(|window| window == expected)
                {
                    return Err("authored-reply-mismatch".into());
                }
                Some(Replay::Talk(actor, keyword))
            }
            "terrain_use" => {
                let placement = callback["placement_key"]
                    .as_u64()
                    .ok_or("missing-terrain-placement")?;
                let (target, _) =
                    crate::map::view::placement_of(placement).ok_or("invalid-terrain-placement")?;
                self.approach(target, 1).await?;
                self.use_terrain_tool(placement, array(&callback["tools"])?)
                    .await?;
                None
            }
            "map_use" | "step_in" => {
                let selector = &callback["selector"];
                let position = &selector["position"];
                let target = TilePos {
                    x: u16::try_from(position["x"].as_u64().ok_or("missing-target-x")?)?,
                    y: u16::try_from(position["y"].as_u64().ok_or("missing-target-y")?)?,
                    floor: i8::try_from(position["floor"].as_i64().ok_or("missing-target-floor")?)?,
                };
                if stage.trigger.kind == "step_in" {
                    if tile(self.client.world_spatial().actor_position)? == target {
                        return Err("step-in-already-on-target".into());
                    }
                    self.approach(target, 0).await?;
                    None
                } else {
                    let ordinal = target_ordinal(self.world, target, selector)?;
                    let placement = crate::map::view::placement_key(target, ordinal)
                        .ok_or("invalid-target-placement")?;
                    self.approach(target, 1).await?;
                    if self.use_object(placement).await? != UseDisposition::Committed {
                        return Err("map-use-refused".into());
                    }
                    Some(Replay::Use(placement))
                }
            }
            _ => return Err("unsupported-wire-trigger".into()),
        };
        let receipts = self.receipts(quest, transition).await?;
        if !matches!(receipts.as_slice(), [receipt] if receipt_matches(receipt, self.catalogue, definition, &before_tracks))
        {
            return Err("expected-stage-receipt-missing".into());
        }
        self.check_log(quest).await?;
        let item_delta = item_delta?;
        let xp_delta = definition.experience;
        if item_delta.is_empty() && xp_delta.is_none() && outfit_keys.is_empty() {
            return Ok(());
        }
        let after_outfits = self.outfit_receipts(&outfit_keys).await?;
        if outfit_keys
            .iter()
            .any(|key| after_outfits.get(key) != Some(&1))
        {
            return Err("exact-quest-outfit-reward-missing".into());
        }
        let after_items = wait_inventory(self.client, &before_items, &item_delta).await?;
        let after_xp = match before_xp.zip(xp_delta) {
            Some((before, delta)) => {
                let actual = experience(self.client).await?;
                if actual.checked_sub(before) != Some(u64::try_from(delta)?) {
                    return Err("exact-quest-xp-reward-missing".into());
                }
                Some(actual)
            }
            None => None,
        };
        let refused = match replay.ok_or("reward-trigger-replay-unsupported")? {
            Replay::Use(placement) => {
                self.use_object(placement).await? == UseDisposition::NothingToUse
            }
            Replay::Talk(actor, keyword) => {
                npc_shop::talk(
                    self.client.session_mut(),
                    &npc_service::NpcTalkIntent {
                        npc_actor: actor,
                        text: keyword.to_owned(),
                    },
                )
                .await?
                    != npc_service::NpcIntentDisposition::Ok
            }
        };
        self.check_log(quest).await?;
        if !refused
            || self.outfit_receipts(&outfit_keys).await? != after_outfits
            || inventory(self.client)? != after_items
            || self.receipts(quest, transition).await? != receipts
        {
            return Err("duplicate-authored-reward-not-refused".into());
        }
        if let Some(after) = after_xp
            && experience(self.client).await? != after
        {
            return Err("duplicate-quest-xp-awarded".into());
        }
        Ok(())
    }

    // As in postgres_tests/grizzly_durability.rs:277, only predecessor facts
    // use the disposable fixture connection. Admission and all final-step and
    // reward writes run with the node's normal owners and triggers enabled.
    async fn seed_predecessors(&mut self, quest: &str, stages: &[Stage]) -> TestResult {
        let current = self.tracks(quest).await?;
        let mut expected = current.clone();
        let now = now_seconds()?;
        for stage in stages {
            let Some(callback) = &stage.trigger.callback else {
                continue;
            };
            let key = text(&callback["key"])?;
            let Some(transition) = self.catalogue.transition(key) else {
                continue;
            };
            if transition.quest != quest
                || transition.completes
                || !self.receipts(quest, key).await?.is_empty()
            {
                continue;
            }
            if let Ok(changes) = self.catalogue.evaluate(transition, &expected, now) {
                expected.extend(
                    changes
                        .into_iter()
                        .map(|change| (change.track, change.after)),
                );
            }
        }
        expected.retain(|key, value| current.get(key) != Some(value));
        if expected.is_empty() {
            return Err("no-authored-predecessors-to-seed".into());
        }
        let (result, _, _, _) = logout_retrying_busy(self.client).await?;
        if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err("seed-logout-refused".into());
        }
        wait_character_released(self.clients, CHARACTER).await?;
        let result = self.write_predecessor_tracks(quest, &expected).await;
        *self.client = connect_character(self.clients, 0x72, CHARACTER).await?;
        result?;
        evidence(&format!(
            "quest_sweep prerequisites mode=seeded quest={} seeded_tracks={}",
            row_key(quest),
            expected.len()
        ));
        Ok(())
    }

    async fn write_predecessor_tracks(
        &mut self,
        quest: &str,
        expected: &BTreeMap<String, i64>,
    ) -> TestResult {
        let mut tx = self.db.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        for (track, value) in expected {
            let changed = sqlx::query(
                "INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
                 SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots
                 WHERE character_id=encode($1,'hex')::uuid
                 ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,
                 committed_character_revision=EXCLUDED.committed_character_revision
                 WHERE game_character_quest_tracks.quest_key=EXCLUDED.quest_key",
            )
            .bind(self.character)
            .bind(track)
            .bind(quest)
            .bind(value)
            .execute(&mut *tx)
            .await?;
            if changed.rows_affected() != 1 {
                return Err("fixture-predecessor-write-refused".into());
            }
        }
        tx.commit().await?;
        Ok(())
    }

    async fn claimed(&mut self, claim: &str) -> TestResult<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_reward_claims WHERE character_id=encode($1,'hex')::uuid
             AND claim_family='RewardClaim' AND claim_production_key=$2)",
        )
        .bind(self.character)
        .bind(claim)
        .fetch_one(&mut *self.db)
        .await?)
    }

    async fn chest(&mut self, quest: &str, claim: &str, chest: &BoundPlacement) -> TestResult {
        if chest.reward_callback.is_some() || !chest.contents.is_empty() {
            return Err("unsupported-chest-reward-shape".into());
        }
        let [reward] = chest.placement.items.as_slice() else {
            return Err("unsupported-chest-reward-shape".into());
        };
        let reference =
            provisioning::item_reference(self.items, item_id(reward.item.key().as_str())?)?.get();
        let target = TilePos {
            x: chest.cell.x,
            y: chest.cell.y,
            floor: chest.cell.floor,
        };
        self.approach(target, 1).await?;
        if self.claimed(claim).await? {
            return Err("shared-claim-already-consumed".into());
        }
        let before_tracks = self.tracks(quest).await?;
        let before = inventory(self.client)?;
        if self.use_object(chest.bundle_placement_key).await? != UseDisposition::Committed {
            return Err("first-chest-use-refused".into());
        }
        let delta = BTreeMap::from([(reference, i64::from(reward.count))]);
        let after = wait_inventory(self.client, &before, &delta).await?;
        if !self.claimed(claim).await? {
            return Err("durable-claim-missing".into());
        }
        let transition = chest.placement.quest_transition.as_deref();
        if let Some(transition) = transition {
            let expected = self
                .catalogue
                .transition(transition)
                .ok_or("chest-transition-not-admitted")?;
            let receipts = self.receipts(quest, transition).await?;
            if !matches!(receipts.as_slice(), [receipt] if receipt_matches(receipt, self.catalogue, expected, &before_tracks))
            {
                return Err("expected-chest-transition-missing".into());
            }
        }
        self.check_log(quest).await?;
        if self.use_object(chest.bundle_placement_key).await? != UseDisposition::NothingToUse
            || inventory(self.client)? != after
            || !self.claimed(claim).await?
        {
            return Err("duplicate-chest-claim-not-refused".into());
        }
        if let Some(transition) = transition
            && self.receipts(quest, transition).await?.len() != 1
        {
            return Err("duplicate-chest-transition".into());
        }
        self.check_log(quest).await
    }
}

fn target_ordinal(world: &BundleWorld, at: TilePos, selector: &Value) -> TestResult<u8> {
    if let Some(ordinal) = selector["ordinal"]
        .as_u64()
        .or(selector["native"]["ordinal"].as_u64())
    {
        return Ok(u8::try_from(ordinal)?);
    }
    let tile = world
        .base()
        .tile(at.x, at.y, at.floor)
        .ok_or("target-tile-outside-served-bundle")?;
    let appearance = u16::try_from(
        selector["appearance"]
            .as_u64()
            .ok_or("missing-target-appearance")?,
    )?;
    let unique = selector["unique"].as_u64();
    let mut candidates = (0..=u8::MAX).filter(|&ordinal| {
        tile.appearance(ordinal) == Some(appearance)
            && unique.is_none_or(|expected| {
                tile.unique(ordinal)
                    .is_some_and(|entry| u64::from(entry.unique) == expected)
            })
    });
    let first = candidates.next().ok_or("target-placement-absent")?;
    if candidates.next().is_some() {
        return Err("target-placement-ambiguous".into());
    }
    Ok(first)
}

// Fresh-state inputs from quest/classic_talk.rs:218, :338, :381, :400 and :452.
fn classic_input(npc: &str, branch: &str) -> Option<&'static [&'static str]> {
    use oteryn_gameplay_quests::quest::classic_talk::{JEROM, MORRIS, QUEEN, ROTTIN, WOBLIN};
    Some(match (npc, branch) {
        (MORRIS, "line-79") => &["amulet"],
        (MORRIS, "line-98") => &["log book"],
        (MORRIS, "line-120") => &["herbs"],
        (MORRIS, "line-140") => &["key"],
        (MORRIS, "line-154") => &["amulet", "yes"],
        (MORRIS, "line-164") => &["log book", "yes"],
        (MORRIS, "line-173") => &["herbs", "yes"],
        (MORRIS, "line-184") => &["key", "yes"],
        (MORRIS, "line-194") => &["trolls", "yes"],
        (MORRIS, "line-204") => &["goblins", "yes"],
        (MORRIS, "line-214") => &["minotaur", "yes"],
        (MORRIS, "line-228") => &["trolls"],
        (MORRIS, "line-246") => &["goblins"],
        (MORRIS, "line-264") => &["minotaur"],
        (WOBLIN, "line-65") => &["precious"],
        (JEROM, "line-69") => &["mission"],
        (JEROM, "line-82") => &["mission", "yes"],
        (QUEEN, "line-63") => &["seventh seal"],
        (QUEEN, "line-94") => &["seventh seal", "yes"],
        (QUEEN, "line-172") => &["seventh seal", "no"],
        (ROTTIN, "line-91") => &["mission"],
        (ROTTIN, "line-149") => &["mission", "yes"],
        (ROTTIN, "line-297") => &["mission", "no"],
        (ROTTIN, "line-336") => &["task"],
        _ => return None,
    })
}

fn item_id(key: &str) -> TestResult<u32> {
    Ok(key
        .strip_prefix("oteryn:item.tibia.i")
        .ok_or("unsupported-reward-item-key")?
        .parse()?)
}

fn inventory(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let inventory = client.inventory().ok_or("missing-inventory-snapshot")?;
    let mut counts = BTreeMap::new();
    for item in inventory
        .main_backpack
        .iter()
        .chain(&inventory.entries)
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
    {
        *counts.entry(item.item_definition_ref.get()).or_default() += u64::from(item.count.get());
    }
    Ok(counts)
}

enum Replay<'a> {
    Use(u64),
    Talk(oteryn_dev_client::EntityRef, &'a str),
}

fn authored_item(callback: &Value) -> Option<&Value> {
    [&callback["give"], &callback["effects"]["item_reward"]]
        .into_iter()
        .find(|value| !value.is_null())
}

fn outfit_grant_keys(callback: &Value) -> TestResult<Vec<String>> {
    use oteryn_gameplay_quests::quest::reward_callbacks::OutfitReward;
    [&callback["outfit"], &callback["effects"]["outfit_reward"]]
        .into_iter()
        .filter(|reward| !reward.is_null())
        .map(|reward| serde_json::from_value::<OutfitReward>(reward.clone()))
        .collect::<Result<Vec<_>, _>>()
        .map(|rewards| {
            rewards
                .into_iter()
                .flat_map(|reward| {
                    (0..=2).filter_map(move |addon| {
                        (addon == 0 || reward.addons & (1 << (addon - 1)) != 0)
                            .then(|| reward.grant_key(addon))
                    })
                })
                .collect()
        })
        .map_err(Into::into)
}

fn stage_item_delta(
    items: &crate::content::item_ref::ItemDefinitionIndex,
    callback: &Value,
) -> TestResult<BTreeMap<u32, i64>> {
    if !callback["access"].is_null() || !callback["effects"]["access"].is_null() {
        return Err("access-reward-oracle-unavailable".into());
    }
    let mut expected = BTreeMap::new();
    let mut record = |value: &Value, direction: i64| -> TestResult {
        let reference = provisioning::item_reference(items, item_id(text(&value["item"])?)?)?.get();
        let count = value["count"].as_u64().ok_or("reward-item-count-missing")?;
        *expected.entry(reference).or_default() += direction * i64::from(u32::try_from(count)?);
        Ok(())
    };
    if let Some(reward) = authored_item(callback) {
        if reward["contents"]
            .as_array()
            .is_some_and(|children| !children.is_empty())
        {
            return Err("nested-reward-oracle-unavailable".into());
        }
        record(reward, 1)?;
        for item in reward["additional"].as_array().into_iter().flatten() {
            record(item, 1)?;
        }
    }
    for item in callback["take"].as_array().into_iter().flatten() {
        if authored_item(callback).is_some() {
            record(item, -1)?;
        }
    }
    Ok(expected)
}

fn exact_inventory_delta(
    before: &BTreeMap<u32, u64>,
    after: &BTreeMap<u32, u64>,
    delta: &BTreeMap<u32, i64>,
) -> bool {
    before
        .keys()
        .chain(after.keys())
        .chain(delta.keys())
        .all(|key| {
            before
                .get(key)
                .copied()
                .unwrap_or(0)
                .checked_add_signed(delta.get(key).copied().unwrap_or(0))
                == Some(after.get(key).copied().unwrap_or(0))
        })
}

async fn wait_inventory(
    client: &mut DevClientSession,
    before: &BTreeMap<u32, u64>,
    delta: &BTreeMap<u32, i64>,
) -> TestResult<BTreeMap<u32, u64>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let observed = inventory(client)?;
        if exact_inventory_delta(before, &observed, delta) {
            return Ok(observed);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("exact-item-reward-missing".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

// Quest XP uses the durable obligation settled by durability/quest_state.rs:684.
async fn experience(client: &mut DevClientSession) -> TestResult<u64> {
    use oteryn_dev_client::client_ui::cyclopedia_character as panel;
    use oteryn_protocol_oteryn::cyclopedia_character::CharacterTab;
    let mut view = panel::CyclopediaCharacterPanel {
        selected: true,
        extensions: panel::TabExtensions {
            vocation: general_stats::selected(client),
            death_details: false,
        },
        ..Default::default()
    };
    let command = panel::request(
        &mut view,
        panel::Intent::SelectTab(CharacterTab::GeneralStats),
    )
    .ok_or("experience-command-unavailable")?;
    let reply = client
        .session_mut()
        .gated_command(command.capability, command.kind, &command.payload)
        .await?;
    if reply.status != CommandStatus::Accepted || !panel::apply_result(&mut view, &reply) {
        return Err("experience-query-refused".into());
    }
    Ok(view.general.ok_or("experience-view-missing")?.experience)
}

fn receipt_matches(
    receipt: &Receipt,
    catalogue: &QuestStateCatalogue,
    transition: &QuestTransition,
    stored: &BTreeMap<String, i64>,
) -> bool {
    let (completed, keys, before, after, now) = receipt;
    if keys.len() != before.len() || keys.len() != after.len() {
        return false;
    }
    catalogue
        .evaluate(transition, stored, *now)
        .is_ok_and(|expected| {
            *completed == catalogue.completes_after_claim_sets(transition, stored, &expected)
                && expected.len() == keys.len()
                && expected.iter().zip(keys.iter().zip(before).zip(after)).all(
                    |(change, ((key, before), after))| {
                        change.track == *key && change.before == *before && change.after == *after
                    },
                )
        })
}

struct Check<'a> {
    name: String,
    action: Action<'a>,
}

enum Action<'a> {
    Stage(&'a Stage, &'a Value),
    Chest(&'a str, &'a BoundPlacement),
    Missing(&'static str),
    MissingStage(&'static str),
    Journal,
}

fn plan<'a>(quest: &'a Quest, claims: &'a BTreeMap<String, Vec<BoundPlacement>>) -> Vec<Check<'a>> {
    let mut checks = Vec::new();
    if quest.stages.is_empty() && quest.claims.is_empty() && quest.recipe_stages.is_empty() {
        checks.push(Check {
            name: "start".into(),
            action: Action::MissingStage("missing-wire-start-binding"),
        });
    }
    let mut recipe = quest.recipe_stages.iter();
    for stage in &quest.stages {
        if recipe.clone().any(|key| key == &stage.key) {
            for key in recipe.by_ref() {
                if key == &stage.key {
                    break;
                }
                checks.push(Check {
                    name: key.clone(),
                    action: Action::MissingStage("missing-authored-stage-driver"),
                });
            }
        }
        checks.push(Check {
            name: stage.key.clone(),
            action: stage
                .trigger
                .callback
                .as_ref()
                .or_else(|| {
                    (stage.trigger.kind == "terrain_use")
                        .then_some(stage.trigger.usage.as_ref())
                        .flatten()
                })
                .map_or(
                    Action::MissingStage("unsupported-wire-trigger"),
                    |callback| Action::Stage(stage, callback),
                ),
        });
        for (index, alternative) in stage.trigger.alternatives.iter().enumerate() {
            checks.push(Check {
                name: format!("{}.alternative-{}", stage.key, index + 1),
                action: Action::Stage(stage, alternative),
            });
        }
    }
    for key in recipe {
        if !quest.stages.iter().any(|stage| &stage.key == key) {
            checks.push(Check {
                name: key.clone(),
                action: Action::MissingStage("missing-authored-stage-driver"),
            });
        }
    }
    for claim in &quest.claims {
        let name = claim.strip_prefix("oteryn:").unwrap_or(claim);
        if let Some(placements) = claims.get(claim) {
            for (index, chest) in placements.iter().enumerate() {
                checks.push(Check {
                    name: format!("chest.{}.{}", row_key(name), index + 1),
                    action: Action::Chest(claim, chest),
                });
            }
        } else {
            checks.push(Check {
                name: format!("chest.{}", row_key(name)),
                action: Action::Missing("claim-not-bound-in-served-bundle"),
            });
        }
    }
    if quest.claims.is_empty()
        && !quest.stages.iter().any(|stage| {
            stage.completes
                && stage.trigger.callback.as_ref().is_some_and(|callback| {
                    authored_item(callback).is_some()
                        || outfit_grant_keys(callback).is_ok_and(|keys| !keys.is_empty())
                })
        })
    {
        checks.push(Check {
            name: "reward".into(),
            action: Action::Missing("missing-wire-reward-driver"),
        });
    }
    checks.push(Check {
        name: "journal".into(),
        action: Action::Journal,
    });
    checks
}

fn row_key(key: &str) -> String {
    key.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "._-".contains(character) {
                character
            } else {
                '-'
            }
        })
        .collect()
}

struct StepFailure {
    reason: String,
    reconnect: bool,
}

impl StepFailure {
    #[cfg(test)]
    fn blocked(reason: &'static str) -> Self {
        Self {
            reason: reason.to_owned(),
            reconnect: false,
        }
    }

    fn from_error(error: &(dyn std::error::Error + 'static)) -> Self {
        let mut source = Some(error);
        let mut reconnect = false;
        while let Some(error) = source {
            if error.is::<oteryn_dev_client::SessionError>()
                || error.is::<oteryn_dev_client::DevClientError>()
            {
                reconnect = true;
                break;
            }
            source = error.source();
        }
        Self {
            reason: row_key(&error.to_string()).chars().take(240).collect(),
            reconnect,
        }
    }
}

async fn bounded(work: impl Future<Output = TestResult>) -> Result<(), StepFailure> {
    match tokio::time::timeout(Duration::from_secs(30), work).await {
        Ok(result) => result.map_err(|error| StepFailure::from_error(error.as_ref())),
        Err(_) => Err(StepFailure {
            reason: "wire-step-deadline".into(),
            reconnect: true,
        }),
    }
}

pub(super) async fn run(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    let batch = std::env::var(BATCH_VARIABLE);
    let (start, len) = match &batch {
        Ok(text) => quest_batch(Some(text))?,
        Err(std::env::VarError::NotPresent) => quest_batch(None)?,
        Err(error) => return Err(format!("invalid {BATCH_VARIABLE}: {error}").into()),
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let entries = census(&root)?;
    let offers = load_offers(&root)?;
    let total = entries.len();
    let quests = entries
        .values()
        .filter(|entry| matches!(entry, Entry::Quest(_)))
        .count();
    evidence(&format!(
        "quest_sweep census total={total} quests={quests} classic_replies={}",
        total - quests
    ));
    if start >= total {
        return Err(format!("{BATCH_VARIABLE} start {start} is beyond the {total} entries").into());
    }
    let selected = len.min(total - start);
    let items = provisioning::item_index()?;
    let report =
        world_reward_claims::bind_embedded_reward_claims(world.base().as_ref(), &|item| {
            item_id(item.key().as_str())
                .is_ok_and(|id| provisioning::item_reference(&items, id).is_ok())
        })?;
    let claims = report
        .served
        .into_iter()
        .map(|claim| (claim.key.as_str().to_owned(), claim.placements))
        .collect::<BTreeMap<_, _>>();
    let lowered = load_embedded_quest_state(SERVED_CONTENT_REVISION)
        .map_err(|error| format!("quest state catalogue: {error:?}"))?;
    let log =
        oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?.content;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let mut tally = Tally::default();
    let client = connect_character(clients, 0x71, CHARACTER).await?;
    evidence("quest_sweep preparation mode=seeded level=100 owner=qualification_fixture.rs:50");
    let mut session =
        Some(provisioning::provision_fixture(client, clients, CHARACTER, SEED_LEVEL).await?);
    let mut isolate_entries = false;
    for (key, entry) in entries.iter().skip(start).take(selected) {
        verdict::Harness::global().progress("quest-sweep-entry");
        let short = row_key(key.strip_prefix("oteryn:").unwrap_or(key));
        if session.is_none() {
            session = Some(connect_character(clients, 0x73, CHARACTER).await?);
            evidence("quest_sweep recovery fresh_admission=accepted");
        }
        let mut journey = Journey {
            clients,
            world,
            npcs,
            offers: &offers,
            db: &mut db,
            character: &clients.characters[CHARACTER],
            log: &log,
            catalogue: lowered.catalogue(),
            items: &items,
            client: session.as_mut().ok_or("quest sweep session absent")?,
            reconnect: false,
        };
        let scenario = Box::leak(format!("QS.{short}.full").into_boxed_str());
        verdict::indexed(
            scenario,
            Box::pin(run_entry(
                &mut journey,
                key,
                &short,
                entry,
                &claims,
                &mut tally,
            )),
        )
        .await;
        if journey.reconnect {
            isolate_entries = true;
            // An abandoned exchange can leave its reply on the connection.
            // Close it before the next quest so the next admission starts at command one.
            drop(session.take());
        } else if isolate_entries && let Some(mut client) = session.take() {
            if !matches!(
                logout_retrying_busy(&mut client).await,
                Ok((
                    Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted),
                    _,
                    _,
                    _
                ))
            ) {
                tally.row(&short, "release", Err("logout-refused"), 0.0);
            }
        }
    }
    let released = if let Some(mut client) = session {
        matches!(
            logout_retrying_busy(&mut client).await,
            Ok((
                Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted),
                _,
                _,
                _
            ))
        )
    } else {
        wait_character_released(clients, CHARACTER).await.is_ok()
    };
    if !released {
        tally.row("session", "release", Err("logout-refused"), 0.0);
    }
    evidence(&format!(
        "quest_sweep summary total={total} start={start} selected={selected} rows_pass={} rows_fail={} rows_blocked={}",
        tally.pass, tally.fail, tally.blocked
    ));
    if tally.fail > 0 {
        return Err(format!("quest sweep: {} FAIL rows (see QS.*)", tally.fail).into());
    }
    Ok(())
}

async fn run_entry(
    journey: &mut Journey<'_, '_>,
    key: &str,
    short: &str,
    entry: &Entry,
    claims: &BTreeMap<String, Vec<BoundPlacement>>,
    tally: &mut Tally,
) -> TestResult {
    let entered = std::time::Instant::now();
    let mut first_failure = None;
    match entry {
        Entry::ClassicReply { npc, branch, reply } => {
            let result = bounded(journey.classic(npc, branch, reply)).await;
            tally.row(
                short,
                "dialogue",
                result
                    .as_ref()
                    .map(|_| ())
                    .map_err(|failure| failure.reason.as_str()),
                entered.elapsed().as_secs_f64(),
            );
            journey.reconnect |= result.as_ref().is_err_and(|failure| failure.reconnect);
            first_failure = result.err().map(|failure| failure.reason);
        }
        Entry::Quest(quest) => {
            let mut blocked = None;
            let mut wire_root = None;
            for check in plan(quest, claims) {
                verdict::Harness::global().progress("quest-sweep-step");
                let started = std::time::Instant::now();
                let advances = matches!(&check.action, Action::Stage(..) | Action::MissingStage(_));
                let dependency = !matches!(&check.action, Action::Missing(_));
                let seeded_final = if blocked.is_some() {
                    match &check.action {
                        Action::Stage(stage, _)
                            if stage.completes && stage.trigger.callback.is_some() =>
                        {
                            quest
                                .stages
                                .iter()
                                .position(|candidate| std::ptr::eq(candidate, *stage))
                        }
                        _ => None,
                    }
                } else {
                    None
                };
                if dependency
                    && let Some(root) = wire_root
                        .as_deref()
                        .or(blocked.as_deref().filter(|_| seeded_final.is_none()))
                {
                    tally.blocked(short, &check.name, root);
                    continue;
                }
                let result = if let (Some(index), Action::Stage(stage, callback)) =
                    (seeded_final, &check.action)
                {
                    bounded(async {
                        journey
                            .seed_predecessors(key, &quest.stages[..index])
                            .await?;
                        journey.stage(key, stage, callback).await
                    })
                    .await
                } else {
                    bounded(async {
                        match check.action {
                            Action::Stage(stage, callback) => {
                                journey.stage(key, stage, callback).await
                            }
                            Action::Chest(claim, chest) => journey.chest(key, claim, chest).await,
                            Action::Journal => journey.check_log(key).await,
                            Action::Missing(reason) | Action::MissingStage(reason) => {
                                Err(reason.into())
                            }
                        }
                    })
                    .await
                };
                if let Err(failure) = &result {
                    if advances {
                        blocked.get_or_insert_with(|| check.name.clone());
                    }
                    if failure.reconnect {
                        wire_root.get_or_insert_with(|| check.name.clone());
                        journey.reconnect = true;
                    }
                } else if seeded_final.is_some() {
                    blocked = None;
                }
                tally.row(
                    short,
                    &check.name,
                    result
                        .as_ref()
                        .map(|_| ())
                        .map_err(|failure| failure.reason.as_str()),
                    started.elapsed().as_secs_f64(),
                );
                if first_failure.is_none() {
                    first_failure = result.err().map(|failure| failure.reason);
                }
            }
        }
    }
    let elapsed = entered.elapsed().as_secs_f64();
    if let Some(cause) = first_failure {
        evidence(&format!(
            "e2e_all scenario=QS.{short}.full status=FAIL seconds={elapsed:.3} evidence=mode=seeded cause={cause}"
        ));
    } else {
        evidence(&format!(
            "e2e_all scenario=QS.{short}.full status=PASS seconds={elapsed:.3} evidence=mode=seeded"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_requires_two_decimal_fields_and_nonzero_length() -> TestResult {
        assert_eq!(quest_batch(None)?, (0, 3));
        assert_eq!(quest_batch(Some("0:20"))?, (0, 20));
        for invalid in [
            "",
            "1",
            "-1:3",
            "1:0",
            "1: 2",
            "1:2:3",
            "1:+2",
            "999999999999999999999999:3",
        ] {
            assert!(quest_batch(Some(invalid)).is_err(), "{invalid}");
        }
        Ok(())
    }

    #[test]
    fn exact_reward_rejects_duplicate_and_unrelated_deltas() {
        let before = BTreeMap::from([(1, 4)]);
        let reward = BTreeMap::from([(2, 3)]);
        assert!(exact_inventory_delta(
            &before,
            &BTreeMap::from([(1, 4), (2, 3)]),
            &reward
        ));
        assert!(!exact_inventory_delta(
            &before,
            &BTreeMap::from([(1, 4), (2, 6)]),
            &reward
        ));
        assert!(!exact_inventory_delta(
            &before,
            &BTreeMap::from([(1, 5), (2, 3)]),
            &reward
        ));
        assert!(!exact_inventory_delta(&before, &before, &reward));
    }

    #[test]
    fn receipt_requires_the_authored_track_and_completion() -> TestResult {
        let lowered = load_embedded_quest_state(SERVED_CONTENT_REVISION)
            .map_err(|error| format!("quest state catalogue: {error:?}"))?;
        let catalogue = lowered.catalogue();
        let key = "oteryn:quest-callback/authored-small-sapphire-chest";
        let transition = catalogue
            .transition(key)
            .ok_or("fixture transition missing")?;
        let track = transition
            .effects
            .first()
            .ok_or("fixture track missing")?
            .track
            .clone();
        let valid = (true, vec![track.clone()], vec![0], vec![1], 100);
        assert!(receipt_matches(
            &valid,
            catalogue,
            transition,
            &BTreeMap::new()
        ));
        assert!(!receipt_matches(
            &(false, vec![track.clone()], vec![0], vec![1], 100),
            catalogue,
            transition,
            &BTreeMap::new()
        ));
        assert!(!receipt_matches(
            &(true, vec![track], vec![0], vec![0], 100),
            catalogue,
            transition,
            &BTreeMap::new()
        ));
        Ok(())
    }

    #[test]
    fn session_failures_require_fresh_admission() {
        let error = oteryn_dev_client::DevClientError::from(
            oteryn_dev_client::SessionError::SessionUnusable,
        );
        let failure = StepFailure::from_error(&error);
        assert!(failure.reconnect);
        assert!(!StepFailure::blocked("missing-content").reconnect);
    }

    #[test]
    fn offer_route_respects_npc_and_topic_order() {
        let offer = |npc: &str, word: &str, topic, next_topic| Offer {
            npc: npc.into(),
            keywords: vec![word.into()],
            topic,
            next_topic,
        };
        let offers = [
            offer("other", "wrong", 0, 2),
            offer("target", "confirm", 1, 2),
            offer("target", "mission", 0, 1),
            offer("target", "cycle", 2, 1),
        ];
        assert_eq!(
            offer_route(&offers, "target", 2),
            Some(vec!["mission", "confirm"])
        );
        assert_eq!(offer_route(&offers, "target", 3), None);
    }

    #[test]
    fn census_keeps_authored_and_classic_entries() -> TestResult {
        let entries = census(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))?;
        assert!(matches!(
            entries.get("oteryn:reply.mr_morris.line-79"),
            Some(Entry::ClassicReply { .. })
        ));
        assert!(
            matches!(entries.get("oteryn:quest.25_years_of_tibia_quest"), Some(Entry::Quest(quest)) if quest.stages.iter().any(|stage| stage.key == "s1-return-part-25099"))
        );
        assert!(entries.contains_key("oteryn:quest.adorned_uh_rune_quest"));
        assert!(
            matches!(entries.get("oteryn:quest.bear_room_quest"), Some(Entry::Quest(quest))
                if quest.claims.iter().any(|claim| claim == "oteryn:reward-claim.kv.bear_chest1"))
        );
        Ok(())
    }
}
