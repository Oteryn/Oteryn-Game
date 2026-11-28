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
    let key = &book.spoken(words).ok_or("C04 client spell absent")?.spell.key;
    (1..=book.source_len())
        .filter_map(|value| {
            u32::try_from(value)
                .ok()
                .and_then(std::num::NonZeroU32::new)
        })
        .find(|value| book.indexed(*value).is_some_and(|spell| &spell.key == key))
        .ok_or_else(|| "C04 client spell index absent".into())
}

fn capture(path: &Path, name: &str, bytes: &[u8]) {
    if let Err(error) = std::fs::write(path.join(name), bytes) {
        evidence(&format!(
            "C04_CLIENT_CAPTURE_WRITE name={name} error={error}"
        ));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let Some(capture_dir) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let capture_dir = Path::new(&capture_dir);
    let book = spell_book()?;
    let ordinary = index(&book, "exura")?;
    let aggressive = index(&book, "exevo vis lux")?;
    let source = book.indexed(ordinary).ok_or("C04 client indexed spell absent")?;
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
            let ordinal = authored_groups
                .iter()
                .position(|value| {
                    value.get("key").and_then(serde_json::Value::as_str) == Some(&group.key)
                })
                .and_then(|index| u32::try_from(index + 1).ok())
                .ok_or("C04 client cooldown group ordinal absent")?;
            Ok::<_, &str>((ordinal, group.cooldown_micros.div_ceil(1000)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let client = connect_world(clients, 0xc6).await?;
    let mut client = provisioning::provision_fixture(client, clients, 1, 100).await?;
    let snapshot = client
        .session_mut()
        .gated_snapshot(20)
        .ok_or("C04 client cooldown snapshot absent")?
        .payload
        .clone();

    let before = client
        .actor_vitals()
        .copied()
        .ok_or("C04 client vitals absent")?;
    let pz = client
        .cast_spell(aggressive, SpellTarget::None, false)
        .await?;
    if pz.status != CommandStatus::Rejected
        || pz.disposition != SpellCastDisposition::Rejected
        || pz.actor_vitals_delta.is_some()
        || client.actor_vitals().copied() != Some(before)
    {
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
    client.take_events();
    let cast_started = std::time::Instant::now();
    let cast = client
        .cast_spell(ordinary, SpellTarget::None, false)
        .await?;
    let elapsed_ms = u64::try_from(cast_started.elapsed().as_micros().div_ceil(1000))?;
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
    let spell_ms = source.cooldown_micros.div_ceil(1000);
    for (kind, id, duration_ms) in
        std::iter::once((ActorCooldownKind::Spell, ordinary.get(), spell_ms)).chain(
            groups
                .iter()
                .map(|&(id, ms)| (ActorCooldownKind::Group, id, ms)),
        )
    {
        let remaining = cooldowns
            .entries
            .iter()
            .find(|entry| entry.kind == kind && entry.id == id)
            .map_or(0, |entry| entry.expires_at_ms.saturating_sub(cooldowns.server_now_ms));
        // The cast and cooldown sample both occurred within the observed request.
        // Complete updates round the absolute deadline up and the sample time down.
        let minimum_ms = duration_ms.saturating_sub(elapsed_ms).max(1);
        if !(minimum_ms..=duration_ms + u64::from(cooldowns.snapshot)).contains(&remaining) {
            return Err(format!(
                "C04 client cast cooldown {kind:?}/{id}: remaining_ms={remaining}, authored_ms={duration_ms}"
            ).into());
        }
    }
    let payload = encode_actor_cooldowns(&cooldowns)
        .map_err(|error| format!("C04 client cooldown encode: {error:?}"))?;
    capture(capture_dir, "c04-snapshot.bin", &snapshot);
    capture(capture_dir, "c04-cooldowns.bin", &payload);
    capture(
        capture_dir,
        "c04-expected.txt",
        format!(
            "spell={}:{} groups={} snapshot={} elapsed_ms={} pz=Rejected cast=Cast\n",
            ordinary,
            spell_ms,
            groups.iter().map(|(id, ms)| format!("{id}:{ms}")).collect::<Vec<_>>().join(","),
            cooldowns.snapshot,
            elapsed_ms,
        )
        .as_bytes(),
    );
    evidence(&format!(
        "e2e_all scenario=C04.client status=PASS evidence=cooldown-payload;spell={ordinary};authored_ms={spell_ms};snapshot={}",
        cooldowns.snapshot,
    ));
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = verdict::observe(journey(clients)).await {
        evidence(&format!(
            "e2e_all scenario=C04.client status=FAIL evidence={error}"
        ));
    }
}
