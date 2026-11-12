//! E2EF18 C14: ordinary combat progression, skill training, spell learning and reload.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityDetail, EntityKind};
use oteryn_protocol_oteryn::{
    attack::AttackIntentDisposition,
    cyclopedia_character::{self as character, CharacterTab, GeneralStats},
    skill_progress::{self, SkillProgressSnapshot},
};

const ACTIONS: &[&str] = &[
    "non-attackable-refusal",
    "walk-to-weak-spawn",
    "exact-kill-experience",
    "level-and-maxima",
    "melee-skill-progress",
    "automatic-spell-learning",
    "fresh-admission-persistence",
];

struct Row {
    passed: std::collections::BTreeSet<&'static str>,
    started: std::time::Instant,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: Default::default(),
            started: std::time::Instant::now(),
        }
    }

    fn pass(&mut self, action: &'static str, detail: impl std::fmt::Display) {
        if ACTIONS.contains(&action) && self.passed.insert(action) {
            evidence(&format!("E2EF18_CHECK C14 {action} PASS {detail}"));
        }
    }

    fn finish(self, result: &TestResult) {
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("E2EF18_GAP C14 step={action} observed=unproven"));
            }
        }
        if let Err(error) = result {
            evidence(&format!(
                "E2EF18_GAP C14 observed={}",
                error.to_string().replace(['\n', '\r'], " ")
            ));
        }
        evidence(&format!(
            "e2e_all scenario=C14.full status={} seconds={:.3}",
            if result.is_ok() && self.passed.len() == ACTIONS.len() {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Progress {
    stats: GeneralStats,
    skills: SkillProgressSnapshot,
    vitals: oteryn_protocol_oteryn::actor_spell::ActorVitals,
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = tokio::time::timeout(Duration::from_secs(600), journey(&mut row, clients))
        .await
        .unwrap_or_else(|_| Err("step=journey observed=timeout-600s need=responsive-combat-progression".into()));
    row.finish(&result);
}

async fn stats(client: &mut DevClientSession) -> TestResult<GeneralStats> {
    let reply = client
        .session_mut()
        .gated_command(
            character::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            character::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[CharacterTab::GeneralStats as u8, 0, 0, 50],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=general-stats observed={reply:?} need=accepted-GeneralStats").into());
    }
    if general_stats::validate(&reply.payload, general_stats::selected(client)).is_err()
        || reply.payload.first() != Some(&(CharacterTab::GeneralStats as u8))
        || reply.payload[33] > 1
    {
        return Err("step=general-stats observed=malformed-frame need=negotiated-GeneralStats".into());
    }
    let mut skills = [0; 7];
    for (slot, bytes) in skills
        .iter_mut()
        .zip(reply.payload[15..29].as_chunks::<2>().0)
    {
        *slot = u16::from_le_bytes(*bytes);
    }
    let stats = GeneralStats {
        experience: u64::from_le_bytes(reply.payload[1..9].try_into()?),
        level: u32::from_le_bytes(reply.payload[9..13].try_into()?),
        magic_level: u16::from_le_bytes(reply.payload[13..15].try_into()?),
        skills,
        stamina_minutes: u16::from_le_bytes(reply.payload[29..31].try_into()?),
        offline_training_minutes: u16::from_le_bytes(reply.payload[31..33].try_into()?),
        last_vitals: (reply.payload[33] == 1).then(|| {
            (
                u32::from_le_bytes(reply.payload[34..38].try_into().unwrap_or_default()),
                u32::from_le_bytes(reply.payload[38..42].try_into().unwrap_or_default()),
            )
        }),
    };
    if stats.level == 0 {
        return Err("step=general-stats observed=zero-level need=nonzero-level".into());
    }
    Ok(stats)
}

async fn skills(client: &mut DevClientSession) -> TestResult<SkillProgressSnapshot> {
    let reply = client
        .session_mut()
        .gated_command(
            skill_progress::CAPABILITY_SKILL_PROGRESS_V1,
            skill_progress::COMMAND_TYPE_SKILL_PROGRESS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=skill-progress observed={reply:?} need=accepted-capability-279").into());
    }
    SkillProgressSnapshot::decode(&reply.payload)
        .ok_or_else(|| "step=skill-progress observed=malformed-frame need=160-byte-snapshot".into())
}

async fn progress(client: &mut DevClientSession) -> TestResult<Progress> {
    Ok(Progress {
        stats: stats(client).await?,
        skills: skills(client).await?,
        vitals: *client.actor_vitals().ok_or("step=vitals observed=missing-frame")?,
    })
}

fn rat_spawns() -> TestResult<Vec<TilePos>> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let mut positions: Vec<_> = metadata
        .spawns
        .sources
        .iter()
        .flat_map(|source| source.points.iter().map(move |point| (source.floor, point)))
        .filter(|(_, point)| {
            metadata
                .spawns
                .creatures
                .get(point.creature as usize)
                .is_some_and(|creature| creature.key == "oteryn:creature.rat")
        })
        .map(|(floor, point)| TilePos {
            x: point.x,
            y: point.y,
            floor,
        })
        .collect();
    positions.sort_by_key(|position| (position.x.abs_diff(TOWN.x) + position.y.abs_diff(TOWN.y), position.floor));
    positions.dedup();
    if positions.is_empty() {
        return Err("step=spawn-selection observed=no-rat-in-bundle need=weak-creature-spawn".into());
    }
    Ok(positions)
}

fn qualification_world(clients: &SeamClients<'_>) -> TestResult<&'static BundleWorld> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )
}

async fn approach_rat(
    client: &mut DevClientSession,
    world: &BundleWorld,
    positions: &[TilePos],
) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|cell| !cell.protection_zone())
            && positions.iter().any(|spawn| {
                spawn.floor == at.floor && spawn.x.abs_diff(at.x).max(spawn.y.abs_diff(at.y)) <= 1
            })
    })
    .ok_or("step=walk-to-weak-spawn observed=no-route-within-160-tiles need=reachable-rat")?;
    walk(client, world, &path).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        let here = client.world_spatial().actor_position;
        if let Some(target) = client.world_entities().and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
                    && entry.position.floor == here.floor
                    && entry.position.x.abs_diff(here.x).max(entry.position.y.abs_diff(here.y)) <= 1
            })
        }) {
            return Ok(*target);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=walk-to-weak-spawn observed=no-adjacent-creature-frame need=spawn-respawn".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
}

async fn kill(
    client: &mut DevClientSession,
    target: oteryn_dev_client::WorldSpatialEntity,
) -> TestResult {
    let reply = client.session_mut().attack_target(Some(&target.entity)).await?;
    if reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("step=melee-attack observed={reply:?} need=accepted-adjacent-attack").into());
    }
    let before_health = match target.detail {
        EntityDetail::Actor { health_percent, .. } => health_percent,
        _ => return Err("step=melee-attack observed=target-without-health need=actor-frame".into()),
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    let mut damaged = false;
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::WorldSpatialEntities(delta) = event {
                damaged |= delta.value.update.iter().any(|entry| {
                    entry.entity == target.entity
                        && matches!(entry.detail, EntityDetail::Actor { health_percent, .. } if health_percent < before_health)
                });
            }
        }
        if client.world_entities().is_some_and(|entities| entities.get(&target.entity).is_none()) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=melee-kill observed=timeout damage-frame={damaged} need=creature-death").into());
        }
    }
    let stopped = client.session_mut().attack_target(None).await?;
    if !damaged || stopped.disposition != AttackIntentDisposition::Ok {
        return Err(format!("step=melee-kill observed=damage-{damaged}/stop-{stopped:?} need=damage-and-stop").into());
    }
    Ok(())
}

fn melee_progress(before: &SkillProgressSnapshot, after: &SkillProgressSnapshot) -> bool {
    // Fist, club, sword and axe are indices 1..=4; the equipped starter weapon decides which.
    (1..=4).any(|index| {
        after.skills[index].level > before.skills[index].level
            || after.skills[index].progress > before.skills[index].progress
    })
}

fn learned(
    client: &mut DevClientSession,
) -> TestResult<Vec<oteryn_protocol_oteryn::actor_spell_catalog::SpellCatalogEntry>> {
    Ok(client
        .session_mut()
        .spell_catalog()?
        .ok_or("step=spell-list observed=missing-catalog need=capability-287")?
        .entries)
}

async fn ordinary_logout(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    loop {
        let reply = client.session_mut().logout().await?;
        match reply.result {
            Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) => return Ok(()),
            Some(oteryn_protocol_oteryn::logout::LogoutResult::InFight { .. })
                if tokio::time::Instant::now() < deadline =>
            {
                client.service_liveness(Duration::from_millis(250)).await?;
                client.take_events();
            }
            _ => return Err(format!("step=logout observed={reply:?} need=accepted-after-lock").into()),
        }
    }
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let world = qualification_world(clients)?;
    let positions = rat_spawns()?;
    let mut client = connect_character(clients, 0xe2, 0).await?;
    let old_session = client.join_snapshot().game_session_id;
    let baseline = progress(&mut client).await?;
    let spells_before = learned(&mut client)?;

    let own = client
        .world_entities()
        .and_then(|entities| entities.iter().find(|entry| entry.entity.identity == *entities.own_identity()))
        .copied()
        .ok_or("step=non-attackable-refusal observed=own-entity-absent")?;
    let refusal = client.session_mut().attack_target(Some(&own.entity)).await?;
    if refusal.disposition == AttackIntentDisposition::Ok {
        return Err("step=non-attackable-refusal observed=accepted-player-self need=refusal".into());
    }
    row.pass("non-attackable-refusal", format!("reply={:?}", refusal.disposition));

    let mut previous = baseline;
    let mut exact_xp = None;
    let mut levelled = None;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(420);
    while levelled.is_none() {
        let target = approach_rat(&mut client, world, &positions).await?;
        row.pass("walk-to-weak-spawn", format!("target={:?}", target.entity));
        kill(&mut client, target).await?;
        let after = progress(&mut client).await?;
        let delta = after.stats.experience.checked_sub(previous.stats.experience).ok_or(
            "step=exact-kill-experience observed=experience-regressed need=positive-delta",
        )?;
        if delta == 0 {
            return Err("step=exact-kill-experience observed=zero-delta need=creature-experience".into());
        }
        if let Some(expected) = exact_xp {
            if delta != expected {
                return Err(format!("step=exact-kill-experience observed={delta} expected={expected} need=stable-exact-rat-award").into());
            }
        } else {
            exact_xp = Some(delta);
            row.pass("exact-kill-experience", format!("creature=oteryn:creature.rat delta={delta}"));
        }
        if melee_progress(&baseline.skills, &after.skills) {
            row.pass("melee-skill-progress", format!("before={:?} after={:?}", baseline.skills.skills, after.skills.skills));
        }
        if after.stats.level > baseline.stats.level {
            levelled = Some(after);
        }
        previous = after;
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=level-and-maxima observed=timeout level={} xp={} need=next-level", after.stats.level, after.stats.experience).into());
        }
    }
    let reached = levelled.ok_or("step=level-and-maxima observed=no-level")?;
    if reached.stats.level != baseline.stats.level + 1
        || reached.vitals.max_health <= baseline.vitals.max_health
        || reached.vitals.max_mana < baseline.vitals.max_mana
    {
        return Err(format!("step=level-and-maxima observed=before-{baseline:?}/after-{reached:?} need=level+1-and-vital-maxima").into());
    }
    evidence(&format!("E2EF18_OBSERVATION C14 level-and-vitals level={}=>{} health={}=>{} mana={}=>{} capacity=absent-from-server-frame", baseline.stats.level, reached.stats.level, baseline.vitals.max_health, reached.vitals.max_health, baseline.vitals.max_mana, reached.vitals.max_mana));

    let spells_after = learned(&mut client)?;
    let eligible: Vec<_> = spells_after
        .iter()
        .filter(|spell| {
            spell.level == reached.stats.level
                && spell.learned
                && !spells_before
                    .iter()
                    .any(|before| before.cast_index == spell.cast_index && before.learned)
        })
        .map(|spell| spell.name.as_str())
        .collect();
    let next = spells_after.iter().filter(|spell| !spell.learned && spell.level > reached.stats.level).map(|spell| spell.level).min();
    if spells_after
        .iter()
        .any(|spell| spell.level == reached.stats.level && !spell.learned)
        && eligible.is_empty()
    {
        return Err(format!("step=automatic-spell-learning observed=no-learned-level-{}-spell need=catalog-delta", reached.stats.level).into());
    }
    row.pass("automatic-spell-learning", if eligible.is_empty() { format!("no-unlock-at-level-{} next-required-level={next:?}", reached.stats.level) } else { format!("new-level-spells={eligible:?}") });

    ordinary_logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xe3, 0).await?;
    let persisted = progress(&mut fresh).await?;
    if fresh.join_snapshot().game_session_id == old_session
        || persisted.stats.experience != reached.stats.experience
        || persisted.stats.level != reached.stats.level
        || persisted.skills != reached.skills
    {
        return Err(format!("step=fresh-admission-persistence observed={persisted:?} expected={reached:?} need=durable-xp-level-skills").into());
    }
    let persisted_spells = learned(&mut fresh)?;
    if spells_after.iter().filter(|spell| spell.learned).any(|spell| !persisted_spells.iter().any(|saved| saved.cast_index == spell.cast_index && saved.learned)) {
        return Err("step=fresh-admission-persistence observed=learned-spell-missing need=durable-known-spells".into());
    }
    row.pass("fresh-admission-persistence", format!("xp={} level={} skill-progress-retained", persisted.stats.experience, persisted.stats.level));
    ordinary_logout(&mut fresh).await?;

    // Capacity has no field in ActorVitalsV1 or either native progression reply. Do not
    // silently claim it: this failure is reached only after the complete live lifecycle.
    Err("step=level-and-maxima observed=capacity-field-absent-in-server-frames need=server-reported-carry-capacity-delta;source:crates/protocol-oteryn/src/actor_spell.rs:128".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn melee_progress_accepts_only_melee_family_growth() {
        let before = SkillProgressSnapshot::default();
        let mut after = before;
        after.skills[4].progress = 1;
        assert!(melee_progress(&before, &after));
        after = before;
        after.skills[5].progress = 1;
        assert!(!melee_progress(&before, &after));
    }
}
