//! Client replay capture for SCENARIO_MATRIX C17b.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent, native_session as oteryn_session};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut trace = CloseTrace::new();
    let caster_wire = trace.caster.clone();
    let observer_wire = trace.observer.clone();
    let result = verdict::observe(caster_wire.observe(journey(clients, &caster_wire, &observer_wire))).await;
    trace.finished = true;
    trace.report(result.as_ref().err().map(|error| error.as_ref()));
    match result {
        Ok(()) => evidence(
            "e2e_all scenario=C17b.client status=PASS evidence=live-cast-and-visibility-captured",
        ),
        Err(error) => {
            evidence(&format!(
                "SEAM_EVIDENCE e2e_all scenario=C17b.client status=FAIL evidence={error}"
            ));
        }
    }
}

// Only C17b.client consumes these live diagnostics. Replay takes its own
// immutable prefix from the observer capture before relog.
struct CloseTrace {
    caster: oteryn_session::test_support::WireCapture,
    observer: oteryn_session::test_support::WireCapture,
    started: tokio::time::Instant,
    finished: bool,
}

impl CloseTrace {
    fn new() -> Self {
        Self {
            caster: Default::default(),
            observer: Default::default(),
            started: tokio::time::Instant::now(),
            finished: false,
        }
    }

    fn report(&self, mut error: Option<&(dyn std::error::Error + 'static)>) {
        let detail = error.map(ToString::to_string);
        let mut side = if self.finished || detail.is_some() {
            "local_scope_exit"
        } else {
            "local_completed_or_cancelled"
        };
        let mut client_timeout = None;
        while let Some(cause) = error {
            if let Some(io) = cause.downcast_ref::<std::io::Error>() {
                side = match io.kind() {
                    std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::ConnectionReset => "peer_or_transport",
                    _ => "transport_unknown",
                };
            }
            if let Some(oteryn_session::SessionError::Timeout(stage)) = cause.downcast_ref() {
                client_timeout = Some(*stage);
            }
            if let Some(oteryn_dev_client::DevClientError::Timeout(stage)) = cause.downcast_ref() {
                client_timeout = Some(*stage);
            }
            error = cause.source();
        }
        for (connection, capture) in [("caster", &self.caster), ("observer", &self.observer)] {
            use oteryn_protocol_oteryn::decode_wire_envelope;
            let (received, sent) = capture.take();
            let frame = received.last().and_then(|raw| decode_wire_envelope(raw).ok())
                .map(|frame| (frame.connection_generation(), frame.server_sequence(), frame.message_type()));
            let command = sent.last().and_then(|raw| decode_wire_envelope(raw).ok())
                .and_then(|frame| frame.client_command(frame.connection_generation()).ok())
                .map(|command| (command.command_id, command.command_type));
            evidence(&format!("C17B_CLIENT_CLOSE connection={connection} row_end={side} last_command_sent={command:?} last_frame_received={frame:?} client_timeout={client_timeout:?} elapsed_ms={} detail={detail:?}", self.started.elapsed().as_millis()));
        }
    }
}

impl Drop for CloseTrace {
    fn drop(&mut self) {
        if !self.finished {
            self.report(None);
        }
    }
}

fn own(client: &DevClientSession) -> TestResult<oteryn_dev_client::EntityRef> {
    let entities = client
        .world_entities()
        .ok_or("C17b.client entity snapshot absent")?;
    entities
        .iter()
        .find(|entity| &entity.entity.identity == entities.own_identity())
        .map(|entity| entity.entity)
        .ok_or_else(|| "C17b.client own entity absent".into())
}

async fn journey(clients: &SeamClients<'_>, caster_capture: &oteryn_session::test_support::WireCapture, capture: &oteryn_session::test_support::WireCapture) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let fixture = character_prep::dedicated_character(clients, "NODE_BOOT_C17B_ACCOUNT").await?;
    let caster_clients = fixture.clients(clients);
    let book = full_row_c17b::spell_book()?;
    let spell = book
        .spoken("utana vid")
        .ok_or("C17b.client spell absent")?
        .spell;
    let mut caster =
        full_row_c17b::seeded_caster(&caster_clients, "C17b.client", 0xc1, spell).await?;
    let mut observer = capture.observe(connect_character(clients, 0xc2, 0)).await?;
    full_row_c17b::Staging::new(world, &metadata.spawns)
        .encounter(&mut caster, &mut observer, capture)
        .await?;
    let rat = full_row_c17b::place_rat(&mut caster).await?;
    super::mp::observations::while_servicing_liveness(
        full_row_c17b::wait_for_rat_hit(&mut caster, rat),
        async || {
            capture
                .observe(observer.service_liveness(Duration::from_millis(25)))
                .await?;
            observer.take_events();
            Ok(())
        },
    )
    .await??;
    let caster_ref = own(&caster)?;
    let entities = observer
        .world_entities()
        .ok_or("C17b.client observer entity snapshot absent")?;
    if entities.get(&caster_ref).is_none() {
        return Err("C17b.client caster was not initially visible".into());
    }
    observer.take_events();
    if full_row_c17b::invisibility_remaining(&mut caster)
        .await?
        .is_some()
    {
        return Err("C17b caster was already invisible before first cast".into());
    }
    let mana_before = caster
        .actor_vitals()
        .ok_or("C17b pre-cast vitals absent")?
        .mana;
    let spell = full_row_c17b::spell_index(&book)?;
    let cast_sent = tokio::time::Instant::now();
    let (cast, frames) = oteryn_session::test_support::capture_command_results(caster.cast_spell(
        spell,
        SpellTarget::None,
        false,
    ))
    .await;
    let cast = cast?;
    require_mechanic_acceptance(cast.status, "C17b.client Invisibility")?;
    if cast.disposition != SpellCastDisposition::Cast {
        return Err(format!("C17b.client Invisibility cast refused: {cast:?}").into());
    }
    let paid = cast.actor_vitals_delta.as_ref().ok_or("C17b cast has no mana delta")?;
    if mana_before.checked_sub(440) != Some(paid.value.mana) {
        return Err("C17b cast did not debit 440 mana".into());
    }
    let started = tokio::time::Instant::now();
    let cooldown = caster.cast_spell(spell, SpellTarget::None, false).await?;
    let mut events = caster.take_events();
    full_row_c17b::verify_recast(&cooldown, paid.value, paid.server_sequence, caster.actor_vitals().copied(), &events, started.elapsed())?;
    use oteryn_protocol_oteryn::presentation::ActorCooldownKind;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let support = support_group_ordinal()?;
    loop {
        events.extend(caster.take_events());
        if let Some(projection) = events.drain(..).find_map(|event| match event {
            SessionEvent::ActorCooldowns(delta) if delta.server_sequence > cast.result_server_sequence
                && delta.value.entries.iter().any(|entry| entry.kind == ActorCooldownKind::Spell && entry.id == spell.get()) => Some(delta.value),
            _ => None,
        }) {
            verify_cooldown_projection(&projection, spell.get(), support, cast_sent.elapsed())?;
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C17b missing Invisible spell cooldown projection".into());
        }
        caster.service_liveness(Duration::from_millis(25)).await?;
    }
    let initial = full_row_c17b::invisibility_remaining(&mut caster)
        .await?
        .ok_or("C17b cast did not install Invisible")?;
    verify_timer("Invisible", u64::try_from(initial)?, 200_000, 195_000, cast_sent.elapsed())?;
    full_row_c17b::require_rat_stops(&mut caster, rat).await?;
    capture
        .observe(observer.service_liveness(Duration::from_millis(100)))
        .await?;
    if observer.take_events().into_iter().any(|event| matches!(event,
        SessionEvent::WorldSpatialEntities(delta) if delta.value.leave.iter().any(|entity| entity.identity == caster_ref.identity)))
        || observer.world_entities().is_none_or(|view| view.get(&caster_ref).is_none()) {
        return Err("Global parity: Invisible removed the caster from the other player's view".into());
    }
    let observer_sequence = observer.last_server_sequence();
    let observer_session = uuid_text(observer.join_snapshot().game_session_id.as_bytes());
    let (received, commands) = capture.take();
    let caster_session = uuid_text(caster.join_snapshot().game_session_id.as_bytes());
    let position = caster.world_spatial().actor_position;
    observer.take_events();
    use tokio::time::Instant;
    let (durable_vitals, before, before_sequence, before_sent, before_received, logout_sent, logout_received, logout_sequence) =
        capture.observe(full_e2ef5::with_observer_liveness(&mut observer, caster_capture.observe(async {
            full_row_c17b::wait_rat_combat_end(&mut caster).await?;
            let sent = Instant::now();
            let (before, sequence) = full_row_c17b::invisibility_sample(&mut caster).await?;
            let received = Instant::now();
            let vitals = caster.actor_vitals().ok_or("C17b logout vitals absent")?;
            let durable = (vitals.mana, vitals.soul);
            let logout_sent = Instant::now();
            let result = caster.session_mut().logout().await?;
            let logout_received = Instant::now();
            require_mechanic_acceptance(result.status, "C17b first logout")?;
            if result.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
                || result.result_server_sequence <= sequence {
                return Err(format!("C17b first logout invalid after condition sequence {sequence}: {result:?}").into());
            }
            Ok((durable, before.ok_or("C17b Invisible absent before logout")?, sequence, sent, received, logout_sent, logout_received, result.result_server_sequence))
        }))).await?;
    if durable_vitals.0 >= mana_before {
        return Err("C17b spent mana did not persist until logout".into());
    }
    capture.observe(full_row_c17b::wait_visible(&mut observer, caster_ref, false)).await?;
    // Keep the character definitely offline long enough to distinguish a
    // paused timer from one running across the admission round trips.
    capture.observe(full_e2ef5::with_observer_liveness(&mut observer, caster_capture.observe(async {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        Ok(())
    }))).await?;
    let login_sent = Instant::now();
    caster = capture.observe(full_e2ef5::with_observer_liveness(
        &mut observer,
        caster_capture.observe(connect_character(&caster_clients, 0xc3, 0)),
    )).await?;
    let login_received = Instant::now();
    let admission_sequence = caster.last_server_sequence();
    let relog_session = uuid_text(caster.join_snapshot().game_session_id.as_bytes());
    let admission_position = caster.world_spatial().actor_position;
    let admission_vitals = caster.actor_vitals().map(|vitals| (vitals.mana, vitals.soul));
    let fresh_ref = own(&caster)?;
    evidence(&format!("C17B_CLIENT_ADMISSION old_session={caster_session} new_session={relog_session} old_entity={caster_ref:?} new_entity={fresh_ref:?} expected_position={position:?} actual_position={admission_position:?} expected_mana_soul={durable_vitals:?} actual_mana_soul={admission_vitals:?} sequence={admission_sequence}"));
    let after_sent = Instant::now();
    let (remaining, after_sequence) = full_row_c17b::invisibility_sample(&mut caster).await?;
    let after_received = Instant::now();
    let online_min = (logout_sent - before_received) + (after_sent - login_received);
    let online_max = (logout_received - before_sent) + (after_received - login_sent);
    let offline_min = login_sent - logout_received;
    let remaining = remaining.ok_or("C17b Invisible missing after relog")?;
    evidence(&format!("C17B_CLIENT_CONDITION before_sequence={before_sequence} logout_sequence={logout_sequence} admission_sequence={admission_sequence} after_sequence={after_sequence} before_ms={before} after_ms={remaining} online_min_us={} online_max_us={} offline_min_us={}", online_min.as_micros(), online_max.as_micros(), offline_min.as_micros()));
    verify_condition_pause(before, remaining, online_min, online_max, offline_min)?;
    if caster_session == relog_session || fresh_ref == caster_ref
        || admission_position != position || admission_vitals != Some(durable_vitals)
        || after_sequence <= admission_sequence {
        return Err("C17b relog admission state mismatch; see C17B_CLIENT_ADMISSION".into());
    }
    capture.observe(full_row_c17b::wait_visible(&mut observer, fresh_ref, true)).await?;
    evidence(&format!(
        "C17B_CLIENT_RELOG fresh_session=true paid_mana={} invisible_before_ms={initial} remaining_ms={remaining}",
        paid.value.mana
    ));
    use oteryn_protocol_oteryn::{decode_command_result, decode_wire_envelope};
    let [frame] = frames.as_slice() else {
        return Err("C17b expected one original cast reply".into());
    };
    let frame = decode_wire_envelope(frame)?;
    let reply = decode_command_result(frame.payload())?;
    if reply.command_id != cast.command_id
        || reply.status != cast.status
        || frame.server_sequence() != cast.result_server_sequence
    {
        return Err("C17b original cast reply correlation mismatch".into());
    }
    let root = std::path::PathBuf::from(required("E2E_ALL_CAPTURE_DIR")?);
    let run = root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("C17b invalid capture directory")?;
    let metadata = serde_json::json!({
        "run": run, "caster": caster_session, "observer": observer_session,
        "cast_command": cast.command_id, "cast_sequence": cast.result_server_sequence,
        "observer_sequence": observer_sequence, "identity": caster_ref.identity,
        "generation": caster_ref.generation, "observer_character": clients.characters[0],
        "received": received, "commands": commands, "cast_frame": frames[0],
    });
    for (name, value) in [
        (
            "c17b-cast.json",
            envelope(
                &caster_session,
                Some(reply.command_id),
                frame.server_sequence(),
                Some(reply.status as u32),
                reply.payload,
            ),
        ),
        ("c17b-expected.json", metadata),
    ] {
        use std::io::Write as _;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(name))?;
        output.write_all(&serde_json::to_vec(&value)?)?;
    }
    Ok(())
}

fn envelope(
    session: &str,
    command: Option<u64>,
    sequence: u64,
    status: Option<u32>,
    payload: &[u8],
) -> serde_json::Value {
    serde_json::json!({"command_id": command, "game_session_id": session,
        "payload": payload, "server_sequence": sequence, "status": status})
}

fn support_group_ordinal() -> TestResult<u32> {
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tools/content-schema/spell-authoring/cooldown-groups.json"
    ))?;
    let index = document["groups"].as_array()
        .and_then(|groups| groups.iter().position(|group| group["key"] == "support"))
        .ok_or("C17b Support cooldown group missing from the wire catalogue")?;
    Ok(u32::try_from(index + 1)?)
}

fn verify_cooldown_projection(
    projection: &oteryn_protocol_oteryn::presentation::ActorCooldowns,
    spell: u32,
    support: u32,
    elapsed: std::time::Duration,
) -> TestResult {
    use oteryn_protocol_oteryn::presentation::ActorCooldownKind;
    let expiry = |kind, id| projection.entries.iter()
        .find(|entry| entry.kind == kind && entry.id == id)
        .map(|entry| entry.expires_at_ms)
        .ok_or("C17b spell or Support cooldown projection missing");
    let spell_expiry = expiry(ActorCooldownKind::Spell, spell)?;
    let support_expiry = expiry(ActorCooldownKind::Group, support)?;
    if spell_expiry != support_expiry {
        return Err(format!("C17b spell and Support deadlines differ: {spell_expiry} != {support_expiry}").into());
    }
    let remaining = spell_expiry.checked_sub(projection.server_now_ms)
        .ok_or("C17b expired cooldown projection")?;
    verify_timer("spell and Support cooldown", remaining, 2_000, 1_500, elapsed)
}

fn verify_timer(label: &str, remaining: u64, duration: u64, floor: u64, elapsed: std::time::Duration) -> TestResult {
    let elapsed_ms = u64::try_from(elapsed.as_micros().div_ceil(1_000))?;
    let min = floor.max(duration.saturating_sub(elapsed_ms));
    if !(min..=duration).contains(&remaining) {
        return Err(format!("C17b {label} remaining={remaining}ms expected={min}..={duration}ms cast_elapsed={elapsed_ms}ms").into());
    }
    Ok(())
}

fn verify_condition_pause(
    before: i64,
    after: i64,
    online_min: std::time::Duration,
    online_max: std::time::Duration,
    offline_min: std::time::Duration,
) -> TestResult {
    // The status response quantizes the owner clock to whole milliseconds.
    let min_ms = i64::try_from(online_min.as_millis())?.saturating_sub(1).max(0);
    let max_ms = i64::try_from(online_max.as_micros().div_ceil(1_000))? + 1;
    if i64::try_from(offline_min.as_millis())? <= max_ms - min_ms {
        return Err("C17b offline interval cannot distinguish a running clock from a paused clock".into());
    }
    let lost = before.checked_sub(after).ok_or("C17b condition time overflow")?;
    if lost > max_ms {
        return Err(format!("BLOCKER-SERVER C17b condition lost {lost} ms with at most {max_ms} ms online").into());
    }
    if after <= 0 || lost < min_ms {
        return Err(format!("C17b condition lost {lost} ms with at least {min_ms} ms online").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn initial_timers_reject_shortened_lifetimes_and_missing_support() -> TestResult {
        use oteryn_protocol_oteryn::presentation::{ActorCooldown, ActorCooldownKind, ActorCooldowns};
        let elapsed = Duration::from_millis(50);
        let mut observation = ActorCooldowns {
            server_now_ms: 1_000, rtt_ms: None,
            entries: vec![ActorCooldown { kind: ActorCooldownKind::Spell, id: 1, expires_at_ms: 2_980 }],
        };
        assert!(verify_cooldown_projection(&observation, 1, 3, elapsed).is_err());
        observation.entries.push(ActorCooldown { kind: ActorCooldownKind::Group, id: 3, expires_at_ms: 2_980 });
        verify_cooldown_projection(&observation, 1, 3, elapsed)?;
        observation.entries[1].expires_at_ms = 2_979;
        assert!(verify_cooldown_projection(&observation, 1, 3, elapsed).is_err());
        assert!(verify_timer("Invisible", 196_000, 200_000, 195_000, elapsed).is_err());
        assert!(verify_timer("cooldown", 1_500, 2_000, 1_500, elapsed).is_err());
        Ok(())
    }

    #[test]
    fn condition_pause_rejects_offline_decay_and_a_reset() -> TestResult {
        let min = Duration::from_secs(2);
        let max = Duration::from_secs(3);
        let offline = Duration::from_secs(10);
        verify_condition_pause(198_000, 195_500, min, max, offline)?;
        assert!(verify_condition_pause(198_000, 185_500, min, max, offline).is_err());
        assert!(verify_condition_pause(198_000, 200_000, min, max, offline).is_err());
        assert!(verify_condition_pause(198_000, 195_500, min, max, Duration::from_secs(1)).is_err());
        Ok(())
    }
}
