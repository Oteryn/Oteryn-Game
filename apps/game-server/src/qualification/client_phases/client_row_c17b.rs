//! Client replay capture for SCENARIO_MATRIX C17b.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent, native_session as oteryn_session};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::observe(journey(clients)).await {
        Ok(()) => evidence(
            "e2e_capture scenario=C17b.client status=CAPTURED evidence=live-cast-and-visibility",
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

fn spell_index() -> TestResult<std::num::NonZeroU32> {
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
    let book = catalog.into_spell_book_with_selections(&selections)?;
    (1..=book.source_len())
        .filter_map(|index| {
            u32::try_from(index)
                .ok()
                .and_then(std::num::NonZeroU32::new)
        })
        .find(|index| {
            book.indexed(*index)
                .is_some_and(|spell| spell.name.eq_ignore_ascii_case("Invisibility"))
        })
        .ok_or_else(|| "C17b.client Invisibility spell absent".into())
}

#[derive(Default)]
struct ObserverCapture {
    snapshot: Option<(u64, oteryn_session::GatedSnapshot)>,
    deltas: Vec<oteryn_session::RawReplayDelta>,
}

impl ObserverCapture {
    fn observe(&mut self, client: &mut DevClientSession) -> TestResult {
        let session = client.session_mut();
        let (sequence, snapshot) = oteryn_session::test_support::visibility_snapshot(session)
            .ok_or("C17b original visibility snapshot missing")?;
        if self.snapshot.as_ref().is_none_or(|(previous, state)| {
            *previous != sequence || state.revision != snapshot.revision
        }) {
            self.snapshot = Some((sequence, snapshot.clone()));
            self.deltas.clear();
        }
        let previous = self
            .deltas
            .last()
            .map_or(sequence, |delta| delta.server_sequence);
        for delta in session
            .raw_world_entity_deltas()
            .iter()
            .filter(|delta| delta.server_sequence > previous)
        {
            let base = self
                .deltas
                .last()
                .map_or(snapshot.revision, |previous| previous.new_revision);
            if delta.base_revision != base {
                return Err("C17b original visibility history has a revision gap".into());
            }
            self.deltas.push(delta.clone());
        }
        Ok(())
    }
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
    let mut observer = connect_character(clients, 0xc2, 0).await?;
    let mut capture = ObserverCapture::default();
    full_row_c17b::Staging::new(world, &metadata.spawns)
        .encounter(&mut caster, &mut observer, |client| capture.observe(client))
        .await?;
    let caster_ref = own(&caster)?;
    let entities = observer
        .world_entities()
        .ok_or("C17b.client observer entity snapshot absent")?;
    if entities.get(&caster_ref).is_none() {
        return Err("C17b.client caster was not initially visible".into());
    }
    observer.take_events();
    let spell = spell_index()?;
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
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let removed = loop {
        observer.service_liveness(Duration::from_millis(50)).await?;
        capture.observe(&mut observer)?;
        let found = observer
            .take_events()
            .into_iter()
            .find_map(|event| match event {
                SessionEvent::WorldSpatialEntities(delta)
                    if delta.value.leave.contains(&caster_ref) =>
                {
                    Some(delta)
                }
                _ => None,
            });
        if let Some(delta) = found {
            break delta;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C17b.client visibility leave delta absent".into());
        }
    };
    if observer
        .world_entities()
        .is_none_or(|view| view.get(&caster_ref).is_some())
    {
        return Err("C17b.client live reducer retained invisible caster".into());
    }
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
    let caster_session = uuid_text(caster.join_snapshot().game_session_id.as_bytes());
    let observer_session = uuid_text(observer.join_snapshot().game_session_id.as_bytes());
    let session = observer.session_mut();
    let (baseline_sequence, snapshot) = capture
        .snapshot
        .ok_or("C17b original visibility snapshot missing")?;
    let deltas = capture.deltas.iter()
        .take_while(|delta| delta.server_sequence <= removed.server_sequence)
        .map(|delta| serde_json::json!({
            "base": delta.base_revision, "revision": delta.new_revision, "delta_type": delta.delta_type,
            "frame": envelope(&observer_session, None, delta.server_sequence, None, &delta.payload),
        })).collect::<Vec<_>>();
    let metadata = serde_json::json!({
        "run": run, "caster": caster_session, "observer": observer_session,
        "cast_command": cast.command_id, "cast_sequence": cast.result_server_sequence,
        "leave_sequence": removed.server_sequence, "identity": caster_ref.identity,
        "generation": caster_ref.generation, "selected": session.selected_capabilities(),
        "baseline_revision": snapshot.revision, "deltas": deltas,
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
        (
            "c17b-snapshot.json",
            envelope(
                &observer_session,
                None,
                baseline_sequence,
                None,
                &snapshot.payload,
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
