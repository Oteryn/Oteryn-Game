//! Opt-in over the native wire. Select with NODE_BOOT_E2E_ONLY=cyclopedia_sweep.
//! The census is independent of the replies. Bestiary ordinals use the content order
//! in crates/gameplay/dispatch/src/gameplay_transport/charm_progression.rs:91; boss thresholds
//! use crates/gameplay/domain/src/domain/bosstiary.rs:33. Item names come from definition semantics
//! as in crates/gameplay/dispatch/src/gameplay_transport/inspection.rs:362.
//! Map progress uses crates/gameplay/dispatch/src/gameplay_transport/cyclopedia.rs:168.
//! Weather uses the authored area anchors and the period owner at crates/gameplay/weather/src/lib.rs:375,
//! compared to the admission snapshot at crates/gameplay/dispatch/src/gameplay_transport/connection/serve/snapshot.rs:677.
use super::*;
use oteryn_dev_client::{DevClientSession, client_ui::cyclopedia_character as panel};
use oteryn_gameplay_weather::{WEATHER_SLOT_MS, WorldWeatherSource};
use oteryn_protocol_oteryn::{
    attack, bestiary::BestiaryRaceProgress, boss_encounter as boss, charm_progression as charm,
    cyclopedia as map, cyclopedia_character as character, inspection, world_weather as weather,
};
use serde::Deserialize;
use std::num::NonZeroU32;

const BATCH: &str = "OTERYN_E2E_CYCLOPEDIA_SWEEP_BATCH";

fn parse_batch(value: Option<&str>) -> TestResult<(usize, usize)> {
    let Some(text) = value else {
        return Ok((0, 3));
    };
    let number = |part: &str| {
        if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        part.parse::<usize>().ok()
    };
    match text
        .split_once(':')
        .and_then(|(a, b)| Some((number(a)?, number(b)?)))
    {
        Some((start, len)) if len > 0 => Ok((start, len)),
        _ => Err(format!("invalid {BATCH}={text:?}: expected start:len with len > 0").into()),
    }
}

fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

#[derive(Deserialize)]
struct WeatherAreas {
    areas: Vec<WeatherArea>,
}

#[derive(Deserialize)]
struct WeatherArea {
    key: String,
    anchor: Option<Anchor>,
}

#[derive(Deserialize)]
struct Anchor {
    x: i32,
    y: i32,
    z: i16,
}

#[derive(Deserialize)]
struct BasicArea {
    area: u32,
    name: String,
    generated_count: u8,
}

enum Entry {
    Bestiary {
        ordinal: NonZeroU32,
        name: Option<String>,
        thresholds: [u32; 3],
    },
    Boss {
        name: Option<String>,
        category: String,
    },
    Character(character::CharacterTab),
    CharacterItems,
    Charm {
        ordinal: u16,
        name: String,
        category: crate::domain::charm::CharmCategory,
        stage_costs: [u32; 3],
    },
    Item {
        revision: String,
        name: String,
    },
    Map(BasicArea),
    MapAtlas(oteryn_world_bundle::subarea::Area),
    MapSpeed,
    Weather(WeatherArea),
}

struct Content {
    entries: BTreeMap<String, Entry>,
    weather: WorldWeatherSource,
}

fn census(root: &Path) -> TestResult<Content> {
    let mut names = BTreeMap::new();
    for row in
        creature_sweep::read_shards(&root.join("content/creatures/definitions"), "creatures-")?
    {
        let key = row["definition"]["identity"]["key"]
            .as_str()
            .ok_or("missing creature key")?;
        if let Some(name) = row["authoring"]["profile"]["details"]["display_name"].as_str() {
            names.insert(key.to_owned(), name.to_owned());
        }
    }
    let mut entries = BTreeMap::new();
    let charms = crate::content::charm_source::CanonicalCharmCatalogue::embedded()?;
    let source: serde_json::Value = serde_json::from_slice(charms.source_bytes().1)?;
    let records = source["records"]
        .as_array()
        .ok_or("charm source records missing")?;
    for (index, definition) in charms.catalogue().definitions().enumerate() {
        let key = definition.key.as_str();
        let name = records
            .iter()
            .find(|row| row["definition"]["identity"]["key"].as_str() == Some(key))
            .and_then(|row| row["definition"]["name"].as_str())
            .ok_or("charm source name missing")?;
        entries.insert(
            format!("charms.{key}"),
            Entry::Charm {
                ordinal: u16::try_from(index + 1)?,
                name: name.into(),
                category: definition.category,
                stage_costs: definition.stage_costs,
            },
        );
    }
    for (index, (race, _)) in full_row_p05::canonical_races()?.iter().enumerate() {
        entries.insert(
            format!("bestiary.{}", race.key()),
            Entry::Bestiary {
                ordinal: NonZeroU32::new(u32::try_from(index + 1)?).ok_or("zero race ordinal")?,
                name: names.get(race.key()).cloned(),
                thresholds: race.kill_thresholds().try_into()?,
            },
        );
    }
    let catalogue =
        crate::content::canonical_title_catalogue(crate::content::ProjectEvidenceLimits {
            max_documents: 5,
            max_document_bytes: 2_500_000,
            max_total_bytes: 8_000_000,
            max_json_depth: 10,
            max_decoded_fields: 65_000,
            max_string_bytes: 1_000_000,
            max_locator_bytes: 128,
            max_locator_segments: 5,
            max_reference_records: 4096,
            max_import_records: 1,
            max_reimport_states: 1,
        })
        .map_err(debug)?;
    for boss in catalogue.bosstiary {
        entries.insert(
            format!("bosstiary.{}", boss.key),
            Entry::Boss {
                name: names.get(&boss.key).cloned(),
                category: boss.category,
            },
        );
    }
    for tab in (1..=u8::MAX).filter_map(character::CharacterTab::from_id) {
        entries.insert(format!("character.{tab:?}"), Entry::Character(tab));
    }
    entries.insert("character.Items".into(), Entry::CharacterItems);
    for row in creature_sweep::read_shards(&root.join("content/items/definitions"), "items-")? {
        let definition = &row["definition"];
        if definition["client_projection"] != "ClientSafe" {
            continue;
        }
        let Some(name) = definition["semantics"]["presentation"]["value"]["name"]["value"].as_str()
        else {
            continue;
        };
        let key = definition["identity"]["key"]
            .as_str()
            .ok_or("missing item key")?;
        let revision = definition["identity"]["revision"]
            .as_str()
            .ok_or("missing item revision")?;
        entries.insert(
            format!("items.{key}"),
            Entry::Item {
                revision: revision.into(),
                name: name.into(),
            },
        );
    }
    let area: BasicArea = serde_json::from_slice(&std::fs::read(
        root.join("content/cyclopedia/basic-area.json"),
    )?)?;
    entries.insert(format!("map.{}", area.area), Entry::Map(area));
    entries.insert("map.discovery-speed".into(), Entry::MapSpeed);
    let weather_root = root.join("content/weather");
    let areas = std::fs::read_to_string(weather_root.join("areas.json"))?;
    let weather = WorldWeatherSource::from_json(
        &std::fs::read_to_string(weather_root.join("index.json"))?,
        &std::fs::read_to_string(weather_root.join("climates.json"))?,
        &areas,
    )
    .map_err(debug)?;
    for area in serde_json::from_str::<WeatherAreas>(&areas)?.areas {
        entries.insert(format!("weather.{}", area.key), Entry::Weather(area));
    }
    Ok(Content { entries, weather })
}

#[derive(Default)]
struct Tally {
    pass: usize,
    fail: usize,
}

fn lost_transport(mut error: &(dyn std::error::Error + 'static)) -> bool {
    use oteryn_dev_client::{DevClientError, SessionError};
    loop {
        if matches!(
            error.downcast_ref::<SessionError>(),
            Some(
                SessionError::Io(_)
                    | SessionError::Timeout(_)
                    | SessionError::SessionUnusable
                    | SessionError::ServerProtocolError { .. }
            )
        ) || matches!(
            error.downcast_ref::<DevClientError>(),
            Some(DevClientError::Io(_))
        ) {
            return true;
        }
        let Some(source) = error.source() else {
            return false;
        };
        error = source;
    }
}

/// One row per step, with remaining rows failed if a dependency fails.
struct Rows<'a> {
    key: &'a str,
    remaining: &'static [&'static str],
    first_failure: Option<&'static str>,
    transport_lost: bool,
    tally: &'a mut Tally,
    lap: std::time::Instant,
}

impl Rows<'_> {
    fn emit(&mut self, result: TestResult<String>, cause: &str) -> bool {
        let Some((&step, remaining)) = self.remaining.split_first() else {
            return false;
        };
        self.remaining = remaining;
        let seconds = self.lap.elapsed().as_secs_f64();
        self.lap = std::time::Instant::now();
        let (status, detail, reason) = match result {
            Ok(detail) => {
                self.tally.pass += 1;
                ("PASS", detail, String::new())
            }
            Err(error) => {
                self.transport_lost |= lost_transport(error.as_ref());
                self.tally.fail += 1;
                self.first_failure.get_or_insert(step);
                ("FAIL", error.to_string(), format!(" cause={cause}"))
            }
        };
        evidence(&format!(
            "e2e_all scenario=CY.{}.{step} status={status} seconds={seconds:.3} mode=seeded evidence={}{reason}",
            safe(self.key),
            safe(&detail),
        ));
        status == "PASS"
    }

    fn finish(&mut self) {
        let root = self.first_failure.unwrap_or("entry-aborted");
        while !self.remaining.is_empty() {
            self.emit(Err(format!("blocked-by-{root}").into()), "blocked");
        }
    }
}

fn safe(value: &str) -> String {
    value
        .chars()
        .take(400)
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._-".contains(c) {
                c
            } else {
                '-'
            }
        })
        .collect()
}

impl Entry {
    fn steps(&self) -> &'static [&'static str] {
        match self {
            Self::Bestiary { .. } | Self::Boss { .. } => {
                &["query", "prepare", "spawn", "kill", "progress"]
            }
            Self::Map(_) => &["query", "visit", "progress"],
            Self::MapAtlas(_) => &["query", "visit", "progress"],
            Self::Weather(_) => &["visit", "state"],
            Self::Item { .. }
            | Self::Character(_)
            | Self::CharacterItems
            | Self::Charm { .. }
            | Self::MapSpeed => &["query"],
        }
    }
}

pub(super) async fn run(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult {
    let (start, len) = match std::env::var(BATCH) {
        Ok(text) => parse_batch(Some(&text))?,
        Err(std::env::VarError::NotPresent) => parse_batch(None)?,
        Err(error) => return Err(error.into()),
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let Content {
        mut entries,
        weather,
    } = census(&root)?;
    let subareas = &world.base().subareas;
    let present: std::collections::BTreeSet<_> =
        subareas.runs.iter().map(|run| run.subarea).collect();
    let parents: std::collections::BTreeSet<_> = subareas
        .areas
        .iter()
        .filter(|area| present.contains(&area.id))
        .filter_map(|area| area.parent)
        .collect();
    for area in &subareas.areas {
        if present.contains(&area.id) || parents.contains(&area.id) {
            entries.insert(format!("map.{}", area.key), Entry::MapAtlas(area.clone()));
        }
    }
    let total = entries.len();
    if start >= total {
        return Err(format!("{BATCH} start {start} is beyond {total} entries").into());
    }
    let selected = len.min(total - start);
    evidence(&format!(
        "cyclopedia_sweep census total={total} start={start} len={len} selected={selected} mode=seeded"
    ));
    let mut tally = Tally::default();
    let mut sweep = Sweep {
        clients,
        world,
        weather: &weather,
        reader: None,
        hunter: None,
        items: None,
    };
    for (key, entry) in entries.iter().skip(start).take(selected) {
        verdict::Harness::global().progress("cyclopedia-sweep-entry");
        let scenario = Box::leak(format!("CY.{}.full", safe(key)).into_boxed_str());
        let mut rows = Rows {
            key,
            remaining: entry.steps(),
            first_failure: None,
            transport_lost: false,
            tally: &mut tally,
            lap: std::time::Instant::now(),
        };
        verdict::indexed(
            scenario,
            sweep_entry(&mut sweep, entry, &mut rows, scenario),
        )
        .await;
        if !rows.remaining.is_empty() {
            rows.emit(Err("indexed watchdog interrupted entry".into()), "deadline");
            rows.finish();
            sweep.reader = None;
            sweep.hunter = None;
        }
    }
    let mut release_failures = 0;
    for mut client in [sweep.reader, sweep.hunter].into_iter().flatten() {
        if let Err(error) = creature_sweep::release(&mut client).await {
            release_failures += 1;
            evidence(&format!(
                "cyclopedia_sweep release_failed error={}",
                safe(&error.to_string())
            ));
        }
    }
    evidence(&format!(
        "cyclopedia_sweep summary total={total} selected={selected} rows_pass={} rows_fail={} release_failures={release_failures} mode=seeded",
        tally.pass, tally.fail
    ));
    if tally.fail != 0 || release_failures != 0 {
        return Err(format!(
            "cyclopedia sweep: {} FAIL rows and {release_failures} release failures",
            tally.fail
        )
        .into());
    }
    Ok(())
}

async fn sweep_entry(
    sweep: &mut Sweep<'_, '_>,
    entry: &Entry,
    rows: &mut Rows<'_>,
    scenario: &str,
) -> TestResult {
    let started = rows.lap;
    let key = rows.key;
    let failed_before = rows.tally.fail;
    let completed = tokio::time::timeout(
        Duration::from_secs(240),
        Box::pin(check_entry(sweep, key, entry, rows)),
    )
    .await;
    if completed.is_err() {
        rows.emit(Err("entry deadline exceeded".into()), "deadline");
        sweep.reader = None;
        sweep.hunter = None;
    }
    if rows.transport_lost {
        match entry {
            Entry::Bestiary { .. } | Entry::Boss { .. } => drop(sweep.hunter.take()),
            _ => drop(sweep.reader.take()),
        }
    }
    for session in [&mut sweep.reader, &mut sweep.hunter] {
        if session.as_ref().is_some_and(|client| !client.is_usable()) {
            drop(session.take());
        }
    }
    rows.finish();
    let failures = rows.tally.fail - failed_before;
    let (status, cause) = if failures == 0 {
        ("PASS", "")
    } else {
        ("FAIL", " cause=step-failed")
    };
    evidence(&format!(
        "e2e_all scenario={scenario} status={status} seconds={:.3} mode=seeded evidence=failed-steps:{failures}{cause}",
        started.elapsed().as_secs_f64()
    ));
    if failures != 0 {
        return Err(format!("{failures} failed steps for {}", safe(key)).into());
    }
    Ok(())
}

async fn command(
    client: &mut DevClientSession,
    cap: u32,
    kind: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let reply = client
        .session_mut()
        .gated_command(cap, kind, payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "capability {cap} command {kind} returned {:?}",
            reply.status
        )
        .into());
    }
    Ok(reply.payload)
}

async fn bestiary(
    client: &mut DevClientSession,
    ordinal: NonZeroU32,
) -> TestResult<Option<BestiaryRaceProgress>> {
    Ok(charm_progression(client)
        .await?
        .bestiary
        .into_iter()
        .find(|row| row.race == ordinal))
}

async fn charm_progression(
    client: &mut DevClientSession,
) -> TestResult<charm::CharmProgressionReply> {
    let payload = charm::encode_charm_progression_request(&charm::CharmProgressionAction::Query)
        .map_err(debug)?;
    let reply = command(
        client,
        charm::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
        charm::COMMAND_TYPE_CHARM_PROGRESSION,
        &payload,
    )
    .await?;
    let reply = charm::decode_charm_progression_reply(&reply).map_err(debug)?;
    if reply.outcome != charm::CharmProgressionOutcome::Query {
        return Err("Bestiary progress query returned the wrong outcome".into());
    }
    Ok(reply)
}

fn progress_oracle(
    row: &BestiaryRaceProgress,
    thresholds: [u32; 3],
    kills: u32,
) -> TestResult<String> {
    let stage = thresholds.iter().filter(|&&goal| goal <= kills).count() as u8;
    let next = thresholds.get(usize::from(stage)).copied();
    if row.kill_count != kills
        || row.kill_thresholds != thresholds
        || row.unlocked_stage() != stage
        || row.next_threshold() != next
        || row.is_complete() != next.is_none()
    {
        return Err(format!(
            "expected kills {kills}, stage {stage}, thresholds {thresholds:?}; observed {row:?}"
        )
        .into());
    }
    Ok(format!("kills:{kills},stage:{stage},next:{next:?}"))
}

async fn bestiary_card(
    client: &mut DevClientSession,
    ordinal: NonZeroU32,
    name: &str,
    thresholds: [u32; 3],
) -> TestResult {
    let race = u16::try_from(ordinal.get())?;
    let body = charm::catalogue::encode_request(charm::catalogue::Request {
        after: 0,
        race,
        category: 0,
    })
    .map_err(debug)?;
    let reply = command(
        client,
        charm::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
        charm::COMMAND_TYPE_CHARM_PROGRESSION,
        &body,
    )
    .await?;
    let page = charm::catalogue::decode_page(&reply).map_err(debug)?;
    let expected_thresholds = format!("Discovery thresholds: {thresholds:?}");
    if !matches!(page.cards.as_slice(), [card] if card.race == race && card.name == name && card.detail.lines().any(|line| line == expected_thresholds))
    {
        return Err(format!("missing Bestiary card for {race} named {name}").into());
    }
    Ok(())
}

async fn boss_query(
    client: &mut DevClientSession,
    key: &str,
) -> TestResult<boss::catalogue::Entry> {
    let mut after = 0;
    loop {
        let payload = boss::catalogue::Query {
            after,
            category: 0,
            search: String::new(),
        }
        .encode()
        .ok_or("invalid boss catalogue query")?;
        let reply = command(
            client,
            boss::CAPABILITY_BOSS_ENCOUNTER_V1,
            boss::COMMAND_TYPE_BOSS_COOLDOWNS,
            &payload,
        )
        .await?;
        let page = boss::catalogue::Page::decode(&reply).ok_or("invalid boss catalogue reply")?;
        if let Some(row) = page.entries.into_iter().find(|row| row.key == key) {
            return Ok(row);
        }
        after = page
            .next
            .filter(|&next| next > after)
            .ok_or("boss catalogue row absent")?;
    }
}

fn boss_oracle(
    row: &boss::catalogue::Entry,
    name: &str,
    category: &str,
    kills: u64,
) -> TestResult<String> {
    use crate::domain::bosstiary::{kill_thresholds, progress};
    let thresholds = kill_thresholds(category).ok_or("missing authored boss thresholds")?;
    let expected = progress(category, kills).ok_or("missing authored boss points")?;
    let mut points = [0; 3];
    for (points, threshold) in points.iter_mut().zip(thresholds) {
        *points = progress(category, threshold)
            .ok_or("missing authored boss points")?
            .points;
    }
    if row.name != name
        || !row.category_name().eq_ignore_ascii_case(category)
        || row.thresholds != thresholds
        || row.points != points
        || row.kills != kills
        || row.stage() != usize::from(expected.stage)
        || row.earned_points() != expected.points
    {
        return Err(format!(
            "expected {name} {category} kills {kills} thresholds {thresholds:?}; observed {row:?}"
        )
        .into());
    }
    Ok(format!(
        "name:{name},kills:{kills},stage:{}",
        expected.stage
    ))
}

async fn spawn(
    client: &mut DevClientSession,
    key: &str,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let answer = request_spawn(session, key.into()).await?;
    let target = parse_spawned_entity(&answer)?;
    let visible = tokio::time::Instant::now() + Duration::from_secs(30);
    while !client
        .world_entities()
        .is_some_and(|entities| entities.get(&target).is_some())
    {
        if tokio::time::Instant::now() >= visible {
            return Err("spawned entity never appeared".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
    Ok(target)
}

async fn kill(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    key: &str,
    target: oteryn_dev_client::EntityRef,
) -> TestResult<String> {
    let modes = client
        .session_mut()
        .set_fight_modes(&attack::FightModes {
            chase: attack::ChaseMode::Chase,
            ..attack::FightModes::DEFAULT
        })
        .await?;
    if modes.disposition != attack::AttackIntentDisposition::Ok {
        return Err(format!("chase modes refused: {modes:?}").into());
    }
    let result = client.session_mut().attack_target(Some(&target)).await?;
    if result.status != CommandStatus::Accepted
        || result.disposition != attack::AttackIntentDisposition::Ok
    {
        return Err(format!("attack refused: {result:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(150);
    let mut next_recovery = tokio::time::Instant::now();
    while client
        .world_entities()
        .is_some_and(|entities| entities.get(&target).is_some())
    {
        verdict::Harness::global().progress("cyclopedia-sweep-combat");
        if tokio::time::Instant::now() >= deadline {
            let _ = client.session_mut().attack_target(None).await;
            return Err("spawned creature survived the combat deadline".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
        let vitals = client.actor_vitals().ok_or("combat vitals absent")?;
        if vitals.health == 0 {
            return Err("hunter reached zero health during combat".into());
        }
        let now = tokio::time::Instant::now();
        if now >= next_recovery && vitals.health.saturating_mul(2) < vitals.max_health {
            next_recovery = now + Duration::from_secs(1);
            tokio::time::timeout_at(
                deadline,
                creature_sweep::recover_in_combat(client, world, &target),
            )
            .await
            .map_err(|_| "combat recovery exceeded the combat deadline")??;
            if client
                .actor_vitals()
                .is_some_and(|vitals| vitals.health == 0)
            {
                return Err("hunter reached zero health during recovery".into());
            }
        }
    }
    committed_hunt_death(
        client,
        clients,
        Some(clients.characters[0]),
        key,
        target,
        tokio::time::Instant::now() + Duration::from_secs(20),
    )
    .await?;
    Ok("exact-target credited death".into())
}

struct Sweep<'a, 'b> {
    clients: &'a SeamClients<'b>,
    world: &'a BundleWorld,
    weather: &'a WorldWeatherSource,
    reader: Option<DevClientSession>,
    hunter: Option<DevClientSession>,
    items: Option<crate::content::item_ref::ItemDefinitionIndex>,
}

async fn check_entry(sweep: &mut Sweep<'_, '_>, key: &str, entry: &Entry, rows: &mut Rows<'_>) {
    match entry {
        Entry::Bestiary { .. } | Entry::Boss { .. } => {
            let key = key.split_once('.').map_or(key, |(_, key)| key);
            if sweep.hunter.is_none() {
                match connect_character(sweep.clients, 0xc0, 0).await {
                    Ok(client) => sweep.hunter = Some(client),
                    Err(error) => {
                        rows.emit(Err(error), "admission");
                        return;
                    }
                }
            }
            let Some(client) = &mut sweep.hunter else {
                return;
            };
            let before = creature_query(client, key, entry, 0).await;
            let kills = match before {
                Ok((kills, detail)) => {
                    rows.emit(Ok(detail), "catalogue");
                    kills
                }
                Err(error) => {
                    rows.emit(Err(error), "catalogue");
                    return;
                }
            };
            let prepared = async {
                prepare_hunter(sweep).await?;
                let client = sweep.hunter.as_mut().ok_or("hunter session absent")?;
                let credit = match entry {
                    Entry::Boss { .. } => {
                        let boost = client_daily_boost::reward_selection(client).await?;
                        // The owner applies this observed daily selection to the
                        // exact kill (durability/src/durability/bosstiary.rs:76).
                        crate::domain::bosstiary::kill_credit(boost.boss_key == key)
                    }
                    _ => 1,
                };
                Ok(u64::from(credit))
            }
            .await;
            let credit = match prepared {
                Ok(credit) => {
                    rows.emit(
                        Ok(format!(
                            "classed fixture outside protection zone, expected kill credit:{credit}"
                        )),
                        "precondition",
                    );
                    credit
                }
                Err(error) => {
                    rows.emit(Err(error), "precondition");
                    // The same live fixture can still serve the next catalogue
                    // query when movement prerequisites fail. Re-admit only on
                    // transport loss or zero health.
                    if sweep
                        .hunter
                        .as_ref()
                        .and_then(|client| client.actor_vitals())
                        .is_some_and(|vitals| vitals.health == 0)
                    {
                        drop(sweep.hunter.take());
                    }
                    return;
                }
            };
            let Some(client) = &mut sweep.hunter else {
                return;
            };
            let target = match spawn(client, key).await {
                Ok(target) => {
                    rows.emit(Ok(format!("visible:{target:?}")), "spawn");
                    target
                }
                Err(error) => {
                    rows.emit(Err(error), "spawn");
                    return;
                }
            };
            let result = kill(client, sweep.clients, sweep.world, key, target).await;
            let dead = client
                .actor_vitals()
                .is_some_and(|vitals| vitals.health == 0);
            if rows.emit(result, if dead { "player-death" } else { "combat" }) {
                let next = creature_query(client, key, entry, kills.saturating_add(credit)).await;
                rows.emit(next.map(|(_, detail)| detail), "progress");
            } else if dead {
                drop(sweep.hunter.take());
            }
        }
        _ => {
            if sweep.reader.is_none() {
                match connect_character(sweep.clients, 0xc1, 1).await {
                    Ok(client) => sweep.reader = Some(client),
                    Err(error) => {
                        rows.emit(Err(error), "admission");
                        return;
                    }
                }
            }
            let Some(client) = &mut sweep.reader else {
                return;
            };
            match entry {
                Entry::Map(area) => check_map(client, sweep.world, area, rows).await,
                Entry::MapAtlas(area) => check_atlas(client, sweep.world, area, rows).await,
                Entry::Weather(area) => check_weather(sweep, area, rows).await,
                _ => {
                    rows.emit(
                        read_page(client, key, entry, &mut sweep.items).await,
                        "catalogue",
                    );
                }
            }
        }
    }
}

async fn prepare_hunter(sweep: &mut Sweep<'_, '_>) -> TestResult {
    creature_sweep::ready(sweep.clients, sweep.world, &mut sweep.hunter).await
}

async fn creature_query(
    client: &mut DevClientSession,
    key: &str,
    entry: &Entry,
    kills: u64,
) -> TestResult<(u64, String)> {
    match entry {
        Entry::Bestiary {
            ordinal,
            name,
            thresholds,
        } => {
            let name = name
                .as_deref()
                .ok_or("missing authored creature display name")?;
            bestiary_card(client, *ordinal, name, *thresholds).await?;
            let row = bestiary(client, *ordinal).await?;
            // The server projects only nonzero history (charm_native.rs:147).
            let expected = u32::try_from(kills)?.min(thresholds[2]);
            match row {
                Some(row) => Ok((
                    u64::from(row.kill_count),
                    progress_oracle(&row, *thresholds, expected)?,
                )),
                None if expected == 0 => Ok((
                    0,
                    format!("name:{name},kills:0,stage:0,next:{}", thresholds[0]),
                )),
                None => Err("credited race missing from Bestiary progress".into()),
            }
        }
        Entry::Boss { name, category } => {
            let name = name
                .as_deref()
                .ok_or("missing authored boss display name")?;
            let row = boss_query(client, key).await?;
            Ok((row.kills, boss_oracle(&row, name, category, kills)?))
        }
        _ => Err("entry does not have creature progress".into()),
    }
}

async fn read_charm(
    client: &mut DevClientSession,
    ordinal: u16,
    name: &str,
    category: crate::domain::charm::CharmCategory,
    stage_costs: &[u32; 3],
) -> TestResult<String> {
    use crate::domain::charm::CharmCategory;
    use oteryn_protocol_oteryn::charm::CharmKind;
    let (kind, title, currency) = match category {
        CharmCategory::Major => (CharmKind::Major, "Major", "Charm Points"),
        CharmCategory::Minor => (CharmKind::Minor, "Minor", "Minor Charm Echoes"),
    };
    // T081 metadata pages follow the same ordinal as the progression view.
    let after = ordinal.checked_sub(1).ok_or("zero charm ordinal")?;
    let payload = charm::catalogue::encode_charm_request(after).map_err(debug)?;
    let reply = command(
        client,
        charm::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
        charm::COMMAND_TYPE_CHARM_PROGRESSION,
        &payload,
    )
    .await?;
    let page = charm::catalogue::decode_page(&reply).map_err(debug)?;
    let card = page
        .cards
        .first()
        .filter(|card| card.race == ordinal)
        .ok_or("authored charm missing from metadata page")?;
    if card.name != name
        || card.category != title
        || !stage_costs.iter().enumerate().all(|(index, cost)| {
            let prefix = format!("Stage {}: {cost} {currency};", index + 1);
            card.detail.lines().any(|line| line.starts_with(&prefix))
        })
    {
        return Err(format!("charm metadata differs for {name}: {card:?}").into());
    }
    let progression = charm_progression(client).await?;
    let row = progression
        .charms
        .charms
        .iter()
        .find(|row| row.charm.get() == u32::from(ordinal))
        .ok_or("authored charm missing from progression view")?;
    if row.kind != kind
        || row.unlocked_stage != 0
        || row.assigned_race.is_some()
        || row.next_stage_cost != stage_costs[0]
    {
        return Err(format!("unexpected seeded charm state for {name}: {row:?}").into());
    }
    Ok(format!(
        "verified {name} metadata, stage costs and locked state"
    ))
}

async fn read_page(
    client: &mut DevClientSession,
    key: &str,
    entry: &Entry,
    items: &mut Option<crate::content::item_ref::ItemDefinitionIndex>,
) -> TestResult<String> {
    let cap = character::CAPABILITY_CYCLOPEDIA_CHARACTER_V1;
    let kind = character::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB;
    match entry {
        Entry::Charm {
            ordinal,
            name,
            category,
            stage_costs,
        } => read_charm(client, *ordinal, name, *category, stage_costs).await,
        Entry::Character(tab) => {
            let mut model = panel::CyclopediaCharacterPanel {
                selected: true,
                limit: 1,
                extensions: panel::TabExtensions {
                    vocation: general_stats::selected(client),
                    death_details: client
                        .selected_capabilities()
                        .contains(&character::CAPABILITY_CYCLOPEDIA_DEATH_DETAILS_V1),
                },
                ..Default::default()
            };
            let req = panel::request(&mut model, panel::Intent::SelectTab(*tab))
                .ok_or("no tab request")?;
            let reply = client
                .session_mut()
                .gated_command(req.capability, req.kind, &req.payload)
                .await?;
            if reply.status != CommandStatus::Accepted || !panel::apply_result(&mut model, &reply) {
                return Err(format!("tab {tab:?} refused or malformed: {:?}", reply.status).into());
            }
            if *tab == character::CharacterTab::GeneralStats
                && !model.general.is_some_and(|stats| stats.level > 0)
            {
                return Err("GeneralStats has no positive character level".into());
            }
            Ok(format!("decoded:{tab:?}"))
        }
        Entry::CharacterItems => {
            use character::items;
            let body = command(client, cap, kind, &items::request(0)).await?;
            let page = items::Page::decode(&body).map_err(debug)?;
            if page.offset != 0 || page.rows.len() != usize::from(page.total).min(items::PAGE_SIZE)
            {
                return Err(format!("incorrect Character Items page: {page:?}").into());
            }
            Ok(format!("total:{}", page.total))
        }
        Entry::MapSpeed => {
            let rules = crate::domain::cyclopedia::DiscoverySpeed::embedded()
                .ok_or("authored discovery speed unavailable")?;
            let body = command(
                client,
                map::CAPABILITY_CYCLOPEDIA_MAP_V1,
                map::COMMAND_TYPE_CYCLOPEDIA_SPEED_QUERY,
                &[],
            )
            .await?;
            let actual = u16::from_le_bytes(body.as_slice().try_into()?);
            let expected = rules.bonus(0, 1, false);
            if actual != expected {
                return Err(format!(
                    "expected discovery speed {expected} for incomplete exploration; got {actual}"
                )
                .into());
            }
            Ok(format!("bonus:{actual}"))
        }
        Entry::Item { revision, name } => {
            let key = key.strip_prefix("items.").ok_or("missing item key")?;
            if items.is_none() {
                *items = Some(provisioning::item_index()?);
            }
            let index = items.as_ref().ok_or("item index unavailable")?;
            let reference = index
                .definition_ref("Item", key, revision)
                .ok_or("item missing from pinned definition index")?;
            let payload =
                inspection::encode_request(inspection::Request::Catalogue(reference.get()))
                    .map_err(debug)?;
            let body = command(
                client,
                inspection::CAPABILITY_INSPECTION_V1,
                inspection::COMMAND_TYPE_INSPECTION,
                &payload,
            )
            .await?;
            let reply = inspection::decode_reply(&body).map_err(debug)?;
            if reply.status != 1
                || !matches!(reply.items.as_slice(), [row] if row.definition_ref == reference.get() && row.name == *name)
            {
                return Err(format!("expected item {reference} {name}; observed {reply:?}").into());
            }
            Ok(format!("name:{name},definition:{reference}"))
        }
        _ => Err("entry is not a static page".into()),
    }
}

async fn map_query(client: &mut DevClientSession) -> TestResult<map::MapResult> {
    let bytes = command(
        client,
        map::CAPABILITY_CYCLOPEDIA_MAP_V1,
        map::COMMAND_TYPE_CYCLOPEDIA_QUERY,
        &[],
    )
    .await?;
    let result = map::decode_result(&bytes).map_err(debug)?;
    if result.code != 0 {
        return Err(format!("map query returned {result:?}").into());
    }
    Ok(result)
}

async fn verify_atlas_tile(
    client: &mut DevClientSession,
    area: u32,
    at: TilePos,
    subarea: u32,
) -> TestResult {
    use oteryn_protocol_oteryn::cyclopedia::geometry as atlas;
    let floor = -at.floor;
    let mut after = 0;
    loop {
        let payload =
            atlas::encode_request(atlas::Request { area, floor, after }).map_err(debug)?;
        let body = command(
            client,
            map::CAPABILITY_CYCLOPEDIA_MAP_V1,
            map::COMMAND_TYPE_CYCLOPEDIA_QUERY,
            &payload,
        )
        .await?;
        let atlas::Reply::Geometry(page) = atlas::decode_reply(&body).map_err(debug)? else {
            return Err("atlas returned an area list for a visited tile".into());
        };
        if page.area != area || page.floor != floor || page.page != after {
            return Err("atlas geometry response does not match its requested page".into());
        }
        if let Some(run) = page
            .runs
            .iter()
            .find(|run| run.y == at.y && (run.begin..run.end).contains(&at.x))
        {
            if run.subarea != subarea {
                return Err(format!(
                    "wire atlas labels {at:?} as {}, expected {subarea}",
                    run.subarea
                )
                .into());
            }
            return Ok(());
        }
        after = page.next.filter(|&next| next > after).ok_or_else(|| {
            format!("wire atlas geometry omits visited tile {at:?} in subarea {subarea}")
        })?;
    }
}

async fn check_atlas(
    client: &mut DevClientSession,
    world: &BundleWorld,
    area: &oteryn_world_bundle::subarea::Area,
    rows: &mut Rows<'_>,
) {
    use oteryn_protocol_oteryn::cyclopedia::geometry as atlas;
    let query = async {
        let mut after = 0;
        let parent = area.parent.unwrap_or(0);
        loop {
            let payload = atlas::encode_request(atlas::Request {
                area: parent,
                floor: atlas::AREA_LIST,
                after,
            })
            .map_err(debug)?;
            let body = command(
                client,
                map::CAPABILITY_CYCLOPEDIA_MAP_V1,
                map::COMMAND_TYPE_CYCLOPEDIA_QUERY,
                &payload,
            )
            .await?;
            let atlas::Reply::Areas(page) = atlas::decode_reply(&body).map_err(debug)? else {
                return Err("atlas returned geometry in response to an area list".into());
            };
            if page.parent != parent || page.after != after {
                return Err("atlas cursor or parent mismatch".into());
            }
            if let Some(row) = page.areas.iter().find(|row| row.id == area.id) {
                if row.name != area.name || row.parent != parent {
                    return Err(
                        format!("expected atlas area {} {parent}; got {row:?}", area.name).into(),
                    );
                }
                return Ok(format!("name:{},id:{}", row.name, row.id));
            }
            after = page
                .next
                .filter(|&next| next > after)
                .ok_or("authored atlas area absent")?;
        }
    }
    .await;
    if !rows.emit(query, "map-atlas") {
        return;
    }
    let visit = async {
        let table = &world.base().subareas;
        let matches = |at: TilePos| {
            table
                .resolve(at.x, at.y, at.floor)
                .is_some_and(|(parent, subarea)| {
                    if area.parent.is_some() {
                        subarea == area.id
                    } else {
                        parent == area.id
                    }
                })
        };
        let from = tile(client.world_spatial().actor_position)?;
        let path = route(world, from, matches).ok_or("no reachable tile in authored atlas area")?;
        walk(client, world, &path).await?;
        let at = tile(client.world_spatial().actor_position)?;
        let membership = table
            .resolve(at.x, at.y, at.floor)
            .ok_or("position outside atlas")?;
        if !matches(at) {
            return Err(format!("wire position {at:?} is outside the requested atlas area").into());
        }
        verify_atlas_tile(client, area.id, at, membership.1).await?;
        Ok(format!("membership:{membership:?}"))
    }
    .await;
    if rows.emit(visit, "map-visit") {
        let progress = async {
            let rules = crate::domain::cyclopedia::BasicArea::embedded()
                .ok_or("missing authored POI rules")?;
            let before = map_query(client).await?;
            let found_before = before.found;
            visit_poi(client, world, before).await?;
            let after = map_query(client).await?;
            map_progress_oracle(&after, found_before, rules.generated_count)
        }
        .await;
        rows.emit(progress, "map-progress");
    }
}

async fn check_map(
    client: &mut DevClientSession,
    world: &BundleWorld,
    area: &BasicArea,
    rows: &mut Rows<'_>,
) {
    let before = match map_query(client).await {
        Ok(result) if result.found == 0 && !result.discovered => {
            rows.emit(
                Ok(format!("authored-area:{},unlocked:false", area.name)),
                "map-query",
            );
            result
        }
        Ok(result) => {
            rows.emit(
                Err(format!("expected unvisited map; got {result:?}").into()),
                "map-query",
            );
            return;
        }
        Err(error) => {
            rows.emit(Err(error), "map-query");
            return;
        }
    };
    let visit = visit_poi(client, world, before)
        .await
        .map(|at| format!("visited:{at:?}"));
    if rows.emit(visit, "map-visit") {
        let progress = map_query(client)
            .await
            .and_then(|after| map_progress_oracle(&after, 0, usize::from(area.generated_count)));
        rows.emit(progress, "map-progress");
    }
}

fn map_progress_oracle(
    after: &map::MapResult,
    before: u8,
    generated_count: usize,
) -> TestResult<String> {
    let found = usize::from(after.found);
    if after.found <= before
        || found > generated_count
        || after.discovered != (found == generated_count)
    {
        return Err(format!(
            "expected progress beyond {before} of {generated_count} authored POIs; got {after:?}"
        )
        .into());
    }
    Ok(format!("found-before:{before},found-after:{found}"))
}

async fn visit_poi(
    client: &mut DevClientSession,
    world: &BundleWorld,
    mut state: map::MapResult,
) -> TestResult<TilePos> {
    use crate::domain::cyclopedia::Point;
    if state.discovered {
        return Err("current subarea already completed all POIs before this visit".into());
    }
    let from = tile(client.world_spatial().actor_position)?;
    let table = &world.base().subareas;
    let (_, subarea) = table
        .resolve(from.x, from.y, from.floor)
        .ok_or("entry has no native subarea")?;
    let inside = |at: TilePos| {
        table
            .resolve(at.x, at.y, at.floor)
            .is_some_and(|(_, id)| id == subarea)
    };
    let point = |at: TilePos| Point {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: i16::from(at.floor),
    };
    // The owner hides generated POIs outside its view window. Visit reachable
    // vantage points within the same subarea until a POI is visible.
    let before = state.found;
    let mut surveyed = Vec::new();
    while state.visible.is_empty() && state.found == before {
        let here = tile(client.world_spatial().actor_position)?;
        surveyed.push(point(here));
        let path = route_avoiding(
            world,
            here,
            |at| surveyed.iter().all(|old| !point(at).in_window(*old)),
            inside,
        )
        .ok_or("no POI visible from surveyed subarea")?;
        walk(client, world, &path).await?;
        state = map_query(client).await?;
    }
    if state.found == before
        && let Some(&(x, y, z)) = state.visible.first()
    {
        // The MapResult uses client z; the local bundle uses its negation.
        let target = TilePos { x, y, floor: -z };
        let here = tile(client.world_spatial().actor_position)?;
        let path = route_avoiding(world, here, |at| at == target, inside)
            .ok_or("visible POI is unreachable within the subarea")?;
        walk(client, world, &path).await?;
    }
    tile(client.world_spatial().actor_position)
}

fn weather_now() -> TestResult<u64> {
    crate::world_clock::now_utc_ms().ok_or_else(|| "UTC clock unavailable".into())
}

fn weather_oracle(
    source: &WorldWeatherSource,
    position: ActorPosition,
    before: u64,
    after: u64,
    observed: weather::WorldWeather,
) -> TestResult<String> {
    let first = source.observe(position, before);
    let last = source.observe(position, after);
    if before > after
        || after - before >= WEATHER_SLOT_MS
        || observed == weather::WorldWeather::Unavailable
        || (observed != first && observed != last)
    {
        return Err(format!(
            "weather at {position:?} expected {first:?} or {last:?}; observed {observed:?}"
        )
        .into());
    }
    let weather::WorldWeather::Active {
        absolute_slot,
        kind,
        intensity,
        ..
    } = observed
    else {
        return Err("weather observation unavailable".into());
    };
    Ok(format!(
        "slot:{absolute_slot},kind:{kind},intensity:{intensity}"
    ))
}

async fn check_weather(sweep: &mut Sweep<'_, '_>, area: &WeatherArea, rows: &mut Rows<'_>) {
    let visit = async {
        let anchor = area
            .anchor
            .as_ref()
            .ok_or("weather area has no authored anchor")?;
        let anchor = ActorPosition {
            x: anchor.x,
            y: anchor.y,
            floor: anchor.z,
        };
        let client = sweep.reader.as_mut().ok_or("no reader session")?;
        let matches_area = |pos| {
            sweep
                .weather
                .area_at(pos)
                .is_some_and(|row| row.key == area.key)
        };
        if !matches_area(client.world_spatial().actor_position) {
            let anchor_tile = tile(anchor)?;
            let position = (0_i16..=24)
                .flat_map(|distance| {
                    (-distance..=distance).flat_map(move |dx| {
                        let dy = distance - dx.abs();
                        [(dx, dy), (dx, -dy)]
                    })
                })
                .filter_map(|(dx, dy)| {
                    Some(TilePos {
                        x: anchor_tile.x.checked_add_signed(dx)?,
                        y: anchor_tile.y.checked_add_signed(dy)?,
                        floor: anchor_tile.floor,
                    })
                })
                .find(|at| {
                    sweep
                        .world
                        .base()
                        .tile(at.x, at.y, at.floor)
                        .is_some_and(|tile| {
                            tile.walkable() && !tile.protection_zone() && !tile.no_logout()
                        })
                        && sweep.world.enterable(*at)
                        && matches_area(ActorPosition {
                            x: i32::from(at.x),
                            y: i32::from(at.y),
                            floor: -i16::from(at.floor),
                        })
                })
                .ok_or_else(|| {
                    format!("no walkable fixture tile near weather anchor {anchor:?}")
                })?;
            let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
            let floor = -position.floor;
            let coordinates = format!("{},{},{floor}", position.x, position.y);
            let answer =
                request_fixture_control(session, "--place-weather-player", coordinates).await?;
            if answer != format!("placed {} {} {floor}", position.x, position.y) {
                return Err("qualification fixture did not place the weather reader".into());
            }
            let target = ActorPosition {
                x: i32::from(position.x),
                y: i32::from(position.y),
                floor: i16::from(floor),
            };
            tokio::time::timeout(Duration::from_secs(15), async {
                while client.world_spatial().actor_position != target {
                    client.service_liveness(Duration::from_millis(100)).await?;
                    client.take_events();
                }
                Ok::<_, oteryn_dev_client::DevClientError>(())
            })
            .await
            .map_err(|_| "weather placement was not served on the wire")??;
        }
        let position = client.world_spatial().actor_position;
        if !matches_area(position) {
            return Err(format!("wire position {position:?} is outside weather region").into());
        }
        Ok(format!("region:{},position:{position:?}", area.key))
    }
    .await;
    if !rows.emit(visit, "weather-region") {
        return;
    }
    let state = async {
        let mut previous = sweep.reader.take().ok_or("no reader session")?;
        creature_sweep::release(&mut previous).await?;
        drop(previous);
        let before = weather_now()?;
        let mut client = connect_character(sweep.clients, 0xc2, 1).await?;
        let after = weather_now()?;
        let position = client.world_spatial().actor_position;
        let result = (|| {
            if sweep
                .weather
                .area_at(position)
                .is_none_or(|row| row.key != area.key)
            {
                return Err(
                    format!("fresh wire position {position:?} is outside weather region").into(),
                );
            }
            let snapshot = client
                .session_mut()
                .gated_snapshot(weather::STATE_DOMAIN_WORLD_WEATHER)
                .ok_or("weather snapshot missing")?;
            if snapshot.snapshot_type != weather::SNAPSHOT_TYPE_WORLD_WEATHER_V1 {
                return Err("unexpected weather snapshot type".into());
            }
            let observed = weather::decode_world_weather(&snapshot.payload).map_err(debug)?;
            weather_oracle(sweep.weather, position, before, after, observed)
        })();
        sweep.reader = Some(client);
        result
    }
    .await;
    rows.emit(state, "weather-state");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_transport_loss_through_dev_client_context() {
        use oteryn_dev_client::{DevClientError, SessionError};
        let error = DevClientError::Context {
            phase: "command",
            elapsed: Duration::ZERO,
            last_frame: String::new(),
            source: Box::new(DevClientError::Session(SessionError::Io(
                std::io::Error::from(std::io::ErrorKind::UnexpectedEof),
            ))),
        };
        assert!(lost_transport(&error));
        assert!(lost_transport(&SessionError::SessionUnusable));
        assert!(!lost_transport(&SessionError::ActivityScopeChanged));
        assert!(!lost_transport(&std::io::Error::other("content fixture")));
    }

    #[test]
    fn batch_accepts_only_plain_bounded_positive_slices() -> TestResult {
        assert_eq!(parse_batch(None)?, (0, 3));
        assert_eq!(parse_batch(Some("0:20"))?, (0, 20));
        for bad in [
            "",
            "1",
            "1:0",
            ":1",
            "1:",
            "+1:2",
            "1:2:3",
            " 1:2",
            "99999999999999999999999999:1",
        ] {
            assert!(parse_batch(Some(bad)).is_err());
        }
        Ok(())
    }

    #[test]
    fn progress_oracle_rejects_wrong_kills_and_thresholds() -> TestResult {
        let row = BestiaryRaceProgress {
            race: NonZeroU32::MIN,
            kill_count: 5,
            kill_thresholds: [2, 5, 10],
        };
        assert!(progress_oracle(&row, [2, 5, 10], 5)?.contains("stage:2"));
        assert!(progress_oracle(&row, [2, 5, 10], 4).is_err());
        assert!(progress_oracle(&row, [2, 6, 10], 5).is_err());
        Ok(())
    }

    #[test]
    fn boss_oracle_checks_stage_points_even_before_the_first_kill() -> TestResult {
        let mut row = boss::catalogue::Entry {
            key: "boss".into(),
            name: "Boss".into(),
            category: 3,
            kills: 0,
            thresholds: [1, 3, 5],
            points: [10, 40, 100],
            look: 0,
            object: false,
        };
        assert!(boss_oracle(&row, "Boss", "nemesis", 0)?.contains("stage:0"));
        row.kills = 1;
        assert!(boss_oracle(&row, "Boss", "nemesis", 1)?.contains("stage:1"));
        row.points[2] = 99;
        assert!(boss_oracle(&row, "Boss", "nemesis", 1).is_err());
        Ok(())
    }

    #[test]
    fn map_oracle_requires_a_new_find_and_consistent_unlock() {
        let mut after = map::MapResult {
            code: 0,
            revision: 1,
            discovered: false,
            found: 1,
            area_percent: 0,
            pool: 0,
            epoch: 0,
            improved: false,
            visible: Vec::new(),
        };
        assert!(map_progress_oracle(&after, 0, 7).is_ok());
        assert!(map_progress_oracle(&after, 1, 7).is_err());
        after.discovered = true;
        assert!(map_progress_oracle(&after, 0, 7).is_err());
        after.found = 7;
        assert!(map_progress_oracle(&after, 1, 7).is_ok());
    }

    #[test]
    fn weather_oracle_requires_the_authored_active_observation() -> TestResult {
        let source = oteryn_gameplay_weather::shipped().ok_or("weather content absent")?;
        let position = ActorPosition {
            x: 32732,
            y: 31634,
            floor: 7,
        };
        let observed = source.observe(position, WEATHER_SLOT_MS);
        assert!(
            weather_oracle(
                source,
                position,
                WEATHER_SLOT_MS,
                WEATHER_SLOT_MS + 1,
                observed
            )
            .is_ok()
        );
        assert!(
            weather_oracle(
                source,
                position,
                WEATHER_SLOT_MS,
                WEATHER_SLOT_MS + 1,
                weather::WorldWeather::Unavailable
            )
            .is_err()
        );
        assert!(
            weather_oracle(
                source,
                position,
                WEATHER_SLOT_MS * 100,
                WEATHER_SLOT_MS * 100 + 1,
                observed
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn authored_census_includes_all_families() -> TestResult {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let content = census(&root)?;
        for family in [
            "bestiary.",
            "bosstiary.",
            "character.",
            "charms.",
            "items.",
            "map.",
            "weather.",
        ] {
            assert!(
                content.entries.keys().any(|key| key.starts_with(family)),
                "{family}"
            );
        }
        let counts = content
            .entries
            .keys()
            .fold(BTreeMap::new(), |mut counts, key| {
                *counts
                    .entry(key.split('.').next().unwrap_or_default())
                    .or_insert(0) += 1;
                counts
            });
        eprintln!(
            "cyclopedia_sweep census total={} {counts:?}",
            content.entries.len()
        );
        Ok(())
    }
}
