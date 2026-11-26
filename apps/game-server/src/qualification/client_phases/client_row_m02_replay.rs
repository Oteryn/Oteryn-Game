//! The M02 replay consumes the sequenced payloads recorded by the live session.

use super::*;
use oteryn_protocol_oteryn::{
    CommandStatus, MessageType, decode_command_result, decode_state_delta, refusal_detail,
    world_map, world_spatial, world_spatial_entities,
};
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Capture {
    pub game_session_id: String,
    pub command_id: u64,
    pub first_sequence: u64,
    pub result_sequence: u64,
    pub last_sequence: u64,
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

pub(super) fn write_capture(
    root: &Path,
    binding: &Capture,
    frames: Vec<(u64, MessageType, Vec<u8>)>,
) -> TestResult {
    let mut envelopes = Vec::with_capacity(frames.len());
    for (server_sequence, kind, payload) in frames {
        let required_type = if server_sequence == binding.result_sequence {
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
            command_id: binding.command_id,
            game_session_id: binding.game_session_id.clone(),
            payload,
            server_sequence,
            status: CommandStatus::Accepted as u32,
        });
    }
    std::fs::File::create_new(root.join("m02-binding.json"))?
        .write_all(&serde_json::to_vec(binding)?)?;
    serde_json::to_writer(
        std::fs::File::create_new(root.join("m02-envelope.json"))?,
        &envelopes,
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
    if binding.command_id == 0
        || binding.first_sequence >= binding.result_sequence
        || binding.result_sequence >= binding.last_sequence
        || !(1..=64).contains(&frames.len())
        || binding.last_sequence.checked_sub(binding.first_sequence)
            != Some(frames.len() as u64 - 1)
        || frames.iter().enumerate().any(|(index, frame)| {
            frame.game_session_id != binding.game_session_id
                || frame.command_id != binding.command_id
                || frame.status != CommandStatus::Accepted as u32
                || binding.first_sequence.checked_add(index as u64) != Some(frame.server_sequence)
        })
    {
        return Err("M02 invalid capture command, status, or sequence".into());
    }
    Ok(())
}

fn replay(binding: &Capture, frames: &[Envelope]) -> TestResult {
    let approach = ActorPosition {
        x: i32::from(APPROACH.x),
        y: i32::from(APPROACH.y),
        floor: 7,
    };
    let mut previous = approach;
    let mut steps = Vec::new();
    let mut gate = None;
    let mut spatial_revision = None;
    let mut map_revision = None;
    let fields = world_spatial_entities::EntityFields::selected(&binding.capabilities);
    let mut map_binding = world_map::MapViewBinding {
        content_generation: binding.content_generation,
        bundle_digest: binding.bundle_digest,
        reset_epoch: binding.reset_epoch,
        origin: approach,
        first_visible_floor: binding.first_visible_floor,
    };
    for frame in frames {
        if frame.server_sequence == binding.result_sequence {
            let result = decode_command_result(&frame.payload)?;
            let payload = if binding.capabilities.contains(&66) {
                let (payload, refusal) = refusal_detail::split_refusal_detail(result.payload)
                    .map_err(|error| format!("M02 replay refusal detail: {error:?}"))?;
                if refusal.is_some() {
                    return Err("M02 committed result contains refusal detail".into());
                }
                payload
            } else {
                result.payload
            };
            if result.command_id != binding.command_id
                || result.status != CommandStatus::Accepted
                || oteryn_protocol_oteryn::world_object::decode_use_result(payload)
                    .map_err(|error| format!("M02 replay USE result: {error:?}"))?
                    != UseDisposition::Committed
            {
                return Err("M02 replay did not receive the committed far-use result".into());
            }
            continue;
        }
        let delta = decode_state_delta(&frame.payload)?;
        match delta.domain_id {
            world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY => {
                if delta.delta_type != world_spatial_entities::DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2
                {
                    return Err("M02 replay expected negotiated entity movement".into());
                }
                if spatial_revision.is_some_and(|revision| revision != delta.base_revision)
                    || delta.new_revision <= delta.base_revision
                {
                    return Err("M02 replay movement revision mismatch".into());
                }
                spatial_revision = Some(delta.new_revision);
                let (movement, _, _) =
                    world_spatial_entities::decode_world_spatial_entities_delta_with_sidecar(
                        delta.payload,
                        fields,
                    )
                    .map_err(|error| format!("M02 replay movement: {error:?}"))?;
                if movement.content_generation != binding.content_generation {
                    return Err("M02 replay movement content generation mismatch".into());
                }
                if movement.actor_position != previous {
                    if frame.server_sequence > binding.result_sequence {
                        return Err("M02 replay moved after the USE result".into());
                    }
                    previous = movement.actor_position;
                    steps.push(previous);
                }
            }
            world_map::STATE_DOMAIN_WORLD_MAP_VIEW => {
                if delta.delta_type != world_map::DELTA_TYPE_WORLD_MAP_VIEW_DELTA_V1
                    || map_revision.is_some_and(|revision| revision != delta.base_revision)
                    || delta.new_revision <= delta.base_revision
                {
                    return Err("M02 replay map revision mismatch".into());
                }
                map_revision = Some(delta.new_revision);
                let change = world_map::decode_world_map_delta(&map_binding, delta.payload)
                    .map_err(|error| format!("M02 replay map delta: {error:?}"))?;
                map_binding = change.header;
                if change.cleared.contains(&GATE_POSITION) {
                    gate = None;
                }
                if let Some(tile) = change
                    .tiles
                    .into_iter()
                    .find(|tile| tile.position == GATE_POSITION)
                {
                    gate = Some(tile);
                }
            }
            _ => {}
        }
    }
    if !is_expected_walk(&steps)
        || !gate
            .as_ref()
            .is_some_and(|tile| gate_matches(tile, binding.ordinal, OPEN_GATE, 1))
    {
        return Err("M02 recorded deltas do not prove the exact walk and open gate".into());
    }
    Ok(())
}

#[test]
#[ignore = "requires the shared wrapper's fresh live capture"]
fn m02_replay_captured_autowalk() -> TestResult {
    let root = Path::new(&required("E2E_ALL_CAPTURE_DIR")?).to_path_buf();
    let expected = required("OTERYN_M02_REPLAY_IDENTITY")?;
    let binding: Capture =
        serde_json::from_reader(std::fs::File::open(root.join("m02-binding.json"))?)?;
    let frames: Vec<Envelope> =
        serde_json::from_reader(std::fs::File::open(root.join("m02-envelope.json"))?)?;
    verify_identity(&binding, &expected, &frames)?;
    replay(&binding, &frames)
}

#[test]
fn m02_requires_exact_capture_identity() -> TestResult {
    let binding = Capture {
        game_session_id: "01934f10-7c00-7000-8000-000000000001".into(),
        command_id: 7,
        first_sequence: 10,
        result_sequence: 11,
        last_sequence: 12,
        capabilities: vec![],
        content_generation: [0; 32],
        bundle_digest: [0; 32],
        reset_epoch: 0,
        first_visible_floor: 7,
        ordinal: 1,
    };
    let expected = binding.identity();
    let mut frames: Vec<_> = (10..=12)
        .map(|server_sequence| Envelope {
            command_id: 7,
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
