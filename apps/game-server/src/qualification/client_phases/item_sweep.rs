//! Opt-in node-boot item census and wire checks. Select item_sweep with NODE_BOOT_E2E_ONLY.
//! OTERYN_E2E_ITEM_SWEEP_BATCH=start:len selects a key-sorted slice, default 0:3.
//! The node's qualification fixture seeds the build and item through its fenced owners.
//! Equipment, USE and container assertions use command results and client projections.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{
    cyclopedia_character::{CharacterTab, combat::CombatStat as Stat},
    item_view::{
        EquipmentSlot, ItemEntry, ItemHandle, ItemMoveDestination as Destination,
        ItemMoveOutcome as Move, WorldTilePosition,
    },
    world_object::UseDisposition as Use,
};
use serde_json::Value;
use std::num::NonZeroU32;

const BATCH_VARIABLE: &str = "OTERYN_E2E_ITEM_SWEEP_BATCH";
const VOCATIONS: [&str; 5] = ["knight", "paladin", "sorcerer", "druid", "monk"];
const HIGH: u32 = 600;
const LOW: u32 = 1;

fn batch(value: Option<&str>) -> TestResult<(usize, usize)> {
    let Some(value) = value else {
        return Ok((0, 3));
    };
    let number = |part: &str| {
        part.bytes()
            .all(|byte| byte.is_ascii_digit())
            .then(|| part.parse().ok())
            .flatten()
    };
    match value
        .split_once(':')
        .and_then(|(start, len)| Some((number(start)?, number(len)?)))
    {
        Some((start, len)) if len > 0 => Ok((start, len)),
        _ => Err(format!("{BATCH_VARIABLE} requires start:len with len > 0").into()),
    }
}

fn known(field: &Value) -> &Value {
    if field["state"] == "KNOWN" {
        &field["value"]
    } else {
        &Value::Null
    }
}

fn list(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

fn document(path: &Path) -> TestResult<Value> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

#[derive(Default)]
struct Item {
    semantics: Value,
    attributes: Value,
    observation: bool,
    materializable: bool,
    ground_only: bool,
}

impl Item {
    fn group(&self, name: &str) -> &Value {
        known(&self.semantics[name])
    }

    fn patterns(&self) -> &[Value] {
        list(known(&self.group("equipment")["patterns"]))
    }

    fn target(&self, kind: &str) -> Option<&str> {
        list(&self.group("use_transform")["targets"])
            .iter()
            .find(|target| target["kind"] == kind)
            .and_then(|target| known(&target["target"])["key"].as_str())
    }

    fn capacity(&self) -> Option<u64> {
        known(&self.group("container")["capacity"]).as_u64()
    }

    fn usable(&self) -> bool {
        self.observation
            || self.target("USE").is_some()
            || [
                "bed",
                "fluid",
                "readable_writeable",
                "consumption",
                "use_requirements",
            ]
            .iter()
            .any(|group| !self.group(group).is_null())
            || !self.attributes["rune_consumption"].is_null()
    }

    fn in_sweep(&self) -> bool {
        self.materializable
            && (!self.patterns().is_empty() || !self.group("container").is_null() || self.usable())
    }
}

/// content/items/index.json owns the shard list. Profile overlays supply additional admitted
/// definitions and runtime overrides (content/native_gameplay/item_profile_updates.rs).
fn census(root: &Path) -> TestResult<BTreeMap<String, Item>> {
    let index = document(&root.join("content/items/index.json"))?;
    let mut items = BTreeMap::new();
    for shard in index["shards"]
        .as_array()
        .ok_or("Item index missing shards")?
    {
        let path = shard.as_str().ok_or("Item shard path missing")?;
        let mut doc = document(&root.join(path))?;
        for record in doc["records"]
            .as_array_mut()
            .ok_or("Item shard missing records")?
        {
            let def = &mut record["definition"];
            let key = def["identity"]["key"]
                .as_str()
                .ok_or("Item key missing")?
                .to_owned();
            let item = Item {
                materializable: def["materializable"]
                    .as_bool()
                    .ok_or("Item admission missing")?,
                semantics: def["semantics"].take(),
                observation: !record["authoring"]["use_observation"].is_null(),
                ..Item::default()
            };
            if items.insert(key, item).is_some() {
                return Err("duplicate Item definition".into());
            }
        }
    }
    for name in [
        "spell-native-profiles",
        "starting-loadout-profiles",
        "srx1-item-updates",
    ] {
        let mut profile = document(&root.join(format!("content/items/definitions/{name}.json")))?;
        for record in profile["records"]
            .as_array_mut()
            .ok_or("Item profile missing records")?
        {
            let key = record["production_definition"]["production_key"]
                .as_str()
                .ok_or("Item profile key missing")?;
            let item = items.entry(key.to_owned()).or_default();
            if record["admission"]["materializable"] == true {
                item.materializable = true;
                let destinations = list(&record["admission"]["legal_destinations"]);
                item.ground_only = destinations.iter().any(|d| d == "Ground")
                    && !destinations.iter().any(|d| d == "CharacterInventory");
                item.attributes = record["attributes"].take();
                // Authored facts survive an overlay's UNKNOWN or NOT_APPLICABLE fields.
                if let Some(semantics) = record["semantics"].as_object_mut() {
                    for (group, fact) in semantics {
                        if fact["state"] == "KNOWN" && item.group(group).is_null() {
                            item.semantics[group] = fact.take();
                        }
                    }
                }
            }
        }
    }
    // Tools and rune-like items may use an owner selected by item identity, without
    // declaring a USE transform on the item definition.
    let taxonomy = document(&root.join("content/items/taxonomy/items.json"))?;
    for record in taxonomy["records"]
        .as_array()
        .ok_or("Item taxonomy missing records")?
    {
        if matches!(record["family_profile"].as_str(), Some("tool" | "rune"))
            && let Some(key) = record["target"]["key"].as_str()
            && let Some(item) = items.get_mut(key)
        {
            item.observation = true;
        }
    }
    Ok(items)
}

#[derive(Clone, Copy)]
struct Build {
    vocation: usize,
    level: u32,
}

impl Build {
    fn permits(self, required: &Value) -> bool {
        let vocations = list(known(&required["vocations"]));
        let level = known(&required["level"])
            .as_u64()
            .or_else(|| known(&required["min_level"]).as_u64());
        level.is_none_or(|level| u64::from(self.level) >= level)
            && (vocations.is_empty() || allowed_vocation(self.vocation, vocations))
    }

    fn supported(required: &Value) -> Option<Self> {
        (0..VOCATIONS.len())
            .map(|vocation| Self {
                vocation,
                level: HIGH,
            })
            .find(|build| build.permits(required))
    }
}

fn allowed_vocation(index: usize, vocations: &[Value]) -> bool {
    vocations.iter().any(|vocation| {
        vocation
            .as_str()
            .is_some_and(|vocation| vocation.eq_ignore_ascii_case(VOCATIONS[index]))
    })
}

fn slot(pattern: &Value) -> Option<EquipmentSlot> {
    Some(match known(&pattern["primary_slot"]).as_str()? {
        "HEAD" => EquipmentSlot::Head,
        "AMULET" => EquipmentSlot::Necklace,
        "TORSO" => EquipmentSlot::Armor,
        "WEAPON" => EquipmentSlot::RightHand,
        "SHIELD" => EquipmentSlot::LeftHand,
        "LEGS" => EquipmentSlot::Legs,
        "FEET" => EquipmentSlot::Feet,
        "RING" => EquipmentSlot::Ring,
        "EXTRA" => EquipmentSlot::Ammo,
        _ => return None,
    })
}

#[derive(Default)]
struct Tally {
    pass: usize,
    fail: usize,
}

pub(super) struct Rows<'a> {
    key: &'a str,
    pub(super) pending: Vec<&'static str>,
    counts: Tally,
    lap: std::time::Instant,
}

impl Rows<'_> {
    pub(super) fn emit(&mut self, step: &str, result: TestResult) {
        self.pending.retain(|name| *name != step);
        let seconds = self.lap.elapsed().as_secs_f64();
        self.lap = std::time::Instant::now();
        let (status, detail) = match result {
            Ok(()) => {
                self.counts.pass += 1;
                ("PASS", "wire-verified".to_owned())
            }
            Err(error) => {
                self.counts.fail += 1;
                ("FAIL", super::creature_sweep::token(&error.to_string()))
            }
        };
        evidence(&format!(
            "e2e_all scenario=IS.{}.{step}.full status={status} seconds={seconds:.3} mode=seeded evidence={detail}",
            self.key,
        ));
    }

    fn blocked(&mut self, step: &str) {
        for pending in std::mem::take(&mut self.pending) {
            self.emit(pending, Err(format!("blocked-by-{step}").into()));
        }
    }
}

struct Fixtures<'a> {
    scopes: [&'a SeamClients<'a>; 5],
    prepared: [bool; 10],
    socket: String,
}

impl Fixtures<'_> {
    fn fixture(&self, build: Build) -> (&SeamClients<'_>, usize) {
        (
            self.scopes[build.vocation],
            usize::from(build.level == HIGH),
        )
    }

    async fn seed(&mut self, key: &str, build: Build) -> TestResult<DevClientSession> {
        self.seed_with_gear(key, build, &[]).await
    }

    async fn seed_with_gear(
        &mut self,
        key: &str,
        build: Build,
        gear: &[&str],
    ) -> TestResult<DevClientSession> {
        let index = build.vocation * 2 + usize::from(build.level == HIGH);
        let scope = self.scopes[index / 2];
        let client = connect_character(scope, 0x71, index % 2).await?;
        let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
        let script = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/qualification/node_boot/provision.py");
        let socket = self.socket.clone();
        let mut keys: Vec<_> = std::iter::once(key)
            .chain(gear.iter().copied())
            .map(str::to_owned)
            .collect();
        let level = if self.prepared[index] { 0 } else { build.level };
        let status = tokio::task::spawn_blocking(move || {
            for (index, key) in keys.drain(..).enumerate() {
                let status = crate::qualification::provision_python()
                    .arg(&script)
                    .args([
                        "--socket",
                        &socket,
                        "--session",
                        &session,
                        "--item",
                        VOCATIONS[build.vocation],
                        &(if index == 0 { level } else { 0 }).to_string(),
                        &key,
                    ])
                    .status()?;
                if !status.success() {
                    return Ok::<_, std::io::Error>(false);
                }
            }
            Ok(true)
        })
        .await;
        // Release even after fixture refusal. Dropping an admitted socket instead
        // would park the Character in reconnect grace and block the next item.
        let released = release_fixture(client, scope, index % 2).await;
        if !status?? {
            return Err("fixture-item-preparation-refused".into());
        }
        released?;
        let mut client = connect_character(scope, 0x72, index % 2).await?;
        let facts = super::creature_sweep::read_stats(&mut client).await;
        let verification = facts.and_then(|(_, level, vocation)| {
            if level == build.level
                && vocation
                    == oteryn_protocol_oteryn::cyclopedia_character::vocation_id(
                        VOCATIONS[build.vocation],
                    )
            {
                Ok(())
            } else {
                Err(format!("fixture-build-observed-level-{level}-vocation-{vocation:?}").into())
            }
        });
        if let Err(error) = verification {
            let _ = release_fixture(client, scope, index % 2).await;
            return Err(error);
        }
        self.prepared[index] = true;
        Ok(client)
    }
}

async fn release_fixture(
    mut client: DevClientSession,
    scope: &SeamClients<'_>,
    index: usize,
) -> TestResult {
    if let Err(initial) = super::creature_sweep::release(&mut client).await {
        let id = *client.join_snapshot().game_session_id.as_bytes();
        let previous = client.disconnect();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        verdict::Harness::global().bounded_wait(deadline, "item-fixture-recovery");
        loop {
            match session_loss_row(scope.url, id).await? {
                Some((1, _, _)) => break,
                Some((3, _, _)) | None => return Ok(()),
                _ if tokio::time::Instant::now() >= deadline => {
                    return Err(format!("fixture-recovery-loss-timeout: {initial}").into());
                }
                _ => tokio::time::sleep(Duration::from_millis(100)).await,
            }
        }
        let generation = platform_generation(scope.descriptor, &scope.accounts[index]).await?;
        let (nonce, _) = fresh_grant_material(0x73)?;
        let token = sign_recovery(
            scope.recovery,
            &scope.accounts[index],
            &scope.characters[index],
            &scope.world,
            generation,
            now_seconds()?,
            nonce,
        );
        let mut resumed = oteryn_dev_client::resume_session(
            oteryn_dev_client::RecoveryTransport {
                address: scope.address,
                server_name: "localhost",
                root_certificate: scope.certificate,
                deadline: Duration::from_secs(20),
            },
            previous,
            oteryn_dev_client::Recovery {
                reconnect_material: token.as_bytes(),
                schema_revision: 1,
                client_build_id: "oteryn-dev-client/item-sweep",
                deadline: Duration::from_secs(20),
                appearance_items: None,
            },
        )
        .await?;
        super::creature_sweep::release(&mut resumed).await?;
    }
    Ok(())
}

/// Remove only scratch-world ground custody created by this entry's fixture
/// characters. The MOVE receipt and line keep the historical item identity.
async fn clear_fixture_drops(
    db: &mut sqlx::PgConnection,
    fixtures: &Fixtures<'_>,
    after: i64,
) -> TestResult {
    let characters: Vec<_> = fixtures
        .scopes
        .iter()
        .flat_map(|scope| {
            scope
                .characters
                .iter()
                .map(|character| uuid_text(character))
        })
        .collect();
    let scope = fixtures.scopes[0];
    let mut tx = db.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "DELETE FROM game_item_ground_locations g USING game_item_loop_move_lines l, \
         game_item_loop_move_receipts r WHERE g.item_instance_id=l.item_instance_id \
         AND l.transaction_id=r.transaction_id AND l.to_kind=3 \
         AND g.spatial_position=l.to_spatial_position AND g.stack_ordinal>$1 \
         AND g.world_id=$2::uuid AND g.channel_id=$3::uuid \
         AND r.world_id=g.world_id AND r.channel_id=g.channel_id \
         AND r.character_id::text=ANY($4)",
    )
    .bind(after)
    .bind(uuid_text(&scope.world))
    .bind(uuid_text(&scope.channel))
    .bind(&characters)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

fn carried(client: &DevClientSession, reference: NonZeroU32) -> Option<ItemEntry> {
    client
        .inventory()?
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == reference)
        .copied()
}

async fn observe<T>(
    client: &mut DevClientSession,
    read: impl Fn(&mut DevClientSession) -> Option<T>,
) -> TestResult<T> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(value) = read(client) {
            return Ok(value);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("wire-projection-timeout".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

async fn move_item(
    client: &mut DevClientSession,
    source: ItemHandle,
    destination: Destination,
    expected: Move,
) -> TestResult {
    let reply = client
        .session_mut()
        .move_item_counted(oteryn_dev_client::client_runtime::inventory::MoveRequest {
            source,
            destination,
            count: None,
        })
        .await?;
    if reply.outcome != expected {
        return Err(format!("ITEM_MOVE-source-{source}-destination-{destination:?}-expected-{expected:?}-observed-{reply:?}").into());
    }
    Ok(())
}

async fn drop_carried(client: &mut DevClientSession, reference: NonZeroU32) -> TestResult {
    if let Some(worn) = client.inventory().and_then(|inventory| {
        inventory
            .equipment
            .iter()
            .find(|worn| worn.item.item_definition_ref == reference)
            .map(|worn| worn.item.handle)
    }) {
        move_item(client, worn, Destination::MainBackpack, Move::Moved).await?;
        observe(client, |client| carried(client, reference)).await?;
    }
    while let Some(item) = carried(client, reference) {
        let at = client.world_spatial().actor_position;
        move_item(
            client,
            item.handle,
            Destination::Ground(WorldTilePosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            }),
            Move::Moved,
        )
        .await?;
        observe(client, |client| {
            let remaining = client
                .inventory()?
                .entries
                .iter()
                .find(|entry| entry.handle == item.handle);
            remaining
                .is_none_or(|entry| entry.count < item.count)
                .then_some(())
        })
        .await?;
    }
    Ok(())
}

// Fixture roots are compiled onto the disposable base map by
// tools/world-bundle-compiler/src/source.rs. An ordinary join publishes their
// handles with the initial WorldMap snapshot (world_map.rs:188–199).
async fn rack(
    fixtures: &Fixtures<'_>,
    reference: NonZeroU32,
    item: &Item,
    rows: &mut Rows<'_>,
) -> TestResult {
    use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};
    let mut client = connect_character(fixtures.scopes[0], 0x71, 0).await?;
    let opened = async {
        let position = client.world_spatial().actor_position;
        let (tile, handle) = observe(&mut client, |client| {
            client
                .session_mut()
                .world_map_snapshot()?
                .tiles
                .iter()
                .filter(|tile| {
                    tile.position.floor == position.floor
                        && tile.position.x.abs_diff(position.x)
                            + tile.position.y.abs_diff(position.y)
                            == 1
                })
                .find_map(|tile| {
                    tile.items.iter().find_map(|entry| match (entry.definition, entry.origin) {
                        (MapDefinition::Item(item), MapOrigin::Handle(handle)) if item == reference => {
                            Some((tile.position, handle))
                        }
                        _ => None,
                    })
                })
        })
        .await?;
        evidence(&format!(
            "item_sweep rack key={} mode=seeded custody=map-base actor={position:?} tile={tile:?} command=USE handle={handle}",
            rows.key
        ));
        rows.emit("seed", Ok(()));
        if rows.pending.contains(&"container") {
            rows.emit(
                "container",
                container(&mut client, handle, item.capacity()).await,
            );
        }
        Ok(())
    }
    .await;
    let released = release_fixture(client, fixtures.scopes[0], 0).await;
    if rows.pending.contains(&"seed") {
        return opened.and(released);
    }
    rows.emit("cleanup", opened.and(released));
    Ok(())
}

/// Bind exact fresh corpse definitions to the weakest authored creature that
/// can produce them. Decayed and alternate forms need a separate transition.
fn corpse_sources(root: &Path) -> TestResult<BTreeMap<String, String>> {
    let path = root.join("content/creatures/definitions/spell-native-profiles.json");
    let source = document(&path)?;
    let mut candidates = BTreeMap::<String, (u64, String)>::new();
    for entry in list(&source["records"]) {
        let row = &entry["profile"];
        let profile = &row["data"]["profile"];
        let Some(item) = profile["details"]["corpse_item"]["key"].as_str() else {
            continue;
        };
        let Some(creature) = row["target"]["key"].as_str() else {
            continue;
        };
        let item = item
            .strip_prefix("canary:item/")
            .map_or_else(|| item.to_owned(), |id| format!("oteryn:item.tibia.i{id}"));
        let creature = creature.strip_prefix("canary:creature/").map_or_else(
            || creature.to_owned(),
            |name| format!("oteryn:creature.{name}"),
        );
        let choice = (profile["health"].as_u64().unwrap_or(u64::MAX), creature);
        if candidates.get(&item).is_none_or(|old| choice < *old) {
            candidates.insert(item, choice);
        }
    }
    Ok(candidates
        .into_iter()
        .map(|(item, (_, creature))| (item, creature))
        .collect())
}

async fn corpse(
    fixtures: &mut Fixtures<'_>,
    world: &BundleWorld,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    creature: &str,
    reference: NonZeroU32,
    item: &Item,
    rows: &mut Rows<'_>,
) -> TestResult {
    use oteryn_dev_client::EntityDetail;
    use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, ChaseMode, FightModes};
    let sword_key = "oteryn:item.tibia.i3265";
    let sword = self::reference(index, sword_key)?;
    let build = Build {
        vocation: 0,
        level: HIGH,
    };
    let mut client = fixtures.seed(sword_key, build).await?;
    let (scope, character) = fixtures.fixture(build);
    let probe = async {
        let weapon = carried(&client, sword).ok_or("corpse-fixture-sword-missing")?;
        move_item(
            &mut client,
            weapon.handle,
            Destination::Equipment(EquipmentSlot::RightHand),
            Move::Moved,
        )
        .await?;
        super::creature_sweep::leave_protection_zone(&mut client, world).await?;
        let answer = request_spawn(
            uuid_text(client.join_snapshot().game_session_id.as_bytes()),
            creature.to_owned(),
        )
        .await?;
        let target = parse_spawned_entity(&answer)?;
        observe(&mut client, |client| {
            client.world_entities()?.get(&target).copied()
        })
        .await?;
        let modes = FightModes {
            chase: ChaseMode::Chase,
            ..FightModes::DEFAULT
        };
        if client
            .session_mut()
            .set_fight_modes(&modes)
            .await?
            .disposition
            != AttackIntentDisposition::Ok
            || client
                .session_mut()
                .attack_target(Some(&target))
                .await?
                .disposition
                != AttackIntentDisposition::Ok
        {
            return Err("corpse-fixture-attack-refused".into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(75);
        loop {
            verdict::Harness::global().progress("item-corpse-fight");
            client.service_liveness(Duration::from_millis(50)).await?;
            if client
                .world_entities()
                .is_some_and(|entities| entities.get(&target).is_none())
            {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("corpse-fixture-kill-deadline".into());
            }
        }
        let death = committed_hunt_death(
            &mut client,
            scope,
            Some(scope.characters[character]),
            creature,
            target,
            tokio::time::Instant::now() + Duration::from_secs(20),
        )
        .await?;
        let identity = death.actor.corpse_identity();
        let handle = observe(&mut client, |client| {
            client.world_entities()?.iter().find_map(|entity| {
                if entity.entity.identity != identity
                    || entity.kind != oteryn_dev_client::EntityKind::Corpse
                {
                    return None;
                }
                match entity.detail {
                    EntityDetail::Object {
                        item_handle,
                        item_definition_ref,
                        ..
                    }
                    | EntityDetail::ObjectWithAppearance {
                        item_handle,
                        item_definition_ref,
                        ..
                    } if item_definition_ref == reference.get() => item_handle,
                    _ => None,
                }
            })
        })
        .await?;
        rows.emit("seed", Ok(()));
        if rows.pending.contains(&"container") {
            rows.emit(
                "container",
                async {
                    super::creature_sweep::open_corpse(&mut client, handle).await?;
                    let capacity = client.session_mut().container_views().and_then(|views| {
                        views
                            .views
                            .iter()
                            .find(|view| view.container_handle == handle)
                            .map(|view| u64::from(view.capacity))
                    });
                    if capacity != item.capacity() {
                        return Err(format!(
                            "corpse-capacity-expected-{:?}-observed-{capacity:?}",
                            item.capacity()
                        )
                        .into());
                    }
                    Ok(())
                }
                .await,
            );
        }
        Ok(())
    }
    .await;
    let _ = client.session_mut().attack_target(None).await;
    let cleanup = drop_carried(&mut client, sword).await;
    let released = release_fixture(client, scope, character).await;
    rows.emit("cleanup", cleanup.and(released));
    probe
}

/// Seed one disposable Ground root using the Channel's current content pin. The
/// historical-row setup follows event_containers/ground.rs:201–224. Browse and USE
/// subsequently go through the normal wire path and the live item's custody checks.
async fn ground_container(
    fixtures: &mut Fixtures<'_>,
    world: &BundleWorld,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    key: &str,
    revision: &str,
    reference: NonZeroU32,
    item: &Item,
    rows: &mut Rows<'_>,
) -> TestResult {
    use oteryn_protocol_oteryn::{browse_field as browse, container_tree as tree};
    // A real drop publishes a Ground projection invalidation after the
    // fixture write (loop_move.rs:1253).
    let marker = provisioning::item_reference(index, 3003)?;
    let mut client = fixtures
        .seed(
            "oteryn:item.tibia.i3003",
            Build {
                vocation: 0,
                level: HIGH,
            },
        )
        .await?;
    let scope = fixtures.scopes[0];
    let mut db_result = sqlx::PgConnection::connect(scope.url).await;
    let mut marker_on_ground = None;
    let result = async {
        let db = db_result.as_mut().map_err(|_| "ground-fixture-database-connect")?;
        let marker_item = carried(&client, marker).ok_or("ground-fixture-marker-missing")?;
        let at = client.world_spatial().actor_position;
        let (x, y, floor) = (at.x, at.y, -at.floor);
        let tile = [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]
            .into_iter()
            .find(|&(x, y)| {
                u16::try_from(x).ok().zip(u16::try_from(y).ok())
                    .zip(i8::try_from(floor).ok())
                    .is_some_and(|((x, y), z)| world.base().tile(x, y, z).is_some())
            })
            .ok_or("ground-fixture-no-adjacent-tile")?;
        let position = WorldTilePosition { x: tile.0, y: tile.1, floor: at.floor };
        let cell = [tile.0.to_be_bytes().as_slice(), tile.1.to_be_bytes().as_slice(),
            floor.to_be_bytes().as_slice()].concat();
        let frame = world.frame_binding_digest(WorldId::decode(&scope.world)?);
        let mut tx = db.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        let id: String = sqlx::query_scalar(
            "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),$1::uuid,'Item',$2,$3,1,1,game_character_uuid_v7()) RETURNING item_instance_id::text",
        ).bind(uuid_text(&scope.world)).bind(key).bind(revision).fetch_one(&mut *tx).await?;
        let inserted = sqlx::query(
            "INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) SELECT $1::uuid,world_id,channel_id,$4::text::numeric,$5,convert_to('OTERYN_ITEM_LOOP_DROP/v1:','UTF8') || uuid_send($1::uuid),$6,'sha256:' || encode(server_artifact_digest,'hex'),$7,nextval('game_item_ground_stack_ordinal_seq') FROM game_content_activations WHERE world_id=$2::uuid AND channel_id=$3::uuid ORDER BY activation_sequence DESC LIMIT 1",
        ).bind(&id).bind(uuid_text(&scope.world)).bind(uuid_text(&scope.channel))
            .bind(scope.scope_generation.to_string()).bind(&cell)
            .bind(crate::map::overlay::map_revision(world.base())).bind(frame.as_slice())
            .execute(&mut *tx).await?;
        if inserted.rows_affected() != 1 {
            return Err("ground-fixture-active-pin-unavailable".into());
        }
        tx.commit().await?;

        let opened = async {
            move_item(&mut client, marker_item.handle, Destination::Ground(position), Move::Moved).await?;
            marker_on_ground = Some(position);
            let payload = browse::encode_browse_field_intent(&browse::BrowseFieldIntent::Open { position })
                .map_err(|error| format!("ground-fixture-BROWSE_FIELD-encode-{error:?}"))?;
            let reply = client.session_mut().gated_command(browse::CAPABILITY_BROWSE_FIELD_V1,
                browse::COMMAND_TYPE_BROWSE_FIELD_INTENT, &payload).await?;
            if reply.status != CommandStatus::Accepted
                || tree::decode_container_view_result(&reply.payload).ok() != Some(tree::ContainerViewOutcome::Opened)
            {
                return Err(format!("ground-fixture-BROWSE_FIELD-{position:?}-{reply:?}").into());
            }
            observe(&mut client, |client| {
                let view = client.session_mut().container_views()?.views.iter()
                    .find(|view| view.browse.is_some_and(|page| page.position == position))?;
                let mut matches = view.entries.iter().filter(|entry| entry.item_definition_ref == reference);
                let entry = *matches.next()?;
                matches.next().is_none().then_some(entry)
            }).await
        }.await;
        match opened {
            Ok(entry) => {
                rows.emit("seed", Ok(()));
                if rows.pending.contains(&"container") {
                    rows.emit("container", container(&mut client, entry.handle, item.capacity()).await);
                }
                if rows.pending.contains(&"use") {
                    rows.emit("use", use_effect_ground(&mut client, entry).await);
                }
            }
            Err(error) => rows.emit("seed", Err(error)),
        }

        let mut tx = db.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=$1::uuid")
            .bind(&id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM game_item_instances WHERE item_instance_id=$1::uuid")
            .bind(&id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }.await;
    let marker_cleanup = async {
        if let Some(position) = marker_on_ground {
            let handle = observe(&mut client, |client| {
                client
                    .session_mut()
                    .container_views()?
                    .views
                    .iter()
                    .find(|view| view.browse.is_some_and(|page| page.position == position))?
                    .entries
                    .iter()
                    .find(|entry| entry.item_definition_ref == marker)
                    .map(|entry| entry.handle)
            })
            .await?;
            move_item(&mut client, handle, Destination::MainBackpack, Move::Moved).await?;
        }
        Ok(())
    }
    .await;
    let released = release_fixture(client, fixtures.scopes[0], 1).await;
    if rows.pending.contains(&"seed") {
        return result.and(released);
    }
    rows.emit("cleanup", result.and(marker_cleanup).and(released));
    Ok(())
}

async fn use_effect_ground(client: &mut DevClientSession, entry: ItemEntry) -> TestResult {
    let reply = client.use_item(entry.handle).await?;
    Err(format!(
        "ground-USE-handle-{}-observed-{reply:?}-effect-oracle-unavailable",
        entry.handle
    )
    .into())
}

async fn container(
    client: &mut DevClientSession,
    handle: ItemHandle,
    capacity: Option<u64>,
) -> TestResult {
    let reply = client.use_item(handle).await?;
    if reply.status != CommandStatus::Accepted || reply.disposition != Use::Committed {
        return Err(format!("container-USE-handle-{}-{reply:?}", handle).into());
    }
    let observed = observe(client, |client| {
        client
            .session_mut()
            .container_views()?
            .views
            .iter()
            .find(|view| view.container_handle == handle)
            .map(|view| u64::from(view.capacity))
    })
    .await?;
    if Some(observed) != capacity {
        return Err(format!("container-capacity-expected-{capacity:?}-observed-{observed}").into());
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct StatCheck {
    stat: Stat,
    expected: i64,
    relative: bool,
}

fn points(group: &Value, field: &str) -> Option<i64> {
    known(&group[field]).as_i64()
}

fn percent(value: &Value) -> Option<i64> {
    value["numerator"]
        .as_i64()?
        .checked_mul(10_000)?
        .checked_div(value["denominator"].as_i64()?)
}

// Live owner projections: attack/cyclopedia.rs:350 (attack), :446 (armour), :476
// (hand defence) and :193 (resistances). The single hand owner is
// combat-rules/src/resolution/player_defence.rs:37.
fn stat_checks(item: &Item) -> (Vec<StatCheck>, i64, Vec<&str>) {
    let mut checks = Vec::new();
    let mut speed = 0;
    let mut missing = Vec::new();
    let mut add = |stat, amount, relative| {
        checks.push(StatCheck {
            stat,
            expected: amount,
            relative,
        })
    };
    let protection = item.group("protection");
    if let Some(armor) = points(protection, "armor") {
        add(Stat::Armor, armor, true);
    }
    let weapon = item.group("weapon");
    if let Some(attack) = points(weapon, "attack") {
        let elemental = list(known(&weapon["elemental"]))
            .iter()
            .filter_map(|entry| points(entry, "points"))
            .sum::<i64>();
        match known(&weapon["weapon_type"]).as_str() {
            Some("AXE" | "CLUB" | "SWORD" | "FIST") => add(Stat::AttackEquipment, attack, false),
            Some("DISTANCE") if known(&weapon["ammunition"]).is_null() => {
                add(Stat::AttackEquipment, attack + elemental, false);
            }
            _ => missing.push("weapon-loadout"),
        }
    }
    // Crystal player.cpp:646–655 combines base and extra defence for a
    // weapon. An absent extra value contributes zero (inspection.rs:474–486).
    if let Some(defence) = points(weapon, "defense") {
        let shield = matches!(
            known(&weapon["weapon_type"]).as_str(),
            Some("SHIELD" | "SPELLBOOK")
        );
        let extra = if shield {
            0
        } else {
            points(weapon, "extra_defense").unwrap_or_default()
        };
        add(Stat::DefenceEquipment, (defence + extra).max(0), false);
    }
    for resistance in list(known(&protection["resistances"])) {
        let stat = match resistance["kind"].as_str() {
            Some("PHYSICAL") => Stat::AbsorbPhysical,
            Some("FIRE") => Stat::AbsorbFire,
            Some("EARTH") => Stat::AbsorbEarth,
            Some("ENERGY") => Stat::AbsorbEnergy,
            Some("ICE") => Stat::AbsorbIce,
            Some("HOLY") => Stat::AbsorbHoly,
            Some("DEATH") => Stat::AbsorbDeath,
            _ => {
                missing.push("resistance-kind");
                continue;
            }
        };
        if let Some(ppm) = percent(known(&resistance["percent"])) {
            add(stat, ppm, true);
        } else {
            missing.push("resistance-percent");
        }
    }
    for modifier in list(known(&item.group("skill_modifiers")["modifiers"])) {
        let kind = modifier["kind"].as_str().unwrap_or("unknown-modifier");
        let param = known(&modifier["parameter"]);
        let value = &param["value"];
        match (kind, param["kind"].as_str()) {
            ("SPEED", Some("SIGNED_POINTS")) => speed += value.as_i64().unwrap_or_default(),
            ("SKILL_SHIELD", Some("SIGNED_POINTS")) => {
                add(
                    Stat::ShieldingSkill,
                    value.as_i64().unwrap_or_default(),
                    true,
                );
            }
            ("MANTRA", Some("SIGNED_POINTS")) => {
                add(Stat::Mantra, value.as_i64().unwrap_or_default(), true);
            }
            ("CRITICAL_HIT_CHANCE", Some("RATIONAL_PERCENT")) => {
                add(
                    Stat::CriticalChanceEquipment,
                    percent(value).unwrap_or_default(),
                    true,
                );
            }
            ("CRITICAL_HIT_DAMAGE", Some("RATIONAL_PERCENT")) => {
                add(
                    Stat::CriticalDamageEquipment,
                    percent(value).unwrap_or_default(),
                    true,
                );
            }
            ("ELEMENTAL_BOND", Some("ELEMENT")) if value.is_string() => {}
            _ => missing.push(kind),
        }
    }
    (checks, speed, missing)
}

async fn stats(
    client: &mut DevClientSession,
    checks: &[StatCheck],
) -> TestResult<Vec<Option<i64>>> {
    let mut observed = vec![None; checks.len()];
    for tab in [
        CharacterTab::Offence,
        CharacterTab::Defence,
        CharacterTab::Misc,
    ] {
        if checks.iter().any(|check| check.stat.tab() == tab) {
            let page = full_row_c03::tab(client, tab).await?;
            for (i, check) in checks
                .iter()
                .enumerate()
                .filter(|(_, check)| check.stat.tab() == tab)
            {
                observed[i] = page.get(check.stat);
            }
        }
    }
    Ok(observed)
}

fn check_stats(
    checks: &[StatCheck],
    baseline: &[Option<i64>],
    observed: &[Option<i64>],
) -> TestResult {
    for ((check, baseline), observed) in checks.iter().zip(baseline).zip(observed) {
        let expected = if check.relative {
            baseline.and_then(|before| before.checked_add(check.expected))
        } else {
            Some(check.expected)
        };
        if expected.is_none() || expected != *observed {
            return Err(format!(
                "{:?}-expected-{expected:?}-observed-{observed:?}",
                check.stat
            )
            .into());
        }
    }
    Ok(())
}

async fn speed_trial(
    client: &mut DevClientSession,
    world: &BundleWorld,
    build: Build,
    bonus: i64,
) -> TestResult {
    use oteryn_gameplay_characters::speed::{StepSpeedTable, effective_speed, player_base_speed};
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.protection_zone())
            && full_e2ef5::corridor(world, at).is_some()
    })
    .ok_or("speed-corridor-unreachable")?;
    walk(client, world, &path).await?;
    let at = tile(client.world_spatial().actor_position)?;
    let cells = full_e2ef5::corridor(world, at).ok_or("speed-corridor-missing")?;
    // characters/src/speed.rs: the production logarithmic step-speed table.
    let base = player_base_speed(build.level).ok_or("speed-level-invalid")?;
    let expected = StepSpeedTable::embedded()
        .and_then(|table| {
            table.step_duration(
                effective_speed(i64::from(base), bonus, 0),
                world.ground_speed(at),
            )
        })
        .ok_or("speed-oracle-missing")?;
    let observed = full_e2ef5::cadence(client, &cells).await?;
    if observed.abs_diff(expected) > Duration::from_millis(25) {
        return Err(format!("speed-expected-{expected:?}-observed-{observed:?}").into());
    }
    Ok(())
}

async fn equipment(
    client: &mut DevClientSession,
    world: &BundleWorld,
    build: Build,
    entry: ItemEntry,
    item: &Item,
    active: &Item,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    rows: &mut Rows<'_>,
) -> TestResult {
    let pattern = item.patterns().first().ok_or("equipment-pattern-missing")?;
    let slot = slot(pattern).ok_or("equipment-slot-has-no-wire-destination")?;
    let active_ref = item
        .target("EQUIP")
        .map_or(Ok(entry.item_definition_ref), |key| reference(index, key))?;
    let (checks, speed, missing) = stat_checks(active);
    let baseline = stats(client, &checks).await;
    let baseline_speed = if speed == 0 {
        Ok(())
    } else {
        speed_trial(client, world, build, 0).await
    };
    let before = client.inventory().ok_or("equip-inventory-missing")?.clone();
    let destination = Destination::Equipment(slot);
    let reply = client
        .session_mut()
        .move_item_counted(oteryn_dev_client::client_runtime::inventory::MoveRequest {
            source: entry.handle,
            destination,
            count: None,
        })
        .await?;
    if reply.outcome != Move::Moved {
        let refusal = format!(
            "ITEM_MOVE-source-{}-destination-{destination:?}-expected-Moved-observed-{reply:?}",
            entry.handle
        );
        if reply.outcome != Move::SlotMismatch || client.inventory() != Some(&before) {
            return Err(refusal.into());
        }
        // Crystal has both slot moves and definition hotkeys (game.cpp:3703).
        // Preserve the explicit refusal even if the alternate route establishes custody.
        let hotkey = client
            .session_mut()
            .equip_item_definition(entry.item_definition_ref)
            .await;
        if !hotkey.as_ref().is_ok_and(|reply| {
            reply.status == CommandStatus::Accepted && reply.outcome == Move::Moved
        }) {
            return Err(format!(
                "{refusal}; HOTKEY_EQUIP-definition-{}-observed-{hotkey:?}",
                entry.item_definition_ref
            )
            .into());
        }
        rows.emit("equip", Err(refusal.into()));
    }
    let worn = observe(client, |client| {
        client
            .inventory()?
            .equipment
            .iter()
            .find(|worn| worn.slot == slot && worn.item.item_definition_ref == active_ref)
            .map(|worn| worn.item)
    })
    .await?;
    if rows.pending.contains(&"equip") {
        rows.emit("equip", Ok(()));
    }
    let attributes = async {
        elemental_bond(client, worn, active).await?;
        let baseline = baseline
            .as_ref()
            .map_err(|error| format!("baseline-{error}"))?;
        check_stats(&checks, baseline, &stats(client, &checks).await?)?;
        baseline_speed?;
        if speed != 0 {
            speed_trial(client, world, build, speed).await?;
        }
        if !missing.is_empty() {
            return Err(format!("attribute-oracle-unavailable-{}", missing.join(",")).into());
        }
        Ok(())
    }
    .await;
    rows.emit("attributes", attributes);
    let inactive_ref = active
        .target("DEEQUIP")
        .map_or(Ok(entry.item_definition_ref), |key| reference(index, key))?;
    if let Err(error) = move_item(client, worn.handle, Destination::MainBackpack, Move::Moved).await
    {
        rows.emit("unequip", Err(error));
        return Err("unequip".into());
    }
    let returned = observe(client, |client| {
        (!client
            .inventory()?
            .equipment
            .iter()
            .any(|worn| worn.slot == slot))
        .then(|| carried(client, inactive_ref))
        .flatten()
    })
    .await;
    let returned = match returned {
        Ok(entry) => entry,
        Err(error) => {
            rows.emit("unequip", Err(error));
            return Err("unequip".into());
        }
    };
    rows.emit("unequip", Ok(()));
    let removed = async {
        elemental_bond(client, returned, item).await?;
        if stats(client, &checks).await? != baseline? {
            return Err("unequip-did-not-restore-combat-baseline".into());
        }
        if speed != 0 {
            speed_trial(client, world, build, 0).await?;
        }
        if !missing.is_empty() {
            return Err(
                format!("attribute-removal-oracle-unavailable-{}", missing.join(",")).into(),
            );
        }
        Ok(())
    }
    .await;
    rows.emit("attributes_removed", removed);
    if returned.item_definition_ref != entry.item_definition_ref {
        drop_carried(client, returned.item_definition_ref).await?;
    }
    Ok(())
}

// Crystal item.cpp:3257–3262 and the native inspection projection
// (gameplay_transport/inspection.rs:576) expose the bond as a named element.
async fn elemental_bond(
    client: &mut DevClientSession,
    entry: ItemEntry,
    item: &Item,
) -> TestResult {
    use oteryn_protocol_oteryn::inspection as wire;
    let bonds: Vec<_> = list(known(&item.group("skill_modifiers")["modifiers"]))
        .iter()
        .filter(|m| m["kind"] == "ELEMENTAL_BOND")
        .filter_map(|m| known(&m["parameter"])["value"].as_str())
        .collect();
    if bonds.is_empty() {
        return Ok(());
    }
    let payload = wire::encode_request(wire::Request::Item(entry.handle))
        .map_err(|error| format!("elemental-bond-inspection-encode-{error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_INSPECTION_V1,
            wire::COMMAND_TYPE_INSPECTION,
            &payload,
        )
        .await?;
    let reply = wire::decode_reply(&response.payload)
        .map_err(|error| format!("elemental-bond-inspection-decode-{error:?}"))?;
    let [observed] = reply.items.as_slice() else {
        return Err(format!(
            "elemental-bond-inspection-handle-{}-status-{}",
            entry.handle, reply.status
        )
        .into());
    };
    let matching = observed
        .attributes
        .iter()
        .filter(|(label, _)| label == "Elemental Bond");
    if response.status != CommandStatus::Accepted
        || reply.status != 1
        || observed.definition_ref != entry.item_definition_ref.get()
        || !matching
            .map(|(_, value)| value.to_ascii_uppercase())
            .eq(bonds.iter().copied())
    {
        return Err(format!("elemental-bond-expected-{bonds:?}-observed-{observed:?}").into());
    }
    Ok(())
}

fn reference(
    index: &crate::content::item_ref::ItemDefinitionIndex,
    key: &str,
) -> TestResult<NonZeroU32> {
    index
        .revision(key)
        .and_then(|revision| index.definition_ref("Item", key, revision))
        .ok_or_else(|| "item-definition-has-no-wire-reference".into())
}

fn food_matches(before: i32, after: i32, seconds: u64, elapsed: Duration) -> bool {
    let maximum = i128::from(before) + i128::from(seconds) * 1000;
    let observed = i128::from(after);
    before >= 0
        && observed > 0
        && observed <= maximum
        && maximum - observed <= elapsed.as_millis().min(i128::MAX as u128) as i128 + 1000
}

const AMMO_GEAR: [&str; 3] = [
    "oteryn:item.tibia.i35562", // Quiver
    "oteryn:item.tibia.i3350",  // Bow
    "oteryn:item.tibia.i3349",  // Crossbow
];

fn ammunition(item: &Item) -> bool {
    known(&item.group("weapon")["weapon_type"]).as_str() == Some("AMMUNITION")
}

async fn ammo_attack(
    client: &mut DevClientSession,
    world: &BundleWorld,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    entry: ItemEntry,
) -> TestResult {
    use oteryn_dev_client::client_runtime::hotkey_equip::equip_definition;
    use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
    let quiver = reference(index, AMMO_GEAR[0])?;
    for definition in [quiver, entry.item_definition_ref] {
        let moved = equip_definition(client.session_mut(), definition).await?;
        if moved != Move::Moved {
            return Err(format!("ammo-quiver-move-{definition}-{moved:?}").into());
        }
    }
    let quiver_handle = observe(client, |client| {
        client
            .inventory()?
            .equipment
            .iter()
            .find(|worn| worn.item.item_definition_ref == quiver)
            .map(|worn| worn.item.handle)
    })
    .await?;
    let target = full_e2ef5::wild_rat(client, world, &[]).await?.entity;
    let quiver_count = |client: &mut DevClientSession| {
        let views = client.session_mut().container_views()?;
        let view = views
            .views
            .iter()
            .find(|view| view.container_handle == quiver_handle)?;
        Some(
            view.entries
                .iter()
                .filter(|ammo| ammo.item_definition_ref == entry.item_definition_ref)
                .map(|ammo| ammo.count.get())
                .sum::<u32>(),
        )
    };
    let opened = client.use_item(quiver_handle).await?;
    if opened.status != CommandStatus::Accepted || opened.disposition != Use::Committed {
        return Err(format!("ammo-quiver-open-{opened:?}").into());
    }
    let count = observe(client, quiver_count).await?;
    if count == 0 {
        return Err("ammo-quiver-credit-absent".into());
    }
    for launcher in &AMMO_GEAR[1..] {
        let launcher = reference(index, launcher)?;
        if equip_definition(client.session_mut(), launcher).await? != Move::Moved {
            continue;
        }
        let attack = client.session_mut().attack_target(Some(&target)).await?;
        if attack.disposition == AttackIntentDisposition::Ok {
            let debit = observe(client, |client| {
                quiver_count(client).filter(|&remaining| remaining < count)
            })
            .await;
            client.session_mut().attack_target(None).await?;
            if debit.is_ok() {
                return Ok(());
            }
        }
    }
    Err("ammo-ranged-attack-did-not-debit-quiver".into())
}

// RUNE_USE targets come from the native spell book; the generic direct USE
// cannot exercise a creature or tile rune (rune_use_bridge.rs).
async fn rune_probe(
    client: &mut DevClientSession,
    world: &BundleWorld,
    entry: ItemEntry,
) -> TestResult<(Use, u64)> {
    let spell = native_spell_book()?
        .rune(entry.item_definition_ref.get())
        .ok_or("rune-native-definition-unavailable")?;
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    })
    .ok_or("rune-no-reachable-combat-tile")?;
    walk(client, world, &path).await?;
    let creature = if spell.needs_target && !spell.self_target {
        Some(full_e2ef5::wild_rat(client, world, &[]).await?.entity)
    } else {
        None
    };
    let position = if creature.is_none() && !spell.self_target {
        let at = client.world_spatial().actor_position;
        let from = tile(at)?;
        let target = [
            StepDirection::North,
            StepDirection::East,
            StepDirection::South,
            StepDirection::West,
        ]
        .into_iter()
        .filter_map(|direction| destination(world, from, direction))
        .find(|cell| {
            cell.floor == from.floor
                && world
                    .base()
                    .tile(cell.x, cell.y, cell.floor)
                    .is_some_and(|tile| !tile.protection_zone())
        })
        .ok_or("rune-no-adjacent-combat-target")?;
        Some(ActorPosition {
            x: i32::from(target.x),
            y: i32::from(target.y),
            floor: at.floor,
        })
    } else {
        None
    };
    full_e2ef5::rune_use(client, entry.item_definition_ref, creature, position).await
}

async fn use_effect(
    client: &mut DevClientSession,
    world: &BundleWorld,
    entry: ItemEntry,
    item: &Item,
    index: &crate::content::item_ref::ItemDefinitionIndex,
) -> TestResult {
    if !item.attributes["rune_consumption"].is_null() {
        let (disposition, _) = rune_probe(client, world, entry).await?;
        if disposition != Use::Committed {
            return Err(format!("targeted-rune-USE-{disposition:?}").into());
        }
        observe(client, |client| {
            let count =
                carried(client, entry.item_definition_ref).map_or(0, |entry| entry.count.get());
            (count == entry.count.get() - 1).then_some(())
        })
        .await?;
        return Err("targeted-rune-debit-observed-effect-oracle-unavailable".into());
    }
    let text = item.group("readable_writeable");
    if known(&text["readable"]).as_bool() == Some(true) {
        use oteryn_protocol_oteryn::item_inscription as wire;
        let payload = wire::Request {
            item_handle: entry.handle.get(),
        }
        .encode()
        .ok_or("inscription-request-encode")?;
        let reply = client
            .session_mut()
            .gated_command(
                wire::CAPABILITY_ITEM_INSCRIPTION_READ_V1,
                wire::COMMAND_TYPE_ITEM_INSCRIPTION_READ,
                &payload,
            )
            .await?;
        let observed = wire::Reply::decode(&reply.payload)
            .ok_or_else(|| format!("inscription-reply-unavailable-{:?}", reply.status))?;
        let max = known(&text["max_text_length"]).as_u64();
        let writable = known(&text["writeable"]).as_bool();
        if reply.status != CommandStatus::Accepted
            || max != Some(u64::from(observed.max_length))
            || writable.is_some_and(|value| value != observed.writable)
        {
            return Err(format!("inscription expected max={max:?} writable={writable:?}; observed status={:?} max={} writable={}", reply.status, observed.max_length, observed.writable).into());
        }
        return Ok(());
    }
    let consumption = item.group("consumption");
    let mut food = None;
    let mut potion = None;
    if let Some(seconds) = consumption["value"]["regeneration_seconds"].as_u64() {
        food = Some((
            full_row_c09::food_remaining(client, None).await?,
            seconds,
            std::time::Instant::now(),
        ));
    } else if consumption["kind"] == "POTION" {
        let restores = list(&consumption["value"]["restores"]);
        if restores
            .iter()
            .any(|restore| restore["resource"] == "HEALTH")
        {
            full_row_c09::damage_and_spend_mana(client, world).await?;
        } else {
            full_row_c09::spend_mana(client).await?;
        }
        potion = client.actor_vitals().copied();
    }
    let reply = client.use_item(entry.handle).await?;
    if reply.status != CommandStatus::Accepted || reply.disposition != Use::Committed {
        return Err(format!("use-{reply:?}").into());
    }
    if !consumption.is_null() {
        observe(client, |client| {
            let count =
                carried(client, entry.item_definition_ref).map_or(0, |entry| entry.count.get());
            (count == entry.count.get() - 1).then_some(())
        })
        .await?;
    }
    // The rest-status owner reads the live food condition (rest_status.rs:90);
    // ITEM-USE's food and potion plans are keyed by content (item_use.rs:677).
    if let Some((before, seconds, started)) = food {
        let after =
            full_row_c09::food_remaining(client, Some(reply.result_server_sequence)).await?;
        if !food_matches(before, after, seconds, started.elapsed()) {
            return Err(
                format!("food-duration-before-{before}-added-{seconds}s-after-{after}").into(),
            );
        }
    } else if let Some(before) = potion {
        let after = observe(client, |client| {
            client
                .actor_vitals()
                .copied()
                .filter(|vitals| *vitals != before)
        })
        .await?;
        for restore in list(&consumption["value"]["restores"]) {
            let (before, after, max) = match restore["resource"].as_str() {
                Some("HEALTH") => (before.health, after.health, after.max_health),
                Some("MANA") => (before.mana, after.mana, after.max_mana),
                _ => return Err("potion-resource-oracle-unavailable".into()),
            };
            let min = u32::try_from(restore["min"].as_u64().ok_or("restore-min-missing")?)?;
            let high = u32::try_from(restore["max"].as_u64().ok_or("restore-max-missing")?)?;
            if before == max
                || !(before.saturating_add(min).min(max)..=before.saturating_add(high).min(max))
                    .contains(&after)
            {
                return Err(format!(
                    "potion-restore-before-{before}-after-{after}-max-{max}-roll-{min}-{high}"
                )
                .into());
            }
        }
        if let Some(key) = known(&consumption["value"]["empty_flask"])["key"].as_str() {
            let flask = reference(index, key)?;
            observe(client, |client| carried(client, flask)).await?;
        }
    } else if let Some(key) = item.target("USE") {
        let target = reference(index, key)?;
        observe(client, |client| {
            (carried(client, entry.item_definition_ref).is_none())
                .then(|| carried(client, target))
                .flatten()
        })
        .await?;
        drop_carried(client, target).await?;
    } else {
        return Err("use-effect-oracle-unavailable".into());
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Action {
    Equip(Option<EquipmentSlot>),
    Use,
}

fn min_level(required: &Value) -> Option<u64> {
    points(required, "level")
        .or_else(|| points(required, "min_level"))
        .and_then(|value| u64::try_from(value).ok())
}

fn restriction_builds(required: &Value) -> (Option<Build>, Option<Build>) {
    let Some(allowed) = Build::supported(required) else {
        return (None, None);
    };
    let low = Build {
        level: LOW,
        ..allowed
    };
    let wrong = (0..VOCATIONS.len())
        .map(|vocation| Build {
            vocation,
            level: HIGH,
        })
        .find(|build| !build.permits(required));
    ((!low.permits(required)).then_some(low), wrong)
}

async fn restriction(
    fixtures: &mut Fixtures<'_>,
    key: &str,
    reference: NonZeroU32,
    build: Build,
    action: Action,
) -> TestResult {
    let mut client = fixtures.seed(key, build).await?;
    let result = async {
        let entry = carried(&client, reference).ok_or("restricted-item-not-carried")?;
        let before = client
            .inventory()
            .ok_or("restriction-inventory-missing")?
            .clone();
        match action {
            Action::Equip(slot) => {
                move_item(
                    &mut client,
                    entry.handle,
                    Destination::Equipment(slot.ok_or("equipment-slot-has-no-wire-destination")?),
                    Move::RequirementNotMet,
                )
                .await?;
            }
            Action::Use => {
                let reply = client.use_item(entry.handle).await?;
                if reply.disposition != Use::RequirementNotMet {
                    return Err(format!("use-expected-RequirementNotMet-observed-{reply:?}").into());
                }
            }
        }
        if client.inventory() != Some(&before) {
            return Err("restriction-mutated-inventory".into());
        }
        Ok(())
    }
    .await;
    let cleanup = drop_carried(&mut client, reference).await;
    let (scope, index) = fixtures.fixture(build);
    let released = release_fixture(client, scope, index).await;
    result.and(cleanup).and(released)
}

fn restrictions(item: &Item) -> Vec<(&'static str, &'static str, &Value, Action)> {
    let mut checks = Vec::new();
    if let Some(pattern) = item.patterns().first() {
        checks.push((
            "equip_level",
            "equip_vocation",
            pattern,
            Action::Equip(slot(pattern)),
        ));
    }
    if item.usable() {
        checks.push((
            "use_level",
            "use_vocation",
            item.group("use_requirements"),
            Action::Use,
        ));
    }
    checks
}

async fn sweep_item(
    fixtures: &mut Fixtures<'_>,
    world: &BundleWorld,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    items: &BTreeMap<String, Item>,
    corpse_sources: &BTreeMap<String, String>,
    key: &str,
    item: &Item,
    rows: &mut Rows<'_>,
) {
    let reference = match reference(index, key) {
        Ok(reference) => reference,
        Err(error) => {
            rows.emit("seed", Err(error));
            rows.blocked("seed");
            return;
        }
    };
    if matches!(key, "oteryn:item.tibia.i10210" | "oteryn:item.tibia.i10211") {
        if let Err(error) = rack(fixtures, reference, item, rows).await {
            rows.emit("seed", Err(error));
        }
        rows.blocked("map-custody");
        return;
    }
    if !item.group("bed").is_null() {
        let scope = fixtures.scopes[0];
        match connect_character(scope, 0x71, 0).await {
            Ok(mut client) => {
                let seeded = full_row_c10::sweep_bed(&mut client, world, reference, rows).await;
                if let Err(error) = seeded {
                    rows.emit("seed", Err(error));
                }
                rows.emit("cleanup", release_fixture(client, scope, 0).await);
            }
            Err(error) => rows.emit("seed", Err(error)),
        }
        rows.blocked("house-custody");
        return;
    }
    if let Some(creature) = corpse_sources
        .get(key)
        .filter(|_| item.ground_only && item.capacity().is_some())
    {
        if let Err(error) = corpse(fixtures, world, index, creature, reference, item, rows).await {
            rows.emit("seed", Err(error));
        }
        rows.blocked("corpse-custody");
        return;
    }
    if item.ground_only {
        let result = match index.revision(key) {
            Some(revision) => {
                ground_container(fixtures, world, index, key, revision, reference, item, rows).await
            }
            None => Err("ground-fixture-revision-unavailable".into()),
        };
        if let Err(error) = result {
            rows.emit("seed", Err(error));
        }
        rows.blocked("ground-custody");
        return;
    }
    let positive = item
        .patterns()
        .first()
        .unwrap_or_else(|| item.group("use_requirements"));
    let ammo_gear = ammunition(item) && rows.pending.contains(&"use");
    let paladin = Build {
        vocation: 1,
        level: HIGH,
    };
    let build = if ammo_gear && paladin.permits(positive) {
        Some(paladin)
    } else {
        Build::supported(positive)
    };
    let Some(build) = build else {
        rows.emit("seed", Err("eligible-build-unavailable".into()));
        rows.blocked("seed");
        return;
    };
    let gear = if ammo_gear { AMMO_GEAR.as_slice() } else { &[] };
    let mut client = match fixtures.seed_with_gear(key, build, gear).await {
        Ok(client) => client,
        Err(error) => {
            // System errors may contain external transport detail. Report the classified
            // fixture failure and keep process diagnostics in the private node log.
            let message = if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound)
            {
                "fixture-launcher-not-found"
            } else {
                "fixture-seed-or-admission-failed"
            };
            let diagnostic = error.to_string();
            let message = if diagnostic.starts_with("fixture-") {
                diagnostic.as_str()
            } else {
                message
            };
            rows.emit("seed", Err(message.to_owned().into()));
            rows.blocked("seed");
            return;
        }
    };
    let Some(entry) = carried(&client, reference) else {
        rows.emit("seed", Err("fresh-admission-missing-seeded-item".into()));
        let (scope, index) = fixtures.fixture(build);
        rows.emit("cleanup", release_fixture(client, scope, index).await);
        rows.blocked("seed");
        return;
    };
    rows.emit("seed", Ok(()));
    if rows.pending.as_slice() == ["cleanup"] {
        let cleanup = drop_carried(&mut client, reference).await;
        let (scope, index) = fixtures.fixture(build);
        let released = release_fixture(client, scope, index).await;
        rows.emit("cleanup", cleanup.and(released));
        return;
    }
    if !item.group("container").is_null() {
        rows.emit(
            "container",
            container(&mut client, entry.handle, item.capacity()).await,
        );
    }
    if !item.patterns().is_empty() && !ammo_gear {
        let result = match item.target("EQUIP") {
            Some(key) => items.get(key).ok_or("equip-transform-definition-missing"),
            None => Ok(item),
        };
        let result = match result {
            Ok(active) => {
                equipment(&mut client, world, build, entry, item, active, index, rows).await
            }
            Err(error) => Err(error.into()),
        };
        if let Err(error) = result {
            if rows.pending.contains(&"equip") {
                rows.emit("equip", Err(error));
            }
            for step in ["attributes", "unequip", "attributes_removed"] {
                if rows.pending.contains(&step) {
                    rows.emit(step, Err("blocked-by-equipment-transition".into()));
                }
            }
        }
    }
    if ammo_gear {
        rows.emit("use", ammo_attack(&mut client, world, index, entry).await);
        for step in ["equip", "attributes", "unequip", "attributes_removed"] {
            if rows.pending.contains(&step) {
                rows.emit(step, Err("ammo-quiver-equipment-oracle-unavailable".into()));
            }
        }
    } else if item.usable() {
        let result = match carried(&client, reference) {
            Some(entry) => use_effect(&mut client, world, entry, item, index).await,
            None => Err("use-blocked-by-item-custody".into()),
        };
        rows.emit("use", result);
    }
    let mut cleanup = drop_carried(&mut client, reference).await;
    for key in gear {
        match self::reference(index, key) {
            Ok(definition) => cleanup = cleanup.and(drop_carried(&mut client, definition).await),
            Err(error) => cleanup = cleanup.and(Err(error)),
        }
    }
    let (scope, index) = fixtures.fixture(build);
    let release = release_fixture(client, scope, index).await;
    rows.emit("cleanup", cleanup.and(release));
    for (level, vocation, required, action) in restrictions(item) {
        let (low, wrong) = restriction_builds(required);
        for (name, build) in [(level, low), (vocation, wrong)] {
            if rows.pending.contains(&name) {
                let result = match build {
                    Some(build) => restriction(fixtures, key, reference, build, action).await,
                    None => Err("restriction-fixture-unavailable".into()),
                };
                rows.emit(name, result);
            }
        }
    }
}

fn steps(item: &Item) -> Vec<&'static str> {
    let mut steps = vec!["seed"];
    if !item.group("container").is_null() {
        steps.push("container");
    }
    if !item.patterns().is_empty() {
        steps.extend(["equip", "attributes", "unequip", "attributes_removed"]);
    }
    if item.usable() || ammunition(item) {
        steps.push("use");
    }
    steps.push("cleanup");
    for (level, vocation, required, _) in restrictions(item) {
        if min_level(required).is_some_and(|level| level > u64::from(LOW)) {
            steps.push(level);
        }
        let allowed = list(known(&required["vocations"]));
        if !allowed.is_empty()
            && !(0..VOCATIONS.len()).all(|index| allowed_vocation(index, allowed))
        {
            steps.push(vocation);
        }
    }
    steps
}

pub(super) async fn run<'a>(scopes: [&'a SeamClients<'a>; 5], world: &BundleWorld) -> TestResult {
    let (start, len) = match std::env::var(BATCH_VARIABLE) {
        Ok(value) => batch(Some(&value)),
        Err(std::env::VarError::NotPresent) => batch(None),
        Err(error) => Err(error.into()),
    }?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let items = census(&root)?;
    let corpse_sources = corpse_sources(&root)?;
    let mut requested = match std::env::var_os("OTERYN_E2E_ITEM_SWEEP_KEYS") {
        Some(path) => Some(
            std::fs::read_to_string(path)?
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(|line| format!("oteryn:item.{line}"))
                .collect::<std::collections::BTreeSet<_>>(),
        ),
        None => None,
    };
    let seed_only =
        std::env::var_os("OTERYN_E2E_ITEM_SWEEP_SEED_ONLY").is_some_and(|value| value == "1");
    if seed_only && requested.is_none() {
        return Err("seed-only item sweep requires OTERYN_E2E_ITEM_SWEEP_KEYS".into());
    }
    let mut entries = Vec::new();
    for (key, item) in items.iter().filter(|(_, item)| item.in_sweep()) {
        if requested.as_mut().is_none_or(|keys| keys.remove(key)) {
            entries.push((key, item));
        }
    }
    if let Some(unknown) = requested.filter(|keys| !keys.is_empty()) {
        return Err(
            format!("item sweep manifest contains keys outside the census: {unknown:?}").into(),
        );
    }
    let total = entries.len();
    evidence(&format!(
        "item_sweep census definitions={} total={total} mode=seeded",
        items.len()
    ));
    if start >= total {
        return Err(format!("{BATCH_VARIABLE} start {start} is beyond {total} entries").into());
    }
    let end = start.saturating_add(len).min(total);
    let index = provisioning::item_index()?;
    let mut fixtures = Fixtures {
        scopes,
        prepared: [false; 10],
        socket: required("NODE_BOOT_CONTROL_SOCKET")?,
    };
    let mut cleanup_db = sqlx::PgConnection::connect(scopes[0].url).await?;
    let mut tally = Tally::default();
    for &(key, item) in &entries[start..end] {
        verdict::Harness::global().progress("item-sweep-entry");
        let mut rows = Rows {
            key: key.strip_prefix("oteryn:item.").unwrap_or(key),
            pending: if seed_only {
                vec!["seed", "cleanup"]
            } else {
                steps(item)
            },
            counts: Tally::default(),
            lap: std::time::Instant::now(),
        };
        let watermark: i64 = sqlx::query_scalar(
            "SELECT COALESCE(max(stack_ordinal),0) FROM game_item_ground_locations",
        )
        .fetch_one(&mut cleanup_db)
        .await?;
        let result = tokio::time::timeout(
            Duration::from_secs(if seed_only { 240 } else { 1200 }),
            Box::pin(sweep_item(
                &mut fixtures,
                world,
                &index,
                &items,
                &corpse_sources,
                key,
                item,
                &mut rows,
            )),
        )
        .await;
        rows.blocked(if result.is_ok() {
            "incomplete-step"
        } else {
            "entry-deadline"
        });
        let cleared = clear_fixture_drops(&mut cleanup_db, &fixtures, watermark).await;
        if cleared.is_err() {
            rows.emit("ground_cleanup", cleared);
        }
        tally.pass += rows.counts.pass;
        tally.fail += rows.counts.fail;
    }
    evidence(&format!(
        "item_sweep summary total={total} start={start} len={len} selected={} rows_pass={} rows_fail={} mode=seeded seed_only={seed_only}",
        end - start,
        tally.pass,
        tally.fail,
    ));
    if tally.fail > 0 {
        return Err(format!("item_sweep had {} FAIL rows", tally.fail).into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_requires_two_unsigned_decimal_fields_and_a_positive_length() -> TestResult {
        assert_eq!(batch(None)?, (0, 3));
        assert_eq!(batch(Some("0:20"))?, (0, 20));
        assert_eq!(batch(Some("21:7"))?, (21, 7));
        for invalid in [
            "",
            "0",
            ":3",
            "1:",
            "0:0",
            "-1:2",
            "1:+2",
            "1:2:3",
            " 1:2",
            "9999999999999999999999999:1",
        ] {
            assert!(batch(Some(invalid)).is_err(), "{invalid}");
        }
        Ok(())
    }

    #[test]
    fn content_oracles_cover_sword_food_and_restricted_equipment() -> TestResult {
        let items = census(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))?;
        let sword = &items["oteryn:item.tibia.i3264"];
        assert!(sword.in_sweep());
        assert_eq!(
            sword.patterns().first().and_then(slot),
            Some(EquipmentSlot::RightHand)
        );
        let (checks, _, _) = stat_checks(sword);
        assert!(
            checks
                .iter()
                .any(|check| check.stat == Stat::AttackEquipment && check.expected == 14)
        );
        assert!(food_matches(
            42_000,
            101_900,
            60,
            Duration::from_millis(200)
        ));
        assert!(!food_matches(
            42_000,
            42_000,
            60,
            Duration::from_millis(200)
        ));
        let pattern = serde_json::json!({
            "level": {"state": "KNOWN", "value": 60},
            "vocations": {"state": "KNOWN", "value": ["PALADIN"]}
        });
        let (low, wrong) = restriction_builds(&pattern);
        let allowed = Build::supported(&pattern).ok_or("missing allowed build")?;
        assert!(allowed.permits(&pattern));
        assert_eq!(
            low.map(|build| (build.vocation, build.level)),
            Some((allowed.vocation, LOW))
        );
        assert_eq!(wrong.map(|build| build.level), Some(HIGH));
        assert!(wrong.is_some_and(|build| build.vocation != allowed.vocation));
        Ok(())
    }
}
