//! CLIENT-C17d capture: cast the real Challenge spell beside a rat which is attacking a second
//! player, then retain the exact ACTOR_VITALS and WORLD_PRESENTATION domain payloads consumed by
//! `Session::exchange_cast` (`crates/session/src/session_exchange.rs`) and
//! `presentation::apply` (`crates/session/src/presentation.rs`).
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget, encode_actor_vitals};
use oteryn_protocol_oteryn::presentation::{PresentationEventKind, encode_presentation_batch};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    match result {
        Ok(Ok(())) => evidence("SEAM_EVIDENCE e2e_all scenario=C17d.client status=PASS"),
        Ok(Err(error)) => evidence(&format!("SEAM_EVIDENCE e2e_all scenario=C17d.client status=FAIL evidence={error}")),
        Err(_) => evidence("SEAM_EVIDENCE e2e_all scenario=C17d.client status=FAIL evidence=journey-timeout"),
    }
}

fn spell_book() -> TestResult<crate::spell::SpellBook> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(
        &root.join("content/spells.manifest.json"),
    )?;
    let catalog = crate::spell::executable_catalog::compile(&input.catalog.bytes, &input.catalog.sha256)?;
    let selections = crate::spell::executable_catalog::compile_source_selection(
        &input.source_selection.bytes, &input.source_selection.sha256, &catalog,
    )?;
    Ok(catalog.into_spell_book_with_selections(&selections)?)
}

fn challenge_index(book: &crate::spell::SpellBook) -> TestResult<std::num::NonZeroU32> {
    let spell = book.spoken("exeta res").ok_or("Challenge absent")?.spell;
    (1..=book.source_len())
        .filter_map(|index| u32::try_from(index).ok().and_then(std::num::NonZeroU32::new))
        .find(|index| book.indexed(*index).is_some_and(|entry| entry.key == spell.key))
        .ok_or_else(|| "Challenge has no served index".into())
}

fn own(client: &DevClientSession) -> TestResult<oteryn_dev_client::EntityRef> {
    let entities = client.world_entities().ok_or("entity view absent")?;
    entities.iter().find(|entry| &entry.entity.identity == entities.own_identity())
        .map(|entry| entry.entity).ok_or_else(|| "own entity absent".into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let mut victim = connect_character(clients, 0xd4, 1).await?;
    let mut caster = connect_character(clients, 0xd5, 0).await?;
    travel_to_spawn(&mut victim, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let victim_at = tile(victim.world_spatial().actor_position)?;
    let path = route(world, tile(caster.world_spatial().actor_position)?, |at| {
        at.floor == victim_at.floor && at.x.abs_diff(victim_at.x).max(at.y.abs_diff(victim_at.y)) <= 1
    }).ok_or("no route for Challenge caster")?;
    walk(&mut caster, world, &path).await?;
    let rat = caster.world_entities().and_then(|entities| entities.iter().find(|entry| {
        entry.kind == oteryn_dev_client::EntityKind::Creature
            && entry.position.floor == caster.world_spatial().actor_position.floor
            && entry.position.x.abs_diff(caster.world_spatial().actor_position.x)
                .max(entry.position.y.abs_diff(caster.world_spatial().actor_position.y)) <= 1
    })).map(|entry| entry.entity).ok_or("adjacent rat absent")?;
    let caster_ref = own(&caster)?;
    caster.take_events();
    victim.take_events();
    let book = spell_book()?;
    let before = caster.actor_vitals().copied().ok_or("caster vitals absent")?;
    let cast = caster.cast_spell(challenge_index(&book)?, SpellTarget::None, false).await?;
    if cast.status != CommandStatus::Accepted || cast.disposition != SpellCastDisposition::Cast {
        return Err(format!("Challenge refused: {cast:?}").into());
    }
    let paid = cast.actor_vitals_delta.as_ref().ok_or("Challenge omitted vitals delta")?.value;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    let mut captured = None;
    while tokio::time::Instant::now() < deadline && captured.is_none() {
        caster.service_liveness(Duration::from_millis(100)).await?;
        victim.service_liveness(Duration::from_millis(20)).await?;
        for event in caster.take_events() {
            if let SessionEvent::Presentation(delta) = event
                && delta.value.iter().any(|event| matches!(event.kind,
                    PresentationEventKind::ValueText { attacker: Some(source), target, primary, .. }
                    if source == rat && target == caster_ref && primary.value > 0))
            {
                captured = Some(delta.value);
                break;
            }
        }
    }
    let presentation = captured.ok_or("challenged rat did not damage caster")?;
    let damage = presentation.iter().find_map(|event| match event.kind {
        PresentationEventKind::ValueText { attacker: Some(source), target, primary, .. }
            if source == rat && target == caster_ref && primary.value > 0 => Some(primary.value),
        _ => None,
    }).ok_or("attributed damage absent")?;
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = Path::new(&directory);
        let writes = [
            ("c17d-vitals.bin", encode_actor_vitals(&paid).map_err(|error| format!("vitals codec: {error:?}"))?),
            ("c17d-presentation.bin", encode_presentation_batch(&presentation).map_err(|error| format!("presentation codec: {error:?}"))?),
            ("c17d-expected.txt", format!("before_mana {}\nmana {}\ndamage {}\n", before.mana, paid.mana, damage).into_bytes()),
        ];
        for (name, bytes) in writes {
            if let Err(error) = std::fs::write(directory.join(name), bytes) {
                evidence(&format!("C17d.client capture-write-error file={name} error={error}"));
            }
        }
    }
    Ok(())
}
