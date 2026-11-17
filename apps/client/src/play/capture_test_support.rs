//! Replays unmodified production server transcripts, with their actual admission metadata.
use oteryn_session::test_support::wire;
use serde_json::Value;
pub(crate) type Error = Box<dyn std::error::Error + Send + Sync>;
type FrameBatch = Vec<Vec<u8>>;

pub(crate) struct Capture {
    pub(crate) value: Value,
    pub(crate) character: [u8; 16],
}
impl Capture {
    pub(crate) fn load(variable: &str) -> Result<Option<Self>, Error> {
        Self::load_schema(variable, "OTERYN_NATIVE_SCOPE_REPLAY/v1")
    }
    pub(crate) fn load_schema(variable: &str, schema: &str) -> Result<Option<Self>, Error> {
        let Ok(path) = std::env::var(variable) else {
            eprintln!("SRX3_NATIVE_CAPTURE_SKIPPED {variable} unset; no actual transcript proof");
            return Ok(None);
        };
        let bytes = std::fs::read(path)?;
        if bytes.len() > 32 * 1024 * 1024 {
            return Err("native capture exceeds test bound".into());
        }
        let value: Value = serde_json::from_slice(&bytes)?;
        if value["schema"] != schema {
            return Err("unsupported actual native transcript schema".into());
        }
        let character = hex(&value["character_id"])?
            .try_into()
            .map_err(|_| "actual native character UUID length")?;
        Ok(Some(Self { value, character }))
    }
    pub(crate) fn frames(&self, key: &str) -> Result<Vec<Vec<u8>>, Error> {
        frames(&self.value[key])
    }
}
pub(crate) fn frames(value: &Value) -> Result<Vec<Vec<u8>>, Error> {
    value
        .as_array()
        .ok_or("actual native frame array")?
        .iter()
        .map(hex)
        .collect()
}
pub(crate) fn hex(value: &Value) -> Result<Vec<u8>, Error> {
    let text = value.as_str().ok_or("actual native hex string")?;
    if !text.len().is_multiple_of(2) {
        return Err("odd actual native hex string".into());
    }
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let digit = |v| match v {
                b'0'..=b'9' => Some(v - b'0'),
                b'a'..=b'f' => Some(v - b'a' + 10),
                _ => None,
            };
            Ok(digit(pair[0]).ok_or("native hex digit")? * 16
                + digit(pair[1]).ok_or("native hex digit")?)
        })
        .collect()
}
pub(crate) fn position(value: &Value) -> Result<(oteryn_renderer::TileCoord, i16), Error> {
    let coordinates = value.as_array().ok_or("actual native position")?;
    if coordinates.len() != 3 {
        return Err("actual native position arity".into());
    }
    Ok((
        oteryn_renderer::TileCoord::new(
            i32::try_from(coordinates[0].as_i64().ok_or("native x")?)?,
            i32::try_from(coordinates[1].as_i64().ok_or("native y")?)?,
        ),
        i16::try_from(coordinates[2].as_i64().ok_or("native floor")?)?,
    ))
}
pub(crate) fn split_initial(raw: &[Vec<u8>]) -> Result<(FrameBatch, FrameBatch), Error> {
    let boundary = raw
        .iter()
        .position(|frame| {
            wire::decode_wire_envelope(frame)
                .is_ok_and(|envelope| envelope.message_type() == wire::MessageType::SnapshotCommit)
        })
        .ok_or("actual native initial snapshot commit")?
        + 1;
    Ok((raw[..boundary].to_vec(), raw[boundary..].to_vec()))
}

#[derive(Clone)]
pub(crate) struct Command {
    pub(crate) kind: u32,
    pub(crate) payload: Vec<u8>,
}
impl Command {
    pub(crate) fn read(value: &Value) -> Result<Self, Error> {
        Ok(Self {
            kind: u32::try_from(value["kind"].as_u64().ok_or("actual command kind")?)?,
            payload: hex(&value["payload"])?,
        })
    }
}
pub(crate) struct Epoch {
    pub(crate) session: oteryn_session::Session<tokio::io::DuplexStream>,
    pub(crate) trigger: tokio::sync::mpsc::Sender<()>,
    pub(crate) peer: tokio::task::JoinHandle<Result<(), Error>>,
}
fn number(metadata: &Value, key: &str) -> Result<u64, Error> {
    metadata[key]
        .as_u64()
        .ok_or_else(|| format!("missing actual admission {key}").into())
}
pub(crate) async fn epoch(
    metadata: Value,
    character: [u8; 16],
    initial: Vec<Vec<u8>>,
    command: Option<Command>,
    subsequent: Vec<Vec<u8>>,
) -> Result<Epoch, Error> {
    let capabilities = metadata["selected_capabilities"]
        .as_array()
        .ok_or("actual selected capabilities")?
        .iter()
        .map(|value| u32::try_from(value.as_u64().ok_or("actual capability")?).map_err(Into::into))
        .collect::<Result<Vec<u32>, Error>>()?;
    let peer_caps = capabilities.clone();
    let generation = number(&metadata, "connection_generation")?;
    let command_id = number(&metadata, "next_command_id")?;
    let actual_session_id = hex(&metadata["game_session_id"])?;
    let accepted = wire::encode_server_accepted(&wire::ServerAcceptedValue {
        game_session_id: wire::GameSessionId::decode(&actual_session_id)?,
        world_id: wire::WorldId::decode(&hex(&metadata["world_id"])?)?,
        channel_id: wire::ChannelId::decode(&hex(&metadata["channel_id"])?)?,
        connection_generation: generation,
        current_server_sequence: number(&metadata, "current_server_sequence")?,
        next_command_id: command_id,
        schema_revision: u32::try_from(number(&metadata, "schema_revision")?)?,
        selected_capabilities: &capabilities,
    })?;
    let (client, mut server) = tokio::io::duplex(4 * 1024 * 1024);
    let (trigger, mut control) = tokio::sync::mpsc::channel(1);
    let (scope_contract, admitted_scope) = tokio::sync::oneshot::channel();
    let peer = tokio::spawn(async move {
        let bootstrap = oteryn_session::read_frame(&mut server).await?;
        assert_eq!(
            wire::decode_wire_envelope(&bootstrap)?
                .client_bootstrap()?
                .supported_capabilities,
            peer_caps
        );
        oteryn_session::write_frame(&mut server, &accepted).await?;
        for frame in &initial {
            oteryn_session::write_frame(&mut server, frame).await?;
        }
        let scope_bound = admitted_scope
            .await
            .map_err(|_| "actual admission scope contract closed")?;
        if let Some(expected) = command {
            loop {
                let frame = oteryn_session::read_frame(&mut server).await?;
                let envelope = wire::decode_wire_envelope(&frame)?;
                if envelope.message_type() == wire::MessageType::LivenessAck {
                    continue;
                }
                let actual = envelope.client_command(generation)?;
                let binding = envelope.client_command_session_id()?;
                if scope_bound || binding.is_some() {
                    assert_eq!(
                        binding,
                        Some(actual_session_id.as_slice()),
                        "native scope command must bind the actual captured current session"
                    );
                }
                assert_eq!(
                    actual.command_id, command_id,
                    "native gesture must retain actual command counter"
                );
                assert_eq!(
                    actual.command_type, expected.kind,
                    "uncaptured native command"
                );
                assert_eq!(
                    actual.payload,
                    expected.payload.as_slice(),
                    "native gesture must encode the actual production request"
                );
                eprintln!(
                    "NATIVE_CAPTURE_COMMAND_RECEIVED kind={}",
                    actual.command_type
                );
                break;
            }
        } else {
            control
                .recv()
                .await
                .ok_or("actual autonomous replay trigger closed")?;
        }
        for frame in &subsequent {
            oteryn_session::write_frame(&mut server, frame).await?;
        }
        eprintln!(
            "NATIVE_CAPTURE_RESPONSE_DELIVERED frames={}",
            subsequent.len()
        );
        control
            .recv()
            .await
            .ok_or("actual native epoch completion closed")?;
        Ok::<(), Error>(())
    });
    let session = oteryn_session::Session::admit(
        client,
        oteryn_session::Admission {
            appearance_items: None,
            schema_revision: u32::try_from(number(&metadata, "schema_revision")?)?,
            character_id: oteryn_session::CharacterId::decode(&character)?,
            admission_material: b"actual-production-capture",
            client_build_id: "srx3-actual-native-replay",
            supported_capabilities: &capabilities,
            deadline: std::time::Duration::from_secs(5),
        },
    )
    .await;
    let session = match session {
        Ok(session) => session,
        Err(error) => {
            if peer.is_finished() {
                return Err(format!(
                    "actual native admission: {error:?}; peer: {:?}",
                    peer.await?
                )
                .into());
            }
            peer.abort();
            return Err(format!("actual native admission: {error:?}").into());
        }
    };
    scope_contract
        .send(session.activity_scope().is_some())
        .map_err(|_| "actual native peer closed before its admitted scope contract")?;
    Ok(Epoch {
        session,
        trigger,
        peer,
    })
}
