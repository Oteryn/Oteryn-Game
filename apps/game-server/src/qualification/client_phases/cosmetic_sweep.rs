//! Opt-in content census and real client appearance journeys.
//! APP-3 catalogue: crates/gameplay/dispatch/src/gameplay_transport/appearance_change/catalogue.rs:10
//! Riding: crates/gameplay/dispatch/src/gameplay_transport/appearance_change.rs:97
//! Speed forecast: crates/gameplay/dispatch/src/gameplay_transport/actor_movement.rs:186,221
//! Seed receipts: apps/game-server/migrations/0093_character_appearance.sql:69
use super::*;
use full_row_s06::{apply_appearance, change, own_binding, snapshot, wait_for_selection};
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_gameplay_characters::speed::{StepSpeedTable, player_effective_speed};
use oteryn_gameplay_quests::quest::{
    loader::{LoweredQuestState, load_embedded_quest_state},
    reward_callbacks,
    reward_callbacks::{CreationSex, ItemAmount, OutfitReward, TrackRequirement},
    world_callbacks::Storage,
};
use oteryn_protocol_oteryn::logout::LogoutResult;
use oteryn_protocol_oteryn::{actor_motion, appearance};
use serde::Deserialize;

const BATCH: &str = "OTERYN_E2E_COSMETIC_SWEEP_BATCH";
const DEFAULT: (usize, usize) = (0, 3);

pub(super) fn opted_in() -> bool {
    std::env::var("NODE_BOOT_E2E_ONLY").is_ok_and(|list| {
        list.split(',')
            .any(|group| group.trim() == "cosmetic_sweep")
    })
}

fn batch(value: Option<&str>) -> TestResult<(usize, usize)> {
    let Some(value) = value else {
        return Ok(DEFAULT);
    };
    let number = |part: &str| {
        (!part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| part.parse::<usize>().ok())
            .flatten()
    };
    value
        .split_once(':')
        .and_then(|(start, len)| Some((number(start)?, number(len)?)).filter(|&(_, len)| len > 0))
        .ok_or_else(|| format!("invalid {BATCH}: expected start:len with len > 0").into())
}

#[derive(Deserialize)]
struct Authoring {
    document: Content,
}

#[derive(Deserialize)]
struct Content {
    content_revision: String,
    outfits: Vec<Outfit>,
    mounts: Vec<Mount>,
    grant_mappings: Vec<Grant>,
    fallbacks: Fallbacks,
}

#[derive(Deserialize)]
struct Fallbacks {
    female: Fallback,
}

#[derive(Deserialize)]
struct Fallback {
    outfit_key: String,
}

#[derive(Deserialize)]
struct LookId {
    id: u32,
}

#[derive(Deserialize)]
struct Outfit {
    key: String,
    female: LookId,
    addon_count: u8,
    starter_female: bool,
    starter_addons_female: u8,
}

#[derive(Deserialize)]
struct Mount {
    key: String,
    look: LookId,
    speed_bonus: u16,
}

#[derive(Deserialize)]
struct Grant {
    grant_key: String,
    target: Target,
    compatible_provenance: Vec<String>,
    sex: Option<String>,
    #[serde(default)]
    outfit_aliases: Vec<String>,
    #[serde(default)]
    mount_aliases: Vec<String>,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(tag = "kind")]
enum Target {
    Outfit { outfit_key: String },
    Addon { outfit_key: String, addon: u8 },
    Mount { mount_key: String },
}

#[derive(Deserialize)]
struct Taming {
    item_key: String,
    creature_key: String,
    target_item_key: Option<String>,
    success_text: String,
    failures: Vec<TamingFailure>,
    #[serde(default)]
    required_items: Vec<String>,
    required_equipment: Option<String>,
    grant_key: String,
    mount_key: String,
}

#[derive(Deserialize)]
struct TamingFailure {
    text: String,
}

impl Taming {
    fn census_key(&self) -> String {
        format!("{}.{}", self.grant_key, self.item_key)
    }
}

#[derive(Clone, Copy)]
enum Entry<'a> {
    Outfit(&'a Outfit, u8),
    Mount(&'a Mount, Option<&'a Taming>),
    Unadmitted,
}

fn content(root: &Path) -> TestResult<Content> {
    Ok(
        serde_json::from_slice::<Authoring>(&std::fs::read(root.join("outfits/authoring.json"))?)?
            .document,
    )
}

fn census<'a>(
    content: &'a Content,
    taming: &'a [Taming],
    root: &Path,
) -> TestResult<BTreeMap<String, Entry<'a>>> {
    let mut entries = BTreeMap::new();
    let mut insert = |key: String, entry| -> TestResult {
        if entries.insert(key.clone(), entry).is_some() {
            return Err(format!("duplicate cosmetic census key {key}").into());
        }
        Ok(())
    };
    for outfit in &content.outfits {
        insert(outfit.key.clone(), Entry::Outfit(outfit, 0))?;
        for addon in 1..=outfit.addon_count {
            insert(
                format!("{}.addon{addon}", outfit.key),
                Entry::Outfit(outfit, addon),
            )?;
        }
    }
    for mount in &content.mounts {
        insert(mount.key.clone(), Entry::Mount(mount, None))?;
    }
    for tame in taming {
        let mount = content
            .mounts
            .iter()
            .find(|mount| mount.key == tame.mount_key);
        insert(
            tame.census_key(),
            mount.map_or(Entry::Unadmitted, |mount| Entry::Mount(mount, Some(tame))),
        )?;
    }
    // The mount authoring index also contains legacy declarations. Preserve any
    // declaration with no admitted appearance metadata as an explicit failing entry.
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("mounts/index.json"))?)?;
    for shard in index["shards"]
        .as_array()
        .ok_or("mount index missing shards")?
    {
        let path = shard
            .as_str()
            .and_then(|value| value.strip_prefix("content/cosmetics/"))
            .ok_or("invalid mount shard path")?;
        let data: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join(path))?)?;
        for record in data["records"]
            .as_array()
            .ok_or("mount shard missing records")?
        {
            let key = record["declaration"]["identity"]["key"]
                .as_str()
                .ok_or("invalid mount key")?;
            entries.entry(key.to_owned()).or_insert(Entry::Unadmitted);
        }
    }
    Ok(entries)
}

fn target(outfit: &Outfit, addon: u8) -> Target {
    if addon == 0 {
        Target::Outfit {
            outfit_key: outfit.key.clone(),
        }
    } else {
        Target::Addon {
            outfit_key: outfit.key.clone(),
            addon,
        }
    }
}

fn addon_mask(addon: u8) -> u8 {
    if addon == 0 { 0 } else { 1 << (addon - 1) }
}

impl Grant {
    fn applies_to(&self, target: &Target) -> bool {
        self.sex.as_deref().is_none_or(|sex| sex == "female")
            && (self.target == *target
                || match (&self.target, target) {
                    (Target::Outfit { .. }, Target::Outfit { outfit_key }) => {
                        self.outfit_aliases.contains(outfit_key)
                    }
                    (Target::Addon { addon: source, .. }, Target::Addon { outfit_key, addon }) => {
                        source == addon && self.outfit_aliases.contains(outfit_key)
                    }
                    (Target::Mount { .. }, Target::Mount { mount_key }) => {
                        self.mount_aliases.contains(mount_key)
                    }
                    _ => false,
                })
    }
}

fn grant<'a>(
    content: &'a Content,
    target: &Target,
    only: Option<&str>,
) -> Option<(&'a str, &'a str)> {
    content.grant_mappings.iter().find_map(|grant| {
        (grant.applies_to(target) && only.is_none_or(|key| key == grant.grant_key))
            .then(|| {
                Some((
                    grant.grant_key.as_str(),
                    grant.compatible_provenance.first()?.as_str(),
                ))
            })
            .flatten()
    })
}

// The same receipt/fact pair consumed by character_appearance/earned_unlocks.rs:31.
// Write before admission on the dedicated sweep fixture, in one statement.
async fn seed(clients: &SeamClients<'_>, key: &str, provenance: &str) -> TestResult {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    sqlx::query(
        "WITH candidate AS (
           SELECT game_character_uuid_v7() AS id, account_id, character_id
           FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid
         ), receipt AS (
           INSERT INTO game_account_appearance_unlock_receipts
             (grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id)
           SELECT id,account_id,$2,$3,character_id,'quest',uuid_send(id) FROM candidate
           WHERE NOT EXISTS (
             SELECT 1 FROM game_account_appearance_unlocks u
             WHERE u.account_id=candidate.account_id AND u.unlock_key=$2
           )
           RETURNING grant_occurrence_id,account_id,unlock_key
         )
         INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id)
         SELECT account_id,unlock_key,grant_occurrence_id FROM receipt",
    ).bind(clients.characters[0].as_slice()).bind(key).bind(provenance)
        .execute(&mut db).await?;
    Ok(())
}

struct Rows<'a> {
    key: &'a str,
    steps: &'a [&'a str],
    next: usize,
    failed: usize,
    last: std::time::Instant,
}

impl Rows<'_> {
    fn emit(&mut self, step: &str, status: &str, detail: &str, cause: &str) {
        let seconds = self.last.elapsed().as_secs_f64();
        self.last = std::time::Instant::now();
        let (mode, detail) = detail.strip_prefix("mode=").map_or(("", detail), |text| {
            let (mode, detail) = text.split_once(' ').unwrap_or((text, ""));
            (mode, detail)
        });
        let detail = detail.replace(|c: char| c.is_whitespace() || matches!(c, '=' | '|'), "_");
        let detail = if mode.is_empty() {
            detail
        } else {
            format!("mode={mode}_{detail}")
        };
        // The cloud result reader strips the .full transport suffix.
        let key = self.key.replace(':', ".");
        evidence(&format!(
            "e2e_all scenario=COS.{key}.{step}.full status={status} seconds={seconds:.3} evidence={detail} {cause}"
        ));
    }

    fn pass(&mut self, detail: &str) {
        self.emit(self.steps[self.next], "PASS", detail, "");
        self.next += 1;
    }

    fn fail(&mut self, error: &dyn std::error::Error) {
        let step = self.steps[self.next];
        let detail = error.to_string();
        let cause = if let Some((cause, _)) = detail.split_once(": ")
            && (cause.starts_with("harness-")
                || cause.starts_with("server-")
                || cause.starts_with("content-"))
        {
            cause
        } else if detail.contains("no admitted appearance definition") {
            "unadmitted-definition"
        } else if detail.contains("no authored grant mapping") {
            "missing-grant-mapping"
        } else {
            step
        };
        self.emit(step, "FAIL", &detail, &format!("cause={cause}"));
        self.next += 1;
        self.failed += 1;
    }

    fn block_rest(&mut self, error: &dyn std::error::Error) {
        let failed = self.steps[self.next];
        self.fail(error);
        while self.next < self.steps.len() {
            self.emit(
                self.steps[self.next],
                "FAIL",
                &format!("blocked-by-{failed}"),
                &format!("cause=blocked-by-{failed}"),
            );
            self.next += 1;
            self.failed += 1;
        }
    }
}

async fn release_pair(clients: &SeamClients<'_>) -> TestResult {
    let (actor, observer) = tokio::join!(
        wait_character_released(clients, 0),
        wait_character_released(clients, 1)
    );
    actor.and(observer)
}

// The first admission initializes the backpack before any fixture item is seeded.
async fn initialize_pair(clients: &SeamClients<'_>) -> TestResult {
    for index in 0..2 {
        let mut client = connect_character(clients, 0xc8, index).await?;
        logout(&mut client).await?;
    }
    release_pair(clients).await
}

pub(super) async fn run(
    fixtures: &[SeamClients<'_>],
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    if !opted_in() {
        return Ok(());
    }
    let (start, len) = match std::env::var(BATCH) {
        Ok(value) => batch(Some(&value))?,
        Err(std::env::VarError::NotPresent) => batch(None)?,
        Err(error) => return Err(error.into()),
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/cosmetics");
    let content = content(&root)?;
    let obtain = ObtainContent::load(&root.join("../.."))?;
    let taming: Vec<Taming> =
        serde_json::from_slice(&std::fs::read(root.join("taming-native.json"))?)?;
    let entries = census(&content, &taming, &root)?;
    let family = std::env::var("OTERYN_E2E_COSMETIC_SWEEP_FAMILY").unwrap_or_else(|_| "all".into());
    if !matches!(
        family.as_str(),
        "all" | "outfits" | "addons" | "mounts" | "taming"
    ) {
        return Err(format!("unknown cosmetic sweep family {family}").into());
    }
    let entries: Vec<_> = entries
        .iter()
        .filter(|(key, entry)| {
            family == "all"
                || family
                    == match entry {
                        Entry::Outfit(_, 0) => "outfits",
                        Entry::Outfit(..) => "addons",
                        Entry::Mount(_, Some(_)) => "taming",
                        Entry::Unadmitted if key.starts_with("oteryn:taming.") => "taming",
                        _ => "mounts",
                    }
        })
        .collect();
    let total = entries.len();
    evidence(&format!("cosmetic_sweep census total={total}"));
    if start >= total {
        return Err(format!("{BATCH} start {start} is beyond {total} entries").into());
    }
    let selected = len.min(total - start);
    let mut pass = 0;
    let mut fail = 0;
    let mut mounts_seen = std::collections::BTreeSet::new();
    let mut fixture_index = 0;
    let mut initialized = false;
    for (key, entry) in entries.into_iter().skip(start).take(selected) {
        // Canonical and legacy aliases can share one entitlement. Give the
        // second representation a fresh acquisition on the other fixture account.
        let entry = match *entry {
            Entry::Mount(mount, None) => {
                Entry::Mount(mount, taming_binding(&content, &taming, &mount.key))
            }
            _ => *entry,
        };
        verdict::Harness::global().progress("cosmetic-sweep-entry");
        let steps: &[&str] = match entry {
            Entry::Outfit(..) => &["unlock", "equip", "observer", "relog"],
            _ => &[
                "unlock", "equip", "observer", "mount", "speed", "unmount", "relog",
            ],
        };
        let mut rows = Rows {
            key,
            steps,
            next: 0,
            failed: 0,
            last: std::time::Instant::now(),
        };
        let result = loop {
            let Some(pair) = fixtures.get(fixture_index) else {
                break Err("harness-fixture-release: no unused fixture pairs remain".into());
            };
            if !initialized {
                if let Err(error) = initialize_pair(pair).await {
                    evidence(&format!(
                        "cosmetic_sweep fixture=retired pair={fixture_index} phase=initialization error={error}"
                    ));
                    fixture_index += 1;
                    continue;
                }
                initialized = true;
            }
            let accounts = [pair.accounts[1].clone(), pair.accounts[0].clone()];
            let characters = [pair.characters[1], pair.characters[0]];
            let alternate = SeamClients {
                accounts: &accounts,
                characters: &characters,
                ..*pair
            };
            let clients = match entry {
                Entry::Mount(_, Some(binding))
                    if mounts_seen.contains(binding.mount_key.as_str()) =>
                {
                    &alternate
                }
                _ => pair,
            };
            if let Err(error) = release_pair(clients).await {
                evidence(&format!(
                    "cosmetic_sweep fixture=retired pair={fixture_index} error={error}"
                ));
                fixture_index += 1;
                initialized = false;
                mounts_seen.clear();
                continue;
            }
            if let Entry::Mount(_, Some(binding)) = entry {
                mounts_seen.insert(binding.mount_key.as_str());
            }
            let mut items = Vec::new();
            let result = journey(
                &mut rows, clients, world, &content, &entry, &obtain, npcs, &mut items,
            )
            .await;
            let release = release_pair(clients).await;
            if let Err(error) = &release {
                evidence(&format!(
                    "cosmetic_sweep fixture=retired pair={fixture_index} error={error}"
                ));
                fixture_index += 1;
                initialized = false;
                mounts_seen.clear();
                if rows.next == 0 && fixture_index < fixtures.len() {
                    continue;
                }
            }
            let cleanup = match release {
                Ok(()) => retire_fixture_items(clients, &items).await,
                Err(error) => Err(error),
            };
            break result.and(cleanup);
        };
        match result {
            Ok(()) => rows.pass("fresh admission retained selected look, colours and unlock"),
            Err(error) => rows.block_rest(error.as_ref()),
        }
        pass += rows.next - rows.failed;
        fail += rows.failed;
    }
    evidence(&format!(
        "cosmetic_sweep summary family={family} total={total} start={start} selected={selected} rows_pass={pass} rows_fail={fail}"
    ));
    if fail != 0 {
        return Err(format!("cosmetic sweep: {fail} FAIL rows").into());
    }
    Ok(())
}

#[derive(Deserialize)]
struct NpcReward {
    #[serde(skip)]
    offer: bool,
    #[serde(default)]
    any_topic: bool,
    #[serde(default)]
    storage: Vec<Storage>,
    #[serde(default)]
    requires: Vec<TrackRequirement>,
    npc: String,
    keywords: Vec<String>,
    topic: u16,
    next_topic: u16,
    quest: Option<String>,
    #[serde(default)]
    take: Vec<ItemAmount>,
    outfit: Option<OutfitReward>,
    #[serde(default)]
    outfits: Vec<OutfitReward>,
}

struct ObtainContent {
    callbacks: Vec<NpcReward>,
    quest_state: LoweredQuestState,
    item_rules: serde_json::Value,
}

impl ObtainContent {
    fn load(root: &Path) -> TestResult<Self> {
        let quest_root = root.join("content/quests");
        // Use the production catalogue loader and the NPC owner's admitted callbacks.
        // crates/gameplay/dispatch/src/gameplay_transport/npc_command/reward_callbacks.rs:73
        let quest_state = load_embedded_quest_state(super::super::SERVED_CONTENT_REVISION)
            .map_err(|error| format!("quest catalogue: {error:?}"))?;
        let mut callbacks = reward_callbacks::offered_for(quest_state.catalogue())?
            .into_iter()
            .filter(|callback| {
                callback.carried_item.is_none()
                    && callback.world_claim.is_none()
                    && !callback.keywords.is_empty()
                    && callback.creation_sex != Some(CreationSex::Male)
            })
            .map(|callback| NpcReward {
                offer: false,
                any_topic: false,
                storage: callback.storage,
                requires: callback.requires,
                npc: callback.npc,
                keywords: callback.keywords,
                topic: callback.topic,
                next_topic: callback.next_topic,
                quest: Some(callback.quest),
                take: callback.take,
                outfit: callback.outfit,
                outfits: callback.outfits,
            })
            .collect::<Vec<_>>();
        // Server reads the same shared offers in NPC reward_callbacks/offers.rs:10.
        let mut offer_paths = vec![
            quest_root.join("shared-reward-offers.json"),
            quest_root.join("chosen-reward-offers.json"),
        ];
        for entry in std::fs::read_dir(quest_root.join("npc-offers"))? {
            let path = entry?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                offer_paths.push(path);
            }
        }
        offer_paths.sort();
        for path in offer_paths {
            let document: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
            for value in document["offers"].as_array().into_iter().flatten() {
                if let Ok(mut offer) = serde_json::from_value::<NpcReward>(value.clone())
                    && !offer.keywords.is_empty()
                {
                    offer.offer = true;
                    callbacks.push(offer);
                }
            }
        }
        let item_rules = serde_json::from_slice(&std::fs::read(
            root.join("tools/content-schema/native-gameplay/item-cosmetic-actions-native.json"),
        )?)?;
        Ok(Self {
            callbacks,
            quest_state,
            item_rules,
        })
    }

    fn npc_route(
        &self,
        target: &Target,
        npcs: &oteryn_world_bundle::npc::Table,
    ) -> Option<Vec<&NpcReward>> {
        self.npc_route_matching(target, |callback| {
            npcs.npcs
                .iter()
                .any(|npc| npc.key == callback.npc && !npc.placements.is_empty())
        })
    }

    fn npc_route_matching(
        &self,
        target: &Target,
        available: impl Fn(&NpcReward) -> bool,
    ) -> Option<Vec<&NpcReward>> {
        let mut rewards = self
            .callbacks
            .iter()
            .filter(|callback| {
                available(callback)
                    && callback
                        .outfit
                        .iter()
                        .chain(&callback.outfits)
                        .any(|reward| {
                            reward.sex != Some(CreationSex::Male)
                                && match target {
                                    Target::Outfit { outfit_key } => outfit_key == &reward.outfit,
                                    Target::Addon { outfit_key, addon } => {
                                        outfit_key == &reward.outfit
                                            && reward.addons & addon_mask(*addon) != 0
                                    }
                                    Target::Mount { .. } => false,
                                }
                        })
            })
            .collect::<Vec<_>>();
        rewards.sort_by_key(|callback| {
            callback
                .outfit
                .iter()
                .chain(&callback.outfits)
                .map(|outfit| outfit.addons.count_ones())
                .min()
                .unwrap_or(0)
        });
        rewards
            .into_iter()
            .find_map(|reward| self.dialogue_to(reward))
    }

    fn dialogue_to<'a>(&'a self, reward: &'a NpcReward) -> Option<Vec<&'a NpcReward>> {
        use std::collections::{BTreeSet, VecDeque};
        let steps = self
            .callbacks
            .iter()
            .filter(|step| step.npc == reward.npc && (step.offer || step.quest == reward.quest))
            .collect::<Vec<_>>();
        let start = (0, std::collections::BTreeMap::new());
        let mut queue = VecDeque::from([(start.clone(), Vec::new())]);
        let mut seen = BTreeSet::from([start]);
        // Bound authored graph traversal. Runtime dialogue is still the oracle.
        while let Some(((topic, tracks), path)) = queue.pop_front() {
            let value = |key: &str| {
                tracks.get(key).copied().or_else(|| {
                    self.quest_state
                        .catalogue()
                        .track(key)
                        .map(|track| track.initial)
                })
            };
            for &step in &steps {
                if (!step.any_topic && step.topic != topic)
                    || !step.storage.iter().all(|s| {
                        value(&s.track).is_some_and(|v| (s.minimum..=s.maximum).contains(&v))
                    })
                    || !step.requires.iter().all(|s| {
                        value(&s.track).is_some_and(|v| (s.minimum..=s.maximum).contains(&v))
                    })
                {
                    continue;
                }
                let mut next_path = path.clone();
                next_path.push(step);
                if std::ptr::eq(step, reward) {
                    return Some(next_path);
                }
                let mut next_tracks = tracks.clone();
                // Offer predicates only gate the prompt. The confirmed callback owns writes.
                if !step.offer {
                    for s in &step.storage {
                        let prior = value(&s.track)?;
                        let next = if s.read_only
                            || s.preserve
                            || s.set_if.is_some_and(|expected| expected != prior)
                        {
                            prior
                        } else if let Some(add) = s.add {
                            prior.saturating_add(add).min(s.set)
                        } else if s.increment {
                            prior.saturating_add(s.set)
                        } else if s.at_least {
                            prior.max(s.set)
                        } else {
                            s.set
                        };
                        if Some(next)
                            != self
                                .quest_state
                                .catalogue()
                                .track(&s.track)
                                .map(|track| track.initial)
                        {
                            next_tracks.insert(s.track.clone(), next);
                        } else {
                            next_tracks.remove(&s.track);
                        }
                    }
                }
                let next = (step.next_topic, next_tracks);
                if seen.len() < 4096 && seen.insert(next.clone()) {
                    queue.push_back((next, next_path));
                }
            }
        }
        None
    }

    fn direct_item(&self, grant: &str) -> Option<String> {
        self.item_rules["rules"]
            .as_array()?
            .iter()
            .find_map(|rule| {
                let choices = rule["choices"].as_array()?;
                let choice = choices.first()?;
                (choices.len() == 1
                    && !rule["premium"].as_bool().unwrap_or(false)
                    && choice["grants"]
                        .as_array()?
                        .iter()
                        .any(|key| key.as_str() == Some(grant)))
                .then(|| Some(format!("oteryn:item.tibia.i{}", rule["item_id"].as_u64()?)))
                .flatten()
            })
    }
}

enum ObtainAction<'a> {
    Npc(Target, Vec<&'a NpcReward>),
    Item(Target, String),
    Tame(&'a Taming),
}

impl Target {
    fn owned(&self, catalogue: &appearance::Catalogue) -> bool {
        match self {
            Self::Outfit { outfit_key } => catalogue
                .outfits
                .iter()
                .any(|outfit| outfit.key == *outfit_key),
            Self::Addon { outfit_key, addon } => catalogue.outfits.iter().any(|outfit| {
                outfit.key == *outfit_key && outfit.permitted_addons & addon_mask(*addon) != 0
            }),
            Self::Mount { mount_key } => catalogue.mounts.contains(mount_key),
        }
    }
}

async fn wait_target(
    client: &mut DevClientSession,
    state: &mut appearance::Snapshot,
    target: &Target,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::Appearance(delta) = event {
                apply_appearance(state, delta.value);
            }
        }
        if target.owned(&state.catalogue) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("server-unlock-wire: obtain action did not publish its entitlement".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn return_to(
    client: &mut DevClientSession,
    world: &BundleWorld,
    home: TilePos,
) -> TestResult {
    let here = tile(client.world_spatial().actor_position)?;
    let path =
        route(world, here, |at| at == home).ok_or("harness-obtain-path: return route absent")?;
    walk(client, world, &path).await
}

// Position is a disposable fixture prerequisite. Keep the server-owned world,
// frame, and map binding and let admission validate the seeded cell.
// apps/game-server/migrations/0080_character_last_position.sql:23,52
async fn seed_npc_position(clients: &SeamClients<'_>, at: TilePos) -> TestResult {
    wait_character_released(clients, 0).await?;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let changed = sqlx::query(
        "UPDATE game_character_last_positions
         SET tile_x=$3, tile_y=$4, tile_floor=$5, write_sequence=write_sequence+1
         WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid",
    )
    .bind(clients.characters[0].as_slice())
    .bind(clients.world.as_slice())
    .bind(i32::from(at.x))
    .bind(i32::from(at.y))
    .bind(-i16::from(at.floor))
    .execute(&mut db)
    .await?
    .rows_affected();
    if changed != 1 {
        return Err("harness-obtain-path: disposable actor has no saved position".into());
    }
    Ok(())
}

async fn obtain_npc(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    steps: &[&NpcReward],
    target: &Target,
    state: &mut appearance::Snapshot,
) -> TestResult {
    let first = steps
        .first()
        .ok_or("harness-obtain-path: NPC dialogue empty")?;
    let npc = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == first.npc)
        .ok_or_else(|| format!("harness-obtain-path: NPC {} absent from fixture", first.npc))?;
    let home = tile(client.world_spatial().actor_position)?;
    let greeting_range = |at: TilePos| {
        npc.placements.iter().any(|placement| {
            placement.floor == at.floor
                && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 2
        })
    };
    if let Some(path) = route(world, home, greeting_range) {
        walk(client, world, &path).await?;
        talk_for_reward(client, npc, steps, target, state).await?;
        return return_to(client, world, home).await;
    }
    let (landing, path) = nearby_npc_approach(world, npc, greeting_range).ok_or_else(|| {
        format!(
            "content-npc-map: {} has no enterable tile in greeting range in this map bundle",
            npc.key
        )
    })?;
    logout(client).await?;
    evidence(&format!(
        "cosmetic_sweep prerequisite mode=seeded source=saved-position npc={}",
        first.npc
    ));
    let result = async {
        seed_npc_position(clients, landing).await?;
        let mut nearby = connect_character(clients, 0xc9, 0).await?;
        let dialogue = async {
            if tile(nearby.world_spatial().actor_position)? != landing {
                return Err("harness-obtain-path: admission refused seeded NPC position".into());
            }
            walk(&mut nearby, world, &path).await?;
            let mut nearby_state = snapshot(&mut nearby)?;
            talk_for_reward(&mut nearby, npc, steps, target, &mut nearby_state).await
        }
        .await;
        dialogue.and(logout(&mut nearby).await)
    }
    .await;
    let restored = async {
        seed_npc_position(clients, home).await?;
        *client = connect_character(clients, 0xc9, 0).await?;
        *state = snapshot(client)?;
        Ok(())
    }
    .await;
    result.and(restored)
}

fn nearby_npc_approach(
    world: &BundleWorld,
    npc: &oteryn_world_bundle::npc::Npc,
    greeting_range: impl Fn(TilePos) -> bool,
) -> Option<(TilePos, Vec<StepDirection>)> {
    let mut greeting_tile = None;
    for placement in &npc.placements {
        let distance = |at: TilePos| at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y));
        for radius in [3i16, 4, 5, 6, 7, 8, 2, 1] {
            for (dx, dy) in
                (-radius..=radius).flat_map(|dx| (-radius..=radius).map(move |dy| (dx, dy)))
            {
                if dx.abs().max(dy.abs()) != radius {
                    continue;
                }
                let (Some(x), Some(y)) = (
                    placement.x.checked_add_signed(dx),
                    placement.y.checked_add_signed(dy),
                ) else {
                    continue;
                };
                let landing = TilePos {
                    x,
                    y,
                    floor: placement.floor,
                };
                if world.base().tile(x, y, landing.floor).is_none() || !world.enterable(landing) {
                    continue;
                }
                if let Ok(path) = find_route(landing, &greeting_range, |at, direction| {
                    let next = destination_with_teleports(world, at, direction, false)?;
                    (next.floor == placement.floor && distance(next) <= 8).then_some(next)
                }) {
                    if !path.is_empty() {
                        return Some((landing, path));
                    }
                    // A small enclosed room can have only an in-range landing.
                    greeting_tile.get_or_insert(landing);
                }
            }
        }
    }
    greeting_tile.map(|landing| (landing, Vec::new()))
}

async fn talk_for_reward(
    client: &mut DevClientSession,
    npc: &oteryn_world_bundle::npc::Npc,
    steps: &[&NpcReward],
    target: &Target,
    state: &mut appearance::Snapshot,
) -> TestResult {
    use oteryn_dev_client::{client_runtime::npc_shop, client_ui::npc_shop as ui};
    let basis = spatial_basis::current(client)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let entity = loop {
        if let Some(entity) = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entity| {
                    entity.kind == oteryn_dev_client::EntityKind::Npc
                        && npc.placements.iter().any(|placement| {
                            entity.position.x == i32::from(placement.x)
                                && entity.position.y == i32::from(placement.y)
                                && entity.position.floor == basis.floor(placement.floor)
                        })
                })
            })
            .map(|entry| entry.entity)
        {
            break entity;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "content-npc-map: {} absent from wire at authored placement",
                npc.key
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    };
    for text in std::iter::once("hi").chain(
        steps
            .iter()
            .filter_map(|step| step.keywords.first().map(String::as_str)),
    ) {
        let result = npc_shop::talk(
            client.session_mut(),
            &ui::npc_service::NpcTalkIntent {
                npc_actor: entity,
                text: text.into(),
            },
        )
        .await?;
        if result != ui::NpcIntentDisposition::Ok {
            return Err(format!("server-npc-dialogue: {text} returned {result:?}").into());
        }
    }
    wait_target(client, state, target).await
}

#[allow(clippy::too_many_arguments)]
async fn prepare<'a>(
    clients: &SeamClients<'_>,
    content: &Content,
    outfit: &Outfit,
    addon: u8,
    mount: Option<(&Mount, Option<&'a Taming>)>,
    obtain: &'a ObtainContent,
    npcs: &oteryn_world_bundle::npc::Table,
    items: &mut Vec<Vec<u8>>,
) -> TestResult<(String, Vec<ObtainAction<'a>>)> {
    wait_character_released(clients, 0).await?;
    let mut targets = Vec::new();
    if !outfit.starter_female {
        targets.push(target(outfit, 0));
    }
    if addon != 0 && outfit.starter_addons_female & addon_mask(addon) == 0 {
        targets.push(target(outfit, addon));
    }
    if let Some((mount, None)) = mount {
        targets.push(Target::Mount {
            mount_key: mount.key.clone(),
        });
    }
    let mut seeded = Vec::new();
    let mut actions = Vec::new();
    for target in targets {
        if let Some(steps) = obtain.npc_route(&target, npcs) {
            for item in steps.iter().flat_map(|step| &step.take) {
                seed_item(clients, &item.item, i64::from(item.count), items).await?;
            }
            actions.push(ObtainAction::Npc(target, steps));
            continue;
        }
        let candidates = content
            .grant_mappings
            .iter()
            .filter(|grant| grant.applies_to(&target));
        if let Some(item) = candidates
            .clone()
            .find_map(|grant| obtain.direct_item(&grant.grant_key))
        {
            seed_item(clients, &item, 1, items).await?;
            actions.push(ObtainAction::Item(target, item));
            continue;
        }
        if let Some(steps) = obtain.npc_route_matching(&target, |_| true)
            && let Some(donor) = steps.first()
        {
            return Err(format!(
                "content-npc-map: {} has no placement in this map bundle",
                donor.npc
            )
            .into());
        }
        let (key, provenance) = grant(content, &target, None)
            .ok_or("content cosmetic has no authored grant mapping")?;
        let reason = if key.contains("/quest-grant")
            || key.contains("/npc-grant")
            || key.contains("/quest/")
        {
            "quest-journey-unavailable-in-fixture"
        } else {
            "no-supported-direct-obtain-path"
        };
        seed(clients, key, provenance).await?;
        seeded.push(format!("{key}[{reason}]"));
    }
    if let Some((_, Some(binding))) = mount {
        seed_taming_prerequisites(clients, binding, items).await?;
        actions.push(ObtainAction::Tame(binding));
    }
    let mode = if !seeded.is_empty() {
        format!("mode=seeded grants:{}", seeded.join(","))
    } else if actions.is_empty() {
        "mode=starter".into()
    } else {
        "mode=wire prerequisites=seeded-items-and-optional-spawn".into()
    };
    Ok((mode, actions))
}

fn taming_binding<'a>(
    content: &Content,
    bindings: &'a [Taming],
    mount: &str,
) -> Option<&'a Taming> {
    let target = Target::Mount {
        mount_key: mount.into(),
    };
    bindings
        .iter()
        .find(|binding| binding.mount_key == mount)
        .or_else(|| {
            bindings
                .iter()
                .find(|binding| grant(content, &target, Some(&binding.grant_key)).is_some())
        })
}

// Disposable inventory setup uses the same migration-owned fixture approach as
// gameplay_transport/postgres_tests/support/item_look_fixtures.rs:592.
fn fixture_stack_limit(key: &str) -> TestResult<i64> {
    use crate::content::{ReferenceItemField::Known, native_gameplay::ItemProfilesDocument};
    use std::sync::OnceLock;

    static LIMITS: OnceLock<Result<BTreeMap<String, u16>, String>> = OnceLock::new();
    let limits = LIMITS.get_or_init(|| {
        let input = native_input().map_err(|error| error.to_string())?;
        let Some(item_profiles) = &input.item_profiles else {
            return Ok(BTreeMap::new());
        };
        let document: ItemProfilesDocument =
            serde_json::from_slice(&item_profiles.bytes).map_err(|error| error.to_string())?;
        Ok(document
            .records
            .into_iter()
            .filter_map(|record| {
                if let Known(stack) = record.semantics.stack
                    && stack.stackable == Known(true)
                    && let Known(limit @ 1..) = stack.stack_max
                {
                    return Some((record.production_definition.production_key, limit));
                }
                None
            })
            .collect())
    });
    Ok(i64::from(
        *limits
            .as_ref()
            .map_err(Clone::clone)?
            .get(key)
            .unwrap_or(&1),
    ))
}

async fn seed_item(
    clients: &SeamClients<'_>,
    key: &str,
    count: i64,
    items: &mut Vec<Vec<u8>>,
) -> TestResult {
    let index = provisioning::item_index()?;
    let revision = index
        .revision(key)
        .ok_or("content cosmetic prerequisite item absent")?;
    let stack_limit = if count > 1 {
        fixture_stack_limit(key)?
    } else {
        1
    };
    let stack_count = (count + stack_limit - 1) / stack_limit;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let mut tx = sqlx::Connection::begin(&mut db).await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    let placed: Vec<Vec<u8>> = sqlx::query_scalar(
        "WITH backpack AS (
            SELECT character_id, world_id, item_instance_id
            FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid
        ), minted AS (
            INSERT INTO game_item_instances(item_instance_id, world_id, definition_family,
                definition_production_key, definition_revision_ref, quantity, lifecycle, minted_transaction_id)
            SELECT game_character_uuid_v7(), world_id, 'Item', $2, $3,
                LEAST($5, $4 - stack_n * $5), 1, game_character_uuid_v7()
            FROM backpack CROSS JOIN generate_series(0, $6::bigint - 1) AS stack_n
            RETURNING item_instance_id, world_id, minted_transaction_id
        )
        INSERT INTO game_item_container_entries(character_id, world_id, parent_item_instance_id,
            item_instance_id, placement_ordinal, placed_transaction_id)
        SELECT b.character_id, m.world_id, b.item_instance_id, m.item_instance_id,
            COALESCE((SELECT max(e.placement_ordinal) FROM game_item_container_entries e
                WHERE e.parent_item_instance_id=b.item_instance_id), 0) + row_number() OVER (),
            m.minted_transaction_id
        FROM minted m CROSS JOIN backpack b
        RETURNING uuid_send(item_instance_id)",
    ).bind(clients.characters[0].as_slice()).bind(key).bind(revision).bind(count).bind(stack_limit).bind(stack_count)
        .fetch_all(&mut *tx).await?;
    if placed.len() != usize::try_from(stack_count)? {
        return Err(format!(
            "harness-item-fixture: expected {stack_count} item instances, placed {}",
            placed.len()
        )
        .into());
    }
    tx.commit().await?;
    items.extend(placed);
    Ok(())
}

async fn seed_taming_prerequisites(
    clients: &SeamClients<'_>,
    binding: &Taming,
    items: &mut Vec<Vec<u8>>,
) -> TestResult {
    if binding.required_items.is_empty() {
        let attempts = if binding.creature_key.is_empty() {
            1
        } else {
            12
        };
        seed_item(clients, &binding.item_key, attempts, items).await?;
    } else {
        for key in &binding.required_items {
            seed_item(clients, key, 1, items).await?;
        }
    }
    if let Some(key) = &binding.required_equipment {
        seed_item(clients, key, 1, items).await?;
    }
    Ok(())
}

// Keep the disposable backpack below the protocol inventory cap. Retire only
// this row's remaining fixture items after the ordinary session has released.
async fn retire_fixture_items(clients: &SeamClients<'_>, items: &[Vec<u8>]) -> TestResult {
    if items.is_empty() {
        return Ok(());
    }
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let mut tx = sqlx::Connection::begin(&mut db).await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "WITH backpack AS (
            DELETE FROM game_item_container_entries
            WHERE character_id=encode($1,'hex')::uuid AND uuid_send(item_instance_id)=ANY($2::bytea[])
            RETURNING item_instance_id
        ), equipment AS (
            DELETE FROM game_character_equipment_slots
            WHERE character_id=encode($1,'hex')::uuid AND uuid_send(item_instance_id)=ANY($2::bytea[])
            RETURNING item_instance_id
        )
        UPDATE game_item_instances SET quantity=0, lifecycle=2, last_transaction_id=game_character_uuid_v7()
        WHERE item_instance_id IN (
            SELECT item_instance_id FROM backpack UNION ALL SELECT item_instance_id FROM equipment
        )",
    ).bind(clients.characters[0].as_slice()).bind(items).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

fn item_ref(key: &str) -> TestResult<std::num::NonZeroU32> {
    let index = provisioning::item_index()?;
    index
        .revision(key)
        .and_then(|revision| index.definition_ref("Item", key, revision))
        .ok_or_else(|| format!("content item definition absent: {key}").into())
}

fn carried_handle(
    client: &DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<oteryn_protocol_oteryn::world_object::UseTarget> {
    client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == reference)
        })
        .map(|item| oteryn_protocol_oteryn::world_object::UseTarget::Item(item.handle))
        .ok_or_else(|| "harness-item-custody: seeded item absent from wire inventory".into())
}

async fn use_cosmetic_item(client: &mut DevClientSession, key: &str) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{self, UseDisposition, UseIntent};
    let payload = world_object::encode_item_use_intent(&UseIntent {
        target: carried_handle(client, item_ref(key)?)?,
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|error| format!("harness-cosmetic-item: {error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let (outcome, _) = world_object::decode_use_result_with_gate(&reply.payload)
        .map_err(|error| format!("server-cosmetic-item: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || outcome != UseDisposition::Committed {
        return Err(format!("server-cosmetic-item: {:?}/{outcome:?}", reply.status).into());
    }
    Ok(())
}

async fn obtain_taming(
    client: &mut DevClientSession,
    world: &BundleWorld,
    binding: &Taming,
    state: &mut appearance::Snapshot,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{self, UseDisposition, UseIntent, UseTarget};
    if state.catalogue.mounts.contains(&binding.mount_key) {
        return Err("harness-taming-fixture: mount already unlocked on sweep account".into());
    }
    let reference = item_ref(&binding.item_key)?;
    if let Some(key) = &binding.required_equipment {
        let equipment = item_ref(key)?;
        if !client.inventory().is_some_and(|inventory| {
            inventory
                .equipment
                .iter()
                .any(|slot| slot.item.item_definition_ref == equipment)
        }) {
            let result = client
                .session_mut()
                .equip_item_definition(equipment)
                .await?;
            if result.status != CommandStatus::Accepted
                || result.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
            {
                return Err(format!(
                    "server-taming-equipment: {key} equip returned {:?}/{:?}",
                    result.status, result.outcome
                )
                .into());
            }
        }
    }
    let home = tile(client.world_spatial().actor_position)?;
    if !binding.creature_key.is_empty() {
        creature_sweep::leave_protection_zone(client, world).await?;
    } else if binding.target_item_key.as_deref() != Some(binding.item_key.as_str()) {
        return Err("harness-obtain-path: construction needs multiple carried components".into());
    }
    let mut previous = None;
    for _ in 0..12 {
        verdict::Harness::global().progress("cosmetic-taming-attempt");
        let target = if binding.creature_key.is_empty() {
            None
        } else if let Some(entity) = previous
            && client
                .world_entities()
                .is_some_and(|view| view.get(&entity).is_some())
        {
            Some(entity)
        } else {
            let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
            let reply = request_spawn(session, binding.creature_key.clone())
                .await
                .map_err(|error| format!("harness-taming-spawn: {error}"))?;
            let entity = parse_spawned_entity(&reply)?;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            while !client
                .world_entities()
                .is_some_and(|view| view.get(&entity).is_some())
            {
                if tokio::time::Instant::now() >= deadline {
                    return Err(
                        "harness-taming-spawn: spawned creature never became visible".into(),
                    );
                }
                client.service_liveness(Duration::from_millis(50)).await?;
            }
            Some(entity)
        };
        previous = target;
        client.take_events();
        let payload = world_object::encode_item_use_intent(&UseIntent {
            target: if binding.required_items.len() > 1 {
                carried_handle(client, reference)?
            } else {
                UseTarget::ItemByDefinition(reference)
            },
            use_with: target,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(|error| format!("harness-taming-encode: {error:?}"))?;
        let reply = client.session_mut().world_context_use(&payload).await?;
        let (outcome, _) = world_object::decode_use_result_with_gate(&reply.payload)
            .map_err(|error| format!("server-taming-reply: {error:?}"))?;
        if reply.status != CommandStatus::Accepted || outcome != UseDisposition::Committed {
            return Err(format!("server-taming-use: {:?}/{outcome:?}", reply.status).into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        let mut success = false;
        let mut feedback = false;
        loop {
            for event in client.take_events() {
                match event {
                    SessionEvent::Appearance(delta) => apply_appearance(state, delta.value),
                    SessionEvent::ChatLine(delta) => {
                        if let oteryn_dev_client::ChatLine::Private { speaker_name, text } =
                            delta.value
                            && speaker_name == "Game"
                        {
                            feedback |= binding.failures.iter().any(|failure| failure.text == text);
                            success |= text == binding.success_text;
                        }
                    }
                    _ => {}
                }
            }
            let removed = target.is_none_or(|entity| {
                client
                    .world_entities()
                    .is_some_and(|view| view.get(&entity).is_none())
            });
            if success && removed && state.catalogue.mounts.contains(&binding.mount_key) {
                return_to(client, world, home).await?;
                return Ok(());
            }
            if feedback && !success {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(
                    "server-taming-outcome: committed use lacked expected feedback or unlock delta"
                        .into(),
                );
            }
            client.service_liveness(Duration::from_millis(50)).await?;
        }
    }
    Err("taming-roll-budget: exhausted twelve committed attempts".into())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let (result, _, _, _) = logout_retrying_busy(client).await?;
    if result != Some(LogoutResult::Accepted) {
        return Err(format!("logout was refused: {result:?}").into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn journey(
    rows: &mut Rows<'_>,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    content: &Content,
    entry: &Entry<'_>,
    obtain: &ObtainContent,
    npcs: &oteryn_world_bundle::npc::Table,
    items: &mut Vec<Vec<u8>>,
) -> TestResult {
    let (outfit, addon, mount) = match entry {
        Entry::Outfit(outfit, addon) => (*outfit, *addon, None),
        Entry::Mount(mount, tame) => {
            let fallback = content
                .outfits
                .iter()
                .find(|outfit| outfit.key == content.fallbacks.female.outfit_key)
                .ok_or("content fallback outfit missing")?;
            (fallback, 0, Some((*mount, *tame)))
        }
        Entry::Unadmitted => {
            return Err("no admitted appearance definition for this content key".into());
        }
    };
    let (mode, actions) =
        prepare(clients, content, outfit, addon, mount, obtain, npcs, items).await?;
    let mut client = connect_character(clients, 0xc9, 0).await?;
    let home = tile(client.world_spatial().actor_position)?;
    let mut observer = None;
    let result = async {
        let mut state = snapshot(&mut client)?;
        for action in actions {
            match action {
                ObtainAction::Npc(target, steps) => {
                    if !target.owned(&state.catalogue) {
                        obtain_npc(
                            &mut client,
                            clients,
                            world,
                            npcs,
                            &steps,
                            &target,
                            &mut state,
                        )
                        .await?;
                    }
                }
                ObtainAction::Item(target, key) => {
                    if !target.owned(&state.catalogue) {
                        use_cosmetic_item(&mut client, &key).await?;
                        wait_target(&mut client, &mut state, &target).await?;
                    }
                }
                ObtainAction::Tame(binding) => {
                    obtain_taming(&mut client, world, binding, &mut state).await?
                }
            }
        }
        let mount_key = mount.map(|(mount, _)| mount.key.as_str());
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            for event in client.take_events() {
                if let SessionEvent::Appearance(delta) = event {
                    apply_appearance(&mut state, delta.value);
                }
            }
            if has_unlock(&state.catalogue, outfit, addon_mask(addon), mount_key) {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "server-unlock-wire: {mode} expected {} addon {addon} mount {mount_key:?}",
                    outfit.key
                )
                .into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
        }
        exercise(
            rows,
            &mut client,
            &mut observer,
            clients,
            world,
            content,
            outfit,
            addon,
            mount,
            &mode,
            state,
        )
        .await
    }
    .await;
    // Acquisition can fail after a walk. Return even on failure so the next
    // entry starts beside its observer rather than outside the temple.
    let actor_return = return_to(&mut client, world, home).await;
    let observer_release = if let Some((mut observer, home)) = observer {
        let back = return_to(&mut observer, world, home).await;
        back.and(logout(&mut observer).await)
    } else {
        Ok(())
    };
    let release = logout(&mut client).await;
    result?;
    actor_return?;
    observer_release?;
    release
}

fn has_unlock(
    catalogue: &appearance::Catalogue,
    outfit: &Outfit,
    addons: u8,
    mount: Option<&str>,
) -> bool {
    catalogue.outfits.iter().any(|option| {
        option.key == outfit.key
            && option.look_type == outfit.female.id
            && option.permitted_addons & addons == addons
    }) && mount.is_none_or(|mount| catalogue.mounts.iter().any(|key| key == mount))
}

#[allow(
    clippy::too_many_arguments,
    reason = "one journey binds distinct fixture, content, and wire state"
)]
async fn exercise(
    rows: &mut Rows<'_>,
    client: &mut DevClientSession,
    observer: &mut Option<(DevClientSession, TilePos)>,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    content: &Content,
    outfit: &Outfit,
    addon: u8,
    mount: Option<(&Mount, Option<&Taming>)>,
    mode: &str,
    mut state: appearance::Snapshot,
) -> TestResult {
    let mount_key = mount.map(|(mount, _)| mount.key.as_str());
    let addons = addon_mask(addon);
    rows.pass(mode);
    let peer = connect_character(clients, 0xca, 1).await?;
    let peer_home = tile(peer.world_spatial().actor_position)?;
    *observer = Some((peer, peer_home));
    let (observer, _) = observer.as_mut().ok_or("observer fixture missing")?;
    let actor_position = tile(client.world_spatial().actor_position)?;
    let near = |at: TilePos| {
        at.floor == actor_position.floor
            && at
                .x
                .abs_diff(actor_position.x)
                .max(at.y.abs_diff(actor_position.y))
                <= 3
    };
    if !near(peer_home) {
        let path = route(world, peer_home, near)
            .ok_or("harness-observer-route: no route into actor sight range")?;
        walk(observer, world, &path).await?;
    }
    let mut observer_state = snapshot(observer)?;
    let previous_colours = state
        .catalogue
        .selected
        .as_ref()
        .map(|s| s.selection.colours);
    let colours = if previous_colours == Some([11, 22, 33, 44]) {
        [45, 34, 23, 12]
    } else {
        [11, 22, 33, 44]
    };
    let selection = appearance::Selection {
        outfit_key: outfit.key.clone(),
        colours,
        addons,
        mount_key: mount_key.map(str::to_owned),
    };
    let expected = appearance::Look::Outfit {
        look_type: outfit.female.id,
        colours,
        addons,
    };
    let request = appearance::Change {
        context: state.catalogue.context,
        expected_root_revision: state.catalogue.root_revision,
        prior: state.catalogue.selected.clone(),
        selection: selection.clone(),
        content_revision: content.content_revision.clone(),
    };
    let (status, result, _) = change(client, &request).await?;
    let (CommandStatus::Accepted, appearance::ChangeResult::Committed(revision)) = (status, result)
    else {
        return Err(format!("appearance change refused: {status:?} {result:?}").into());
    };
    wait_for_selection(client, &mut state, revision, &selection).await?;
    let binding = own_binding(client, &state)?;
    if binding.look != expected {
        return Err(format!(
            "self look mismatch: expected {expected:?}, got {:?}",
            binding.look
        )
        .into());
    }
    let entity = binding.entity;
    let selection_revision = binding.selection_revision;
    rows.pass("own wire binding matches authored look, colour and addon");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in observer.take_events() {
            if let SessionEvent::Appearance(delta) = event {
                apply_appearance(&mut observer_state, delta.value);
            }
        }
        if observer
            .world_entities()
            .is_some_and(|entities| entities.get(&entity).is_some())
            && observer_state.bindings.iter().any(|binding| {
                binding.entity == entity
                    && binding.look == expected
                    && binding.selection_revision == selection_revision
            })
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("observer did not receive target entity with the selected look".into());
        }
        observer.service_liveness(Duration::from_millis(25)).await?;
    }
    rows.pass("second client received changed look for same entity");
    if let Some((mount, _)) = mount {
        let (_, level, _) = creature_sweep::read_stats(client).await?;
        ride(client, true).await?;
        let (motion, position) = wait_motion(client, entity, mount.look.id).await?;
        wait_motion(observer, entity, mount.look.id).await?;
        rows.pass("both clients received the authored mounted look");
        match check_speed(world, position, &motion, level, mount.speed_bonus) {
            Ok(()) => rows.pass("wire prewalk forecasts match content speed bonus"),
            Err(error) => rows.fail(error.as_ref()),
        }
        ride(client, false).await?;
        let (motion, position) = wait_motion(client, entity, 0).await?;
        wait_motion(observer, entity, 0).await?;
        check_speed(world, position, &motion, level, 0)?;
        rows.pass("both clients cleared mount look and base forecast returned");
    }
    let previous = client.join_snapshot().game_session_id;
    logout(client).await?;
    *client = connect_character(clients, 0xcb, 0).await?;
    let persisted = snapshot(client)?;
    let binding = own_binding(client, &persisted)?;
    if client.join_snapshot().game_session_id == previous
        || persisted
            .catalogue
            .selected
            .as_ref()
            .map(|selected| &selected.selection)
            != Some(&selection)
        || binding.look != expected
        || binding.selection_revision != selection_revision
        || !has_unlock(&persisted.catalogue, outfit, addons, mount_key)
    {
        return Err("fresh admission did not preserve selection, look, colours and unlock".into());
    }
    Ok(())
}

async fn ride(client: &mut DevClientSession, riding: bool) -> TestResult {
    let reply = client
        .session_mut()
        .gated_command(
            actor_motion::CAPABILITY_ACTOR_MOTION_V1,
            actor_motion::COMMAND_TYPE_MOUNT,
            &actor_motion::encode_mount(riding),
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "mount command riding={riding} was refused: {:?}",
            reply.status
        )
        .into());
    }
    Ok(())
}

async fn wait_motion(
    client: &mut DevClientSession,
    entity: appearance::EntityRef,
    look: u32,
) -> TestResult<(actor_motion::ActorMotion, TilePos)> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(entry) = client
            .world_entities()
            .and_then(|entities| entities.get(&entity))
            && let Some(motion) = entry.motion
            && motion.mount_look_type == look
        {
            return Ok((motion, tile(entry.position)?));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("wire motion did not show mount look {look}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

fn forecast(level: u32, bonus: u16, ground: u16) -> Option<u32> {
    let speed = player_effective_speed(level, 0, i64::from(bonus));
    let duration = StepSpeedTable::embedded()?.step_duration(speed, ground)?;
    u32::try_from(duration.as_millis()).ok()
}

fn check_speed(
    world: &BundleWorld,
    position: TilePos,
    motion: &actor_motion::ActorMotion,
    level: u32,
    bonus: u16,
) -> TestResult {
    let mut checked = false;
    for (&ms, (dx, dy)) in motion.prewalk_ms[..4]
        .iter()
        .zip([(0, -1), (1, 0), (0, 1), (-1, 0)])
    {
        if ms == 0 {
            continue;
        }
        let x = position
            .x
            .checked_add_signed(dx)
            .ok_or("forecast x outside map")?;
        let y = position
            .y
            .checked_add_signed(dy)
            .ok_or("forecast y outside map")?;
        let ground = world.ground_speed(TilePos {
            x,
            y,
            floor: position.floor,
        });
        let expected = forecast(level, bonus, ground);
        if expected != Some(ms) {
            return Err(format!("forecast mismatch observed {ms}ms, expected {expected:?}ms at ground {ground}, level {level}, bonus {bonus}").into());
        }
        checked = true;
    }
    if !checked {
        return Err("no nonzero cardinal speed forecast on the wire".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assassin_journey_uses_shared_reward_and_requires_npc_placement() -> TestResult {
        use oteryn_world_bundle::npc::{Direction, Npc, Placement, Table};

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let obtain = ObtainContent::load(&root)?;
        let target = Target::Outfit {
            outfit_key: "oteryn:outfit.assassin".into(),
        };
        assert!(obtain.npc_route(&target, &Table::default()).is_none());
        let npcs = Table {
            npcs: vec![Npc {
                key: "oteryn:npc.vescu".into(),
                placements: vec![Placement {
                    floor: 7,
                    x: 1,
                    y: 1,
                    direction: Direction::North,
                }],
            }],
        };
        let steps = obtain
            .npc_route(&target, &npcs)
            .ok_or("missing Vescu reward route")?;
        let words = steps
            .iter()
            .map(|step| step.keywords[0].as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            words,
            [
                "sober",
                "potion",
                "yes",
                "yes",
                "30 bonelord eyes",
                "yes",
                "10 red dragon scales",
                "yes",
                "30 lizard scales",
                "yes",
                "20 fish fins",
                "yes",
                "20 vampire dust",
                "yes",
                "10 demon dust",
                "yes",
                "warrior's sweat",
                "yes",
                "secret",
            ]
        );
        Ok(())
    }

    #[test]
    fn legacy_mount_alias_uses_the_canonical_taming_grant() -> TestResult {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/cosmetics");
        let mut cosmetics = content(&root)?;
        cosmetics.grant_mappings = vec![serde_json::from_value(serde_json::json!({
            "grant_key": "oteryn:taming.black_sheep",
            "target": {"kind": "Mount", "mount_key": "oteryn:content.mount.black_sheep"},
            "compatible_provenance": ["mount-taming-v1"],
            "mount_aliases": ["oteryn:content.mount.black_sheep_mount"]
        }))?];
        let old_key = "oteryn:content.mount.black_sheep_mount";
        let target = Target::Mount {
            mount_key: old_key.into(),
        };
        assert_eq!(
            grant(&cosmetics, &target, None),
            Some(("oteryn:taming.black_sheep", "mount-taming-v1"))
        );
        let bindings: Vec<Taming> =
            serde_json::from_slice(&std::fs::read(root.join("taming-native.json"))?)?;
        let binding = taming_binding(&cosmetics, &bindings, old_key)
            .ok_or("legacy mount did not resolve its real taming item binding")?;
        assert_eq!(binding.grant_key, "oteryn:taming.black_sheep");
        assert_eq!(binding.mount_key, "oteryn:content.mount.black_sheep");
        Ok(())
    }

    #[test]
    fn batch_requires_nonempty_decimal_slice() -> TestResult {
        assert_eq!(batch(None)?, (0, 3));
        assert_eq!(batch(Some("0:20"))?, (0, 20));
        for invalid in [
            "",
            "0",
            ":1",
            "1:",
            "1:0",
            "+1:2",
            " 1:2",
            "2:1:1",
            "999999999999999999999999999999:1",
        ] {
            assert!(batch(Some(invalid)).is_err(), "{invalid}");
        }
        Ok(())
    }

    #[test]
    fn authored_census_includes_each_addon_mount_and_taming_key() -> TestResult {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/cosmetics");
        let content = content(&root)?;
        let native: Vec<Taming> =
            serde_json::from_slice(&std::fs::read(root.join("taming-native.json"))?)?;
        let source: Vec<Taming> =
            serde_json::from_slice(&std::fs::read(root.join("taming.json"))?)?;
        let entries = census(&content, &native, &root)?;
        for tame in source {
            assert!(
                native
                    .iter()
                    .any(|native| native.grant_key == tame.grant_key
                        && native.mount_key == tame.mount_key)
            );
            assert!(entries.contains_key(&tame.census_key()));
        }
        for outfit in &content.outfits {
            assert!(entries.contains_key(&outfit.key));
            for addon in 1..=outfit.addon_count {
                assert_eq!(addon_mask(addon).count_ones(), 1);
                assert!(entries.contains_key(&format!("{}.addon{addon}", outfit.key)));
            }
        }
        for mount in &content.mounts {
            assert!(entries.contains_key(&mount.key));
        }
        Ok(())
    }

    #[test]
    fn unlock_oracle_requires_authored_look_addon_and_mount() {
        let outfit = Outfit {
            key: "outfit".into(),
            female: LookId { id: 431 },
            addon_count: 2,
            starter_female: false,
            starter_addons_female: 0,
        };
        let mut catalogue = appearance::Catalogue {
            context: appearance::Context {
                content_generation: [0; 32],
                cosmetics_digest: [0; 32],
            },
            root_revision: 1,
            selected: None,
            palette: Vec::new(),
            outfits: vec![appearance::OutfitOption {
                key: outfit.key.clone(),
                look_type: 431,
                permitted_addons: 2,
            }],
            mounts: vec!["mount".into()],
        };
        assert!(has_unlock(
            &catalogue,
            &outfit,
            addon_mask(2),
            Some("mount")
        ));
        assert!(!has_unlock(
            &catalogue,
            &outfit,
            addon_mask(1),
            Some("mount")
        ));
        assert!(!has_unlock(
            &catalogue,
            &outfit,
            addon_mask(2),
            Some("other")
        ));
        catalogue.outfits[0].look_type = 430;
        assert!(!has_unlock(
            &catalogue,
            &outfit,
            addon_mask(2),
            Some("mount")
        ));
    }

    #[test]
    fn forecast_applies_content_bonus_before_table_lookup() {
        assert_eq!(forecast(100, 10, 150), forecast(110, 0, 150));
        assert!(
            forecast(100, 10, 150)
                .zip(forecast(100, 0, 150))
                .is_some_and(|(mounted, walking)| mounted < walking)
        );
        assert_eq!(forecast(100, 10, 0), None);
    }
}
