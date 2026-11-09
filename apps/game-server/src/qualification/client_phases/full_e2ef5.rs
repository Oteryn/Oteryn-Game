//! E2EF5: player speech -> cast dispatcher -> appearance frames -> fresh admission.
//! No privileged provisioning or database access. Unfinished rows emit no invented verdict.
use super::*;
use oteryn_dev_client::{
    ChatDisposition, ChatIntent, ChatSpeechMode, DevClientSession, SessionEvent,
};
use oteryn_protocol_oteryn::appearance::{self, Binding, Look};
use std::collections::BTreeSet;

const ILLUSION_ACTIONS: &[&str] = &[
    "cast-rat",
    "rat-appearance",
    "unknown-creature-refusal",
    "nonillusionable-creature-refusal",
    "refusal-no-cost",
    "fresh-admission-condition",
    "expiry-restores-outfit",
    "fresh-admission-expiry",
];

struct Row {
    id: &'static str,
    required: &'static [&'static str],
    passed: BTreeSet<&'static str>,
    started: std::time::Instant,
    invalid: bool,
}
impl Row {
    fn new(id: &'static str, required: &'static [&'static str]) -> Self {
        Self {
            id,
            required,
            passed: BTreeSet::new(),
            started: std::time::Instant::now(),
            invalid: false,
        }
    }
    fn pass(&mut self, action: &'static str, sequence: u64) {
        // A zero sequence cannot identify an observed server frame.
        if sequence == 0 || !self.required.contains(&action) || !self.passed.insert(action) {
            self.invalid = true;
            return;
        }
        evidence(&format!(
            "E2EF5_CHECK {} {action} PASS server_sequence={sequence}",
            self.id
        ));
    }
    fn complete(&self) -> bool {
        !self.invalid
            && !self.required.is_empty()
            && self
                .required
                .iter()
                .all(|action| self.passed.contains(action))
    }
    fn finish(self, result: TestResult) {
        if let Err(error) = &result {
            evidence(&format!("E2EF5_GAP {} {error}", self.id));
        }
        for action in self.required {
            if !self.passed.contains(action) {
                evidence(&format!(
                    "E2EF5_CHECK {} {action} FAIL assertion-not-completed",
                    self.id
                ));
            }
        }
        evidence(&format!(
            "e2e_all scenario={}.full status={} seconds={:.3} evidence={}",
            self.id,
            if result.is_ok() && self.complete() {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64(),
            if result.is_ok() && self.complete() {
                "observed-all-required-actions"
            } else {
                "required-actions-unproven-see-E2EF5_GAP"
            },
        ));
    }
}

const HASTE_ACTIONS: &[&str] = &[
    "haste-cast",
    "haste-speed",
    "cooldown-no-cost",
    "haste-fresh-admission",
    "haste-expiry",
    "haste-recast",
    "strong-cast",
    "strong-speed-replacement",
    "strong-fresh-admission",
    "strong-expiry",
    "expiry-fresh-admission",
];

pub(super) async fn run(clients: &SeamClients<'_>, world: &BundleWorld) {
    // An error, including admission failure, belongs to one row only.
    for (id, actions, tag, limit) in [
        ("C17a", HASTE_ACTIONS, 0xa4, 140),
        ("C17c", ILLUSION_ACTIONS, 0xa1, 220),
    ] {
        let mut row = Row::new(id, actions);
        let result: TestResult = async {
            let mut client = connect_world(clients, tag).await?;
            let result = tokio::time::timeout(Duration::from_secs(limit), async {
                if id == "C17a" {
                    haste(&mut row, &mut client, clients, world).await
                } else {
                    illusion(&mut row, &mut client, clients).await
                }
            })
            .await;
            // Restore the entry tile even after a failed movement/cast assertion.
            let cleanup: TestResult = async {
                let back = route(world, tile(client.world_spatial().actor_position)?, |at| {
                    at == TOWN
                })
                .ok_or("E2EF5 return route unavailable")?;
                walk(&mut client, world, &back).await?;
                logout(&mut client).await
            }
            .await;
            if let Err(error) = &cleanup {
                evidence(&format!("E2EF5_GAP {id} cleanup: {error}"));
            }
            match result {
                Ok(result) => result?,
                Err(_) => {
                    return Err(format!(
                        "{id} native journey exceeded {limit}s; remaining actions unproven"
                    )
                    .into());
                }
            }
            cleanup
        }
        .await;
        row.finish(result);
    }
    rune_rows(clients, world).await;
    combat_rows(clients, world).await;
    companion_rows(clients, world).await;
    tile_item_row(clients, world).await;
}

fn own_binding(client: &mut DevClientSession) -> TestResult<Binding> {
    let own = *client
        .world_entities()
        .ok_or("C17c entities absent")?
        .own_identity();
    let snapshot = client
        .session_mut()
        .gated_snapshot(appearance::STATE_DOMAIN_APPEARANCE)
        .ok_or("C17c appearance snapshot absent")?;
    let snapshot = appearance::Snapshot::decode(&snapshot.payload)
        .map_err(|error| format!("appearance snapshot: {error:?}"))?;
    snapshot
        .bindings
        .into_iter()
        .find(|binding| binding.entity.identity == own)
        .ok_or_else(|| "C17c own appearance binding absent".into())
}

// GeneralStats wire has a fixed body (tab, xp, level, magic, seven skills,
// stamina, offline training, known-vitals flag, health, mana). Validate before slicing.
fn level_from_stats(payload: &[u8]) -> TestResult<u32> {
    use oteryn_protocol_oteryn::cyclopedia_character::CharacterTab;
    if payload.len() != 42
        || payload.first() != Some(&(CharacterTab::GeneralStats as u8))
        || payload[33] > 1
    {
        return Err("C17c invalid GeneralStats reply".into());
    }
    Ok(u32::from_le_bytes(payload[9..13].try_into()?))
}
async fn level(client: &mut DevClientSession) -> TestResult<u32> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let request = [wire::CharacterTab::GeneralStats as u8, 0, 0, 1];
    let result = client
        .session_mut()
        .gated_command(wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1, 388, &request)
        .await?;
    if result.status != CommandStatus::Accepted {
        return Err("C17c player level query refused".into());
    }
    level_from_stats(&result.payload)
}

async fn say(
    client: &mut DevClientSession,
    words: &str,
) -> TestResult<oteryn_dev_client::ChatOutcome> {
    Ok(client
        .chat(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: words.into(),
        })
        .await?)
}
async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("C17c logout refused".into());
    }
    Ok(())
}
async fn fresh(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    tag: u8,
) -> TestResult<Binding> {
    let previous = client.join_snapshot().game_session_id;
    let position = client.world_spatial().actor_position;
    let vitals = client
        .actor_vitals()
        .copied()
        .ok_or("E2EF5 logout vitals absent")?;
    logout(client).await?;
    *client = connect_world(clients, tag).await?;
    if previous == client.join_snapshot().game_session_id
        || client.world_spatial().actor_position != position
        || client
            .actor_vitals()
            .is_none_or(|after| after.mana != vitals.mana || after.soul != vitals.soul)
    {
        return Err(
            "C17c fresh admission did not retain position/mana/soul with a new game session".into(),
        );
    }
    own_binding(client)
}

fn own_reference(client: &DevClientSession) -> TestResult<u32> {
    let entities = client.world_entities().ok_or("C17c entities absent")?;
    let own = entities
        .iter()
        .find(|entity| &entity.entity.identity == entities.own_identity())
        .ok_or("C17c own entity absent")?;
    match own.detail {
        oteryn_dev_client::EntityDetail::Actor { appearance_ref, .. } => Ok(appearance_ref),
        _ => Err("C17c own actor appearance reference absent".into()),
    }
}

// Consume validated pushed frames, retaining only this connection's exact own actor.
fn observe(
    client: &mut DevClientSession,
    appearance_sequence: &mut u64,
    messages: &mut Vec<String>,
) -> TestResult {
    for event in client.take_events() {
        match event {
            SessionEvent::Appearance(delta) => {
                *appearance_sequence = delta.server_sequence;
            }
            SessionEvent::ChatLine(delta) => {
                if let oteryn_dev_client::ChatLine::Private { speaker_name, text } = delta.value
                    && speaker_name == "Game"
                    && messages.len() < 16
                {
                    messages.push(text);
                }
            }
            _ => {}
        }
    }
    Ok(())
}
async fn pump(
    client: &mut DevClientSession,
    appearance_sequence: &mut u64,
    messages: &mut Vec<String>,
) -> TestResult {
    client.service_liveness(Duration::from_millis(50)).await?;
    observe(client, appearance_sequence, messages)
}

async fn illusion(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult {
    let input = native_input()?;
    let book = native_spell_book()?;
    // Verify the complete player speech, including its parameter, resolves as a cast.
    let words = "utevo res ina \"rat\"";
    let spell = book
        .spoken(words)
        .ok_or("C17c rat illusion speech is not in pinned spell book")?
        .spell;
    if spell.learning_required {
        return Err(
            "C17c now requires player spell learning; no teacher journey implemented".into(),
        );
    }
    let crate::spell::Execution::NativeProfile(profile) = &spell.execution else {
        return Err("C17c native illusion profile absent".into());
    };
    let duration_ms = profile.spell()["execution"]["native_behavior"]["parameters"]["duration_ms"]
        .as_u64()
        .filter(|ms| *ms > 0 && *ms <= 180_000)
        .ok_or("C17c unqualified illusion duration")?;
    let crate::spell::ManaCost::Fixed(mana) = spell.mana else {
        return Err("C17c expected fixed illusion mana cost".into());
    };
    let appearances: serde_json::Value = serde_json::from_slice(
        &input
            .spell_appearances
            .as_ref()
            .ok_or("C17c spell appearances absent")?
            .bytes,
    )?;
    let rat = appearances["records"]
        .as_array()
        .and_then(|records| {
            records
                .iter()
                .find(|r| r["creature"]["key"] == "canary:creature/rat")
        })
        .ok_or("C17c exact rat appearance content absent")?;
    let expected = Look::Outfit {
        look_type: u32::try_from(rat["look_type"].as_u64().ok_or("C17c rat look type")?)?,
        colours: serde_json::from_value(rat["colours"].clone())?,
        addons: u8::try_from(rat["addons"].as_u64().ok_or("C17c rat addons")?)?,
    };
    let mut binding = own_binding(client)?;
    let baseline = binding.look.clone();
    if baseline == expected {
        return Err("C17c baseline already has rat appearance; cast would be unproven".into());
    }
    let player_level = level(client).await?;
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("C17c own vitals absent")?;
    client.take_events();
    let cast_started = tokio::time::Instant::now();
    let cast = say(client, words).await?;
    if cast.status != CommandStatus::Accepted || cast.disposition != ChatDisposition::Ok {
        return Err(format!("C17c legitimate player cast refused: status={:?} disposition={:?}; observed_level={player_level} required_level={} observed_mana={} required_mana={mana} required_vocations={:?}; ordinary starter has no journey to a qualifying vocation/level; source:crates/gameplay/dispatch/src/gameplay_transport/composed_admission/spells.rs:225", cast.status, cast.disposition, spell.level, before.mana, spell.vocations).into());
    }
    if player_level < spell.level {
        return Err("C17c server accepted illusion below its required level".into());
    }
    let mut messages = Vec::new();
    let mut appearance_sequence = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        observe(client, &mut appearance_sequence, &mut messages)?;
        if client.actor_vitals().is_some_and(|after| {
            before.mana.checked_sub(mana) == Some(after.mana) && after.soul == before.soul
        }) && own_reference(client)? != binding.appearance_ref
            && appearance_sequence > cast.result_server_sequence
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C17c cast lacks exact mana debit and rat appearance frames".into());
        }
        pump(client, &mut appearance_sequence, &mut messages).await?;
    }
    row.pass("cast-rat", cast.result_server_sequence);
    let active_reference = own_reference(client)?;

    // Wait for the authored own/group cooldowns, servicing real liveness throughout.
    let cooldown = spell
        .groups
        .iter()
        .map(|group| group.cooldown_micros)
        .chain([spell.cooldown_micros])
        .max()
        .ok_or("C17c cooldown absent")?;
    let ready = cast_started + Duration::from_micros(cooldown) + Duration::from_millis(100);
    while tokio::time::Instant::now() < ready {
        pump(client, &mut appearance_sequence, &mut messages).await?;
    }
    messages.clear();
    observe(client, &mut appearance_sequence, &mut messages)?;
    messages.clear();
    let before_negative = client
        .actor_vitals()
        .copied()
        .ok_or("C17c negative-path vitals absent")?;
    let sequence = refuse_illusion(
        client,
        book,
        "E2EF5 nonexistent creature",
        "A creature with this name does not exist.",
        active_reference,
    )
    .await?;
    row.pass("unknown-creature-refusal", sequence);
    let creatures: serde_json::Value = serde_json::from_slice(&input.creature_profiles.bytes)?;
    let denied_name = creatures["records"]
        .as_array()
        .and_then(|records| {
            records.iter().find_map(|record| {
                let details = &record["profile"]["data"]["profile"]["details"];
                (details["flags"]["illusionable"] == false)
                    .then(|| details["display_name"].as_str())
                    .flatten()
            })
        })
        .ok_or("C17c no exact nonillusionable creature in pinned content")?;
    let sequence = refuse_illusion(
        client,
        book,
        denied_name,
        "Sorry, not possible.",
        active_reference,
    )
    .await?;
    row.pass("nonillusionable-creature-refusal", sequence);
    binding = fresh(client, clients, 0xa2).await?;
    if binding.look != expected || binding.condition_revision == 0 {
        return Err("C17c fresh admission lost active rat illusion".into());
    }
    if client.actor_vitals().copied() != Some(before_negative) {
        return Err("C17c refusal resource change survived fresh admission".into());
    }
    row.pass("refusal-no-cost", client.last_server_sequence());
    row.pass("rat-appearance", client.last_server_sequence());
    row.pass("fresh-admission-condition", client.last_server_sequence());
    let expiry_deadline =
        tokio::time::Instant::now() + Duration::from_millis(duration_ms) + Duration::from_secs(10);
    let active_reference = own_reference(client)?;
    appearance_sequence = 0;
    while own_reference(client)? == active_reference || appearance_sequence == 0 {
        if tokio::time::Instant::now() >= expiry_deadline {
            return Err("C17c illusion never expired on real server clock".into());
        }
        pump(client, &mut appearance_sequence, &mut messages).await?;
    }
    if tokio::time::Instant::now() + Duration::from_secs(1)
        < cast_started + Duration::from_millis(duration_ms)
    {
        return Err("C17c expiry was early or did not restore original outfit".into());
    }
    // The next snapshot verifies that the changed reference resolves to the original outfit.
    let expiry_sequence = appearance_sequence;
    binding = fresh(client, clients, 0xa3).await?;
    if binding.look != baseline {
        return Err("C17c expired illusion reappeared after fresh admission".into());
    }
    row.pass("expiry-restores-outfit", expiry_sequence);
    row.pass("fresh-admission-expiry", client.last_server_sequence());
    Ok(())
}

async fn refuse_illusion(
    client: &mut DevClientSession,
    book: &crate::spell::SpellBook,
    name: &str,
    expected_feedback: &str,
    active_reference: u32,
) -> TestResult<u64> {
    if name.contains('"') {
        return Err("C17c creature name cannot be encoded as player speech".into());
    }
    let words = format!("utevo res ina \"{name}\"");
    if book.spoken(&words).is_none() {
        return Err("C17c refusal speech resolved as ordinary chat".into());
    }
    client.take_events();
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("C17c refusal vitals absent")?;
    let refused = say(client, &words).await?;
    if refused.disposition != ChatDisposition::Rejected {
        return Err("C17c invalid creature was not refused".into());
    }
    let mut messages = Vec::new();
    let mut appearance_sequence = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        observe(client, &mut appearance_sequence, &mut messages)?;
        if messages.iter().any(|message| message == expected_feedback) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                format!("C17c refusal missing exact Game feedback: {expected_feedback}").into(),
            );
        }
        pump(client, &mut appearance_sequence, &mut messages).await?;
    }
    if client.actor_vitals().copied() != Some(before) || own_reference(client)? != active_reference
    {
        return Err("C17c refused creature changed vitals or active illusion".into());
    }
    Ok(refused.result_server_sequence)
}

// A four-step, same-floor PZ corridor keeps the two speed trials on identical
// ground without monster combat, floor transitions or path-selection differences.
fn corridor(world: &BundleWorld, from: TilePos) -> Option<Vec<TilePos>> {
    for direction in [
        StepDirection::North,
        StepDirection::East,
        StepDirection::South,
        StepDirection::West,
    ] {
        let mut cells = vec![from];
        let mut at = from;
        for _ in 0..4 {
            let Some(next) = destination(world, at, direction) else {
                break;
            };
            if next.floor != from.floor
                || world.ground_speed(next) != world.ground_speed(from)
                || !world
                    .base()
                    .tile(next.x, next.y, next.floor)
                    .is_some_and(|tile| tile.protection_zone())
            {
                break;
            }
            cells.push(next);
            at = next;
        }
        if cells.len() == 5 {
            return Some(cells);
        }
    }
    None
}

// Time actual server STEP deltas of automatic walking. Client retries/sleeps
// never determine the cadence. Every intermediate tile must arrive in order.
async fn cadence_leg(client: &mut DevClientSession, cells: &[TilePos]) -> TestResult<Duration> {
    let (&from, rest) = cells.split_first().ok_or("C17a empty speed corridor")?;
    let &to = rest.last().ok_or("C17a speed corridor has no steps")?;
    if tile(client.world_spatial().actor_position)? != from {
        return Err("C17a speed trial starts on wrong tile".into());
    }
    client.take_events();
    let result = client
        .session_mut()
        .target_walk(ActorPosition {
            x: i32::from(to.x),
            y: i32::from(to.y),
            floor: -i16::from(to.floor),
        })
        .await?;
    if result.status != CommandStatus::Accepted || result.payload != [8, 1] {
        return Err("C17a native automatic walk refused".into());
    }
    let mut positions = rest.iter();
    let mut previous = from;
    let mut times = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        for event in client.take_events() {
            let observed = match event {
                SessionEvent::WorldSpatial(delta) => Some(delta.value.actor_position),
                SessionEvent::WorldSpatialEntities(delta) => Some(delta.value.actor_position),
                _ => None,
            };
            if let Some(position) = observed {
                let at = tile(position)?;
                if at == previous {
                    continue;
                }
                if positions.next() != Some(&at) {
                    return Err("C17a automatic walk deviated from fixed corridor".into());
                }
                previous = at;
                times.push(tokio::time::Instant::now());
            }
        }
        if previous == to {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C17a missing automatic walk position frames".into());
        }
        client.service_liveness(Duration::from_millis(5)).await?;
    }
    // Ignore command-to-first-step latency. Reject batched/jittery observations
    // instead of interpreting socket scheduling as an increase in player speed.
    let mut intervals: Vec<_> = times
        .windows(2)
        .map(|pair| pair[1].duration_since(pair[0]))
        .collect();
    intervals.sort();
    let first = *intervals.first().ok_or("C17a too few movement frames")?;
    let last = *intervals.last().ok_or("C17a too few movement frames")?;
    if first < Duration::from_millis(50) || last.saturating_sub(first) > Duration::from_millis(25) {
        return Err("C17a network jitter/batched frames prevent a speed assertion".into());
    }
    Ok(intervals[intervals.len() / 2])
}
async fn cadence(client: &mut DevClientSession, cells: &[TilePos]) -> TestResult<Duration> {
    let outward = cadence_leg(client, cells).await?;
    let reverse: Vec<_> = cells.iter().rev().copied().collect();
    let inward = cadence_leg(client, &reverse).await?;
    if outward.abs_diff(inward) > Duration::from_millis(25) {
        return Err("C17a forward/reverse speed measurements disagree".into());
    }
    Ok((outward + inward) / 2)
}
fn same_cadence(a: Duration, b: Duration) -> bool {
    a.abs_diff(b) <= Duration::from_millis(25)
}
fn faster(a: Duration, baseline: Duration) -> bool {
    a + Duration::from_millis(25) < baseline
}

// Expected values come from the pinned spell plan and production movement table;
// only the separately observed position frames can satisfy the row actions.
fn expected_cadence(
    spell: Option<&crate::spell::SpellDefinition>,
    level: u32,
    ground: u16,
) -> TestResult<Duration> {
    use oteryn_gameplay_characters::speed::{StepSpeedTable, effective_speed, player_base_speed};
    use oteryn_gameplay_spell_rules::native_companions as native;
    let base = player_base_speed(level).ok_or("C17a invalid observed player level")?;
    let delta = if let Some(spell) = spell {
        let crate::spell::Execution::NativeProfile(profile) = &spell.execution else {
            return Err("C17a missing native speed profile".into());
        };
        let facts = native::CompanionFacts::Haste(native::HasteFacts {
            caster: native::SpeedRecipient {
                id: 1,
                base_speed: i32::try_from(base)?,
                is_familiar: false,
                haste_suppressed: false,
                paralyze_suppressed: false,
                conditions: Vec::new(),
            },
            owned_summons: Vec::new(),
            swift_grade: native::SwiftGrade::None,
        });
        let native::CompanionPlan::Haste(plan) = native::plan(
            &profile.spell()["execution"]["native_behavior"]["parameters"],
            &facts,
        )
        .map_err(|error| format!("C17a expected speed plan: {error:?}"))?
        else {
            return Err("C17a unexpected speed plan".into());
        };
        match plan.caster_combat.operation {
            native::SpeedOperation::Start { condition, .. } => i64::from(condition.delta),
            _ => return Err("C17a expected new speed condition".into()),
        }
    } else {
        0
    };
    StepSpeedTable::embedded()
        .and_then(|table| table.step_duration(effective_speed(i64::from(base), delta, 0), ground))
        .ok_or_else(|| "C17a no production speed-table duration".into())
}

async fn cast_support(
    client: &mut DevClientSession,
    book: &crate::spell::SpellBook,
    words: &str,
) -> TestResult<u64> {
    let spell = book.spoken(words).ok_or("C17a support spell absent")?.spell;
    if spell.learning_required {
        return Err(
            format!("C17a {words} requires an unimplemented player teacher journey").into(),
        );
    }
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("C17a caster vitals absent")?;
    let player_level = level(client).await?;
    let cast = say(client, words).await?;
    if cast.status != CommandStatus::Accepted || cast.disposition != ChatDisposition::Ok {
        return Err(format!("C17a player cast {words} refused: {:?}; observed_level={player_level} required_level={} observed_mana={} cost={:?} required_vocations={:?} premium_required={}; no fixture promotion or mana minting permitted; source:crates/gameplay/dispatch/src/gameplay_transport/composed_admission/spells.rs:225", cast.disposition, spell.level, before.mana, spell.mana, spell.vocations, spell.premium).into());
    }
    let crate::spell::ManaCost::Fixed(mana) = spell.mana else {
        return Err("C17a non-fixed mana cost".into());
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if client.actor_vitals().is_some_and(|after| {
            before.mana.checked_sub(mana) == Some(after.mana) && after.soul == before.soul
        }) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C17a support cast has no exact mana debit frame".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
    Ok(cast.result_server_sequence)
}
async fn expire_speed(
    client: &mut DevClientSession,
    cells: &[TilePos],
    baseline: Duration,
    started: tokio::time::Instant,
    duration: Duration,
) -> TestResult {
    // A condition restored through admission can pause while offline. Allow one
    // complete authored lifetime since the last active-speed observation, while
    // keeping the total expiry bound tied to the actual cast.
    let until = tokio::time::Instant::now() + duration;
    let deadline = started + duration + Duration::from_secs(15);
    if until > deadline {
        return Err("C17a fresh-admission work exhausted expiry observation bound".into());
    }
    while tokio::time::Instant::now() < until {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    if !same_cadence(cadence(client, cells).await?, baseline) {
        return Err("C17a expired speed condition did not restore baseline cadence".into());
    }
    Ok(())
}
async fn haste(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let book = native_spell_book()?;
    let haste = book.spoken("utani hur").ok_or("C17a haste absent")?.spell;
    let strong = book
        .spoken("utani gran hur")
        .ok_or("C17a strong haste absent")?
        .spell;
    let duration = |spell: &crate::spell::SpellDefinition| -> TestResult<Duration> {
        let crate::spell::Execution::NativeProfile(profile) = &spell.execution else {
            return Err("C17a native speed profile absent".into());
        };
        let ms =
            profile.spell()["execution"]["native_behavior"]["parameters"]["caster"]["duration_ms"]
                .as_u64()
                .filter(|ms| *ms > 0 && *ms <= 30_000)
                .ok_or("C17a invalid speed condition duration")?;
        Ok(Duration::from_millis(ms))
    };
    let player_level = level(client).await?;
    if player_level < haste.level {
        return Err(format!("C17a player level frame={player_level}, Haste requires level {}, Strong Haste requires level {}; this packet has no legitimate advancement/vocation journey; source:crates/gameplay/dispatch/src/gameplay_transport/composed_admission/spells.rs:225", haste.level, strong.level).into());
    }
    let haste_duration = duration(haste)?;
    let strong_duration = duration(strong)?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.protection_zone())
            && corridor(world, at).is_some()
    })
    .ok_or("C17a no reachable fixed-ground PZ speed corridor")?;
    walk(client, world, &path).await?;
    let cells = corridor(world, tile(client.world_spatial().actor_position)?)
        .ok_or("C17a speed corridor unavailable")?;
    let ground = world.ground_speed(cells[0]);
    let baseline = cadence(client, &cells).await?;
    if !same_cadence(baseline, expected_cadence(None, player_level, ground)?) {
        return Err(
            "C17a baseline has equipment/other conditions or differs from production speed table"
                .into(),
        );
    }
    let started = tokio::time::Instant::now();
    let sequence = cast_support(client, book, "utani hur").await?;
    row.pass("haste-cast", sequence);
    let before_retry = client.actor_vitals().copied();
    // Indexed retry distinguishes spell cooldown from speech flood control.
    let index = (1..=book.source_len())
        .filter_map(|n| u32::try_from(n).ok().and_then(std::num::NonZeroU32::new))
        .find(|index| {
            book.indexed(*index)
                .is_some_and(|spell| spell.key == haste.key)
        })
        .ok_or("C17a haste index absent")?;
    let retry = client
        .cast_spell(
            index,
            oteryn_protocol_oteryn::actor_spell::SpellTarget::None,
            false,
        )
        .await?;
    if retry.disposition != oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::CoolingDown
        || retry.actor_vitals_delta.is_some()
        || client.actor_vitals().copied() != before_retry
    {
        return Err("C17a immediate repeat was not a resource-free spell cooldown refusal".into());
    }
    row.pass("cooldown-no-cost", retry.result_server_sequence);
    let hasted = cadence(client, &cells).await?;
    if !faster(hasted, baseline)
        || !same_cadence(hasted, expected_cadence(Some(haste), player_level, ground)?)
    {
        return Err("C17a haste did not shorten observed server movement cadence".into());
    }
    row.pass("haste-speed", client.last_server_sequence());
    fresh(client, clients, 0xa5).await?;
    if !same_cadence(cadence(client, &cells).await?, hasted) {
        return Err("C17a fresh admission lost haste speed".into());
    }
    row.pass("haste-fresh-admission", client.last_server_sequence());
    expire_speed(client, &cells, baseline, started, haste_duration).await?;
    row.pass("haste-expiry", client.last_server_sequence());
    let recast_started = tokio::time::Instant::now();
    let sequence = cast_support(client, book, "utani hur").await?;
    row.pass("haste-recast", sequence);
    if !same_cadence(cadence(client, &cells).await?, hasted) {
        return Err("C17a recast did not reapply haste".into());
    }
    let cooldown = haste
        .groups
        .iter()
        .map(|group| group.cooldown_micros)
        .chain([haste.cooldown_micros])
        .max()
        .ok_or("C17a support cooldown absent")?;
    while tokio::time::Instant::now()
        < recast_started + Duration::from_micros(cooldown) + Duration::from_millis(100)
    {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    let started = tokio::time::Instant::now();
    let sequence = cast_support(client, book, "utani gran hur").await?;
    row.pass("strong-cast", sequence);
    let stronger = cadence(client, &cells).await?;
    if !faster(stronger, hasted)
        || !same_cadence(
            stronger,
            expected_cadence(Some(strong), player_level, ground)?,
        )
    {
        return Err("C17a strong haste did not replace haste with a faster condition".into());
    }
    row.pass("strong-speed-replacement", client.last_server_sequence());
    fresh(client, clients, 0xa6).await?;
    if !same_cadence(cadence(client, &cells).await?, stronger) {
        return Err("C17a fresh admission lost strong haste speed".into());
    }
    row.pass("strong-fresh-admission", client.last_server_sequence());
    expire_speed(client, &cells, baseline, started, strong_duration).await?;
    row.pass("strong-expiry", client.last_server_sequence());
    fresh(client, clients, 0xa7).await?;
    if !same_cadence(cadence(client, &cells).await?, baseline) {
        return Err("C17a expired speed condition returned after fresh admission".into());
    }
    row.pass("expiry-fresh-admission", client.last_server_sequence());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verdict_requires_every_distinct_action_and_nonzero_frame() {
        let mut row = Row::new("C17c", &["cast", "expiry"]);
        assert!(!row.complete());
        row.pass("cast", 1);
        assert!(!row.complete());
        row.pass("expiry", 2);
        assert!(row.complete());
        row.pass("expiry", 3);
        assert!(!row.complete());
        let mut row = Row::new("C17c", &["cast"]);
        row.pass("cast", 0);
        assert!(!row.complete());
        assert!(!Row::new("C17c", &[]).complete());
    }
    #[test]
    fn speed_comparisons_do_not_pass_for_equal_or_jitter_only_cadence() {
        let ms = Duration::from_millis;
        assert!(same_cadence(ms(300), ms(320)));
        assert!(!faster(ms(300), ms(320)));
        assert!(!faster(ms(300), ms(300)));
        assert!(faster(ms(250), ms(300)));
        assert!(!same_cadence(ms(250), ms(300)));
    }
    #[test]
    fn pinned_haste_profiles_have_distinct_observable_speed_cadences() -> TestResult {
        let book = native_spell_book()?;
        for level in [23, 100] {
            let baseline = expected_cadence(None, level, 150)?;
            let haste = expected_cadence(
                Some(book.spoken("utani hur").ok_or("haste")?.spell),
                level,
                150,
            )?;
            let strong = expected_cadence(
                Some(book.spoken("utani gran hur").ok_or("strong haste")?.spell),
                level,
                150,
            )?;
            assert!(faster(haste, baseline));
            assert!(faster(strong, haste));
        }
        assert!(expected_cadence(None, 0, 150).is_err());
        Ok(())
    }
    #[test]
    fn general_stats_level_uses_the_production_encoder() -> TestResult {
        use oteryn_protocol_oteryn::cyclopedia_character::{GeneralStats, TabView};
        let bytes = TabView::GeneralStats(GeneralStats {
            level: 23,
            ..Default::default()
        })
        .encode()
        .map_err(|error| format!("stats encode: {error:?}"))?;
        assert_eq!(level_from_stats(&bytes)?, 23);
        assert!(level_from_stats(&bytes[..13]).is_err());
        assert!(level_from_stats(&[0; 42]).is_err());
        Ok(())
    }
    #[test]
    fn illusion_speech_resolves_both_real_and_negative_parameters() -> TestResult {
        let book = native_spell_book()?;
        for (words, parameter) in [
            ("utevo res ina \"rat\"", "rat"),
            (
                "utevo res ina \"E2EF5 nonexistent creature\"",
                "E2EF5 nonexistent creature",
            ),
        ] {
            let spoken = book.spoken(words).ok_or("parameter speech not resolved")?;
            assert_eq!(spoken.spell.key, "candidate:spell/creature_illusion");
            assert_eq!(spoken.parameter.as_deref(), Some(parameter));
        }
        Ok(())
    }
}

const PARALYZE_ACTIONS: &[&str] = &[
    "acquire-paralyze",
    "illegal-floor-no-cost",
    "target-baseline",
    "paralyze-cast-and-debit",
    "target-paralyzed-speed",
    "target-expiry",
    "fresh-admission-rune-custody",
];
const BARRIER_ACTIONS: &[&str] = &[
    "acquire-magic-wall",
    "wall-illegal-floor-no-cost",
    "wall-cast-and-debit",
    "wall-map-item",
    "wall-blocked-traversal",
    "wall-expiry-and-traversal",
    "acquire-wild-growth",
    "growth-illegal-floor-no-cost",
    "growth-cast-and-debit",
    "growth-map-item",
    "growth-blocked-traversal",
    "growth-expiry-and-traversal",
    "fresh-admission-barrier-custody",
];

async fn rune_rows(clients: &SeamClients<'_>, world: &BundleWorld) {
    for (id, actions, tag) in [
        ("C16b", PARALYZE_ACTIONS, 0xb0),
        ("C16a", BARRIER_ACTIONS, 0xb4),
    ] {
        let mut row = Row::new(id, actions);
        let result: TestResult = async {
            let mut client = connect_world(clients, tag).await?;
            let result = tokio::time::timeout(Duration::from_secs(240), async {
                if id == "C16b" {
                    paralyze(&mut row, &mut client, clients, world).await
                } else {
                    barriers(&mut row, &mut client, clients, world).await
                }
            })
            .await;
            // Also close this admission on prerequisite failures and assertion errors.
            let cleanup = rune_cleanup(&mut client, world).await;
            if let Err(error) = &cleanup {
                evidence(&format!("E2EF5_GAP {id} cleanup: {error}"));
            }
            result
                .map_err(|_| format!("{id} 240s journey deadline; remaining actions unproven"))??;
            cleanup
        }
        .await;
        row.finish(result);
    }
}

async fn rune_cleanup(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let travel: TestResult = async {
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
            at == TOWN
        })
        .ok_or("rune journey return route unavailable")?;
        walk(client, world, &path).await
    }
    .await;
    let close = player_logout(client).await;
    travel?;
    close
}

// An independently admitted observer must answer native liveness while the
// other player walks or waits out combat logout. Keep pushed evidence queued.
async fn with_observer_liveness<T>(
    observer: &mut DevClientSession,
    operation: impl std::future::Future<Output = TestResult<T>>,
) -> TestResult<T> {
    let mut operation = std::pin::pin!(operation);
    let mut completed = None;
    loop {
        let mut service = std::pin::pin!(observer.service_liveness(Duration::from_millis(50)));
        let serviced = std::future::poll_fn(|context| {
            if completed.is_none()
                && let std::task::Poll::Ready(result) = operation.as_mut().poll(context)
            {
                completed = Some(result);
            }
            // Finish an in-progress frame even when the operation completed;
            // cancelling service_liveness mid-frame would corrupt this socket.
            service.as_mut().poll(context)
        })
        .await;
        if let Err(error) = serviced {
            // In particular, let peer cleanup finish if the caster connection
            // failed. Do not cancel a logout/return operation with its observer.
            if let Some(result) = completed.take() {
                result?;
            } else {
                operation.await?;
            }
            return Err(error.into());
        }
        if let Some(result) = completed.take() {
            return result;
        }
    }
}

async fn player_logout(client: &mut DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(75);
    loop {
        let reply = client.session_mut().logout().await?;
        if reply.result == Some(LogoutResult::Accepted) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "player logout still refused after combat wait: {:?}",
                reply.result
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(250)).await?;
    }
}

fn rune_quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("native inventory absent")?
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == reference)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}
fn rune_handle(
    client: &DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<oteryn_protocol_oteryn::item_view::ItemHandle> {
    client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == reference)
        })
        .map(|entry| entry.handle)
        .ok_or_else(|| format!("no carried rune handle for definition {reference}").into())
}

// Content selects a real seller and navigation target. Neither source data nor an
// accepted trade receipt proves acquisition: require the subsequent inventory frame.
async fn acquire_rune(
    client: &mut DevClientSession,
    world: &BundleWorld,
    id: u32,
) -> TestResult<(std::num::NonZeroU32, u64)> {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::npc_service as wire;
    let index = provisioning::item_index()?;
    let reference = provisioning::item_reference(&index, id)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let trades: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let item_key = format!("oteryn:item.tibia.i{id}");
    let sellers: BTreeSet<_> = trades["records"]
        .as_array()
        .ok_or("trade records absent")?
        .iter()
        .filter_map(|record| {
            let declaration = &record["declaration"];
            declaration["offers"]
                .as_array()?
                .iter()
                .any(|offer| {
                    offer["direction"] == "SellToPlayer" && offer["item"]["key"] == item_key
                })
                .then(|| declaration["identity"]["key"].as_str())
                .flatten()
                .and_then(|key| key.strip_prefix("oteryn:service.trade."))
                .map(|name| format!("oteryn:npc.{name}"))
        })
        .collect();
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let here = tile(client.world_spatial().actor_position)?;
    let selected = metadata
        .npcs
        .npcs
        .iter()
        .filter(|npc| sellers.contains(&npc.key))
        .filter_map(|npc| {
            route(world, here, |at| {
                npc.placements
                    .iter()
                    .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
            })
            .map(|path| (npc, path))
        })
        .min_by_key(|(_, path)| path.len());
    let Some((npc, path)) = selected else {
        let balances = runtime::balances(client.session_mut()).await?;
        return Err(format!("rune {id} acquisition blocked: no player-walk route within the admitted town window to a compiled-world seller; eligible_sellers={sellers:?}; observed_balances={balances:?}; carried_quantity={}; no item mint or remote vendor interaction; source:crates/gameplay/dispatch/src/gameplay_transport/npc_trade_command.rs:1", rune_quantity(client, reference)?).into());
    };
    walk(client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|p| {
                        entity.position.x == i32::from(p.x)
                            && entity.position.y == i32::from(p.y)
                            && entity.position.floor == -i16::from(p.floor)
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or("rune vendor not visible at compiled placement")?;
    for text in ["hi", "trade"] {
        let reply = runtime::talk(
            client.session_mut(),
            &wire::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        if reply != wire::NpcIntentDisposition::Ok {
            return Err(format!("rune {id} vendor {} {text} refused: {reply:?}", npc.key).into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = wire::decode_npc_trade_window(&delta.payload)
        .map_err(|error| format!("rune shop decode: {error:?}"))?;
    let (offer_index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == wire::NpcTradeSide::Buy && offer.item_definition_ref == reference.get()
        })
        .ok_or_else(|| format!("rune {id} not offered by live vendor {}", npc.key))?;
    let before = rune_quantity(client, reference)?;
    let balances = runtime::balances(client.session_mut()).await?;
    client.take_events();
    let reply = runtime::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("rune shop closed")?.1,
            offer_index: u32::try_from(offer_index)?,
            side: offer.side,
            quantity: 2,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != wire::NpcIntentDisposition::Ok {
        return Err(format!("rune {id} player purchase refused by {}: {reply:?}; quantity=2 unit_price={} observed_balances={balances:?} carried_quantity={before}; no earned funds or rune were granted by this journey; source:crates/gameplay/dispatch/src/gameplay_transport/npc_trade_command.rs:1", npc.key, offer.unit_price).into());
    }
    let expected = before
        .checked_add(u64::from(offer.count) * 2)
        .ok_or("rune quantity overflow")?;
    let sequence = inventory_quantity(client, reference, expected).await?;
    Ok((reference, sequence))
}

async fn inventory_quantity(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
    expected: u64,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event
                && delta
                    .value
                    .entries
                    .iter()
                    .filter(|entry| entry.item_definition_ref == reference)
                    .map(|entry| u64::from(entry.count.get()))
                    .sum::<u64>()
                    == expected
            {
                return Ok(delta.server_sequence);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("rune inventory frame missing: definition={reference} expected={expected} observed={}", rune_quantity(client, reference)?).into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn rune_use(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
    creature: Option<oteryn_dev_client::EntityRef>,
    position: Option<ActorPosition>,
) -> TestResult<(oteryn_protocol_oteryn::world_object::UseDisposition, u64)> {
    use oteryn_protocol_oteryn::{actor_spell::SpellTargetPosition, world_object as wire};
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_RUNE_USE_V1)
    {
        return Err("RUNE_USE_V1 not negotiated on admitted native client".into());
    }
    let payload = wire::encode_item_use_intent(&wire::UseIntent {
        target: wire::UseTarget::Item(rune_handle(client, reference)?),
        use_with: creature,
        use_with_map_item: None,
        use_with_position: position.map(|at| SpellTargetPosition {
            x: at.x,
            y: at.y,
            floor: at.floor,
        }),
    })
    .map_err(|error| format!("rune USE encode: {error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let (payload, _) = oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&reply.payload)
        .map_err(|error| format!("rune refusal decode: {error:?}"))?;
    let disposition =
        wire::decode_use_result(payload).map_err(|error| format!("rune USE decode: {error:?}"))?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "native rune command status={:?} disposition={disposition:?}",
            reply.status
        )
        .into());
    }
    Ok((disposition, reply.server_sequence))
}

async fn illegal_floor(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<u64> {
    let before = client
        .inventory()
        .cloned()
        .ok_or("rune refusal inventory absent")?;
    let vitals = client
        .actor_vitals()
        .copied()
        .ok_or("rune refusal vitals absent")?;
    let mut wrong = client.world_spatial().actor_position;
    wrong.floor = if wrong.floor == -15 {
        -14
    } else {
        wrong.floor - 1
    };
    let (reply, sequence) = rune_use(client, reference, None, Some(wrong)).await?;
    if reply != oteryn_protocol_oteryn::world_object::UseDisposition::TargetIllegal {
        return Err(format!("cross-floor rune expected TargetIllegal, observed {reply:?}").into());
    }
    // A later native read is an ordering barrier; the final re-admission also
    // checks that neither refusal nor successful use resurrects consumed runes.
    level(client).await?;
    if client.inventory() != Some(&before) || client.actor_vitals().copied() != Some(vitals) {
        return Err("cross-floor refusal changed carried items or vitals".into());
    }
    Ok(sequence)
}

async fn cast_rune(
    client: &mut DevClientSession,
    spell: &crate::spell::SpellDefinition,
    reference: std::num::NonZeroU32,
    creature: Option<oteryn_dev_client::EntityRef>,
    position: Option<ActorPosition>,
) -> TestResult<(u64, u64)> {
    let before = rune_quantity(client, reference)?;
    let vitals = client
        .actor_vitals()
        .copied()
        .ok_or("rune cast vitals absent")?;
    let player_level = level(client).await?;
    let crate::spell::ManaCost::Fixed(mana) = spell.mana else {
        return Err("rune cast requires fixed authored mana".into());
    };
    client.take_events();
    let (reply, sequence) = rune_use(client, reference, creature, position).await?;
    if reply != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("rune {} player USE refused: {reply:?}; observed_level={player_level} required_level={} observed_mana={} required_mana={mana} required_vocations={:?} premium_required={}; current player eligibility/PvP legality was not bypassed; source:crates/gameplay/dispatch/src/gameplay_transport/rune_use_bridge.rs:231", spell.key, spell.level, vitals.mana, spell.vocations, spell.premium).into());
    }
    let remaining = before
        .checked_sub(1)
        .ok_or("cast consumed an absent rune")?;
    // Preserve map deltas for the barrier caller while awaiting the debit frame.
    // The effect frame is checked by that caller before this payment check.
    Ok((sequence, remaining))
}

async fn paid_rune(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
    remaining: u64,
    before: oteryn_protocol_oteryn::actor_spell::ActorVitals,
    spell: &crate::spell::SpellDefinition,
) -> TestResult<u64> {
    let crate::spell::ManaCost::Fixed(mana) = spell.mana else {
        return Err("rune payment is not fixed".into());
    };
    // Inventory deltas may already have been consumed by the effect observer;
    // query a later native frame, then compare the applied inventory and vitals.
    level(client).await?;
    if rune_quantity(client, reference)? != remaining
        || client.actor_vitals().is_none_or(|after| {
            before.mana.checked_sub(mana) != Some(after.mana)
                || before.soul.checked_sub(spell.soul) != Some(after.soul)
        })
    {
        return Err(
            "rune cast lacks exact one-item and mana/soul debit in applied server frames".into(),
        );
    }
    Ok(client.last_server_sequence())
}

async fn rune_rejoin(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    tag: u8,
    expected: &[(std::num::NonZeroU32, u64)],
) -> TestResult<u64> {
    let previous = client.join_snapshot().game_session_id;
    let position = client.world_spatial().actor_position;
    player_logout(client).await?;
    *client = connect_world(clients, tag).await?;
    if previous == client.join_snapshot().game_session_id
        || client.world_spatial().actor_position != position
    {
        return Err("rune persistence did not use a fresh session at retained position".into());
    }
    for &(reference, quantity) in expected {
        if rune_quantity(client, reference)? != quantity {
            return Err(format!("fresh admission rune custody mismatch: definition={reference} expected={quantity} observed={}", rune_quantity(client, reference)?).into());
        }
    }
    Ok(client.last_server_sequence())
}

// Three equal-ground tiles keep the two observed step timestamps inside the
// six-second paralysis lifetime. The caster stands off the measurement path.
fn rune_corridor(world: &BundleWorld, from: TilePos) -> Option<Vec<TilePos>> {
    for direction in [
        StepDirection::North,
        StepDirection::East,
        StepDirection::South,
        StepDirection::West,
    ] {
        let mut cells = vec![from];
        let mut at = from;
        for _ in 0..2 {
            let Some(next) = destination(world, at, direction) else {
                break;
            };
            if next.floor != from.floor
                || world.ground_speed(next) != world.ground_speed(from)
                || world
                    .base()
                    .tile(next.x, next.y, next.floor)
                    .is_none_or(|tile| tile.protection_zone())
            {
                break;
            }
            cells.push(next);
            at = next;
        }
        if cells.len() == 3
            && world
                .base()
                .tile(from.x, from.y, from.floor)
                .is_some_and(|tile| !tile.protection_zone())
        {
            return Some(cells);
        }
    }
    None
}
fn actor_position(at: TilePos) -> ActorPosition {
    ActorPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    }
}

async fn paralyze(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let book = native_spell_book()?;
    let spell = book
        .rune(3165)
        .ok_or("C16b Paralyze rune not in pinned spell book")?;
    let (reference, sequence) = acquire_rune(client, world, 3165).await?;
    row.pass("acquire-paralyze", sequence);
    row.pass(
        "illegal-floor-no-cost",
        illegal_floor(client, reference).await?,
    );
    // Character 0 is still an ordinary player here: this mount precedes all
    // profile provisioning. No PvP policy, war, level or vocation is granted.
    let mut peer = connect_character(clients, 0xb1, 0).await?;
    let result: Result<TestResult, _> = tokio::time::timeout(Duration::from_secs(140), async {
        let path = route(world, tile(peer.world_spatial().actor_position)?, |at| rune_corridor(world, at).is_some())
            .ok_or("C16b no reachable non-PZ equal-ground target corridor")?;
        with_observer_liveness(client, walk(&mut peer, world, &path)).await?;
        let cells = rune_corridor(world, tile(peer.world_spatial().actor_position)?)
            .ok_or("C16b target corridor vanished")?;
        let caster_path = route(world, tile(client.world_spatial().actor_position)?, |at|
            at.floor == cells[0].floor && !cells.contains(&at)
                && at.x.abs_diff(cells[0].x).max(at.y.abs_diff(cells[0].y)) <= 3
                && world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| !tile.protection_zone()))
            .ok_or("C16b caster cannot approach the independent target")?;
        with_observer_liveness(&mut peer, walk(client, world, &caster_path)).await?;
        let target_level = level(&mut peer).await?;
        let ground = world.ground_speed(cells[0]);
        let baseline = with_observer_liveness(client, cadence_leg(&mut peer, &cells)).await?;
        if !same_cadence(baseline, expected_cadence(None, target_level, ground)?) {
            return Err("C16b target baseline does not match production movement speed".into());
        }
        let back = route(world, tile(peer.world_spatial().actor_position)?, |at| at == cells[0])
            .ok_or("C16b target cannot return to speed trial start")?;
        with_observer_liveness(client, walk(&mut peer, world, &back)).await?;
        row.pass("target-baseline", peer.last_server_sequence());
        client.service_liveness(Duration::from_millis(50)).await?;
        let target = peer.world_entities().and_then(|entities|
            entities.iter().find(|entity| &entity.entity.identity == entities.own_identity()))
            .map(|entity| entity.entity).ok_or("C16b target own native entity absent")?;
        if client.world_entities().is_none_or(|entities| !entities.iter().any(|entity| entity.entity == target)) {
            return Err("C16b independent target generation is not visible to caster".into());
        }
        let vitals = client.actor_vitals().copied().ok_or("C16b caster vitals absent")?;
        let started = tokio::time::Instant::now();
        let (_, remaining) = cast_rune(client, spell, reference, Some(target), None).await?;
        // This is the server's automatic walking path. Both intermediate own
        // position deltas are required; elapsed client sleeps are not evidence.
        let slowed = with_observer_liveness(client, cadence_leg(&mut peer, &cells)).await?;
        let expected = oteryn_gameplay_characters::speed::StepSpeedTable::embedded()
            .and_then(|table| table.step_duration(40, ground)).ok_or("C16b speed-40 table duration absent")?;
        if tokio::time::Instant::now() >= started + Duration::from_secs(6)
            || !same_cadence(slowed, expected) || !faster(baseline, slowed)
        {
            return Err(format!("C16b no active speed-40 paralysis frames: baseline={baseline:?} observed={slowed:?} expected={expected:?}").into());
        }
        row.pass("target-paralyzed-speed", peer.last_server_sequence());
        row.pass("paralyze-cast-and-debit", paid_rune(client, reference, remaining, vitals, spell).await?);
        while tokio::time::Instant::now() < started + Duration::from_millis(6200) {
            peer.service_liveness(Duration::from_millis(25)).await?;
            client.service_liveness(Duration::from_millis(25)).await?;
        }
        let reverse: Vec<_> = cells.iter().rev().copied().collect();
        if !same_cadence(with_observer_liveness(client, cadence_leg(&mut peer, &reverse)).await?, baseline) {
            return Err("C16b paralysis failed to expire to the original target speed".into());
        }
        row.pass("target-expiry", peer.last_server_sequence());
        row.pass("fresh-admission-rune-custody", with_observer_liveness(&mut peer,
            rune_rejoin(client, clients, 0xb2, &[(reference, remaining)])).await?);
        Ok(())
    }).await;
    // The second character is restored/closed even when a positive cast fails.
    let cleanup = with_observer_liveness(client, rune_cleanup(&mut peer, world)).await;
    if let Err(error) = &cleanup {
        evidence(&format!("E2EF5_GAP C16b peer cleanup: {error}"));
    }
    result.map_err(|_| "C16b independent-target journey exceeded 140s")??;
    cleanup
}

fn barrier_parameters(spell: &crate::spell::SpellDefinition) -> TestResult<(u64, u64, [u32; 2])> {
    let crate::spell::Execution::NativeProfile(profile) = &spell.execution else {
        return Err("C16a native barrier profile absent".into());
    };
    let effect = &profile.dependencies()["effects"][0];
    let minimum = effect["duration_range_ms"]["minimum"]
        .as_u64()
        .ok_or("barrier minimum duration absent")?;
    let maximum = effect["duration_range_ms"]["maximum"]
        .as_u64()
        .ok_or("barrier maximum duration absent")?;
    if minimum == 0 || maximum < minimum || maximum > 60_000 {
        return Err("C16a unsupported barrier lifetime bounds".into());
    }
    let id = |field: &str| -> TestResult<u32> {
        Ok(effect[field]["key"]
            .as_str()
            .and_then(|key| key.strip_prefix("candidate:item/"))
            .ok_or("barrier exact created item absent")?
            .parse()?)
    };
    Ok((
        minimum,
        maximum,
        [id("created_item")?, id("pvp_safe_item")?],
    ))
}

// Only a pushed map frame for this exact tile can satisfy placement or expiry.
async fn barrier_frame(
    client: &mut DevClientSession,
    at: ActorPosition,
    references: &[std::num::NonZeroU32; 2],
    present: bool,
    after: u64,
    deadline: tokio::time::Instant,
) -> TestResult<(u64, Option<std::num::NonZeroU32>)> {
    use oteryn_protocol_oteryn::world_map::MapDefinition;
    loop {
        for event in client.take_events() {
            if let SessionEvent::WorldMap(delta) = event
                && delta.server_sequence > after
                && let Some(tile) = delta.value.tiles.iter().find(|tile| tile.position == at)
            {
                let found = tile.items.iter().find_map(|item| match item.definition {
                    MapDefinition::Item(reference) if references.contains(&reference) => {
                        Some(reference)
                    }
                    _ => None,
                });
                if found.is_some() == present {
                    return Ok((delta.server_sequence, found));
                }
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "C16a missing exact barrier map frame: tile={at:?} present={present}"
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn barriers(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let book = native_spell_book()?;
    let index = provisioning::item_index()?;
    let mut expected = Vec::new();
    for (id, acquire, refusal, cast, map, block, expiry) in [
        (
            3180,
            "acquire-magic-wall",
            "wall-illegal-floor-no-cost",
            "wall-cast-and-debit",
            "wall-map-item",
            "wall-blocked-traversal",
            "wall-expiry-and-traversal",
        ),
        (
            3156,
            "acquire-wild-growth",
            "growth-illegal-floor-no-cost",
            "growth-cast-and-debit",
            "growth-map-item",
            "growth-blocked-traversal",
            "growth-expiry-and-traversal",
        ),
    ] {
        let spell = book
            .rune(id)
            .ok_or_else(|| format!("C16a rune {id} absent from pinned book"))?;
        let (minimum, maximum, ids) = barrier_parameters(spell)?;
        let references = [
            provisioning::item_reference(&index, ids[0])?,
            provisioning::item_reference(&index, ids[1])?,
        ];
        let (reference, sequence) = acquire_rune(client, world, id).await?;
        row.pass(acquire, sequence);
        row.pass(refusal, illegal_floor(client, reference).await?);
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
            rune_corridor(world, at).is_some()
        })
        .ok_or("C16a no reachable non-PZ barrier traversal site")?;
        walk(client, world, &path).await?;
        let from = tile(client.world_spatial().actor_position)?;
        let cells = rune_corridor(world, from).ok_or("C16a barrier approach unavailable")?;
        let target = actor_position(cells[1]);
        let direction = [
            StepDirection::North,
            StepDirection::East,
            StepDirection::South,
            StepDirection::West,
        ]
        .into_iter()
        .find(|&direction| destination(world, from, direction) == Some(cells[1]))
        .ok_or("C16a barrier target is not adjacent")?;
        // First prove the target is traversable without the wall, and return.
        walk(client, world, &[direction]).await?;
        let back =
            route(world, cells[1], |at| at == from).ok_or("C16a approach return unavailable")?;
        walk(client, world, &back).await?;
        let vitals = client
            .actor_vitals()
            .copied()
            .ok_or("C16a caster vitals absent")?;
        let started = tokio::time::Instant::now();
        let (sequence, remaining) = cast_rune(client, spell, reference, None, Some(target)).await?;
        let (placed, definition) = barrier_frame(
            client,
            target,
            &references,
            true,
            sequence,
            started + Duration::from_secs(5),
        )
        .await?;
        row.pass(map, placed);
        row.pass(
            cast,
            paid_rune(client, reference, remaining, vitals, spell).await?,
        );
        if definition == Some(references[1]) {
            return Err(format!("C16a rune {id} produced Optional-PvP safe item {}; current world permits traversal/collapse, so this row's blocked-traversal requirement cannot be asserted without changing world policy; source:crates/gameplay/dispatch/src/gameplay_transport/native_world_item_cast.rs:716", ids[1]).into());
        }
        let blocked = client.step_retrying(direction).await?;
        if blocked.disposition != StepDisposition::Blocked
            || client.world_spatial().actor_position != actor_position(from)
        {
            return Err(format!(
                "C16a rune {id} normal barrier did not block traversal: {blocked:?}"
            )
            .into());
        }
        row.pass(block, blocked.result_server_sequence);
        let (expired, _) = barrier_frame(
            client,
            target,
            &references,
            false,
            placed,
            started + Duration::from_millis(maximum) + Duration::from_secs(5),
        )
        .await?;
        if tokio::time::Instant::now() + Duration::from_millis(100)
            < started + Duration::from_millis(minimum)
        {
            return Err("C16a barrier disappeared before its minimum authored lifetime".into());
        }
        walk(client, world, &[direction]).await?;
        row.pass(expiry, expired);
        expected.push((reference, remaining));
    }
    row.pass(
        "fresh-admission-barrier-custody",
        rune_rejoin(client, clients, 0xb5, &expected).await?,
    );
    Ok(())
}

#[cfg(test)]
mod rune_tests {
    use super::*;
    #[test]
    fn paralyze_speed_trial_matches_the_selected_condition_lifetime_and_formula() -> TestResult {
        let book = native_spell_book()?;
        let spell = book.rune(3165).ok_or("selected Paralyze rune absent")?;
        assert_eq!(spell.mana, crate::spell::ManaCost::Fixed(1400));
        // Paralyze is compiled through the ordinary condition owner, unlike
        // the NativeProfile barrier callbacks. Its exact authored AST remains.
        let profile = spell
            .authored
            .as_ref()
            .ok_or("selected Paralyze profile absent")?;
        let effect = profile
            .dependencies
            .effects
            .first()
            .ok_or("Paralyze effect absent")?;
        assert_eq!(effect.duration_ms, Some(6000));
        assert_eq!(
            effect
                .condition
                .as_ref()
                .ok_or("Paralyze condition absent")?
                .kind,
            "paralyze"
        );
        let speed = profile
            .dependencies
            .formulas
            .first()
            .and_then(|formula| formula.speed.as_ref())
            .ok_or("Paralyze speed formula absent")?;
        assert_eq!(speed.minimum_multiplier.numerator, 0);
        assert_eq!(speed.maximum_multiplier.numerator, 0);
        assert_eq!(speed.minimum_offset, 40);
        assert_eq!(speed.maximum_offset, 40);
        Ok(())
    }
    #[test]
    fn barrier_lifetimes_and_item_pairs_come_from_selected_rune_profiles() -> TestResult {
        let book = native_spell_book()?;
        for (id, minimum, maximum, items) in [
            (3180, 16_000, 24_000, [2128, 10181]),
            (3156, 30_000, 60_000, [2130, 10182]),
        ] {
            assert_eq!(
                barrier_parameters(book.rune(id).ok_or("selected barrier rune absent")?)?,
                (minimum, maximum, items)
            );
        }
        assert!(barrier_parameters(book.rune(3165).ok_or("Paralyze absent")?).is_err());
        Ok(())
    }
    #[test]
    fn rune_rows_cannot_pass_with_only_acquisition_or_effect_receipts() {
        for (id, actions) in [("C16b", PARALYZE_ACTIONS), ("C16a", BARRIER_ACTIONS)] {
            let mut row = Row::new(id, actions);
            for (index, &action) in actions.iter().enumerate() {
                assert!(!row.complete());
                row.pass(action, index as u64 + 1);
            }
            assert!(row.complete());
        }
    }
}

// Continuation pass 2: independent complete combat rows, before fixture provisioning.
const AIM_ACTIONS: &[&str] = &[
    "player-spell-access",
    "facing-away-misses",
    "selected-target-hit",
    "facing-forward-hit",
    "missing-target-no-cost",
    "fresh-admission-costs",
    "fresh-admission-spell-access",
];
const CHALLENGE_ACTIONS: &[&str] = &[
    "player-challenge-access",
    "competing-player-attacked",
    "challenge-cast-and-cost",
    "cooldown-no-cost",
    "forced-caster-attacked",
    "challenge-holds",
    "expiry-normal-targeting",
    "fresh-admission-costs",
    "fresh-admission-spell-access",
];

async fn combat_rows(clients: &SeamClients<'_>, world: &BundleWorld) {
    for (id, actions, tag) in [
        ("C18a", AIM_ACTIONS, 0xc0),
        ("C17d", CHALLENGE_ACTIONS, 0xc4),
    ] {
        let mut row = Row::new(id, actions);
        let result: TestResult = async {
            let mut client = connect_character(clients, tag, usize::from(id != "C17d")).await?;
            let result = tokio::time::timeout(Duration::from_secs(240), async {
                if id == "C18a" {
                    directional_aim(&mut row, &mut client, clients, world).await
                } else {
                    challenge(&mut row, &mut client, clients, world).await
                }
            })
            .await;
            let cleanup = rune_cleanup(&mut client, world).await;
            if let Err(error) = &cleanup {
                evidence(&format!("E2EF5_GAP {id} cleanup: {error}"));
            }
            result
                .map_err(|_| format!("{id} 240s journey deadline; required actions unproven"))??;
            cleanup
        }
        .await;
        row.finish(result);
    }
}

fn player_spell_index(
    book: &crate::spell::SpellBook,
    words: &str,
) -> TestResult<std::num::NonZeroU32> {
    let spell = book.spoken(words).ok_or("player spell words absent")?.spell;
    (1..=book.source_len())
        .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
        .find(|i| book.indexed(*i).is_some_and(|entry| entry.key == spell.key))
        .ok_or_else(|| format!("{words} has no served spell index").into())
}

async fn spell_ready(
    client: &mut DevClientSession,
    spell: &crate::spell::SpellDefinition,
) -> TestResult {
    let wait = spell
        .groups
        .iter()
        .map(|group| group.cooldown_micros)
        .max()
        .unwrap_or(0)
        .max(spell.cooldown_micros);
    let until = tokio::time::Instant::now() + Duration::from_micros(wait.saturating_add(200_000));
    while tokio::time::Instant::now() < until {
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    Ok(())
}

// Only the reply and its validated ACTOR_VITALS delta can prove payment.
async fn player_spell(
    client: &mut DevClientSession,
    book: &crate::spell::SpellBook,
    words: &str,
    target: oteryn_protocol_oteryn::actor_spell::SpellTarget,
    aim: bool,
) -> TestResult<oteryn_dev_client::CastOutcome> {
    use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
    let spell = book.spoken(words).ok_or("player spell absent")?.spell;
    if spell.learning_required {
        return Err(format!("{words}: compiled spell requires player teacher acquisition; no teacher route implemented in this journey").into());
    }
    let observed_level = level(client).await?;
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("caster vitals absent")?;
    let result = client
        .cast_spell(player_spell_index(book, words)?, target, aim)
        .await?;
    let crate::spell::ManaCost::Fixed(cost) = spell.mana else {
        return Err("combat journey requires the pinned fixed mana cost".into());
    };
    if result.status != CommandStatus::Accepted || result.disposition != SpellCastDisposition::Cast
    {
        return Err(format!("{words} native player cast refused: {:?}; observed_level={observed_level} required_level={} observed_mana={} required_mana={cost} required_vocations={:?} premium_required={}; ordinary provisioned character has no fixture progression; source:crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast.rs:2520", result.disposition, spell.level, before.mana, spell.vocations, spell.premium).into());
    }
    if result.actor_vitals_delta.as_ref().is_none_or(|delta| {
        delta.server_sequence == 0
            || Some(delta.value.mana) != before.mana.checked_sub(cost)
            || Some(delta.value.soul) != before.soul.checked_sub(spell.soul)
    }) {
        return Err(format!("{words}: cast reply lacks exact mana/soul payment frame").into());
    }
    Ok(result)
}

async fn combat_access(
    client: &mut DevClientSession,
    world: &BundleWorld,
    book: &crate::spell::SpellBook,
    words: &str,
) -> TestResult<u64> {
    // Normal speech/indexed spells have learning_required=false in the served
    // book. Walk out of PZ; ask the actual owner to admit this player's cast.
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    })
    .ok_or("no ordinary walk route out of PZ for combat spell access")?;
    walk(client, world, &path).await?;
    let spell = book.spoken(words).ok_or("combat spell absent")?.spell;
    spell_ready(client, spell).await?;
    use oteryn_protocol_oteryn::actor_spell::SpellTarget;
    match player_spell(client, book, words, SpellTarget::None, false).await {
        Ok(cast) => Ok(cast.result_server_sequence),
        Err(refusal) => {
            // Try the actual player first-vocation route rather than assuming
            // that a refused starter can never become eligible. Its NPC must
            // be in this compiled world and reachable through ordinary STEPs.
            let vocation = if words == "exeta res" {
                "knight"
            } else {
                "sorcerer"
            };
            if let Err(blocker) = player_first_vocation(client, world, vocation).await {
                return Err(format!("{refusal}; player prerequisite route: {blocker}").into());
            }
            let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
                world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|tile| !tile.protection_zone())
            })
            .ok_or("no walk route from vocation NPC to non-PZ cast tile")?;
            walk(client, world, &path).await?;
            spell_ready(client, spell).await?;
            Ok(player_spell(client, book, words, SpellTarget::None, false)
                .await?
                .result_server_sequence)
        }
    }
}

async fn player_first_vocation(
    client: &mut DevClientSession,
    world: &BundleWorld,
    vocation: &str,
) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::npc_service as wire;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let here = tile(client.world_spatial().actor_position)?;
    let observed_level = level(client).await?;
    let npc = metadata.npcs.npcs.iter().find(|npc| npc.key == "oteryn:npc.oressa")
        .ok_or_else(|| format!("first vocation {vocation}: Oressa absent from compiled world; observed_level={observed_level}; source:crates/gameplay/content/src/content/npc_catalogue.rs:5"))?;
    let path = route(world, here, |at| npc.placements.iter().any(|p|
        p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3))
        .ok_or_else(|| format!("first vocation {vocation}: no ordinary player route to Oressa within the admitted town window; observed_level={observed_level}; source:crates/gameplay/content/src/content/npc_catalogue.rs:5"))?;
    walk(client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|p| {
                        entry.position.x == i32::from(p.x)
                            && entry.position.y == i32::from(p.y)
                            && entry.position.floor == -i16::from(p.floor)
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or("Oressa not visible at compiled placement")?;
    for text in ["hi", vocation, "yes"] {
        client.take_events();
        let disposition = runtime::talk(
            client.session_mut(),
            &wire::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        let until = tokio::time::Instant::now() + Duration::from_secs(3);
        let mut lines = Vec::new();
        loop {
            for event in client.take_events() {
                if let SessionEvent::NpcConversation(delta) = event {
                    lines.extend(delta.value.lines);
                }
            }
            if !lines.is_empty() {
                break;
            }
            if tokio::time::Instant::now() >= until {
                break;
            }
            client.service_liveness(Duration::from_millis(20)).await?;
        }
        if disposition != wire::NpcIntentDisposition::Ok {
            return Err(format!("Oressa {text} refused: {disposition:?}; observed_level={observed_level}; native_conversation={lines:?}; source:crates/gameplay/dispatch/src/gameplay_transport/vocation_choice.rs:347").into());
        }
        if text == "yes"
            && !lines
                .iter()
                .any(|line| line == &format!("Your vocation is now {vocation}."))
        {
            return Err(format!("Oressa confirmation has no committed vocation frame: {lines:?}; observed_level={observed_level}").into());
        }
    }
    Ok(())
}

fn own_entity(client: &DevClientSession) -> TestResult<oteryn_dev_client::EntityRef> {
    let entities = client.world_entities().ok_or("combat entities absent")?;
    entities
        .iter()
        .find(|entry| &entry.entity.identity == entities.own_identity())
        .map(|entry| entry.entity)
        .ok_or_else(|| "own combat entity absent".into())
}

fn creature(
    client: &DevClientSession,
    entity: oteryn_dev_client::EntityRef,
) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    client
        .world_entities()
        .and_then(|entities| entities.iter().find(|entry| entry.entity == entity))
        .copied()
        .ok_or_else(|| "tracked creature generation left visibility".into())
}

fn hp(entry: &oteryn_dev_client::WorldSpatialEntity) -> TestResult<u8> {
    match entry.detail {
        oteryn_dev_client::EntityDetail::Actor { health_percent, .. } => Ok(health_percent),
        _ => Err("tracked creature has no HP observation".into()),
    }
}

async fn rat(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    travel_to_spawn(client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let here = client.world_spatial().actor_position;
    let mut candidates = client
        .world_entities()
        .ok_or("spawn entities absent")?
        .iter()
        .filter(|entry| {
            entry.kind == oteryn_dev_client::EntityKind::Creature
                && entry.position.floor == here.floor
                && entry
                    .position
                    .x
                    .abs_diff(here.x)
                    .max(entry.position.y.abs_diff(here.y))
                    <= 1
        });
    let target = candidates
        .next()
        .copied()
        .ok_or("rat spawn has no adjacent live monster")?;
    if candidates.next().is_some() {
        return Err(
            "combat journey requires one isolated adjacent monster, multiple visible in range"
                .into(),
        );
    }
    hp(&target)?;
    Ok(target)
}

fn attributed_damage(
    kind: &oteryn_protocol_oteryn::presentation::PresentationEventKind,
    source: oteryn_dev_client::EntityRef,
    victim: oteryn_dev_client::EntityRef,
) -> bool {
    use oteryn_protocol_oteryn::presentation::{PresentationEventKind, PresentationValueKind};
    matches!(kind, PresentationEventKind::ValueText { kind, target, attacker: Some(attacker), primary, .. }
        if *target == victim && *attacker == source && primary.value > 0
            && matches!(kind, PresentationValueKind::DamageDealt | PresentationValueKind::DamageReceived | PresentationValueKind::DamageOthers))
}

fn damage_frames(
    client: &mut DevClientSession,
    source: oteryn_dev_client::EntityRef,
    victim: oteryn_dev_client::EntityRef,
) -> u64 {
    client
        .take_events()
        .into_iter()
        .filter_map(|event| match event {
            SessionEvent::Presentation(delta)
                if delta
                    .value
                    .iter()
                    .any(|event| attributed_damage(&event.kind, source, victim)) =>
            {
                Some(delta.server_sequence)
            }
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

async fn select_monster(
    client: &mut DevClientSession,
    target: Option<oteryn_dev_client::EntityRef>,
) -> TestResult {
    use oteryn_protocol_oteryn::attack::{
        AttackIntentDisposition, ChaseMode, FightMode, FightModes,
    };
    let modes = FightModes {
        fight_mode: FightMode::Defensive,
        chase: ChaseMode::Stand,
        secure: true,
    };
    if client
        .session_mut()
        .set_fight_modes(&modes)
        .await?
        .disposition
        != AttackIntentDisposition::Ok
        || client
            .session_mut()
            .attack_target(target.as_ref())
            .await?
            .disposition
            != AttackIntentDisposition::Ok
    {
        return Err("ordinary attack target/modes refused".into());
    }
    client.service_liveness(Duration::from_millis(25)).await?;
    if client
        .session_mut()
        .combat_state()
        .is_none_or(|state| state.target != target || state.modes != modes)
    {
        return Err("attack state frame did not retain the exact target and stand mode".into());
    }
    Ok(())
}

// The ordinary target-move TURN command changes facing without introducing
// an extra movement/timing difference between the selected-target comparisons.
async fn orient(
    client: &mut DevClientSession,
    world: &BundleWorld,
    target: oteryn_dev_client::EntityRef,
    toward: bool,
) -> TestResult<ActorPosition> {
    use oteryn_protocol_oteryn::target_move as wire;
    let target_at = creature(client, target)?.position;
    let from = tile(client.world_spatial().actor_position)?;
    let here = actor_position(from);
    if here
        .x
        .abs_diff(target_at.x)
        .max(here.y.abs_diff(target_at.y))
        <= 1
    {
        let direction = [
            StepDirection::North,
            StepDirection::East,
            StepDirection::South,
            StepDirection::West,
        ]
        .into_iter()
        .find(|direction| {
            destination(world, from, *direction).is_some_and(|next| {
                let next = actor_position(next);
                next.floor == here.floor
                    && next
                        .x
                        .abs_diff(target_at.x)
                        .max(next.y.abs_diff(target_at.y))
                        == 2
            })
        })
        .ok_or("no ordinary retreat STEP to isolate spell from melee damage")?;
        walk(client, world, &[direction]).await?;
    }
    let here = client.world_spatial().actor_position;
    let dx = target_at.x - here.x;
    let dy = target_at.y - here.y;
    if here.floor != target_at.floor
        || !(2..=3).contains(&dx.abs().max(dy.abs()))
        || (dx != 0 && dy != 0)
    {
        return Err("directional comparison needs an isolated axial target at range 2..3".into());
    }
    let forward = if dx > 0 {
        StepDirection::East
    } else if dx < 0 {
        StepDirection::West
    } else if dy > 0 {
        StepDirection::South
    } else {
        StepDirection::North
    };
    let direction = if toward {
        forward
    } else {
        match forward {
            StepDirection::North => StepDirection::South,
            StepDirection::East => StepDirection::West,
            StepDirection::South => StepDirection::North,
            StepDirection::West => StepDirection::East,
        }
    };
    // TARGET-MOVE-1's oneof field 3 contains the strict STEP direction codec.
    let result = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_TARGET_MOVE_V1,
            wire::COMMAND_TYPE_TARGET_MOVE_INTENT,
            &[26, 2, 8, direction as u8],
        )
        .await?;
    if result.status != CommandStatus::Accepted
        || result.payload != [8, 1]
        || client.world_spatial().actor_position != here
        || creature(client, target)?.position != target_at
    {
        return Err("native TURN refused or changed the directional comparison positions".into());
    }
    let own = own_entity(client)?;
    let until = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        if creature(client, own).is_ok_and(|entry| matches!(entry.detail,
            oteryn_dev_client::EntityDetail::Actor { direction: observed, .. } if observed == direction)) {
            break;
        }
        if tokio::time::Instant::now() >= until {
            return Err("native TURN has no own-actor facing frame".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
    if creature(client, target)?.position != target_at {
        return Err("monster moved while observing native TURN".into());
    }
    Ok(target_at)
}

async fn aim_observation(
    client: &mut DevClientSession,
    target: oteryn_dev_client::EntityRef,
    expected_position: ActorPosition,
    health: u8,
    expect_hit: bool,
    after_sequence: u64,
) -> TestResult<u64> {
    let own = own_entity(client)?;
    let until = tokio::time::Instant::now() + Duration::from_millis(700);
    let mut damage = 0;
    loop {
        for event in client.take_events() {
            match event {
                SessionEvent::Presentation(delta)
                    if delta.server_sequence > after_sequence
                        && delta
                            .value
                            .iter()
                            .any(|event| attributed_damage(&event.kind, own, target)) =>
                {
                    damage = damage.max(delta.server_sequence);
                }
                SessionEvent::WorldSpatialEntities(delta)
                    if delta.value.update.iter().any(|entry| {
                        entry.entity == target && entry.position != expected_position
                    }) =>
                {
                    return Err("monster moved in an intermediate comparison frame".into());
                }
                _ => {}
            }
        }
        if let Ok(entry) = creature(client, target) {
            if entry.position != expected_position {
                return Err(
                    "directional comparison monster moved before outcome observation".into(),
                );
            }
            if !expect_hit && hp(&entry)? != health {
                return Err("facing-away comparison changed monster HP".into());
            }
            if expect_hit && damage > after_sequence && hp(&entry)? < health {
                return Ok(damage);
            }
        } else if expect_hit && damage > after_sequence {
            return Ok(damage); // Lethal hit: exact generation left after attributed damage.
        } else {
            return Err("monster left visibility without the required attributed spell hit".into());
        }
        if tokio::time::Instant::now() >= until {
            if !expect_hit && damage == 0 {
                return Ok(client.last_server_sequence());
            }
            return Err("directional cast has no attributed damage and HP/removal frames".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn directional_aim(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
    let book = native_spell_book()?;
    let words = "exevo infir flam hur";
    let spell = book
        .spoken(words)
        .ok_or("Scorch absent from served book")?
        .spell;
    row.pass(
        "player-spell-access",
        combat_access(client, world, book, words).await?,
    );
    // No equipped weapon: a held attack target must not introduce automatic
    // ranged/melee damage into the spell-only observation interval.
    if client.inventory().is_none_or(|inventory| {
        inventory.equipment.iter().any(|entry| {
            matches!(
                entry.slot,
                oteryn_protocol_oteryn::item_view::EquipmentSlot::RightHand
                    | oteryn_protocol_oteryn::item_view::EquipmentSlot::LeftHand
            )
        })
    }) {
        return Err("C18a requires ordinary unequipped hands to isolate spell damage".into());
    }
    let target = rat(client, world).await?.entity;
    spell_ready(client, spell).await?;
    select_monster(client, None).await?;
    let position = orient(client, world, target, false).await?;
    select_monster(client, Some(target)).await?;
    let health = hp(&creature(client, target)?)?;
    client.take_events();
    let before_cast = client.last_server_sequence();
    let _cast = player_spell(client, book, words, SpellTarget::None, false).await?;
    row.pass(
        "facing-away-misses",
        aim_observation(client, target, position, health, false, before_cast).await?,
    );
    spell_ready(client, spell).await?;
    // Hold the same selected target and wrong facing, changing the cast's
    // explicit target/aim. ACTOR_COMBAT_STATE proves the exact selection.
    let position = orient(client, world, target, false).await?;
    select_monster(client, Some(target)).await?;
    let health = hp(&creature(client, target)?)?;
    client.take_events();
    let before_cast = client.last_server_sequence();
    let _cast = player_spell(client, book, words, SpellTarget::AttackTarget, true).await?;
    let hit = aim_observation(client, target, position, health, true, before_cast).await;
    let stop = select_monster(client, None).await;
    row.pass("selected-target-hit", hit?);
    stop?;
    spell_ready(client, spell).await?;
    // A killed target is replaced through the same real spawn/player route.
    let target = if creature(client, target).is_ok() {
        target
    } else {
        rat(client, world).await?.entity
    };
    let position = orient(client, world, target, true).await?;
    let health = hp(&creature(client, target)?)?;
    client.take_events();
    let before_cast = client.last_server_sequence();
    let _cast = player_spell(client, book, words, SpellTarget::None, false).await?;
    row.pass(
        "facing-forward-hit",
        aim_observation(client, target, position, health, true, before_cast).await?,
    );
    spell_ready(client, spell).await?;
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("aim refusal vitals absent")?;
    let refusal = client
        .cast_spell(
            player_spell_index(book, words)?,
            SpellTarget::AttackTarget,
            true,
        )
        .await?;
    if refusal.status != CommandStatus::Accepted
        || refusal.disposition != SpellCastDisposition::TargetRequired
        || refusal.actor_vitals_delta.is_some()
        || client
            .actor_vitals()
            .is_none_or(|v| v.mana != before.mana || v.soul != before.soul)
    {
        return Err(format!(
            "C18a no-target refusal was not resource-free TargetRequired: {refusal:?}"
        )
        .into());
    }
    row.pass("missing-target-no-cost", refusal.result_server_sequence);
    combat_rejoin(row, client, clients, world, 0xc1).await
}

async fn combat_rejoin(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    tag: u8,
) -> TestResult {
    select_monster(client, None).await?;
    let back = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == TOWN
    })
    .ok_or("combat row cannot return to TOWN for fresh admission")?;
    walk(client, world, &back).await?;
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("combat logout vitals absent")?;
    let previous = client.join_snapshot().game_session_id;
    player_logout(client).await?;
    *client = connect_character(clients, tag, usize::from(row.id != "C17d")).await?;
    if client.join_snapshot().game_session_id == previous
        || tile(client.world_spatial().actor_position)? != TOWN
        || client
            .actor_vitals()
            .is_none_or(|v| v.mana != before.mana || v.soul != before.soul)
    {
        return Err(
            "combat fresh admission did not retain paid mana/soul and TOWN under a new session"
                .into(),
        );
    }
    row.pass("fresh-admission-costs", client.last_server_sequence());
    // In particular, any first vocation obtained from Oressa must survive.
    // Recast directly; reacquiring it here would hide a persistence defect.
    let book = native_spell_book()?;
    let words = if row.id == "C17d" {
        "exeta res"
    } else {
        "exevo infir flam hur"
    };
    let spell = book
        .spoken(words)
        .ok_or("fresh-admission spell absent")?
        .spell;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    })
    .ok_or("fresh admission cannot walk to a non-PZ cast tile")?;
    walk(client, world, &path).await?;
    spell_ready(client, spell).await?;
    let cast = player_spell(
        client,
        book,
        words,
        oteryn_protocol_oteryn::actor_spell::SpellTarget::None,
        false,
    )
    .await?;
    row.pass("fresh-admission-spell-access", cast.result_server_sequence);
    Ok(())
}

async fn monster_hit(
    victim: &mut DevClientSession,
    observer: &mut DevClientSession,
    monster: oteryn_dev_client::EntityRef,
    seconds: u64,
) -> TestResult<u64> {
    let own = own_entity(victim)?;
    let health = victim
        .actor_vitals()
        .ok_or("monster victim vitals absent")?
        .health;
    victim.take_events();
    let until = tokio::time::Instant::now() + Duration::from_secs(seconds);
    let mut hit = 0;
    loop {
        victim.service_liveness(Duration::from_millis(20)).await?;
        observer.service_liveness(Duration::from_millis(20)).await?;
        hit = hit.max(damage_frames(victim, monster, own));
        if hit > 0 && victim.actor_vitals().is_some_and(|v| v.health < health) {
            return Ok(hit);
        }
        if tokio::time::Instant::now() >= until {
            return Err(
                "no exact monster->victim damage presentation plus health frame within deadline"
                    .into(),
            );
        }
    }
}

async fn challenge(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
    let book = native_spell_book()?;
    let words = "exeta res";
    let spell = book
        .spoken(words)
        .ok_or("Challenge absent from served book")?
        .spell;
    row.pass(
        "player-challenge-access",
        combat_access(client, world, book, words).await?,
    );
    let duration = challenge_duration(spell)?;
    let mut peer = connect_character(clients, 0xc5, 1).await?;
    let result = tokio::time::timeout(Duration::from_secs(130), async {
        let monster = with_observer_liveness(client, rat(&mut peer, world))
            .await?
            .entity;
        row.pass(
            "competing-player-attacked",
            monster_hit(&mut peer, client, monster, 15).await?,
        );
        let target_at = creature(&peer, monster)?.position;
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
            let at = actor_position(at);
            at.floor == target_at.floor
                && at != peer.world_spatial().actor_position
                && at.x.abs_diff(target_at.x).max(at.y.abs_diff(target_at.y)) == 1
                && world
                    .base()
                    .tile(
                        u16::try_from(at.x).unwrap_or(0),
                        u16::try_from(at.y).unwrap_or(0),
                        -i8::try_from(at.floor).unwrap_or(0),
                    )
                    .is_some_and(|tile| !tile.protection_zone())
        })
        .ok_or("C17d no player route to Challenge range beside competing victim")?;
        with_observer_liveness(&mut peer, walk(client, world, &path)).await?;
        with_observer_liveness(&mut peer, spell_ready(client, spell)).await?;
        // Prove B is still the victim immediately before the cast, with both
        // players present. No attack, fixture target or AI clock override.
        monster_hit(&mut peer, client, monster, 10).await?;
        if creature(client, monster)?
            .position
            .x
            .abs_diff(client.world_spatial().actor_position.x)
            .max(
                creature(client, monster)?
                    .position
                    .y
                    .abs_diff(client.world_spatial().actor_position.y),
            )
            > 1
        {
            return Err("C17d monster moved out of the authored Challenge area".into());
        }
        client.take_events();
        peer.take_events();
        let started = tokio::time::Instant::now();
        let cast = with_observer_liveness(
            &mut peer,
            player_spell(client, book, words, SpellTarget::None, false),
        )
        .await?;
        row.pass("challenge-cast-and-cost", cast.result_server_sequence);
        let before = client
            .actor_vitals()
            .copied()
            .ok_or("Challenge cooldown vitals absent")?;
        let retry = client
            .cast_spell(player_spell_index(book, words)?, SpellTarget::None, false)
            .await?;
        if retry.status != CommandStatus::Accepted
            || retry.disposition != SpellCastDisposition::CoolingDown
            || retry.actor_vitals_delta.is_some()
            || client
                .actor_vitals()
                .is_none_or(|v| v.mana != before.mana || v.soul != before.soul)
        {
            return Err(
                format!("C17d immediate retry not resource-free CoolingDown: {retry:?}").into(),
            );
        }
        row.pass("cooldown-no-cost", retry.result_server_sequence);
        let hit = monster_hit(client, &mut peer, monster, 4).await?;
        if tokio::time::Instant::now() >= started + duration {
            return Err("C17d caster hit occurred after Challenge expiry".into());
        }
        row.pass("forced-caster-attacked", hit);
        let peer_identity = own_entity(&peer)?;
        let own = own_entity(client)?;
        let mut held = hit;
        // Repeated, attributed attacks while both players remain present prove
        // the forced interval; a successful cast alone proves none of this.
        while tokio::time::Instant::now() < started + duration {
            client.service_liveness(Duration::from_millis(20)).await?;
            peer.service_liveness(Duration::from_millis(20)).await?;
            if damage_frames(&mut peer, monster, peer_identity) != 0 {
                return Err(
                    "C17d challenged monster hit the competing player during its forced interval"
                        .into(),
                );
            }
            held = held.max(damage_frames(client, monster, own));
        }
        if held <= hit {
            return Err("C17d no second attributed attack during the forced interval".into());
        }
        row.pass("challenge-holds", held);
        // After real-clock expiry the same monster must be free to select B
        // while A is STILL an eligible visible target. Walking out of sight
        // would only prove target invalidation, not expiry of Challenge.
        let monster_at = creature(&peer, monster)?.position;
        let retreat = route(world, tile(client.world_spatial().actor_position)?, |at| {
            let at = actor_position(at);
            at.floor == monster_at.floor
                && at.x.abs_diff(monster_at.x).max(at.y.abs_diff(monster_at.y)) == 3
                && tile(at)
                    .ok()
                    .and_then(|p| world.base().tile(p.x, p.y, p.floor))
                    .is_some_and(|tile| !tile.protection_zone())
        })
        .ok_or("C17d no ordinary same-floor retreat within monster perception after expiry")?;
        with_observer_liveness(&mut peer, walk(client, world, &retreat)).await?;
        let expiry = monster_hit(&mut peer, client, monster, 30).await?;
        let monster_at = creature(client, monster)?.position;
        let caster_at = client.world_spatial().actor_position;
        let cell = tile(caster_at)?;
        if world
            .base()
            .tile(cell.x, cell.y, cell.floor)
            .is_none_or(|tile| tile.protection_zone())
            || client.actor_vitals().is_none_or(|v| v.health == 0)
            || monster_at.floor != caster_at.floor
            || monster_at
                .x
                .abs_diff(caster_at.x)
                .max(monster_at.y.abs_diff(caster_at.y))
                > 5
        {
            return Err(
                "C17d competitor hit after caster became ineligible; cannot prove timer expiry"
                    .into(),
            );
        }
        row.pass("expiry-normal-targeting", expiry);
        with_observer_liveness(&mut peer, combat_rejoin(row, client, clients, world, 0xc6)).await
    })
    .await;
    let cleanup = with_observer_liveness(client, rune_cleanup(&mut peer, world)).await;
    if let Err(error) = &cleanup {
        evidence(&format!("E2EF5_GAP C17d peer cleanup: {error}"));
    }
    result.map_err(|_| "C17d 130s peer journey deadline; required actions unproven")??;
    cleanup
}

fn challenge_duration(spell: &crate::spell::SpellDefinition) -> TestResult<Duration> {
    let crate::spell::Execution::NativeProfile(profile) = &spell.execution else {
        return Err("Challenge has no authored native behavior".into());
    };
    let recipe = &profile.spell()["execution"]["native_behavior"];
    if recipe["key"] != "monster_ai_override"
        || recipe["parameters"]["challenge_target"] != "caster"
    {
        return Err("Challenge recipe does not force the caster".into());
    }
    let millis = recipe["parameters"]["challenge_ms"]
        .as_u64()
        .filter(|ms| *ms > 0)
        .ok_or("Challenge has no positive authored lifetime")?;
    Ok(Duration::from_millis(millis))
}

#[cfg(test)]
mod continuation_two_tests {
    use super::*;

    #[test]
    fn combat_rows_require_every_nonzero_action_receipt() {
        for (id, actions) in [("C18a", AIM_ACTIONS), ("C17d", CHALLENGE_ACTIONS)] {
            let mut row = Row::new(id, actions);
            for action in &actions[..actions.len() - 1] {
                row.pass(action, 1);
            }
            assert!(!row.complete());
            row.pass(actions[actions.len() - 1], 2);
            assert!(row.complete());
            row.pass(actions[0], 3);
            assert!(!row.complete());
        }
    }

    #[test]
    fn damage_receipts_require_exact_source_victim_generation_and_damage_kind() -> TestResult {
        use oteryn_protocol_oteryn::presentation::{
            PresentationColor, PresentationEventKind, PresentationValue, PresentationValueKind,
        };
        let source = oteryn_dev_client::EntityRef {
            identity: [1; 16],
            generation: 1,
        };
        let victim = oteryn_dev_client::EntityRef {
            identity: [2; 16],
            generation: 2,
        };
        let event = |kind, attacker, target| PresentationEventKind::ValueText {
            kind,
            target,
            attacker,
            position: ActorPosition {
                x: 1,
                y: 2,
                floor: 7,
            },
            primary: PresentationValue {
                value: 3,
                color: PresentationColor::Red,
            },
            secondary: None,
        };
        assert!(attributed_damage(
            &event(PresentationValueKind::DamageReceived, Some(source), victim),
            source,
            victim
        ));
        assert!(!attributed_damage(
            &event(PresentationValueKind::Healed, Some(source), victim),
            source,
            victim
        ));
        assert!(!attributed_damage(
            &event(PresentationValueKind::DamageReceived, None, victim),
            source,
            victim
        ));
        let stale = oteryn_dev_client::EntityRef {
            generation: 1,
            ..victim
        };
        assert!(!attributed_damage(
            &event(PresentationValueKind::DamageReceived, Some(source), stale),
            source,
            victim
        ));
        let entry = oteryn_dev_client::WorldSpatialEntity {
            display_name: None,
            entity: victim,
            kind: oteryn_dev_client::EntityKind::Creature,
            position: ActorPosition {
                x: 1,
                y: 2,
                floor: 7,
            },
            detail: oteryn_dev_client::EntityDetail::Actor {
                direction: StepDirection::North,
                appearance_ref: 1,
                health_percent: 80,
            },
        };
        assert_eq!(hp(&entry)?, 80);
        let object = oteryn_dev_client::WorldSpatialEntity {
            detail: oteryn_dev_client::EntityDetail::Object {
                item_definition_ref: 1,
                quantity: 1,
                item_handle: None,
            },
            ..entry
        };
        assert!(hp(&object).is_err());
        Ok(())
    }

    #[test]
    fn combat_spells_resolve_real_selected_player_recipes() -> TestResult {
        let book = native_spell_book()?;
        for words in ["exevo infir flam hur", "exeta res"] {
            let spell = book.spoken(words).ok_or("combat spell absent")?.spell;
            assert_eq!(
                book.indexed(player_spell_index(book, words)?)
                    .map(|s| &s.key),
                Some(&spell.key)
            );
            assert!(!spell.learning_required);
            assert_eq!(
                spell.mana,
                crate::spell::ManaCost::Fixed(if words == "exeta res" { 40 } else { 8 })
            );
        }
        let scorch = book
            .spoken("exevo infir flam hur")
            .ok_or("Scorch absent")?
            .spell;
        assert_eq!(scorch.level, 1);
        assert!(
            scorch
                .authored
                .as_ref()
                .is_some_and(|p| p.header.targeting.needs_direction)
        );
        assert_eq!(
            challenge_duration(book.spoken("exeta res").ok_or("Challenge absent")?.spell)?,
            Duration::from_secs(6)
        );
        Ok(())
    }
}

// Continuation 3: ordinary companion acquisition, behavior and owner end paths.
const SUMMON_ACTIONS: &[&str] = &[
    "summon-payment-spawn",
    "summon-follow-attack",
    "invalid-name-no-cost",
    "summon-cap-no-cost",
    "summon-logout-removal",
    "familiar-payment-spawn",
    "familiar-cooldown-no-cost",
    "familiar-follow-attack",
    "familiar-logout-return",
    "familiar-retained-cooldown",
    "owner-death-removal",
    "death-fresh-admission",
];
const ACQUIRE_ACTIONS: &[&str] = &[
    "buy-convince",
    "buy-animate",
    "convince-floor-refusal",
    "animate-floor-refusal",
    "convince-payment-master",
    "owned-target-refusal",
    "convinced-follow-attack",
    "player-kill-corpse",
    "animate-payment-corpse-removal",
    "animated-follow-attack",
    "summon-cap-no-cost",
    "logout-removes-both",
    "relog-rune-custody",
    "reacquire-for-death",
    "owner-death-removal",
    "death-fresh-admission",
];

async fn companion_rows(clients: &SeamClients<'_>, world: &BundleWorld) {
    for (id, actions, tag) in [
        ("C13", SUMMON_ACTIONS, 0xd0),
        ("C16c", ACQUIRE_ACTIONS, 0xd8),
    ] {
        let mut row = Row::new(id, actions);
        let result: TestResult = async {
            let mut client = connect_world(clients, tag).await?;
            let result = tokio::time::timeout(Duration::from_secs(600), async {
                if id == "C13" {
                    summons(&mut row, &mut client, clients, world).await
                } else {
                    acquired_companions(&mut row, &mut client, clients, world).await
                }
            })
            .await;
            let cleanup = rune_cleanup(&mut client, world).await;
            if let Err(error) = &cleanup {
                evidence(&format!("E2EF5_GAP {id} cleanup: {error}"));
            }
            result.map_err(|_| {
                format!("{id}: 600s companion journey deadline; required actions unproven")
            })??;
            cleanup
        }
        .await;
        row.finish(result);
    }
}

fn named(entry: &oteryn_dev_client::WorldSpatialEntity, name: &str) -> bool {
    entry.kind == oteryn_dev_client::EntityKind::Creature
        && entry.entity.generation != 0
        && entry
            .display_name
            .as_ref()
            .and_then(|n| n.as_str().ok())
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
}
fn census(client: &DevClientSession) -> TestResult<BTreeSet<oteryn_dev_client::EntityRef>> {
    Ok(client
        .world_entities()
        .ok_or("companion visibility absent")?
        .iter()
        .map(|entry| entry.entity)
        .collect())
}
fn companion_mana(
    input: &crate::content::native_gameplay::NativeGameplayInput,
    name: &str,
) -> TestResult<u32> {
    let document: serde_json::Value = serde_json::from_slice(&input.creature_profiles.bytes)?;
    // Use the current Game-owned creature, never a similarly named legacy reference.
    let details = document["records"]
        .as_array()
        .ok_or("creature records absent")?
        .iter()
        .find_map(|r| {
            let details = &r["profile"]["data"]["profile"]["details"];
            (r["profile"]["target"]["key"]
                .as_str()
                .is_some_and(|key| key.starts_with("oteryn:"))
                && details["display_name"]
                    .as_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(name)))
            .then_some(details)
        })
        .ok_or_else(|| format!("exact Game-owned {name} creature policy absent"))?;
    Ok(u32::try_from(
        details["summoning"]["mana_cost"]
            .as_u64()
            .ok_or("creature type mana cost absent")?,
    )?)
}
async fn companion_payment(
    client: &mut DevClientSession,
    before: oteryn_protocol_oteryn::actor_spell::ActorVitals,
    mana: u32,
    soul: u32,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event
                && before.mana.checked_sub(mana) == Some(delta.value.mana)
                && before.soul.checked_sub(soul) == Some(delta.value.soul)
            {
                return Ok(delta.server_sequence);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("companion payment frame absent: before_mana={} cost={mana} soul={soul} observed={:?}", before.mana, client.actor_vitals()).into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}
async fn companion_spawn(
    client: &mut DevClientSession,
    before: &BTreeSet<oteryn_dev_client::EntityRef>,
    name: &str,
    origin: ActorPosition,
) -> TestResult<(oteryn_dev_client::EntityRef, u64)> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let candidates: Vec<_> = client
            .world_entities()
            .ok_or("spawn visibility absent")?
            .iter()
            .filter(|entry| {
                !before.contains(&entry.entity)
                    && named(entry, name)
                    && entry.position.floor == origin.floor
                    && entry
                        .position
                        .x
                        .abs_diff(origin.x)
                        .max(entry.position.y.abs_diff(origin.y))
                        <= 1
            })
            .copied()
            .collect();
        if let [entry] = candidates.as_slice() {
            // The applied VIS snapshot/delta, including its name and exact incarnation,
            // is the evidence. An accepted command alone cannot identify a summon.
            return Ok((entry.entity, client.last_server_sequence()));
        }
        if !candidates.is_empty() || tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "{name}: expected exactly one newly observed creature, got {}",
                candidates.len()
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}
async fn non_pz(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    })
    .ok_or("companion: no player walk route outside PZ")?;
    walk(client, world, &path).await
}
async fn summon_rat(
    client: &mut DevClientSession,
    world: &BundleWorld,
    book: &crate::spell::SpellBook,
    mana: u32,
    try_vocation: bool,
) -> TestResult<(oteryn_dev_client::EntityRef, u64)> {
    let words = "utevo res \"rat\"";
    let spell = book.spoken(words).ok_or("Summon Creature absent")?.spell;
    non_pz(client, world).await?;
    for attempt in 0..2 {
        spell_ready(client, spell).await?;
        let before = census(client)?;
        let vitals = client
            .actor_vitals()
            .copied()
            .ok_or("summon vitals absent")?;
        let observed_level = level(client).await?;
        client.take_events();
        let reply = say(client, words).await?;
        if reply.status == CommandStatus::Accepted && reply.disposition == ChatDisposition::Ok {
            let sequence = companion_payment(client, vitals, mana, spell.soul).await?;
            let (entity, _) = companion_spawn(
                client,
                &before,
                "Rat",
                client.world_spatial().actor_position,
            )
            .await?;
            return Ok((entity, sequence));
        }
        let blocker = format!(
            "Summon Creature refused: {:?}; observed_level={observed_level} required_level={} observed_mana={} creature_type_mana={mana} vocations={:?}; source:crates/gameplay/dispatch/src/gameplay_transport/native_companion_item_cast.rs:77",
            reply.disposition, spell.level, vitals.mana, spell.vocations
        );
        if attempt == 0 && try_vocation {
            player_first_vocation(client, world, "sorcerer")
                .await
                .map_err(|error| format!("{blocker}; player vocation route: {error}"))?;
            non_pz(client, world).await?;
        } else {
            return Err(blocker.into());
        }
    }
    Err("summon attempt exhausted".into())
}
async fn companion_speech_refusal(client: &mut DevClientSession, words: &str) -> TestResult<u64> {
    let before = census(client)?;
    let vitals = client
        .actor_vitals()
        .copied()
        .ok_or("refusal vitals absent")?;
    let reply = say(client, words).await?;
    if reply.status != CommandStatus::Rejected || reply.disposition != ChatDisposition::Rejected {
        return Err(format!("{words}: expected native cast refusal, got {reply:?}").into());
    }
    level(client).await?;
    if census(client)? != before || client.actor_vitals().copied() != Some(vitals) {
        return Err("refused companion speech changed creature roster or resources".into());
    }
    Ok(reply.result_server_sequence)
}

// Ownership is behavioral: this exact creature follows the owner's movement,
// then attacks the owner's exact selected enemy. No master bit is inferred
// from a name, placement or a successful cast command.
async fn companion_behavior(
    client: &mut DevClientSession,
    world: &BundleWorld,
    owned: oteryn_dev_client::EntityRef,
    excluded: &[oteryn_dev_client::EntityRef],
) -> TestResult<u64> {
    select_monster(client, None).await?;
    let start = creature(client, owned)?.position;
    let here = tile(client.world_spatial().actor_position)?;
    let path = route(world, here, |at| {
        at.floor == here.floor
            && at.x.abs_diff(here.x).max(at.y.abs_diff(here.y)) == 4
            && world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone())
    })
    .ok_or("no four-tile ordinary companion follow route")?;
    client.take_events();
    walk(client, world, &path).await?;
    let until = tokio::time::Instant::now() + Duration::from_secs(15);
    let mut followed = false;
    loop {
        for event in client.take_events() {
            if let SessionEvent::WorldSpatialEntities(delta) = event {
                followed |= delta
                    .value
                    .update
                    .iter()
                    .any(|entry| entry.entity == owned && entry.position != start);
            }
        }
        let owner = client.world_spatial().actor_position;
        let entry = creature(client, owned)?;
        if followed
            && entry.position.floor == owner.floor
            && entry
                .position
                .x
                .abs_diff(owner.x)
                .max(entry.position.y.abs_diff(owner.y))
                <= 2
        {
            break;
        }
        if tokio::time::Instant::now() >= until {
            return Err("exact companion did not follow real player movement".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    let victim = wild_rat(client, world, excluded).await?;
    // Stand beyond fist range; the damage must be from the companion.
    let target = tile(victim.position)?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == target.floor
            && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) == 3
            && world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone())
    })
    .ok_or("no companion attack observation tile")?;
    walk(client, world, &path).await?;
    client.take_events();
    let initial_hp = hp(&creature(client, victim.entity)?)?;
    select_monster(client, Some(victim.entity)).await?;
    let until = tokio::time::Instant::now() + Duration::from_secs(35);
    let mut hit = 0;
    loop {
        hit = hit.max(damage_frames(client, owned, victim.entity));
        let after = creature(client, victim.entity).ok();
        if hit != 0
            && after
                .as_ref()
                .is_none_or(|entry| hp(entry).is_ok_and(|hp| hp < initial_hp))
        {
            select_monster(client, None).await?;
            return Ok(hit);
        }
        if tokio::time::Instant::now() >= until {
            return Err(
                "companion lacks exact attributed damage and target HP loss after owner selection"
                    .into(),
            );
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}
async fn wild_rat(
    client: &mut DevClientSession,
    world: &BundleWorld,
    excluded: &[oteryn_dev_client::EntityRef],
) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    travel_to_spawn(client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let until = tokio::time::Instant::now() + Duration::from_secs(25);
    loop {
        let here = client.world_spatial().actor_position;
        let candidates: Vec<_> = client
            .world_entities()
            .ok_or("rat visibility absent")?
            .iter()
            .filter(|entry| {
                named(entry, "Rat")
                    && !excluded.contains(&entry.entity)
                    && entry.position.floor == here.floor
                    && entry
                        .position
                        .x
                        .abs_diff(here.x)
                        .max(entry.position.y.abs_diff(here.y))
                        <= 4
            })
            .copied()
            .collect();
        if let [entry] = candidates.as_slice() {
            return Ok(*entry);
        }
        if tokio::time::Instant::now() >= until {
            return Err(format!("compiled rat spawn needs one exact named live target, observed {} (owned exclusions={excluded:?})", candidates.len()).into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

// A second, ordinary player stays at the owner position and witnesses exact
// generation removal. Fresh admission checks physical return and retained cost.
async fn companion_logout(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    owned: &[oteryn_dev_client::EntityRef],
    familiar: bool,
    tag: u8,
) -> TestResult<(Option<oteryn_dev_client::EntityRef>, u64)> {
    select_monster(client, None).await?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == TOWN
    })
    .ok_or("companion logout: no player route back to town")?;
    walk(client, world, &path).await?;
    let until = tokio::time::Instant::now() + Duration::from_secs(20);
    while owned
        .iter()
        .any(|entity| creature(client, *entity).is_err())
    {
        if tokio::time::Instant::now() >= until {
            return Err("companion lost before logout; removal would be unproven".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    let mut observer = connect_character(clients, tag, 0).await?;
    let result: TestResult<_> = async {
        let owner_position = tile(client.world_spatial().actor_position)?;
        let path = route(
            world,
            tile(observer.world_spatial().actor_position)?,
            |at| {
                at.floor == owner_position.floor
                    && at
                        .x
                        .abs_diff(owner_position.x)
                        .max(at.y.abs_diff(owner_position.y))
                        <= 2
                    && at != owner_position
            },
        )
        .ok_or("companion observer cannot reach logout site")?;
        with_observer_liveness(client, walk(&mut observer, world, &path)).await?;
        for entity in owned {
            creature(&observer, *entity)?;
        }
        let session = client.join_snapshot().game_session_id;
        let vitals = client
            .actor_vitals()
            .copied()
            .ok_or("logout companion vitals absent")?;
        observer.take_events();
        with_observer_liveness(&mut observer, player_logout(client)).await?;
        let removal = companion_leaves(&mut observer, owned).await?;
        *client = with_observer_liveness(&mut observer, connect_world(clients, tag + 1)).await?;
        if client.join_snapshot().game_session_id == session
            || tile(client.world_spatial().actor_position)? != TOWN
            || client
                .actor_vitals()
                .is_none_or(|after| after.mana != vitals.mana || after.soul != vitals.soul)
            || owned.iter().any(|entity| creature(client, *entity).is_ok())
        {
            return Err(
                "fresh admission lost companion cost/position or retained old actor generation"
                    .into(),
            );
        }
        let returned = if familiar {
            let before = owned.iter().copied().collect();
            let (entity, _) = with_observer_liveness(
                &mut observer,
                companion_spawn(
                    client,
                    &before,
                    "Sorcerer familiar",
                    client.world_spatial().actor_position,
                ),
            )
            .await?;
            Some(entity)
        } else {
            // A five-second visible observation catches an erroneously restored
            // ordinary summon; neither an empty command reply nor logout suffices.
            let until = tokio::time::Instant::now() + Duration::from_secs(5);
            while tokio::time::Instant::now() < until {
                with_observer_liveness(&mut observer, async {
                    client.service_liveness(Duration::from_millis(50)).await?;
                    Ok(())
                })
                .await?;
                if client
                    .world_entities()
                    .ok_or("relog entities absent")?
                    .iter()
                    .any(|entry| named(entry, "Rat") || named(entry, "Skeleton"))
                {
                    return Err("ordinary summon returned after logout at town".into());
                }
            }
            None
        };
        Ok((returned, removal))
    }
    .await;
    let cleanup = with_observer_liveness(client, rune_cleanup(&mut observer, world)).await;
    if let Err(error) = &cleanup {
        evidence(&format!("E2EF5_GAP companion observer cleanup: {error}"));
    }
    let value = result?;
    cleanup?;
    Ok(value)
}
async fn companion_leaves(
    client: &mut DevClientSession,
    owned: &[oteryn_dev_client::EntityRef],
) -> TestResult<u64> {
    let mut remaining: BTreeSet<_> = owned.iter().copied().collect();
    let mut sequence = 0;
    let until = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        for event in client.take_events() {
            if let SessionEvent::WorldSpatialEntities(delta) = event {
                for entity in delta.value.leave {
                    if remaining.remove(&entity) {
                        sequence = sequence.max(delta.server_sequence);
                    }
                }
            }
        }
        if remaining.is_empty() && sequence != 0 {
            return Ok(sequence);
        }
        if tokio::time::Instant::now() >= until {
            return Err(
                format!("companion owner end lacks exact VIS leave frames: {remaining:?}").into(),
            );
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

async fn summons(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let input = native_input()?;
    let book = native_spell_book()?;
    let mana = companion_mana(input, "Rat")?;
    let (rat, sequence) = summon_rat(client, world, book, mana, true).await?;
    row.pass("summon-payment-spawn", sequence);
    row.pass(
        "summon-follow-attack",
        companion_behavior(client, world, rat, &[rat]).await?,
    );
    let spell = book
        .spoken("utevo res \"rat\"")
        .ok_or("summon profile absent")?
        .spell;
    spell_ready(client, spell).await?;
    row.pass(
        "invalid-name-no-cost",
        companion_speech_refusal(client, "utevo res \"E2EF5 nonexistent creature\"").await?,
    );
    let (second, _) = summon_rat(client, world, book, mana, false).await?;
    spell_ready(client, spell).await?;
    if client.actor_vitals().is_none_or(|v| v.mana < mana) {
        return Err(format!("summon-cap comparison prerequisite: needs {mana} spare mana so insufficient mana cannot explain the refusal; observed={:?}", client.actor_vitals()).into());
    }
    row.pass(
        "summon-cap-no-cost",
        companion_speech_refusal(client, "utevo res \"rat\"").await?,
    );
    let (_, sequence) =
        companion_logout(client, clients, world, &[rat, second], false, 0xd1).await?;
    row.pass("summon-logout-removal", sequence);

    non_pz(client, world).await?;
    let words = "utevo gran res ven";
    let before = census(client)?;
    client.take_events();
    let cast = player_spell(
        client,
        book,
        words,
        oteryn_protocol_oteryn::actor_spell::SpellTarget::None,
        false,
    )
    .await?;
    let (familiar, sequence) = companion_spawn(
        client,
        &before,
        "Sorcerer familiar",
        client.world_spatial().actor_position,
    )
    .await?;
    row.pass(
        "familiar-payment-spawn",
        sequence.max(cast.result_server_sequence),
    );
    row.pass(
        "familiar-cooldown-no-cost",
        familiar_cooldown(client, book).await?,
    );
    row.pass(
        "familiar-follow-attack",
        companion_behavior(client, world, familiar, &[familiar]).await?,
    );
    let (returned, sequence) =
        companion_logout(client, clients, world, &[familiar], true, 0xd3).await?;
    let returned = returned.ok_or("familiar did not return on ordinary admission")?;
    row.pass("familiar-logout-return", sequence);
    row.pass(
        "familiar-retained-cooldown",
        familiar_cooldown(client, book).await?,
    );
    // Ordinary summons and the familiar must both end on the same real death.
    let (rat, _) = summon_rat(client, world, book, mana, false).await?;
    row.pass(
        "owner-death-removal",
        companion_owner_death(client, clients, world, &[returned, rat], 0xd5).await?,
    );
    row.pass(
        "death-fresh-admission",
        companion_after_death(client, clients, world, true, &[], 0xd7).await?,
    );
    Ok(())
}
async fn familiar_cooldown(
    client: &mut DevClientSession,
    book: &crate::spell::SpellBook,
) -> TestResult<u64> {
    let vitals = client
        .actor_vitals()
        .copied()
        .ok_or("familiar cooldown vitals absent")?;
    let before = census(client)?;
    let reply = client
        .cast_spell(
            player_spell_index(book, "utevo gran res ven")?,
            oteryn_protocol_oteryn::actor_spell::SpellTarget::None,
            false,
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || reply.disposition
            != oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::CoolingDown
    {
        return Err(format!(
            "familiar expected retained CoolingDown, observed {:?}",
            reply.disposition
        )
        .into());
    }
    level(client).await?;
    if client.actor_vitals().copied() != Some(vitals) || census(client)? != before {
        return Err("familiar cooldown refusal changed resources/roster".into());
    }
    Ok(reply.result_server_sequence)
}

// A real hostile spawn supplies lethal damage. The witness stays in the same
// viewport as the live companions until death, so walking out of visibility
// cannot masquerade as owner-death cleanup.
async fn companion_owner_death(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    owned: &[oteryn_dev_client::EntityRef],
    tag: u8,
) -> TestResult<u64> {
    select_monster(client, None).await?;
    let previous_death = recent_player_deaths(client).await?.0.first().cloned();
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    travel_to_spawn(client, world, &metadata.spawns, "oteryn:creature.dragon")
        .await
        .map_err(|error| {
            format!(
                "owner-death prerequisite: real lethal dragon spawn not player-reachable: {error}"
            )
        })?;
    let encounter = client.world_spatial().actor_position;
    let attacker = client
        .world_entities()
        .ok_or("death encounter visibility absent")?
        .iter()
        .find(|entry| {
            named(entry, "Dragon")
                && entry.position.floor == encounter.floor
                && entry
                    .position
                    .x
                    .abs_diff(encounter.x)
                    .max(entry.position.y.abs_diff(encounter.y))
                    <= 4
        })
        .map(|entry| entry.entity)
        .ok_or("owner death: compiled dragon spawn has no live named hostile")?;
    for entity in owned {
        creature(client, *entity)?;
    }
    let mut observer = connect_character(clients, tag, 0).await?;
    let result: TestResult<_> = async {
        let site = tile(encounter)?;
        let path = route(world, tile(observer.world_spatial().actor_position)?, |at| {
            at.floor == site.floor && at.x.abs_diff(site.x).max(at.y.abs_diff(site.y)) <= 3 && at != site
        }).ok_or("ordinary witness cannot reach the lethal encounter")?;
        with_observer_liveness(client, walk(&mut observer, world, &path)).await?;
        for entity in owned { creature(&observer, *entity)?; }
        if client.world_spatial().actor_position != encounter {
            return Err("owner respawned before witness reached encounter; end-of-life unproven".into());
        }
        let observer_position = observer.world_spatial().actor_position;
        let mut positions: std::collections::BTreeMap<_, _> = owned.iter().map(|entity| {
            Ok((*entity, creature(&observer, *entity)?.position))
        }).collect::<TestResult<_>>()?;
        let owner = own_entity(client)?;
        let before_health = client.actor_vitals().ok_or("owner death vitals absent")?.health;
        client.take_events();
        observer.take_events();
        let until = tokio::time::Instant::now() + Duration::from_secs(150);
        let mut damaged = false;
        let mut attributed = false;
        let mut left = BTreeSet::new();
        let mut sequence = 0;
        loop {
            client.service_liveness(Duration::from_millis(25)).await?;
            observer.service_liveness(Duration::from_millis(25)).await?;
            for event in client.take_events() {
                match event {
                    SessionEvent::Presentation(delta) => {
                        attributed |= delta.value.iter().any(|event| attributed_damage(&event.kind, attacker, owner));
                    }
                    SessionEvent::ActorVitals(delta) => {
                        damaged |= delta.value.health < before_health;
                    }
                    _ => {}
                }
            }
            for event in observer.take_events() {
                match event {
                    SessionEvent::Presentation(delta) => {
                        if delta.value.iter().any(|event| matches!(&event.kind,
                            oteryn_protocol_oteryn::presentation::PresentationEventKind::ValueText {
                                kind, target, primary, ..
                            } if owned.contains(target) && primary.value > 0
                                && matches!(kind, oteryn_protocol_oteryn::presentation::PresentationValueKind::DamageOthers
                                    | oteryn_protocol_oteryn::presentation::PresentationValueKind::DamageReceived
                                    | oteryn_protocol_oteryn::presentation::PresentationValueKind::DamageDealt))) {
                            return Err("companion took hostile damage during owner death; independent companion death would make cleanup ambiguous".into());
                        }
                    }
                    SessionEvent::WorldSpatialEntities(delta) => {
                        for entry in &delta.value.update {
                            if owned.contains(&entry.entity) { positions.insert(entry.entity, entry.position); }
                        }
                        for entity in delta.value.leave {
                            if owned.contains(&entity) {
                                let at = positions.get(&entity).ok_or("death witness lost companion position")?;
                                if at.floor != observer_position.floor
                                    || at.x.abs_diff(observer_position.x).max(at.y.abs_diff(observer_position.y)) > 5 {
                                    return Err("companion left stationary witness viewport before death; removal unproven".into());
                                }
                                left.insert(entity);
                                sequence = sequence.max(delta.server_sequence);
                            }
                        }
                    }
                    _ => {}
                }
            }
            let now = client.world_spatial().actor_position;
            let respawned = now.floor != encounter.floor
                || now.x.abs_diff(encounter.x).max(now.y.abs_diff(encounter.y)) > 8;
            if attributed && damaged && respawned
                && client.actor_vitals().is_some_and(|v| v.health == v.max_health)
                && owned.iter().all(|entity| left.contains(entity))
            {
                // The cadence may coalesce zero HP and respawn into one vitals
                // frame. A new native durable death-history entry proves the
                // death without demanding an unpublished intermediate frame.
                let (deaths, _) = with_observer_liveness(&mut observer, recent_player_deaths(client)).await?;
                if !deaths.is_empty() && deaths.first() != previous_death.as_ref() {
                    return Ok(sequence);
                }
            }
            if tokio::time::Instant::now() >= until {
                return Err(format!("real owner death incomplete: attributed={attributed} damaged={damaged} respawned={respawned} companion_leaves={left:?}; no synthetic death or state mutation").into());
            }
        }
    }.await;
    let cleanup = with_observer_liveness(client, rune_cleanup(&mut observer, world)).await;
    if let Err(error) = &cleanup {
        evidence(&format!("E2EF5_GAP death observer cleanup: {error}"));
    }
    let sequence = result?;
    cleanup?;
    Ok(sequence)
}
async fn companion_after_death(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    familiar: bool,
    quantities: &[(std::num::NonZeroU32, u64)],
    tag: u8,
) -> TestResult<u64> {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == TOWN
    })
    .ok_or("post-death return to town unavailable")?;
    walk(client, world, &path).await?;
    let death = recent_player_deaths(client)
        .await?
        .0
        .first()
        .cloned()
        .ok_or("post-death history absent")?;
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("post-death vitals absent")?;
    let sequence = rune_rejoin(client, clients, tag, quantities).await?;
    if recent_player_deaths(client).await?.0.first() != Some(&death)
        || client
            .actor_vitals()
            .is_none_or(|after| after.mana != before.mana || after.soul != before.soul)
    {
        return Err("death history or post-death resources lost across fresh admission".into());
    }
    let until = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < until {
        client.service_liveness(Duration::from_millis(50)).await?;
        if client
            .world_entities()
            .ok_or("post-death visibility absent")?
            .iter()
            .any(|entry| {
                named(entry, "Rat") || named(entry, "Skeleton") || named(entry, "Sorcerer familiar")
            })
        {
            return Err(
                "owner-death companion removal did not survive fresh admission at town".into(),
            );
        }
    }
    if familiar {
        familiar_cooldown(client, native_spell_book()?).await?;
    }
    Ok(sequence)
}

async fn companion_rune_refusal(
    client: &mut DevClientSession,
    spell: &crate::spell::SpellDefinition,
    reference: std::num::NonZeroU32,
    target: oteryn_dev_client::EntityRef,
) -> TestResult<u64> {
    spell_ready(client, spell).await?;
    let quantity = rune_quantity(client, reference)?;
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("utility rune refusal vitals absent")?;
    let reply = rune_use(client, reference, Some(target), None).await?;
    if reply.0 != oteryn_protocol_oteryn::world_object::UseDisposition::TargetIllegal {
        return Err(format!(
            "utility companion rune must refuse target, got {:?}",
            reply.0
        )
        .into());
    }
    level(client).await?;
    if rune_quantity(client, reference)? != quantity
        || client.actor_vitals().copied() != Some(before)
    {
        return Err("refused companion rune consumed a charge or resources".into());
    }
    Ok(reply.1)
}
async fn convince_rat(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spell: &crate::spell::SpellDefinition,
    reference: std::num::NonZeroU32,
    excluded: &[oteryn_dev_client::EntityRef],
    mana: u32,
) -> TestResult<(oteryn_dev_client::EntityRef, u64)> {
    let target = wild_rat(client, world, excluded).await?;
    spell_ready(client, spell).await?;
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("convince vitals absent")?;
    client.take_events();
    let (_, remaining) = cast_rune(client, spell, reference, Some(target.entity), None).await
        .map_err(|error| format!("{error}; Convince Creature additionally requires current creature mana={mana}; observed_mana={}", before.mana))?;
    let sequence = companion_payment(client, before, mana, spell.soul).await?;
    inventory_quantity_or_applied(client, reference, remaining).await?;
    creature(client, target.entity)?;
    // Conversion retains the existing generation; follow/attack proves master.
    Ok((target.entity, sequence))
}
async fn inventory_quantity_or_applied(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
    remaining: u64,
) -> TestResult {
    level(client).await?;
    if rune_quantity(client, reference)? != remaining {
        return Err(format!(
            "companion rune exact charge debit absent: expected={remaining} observed={}",
            rune_quantity(client, reference)?
        )
        .into());
    }
    Ok(())
}

async fn player_corpse(
    client: &mut DevClientSession,
    world: &BundleWorld,
    owned: &[oteryn_dev_client::EntityRef],
) -> TestResult<(oteryn_dev_client::WorldSpatialEntity, u64)> {
    let target = wild_rat(client, world, owned).await?;
    let index = provisioning::item_index()?;
    let corpse_ref = provisioning::item_reference(&index, 5964)?;
    let existing = census(client)?;
    let owner = own_entity(client)?;
    let at = tile(target.position)?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |p| {
        p.floor == at.floor && p.x.abs_diff(at.x).max(p.y.abs_diff(at.y)) <= 1
    })
    .ok_or("no ordinary melee approach to corpse-source rat")?;
    walk(client, world, &path).await?;
    client.take_events();
    select_monster(client, Some(target.entity)).await?;
    let until = tokio::time::Instant::now() + Duration::from_secs(75);
    let mut damage = false;
    let mut removed = false;
    let mut last = target.position;
    let mut corpse = None;
    loop {
        for event in client.take_events() {
            match event {
                SessionEvent::Presentation(delta) => {
                    damage |= delta
                        .value
                        .iter()
                        .any(|event| attributed_damage(&event.kind, owner, target.entity));
                }
                SessionEvent::WorldSpatialEntities(delta) => {
                    for entry in &delta.value.update {
                        if entry.entity == target.entity {
                            last = entry.position;
                        }
                    }
                    removed |= delta.value.leave.contains(&target.entity);
                    for entry in delta.value.enter {
                        if entry.kind == oteryn_dev_client::EntityKind::Corpse
                            && !existing.contains(&entry.entity)
                            && entry.position == last
                            && matches!(entry.detail, oteryn_dev_client::EntityDetail::Object { item_definition_ref, item_handle: Some(_), .. } if item_definition_ref == corpse_ref.get())
                        {
                            corpse = Some((entry, delta.server_sequence));
                        }
                    }
                }
                _ => {}
            }
        }
        if damage
            && removed
            && let Some(corpse) = corpse
        {
            select_monster(client, None).await?;
            return Ok(corpse);
        }
        if tokio::time::Instant::now() >= until {
            return Err(format!("ordinary player kill lacks attributed fist damage, victim leave and exact rat corpse: damage={damage} removed={removed} corpse={}", corpse.is_some()).into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}
async fn animate_corpse(
    client: &mut DevClientSession,
    spell: &crate::spell::SpellDefinition,
    reference: std::num::NonZeroU32,
    corpse: oteryn_dev_client::WorldSpatialEntity,
) -> TestResult<(oteryn_dev_client::EntityRef, u64)> {
    spell_ready(client, spell).await?;
    creature(client, corpse.entity)?;
    let before = census(client)?;
    let vitals = client
        .actor_vitals()
        .copied()
        .ok_or("animate vitals absent")?;
    client.take_events();
    let (_, remaining) = cast_rune(client, spell, reference, None, Some(corpse.position)).await?;
    // A zero-mana rune need not emit an unchanged vitals delta. The charge
    // delta and exact corpse removal/creature spawn are still mandatory.
    let sequence = inventory_quantity(client, reference, remaining).await?;
    level(client).await?;
    if client
        .actor_vitals()
        .is_none_or(|after| after.mana != vitals.mana || after.soul != vitals.soul)
        || creature(client, corpse.entity).is_ok()
    {
        return Err("Animate Dead did not remove the exact corpse with zero resource cost".into());
    }
    let (skeleton, _) = companion_spawn(client, &before, "Skeleton", corpse.position).await?;
    Ok((skeleton, sequence))
}
async fn acquired_companions(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let input = native_input()?;
    let book = native_spell_book()?;
    let convince = book.rune(3177).ok_or("Convince Creature rune absent")?;
    let animate = book.rune(3203).ok_or("Animate Dead rune absent")?;
    let mana = companion_mana(input, "Rat")?;
    let (convince_ref, sequence) = acquire_rune(client, world, 3177).await?;
    row.pass("buy-convince", sequence);
    let (animate_ref, sequence) = acquire_rune(client, world, 3203).await?;
    row.pass("buy-animate", sequence);
    non_pz(client, world).await?;
    row.pass(
        "convince-floor-refusal",
        illegal_floor(client, convince_ref).await?,
    );
    row.pass(
        "animate-floor-refusal",
        illegal_floor(client, animate_ref).await?,
    );
    let (rat, sequence) = convince_rat(client, world, convince, convince_ref, &[], mana).await?;
    // This receipt is only marked after follow/attack, which proves conversion.
    let behavior = companion_behavior(client, world, rat, &[rat]).await?;
    row.pass("convince-payment-master", sequence);
    row.pass("convinced-follow-attack", behavior);
    row.pass(
        "owned-target-refusal",
        companion_rune_refusal(client, convince, convince_ref, rat).await?,
    );
    let (corpse, sequence) = player_corpse(client, world, &[rat]).await?;
    row.pass("player-kill-corpse", sequence);
    let (skeleton, sequence) = animate_corpse(client, animate, animate_ref, corpse).await?;
    row.pass("animate-payment-corpse-removal", sequence);
    row.pass(
        "animated-follow-attack",
        companion_behavior(client, world, skeleton, &[rat, skeleton]).await?,
    );
    let third = wild_rat(client, world, &[rat, skeleton]).await?;
    if client.actor_vitals().is_none_or(|v| v.mana < mana) {
        return Err(format!("convince-cap comparison prerequisite: needs {mana} spare mana to isolate the cap refusal; observed={:?}", client.actor_vitals()).into());
    }
    row.pass(
        "summon-cap-no-cost",
        companion_rune_refusal(client, convince, convince_ref, third.entity).await?,
    );
    let retained = [
        (convince_ref, rune_quantity(client, convince_ref)?),
        (animate_ref, rune_quantity(client, animate_ref)?),
    ];
    let (_, sequence) =
        companion_logout(client, clients, world, &[rat, skeleton], false, 0xd9).await?;
    row.pass("logout-removes-both", sequence);
    for (reference, expected) in retained {
        if rune_quantity(client, reference)? != expected {
            return Err("relog companion rune debit was lost".into());
        }
    }
    row.pass("relog-rune-custody", client.last_server_sequence());
    // Use the second legitimately purchased charge of each rune; nothing is
    // resurrected or replenished after the clean logout.
    let (rat, _) = convince_rat(client, world, convince, convince_ref, &[], mana).await?;
    let (corpse, _) = player_corpse(client, world, &[rat]).await?;
    let (skeleton, sequence) = animate_corpse(client, animate, animate_ref, corpse).await?;
    row.pass("reacquire-for-death", sequence);
    row.pass(
        "owner-death-removal",
        companion_owner_death(client, clients, world, &[rat, skeleton], 0xdb).await?,
    );
    // Death can legitimately drop carried items; record the observed post-death
    // quantities and require their retention, never assume a fixture blessing.
    let retained = [
        (convince_ref, rune_quantity(client, convince_ref)?),
        (animate_ref, rune_quantity(client, animate_ref)?),
    ];
    row.pass(
        "death-fresh-admission",
        companion_after_death(client, clients, world, false, &retained, 0xdd).await?,
    );
    Ok(())
}

#[cfg(test)]
mod continuation_three_tests {
    use super::*;
    #[test]
    fn companion_rows_fail_closed_on_each_missing_action() {
        for (id, actions) in [("C13", SUMMON_ACTIONS), ("C16c", ACQUIRE_ACTIONS)] {
            for omitted in actions {
                let mut row = Row::new(id, actions);
                for action in actions {
                    if action != omitted {
                        row.pass(action, 1);
                    }
                }
                assert!(!row.complete(), "omitting {omitted} must fail {id}");
            }
            let mut row = Row::new(id, actions);
            for action in actions {
                row.pass(action, 1);
            }
            assert!(row.complete());
            row.pass(actions[0], 2);
            assert!(!row.complete());
        }
    }
    #[test]
    fn companion_identity_requires_named_live_creature() -> TestResult {
        use oteryn_dev_client::{EntityDetail, EntityKind, EntityRef, WorldSpatialEntity};
        let mut entry = WorldSpatialEntity {
            display_name: Some(
                oteryn_protocol_oteryn::entity_names::EntityDisplayName::new("Rat")
                    .map_err(|error| format!("display name: {error:?}"))?,
            ),
            kind: EntityKind::Creature,
            entity: EntityRef {
                identity: [7; 16],
                generation: 1,
            },
            position: actor_position(TOWN),
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        };
        assert!(named(&entry, "rat"));
        assert!(!named(&entry, "Skeleton"));
        entry.entity.generation = 0;
        assert!(!named(&entry, "Rat"));
        entry.entity.generation = 1;
        entry.kind = EntityKind::Player;
        assert!(!named(&entry, "Rat"));
        entry.kind = EntityKind::Creature;
        entry.display_name = None;
        assert!(!named(&entry, "Rat"));
        Ok(())
    }
    #[test]
    fn companion_cost_is_the_current_game_creature_cost() -> TestResult {
        assert_eq!(companion_mana(native_input()?, "Rat")?, 200);
        assert_eq!(companion_mana(native_input()?, "Skeleton")?, 300);
        assert!(companion_mana(native_input()?, "E2EF5 nonexistent creature").is_err());
        Ok(())
    }
}

// Bounded native Cyclopedia page, never a database receipt read.
fn decode_companion_deaths(payload: &[u8]) -> TestResult<Vec<(i64, String)>> {
    use oteryn_protocol_oteryn::cyclopedia_character::CharacterTab;
    if payload.len() < 3 || payload[0] != CharacterTab::RecentDeaths as u8 {
        return Err("invalid native death-history header".into());
    }
    let count = usize::from(u16::from_le_bytes([payload[1], payload[2]]));
    if count > 2 {
        return Err("native death-history page exceeds requested limit".into());
    }
    let mut cursor = 3;
    let mut deaths = Vec::with_capacity(count);
    for _ in 0..count {
        let timestamp = payload
            .get(cursor..cursor + 8)
            .ok_or("truncated death timestamp")?;
        let timestamp = i64::from_le_bytes(timestamp.try_into()?);
        cursor += 8;
        let length = usize::from(*payload.get(cursor).ok_or("missing death text length")?);
        cursor += 1;
        let text = std::str::from_utf8(
            payload
                .get(cursor..cursor + length)
                .ok_or("truncated death text")?,
        )?;
        cursor += length;
        deaths.push((timestamp, text.to_owned()));
    }
    if cursor != payload.len() {
        return Err("native death history has trailing bytes".into());
    }
    Ok(deaths)
}
async fn recent_player_deaths(
    client: &mut DevClientSession,
) -> TestResult<(Vec<(i64, String)>, u64)> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let result = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[wire::CharacterTab::RecentDeaths as u8, 0, 0, 2],
        )
        .await?;
    if result.status != CommandStatus::Accepted {
        return Err("native RecentDeaths player request refused".into());
    }
    Ok((
        decode_companion_deaths(&result.payload)?,
        result.server_sequence,
    ))
}

#[cfg(test)]
mod companion_death_tests {
    use super::*;
    #[test]
    fn native_death_history_rejects_truncated_unbounded_and_wrong_tab_pages() -> TestResult {
        use oteryn_protocol_oteryn::cyclopedia_character::TabView;
        let encoded = TabView::RecentDeaths(vec![(42, "Died at Level 25".into())])
            .encode()
            .map_err(|error| format!("death page: {error:?}"))?;
        assert_eq!(
            decode_companion_deaths(&encoded)?,
            vec![(42, "Died at Level 25".into())]
        );
        for end in 0..encoded.len() {
            assert!(decode_companion_deaths(&encoded[..end]).is_err());
        }
        let mut trailing = encoded.clone();
        trailing.push(0);
        assert!(decode_companion_deaths(&trailing).is_err());
        assert!(decode_companion_deaths(&[2, 3, 0]).is_err());
        assert!(decode_companion_deaths(&[1, 0, 0]).is_err());
        assert!(decode_companion_deaths(&[2, 0, 0])?.is_empty());
        Ok(())
    }
}

const TILE_ITEM_ACTIONS: &[&str] = &[
    "buy-chameleon",
    "buy-disintegrate",
    "buy-movable-target",
    "player-drop-target",
    "chameleon-floor-refusal",
    "chameleon-creature-refusal",
    "chameleon-payment-appearance",
    "chameleon-fresh-admission",
    "disintegrate-floor-refusal",
    "disintegrate-payment-removal",
    "empty-tile-success",
    "removal-fresh-admission",
    "chameleon-natural-expiry",
    "expiry-fresh-admission",
];

async fn tile_item_row(clients: &SeamClients<'_>, world: &BundleWorld) {
    let mut row = Row::new("C16d", TILE_ITEM_ACTIONS);
    let result: TestResult = async {
        let mut client = connect_world(clients, 0xe0).await?;
        let result = tokio::time::timeout(
            Duration::from_secs(400),
            tile_items(&mut row, &mut client, clients, world),
        )
        .await;
        let cleanup = rune_cleanup(&mut client, world).await;
        if let Err(error) = &cleanup {
            evidence(&format!("E2EF5_GAP C16d cleanup: {error}"));
        }
        result.map_err(|_| "C16d 400s journey deadline; required actions unproven")??;
        cleanup
    }
    .await;
    row.finish(result);
}

fn tile_item_duration(spell: &crate::spell::SpellDefinition) -> TestResult<Duration> {
    let crate::spell::Execution::NativeProfile(profile) = &spell.execution else {
        return Err("C16d Chameleon native profile absent".into());
    };
    let parameters = &profile.spell()["execution"]["native_behavior"]["parameters"];
    if parameters["operation"] != "mimic_item" {
        return Err("C16d Chameleon item operation changed".into());
    }
    Ok(Duration::from_millis(
        parameters["duration_ms"]
            .as_u64()
            .filter(|ms| *ms > 0 && *ms <= 200_000)
            .ok_or("C16d unqualified item illusion lifetime")?,
    ))
}

// This API is the join snapshot, not the accumulated live map. Only call after
// admission or before any mutation. Mutation assertions use pushed frames below.
fn admission_tile_contents(
    client: &mut DevClientSession,
    at: ActorPosition,
) -> TestResult<Vec<(oteryn_protocol_oteryn::world_map::MapDefinition, u32, u32)>> {
    let snapshot = client
        .session_mut()
        .world_map_snapshot()
        .ok_or("C16d native map snapshot absent")?;
    let tile = snapshot
        .tiles
        .iter()
        .find(|tile| tile.position == at)
        .filter(|tile| !tile.more)
        .ok_or("C16d target tile absent or truncated in native map")?;
    Ok(tile
        .items
        .iter()
        .map(|item| (item.definition, item.count, item.sub_type))
        .collect())
}

type TileItemContents = Vec<(oteryn_protocol_oteryn::world_map::MapDefinition, u32, u32)>;

async fn tile_item_frame(
    client: &mut DevClientSession,
    at: ActorPosition,
    reference: std::num::NonZeroU32,
    present: bool,
    after: u64,
) -> TestResult<(u64, TileItemContents)> {
    use oteryn_protocol_oteryn::world_map::MapDefinition;
    let until = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::WorldMap(delta) = event
                && delta.server_sequence > after
                && let Some(tile) = delta
                    .value
                    .tiles
                    .iter()
                    .find(|tile| tile.position == at && !tile.more)
                && tile
                    .items
                    .iter()
                    .any(|item| item.definition == MapDefinition::Item(reference))
                    == present
            {
                return Ok((
                    delta.server_sequence,
                    tile.items
                        .iter()
                        .map(|item| (item.definition, item.count, item.sub_type))
                        .collect(),
                ));
            }
        }
        if tokio::time::Instant::now() >= until {
            return Err(format!("C16d missing pushed target map frame: tile={at:?} definition={reference} present={present}").into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn tile_item_rejoin(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    tag: u8,
    quantities: &[(std::num::NonZeroU32, u64)],
) -> TestResult<u64> {
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("C16d pre-logout vitals absent")?;
    let sequence = rune_rejoin(client, clients, tag, quantities).await?;
    if client
        .actor_vitals()
        .is_none_or(|after| after.mana != before.mana || after.soul != before.soul)
    {
        return Err("C16d fresh admission lost paid mana/soul".into());
    }
    Ok(sequence)
}

async fn tile_item_refusal(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<u64> {
    let inventory = client.inventory().cloned().ok_or("C16d inventory absent")?;
    let vitals = client.actor_vitals().copied().ok_or("C16d vitals absent")?;
    let appearance = own_reference(client)?;
    let (reply, sequence) = rune_use(client, reference, Some(own_entity(client)?), None).await?;
    if reply != oteryn_protocol_oteryn::world_object::UseDisposition::TargetIllegal {
        return Err(
            format!("C16d Chameleon creature target expected TargetIllegal: {reply:?}").into(),
        );
    }
    level(client).await?;
    if client.inventory() != Some(&inventory)
        || client.actor_vitals().copied() != Some(vitals)
        || own_reference(client)? != appearance
    {
        return Err("C16d creature-target refusal changed inventory/vitals/appearance".into());
    }
    Ok(sequence)
}

async fn tile_items(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    use oteryn_protocol_oteryn::world_map::MapDefinition;
    let book = native_spell_book()?;
    let chameleon = book.rune(3178).ok_or("C16d Chameleon rune absent")?;
    let disintegrate = book.rune(3197).ok_or("C16d Disintegrate rune absent")?;
    let duration = tile_item_duration(chameleon)?;
    let baseline = own_binding(client)?.look;
    let (chameleon_ref, sequence) = acquire_rune(client, world, 3178).await?;
    row.pass("buy-chameleon", sequence);
    let (disintegrate_ref, sequence) = acquire_rune(client, world, 3197).await?;
    row.pass("buy-disintegrate", sequence);
    // Ordinary shop purchase, never a provisioned object or a pre-existing map
    // item whose disappearance could be decay. Backpacks have no decay clock.
    let (target_ref, sequence) = acquire_rune(client, world, 2854).await?;
    row.pass("buy-movable-target", sequence);
    // Shop travel changes the live viewport; obtain a current join map before
    // choosing a tile. Session::world_map_snapshot exposes only that join map.
    let purchased = [
        (chameleon_ref, rune_quantity(client, chameleon_ref)?),
        (disintegrate_ref, rune_quantity(client, disintegrate_ref)?),
        (target_ref, rune_quantity(client, target_ref)?),
    ];
    tile_item_rejoin(client, clients, 0xe4, &purchased).await?;
    let here = client.world_spatial().actor_position;
    let map = client
        .session_mut()
        .world_map_snapshot()
        .ok_or("C16d map absent")?;
    let at = map
        .tiles
        .iter()
        .find(|tile| {
            let p = tile.position;
            p.floor == here.floor
                && p != here
                && p.x.abs_diff(here.x).max(p.y.abs_diff(here.y)) <= 1
                && tile.ground_speed > 0
                && !tile.more
                && tile.items.len() == 1
                && client
                    .world_entities()
                    .is_some_and(|entities| !entities.iter().any(|entity| entity.position == p))
        })
        .map(|tile| tile.position)
        .ok_or("C16d no fully visible adjacent empty ground tile for a player-dropped target")?;
    let underlying = admission_tile_contents(client, at)?;
    let target_count = rune_quantity(client, target_ref)?;
    let handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == target_ref && entry.count.get() == 1)
        })
        .map(|entry| entry.handle)
        .ok_or("C16d purchased single backpack handle absent")?;
    client.take_events();
    let after = client.last_server_sequence();
    let moved = client
        .move_item(&oteryn_dev_client::ItemMoveIntent {
            source: handle,
            destination: oteryn_dev_client::ItemMoveDestination::Ground(
                oteryn_protocol_oteryn::item_view::WorldTilePosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                },
            ),
        })
        .await?;
    if moved.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved {
        return Err(format!("C16d ordinary backpack drop refused: {moved:?}").into());
    }
    let (sequence, placed) = tile_item_frame(client, at, target_ref, true, after).await?;
    level(client).await?;
    if rune_quantity(client, target_ref)?.checked_add(1) != Some(target_count)
        || placed.len() != underlying.len() + 1
        || !placed.starts_with(&underlying)
        || placed.last() != Some(&(MapDefinition::Item(target_ref), 1, 0))
    {
        return Err("C16d drop lacks exact carried debit and single backpack map addition".into());
    }
    row.pass("player-drop-target", sequence);
    row.pass(
        "chameleon-floor-refusal",
        illegal_floor(client, chameleon_ref).await?,
    );
    row.pass(
        "chameleon-creature-refusal",
        tile_item_refusal(client, chameleon_ref).await?,
    );
    spell_ready(client, chameleon).await?;
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("C16d Chameleon vitals absent")?;
    let original_reference = own_reference(client)?;
    let started = tokio::time::Instant::now();
    let (cast_sequence, chameleon_left) =
        cast_rune(client, chameleon, chameleon_ref, None, Some(at)).await?;
    let mut appearance_sequence = 0;
    let mut messages = Vec::new();
    let until = tokio::time::Instant::now() + Duration::from_secs(5);
    while appearance_sequence <= cast_sequence || own_reference(client)? == original_reference {
        pump(client, &mut appearance_sequence, &mut messages).await?;
        if tokio::time::Instant::now() >= until {
            return Err("C16d Chameleon cast lacks changed own-actor appearance frame".into());
        }
    }
    paid_rune(client, chameleon_ref, chameleon_left, before, chameleon).await?;
    let mut quantities = [
        (chameleon_ref, chameleon_left),
        (disintegrate_ref, rune_quantity(client, disintegrate_ref)?),
        (target_ref, target_count - 1),
    ];
    tile_item_rejoin(client, clients, 0xe1, &quantities).await?;
    let expected = Look::Item {
        definition_ref: target_ref.get(),
    };
    let binding = own_binding(client)?;
    if binding.look != expected
        || binding.condition_revision == 0
        || admission_tile_contents(client, at)? != placed
    {
        return Err(
            "C16d fresh admission lost exact backpack appearance/condition or changed its target"
                .into(),
        );
    }
    row.pass("chameleon-payment-appearance", appearance_sequence);
    row.pass("chameleon-fresh-admission", client.last_server_sequence());
    row.pass(
        "disintegrate-floor-refusal",
        illegal_floor(client, disintegrate_ref).await?,
    );
    spell_ready(client, disintegrate).await?;
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("C16d Disintegrate vitals absent")?;
    let (cast_sequence, remaining) =
        cast_rune(client, disintegrate, disintegrate_ref, None, Some(at)).await?;
    let (sequence, removed) = tile_item_frame(client, at, target_ref, false, cast_sequence).await?;
    paid_rune(client, disintegrate_ref, remaining, before, disintegrate).await?;
    if removed != underlying {
        return Err("C16d Disintegrate removed or changed underlying protected ground".into());
    }
    row.pass("disintegrate-payment-removal", sequence);
    spell_ready(client, disintegrate).await?;
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("C16d empty tile vitals absent")?;
    let (_, remaining) = cast_rune(client, disintegrate, disintegrate_ref, None, Some(at)).await?;
    let sequence = inventory_quantity(client, disintegrate_ref, remaining).await?;
    paid_rune(client, disintegrate_ref, remaining, before, disintegrate).await?;
    let empty_sequence = sequence;
    quantities[1].1 = remaining;
    tile_item_rejoin(client, clients, 0xe2, &quantities).await?;
    if admission_tile_contents(client, at)? != underlying || own_binding(client)?.look != expected {
        return Err(
            "C16d removal not durable or active item illusion lost after its source was destroyed"
                .into(),
        );
    }
    row.pass("empty-tile-success", empty_sequence);
    row.pass("removal-fresh-admission", client.last_server_sequence());
    let active_reference = own_reference(client)?;
    appearance_sequence = 0;
    while own_reference(client)? == active_reference || appearance_sequence == 0 {
        if tokio::time::Instant::now() >= started + duration + Duration::from_secs(10) {
            return Err("C16d Chameleon did not expire on real server clock".into());
        }
        pump(client, &mut appearance_sequence, &mut messages).await?;
    }
    if tokio::time::Instant::now() + Duration::from_secs(1) < started + duration {
        return Err("C16d item illusion expired before its authored lifetime".into());
    }
    let expiry_sequence = appearance_sequence;
    tile_item_rejoin(client, clients, 0xe3, &quantities).await?;
    if own_binding(client)?.look != baseline || admission_tile_contents(client, at)? != underlying {
        return Err(
            "C16d expired item illusion or destroyed backpack returned on fresh admission".into(),
        );
    }
    row.pass("chameleon-natural-expiry", expiry_sequence);
    row.pass("expiry-fresh-admission", client.last_server_sequence());
    Ok(())
}

#[cfg(test)]
mod tile_item_tests {
    use super::*;
    #[test]
    fn every_tile_item_action_is_required_for_a_verdict() {
        for omitted in TILE_ITEM_ACTIONS {
            let mut row = Row::new("C16d", TILE_ITEM_ACTIONS);
            for action in TILE_ITEM_ACTIONS {
                if action != omitted {
                    row.pass(action, 1);
                }
            }
            assert!(!row.complete());
            row.pass(omitted, 2);
            assert!(row.complete());
        }
    }
    #[test]
    fn tile_item_verdict_rejects_invalid_receipts() {
        for (action, sequence) in [("buy-chameleon", 0), ("unknown", 1), ("buy-chameleon", 1)] {
            let mut row = Row::new("C16d", TILE_ITEM_ACTIONS);
            for required in TILE_ITEM_ACTIONS {
                row.pass(required, 1);
            }
            row.pass(action, sequence);
            assert!(!row.complete());
        }
    }
    #[test]
    fn item_illusion_uses_the_pinned_two_hundred_second_lifetime() -> TestResult {
        let book = native_spell_book()?;
        assert_eq!(
            tile_item_duration(book.rune(3178).ok_or("Chameleon absent")?)?,
            Duration::from_secs(200)
        );
        assert!(tile_item_duration(book.rune(3197).ok_or("Disintegrate absent")?).is_err());
        Ok(())
    }
}
