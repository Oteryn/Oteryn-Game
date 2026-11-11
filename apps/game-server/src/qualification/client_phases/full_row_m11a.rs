//! M11a — Levitate (`exani hur up` / `exani hur down`) full-row journey.
//!
//! Concrete player scenario, derived from the M11a audit/matrix row and the
//! `vertical_move` parameter-cast owner:
//! 1. Admit the node-boot character and read the server-owned spell catalogue;
//!    Levitate must be served at its canonical cast index and its availability
//!    must reflect the character's legitimately earned level/vocation state.
//! 2. Send the real player command `exani hur up`.  On a legal ledge the server
//!    must accept the speech/cast, debit exactly 50 mana, emit vitals and spatial
//!    deltas, and move the character one floor up and one tile forward.
//! 3. After the support cooldown, send `exani hur down`; the server must emit the
//!    same reply/delta classes and return the character to the original floor.
//! 4. Send the refusal command `exani hur sideways`; the server must refuse it
//!    without moving the actor or changing mana/soul.
//! 5. Log out, obtain a fresh Platform admission, and re-check the durable
//!    position, mana/soul, and served Levitate catalogue entry.  The spell has
//!    no durable condition of its own; the committed movement and cost are the
//!    durable state under test.
//!
//! This phase never adjusts level, vocation, mana, inventory, position, or the
//! database.  Speech is the native client command path for parameter spells.
use super::*;
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatSpeechMode, DevClientSession};
use oteryn_protocol_oteryn::{actor_spell_catalog as catalog, logout::LogoutResult};

const WORDS_UP: &str = "exani hur up";
const WORDS_DOWN: &str = "exani hur down";
const WORDS_REFUSAL: &str = "exani hur sideways";
const LEVITATE_CAST_INDEX: u32 = 130;

#[derive(Debug)]
struct Observation {
    command: &'static str,
    status: CommandStatus,
    disposition: ChatDisposition,
    before: oteryn_protocol_oteryn::actor_spell::ActorVitals,
    after: oteryn_protocol_oteryn::actor_spell::ActorVitals,
    before_position: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    after_position: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    result_sequence: u64,
    saw_vitals_delta: bool,
    saw_spatial_delta: bool,
    private_feedback: Vec<String>,
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete(journey(clients)).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err(
            "step=journey observed=timeout-90s server-needed=bounded-parameter-cast-reply".into(),
        ),
    };
    match result {
        Ok(evidence_text) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=M11a.full status=PASS seconds={:.3} evidence={evidence_text}",
            started.elapsed().as_secs_f64()
        )),
        Err(error) => {
            evidence(&format!("M11A_GAP {error}"));
            evidence(&format!(
                "SEAM_EVIDENCE e2e_all scenario=M11a.full status=FAIL seconds={:.3} evidence=observed-runtime-failure-see-M11A_GAP",
                started.elapsed().as_secs_f64()
            ));
        }
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<&'static str> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut client = connect_world(clients, 0xb1).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let levitate = levitate_entry(&mut client)?;
        let player_level = observed_level(&mut client).await?;
        let initial_session = client.join_snapshot().game_session_id;

        // Send every mechanic command even when the admitted character is not yet
        // eligible or a particular bundle tile refuses relocation.  This preserves
        // the observed server replies needed to diagnose a real row failure.
        let up = observe_speech(&mut client, WORDS_UP).await?;
        settle_support_cooldown(&mut client).await?;
        let down = observe_speech(&mut client, WORDS_DOWN).await?;
        settle_support_cooldown(&mut client).await?;
        let refusal = observe_speech(&mut client, WORDS_REFUSAL).await?;

        let before_logout_position = client.world_spatial().actor_position;
        let before_logout_vitals = client.actor_vitals().copied().ok_or(
            "step=5 observed=missing-vitals-before-logout server-needed=actor-vitals-snapshot",
        )?;
        logout(&mut client).await?;
        row_needs_logout_0 = false;
        client = connect_world(clients, 0xb2).await?;
        row_needs_logout_0 = true;
        let after_entry = levitate_entry(&mut client)?;
        let durable = client.join_snapshot().game_session_id != initial_session
            && client.world_spatial().actor_position == before_logout_position
            && client.actor_vitals() == Some(&before_logout_vitals)
            && after_entry == levitate;

        validate_success(&up, &down, &refusal, durable, &levitate, player_level)?;
        Ok("up-down-refusal-and-fresh-admission-frames-validated")
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        row_logout(&mut client).await?;
    }
    row_result
}

fn levitate_entry(client: &mut DevClientSession) -> TestResult<catalog::SpellCatalogEntry> {
    let snapshot = client
        .session_mut()
        .gated_snapshot(catalog::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("step=1 observed=missing-spell-catalog server-needed=served-own-actor-catalog")?;
    let entries = catalog::decode_catalog(&snapshot.payload).map_err(|error| {
        format!("step=1 observed=malformed-catalog-{error:?} server-needed=valid-catalog-frame")
    })?;
    let entry = entries
        .into_iter()
        .find(|entry| entry.name == "Levitate" && entry.words == "exani hur")
        .ok_or("step=1 observed=levitate-absent server-needed=served-levitate-entry")?;
    if entry.cast_index != LEVITATE_CAST_INDEX {
        return Err(format!(
            "step=1 observed=levitate-cast-index-{} expected={LEVITATE_CAST_INDEX} server-needed=canonical-catalogue-cast-index",
            entry.cast_index
        )
        .into());
    }
    Ok(entry)
}

async fn observed_level(client: &mut DevClientSession) -> TestResult<u32> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;

    let request = [wire::CharacterTab::GeneralStats as u8, 0, 0, 1];
    let reply = client
        .session_mut()
        .gated_command(wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1, 388, &request)
        .await?;
    if reply.status != CommandStatus::Accepted
        || reply.payload.len() != 42
        || reply.payload.first() != Some(&(wire::CharacterTab::GeneralStats as u8))
        || reply.payload[33] > 1
    {
        return Err(format!(
            "step=1 observed=level-query-{reply:?} server-needed=accepted-valid-general-stats"
        )
        .into());
    }
    Ok(u32::from_le_bytes(reply.payload[9..13].try_into()?))
}

async fn observe_speech(
    client: &mut DevClientSession,
    command: &'static str,
) -> TestResult<Observation> {
    let before = client.actor_vitals().copied().ok_or_else(|| {
        format!("step={command} observed=missing-pre-vitals server-needed=vitals-snapshot")
    })?;
    let before_position = client.world_spatial().actor_position;
    client.take_events();
    let reply = client
        .chat(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: command.into(),
        })
        .await?;
    client.service_liveness(Duration::from_millis(100)).await?;
    let events = client.take_events();
    let after = client.actor_vitals().copied().ok_or_else(|| {
        format!("step={command} observed=missing-post-vitals server-needed=vitals-state")
    })?;
    let after_position = client.world_spatial().actor_position;
    Ok(Observation {
        command,
        status: reply.status,
        disposition: reply.disposition,
        before,
        after,
        before_position,
        after_position,
        result_sequence: reply.result_server_sequence,
        saw_vitals_delta: events
            .iter()
            .any(|event| matches!(event, oteryn_dev_client::SessionEvent::ActorVitals(_))),
        saw_spatial_delta: events
            .iter()
            .any(|event| matches!(event, oteryn_dev_client::SessionEvent::WorldSpatial(_))),
        private_feedback: events
            .into_iter()
            .filter_map(|event| match event {
                oteryn_dev_client::SessionEvent::ChatLine(delta) => match delta.value {
                    oteryn_dev_client::ChatLine::Private { speaker_name, text }
                        if speaker_name == "Game" =>
                    {
                        Some(text)
                    }
                    _ => None,
                },
                _ => None,
            })
            .collect(),
    })
}

async fn settle_support_cooldown(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(2_100);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(LogoutResult::Accepted) {
        return Err(
            format!("step=5 observed=logout-{reply:?} server-needed=accepted-safe-logout").into(),
        );
    }
    Ok(())
}

fn validate_success(
    up: &Observation,
    down: &Observation,
    refusal: &Observation,
    durable: bool,
    levitate: &catalog::SpellCatalogEntry,
    player_level: u32,
) -> TestResult {
    if levitate.available != (player_level >= levitate.level) {
        return Err(format!(
            "step=1 observed=level-{player_level}-required-{}-available-{} server-needed=availability-matching-earned-level-and-vocation-state",
            levitate.level, levitate.available
        )
        .into());
    }
    for (step, observed) in [("2-up", up), ("3-down", down)] {
        let paid = observed.before.mana.checked_sub(50) == Some(observed.after.mana)
            && observed.before.soul == observed.after.soul;
        if observed.status != CommandStatus::Accepted
            || observed.disposition != ChatDisposition::Ok
            || observed.result_sequence == 0
            || !paid
            || !observed.saw_vitals_delta
            || !observed.saw_spatial_delta
        {
            return Err(format!(
                "step={step} command={} observed={observed:?} catalog_available={} server-needed=accepted-parameter-cast-with-vitals-and-spatial-deltas",
                observed.command, levitate.available
            ).into());
        }
    }
    let one_floor = up.after_position.floor.abs_diff(up.before_position.floor) == 1;
    let one_tile = (up.after_position.x.abs_diff(up.before_position.x) == 1
        && up.after_position.y == up.before_position.y)
        || (up.after_position.y.abs_diff(up.before_position.y) == 1
            && up.after_position.x == up.before_position.x);
    if player_level < levitate.level || !levitate.available || !one_floor || !one_tile {
        return Err(format!(
            "step=2-up command={} observed={up:?} player_level={player_level} required_level={} catalog_available={} server-needed=eligible-cast-moving-exactly-one-floor-and-one-forward-tile",
            up.command, levitate.level, levitate.available
        )
        .into());
    }
    if down.before_position != up.after_position
        || down.after_position.floor != up.before_position.floor
    {
        return Err(format!(
            "step=3-down command={} original={:?} observed={down:?} server-needed=return-to-original-floor",
            down.command, up.before_position
        )
        .into());
    }
    if refusal.status != CommandStatus::Rejected
        || refusal.disposition != ChatDisposition::Rejected
        || refusal.result_sequence == 0
        || refusal.before_position != refusal.after_position
        || refusal.before != refusal.after
        || refusal.saw_vitals_delta
        || refusal.saw_spatial_delta
        || refusal.private_feedback.is_empty()
    {
        return Err(format!(
            "step=4-refusal command={} observed={refusal:?} server-needed=side-effect-free-invalid-parameter-refusal",
            refusal.command
        ).into());
    }
    if !durable {
        return Err("step=5 observed=fresh-admission-state-mismatch server-needed=durable-position-cost-and-catalog".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_commands_are_exact_and_distinct() {
        assert_eq!(WORDS_UP, "exani hur up");
        assert_eq!(WORDS_DOWN, "exani hur down");
        assert_eq!(WORDS_REFUSAL, "exani hur sideways");
        assert_ne!(WORDS_UP, WORDS_DOWN);
    }
}
