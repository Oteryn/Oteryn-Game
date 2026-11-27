//! Client replay capture for SCENARIO_MATRIX C17b.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent, native_session as oteryn_session};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::observe(journey(clients)).await {
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

async fn journey(clients: &SeamClients<'_>) -> TestResult {
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
    let capture = oteryn_session::test_support::WireCapture::default();
    let mut observer = capture.observe(connect_character(clients, 0xc2, 0)).await?;
    full_row_c17b::Staging::new(world, &metadata.spawns)
        .encounter(&mut caster, &mut observer, &capture)
        .await?;
    let rat = full_row_c17b::place_rat(&caster).await?;
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
    let paid = cast
        .actor_vitals_delta
        .as_ref()
        .ok_or("C17b cast has no mana delta")?
        .value;
    if mana_before.checked_sub(440) != Some(paid.mana) {
        return Err("C17b cast did not debit 440 mana".into());
    }
    let cooldown = caster.cast_spell(spell, SpellTarget::None, false).await?;
    if cooldown.disposition != SpellCastDisposition::CoolingDown
        || cooldown.actor_vitals_delta.is_some()
        || caster
            .actor_vitals()
            .is_none_or(|vitals| vitals.mana != paid.mana)
    {
        return Err("C17b immediate recast did not refuse without charging".into());
    }
    use oteryn_protocol_oteryn::presentation::ActorCooldownKind;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let cooldown_ms = loop {
        if let Some(remaining) = caster.take_events().into_iter().find_map(|event| {
            let SessionEvent::ActorCooldowns(delta) = event else {
                return None;
            };
            let cooldown =
                delta.value.entries.iter().find(|entry| {
                    entry.kind == ActorCooldownKind::Spell && entry.id == spell.get()
                })?;
            cooldown
                .expires_at_ms
                .checked_sub(delta.value.server_now_ms)
        }) {
            break remaining;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C17b missing Invisible spell cooldown projection".into());
        }
        caster.service_liveness(Duration::from_millis(25)).await?;
    };
    if !(1500..=2000).contains(&cooldown_ms) {
        return Err(format!(
            "C17b Invisible cooldown projection {cooldown_ms}ms outside Global range"
        )
        .into());
    }
    let initial = full_row_c17b::invisibility_remaining(&mut caster)
        .await?
        .ok_or("C17b cast did not install Invisible")?;
    if !(195_000..=200_000).contains(&initial) {
        return Err(
            format!("C17b initial invisibility duration {initial}ms outside Global range").into(),
        );
    }
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
    let durable_vitals = full_e2ef5::with_observer_liveness(
        &mut observer,
        full_row_c17b::logout_after_rat_combat(&mut caster),
    )
    .await?;
    if durable_vitals.0 >= mana_before {
        return Err("C17b spent mana did not persist until logout".into());
    }
    full_row_c17b::wait_visible(&mut observer, caster_ref, false).await?;
    caster = full_e2ef5::with_observer_liveness(
        &mut observer,
        connect_character(&caster_clients, 0xc3, 0),
    )
    .await?;
    let fresh_ref = own(&caster)?;
    if fresh_ref == caster_ref {
        return Err("C17b fresh admission reused entity incarnation".into());
    }
    full_row_c17b::wait_visible(&mut observer, fresh_ref, true).await?;
    let relog_session = uuid_text(caster.join_snapshot().game_session_id.as_bytes());
    let remaining = full_row_c17b::invisibility_remaining(&mut caster)
        .await?
        .ok_or("C17b Invisible missing after relog")?;
    if caster_session == relog_session
        || caster.world_spatial().actor_position != position
        || caster
            .actor_vitals()
            .is_none_or(|vitals| (vitals.mana, vitals.soul) != durable_vitals)
        || remaining <= 0
        || remaining >= initial
    {
        return Err(format!(
            "C17b relog state mismatch invisible_before_ms={initial} after_ms={remaining}"
        )
        .into());
    }
    evidence(&format!(
        "C17B_CLIENT_RELOG fresh_session=true paid_mana={} invisible_before_ms={initial} remaining_ms={remaining}",
        paid.mana
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
