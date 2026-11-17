//! C16a replay owner: `crates/session/src/domain_dispatch.rs` routes domain 17 through
//! `map_transport::apply` in `crates/session/src/map_transport.rs`, which validates and exposes
//! `SessionEvent::WorldMap`. The searched UI (`crates/client-ui/src/features/0091-field-runes/`)
//! has no state reducer, and client-runtime has no field-rune reducer; `Session` is therefore the
//! native client owner consumed by `tools/dev-client/src/lib.rs`.
use super::*;
use oteryn_protocol_oteryn::world_map::*;

#[test]
fn e2e_c16a_replays_barrier_map_payloads() -> Result<(), BoxError> {
    block_on(async {
        let header = MapViewHeader {
            content_generation: CONTENT_GENERATION,
            bundle_digest: [0x22; 32],
            reset_epoch: 0,
            origin: ActorPosition {
                x: 100,
                y: 100,
                floor: 0,
            },
            first_visible_floor: 0,
        };
        let (snapshot, cases, expected) = fixture(&header)?;
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let peer = tokio::spawn(async move {
            read_frame(&mut server).await?;
            write_frame(
                &mut server,
                &encode_server_accepted(&ServerAcceptedValue {
                    game_session_id: GameSessionId::decode(&uuid_v7(1))?,
                    world_id: WorldId::decode(&uuid_v7(2))?,
                    channel_id: ChannelId::decode(&uuid_v7(3))?,
                    connection_generation: 1,
                    current_server_sequence: 0,
                    next_command_id: 7,
                    schema_revision: 1,
                    selected_capabilities: &[CAPABILITY_WORLD_MAP_VIEW_V1],
                })?,
            )
            .await?;
            let spatial_payload = spatial(100);
            for frame in encode_single_chunk_snapshot(
                1,
                1,
                40,
                &[
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                        revision: 1,
                        snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                        payload: &spatial_payload,
                    },
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_WORLD_MAP_VIEW,
                        revision: 1,
                        snapshot_type: SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1,
                        payload: &snapshot,
                    },
                ],
            )? {
                write_frame(&mut server, &frame).await?;
            }
            let mut rest = Vec::new();
            let _ = server.read_to_end(&mut rest).await;
            Ok::<_, BoxError>(())
        });
        let mut admission = admission()?;
        admission.supported_capabilities = &[CAPABILITY_WORLD_MAP_VIEW_V1];
        let mut session = Session::admit(client, admission).await?;
        for (case, want) in cases.iter().zip(&expected) {
            session.map_header = Some(header);
            session.gated_revisions = vec![(STATE_DOMAIN_WORLD_MAP_VIEW, case.base)];
            let wire = encode_state_delta(
                1,
                case.sequence,
                STATE_DOMAIN_WORLD_MAP_VIEW,
                case.base,
                case.new,
                DELTA_TYPE_WORLD_MAP_VIEW_DELTA_V1,
                &case.payload,
            )?;
            let event =
                session.apply_state_delta(case.sequence, decode_wire_envelope(&wire)?.payload())?;
            let SessionEvent::WorldMap(delta) = event else {
                return Err("C16a payload did not produce a world-map event".into());
            };
            let found = delta.value.tiles.iter().any(|tile| {
                tile.position == want.position
                    && tile.items.iter().any(|item| {
                        matches!(item.definition, MapDefinition::Item(reference) if reference.get() == want.reference)
                    })
            });
            assert_eq!(found, case.present);
            assert_eq!(delta.new_revision, case.new);
        }

        let before_header = session.map_header;
        let before_revision = session.gated_revisions.clone();
        let last = cases.last().ok_or("C16a fixture has no cases")?;
        let invalid = encode_state_delta(
            1,
            last.sequence + 1,
            STATE_DOMAIN_WORLD_MAP_VIEW,
            last.new,
            last.new + 1,
            DELTA_TYPE_WORLD_MAP_VIEW_DELTA_V1,
            &[0xff],
        )?;
        assert!(
            session
                .apply_state_delta(last.sequence + 1, decode_wire_envelope(&invalid)?.payload())
                .is_err()
        );
        assert_eq!(session.map_header, before_header);
        assert_eq!(session.gated_revisions, before_revision);
        drop(session);
        peer.await??;
        Ok(())
    })?
}

struct Case {
    base: u64,
    new: u64,
    sequence: u64,
    payload: Vec<u8>,
    present: bool,
}
struct Expected {
    position: ActorPosition,
    reference: u32,
}

type Fixture = (Vec<u8>, Vec<Case>, Vec<Expected>);

fn fixture(header: &MapViewHeader) -> Result<Fixture, BoxError> {
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&directory);
        let snapshot = std::fs::read(path.join("c16a-snapshot.bin"))?;
        let expected_text = std::fs::read_to_string(path.join("c16a-expected.txt"))?;
        let expected = expected_text
            .lines()
            .map(parse_expected)
            .collect::<Result<Vec<_>, _>>()?;
        let mut cases = Vec::new();
        for stem in ["wall", "growth"] {
            for (phase, present) in [("placed", true), ("expired", false)] {
                let metadata =
                    std::fs::read_to_string(path.join(format!("c16a-{stem}-{phase}-meta.txt")))?;
                let numbers = metadata
                    .split_whitespace()
                    .map(str::parse::<u64>)
                    .collect::<Result<Vec<_>, _>>()?;
                let [base, new, sequence] = numbers.as_slice() else {
                    return Err(format!("invalid C16a {stem} {phase} metadata").into());
                };
                cases.push(Case {
                    base: *base,
                    new: *new,
                    sequence: *sequence,
                    payload: std::fs::read(path.join(format!("c16a-{stem}-{phase}.bin")))?,
                    present,
                });
            }
        }
        let expanded = expected
            .into_iter()
            .flat_map(|value| {
                [
                    Expected {
                        position: value.position,
                        reference: value.reference,
                    },
                    value,
                ]
            })
            .collect();
        return Ok((snapshot, cases, expanded));
    }
    let snapshot = encode_world_map_snapshot(&WorldMapViewSnapshot {
        header: *header,
        tiles: vec![],
    })
    .map_err(|error| format!("{error:?}"))?;
    let mut cases = Vec::new();
    let mut expected = Vec::new();
    for (index, (position, reference)) in [
        (
            ActorPosition {
                x: 101,
                y: 100,
                floor: 0,
            },
            2128,
        ),
        (
            ActorPosition {
                x: 102,
                y: 100,
                floor: 0,
            },
            2130,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let reference = std::num::NonZeroU32::new(reference).ok_or("zero item reference")?;
        for present in [true, false] {
            let revision = u64::try_from(index * 2 + usize::from(!present) + 1)?;
            let delta = WorldMapViewDelta {
                header: *header,
                tiles: if present {
                    vec![MapTile {
                        position,
                        items: vec![MapItem {
                            definition: MapDefinition::Item(reference),
                            origin: MapOrigin::DisplayOnly,
                            count: 1,
                            sub_type: 0,
                            appearance_id: 0,
                        }],
                        more: false,
                        ground_speed: 0,
                    }]
                } else {
                    vec![]
                },
                cleared: if present { vec![] } else { vec![position] },
            };
            cases.push(Case {
                base: revision,
                new: revision + 1,
                sequence: 40 + revision,
                payload: encode_world_map_delta(header, &delta)
                    .map_err(|error| format!("{error:?}"))?,
                present,
            });
            expected.push(Expected {
                position,
                reference: reference.get(),
            });
        }
    }
    Ok((snapshot, cases, expected))
}

fn parse_expected(line: &str) -> Result<Expected, BoxError> {
    let fields = line.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 5 {
        return Err("invalid C16a expected row".into());
    }
    Ok(Expected {
        position: ActorPosition {
            x: fields[1].parse()?,
            y: fields[2].parse()?,
            floor: fields[3].parse()?,
        },
        reference: fields[4].parse()?,
    })
}
