//! Real node NPC content sweep. Select explicitly with NODE_BOOT_E2E_ONLY=npc_sweep.
//! The batch is a slice of the key-sorted NPC census, including unplaced definitions.
//! Placement owner: crates/gameplay/entities/src/carrier/npc_actor.rs:346.
//! Reply oracles: crates/gameplay/content/src/content/npc_catalogue/replies.rs:56 and
//! crates/gameplay/npc/src/quest_greetings.rs:142.
//! Shop projection and charge: crates/gameplay/npc/src/npc_command.rs:117 and
//! crates/gameplay/durability/src/durability/npc_trade.rs:843.
//! Travel: crates/gameplay/dispatch/src/gameplay_transport/paid_travel.rs:150.
//! Blessings and promotion: crates/gameplay/content/src/content/blessing_catalogue.rs:26 and
//! crates/gameplay/progression/src/promotion.rs:6.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef, client_runtime::npc_shop};
use oteryn_gameplay_content::content::blessing_catalogue::BlessingCatalogue;
use oteryn_protocol_oteryn::npc_service::{
    self as wire, NpcConversation, NpcConversationState, NpcIntentDisposition, NpcTradeSide,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

const BATCH_VARIABLE: &str = "OTERYN_E2E_NPC_SWEEP_BATCH";
const CHARACTER: usize = 0;
const LEVEL: u32 = 100;
const STEP_DEADLINE: Duration = Duration::from_secs(180);

fn batch(value: Option<&str>) -> TestResult<(usize, usize)> {
    let Some(text) = value else { return Ok((0, 3)) };
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

#[derive(Deserialize)]
struct Identity {
    key: String,
    revision: String,
}

#[derive(Deserialize)]
struct Definition {
    identity: Identity,
    dialogue: Option<Identity>,
    services: Vec<Identity>,
    fields: Vec<Field>,
}

#[derive(Deserialize)]
struct Field {
    field_path: String,
    value: serde_json::Value,
}

#[derive(Deserialize)]
struct Placement {
    identity: Identity,
    npc: String,
    cell: Cell,
}

#[derive(Clone, Copy, Deserialize)]
struct Cell {
    x: u16,
    y: u16,
    floor: i8,
}

impl Cell {
    fn native(self) -> TestResult<TilePos> {
        if !(0..=15).contains(&self.floor) {
            return Err("authored floor outside global coordinates".into());
        }
        Ok(TilePos {
            x: self.x,
            y: self.y,
            floor: -self.floor,
        })
    }
}

#[derive(Deserialize)]
struct Dialogue {
    identity: Identity,
    #[serde(default)]
    greet: Vec<String>,
    #[serde(default)]
    keywords: Vec<Keyword>,
}

#[derive(Deserialize)]
struct Keyword {
    key: String,
    #[serde(default)]
    triggers: Vec<String>,
    reply: Vec<String>,
    #[serde(default)]
    children: Vec<Keyword>,
    #[serde(default)]
    only_unfocus: bool,
    #[serde(default)]
    ungreet: bool,
}

#[derive(Deserialize)]
struct Service {
    identity: Identity,
    #[serde(default)]
    offers: Vec<Offer>,
    #[serde(default)]
    routes: Vec<Route>,
}

#[derive(Deserialize)]
struct Offer {
    direction: Direction,
    item: Identity,
    unit_price: u64,
    currency: Option<Identity>,
    #[serde(default = "one")]
    count: u32,
}

const fn one() -> u32 {
    1
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
enum Direction {
    BuyFromPlayer,
    SellToPlayer,
}

impl Direction {
    const fn side(self) -> NpcTradeSide {
        match self {
            Self::BuyFromPlayer => NpcTradeSide::Sell,
            Self::SellToPlayer => NpcTradeSide::Buy,
        }
    }

    const fn deltas(self, price: u64, count: u32) -> (i128, i128) {
        match self {
            Self::BuyFromPlayer => (price as i128, -(count as i128)),
            Self::SellToPlayer => (-(price as i128), count as i128),
        }
    }
}

#[derive(Deserialize)]
struct Route {
    key: String,
    destination: Cell,
    price: u64,
}

#[derive(Deserialize)]
struct Index {
    #[serde(default)]
    shards: Vec<String>,
    record_count: Option<usize>,
    population_state: Option<String>,
}

#[derive(Deserialize)]
struct Shard<T> {
    records: Vec<Record<T>>,
}

#[derive(Deserialize)]
struct Record<T> {
    declaration: T,
}

fn read_family<T: serde::de::DeserializeOwned>(root: &Path, family: &Path) -> TestResult<Vec<T>> {
    let index: Index = serde_json::from_reader(std::fs::File::open(family.join("index.json"))?)?;
    let mut records = Vec::new();
    for path in index.shards {
        let shard: Shard<T> = serde_json::from_reader(std::fs::File::open(root.join(path))?)?;
        records.extend(shard.records.into_iter().map(|row| row.declaration));
    }
    if index.record_count != Some(records.len())
        && !(records.is_empty() && index.population_state.as_deref() == Some("READY_UNPOPULATED"))
    {
        return Err(format!("NPC census disagrees with {}", family.display()).into());
    }
    Ok(records)
}

struct Census {
    npcs: BTreeMap<String, Definition>,
    placements: BTreeMap<String, Vec<Placement>>,
    dialogues: BTreeMap<String, Dialogue>,
    services: BTreeMap<String, Service>,
}

fn keyed<T>(values: Vec<T>, identity: impl Fn(&T) -> &Identity) -> TestResult<BTreeMap<String, T>> {
    let mut sorted = BTreeMap::new();
    for value in values {
        let key = identity(&value).key.clone();
        if sorted.insert(key.clone(), value).is_some() {
            return Err(format!("duplicate NPC census key {key}").into());
        }
    }
    Ok(sorted)
}

impl Census {
    fn load(root: &Path) -> TestResult<Self> {
        let npcs = keyed(
            read_family(root, &root.join("content/npcs/definitions"))?,
            |npc: &Definition| &npc.identity,
        )?;
        let dialogues = keyed(
            read_family(root, &root.join("content/dialogues/definitions"))?,
            |dialogue: &Dialogue| &dialogue.identity,
        )?;
        let mut placements: BTreeMap<String, Vec<Placement>> = BTreeMap::new();
        for placement in keyed(
            read_family(root, &root.join("content/world/npc-placements"))?,
            |placement: &Placement| &placement.identity,
        )?
        .into_values()
        {
            placements
                .entry(placement.npc.clone())
                .or_default()
                .push(placement);
        }
        let mut services = Vec::new();
        for entry in std::fs::read_dir(root.join("content/services"))? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                services.extend(read_family(root, &entry.path())?);
            }
        }
        let services = keyed(services, |service: &Service| &service.identity)?;
        let bound: BTreeSet<&str> = npcs
            .values()
            .flat_map(|npc| npc.services.iter().map(|service| service.key.as_str()))
            .collect();
        if placements.keys().any(|key| !npcs.contains_key(key))
            || services.keys().any(|key| !bound.contains(key.as_str()))
        {
            return Err("NPC census contains an unbound placement or service".into());
        }
        Ok(Self {
            npcs,
            placements,
            dialogues,
            services,
        })
    }
}

#[derive(Default)]
struct Tally {
    pass: usize,
    fail: usize,
}

struct Rows<'a> {
    npc: &'a str,
    lap: std::time::Instant,
    counts: Tally,
}

fn token(text: &str) -> String {
    text.chars()
        .take(400)
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._:-,;/".contains(c) {
                c
            } else {
                '-'
            }
        })
        .collect()
}

impl Rows<'_> {
    fn emit(&mut self, step: &str, cause: &str, result: TestResult) -> bool {
        let (status, detail, cause) = match result {
            Ok(()) => {
                self.counts.pass += 1;
                ("PASS", "verified".to_owned(), String::new())
            }
            Err(error) => {
                self.counts.fail += 1;
                ("FAIL", token(&error.to_string()), format!(" cause={cause}"))
            }
        };
        let seconds = self.lap.elapsed().as_secs_f64();
        self.lap = std::time::Instant::now();
        evidence(&format!(
            "e2e_all scenario=NS.{}.{step} status={status} seconds={seconds:.3} evidence={detail} mode=seeded{cause}",
            self.npc,
        ));
        status == "PASS"
    }
}

enum Action<'a> {
    Greet(&'a [String]),
    Keyword(Vec<&'a Keyword>),
    Missing(&'a str),
    Offer(&'a Offer, usize),
    Route(&'a Route),
    Service(&'a Service),
}

fn actions<'a>(census: &'a Census, npc: &'a Definition) -> Vec<(String, Action<'a>)> {
    let dialogue = npc.dialogue.as_ref().and_then(|reference| {
        census
            .dialogues
            .get(&reference.key)
            .filter(|row| row.identity.revision == reference.revision)
    });
    let mut steps = vec![(
        "greet".into(),
        Action::Greet(dialogue.map_or(&[], |row| row.greet.as_slice())),
    )];
    if let Some(dialogue) = dialogue {
        dialogue_steps(&dialogue.keywords, "dialogue", &mut Vec::new(), &mut steps);
    } else if let Some(reference) = &npc.dialogue {
        steps.push(("dialogue".into(), Action::Missing(&reference.key)));
    }
    let mut quote_occurrences = BTreeMap::new();
    for reference in &npc.services {
        let name = token(reference.key.trim_start_matches("oteryn:service."));
        let Some(service) = census
            .services
            .get(&reference.key)
            .filter(|row| row.identity.revision == reference.revision)
        else {
            steps.push((format!("service.{name}"), Action::Missing(&reference.key)));
            continue;
        };
        if service.offers.is_empty() && service.routes.is_empty() {
            steps.push((format!("service.{name}"), Action::Service(service)));
        }
        for (index, offer) in service.offers.iter().enumerate() {
            let side = match offer.direction.side() {
                NpcTradeSide::Buy => "buy",
                NpcTradeSide::Sell => "sell",
            };
            let occurrence = quote_occurrences
                .entry((
                    offer.item.key.as_str(),
                    offer.item.revision.as_str(),
                    offer.direction,
                    offer.unit_price,
                    offer.count,
                ))
                .or_insert(0);
            steps.push((
                format!("service.{name}.{side}.{index}"),
                Action::Offer(offer, *occurrence),
            ));
            *occurrence += 1;
        }
        for route in &service.routes {
            steps.push((
                format!("service.{name}.{}", token(&route.key)),
                Action::Route(route),
            ));
        }
    }
    steps
}

fn dialogue_steps<'a>(
    keywords: &'a [Keyword],
    prefix: &str,
    path: &mut Vec<&'a Keyword>,
    steps: &mut Vec<(String, Action<'a>)>,
) {
    for (index, keyword) in keywords.iter().enumerate() {
        let step = format!("{prefix}.{index}.{}", token(&keyword.key));
        path.push(keyword);
        steps.push((step.clone(), Action::Keyword(path.clone())));
        dialogue_steps(&keyword.children, &step, path, steps);
        path.pop();
    }
}

pub(super) async fn run(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult {
    if !std::env::var("NODE_BOOT_E2E_ONLY").is_ok_and(|list| !list.trim().is_empty()) {
        return Ok(());
    }
    let (start, len) = match std::env::var(BATCH_VARIABLE) {
        Ok(text) => batch(Some(&text))?,
        Err(std::env::VarError::NotPresent) => batch(None)?,
        Err(error) => return Err(error.into()),
    };
    let census = Census::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))?;
    let total = census.npcs.len();
    if start >= total {
        return Err(format!("{BATCH_VARIABLE} start {start} is beyond the {total} NPCs").into());
    }
    let selected = len.min(total - start);
    evidence(&format!(
        "npc_sweep census total={total} placements={} dialogues={} services={} offers={} routes={} start={start} selected={selected} mode=seeded",
        census.placements.values().map(Vec::len).sum::<usize>(),
        census.dialogues.len(),
        census.services.len(),
        census
            .services
            .values()
            .map(|service| service.offers.len())
            .sum::<usize>(),
        census
            .services
            .values()
            .map(|service| service.routes.len())
            .sum::<usize>(),
    ));
    let mut fixture = Fixture {
        clients,
        world,
        classed: false,
        client: None,
        focused: false,
    };
    let mut tally = Tally::default();
    let started = std::time::Instant::now();
    for (key, npc) in census.npcs.iter().skip(start).take(selected) {
        verdict::Harness::global().progress("npc-sweep-entry");
        let short = token(key.trim_start_matches("oteryn:npc."));
        let mut rows = Rows {
            npc: &short,
            lap: std::time::Instant::now(),
            counts: Tally::default(),
        };
        let steps = actions(&census, npc);
        if let Some(placements) = census.placements.get(key) {
            for (index, placement) in placements.iter().enumerate() {
                let prefix = format!("p{index}");
                fixture
                    .sweep_placement(&mut rows, npc, placement, &prefix, &steps)
                    .await;
            }
        } else {
            rows.emit(
                "placement",
                "missing_placement",
                Err("NPC has no authored placement".into()),
            );
            for (step, _) in &steps {
                rows.emit(step, "blocked", Err("blocked-by-placement".into()));
            }
        }
        rows.emit(
            "full",
            "entry_failed",
            if rows.counts.fail == 0 {
                Ok(())
            } else {
                Err("see NPC step rows".into())
            },
        );
        tally.pass += rows.counts.pass;
        tally.fail += rows.counts.fail;
    }
    evidence(&format!(
        "npc_sweep summary total={total} start={start} selected={selected} rows_pass={} rows_fail={} seconds={:.3} mode=seeded",
        tally.pass,
        tally.fail,
        started.elapsed().as_secs_f64(),
    ));
    if tally.fail > 0 {
        return Err(format!("NPC sweep: {} FAIL rows (see NS.*)", tally.fail).into());
    }
    Ok(())
}

async fn within_step<T>(future: impl std::future::Future<Output = TestResult<T>>) -> TestResult<T> {
    tokio::time::timeout(STEP_DEADLINE, future)
        .await
        .map_err(|_| "NPC sweep step timed out")?
}

struct Fixture<'a> {
    clients: &'a SeamClients<'a>,
    world: &'a BundleWorld,
    classed: bool,
    client: Option<DevClientSession>,
    focused: bool,
}

fn npc_name(npc: &Definition) -> String {
    npc.fields
        .iter()
        .filter(|field| field.field_path == "oteryn:source.npc.display_name")
        .find_map(|field| {
            let name = field.value.get("value")?.as_str()?;
            oteryn_protocol_oteryn::entity_names::EntityDisplayName::new(name).ok()?;
            Some(name)
        })
        .map_or_else(
            || {
                npc.identity
                    .key
                    .rsplit('.')
                    .next()
                    .unwrap_or(&npc.identity.key)
                    .split('_')
                    .filter_map(|word| {
                        let mut chars = word.chars();
                        Some(
                            chars
                                .next()?
                                .to_uppercase()
                                .chain(chars)
                                .collect::<String>(),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            },
            str::to_owned,
        )
}

fn nearby(world: &BundleWorld, at: TilePos) -> TestResult<TilePos> {
    for distance in 1_i32..=3 {
        for dx in -distance..=distance {
            for dy in -distance..=distance {
                if dx.abs().max(dy.abs()) != distance {
                    continue;
                }
                let (Ok(x), Ok(y)) = (
                    u16::try_from(i32::from(at.x) + dx),
                    u16::try_from(i32::from(at.y) + dy),
                ) else {
                    continue;
                };
                let pos = TilePos {
                    x,
                    y,
                    floor: at.floor,
                };
                if world.enterable(pos)
                    && world
                        .base()
                        .tile(x, y, at.floor)
                        .is_some_and(|tile| tile.walkable())
                {
                    return Ok(pos);
                }
            }
        }
    }
    Err("no walkable fixture tile within NPC talk range".into())
}

impl Fixture<'_> {
    async fn release(&mut self) -> TestResult {
        self.focused = false;
        if let Some(mut client) = self.client.take() {
            let (result, detail, _, _) = logout_retrying_busy(&mut client).await?;
            if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
                return Err(format!("fixture logout refused: {detail}").into());
            }
        }
        wait_character_released(self.clients, CHARACTER).await
    }

    /// Edit only this sweep's released fixture. Position retains the owner-written frame
    /// digests and generation. Item seeding follows item_look_fixtures.rs:592.
    async fn seed(&self, at: TilePos, sell: Option<&Offer>, fund: bool) -> TestResult {
        let mut database = sqlx::PgConnection::connect(self.clients.url).await?;
        let mut tx = sqlx::Connection::begin(&mut database).await?;
        let character = self.clients.characters[CHARACTER].as_slice();
        let world = self.clients.world.as_slice();
        sqlx::query("SELECT character_id FROM game_character_roots
            WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
            .bind(character).bind(world).fetch_one(&mut *tx).await?;
        let live: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions
            WHERE character_id=encode($1,'hex')::uuid AND session_state<>3)",
        )
        .bind(character)
        .fetch_one(&mut *tx)
        .await?;
        if live {
            return Err("refusing to seed an admitted NPC sweep fixture".into());
        }
        let changed = sqlx::query(
            "UPDATE game_character_last_positions SET
            tile_x=$3, tile_y=$4, tile_floor=$5, write_sequence=write_sequence+1
            WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid",
        )
        .bind(character)
        .bind(world)
        .bind(i32::from(at.x))
        .bind(i32::from(at.y))
        .bind(-i16::from(at.floor))
        .execute(&mut *tx)
        .await?;
        if changed.rows_affected() != 1 {
            return Err("fixture last position missing after owner logout".into());
        }
        if sell.is_some() || fund {
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            let stock = sell.map(|offer| {
                (
                    offer.item.key.as_str(),
                    offer.item.revision.as_str(),
                    offer.count,
                )
            });
            let gold = fund.then_some(("oteryn:item.tibia.i3043", "definition-r1", 100));
            for (key, revision, quantity) in gold.into_iter().chain(stock) {
                let inserted = sqlx::query("WITH minted AS (
                    INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,
                        definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
                    SELECT uuidv7(),world_id,'Item',$3,$4,$5,1,uuidv7()
                    FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid
                        AND world_id=encode($2,'hex')::uuid
                    RETURNING item_instance_id
                )
                INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,
                    item_instance_id,placement_ordinal,placed_transaction_id)
                SELECT s.character_id,s.world_id,s.item_instance_id,minted.item_instance_id,
                    COALESCE((SELECT max(e.placement_ordinal) FROM game_item_container_entries e
                        WHERE e.parent_item_instance_id=s.item_instance_id),0)+1,uuidv7()
                FROM game_item_container_slots s CROSS JOIN minted
                WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid")
                    .bind(character).bind(world).bind(key).bind(revision).bind(i64::from(quantity))
                    .execute(&mut *tx).await?;
                if inserted.rows_affected() != 1 {
                    return Err("fixture backpack missing after provisioning".into());
                }
            }
        }
        tx.commit().await?;
        Ok(())
    }

    async fn enter(
        &mut self,
        at: TilePos,
        sell: Option<&Offer>,
    ) -> TestResult<&mut DevClientSession> {
        let fund = !self.classed;
        if fund {
            let client = connect_character(self.clients, 0x72, CHARACTER).await?;
            self.client = Some(
                provisioning::provision_fixture(client, self.clients, CHARACTER, LEVEL).await?,
            );
        }
        self.release().await?;
        self.seed(at, sell, fund).await?;
        self.classed = true;
        let client = connect_character(self.clients, 0x72, CHARACTER).await?;
        let observed = tile(client.world_spatial().actor_position)?;
        self.client = Some(client);
        if observed != at {
            return Err(format!("fixture admission expected {at:?}, got {observed:?}").into());
        }
        self.client
            .as_mut()
            .ok_or_else(|| "fixture client missing".into())
    }

    async fn sweep_placement(
        &mut self,
        rows: &mut Rows<'_>,
        npc: &Definition,
        placement: &Placement,
        prefix: &str,
        steps: &[(String, Action<'_>)],
    ) {
        let name = npc_name(npc);
        let setup = within_step(async {
            let there = placement.cell.native()?;
            let at = nearby(self.world, there)?;
            let client = self.enter(at, None).await?;
            let actor = visible_npc(client, there, &name).await?;
            Ok((at, there, actor))
        })
        .await;
        let (at, there, actor) = match setup {
            Ok(value) => {
                rows.emit(&format!("{prefix}.placement"), "", Ok(()));
                value
            }
            Err(error) => {
                rows.emit(
                    &format!("{prefix}.placement"),
                    "placement_or_fixture",
                    Err(error),
                );
                for (step, _) in steps {
                    rows.emit(
                        &format!("{prefix}.{step}"),
                        "blocked",
                        Err("blocked-by-placement".into()),
                    );
                }
                self.finish_placement(rows, prefix).await;
                return;
            }
        };
        let mut failures: Vec<&str> = Vec::new();
        for (step, action) in steps {
            verdict::Harness::global().progress("npc-sweep-step");
            let row = format!("{prefix}.{step}");
            if let Some(root) = failures.iter().find(|root| {
                **root == "greet"
                    || step
                        .strip_prefix(**root)
                        .is_some_and(|suffix| suffix.starts_with('.'))
            }) {
                rows.emit(&row, "blocked", Err(format!("blocked-by-{root}").into()));
                continue;
            }
            let result = within_step(self.step(at, there, actor, &name, action)).await;
            if !rows.emit(&row, action.cause(), result) {
                failures.push(step);
            }
        }
        self.finish_placement(rows, prefix).await;
    }

    async fn finish_placement(&mut self, rows: &mut Rows<'_>, prefix: &str) {
        if self.client.is_some() {
            let released = within_step(self.release()).await;
            rows.emit(&format!("{prefix}.release"), "fixture_release", released);
        }
    }

    async fn step(
        &mut self,
        at: TilePos,
        there: TilePos,
        actor: EntityRef,
        name: &str,
        action: &Action<'_>,
    ) -> TestResult {
        let clients = self.clients;
        match action {
            Action::Missing(key) => Err(format!("missing or stale authored binding {key}").into()),
            Action::Greet(expected) => {
                let client = self.client.as_mut().ok_or("NPC fixture client missing")?;
                let reply = talk(client, actor, "hi").await?;
                if reply.open != Some((actor, NpcConversationState::Open)) {
                    return Err("NPC greeting did not open the expected actor conversation".into());
                }
                if expected.is_empty() {
                    check_reply(&reply, &[format!("Greetings, I am {name}.")], "")?;
                } else {
                    check_reply(&reply, expected, &player_name(clients).await?)?;
                }
                self.focused = true;
                Ok(())
            }
            Action::Keyword(path) => {
                let keyword = path.last().ok_or("authored dialogue path is empty")?;
                let client = self.client.as_mut().ok_or("NPC fixture client missing")?;
                if self.focused && (keyword.only_unfocus || path.len() > 1) {
                    talk(client, actor, "bye").await?;
                    self.focused = false;
                }
                if !self.focused && !keyword.only_unfocus {
                    talk(client, actor, "hi").await?;
                    self.focused = true;
                }
                let player = player_name(clients).await?;
                for (index, node) in path.iter().enumerate() {
                    let trigger = if node.triggers.is_empty() {
                        "npc sweep unmatched phrase".to_owned()
                    } else {
                        node.triggers.join(" ")
                    };
                    let reply = talk(client, actor, &trigger)
                        .await
                        .map_err(|error| format!("dialogue path node {}: {error}", node.key))?;
                    self.focused = reply.open.is_some();
                    check_reply(&reply, &node.reply, &player)?;
                    if node.ungreet && self.focused {
                        return Err("ungreet keyword left the conversation open".into());
                    }
                    if !self.focused && index + 1 < path.len() {
                        return Err(format!("dialogue path blocked after {}", node.key).into());
                    }
                }
                Ok(())
            }
            Action::Offer(offer, occurrence) => {
                let sell = (offer.direction.side() == NpcTradeSide::Sell).then_some(*offer);
                let client = self.enter(at, sell).await?;
                let actor = visible_npc(client, there, name).await?;
                talk(client, actor, "hi").await?;
                trade(client, actor, offer, *occurrence).await
            }
            Action::Route(route) => {
                let client = self.enter(at, None).await?;
                let actor = visible_npc(client, there, name).await?;
                talk(client, actor, "hi").await?;
                travel(client, actor, route).await
            }
            Action::Service(service) => {
                let client = self.enter(at, None).await?;
                let actor = visible_npc(client, there, name).await?;
                talk(client, actor, "hi").await?;
                perform_service(client, actor, clients, service).await
            }
        }
    }
}

impl Action<'_> {
    fn cause(&self) -> &'static str {
        match self {
            Self::Greet(_) => "greeting",
            Self::Keyword(_) => "dialogue",
            Self::Missing(_) => "missing_binding",
            Self::Offer(..) => "shop_offer",
            Self::Route(_) => "travel",
            Self::Service(_) => "service",
        }
    }
}

async fn visible_npc(
    client: &mut DevClientSession,
    at: TilePos,
    name: &str,
) -> TestResult<EntityRef> {
    let expected = spatial_basis::current(client)?.position(at);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(npc) = client.world_entities().and_then(|entities| {
            entities.iter().find(|npc| {
                npc.kind == EntityKind::Npc
                    && npc.position == expected
                    && npc
                        .display_name
                        .as_ref()
                        .and_then(|name| name.as_str().ok())
                        == Some(name)
            })
        }) {
            return Ok(npc.entity);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("NPC {name} missing from wire at {expected:?}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

async fn player_name(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    Ok(sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(clients.characters[CHARACTER].as_slice())
    .fetch_one(&mut database)
    .await?)
}

async fn say(client: &mut DevClientSession, actor: EntityRef, text: &str) -> TestResult {
    let request = wire::NpcTalkIntent {
        npc_actor: actor,
        text: text.into(),
    };
    // The owner allows four intents in a second (npc_service.rs:45).
    // Pace the single-client fixture so content cannot fail due to its own bursts.
    tokio::time::sleep(Duration::from_millis(260)).await;
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        npc_shop::talk(client.session_mut(), &request),
    )
    .await??;
    if result != NpcIntentDisposition::Ok {
        return Err(format!("NPC talk {text:?} returned {result:?}").into());
    }
    Ok(())
}

async fn talk(
    client: &mut DevClientSession,
    actor: EntityRef,
    text: &str,
) -> TestResult<NpcConversation> {
    say(client, actor, text).await?;
    let delta = tokio::time::timeout(
        Duration::from_secs(2),
        client
            .session_mut()
            .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1),
    )
    .await
    .map_err(|_| format!("no conversation update for {text:?}"))??;
    let reply = wire::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("NPC conversation wire: {error:?}"))?;
    if reply.open.is_some_and(|(npc, _)| npc != actor) {
        return Err(format!("conversation update for {text:?} names another NPC").into());
    }
    Ok(reply)
}

fn reply_matches(actual: &[String], expected: &[String], player: &str) -> bool {
    actual.iter().map(String::as_str).eq(expected
        .iter()
        .map(|line| oteryn_gameplay_npc::quest_greetings::render_player_name(line, player)))
}

fn check_reply(reply: &NpcConversation, expected: &[String], player: &str) -> TestResult {
    if !reply_matches(&reply.lines, expected, player) {
        return Err(format!("expected {expected:?}, got {:?}", reply.lines).into());
    }
    Ok(())
}

async fn balances(
    client: &mut DevClientSession,
) -> TestResult<npc_shop::resources::ResourceBalances> {
    tokio::time::timeout(
        Duration::from_secs(5),
        npc_shop::balances(client.session_mut()),
    )
    .await??
    .ok_or_else(|| "live carried and bank gold unavailable".into())
}

fn gold(value: npc_shop::resources::ResourceBalances) -> i128 {
    i128::from(value.bank_gold) + i128::from(value.carried_gold)
}

fn units(client: &DevClientSession, definition: NonZeroU32) -> TestResult<i128> {
    let inventory = client.inventory().ok_or("wire inventory missing")?;
    Ok(inventory
        .entries
        .iter()
        .chain(inventory.equipment.iter().map(|slot| &slot.item))
        .filter(|entry| entry.item_definition_ref == definition)
        .map(|entry| i128::from(entry.count.get()))
        .sum())
}

fn exact_delta(label: &str, before: i128, after: i128, expected: i128) -> TestResult {
    let actual = after - before;
    if actual != expected {
        return Err(format!("{label} delta expected {expected}, observed {actual}").into());
    }
    Ok(())
}

async fn shop_window(
    client: &mut DevClientSession,
    actor: EntityRef,
) -> TestResult<wire::NpcTradeWindow> {
    say(client, actor, "trade").await?;
    let delta = tokio::time::timeout(
        Duration::from_secs(2),
        client
            .session_mut()
            .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1),
    )
    .await??;
    wire::decode_npc_trade_window_for(
        &delta.payload,
        client
            .selected_capabilities()
            .contains(&oteryn_protocol_oteryn::trade_details::CAPABILITY_TRADE_DETAILS_V1),
    )
    .map_err(|error| format!("NPC trade window wire: {error:?}").into())
}

async fn trade(
    client: &mut DevClientSession,
    actor: EntityRef,
    offer: &Offer,
    occurrence: usize,
) -> TestResult {
    let items = provisioning::item_index()?;
    let definition = items
        .definition_ref("Item", &offer.item.key, &offer.item.revision)
        .ok_or("authored item reference missing")?;
    let window = shop_window(client, actor).await?;
    let (owner, revision) = window.open.ok_or("shop window closed")?;
    if owner != actor {
        return Err("shop window names another NPC".into());
    }
    let side = offer.direction.side();
    let price = u32::try_from(offer.unit_price)?;
    let mut matching = window.offers.iter().enumerate().filter(|(_, quote)| {
        quote.item_definition_ref == definition.get()
            && quote.side == side
            && quote.unit_price == price
            && quote.count == offer.count
    });
    let (index, _) = matching.nth(occurrence).ok_or_else(|| {
        format!(
            "content quote missing: {} {side:?} count={} price={price} currency={}",
            offer.item.key,
            offer.count,
            offer.currency.as_ref().map_or("gold", |row| &row.key),
        )
    })?;
    // The current catalogue owner holds non-gold currency offers before the wire projection
    // (crates/gameplay/content/src/content/npc_catalogue/service.rs:289).
    if let Some(currency) = &offer.currency {
        return Err(format!(
            "offer requires {} and the wire quote settles gold",
            currency.key
        )
        .into());
    }
    let before_gold = gold(balances(client).await?);
    let before_units = units(client, definition)?;
    let prior_stacks: BTreeMap<_, _> = client
        .inventory()
        .ok_or("wire inventory missing")?
        .entries
        .iter()
        .filter(|item| item.item_definition_ref == definition)
        .map(|item| (item.handle.get(), item.count.get()))
        .collect();
    let request = wire::NpcTradeIntent {
        npc_actor: actor,
        catalogue_revision: revision,
        offer_index: u32::try_from(index)?,
        side,
        quantity: 1,
        expected_unit_price: price,
    };
    let disposition = tokio::time::timeout(
        Duration::from_secs(5),
        npc_shop::trade(client.session_mut(), &request),
    )
    .await??;
    if disposition != NpcIntentDisposition::Ok {
        return Err(format!("shop {side:?} returned {disposition:?}").into());
    }
    let (gold_delta, item_delta) = offer.direction.deltas(offer.unit_price, offer.count);
    let after_gold = gold(balances(client).await?);
    exact_delta("gold", before_gold, after_gold, gold_delta)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        let after_units = units(client, definition)?;
        if after_units - before_units == item_delta {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return exact_delta("item", before_units, after_units, item_delta);
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    if side == NpcTradeSide::Buy {
        // Free backpack slots with normal wire item moves so later offers start with capacity.
        discard_purchase(client, definition, &prior_stacks).await?;
    }
    Ok(())
}

async fn discard_purchase(
    client: &mut DevClientSession,
    definition: NonZeroU32,
    prior_stacks: &BTreeMap<u64, u32>,
) -> TestResult {
    use oteryn_dev_client::client_runtime::inventory::MoveRequest;
    use oteryn_dev_client::{ItemMoveDestination, ItemMoveOutcome};
    use oteryn_protocol_oteryn::item_view::WorldTilePosition;

    let ground = client.world_spatial().actor_position;
    let destination = ItemMoveDestination::Ground(WorldTilePosition {
        x: ground.x,
        y: ground.y,
        floor: ground.floor,
    });
    let inventory = client
        .inventory()
        .ok_or("wire inventory missing after purchase")?;
    let mut moves = Vec::new();
    for item in &inventory.entries {
        if item.item_definition_ref == definition {
            let count = item.count.get();
            let added =
                count.saturating_sub(prior_stacks.get(&item.handle.get()).copied().unwrap_or(0));
            if added != 0 {
                moves.push(MoveRequest {
                    source: item.handle,
                    destination,
                    count: Some(added),
                });
            }
        }
    }
    for request in moves {
        let dropped = tokio::time::timeout(
            Duration::from_secs(5),
            client.session_mut().move_item_counted(request),
        )
        .await??;
        if dropped.outcome != ItemMoveOutcome::Moved {
            return Err(format!("fixture purchase cleanup refused: {dropped:?}").into());
        }
    }
    Ok(())
}

async fn travel(client: &mut DevClientSession, actor: EntityRef, route: &Route) -> TestResult {
    let before = gold(balances(client).await?);
    let prompt = talk(client, actor, &route.key.replace('_', " ")).await?;
    // Paid travel uses an Open conversation with the fare prompt (paid_travel.rs:181);
    // the legacy travel owner can use Confirming.
    let fare = format!("for {} gold?", route.price);
    let quoted = prompt.lines.iter().any(|line| line.ends_with(&fare));
    if !matches!(
        prompt.open,
        Some((npc, NpcConversationState::Confirming)) if npc == actor
    ) && !(prompt.open == Some((actor, NpcConversationState::Open)) && quoted)
    {
        return Err(format!(
            "expected a travel confirmation at price {}, got {:?}",
            route.price, prompt.lines
        )
        .into());
    }
    say(client, actor, "yes").await?;
    let destination = route.destination.native()?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tile(client.world_spatial().actor_position)? != destination {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("departure did not arrive at {destination:?}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    exact_delta(
        "travel gold",
        before,
        gold(balances(client).await?),
        -i128::from(route.price),
    )
}

async fn perform_service(
    client: &mut DevClientSession,
    actor: EntityRef,
    clients: &SeamClients<'_>,
    service: &Service,
) -> TestResult {
    let key = service.identity.key.as_str();
    if key.starts_with("oteryn:service.bank.") {
        let before = balances(client).await?;
        let balance_reply = talk(client, actor, "balance").await?;
        if !balance_reply.lines.iter().any(|line| {
            line.split(|c: char| !c.is_ascii_digit())
                .any(|word| word.parse::<u64>().ok() == Some(before.bank_gold))
        }) {
            return Err(format!("bank balance reply disagrees with {}", before.bank_gold).into());
        }
        for (text, delta) in [("deposit 1 gold", 1), ("withdraw 1 gold", -1)] {
            let prior = balances(client).await?;
            talk(client, actor, text).await?;
            talk(client, actor, "yes").await?;
            let after = balances(client).await?;
            exact_delta(
                "bank balance",
                prior.bank_gold.into(),
                after.bank_gold.into(),
                delta,
            )?;
            exact_delta(
                "carried gold",
                prior.carried_gold.into(),
                after.carried_gold.into(),
                -delta,
            )?;
        }
        return Ok(());
    }
    if key.starts_with("oteryn:service.trade.") {
        let window = shop_window(client, actor).await?;
        if window.open.is_some_and(|(npc, _)| npc == actor) && window.offers.is_empty() {
            return Ok(());
        }
        return Err("empty authored service did not produce an empty shop".into());
    }

    let (topic, fee) = service_price(key)?;
    let before = gold(balances(client).await?);
    talk(client, actor, &topic).await?;
    say(client, actor, "yes").await?;
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let session = client.join_snapshot().game_session_id.as_bytes().as_slice();
    // Receipt binds the real confirmation to this NPC, session and Service. The zero-price
    // phoenix stage has its own journal (durability/npc_purchase/receipt.rs:30).
    let charged: Option<i64> = sqlx::query_scalar(
        "SELECT fee_gold_units FROM game_character_npc_purchase_receipts
        WHERE character_id=encode($1,'hex')::uuid AND command_session_id=encode($2,'hex')::uuid
            AND service_key=$3 AND npc_identity=$4 ORDER BY command_id DESC LIMIT 1",
    )
    .bind(clients.characters[CHARACTER].as_slice())
    .bind(session)
    .bind(key)
    .bind(actor.identity.as_slice())
    .fetch_optional(&mut connection)
    .await?;
    if fee == 0 {
        let prepared: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_npc_stage_receipts
            WHERE character_id=encode($1,'hex')::uuid AND command_session_id=encode($2,'hex')::uuid AND npc_identity=$3)")
            .bind(clients.characters[CHARACTER].as_slice()).bind(session).bind(actor.identity.as_slice())
            .fetch_one(&mut connection).await?;
        if !prepared {
            return Err("phoenix stage receipt missing".into());
        }
    } else if charged != Some(i64::try_from(fee)?) {
        return Err(format!("purchase receipt expected price {fee}, observed {charged:?}").into());
    }
    exact_delta(
        "service gold",
        before,
        gold(balances(client).await?),
        -i128::from(fee),
    )
}

fn service_price(key: &str) -> TestResult<(String, u64)> {
    if key.starts_with("oteryn:service.promotion.") {
        return Ok((
            "promotion".into(),
            oteryn_gameplay_progression::promotion::PROMOTION_PRICE,
        ));
    }
    let catalogue = BlessingCatalogue::load()?;
    match key {
        "oteryn:service.blessing.henricus" => catalogue
            .henricus_quote(LEVEL, &[])
            .map(|(price, _)| ("blessing".into(), price))
            .ok_or_else(|| "owner blessing bundle missing".into()),
        "oteryn:service.blessing.phoenix_prerequisite" => Ok(("phoenix".into(), 0)),
        _ => catalogue
            .blessings
            .iter()
            .find(|blessing| blessing.service == key)
            .map(|blessing| (blessing.name.clone(), blessing.price(LEVEL)))
            .ok_or_else(|| format!("no service oracle for {key}").into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_positive_decimal_batch_lengths() -> TestResult {
        assert_eq!(batch(None)?, (0, 3));
        assert_eq!(batch(Some("0:20"))?, (0, 20));
        assert_eq!(batch(Some("100:2"))?, (100, 2));
        for text in [
            "",
            "1",
            "1:0",
            " 1:2",
            "1:2 ",
            "+1:2",
            "-1:2",
            "1:2:3",
            "9999999999999999999999999:1",
        ] {
            assert!(batch(Some(text)).is_err(), "{text:?}");
        }
        Ok(())
    }

    #[test]
    fn shop_direction_requires_both_gold_and_item_deltas() {
        assert_eq!(Direction::BuyFromPlayer.side(), NpcTradeSide::Sell);
        assert_eq!(Direction::BuyFromPlayer.deltas(25, 3), (25, -3));
        assert_eq!(Direction::SellToPlayer.side(), NpcTradeSide::Buy);
        assert_eq!(Direction::SellToPlayer.deltas(25, 3), (-25, 3));
        assert!(exact_delta("coin", 27, 2, -25).is_ok());
        assert!(exact_delta("coin", 27, 3, -25).is_err());
    }

    #[test]
    fn dialogue_oracle_requires_all_lines_in_order() {
        let expected = ["Hello |PLAYERNAME|.".to_owned(), "Have a look.".to_owned()];
        let actual = ["Hello Fixture.".to_owned(), "Have a look.".to_owned()];
        assert!(reply_matches(&actual, &expected, "Fixture"));
        assert!(!reply_matches(&actual[..1], &expected, "Fixture"));
    }

    #[test]
    fn owner_service_prices_and_content_bindings_resolve() -> TestResult {
        let census = Census::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))?;
        for (key, service) in &census.services {
            if service.offers.is_empty()
                && service.routes.is_empty()
                && !key.starts_with("oteryn:service.bank.")
                && !key.starts_with("oteryn:service.trade.")
            {
                service_price(key)?;
            }
        }
        assert_eq!(
            service_price("oteryn:service.promotion.eremo")?,
            ("promotion".into(), 20_000)
        );
        assert_eq!(
            service_price("oteryn:service.blessing.the_embrace_of_tibia")?,
            ("The Embrace of Tibia".into(), 16_000)
        );
        assert_eq!(
            service_price("oteryn:service.blessing.henricus")?,
            ("blessing".into(), 88_000)
        );
        Ok(())
    }
}
