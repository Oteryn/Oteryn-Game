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
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let fixture = character_prep::dedicated_character(clients, "NODE_BOOT_F5_C17D_ACCOUNT").await?;
    let (accounts, characters) = fixture.replace_slot(clients, 0)?;
    let clients = &SeamClients { accounts: &accounts, characters: &characters, ..*clients };
    character_prep::declare_seeded("C17d.client", "knight,level-20,premium", "spell access inputs");
    let mut caster = character_prep::Seeded {
        profile: "f5-c17d",
        reason: "Challenge tests retargeting rather than leveling",
    }.connect(clients, 0xd5, 0).await?;
    character_prep::grant_premium(&caster).await?;
    let mut victim = connect_character(clients, 0xd4, 1).await?;
    let open = |at: TilePos| {
        world.base().tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.walkable() && !tile.protection_zone())
    };
    let from = tile(victim.world_spatial().actor_position)?;
    let caster_at = tile(caster.world_spatial().actor_position)?;
    let path = route_avoiding(world, from, |at| {
        at.floor == from.floor && open(at)
            && [StepDirection::North, StepDirection::East, StepDirection::South, StepDirection::West]
                .into_iter()
                .filter(|direction| destination(world, at, *direction)
                    .is_some_and(|next| next.floor == at.floor && open(next)))
                .count() >= 2
    }, |at| at != caster_at).ok_or("no nearby open Challenge staging ground")?;
    evidence(&format!("C17d.client victim staging steps={}", path.len()));
    full_e2ef5::with_observer_liveness(&mut caster, walk(&mut victim, world, &path)).await?;
    let victim_at = tile(victim.world_spatial().actor_position)?;
    let session = uuid_text(victim.join_snapshot().game_session_id.as_bytes());
    let answer = request_spawn(session, "oteryn:creature.rat".into()).await?;
    let rat = parse_spawned_entity(&answer)?;
    evidence(&format!("C17d.client FIXTURE rat={rat:?} victim_at={victim_at:?}"));
    let rat_at = full_e2ef5::with_observer_liveness(&mut caster, async {
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if let Some(entity) = victim.world_entities().and_then(|entities| entities.get(&rat)) {
                    return tile(entity.position);
                }
                victim.service_liveness(Duration::from_millis(50)).await?;
            }
        }).await.map_err(|_| "fixture rat not visible to victim")?
    }).await?;
    let path = route_avoiding(world, caster_at, |at| {
        open(at) && at.floor == rat_at.floor
            && at.x.abs_diff(rat_at.x).max(at.y.abs_diff(rat_at.y)) == 1
    }, |at| at != victim_at && at != rat_at).ok_or("no route for Challenge caster")?;
    evidence(&format!("C17d.client caster staging steps={}", path.len()));
    full_e2ef5::with_observer_liveness(&mut victim, walk(&mut caster, world, &path)).await?;
    let caster_at = tile(caster.world_spatial().actor_position)?;
    let rat_at = caster.world_entities().and_then(|entities| entities.get(&rat))
        .map(|entity| tile(entity.position)).transpose()?.ok_or("fixture rat absent")?;
    if rat_at.floor != caster_at.floor
        || rat_at.x.abs_diff(caster_at.x).max(rat_at.y.abs_diff(caster_at.y)) != 1
    {
        return Err(format!("fixture rat outside Challenge range: rat={rat_at:?} caster={caster_at:?}").into());
    }
    let caster_ref = own(&caster)?;
    let victim_ref = own(&victim)?;
    if !victim.take_events().into_iter().any(|event| {
        matches!(event, SessionEvent::Presentation(delta) if delta.value.iter().any(|event| {
            matches!(event.kind, PresentationEventKind::ValueText {
                attacker: Some(source), target, primary,
                kind: oteryn_protocol_oteryn::presentation::PresentationValueKind::DamageReceived,
                ..
            } if source == rat && target == victim_ref && primary.value > 0)
        }))
    }) {
        return Err("Challenge pre-state lacks damage from the same rat to the victim".into());
    }
    caster.take_events();
    let book = spell_book()?;
    let before = caster.actor_vitals().copied().ok_or("caster vitals absent")?;
    let cast = caster.cast_spell(challenge_index(&book)?, SpellTarget::None, false).await?;
    if cast.status != CommandStatus::Accepted || cast.disposition != SpellCastDisposition::Cast {
        return Err(format!("Challenge refused: {cast:?}").into());
    }
    let paid = cast.actor_vitals_delta.as_ref().ok_or("Challenge omitted vitals delta")?.value;
    if before.mana.checked_sub(40) != Some(paid.mana) {
        return Err(format!("Challenge must debit 40 mana: before={} after={}", before.mana, paid.mana).into());
    }
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
