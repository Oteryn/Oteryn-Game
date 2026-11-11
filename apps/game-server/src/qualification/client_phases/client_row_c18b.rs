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
    let result = row_complete(capture(clients)).await;
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

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
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
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
            world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone())
        })
        .ok_or("C18b.client needs reachable non-PZ tile")?;
        walk(&mut client, world, &path).await?;
        let here = client.world_spatial().actor_position;
        let target = SpellTarget::Position(SpellTargetPosition {
            x: here.x.saturating_add(1),
            y: here.y,
            floor: here.floor,
        });

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
            let outcome = client.cast_spell(index, target, false).await?;
            if outcome.disposition != SpellCastDisposition::Cast {
                continue;
            }
            let delta = outcome
                .actor_vitals_delta
                .ok_or("C18b.client cast omitted actor-vitals delta")?;
            if delta.value.mana >= before.mana || client.actor_vitals() != Some(&delta.value) {
                return Err(format!(
                    "C18b.client cast did not debit mana in the applied vitals: {before:?} -> {:?}",
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
                )?;
                let payload =
                    oteryn_protocol_oteryn::actor_spell::encode_actor_vitals(&delta.value)
                        .map_err(|error| format!("C18b.client vitals encode: {error:?}"))?;
                write_capture(path.join("c18b-vitals.bin"), &payload)?;
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
                )?;
            }
            return Ok(());
        }
        Err("C18b.client no crosshair spell was cast by the real player".into())
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
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

fn spell_index(book: &crate::spell::SpellBook, words: &str) -> TestResult<std::num::NonZeroU32> {
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
    let deadline =
        tokio::time::Instant::now() + Duration::from_micros(micros.saturating_add(200_000));
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    Ok(())
}

fn write_capture(path: std::path::PathBuf, bytes: &[u8]) -> TestResult {
    std::fs::write(&path, bytes)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_spell_has_no_wire_index() -> TestResult {
        let book = spell_book()?;
        assert!(spell_index(&book, "not an authored spell").is_err());
        assert!(spell_index(&book, "").is_err());
        Ok(())
    }
}
