//! E2EF18 C14: ordinary combat progression, skill training, spell learning and reload.
use super::*;
use oteryn_dev_client::DevClientSession;
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
        let suffix = result.as_ref().err().map_or_else(String::new, |error|
            verdict::classify(error.as_ref()).suffix("C14.full/journey"));
        evidence(&format!(
            "e2e_all scenario=C14.full status={} seconds={:.3}{suffix}",
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

pub(super) async fn run_packet(clients: &SeamClients<'_>) -> TestResult {
    let mut row = Row::new();
    let result = tokio::time::timeout(Duration::from_secs(600), journey(&mut row, clients))
        .await.unwrap_or_else(|elapsed| Err(elapsed.into()));
    let released = release_row_characters(clients).await;
    row.finish(verdict::release_affects_verdict(&result, &released));
    released
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
        return Err(
            format!("step=general-stats observed={reply:?} need=accepted-GeneralStats").into(),
        );
    }
    if general_stats::validate(&reply.payload, general_stats::selected(client)).is_err()
        || reply.payload.first() != Some(&(CharacterTab::GeneralStats as u8))
        || reply.payload[33] > 1
    {
        return Err(
            "step=general-stats observed=malformed-frame need=negotiated-GeneralStats".into(),
        );
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
        return Err(
            format!("step=skill-progress observed={reply:?} need=accepted-capability-279").into(),
        );
    }
    SkillProgressSnapshot::decode(&reply.payload)
        .ok_or_else(|| "step=skill-progress observed=malformed-frame need=160-byte-snapshot".into())
}

async fn progress(client: &mut DevClientSession) -> TestResult<Progress> {
    Ok(Progress {
        stats: stats(client).await?,
        skills: skills(client).await?,
        vitals: *client
            .actor_vitals()
            .ok_or("step=vitals observed=missing-frame")?,
    })
}

fn qualification_world(
    clients: &SeamClients<'_>,
) -> TestResult<(&'static BundleWorld, oteryn_world_bundle::spawn::Table)> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let spawns = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?.spawns;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    Ok((world, spawns))
}

async fn rat_experience(client: &mut DevClientSession) -> TestResult<u64> {
    use oteryn_protocol_oteryn::daily_boost as wire;

    const RAT: &str = "oteryn:creature.rat";
    let (_, base) = full_gold::creature_profile(RAT)?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_DAILY_BOOST_V1)
    {
        return Ok(base);
    }
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DAILY_BOOST_V1,
            wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
            &[],
        )
        .await?;
    let mut panel = oteryn_dev_client::client_ui::daily_boost::DailyBoostPanel::default();
    oteryn_dev_client::client_ui::daily_boost::apply_result(&mut panel, &reply);
    let boost = panel
        .boost
        .ok_or("step=exact-kill-experience observed=missing-daily-boost")?;
    if boost.creature_key == RAT {
        return base
            .checked_mul(2)
            .ok_or_else(|| "step=exact-kill-experience observed=boost-overflow".into());
    }
    Ok(base)
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

fn carry_capacity_grew(before: Option<u64>, after: Option<u64>) -> bool {
    matches!((before, after), (Some(before), Some(after)) if before > 0 && after > before)
}

// C14 uses a solo free account with no personal boosts. Keep its oracle
// independent of the award owner, with a single final truncation.
fn expected_rat_award(base: u64, level: u32, stamina: u16) -> u64 {
    let steps = u64::from(level.clamp(2, 50) - 2);
    let scaled = u128::from(base) * u128::from(2400 - 25 * steps);
    match stamina {
        0 => 0,
        1..=840 => (scaled / 2400) as u64,
        _ => (scaled / 1200) as u64,
    }
}

async fn await_level_publication(
    client: &mut DevClientSession,
    deadline: tokio::time::Instant,
    expected_health: u32,
    expected_mana: u32,
) -> TestResult<oteryn_protocol_oteryn::actor_spell::ActorVitals> {
    while tokio::time::Instant::now() < deadline {
        if let Some(vitals) = client.actor_vitals().copied()
            && vitals.max_health == expected_health && vitals.max_mana == expected_mana
        {
            return Ok(vitals);
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    Err(format!("level-and-maxima: expected health={expected_health} mana={expected_mana}, observed {:?}", client.actor_vitals()).into())
}

pub(super) async fn fixture_request(
    client: &mut DevClientSession,
    arguments: &'static [&'static str],
) -> TestResult<String> {
    let socket = required("NODE_BOOT_CONTROL_SOCKET")?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let mut command = fixture_command();
    command.args(["--socket", &socket, "--session", &session]).args(arguments);
    let task = async {
        let output = run_fixture_command(&mut command).await?;
        if !output.status.success() {
            return Err(format!(
                "C14 fixture command refused: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ).into());
        }
        Ok(String::from_utf8(output.stdout)?)
    };
    wait_fixture_command(task, async || {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
        Ok(())
    })
    .await
}

async fn carry_capacity(client: &mut DevClientSession, expected_level: u32) -> TestResult<u64> {
    let answer = fixture_request(client, &["--capacity"]).await?;
    let mut tokens = answer.split_ascii_whitespace();
    let kind = tokens.next();
    let level = tokens.next().and_then(|value| value.parse().ok());
    let capacity = tokens.next().and_then(|value| value.parse::<u64>().ok());
    match capacity {
        Some(capacity)
            if kind == Some("capacity")
                && level == Some(expected_level)
                && capacity > 0
                && tokens.next().is_none() =>
        {
            Ok(capacity)
        }
        _ => Err(format!(
            "step=level-and-maxima observed={answer:?} expected-level={expected_level}"
        )
        .into()),
    }
}

fn exact_kill_award(delta: u64, expected: Option<u64>) -> bool {
    delta > 0 && expected == Some(delta)
}

fn automatic_unlocks_observed(expected: &[u32], newly_learned: &[u32]) -> bool {
    !expected.is_empty() && expected.iter().all(|index| newly_learned.contains(index))
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
            _ => {
                return Err(
                    format!("step=logout observed={reply:?} need=accepted-after-lock").into(),
                );
            }
        }
    }
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let (world, spawns) = qualification_world(clients)?;
    let mut client = connect_character(clients, 0xe2, 0).await?;
    let (_, vocation) = provisioning::e2ef1::fixture_spell_facts(&mut client).await?;
    if vocation == crate::spell::Vocation::None {
        // Setup ends before any baseline is sampled. A live kill must cross the authored
        // unlock threshold and publish the full award across it.
        fixture_request(&mut client, &["--profile", "combat-progression"]).await?;
        ordinary_logout(&mut client).await?;
        drop(client);
        client = connect_character(clients, 0xe4, 0).await?;
        evidence("E2EF18_SETUP C14 fixture=combat-progression before-baseline");
    }
    let old_session = client.join_snapshot().game_session_id;
    let baseline = progress(&mut client).await?;
    let mut capacity = carry_capacity(&mut client, baseline.stats.level).await?;
    let spells_before = learned(&mut client)?;
    let (_, vocation) = provisioning::e2ef1::fixture_spell_facts(&mut client).await?;
    let book = native_spell_book()?;
    let eligible_at_level = |spell: &crate::spell::SpellDefinition| {
        !spell.learning_required && spell.vocations.contains(&vocation)
    };
    let unlock_level = book
        .spells
        .iter()
        .enumerate()
        .filter(|(index, spell)| {
            !book.inactive_aliases.contains(index)
                && eligible_at_level(spell)
                && spell.level > baseline.stats.level
        })
        .map(|(_, spell)| spell.level)
        .min()
        .ok_or("step=automatic-spell-learning observed=no-authored-future-unlock")?;
    let mut expected_unlocks = Vec::new();
    for (index, spell) in book.spells.iter().enumerate() {
        if !book.inactive_aliases.contains(&index)
            && eligible_at_level(spell)
            && spell.level == unlock_level
        {
            expected_unlocks.push(u32::try_from(index + 1)?);
        }
    }
    let rat_experience = rat_experience(&mut client).await?;

    let own = client
        .world_entities()
        .and_then(|entities| {
            entities
                .iter()
                .find(|entry| entry.entity.identity == *entities.own_identity())
        })
        .copied()
        .ok_or("step=non-attackable-refusal observed=own-entity-absent")?;
    let refusal = client
        .session_mut()
        .attack_target(Some(&own.entity))
        .await?;
    if refusal.disposition == AttackIntentDisposition::Ok {
        return Err(
            "step=non-attackable-refusal observed=accepted-player-self need=refusal".into(),
        );
    }
    row.pass(
        "non-attackable-refusal",
        format!("reply={:?}", refusal.disposition),
    );

    let mut previous = baseline;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(420);
    let progression = tokio::time::timeout_at(deadline, async {
        while previous.stats.level < unlock_level {
            let target = approach_spawn_target(&mut client, world, &spawns, "oteryn:creature.rat", SpawnRespawn::Wait, None)
                .await?.ok_or("step=walk-to-weak-spawn observed=no-reachable-adjacent-rat")?;
            row.pass("walk-to-weak-spawn", format!("target={:?}", target.entity));
            client_hunt::kill(&mut client, target, clients.url, Duration::from_secs(60)).await?;
            let mut after = progress(&mut client).await?;
            let delta = after.stats.experience.checked_sub(previous.stats.experience)
                .ok_or("step=exact-kill-experience observed=experience-regressed")?;
            let expected = expected_rat_award(rat_experience, previous.stats.level, after.stats.stamina_minutes);
            if !exact_kill_award(delta, Some(expected)) {
                return Err(format!("step=exact-kill-experience observed={delta} expected={expected}").into());
            }
            row.pass("exact-kill-experience", format!("creature=oteryn:creature.rat delta={delta}"));
            if melee_progress(&baseline.skills, &after.skills) {
                row.pass("melee-skill-progress", format!("before={:?} after={:?}", baseline.skills.skills, after.skills.skills));
            }
            if after.stats.level != previous.stats.level {
                let grown = carry_capacity(&mut client, after.stats.level).await?;
                let (expected_health, expected_mana, _) = oteryn_gameplay_characters::vitals::maxima(
                    match vocation {
                        crate::spell::Vocation::None => "none",
                        crate::spell::Vocation::Druid => "druid",
                        crate::spell::Vocation::ElderDruid => "elder_druid",
                        crate::spell::Vocation::Sorcerer => "sorcerer",
                        crate::spell::Vocation::MasterSorcerer => "master_sorcerer",
                        crate::spell::Vocation::Knight => "knight",
                        crate::spell::Vocation::EliteKnight => "elite_knight",
                        crate::spell::Vocation::Paladin => "paladin",
                        crate::spell::Vocation::RoyalPaladin => "royal_paladin",
                        crate::spell::Vocation::Monk => "monk",
                        crate::spell::Vocation::ExaltedMonk => "exalted_monk",
                    }, after.stats.level,
                    oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
                    oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
                ).ok_or("level-and-maxima: missing authored maxima")?;
                after.vitals = await_level_publication(
                    &mut client, deadline.min(tokio::time::Instant::now() + Duration::from_secs(6)),
                    expected_health, expected_mana,
                ).await?;
                if after.stats.level != previous.stats.level + 1
                    || after.vitals.max_health <= previous.vitals.max_health
                    || after.vitals.max_mana < previous.vitals.max_mana
                    || (vocation == crate::spell::Vocation::Sorcerer
                        && after.vitals.max_mana == previous.vitals.max_mana)
                    || !carry_capacity_grew(Some(capacity), Some(grown))
                {
                    return Err(format!("step=level-and-maxima observed=before-{previous:?}/after-{after:?} capacity={capacity}=>{grown} need=level+1-vital-and-capacity-growth").into());
                }
                row.pass("level-and-maxima", format!("level={}=>{} health={}=>{} mana={}=>{} capacity_centi_oz={capacity}=>{grown}",
                    previous.stats.level, after.stats.level, previous.vitals.max_health, after.vitals.max_health,
                    previous.vitals.max_mana, after.vitals.max_mana));
                capacity = grown;
            }
            previous = after;
            client.take_events();
        }
        Ok::<_, Box<dyn std::error::Error>>(previous)
    }).await.map_err(|_| "step=level-and-maxima observed=timeout-420s")??;
    if tokio::time::Instant::now() >= deadline {
        return Err("step=level-and-maxima observed=timeout-420s".into());
    }
    let reached = progression;

    let catalog_deadline = deadline.min(tokio::time::Instant::now() + Duration::from_secs(6));
    let spells_after = loop {
        let spells = learned(&mut client)?;
        let newly_learned: Vec<_> = spells.iter().filter(|spell| {
            spell.learned && !spells_before.iter().any(|before| before.cast_index == spell.cast_index && before.learned)
        }).map(|spell| spell.cast_index).collect();
        if tokio::time::Instant::now() >= catalog_deadline {
            return Err(format!("step=automatic-spell-learning observed={newly_learned:?} expected={expected_unlocks:?} level={unlock_level}").into());
        }
        if automatic_unlocks_observed(&expected_unlocks, &newly_learned) {
            break spells;
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    };
    row.pass(
        "automatic-spell-learning",
        format!("new-level-spells={expected_unlocks:?}"),
    );

    ordinary_logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xe3, 0).await?;
    let persisted = progress(&mut fresh).await?;
    let persisted_capacity = carry_capacity(&mut fresh, persisted.stats.level).await?;
    if fresh.join_snapshot().game_session_id == old_session
        || persisted.stats.experience != reached.stats.experience
        || persisted.stats.level != reached.stats.level
        || persisted.skills != reached.skills
        || persisted.vitals.max_health != reached.vitals.max_health
        || persisted.vitals.max_mana != reached.vitals.max_mana
        || persisted_capacity != capacity
    {
        return Err(format!("step=fresh-admission-persistence observed={persisted:?} expected={reached:?} need=durable-xp-level-skills").into());
    }
    let persisted_spells = learned(&mut fresh)?;
    let persisted_indices: Vec<_> = persisted_spells
        .iter()
        .filter(|spell| spell.learned)
        .map(|spell| spell.cast_index)
        .collect();
    if !automatic_unlocks_observed(&expected_unlocks, &persisted_indices)
        || spells_after
            .iter()
            .filter(|spell| spell.learned)
            .any(|spell| !persisted_indices.contains(&spell.cast_index))
    {
        return Err("step=fresh-admission-persistence observed=learned-spell-missing need=durable-known-spells".into());
    }
    row.pass(
        "fresh-admission-persistence",
        format!(
            "xp={} level={} skill-progress-retained",
            persisted.stats.experience, persisted.stats.level
        ),
    );
    ordinary_logout(&mut fresh).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rat_oracle_requires_the_low_level_bonus_and_stamina_penalty() {
        assert_eq!(expected_rat_award(5, 5, 2520), 9);
        assert_eq!(expected_rat_award(5, 5, 840), 4);
        assert_eq!(expected_rat_award(5, 5, 0), 0);
    }

    #[test]
    fn level_and_maxima_requires_observed_carry_capacity_growth() {
        assert!(carry_capacity_grew(Some(58_000), Some(60_000)));
        assert!(!carry_capacity_grew(Some(58_000), Some(58_000)));
        assert!(!carry_capacity_grew(Some(58_000), Some(57_000)));
        assert!(!carry_capacity_grew(None, Some(60_000)));
        assert!(!carry_capacity_grew(Some(58_000), None));
    }

    #[test]
    fn first_kill_requires_the_authored_award() {
        assert!(!exact_kill_award(11, None));
        assert!(!exact_kill_award(11, Some(10)));
        assert!(exact_kill_award(10, Some(10)));
    }

    #[test]
    fn automatic_learning_requires_every_authored_unlock() {
        assert!(!automatic_unlocks_observed(&[], &[]));
        assert!(!automatic_unlocks_observed(&[11, 12], &[]));
        assert!(!automatic_unlocks_observed(&[11, 12], &[11]));
        assert!(automatic_unlocks_observed(&[11, 12], &[11, 12]));
    }

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
