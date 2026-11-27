use super::{
    Admission, AppearanceItemDictionary, CLIENT_SUPPORTED_CAPABILITIES, CharacterId, Direction,
    GameSessionId, GatedSnapshot, Session, SessionEvent, decode_wire_envelope, read_frame,
    world_spatial, write_frame,
};
use oteryn_protocol_oteryn::{
    ChannelId, DomainSnapshot, MessageType, ServerAcceptedValue, WorldId, encode_server_accepted,
    world_object::{
        SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
        encode_world_object_overlay_snapshot,
    },
    world_spatial_entities,
};
use tokio::io::DuplexStream;
type BoxError = Box<dyn std::error::Error + Send + Sync>;

fn uuid_v7(marker: u8) -> [u8; 16] {
    [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, marker]
}

#[cfg(feature = "test-support")]
pub async fn p19_session(
    path: &Path,
    mut observe: impl FnMut(
        &mut Session<DuplexStream>,
        EntityRef,
        Option<&SessionEvent>,
        bool,
    ) -> Result<(), BoxError>,
) -> Result<(), BoxError> {
    let input = ReplayInput::read(path, super::replay_capture::P19_FILES, APPEARANCE)?;
    let mut session = input.admit().await?;
    observe(&mut session, input.entity, None, false)?;
    input.apply_observed(&mut session, |session, event| {
        observe(session, input.entity, Some(event), false)
    })?;
    observe(&mut session, input.entity, None, true)
}
use std::path::Path;
use world_spatial_entities::EntityRef;

pub(crate) const WORLD: u32 = world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY;
pub(crate) const APPEARANCE: u32 = oteryn_protocol_oteryn::appearance::STATE_DOMAIN_APPEARANCE;

pub(crate) struct ReplayInput {
    pub(crate) sequence: u64,
    pub(crate) selected: Vec<u32>,
    pub(crate) refs: Vec<u32>,
    pub(crate) snapshots: Vec<GatedSnapshot>,
    pub(crate) deltas: Vec<Vec<u8>>,
    pub(crate) entity: EntityRef,
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) expected: u8,
}

impl ReplayInput {
    pub(crate) fn read(directory: &Path, names: [&str; 3], primary: u32) -> Result<Self, BoxError> {
        let [snapshot, transcript, expected] =
            names.map(|name| std::fs::read(directory.join(name)));
        Self::decode(&snapshot?, &transcript?, &expected?, primary)
    }

    fn decode(
        snapshot: &[u8],
        mut transcript: &[u8],
        expected: &[u8],
        primary: u32,
    ) -> Result<Self, BoxError> {
        if take::<4>(&mut transcript)? != *b"OTR1" {
            return Err("unrecognized replay transcript".into());
        }
        let sequence = u64::from_le_bytes(take(&mut transcript)?);
        let selected = read_list(&mut transcript)?;
        let refs = read_list(&mut transcript)?;
        let count = u32::from_le_bytes(take(&mut transcript)?);
        let mut snapshots = Vec::new();
        let mut found = false;
        for _ in 0..count {
            let domain_id = u32::from_le_bytes(take(&mut transcript)?);
            let revision = u64::from_le_bytes(take(&mut transcript)?);
            let snapshot_type = u32::from_le_bytes(take(&mut transcript)?);
            let mut payload = read_bytes(&mut transcript)?;
            if domain_id == primary {
                if found || !payload.is_empty() || snapshot.is_empty() {
                    return Err("invalid primary replay snapshot".into());
                }
                found = true;
                payload = snapshot;
            }
            snapshots.push(GatedSnapshot {
                domain_id,
                revision,
                snapshot_type,
                payload: payload.to_vec(),
            });
        }
        if !found {
            return Err("missing primary replay snapshot".into());
        }
        let mut deltas = Vec::new();
        while !transcript.is_empty() {
            deltas.push(read_bytes(&mut transcript)?.to_vec());
        }

        let mut fields = std::str::from_utf8(expected)?.split_ascii_whitespace();
        let hex = fields.next().ok_or("missing expected identity")?.as_bytes();
        let mut identity = [0; 16];
        if hex.len() != identity.len() * 2 {
            return Err("expected a 16-byte identity in hexadecimal".into());
        }
        let (digits, _) = hex.as_chunks::<2>();
        for (byte, digits) in identity.iter_mut().zip(digits) {
            *byte = u8::from_str_radix(std::str::from_utf8(digits)?, 16)?;
        }
        let generation = fields
            .next()
            .ok_or("missing expected generation")?
            .parse()?;
        let expected = fields.next().ok_or("missing expected value")?.parse()?;
        if fields.next().is_some() {
            return Err("unexpected trailing expected fields".into());
        }
        Ok(Self {
            sequence,
            selected,
            refs,
            snapshots,
            deltas,
            entity: EntityRef {
                identity,
                generation,
            },
            expected,
        })
    }

    pub(crate) async fn admit(&self) -> Result<Session<DuplexStream>, BoxError> {
        let world = self
            .snapshots
            .iter()
            .find(|s| s.domain_id == WORLD)
            .ok_or("world snapshot")?;
        let generation =
            world_spatial_entities::decode_world_spatial_entities_snapshot_with_fields(
                &world.payload,
                world_spatial_entities::EntityFields::selected(&self.selected),
            )
            .map_err(|e| format!("{e:?}"))?
            .0
            .content_generation;
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let selected = self.selected.clone();
        let snapshots = self.snapshots.clone();
        let sequence = self.sequence;
        let peer =
            tokio::spawn(
                async move { server_join(&mut server, &selected, &snapshots, sequence).await },
            );
        let session = Session::admit(
            client,
            Admission {
                appearance_items: (!self.refs.is_empty()).then_some(AppearanceItemDictionary {
                    content_generation: generation,
                    definition_refs: &self.refs,
                }),
                schema_revision: 1,
                character_id: CharacterId::decode(&uuid_v7(4))?,
                admission_material: b"grant",
                client_build_id: "oteryn-session-replay",
                supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
                deadline: std::time::Duration::from_secs(5),
            },
        )
        .await;
        let session = session?;
        peer.await??;
        Ok(session)
    }

    #[cfg(test)]
    pub(crate) fn apply(
        &self,
        session: &mut Session<DuplexStream>,
    ) -> Result<Vec<SessionEvent>, BoxError> {
        self.apply_observed(session, |_, _| Ok(()))
    }

    fn apply_observed(
        &self,
        session: &mut Session<DuplexStream>,
        mut observe: impl FnMut(&mut Session<DuplexStream>, &SessionEvent) -> Result<(), BoxError>,
    ) -> Result<Vec<SessionEvent>, BoxError> {
        let mut last = self.sequence;
        self.deltas
            .iter()
            .map(|frame| {
                let envelope = decode_wire_envelope(frame)?;
                envelope.validate(Direction::ServerToClient, true)?;
                if envelope.message_type() != MessageType::StateDelta
                    || envelope.server_sequence() <= last
                {
                    return Err("out-of-order replay delta".into());
                }
                last = envelope.server_sequence();
                let event = session.apply_state_delta(last, envelope.payload())?;
                observe(session, &event)?;
                Ok(event)
            })
            .collect()
    }
}

async fn server_join(
    server: &mut DuplexStream,
    selected: &[u32],
    snapshots: &[GatedSnapshot],
    sequence: u64,
) -> Result<(), BoxError> {
    let _ = read_frame(server).await?;
    write_frame(
        server,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&uuid_v7(1))?,
            world_id: WorldId::decode(&uuid_v7(2))?,
            channel_id: ChannelId::decode(&uuid_v7(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 7,
            schema_revision: 1,
            selected_capabilities: selected,
        })?,
    )
    .await?;
    let overlay = encode_world_object_overlay_snapshot(&[]).map_err(|e| format!("{e:?}"))?;
    let mut domains = vec![DomainSnapshot {
        domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
        snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
        revision: 1,
        payload: &overlay,
    }];
    domains.extend(snapshots.iter().map(|snapshot| DomainSnapshot {
        domain_id: snapshot.domain_id,
        snapshot_type: snapshot.snapshot_type,
        revision: snapshot.revision,
        payload: &snapshot.payload,
    }));
    for frame in oteryn_protocol_oteryn::encode_snapshot(1, 1, sequence, &domains)? {
        write_frame(server, &frame).await?;
    }
    Ok(())
}

fn take<const LEN: usize>(input: &mut &[u8]) -> Result<[u8; LEN], BoxError> {
    Ok(take_bytes(input, LEN)?.try_into()?)
}

fn take_bytes<'a>(input: &mut &'a [u8], len: usize) -> Result<&'a [u8], BoxError> {
    let (bytes, rest) = input
        .split_at_checked(len)
        .ok_or("truncated replay transcript")?;
    *input = rest;
    Ok(bytes)
}

fn read_bytes<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], BoxError> {
    let len = u32::from_le_bytes(take(input)?) as usize;
    take_bytes(input, len)
}

fn read_list(input: &mut &[u8]) -> Result<Vec<u32>, BoxError> {
    let count = u32::from_le_bytes(take(input)?);
    (0..count)
        .map(|_| Ok(u32::from_le_bytes(take(input)?)))
        .collect()
}

#[cfg(test)]
pub(crate) fn apply_frame(
    session: &mut Session<DuplexStream>,
    frame: &[u8],
) -> Result<SessionEvent, super::SessionError> {
    let envelope = decode_wire_envelope(frame)?;
    session.apply_state_delta(envelope.server_sequence(), envelope.payload())
}

/// Feed the three named file buffers into the same decoder as capture mode.
#[cfg(test)]
pub(crate) fn round_trip(
    files: [(&str, Vec<u8>); 3],
    names: [&str; 3],
    primary: u32,
) -> Result<ReplayInput, BoxError> {
    let [
        (snapshot_name, snapshot),
        (transcript_name, transcript),
        (expected_name, expected),
    ] = files;
    assert_eq!([snapshot_name, transcript_name, expected_name], names);
    ReplayInput::decode(&snapshot, &transcript, &expected, primary)
}
