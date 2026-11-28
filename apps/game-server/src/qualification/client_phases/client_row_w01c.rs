//! W01c captures the original server envelope for the live damage delta.
use super::*;
use oteryn_protocol_oteryn::{actor_spell, decode_wire_envelope};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::logout::LogoutResult;
use std::io::Write;

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    let result =
        std::fs::File::create_new(root.join(name)).and_then(|mut file| file.write_all(bytes));
    result.map_err(|error| format!("W01c client capture: {name}: {error}"))?;
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let capture_root = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .ok_or("W01c client capture: E2E_ALL_CAPTURE_DIR missing")?;
    let capture_root = Path::new(&capture_root);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0x0d).await?;
    travel_to_spawn(
        &mut client,
        world,
        &metadata.spawns,
        "oteryn:creature.poison_spider",
    )
    .await?;
    let cancel = client.session_mut().attack_target(None).await?;
    if cancel.disposition != AttackIntentDisposition::Ok {
        return Err(format!("W01c client capture: attack-cancel refused: {cancel:?}").into());
    }
    client.take_events();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    let (frame, before, damaged, revision, previous_sequence) = loop {
        let mut captured = None;
        client.session_mut().capture_vitals_damage(Duration::from_millis(50),
            |frame, before, after, revision, sequence| {
                captured = Some((frame.to_vec(), *before, *after, revision, sequence));
            }).await?;
        client.take_events();
        if let Some(captured) = captured {
            break captured;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("W01c client capture: no spider damage delta in 45s".into());
        }
    };
    let generation = decode_wire_envelope(&frame)?.connection_generation();

    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());

    // Release the character through the normal logout path. Dropping the
    // session at the encounter leaves the fixture held by combat loss/grace.
    let home = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == TOWN
    })
    .ok_or("W01c client capture: no walking route back to town")?;
    walk(&mut client, world, &home).await?;
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(LogoutResult::Accepted) {
        return Err(format!("W01c client capture: town logout refused: {logout:?}").into());
    }

    capture(capture_root, "w01c-vitals.bin", &frame)?;
    let checkpoint = actor_spell::encode_actor_vitals(&before)
        .map_err(|error| format!("encode prior vitals: {error:?}"))?;
    capture(capture_root, "w01c-before.bin", &checkpoint)?;
    let expected = format!(
        "session={session}\ngeneration={generation}\nprevious_sequence={previous_sequence}\nrevision={revision}\nhealth_before={}\nhealth={}\nmax_health={}\nmana={}\nmax_mana={}\nsoul={}\n",
        before.health,
        damaged.health,
        damaged.max_health,
        damaged.mana,
        damaged.max_mana,
        damaged.soul
    );
    capture(capture_root, "w01c-expected.txt", expected.as_bytes())?;
    Ok(expected)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    // Include both live town traversals and the bounded damage observation.
    match verdict::indexed_timeout(Duration::from_secs(540), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=W01c.client status=CAPTURED evidence={:?}",
            expected.replace('\n', ";")
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W01c.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W01c.client status=FAIL evidence=timeout"),
    }
}
