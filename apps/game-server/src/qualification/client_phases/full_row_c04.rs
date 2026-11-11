//! C04 full-row journey, derived from the matrix, re-audit, and the cast gate owner.
//!
//! 1. Admit the provisioned Character in the temple PZ; send an aggressive indexed CAST and
//!    require an accepted command carrying a rejected cast disposition, no vitals delta, and no
//!    mana change.
//! 2. Walk out of the PZ using native STEP commands; send the same CAST and require `Cast`, an
//!    authoritative vitals delta, and the exact mana debit.
//! 3. Send that spell again immediately; require `CoolingDown`, no vitals delta, and no debit.
//! 4. Send a different spell sharing its primary group, then one sharing a secondary group;
//!    require both cooldown refusals without resource deltas.
//! 5. Walk to an authored creature spawn, send ATTACK, wait for the server's feared condition
//!    status, then send the ordinary spell CAST; require `Rejected`, no delta, and no debit.
//! 6. After fear expires, send CAST again and require a normal cast and debit.
//! 7. LOGOUT, obtain a fresh admission, and re-query vitals/conditions; require the paid mana to
//!    remain paid, fear to be absent, and a post-cooldown cast to work. Cooldowns and conditions
//!    are temporal rather than durable; the durable re-check is the committed mana state.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::condition_status::{
    CAPABILITY_CONDITION_STATUS_V1, COMMAND_TYPE_CONDITION_STATUS_QUERY, ConditionStatus,
};

struct Row {
    started: std::time::Instant,
    completed: usize,
    failure: Option<String>,
}

impl Row {
    fn new() -> Self {
        Self {
            started: std::time::Instant::now(),
            completed: 0,
            failure: None,
        }
    }

    fn pass(&mut self, step: usize, observed: impl std::fmt::Debug) {
        self.completed = step;
        evidence(&format!("C04_CHECK step={step} PASS observed={observed:?}"));
    }

    fn finish(mut self, result: TestResult) {
        if let Err(error) = result {
            self.failure = Some(format!("step={} observed={error}", self.completed + 1));
        }
        if let Some(failure) = &self.failure {
            evidence(&format!(
                "C04_CHECK FAIL {failure}; need=reachable authored fear-applying player combat path and native reply/delta frames"
            ));
        }
        evidence(&format!(
            "e2e_all scenario=C04.full status={} seconds={:.3}{}",
            if self.failure.is_none() && self.completed == 7 {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64(),
            self.failure.as_ref().map_or(String::new(), |value| format!(
                " evidence={}",
                evidence_token(value)
            )),
        ));
    }
}

fn evidence_token(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._:-".contains(c) {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn spell_book() -> TestResult<crate::spell::SpellBook> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(
        &root.join("content/spells.manifest.json"),
    )?;
    let catalog =
        crate::spell::executable_catalog::compile(&input.catalog.bytes, &input.catalog.sha256)?;
    let selections = crate::spell::executable_catalog::compile_source_selection(
        &input.source_selection.bytes,
        &input.source_selection.sha256,
        &catalog,
    )?;
    Ok(catalog.into_spell_book_with_selections(&selections)?)
}

fn index(book: &crate::spell::SpellBook, words: &str) -> TestResult<std::num::NonZeroU32> {
    let wanted = &book
        .spoken(words)
        .ok_or_else(|| format!("served spell absent: {words}"))?
        .spell
        .key;
    (1..=book.source_len())
        .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
        .find(|i| book.indexed(*i).is_some_and(|spell| &spell.key == wanted))
        .ok_or_else(|| format!("served spell has no index: {words}").into())
}

async fn conditions(client: &mut DevClientSession) -> TestResult<ConditionStatus> {
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_CONDITION_STATUS_V1,
            COMMAND_TYPE_CONDITION_STATUS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("CONDITION_STATUS query reply={reply:?}").into());
    }
    ConditionStatus::decode(&reply.payload).ok_or_else(|| "malformed CONDITION_STATUS reply".into())
}

fn mana(client: &DevClientSession) -> TestResult<u32> {
    Ok(client.actor_vitals().ok_or("actor vitals absent")?.mana)
}

async fn cast(
    client: &mut DevClientSession,
    spell: std::num::NonZeroU32,
) -> TestResult<oteryn_dev_client::CastOutcome> {
    Ok(client.cast_spell(spell, SpellTarget::None, false).await?)
}

async fn wait_ready(
    client: &mut DevClientSession,
    spell: std::num::NonZeroU32,
) -> TestResult<oteryn_dev_client::CastOutcome> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    loop {
        let outcome = cast(client, spell).await?;
        if outcome.disposition != SpellCastDisposition::CoolingDown {
            return Ok(outcome);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("CAST cooldown did not expire reply={outcome:?}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

async fn journey(
    row: &mut Row,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let book = spell_book()?;
    let ordinary = index(&book, "exura")?;
    let primary_peer = index(&book, "exura gran")?;
    let aggressive = index(&book, "exevo vis lux")?;
    let mut client = connect_world(clients, 0xc4).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = async {

    let before = mana(&client)?;
    let pz = cast(&mut client, aggressive).await?;
    if pz.status != CommandStatus::Accepted
        || pz.disposition != SpellCastDisposition::Rejected
        || pz.actor_vitals_delta.is_some()
        || mana(&client)? != before
    {
        return Err(format!(
            "PZ CAST expected accepted/rejected/no-delta reply={pz:?} mana={before}->{}",
            mana(&client)?
        )
        .into());
    }
    row.pass(1, &pz);

    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    })
    .ok_or("no player STEP route out of protection zone")?;
    walk(&mut client, world, &path).await?;
    let before = mana(&client)?;
    let outside = wait_ready(&mut client, ordinary).await?;
    if outside.status != CommandStatus::Accepted
        || outside.disposition != SpellCastDisposition::Cast
        || outside.actor_vitals_delta.is_none()
        || mana(&client)? >= before
    {
        return Err(format!(
            "outside CAST expected cast/vitals debit reply={outside:?} mana={before}->{}",
            mana(&client)?
        )
        .into());
    }
    row.pass(2, &outside);

    let paid = mana(&client)?;
    let retry = cast(&mut client, ordinary).await?;
    if retry.disposition != SpellCastDisposition::CoolingDown
        || retry.actor_vitals_delta.is_some()
        || mana(&client)? != paid
    {
        return Err(format!(
            "same-spell cooldown expected no debit reply={retry:?} mana={paid}->{}",
            mana(&client)?
        )
        .into());
    }
    row.pass(3, &retry);

    let group = cast(&mut client, primary_peer).await?;
    if group.disposition != SpellCastDisposition::CoolingDown
        || group.actor_vitals_delta.is_some()
        || mana(&client)? != paid
    {
        return Err(format!("primary-group cooldown expected no debit reply={group:?}").into());
    }
    let source = book
        .indexed(ordinary)
        .ok_or("ordinary spell index vanished")?;
    let secondary_key = source
        .groups
        .get(1)
        .ok_or("served exura has no secondary cooldown group")?
        .key
        .as_str();
    let secondary = (1..=book.source_len())
        .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
        .find(|candidate| {
            *candidate != ordinary
                && book.indexed(*candidate).is_some_and(|spell| {
                    spell.groups.iter().skip(1).any(|g| g.key == secondary_key)
                })
        })
        .ok_or_else(|| {
            format!("no distinct served spell shares secondary group {secondary_key}")
        })?;
    let second = cast(&mut client, secondary).await?;
    if second.disposition != SpellCastDisposition::CoolingDown
        || second.actor_vitals_delta.is_some()
        || mana(&client)? != paid
    {
        return Err(format!("secondary-group cooldown expected no debit reply={second:?}").into());
    }
    row.pass(4, (&group, &second));

    let mut fear_source = None;
    let mut blocked = Vec::new();
    for creature in [
        "oteryn:creature.fungosaurus",
        "oteryn:creature.shrieking_cry_stal",
        "oteryn:creature.devoted_radiant_inquisitor",
        "oteryn:creature.devoted_radiant_paragon",
        "oteryn:creature.devoted_radiant_templar",
        "oteryn:creature.devoted_radiant_zealot",
        "oteryn:creature.doctor_marrow",
    ] {
        match travel_to_spawn(&mut client, world, spawns, creature).await {
            Ok(()) => {
                fear_source = Some(creature);
                break;
            }
            Err(error) => blocked.push(format!("{creature}:{error}")),
        }
    }
    let fear_source = fear_source.ok_or_else(|| {
        format!(
            "no player-walkable authored fear-source spawn; attempted={}",
            blocked.join("|")
        )
    })?;
    let here = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Creature
                    && entry.position.floor == here.floor
                    && entry
                        .position
                        .x
                        .abs_diff(here.x)
                        .max(entry.position.y.abs_diff(here.y))
                        <= 1
            })
        })
        .copied()
        .ok_or("combat approach has no adjacent authored creature")?;
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        let status = conditions(&mut client).await?;
        if status
            .conditions
            .iter()
            .any(|condition| condition.icon == 12)
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("fear_source={fear_source} ATTACK reply={attack:?}; CONDITION_STATUS timeout without feared icon=12 last={status:?}").into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
    client.session_mut().attack_target(None).await?;
    let fear_mana = mana(&client)?;
    let feared = cast(&mut client, ordinary).await?;
    if feared.disposition != SpellCastDisposition::Rejected
        || feared.actor_vitals_delta.is_some()
        || mana(&client)? != fear_mana
    {
        return Err(format!("feared CAST expected rejected/no debit reply={feared:?}").into());
    }
    row.pass(5, &feared);

    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    while conditions(&mut client)
        .await?
        .conditions
        .iter()
        .any(|condition| condition.icon == 12)
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("feared condition did not expire within 15s".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    let recovered = wait_ready(&mut client, ordinary).await?;
    if recovered.disposition != SpellCastDisposition::Cast || recovered.actor_vitals_delta.is_none()
    {
        return Err(format!("post-fear CAST reply={recovered:?}").into());
    }
    row.pass(6, &recovered);

    let durable_mana = mana(&client)?;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("LOGOUT refused after cast lifecycle".into());
    }
    row_needs_logout_0 = false;
    let mut fresh = connect_world(clients, 0xc5).await?;
    let row_needs_logout_1 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    if mana(&fresh)? != durable_mana
        || conditions(&mut fresh)
            .await?
            .conditions
            .iter()
            .any(|condition| condition.icon == 12)
    {
        return Err(format!(
            "fresh admission durable state mismatch expected_mana={durable_mana} observed_mana={}",
            mana(&fresh)?
        )
        .into());
    }
    let post = wait_ready(&mut fresh, ordinary).await?;
    if post.disposition != SpellCastDisposition::Cast || post.actor_vitals_delta.is_none() {
        return Err(format!("fresh admission CAST reply={post:?}").into());
    }
    row.pass(7, (&post, durable_mana));
    Ok(())

    }).await.unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_1 { row_logout(&mut fresh).await?; }
    row_result

    }.await;
    if row_needs_logout_0 {
        row_logout(&mut client).await?;
    }
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result: TestResult = async {
        let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
        let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
        let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
        let world = load_world(
            bytes,
            &pin,
            WorldId::decode(&clients.world)?,
            ChannelId::decode(&clients.channel)?,
        )?;
        row_complete(journey(&mut row, clients, world, &metadata.spawns))
            .await
            .map_err(|_| "C04 journey deadline after real player commands")?
    }
    .await;
    row.finish(result);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn evidence_tokens_are_report_safe() {
        assert_eq!(
            evidence_token("step 5: timeout (icon=12)"),
            "step-5:-timeout--icon-12-"
        );
    }
}
