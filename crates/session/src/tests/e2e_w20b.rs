//! W20b client replay: Light (`utevo lux`) gives the caster's own entity a light. The spatial
//! entity delta is session-owned (the join and delta bytes are not tapped), so the row re-encodes
//! it with the production encoder from the session's own visible entity (the C17c practice); this
//! test replays it through `Session::apply_state_delta` and reads the result through
//! `Session::visible_appearances`. A malformed delta is refused and leaves the light in place.
use super::*;
use crate::tests::{BoxError, join_peer_with};
use oteryn_protocol_oteryn::encode_state_delta;
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityFields, EntityLights, encode_world_spatial_entities_snapshot_with_fields,
};

struct Capture {
    delta: Vec<u8>,
    expected: String,
}

fn fixture() -> Result<Capture, BoxError> {
    let root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("W20b capture directory is required")?;
    let run_id = std::env::var("OTERYN_W20B_CAPTURE_ID")?;
    let path = std::path::Path::new(&root);
    let expected = std::fs::read_to_string(path.join("w20b-expected.txt"))?;
    validate_capture(&expected, &run_id)?;
    Ok(Capture {
        delta: std::fs::read(path.join("w20b-entities.bin"))?,
        expected,
    })
}

fn validate_capture(expected: &str, run_id: &str) -> Result<(), BoxError> {
    if run_id.len() != 32
        || !run_id.bytes().all(|byte| byte.is_ascii_hexdigit())
        || value(expected, "run_id")? != run_id
        || value(expected, "words")? != "utevo lux"
        || value(expected, "light")? != "6:215"
    {
        return Err("W20b capture does not match this run and Crystal Light".into());
    }
    Ok(())
}

#[test]
fn w20b_rejects_stale_and_wrong_light_metadata() {
    let current = "1".repeat(32);
    let stale = "2".repeat(32);
    let metadata = format!("run_id={current}\nwords=utevo lux\nlight=6:215\n");
    assert!(validate_capture(&metadata, &current).is_ok());
    assert!(validate_capture(&metadata, &stale).is_err());
    assert!(validate_capture(&metadata.replace("6:215", "8:215"), &current).is_err());
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, BoxError> {
    expected
        .lines()
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.strip_prefix('='))
        })
        .ok_or_else(|| format!("w20b expected lacks {key}").into())
}

fn unhex<const N: usize>(text: &str) -> Result<[u8; N], BoxError> {
    let mut out = [0_u8; N];
    if text.len() != N * 2 {
        return Err("w20b hex length".into());
    }
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(index * 2..index * 2 + 2).ok_or("w20b hex")?, 16)?;
    }
    Ok(out)
}

#[test]
#[ignore = "requires a fresh live W20b capture and its run ID"]
fn e2e_w20b_light_spell_replay() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(scenario)?
        .join()
        .map_err(|_| "W20b light session panicked")?
}

fn scenario() -> Result<(), BoxError> {
    block_on(async {
        let capture = fixture()?;
        let expected = capture.expected.as_str();
        let identity: [u8; 16] = unhex(value(expected, "identity")?)?;
        let generation: u64 = value(expected, "generation")?.parse()?;
        let content: [u8; 32] = unhex(value(expected, "content")?)?;
        let (level, color) = (6, 215);
        let mut player = own(at(0, 0));
        player.entity = EntityRef {
            identity,
            generation,
        };
        let snapshot = WorldSpatialEntitiesSnapshot {
            content_generation: content,
            actor_position: at(0, 0),
            own_identity: identity,
            entities: vec![player],
        };
        let bytes = encode_world_spatial_entities_snapshot_with_fields(
            &snapshot,
            EntityFields::selected(&[6, 19]),
            &EntityLights::new(),
        )
        .map_err(|error| format!("{error:?}"))?;
        let clock = oteryn_protocol_oteryn::world_clock::encode_world_clock(
            oteryn_protocol_oteryn::world_clock::WorldClock {
                tibian_minute: 0,
                light_level: 40,
                light_color: 215,
            },
        )
        .map_err(|error| format!("{error:?}"))?;
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                CLIENT_SUPPORTED_CAPABILITIES,
                &[6, 19],
                &[],
                (2, bytes),
                &[(18, 0, 1, clock)],
            )
            .await?;
            let mut rest = Vec::new();
            let _ = server.read_to_end(&mut rest).await;
            Ok::<_, BoxError>(())
        });
        let mut session = Session::admit(client, entity_admission()?).await?;
        let light_of = |session: &Session<_>| {
            session
                .visible_appearances()
                .into_iter()
                .find(|entry| entry.entity.entity.identity == identity)
                .map(|entry| entry.light)
        };
        assert_eq!(light_of(&session), Some(None));
        let frame = encode_state_delta(1, 41, 1, 5, 6, 2, &capture.delta)?;
        session.apply_state_delta(
            41,
            oteryn_protocol_oteryn::decode_wire_envelope(&frame)?.payload(),
        )?;
        assert_eq!(light_of(&session), Some(Some((level, color))));
        let bad = encode_state_delta(1, 42, 1, 6, 7, 2, &[0xff])?;
        assert!(
            session
                .apply_state_delta(
                    42,
                    oteryn_protocol_oteryn::decode_wire_envelope(&bad)?.payload()
                )
                .is_err()
        );
        assert_eq!(light_of(&session), Some(Some((level, color))));
        drop(session);
        peer.await??;
        Ok(())
    })?
}
