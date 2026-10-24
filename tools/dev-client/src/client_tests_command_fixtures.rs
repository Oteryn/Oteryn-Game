// --- `step` / `use_object` against a scripted fake server. Every command the client sends is
// decoded by the fake server through `oteryn-protocol-oteryn`'s own server-side ingress
// (`WireEnvelopeView::client_command`), and every reply is built with the crate's own server
// encoders. The join deliberately starts away from the fresh-admission constants (first
// CommandId 7, snapshot target sequence 40, spatial revision 5, overlay revision 2) so the
// client is proven to take them from the join, not assume them.

type BoxError = Box<dyn StdError + Send + Sync>;
type ServerStream = tokio_rustls::server::TlsStream<TcpStream>;

const GENERATION: u64 = 1;
const FIRST_COMMAND_ID: u64 = 7;
const JOIN_SEQUENCE: u64 = 40;
const JOIN_SPATIAL_REVISION: u64 = 5;
const JOIN_DOOR_REVISION: u64 = 2;
const CONTENT_GENERATION: [u8; 32] = [0x11; 32];
const OTHER_GENERATION: [u8; 32] = [0x99; 32];
const DOOR_OPEN_STATE: &str = "oteryn:reference.state.open";

fn spatial_payload(x: i32, y: i32, content_generation: [u8; 32]) -> Vec<u8> {
    encode_world_spatial(&WireSpatialObservation {
        content_generation,
        actor_position: ActorPosition { x, y, floor: 0 },
    })
}

fn door_entry(state: &str, revision: u64, content_generation: [u8; 32]) -> WireOverlayEntry {
    WireOverlayEntry {
        content_generation,
        placement: DOOR_PLACEMENT.as_bytes().to_vec(),
        state: state.as_bytes().to_vec(),
        revision,
    }
}

fn command_result_frame(
    sequence: u64,
    command_id: u64,
    status: CommandStatus,
    payload: &[u8],
) -> Result<Vec<u8>, BoxError> {
    Ok(oteryn_protocol_oteryn::encode_command_result(
        GENERATION, sequence, command_id, status, payload,
    )?)
}

fn step_result_frame(
    sequence: u64,
    command_id: u64,
    disposition: StepDisposition,
) -> Result<Vec<u8>, BoxError> {
    let status = if disposition == StepDisposition::Rejected {
        CommandStatus::Rejected
    } else {
        CommandStatus::Accepted
    };
    command_result_frame(
        sequence,
        command_id,
        status,
        &world_spatial::encode_step_result(disposition),
    )
}

fn use_result_frame(
    sequence: u64,
    command_id: u64,
    disposition: UseDisposition,
) -> Result<Vec<u8>, BoxError> {
    command_result_frame(
        sequence,
        command_id,
        CommandStatus::Accepted,
        &world_object::encode_use_result(disposition).unwrap_or_default(),
    )
}

fn state_delta_frame(
    sequence: u64,
    domain_id: u32,
    base: u64,
    new: u64,
    delta_type: u32,
    payload: &[u8],
) -> Result<Vec<u8>, BoxError> {
    Ok(oteryn_protocol_oteryn::encode_state_delta(
        GENERATION, sequence, domain_id, base, new, delta_type, payload,
    )?)
}

fn spatial_delta_frame(sequence: u64, base: u64, x: i32, y: i32) -> Result<Vec<u8>, BoxError> {
    state_delta_frame(
        sequence,
        STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
        base,
        base + 1,
        world_spatial::DELTA_TYPE_WORLD_SPATIAL_V1,
        &spatial_payload(x, y, CONTENT_GENERATION),
    )
}

fn overlay_delta_frame(sequence: u64, base: u64, state: &str) -> Result<Vec<u8>, BoxError> {
    let payload = world_object::encode_world_object_overlay_delta(&door_entry(
        state,
        base + 1,
        CONTENT_GENERATION,
    ))
    .map_err(|error| format!("encode_world_object_overlay_delta: {error:?}"))?;
    state_delta_frame(
        sequence,
        STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
        base,
        base + 1,
        world_object::DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
        &payload,
    )
}

async fn read_command(stream: &mut ServerStream) -> Result<(u64, u32, Vec<u8>), BoxError> {
    let body = read_frame(stream).await?;
    let command = decode_wire_envelope(&body)?.client_command(GENERATION)?;
    Ok((
        command.command_id,
        command.command_type,
        command.payload.to_vec(),
    ))
}

async fn expect_step(
    stream: &mut ServerStream,
    command_id: u64,
    direction: StepDirection,
) -> Result<(), BoxError> {
    let (id, command_type, payload) = read_command(stream).await?;
    assert_eq!(
        (id, command_type),
        (
            command_id,
            world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT
        )
    );
    assert_eq!(
        world_spatial::decode_step_intent(&payload),
        Ok(direction),
        "command {command_id}"
    );
    Ok(())
}

async fn expect_use(
    stream: &mut ServerStream,
    command_id: u64,
    placement: &[u8],
    expected_revision: u64,
) -> Result<(), BoxError> {
    let (id, command_type, payload) = read_command(stream).await?;
    assert_eq!(
        (id, command_type),
        (command_id, world_object::COMMAND_TYPE_USE_INTENT)
    );
    assert_eq!(
        world_object::decode_use_intent(&payload),
        Ok(WorldObjectTarget {
            placement: placement.to_vec(),
            expected_revision,
        }),
        "command {command_id}"
    );
    Ok(())
}

async fn send(stream: &mut ServerStream, frames: &[Vec<u8>]) -> Result<(), BoxError> {
    for frame in frames {
        write_frame(stream, frame).await?;
    }
    stream.flush().await?;
    Ok(())
}

type ServerTask = tokio::task::JoinHandle<Result<(), BoxError>>;

/// Admits one client over real TLS, sends `ServerAccepted` (`next_command_id`
/// `FIRST_COMMAND_ID`, `current_server_sequence` `JOIN_SEQUENCE`) and a join snapshot (target
/// sequence `JOIN_SEQUENCE`, spatial domain revision `JOIN_SPATIAL_REVISION` at (0,0,0), the
/// closed door at revision `JOIN_DOOR_REVISION`), then hands the stream to `script`.
async fn joined_session<F, Fut>(
    deadline: Duration,
    script: F,
) -> Result<(DevClientSession, ServerTask), BoxError>
where
    F: FnOnce(ServerStream) -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), BoxError>> + Send + 'static,
{
    let (certificate, acceptor, listener, address) = tls_test_listener().await?;
    let server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await?;
        let mut stream = acceptor.accept(tcp).await?;
        read_and_discard_bootstrap(&mut stream).await?;
        let accepted = encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&test_uuid_v7(1))?,
            world_id: WorldId::decode(&test_uuid_v7(2))?,
            channel_id: ChannelId::decode(&test_uuid_v7(3))?,
            connection_generation: GENERATION,
            current_server_sequence: JOIN_SEQUENCE,
            next_command_id: FIRST_COMMAND_ID,
            schema_revision: 1,
            selected_capabilities: &[],
        })?;
        write_frame(&mut stream, &accepted).await?;
        let overlay = encode_world_object_overlay_snapshot(&[door_entry(
            DOOR_STATE,
            JOIN_DOOR_REVISION,
            CONTENT_GENERATION,
        )])
        .map_err(|error| format!("encode_world_object_overlay_snapshot: {error:?}"))?;
        let spatial = spatial_payload(0, 0, CONTENT_GENERATION);
        let snapshot = encode_single_chunk_snapshot(
            GENERATION,
            1,
            JOIN_SEQUENCE,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: JOIN_SPATIAL_REVISION,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &spatial,
                },
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: JOIN_DOOR_REVISION,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &overlay,
                },
            ],
        )?;
        send(&mut stream, &snapshot).await?;
        script(stream).await
    });
    let session = connect_session(JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: &certificate,
        schema_revision: 1,
        character_id: CharacterId::decode(&test_uuid_v7(4))?,
        admission_material: b"fixture-grant",
        client_build_id: CLIENT_BUILD_ID,
        deadline,
    })
    .await
    .map_err(|error| format!("connect_session: {error}"))?;
    Ok((session, server))
}

/// Waits (bounded) for the client to close, so the scripted server never ends first.
async fn wait_for_client_close(stream: &mut ServerStream) {
    let mut buffer = [0_u8; 1];
    let _ = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buffer)).await;
}

fn door_use_outcome(
    command_id: u64,
    result_server_sequence: u64,
    disposition: UseDisposition,
    overlay: Option<AppliedDelta<WireOverlayEntry>>,
) -> UseOutcome {
    CommandOutcome {
        command_id,
        status: CommandStatus::Accepted,
        disposition,
        result_server_sequence,
        world_spatial_delta: None,
        world_object_overlay_delta: overlay,
        gate_message_id: None,
    }
}

fn step_outcome(
    command_id: u64,
    result_server_sequence: u64,
    disposition: StepDisposition,
    spatial: Option<AppliedDelta<WireSpatialObservation>>,
) -> StepOutcome {
    CommandOutcome {
        command_id,
        status: CommandStatus::Accepted,
        disposition,
        result_server_sequence,
        world_spatial_delta: spatial,
        world_object_overlay_delta: None,
        gate_message_id: None,
    }
}

fn moved_to(
    server_sequence: u64,
    base_revision: u64,
    x: i32,
    y: i32,
) -> Option<AppliedDelta<WireSpatialObservation>> {
    Some(AppliedDelta {
        server_sequence,
        base_revision,
        new_revision: base_revision + 1,
        value: WireSpatialObservation {
            content_generation: CONTENT_GENERATION,
            actor_position: ActorPosition { x, y, floor: 0 },
        },
    })
}

fn door_delta(
    server_sequence: u64,
    base_revision: u64,
    state: &str,
) -> Option<AppliedDelta<WireOverlayEntry>> {
    Some(AppliedDelta {
        server_sequence,
        base_revision,
        new_revision: base_revision + 1,
        value: door_entry(state, base_revision + 1, CONTENT_GENERATION),
    })
}
