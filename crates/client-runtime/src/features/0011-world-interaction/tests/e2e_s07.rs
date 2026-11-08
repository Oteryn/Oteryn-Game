//! S07 replay coverage for the native bed result consumer.
//!
//! The production path is `world_interaction::send` with `Action::Bed` in
//! `crates/client-runtime/src/features/0011-world-interaction/protocol.rs`: it revalidates the
//! selected bed item against the session stores, sends BED_SLEEP_INTENT (capability 11,
//! command 31) through `Session::gated_command`, and projects the raw command result through
//! `world_object::decode_use_result_with_gate` into the popup string. A scripted peer admits a
//! real `Session` (with a ground-item entity carrying the bed handle), checks the exact intent
//! bytes it receives, and answers with the captured server bed payloads.

use super::*;
use oteryn_session::{
    ActorPosition, Admission, CLIENT_SUPPORTED_CAPABILITIES, CharacterId, CommandStatus, Session,
    read_frame, write_frame,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, DuplexStream};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const SPOT: ActorPosition = ActorPosition {
    x: 32_311,
    y: 32_804,
    floor: 7,
};
const SKILL: u8 = 6;

struct Replies {
    sleep: Vec<u8>,
    occupied: Vec<u8>,
    expected: String,
}

fn replies() -> Result<Replies, BoxError> {
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = std::path::Path::new(&directory);
        return Ok(Replies {
            sleep: std::fs::read(directory.join("s07-sleep.bin"))?,
            occupied: std::fs::read(directory.join("s07-occupied.bin"))?,
            expected: std::fs::read_to_string(directory.join("s07-expected.txt"))?,
        });
    }
    let encode = |disposition| {
        world_object::encode_use_result(disposition)
            .map_err(|error| format!("S07 fixture: {error:?}"))
    };
    Ok(Replies {
        sleep: encode(world_object::UseDisposition::OfflineTraining)?,
        occupied: encode(world_object::UseDisposition::BedOccupied)?,
        expected: "sleep=OfflineTraining\noccupied=BedOccupied\n".into(),
    })
}

/// The popup text a player must see for a server-observed disposition name.
fn popup(name: &str) -> Result<&'static str, BoxError> {
    match name {
        "OfflineTraining" => Ok("Bed: offline training started"),
        "BedOccupied" => Ok("Bed: this bed is occupied"),
        other => Err(format!("S07 expected disposition {other} has no pinned popup").into()),
    }
}

fn bed_command() -> Command {
    Command {
        target: Target::Item(ItemHandle::MIN),
        action: Action::Bed(SKILL),
        selection: Some(Selection {
            position: SPOT,
            binding: None,
        }),
    }
}

#[test]
fn e2e_s07_replays_server_bed_results_through_world_interaction_send() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .name("s07-bed-session".into())
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(exchange())
        })?
        .join()
        .map_err(|_| std::io::Error::other("S07 bed session test thread panicked"))?
}

async fn exchange() -> Result<(), BoxError> {
    let replies = replies()?;
    let expected = |key: &str| -> Result<&'static str, BoxError> {
        popup(
            replies
                .expected
                .lines()
                .find_map(|line| {
                    line.strip_prefix(key)
                        .and_then(|rest| rest.strip_prefix('='))
                })
                .ok_or_else(|| format!("S07 expected lacks {key}"))?,
        )
    };
    let intent = bed_payload(ItemHandle::MIN, SKILL)?;
    // Command ids 7.. : sleep, occupied, refused, malformed result, then sleep again.
    let script: Vec<(Vec<u8>, CommandStatus)> = vec![
        (replies.sleep.clone(), CommandStatus::Accepted),
        (replies.occupied.clone(), CommandStatus::Accepted),
        (Vec::new(), CommandStatus::Rejected),
        (vec![0xff, 0x01], CommandStatus::Accepted),
        (replies.sleep.clone(), CommandStatus::Accepted),
    ];
    let (client, mut server) = tokio::io::duplex(16 * 1024);
    let peer = tokio::spawn({
        let intent = intent.clone();
        async move {
            admit_peer(&mut server).await?;
            for (index, (reply, status)) in script.iter().enumerate() {
                let command_id = 7 + index as u64;
                let mut command = vec![8, command_id as u8];
                scalar(&mut command, 2, 31);
                bytes(&mut command, 4, &intent);
                assert_eq!(
                    read_frame(&mut server).await?,
                    envelope(7, 1, 0, &command),
                    "unexpected BED command at position {index}",
                );
                write_frame(
                    &mut server,
                    &answer(41 + index as u64, command_id, *status, reply),
                )
                .await?;
            }
            let mut extra = [0];
            assert_eq!(
                server.read(&mut extra).await?,
                0,
                "unexpected extra command"
            );
            Ok::<_, BoxError>(())
        }
    });
    let client = async {
        let mut session = Session::admit(
            client,
            Admission {
                appearance_items: None,
                schema_revision: 1,
                character_id: CharacterId::decode(&identity(4))?,
                admission_material: b"grant",
                client_build_id: "s07-bed-session-test",
                supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
                deadline: Duration::from_secs(5),
            },
        )
        .await?;
        assert!(session.is_selected(11));

        let command = bed_command();
        let slept = send(&mut session, &command).await?;
        let occupied = send(&mut session, &command).await?;
        assert_eq!(slept, expected("sleep")?);
        assert_eq!(occupied, expected("occupied")?);
        assert_eq!(send(&mut session, &command).await?, "Bed refused");

        // A malformed result is reported as unknown and leaves the session usable: the same
        // selection still resolves to the identical server text afterwards.
        assert_eq!(send(&mut session, &command).await?, "Bed: unknown result");
        assert_eq!(send(&mut session, &command).await?, slept);

        // A selection that no longer matches the entity store never reaches the wire.
        let mut moved = bed_command();
        moved.selection = Some(Selection {
            position: ActorPosition {
                x: SPOT.x + 1,
                ..SPOT
            },
            binding: None,
        });
        assert_eq!(send(&mut session, &moved).await?, "Target changed");
        drop(session);
        Ok::<_, BoxError>(())
    };
    let client = client.await;
    let peer = peer.await?;
    client?;
    peer
}

async fn admit_peer(stream: &mut DuplexStream) -> Result<(), BoxError> {
    let mut bootstrap = vec![8, 1, 16, 1, 24, 1];
    for &capability in CLIENT_SUPPORTED_CAPABILITIES {
        scalar(&mut bootstrap, 4, u64::from(capability));
    }
    bytes(&mut bootstrap, 5, b"grant");
    bytes(&mut bootstrap, 6, &identity(4));
    bytes(&mut bootstrap, 7, b"s07-bed-session-test");
    assert_eq!(read_frame(stream).await?, envelope(1, 0, 0, &bootstrap));

    let mut accepted = Vec::new();
    for marker in 1..=3 {
        bytes(&mut accepted, marker, &identity(marker as u8));
    }
    accepted.extend_from_slice(&[32, 1, 48, 7, 56, 1, 64, 1, 72, 1]);
    for capability in [4, 6, 11, 15, 18] {
        scalar(&mut accepted, 10, capability);
    }
    write_frame(stream, &envelope(2, 0, 0, &accepted)).await?;

    // Capability-6 snapshot (item handles under capability 4): the own player and the bed.
    let own_identity = identity(9);
    let at = (SPOT.x + 1, SPOT.y, 7);
    let mut entities = Vec::new();
    bytes(&mut entities, 1, &[0x11; 32]);
    bytes(&mut entities, 2, &position(at));
    bytes(&mut entities, 3, &own_identity);
    bytes(&mut entities, 4, &actor(1, own_identity, 1, at));
    let mut bed = Vec::new();
    scalar(&mut bed, 1, 4);
    bytes(&mut bed, 2, &identity(10));
    bytes(&mut bed, 4, &position((SPOT.x, SPOT.y, 7)));
    scalar(&mut bed, 8, 1);
    scalar(&mut bed, 9, 1);
    scalar(&mut bed, 10, ItemHandle::MIN.get());
    bytes(&mut entities, 4, &bed);
    // Capability 15 and 18 domains, encoded with the production codecs (empty views).
    use oteryn_session::{item_view, world_map};
    let inventory =
        item_view::encode_character_inventory(&item_view::CharacterInventory::default())
            .map_err(|error| format!("{error:?}"))?;
    let container = item_view::encode_open_container(&item_view::OpenContainer::default())
        .map_err(|error| format!("{error:?}"))?;
    let map = world_map::encode_world_map_snapshot(&world_map::WorldMapViewSnapshot {
        header: world_map::MapViewHeader {
            content_generation: [0x11; 32],
            bundle_digest: [2; 32],
            reset_epoch: 0,
            origin: ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
            first_visible_floor: 0,
        },
        tiles: vec![],
    })
    .map_err(|error| format!("{error:?}"))?;
    let mut body = Vec::new();
    for (domain, revision, snapshot_type, payload) in [
        (1, 5, 2, entities.as_slice()),
        (9, 0, 1, inventory.as_slice()),
        (11, 0, 1, container.as_slice()),
        (17, 0, 1, map.as_slice()),
    ] {
        let mut entry = vec![8, domain, 16, revision, 24, snapshot_type];
        bytes(&mut entry, 4, payload);
        bytes(&mut body, 1, &entry);
    }
    let mut begin = vec![8, 1, 16, 1];
    scalar(&mut begin, 3, body.len() as u64);
    scalar(&mut begin, 4, 40);
    let mut chunk = vec![8, 1];
    bytes(&mut chunk, 3, &body);
    for frame in [
        envelope(11, 1, 0, &begin),
        envelope(12, 1, 0, &chunk),
        envelope(13, 1, 0, &[8, 1]),
    ] {
        write_frame(stream, &frame).await?;
    }
    Ok(())
}

fn position((x, y, floor): (i32, i32, i32)) -> Vec<u8> {
    let mut output = Vec::new();
    for (field, value) in [(1, x), (2, y), (3, floor)] {
        scalar(
            &mut output,
            field,
            u64::from(((value << 1) ^ (value >> 31)) as u32),
        );
    }
    output
}

fn actor(kind: u64, identity: [u8; 16], generation: u64, at: (i32, i32, i32)) -> Vec<u8> {
    let mut output = Vec::new();
    scalar(&mut output, 1, kind);
    bytes(&mut output, 2, &identity);
    scalar(&mut output, 3, generation);
    bytes(&mut output, 4, &position(at));
    scalar(&mut output, 5, 3);
    scalar(&mut output, 6, 1);
    scalar(&mut output, 7, 100);
    output
}

fn answer(sequence: u64, command: u64, status: CommandStatus, payload: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    scalar(&mut result, 1, command);
    scalar(&mut result, 2, status as u64);
    bytes(&mut result, 5, payload);
    envelope(8, 1, sequence, &result)
}

fn identity(marker: u8) -> [u8; 16] {
    [
        1, 0x93, 0x4f, 0x10, 0x7c, 0, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}

fn envelope(kind: u64, generation: u64, sequence: u64, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();
    scalar(&mut frame, 1, kind);
    scalar(&mut frame, 2, generation);
    scalar(&mut frame, 3, sequence);
    bytes(&mut frame, 4, payload);
    frame
}

fn scalar(output: &mut Vec<u8>, field: u32, value: u64) {
    if value != 0 {
        varint(output, u64::from(field) << 3);
        varint(output, value);
    }
}

fn bytes(output: &mut Vec<u8>, field: u32, value: &[u8]) {
    varint(output, (u64::from(field) << 3) | 2);
    varint(output, value.len() as u64);
    output.extend_from_slice(value);
}

fn varint(output: &mut Vec<u8>, mut value: u64) {
    while value >= 128 {
        output.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}
