//! Opt-in player spell census and wire checks, using the node's fenced fixture.
//!
//! Select with `NODE_BOOT_E2E_ONLY=spell_sweep`. The authored spell keys
//! (instant and rune carriers) determine the batch order, independently of
//! the native spell book's active-index selection. Missing indexes get FAIL rows.
//!
//! Fixture preparation uses the fenced progression and inventory owners in
//! `apps/game-server/src/node/serve/qualification_fixture.rs:632`.
//! Casts and immediate retries go through the ordinary actor-spell or item-use
//! wire commands. Rune refusals follow the bridge at
//! `crates/gameplay/dispatch/src/gameplay_transport/rune_use_bridge.rs:257`.
//! The exact mana and soul oracle uses authored costs, with percent-of-max
//! rounding from `crates/gameplay/spell-rules/src/cast.rs:272`.
use super::*;
use crate::spell::{Carrier, Execution, ManaCost, SpellDefinition, SpellEffect, Vocation};
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{
    actor_spell::{ActorVitals, SpellCastDisposition, SpellTarget, SpellTargetPosition},
    logout::LogoutResult,
    world_object::UseDisposition,
};
use std::num::NonZeroU32;

const FIXTURE_LEVEL: u32 = 300;
const BATCH_VARIABLE: &str = "OTERYN_E2E_SPELL_SWEEP_BATCH";
const STEPS: [&str; 6] = [
    "prepare",
    "cast_cost",
    "effect",
    "ineligible",
    "cooldown",
    "release",
];

fn batch(value: Option<&str>) -> TestResult<(usize, usize)> {
    let Some(value) = value else {
        return Ok((0, 3));
    };
    let decimal = |part: &str| {
        part.bytes()
            .all(|byte| byte.is_ascii_digit())
            .then(|| part.parse::<usize>().ok())
            .flatten()
    };
    match value
        .split_once(':')
        .and_then(|(start, len)| Some((decimal(start)?, decimal(len)?)))
    {
        Some((start, len)) if len > 0 => Ok((start, len)),
        _ => Err(
            format!("invalid {BATCH_VARIABLE}={value:?}: expected start:len with len > 0").into(),
        ),
    }
}

fn keys(root: &Path) -> TestResult<Vec<String>> {
    let source: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/abilities/definitions/player-spells.json"),
    )?)?;
    let mut keys = source["bundles"]
        .as_array()
        .ok_or("spell bundles absent")?
        .iter()
        .map(|entry| {
            entry
                .pointer("/bundle/spell/identity/key")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .ok_or("spell identity absent")
        })
        .collect::<Result<Vec<_>, _>>()?;
    keys.sort_unstable();
    if keys.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("duplicate authored spell identity".into());
    }
    Ok(keys)
}

#[derive(Default)]
struct Tally {
    pass: usize,
    fail: usize,
}

struct Rows<'a> {
    key: &'a str,
    tally: &'a mut Tally,
    pending: Vec<&'static str>,
    lap: std::time::Instant,
}

impl Rows<'_> {
    fn record(&mut self, step: &str, result: TestResult) {
        if !self.pending.contains(&step) {
            return;
        }
        self.pending.retain(|pending| *pending != step);
        let (status, detail) = match result {
            Ok(()) => {
                self.tally.pass += 1;
                ("PASS", "observed".to_owned())
            }
            Err(error) => {
                self.tally.fail += 1;
                (
                    "FAIL",
                    error
                        .to_string()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join("_"),
                )
            }
        };
        evidence(&format!(
            "e2e_all scenario=SS.{}.{step} status={status} seconds={:.3} mode=seeded evidence={detail}",
            self.key.replace('/', "."),
            self.lap.elapsed().as_secs_f64()
        ));
        self.lap = std::time::Instant::now();
    }

    fn blocked(&mut self, step: &str) {
        for pending in self.pending.clone() {
            self.record(pending, Err(format!("blocked-by-{step}").into()));
        }
    }
}

pub(super) async fn run(clients: &[SeamClients<'_>; 3], world: &BundleWorld) -> TestResult {
    let value = match std::env::var(BATCH_VARIABLE) {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => return Err(error.into()),
    };
    let (start, len) = batch(value.as_deref())?;
    let keys = keys(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))?;
    if start >= keys.len() {
        return Err(format!(
            "{BATCH_VARIABLE} start {start} outside {} entries",
            keys.len()
        )
        .into());
    }
    let end = start.saturating_add(len).min(keys.len());
    evidence(&format!(
        "spell_sweep census total={} start={start} selected={} mode=seeded",
        keys.len(),
        end - start
    ));
    let book = native_spell_book()?;
    let indexed: BTreeMap<_, _> = (1..=u32::try_from(book.source_len())?)
        .filter_map(NonZeroU32::new)
        .filter_map(|index| {
            book.indexed(index)
                .map(|spell| (spell.key.as_str(), (index, spell)))
        })
        .collect();
    let mut tally = Tally::default();
    for key in &keys[start..end] {
        verdict::Harness::global().progress("spell-sweep-entry");
        let mut rows = Rows {
            key: key.strip_prefix("candidate:spell/").unwrap_or(key),
            tally: &mut tally,
            pending: STEPS.to_vec(),
            lap: std::time::Instant::now(),
        };
        match indexed.get(key.as_str()) {
            Some(&(index, spell)) => sweep(clients, world, index, spell, &mut rows).await,
            None => {
                rows.record(
                    "prepare",
                    Err("authored entry missing from native spell book".into()),
                );
                rows.blocked("prepare");
            }
        }
    }
    evidence(&format!(
        "spell_sweep summary total={} selected={} rows_pass={} rows_fail={} mode=seeded",
        keys.len(),
        end - start,
        tally.pass,
        tally.fail
    ));
    require(
        tally.fail == 0,
        format!("spell sweep: {} FAIL rows", tally.fail),
    )
}

fn require(condition: bool, reason: impl Into<String>) -> TestResult {
    if condition {
        Ok(())
    } else {
        Err(reason.into().into())
    }
}

fn profile(spell: &SpellDefinition) -> TestResult<(usize, &'static str)> {
    ["sorcerer", "druid", "knight", "paladin", "monk"]
        .into_iter()
        .enumerate()
        .find(|(_, profile)| {
            Vocation::from_key(profile).is_some_and(|vocation| {
                spell.vocations.is_empty() || spell.vocations.contains(&vocation)
            })
        })
        .ok_or_else(|| "qualification fixture has no eligible base vocation".into())
}

async fn sweep(
    clients: &[SeamClients<'_>; 3],
    world: &BundleWorld,
    index: NonZeroU32,
    spell: &SpellDefinition,
    rows: &mut Rows<'_>,
) {
    rows.record(
        "ineligible",
        ineligible(&clients[2], world, index, spell).await,
    );
    let result = prepare(clients, world, index, spell).await;
    let (mut client, rune) = match result {
        Ok(value) => {
            rows.record("prepare", Ok(()));
            value
        }
        Err(error) => {
            rows.record("prepare", Err(error));
            rows.blocked("prepare");
            return;
        }
    };
    let result = positive(&mut client, spell, index, rune, rows).await;
    if let Err(error) = result {
        rows.record("cast_cost", Err(error));
        rows.record("effect", Err("blocked-by-cast_cost".into()));
        rows.record("cooldown", Err("blocked-by-cast_cost".into()));
    }
    rows.record("release", release_fixture(&mut client).await);
}

async fn reach_ground(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let open = |at: TilePos| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    };
    if open(from) {
        return Ok(());
    }
    let occupied: std::collections::BTreeSet<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter_map(|actor| tile(actor.position).ok())
        .collect();
    let path =
        route_avoiding(world, from, open, |at| !occupied.contains(&at)).ok_or_else(|| {
            format!(
                "no unoccupied route to non-protection ground from {:?}",
                from
            )
        })?;
    walk(client, world, &path).await
}

async fn prepare(
    clients: &[SeamClients<'_>; 3],
    world: &BundleWorld,
    spell_index: NonZeroU32,
    spell: &SpellDefinition,
) -> TestResult<(DevClientSession, Option<NonZeroU32>)> {
    require(
        spell.level <= FIXTURE_LEVEL,
        format!(
            "required-level-{}-exceeds-fixture-{FIXTURE_LEVEL}",
            spell.level
        ),
    )?;
    let (slot, profile) = profile(spell)?;
    // Vocation choice is durable and one-way. Each base vocation has its own
    // Character. The sixth Character stays unclassed for the refusal probe.
    let pair = &clients[slot / 2];
    let index = slot % 2;
    let client = connect_character(pair, 0x80, index).await?;
    let mut client = provisioning::original::provision_profile(
        client,
        pair,
        index,
        FIXTURE_LEVEL,
        &format!("spell-sweep-{profile}:{spell_index}"),
    )
    .await?;
    let setup = async {
        let item = match spell.carrier {
            Carrier::Instant { .. } => match spell.execution {
                Execution::Conjure { reagent, .. } => reagent,
                _ => None,
            },
            Carrier::Rune { item, .. } => Some(item),
        };
        let reference = item
            .map(|id| provisioning::item_reference(&provisioning::item_index()?, id))
            .transpose()?;
        if let Some(reference) = reference {
            require(
                full_e2ef5::rune_quantity(&client, reference)? > 0,
                format!("seeded backpack item {reference} missing after reload"),
            )?;
        }
        let rune = reference.filter(|_| matches!(spell.carrier, Carrier::Rune { .. }));
        let pools = vitals(&client)?;
        require(
            pools.mana >= mana_cost(pools, spell.mana) && pools.soul >= spell.soul,
            format!(
                "fixture resources insufficient mana={} required_mana={} soul={} required_soul={}",
                pools.mana,
                mana_cost(pools, spell.mana),
                pools.soul,
                spell.soul
            ),
        )?;
        if spell.aggressive || spell.needs_target || pure_effect(spell) == Some(HealthEffect::Heal)
        {
            reach_ground(&mut client, world).await?;
        }
        if spell.premium {
            request_fixture_control(
                uuid_text(client.join_snapshot().game_session_id.as_bytes()),
                "--profile",
                "spell-sweep-premium".into(),
            )
            .await?;
            full_e2ef7::require_current_light_premium(&mut client).await?;
        }
        Ok::<_, Box<dyn std::error::Error>>(rune)
    }
    .await;
    match setup {
        Ok(rune) => Ok((client, rune)),
        Err(error) => match release_fixture(&mut client).await {
            Ok(()) => Err(error),
            Err(release) => Err(format!("{error}; release: {release}").into()),
        },
    }
}

async fn acquire(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spell: &SpellDefinition,
) -> TestResult<Option<NonZeroU32>> {
    match spell.carrier {
        Carrier::Instant { .. } => Ok(None),
        Carrier::Rune { item, .. } => {
            Ok(Some(full_e2ef5::acquire_rune(client, world, item).await?.0))
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Reply {
    Spell(SpellCastDisposition, bool),
    Rune(UseDisposition),
}

async fn cast(
    client: &mut DevClientSession,
    index: NonZeroU32,
    rune: Option<NonZeroU32>,
    target: Option<(oteryn_dev_client::EntityRef, ActorPosition)>,
    needs_target: bool,
) -> TestResult<Reply> {
    if let Some(reference) = rune {
        let (entity, position) = target.map_or(
            (None, client.world_spatial().actor_position),
            |(entity, position)| (Some(entity), position),
        );
        let (reply, _) = if needs_target && entity.is_some() {
            full_e2ef5::rune_use(client, reference, entity, None).await?
        } else {
            full_e2ef5::rune_use(client, reference, None, Some(position)).await?
        };
        Ok(Reply::Rune(reply))
    } else {
        let aimed = needs_target && target.is_some();
        let target = if aimed {
            SpellTarget::AttackTarget
        } else {
            target.map_or(SpellTarget::None, |(_, position)| {
                SpellTarget::Position(SpellTargetPosition {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                })
            })
        };
        let outcome = client.cast_spell(index, target, aimed).await?;
        require(
            outcome.status == CommandStatus::Accepted,
            format!(
                "spell command={} status={:?} disposition={:?} server_sequence={}",
                outcome.command_id,
                outcome.status,
                outcome.disposition,
                outcome.result_server_sequence
            ),
        )?;
        Ok(Reply::Spell(
            outcome.disposition,
            outcome.actor_vitals_delta.is_some(),
        ))
    }
}

async fn release_fixture(client: &mut DevClientSession) -> TestResult {
    let clear = async {
        if client
            .session_mut()
            .combat_state()
            .is_some_and(|state| state.target.is_some())
        {
            let reply = client.session_mut().attack_target(None).await?;
            require(
                reply.disposition == oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok,
                format!("could not clear temporary attack target: {reply:?}"),
            )?;
        }
        Ok(())
    }
    .await;
    let release = async {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
        loop {
            let (result, reply, attempts, busy_timed_out) = logout_retrying_busy(client).await?;
            match result {
                Some(LogoutResult::Accepted) => return Ok(()),
                Some(LogoutResult::InFight { retry_after_ms })
                    if tokio::time::Instant::now() < deadline =>
                {
                    if std::env::var_os("OTERYN_QUALIFICATION_CLOCK_FILE").is_some() {
                        full_e2ef7::advance_qualification_time(Duration::from_millis(
                            u64::from(retry_after_ms) + 1,
                        ))?;
                    }
                    client.service_liveness(Duration::from_millis(250)).await?;
                    client.take_events();
                }
                _ => return Err(format!("fixture logout refused: {reply}; attempts={attempts}; busy_timed_out={busy_timed_out}").into()),
            }
        }
    }.await;
    combine(clear, release)
}

fn combine(action: TestResult, release: TestResult) -> TestResult {
    match (action, release) {
        (Err(action), Err(release)) => Err(format!("{action}; release: {release}").into()),
        (action, release) => action.and(release),
    }
}

async fn select_attack_target(
    client: &mut DevClientSession,
    rune: Option<NonZeroU32>,
    target: Option<(oteryn_dev_client::EntityRef, ActorPosition)>,
    needs_target: bool,
) -> TestResult {
    if let Some((entity, _)) = target.filter(|_| rune.is_none() && needs_target) {
        let attack = client.session_mut().attack_target(Some(&entity)).await?;
        require(
            attack.status == CommandStatus::Accepted
                && attack.disposition
                    == oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok,
            format!("fixture attack target rejected: {attack:?}"),
        )?;
    }
    Ok(())
}

async fn ineligible(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    index: NonZeroU32,
    spell: &SpellDefinition,
) -> TestResult {
    let mut client = connect_character(clients, 0x81, 1).await?;
    let result = async {
        let (_, level, vocation) = creature_sweep::read_stats(&mut client).await?;
        let wrong_vocation = vocation.is_some_and(|id| {
            ["none", "sorcerer", "druid", "knight", "paladin", "monk"]
                .into_iter()
                .find(|key| oteryn_protocol_oteryn::cyclopedia_character::vocation_id(key) == Some(id))
                .and_then(Vocation::from_key)
                .is_some_and(|actual| !spell.vocations.is_empty() && !spell.vocations.contains(&actual))
        });
        require(
            level < spell.level || wrong_vocation,
            format!("negative fixture mismatch unproven: level={level}, required={}, vocation_id={vocation:?}, allowed={:?}", spell.level, spell.vocations),
        )?;
        let rune = acquire(&mut client, world, spell).await?;
        if spell.aggressive || spell.needs_target {
            reach_ground(&mut client, world).await?;
        }
        let target = if spell.needs_target && !spell.self_target {
            Some(spawn_target(&mut client, "deer").await?)
        } else {
            None
        };
        select_attack_target(&mut client, rune, target, spell.needs_target).await?;
        let before = vitals(&client)?;
        let reply = cast(&mut client, index, rune, target, spell.needs_target).await?;
        let denied = eligibility_refusal(&reply, level < spell.level);
        require(
            denied && pools_unchanged(before, vitals(&client)?),
            format!("expected eligibility refusal without cost, got {reply:?}"),
        )
    }
    .await;
    let release = release_fixture(&mut client).await;
    combine(result, release)
}

// Level precedes vocation and magic level in spell/mod.rs:514, and the
// rune bridge projects the eligibility gate as RequirementNotMet.
fn eligibility_refusal(reply: &Reply, underlevel: bool) -> bool {
    match reply {
        Reply::Spell(SpellCastDisposition::LevelTooLow, false) => underlevel,
        Reply::Spell(SpellCastDisposition::NotAvailable, false) => !underlevel,
        Reply::Rune(UseDisposition::RequirementNotMet) => underlevel,
        _ => false,
    }
}

async fn positive(
    client: &mut DevClientSession,
    spell: &SpellDefinition,
    index: NonZeroU32,
    rune: Option<NonZeroU32>,
    rows: &mut Rows<'_>,
) -> TestResult {
    let health_effect = pure_effect(spell);
    let dispel = match &spell.execution {
        Execution::Effects(effects) => effects.iter().any(|effect| {
            matches!(effect, SpellEffect::RemoveCondition { condition, .. } if condition == "paralyze")
        }),
        _ => false,
    };
    if dispel {
        spawn_target(client, "rot_elemental").await?;
    }
    let target = match health_effect {
        Some(HealthEffect::Damage) => Some(spawn_target(client, "deer").await?),
        Some(HealthEffect::Heal) if spell.self_target || !spell.needs_target => {
            if vitals(client)?.health == vitals(client)?.max_health {
                if !dispel {
                    spawn_target(client, "rat").await?;
                }
                let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
                while vitals(client)?.health == vitals(client)?.max_health {
                    require(
                        tokio::time::Instant::now() < deadline,
                        "fixture creature did not wound caster",
                    )?;
                    client.service_liveness(Duration::from_millis(50)).await?;
                    client.take_events();
                }
            }
            None
        }
        _ if spell.needs_target && !spell.self_target => Some(spawn_target(client, "deer").await?),
        _ => None,
    };
    select_attack_target(client, rune, target, spell.needs_target).await?;
    let dispel_before = if dispel {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        loop {
            let status = condition_status(client).await?;
            if status
                .conditions
                .iter()
                .any(|condition| condition.icon == 14)
            {
                break true;
            }
            if tokio::time::Instant::now() >= deadline {
                break false;
            }
            client.service_liveness(Duration::from_millis(250)).await?;
        }
    } else {
        false
    };
    let condition = condition_icon(spell);
    let conditions_before = if condition.is_some() {
        Some(condition_status(client).await?)
    } else {
        None
    };
    let familiar = match &spell.execution {
        Execution::NativeProfile(profile) if profile.familiar_creature_definition().is_some() => {
            profile
                .spell()
                .pointer("/execution/native_behavior/parameters/creature_name")
                .and_then(serde_json::Value::as_str)
        }
        _ => None,
    };
    let familiar_before: std::collections::BTreeSet<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .map(|actor| actor.entity)
        .collect();
    let before = vitals(client)?;
    let product = match &spell.execution {
        Execution::Conjure { result, count, .. } => Some((
            provisioning::item_reference(&provisioning::item_index()?, *result)?,
            *count,
        )),
        _ => None,
    };
    let quantity = product
        .map(|(reference, _)| full_e2ef5::rune_quantity(client, reference))
        .transpose()?;
    let rune_before = rune
        .map(|id| full_e2ef5::rune_quantity(client, id))
        .transpose()?;
    let first_sent = std::time::Instant::now();
    let reply = cast(client, index, rune, target, spell.needs_target).await?;
    let first_received = std::time::Instant::now();
    require(
        matches!(
            reply,
            Reply::Spell(SpellCastDisposition::Cast, true) | Reply::Rune(UseDisposition::Committed)
        ),
        format!("eligible cast refused: {reply:?}"),
    )?;
    if rune.is_some() {
        creature_sweep::read_stats(client).await?;
    }
    let after = vitals(client)?;
    let rune_after = rune
        .map(|id| full_e2ef5::rune_quantity(client, id))
        .transpose()?;
    let rune_paid = rune_before
        .zip(rune_after)
        .is_none_or(|(before, after)| before.checked_sub(1) == Some(after));
    let cooldown_us = spell
        .groups
        .iter()
        .map(|group| group.cooldown_micros)
        .chain(std::iter::once(spell.cooldown_micros))
        .max()
        .unwrap_or(0);
    let has_cooldown = cooldown_us > 0;
    rows.record(
        "cast_cost",
        require(
            paid(before, after, spell.mana, spell.soul) && rune_paid,
            format!(
                "expected mana={:?} soul={} before={before:?} after={after:?} rune_before={rune_before:?} rune_after={rune_after:?}",
                spell.mana, spell.soul
            ),
        ),
    );
    let retry_sent = std::time::Instant::now();
    let retry = if has_cooldown {
        Some(cast(client, index, rune, target, spell.needs_target).await)
    } else {
        None
    };
    let retry_received = std::time::Instant::now();
    let upper_us = retry_received.duration_since(first_sent).as_micros();
    let lower_us = retry_sent.duration_since(first_received).as_micros();
    evidence(&format!(
        "spell_sweep timing key={} first_sent_us=0 first_received_us={} retry_sent_us={} retry_received_us={upper_us} cooldown_us={cooldown_us}",
        spell.key,
        first_received.duration_since(first_sent).as_micros(),
        retry_sent.duration_since(first_sent).as_micros()
    ));
    let effect = if matches!(
        &retry,
        Some(Ok(
            Reply::Spell(SpellCastDisposition::Cast, _) | Reply::Rune(UseDisposition::Committed)
        ))
    ) {
        Err("retry also committed before effect observation".into())
    } else if let Some(((reference, count), quantity)) = product.zip(quantity) {
        require(
            full_e2ef5::rune_quantity(client, reference)? == quantity + u64::from(count),
            "authored conjure inventory output absent",
        )
    } else if let Some((icon, before)) = condition.zip(conditions_before) {
        let after = condition_status(client).await?;
        require(
            condition_increased(icon, &before, &after),
            format!("authored condition icon={icon} was not applied or refreshed"),
        )
    } else if dispel {
        async {
            let status = condition_status(client).await?;
            require(
                dispel_before && !status.conditions.iter().any(|condition| condition.icon == 14)
                    && after.health > before.health,
                format!("authored heal+dispel missing: paralyzed_before={dispel_before} health_before={} health_after={}", before.health, after.health),
            )
        }.await
    } else if let Some(name) = familiar {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            if client.world_entities().is_some_and(|entities| {
                entities.iter().any(|actor| {
                    !familiar_before.contains(&actor.entity)
                        && actor.display_name.as_ref().is_some_and(|display| {
                            display
                                .as_str()
                                .is_ok_and(|display| display.eq_ignore_ascii_case(name))
                        })
                })
            }) {
                break Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                break Err(format!("authored familiar {name} did not enter the wire view").into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
        }
    } else {
        match (health_effect, target) {
            (Some(HealthEffect::Heal), _) => require(
                after.health > before.health,
                "authored heal did not increase caster health",
            ),
            (Some(HealthEffect::Damage), Some((entity, _))) => {
                if rune.is_none() && spell.needs_target {
                    Err(
                        "aggregate damage also includes the selected attack target's auto-attacks"
                            .into(),
                    )
                } else {
                    damage_observed(client, entity).await
                }
            }
            _ => Err("authored effect has no implemented wire oracle".into()),
        }
    };
    rows.record("effect", effect);
    if let Some(retry) = retry {
        let cooldown = async {
            let reply = retry?;
            let expected = matches!(
                reply,
                Reply::Spell(SpellCastDisposition::CoolingDown, false)
                    | Reply::Rune(UseDisposition::Exhausted)
            ) && pools_unchanged(after, vitals(client)?);
            require(expected && upper_us < u128::from(cooldown_us), format!(
                "cooldown reply={reply:?} elapsed_lower_us={lower_us} elapsed_upper_us={upper_us} authored_us={cooldown_us} classification={}",
                if upper_us < u128::from(cooldown_us) { "within-window" }
                else { "inconclusive-wire-timing" }
            ))?;
            // The node owner adds the qualification offset to its monotonic clock
            // (gameplay_transport/monk_save.rs:43–60). Advancing by the entire authored
            // window guarantees the second probe is on the expired side of the deadline.
            if dispel && std::env::var_os("OTERYN_QUALIFICATION_CLOCK_FILE").is_some() {
                let step = Duration::from_millis(cooldown_us.div_ceil(1000));
                full_e2ef7::advance_qualification_time(step)?;
                let now = std::time::Instant::now();
                let reply = cast(client, index, rune, target, spell.needs_target).await?;
                let owner_lower_us = now.duration_since(first_received).as_micros() + step.as_micros();
                let owner_upper_us = first_sent.elapsed().as_micros() + step.as_micros();
                evidence(&format!(
                    "spell_sweep cooldown_expiry key={} owner_elapsed_lower_us={owner_lower_us} owner_elapsed_upper_us={owner_upper_us} injected_ms={} authored_us={cooldown_us} result={reply:?}",
                    spell.key, step.as_millis()
                ));
                require(
                    matches!(reply, Reply::Spell(SpellCastDisposition::Cast, true)),
                    format!("post-expiry cast refused: {reply:?}, owner_elapsed_lower_us={owner_lower_us}, authored_us={cooldown_us}"),
                )?;
            }
            Ok(())
        }
        .await;
        rows.record("cooldown", cooldown);
    }
    if !has_cooldown {
        let cooldown = async {
            let reply = cast(client, index, rune, target, spell.needs_target).await?;
            require(
                matches!(
                    reply,
                    Reply::Spell(SpellCastDisposition::Cast, true)
                        | Reply::Rune(UseDisposition::Committed)
                ),
                format!("zero-cooldown recast refused: {reply:?}"),
            )
        }
        .await;
        rows.record("cooldown", cooldown);
    }
    Ok(())
}

// The condition-status projection owns the wire icon mapping.
// crates/gameplay/dispatch/src/gameplay_transport/condition_status.rs:61.
fn condition_icon(spell: &SpellDefinition) -> Option<u8> {
    let Execution::Effects(effects) = &spell.execution else {
        return None;
    };
    let [SpellEffect::ResolvedOther { profile, .. }] = effects.as_slice() else {
        return None;
    };
    match profile.condition.as_ref()?.kind.as_str() {
        "invisible" | "invisibility" => Some(9),
        "haste" => Some(13),
        "recovery" => Some(16),
        "mana_shield" => Some(17),
        "light" => Some(18),
        "attributes" => Some(19),
        "outfit" => Some(22),
        _ => None,
    }
}

use oteryn_protocol_oteryn::condition_status::{self, ConditionStatus};

async fn condition_status(client: &mut DevClientSession) -> TestResult<ConditionStatus> {
    let reply = client
        .session_mut()
        .gated_command(
            condition_status::CAPABILITY_CONDITION_STATUS_V1,
            condition_status::COMMAND_TYPE_CONDITION_STATUS_QUERY,
            &[],
        )
        .await?;
    require(
        reply.status == CommandStatus::Accepted,
        "condition status query refused",
    )?;
    ConditionStatus::decode(&reply.payload).ok_or_else(|| "malformed condition status".into())
}

fn condition_increased(icon: u8, before: &ConditionStatus, after: &ConditionStatus) -> bool {
    after.conditions.iter().any(|new| {
        new.icon == icon
            && before
                .conditions
                .iter()
                .find(|old| (old.icon, old.sub_id) == (new.icon, new.sub_id))
                .is_none_or(|old| {
                    old.remaining_ms != -1
                        && (new.remaining_ms == -1 || new.remaining_ms > old.remaining_ms)
                })
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HealthEffect {
    Heal,
    Damage,
}

fn pure_effect(spell: &SpellDefinition) -> Option<HealthEffect> {
    let classify = |effects: &[SpellEffect]| match effects {
        [SpellEffect::Heal { .. }]
        | [
            SpellEffect::Heal { .. },
            SpellEffect::RemoveCondition { .. },
        ] => Some(HealthEffect::Heal),
        [SpellEffect::Damage { .. }] => Some(HealthEffect::Damage),
        _ => None,
    };
    match &spell.execution {
        Execution::Effects(effects) => classify(effects),
        Execution::AbilityVariants(variants) => {
            let first = classify(&variants.first()?.effects)?;
            variants
                .iter()
                .all(|variant| classify(&variant.effects) == Some(first))
                .then_some(first)
        }
        _ => None,
    }
}

async fn spawn_target(
    client: &mut DevClientSession,
    species: &str,
) -> TestResult<(oteryn_dev_client::EntityRef, ActorPosition)> {
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let answer = request_spawn(session, format!("oteryn:creature.{species}")).await?;
    let entity = parse_spawned_entity(&answer)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(actor) = client
            .world_entities()
            .and_then(|entities| entities.get(&entity))
        {
            return Ok((entity, actor.position));
        }
        require(
            tokio::time::Instant::now() < deadline,
            format!("fixture {species} absent from wire view"),
        )?;
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

async fn damage_observed(
    client: &DevClientSession,
    entity: oteryn_dev_client::EntityRef,
) -> TestResult {
    // The combat-death owner tracks only this session's damage to the exact fresh actor.
    // apps/game-server/src/node/serve/qualification_fixture.rs:160.
    let identity: String = entity
        .identity
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let answer = request_fixture_control(
        session,
        "--inspect-damage",
        format!("{identity} {}", entity.generation),
    )
    .await?;
    let mut fields = answer.split_ascii_whitespace();
    match (fields.next(), fields.next(), fields.next(), fields.next()) {
        (Some("damage"), Some(total), Some(_), None) => require(
            total.parse::<u64>()? > 0,
            "cast credited no damage to the fresh fixture creature",
        ),
        _ => Err(format!("invalid damage inspection: {answer}").into()),
    }
}

fn vitals(client: &DevClientSession) -> TestResult<ActorVitals> {
    client
        .actor_vitals()
        .copied()
        .ok_or_else(|| "actor vitals absent".into())
}

fn pools_unchanged(before: ActorVitals, after: ActorVitals) -> bool {
    (before.mana, before.soul) == (after.mana, after.soul)
}

// Source mana calculation: crates/gameplay/spell-rules/src/cast.rs:272.
fn mana_cost(before: ActorVitals, mana: ManaCost) -> u32 {
    match mana {
        ManaCost::Fixed(mana) => mana,
        ManaCost::PercentOfMaximum(percent) => {
            u32::try_from(u64::from(before.max_mana) * u64::from(percent) / 100).unwrap_or(u32::MAX)
        }
    }
}

fn paid(before: ActorVitals, after: ActorVitals, mana: ManaCost, soul: u32) -> bool {
    before.mana.checked_sub(mana_cost(before, mana)) == Some(after.mana)
        && before.soul.checked_sub(soul) == Some(after.soul)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_rows_are_failures_and_emit_once() {
        let mut tally = Tally::default();
        let mut rows = Rows {
            key: "fixture",
            tally: &mut tally,
            pending: STEPS.to_vec(),
            lap: std::time::Instant::now(),
        };
        rows.record("prepare", Err("fixture refused".into()));
        rows.blocked("prepare");
        rows.blocked("prepare");
        assert_eq!(tally.pass, 0);
        assert_eq!(tally.fail, STEPS.len());
    }

    #[test]
    fn batch_rejects_non_decimal_and_empty_ranges() -> TestResult {
        assert_eq!(batch(None)?, (0, 3));
        assert_eq!(batch(Some("0:20"))?, (0, 20));
        for bad in ["", "0:0", ":3", "+1:2", "4: 2", "1:2:3"] {
            assert!(batch(Some(bad)).is_err(), "{bad}");
        }
        Ok(())
    }

    #[test]
    fn condition_must_be_new_or_refreshed() {
        use condition_status::ActiveCondition;
        let original = ConditionStatus {
            conditions: vec![ActiveCondition {
                icon: 13,
                sub_id: 0,
                remaining_ms: 20,
            }],
        };
        let mut refreshed = original.clone();
        refreshed.conditions[0].remaining_ms = 30;
        assert!(condition_increased(13, &original, &refreshed));
        assert!(!condition_increased(17, &original, &refreshed));
        assert!(!condition_increased(13, &refreshed, &refreshed));
        assert!(!condition_increased(13, &refreshed, &original));
    }

    #[test]
    fn eligibility_needs_the_expected_wire_refusal_and_no_debit() {
        use SpellCastDisposition::{LevelTooLow, NotAvailable, NotEnoughMana};
        assert!(eligibility_refusal(&Reply::Spell(LevelTooLow, false), true));
        assert!(eligibility_refusal(
            &Reply::Spell(NotAvailable, false),
            false
        ));
        assert!(!eligibility_refusal(
            &Reply::Spell(NotAvailable, false),
            true
        ));
        assert!(!eligibility_refusal(&Reply::Spell(LevelTooLow, true), true));
        assert!(!eligibility_refusal(
            &Reply::Spell(NotEnoughMana, false),
            true
        ));
        assert!(!eligibility_refusal(
            &Reply::Rune(UseDisposition::RequirementNotMet),
            false
        ));
    }

    #[test]
    fn exact_cost_uses_maximum_mana_and_requires_soul_debit() {
        let before = ActorVitals {
            mana: 99,
            max_mana: 251,
            soul: 10,
            ..ActorVitals::default()
        };
        let after = ActorVitals {
            mana: 74,
            soul: 8,
            ..before
        };
        assert!(paid(before, after, ManaCost::PercentOfMaximum(10), 2));
        assert!(paid(before, after, ManaCost::Fixed(25), 2));
        assert!(!paid(before, after, ManaCost::Fixed(24), 2));
        assert!(!paid(before, after, ManaCost::Fixed(25), 0));
        assert!(!paid(before, after, ManaCost::Fixed(25), 11));
    }
}
