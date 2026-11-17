//! Client replay capture for SCENARIO_MATRIX C04.

use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::presentation::{ActorCooldownKind, ActorCooldowns, encode_actor_cooldowns};

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

fn cooldown_remaining(
    cooldowns: &ActorCooldowns,
    ordinary: std::num::NonZeroU32,
    groups: &[u32],
) -> TestResult<u64> {
    let active = |kind, id| {
        cooldowns.entries.iter().find(|entry| {
            entry.kind == kind && entry.id == id && entry.expires_at_ms > cooldowns.server_now_ms
        }).map(|entry| entry.expires_at_ms - cooldowns.server_now_ms)
    };
    let mut remaining = active(ActorCooldownKind::Spell, ordinary.get())
        .ok_or("C04 client cast emitted no active spell cooldown")?;
    for &group in groups {
        remaining = remaining.max(active(ActorCooldownKind::Group, group)
            .ok_or_else(|| format!("C04 client cast emitted no active cooldown for group {group}"))?);
    }
    Ok(remaining)
}

fn pz_refusal_matches(in_pz: bool, refused: SpellCastDisposition, outside: SpellCastDisposition) -> bool {
    in_pz && refused == SpellCastDisposition::Rejected && outside == SpellCastDisposition::Cast
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
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
    let snapshot = client
        .session_mut()
        .gated_snapshot(20)
        .ok_or("C04 client cooldown snapshot absent")?
        .payload
        .clone();

    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let in_pz = |at: TilePos| world.base().tile(at.x, at.y, at.floor)
        .is_some_and(|tile| tile.protection_zone());
    let here = tile(client.world_spatial().actor_position)?;
    let path = route(world, here, in_pz).ok_or("C04 client route into PZ absent")?;
    walk(&mut client, world, &path).await?;
    let in_zone = in_pz(tile(client.world_spatial().actor_position)?);
    let pz = client.cast_spell(aggressive, SpellTarget::None, false).await?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| !in_pz(at))
        .ok_or("C04 client route out of PZ absent")?;
    walk(&mut client, world, &path).await?;
    let outside = client.cast_spell(aggressive, SpellTarget::None, false).await?;
    if in_pz(tile(client.world_spatial().actor_position)?)
        || !pz_refusal_matches(in_zone, pz.disposition, outside.disposition)
    {
        return Err(format!("C04 same spell inside and outside PZ: {pz:?} / {outside:?}").into());
    }
    client.take_events();
    let cast = client
        .cast_spell(ordinary, SpellTarget::None, false)
        .await?;
    if cast.disposition != SpellCastDisposition::Cast {
        return Err(format!("C04 client ordinary cast reply={cast:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let (cooldowns, remaining) = loop {
        if let Some(observed) = client.take_events().into_iter().find_map(|event| match event {
            SessionEvent::ActorCooldowns(delta) if delta.server_sequence > cast.result_server_sequence => {
                cooldown_remaining(&delta.value, ordinary, &groups).ok().map(|remaining| (delta.value, remaining))
            }
            _ => None,
        }) {
            break observed;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("C04 missing active cooldown for spell {ordinary} and groups {groups:?}").into());
        }
        client.service_liveness(Duration::from_millis(30)).await?;
    };
    if let Some(capture_dir) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let capture_dir = Path::new(&capture_dir);
        let payload = encode_actor_cooldowns(&cooldowns)
            .map_err(|error| format!("C04 client cooldown encode: {error:?}"))?;
        capture(capture_dir, "c04-snapshot.bin", &snapshot);
        capture(capture_dir, "c04-cooldowns.bin", &payload);
        capture(
            capture_dir,
            "c04-expected.txt",
            format!(
                "spell={} groups={} remaining_ms={} pz=Rejected cast=Cast\n",
                ordinary, groups.iter().map(u32::to_string).collect::<Vec<_>>().join(","), remaining
            ).as_bytes(),
        );
    }
    evidence(&format!(
        "e2e_all scenario=C04.client status=PASS evidence=cooldown-payload;spell={ordinary};remaining_ms={remaining}"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pz_refusal_requires_the_same_cast_to_succeed_outside() {
        assert!(!pz_refusal_matches(true, SpellCastDisposition::Rejected, SpellCastDisposition::TargetIllegal));
        assert!(!pz_refusal_matches(false, SpellCastDisposition::Rejected, SpellCastDisposition::Cast));
        assert!(pz_refusal_matches(true, SpellCastDisposition::Rejected, SpellCastDisposition::Cast));
    }

    use oteryn_protocol_oteryn::presentation::ActorCooldown;

    #[test]
    fn cooldown_requires_spell_and_each_authored_group() -> TestResult {
        let spell = std::num::NonZeroU32::MIN;
        let mut state = ActorCooldowns {
            server_now_ms: 100,
            rtt_ms: None,
            entries: vec![ActorCooldown {
                kind: ActorCooldownKind::Spell,
                id: 2,
                expires_at_ms: 200,
            }],
        };
        assert!(cooldown_remaining(&state, spell, &[3]).is_err());
        state.entries[0].id = spell.get();
        assert!(cooldown_remaining(&state, spell, &[3]).is_err());
        state.entries.push(ActorCooldown {
            kind: ActorCooldownKind::Group,
            id: 3,
            expires_at_ms: 100,
        });
        assert!(cooldown_remaining(&state, spell, &[3]).is_err());
        state.entries[1].expires_at_ms = 250;
        assert_eq!(cooldown_remaining(&state, spell, &[3])?, 150);
        Ok(())
    }
}
