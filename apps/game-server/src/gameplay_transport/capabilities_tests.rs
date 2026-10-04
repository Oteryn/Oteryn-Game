#![allow(clippy::expect_used)]

use super::super::connection::{
    AdmissionRefusal, AdmittedSession, ConnectionEnd, ConnectionIdentifiers, FirstEntryOutcome,
    FreshAdmissionAttempt, FreshAdmissionAuthority, IDLE_LIVENESS, ResumeAttempt,
    SessionContinuity, StepOutcome, admit_frame, serve_admitted,
};
use super::super::world_object::{
    SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
};
use super::super::world_spatial::{
    ActorPosition, CAPABILITY_WORLD_SPATIAL_ENTITIES, SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
    STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, StepDirection, StepDisposition, WorldSpatialObservation,
    encode_world_spatial,
};
use super::*;
use crate::foundation::{
    AuthenticatedTransportRefV1, ChannelId, CharacterId, ExactActorRef, GameSessionId, MessageType,
    ServerAcceptedValue, ServerResumeAcceptedValue, WorldId, decode_wire_envelope,
    encode_server_accepted, encode_server_resume_accepted,
};
use crate::foundation::{DomainSnapshot, encode_single_chunk_snapshot};
use oteryn_protocol_oteryn::achievement_notices::{
    AchievementWatermark, SNAPSHOT_TYPE_ACHIEVEMENT_NOTICES_V1, encode_achievement_notices_snapshot,
};
use oteryn_protocol_oteryn::{
    ClientBootstrapValue, decode_server_accepted, encode_client_bootstrap,
};
use serde_json::Value;
use std::cell::Cell;
use std::error::Error;
use std::future::Future;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

const REGISTRY: &str = include_str!("../../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");

const CHARACTER: [u8; 16] = uuid_v7(0x11);
const SESSION: [u8; 16] = uuid_v7(0x22);
const WORLD: [u8; 16] = uuid_v7(0x33);
const CHANNEL: [u8; 16] = uuid_v7(0x44);
const ACCOUNT: [u8; 16] = uuid_v7(0x55);
const TRANSPORT: [u8; 16] = [0x5a; 16];

const fn uuid_v7(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70 | (tag & 0x0f);
    bytes[8] = 0x80 | (tag & 0x3f);
    bytes
}

/// An injected offered set: 1 alone, 7 requiring 8, 8, 10 requiring 1, 6 requiring an unoffered 99.
const INJECTED: &[OfferedCapability] = &[
    OfferedCapability {
        id: 1,
        requires: &[],
    },
    OfferedCapability {
        id: 6,
        requires: &[99],
    },
    OfferedCapability {
        id: 7,
        requires: &[8],
    },
    OfferedCapability {
        id: 8,
        requires: &[],
    },
    OfferedCapability {
        id: 10,
        requires: &[1],
    },
];

fn registry_capabilities() -> Result<Vec<Value>, Box<dyn Error>> {
    let registry: Value = serde_json::from_str(REGISTRY)?;
    Ok(registry["capabilities"]
        .as_array()
        .ok_or("capabilities")?
        .clone())
}

fn ids(value: &Value) -> Result<Vec<u32>, Box<dyn Error>> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Array(items) => items
            .iter()
            .map(|item| {
                item.as_u64()
                    .and_then(|id| u32::try_from(id).ok())
                    .ok_or_else(|| "id".into())
            })
            .collect(),
        _ => Err("ids".into()),
    }
}

fn selection(ids: &[u32]) -> SelectedCapabilities {
    let offered: Vec<OfferedCapability> = ids
        .iter()
        .map(|&id| OfferedCapability { id, requires: &[] })
        .collect();
    SelectedCapabilities::select(&offered, ids).expect("selection")
}

#[test]
fn the_production_offered_set_is_the_registry_offered_set_and_registered()
-> Result<(), Box<dyn Error>> {
    let mut registered = Vec::new();
    let mut offered = Vec::new();
    for capability in registry_capabilities()? {
        let id = u32::try_from(capability["id"].as_u64().ok_or("id")?)?;
        registered.push(id);
        if capability["offered"].as_bool().ok_or("offered")? {
            offered.push((id, ids(&capability["requires"])?));
        }
    }
    let production: Vec<(u32, Vec<u32>)> = PRODUCTION_OFFERED_CAPABILITIES
        .iter()
        .map(|capability| (capability.id, capability.requires.to_vec()))
        .collect();
    assert_eq!(production, offered);
    assert!(
        PRODUCTION_OFFERED_CAPABILITIES
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id)
    );
    assert!(PRODUCTION_OFFERED_CAPABILITIES.len() <= SELECTED_CAPACITY);
    for capability in PRODUCTION_OFFERED_CAPABILITIES {
        assert!(
            capability
                .requires
                .iter()
                .all(|required| registered.contains(required))
        );
    }
    // `REGISTERED_CAPABILITY_IDS_V1` is what the acceptance encoder checks a selected set
    // against: the whole offered set, selected at once, is accepted, and an unregistered ID is
    // not.
    let accepted = |selected: &[u32]| {
        encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&SESSION).expect("session"),
            world_id: WorldId::decode(&WORLD).expect("world"),
            channel_id: ChannelId::decode(&CHANNEL).expect("channel"),
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 1,
            schema_revision: 1,
            selected_capabilities: selected,
        })
    };
    let all: Vec<u32> = PRODUCTION_OFFERED_CAPABILITIES
        .iter()
        .map(|capability| capability.id)
        .collect();
    assert!(accepted(&all).is_ok());
    assert!(accepted(&registered).is_ok());
    assert!(accepted(&[2]).is_err());
    Ok(())
}

#[test]
fn the_gated_commands_and_domains_are_the_registry_ones() -> Result<(), Box<dyn Error>> {
    let mut expected = Vec::new();
    for capability in registry_capabilities()? {
        let id = u32::try_from(capability["id"].as_u64().ok_or("id")?)?;
        // Capability 6 extends core domain 1 with a payload type; it gates no whole domain.
        if id == CAPABILITY_WORLD_SPATIAL_ENTITIES {
            assert!(ids(&capability["command_types"])?.is_empty());
            assert_eq!(ids(&capability["state_domains"])?, [1]);
            continue;
        }
        expected.push((
            id,
            ids(&capability["command_types"])?,
            ids(&capability["state_domains"])?,
        ));
    }
    let table: Vec<(u32, Vec<u32>, Vec<u32>)> = gated_table()
        .iter()
        .map(|(id, commands, domains)| (*id, commands.to_vec(), domains.to_vec()))
        .collect();
    assert_eq!(table, expected);
    Ok(())
}

#[test]
fn selection_is_the_offered_supported_intersection_and_ignores_unknown_ids() {
    let selected = SelectedCapabilities::select(INJECTED, &[1, 3, 8, 500, 4_000_000]);
    assert_eq!(selected.map(|s| s.as_slice().to_vec()), Some(vec![1, 8]));
    let none = SelectedCapabilities::select(INJECTED, &[2, 3, 500]);
    assert_eq!(none, Some(SelectedCapabilities::NONE));
    assert_eq!(
        SelectedCapabilities::select(INJECTED, &[]),
        Some(SelectedCapabilities::NONE)
    );
}

#[test]
fn selection_keeps_a_capability_only_with_all_its_requires() {
    let select = |supported: &[u32]| {
        SelectedCapabilities::select(INJECTED, supported)
            .expect("bounded")
            .as_slice()
            .to_vec()
    };
    // 7 requires 8; 10 requires 1; 6 requires 99, which is never offered.
    assert_eq!(select(&[7]), Vec::<u32>::new());
    assert_eq!(select(&[7, 8]), [7, 8]);
    assert_eq!(select(&[10]), Vec::<u32>::new());
    assert_eq!(select(&[1, 10]), [1, 10]);
    assert_eq!(select(&[6, 99]), Vec::<u32>::new());
    assert_eq!(select(&[1, 6, 7, 8, 10, 99]), [1, 7, 8, 10]);
    // The closure is transitive: a requirement that drops out takes its dependants with it.
    const CHAIN: &[OfferedCapability] = &[
        OfferedCapability {
            id: 1,
            requires: &[6],
        },
        OfferedCapability {
            id: 6,
            requires: &[7],
        },
        OfferedCapability {
            id: 7,
            requires: &[8],
        },
        OfferedCapability {
            id: 8,
            requires: &[],
        },
    ];
    let chain = |supported: &[u32]| {
        SelectedCapabilities::select(CHAIN, supported)
            .expect("bounded")
            .as_slice()
            .to_vec()
    };
    assert_eq!(chain(&[1, 6, 7]), Vec::<u32>::new());
    assert_eq!(chain(&[1, 6, 7, 8]), [1, 6, 7, 8]);
    assert_eq!(chain(&[1, 7, 8]), [7, 8]);
}

#[test]
fn the_production_set_selects_only_capability_13_whatever_the_client_supports()
-> Result<(), Box<dyn Error>> {
    let mut everything: Vec<u32> = Vec::new();
    for capability in registry_capabilities()? {
        everything.push(u32::try_from(capability["id"].as_u64().ok_or("id")?)?);
    }
    everything.extend([2, 3, 4, 5, 9, 11, 12, 13, 14]);
    everything.sort_unstable();
    everything.dedup();
    assert_eq!(
        SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, &everything)
            .as_ref()
            .map(SelectedCapabilities::as_slice),
        Some(&[13][..])
    );
    Ok(())
}

#[test]
fn resume_keeps_exactly_the_original_selection() {
    let original = SelectedCapabilities::select(INJECTED, &[1, 8]).expect("bounded");
    assert!(original.resumable_with(&[1, 8]));
    // A wider supported set is fine but never widens the session's selection.
    assert!(original.resumable_with(&[1, 7, 8, 10]));
    assert_eq!(original.as_slice(), [1, 8]);
    // A supported set that lacks a selected capability cannot keep it.
    assert!(!original.resumable_with(&[1]));
    assert!(!original.resumable_with(&[]));
    assert!(SelectedCapabilities::NONE.resumable_with(&[]));
}

#[test]
fn commands_and_domains_follow_the_selection() {
    let none = SelectedCapabilities::NONE;
    // Core commands and domains never depend on a capability.
    for core in [1, 2, 3, 10] {
        assert!(none.command_selected(core), "command {core}");
    }
    for core in [1, 2, 3] {
        assert!(none.domain_selected(core), "domain {core}");
    }
    // Capability 1: commands 4 and 5, domains 4 and 5. Capability 7: command 13, domain 12.
    // Capability 8: domain 13. Capability 10: domain 15. Capability 16: command 22, domain 16.
    // Capability 17: commands 11 and 12, domain 10.
    for (capability, commands, domains) in [
        (1, &[4, 5][..], &[4, 5][..]),
        (7, &[13][..], &[12][..]),
        (8, &[][..], &[13][..]),
        (10, &[][..], &[15][..]),
        (16, &[22][..], &[16][..]),
        (17, &[11, 12][..], &[10][..]),
    ] {
        let selected = selection(&[capability]);
        for &command in commands {
            assert!(!none.command_selected(command), "command {command}");
            assert!(selected.command_selected(command), "command {command}");
        }
        for &domain in domains {
            assert!(!none.domain_selected(domain), "domain {domain}");
            assert!(selected.domain_selected(domain), "domain {domain}");
        }
    }
    assert!(!selection(&[8]).command_selected(13));
    assert!(!selection(&[7]).domain_selected(13));
    assert!(!selection(&[14]).command_selected(22));
    assert!(!selection(&[14]).domain_selected(16));
    assert!(!selection(&[16]).command_selected(11));
    assert!(!selection(&[16]).command_selected(12));
    assert!(!selection(&[16]).domain_selected(10));
}

// The production connection path: `admit_frame` and `serve_admitted` with an injected offered
// set, or the production one.

struct Identifiers;
impl ConnectionIdentifiers for Identifiers {
    fn game_session_id(&self) -> Option<GameSessionId> {
        GameSessionId::decode(&SESSION).ok()
    }
    fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1> {
        AuthenticatedTransportRefV1::decode(&TRANSPORT).ok()
    }
}

/// Admits and resumes; `resume` hands back `lost` without checking the selection, so the
/// connection's own refusal is observable. `offered` is `None` for the production set.
struct NegotiatingAuthority {
    offered: Option<&'static [OfferedCapability]>,
    lost: Cell<Option<AdmittedSession>>,
    steps: Cell<u32>,
}

impl NegotiatingAuthority {
    fn new(offered: Option<&'static [OfferedCapability]>) -> Self {
        Self {
            offered,
            lost: Cell::new(None),
            steps: Cell::new(0),
        }
    }
}

fn session(continuity: SessionContinuity) -> Result<AdmittedSession, Box<dyn Error>> {
    let world_id = WorldId::decode(&WORLD)?;
    let channel_id = ChannelId::decode(&CHANNEL)?;
    Ok(AdmittedSession {
        game_session_id: GameSessionId::decode(&SESSION)?,
        world_id,
        channel_id,
        runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(super::super::connection::ControllerBinding {
            transport: AuthenticatedTransportRefV1::decode(&TRANSPORT)?,
            account_id: ACCOUNT,
        }),
        continuity,
        item_fence: None,
    })
}

impl FreshAdmissionAuthority for NegotiatingAuthority {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        session(SessionContinuity::FRESH).map_err(|_| AdmissionRefusal::Unavailable)
    }

    async fn resume(
        &self,
        _attempt: ResumeAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        self.lost.get().ok_or(AdmissionRefusal::Rejected)
    }

    fn offered_capabilities(&self) -> &'static [OfferedCapability] {
        self.offered.unwrap_or(PRODUCTION_OFFERED_CAPABILITIES)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
        })
    }

    async fn step(&self, _actor: ExactActorRef, _direction: StepDirection) -> StepOutcome {
        self.steps.set(self.steps.get() + 1);
        StepOutcome {
            disposition: StepDisposition::Blocked,
            moved_to: None,
        }
    }

    async fn observe_achievement_notices(
        &self,
        _account_id: [u8; 16],
    ) -> Option<AchievementWatermark> {
        Some(AchievementWatermark {
            fact_count: 1,
            total_points: 3,
        })
    }
}

fn varint(output: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        output.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

fn scalar(output: &mut Vec<u8>, field: u64, value: u64) {
    varint(output, field << 3);
    varint(output, value);
}

fn bytes(output: &mut Vec<u8>, field: u64, value: &[u8]) {
    varint(output, (field << 3) | 2);
    varint(output, value.len() as u64);
    output.extend_from_slice(value);
}

fn envelope(message_type: u64, generation: u64, payload: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    scalar(&mut output, 1, message_type);
    if generation != 0 {
        scalar(&mut output, 2, generation);
    }
    bytes(&mut output, 4, payload);
    output
}

fn bootstrap(supported: &[u32]) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(encode_client_bootstrap(&ClientBootstrapValue {
        schema_revision: 1,
        character_id: CharacterId::decode(&CHARACTER)?,
        admission_material: b"grant",
        client_build_id: "cap-neg-test",
        supported_capabilities: supported,
    })?)
}

/// `ClientResume` (FND-02 §9): session 1, material 2, last applied 3, major 4, profile 5,
/// schema 6, capabilities 7, build 8.
fn resume(supported: &[u32]) -> Vec<u8> {
    let mut payload = Vec::new();
    bytes(&mut payload, 1, &SESSION);
    bytes(&mut payload, 2, b"recovery");
    scalar(&mut payload, 4, 1);
    scalar(&mut payload, 5, 1);
    scalar(&mut payload, 6, 1);
    for &capability in supported {
        scalar(&mut payload, 7, u64::from(capability));
    }
    bytes(&mut payload, 8, b"cap-neg-test");
    envelope(3, 0, &payload)
}

fn command(id: u64, command_type: u32, payload: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    scalar(&mut body, 1, id);
    scalar(&mut body, 2, u64::from(command_type));
    bytes(&mut body, 4, payload);
    envelope(7, 1, &body)
}

fn split(output: &[u8]) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let mut frames = Vec::new();
    let mut cursor = 0;
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        frames.push(output[cursor + 4..cursor + 4 + length].to_vec());
        cursor += 4 + length;
    }
    Ok(frames)
}

fn run<F: Future<Output = Result<(), Box<dyn Error>>>>(test: F) -> Result<(), Box<dyn Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?
        .block_on(test)
}

/// One entry frame through `admit_frame`: the end (or session) and the frames written.
async fn admit(
    authority: &NegotiatingAuthority,
    frame: &[u8],
) -> Result<(Result<AdmittedSession, ConnectionEnd>, Vec<Vec<u8>>), Box<dyn Error>> {
    let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 16);
    let admitted = admit_frame(&mut server, frame, authority, &Identifiers).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    Ok((admitted, split(&output)?))
}

/// Serve `admitted` over `frames`; the frames written.
async fn serve(
    authority: &NegotiatingAuthority,
    admitted: AdmittedSession,
    frames: &[Vec<u8>],
) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {
    let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 20);
    for frame in frames {
        let mut framed = (frame.len() as u32).to_be_bytes().to_vec();
        framed.extend_from_slice(frame);
        client.write_all(&framed).await?;
    }
    client.shutdown().await?;
    let end = serve_admitted(&mut server, admitted, authority, IDLE_LIVENESS).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    Ok((end, split(&output)?))
}

fn accepted_selection(frames: &[Vec<u8>]) -> Result<Vec<u32>, Box<dyn Error>> {
    let [frame] = frames else {
        return Err(format!("expected one frame, got {}", frames.len()).into());
    };
    let envelope = decode_wire_envelope(frame)?;
    assert_eq!(envelope.message_type(), MessageType::ServerAccepted);
    Ok(decode_server_accepted(envelope.payload())?.selected_capabilities)
}

#[test]
fn fresh_admission_echoes_and_keeps_the_selection() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = NegotiatingAuthority::new(Some(INJECTED));
        let (admitted, frames) = admit(&authority, &bootstrap(&[1, 7, 8, 10, 77])?).await?;
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        assert_eq!(accepted_selection(&frames)?, [1, 7, 8, 10]);
        assert_eq!(
            admitted.continuity.selected_capabilities.as_slice(),
            [1, 7, 8, 10]
        );
        // Capability 8 selected: domain 13 starts at revision 0.
        assert_eq!(admitted.continuity.achievement_notice_revision, Some(0));

        // Requires closure on the wire: 7 without 8 is not selected.
        let (admitted, frames) = admit(&authority, &bootstrap(&[7])?).await?;
        assert_eq!(accepted_selection(&frames)?, Vec::<u32>::new());
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        assert_eq!(admitted.continuity.achievement_notice_revision, None);
        Ok(())
    })
}

#[test]
fn production_admission_selects_capability_13_and_nothing_else() -> Result<(), Box<dyn Error>> {
    run(async {
        // SPEED-1 (§1.9): the production offered set selects capability 13 for a client that
        // supports it, and only it.
        let authority = NegotiatingAuthority::new(None);
        let (admitted, frames) = admit(&authority, &bootstrap(&[1, 6, 7, 8, 10, 13])?).await?;
        assert_eq!(accepted_selection(&frames)?, [13]);
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        assert_eq!(admitted.continuity.selected_capabilities.as_slice(), [13]);
        assert_eq!(admitted.continuity.achievement_notice_revision, None);
        // A client without it selects nothing.
        let (admitted, frames) = admit(&authority, &bootstrap(&[1, 6, 7, 8, 10])?).await?;
        assert_eq!(accepted_selection(&frames)?, Vec::<u32>::new());
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        assert_eq!(
            admitted.continuity.selected_capabilities,
            SelectedCapabilities::NONE
        );
        Ok(())
    })
}

#[test]
fn resume_keeps_the_original_selection_and_a_downgrade_falls_back_to_fresh_admission()
-> Result<(), Box<dyn Error>> {
    run(async {
        let authority = NegotiatingAuthority::new(Some(INJECTED));
        let (admitted, _) = admit(&authority, &bootstrap(&[1, 8])?).await?;
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        authority.lost.set(Some(admitted));

        // A resume that supports more keeps exactly the original selection.
        let (resumed, frames) = admit(&authority, &resume(&[1, 7, 8, 10])).await?;
        let resumed = resumed.map_err(|end| format!("{end:?}"))?;
        assert_eq!(resumed.continuity.selected_capabilities.as_slice(), [1, 8]);
        let expected = encode_server_resume_accepted(&ServerResumeAcceptedValue {
            game_session_id: resumed.game_session_id,
            connection_generation: resumed.continuity.connection_generation,
            current_server_sequence: resumed.continuity.server_sequence,
            next_command_id: resumed.continuity.next_command_id,
            schema_revision: 1,
            selected_capabilities: &[1, 8],
        })?;
        assert_eq!(frames, [expected]);

        // A resume that lacks selected capability 8 is not acknowledged: the client falls back
        // to fresh admission, which negotiates again from what it now supports.
        let (refused, frames) = admit(&authority, &resume(&[1])).await?;
        assert!(frames.is_empty());
        assert!(matches!(
            refused,
            Err(ConnectionEnd::AdmittedThenDisconnected(_))
        ));
        let (fresh, frames) = admit(&authority, &bootstrap(&[1])?).await?;
        assert!(fresh.is_ok());
        assert_eq!(accepted_selection(&frames)?, [1]);
        Ok(())
    })
}

#[test]
fn an_unselected_capability_command_is_refused_and_a_core_one_dispatched()
-> Result<(), Box<dyn Error>> {
    run(async {
        let authority = NegotiatingAuthority::new(Some(INJECTED));
        let step = oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::North);
        let frames = [
            command(1, 13, b"chat"),
            command(2, 4, b"charm"),
            command(3, 1, &step),
        ];
        let (_, written) = serve(&authority, session(SessionContinuity::FRESH)?, &frames).await?;
        let rejected = |sequence, id| {
            crate::foundation::encode_command_result(
                1,
                sequence,
                id,
                crate::foundation::CommandStatus::Rejected,
                &[],
            )
        };
        assert!(written.contains(&rejected(1, 1)?));
        assert!(written.contains(&rejected(2, 2)?));
        assert_eq!(authority.steps.get(), 1);
        Ok(())
    })
}

#[test]
fn an_unselected_domain_is_never_sent() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = NegotiatingAuthority::new(Some(INJECTED));
        let spatial = encode_world_spatial(&WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
        });
        let notices = encode_achievement_notices_snapshot(AchievementWatermark {
            fact_count: 1,
            total_points: 3,
        });
        let mut domains = vec![DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            revision: 1,
            snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
            payload: &spatial,
        }];
        // The registered overlay domain is always sent, empty at revision 0.
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            revision: 0,
            snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
            payload: &[],
        });
        let core: Vec<Vec<u8>> = encode_single_chunk_snapshot(1, 1, 0, &domains)?.into();
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
            revision: 0,
            snapshot_type: SNAPSHOT_TYPE_ACHIEVEMENT_NOTICES_V1,
            payload: &notices,
        });
        let with_notices: Vec<Vec<u8>> = encode_single_chunk_snapshot(1, 1, 0, &domains)?.into();

        // A notice revision without capability 8 selected sends nothing of domain 13.
        let unselected = SessionContinuity {
            achievement_notice_revision: Some(0),
            ..SessionContinuity::FRESH
        };
        let (_, frames) = serve(&authority, session(unselected)?, &[]).await?;
        assert_eq!(frames, core);
        let selected = SessionContinuity {
            achievement_notice_revision: Some(0),
            selected_capabilities: selection(&[8]),
            ..SessionContinuity::FRESH
        };
        let (_, frames) = serve(&authority, session(selected)?, &[]).await?;
        assert_eq!(frames, with_notices);
        Ok(())
    })
}
