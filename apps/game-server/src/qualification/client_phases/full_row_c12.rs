//! C12 full-row journey (Spell equipment augments), derived from the matrix, the re-audit and the
//! `item_spell_augments` owner. Every step is driven over the real client connection and every
//! effect is measured, never inferred from a mana debit or a slot.
//!
//! 1. `acquire-50271`: purchase the Fists of Enlightenment (50271) from Enpa-Deia Pema.
//! 2. `baseline-cast`: place one authored Magma Crystal through the node fixture, attack it,
//!    and CAST Greater Tiger Clash repeatedly without
//!    the item; require `Cast` and a mana debit each time and record the spell damage the server
//!    attributes to the cast (the presentation events sharing the cast's decision ordinal).
//! 3. `augmented-cast`: ITEM_MOVE the item to the right hand, CAST again and require attributed
//!    damage on the original damage roll. The weapon swap changes the attack inputs.
//! 4. `unequip`: ITEM_MOVE it back to the backpack; require the inventory delta.
//! 5. `unequipped-cast`: restore the baseline weapon and CAST with item 50271 only carried.
//! 6. `fresh-admission`: re-equip, LOGOUT, admit afresh, require the right-hand item to persist and
//!    CAST again and require the committed cast owner to prove the exact augment on its original roll.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{ActorVitals, SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::item_view as items;
use oteryn_protocol_oteryn::presentation::{PresentationEventKind, PresentationValueKind};
use std::num::NonZeroU32;

const PREPARATION: character_prep::Seeded = character_prep::Seeded {
    profile: "c12-monk-augment",
    reason: "C12 requires a roll large enough that the authored eight percent changes integer damage.",
};
const ITEM_ID: u32 = 50_271;
const ADVENTURERS_STONE: u32 = 16_277;
pub(super) const GREATER_PROFICIENCY_CATALYST: u32 = 51_589;
const THAIS: u32 = 8;
const STONE_ARRIVAL: TilePos = TilePos {
    x: 32210,
    y: 32300,
    floor: -6,
};
// Action 4253 on the eastern Guild exit and the tile immediately south of it.
const GUILD_EXIT_STEP: TilePos = TilePos {
    x: 32210,
    y: 32293,
    floor: -6,
};
const GUILD_EAST_EXIT: TilePos = TilePos {
    x: 32210,
    y: 32292,
    floor: -6,
};
const GUILD_WEST_EXIT: TilePos = TilePos {
    x: 32209,
    y: 32292,
    floor: -6,
};
pub(super) const AUGMENT_BPS: u64 = 800;
const PEMA_PRICE: u64 = 20_000;
const PREREQUISITES: &str = "level-80-monk,premium,purchase-gold,mana-potions,magma-crystal,adventurers-stone,greater-proficiency-catalyst";
const FIXTURE_TARGET: &str = "oteryn:creature.magma_crystal";
const CRYSTAL_COIN: u32 = 3043;
const CRYSTAL_COIN_VALUE: u64 = 10_000;
const MANA_POTION: u32 = 268;
const CAST_MANA: u32 = 50;
const SPELL_WORDS: &str = "exori nia";
const SPELL_NAME: &str = "greater tiger clash";
const SAMPLES: usize = 3;
const COMMAND_BUDGET: Duration = Duration::from_secs(8);
const BASELINE_BUDGET: Duration = Duration::from_secs(75);
// The authored Blue Valley round trip adds paced steps before the four cast phases.
const JOURNEY_BUDGET: Duration = Duration::from_secs(1200);
const STEPS: [&str; 7] = [
    "acquire-50271",
    "baseline-cast",
    "augmented-cast",
    "unequip",
    "unequipped-cast",
    "fresh-admission",
    "augment-attribution",
];

struct Row {
    started: std::time::Instant,
    passed: Vec<&'static str>,
    failure: Option<(&'static str, String)>,
}

impl Row {
    fn new() -> Self {
        Self {
            started: std::time::Instant::now(),
            passed: Vec::new(),
            failure: None,
        }
    }

    fn current(&self) -> &'static str {
        STEPS.get(self.passed.len()).copied().unwrap_or("journey")
    }

    fn pass(&mut self, step: &'static str, observed: impl std::fmt::Debug) {
        if self.failure.is_none() && STEPS.get(self.passed.len()) == Some(&step) {
            self.passed.push(step);
            evidence(&format!("C12_CHECK {step} PASS observed={observed:?}"));
        }
    }

    fn record_cast(&mut self, step: &'static str, samples: &[u64], equipped: Option<NonZeroU32>) {
        if self.failure.is_none() && STEPS.get(self.passed.len()) == Some(&step) {
            self.passed.push(step);
            evidence(&format!(
                "C12_OBSERVATION step={step} weapon={equipped:?} damage={samples:?}"
            ));
        }
    }

    fn finish(mut self, result: TestResult) {
        if let Err(error) = result {
            self.failure = Some((self.current(), error.to_string()));
        }
        let complete = self.failure.is_none() && self.passed.len() == STEPS.len();
        let detail = match &self.failure {
            _ if complete => String::new(),
            Some((step, observed)) => {
                let not_run = STEPS.get(self.passed.len() + 1..).unwrap_or_default();
                evidence(&format!("C12_CHECK {step} FAIL observed={observed}"));
                evidence(&gap(step, observed, not_run));
                format!(
                    " evidence=step:{step};observed:{};not-run:{}",
                    token(observed),
                    not_run.join(",")
                )
            }
            None => {
                let step = self.current();
                evidence(&gap(
                    step,
                    "journey ended without required check",
                    &STEPS[self.passed.len() + 1..],
                ));
                " evidence=journey-ended-without-every-step".to_owned()
            }
        };
        evidence(&format!(
            "e2e_all scenario=C12.full status={} seconds={:.3} mode=seeded prerequisites={PREREQUISITES}{detail}",
            if complete { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

fn gap(step: &str, observed: &str, remaining: &[&str]) -> String {
    format!(
        "C12_GAP step={step} observed={observed} not-run={}",
        remaining.join(",")
    )
}

fn token(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-_.:/<>=".contains(c) {
                c
            } else {
                '-'
            }
        })
        .collect()
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    character_prep::declare_seeded("C12", PREREQUISITES, "exori nia augment attribution");
    let mut row = Row::new();
    let result = match verdict::indexed_timeout(
        JOURNEY_BUDGET,
        verdict::observe(journey(&mut row, clients)),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(format!("journey-timeout-{}s", JOURNEY_BUDGET.as_secs()).into()),
    };
    row.finish(result);
}

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_json(relative: &str) -> TestResult<serde_json::Value> {
    Ok(serde_json::from_slice(&std::fs::read(
        root().join(relative),
    )?)?)
}

fn spell(client: &mut DevClientSession) -> TestResult<NonZeroU32> {
    spell_by_words(client, SPELL_WORDS)
}

fn spell_by_words(client: &mut DevClientSession, words: &str) -> TestResult<NonZeroU32> {
    let catalog = client
        .session_mut()
        .spell_catalog()?
        .ok_or("served spell catalogue absent")?;
    let entry = catalog
        .entries
        .iter()
        .find(|entry| entry.words == words && entry.rune_item.is_none())
        .ok_or_else(|| format!("C12 spell {words} absent from served catalogue"))?;
    if !entry.available {
        return Err(format!(
            "need=premium-monk-or-exalted-monk;spell={words};cast-min-level={};item-50271-min-level=20;learned={}",
            entry.level, entry.learned
        ).into());
    }
    NonZeroU32::new(entry.cast_index)
        .ok_or_else(|| format!("C12 spell {words} has zero served index").into())
}

/// The total Greater Tiger Clash damage bonus with both selected Fists perks.
fn content_augment() -> TestResult<u64> {
    let profiles = read_json("content/items/definitions/spell-native-profiles.json")?;
    let key = format!("oteryn:item.tibia.i{ITEM_ID}");
    let item = profiles["records"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row["production_definition"]["production_key"] == key)
        })
        .ok_or("C12 Fists item profile absent")?;
    if item["attributes"]["spell_augments"]
        .as_array()
        .ok_or("C12 Fists spell augments absent")?
        .iter()
        .any(|augment| augment["kind"] == "base_damage" && augment["spell"] == SPELL_NAME)
    {
        return Err("C12 Fists must grant Greater Tiger Clash damage through proficiency".into());
    }
    let definitions = read_json("content/proficiencies/proficiencies-00300-00442.json")?;
    let profile = definitions["records"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row["definition"]["identity"]["key"] == "oteryn:proficiency.tibia.p399")
        })
        .ok_or("C12 Fists proficiency profile absent")?;
    let levels = profile["definition"]["levels"]
        .as_array()
        .ok_or("C12 Fists levels absent")?;
    for level in [1, 3] {
        if !levels.iter().any(|row| {
            row["level"] == level
                && row["perks"].as_array().is_some_and(|perks| {
                    perks.iter().any(|perk| {
                        perk["kind"] == "spell_augment"
                            && perk["spell_client_id"] == 292
                            && perk["augment"] == "base_damage"
                            && perk["value"] == 0.04
                    })
                })
        }) {
            return Err(format!(
                "C12 missing 400-bp Greater Tiger Clash proficiency at level {level}"
            )
            .into());
        }
    }
    Ok(AUGMENT_BPS)
}

struct CreatureProfile {
    health: u64,
    display_name: String,
    neutral_physical_target: bool,
}

fn creature_profiles() -> TestResult<std::collections::BTreeMap<String, CreatureProfile>> {
    let mut profiles = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(root().join("content/creatures/definitions"))? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if !(name.starts_with("creatures-") && name.ends_with(".json")) {
            continue;
        }
        let document: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
        for record in document["records"].as_array().into_iter().flatten() {
            let profile = &record["authoring"]["profile"];
            if let (Some(key), Some(health), Some(name)) = (
                record["definition"]["identity"]["key"].as_str(),
                profile["health"].as_u64(),
                profile["details"]["display_name"].as_str(),
            ) {
                profiles.insert(
                    key.to_owned(),
                    CreatureProfile {
                        health,
                        display_name: name.to_owned(),
                        neutral_physical_target: profile["armor"].as_u64() == Some(0)
                            && profile["mitigation"]["numerator"].as_i64() == Some(0)
                            && profile["speed"].as_u64() == Some(0)
                            && profile["resistances"].as_array().is_none_or(|resistances| {
                                !resistances.iter().any(|resistance| {
                                    resistance["damage_type"].as_str() == Some("physical")
                                        && resistance["percent"]["numerator"].as_i64() != Some(0)
                                })
                            }),
                    },
                );
            }
        }
    }
    Ok(profiles)
}

fn item_ref() -> TestResult<NonZeroU32> {
    provisioning::item_reference(&provisioning::item_index()?, ITEM_ID)
}

fn equipped_right(client: &DevClientSession, definition: NonZeroU32) -> bool {
    client.inventory().is_some_and(|inventory| {
        inventory.equipment.iter().any(|entry| {
            entry.slot == items::EquipmentSlot::RightHand
                && entry.item.item_definition_ref == definition
                && entry.item.count.get() == 1
        })
    })
}

fn carried_count(client: &DevClientSession, definition: NonZeroU32) -> u64 {
    client.inventory().map_or(0, |inventory| {
        inventory
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref == definition)
            .map(|entry| u64::from(entry.count.get()))
            .sum()
    })
}

async fn wait_inventory(
    client: &mut DevClientSession,
    what: &str,
    predicate: impl Fn(&DevClientSession) -> bool,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !predicate(client) {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("no inventory delta for {what}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    Ok(())
}

fn own_entity(client: &DevClientSession) -> TestResult<oteryn_dev_client::EntityRef> {
    let entities = client
        .world_entities()
        .ok_or("own-entity-snapshot-missing")?;
    entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .map(|entry| entry.entity)
        .ok_or_else(|| "own-entity-missing".into())
}

struct Hunt {
    entity: oteryn_dev_client::EntityRef,
    profile: CreatureProfile,
    cast_evidence_log: std::path::PathBuf,
}

impl Hunt {
    async fn target(
        &self,
        client: &mut DevClientSession,
    ) -> TestResult<oteryn_dev_client::EntityRef> {
        wait_fixture_target(client.session_mut(), self.entity, &self.profile).await?;
        let entity = self.entity;
        if combat_target(client)? != Some(entity) {
            let sequence = client.last_server_sequence();
            let reply = verdict::indexed_timeout(
                COMMAND_BUDGET,
                client.session_mut().attack_target(Some(&entity)),
            )
            .await
            .map_err(|_| format!("ATTACK deadline=8s last-frame={sequence}"))??;
            require_mechanic_acceptance(reply.status, "C12 ATTACK")?;
            if reply.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
                return Err(format!("ATTACK reply={reply:?}").into());
            }
            wait_combat_target(client, Some(entity)).await?;
        }
        Ok(entity)
    }

    /// Verify damage attributed to the cast and the vitals applied by the client.
    async fn cast_damage(
        &self,
        client: &mut DevClientSession,
        spell: NonZeroU32,
        bps: u64,
    ) -> TestResult<(u64, ActorVitals)> {
        let victim = self.target(client).await?;
        let own = own_entity(client)?;
        let before = *client.actor_vitals().ok_or("pre-cast vitals absent")?;
        client.take_events();
        let sequence = client.last_server_sequence();
        let evidence_start = std::fs::metadata(&self.cast_evidence_log)?.len();
        let outcome = verdict::indexed_timeout(
            COMMAND_BUDGET,
            client.cast_spell(spell, SpellTarget::AttackTarget, true),
        )
        .await
        .map_err(|_| format!("CAST deadline=8s last-frame={sequence}"))??;
        require_mechanic_acceptance(outcome.status, "C12 CAST")?;
        if outcome.disposition != SpellCastDisposition::Cast {
            return Err(format!("CAST reply={outcome:?}").into());
        }
        let delta = outcome
            .actor_vitals_delta
            .as_ref()
            .ok_or("C12 accepted cast omitted actor-vitals")?;
        let vitals = delta.value;
        let in_flight = client.take_events();
        let mana_before_cast = super::client_row_c12::frame_capture::mana_before_cast(
            in_flight.iter().filter_map(|event| match event {
                SessionEvent::ActorVitals(update) => {
                    Some((update.server_sequence, update.value.mana))
                }
                _ => None,
            }),
            before.mana,
            before.max_mana,
            delta.server_sequence,
        )
        .ok_or("C12 full unexpected mana change before the cast delta")?;
        if mana_before_cast.checked_sub(CAST_MANA) != Some(vitals.mana) {
            return Err(format!(
                "C12 expected {CAST_MANA} mana spent, observed {mana_before_cast} -> {} from wire sequence {}",
                vitals.mana, delta.server_sequence
            ).into());
        }
        if !harmony_spent(before.harmony, vitals.harmony) {
            return Err(format!(
                "C12 Greater Tiger Clash must spend Harmony 1 -> 0, observed {} -> {}",
                before.harmony, vitals.harmony
            )
            .into());
        }
        if client.actor_vitals() != Some(&vitals) {
            return Err("C12 cast vitals were not applied to the client view".into());
        }
        // No other Greater Tiger Clash can run during this cooldown. Retain every
        // attributed hit before advancing to the next action, including late batches.
        let mut hits = Vec::new();
        super::client_row_c12::collect_hits(&in_flight, own, victim, spell, &mut hits);
        let session = hex_bytes(client.join_snapshot().game_session_id.as_bytes());
        let cast_at_us = committed_cast_clock(
            &self.cast_evidence_log,
            evidence_start,
            &format!(
                "SEAM_EVIDENCE c12_cast_clock session={session} command={} cast_at_us=",
                outcome.command_id
            ),
        )?;
        let mut timers = validate_full_timers(client.session_mut(), &in_flight, spell, cast_at_us)?;
        // Swift Jab has no personal deadline at this point. Its refusal proves the
        // shared attack-group gate independently of Greater Tiger Clash's timer.
        let builder = spell_by_words(client, "exori infir pug")?;
        super::client_row_c12::require_cooldown(client.session_mut(), builder).await?;
        client.service_liveness(Duration::from_secs(2)).await?;
        super::client_row_c12::require_cooldown(client.session_mut(), spell).await?;
        let window = tokio::time::Instant::now() + Duration::from_secs(8);
        while let Some(remaining) = window.checked_duration_since(tokio::time::Instant::now()) {
            client
                .service_liveness(remaining.min(Duration::from_millis(250)))
                .await?;
            let trailing = client.take_events();
            timers |= validate_full_timers(client.session_mut(), &trailing, spell, cast_at_us)?;
            super::client_row_c12::collect_hits(&trailing, own, victim, spell, &mut hits);
        }
        if !timers {
            return Err("C12 full spender cooldown projection missing".into());
        }
        let damage = hits.first().copied().unwrap_or(0);
        let after = *client.actor_vitals().ok_or("post-cast vitals absent")?;
        if damage == 0 {
            return Err(format!(
                "CAST reply={outcome:?} produced no attributed spell damage frame; mana={}->{}",
                before.mana, after.mana
            )
            .into());
        }
        if damage.saturating_mul(2) >= self.profile.health
            || client
                .world_entities()
                .is_none_or(|entities| entities.get(&victim).is_none())
        {
            return Err(format!("damage={damage} may be capped by {} health={}; needed=creature-with-spell-proof-health", self.profile.display_name, self.profile.health).into());
        }
        let session = client.join_snapshot().game_session_id;
        let marker = format!(
            "SEAM_EVIDENCE c12_base_damage session={} command={} target={} generation={} ",
            hex_bytes(session.as_bytes()),
            outcome.command_id,
            hex_bytes(&victim.identity),
            victim.generation,
        );
        let paired = committed_base_damage(&self.cast_evidence_log, evidence_start, &marker, bps)?;
        verify_harmony_evidence(
            &self.cast_evidence_log,
            evidence_start,
            &hex_bytes(session.as_bytes()),
            outcome.command_id,
            &victim,
            before.harmony,
            &paired,
        )?;
        if !super::client_row_c12::frame_capture::exact_hit(&hits, paired.committed) {
            return Err(format!(
                "C12 owner HP loss does not match served damage: owner={paired:?} served={hits:?}"
            )
            .into());
        }
        evidence(&format!(
            "C12 committed_cast command={} damage={damage} owner={paired:?} expected_bps={bps} harmony={}->{} mana={mana_before_cast}->{}",
            outcome.command_id, before.harmony, vitals.harmony, vitals.mana
        ));
        verdict::Harness::global().progress("c12-verified-cast");
        Ok((damage, vitals))
    }
}

pub(super) fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct CommittedDamage {
    pub(super) before: u64,
    pub(super) augmented: u64,
    pub(super) critical: bool,
    pub(super) committed: u64,
}

pub(super) fn committed_cast_clock(path: &Path, offset: u64, marker: &str) -> TestResult<u64> {
    use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut clock = None;
    for line in BufReader::new(file.take(512 * 1024)).lines() {
        if let Some(value) = line?.strip_prefix(marker) {
            if clock.replace(value.parse()?).is_some() {
                return Err("duplicate C12 applied cast clock".into());
            }
        }
    }
    clock.ok_or_else(|| format!("missing C12 applied cast clock for {marker}").into())
}

pub(super) fn authored_cooldown(
    expires_at_ms: u64,
    read_at_ms: u64,
    cast_at_us: u64,
    duration_ms: u64,
) -> bool {
    // The view floors its sample time and rounds deadlines up to milliseconds.
    let cast_at_ms = cast_at_us / 1_000;
    let elapsed = read_at_ms.checked_sub(cast_at_ms);
    let remaining = expires_at_ms.checked_sub(read_at_ms);
    elapsed
        .zip(remaining)
        .and_then(|(elapsed, remaining)| elapsed.checked_add(remaining))
        .is_some_and(|duration| (duration_ms..=duration_ms + 1).contains(&duration))
}

fn validate_full_timers<S: oteryn_dev_client::native_session::SessionStream>(
    session: &oteryn_dev_client::native_session::Session<S>,
    events: &[SessionEvent],
    spell: NonZeroU32,
    cast_at_us: u64,
) -> TestResult<bool> {
    if !super::client_row_c12::validate_cast_timers(session, events, spell, 8_000)? {
        return Ok(false);
    }
    use oteryn_protocol_oteryn::presentation::ActorCooldownKind;
    let exact = events.iter().any(|event| match event {
        SessionEvent::ActorCooldowns(update) => update.value.entries.iter().any(|entry| {
            entry.kind == ActorCooldownKind::Spell
                && entry.id == spell.get()
                && authored_cooldown(
                    entry.expires_at_ms,
                    update.value.server_now_ms,
                    cast_at_us,
                    8_000,
                )
        }),
        _ => false,
    });
    if !exact {
        return Err(format!(
            "C12 full cooldown deadline is inconsistent with applied cast clock {cast_at_us}us"
        )
        .into());
    }
    Ok(true)
}

#[test]
fn c12_cooldown_accounts_for_projection_delay_without_accepting_shortened_duration() {
    assert!(authored_cooldown(9_000, 1_500, 1_000_000, 8_000));
    assert!(authored_cooldown(9_001, 1_500, 1_000_001, 8_000));
    assert!(!authored_cooldown(8_500, 1_500, 1_000_000, 8_000));
    assert!(!authored_cooldown(9_000, 999, 1_000_000, 8_000));
}

pub(super) fn committed_base_damage(
    path: &Path,
    start: u64,
    marker: &str,
    bps: u64,
) -> TestResult<CommittedDamage> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(start))?;
    read_damage_evidence(
        std::io::BufReader::new(file.take(512 * 1024)),
        marker,
        bps,
        true,
    )
}

pub(super) fn committed_authored_damage(
    path: &Path,
    offset: u64,
    marker: &str,
    bps: u64,
) -> TestResult<CommittedDamage> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    // The authored Tarantula has mitigation; compare the exact augment before mitigation.
    read_damage_evidence(
        std::io::BufReader::new(file.take(512 * 1024)),
        marker,
        bps,
        false,
    )
}

#[cfg(test)]
fn read_base_damage(
    reader: impl std::io::BufRead,
    marker: &str,
    bps: u64,
) -> TestResult<CommittedDamage> {
    read_damage_evidence(reader, marker, bps, true)
}

fn read_damage_evidence(
    reader: impl std::io::BufRead,
    marker: &str,
    bps: u64,
    neutral: bool,
) -> TestResult<CommittedDamage> {
    let mut observed = None;
    for line in reader.lines() {
        let line = line?;
        let Some(fields) = line.strip_prefix(marker) else {
            continue;
        };
        let (before, after) = fields
            .strip_prefix("before=")
            .and_then(|s| s.split_once(" after="))
            .ok_or("malformed committed C12 damage pair")?;
        let (after, modifiers) = after
            .split_once(" critical=")
            .ok_or("C12 critical observation missing")?;
        let (critical, committed) = modifiers
            .split_once(" element=physical committed=")
            .ok_or("C12 physical committed HP loss missing")?;
        let (committed, hits) = committed
            .split_once(" committed_hits=")
            .ok_or("C12 committed hit count missing")?;
        let (hits, observed_bps) = hits
            .split_once(" augment_bps=")
            .ok_or("C12 applied augment evidence missing")?;
        if observed_bps.parse::<u64>()? != bps {
            return Err(format!(
                "C12 expected {bps} base-damage basis points, observed {observed_bps}"
            )
            .into());
        }
        if hits.parse::<usize>()? != 1 {
            return Err(format!("C12 cast committed {hits} HP losses instead of one").into());
        }
        if observed
            .replace(CommittedDamage {
                before: before.parse()?,
                augmented: after.parse()?,
                critical: critical.parse()?,
                committed: committed.parse()?,
            })
            .is_some()
        {
            return Err("multiple committed C12 damage pairs for one cast and target".into());
        }
    }
    let damage =
        observed.ok_or_else(|| format!("committed C12 damage pair absent for {marker}"))?;
    let CommittedDamage {
        before,
        augmented: after,
        critical,
        committed,
    } = damage;
    let increase = before
        .checked_mul(bps)
        .ok_or("C12 augment arithmetic overflow")?
        / 10_000;
    if before == 0
        || committed == 0
        || (bps != 0 && increase == 0)
        || before.checked_add(increase) != Some(after)
    {
        return Err(format!(
            "C12 exact base damage comparison failed: before={before} after={after} bps={bps}"
        )
        .into());
    }
    // The solo, neutral fixture has no damage modifiers after base except innate Monk critical.
    let expected = if critical {
        after
            .checked_add(after / 10)
            .ok_or("C12 critical arithmetic overflow")?
    } else {
        after
    };
    if (neutral && committed != expected) || (!neutral && committed > expected) {
        return Err(format!("C12 committed HP loss {committed} differs from downstream damage {expected} for {damage:?}").into());
    }
    Ok(damage)
}

#[test]
fn c12_authored_damage_rejects_missing_augment_before_mitigation() -> TestResult {
    let marker = "SEAM_EVIDENCE c12_base_damage ";
    let sample = |after| {
        format!(
            "{marker}before=50 after={after} critical=false element=physical committed=40 committed_hits=1 augment_bps=800"
        )
    };
    assert_eq!(
        read_damage_evidence(sample(54).as_bytes(), marker, 800, false)?.committed,
        40
    );
    assert!(read_damage_evidence(sample(50).as_bytes(), marker, 800, false).is_err());
    Ok(())
}

/// Damage to `victim` in presentation batches that also carry `own`'s SpellWords in the same decision.
pub(super) fn cast_damage_in(
    event: &SessionEvent,
    own: oteryn_dev_client::EntityRef,
    victim: oteryn_dev_client::EntityRef,
    spell: u32,
) -> u64 {
    let SessionEvent::Presentation(delta) = event else {
        return 0;
    };
    cast_damage_from_presentation(&delta.value, own, victim, spell)
}

fn cast_damage_from_presentation(
    events: &[oteryn_protocol_oteryn::presentation::PresentationEvent],
    own: oteryn_dev_client::EntityRef,
    victim: oteryn_dev_client::EntityRef,
    spell: u32,
) -> u64 {
    let decisions: Vec<u32> = events
        .iter()
        .filter(
            |e| matches!(e.kind, PresentationEventKind::SpellWords { caster, spell: cue, .. } if caster == own && cue == spell),
        )
        .map(|e| e.decision_ordinal)
        .collect();
    events
        .iter()
        .filter(|e| decisions.contains(&e.decision_ordinal))
        .filter_map(|e| match e.kind {
            PresentationEventKind::ValueText {
                kind: PresentationValueKind::DamageDealt,
                target,
                attacker: Some(attacker),
                primary,
                secondary: None,
                ..
            } if target == victim
                && attacker == own
                && primary.color
                    == oteryn_protocol_oteryn::presentation::PresentationColor::Red =>
            {
                Some(primary.value)
            }
            _ => None,
        })
        .sum()
}

async fn samples(
    hunt: &Hunt,
    client: &mut DevClientSession,
    spell: NonZeroU32,
    bps: u64,
) -> TestResult<Vec<u64>> {
    let builder = spell_by_words(client, "exori infir pug")?;
    let mut out = Vec::new();
    for _ in 0..SAMPLES {
        restore_cast_mana(client).await?;
        build_harmony(hunt, client, builder).await?;
        let before = client
            .actor_vitals()
            .ok_or("pre-sample vitals absent")?
            .mana;
        out.push(hunt.cast_damage(client, spell, bps).await?.0);
        if client
            .actor_vitals()
            .ok_or("post-sample vitals absent")?
            .mana
            >= before
        {
            return Err(format!(
                "Cast without a mana debit: mana {before} -> {}",
                client.actor_vitals().map_or(0, |v| v.mana)
            )
            .into());
        }
    }
    Ok(out)
}

async fn build_harmony(
    hunt: &Hunt,
    client: &mut DevClientSession,
    spell: NonZeroU32,
) -> TestResult {
    let target = hunt.target(client).await?;
    let own = own_entity(client)?;
    let before = *client.actor_vitals().ok_or("C12 builder vitals absent")?;
    if before.harmony != 0 {
        return Err(format!(
            "C12 Swift Jab requires zero Harmony before cast, observed {}",
            before.harmony
        )
        .into());
    }
    client.take_events();
    let outcome = client
        .cast_spell(spell, SpellTarget::AttackTarget, true)
        .await?;
    require_mechanic_acceptance(outcome.status, "C12 Swift Jab CAST")?;
    let effect = outcome
        .actor_vitals_delta
        .ok_or("C12 Swift Jab omitted actor-vitals")?;
    if outcome.disposition != SpellCastDisposition::Cast || effect.value.harmony != 1 {
        return Err(format!(
            "C12 Swift Jab did not build one Harmony: {:?}",
            effect.value
        )
        .into());
    }
    let mut evidence = client.take_events();
    let mana = super::client_row_c12::frame_capture::mana_before_cast(
        evidence.iter().filter_map(|event| match event {
            SessionEvent::ActorVitals(update) => Some((update.server_sequence, update.value.mana)),
            _ => None,
        }),
        before.mana,
        before.max_mana,
        effect.server_sequence,
    );
    if mana.and_then(|mana| mana.checked_sub(3)) != Some(effect.value.mana) {
        return Err("C12 full Swift Jab must spend exactly 3 mana".into());
    }
    // Swift Jab's authored attack group lasts two seconds.
    client.service_liveness(Duration::from_secs(2)).await?;
    evidence.extend(client.take_events());
    if !super::client_row_c12::validate_cast_timers(client.session_mut(), &evidence, spell, 2_000)?
    {
        return Err("C12 full Swift Jab cooldown projection missing".into());
    }
    if evidence
        .iter()
        .all(|event| cast_damage_in(event, own, target, spell.get()) == 0)
    {
        return Err("C12 Swift Jab served no attributed physical damage".into());
    }
    Ok(())
}

async fn restore_cast_mana(client: &mut DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;

    if client
        .actor_vitals()
        .is_some_and(|vitals| vitals.mana >= CAST_MANA + 3)
    {
        return Ok(());
    }
    let potion = provisioning::item_reference(&provisioning::item_index()?, MANA_POTION)?;
    let before = carried_count(client, potion);
    let handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == potion)
        })
        .map(|entry| entry.handle)
        .ok_or("C12 has insufficient mana and no carried Mana Potion")?;
    let result = client.use_item(handle).await?;
    require_mechanic_acceptance(result.status, "C12 Mana Potion USE")?;
    if result.disposition != UseDisposition::Committed {
        return Err(format!("Mana Potion USE refused: {result:?}").into());
    }
    let deadline = tokio::time::Instant::now() + COMMAND_BUDGET;
    while !client
        .actor_vitals()
        .is_some_and(|vitals| vitals.mana >= CAST_MANA + 3)
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("Mana Potion did not publish enough mana for Greater Tiger Clash".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    let expected = before
        .checked_sub(1)
        .ok_or("C12 Mana Potion pre-state was empty")?;
    wait_inventory(client, "Mana Potion consumption", |client| {
        carried_count(client, potion) == expected
    })
    .await
}

pub(super) async fn equip_right(
    client: &mut DevClientSession,
    definition: NonZeroU32,
) -> TestResult {
    let carried = carried_count(client, definition);
    if equipped_right(client, definition) || carried == 0 {
        return Err("right-hand equip requires a carried item and a different pre-state".into());
    }
    let handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|e| e.item_definition_ref == definition)
        })
        .map(|e| e.handle)
        .ok_or("augment item not carried")?;
    let moved = client
        .session_mut()
        .move_item_counted(
            oteryn_dev_client::native_session::item_movement::MoveRequest {
                source: handle,
                destination: items::ItemMoveDestination::Equipment(items::EquipmentSlot::RightHand),
                count: None,
            },
        )
        .await?;
    if moved.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("right-hand ITEM_MOVE reply={moved:?}").into());
    }
    wait_inventory(client, "right-hand equipment", |c| {
        equipped_right(c, definition) && carried_count(c, definition) == carried - 1
    })
    .await
}

fn combat_target(
    client: &mut DevClientSession,
) -> TestResult<Option<oteryn_dev_client::EntityRef>> {
    Ok(client
        .session_mut()
        .combat_state()
        .ok_or("C12 combat state absent")?
        .target)
}

pub(super) async fn wait_combat_target(
    client: &mut DevClientSession,
    expected: Option<oteryn_dev_client::EntityRef>,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + COMMAND_BUDGET;
    while combat_target(client)? != expected {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("C12 combat target did not become {expected:?}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    Ok(())
}

pub(super) async fn logout(client: &mut DevClientSession, prove_mana: bool) -> TestResult<u32> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(75);
    let state = client
        .session_mut()
        .combat_state()
        .ok_or("C12 combat state absent before logout")?;
    if state.target.is_some() {
        let reply = client.session_mut().attack_target(None).await?;
        require_mechanic_acceptance(reply.status, "C12 clear attack")?;
        if reply.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
            return Err(format!("C12 clear attack before logout refused: {reply:?}").into());
        }
        wait_combat_target(client, None).await?;
    }
    if client
        .session_mut()
        .combat_state()
        .is_some_and(|state| state.in_fight)
    {
        verdict::Harness::global().bounded_wait(deadline, "c12-combat-lock");
    }
    while client
        .session_mut()
        .combat_state()
        .is_some_and(|state| state.in_fight)
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("C12 combat lock did not expire before logout".into());
        }
        client.service_liveness(Duration::from_millis(250)).await?;
    }
    verdict::Harness::global().progress("c12-logout");
    let mut mana = client
        .actor_vitals()
        .ok_or("C12 pre-logout vitals absent")?
        .mana;
    if prove_mana {
        // Observe the next wire regeneration tick just before the one logout request.
        // This makes the tested value distinct from an older below-max save as well as max mana.
        let prior = mana;
        let until = tokio::time::Instant::now() + Duration::from_secs(12);
        while mana == prior {
            if tokio::time::Instant::now() >= until {
                return Err("C12 mana regeneration did not publish before logout".into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
            mana = client
                .actor_vitals()
                .ok_or("C12 observed mana tick missing")?
                .mana;
        }
        // The seeded Monk receives 1.5 mana units per tick, with fractional carry.
        if !matches!(mana.checked_sub(prior), Some(1 | 2)) {
            return Err(format!(
                "C12 expected one regeneration tick before logout, observed {prior}->{mana}"
            )
            .into());
        }
    }
    let (result, detail, _, _) = logout_once(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C12 logout refused on first attempt: {detail}").into());
    }
    Ok(mana)
}

fn weapon(client: &DevClientSession) -> TestResult<Option<NonZeroU32>> {
    Ok(client
        .inventory()
        .ok_or("weapon control inventory absent")?
        .equipment
        .iter()
        .find(|entry| entry.slot == items::EquipmentSlot::RightHand)
        .map(|entry| entry.item.item_definition_ref))
}

fn pema_planned_price() -> TestResult<u64> {
    let trades = read_json("content/services/trade/trade-00000-00323.json")?;
    let key = format!("oteryn:item.tibia.i{ITEM_ID}");
    let authored = trades["records"]
        .as_array()
        .ok_or("trade records absent")?
        .iter()
        .map(|record| &record["declaration"])
        .filter(|declaration| {
            declaration["identity"]["key"] == "oteryn:service.trade.enpa_deia_pema"
        })
        .flat_map(|declaration| declaration["offers"].as_array().into_iter().flatten())
        .find(|offer| offer["direction"] == "SellToPlayer" && offer["item"]["key"] == key)
        .and_then(|offer| offer["unit_price"].as_u64())
        .ok_or("authored Enpa-Deia Pema price for item 50271 absent")?;
    if authored != PEMA_PRICE {
        return Err(format!("C12 expected Pema price {PEMA_PRICE}, authored {authored}").into());
    }
    Ok(authored)
}

pub(super) async fn walk_exact_route(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, from, direction).ok_or("C12 planned STEP has no landing")?;
        if expected == from {
            return Err("C12 planned STEP does not change position".into());
        }
        let session = hex_bytes(client.join_snapshot().game_session_id.as_bytes());
        let command = client.session_mut().next_command_id();
        evidence(&format!(
            "C12_FULL_COMMAND session={session} command={command} operation=STEP direction={direction:?} from={from:?} expected={expected:?}"
        ));
        let step = client.step(direction).await?;
        require_mechanic_acceptance(step.status, "C12 STEP")?;
        let actual = tile(client.world_spatial().actor_position)?;
        if step.disposition != StepDisposition::Moved
            || step.world_spatial_delta.is_none()
            || actual != expected
        {
            return Err(format!(
                "C12 purchase STEP {direction:?} from={from:?} expected={expected:?} actual={actual:?}: {step:?}"
            ).into());
        }
        verdict::Harness::global().progress("walk");
        client
            .session_mut()
            .service_liveness_discarding_events(Duration::from_millis(u64::from(step.step_wait_ms)))
            .await?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct GuildPortals {
    entrance: TilePos,
    valley_landing: TilePos,
    exit: TilePos,
    guild_landing: TilePos,
}

// TibiaWiki BR marks the sphere beside Pema at (33605, 31468, 7).
// Global places the other sphere on the Adventurers' Guild island. The World
// callback supplies the exact landing, which must lie in the Guild's sector.
fn guild_portals(callbacks: &serde_json::Value) -> TestResult<GuildPortals> {
    let read_tile = |value: &serde_json::Value| -> TestResult<TilePos> {
        Ok(TilePos {
            x: value["x"]
                .as_u64()
                .ok_or("C12 callback x missing")?
                .try_into()?,
            y: value["y"]
                .as_u64()
                .ok_or("C12 callback y missing")?
                .try_into()?,
            floor: value["floor"]
                .as_i64()
                .ok_or("C12 callback floor missing")?
                .try_into()?,
        })
    };
    let route = |id| -> TestResult<(TilePos, TilePos)> {
        let key = format!("oteryn:quest-callback/batch3/the_way_of_the_monk/blue-valley-{id}");
        let rows = callbacks["callbacks"]
            .as_array()
            .ok_or("C12 callback registry missing")?;
        let mut matches = rows
            .iter()
            .filter(|row| row["key"] == key && row["trigger"] == "use");
        let row = matches
            .next()
            .ok_or_else(|| format!("C12 Global USE callback missing: {key}"))?;
        if matches.next().is_some() {
            return Err(format!("C12 ambiguous Global USE callback: {key}").into());
        }
        Ok((
            read_tile(&row["selector"]["position"])?,
            read_tile(&row["teleport"])?,
        ))
    };
    let (entrance, valley_landing) = route(5)?;
    let (exit, guild_landing) = route(2)?;
    if guild_landing != STONE_ARRIVAL {
        return Err(format!(
            "BLOCKED-CONTENT: blue-valley-2 Guarded Crystal Sphere at ({},{},{}) returns to ({},{},{}), expected exact Guild arrival tile",
            exit.x, exit.y, exit.floor, guild_landing.x, guild_landing.y, guild_landing.floor,
        ).into());
    }
    if entrance
        != (TilePos {
            x: 32203,
            y: 32303,
            floor: -6,
        })
        || valley_landing
            != (TilePos {
                x: 33614,
                y: 31494,
                floor: -7,
            })
        || exit
            != (TilePos {
                x: 33605,
                y: 31468,
                floor: -7,
            })
    {
        return Err(format!("C12 Global portal source or Blue Valley landing mismatch: {entrance:?} -> {valley_landing:?}; {exit:?} -> {guild_landing:?}").into());
    }
    Ok(GuildPortals {
        entrance,
        valley_landing,
        exit,
        guild_landing,
    })
}

async fn use_blue_valley_portal(
    client: &mut DevClientSession,
    world: &BundleWorld,
    source: TilePos,
    sphere: u16,
    landing: TilePos,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;

    let source_tile = world
        .base()
        .tile(source.x, source.y, source.floor)
        .ok_or_else(|| format!("C12 Global sphere tile missing from World bundle: {source:?}"))?;
    let mut ordinals =
        (0..=u8::MAX).filter(|&ordinal| source_tile.appearance(ordinal) == Some(sphere));
    let ordinal = ordinals
        .next()
        .ok_or_else(|| format!("C12 Global sphere {sphere} absent at {source:?}"))?;
    if ordinals.next().is_some() {
        return Err(format!("C12 Global sphere {sphere} ambiguous at {source:?}").into());
    }
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == source.floor && at.x.abs_diff(source.x).max(at.y.abs_diff(source.y)) <= 1
    })
    .ok_or_else(|| format!("C12 no playable route to Blue Valley portal {source:?}"))?;
    walk_exact_route(client, world, &path).await?;
    if tile(client.world_spatial().actor_position)? == landing {
        return Err("C12 actor was already on the portal landing before USE".into());
    }
    let key = crate::map::view::placement_key(source, ordinal)
        .ok_or("C12 Blue Valley portal placement key absent")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let reply = client.use_object(&placement, 0).await?;
    require_mechanic_acceptance(reply.status, "C12 Blue Valley portal USE")?;
    if reply.disposition != UseDisposition::Committed {
        return Err(format!("C12 Blue Valley portal {source:?} USE reply={reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + COMMAND_BUDGET;
    while tile(client.world_spatial().actor_position)? != landing {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "C12 Blue Valley portal {source:?} expected landing {landing:?}, observed {:?}",
                client.world_spatial().actor_position
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    evidence(&format!("C12_PORTAL source={source:?} landing={landing:?}"));
    Ok(())
}

fn thais_temple(world: &BundleWorld) -> TestResult<TilePos> {
    let temple = world
        .base()
        .subareas
        .town(THAIS)
        .and_then(|area| area.temple)
        .ok_or("C12 authored Thais temple absent")?;
    Ok(TilePos {
        x: temple.x,
        y: temple.y,
        floor: temple.floor,
    })
}

async fn stone_origin(clients: &SeamClients<'_>) -> TestResult<Option<i64>> {
    use sqlx::Connection;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    Ok(sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks
          WHERE character_id=encode($1,'hex')::uuid
            AND track_key='oteryn:quest-progress/storage/52130'
            AND quest_key='oteryn:quest.adventurers_guild.stone'",
    )
    .bind(clients.characters[1].as_slice())
    .fetch_optional(&mut db)
    .await?)
}

async fn enter_guild(
    client: &mut DevClientSession,
    world: &BundleWorld,
    clients: &SeamClients<'_>,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let temple = thais_temple(world)?;
    if world
        .base()
        .tile(STONE_ARRIVAL.x, STONE_ARRIVAL.y, STONE_ARRIVAL.floor)
        .is_none_or(|cell| !cell.walkable())
    {
        return Err("C12 authored Stone arrival is missing or unwalkable".into());
    }
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == temple
    })
    .ok_or("C12 cannot walk to the authored Thais temple")?;
    walk_exact_route(client, world, &path).await?;
    if world
        .base()
        .tile(temple.x, temple.y, temple.floor)
        .is_none_or(|cell| !cell.protection_zone())
        || tile(client.world_spatial().actor_position)? != temple
        || stone_origin(clients).await?.is_some()
    {
        return Err("C12 Stone requires the Thais temple and no remembered origin".into());
    }
    let definition = provisioning::item_reference(&provisioning::item_index()?, ADVENTURERS_STONE)?;
    let handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == definition)
        })
        .ok_or("C12 carried Stone absent")?
        .handle;
    if carried_count(client, definition) != 1 {
        return Err("C12 requires exactly one carried Stone".into());
    }
    let used = client.use_item(handle).await?;
    require_mechanic_acceptance(used.status, "C12 Adventurer's Stone USE")?;
    let landed = tile(client.world_spatial().actor_position)?;
    let origin = stone_origin(clients).await?;
    if used.disposition != UseDisposition::Committed
        || landed != STONE_ARRIVAL
        || origin != Some(i64::from(THAIS))
        || carried_count(client, definition) != 1
    {
        return Err(format!("C12 Stone USE expected {STONE_ARRIVAL:?} and recorded Thais, got {used:?} at {landed:?} origin={origin:?}").into());
    }
    evidence(&format!(
        "C12_STONE temple={temple:?} landing={landed:?} origin={THAIS}"
    ));
    Ok(())
}

async fn leave_guild(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    clients: &SeamClients<'_>,
) -> TestResult {
    let origin = stone_origin(clients).await?;
    if origin != Some(i64::from(THAIS)) {
        return Err(format!("C12 Guild exit expected recorded Thais, got {origin:?}").into());
    }
    let npc_cells: std::collections::BTreeSet<_> = npcs
        .npcs
        .iter()
        .flat_map(|npc| {
            npc.placements.iter().map(|p| TilePos {
                x: p.x,
                y: p.y,
                floor: p.floor,
            })
        })
        .collect();
    let path = route_avoiding(
        world,
        tile(client.world_spatial().actor_position)?,
        |at| at == GUILD_EXIT_STEP,
        |at| at != GUILD_EAST_EXIT && at != GUILD_WEST_EXIT && !npc_cells.contains(&at),
    )
    .ok_or("C12 cannot walk to the eastern Adventurers' Guild exit")?;
    walk_exact_route(client, world, &path).await?;
    let step = client.step(StepDirection::North).await?;
    require_mechanic_acceptance(step.status, "C12 Guild exit STEP")?;
    let landing = tile(client.world_spatial().actor_position)?;
    let temple = thais_temple(world)?;
    let consumed = stone_origin(clients).await?;
    if step.disposition != StepDisposition::Moved
        || step.world_spatial_delta.is_none()
        || landing != temple
        || consumed != Some(-1)
    {
        return Err(format!("C12 Guild exit expected {temple:?} and consumed origin, got {step:?} at {landing:?} origin={consumed:?}").into());
    }
    evidence(&format!(
        "C12_GUILD_EXIT source={GUILD_EAST_EXIT:?} landing={landing:?} consumed_origin={THAIS}"
    ));
    client
        .session_mut()
        .service_liveness_discarding_events(Duration::from_millis(u64::from(step.step_wait_ms)))
        .await?;
    Ok(())
}

async fn fists_proficiency(
    clients: &SeamClients<'_>,
) -> TestResult<Option<(i64, Vec<Option<i16>>)>> {
    use sqlx::Connection;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    Ok(sqlx::query_as(
        "SELECT progress, selections FROM game_character_proficiency
          WHERE character_id=encode($1,'hex')::uuid AND item_key='oteryn:item.tibia.i50271'",
    )
    .bind(clients.characters[1].as_slice())
    .fetch_optional(&mut db)
    .await?)
}

fn fists_damage_perks_selected(track: Option<(i64, Vec<Option<i16>>)>) -> bool {
    track.is_some_and(|(xp, perks)| xp == 100_000 && perks == [Some(0), None, Some(0), None])
}

pub(super) async fn require_fists_damage_perks(clients: &SeamClients<'_>) -> TestResult {
    if !fists_damage_perks_selected(fists_proficiency(clients).await?) {
        return Err("C12 expected durable Fists proficiency of 100000 XP and perks 1 and 3".into());
    }
    Ok(())
}

#[test]
fn c12_durable_fists_perks_refuse_missing_xp_or_selections() {
    let selected = vec![Some(0), None, Some(0), None];
    assert!(fists_damage_perks_selected(Some((
        100_000,
        selected.clone()
    ))));
    assert!(!fists_damage_perks_selected(Some((99_999, selected))));
    assert!(!fists_damage_perks_selected(Some((
        100_000,
        vec![Some(0), None, None, None]
    ))));
    assert!(!fists_damage_perks_selected(None));
}

async fn select_fists_damage_perks(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    weapon: NonZeroU32,
) -> TestResult {
    use oteryn_protocol_oteryn::{weapon_proficiency as perks, world_object as wire};
    let catalyst =
        provisioning::item_reference(&provisioning::item_index()?, GREATER_PROFICIENCY_CATALYST)?;
    if carried_count(client, catalyst) != 1 || fists_proficiency(clients).await?.is_some() {
        return Err("C12 requires one catalyst and a fresh Fists proficiency track".into());
    }
    let inventory = client
        .inventory()
        .ok_or("C12 proficiency inventory absent")?;
    let handle = |definition| {
        inventory
            .entries
            .iter()
            .find(|entry| entry.item_definition_ref == definition)
            .map(|entry| entry.handle)
            .ok_or("C12 exact proficiency item handle missing")
    };
    let intent = wire::encode_item_use_intent(&wire::UseIntent {
        target: wire::UseTarget::Item(handle(catalyst)?),
        use_with: None,
        use_with_map_item: Some(wire::MapUseTarget::Item(handle(weapon)?)),
        use_with_position: None,
    })
    .map_err(|error| format!("C12 catalyst USE encode: {error:?}"))?;
    let reply = client.session_mut().world_context_use(&intent).await?;
    require_mechanic_acceptance(reply.status, "C12 Greater Proficiency Catalyst USE")?;
    let (disposition, _) = wire::decode_use_result_with_gate(&reply.payload)
        .map_err(|error| format!("C12 catalyst USE decode: {error:?}"))?;
    if disposition != wire::UseDisposition::Committed {
        return Err(format!("C12 catalyst USE: {disposition:?}").into());
    }
    wait_inventory(
        client,
        "Greater Proficiency Catalyst consumption",
        |client| carried_count(client, catalyst) == 0,
    )
    .await?;
    let mut selected: [Option<u8>; 4] = [None; 4];
    let progress = 100_000;
    if fists_proficiency(clients).await?
        != Some((progress, selected.map(|perk| perk.map(i16::from)).to_vec()))
    {
        return Err("C12 catalyst did not unlock exactly 100000 XP without selections".into());
    }
    for level in [0_u8, 2] {
        let request = perks::SelectPerkRequest {
            definition: weapon,
            level,
            perk: Some(0),
        };
        let reply = client
            .session_mut()
            .gated_command(
                perks::CAPABILITY_WEAPON_PROFICIENCY_V1,
                perks::COMMAND_TYPE_PROFICIENCY_SELECT_PERK,
                &request.encode(),
            )
            .await?;
        require_mechanic_acceptance(reply.status, "C12 select Fists damage perk")?;
        let reply = perks::SelectPerkReply::decode(&reply.payload)
            .ok_or("C12 malformed Select Perk result")?;
        selected[usize::from(level)] = Some(0);
        if reply.result != perks::SelectPerkResult::Accepted
            || reply.selections != selected
            || fists_proficiency(clients).await?
                != Some((progress, selected.map(|perk| perk.map(i16::from)).to_vec()))
        {
            return Err(format!("C12 Fists perk {} did not commit: {reply:?}", level + 1).into());
        }
    }
    evidence("C12_PROFICIENCY xp=100000 selected_levels=1,3");
    Ok(())
}

async fn acquire_pema(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    definition: NonZeroU32,
    portals: GuildPortals,
) -> TestResult<u64> {
    use oteryn_dev_client::client_runtime::npc_shop;
    use oteryn_protocol_oteryn::npc_service as npc;

    let seller = npcs
        .npcs
        .iter()
        .find(|entry| entry.key == "oteryn:npc.enpa_deia_pema")
        .ok_or("Enpa-Deia Pema absent from the compiled NPC table")?;
    const PEMA_TILE: TilePos = TilePos {
        x: 33603,
        y: 31468,
        floor: -7,
    };
    if !matches!(seller.placements.as_slice(), [only]
        if (only.x, only.y, only.floor) == (PEMA_TILE.x, PEMA_TILE.y, PEMA_TILE.floor))
    {
        return Err(
            "C12 Pema must have exactly the independently pinned Blue Valley placement".into(),
        );
    }
    let near_placement = |at: TilePos, placement: TilePos| {
        at.floor == placement.floor
            && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3
    };
    let placements = seller.placements.iter().map(|p| TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    });
    let route_to_seller = |from| {
        route(world, from, |at| {
            placements
                .clone()
                .any(|placement| near_placement(at, placement))
        })
        .ok_or("no playable route within three tiles of a compiled Enpa-Deia Pema placement")
    };
    use_blue_valley_portal(
        client,
        world,
        portals.entrance,
        50_310,
        portals.valley_landing,
    )
    .await?;
    let path = route_to_seller(tile(client.world_spatial().actor_position)?)?;
    walk_exact_route(client, world, &path).await?;
    let here = tile(client.world_spatial().actor_position)?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            let mut matches = entities.iter().filter(|entry| {
                entry.kind == EntityKind::Npc
                    && entry.display_name.is_some_and(|name| {
                        name.as_str().is_ok_and(|name| name == "Enpa-Deia Pema")
                    })
                    && placements.clone().any(|placement| {
                        near_placement(here, placement)
                            && tile(entry.position).ok() == Some(placement)
                    })
            });
            let actor = matches.next()?.entity;
            matches.next().is_none().then_some(actor)
        })
        .ok_or_else(|| {
            let visible: Vec<_> = client
                .world_entities()
                .into_iter()
                .flat_map(|entities| entities.iter())
                .filter(|entry| entry.kind == EntityKind::Npc)
                .map(|entry| (entry.display_name, entry.position))
                .collect();
            format!(
                "expected one live Enpa-Deia Pema at a compiled placement; visible NPCs={visible:?}"
            )
        })?;

    let conversation_before = client
        .session_mut()
        .gated_snapshot(npc::STATE_DOMAIN_NPC_CONVERSATION)
        .ok_or("Pema conversation pre-state absent")?;
    if npc::decode_npc_conversation(&conversation_before.payload)
        .map_err(|error| format!("Pema conversation pre-state: {error:?}"))?
        .open
        .is_some()
    {
        return Err("Pema conversation was already open before greeting".into());
    }
    let reply = npc_shop::talk(
        client.session_mut(),
        &npc::NpcTalkIntent {
            npc_actor: actor,
            text: "hi".into(),
        },
    )
    .await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!("Enpa-Deia Pema hi reply={reply:?}").into());
    }
    let greeting = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    let conversation = npc::decode_npc_conversation(&greeting.payload)
        .map_err(|e| format!("Pema conversation: {e:?}"))?;
    if !conversation
        .open
        .is_some_and(|(speaker, _)| speaker == actor)
    {
        return Err("Pema greeting did not open a conversation for the resolved actor".into());
    }
    let details = client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::trade_details::CAPABILITY_TRADE_DETAILS_V1);
    let trade_before = client
        .session_mut()
        .gated_snapshot(npc::STATE_DOMAIN_NPC_TRADE_WINDOW)
        .ok_or("Pema trade window pre-state absent")?;
    if npc::decode_npc_trade_window_for(&trade_before.payload, details)
        .map_err(|error| format!("Pema trade window pre-state: {error:?}"))?
        .open
        .is_some()
    {
        return Err("Pema trade window was already open before trade".into());
    }
    let reply = npc_shop::talk(
        client.session_mut(),
        &npc::NpcTalkIntent {
            npc_actor: actor,
            text: "trade".into(),
        },
    )
    .await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!("Enpa-Deia Pema trade reply={reply:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = npc::decode_npc_trade_window_for(&delta.payload, details)
        .map_err(|e| format!("Pema trade window: {e:?}"))?;
    let (window_actor, catalogue_revision) = window.open.ok_or("Pema trade window closed")?;
    if window_actor != actor {
        return Err("trade window opened for a different NPC".into());
    }
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == npc::NpcTradeSide::Buy && offer.item_definition_ref == definition.get()
        })
        .ok_or("Pema trade window does not sell item 50271")?;
    if offer.count != 1 {
        return Err(format!(
            "Pema offer must contain one item 50271, served {}",
            offer.count
        )
        .into());
    }
    let price = pema_planned_price()?;
    if u64::from(offer.unit_price) != price {
        return Err(format!(
            "Pema offer expected authored price {price}, served {}",
            offer.unit_price
        )
        .into());
    }
    client.inventory().ok_or("no inventory before Pema BUY")?;
    let before = full_e2ef7::units(client, definition);
    if before != 0 {
        return Err("item 50271 already present after reload, before Pema BUY".into());
    }
    let gold_before = full_e2ef7::player_gold(client).await?;
    client.take_events();
    let reply = npc_shop::buy(
        client.session_mut(),
        &npc::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!("Pema BUY reply={reply:?} price={}", offer.unit_price).into());
    }
    wait_inventory(client, "Pema BUY of item 50271", |c| {
        full_e2ef7::units(c, definition) == before + 1 && carried_count(c, definition) == 1
    })
    .await?;
    let gold_after = full_e2ef7::player_gold(client).await?;
    if gold_before.checked_sub(price) != Some(gold_after) {
        return Err(format!(
            "Pema BUY expected charge {price}, observed {gold_before} -> {gold_after}"
        )
        .into());
    }
    use_blue_valley_portal(client, world, portals.exit, 50_227, portals.guild_landing).await?;
    Ok(gold_after)
}

fn fixture_target_profile() -> TestResult<CreatureProfile> {
    let mut profiles = creature_profiles()?;
    let profile = profiles
        .remove(FIXTURE_TARGET)
        .ok_or("C12 fixture Magma Crystal absent from authored profiles")?;
    if profile.health != 22_000
        || profile.display_name != "Magma Crystal"
        || !profile.neutral_physical_target
    {
        return Err("C12 fixture Magma Crystal authored profile differs".into());
    }
    Ok(profile)
}

#[test]
fn c12_fixture_target_has_no_physical_reduction_or_movement() -> TestResult {
    let profile = fixture_target_profile()?;
    assert!(profile.neutral_physical_target);
    assert_eq!(profile.health, 22_000);
    Ok(())
}

fn fixture_combat_route(
    world: &BundleWorld,
    from: TilePos,
    move_before_cast: bool,
) -> TestResult<Vec<StepDirection>> {
    let combat_tile = |at: TilePos| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.walkable() && !tile.protection_zone())
            && world.enterable(at)
    };
    route(world, from, |at| {
        (!move_before_cast || at != from)
            && combat_tile(at)
            && at
                .y
                .checked_sub(1)
                .is_some_and(|y| combat_tile(TilePos { y, ..at }))
    })
    .ok_or_else(|| "C12 fixture has no reachable combat tile".into())
}

async fn prepare_fixture_hunt(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult<Hunt> {
    let from = tile(client.world_spatial().actor_position)?;
    let path = fixture_combat_route(world, from, false)?;
    walk_exact_route(client, world, &path).await?;
    let profile = fixture_target_profile()?;
    let entity = spawn_and_wait_fixture_target(client.session_mut(), &profile).await?;
    Ok(Hunt {
        entity,
        profile,
        cast_evidence_log: required("NODE_BOOT_CAST_EVIDENCE_LOG")?.into(),
    })
}

pub(super) async fn client_fixture_target<S: oteryn_dev_client::native_session::SessionStream>(
    session: &mut oteryn_dev_client::native_session::Session<S>,
) -> TestResult<oteryn_dev_client::EntityRef> {
    spawn_and_wait_fixture_target(session, &fixture_target_profile()?).await
}

async fn spawn_and_wait_fixture_target<S: oteryn_dev_client::native_session::SessionStream>(
    session: &mut oteryn_dev_client::native_session::Session<S>,
    profile: &CreatureProfile,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let id = uuid_text(session.join_snapshot().game_session_id.as_bytes());
    let answer = request_spawn(id, FIXTURE_TARGET.into())
        .await
        .map_err(|reason| format!("C12 fixture spawn refused: {reason}"))?;
    let entity = parse_spawned_entity(&answer)?;
    wait_fixture_target(session, entity, profile).await?;
    eprintln!("FIXTURE C12 target={FIXTURE_TARGET} {answer}");
    Ok(entity)
}

fn is_fixture_north(
    caster: oteryn_dev_client::native_session::ActorPosition,
    target: oteryn_dev_client::native_session::ActorPosition,
) -> bool {
    target.floor == caster.floor
        && target.x == caster.x
        && target.y.checked_add(1) == Some(caster.y)
}

async fn wait_fixture_target<S: oteryn_dev_client::native_session::SessionStream>(
    session: &mut oteryn_dev_client::native_session::Session<S>,
    entity: oteryn_dev_client::EntityRef,
    profile: &CreatureProfile,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        let at = session.world_spatial().actor_position;
        if let Some(entry) = session
            .world_entities()
            .and_then(|entities| entities.get(&entity))
        {
            if entry.kind != EntityKind::Creature
                || entry
                    .display_name
                    .is_none_or(|name| name.as_str().ok() != Some(profile.display_name.as_str()))
                || !is_fixture_north(at, entry.position)
            {
                return Err(format!(
                    "C12 fixture target differs or is out of range: {entry:?} at={at:?}"
                )
                .into());
            }
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("C12 fixture target never appeared: {entity:?}").into());
        }
        session
            .service_liveness_discarding_events(Duration::from_millis(100))
            .await?;
    }
}

pub(super) fn item_dictionary_pin(
    clients: &SeamClients<'_>,
) -> TestResult<super::client_row_c12::frame_capture::ItemDictionaryPin> {
    let world_bundle = std::path::PathBuf::from(required("NODE_BOOT_WORLD_BUNDLE")?);
    let deploy = world_bundle
        .parent()
        .and_then(Path::parent)
        .ok_or("C12 activation directory absent")?;
    let activation: serde_json::Value =
        serde_json::from_slice(&std::fs::read(deploy.join("state/content-1.json"))?)?;
    if activation["world_id"].as_str() != Some(uuid_text(&clients.world).as_str())
        || activation["channel_id"].as_str() != Some(uuid_text(&clients.channel).as_str())
        || activation["activation_sequence"].as_u64() != Some(1)
    {
        return Err("C12 Item dictionary activation scope differs".into());
    }
    let generation = hex32(
        activation["client_artifact_digest"]
            .as_str()
            .ok_or("C12 Item dictionary generation absent")?,
    )?;
    let index = provisioning::item_index()?;
    let count = (1..=100_001)
        .take_while(|&reference| {
            NonZeroU32::new(reference)
                .is_some_and(|reference| index.definition_at(reference).is_some())
        })
        .count();
    Ok(super::client_row_c12::frame_capture::ItemDictionaryPin {
        generation,
        count: u32::try_from(count)?,
    })
}

/// Read the terminal logout's persisted mana before any successor session can regenerate.
fn harmony_spent(before: u32, after: u32) -> bool {
    before == 1 && after == 0
}

fn mana_survived(observed: u32, saved: u32, reloaded: u32, max: u32) -> bool {
    observed < max && observed == saved && observed == reloaded
}

pub(super) async fn saved_mana(clients: &SeamClients<'_>) -> TestResult<u32> {
    use sqlx::Connection;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let mana = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT mana FROM game_character_last_positions
          WHERE character_id = encode($1,'hex')::uuid AND world_id = encode($2,'hex')::uuid",
    )
    .bind(clients.characters[1].as_slice())
    .bind(clients.world.as_slice())
    .fetch_one(&mut db)
    .await?
    .ok_or("C12 terminal position did not save mana")?;
    Ok(u32::try_from(mana)?)
}

pub(super) async fn connect_c12_character(
    clients: &SeamClients<'_>,
) -> TestResult<DevClientSession> {
    wait_character_released(clients, 1).await?;
    let grant = platform_row_grant(clients, 1).await?;
    let pin = item_dictionary_pin(clients)?;
    let refs = pin.sorted_refs()?;
    let client = oteryn_dev_client::connect_session_with_appearance_items(
        oteryn_dev_client::JoinRequest {
            address: clients.address,
            server_name: "localhost",
            root_certificate: clients.certificate,
            schema_revision: 1,
            character_id: crate::foundation::CharacterId::decode(&clients.characters[1])?,
            admission_material: grant.as_bytes(),
            client_build_id: "oteryn-dev-client/world-qualification",
            deadline: Duration::from_secs(20),
        },
        Some(oteryn_dev_client::AppearanceItemDictionary {
            content_generation: pin.generation,
            definition_refs: &refs,
        }),
    )
    .await?;
    verdict::Harness::global().progress("admission");
    Ok(client)
}

async fn fresh_c12_admission(
    mut client: DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<DevClientSession> {
    let previous = client.join_snapshot().game_session_id;
    logout(&mut client, false).await?;
    drop(client);
    let fresh = connect_c12_character(clients).await?;
    if fresh.join_snapshot().game_session_id == previous {
        return Err("C12 preparation reused its previous session".into());
    }
    Ok(fresh)
}

async fn prepare_c12_monk(clients: &SeamClients<'_>) -> TestResult<DevClientSession> {
    evidence(&format!(
        "character_preparation mode=Seeded profile={} reason={:?}",
        PREPARATION.profile, PREPARATION.reason
    ));
    let client = connect_c12_character(clients).await?;
    request_fixture_control_at(
        "NODE_BOOT_CONTROL_SOCKET",
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        PREPARATION.profile.into(),
    )
    .await?;
    fresh_c12_admission(client, clients).await
}

async fn verify_seeded_monk(client: &mut DevClientSession) -> TestResult {
    let level = full_e2ef7::player_level(client).await?;
    let vitals = client
        .actor_vitals()
        .ok_or("C12 seeded Monk has no actor-vitals snapshot")?;
    // Crystal's level 8 pools are 185 HP and 90 mana. Monk adds 10 to each per level.
    if (level, vitals.max_health, vitals.max_mana) != (80, 905, 810) {
        return Err(format!(
            "C12 expected level 80 Monk with 905 max HP and 810 max mana, observed level={level} vitals={vitals:?}"
        ).into());
    }
    Ok(())
}

pub(super) async fn require_current_premium<S: oteryn_dev_client::native_session::SessionStream>(
    session: &mut oteryn_dev_client::native_session::Session<S>,
) -> TestResult {
    use oteryn_protocol_oteryn::{character_info, premium_overview};
    let reply = session
        .gated_command(
            character_info::CAPABILITY_CHARACTER_INFO_V1,
            character_info::COMMAND_TYPE_CHARACTER_INFO,
            &[premium_overview::OPERATION],
        )
        .await?;
    require_mechanic_acceptance(reply.status, "C12 Premium overview")?;
    verdict::Harness::global().progress("premium");
    let overview = premium_overview::Overview::decode(&reply.payload)
        .ok_or("C12 Premium overview decode failed")?;
    evidence(&format!(
        "C12_PREMIUM state={:?} authority_revision={} remaining_us={}",
        overview.state, overview.authority_revision, overview.valid_for_us,
    ));
    if overview.state != premium_overview::State::Active || overview.valid_for_us == 0 {
        return Err("C12 durable Premium has no usable lease after fresh admission".into());
    }
    Ok(())
}

pub(super) struct ClientCaptureSetup {
    pub accounts: Vec<String>,
    pub characters: Vec<[u8; 16]>,
    pub definition: NonZeroU32,
    pub route: Vec<(StepDirection, TilePos)>,
    pub previous: oteryn_protocol_oteryn::GameSessionId,
    pub position: TilePos,
    pub purchase_balance: u64,
}

pub(super) const CLIENT_TARGET_NAME: &str = "Tarantula";
const CLIENT_TARGET_KEY: &str = "oteryn:creature.tarantula";
const CLIENT_TARGET_SPAWN: TilePos = TilePos {
    x: 32294,
    y: 32333,
    floor: -8,
};
const CLIENT_CAST_TILE: TilePos = TilePos {
    x: 32295,
    y: 32334,
    floor: -8,
};

fn authored_tarantula_respawn(spawns: &oteryn_world_bundle::spawn::Table) -> TestResult<Duration> {
    let mut matches = spawns
        .sources
        .iter()
        .filter(|source| source.floor == CLIENT_TARGET_SPAWN.floor)
        .flat_map(|source| &source.points)
        .filter(|point| {
            point.x == CLIENT_TARGET_SPAWN.x
                && point.y == CLIENT_TARGET_SPAWN.y
                && spawns
                    .creatures
                    .get(point.creature as usize)
                    .is_some_and(|creature| creature.key == CLIENT_TARGET_KEY)
        });
    let spawn = matches
        .next()
        .ok_or("C12 authored Tarantula spawn absent")?;
    if spawn.respawn_ms == 0 || matches.next().is_some() {
        return Err("C12 authored Tarantula respawn is zero or ambiguous".into());
    }
    Ok(Duration::from_millis(u64::from(spawn.respawn_ms)))
}

#[test]
fn c12_respawn_budget_requires_the_exact_authored_point() -> TestResult {
    use oteryn_world_bundle::spawn::{Creature, Direction, Period, Point, Source, Table};
    let mut table = Table {
        creatures: vec![Creature {
            key: CLIENT_TARGET_KEY.into(),
            period: Period::All,
        }],
        sources: vec![Source {
            key: "test".into(),
            floor: CLIENT_TARGET_SPAWN.floor,
            x: CLIENT_TARGET_SPAWN.x,
            y: CLIENT_TARGET_SPAWN.y,
            points: vec![Point {
                creature: 0,
                x: CLIENT_TARGET_SPAWN.x,
                y: CLIENT_TARGET_SPAWN.y,
                direction: Direction::North,
                respawn_ms: 90_000,
                boss_policy: None,
            }],
        }],
    };
    assert_eq!(authored_tarantula_respawn(&table)?, Duration::from_secs(90));
    let mut point = table.sources[0].points[0];
    point.x += 1;
    table.sources[0].points[0] = point;
    assert!(authored_tarantula_respawn(&table).is_err());
    Ok(())
}

fn authored_client_cast_route(
    world: &BundleWorld,
    from: TilePos,
) -> TestResult<Vec<(StepDirection, TilePos)>> {
    if from == CLIENT_CAST_TILE {
        return Err("C12.client authored hunt starting position differs".into());
    }
    let profiles = creature_profiles()?;
    if profiles
        .get(CLIENT_TARGET_KEY)
        .is_none_or(|profile| profile.health != 225 || profile.display_name != CLIENT_TARGET_NAME)
    {
        return Err("C12.client authored Tarantula profile differs".into());
    }
    let path = route(world, from, |at| at == CLIENT_CAST_TILE)
        .ok_or("C12.client no route to the authored Tarantula cast tile")?;
    let mut at = from;
    path.into_iter()
        .map(|direction| {
            let landed = destination(world, at, direction)
                .filter(|landed| *landed != at)
                .ok_or("C12.client planned movement has no changed landing")?;
            at = landed;
            Ok((direction, landed))
        })
        .collect::<Result<_, &'static str>>()
        .map_err(Into::into)
}

fn client_cast_route(
    world: &BundleWorld,
    from: TilePos,
) -> TestResult<Vec<(StepDirection, TilePos)>> {
    let path = fixture_combat_route(world, from, true)?;
    let mut at = from;
    path.into_iter()
        .map(|direction| {
            let landed = destination(world, at, direction)
                .filter(|landed| *landed != at)
                .ok_or("C12.client planned movement has no changed landing")?;
            at = landed;
            Ok((direction, landed))
        })
        .collect::<Result<_, &'static str>>()
        .map_err(Into::into)
}

fn return_occupant(
    kind: EntityKind,
    entity: oteryn_dev_client::EntityRef,
    position: TilePos,
    at: TilePos,
) -> Option<oteryn_dev_client::EntityRef> {
    (position == at
        && matches!(
            kind,
            EntityKind::Creature | EntityKind::Npc | EntityKind::Player
        ))
    .then_some(entity)
}

fn return_occupants(
    client: &mut DevClientSession,
    at: TilePos,
) -> TestResult<Vec<oteryn_dev_client::EntityRef>> {
    let entities = client
        .world_entities()
        .ok_or("C12 return entity view absent")?;
    Ok(entities
        .others()
        .filter_map(|entity| {
            return_occupant(entity.kind, entity.entity, tile(entity.position).ok()?, at)
        })
        .collect())
}

#[test]
fn c12_return_occupant_matches_tile_and_preserves_generation() {
    let at = TilePos {
        x: 32293,
        y: 32334,
        floor: -7,
    };
    let entity = oteryn_dev_client::EntityRef {
        identity: [9; 16],
        generation: 4,
    };
    let successor = oteryn_dev_client::EntityRef {
        generation: 5,
        ..entity
    };
    assert_eq!(
        return_occupant(EntityKind::Creature, entity, at, at),
        Some(entity)
    );
    assert_eq!(
        return_occupant(EntityKind::Creature, successor, at, at),
        Some(successor)
    );
    assert_eq!(
        return_occupant(
            EntityKind::Creature,
            entity,
            TilePos { floor: -8, ..at },
            at
        ),
        None
    );
}

fn projected_return_ground(client: &mut DevClientSession, at: TilePos) -> Option<u16> {
    client
        .session_mut()
        .world_map_snapshot()?
        .tiles
        .into_iter()
        .find(|map_tile| tile(map_tile.position).ok() == Some(at))
        .map(|map_tile| map_tile.ground_speed)
}

async fn wait_for_return_clear(
    client: &mut DevClientSession,
    at: TilePos,
    initial: Vec<oteryn_dev_client::EntityRef>,
) -> TestResult {
    let mut occupants = initial;
    let deadline = tokio::time::Instant::now() + COMMAND_BUDGET;
    while !occupants.is_empty() {
        evidence(&format!(
            "C12_RETURN_OCCUPIED tile={at:?} occupants={occupants:?}"
        ));
        loop {
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("C12 return occupants stayed on {at:?}: {occupants:?}").into());
            }
            client.service_liveness(Duration::from_millis(100)).await?;
            let latest = return_occupants(client, at)?;
            if latest != occupants {
                occupants = latest;
                break;
            }
        }
    }
    Ok(())
}

fn detour_return(
    client: &DevClientSession,
    world: &BundleWorld,
    from: TilePos,
    occupied: TilePos,
    remaining: &VecDeque<StepDirection>,
) -> Option<Vec<StepDirection>> {
    let occupied_tiles = client
        .world_entities()?
        .others()
        .filter_map(|entity| {
            matches!(
                entity.kind,
                EntityKind::Creature | EntityKind::Npc | EntityKind::Player
            )
            .then(|| tile(entity.position).ok())
            .flatten()
        })
        .collect::<std::collections::BTreeSet<_>>();
    route_around_occupant(
        from,
        from,
        occupied,
        remaining,
        |at| !occupied_tiles.contains(&at),
        |from, direction| destination(world, from, direction),
    )
}

async fn walk_authored_return(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
    allow_detour: bool,
) -> TestResult {
    let mut path = VecDeque::from(path.to_vec());
    let mut detours = 0;
    while let Some(direction) = path.pop_front() {
        let from = tile(client.world_spatial().actor_position)?;
        let at = destination(world, from, direction).ok_or("C12 return STEP has no landing")?;
        let occupants = return_occupants(client, at)?;
        if allow_detour
            && detours < 8
            && !occupants.is_empty()
            && let Some(detour) = detour_return(client, world, from, at, &path)
        {
            detours += 1;
            evidence(&format!(
                "C12_RETURN_DETOUR count={detours} tile={at:?} occupants={occupants:?} steps={}",
                detour.len()
            ));
            path = detour.into();
            continue;
        }
        wait_for_return_clear(client, at, occupants).await?;
        let mut step = tokio::time::timeout(COMMAND_BUDGET, client.step(direction)).await??;
        if step.disposition == StepDisposition::Blocked
            && tile(client.world_spatial().actor_position)? == from
        {
            let original = step.command_id;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
            let occupants = loop {
                let ground = projected_return_ground(client, at);
                let occupants = return_occupants(client, at)?;
                if !occupants.is_empty() {
                    break occupants;
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!("BLOCKER-SERVER C12 return STEP blocked with no visible occupant command={original} sequence={} from={from:?} target={at:?} projected_ground_speed={ground:?}", step.result_server_sequence).into());
                }
                client.service_liveness(Duration::from_millis(50)).await?;
            };
            evidence(&format!(
                "C12_RETURN_BLOCKED command={original} target={at:?} occupants={occupants:?} projected_ground_speed={:?}",
                projected_return_ground(client, at)
            ));
            if allow_detour
                && detours < 8
                && let Some(detour) = detour_return(client, world, from, at, &path)
            {
                detours += 1;
                evidence(&format!(
                    "C12_RETURN_DETOUR count={detours} tile={at:?} occupants={occupants:?} steps={}",
                    detour.len()
                ));
                path = detour.into();
                continue;
            }
            wait_for_return_clear(client, at, occupants).await?;
            step = tokio::time::timeout(COMMAND_BUDGET, client.step(direction)).await??;
        }
        require_mechanic_acceptance(step.status, "C12 authored return STEP")?;
        let actual = tile(client.world_spatial().actor_position)?;
        if step.disposition != StepDisposition::Moved
            || step.world_spatial_delta.is_none()
            || actual != at
        {
            return Err(format!(
                "C12 authored return STEP from={from:?} target={at:?} actual={actual:?}: {step:?}"
            )
            .into());
        }
        verdict::Harness::global().progress("walk");
        client
            .session_mut()
            .service_liveness_discarding_events(Duration::from_millis(u64::from(step.step_wait_ms)))
            .await?;
    }
    Ok(())
}

async fn prove_authored_hunt(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    augment_bps: u64,
    allow_return_detour: bool,
) -> TestResult {
    let home = tile(client.world_spatial().actor_position)?;
    let respawn = authored_tarantula_respawn(spawns)?;
    let spawn_wait = if allow_return_detour {
        respawn + COMMAND_BUDGET
    } else {
        COMMAND_BUDGET
    };
    let authored = authored_client_cast_route(world, home)?;
    let path = authored
        .iter()
        .map(|(direction, _)| *direction)
        .collect::<Vec<_>>();
    walk_exact_route(client, world, &path).await?;
    if tile(client.world_spatial().actor_position)? != CLIENT_CAST_TILE {
        return Err("C12 did not reach the authored cast tile".into());
    }
    super::client_row_c12::cast_at_authored_spawn(client.session_mut(), augment_bps, spawn_wait)
        .await?;
    let cleared = client.session_mut().attack_target(None).await?;
    require_mechanic_acceptance(cleared.status, "C12 authored target clear")?;
    if cleared.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("C12 authored target clear refused: {cleared:?}").into());
    }
    wait_combat_target(client, None).await?;
    let rope = TilePos {
        x: 32296,
        y: 32334,
        floor: -8,
    };
    let upstairs = TilePos {
        x: 32296,
        y: 32335,
        floor: -7,
    };
    if !world
        .base()
        .tile(rope.x, rope.y, rope.floor)
        .is_some_and(|tile| tile.appearance(0) == Some(386))
    {
        return Err("C12 original Tarantula cave rope spot is missing".into());
    }
    if destination(world, CLIENT_CAST_TILE, StepDirection::East) != Some(rope) {
        return Err("C12 original Tarantula cave rope spot is unreachable".into());
    }
    walk_exact_route(client, world, &[StepDirection::East]).await?;
    let reply = client
        .chat(&oteryn_dev_client::ChatIntent::Say {
            mode: oteryn_dev_client::ChatSpeechMode::Say,
            text: "exani tera".into(),
        })
        .await?;
    require_mechanic_acceptance(reply.status, "C12 Magic Rope")?;
    if reply.disposition != oteryn_dev_client::ChatDisposition::Ok {
        return Err(format!("C12 Magic Rope failed: {reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + COMMAND_BUDGET;
    while tile(client.world_spatial().actor_position)? == rope {
        if tokio::time::Instant::now() >= deadline {
            return Err("C12 Magic Rope did not leave the authored cave".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    let landing = tile(client.world_spatial().actor_position)?;
    if landing != upstairs {
        return Err(format!("C12 Magic Rope landed at {landing:?}, expected {upstairs:?}").into());
    }
    evidence("C12_AUTHORED_RETURN rope=32296,32334,-8 landing=32296,32335,-7");
    let back = route(world, upstairs, |at| at == home)
        .ok_or("C12 no surface return route from the authored rope exit")?;
    walk_authored_return(client, world, &back, allow_return_detour).await?;
    Ok(())
}

pub(super) async fn prepare_client_capture(
    clients: &SeamClients<'_>,
) -> TestResult<ClientCaptureSetup> {
    character_prep::declare_seeded(
        "C12.client",
        "level-80-monk,premium,purchase-gold,adventurers-stone,greater-proficiency-catalyst,tarantula",
        "capture Greater Tiger Clash with the augment equipped",
    );
    let dedicated =
        character_prep::dedicated_character(clients, "NODE_BOOT_C12_MONK_ACCOUNT").await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
    let dedicated_clients = &SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    if !requires_platform_grant(&dedicated_clients.accounts[1]) {
        return Err(super::full_row_c02::PLATFORM_ADMISSION_GAP.into());
    }
    let mut client = prepare_c12_monk(dedicated_clients).await?;
    character_prep::grant_premium(&client).await?;
    eprintln!("FIXTURE C12 Premium grant applied once for the dedicated Monk");
    character_prep::grant_item(&client, ADVENTURERS_STONE, 1).await?;
    let coins = pema_planned_price()?.div_ceil(CRYSTAL_COIN_VALUE);
    character_prep::grant_item(&client, CRYSTAL_COIN, u32::try_from(coins)?).await?;
    character_prep::grant_item(&client, GREATER_PROFICIENCY_CATALYST, 1).await?;
    eprintln!("FIXTURE C12.client Greater Proficiency Catalyst 51589 granted once");
    client = fresh_c12_admission(client, dedicated_clients).await?;
    verify_seeded_monk(&mut client).await?;
    let definition = item_ref()?;
    if carried_count(&client, definition) != 0 || equipped_right(&client, definition) {
        return Err("C12.client purchase requires the Fists to be absent".into());
    }
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let portals = guild_portals(&read_json("content/quests/world-callbacks.json")?)?;
    enter_guild(&mut client, world, dedicated_clients).await?;
    let purchase_balance =
        acquire_pema(&mut client, world, &metadata.npcs, definition, portals).await?;
    leave_guild(&mut client, world, &metadata.npcs, dedicated_clients).await?;
    select_fists_damage_perks(&mut client, dedicated_clients, definition).await?;
    equip_right(&mut client, definition).await?;

    prove_authored_hunt(&mut client, world, &metadata.spawns, AUGMENT_BPS, false).await?;
    let previous = client.join_snapshot().game_session_id;
    let position = tile(client.world_spatial().actor_position)?;
    let route = client_cast_route(world, position)?;
    evidence(&format!(
        "C12_CLIENT_HUNT fixture={FIXTURE_TARGET} stand={:?} steps={}",
        route.last().map(|(_, at)| at),
        route.len()
    ));
    logout(&mut client, false).await?;
    Ok(ClientCaptureSetup {
        accounts,
        characters,
        definition,
        route,
        previous,
        position,
        purchase_balance,
    })
}

fn use_holder_wait(log: &str, purpose_prefix: &str) -> bool {
    log.lines().any(|line| {
        line.split_ascii_whitespace()
            .any(|field| field == "event=use_holder_wait")
            && line.split_ascii_whitespace().any(|field| {
                field
                    .strip_prefix("purpose=")
                    .is_some_and(|purpose| purpose.starts_with(purpose_prefix))
            })
    })
}

fn require_no_use_holder_wait(path: &Path, offset: u64, purpose_prefix: &str) -> TestResult {
    use std::io::{Read, Seek};
    let mut log = std::fs::File::open(path)?;
    log.seek(std::io::SeekFrom::Start(offset))?;
    let mut text = String::new();
    log.read_to_string(&mut text)?;
    if use_holder_wait(&text, purpose_prefix) {
        return Err(format!("C12 {purpose_prefix} USE encountered Busy holder checkout").into());
    }
    Ok(())
}

#[test]
fn c12_rejects_success_after_holder_contention() {
    let log = "event=use_holder_wait purpose=stone_travel waited_ms=12 outcome=retry_finished";
    assert!(use_holder_wait(log, "stone_"));
    assert!(!use_holder_wait(log, "catalyst_"));
    let log = "event=use_holder_wait purpose=catalyst_commit waited_ms=7 outcome=retry_finished";
    assert!(use_holder_wait(log, "catalyst_"));
    assert!(!use_holder_wait(
        "event=use_complete purpose=catalyst_commit",
        "catalyst_"
    ));
}

fn global_harmony_numerator(charges: u32) -> Option<u64> {
    // Global level 80, unbuffed. The level term adds 0.4 points to the 7% base.
    [1_000, 1_074, 1_148, 1_296, 1_592, 2_184]
        .get(charges as usize)
        .copied()
}

#[test]
fn c12_global_five_harmony_scale_matches_server_rule() -> TestResult {
    use oteryn_gameplay_spell_rules::harmony::{HarmonyMultiplier, MULTIPLIER_DENOMINATOR};
    for charges in 0..=5 {
        let rule = HarmonyMultiplier::new(80, charges, false, false).ok_or("missing multiplier")?;
        let expected =
            global_harmony_numerator(u32::from(charges)).ok_or("missing Global multiplier")?;
        assert_eq!(rule.numerator() * 1_000, expected * MULTIPLIER_DENOMINATOR);
        if charges == 5 {
            assert_eq!(rule.numerator() * 125, 273 * MULTIPLIER_DENOMINATOR);
        }
    }
    Ok(())
}

pub(super) fn verify_harmony_evidence(
    path: &Path,
    offset: u64,
    session: &str,
    command: u64,
    target: &oteryn_dev_client::EntityRef,
    charges: u32,
    damage: &CommittedDamage,
) -> TestResult {
    use std::io::{BufRead, BufReader, Read, Seek};
    let mut file = std::fs::File::open(path)?;
    file.seek(std::io::SeekFrom::Start(offset))?;
    let marker = format!(
        "session={session} command={command} target=actor:{} generation={} ",
        hex_bytes(&target.identity),
        target.generation,
    );
    let mut seen = false;
    for line in BufReader::new(file.take(512 * 1024)).lines() {
        let line = line?;
        if !line.starts_with("SEAM_EVIDENCE c12_harmony_damage ") {
            continue;
        }
        let Some((_, rest)) = line.split_once(&marker) else {
            continue;
        };
        if seen {
            return Err("duplicate Harmony damage evidence for cast and target".into());
        }
        seen = true;
        let fields = rest
            .split_ascii_whitespace()
            .filter_map(|field| field.split_once('='))
            .collect::<std::collections::BTreeMap<_, _>>();
        let number = |key| -> TestResult<u64> {
            Ok(fields
                .get(key)
                .ok_or_else(|| format!("missing Harmony evidence {key}"))?
                .parse()?)
        };
        let pre = number("pre_harmony_roll")?;
        let low = number("pre_harmony_low")?;
        let high = number("pre_harmony_high")?;
        let decision = number("decision_draw")?;
        let rolled = number("rolled_damage")?;
        let expected = global_harmony_numerator(charges).ok_or("invalid Harmony charges")?;
        let denominator = number("multiplier_denominator")?;
        if number("harmony_charges")? != u64::from(charges)
            || denominator == 0
            || u128::from(number("multiplier_numerator")?) * 1_000
                != u128::from(expected) * u128::from(denominator)
            || number("committed_damage")? != damage.committed
            || rolled != damage.before
            || !exact_harmony_draw(low, high, decision, expected, pre, rolled)
        {
            return Err(
                format!("C12 exact Harmony damage mismatch: {line}; committed={damage:?}").into(),
            );
        }
    }
    if !seen {
        return Err(format!("C12 missing Harmony evidence: {marker}").into());
    }
    Ok(())
}

fn exact_harmony_draw(
    low: u64,
    high: u64,
    decision: u64,
    expected: u64,
    pre: u64,
    rolled: u64,
) -> bool {
    let draw =
        |min: u128, max: u128| (max >= min).then(|| min + u128::from(decision) % (max - min + 1));
    let low = u128::from(low);
    let high = u128::from(high);
    let scale = |bound| bound * u128::from(expected) / 1_000;
    draw(low, high) == Some(u128::from(pre))
        && draw(scale(low), scale(high)) == Some(u128::from(rolled))
}

#[test]
fn c12_harmony_rejects_an_overlapping_wrong_roll() {
    assert!(exact_harmony_draw(23, 29, 3, 2184, 26, 53));
    assert!(!exact_harmony_draw(23, 29, 3, 2184, 26, 51));
    assert!(!exact_harmony_draw(23, 29, 3, 2120, 26, 53));
}

async fn unequip_fists(client: &mut DevClientSession, definition: NonZeroU32) -> TestResult {
    let equipped = client
        .inventory()
        .and_then(|i| {
            i.equipment.iter().find(|e| {
                e.slot == items::EquipmentSlot::RightHand
                    && e.item.item_definition_ref == definition
            })
        })
        .map(|e| e.item.handle)
        .ok_or("right-hand item absent before unequip")?;
    let moved = client
        .move_item(&items::ItemMoveIntent {
            source: equipped,
            destination: items::ItemMoveDestination::MainBackpack,
        })
        .await?;
    if moved.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("unequip ITEM_MOVE reply={moved:?}").into());
    }
    wait_inventory(client, "unequip", |c| {
        !equipped_right(c, definition) && carried_count(c, definition) == 1
    })
    .await?;
    Ok(())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let portals = guild_portals(&read_json("content/quests/world-callbacks.json")?)?;
    let definition = item_ref()?;
    let dedicated =
        character_prep::dedicated_character(clients, "NODE_BOOT_C12_MONK_ACCOUNT").await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
    let clients = &SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    if !requires_platform_grant(&clients.accounts[1]) {
        return Err(super::full_row_c02::PLATFORM_ADMISSION_GAP.into());
    }
    let mut client = prepare_c12_monk(clients).await?;
    verify_seeded_monk(&mut client).await?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    if carried_count(&client, definition) != 0 || equipped_right(&client, definition) {
        return Err(
            "item 50271 must first enter the dedicated Monk's backpack through the purchase".into(),
        );
    }
    character_prep::grant_premium(&client).await?;
    eprintln!("FIXTURE C12 Premium grant applied once for the dedicated Monk");
    character_prep::grant_item(&client, MANA_POTION, 12).await?;
    character_prep::grant_item(&client, ADVENTURERS_STONE, 1).await?;
    eprintln!("FIXTURE C12 Adventurer's Stone 16277 granted once");
    character_prep::grant_item(&client, GREATER_PROFICIENCY_CATALYST, 1).await?;
    eprintln!("FIXTURE C12 Greater Proficiency Catalyst 51589 granted once");
    let coins = pema_planned_price()?.div_ceil(CRYSTAL_COIN_VALUE);
    character_prep::grant_item(&client, CRYSTAL_COIN, u32::try_from(coins)?).await?;
    client = fresh_c12_admission(client, clients).await?;
    verify_seeded_monk(&mut client).await?;
    let use_log = std::path::PathBuf::from(required("NODE_BOOT_CAST_EVIDENCE_LOG")?);
    let offset = std::fs::metadata(&use_log)?.len();
    enter_guild(&mut client, world, clients).await?;
    require_no_use_holder_wait(&use_log, offset, "stone_")?;
    let purchase_balance =
        acquire_pema(&mut client, world, &metadata.npcs, definition, portals).await?;
    let acquired = carried_count(&client, definition);
    if acquired != 1 {
        return Err(format!(
            "expected one purchased item {ITEM_ID} in backpack; observed={acquired}"
        )
        .into());
    }
    leave_guild(&mut client, world, &metadata.npcs, clients).await?;
    let offset = std::fs::metadata(&use_log)?.len();
    select_fists_damage_perks(&mut client, clients, definition).await?;
    require_no_use_holder_wait(&use_log, offset, "catalyst_")?;
    let baseline_weapon = weapon(&client)?;
    if baseline_weapon == Some(definition) {
        return Err("need=augment-off-baseline;item-50271-already-equipped".into());
    }
    equip_right(&mut client, definition).await?;
    prove_authored_hunt(&mut client, world, &metadata.spawns, AUGMENT_BPS, true).await?;
    unequip_fists(&mut client, definition).await?;
    if let Some(baseline_weapon) = baseline_weapon {
        equip_right(&mut client, baseline_weapon).await?;
    }
    row.pass("acquire-50271", (acquired, purchase_balance));

    require_current_premium(client.session_mut()).await?;
    let spell_index = spell(&mut client)?;
    let augment = content_augment()?;
    let hunt = prepare_fixture_hunt(&mut client, world).await?;

    let baseline =
        verdict::indexed_timeout(BASELINE_BUDGET, samples(&hunt, &mut client, spell_index, 0))
            .await
            .map_err(|_| {
                format!(
                    "baseline sampling deadline=75s last-frame={}",
                    client.last_server_sequence()
                )
            })??;
    row.record_cast("baseline-cast", &baseline, baseline_weapon);

    equip_right(&mut client, definition).await?;
    let augmented = samples(&hunt, &mut client, spell_index, augment).await?;
    row.record_cast("augmented-cast", &augmented, weapon(&client)?);

    unequip_fists(&mut client, definition).await?;
    row.pass("unequip", client.last_server_sequence());

    if let Some(baseline_weapon) = baseline_weapon {
        equip_right(&mut client, baseline_weapon).await?;
    }
    let carried = samples(&hunt, &mut client, spell_index, 0).await?;
    row.record_cast("unequipped-cast", &carried, weapon(&client)?);

    equip_right(&mut client, definition).await?;
    let old_session = client.join_snapshot().game_session_id;
    let pre_logout_mana = logout(&mut client, true).await?;
    drop(client);
    let durable_mana = saved_mana(clients).await?;
    let mut fresh = connect_c12_character(clients).await?;
    require_fists_damage_perks(clients).await?;
    let fresh_vitals = fresh.actor_vitals().ok_or("C12 post-relog vitals absent")?;
    if !mana_survived(
        pre_logout_mana,
        durable_mana,
        fresh_vitals.mana,
        fresh_vitals.max_mana,
    ) {
        return Err(format!(
            "C12 relog did not retain observed mana: before_logout={pre_logout_mana} saved={durable_mana} observed={}",
            fresh_vitals.mana
        )
        .into());
    }
    evidence(&format!(
        "C12_FULL_RELOG before_logout={pre_logout_mana} durable_mana={durable_mana} fresh_session=true"
    ));
    if fresh.join_snapshot().game_session_id == old_session
        || !equipped_right(&fresh, definition)
        || carried_count(&fresh, definition) != 0
        || full_e2ef7::units(&fresh, definition) != 1
    {
        return Err("fresh admission lost the right-hand item".into());
    }
    let fresh_balance = full_e2ef7::player_gold(&mut fresh).await?;
    if fresh_balance != purchase_balance {
        return Err(format!(
            "Pema purchase balance changed after relog: expected {purchase_balance}, observed {fresh_balance}"
        ).into());
    }
    require_current_premium(fresh.session_mut()).await?;
    let fresh_spell_index = spell(&mut fresh)?;
    let replay = samples(&hunt, &mut fresh, fresh_spell_index, augment).await?;
    row.record_cast("fresh-admission", &replay, weapon(&fresh)?);
    let catalyst =
        provisioning::item_reference(&provisioning::item_index()?, GREATER_PROFICIENCY_CATALYST)?;
    let position = fresh.world_spatial().actor_position;
    let target = hunt.entity;
    let mut fresh = super::client_row_c12::verify_individual_perks(
        fresh, clients, world, definition, catalyst, target, position,
    )
    .await?;
    logout(&mut fresh, false).await?;
    row.pass("augment-attribution", (ITEM_ID, augment));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c12_global_route_rejects_mainland_and_wrong_guild_tile() -> TestResult {
        let mut callbacks = serde_json::json!({"callbacks": [
            {"key": "oteryn:quest-callback/batch3/the_way_of_the_monk/blue-valley-5", "trigger": "use",
             "selector": {"position": {"x": 32203, "y": 32303, "floor": -6}},
             "teleport": {"x": 33614, "y": 31494, "floor": -7}},
            {"key": "oteryn:quest-callback/batch3/the_way_of_the_monk/blue-valley-2", "trigger": "use",
             "selector": {"position": {"x": 33605, "y": 31468, "floor": -7}},
             "teleport": {"x": 32210, "y": 32300, "floor": -6}}
        ]});
        assert!(guild_portals(&callbacks).is_ok());
        callbacks["callbacks"][1]["teleport"] =
            serde_json::json!({"x": 32211, "y": 32300, "floor": -6});
        assert!(guild_portals(&callbacks).is_err());
        callbacks["callbacks"][1]["teleport"] =
            serde_json::json!({"x": 32456, "y": 32291, "floor": -7});
        let Err(error) = guild_portals(&callbacks) else {
            return Err("the mainland return cannot qualify".into());
        };
        assert!(error.to_string().starts_with("BLOCKED-CONTENT:"));
        Ok(())
    }

    #[test]
    fn c12_one_perk_rejects_zero_and_combined_bonus() -> TestResult {
        let marker =
            "SEAM_EVIDENCE c12_base_damage session=test command=12 target=rat generation=2 ";
        let sample = |damage, bps| {
            format!(
                "{marker}before=50 after={damage} critical=false element=physical committed={damage} committed_hits=1 augment_bps={bps}"
            )
        };
        assert_eq!(
            read_base_damage(sample(52, 400).as_bytes(), marker, 400)?.committed,
            52
        );
        assert!(read_base_damage(sample(50, 0).as_bytes(), marker, 400).is_err());
        assert!(read_base_damage(sample(54, 800).as_bytes(), marker, 400).is_err());
        assert!(read_base_damage(sample(50, 400).as_bytes(), marker, 400).is_err());
        Ok(())
    }

    #[test]
    fn c12_rejects_additional_committed_health_loss() {
        let marker =
            "SEAM_EVIDENCE c12_base_damage session=test command=12 target=rat generation=2 ";
        let line = format!(
            "{marker}before=28 after=30 critical=false element=physical committed=30 committed_hits=1 augment_bps=800"
        );
        assert!(read_base_damage(line.as_bytes(), marker, AUGMENT_BPS).is_ok());
        let second_target = line.replace("committed_hits=1", "committed_hits=2");
        assert!(read_base_damage(second_target.as_bytes(), marker, AUGMENT_BPS).is_err());
        let duplicate = format!("{line}\n{line}");
        assert!(read_base_damage(duplicate.as_bytes(), marker, AUGMENT_BPS).is_err());
    }

    #[test]
    fn c12_unequipped_control_rejects_an_unconditional_augment() -> TestResult {
        let marker =
            "SEAM_EVIDENCE c12_base_damage session=test command=11 target=rat generation=2 ";
        let unchanged = format!(
            "{marker}before=28 after=28 critical=false element=physical committed=28 committed_hits=1 augment_bps=0"
        );
        let augmented = format!(
            "{marker}before=28 after=30 critical=false element=physical committed=30 committed_hits=1 augment_bps=800"
        );
        assert_eq!(
            read_base_damage(unchanged.as_bytes(), marker, 0)?.committed,
            28
        );
        assert_eq!(
            read_base_damage(augmented.as_bytes(), marker, AUGMENT_BPS)?.committed,
            30
        );
        assert!(read_base_damage(augmented.as_bytes(), marker, 0).is_err());
        // A low Swift Jab roll can round a leaked augment down to zero damage.
        let leaked = format!(
            "{marker}before=6 after=6 critical=false element=physical committed=6 committed_hits=1 augment_bps=800"
        );
        assert!(read_base_damage(leaked.as_bytes(), marker, 0).is_err());
        Ok(())
    }

    #[test]
    fn committed_pair_requires_exact_unchanged_input_and_expected_delta() -> TestResult {
        const MARKER: &str =
            "SEAM_EVIDENCE c12_base_damage session=test command=1 target=rat generation=2 ";
        let read = |tail: &str, bps| {
            let log = format!("{MARKER}{tail} augment_bps={bps}");
            read_base_damage(log.as_bytes(), MARKER, bps)
        };
        assert_eq!(
            read(
                "before=37 after=40 critical=true element=physical committed=44 committed_hits=1",
                1_000
            )?,
            CommittedDamage {
                before: 37,
                augmented: 40,
                critical: true,
                committed: 44
            },
        );
        assert_eq!(
            read(
                "before=37 after=37 critical=false element=physical committed=37 committed_hits=1",
                0
            )?,
            CommittedDamage {
                before: 37,
                augmented: 37,
                critical: false,
                committed: 37
            },
        );
        assert!(read("before=37 after=40", 1_000).is_err());
        assert!(
            read(
                "before=37 after=40 critical=false element=physical committed=0 committed_hits=1",
                1_000
            )
            .is_err()
        );
        assert!(
            read(
                "before=37 after=41 critical=false element=physical committed=41 committed_hits=1",
                1_000
            )
            .is_err()
        );
        assert!(
            read(
                "before=37 after=37 critical=false element=physical committed=37 committed_hits=1",
                1_000
            )
            .is_err()
        );
        assert!(
            read(
                "before=37 after=40 critical=false element=physical committed=40 committed_hits=1",
                0
            )
            .is_err()
        );
        assert!(
            read(
                "before=37 after=40 critical=false element=physical committed=37 committed_hits=1",
                1_000
            )
            .is_err()
        );
        assert!(
            read(
                "before=37 after=40 critical=true element=physical committed=40 committed_hits=1",
                1_000
            )
            .is_err()
        );
        assert!(
            read(
                "before=37 after=40 critical=false element=fire committed=40 committed_hits=1",
                1_000
            )
            .is_err()
        );
        let duplicate = format!(
            "{MARKER}before=37 after=40 critical=false element=physical committed=40 committed_hits=1 augment_bps=1000
{MARKER}before=37 after=40 critical=false element=physical committed=40 committed_hits=1 augment_bps=1000"
        );
        assert!(read_base_damage(duplicate.as_bytes(), MARKER, 1_000).is_err());
        Ok(())
    }

    #[test]
    fn alternate_fixture_tiles_are_rejected() {
        use oteryn_dev_client::native_session::ActorPosition;
        let caster = ActorPosition {
            x: 50,
            y: 50,
            floor: 7,
        };
        assert!(is_fixture_north(caster, ActorPosition { y: 49, ..caster }));
        assert!(!is_fixture_north(caster, ActorPosition { x: 51, ..caster }));
        assert!(!is_fixture_north(
            caster,
            ActorPosition {
                x: 51,
                y: 49,
                ..caster
            }
        ));
        assert!(!is_fixture_north(
            caster,
            ActorPosition {
                y: 49,
                floor: 6,
                ..caster
            }
        ));
        assert!(!is_fixture_north(caster, caster));
    }

    #[test]
    fn presentation_must_name_the_exact_spell_and_attacker() {
        use oteryn_protocol_oteryn::presentation::{
            PresentationColor, PresentationEvent, PresentationValue,
        };
        let own = oteryn_dev_client::EntityRef {
            identity: [1; 16],
            generation: 1,
        };
        let victim = oteryn_dev_client::EntityRef {
            identity: [2; 16],
            generation: 1,
        };
        let position = oteryn_dev_client::native_session::ActorPosition {
            x: 1,
            y: 1,
            floor: 7,
        };
        let words = |spell| PresentationEvent {
            decision_ordinal: 4,
            emission_ordinal: 1,
            kind: PresentationEventKind::SpellWords {
                caster: own,
                position,
                spell,
            },
        };
        let damage = |attacker, color, decision_ordinal| PresentationEvent {
            decision_ordinal,
            emission_ordinal: 2,
            kind: PresentationEventKind::ValueText {
                kind: PresentationValueKind::DamageDealt,
                position,
                target: victim,
                attacker,
                primary: PresentationValue { value: 40, color },
                secondary: None,
            },
        };
        let red = PresentationColor::Red;
        let spell = 292;
        let check = |events: [PresentationEvent; 2]| {
            cast_damage_from_presentation(&events, own, victim, spell)
        };
        assert_eq!(check([words(spell), damage(Some(own), red, 4)]), 40);
        assert_eq!(check([words(1), damage(Some(own), red, 4)]), 0);
        assert_eq!(check([words(spell), damage(Some(victim), red, 4)]), 0);
        assert_eq!(check([words(spell), damage(None, red, 4)]), 0);
        assert_eq!(
            check([
                words(spell),
                damage(Some(own), PresentationColor::Orange, 4)
            ]),
            0
        );
        assert_eq!(check([words(spell), damage(Some(own), red, 5)]), 0);
    }

    #[test]
    fn harmony_noop_cannot_pass_a_spender() {
        assert!(harmony_spent(1, 0));
        assert!(!harmony_spent(0, 0));
        assert!(!harmony_spent(1, 1));
    }

    #[test]
    fn stale_below_max_mana_cannot_pass_relog() {
        assert!(mana_survived(381, 381, 381, 810));
        assert!(!mana_survived(381, 382, 382, 810));
        assert!(!mana_survived(381, 381, 382, 810));
        assert!(!mana_survived(810, 810, 810, 810));
    }

    #[test]
    fn cast_observation_does_not_complete_augment_attribution() {
        let mut row = Row::new();
        for step in &STEPS[..6] {
            row.record_cast(step, &[1], None);
        }
        assert_eq!(row.current(), "augment-attribution");
    }

    #[test]
    fn c12_gap_names_the_stalled_command_and_unrun_steps() {
        let line = gap("baseline-cast", "CAST deadline", &STEPS[2..]);
        assert!(line.starts_with("C12_GAP step=baseline-cast observed=CAST deadline"));
        assert!(line.contains(
            "not-run=augmented-cast,unequip,unequipped-cast,fresh-admission,augment-attribution"
        ));
    }

    #[test]
    fn pass_requires_every_step_in_order() {
        let mut row = Row::new();
        row.pass("baseline-cast", "out of order");
        assert!(row.passed.is_empty());
        for step in STEPS {
            row.pass(step, "ok");
        }
        assert_eq!(row.passed.len(), STEPS.len());
    }

    #[test]
    fn content_binds_the_augment_to_the_item() -> TestResult {
        assert_eq!(content_augment()?, AUGMENT_BPS);
        assert_ne!(item_ref()?.get(), 0);
        Ok(())
    }

    #[test]
    fn the_declared_trade_sells_the_item() -> TestResult {
        assert_eq!(pema_planned_price()?, PEMA_PRICE);
        Ok(())
    }
}
