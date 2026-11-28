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
        session.connection_generation,
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

fn apply_captured(
    session: &mut Session<tokio::io::DuplexStream>,
    frame: &[u8],
) -> Result<(), BoxError> {
    let envelope = decode_wire_envelope(frame)?;
    envelope.validate(Direction::ServerToClient, true)?;
    session.check_generation(&envelope)?;
    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
    assert_eq!(envelope.message_type(), MessageType::StateDelta);
    assert_eq!(delta.domain_id, actor_spell::STATE_DOMAIN_ACTOR_VITALS);
    let sequence = session.next_sequence(&envelope)?;
    session.apply_state_delta(sequence, envelope.payload())?;
    Ok(())
}

fn session_payload<'a>(file: &'a [u8], expected: &str) -> Result<&'a [u8], BoxError> {
    let (prefix, payload) = file.split_at_checked(16).ok_or("missing capture session")?;
    let id = GameSessionId::decode(prefix)?;
    let session: String = id
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if session != value(expected, "session")?.replace('-', "") {
        return Err("capture belongs to a different game session".into());
    }
    Ok(payload)
}

#[test]
fn e2e_w01c_creature_attack_vitals_replay() -> Result<(), BoxError> {
    let live = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .map(|root| -> Result<_, BoxError> {
            let root = Path::new(&root);
            let expected = std::fs::read_to_string(root.join("w01c-expected.txt"))?;
            let frame = std::fs::read(root.join("w01c-vitals.bin"))?;
            let before = std::fs::read(root.join("w01c-before.bin"))?;
            let checkpoint = actor_spell::decode_actor_vitals(session_payload(&before, &expected)?)
                .map_err(|error| format!("decode prior vitals: {error:?}"))?;
            Ok((
                session_payload(&frame, &expected)?.to_vec(),
                expected,
                checkpoint,
            ))
        })
        .transpose()?;
    let (payload, expected) = match &live {
        Some((frame, expected, _)) => (frame.clone(), expected.clone()),
        None => load()?,
    };
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

        if let Some((_, _, checkpoint)) = &live {
            session.connection_generation = value(&expected, "generation")?.parse()?;
            session.last_server_sequence = value(&expected, "previous_sequence")?.parse()?;
            session.vitals_revision = value(&expected, "revision")?.parse()?;
            session.actor_vitals = Some(*checkpoint);
            assert_eq!(checkpoint.health, before);
            apply_captured(&mut session, &payload)?;
        } else {
            apply(&mut session, 50, 3, &payload)?;
        }
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

        // Use a valid envelope, current revision and next sequence so rejection must
        // come from the ActorVitals decoder itself.
        let after = session.actor_vitals().copied();
        let revision = session.vitals_revision;
        let sequence = session.last_server_sequence + 1;
        let encoded = actor_spell::encode_actor_vitals(&after.ok_or("vitals absent")?)
            .map_err(|error| format!("{error:?}"))?;
        let invalid = encode_state_delta(
            session.connection_generation,
            sequence,
            actor_spell::STATE_DOMAIN_ACTOR_VITALS,
            revision,
            revision + 1,
            actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
            &[0x08, 0x80],
        )?;
        let Err(error) = apply_captured(&mut session, &invalid) else {
            return Err("malformed ActorVitals accepted".into());
        };
        assert!(matches!(
            error.downcast_ref::<SessionError>(),
            Some(SessionError::ActorSpell(_))
        ));
        assert_eq!(session.vitals_revision, revision);
        apply(&mut session, sequence + 1, revision, &encoded)?;
        assert_eq!(session.actor_vitals().copied(), after);
        Ok(())
    })?
}

#[cfg(feature = "test-support")]
#[test]
fn w01c_capture_preserves_wire_and_prior_context() -> Result<(), BoxError> {
    block_on(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            send_vitals_join(&mut server).await?;
            Ok::<_, BoxError>(server)
        });
        let mut session = Session::admit(client, admission()?).await?;
        let mut server = peer.await??;
        let previous = session.last_server_sequence;
        let payload = actor_spell::encode_actor_vitals(&vitals(141, 55))
            .map_err(|error| format!("encode vitals: {error:?}"))?;
        let frame = encode_state_delta(
            1,
            previous + 1,
            actor_spell::STATE_DOMAIN_ACTOR_VITALS,
            3,
            4,
            actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
            &payload,
        )?;
        write_frame(&mut server, &frame).await?;
        let mut captured = None;
        session
            .capture_vitals_damage(Duration::from_secs(1), |wire, before, after, rev, seq| {
                captured = Some((wire.to_vec(), before.health, after.health, rev, seq));
            })
            .await?;
        assert_eq!(captured, Some((frame, 150, 141, 3, previous)));
        Ok(())
    })?
}

#[test]
fn w01c_rejects_identical_payload_from_another_session() -> Result<(), BoxError> {
    let id = uuid_v7(1);
    let expected_id: String = id.iter().map(|byte| format!("{byte:02x}")).collect();
    let expected = format!("session={expected_id}\n");
    let bytes = [1, 2, 3];
    let original = [id.as_slice(), bytes.as_slice()].concat();
    let substituted = [uuid_v7(2).as_slice(), bytes.as_slice()].concat();
    assert_eq!(session_payload(&original, &expected)?, &bytes);
    assert!(session_payload(&substituted, &expected).is_err());
    Ok(())
}
