//! Scripted native-wire checks for the actual bounded party icon reader; no SQL fixture.
use crate::bestiary_postgres_harness::{TestResult, debug};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, DomainSnapshot, encode_client_command,
    party_skull::{COMMAND_TYPE_PARTY_SKULL_QUERY, PartyIcon, encode_party_icons},
    world_spatial::{ActorPosition, StepDirection},
    world_spatial_entities as spatial,
};
use tokio::io::DuplexStream;

const GENERATION: u64 = 7;
const COMMAND: u64 = 11;
const BASELINE: u64 = 5;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Trace {
    ResultFirst,
    DeltaFirst,
    ProbeBetween,
    WrongCommand,
    WrongResultGeneration,
    WrongDeltaGeneration,
    WrongResultIcons,
    WrongDeltaIcons,
    WrongDeltaTarget,
    StaleRevision,
    MissingIcons,
    WrongDomain,
    WrongType,
    ResultOnly,
    DeltaOnly,
}

fn entity() -> spatial::EntityRef {
    spatial::EntityRef {
        identity: [9; spatial::ENTITY_IDENTITY_BYTES],
        generation: 3,
    }
}

fn position() -> ActorPosition {
    ActorPosition {
        x: 1,
        y: 1,
        floor: 7,
    }
}

fn icons(wrong: bool) -> Vec<PartyIcon> {
    vec![PartyIcon {
        entity: entity(),
        shield: if wrong { 0 } else { 4 },
        skull: 2,
    }]
}

async fn snapshot(server: &mut DuplexStream) -> TestResult {
    let payload =
        spatial::encode_world_spatial_entities_snapshot(&spatial::WorldSpatialEntitiesSnapshot {
            content_generation: [1; 32],
            actor_position: position(),
            own_identity: entity().identity,
            entities: vec![spatial::WorldSpatialEntity {
                display_name: None,
                kind: spatial::EntityKind::Player,
                entity: entity(),
                position: position(),
                detail: spatial::EntityDetail::Actor {
                    direction: StepDirection::North,
                    appearance_ref: 1,
                    health_percent: 100,
                },
            }],
        })
        .map_err(debug)?;
    for frame in oteryn_protocol_oteryn::encode_single_chunk_snapshot(
        GENERATION,
        1,
        0,
        &[DomainSnapshot {
            domain_id: 1,
            revision: BASELINE,
            snapshot_type: 2,
            payload: &payload,
        }],
    )
    .map_err(debug)?
    {
        write_frame(server, &frame).await?;
    }
    Ok(())
}

fn frames(trace: Trace) -> TestResult<(Vec<u8>, Vec<u8>)> {
    let result = oteryn_protocol_oteryn::encode_command_result(
        GENERATION + u64::from(trace == Trace::WrongResultGeneration),
        if matches!(trace, Trace::DeltaFirst | Trace::ProbeBetween) {
            2
        } else {
            1
        },
        COMMAND + u64::from(trace == Trace::WrongCommand),
        CommandStatus::Accepted,
        &encode_party_icons(&icons(trace == Trace::WrongResultIcons)).map_err(debug)?,
    )
    .map_err(debug)?;
    let mut payload =
        spatial::encode_world_spatial_entities_delta(&spatial::WorldSpatialEntitiesDelta {
            content_generation: [1; 32],
            actor_position: position(),
            enter: vec![],
            update: vec![],
            leave: vec![],
        })
        .map_err(debug)?;
    if trace != Trace::MissingIcons {
        let mut pushed = icons(trace == Trace::WrongDeltaIcons);
        if trace == Trace::WrongDeltaTarget {
            pushed[0].entity.generation += 1;
        }
        let encoded = encode_party_icons(&pushed).map_err(debug)?;
        // This one-entry canonical bundle fits a single-byte length, as validated below.
        assert!(encoded.len() < 128);
        payload.push(0x32);
        payload.push(u8::try_from(encoded.len())?);
        payload.extend_from_slice(&encoded);
    }
    let base = BASELINE - u64::from(trace == Trace::StaleRevision);
    let delta = oteryn_protocol_oteryn::encode_state_delta(
        GENERATION + u64::from(trace == Trace::WrongDeltaGeneration),
        if matches!(trace, Trace::DeltaFirst | Trace::ProbeBetween) {
            1
        } else {
            2
        },
        if trace == Trace::WrongDomain { 2 } else { 1 },
        base,
        base + 1,
        if trace == Trace::WrongType { 1 } else { 2 },
        &payload,
    )
    .map_err(debug)?;
    Ok((result, delta))
}

async fn script(server: &mut DuplexStream, trace: Trace) -> TestResult {
    snapshot(server).await?;
    let request = read_frame(server).await?;
    let env = decode_wire_envelope(&request).map_err(debug)?;
    let command = env.client_command(GENERATION).map_err(debug)?;
    assert_eq!(command.command_id, COMMAND);
    assert_eq!(command.command_type, COMMAND_TYPE_PARTY_SKULL_QUERY);
    assert!(command.payload.is_empty());
    let (result, delta) = frames(trace)?;
    if trace == Trace::ResultOnly {
        return write_frame(server, &result).await.map_err(Into::into);
    }
    if trace == Trace::DeltaOnly {
        return write_frame(server, &delta).await.map_err(Into::into);
    }
    let order = if matches!(trace, Trace::DeltaFirst | Trace::ProbeBetween) {
        [delta, result]
    } else {
        [result, delta]
    };
    for (index, frame) in order.into_iter().enumerate() {
        write_frame(server, &frame).await?;
        if trace == Trace::ProbeBetween && index == 0 {
            let probe =
                oteryn_protocol_oteryn::encode_liveness_probe(GENERATION, 19).map_err(debug)?;
            write_frame(server, &probe).await?;
            let frame = read_frame(server).await?;
            let ack = decode_wire_envelope(&frame).map_err(debug)?;
            assert_eq!(ack.message_type(), MessageType::LivenessAck);
            let ack = ack.liveness_ack(GENERATION).map_err(debug)?;
            assert_eq!(ack.probe_id, 19);
            assert_eq!(ack.last_applied_server_sequence, Some(0));
        }
    }
    Ok(())
}

fn observe(trace: Trace) -> TestResult {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        let (mut server, mut client) = tokio::io::duplex(8192);
        let serving = async move {
            let result = script(&mut server, trace).await;
            drop(server);
            result
        };
        let driving = async move {
            let baseline =
                super::party_skull::icon_snapshot_revision(&mut client, GENERATION).await?;
            assert_eq!(baseline, BASELINE);
            write_frame(
                &mut client,
                &encode_client_command(
                    GENERATION,
                    &ClientCommandValue {
                        command_id: COMMAND,
                        command_type: COMMAND_TYPE_PARTY_SKULL_QUERY,
                        payload: &[],
                    },
                )
                .map_err(debug)?,
            )
            .await?;
            super::party_skull::read_icons(
                &mut client,
                GENERATION,
                COMMAND,
                baseline,
                &[entity()],
                &[4],
                &[2],
            )
            .await
        };
        let (served, observed) = super::fixture::join_two(serving, driving).await;
        served?;
        observed
    })
}

#[test]
fn party_icons_require_both_real_observations_in_either_order() -> TestResult {
    for trace in [Trace::ResultFirst, Trace::DeltaFirst, Trace::ProbeBetween] {
        observe(trace)?;
    }
    Ok(())
}

#[test]
fn party_icons_reject_wrong_command_generation_and_revision() -> TestResult {
    for (trace, message) in [
        (Trace::WrongCommand, "wrong command id"),
        (Trace::WrongResultGeneration, "wrong connection generation"),
        (Trace::WrongDeltaGeneration, "wrong connection generation"),
        (Trace::StaleRevision, "snapshot baseline"),
        (Trace::WrongDeltaTarget, "visible icon"),
    ] {
        let error = observe(trace)
            .err()
            .ok_or("uncorrelated observation was accepted")?;
        assert!(error.to_string().contains(message), "{error}");
    }
    Ok(())
}

#[test]
fn party_icons_never_complete_from_unrelated_or_single_observations() -> TestResult {
    for trace in [
        Trace::MissingIcons,
        Trace::WrongDomain,
        Trace::WrongType,
        Trace::ResultOnly,
        Trace::DeltaOnly,
    ] {
        let error = observe(trace)
            .err()
            .ok_or("missing actual icon evidence was accepted")?;
        assert_eq!(
            error
                .downcast_ref::<std::io::Error>()
                .map(std::io::Error::kind),
            Some(std::io::ErrorKind::UnexpectedEof)
        );
    }
    Ok(())
}

#[test]
#[should_panic(expected = "party icon values")]
fn party_icons_reject_wrong_query_payload_values() {
    let result = observe(Trace::WrongResultIcons);
    assert!(result.is_ok());
}

#[test]
#[should_panic(expected = "party icon values")]
fn party_icons_reject_wrong_pushed_payload_values() {
    let result = observe(Trace::WrongDeltaIcons);
    assert!(result.is_ok());
}
