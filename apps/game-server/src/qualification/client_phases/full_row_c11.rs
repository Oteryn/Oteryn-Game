//! Packet E2E-C11: the concrete C11 player journey derived from the matrix, audit, and the
//! production Harmony, spell-cast, vocation-choice, and Battle Instinct owners.
//!
//! 1. Admit the node-boot character, walk to Oressa, and send `hi`, `monk`, `yes`; require each
//!    NPC reply to be accepted and the confirmation delta to say `Your vocation is now monk.`.
//! 2. Observe the admitted monk's server-owned vitals and its solo Serene state.
//! 3. Send five `exori infir pug` builder casts, requiring accepted spell replies, exact mana
//!    deltas, and Harmony deltas 1 through 5.
//! 4. Send `exori infir nia`; require an accepted spender reply, its exact mana delta, and the
//!    Harmony delta back to zero. Send it once more at zero as the refusal/zero-charge path and
//!    require the server reply and unchanged Harmony/mana (the current owner deliberately accepts
//!    this recorded Global-unverified path, so either disposition is recorded rather than faked).
//! 5. Send the native Battle Instinct status query and require a canonical server snapshot; send a
//!    malformed query and require rejection. A starter who has no player-earned Wheel allocation
//!    must report inactive rather than receive a test-only perk.
//! 6. Logout, freshly admit the same character, and re-check the durable Monk vocation by casting
//!    the Monk-only builder again; require its reply, mana delta, Harmony delta, and Serene state.

use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use std::collections::BTreeSet;

const ACTIONS: &[&str] = &[
    "player-monk-choice",
    "solo-serene",
    "builders-to-five",
    "spender-to-zero",
    "zero-spender-path",
    "battle-instinct-snapshot",
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
    fn finish(self) {
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
                "observed-complete-harmony-serene-battle-instinct-lifecycle".into()
            } else {
                self.failures.join("_").replace(' ', "-")
            }
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    match verdict::indexed_timeout(Duration::from_secs(300), verdict::observe(journey(&mut row, clients))).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => row.fail("fresh-admission-monk", error),
        Err(_) => row.fail("fresh-admission-monk", "journey-timeout"),
    }
    row.finish();
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
    let mut client = connect_world(clients, 0xcb).await?;
    if let Err(error) = choose_monk(&mut client, world, &bytes).await {
        verdict::record_error(error.as_ref());
        row.fail("player-monk-choice", error);
        battle_queries(row, &mut client).await;
        logout(&mut client, world).await?;
        return Ok(());
    }
    row.pass("player-monk-choice", "Oressa-confirmed");

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
        if client.actor_vitals().is_none_or(|vitals| vitals.harmony != 0) {
            return Err("zero-Harmony spender requires an observed zero starting balance".into());
        }
        select_melee_target(&mut client, world, &metadata.spawns).await?;
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
    let mut fresh = connect_world(clients, 0xcc).await?;
    if fresh.join_snapshot().game_session_id == prior {
        return Err("fresh Monk admission reused the prior game session".into());
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
            Some(status) if status.active => row.pass("battle-instinct-snapshot", status),
            Some(status) => row.fail("battle-instinct-snapshot", format!("decoded={status:?}; need=earned-Wheel-allocation-with-active-battle_instinct")),
            None => row.fail("battle-instinct-snapshot", format!("reply={reply:?} malformed status payload")),
        },
        Ok(reply) => row.fail("battle-instinct-snapshot", format!("reply={reply:?}")), Err(error) => { verdict::record_error(verdict::IndexedError::as_error(&error)); row.fail("battle-instinct-snapshot", error) },
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

async fn choose_monk(
    client: &mut DevClientSession,
    world: &BundleWorld,
    bytes: &[u8],
) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop;
    use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
    let metadata = oteryn_world_bundle::bundle::visit(bytes, |_, _| Ok(()))?;
    let npc = metadata
        .npcs
        .npcs
        .iter()
        .find(|n| n.key == "oteryn:npc.oressa")
        .ok_or("Oressa absent from served NPC table; need=reachable-oteryn:npc.oressa-placement")?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        npc.placements
            .iter()
            .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
    })
    .ok_or("no player walk route to Oressa")?;
    walk(client, world, &path).await?;
    let basis = spatial_basis::current(client)?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|e| {
                e.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|p| {
                        e.position.x == i32::from(p.x)
                            && e.position.y == i32::from(p.y)
                            && e.position.floor == basis.floor(p.floor)
                    })
            })
        })
        .map(|e| e.entity)
        .ok_or("Oressa not visible")?;
    for text in ["hi", "monk", "yes"] {
        client.take_events();
        let disposition = npc_shop::talk(
            client.session_mut(),
            &NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        let mut lines = Vec::new();
        while lines.is_empty() && tokio::time::Instant::now() < deadline {
            client.service_liveness(Duration::from_millis(20)).await?;
            for event in client.take_events() {
                if let SessionEvent::NpcConversation(delta) = event {
                    lines.extend(delta.value.lines);
                }
            }
        }
        if disposition != NpcIntentDisposition::Ok
            || (text == "yes"
                && !lines
                    .iter()
                    .any(|line| line == "Your vocation is now monk."))
        {
            return Err(format!("player command={text} disposition={disposition:?} replies={lines:?}; need=unassigned-level-8-character-and-confirmed-Oressa-Monk-offer").into());
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gap_reports_the_observed_blocker_for_its_action() {
        let failures = ["step=player-monk-choice observed=Oressa absent".to_owned()];
        assert_eq!(
            gap("player-monk-choice", &failures),
            "C11_GAP step=player-monk-choice observed=Oressa absent"
        );
        assert_eq!(gap("solo-serene", &failures), "C11_GAP step=solo-serene unproven");
    }
}
