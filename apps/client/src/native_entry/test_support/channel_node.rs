//! chsw: a scripted node of one World with two Channels. It admits the client on the Channel the
//! Gateway named, selects capability 512 and answers 6251, 6247 and 6248 as the Game server does:
//! an accepted switch ends the session, and the next switch inside the cooldown is refused.
use oteryn_protocol_oteryn::channel_switch::{self as wire, COOLDOWN_MS, SwitchLock, SwitchStatus};
use oteryn_protocol_oteryn::{
    ChannelId, CommandStatus, DomainSnapshot, GameSessionId, MessageType, ServerAcceptedValue,
    WorldId, decode_wire_envelope, encode_command_result, encode_server_accepted,
    encode_single_chunk_snapshot,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) const CHANNEL_B: &str = "01890a5d-ac96-774b-bcce-b302099a805b";
/// Where the join snapshot places the character on either Channel.
pub(crate) const POSITION: [u8; 3] = [50, 60, 7];
/// Characters on each Channel besides the client, in Channel order.
pub(crate) const OTHERS: [u32; 2] = [3, 5];

/// The shared state of the scripted node: the Channel the Gateway admitted last and the switches
/// accepted so far (any switch starts the cooldown).
#[derive(Default)]
pub(crate) struct Node {
    pub(crate) login_channel: Mutex<String>,
    pub(crate) switches: AtomicUsize,
    pub(crate) refuse_destination: std::sync::atomic::AtomicBool,
    pub(crate) full: std::sync::atomic::AtomicBool,
}

pub(crate) fn uuid(text: &str) -> Option<[u8; 16]> {
    oteryn_platform_client::native_login::parse_uuid_v7(text)
}

fn join_frames(session: [u8; 16], channel: [u8; 16]) -> Option<Vec<Vec<u8>>> {
    let mut accepted = vec![
        encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&session).ok()?,
            world_id: WorldId::decode(&uuid(super::WORLD)?).ok()?,
            channel_id: ChannelId::decode(&channel).ok()?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 1,
            schema_revision: 1,
            selected_capabilities: &[wire::CAPABILITY_CHANNEL_SWITCH_V1],
        })
        .ok()?,
    ];
    let [x, y, floor] = POSITION;
    let mut spatial = vec![10, 32];
    spatial.extend([17; 32]);
    // Signed fields, zigzag encoded; each value fits one byte.
    spatial.extend([18, 6, 8, x * 2, 16, y * 2, 24, floor * 2]);
    let domains = [
        DomainSnapshot {
            domain_id: 1,
            revision: 1,
            snapshot_type: 1,
            payload: &spatial,
        },
        DomainSnapshot {
            domain_id: 2,
            revision: 1,
            snapshot_type: 1,
            payload: &[],
        },
    ];
    accepted.extend(encode_single_chunk_snapshot(1, 1, 1, &domains).ok()?);
    Some(accepted)
}

/// One admitted connection: join, then answer commands until the client leaves or a switch is
/// accepted.
pub(crate) async fn serve<S>(stream: &mut S, node: &Node, admission: usize) -> Option<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    oteryn_session::read_frame(stream).await.ok()?;
    let current = uuid(&node.login_channel.lock().ok()?.clone())?;
    if current == uuid(CHANNEL_B)? && node.refuse_destination.load(Ordering::SeqCst) {
        oteryn_session::write_frame(
            stream,
            &oteryn_protocol_oteryn::admission_queue::encode_channel_full(),
        )
        .await
        .ok()?;
        return Some(());
    }
    let mut session = uuid(super::CHARACTER)?;
    session[15] = u8::try_from(admission).ok()?;
    for frame in join_frames(session, current)? {
        oteryn_session::write_frame(stream, &frame).await.ok()?;
    }
    let channels = [uuid(super::CHANNEL)?, uuid(CHANNEL_B)?];
    // The join snapshot targets server sequence 1; every result takes the next one.
    let mut sequence = 2;
    loop {
        let frame = oteryn_session::read_frame(stream).await.ok()?;
        let envelope = decode_wire_envelope(&frame).ok()?;
        if envelope.message_type() != MessageType::ClientCommand {
            continue;
        }
        let command = envelope.client_command(1).ok()?;
        let cooling = node.switches.load(Ordering::SeqCst) > 0;
        let refused = |lock| {
            let cooldown_left_ms = if lock == SwitchLock::Cooldown {
                COOLDOWN_MS
            } else {
                0
            };
            (
                CommandStatus::Rejected,
                wire::encode_status(SwitchStatus {
                    lock,
                    cooldown_left_ms,
                }),
            )
        };
        let mut switched = false;
        let (status, payload) = match command.command_type {
            wire::COMMAND_TYPE_CHANNEL_LOAD => {
                let load: Vec<_> = channels
                    .iter()
                    .zip(OTHERS)
                    .map(|(channel, others)| (*channel, others + u32::from(*channel == current)))
                    .collect();
                (CommandStatus::Accepted, wire::encode_load(&load))
            }
            wire::COMMAND_TYPE_CHANNEL_SWITCH_STATUS if cooling => {
                let status = SwitchStatus {
                    lock: SwitchLock::Cooldown,
                    cooldown_left_ms: COOLDOWN_MS,
                };
                (CommandStatus::Accepted, wire::encode_status(status))
            }
            wire::COMMAND_TYPE_CHANNEL_SWITCH_STATUS => (
                CommandStatus::Accepted,
                wire::encode_status(SwitchStatus::READY),
            ),
            wire::COMMAND_TYPE_CHANNEL_SWITCH_REQUEST => {
                match wire::decode_request(command.payload) {
                    Some(target) if target == current => refused(SwitchLock::SameChannel),
                    Some(target) if !channels.contains(&target) => refused(SwitchLock::Destination),
                    Some(_) if node.full.load(Ordering::SeqCst) => refused(SwitchLock::ChannelFull),
                    Some(_) if cooling => refused(SwitchLock::Cooldown),
                    Some(_) => {
                        switched = true;
                        node.switches.fetch_add(1, Ordering::SeqCst);
                        (
                            CommandStatus::Accepted,
                            wire::encode_status(SwitchStatus::READY),
                        )
                    }
                    None => (CommandStatus::Rejected, Some(Vec::new())),
                }
            }
            _ => (CommandStatus::Rejected, Some(Vec::new())),
        };
        let reply =
            encode_command_result(1, sequence, command.command_id, status, &payload?).ok()?;
        sequence += 1;
        oteryn_session::write_frame(stream, &reply).await.ok()?;
        if switched {
            // The safe exit released the session: the server closes it after the result.
            return Some(());
        }
    }
}
