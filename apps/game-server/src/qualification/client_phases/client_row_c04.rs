//! Client replay capture for SCENARIO_MATRIX C04.

use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::presentation::{ActorCooldownKind, encode_actor_cooldowns};

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

fn index(book: &crate::spell::SpellBook, words: &str) -> TestResult<std::num::NonZeroU32> {
    let key = &book
        .spoken(words)
        .ok_or("C04 client spell absent")?
        .spell
        .key;
    (1..=book.source_len())
        .filter_map(|value| {
            u32::try_from(value)
                .ok()
                .and_then(std::num::NonZeroU32::new)
        })
        .find(|value| book.indexed(*value).is_some_and(|spell| &spell.key == key))
        .ok_or_else(|| "C04 client spell index absent".into())
}

fn capture(path: &Path, name: &str, bytes: &[u8]) -> TestResult {
    std::fs::write(path.join(name), bytes)?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let Some(capture_dir) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let capture_dir = Path::new(&capture_dir);
    let book = spell_book()?;
    let ordinary = index(&book, "exura")?;
    let aggressive = index(&book, "exevo vis lux")?;
    let source = book
        .indexed(ordinary)
        .ok_or("C04 client indexed spell absent")?;
    let authored_groups: serde_json::Value = serde_json::from_slice(&std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/content-schema/spell-authoring/cooldown-groups.json"),
    )?)?;
    let authored_groups = authored_groups
        .get("groups")
        .and_then(serde_json::Value::as_array)
        .ok_or("C04 client cooldown group catalogue absent")?;
    let groups = source
        .groups
        .iter()
        .map(|group| {
            authored_groups
                .iter()
                .position(|value| {
                    value.get("key").and_then(serde_json::Value::as_str) == Some(&group.key)
                })
                .and_then(|index| u32::try_from(index + 1).ok())
                .ok_or("C04 client cooldown group ordinal absent")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut client = connect_world(clients, 0xc6).await?;
    let row_result: TestResult<_> = async {
    let snapshot = client
        .session_mut()
        .gated_snapshot(20)
        .ok_or("C04 client cooldown snapshot absent")?
        .payload
        .clone();

    let pz = client
        .cast_spell(aggressive, SpellTarget::None, false)
        .await?;
    if pz.disposition != SpellCastDisposition::Rejected {
        return Err(format!("C04 client PZ gate reply={pz:?}").into());
    }
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    })
    .ok_or("C04 client route out of PZ absent")?;
    walk(&mut client, world, &path).await?;
    let cast = client
        .cast_spell(ordinary, SpellTarget::None, false)
        .await?;
    if cast.disposition != SpellCastDisposition::Cast {
        return Err(format!("C04 client ordinary cast reply={cast:?}").into());
    }
    let cooldowns = client
        .take_events()
        .into_iter()
        .filter_map(|event| match event {
            SessionEvent::ActorCooldowns(delta) => Some(delta.value),
            _ => None,
        })
        .next_back()
        .ok_or("C04 client cast emitted no cooldown payload")?;
    let payload = encode_actor_cooldowns(&cooldowns)
        .map_err(|error| format!("C04 client cooldown encode: {error:?}"))?;
    let remaining = cooldowns
        .entries
        .iter()
        .filter(|entry| match entry.kind {
            ActorCooldownKind::Spell => entry.id == ordinary.get(),
            ActorCooldownKind::Group => groups.contains(&entry.id),
        })
        .map(|entry| entry.expires_at_ms.saturating_sub(cooldowns.server_now_ms))
        .max()
        .unwrap_or(0);
    capture(capture_dir, "c04-snapshot.bin", &snapshot)?;
    capture(capture_dir, "c04-cooldowns.bin", &payload)?;
    capture(
        capture_dir,
        "c04-expected.txt",
        format!(
            "spell={} groups={} remaining_ms={} pz=Rejected cast=Cast\n",
            ordinary,
            groups.iter().map(u32::to_string).collect::<Vec<_>>().join(","),
            remaining
        )
        .as_bytes(),
    )?;
    evidence(&format!(
        "e2e_all scenario=C04.client status=PASS evidence=cooldown-payload;spell={ordinary};remaining_ms={remaining}"
    ));
    Ok(())

    }.await;
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = journey(clients).await {
        evidence(&format!(
            "e2e_all scenario=C04.client status=FAIL evidence={error}"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_spell_has_no_wire_index() -> TestResult {
        let book = spell_book()?;
        assert!(index(&book, "not an authored spell").is_err());
        assert!(index(&book, "").is_err());
        Ok(())
    }
}
