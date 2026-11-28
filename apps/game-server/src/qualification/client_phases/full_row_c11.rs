//! C11: admit a Character created with the Monk vocation through the Platform
//! intent, exercise the native Harmony and Serene owners, and verify the vocation
//! on fresh admission. The Knight-only Battle Instinct Wheel perk must stay
//! inactive for this Monk. The dispatch route test separately covers a live
//! census granting an eligible Knight its positive bonuses.

use super::full_row_c01::admission_error;
use super::*;
use oteryn_dev_client::DevClientSession;
use std::collections::BTreeSet;

const MONK_CHARACTER: usize = 5;

const ACTIONS: &[&str] = &[
    "player-monk-choice",
    "solo-serene",
    "builders-to-five",
    "spender-to-zero",
    "zero-spender-path",
    "monk-battle-instinct-ineligible",
    "malformed-refusal",
    "fresh-admission-monk",
];

struct Row {
    passed: BTreeSet<&'static str>,
    failures: Vec<String>,
    started: std::time::Instant,
}
impl Row {
    fn new() -> Self {
        Self {
            passed: BTreeSet::new(),
            failures: Vec::new(),
            started: std::time::Instant::now(),
        }
    }
    fn pass(&mut self, action: &'static str, detail: impl std::fmt::Debug) {
        if ACTIONS.contains(&action) && self.passed.insert(action) {
            evidence(&format!("C11_CHECK {action} PASS observed={detail:?}"));
        } else {
            verdict::record_assertion(action);
            self.failures
                .push(format!("step={action} observed=duplicate-or-unknown"));
        }
    }
    fn fail(&mut self, action: &'static str, error: impl std::fmt::Display) {
        verdict::record_assertion(action);
        let error = format!("step={action} observed={error}");
        evidence(&format!("C11_CHECK {action} FAIL {error}"));
        self.failures.push(error);
    }
    fn finish(mut self, terminal: Option<String>) {
        if let Some(detail) = terminal {
            evidence(&format!("C11_GAP {detail}"));
            self.failures.push(detail);
        }
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&gap(action, &self.failures));
            }
        }
        let pass = self.failures.is_empty() && ACTIONS.iter().all(|a| self.passed.contains(a));
        evidence(&format!(
            "e2e_all scenario=C11.full status={} seconds={:.3} evidence={}",
            if pass { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
            if pass {
                "observed-monk-creation-harmony-serene-and-battle-instinct-ineligibility".into()
            } else {
                self.failures.join("_").replace(' ', "-")
            }
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let terminal = match verdict::indexed_timeout(
        Duration::from_secs(300), verdict::observe(journey(&mut row, clients)),
    ).await {
        Ok(Ok(())) => None,
        Ok(Err(error)) => {
            let detail = error.to_string();
            Some(if detail.starts_with("step=") { detail } else { format!("step=journey observed={detail}") })
        }
        Err(_) => Some("step=journey observed=timeout".into()),
    };
    row.finish(terminal);
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let mut client = connect_character(clients, 0xcb, MONK_CHARACTER)
        .await
        .map_err(|error| admission_error("initial-admission", error))?;
    let (vocation, level) = admitted_vocation(&mut client).await?;
    require_created_monk(vocation, level)?;
    row.pass("player-monk-choice", (vocation, level));

    if client.actor_vitals().is_some_and(|v| v.serene) {
        row.pass("solo-serene", client.actor_vitals());
    } else {
        row.fail("solo-serene", format!("vitals={:?}", client.actor_vitals()));
    }

    let book = spell_book()?;
    let mut builders_ok = true;
    for expected in 1..=5 {
        let attempt = async {
            select_melee_target(&mut client, world, &metadata.spawns).await?;
            cast(&mut client, &book, "exori infir pug", Some(expected)).await
        }.await;
        if let Err(error) = attempt {
            verdict::record_error(error.as_ref());
            row.fail("builders-to-five", format!("builder={expected} {error}"));
            builders_ok = false;
        }
        wait_spell(&mut client, &book, "exori infir pug").await?;
    }
    if builders_ok {
        row.pass("builders-to-five", client.actor_vitals());
    }
    let spender = async {
        select_melee_target(&mut client, world, &metadata.spawns).await?;
        require_spender_harmony(&client, 5)?;
        cast(&mut client, &book, "exori infir nia", Some(0)).await
    }.await;
    match spender {
        Ok(reply) => row.pass("spender-to-zero", reply),
        Err(error) => { verdict::record_error(verdict::IndexedError::as_error(&error)); row.fail("spender-to-zero", error) },
    }
    wait_spell(&mut client, &book, "exori infir nia").await?;
    // The native rules permit a zero-Harmony spender at normal mana cost.
    // Require an actual successful cast with no Harmony change.
    let zero_spender = async {
        select_melee_target(&mut client, world, &metadata.spawns).await?;
        require_spender_harmony(&client, 0)?;
        cast(&mut client, &book, "exori infir nia", Some(0)).await
    }.await;
    match zero_spender {
        Ok(outcome) => row.pass("zero-spender-path", outcome),
        Err(error) => {
            verdict::record_error(error.as_ref());
            row.fail("zero-spender-path", error);
        }
    }

    battle_queries(row, &mut client).await;
    let prior = client.join_snapshot().game_session_id;
    logout(&mut client, world).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xcc, MONK_CHARACTER)
        .await
        .map_err(|error| admission_error("fresh-admission-monk", error))?;
    if fresh.join_snapshot().game_session_id == prior {
        return Err("fresh Monk admission reused the prior game session".into());
    }
    let (fresh_vocation, _) = admitted_vocation(&mut fresh).await?;
    if fresh_vocation != vocation {
        return Err("fresh admission lost the chosen Monk vocation".into());
    }
    let fresh_builder = async {
        select_melee_target(&mut fresh, world, &metadata.spawns).await?;
        cast(&mut fresh, &book, "exori infir pug", Some(1)).await
    }.await;
    match fresh_builder {
        Ok(reply) if fresh.actor_vitals().is_some_and(|v| v.serene) => {
            row.pass("fresh-admission-monk", reply)
        }
        Ok(reply) => row.fail(
            "fresh-admission-monk",
            format!("reply={reply:?} vitals={:?}", fresh.actor_vitals()),
        ),
        Err(error) => { verdict::record_error(verdict::IndexedError::as_error(&error)); row.fail("fresh-admission-monk", error) },
    }
    logout(&mut fresh, world).await
}

fn spell_book() -> TestResult<crate::spell::SpellBook> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
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
fn spell_index(book: &crate::spell::SpellBook, words: &str) -> TestResult<std::num::NonZeroU32> {
    let key = &book.spoken(words).ok_or("C11 spell absent")?.spell.key;
    (1..=book.source_len())
        .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
        .find(|i| book.indexed(*i).is_some_and(|s| &s.key == key))
        .ok_or_else(|| "C11 spell index absent".into())
}
async fn select_melee_target(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
    travel_to_spawn(client, world, spawns, "oteryn:creature.rat").await?;
    let here = client.world_spatial().actor_position;
    let target = client.world_entities().and_then(|entities| entities.iter().find(|entry| {
        entry.kind == oteryn_dev_client::EntityKind::Creature && entry.position.floor == here.floor
            && entry.position.x.abs_diff(here.x).max(entry.position.y.abs_diff(here.y)) <= 1
    })).map(|entry| entry.entity).ok_or("no adjacent living creature for targeted Monk spell")?;
    let attack = client.session_mut().attack_target(Some(&target)).await?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("targeted Monk spell could not select attack target: {attack:?}").into());
    }
    Ok(())
}

async fn send_cast(
    client: &mut DevClientSession,
    book: &crate::spell::SpellBook,
    words: &str,
) -> TestResult<oteryn_dev_client::CastOutcome> {
    Ok(client
        .cast_spell(
            spell_index(book, words)?,
            oteryn_protocol_oteryn::actor_spell::SpellTarget::AttackTarget,
            false,
        )
        .await?)
}
async fn cast(
    client: &mut DevClientSession,
    book: &crate::spell::SpellBook,
    words: &str,
    harmony: Option<u32>,
) -> TestResult<oteryn_dev_client::CastOutcome> {
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("C11 pre-cast vitals absent")?;
    let outcome = send_cast(client, book, words).await?;
    client.session_mut().attack_target(None).await?;
    let spell = book.spoken(words).ok_or("C11 spell absent")?.spell;
    let crate::spell::ManaCost::Fixed(cost) = spell.mana else {
        return Err("C11 non-fixed mana".into());
    };
    if outcome.status != CommandStatus::Accepted
        || outcome.disposition != oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::Cast
    {
        return Err(format!("command={words} reply_status={:?} disposition={:?} before={before:?}; need eligible player progression/content", outcome.status, outcome.disposition).into());
    }
    let after = outcome
        .actor_vitals_delta
        .as_ref()
        .map(|d| d.value)
        .ok_or("accepted C11 cast missing vitals delta")?;
    if Some(after.mana) != before.mana.checked_sub(cost) || harmony.is_some_and(|h| after.harmony != h)
    {
        return Err(format!(
            "command={words} reply={outcome:?} expected_mana={} expected_harmony={harmony:?}",
            before.mana.saturating_sub(cost)
        )
        .into());
    }
    Ok(outcome)
}
async fn wait_spell(
    client: &mut DevClientSession,
    book: &crate::spell::SpellBook,
    words: &str,
) -> TestResult {
    let spell = book.spoken(words).ok_or("C11 spell absent")?.spell;
    let micros = spell
        .groups
        .iter()
        .map(|g| g.cooldown_micros)
        .max()
        .unwrap_or(0)
        .max(spell.cooldown_micros)
        + 200_000;
    let until = tokio::time::Instant::now() + Duration::from_micros(micros);
    while tokio::time::Instant::now() < until {
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    Ok(())
}

async fn battle_queries(row: &mut Row, client: &mut DevClientSession) {
    use oteryn_protocol_oteryn::battle_instinct_status as wire;
    match client.session_mut().gated_command(wire::CAPABILITY_BATTLE_INSTINCT_STATUS_V1, wire::COMMAND_TYPE_BATTLE_INSTINCT_STATUS_QUERY, &[]).await {
        Ok(reply) if reply.status == CommandStatus::Accepted => match wire::BattleInstinctStatus::decode(&reply.payload) {
            Some(status) if monk_battle_instinct_inactive(status) => row.pass("monk-battle-instinct-ineligible", status),
            Some(status) => row.fail("monk-battle-instinct-ineligible", format!("Monk received Knight perk: {status:?}")),
            None => row.fail("monk-battle-instinct-ineligible", format!("reply={reply:?} malformed status payload")),
        },
        Ok(reply) => row.fail("monk-battle-instinct-ineligible", format!("reply={reply:?}")), Err(error) => { verdict::record_error(verdict::IndexedError::as_error(&error)); row.fail("monk-battle-instinct-ineligible", error) },
    }
    match client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BATTLE_INSTINCT_STATUS_V1,
            wire::COMMAND_TYPE_BATTLE_INSTINCT_STATUS_QUERY,
            &[0],
        )
        .await
    {
        Ok(reply) if reply.status == CommandStatus::Rejected => {
            row.pass("malformed-refusal", reply.status)
        }
        Ok(reply) => row.fail("malformed-refusal", format!("reply={reply:?}")),
        Err(error) => { verdict::record_error(verdict::IndexedError::as_error(&error)); row.fail("malformed-refusal", error) },
    }
}

async fn logout(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    let safe = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| tile.protection_zone())
    }).ok_or("step=fresh-admission-monk observed=no-reachable-protection-zone need=safe-combat-logout")?;
    walk(client, world, &safe).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let reply = logout_retrying_busy(client).await?.0;
        match reply {
            Some(LogoutResult::Accepted) => return Ok(()),
            Some(LogoutResult::InFight { .. }) if tokio::time::Instant::now() < deadline => {
                client.service_liveness(Duration::from_millis(250)).await?;
                client.take_events();
            }
            _ => return Err(format!("C11 logout refused: {reply:?}").into()),
        }
    }
}

fn gap(action: &str, failures: &[String]) -> String {
    let prefix = format!("step={action} observed=");
    failures.iter().find(|failure| failure.starts_with(&prefix))
        .map_or_else(|| format!("C11_GAP step={action} unproven"), |failure| format!("C11_GAP {failure}"))
}

async fn admitted_vocation(client: &mut DevClientSession) -> TestResult<(u8, u32)> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let reply = client.session_mut().gated_command(wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
        wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, &[1, 0, 0, 1]).await?;
    if reply.status != CommandStatus::Accepted || reply.payload.first() != Some(&1) {
        return Err(format!("step=player-monk-choice observed={reply:?} need=admitted-vocation-and-level").into());
    }
    let vocation = general_stats::validate(&reply.payload, general_stats::selected(client))?
        .ok_or("step=player-monk-choice observed=vocation-extension-missing")?;
    Ok((vocation, u32::from_le_bytes(reply.payload[9..13].try_into()?)))
}

fn require_created_monk(vocation: u8, level: u32) -> TestResult {
    if Some(vocation) != oteryn_protocol_oteryn::cyclopedia_character::vocation_id("monk")
        || level != 1
    {
        return Err(format!("step=player-monk-choice need=Platform-created level-1 Monk observed=vocation-id-{vocation} level-{level}").into());
    }
    Ok(())
}

fn monk_battle_instinct_inactive(
    status: oteryn_protocol_oteryn::battle_instinct_status::BattleInstinctStatus,
) -> bool {
    status == oteryn_protocol_oteryn::battle_instinct_status::BattleInstinctStatus::default()
}

fn require_spender_harmony(client: &DevClientSession, expected: u32) -> TestResult {
    let actual = client.actor_vitals().ok_or("spender pre-cast vitals missing")?.harmony;
    if !spender_precondition(actual, expected) {
        return Err(format!("spender pre-cast harmony={actual} need={expected}").into());
    }
    Ok(())
}

fn spender_precondition(actual: u32, expected: u32) -> bool { actual == expected }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_review_c11_rejects_false_evidence() {
        assert!(spender_precondition(5, 5));
        assert!(!spender_precondition(0, 5));
        assert!(!spender_precondition(4, 5));
        assert!(spender_precondition(0, 0));
    }

    #[test]
    fn c11_requires_created_monk_and_no_knight_perk() -> TestResult {
        use oteryn_protocol_oteryn::battle_instinct_status::BattleInstinctStatus;
        use oteryn_protocol_oteryn::cyclopedia_character::vocation_id;

        let monk = vocation_id("monk").ok_or("Monk vocation id")?;
        let knight = vocation_id("knight").ok_or("Knight vocation id")?;
        require_created_monk(monk, 1)?;
        assert!(require_created_monk(knight, 1).is_err());
        assert!(require_created_monk(monk, 100).is_err());
        assert!(monk_battle_instinct_inactive(BattleInstinctStatus::default()));
        assert!(!monk_battle_instinct_inactive(BattleInstinctStatus {
            active: true,
            ..BattleInstinctStatus::default()
        }));
        Ok(())
    }

    #[test]
    fn gap_reports_the_observed_blocker_for_its_action() {
        let failures = ["step=player-monk-choice observed=creation vocation missing".to_owned()];
        assert_eq!(
            gap("player-monk-choice", &failures),
            "C11_GAP step=player-monk-choice observed=creation vocation missing"
        );
        assert_eq!(gap("solo-serene", &failures), "C11_GAP step=solo-serene unproven");
    }
}
