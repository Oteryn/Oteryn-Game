//! The M02 replay consumes the sequenced payloads recorded by the live session.

use super::*;
use oteryn_dev_client::native_session::{Admission, Session, write_frame};
use oteryn_protocol_oteryn::{
    CharacterId, CommandStatus, MessageType, decode_command_result, decode_wire_envelope,
    refusal_detail,
};
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Capture {
    pub game_session_id: String,
    pub command_id: u64,
    pub character: [u8; 16],
    pub first_sequence: u64,
    pub result_sequence: u64,
    pub last_sequence: u64,
    pub opening_end: u64,
    pub passage: [(u64, u64); 6],
    pub logout: (u64, u64),
    pub setup: [(u64, u64); 3],
    pub setup_end: u64,
    pub capabilities: Vec<u32>,
    pub content_generation: [u8; 32],
    pub bundle_digest: [u8; 32],
    pub reset_epoch: u64,
    pub first_visible_floor: i16,
    pub ordinal: u8,
}

impl Capture {
    pub(super) fn identity(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.game_session_id,
            self.command_id,
            self.first_sequence,
            self.result_sequence,
            self.last_sequence
        )
    }
    fn commands(&self) -> [(u64, u64); 11] {
        [
            self.setup[0],
            self.setup[1],
            self.setup[2],
            (self.command_id, self.result_sequence),
            self.passage[0],
            self.passage[1],
            self.passage[2],
            self.passage[3],
            self.passage[4],
            self.passage[5],
            self.logout,
        ]
    }

    fn command_at(&self, sequence: u64) -> u64 {
        if sequence > self.setup_end && sequence < self.result_sequence {
            return self.command_id;
        }
        self.commands()
            .into_iter()
            .rev()
            .find(|(_, result)| *result <= sequence)
            .map_or(self.setup[0].0, |(command, _)| command)
    }

    fn is_result(&self, sequence: u64) -> bool {
        self.commands()
            .iter()
            .any(|(_, result)| *result == sequence)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    command_id: u64,
    game_session_id: String,
    payload: Vec<u8>,
    server_sequence: u64,
    status: u32,
}

pub(super) type SequencedFrames = Vec<(u64, MessageType, Vec<u8>)>;

pub(super) fn captured_motion(
    frames: &[(u64, MessageType, Vec<u8>)],
    capabilities: &[u32],
    mut previous: ActorPosition,
) -> TestResult<Vec<(u64, ActorPosition)>> {
    use oteryn_protocol_oteryn::{decode_state_delta, world_spatial, world_spatial_entities};
    let fields = world_spatial_entities::EntityFields::selected(capabilities);
    let mut motion = Vec::new();
    for (sequence, kind, payload) in frames {
        if *kind != MessageType::StateDelta {
            continue;
        }
        let delta = decode_state_delta(payload)?;
        if delta.domain_id != world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY {
            continue;
        }
        let position = match delta.delta_type {
            world_spatial::DELTA_TYPE_WORLD_SPATIAL_V1 => {
                world_spatial::decode_world_spatial(delta.payload)
                    .map_err(|error| format!("M02 captured spatial observation: {error:?}"))?
                    .actor_position
            }
            world_spatial_entities::DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2 => {
                world_spatial_entities::decode_world_spatial_entities_delta_with_sidecar(
                    delta.payload,
                    fields,
                )
                .map_err(|error| format!("M02 captured spatial delta: {error:?}"))?
                .0
                .actor_position
            }
            _ => return Err("M02 captured unknown spatial delta type".into()),
        };
        if position != previous {
            motion.push((*sequence, position));
            previous = position;
        }
    }
    Ok(motion)
}

pub(super) fn verify_live_motion(binding: &Capture, frames: &SequencedFrames) -> TestResult {
    verify_motion(
        binding,
        &captured_motion(frames, &binding.capabilities, EXPECTED_ARRIVAL)?,
    )
}

fn verify_motion(binding: &Capture, motion: &[(u64, ActorPosition)]) -> TestResult {
    let mut retreat = Vec::new();
    let mut walk = Vec::new();
    let mut passage = Vec::new();
    for &(sequence, position) in motion {
        if sequence <= binding.setup_end {
            retreat.push((binding.command_at(sequence), position));
        } else if sequence < binding.result_sequence {
            walk.push(position);
        } else if sequence > binding.opening_end && sequence < binding.logout.1 {
            passage.push((binding.command_at(sequence), position));
        } else {
            return Err(
                format!("M02 unexpected movement at sequence {sequence}: {position:?}").into(),
            );
        }
    }
    if retreat
        != [
            (binding.setup[1].0, RETREAT[0]),
            (binding.setup[2].0, RETREAT[1]),
        ]
        || !is_expected_walk(&walk)
        || !passage.into_iter().eq(binding
            .passage
            .iter()
            .map(|(command, _)| *command)
            .zip(PASSAGE))
    {
        return Err(format!(
            "M02 complete spatial stream differs from the pinned route: {motion:?}"
        )
        .into());
    }
    Ok(())
}

pub(super) fn write_capture(
    root: &Path,
    binding: &Capture,
    frames: SequencedFrames,
    wire: Vec<Vec<u8>>,
) -> TestResult {
    let mut envelopes = Vec::with_capacity(frames.len());
    for (server_sequence, kind, payload) in frames {
        let required_type = if binding.is_result(server_sequence) {
            MessageType::CommandResult
        } else {
            MessageType::StateDelta
        };
        if kind != required_type {
            return Err(
                format!("M02 unexpected captured frame {kind:?} at {server_sequence}").into(),
            );
        }
        envelopes.push(Envelope {
            command_id: binding.command_at(server_sequence),
            game_session_id: binding.game_session_id.clone(),
            payload,
            server_sequence,
            status: CommandStatus::Accepted as u32,
        });
    }
    let wire_end = wire
        .iter()
        .position(|body| {
            decode_wire_envelope(body)
                .is_ok_and(|frame| frame.server_sequence() == binding.last_sequence)
        })
        .ok_or("M02 original wire is missing its final frame")?;
    std::fs::File::create_new(root.join("m02-binding.json"))?
        .write_all(&serde_json::to_vec(binding)?)?;
    serde_json::to_writer(
        std::fs::File::create_new(root.join("m02-envelope.json"))?,
        &envelopes,
    )?;
    serde_json::to_writer(
        std::fs::File::create_new(root.join("m02-wire.json"))?,
        &wire[..=wire_end],
    )?;
    Ok(())
}

fn verify_identity(binding: &Capture, expected: &str, frames: &[Envelope]) -> TestResult {
    let hex = binding.game_session_id.replace('-', "");
    let uuid = u128::from_str_radix(&hex, 16)?.to_be_bytes();
    oteryn_protocol_oteryn::GameSessionId::decode(&uuid)?;
    if uuid_text(&uuid) != binding.game_session_id || binding.identity() != expected {
        return Err("M02 capture does not match the live run identity".into());
    }
    if binding.setup[0].0 == 0
        || binding.first_sequence > binding.setup[0].1
        || binding.setup[2].1 >= binding.setup_end
        || binding.setup_end >= binding.result_sequence
        || binding.result_sequence >= binding.opening_end
        || binding.opening_end >= binding.passage[0].1
        || binding.passage[5].1 >= binding.logout.1
        || binding.logout.1 != binding.last_sequence
        || binding
            .commands()
            .windows(2)
            .any(|pair| pair[0].0.checked_add(1) != Some(pair[1].0) || pair[0].1 >= pair[1].1)
        || !(1..=64).contains(&frames.len())
        || binding.last_sequence.checked_sub(binding.first_sequence)
            != Some(frames.len() as u64 - 1)
        || frames.iter().enumerate().any(|(index, frame)| {
            frame.game_session_id != binding.game_session_id
                || frame.command_id != binding.command_at(frame.server_sequence)
                || frame.status != CommandStatus::Accepted as u32
                || binding.first_sequence.checked_add(index as u64) != Some(frame.server_sequence)
        })
    {
        return Err("M02 invalid capture command, status, or sequence".into());
    }
    Ok(())
}

async fn replay(binding: &Capture, frames: &[Envelope], wire: &[Vec<u8>]) -> TestResult {
    let join_end = wire
        .iter()
        .position(|body| {
            decode_wire_envelope(body)
                .is_ok_and(|frame| frame.message_type() == MessageType::SnapshotCommit)
        })
        .ok_or("M02 original admission snapshot is missing")?;
    let (client, mut peer) = tokio::io::duplex(64 * 1024);
    let bootstrap = async {
        oteryn_dev_client::native_session::read_frame(&mut peer).await?;
        for body in &wire[..=join_end] {
            write_frame(&mut peer, body).await?;
        }
        Ok::<_, oteryn_dev_client::SessionError>(())
    };
    let admission = Session::admit(
        client,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: CharacterId::decode(&binding.character)?,
            admission_material: b"M02 offline replay",
            client_build_id: "M02 replay",
            supported_capabilities: oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES,
            deadline: Duration::from_secs(5),
        },
    );
    let (mut session, ()) = tokio::try_join!(admission, bootstrap)?;
    if uuid_text(session.join_snapshot().game_session_id.as_bytes()) != binding.game_session_id {
        return Err("M02 original admission session differs from capture binding".into());
    }

    if session.selected_capabilities() != binding.capabilities {
        return Err("M02 replay negotiation differs from the capture".into());
    }

    let mut previous = EXPECTED_ARRIVAL;
    let mut gate = None;
    let mut arrival = None;
    let mut opening = None;
    let mut frames = frames.iter();
    let mut seeded = false;
    let mut motion = Vec::new();
    for body in &wire[join_end + 1..] {
        let raw = decode_wire_envelope(body)?;
        let sequence = raw.server_sequence();
        if sequence > binding.last_sequence {
            return Err("M02 original wire extends beyond the capture binding".into());
        }
        if sequence == binding.first_sequence {
            if session.world_spatial().actor_position != EXPECTED_ARRIVAL {
                return Err("M02 replay setup did not reach the exact approach".into());
            }
            let map = session
                .world_map_snapshot()
                .ok_or("M02 replay missing setup map")?;
            if map.header.content_generation != binding.content_generation
                || map.header.bundle_digest != binding.bundle_digest
                || map.header.reset_epoch != binding.reset_epoch
                || map.header.first_visible_floor != binding.first_visible_floor
            {
                return Err("M02 replay map differs from the live setup binding".into());
            }
            gate = map
                .tiles
                .into_iter()
                .find(|tile| tile.position == GATE_POSITION);
            if !gate
                .as_ref()
                .is_some_and(|tile| gate_stack_matches(tile, binding.ordinal, CLOSED_GATE, 0))
            {
                return Err("M02 replay setup did not observe the closed gate".into());
            }
            verify_closed_probe_tile(
                gate.as_ref(),
                binding.ordinal,
                session
                    .world_entities()
                    .ok_or("M02 replay requires a visible entity baseline")?
                    .others()
                    .map(|entity| entity.position),
            )?;
            seeded = true;
        }
        let event = session.apply_qualification_frame(body)?;
        // Liveness is unsequenced and may arrive in any phase. Validate it above
        // before bypassing the sequenced setup and command assertions.
        if sequence == 0 {
            continue;
        }
        if sequence < binding.first_sequence {
            if session.world_spatial().actor_position == FAR_SIDE {
                return Err("M02 replay reached the logout tile before the mechanic".into());
            }
            continue;
        }
        let frame = frames.next().ok_or("M02 extra original frame")?;
        if sequence != frame.server_sequence || raw.payload() != frame.payload {
            return Err("M02 original wire differs from the capture envelope".into());
        }
        if binding.is_result(sequence) {
            let result = decode_command_result(&frame.payload)?;
            let payload = if binding.capabilities.contains(&66) {
                let (payload, refusal) = refusal_detail::split_refusal_detail(result.payload)
                    .map_err(|error| format!("M02 replay refusal detail: {error:?}"))?;
                let expected = (sequence == binding.setup[0].1)
                    .then_some(refusal_detail::RefusalDetail::NotPossible);
                if refusal != expected {
                    return Err("M02 replay result has unexpected refusal detail".into());
                }
                payload
            } else {
                result.payload
            };
            let valid_disposition = if sequence == binding.logout.1 {
                oteryn_protocol_oteryn::logout::decode_logout_result(payload)
                    .map_err(|error| format!("M02 replay logout: {error:?}"))?
                    == oteryn_protocol_oteryn::logout::LogoutResult::Accepted
            } else if sequence == binding.result_sequence {
                oteryn_protocol_oteryn::world_object::decode_use_result(payload)
                    .map_err(|error| format!("M02 replay USE result: {error:?}"))?
                    == UseDisposition::Committed
            } else {
                oteryn_protocol_oteryn::world_spatial::decode_step_result_profile(
                    payload,
                    binding.capabilities.contains(&13),
                    binding.capabilities.contains(
                        &oteryn_protocol_oteryn::world_spatial::CAPABILITY_DIAGONAL_MOVEMENT_V1,
                    ),
                )
                .map_err(|error| format!("M02 replay STEP result: {error:?}"))?
                .disposition
                    == if sequence == binding.setup[0].1 {
                        StepDisposition::Blocked
                    } else {
                        StepDisposition::Moved
                    }
            };
            if result.command_id != frame.command_id
                || result.status != CommandStatus::Accepted
                || !valid_disposition
            {
                return Err(
                    "M02 replay did not receive the exact successful command result".into(),
                );
            }
            continue;
        }
        let event = event.ok_or("M02 expected a state event")?;
        let position = session.world_spatial().actor_position;
        if position != previous {
            previous = position;
            motion.push((sequence, position));
            if sequence > binding.setup_end
                && sequence < binding.result_sequence
                && position == EXPECTED_ARRIVAL
            {
                arrival = Some(sequence);
            }
        }
        observe_gate(
            &[event],
            &mut gate,
            binding.ordinal,
            &mut opening,
            sequence > binding.setup_end,
        )?;
        if sequence < binding.setup[0].1 {
            verify_closed_probe_tile(
                gate.as_ref(),
                binding.ordinal,
                session
                    .world_entities()
                    .ok_or("M02 replay lost its visible entities")?
                    .others()
                    .map(|entity| entity.position),
            )?;
        }
    }
    if !seeded || frames.next().is_some() {
        return Err("M02 original frame stream is incomplete".into());
    }
    verify_relog_position(session.world_spatial().actor_position)?;
    verify_open_order(arrival, opening)?;
    if opening.is_some_and(|sequence| sequence > binding.opening_end) {
        return Err("M02 gate first opened during passage".into());
    }
    verify_motion(binding, &motion)?;
    if !gate
        .as_ref()
        .is_some_and(|tile| gate_stack_matches(tile, binding.ordinal, OPEN_GATE, 1))
    {
        return Err("M02 recorded deltas do not prove the exact open gate stack".into());
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires the shared wrapper's fresh live capture"]
async fn m02_replay_captured_autowalk() -> TestResult {
    let root = Path::new(&required("E2E_ALL_CAPTURE_DIR")?).to_path_buf();
    let expected = required("OTERYN_M02_REPLAY_IDENTITY")?;
    let binding: Capture =
        serde_json::from_reader(std::fs::File::open(root.join("m02-binding.json"))?)?;
    let frames: Vec<Envelope> =
        serde_json::from_reader(std::fs::File::open(root.join("m02-envelope.json"))?)?;
    verify_identity(&binding, &expected, &frames)?;
    let wire: Vec<Vec<u8>> =
        serde_json::from_reader(std::fs::File::open(root.join("m02-wire.json"))?)?;
    replay(&binding, &frames, &wire).await
}

#[cfg(test)]
fn test_binding() -> Capture {
    Capture {
        game_session_id: "01934f10-7c00-7000-8000-000000000001".into(),
        command_id: 7,
        character: [0; 16],
        first_sequence: 10,
        result_sequence: 16,
        last_sequence: 31,
        opening_end: 17,
        passage: [(8, 18), (9, 20), (10, 22), (11, 24), (12, 26), (13, 28)],
        logout: (14, 31),
        setup: [(4, 10), (5, 11), (6, 13)],
        setup_end: 14,
        capabilities: vec![],
        content_generation: [0; 32],
        bundle_digest: [0; 32],
        reset_epoch: 0,
        first_visible_floor: 7,
        ordinal: 1,
    }
}

#[test]
fn m02_requires_exact_capture_identity() -> TestResult {
    let binding = test_binding();
    let expected = binding.identity();
    let mut frames: Vec<_> = (10..=31)
        .map(|server_sequence| Envelope {
            command_id: binding.command_at(server_sequence),
            game_session_id: binding.game_session_id.clone(),
            payload: vec![],
            server_sequence,
            status: CommandStatus::Accepted as u32,
        })
        .collect();
    verify_identity(&binding, &expected, &frames)?;
    assert!(verify_identity(&binding, "stale", &frames).is_err());
    assert!(verify_identity(&binding, &expected, &frames[..2]).is_err());
    frames[0].game_session_id = "01934f10-7c00-7000-8000-000000000002".into();
    assert!(verify_identity(&binding, &expected, &frames).is_err());
    frames[0].game_session_id = binding.game_session_id.clone();
    frames[1].server_sequence = frames[0].server_sequence;
    assert!(verify_identity(&binding, &expected, &frames).is_err());
    frames[1].server_sequence = 11;
    frames[1].command_id = 8;
    assert!(verify_identity(&binding, &expected, &frames).is_err());
    Ok(())
}

#[test]
fn m02_counts_extra_movement_and_return_on_spatial_sidecars() -> TestResult {
    use oteryn_protocol_oteryn::{
        guild_emblem, inventory_capacity, party_skull, world_spatial,
        world_spatial_entities as entities,
    };
    let mut binding = test_binding();
    binding.capabilities = vec![
        party_skull::CAPABILITY_PARTY_SKULL_V1,
        guild_emblem::CAPABILITY_GUILD_EMBLEM_V1,
        inventory_capacity::CAPABILITY_INVENTORY_CAPACITY_V1,
    ];
    // Leave two real sequence slots after the last paced step and before logout.
    binding.result_sequence = 17;
    binding.opening_end = 18;
    binding.passage = [(8, 19), (9, 21), (10, 23), (11, 25), (12, 27), (13, 29)];
    binding.logout = (14, 34);
    binding.last_sequence = 34;
    let correct = [
        (12, RETREAT[0]),
        (14, RETREAT[1]),
        (15, INTERMEDIATE),
        (16, EXPECTED_ARRIVAL),
        (20, PASSAGE[0]),
        (22, PASSAGE[1]),
        (24, PASSAGE[2]),
        (26, PASSAGE[3]),
        (28, PASSAGE[4]),
        (30, PASSAGE[5]),
    ];
    verify_motion(&binding, &correct)?;
    for sidecar in [
        entities::DeltaSidecar::PartyIcons(vec![]),
        entities::DeltaSidecar::GuildEmblems(vec![]),
        entities::DeltaSidecar::InventoryCapacity(inventory_capacity::InventoryCapacity {
            capacity: 100,
            free: 100,
        }),
    ] {
        let frames = [GATE_POSITION, FAR_SIDE]
            .into_iter()
            .enumerate()
            .map(|(index, position)| {
                let sequence = 31 + index as u64;
                let delta = entities::encode_world_spatial_entities_sidecar_delta(
                    [0; 32], position, &sidecar,
                )
                .map_err(|error| format!("{error:?}"))?;
                let frame = oteryn_protocol_oteryn::encode_state_delta(
                    1,
                    sequence,
                    world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    sequence,
                    sequence + 1,
                    entities::DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                    &delta,
                )?;
                let raw = decode_wire_envelope(&frame)?;
                Ok((sequence, raw.message_type(), raw.payload().to_vec()))
            })
            .collect::<TestResult<Vec<_>>>()?;
        let extra = captured_motion(&frames, &binding.capabilities, FAR_SIDE)?;
        assert_eq!(extra, [(31, GATE_POSITION), (32, FAR_SIDE)]);
        assert!(verify_motion(&binding, &[correct.as_slice(), &extra].concat()).is_err());
    }
    Ok(())
}
