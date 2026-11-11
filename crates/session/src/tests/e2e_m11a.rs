//! M11a client replay.
//!
//! The owning client path is `crates/session/src/domain_dispatch.rs`:
//! `Session::apply_state_delta` decodes and commits `WORLD_SPATIAL` and
//! `ACTOR_VITALS`; `crates/session/src/session_access.rs` exposes the resulting
//! state through `Session::world_spatial` and `Session::actor_vitals` (and
//! `tools/dev-client/src/lib.rs` forwards those accessors).
use super::*;
use std::path::Path;

#[derive(Clone)]
struct Fixture {
    spatial: [Vec<u8>; 2],
    vitals: [Vec<u8>; 2],
    expected: [(ActorPosition, ActorVitals); 2],
}

#[test]
fn e2e_m11a_replays_levitate_deltas_and_refuses_invalid_payload() -> Result<(), BoxError> {
    block_on(async {
        let fixture = fixture()?;
        let observation =
            world_spatial::decode_world_spatial(&fixture.spatial[0]).map_err(debug)?;
        let baseline = encode_world_spatial(&WireSpatialObservation {
            content_generation: observation.content_generation,
            actor_position: at(0, 0),
        });
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                &[13],
                &[],
                &[],
                (SNAPSHOT_TYPE_WORLD_SPATIAL_V1, baseline),
                &[(
                    3,
                    3,
                    1,
                    actor_spell::encode_actor_vitals(&vitals(150, 55)).map_err(debug)?,
                )],
            )
            .await?;
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, admission()?).await?;
        peer.await??;

        let mut spatial_revision = session.join_snapshot().world_spatial_revision;
        for (vitals_revision, index) in (3..).zip(0..2usize) {
            apply(
                &mut session,
                50 + u64::try_from(index)? * 2,
                world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                spatial_revision,
                world_spatial::DELTA_TYPE_WORLD_SPATIAL_V1,
                &fixture.spatial[index],
            )?;
            spatial_revision += 1;
            apply(
                &mut session,
                51 + u64::try_from(index)? * 2,
                actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                vitals_revision,
                actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
                &fixture.vitals[index],
            )?;
            assert_eq!(
                session.world_spatial().actor_position,
                fixture.expected[index].0
            );
            assert_eq!(session.actor_vitals(), Some(&fixture.expected[index].1));
        }

        let before = (*session.world_spatial(), session.actor_vitals().copied());
        assert!(
            apply(
                &mut session,
                60,
                world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                spatial_revision,
                world_spatial::DELTA_TYPE_WORLD_SPATIAL_V1,
                &[0xff],
            )
            .is_err()
        );
        assert_eq!(
            (*session.world_spatial(), session.actor_vitals().copied()),
            before
        );
        Ok(())
    })?
}

fn apply(
    session: &mut Session<DuplexStream>,
    sequence: u64,
    domain: u32,
    revision: u64,
    delta_type: u32,
    value: &[u8],
) -> Result<(), BoxError> {
    let frame = encode_state_delta(
        1,
        sequence,
        domain,
        revision,
        revision + 1,
        delta_type,
        value,
    )?;
    let envelope = decode_wire_envelope(&frame)?;
    session.apply_state_delta(sequence, envelope.payload())?;
    Ok(())
}

fn fixture() -> Result<Fixture, BoxError> {
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        return captured(Path::new(&directory));
    }
    let positions = [
        ActorPosition {
            x: 101,
            y: 99,
            floor: -1,
        },
        ActorPosition {
            x: 100,
            y: 99,
            floor: 0,
        },
    ];
    let vitals = [vitals(150, 50), vitals(150, 0)];
    Ok(Fixture {
        spatial: positions.map(|actor_position| {
            encode_world_spatial(&WireSpatialObservation {
                content_generation: [0x73; 32],
                actor_position,
            })
        }),
        vitals: [
            actor_spell::encode_actor_vitals(&vitals[0]).map_err(debug)?,
            actor_spell::encode_actor_vitals(&vitals[1]).map_err(debug)?,
        ],
        expected: [(positions[0], vitals[0]), (positions[1], vitals[1])],
    })
}

fn captured(directory: &Path) -> Result<Fixture, BoxError> {
    let spatial = [
        std::fs::read(directory.join("m11a-up-spatial.bin"))?,
        std::fs::read(directory.join("m11a-down-spatial.bin"))?,
    ];
    let vitals = [
        std::fs::read(directory.join("m11a-up-vitals.bin"))?,
        std::fs::read(directory.join("m11a-down-vitals.bin"))?,
    ];
    let expected_text = std::fs::read_to_string(directory.join("m11a-expected.txt"))?;
    let mut expected = Vec::new();
    for line in expected_text.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 7 || fields[1] != "Accepted" {
            return Err(format!("invalid M11a expected line: {line}").into());
        }
        let decoded_vitals =
            actor_spell::decode_actor_vitals(&vitals[expected.len()]).map_err(debug)?;
        let expected_mana: u32 = fields[5].parse()?;
        let expected_soul: u32 = fields[6].parse()?;
        if decoded_vitals.mana != expected_mana || decoded_vitals.soul != expected_soul {
            return Err(format!("M11a captured vitals disagree with expected line: {line}").into());
        }
        expected.push((
            ActorPosition {
                x: fields[2].parse()?,
                y: fields[3].parse()?,
                floor: fields[4].parse()?,
            },
            decoded_vitals,
        ));
    }
    let expected: [(ActorPosition, ActorVitals); 2] = expected
        .try_into()
        .map_err(|_| "M11a expected file must contain two lines")?;
    Ok(Fixture {
        spatial,
        vitals,
        expected,
    })
}

fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}
