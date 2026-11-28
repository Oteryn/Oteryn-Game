//! Packet E2E-C11: the concrete C11 player journey derived from the matrix, audit, and the
//! production Harmony, spell-cast, vocation-choice, and Battle Instinct owners.
//!
//! 1. Follow Crystal 201189f8: Oressa redirects to Manop, four Dawnport shrine USEs
//!    precede his offer, and his confirmation transfers the level-8 candidate to Blue Valley.
//! 2. Observe the admitted monk's server-owned vitals and its solo Serene state.
//! 3. Send five `exori infir pug` builder casts, requiring accepted spell replies, exact mana
//!    deltas, and Harmony deltas 1 through 5.
//! 4. Send `exori infir nia`; require an accepted spender reply, its exact mana delta, and the
//!    Harmony delta back to zero. Send it once more at zero as the refusal/zero-charge path and
//!    require the server reply and unchanged Harmony/mana (the current owner deliberately accepts
//!    this recorded Global-unverified path, so either disposition is recorded rather than faked).
//! 5. On a separate promoted Premium Knight, commit a Wheel allocation and verify that
//!    Battle Instinct activates. Check malformed refusal on that same admission.
//! 6. Logout, freshly admit the same character, and re-check the durable Monk vocation by casting
//!    the Monk-only builder again; require its reply, mana delta, Harmony delta, and Serene state.

use super::full_row_c01::admission_error;
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::collections::BTreeSet;

const ORESSA_PREPARATION: character_prep::Seeded = character_prep::Seeded {
    profile: "c11-unassigned",
    reason: "C11 tests Manop’s vocation transfer and Harmony after reaching level 8.",
};
const KNIGHT_PREPARATION: character_prep::Seeded = character_prep::Seeded {
    profile: "c11-knight",
    reason: "C11 tests Battle Instinct activation after committing a Knight Wheel allocation.",
};

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
                "observed-complete-harmony-serene-battle-instinct-lifecycle".into()
            } else {
                self.failures.join("_").replace(' ', "-")
            }
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    character_prep::declare_seeded(
        "C11",
        "unassigned-level-8,premium-knight,wheel-fee-gold",
        "Manop transfer and Wheel mechanics",
    );
    let mut row = Row::new();
    let terminal = match verdict::indexed_timeout(
        Duration::from_secs(300),
        verdict::observe(async {
            if let Err(error) = knight_journey(&mut row, clients).await {
                verdict::record_error(error.as_ref());
                row.fail("battle-instinct-snapshot", error);
            }
            journey(&mut row, clients).await
        }),
    )
    .await
    {
        Ok(Ok(())) => None,
        Ok(Err(error)) => {
            let detail = error.to_string();
            Some(if detail.starts_with("step=") {
                detail
            } else {
                format!("step=journey observed={detail}")
            })
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
    let dedicated = character_prep::dedicated_named_character(
        clients,
        "NODE_BOOT_C11_UNASSIGNED_ACCOUNT",
        Some("Oressa Candidate"),
    )
    .await?;
    let clients = &dedicated.clients(clients);
    let mut client = ORESSA_PREPARATION
        .connect(clients, 0xcb, 0)
        .await
        .map_err(|error| admission_error("initial-admission", error))?;
    let (vocation, level) = admitted_vocation(&mut client).await?;
    if monk_path(vocation, level) != Some(false) || level != 8 {
        return Err(format!("Oressa candidate must first admit unassigned at level 8; observed vocation={vocation} level={level}").into());
    }
    choose_monk(&mut client, world, &metadata.npcs)
        .await
        .map_err(|error| {
            verdict::context(format!("step=player-monk-choice observed={error}"), error)
        })?;
    let (chosen, chosen_level) = admitted_vocation(&mut client).await?;
    if monk_path(chosen, chosen_level) != Some(true) || chosen_level < level {
        return Err(format!(
            "Manop confirmation did not install Monk: vocation={chosen} level={chosen_level}"
        )
        .into());
    }
    row.pass("player-monk-choice", "Manop-confirmed-at-Blue-Valley");

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

    let prior = client.join_snapshot().game_session_id;
    logout(&mut client, world).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xcc, 0)
        .await
        .map_err(|error| admission_error("fresh-admission-monk", error))?;
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

async fn knight_journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    use oteryn_protocol_oteryn::{battle_instinct_status as wire, wheel};
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_C11_KNIGHT_ACCOUNT").await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
    let clients = &SeamClients { accounts: &accounts, characters: &characters, ..*clients };
    let mut client = KNIGHT_PREPARATION.connect(clients, 0xca, 1).await?;
    character_prep::grant_item(&client, 3043, 2).await?;
    character_prep::reload(&mut client, clients).await?;
    character_prep::grant_premium(&client).await?;
    let king = approach_npc(&mut client, world, &metadata.npcs, "oteryn:npc.king_tibianus").await?;
    if talk(&mut client, king, "hi").await?.is_empty()
        || talk(&mut client, king, "promotion").await? != ["Do you want to be promoted for 20000 gold coins?"]
        || talk(&mut client, king, "yes").await? != ["You are now promoted."]
    {
        return Err("Knight’s real promotion dialogue did not confirm purchase".into());
    }

    let before = battle_status(&mut client).await?;
    if before.active || before.melee != 0 || before.shielding != 0 {
        return Err(format!("Battle Instinct active before allocation: {before:?}").into());
    }
    let request = wheel::WheelAllocationRequest {
        action: 0, occurrence: [0; 16], expected_revision: 0, points: [0; 36], vessels: None,
    };
    let initial = wheel_command(&mut client, &request).await?;
    if initial.refused || initial.points != [0; 36] {
        return Err(format!("expected clean Knight Wheel: {initial:?}").into());
    }
    // Pinned Knight route from core state slot 15 to Battle Instinct in state slot 1.
    let mut points = [0; 36];
    for (slot, capacity) in [(15, 50), (9, 75), (8, 100), (7, 150), (1, 200)] {
        points[slot - 1] = capacity;
    }
    let saved = wheel_command(&mut client, &wheel::WheelAllocationRequest {
        action: 1, occurrence: v7(0xcb, 1), expected_revision: initial.revision, points, vessels: None,
    }).await?;
    if saved.refused || Some(saved.revision) != initial.revision.checked_add(1) || saved.points != points
        || wheel_command(&mut client, &request).await? != saved
    {
        return Err(format!("Knight Wheel allocation did not commit: {saved:?}").into());
    }
    let active = battle_status(&mut client).await?;
    if !active.active || active.melee != 0 || active.shielding != 0 {
        return Err(format!("Battle Instinct did not activate in the empty temple: {active:?}").into());
    }
    row.pass("battle-instinct-snapshot", (saved, before, active));
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
    logout(&mut client, world).await
}

async fn battle_status(client: &mut DevClientSession) -> TestResult<oteryn_protocol_oteryn::battle_instinct_status::BattleInstinctStatus> {
    use oteryn_protocol_oteryn::battle_instinct_status as wire;
    let reply = client.session_mut().gated_command(wire::CAPABILITY_BATTLE_INSTINCT_STATUS_V1, wire::COMMAND_TYPE_BATTLE_INSTINCT_STATUS_QUERY, &[]).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("Battle Instinct query refused: {reply:?}").into());
    }
    wire::BattleInstinctStatus::decode(&reply.payload).ok_or_else(|| "invalid Battle Instinct payload".into())
}

pub(super) async fn wheel_command(client: &mut DevClientSession, request: &oteryn_protocol_oteryn::wheel::WheelAllocationRequest) -> TestResult<oteryn_protocol_oteryn::wheel::WheelAllocationReply> {
    use oteryn_protocol_oteryn::wheel;
    let payload = wheel::encode_wheel_request(request).map_err(|error| format!("Wheel request: {error:?}"))?;
    let reply = client.session_mut().gated_command(wheel::CAPABILITY_WHEEL_ALLOCATION_V1, wheel::COMMAND_TYPE_WHEEL_ALLOCATION, &payload).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("Wheel command refused: {reply:?}").into());
    }
    wheel::decode_wheel_reply(&reply.payload).map_err(|error| format!("Wheel reply: {error:?}").into())
}

// Search the served production graph. Dawnport can lie outside the shared town route radius.
async fn walk_to(
    client: &mut DevClientSession,
    world: &BundleWorld,
    label: &str,
    goal: impl Fn(TilePos) -> bool,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path = find_route(from, goal, |at, direction| {
        destination(world, at, direction)
    })
    .map_err(|visited| {
        format!("no legal STEP route to {label} from {from:?}; visited={visited}")
    })?;
    for direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, before, direction).ok_or("C11 route landing disappeared")?;
        let reply = client.step(direction).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if reply.status != CommandStatus::Accepted
            || reply.disposition != StepDisposition::Moved
            || actual != expected
        {
            return Err(format!("STEP to {label}: from={before:?} expected={expected:?} actual={actual:?} disposition={:?}", reply.disposition).into());
        }
        verdict::Harness::global().progress("C11 walk");
        if reply.step_wait_ms != 0 {
            client.service_liveness(Duration::from_millis(u64::from(reply.step_wait_ms))).await?;
        }
        client.take_events();
    }
    Ok(())
}

pub(super) async fn approach_npc(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    key: &str,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let npc = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == key)
        .ok_or_else(|| format!("served NPC table lacks {key}"))?;
    walk_to(client, world, key, |at| {
        npc.placements
            .iter()
            .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
    })
    .await?;
    let basis = spatial_basis::current(client)?;
    let at = tile(client.world_spatial().actor_position)?;
    client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|p| {
                        p.floor == at.floor
                            && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3
                            && entry.position
                                == basis.position(TilePos {
                                    x: p.x,
                                    y: p.y,
                                    floor: p.floor,
                                })
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or_else(|| format!("{key} was not visible at its served placement").into())
}

pub(super) async fn talk(
    client: &mut DevClientSession,
    npc: oteryn_dev_client::EntityRef,
    text: &str,
) -> TestResult<Vec<String>> {
    use oteryn_protocol_oteryn::npc_service as wire;
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(|error| format!("NPC {text} encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    let disposition = wire::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("NPC {text} decode: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!(
            "NPC {text} status={:?} disposition={disposition:?}",
            reply.status
        )
        .into());
    }
    loop {
        let delta = client
            .session_mut()
            .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
            .await?;
        if delta.server_sequence <= reply.server_sequence {
            continue;
        }
        let conversation = wire::decode_npc_conversation(&delta.payload)
            .map_err(|error| format!("NPC {text} conversation: {error:?}"))?;
        if conversation.open.map(|(actor, _)| actor) != Some(npc) {
            return Err(format!("NPC {text} conversation belongs to another actor").into());
        }
        return Ok(conversation.lines);
    }
}

async fn honour_shrine(
    client: &mut DevClientSession,
    world: &BundleWorld,
    action: u16,
) -> TestResult {
    use crate::map::view::{MapFacts, placement_key};
    let mut matches = world.base().tiles().flat_map(|(floor, x, y, view)| {
        let at = TilePos { x, y, floor };
        let count = view.depths().iter().filter(|depth| **depth == 0).count();
        (0..count).filter_map(move |ordinal| {
            let key = placement_key(at, u8::try_from(ordinal).ok()?)?;
            world
                .facts()
                .quest_callback_matches(key, None, Some(action), None)
                .then_some((at, key))
        })
    });
    let (shrine, key) = matches
        .next()
        .ok_or_else(|| format!("served Dawnport shrine action {action} is missing"))?;
    if matches.next().is_some() {
        return Err(format!("Dawnport shrine action {action} has ambiguous placements").into());
    }
    walk_to(client, world, "Dawnport shrine", |at| {
        at.floor == shrine.floor
            && at != shrine
            && at.x.abs_diff(shrine.x).max(at.y.abs_diff(shrine.y)) <= 1
    })
    .await?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let reply = client
        .session_mut()
        .use_object(&placement, world.facts().object_revision(key))
        .await?;
    if reply.status != CommandStatus::Accepted || reply.disposition != UseDisposition::Committed {
        return Err(format!("Dawnport shrine action {action} USE returned {reply:?}").into());
    }
    evidence(&format!("C11_SHRINE action={action} disposition=Committed"));
    Ok(())
}

async fn choose_monk(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    const MANOP: &str = "oteryn:npc.ambassador_manop";
    let oressa = approach_npc(client, world, npcs, "oteryn:npc.oressa").await?;
    talk(client, oressa, "hi").await?;
    let redirect = talk(client, oressa, "monk").await?;
    if redirect != ["If you are interested in becoming a monk, please speak to Ambassador Manop."] {
        return Err(format!("BLOCKED-SERVER crates/gameplay/npc/src/vocation_choice.rs:207 Oressa redirect observed={redirect:?}").into());
    }
    let manop = approach_npc(client, world, npcs, MANOP).await?;
    talk(client, manop, "hi").await?;
    talk(client, manop, "invitation").await?;
    talk(client, manop, "yes").await?;
    let before = talk(client, manop, "seeker").await?;
    if before
        != [
            "Look for one of our shrines around the outpost and return to me after you have honoured the Merudri.",
        ]
    {
        return Err(format!("BLOCKED-SERVER crates/gameplay/dispatch/src/gameplay_transport/npc_command/monk.rs:23 Manop pre-shrine response={before:?}").into());
    }
    for action in 7101..=7104 {
        honour_shrine(client, world, action).await?;
    }
    let manop = approach_npc(client, world, npcs, MANOP).await?;
    talk(client, manop, "hi").await?;
    let offer = talk(client, manop, "seeker").await?;
    if !offer.iter().any(|line| {
        line == "If you accept, I will send you to our temple in the Blue Valley as a MONK."
    }) {
        return Err(format!("BLOCKED-SERVER crates/gameplay/dispatch/src/gameplay_transport/npc_command/monk.rs:23 Manop transfer offer={offer:?}").into());
    }
    let confirmation = talk(client, manop, "yes").await?;
    if confirmation
        != ["You are now a pilgrim on the Three-Fold Path. Welcome to the Blue Valley, Seeker."]
    {
        return Err(format!("BLOCKED-SERVER crates/gameplay/dispatch/src/gameplay_transport/npc_command/monk.rs:23 Manop transfer confirmation={confirmation:?}").into());
    }
    let expected = ActorPosition {
        x: 33614,
        y: 31494,
        floor: 7,
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while client.world_spatial().actor_position != expected
        && tokio::time::Instant::now() < deadline
    {
        client.service_liveness(Duration::from_millis(20)).await?;
    }
    if client.world_spatial().actor_position != expected {
        return Err(format!("BLOCKED-SERVER crates/gameplay/dispatch/src/gameplay_transport/npc_command/monk.rs:23 Manop transfer landed at {:?}", client.world_spatial().actor_position).into());
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

pub(super) async fn admitted_vocation(client: &mut DevClientSession) -> TestResult<(u8, u32)> {
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

fn monk_path(vocation: u8, level: u32) -> Option<bool> {
    use oteryn_protocol_oteryn::cyclopedia_character::vocation_id;
    if Some(vocation) == vocation_id("monk") || Some(vocation) == vocation_id("exalted_monk") {
        Some(true)
    } else if Some(vocation) == vocation_id("none") && level >= 8 {
        Some(false)
    } else {
        None
    }
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
    fn independent_review_c11_rejects_unproven_evidence() {
        assert_eq!(monk_path(4, 8), None);
        assert_eq!(monk_path(1, 7), None);
        assert_eq!(monk_path(1, 8), Some(false));
        assert_eq!(monk_path(10, 1), Some(true));

    }


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
