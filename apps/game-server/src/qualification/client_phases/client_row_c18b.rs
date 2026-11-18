//! C18b client replay capture: a real position cast produces the result consumed by
//! `crates/session/src/session_exchange.rs::Session::exchange_cast` and the actor-vitals delta
//! consumed by `crates/session/src/domain_dispatch.rs::Session::apply_delta`.

use super::*;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget, SpellTargetPosition};

const SPELLS: [&str; 5] = [
    "exevo mort ora",
    "exevo tempo mas san",
    "exori dir san",
    "exori dir moe",
    "exori mas amp pug",
];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(capture(clients))).await;
    match result {
        Ok(Ok(())) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=C18b.client status=PASS evidence=real-position-cast-payloads",
        ),
        Ok(Err(error)) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C18b.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=C18b.client status=FAIL evidence=journey-timeout",
        ),
    }
}

fn position_cast_matches(mana_before: u32, mana_after: u32, chosen_effect: bool) -> bool {
    chosen_effect && mana_after < mana_before
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let book = spell_book()?;
    let mut client = connect_character(clients, 0x18, 1).await?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    })
    .ok_or("C18b.client needs reachable non-PZ tile")?;
    walk(&mut client, world, &path).await?;
    let here = client.world_spatial().actor_position;
    let position = SpellTargetPosition {
        x: here.x.saturating_add(1),
        y: here.y,
        floor: here.floor,
    };
    let target = SpellTarget::Position(position);
    let mut refusals = Vec::new();

    for words in SPELLS {
        let spell = book
            .spoken(words)
            .ok_or("C18b.client crosshair spell absent")?
            .spell;
        ready(
            &mut client,
            spell
                .groups
                .iter()
                .map(|group| group.cooldown_micros)
                .max()
                .unwrap_or(0)
                .max(spell.cooldown_micros),
        )
        .await?;
        let index = spell_index(&book, words)?;
        let before = *client.actor_vitals().ok_or("C18b.client vitals absent")?;
        client.take_events();
        let outcome = client.cast_spell(index, target, false).await?;
        if outcome.status != CommandStatus::Accepted || outcome.disposition != SpellCastDisposition::Cast {
            refusals.push(format!("{words}: {:?}/{:?}", outcome.status, outcome.disposition));
            continue;
        }
        let delta = outcome
            .actor_vitals_delta
            .ok_or("C18b.client cast omitted actor-vitals delta")?;
        let wanted = ActorPosition { x: position.x, y: position.y, floor: position.floor };
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let chosen_effect = loop {
            use oteryn_protocol_oteryn::presentation::{PresentationEventKind, PresentationSource};
            if client.take_events().into_iter().any(|event| matches!(
                event,
                oteryn_dev_client::SessionEvent::Presentation(delta)
                    if delta.server_sequence > outcome.result_server_sequence && delta.value.iter().any(|effect| {
                        matches!(effect.kind, PresentationEventKind::MagicEffect {
                            position, source: PresentationSource::Own, ..
                        } if position == wanted)
                    })
            )) {
                break true;
            }
            if tokio::time::Instant::now() >= deadline {
                break false;
            }
            client.service_liveness(Duration::from_millis(25)).await?;
        };
        if !position_cast_matches(before.mana, delta.value.mana, chosen_effect) {
            return Err(format!(
                "C18b.client cast missing mana debit or effect at {wanted:?}: {before:?} -> {:?}, effect={chosen_effect}",
                delta.value
            )
            .into());
        }
        if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let path = Path::new(&path);
            write_capture(
                path.join("c18b-result.bin"),
                &oteryn_protocol_oteryn::actor_spell::encode_spell_cast_result(
                    outcome.disposition,
                ),
            );
            let payload = oteryn_protocol_oteryn::actor_spell::encode_actor_vitals(&delta.value)
                .map_err(|error| format!("C18b.client vitals encode: {error:?}"))?;
            write_capture(path.join("c18b-vitals.bin"), &payload);
            write_capture(
                path.join("c18b-expected.txt"),
                format!(
                    "{} {} {} {}\n{} {} {} {} {} {} {} {}\n",
                    index,
                    here.x.saturating_add(1),
                    here.y,
                    here.floor,
                    outcome.disposition as u32,
                    delta.value.health,
                    delta.value.max_health,
                    delta.value.mana,
                    delta.value.max_mana,
                    delta.value.soul,
                    delta.value.harmony,
                    u8::from(delta.value.serene)
                )
                .as_bytes(),
            );
        }
        return Ok(());
    }
    Err(format!("C18b.client no crosshair spell succeeded: {refusals:?}").into())
}

fn spell_book() -> TestResult<crate::spell::SpellBook> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
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

fn spell_index(
    book: &crate::spell::SpellBook,
    words: &str,
) -> TestResult<std::num::NonZeroU32> {
    let key = &book
        .spoken(words)
        .ok_or("C18b.client spell words absent")?
        .spell
        .key;
    (1..=book.source_len())
        .filter_map(|index| {
            u32::try_from(index)
                .ok()
                .and_then(std::num::NonZeroU32::new)
        })
        .find(|index| book.indexed(*index).is_some_and(|spell| &spell.key == key))
        .ok_or_else(|| "C18b.client spell index absent".into())
}

async fn ready(client: &mut oteryn_dev_client::DevClientSession, micros: u64) -> TestResult {
    let deadline = tokio::time::Instant::now()
        + Duration::from_micros(micros.saturating_add(200_000));
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    Ok(())
}

fn write_capture(path: std::path::PathBuf, bytes: &[u8]) {
    if let Err(error) = std::fs::write(&path, bytes) {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C18b.client status=CAPTURE-WRITE-ERROR evidence={}:{}",
            path.display(),
            error
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_capture_requires_effect_at_the_chosen_position() {
        assert!(!position_cast_matches(50, 20, false));
        assert!(!position_cast_matches(50, 50, true));
        assert!(position_cast_matches(50, 20, true));
    }
}
