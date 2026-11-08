//! W01c replay: the admitted creature's melee damage applied through
//! `Session::apply_state_delta` (the `ACTOR_VITALS` decode in `domain_dispatch.rs`) and exposed by
//! `Session::actor_vitals`, the value the health bar reads.
use super::*;
use oteryn_protocol_oteryn::{decode_wire_envelope, encode_state_delta};
use std::path::Path;

fn value<'a>(text: &'a str, key: &str) -> Result<&'a str, BoxError> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn load() -> Result<(Vec<u8>, String), BoxError> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = Path::new(&root);
        return Ok((
            std::fs::read(root.join("w01c-vitals.bin"))?,
            std::fs::read_to_string(root.join("w01c-expected.txt"))?,
        ));
    }
    let payload = actor_spell::encode_actor_vitals(&ActorVitals {
        health: 141,
        max_health: 150,
        mana: 55,
        max_mana: 55,
        soul: 100,
        harmony: 0,
        serene: false,
    })
    .map_err(|error| format!("{error:?}"))?;
    Ok((
        payload,
        "health_before=150\nhealth=141\nmax_health=150\nmana=55\nmax_mana=55\nsoul=100\n".into(),
    ))
}

fn apply(
    session: &mut Session<tokio::io::DuplexStream>,
    sequence: u64,
    revision: u64,
    payload: &[u8],
) -> Result<(), BoxError> {
    let frame = encode_state_delta(
        1,
        sequence,
        actor_spell::STATE_DOMAIN_ACTOR_VITALS,
        revision,
        revision + 1,
        actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
        payload,
    )?;
    session.apply_state_delta(sequence, decode_wire_envelope(&frame)?.payload())?;
    Ok(())
}

#[test]
fn e2e_w01c_creature_attack_vitals_replay() -> Result<(), BoxError> {
    let (payload, expected) = load()?;
    let health: u32 = value(&expected, "health")?.parse()?;
    let before: u32 = value(&expected, "health_before")?.parse()?;
    block_on(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            send_vitals_join(&mut server).await?;
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, admission()?).await?;
        peer.await??;
        assert_eq!(
            session.actor_vitals().map(|vitals| vitals.health),
            Some(150)
        );
        assert!(health < before);

        apply(&mut session, 50, 3, &payload)?;
        let vitals = session.actor_vitals().ok_or("vitals absent")?;
        assert_eq!(vitals.health, health);
        assert_eq!(
            vitals.max_health,
            value(&expected, "max_health")?.parse::<u32>()?
        );
        assert_eq!(vitals.mana, value(&expected, "mana")?.parse::<u32>()?);
        assert_eq!(
            vitals.max_mana,
            value(&expected, "max_mana")?.parse::<u32>()?
        );
        assert_eq!(vitals.soul, value(&expected, "soul")?.parse::<u32>()?);

        // A truncated vitals payload is refused whole: the damaged value stays.
        let after = session.actor_vitals().copied();
        assert!(apply(&mut session, 51, 4, &payload[..payload.len() - 1]).is_err());
        assert_eq!(session.actor_vitals().copied(), after);
        Ok(())
    })?
}
