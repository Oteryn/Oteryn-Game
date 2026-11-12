//! I03 replay coverage for the native LOOK result consumer.
//!
//! The production path is `world_interaction::send` in
//! `crates/client-runtime/src/features/0011-world-interaction/protocol.rs`: it revalidates the
//! popup selection against the session stores, sends the LOOK intent (capability 28, command 30)
//! through `Session::gated_command`, and projects the raw command result through
//! `item_look::decode_item_look_result_for` into the popup string. A scripted peer admits a real
//! `Session` (with the captured entity view), checks the exact intent bytes it receives, and
//! answers with the captured server LOOK payloads.

use super::*;
use oteryn_session::{
    ActorPosition, Admission, CLIENT_SUPPORTED_CAPABILITIES, CharacterId, CommandStatus, Session,
    read_frame, write_frame,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, DuplexStream};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const SELF_TEXT: &str = "You see yourself (Level 8).";
const ITEM_TEXT: &str = "You see a backpack.\nIt weighs 18.00 oz.";
const CREATURE: oteryn_session::EntityRef = oteryn_session::EntityRef {
    identity: [0xb7; 16],
    generation: 4,
};
const SPOT: ActorPosition = ActorPosition {
    x: 32_370,
    y: 32_241,
    floor: 7,
};

struct Replies {
    own: Vec<u8>,
    item: Vec<u8>,
    refusal: Vec<u8>,
    expected: String,
}

fn replies() -> Result<Replies, BoxError> {
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = std::path::Path::new(&directory);
        return Ok(Replies {
            own: std::fs::read(directory.join("i03-self.bin"))?,
            item: std::fs::read(directory.join("i03-item.bin"))?,
            refusal: std::fs::read(directory.join("i03-refusal.bin"))?,
            expected: std::fs::read_to_string(directory.join("i03-expected.txt"))?,
        });
    }
    let selection = item_look::ItemLookSelection { extended: true };
    let encode = |result| {
        item_look::encode_item_look_result_for(&result, selection)
            .map_err(|error| format!("I03 fixture: {error:?}"))
    };
    Ok(Replies {
        own: encode(item_look::ItemLookResult::Text(SELF_TEXT.into()))?,
        item: encode(item_look::ItemLookResult::Text(ITEM_TEXT.into()))?,
        refusal: encode(item_look::ItemLookResult::Refused(
            item_look::ItemLookRefusal::NotAvailable,
        ))?,
        expected: format!("self={SELF_TEXT:?}\nitem={ITEM_TEXT:?}\nrefusal=Look refused\n"),
    })
}

fn look_command() -> Command {
    Command {
        target: Target::Creature(CREATURE),
        action: Action::Look,
        selection: Some(Selection {
            position: SPOT,
            binding: None,
        }),
    }
}

#[test]
fn e2e_i03_replays_server_look_results_through_world_interaction_send() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .name("i03-look-session".into())
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(exchange())
        })?
        .join()
        .map_err(|_| std::io::Error::other("I03 look session test thread panicked"))?
}

async fn exchange() -> Result<(), BoxError> {
    let replies = replies()?;
    let intent = look_payload(&look_command().target)?;
    // Command ids 7.. : own text, item text, refusal, malformed result, then own text again.
    let script: Vec<(Vec<u8>, CommandStatus)> = vec![
        (replies.own.clone(), CommandStatus::Accepted),
        (replies.item.clone(), CommandStatus::Accepted),
        (replies.refusal.clone(), CommandStatus::Accepted),
        (vec![0xff, 0x01], CommandStatus::Accepted),
        (replies.own.clone(), CommandStatus::Accepted),
    ];
    let (client, mut server) = tokio::io::duplex(16 * 1024);
    let peer = tokio::spawn({
        let intent = intent.clone();
        async move {
            admit_peer(&mut server).await?;
            for (index, (reply, status)) in script.iter().enumerate() {
                let command_id = 7 + index as u64;
                let mut command = vec![8, command_id as u8];
                scalar(&mut command, 2, 30);
                bytes(&mut command, 4, &intent);
                assert_eq!(
                    read_frame(&mut server).await?,
                    envelope(7, 1, 0, &command),
                    "unexpected LOOK command at position {index}",
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
                client_build_id: "i03-look-session-test",
                supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
                deadline: Duration::from_secs(5),
            },
        )
        .await?;
        assert!(session.is_selected(28));

        let command = look_command();
        let own = send(&mut session, &command).await?;
        let item = send(&mut session, &command).await?;
        let refusal = send(&mut session, &command).await?;
        assert_eq!(
            format!("self={own:?}\nitem={item:?}\nrefusal={refusal}\n"),
            replies.expected
        );
        assert_eq!(refusal, "Look refused");

        // A malformed LOOK result is reported as unknown and leaves the session usable: the
        // same selection still resolves to the identical server text afterwards.
        assert_eq!(send(&mut session, &command).await?, "Look: unknown result");
        assert_eq!(send(&mut session, &command).await?, own);

        // A selection that no longer matches the entity store never reaches the wire.
        let mut moved = look_command();
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

// Foundation wire fixtures keep this client test on its existing Session dependency; Session
// itself decodes and validates every admission, snapshot and result frame.
async fn admit_peer(stream: &mut DuplexStream) -> Result<(), BoxError> {
    let mut bootstrap = vec![8, 1, 16, 1, 24, 1];
    for &capability in CLIENT_SUPPORTED_CAPABILITIES {
        scalar(&mut bootstrap, 4, u64::from(capability));
    }
    bytes(&mut bootstrap, 5, b"grant");
    bytes(&mut bootstrap, 6, &identity(4));
    bytes(&mut bootstrap, 7, b"i03-look-session-test");
    assert_eq!(read_frame(stream).await?, envelope(1, 0, 0, &bootstrap));

    let mut accepted = Vec::new();
    for marker in 1..=3 {
        bytes(&mut accepted, marker, &identity(marker as u8));
    }
    accepted.extend_from_slice(&[32, 1, 48, 7, 56, 1, 64, 1, 72, 1]);
    scalar(&mut accepted, 10, 6);
    scalar(&mut accepted, 10, 28);
    write_frame(stream, &envelope(2, 0, 0, &accepted)).await?;

    // Capability-6 snapshot: the own player and one other creature at SPOT.
    let own_identity = identity(9);
    let mut entities = Vec::new();
    bytes(&mut entities, 1, &[0x11; 32]);
    bytes(&mut entities, 2, &position((32_369, 32_241, 7)));
    bytes(&mut entities, 3, &own_identity);
    bytes(
        &mut entities,
        4,
        &actor(1, own_identity, 1, (32_369, 32_241, 7)),
    );
    bytes(
        &mut entities,
        4,
        &actor(
            2,
            CREATURE.identity,
            CREATURE.generation,
            (SPOT.x, SPOT.y, 7),
        ),
    );
    let overlay: &[u8] = &[];
    let mut body = Vec::new();
    for (domain, revision, snapshot_type, payload) in
        [(1, 5, 2, entities.as_slice()), (2, 2, 1, overlay)]
    {
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
