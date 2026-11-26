//! C04 full-row journey, derived from the matrix, re-audit, and the cast gate owner.
//!
//! 1. Admit the provisioned Character in the temple PZ; send an aggressive indexed CAST and
//!    require a rejected command and cast disposition, no vitals delta, and no mana change.
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

use super::full_row_c01::admission_error;
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
            let detail = error.to_string();
            self.failure = Some(if detail.starts_with("step=") {
                detail
            } else {
                format!("step={} observed={detail}", self.completed + 1)
            });
        }
        if let Some(failure) = &self.failure {
            evidence(&format!(
                "C04_GAP {failure}"
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

pub(super) fn evidence_token(value: &str) -> String {
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

fn refused_without_cost(
    status: CommandStatus,
    disposition: SpellCastDisposition,
    has_vitals_delta: bool,
    mana_before: u32,
    mana_after: u32,
) -> bool {
    status == CommandStatus::Rejected
        && disposition == SpellCastDisposition::Rejected
        && !has_vitals_delta
        && mana_before == mana_after
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
    let book = spell_book()?;
    let ordinary = index(&book, "exevo infir flam hur")?;
    let primary_peer = index(&book, "exori infir vis")?;
    let mut client = connect_world(clients, 0xc4)
        .await
        .map_err(|error| admission_error("initial-admission", error))?;

    let ordinary_definition = book.indexed(ordinary).ok_or("Scorch definition missing")?;
    let shortest = std::iter::once(ordinary_definition.cooldown_micros)
        .chain(ordinary_definition.groups.iter().map(|group| group.cooldown_micros))
        .filter(|&micros| micros > 0).min().map(Duration::from_micros)
        .ok_or("protection-zone-control: no positive Scorch cooldown to distinguish")?;
    let pz_path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| tile.protection_zone())
            && pz_exit(world, at).is_some()
    }).ok_or("protection-zone-refusal: no reachable PZ boundary with an adjacent non-PZ tile")?;
    walk(&mut client, world, &pz_path).await?;
    let exit = pz_exit(world, tile(client.world_spatial().actor_position)?)
        .ok_or("protection-zone-control: boundary exit disappeared")?;
    let before = mana(&client)?;
    client.take_events();
    let started = tokio::time::Instant::now();
    let pz = cast(&mut client, ordinary).await?;
    if !refused_without_cost(
        pz.status, pz.disposition, pz.actor_vitals_delta.is_some(), before, mana(&client)?,
    ) {
        return Err(format!("PZ CAST expected rejected/rejected/no-delta reply={pz:?}").into());
    }
    assert_no_pz_cooldown(&mut client)?;
    let from = tile(client.world_spatial().actor_position)?;
    let stepped = client.step_retrying(exit).await?;
    if stepped.disposition != StepDisposition::Moved
        || stepped.world_spatial_delta.is_none()
        || Some(tile(client.world_spatial().actor_position)?) != destination(world, from, exit)
    {
        return Err(format!("protection-zone-control: immediate exit failed {stepped:?}").into());
    }
    assert_no_pz_cooldown(&mut client)?;
    let before = mana(&client)?;
    let outside = cast(&mut client, ordinary).await?;
    if !pz_control_in_window(started.elapsed(), shortest) {
        return Err(format!("protection-zone-control: first outside cast exceeded refusal evidence window {shortest:?}").into());
    }
    if outside.status != CommandStatus::Accepted
        || outside.disposition != SpellCastDisposition::Cast
        || outside.actor_vitals_delta.is_none()
        || before.checked_sub(8) != Some(mana(&client)?)
    {
        return Err(format!(
            "protection-zone-control: starter Scorch outside PZ expected Cast and 8 mana debit, reply={outside:?} mana={before}->{}",
            mana(&client)?
        )
        .into());
    }
    row.pass(1, &pz);
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
    use oteryn_protocol_oteryn::actor_spell_catalog as wire;
    let snapshot = client.session_mut().gated_snapshot(wire::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("secondary-group: no served actor spell catalogue")?;
    let catalogue = wire::decode_catalog(&snapshot.payload)
        .map_err(|error| format!("secondary-group: malformed actor spell catalogue {error:?}"))?;
    let eligible: Vec<_> = catalogue.iter().filter(|entry| entry.available && entry.rune_item.is_none()
        && entry.soul == 0 && !entry.mana_percent && entry.mana <= paid)
        .filter_map(|entry| {
            let index = std::num::NonZeroU32::new(entry.cast_index)?;
            let spell = book.indexed(index)?;
            (!spell.needs_target && !spell.groups.is_empty()).then_some((index, spell))
        }).collect();
    let pair = eligible.iter().find_map(|&(trigger, source)| {
        eligible.iter().find_map(|&(peer, other)| {
            (trigger != peer && !source.groups.iter().any(|group| group.key == other.groups[0].key)
                && source.groups.iter().skip(1).any(|secondary| other.groups.iter().skip(1).any(|group| group.key == secondary.key)))
                .then_some((trigger, peer))
        })
    });
    let (trigger, peer) = pair.ok_or("secondary-group: no eligible affordable spell pair with distinct primary groups and a shared secondary group")?;
    // The only earlier successful cast here was the ordinary spell. Wait out
    // its spell and every group timer before attempting the isolated pair.
    let previous = book.indexed(ordinary).ok_or("ordinary spell missing")?;
    let required = Duration::from_micros(previous.groups.iter()
        .map(|group| group.cooldown_micros)
        .fold(previous.cooldown_micros, u64::max));
    let started = tokio::time::Instant::now();
    while !cooldowns_elapsed(started.elapsed(), required) {
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    let trigger = cast(&mut client, trigger).await?;
    if trigger.status != CommandStatus::Accepted || trigger.disposition != SpellCastDisposition::Cast {
        return Err(format!("secondary-group: trigger could not cast {trigger:?}").into());
    }
    let after_trigger = mana(&client)?;
    let second = cast(&mut client, peer).await?;
    if second.disposition != SpellCastDisposition::CoolingDown
        || second.actor_vitals_delta.is_some()
        || mana(&client)? != after_trigger
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
    if !refused_without_cost(
        feared.status, feared.disposition, feared.actor_vitals_delta.is_some(), fear_mana, mana(&client)?,
    ) {
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
    let mut fresh = connect_world(clients, 0xc5)
        .await
        .map_err(|error| admission_error("fresh-admission", error))?;
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
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result: TestResult = verdict::observe(async {
        let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
        let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
        let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
        let world = load_world(
            bytes,
            &pin,
            WorldId::decode(&clients.world)?,
            ChannelId::decode(&clients.channel)?,
        )?;
        verdict::indexed_timeout(
            Duration::from_secs(120),
            journey(&mut row, clients, world, &metadata.spawns),
        )
        .await
        .map_err(|_| "C04 journey deadline after real player commands")?
    })
    .await;
    row.finish(result);
}

fn cooldowns_elapsed(elapsed: Duration, required: Duration) -> bool { elapsed >= required }

fn assert_no_pz_cooldown(client: &mut DevClientSession) -> TestResult {
    if client.take_events().into_iter().any(|event| matches!(event,
        oteryn_dev_client::SessionEvent::ActorCooldowns(delta)
            if delta.value.entries.iter().any(|entry| entry.expires_at_ms > delta.value.server_now_ms)
    )) {
        return Err("protection-zone-refusal: refused cast published an active cooldown".into());
    }
    Ok(())
}

fn pz_exit(world: &BundleWorld, from: TilePos) -> Option<StepDirection> {
    [StepDirection::North, StepDirection::East, StepDirection::South, StepDirection::West]
        .into_iter().find(|&direction| destination(world, from, direction).is_some_and(|at| {
            at.floor == from.floor
                && world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| !tile.protection_zone())
        }))
}

fn pz_control_in_window(elapsed: Duration, shortest: Duration) -> bool { elapsed < shortest }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_review_c04_rejects_false_evidence() {
        assert!(pz_control_in_window(Duration::from_millis(199), Duration::from_millis(200)));
        assert!(!pz_control_in_window(Duration::from_millis(200), Duration::from_millis(200)));
    }

    #[test]
    fn independent_review_c04_rejects_unproven_evidence() {
        assert!(!cooldowns_elapsed(Duration::from_millis(1), Duration::from_secs(2)));
        assert!(cooldowns_elapsed(Duration::from_secs(2), Duration::from_secs(2)));
    }

    #[test]
    fn refusal_requires_rejected_status_and_no_debit() {
        let denied = SpellCastDisposition::Rejected;
        assert!(refused_without_cost(CommandStatus::Rejected, denied, false, 55, 55));
        assert!(!refused_without_cost(CommandStatus::Accepted, denied, false, 55, 55));
        assert!(!refused_without_cost(CommandStatus::Rejected, denied, true, 55, 55));
        assert!(!refused_without_cost(CommandStatus::Rejected, denied, false, 55, 54));
    }

    #[test]
    fn evidence_tokens_are_report_safe() {
        assert_eq!(
            evidence_token("step 5: timeout (icon=12)"),
            "step-5:-timeout--icon-12-"
        );
    }
}
