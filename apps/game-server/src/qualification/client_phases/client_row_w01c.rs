//! W01c captures the original server envelope for the live damage delta.
use super::*;
use super::full_row_w01c::{ENCOUNTER, adjacent_spiders, walk_to};
use oteryn_dev_client::{EntityRef, SessionEvent};
use oteryn_protocol_oteryn::analyser::{
    AnalyserDamageSource, AnalyserFact, AttributedFact, CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1,
};
use oteryn_protocol_oteryn::damage_element::DamageElement;
use sha2::{Digest, Sha256};
use oteryn_protocol_oteryn::{actor_spell, decode_wire_envelope};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::logout::LogoutResult;
use std::io::Write;

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    let result =
        std::fs::File::create_new(root.join(name)).and_then(|mut file| file.write_all(bytes));
    result.map_err(|error| format!("W01c client capture: {name}: {error}"))?;
    Ok(())
}

fn melee_amount(fact: &AttributedFact, spiders: &[EntityRef]) -> TestResult<Option<u32>> {
    let AnalyserFact::DamageInput { value, element, source } = &fact.fact else {
        return Ok(None);
    };
    if *element != DamageElement::Physical
        || !matches!(source, AnalyserDamageSource::Creature(_))
        || !fact.attacker.is_some_and(|id| spiders.contains(&id))
        || value.get() > 20
    {
        return Err(format!("W01c unexpected incoming damage: {fact:?}").into());
    }
    Ok(Some(value.get()))
}

fn verify_relog<T: Eq, P: Eq, V: Eq>(
    original: T, admitted: T, position: P, observed_position: P, saved: V, observed: Option<V>,
) -> TestResult {
    if admitted == original || observed_position != position || observed != Some(saved) {
        return Err("W01c relog did not restore the exact saved position and vitals".into());
    }
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
    if !metadata.spawns.sources.iter().any(|source| {
        source.floor == ENCOUNTER.floor && source.points.iter().any(|point| {
            point.x.abs_diff(ENCOUNTER.x).max(point.y.abs_diff(ENCOUNTER.y)) == 1
                && metadata.spawns.creatures.get(point.creature as usize)
                    .is_some_and(|creature| creature.key == "oteryn:creature.poison_spider")
        })
    }) {
        return Err("W01c fixed encounter has no authored adjacent spider spawn".into());
    }
    walk_to(&mut client, world, ENCOUNTER).await?;
    let spiders = adjacent_spiders(&client)?;
    if !client.selected_capabilities().contains(&CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1) {
        return Err("W01c missing hit attribution capability".into());
    }
    let cancel = client.session_mut().attack_target(None).await?;
    if cancel.disposition != AttackIntentDisposition::Ok {
        return Err(format!("W01c client capture: attack-cancel refused: {cancel:?}").into());
    }
    client.take_events();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    let mut captured = None;
    let mut hit = None;
    let (frame, before, damaged, revision, previous_sequence) = loop {
        if captured.is_none() {
        client.session_mut().capture_vitals_damage(Duration::from_millis(50),
            |frame, before, after, revision, sequence| {
                captured = Some((frame.to_vec(), *before, *after, revision, sequence));
            }).await?;
        } else {
            client.service_liveness(Duration::from_millis(50)).await?;
        }
        for event in client.take_events() {
            if let SessionEvent::AnalyserAttributedFacts(delta) = event {
                for fact in delta.value {
                    if let Some(amount) = melee_amount(&fact, &spiders)?
                        && hit.replace(amount).is_some()
                    {
                        return Err("W01c ambiguous damage attribution".into());
                    }
                }
            }
        }
        if let (Some(capture), Some(amount)) = (&captured, hit) {
            if capture.1.health.checked_sub(amount) != Some(capture.2.health) {
                return Err("W01c attributed melee does not explain the captured HP loss".into());
            }
            break captured.take().ok_or("W01c damage capture missing")?;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("W01c client capture: no spider damage delta in 45s".into());
        }
    };
    let generation = decode_wire_envelope(&frame)?.connection_generation();

    let session_id = client.join_snapshot().game_session_id;
    let session = uuid_text(session_id.as_bytes());

    // Release the character through the normal logout path. Dropping the
    // session at the encounter leaves the fixture held by combat loss/grace.
    walk_to(&mut client, world, TOWN).await?;
    let first_session = client.join_snapshot().game_session_id;
    let position = client.world_spatial().actor_position;
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(LogoutResult::Accepted) {
        return Err(format!("W01c client capture: town logout refused: {logout:?}").into());
    }

    let saved = *client.actor_vitals().ok_or("W01c saved vitals absent")?;
    if saved.health == 0 || saved.health >= before.health
        || client.world_spatial().actor_position != position
    {
        return Err("W01c logout did not preserve a damaged actor at town".into());
    }
    drop(client);
    let mut client = connect_world(clients, 0x0e).await?;
    verify_relog(first_session, client.join_snapshot().game_session_id, position,
        client.world_spatial().actor_position, saved, client.actor_vitals().copied())?;
    evidence(&format!("W01C_CLIENT_RELOG position={position:?} vitals={saved:?}"));
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(LogoutResult::Accepted) {
        return Err(format!("W01c client cleanup logout refused: {logout:?}").into());
    }

    let frame = [session_id.as_bytes().as_slice(), frame.as_slice()].concat();
    capture(capture_root, "w01c-vitals.bin", &frame)?;
    let checkpoint = actor_spell::encode_actor_vitals(&before)
        .map_err(|error| format!("encode prior vitals: {error:?}"))?;
    let checkpoint = [session_id.as_bytes().as_slice(), checkpoint.as_slice()].concat();
    capture(capture_root, "w01c-before.bin", &checkpoint)?;
    let frame_sha = sha256_hex(&frame);
    let checkpoint_sha = sha256_hex(&checkpoint);
    let expected = format!(
        "session={session}\nframe_sha256={frame_sha}\ncheckpoint_sha256={checkpoint_sha}\ngeneration={generation}\nprevious_sequence={previous_sequence}\nrevision={revision}\nhealth_before={}\nhealth={}\nmax_health={}\nmana={}\nmax_mana={}\nsoul={}\n",
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_adjacent_spider_physical_melee_in_global_range_qualifies() -> TestResult {
        let spider = EntityRef { identity: [1; 16], generation: 1 };
        let other = EntityRef { identity: [2; 16], generation: 1 };
        for (attacker, element, amount, accepted) in [
            (Some(spider), DamageElement::Physical, 20, true),
            (Some(other), DamageElement::Physical, 14, false),
            (None, DamageElement::Physical, 14, false),
            (Some(spider), DamageElement::Earth, 2, false),
            (Some(spider), DamageElement::Physical, 21, false),
        ] {
            let fact = AttributedFact {
                fact: AnalyserFact::DamageInput {
                    value: std::num::NonZeroU32::new(amount).ok_or("test amount is zero")?,
                    element,
                    source: AnalyserDamageSource::Creature(None),
                },
                attacker,
            };
            assert_eq!(melee_amount(&fact, &[spider]).ok().flatten(), accepted.then_some(amount));
        }
        Ok(())
    }

    #[test]
    fn relog_rejects_old_session_wrong_tile_and_restored_health() {
        assert!(verify_relog(1, 2, 3, 3, 138, Some(138)).is_ok());
        assert!(verify_relog(1, 1, 3, 3, 138, Some(138)).is_err());
        assert!(verify_relog(1, 2, 3, 4, 138, Some(138)).is_err());
        assert!(verify_relog(1, 2, 3, 3, 138, Some(150)).is_err());
    }
}
