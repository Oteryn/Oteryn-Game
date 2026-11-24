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

struct Rows<'a> {
    key: &'a str,
    pending: Vec<&'static str>,
    counts: Tally,
    lap: std::time::Instant,
}

impl Rows<'_> {
    fn emit(&mut self, step: &str, result: TestResult) {
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
    async fn seed(&mut self, key: &str, build: Build) -> TestResult<DevClientSession> {
        let index = build.vocation * 2 + usize::from(build.level == HIGH);
        let scope = self.scopes[index / 2];
        let mut client = connect_character(scope, 0x71, index % 2).await?;
        let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
        let script = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/qualification/node_boot/provision.py");
        let socket = self.socket.clone();
        let key = key.to_owned();
        let level = if self.prepared[index] { 0 } else { build.level };
        let status = tokio::task::spawn_blocking(move || {
            crate::qualification::provision_python()
                .arg(script)
                .args([
                    "--socket",
                    &socket,
                    "--session",
                    &session,
                    "--item",
                    VOCATIONS[build.vocation],
                    &level.to_string(),
                    &key,
                ])
                .status()
        })
        .await;
        // Release even after fixture refusal. Dropping an admitted socket instead
        // would park the Character in reconnect grace and block the next item.
        let released = release(&mut client).await;
        if !status??.success() {
            return Err("fixture-item-preparation-refused".into());
        }
        released?;
        drop(client);
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
            let _ = release(&mut client).await;
            return Err(error);
        }
        self.prepared[index] = true;
        Ok(client)
    }
}

async fn release(client: &mut DevClientSession) -> TestResult {
    let (result, _, _, _) = logout_retrying_busy(client).await?;
    if result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        Ok(())
    } else {
        Err(format!("fixture-logout-{result:?}").into())
    }
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
        return Err(format!("ITEM_MOVE-expected-{expected:?}-observed-{reply:?}").into());
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
    if let Some(item) = carried(client, reference) {
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
        observe(client, |c| carried(c, reference).is_none().then_some(())).await?;
    }
    Ok(())
}

async fn container(
    client: &mut DevClientSession,
    entry: ItemEntry,
    capacity: Option<u64>,
) -> TestResult {
    let reply = client.use_item(entry.handle).await?;
    if reply.status != CommandStatus::Accepted || reply.disposition != Use::Committed {
        return Err(format!("container-USE-{reply:?}").into());
    }
    let observed = observe(client, |client| {
        client
            .session_mut()
            .container_views()?
            .views
            .iter()
            .find(|view| view.container_handle == entry.handle)
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
    if let (Some(defence), Some(extra)) =
        (points(weapon, "defense"), points(weapon, "extra_defense"))
    {
        let shield = matches!(
            known(&weapon["weapon_type"]).as_str(),
            Some("SHIELD" | "SPELLBOOK")
        );
        add(
            Stat::DefenceEquipment,
            (defence + if shield { 0 } else { extra }).max(0),
            false,
        );
    } else if points(weapon, "defense").is_some() {
        missing.push("hand-defence");
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
    // Use the normal client hotkey route so the effective destination is observed.
    let reply = client
        .session_mut()
        .equip_item_definition(entry.item_definition_ref)
        .await?;
    if reply.status != CommandStatus::Accepted || reply.outcome != Move::Moved {
        return Err(format!("equip-{reply:?}").into());
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
    rows.emit("equip", Ok(()));
    let attributes = async {
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

async fn use_effect(
    client: &mut DevClientSession,
    world: &BundleWorld,
    entry: ItemEntry,
    item: &Item,
    index: &crate::content::item_ref::ItemDefinitionIndex,
) -> TestResult {
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
    let released = release(&mut client).await;
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
    let positive = item
        .patterns()
        .first()
        .unwrap_or_else(|| item.group("use_requirements"));
    let Some(build) = Build::supported(positive) else {
        rows.emit("seed", Err("eligible-build-unavailable".into()));
        rows.blocked("seed");
        return;
    };
    let mut client = match fixtures.seed(key, build).await {
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
            rows.emit("seed", Err(message.into()));
            rows.blocked("seed");
            return;
        }
    };
    let Some(entry) = carried(&client, reference) else {
        rows.emit("seed", Err("fresh-admission-missing-seeded-item".into()));
        rows.emit("cleanup", release(&mut client).await);
        rows.blocked("seed");
        return;
    };
    rows.emit("seed", Ok(()));
    if !item.group("container").is_null() {
        rows.emit(
            "container",
            container(&mut client, entry, item.capacity()).await,
        );
    }
    if !item.patterns().is_empty() {
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
    if item.usable() {
        let result = match carried(&client, reference) {
            Some(entry) => use_effect(&mut client, world, entry, item, index).await,
            None => Err("use-blocked-by-item-custody".into()),
        };
        rows.emit("use", result);
    }
    let cleanup = drop_carried(&mut client, reference).await;
    let release = release(&mut client).await;
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
    if item.usable() {
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
    let entries: Vec<_> = items.iter().filter(|(_, item)| item.in_sweep()).collect();
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
    let mut tally = Tally::default();
    for &(key, item) in &entries[start..end] {
        verdict::Harness::global().progress("item-sweep-entry");
        let mut rows = Rows {
            key: key.strip_prefix("oteryn:item.").unwrap_or(key),
            pending: steps(item),
            counts: Tally::default(),
            lap: std::time::Instant::now(),
        };
        let result = tokio::time::timeout(
            Duration::from_secs(180),
            Box::pin(sweep_item(
                &mut fixtures,
                world,
                &index,
                &items,
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
        tally.pass += rows.counts.pass;
        tally.fail += rows.counts.fail;
    }
    evidence(&format!(
        "item_sweep summary total={total} start={start} len={len} selected={} rows_pass={} rows_fail={} mode=seeded",
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
