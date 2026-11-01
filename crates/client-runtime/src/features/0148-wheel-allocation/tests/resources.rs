//! Real admission and command exchange for the optional live resource projection.
use super::{WheelCommand, atelier, send, wheel};
use oteryn_session::{
    Admission, CLIENT_SUPPORTED_CAPABILITIES, CharacterId, CommandStatus, Session, read_frame,
    write_frame,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, DuplexStream};

type BoxError = Box<dyn std::error::Error + Send + Sync>;
const DEADLINE: Duration = Duration::from_secs(5);

#[test]
fn unselected_resources_preserve_the_three_command_exchange() -> Result<(), BoxError> {
    run(false, false, false)
}

#[test]
fn selected_resources_follow_the_operation_and_both_current_queries() -> Result<(), BoxError> {
    run(true, false, false)
}

#[test]
fn delayed_duplicate_receipt_keeps_the_latest_resource_projection() -> Result<(), BoxError> {
    run(true, true, false)
}

#[test]
fn rejected_resource_query_preserves_the_operation_and_current_views() -> Result<(), BoxError> {
    run(true, false, true)
}

fn run(selected: bool, replay: bool, reject_resources: bool) -> Result<(), BoxError> {
    std::thread::Builder::new()
        .name("atelier-resource-session".into())
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(exchange(selected, replay, reject_resources))
        })?
        .join()
        .map_err(|_| std::io::Error::other("Atelier session test thread panicked"))?
}

async fn exchange(selected: bool, replay: bool, reject_resources: bool) -> Result<(), BoxError> {
    let request = WheelCommand::new(
        atelier::COMMAND_TYPE_ATELIER_MUTATE,
        atelier::encode_mutate(&atelier::MutateRequest {
            occurrence: identity(7),
            expected_revision: 9,
            mutation: atelier::Mutation::Reveal { quality: 0 },
        })
        .map_err(|error| format!("mutation fixture: {error:?}"))?,
    )
    .ok_or("valid Atelier mutation was refused")?;
    let receipt = atelier_reply(10)?;
    let current_atelier = atelier_reply(11)?;
    let current_wheel = wheel::encode_wheel_reply(&wheel::WheelAllocationReply {
        refused: false,
        revision: 2,
        points: [0; wheel::WHEEL_SLOTS],
        vessels: None,
    })
    .map_err(|error| format!("Wheel fixture: {error:?}"))?;
    let balances = atelier::resources::ResourceBalances {
        bank_gold: 123_456_789,
        carried_gold: 12_340,
        lesser_gems: 2,
        regular_gems: 3,
        greater_gems: 4,
        basic_fragments: 27,
        supreme_fragments: 9,
    };
    let current_resources = atelier::resources::encode_reply(&balances);
    let mut script = vec![
        (
            request.kind,
            request.payload.as_slice(),
            CommandStatus::Accepted,
            receipt.as_slice(),
        ),
        (
            wheel::COMMAND_TYPE_WHEEL_ALLOCATION,
            &[8, 0][..],
            CommandStatus::Accepted,
            current_wheel.as_slice(),
        ),
        (
            atelier::COMMAND_TYPE_ATELIER_QUERY,
            &[][..],
            CommandStatus::Accepted,
            current_atelier.as_slice(),
        ),
    ];
    if selected {
        script.push((
            atelier::resources::COMMAND_TYPE_ATELIER_RESOURCES_QUERY,
            &[],
            if reject_resources {
                CommandStatus::Rejected
            } else {
                CommandStatus::Accepted
            },
            if reject_resources {
                &[]
            } else {
                &current_resources
            },
        ));
    }
    let command_count = script.len() as u64;
    let mut sequence = 40;
    let mut frames = Vec::with_capacity(script.len());
    for (index, &(kind, payload, status, reply)) in script.iter().enumerate() {
        let command_id = 7 + index as u64;
        let mut command = vec![8, command_id as u8];
        scalar(&mut command, 2, u64::from(kind));
        bytes(&mut command, 4, payload);
        let mut answers = Vec::new();
        if replay && index == 1 {
            // Session accepts replay only for an earlier sent CommandId. A duplicate
            // of the current command would correctly fail the transport contract.
            sequence += 1;
            answers.push(answer(
                sequence,
                7,
                CommandStatus::DuplicateReplay,
                &receipt,
            ));
        }
        sequence += 1;
        answers.push(answer(sequence, command_id, status, reply));
        frames.push((envelope(7, 1, 0, &command), answers));
    }
    let (client, mut server) = tokio::io::duplex(16 * 1024);
    let peer = tokio::spawn(async move {
        admit_peer(&mut server, selected).await?;
        for (index, (command, answers)) in frames.iter().enumerate() {
            assert_eq!(
                read_frame(&mut server).await?,
                *command,
                "unexpected command at position {index}",
            );
            for frame in answers {
                write_frame(&mut server, frame).await?;
            }
        }
        let mut extra = [0];
        assert_eq!(
            server.read(&mut extra).await?,
            0,
            "unexpected extra command"
        );
        Ok::<_, BoxError>(())
    });
    let client = async {
        let mut session = Session::admit(
            client,
            Admission {
                appearance_items: None,
                schema_revision: 1,
                character_id: CharacterId::decode(&identity(4))?,
                admission_material: b"grant",
                client_build_id: "atelier-resources-session-test",
                supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
                deadline: DEADLINE,
            },
        )
        .await?;
        assert!(session.is_selected(wheel::CAPABILITY_WHEEL_ALLOCATION_V1));
        assert_eq!(
            session.is_selected(atelier::resources::CAPABILITY_ATELIER_RESOURCES_V1),
            selected
        );
        let response = send(&mut session, &request).await?;
        assert_eq!(response.kind, request.kind);
        assert_eq!(response.status, CommandStatus::Accepted);
        assert_eq!(response.payload, receipt);
        assert_eq!(response.wheel.as_deref(), Some(current_wheel.as_slice()));
        assert_eq!(
            response.atelier.as_deref(),
            Some(current_atelier.as_slice())
        );
        if selected && !reject_resources {
            assert_eq!(
                response.resources.as_deref(),
                Some(current_resources.as_slice())
            );
            assert_eq!(
                atelier::resources::decode_reply(
                    response.resources.as_deref().ok_or("missing resources")?
                )
                .map_err(|error| format!("live balances: {error:?}"))?,
                balances,
            );
        } else {
            assert_eq!(response.resources, None);
        }
        assert_eq!(session.next_command_id(), 7 + command_count);
        assert_eq!(
            session.last_server_sequence(),
            40 + command_count + u64::from(replay)
        );
        let duplicates = session.take_duplicate_outcomes();
        assert_eq!(duplicates.len(), usize::from(replay));
        if replay {
            let duplicate = duplicates.first().ok_or("missing delayed duplicate")?;
            assert_eq!(duplicate.command_id, 7);
            assert_eq!(duplicate.status, CommandStatus::DuplicateReplay);
            assert_eq!(duplicate.server_sequence, 42);
            assert_eq!(duplicate.payload, receipt);
        }
        // EOF lets the peer prove that no unnegotiated or extra command was sent.
        drop(session);
        Ok::<_, BoxError>(())
    };
    let client = client.await;
    // Admission and each command have deadlines. Dropping their failed Session
    // closes the peer stream; always join it before propagating either error.
    let peer = peer.await?;
    client?;
    peer
}

fn atelier_reply(revision: u64) -> Result<Vec<u8>, BoxError> {
    atelier::encode_reply(&atelier::AtelierReply {
        refused: false,
        revision,
        gems: Vec::new(),
        grades: Vec::new(),
        quality_quotes: Vec::new(),
        grade_quotes: Vec::new(),
    })
    .map_err(|error| format!("Atelier fixture: {error:?}").into())
}

// Foundation wire fixtures keep this client test on its existing Session dependency.
// Session itself decodes and validates every admission, snapshot and result frame.
async fn admit_peer(stream: &mut DuplexStream, selected: bool) -> Result<(), BoxError> {
    let mut bootstrap = vec![8, 1, 16, 1, 24, 1];
    for &capability in CLIENT_SUPPORTED_CAPABILITIES {
        scalar(&mut bootstrap, 4, u64::from(capability));
    }
    bytes(&mut bootstrap, 5, b"grant");
    bytes(&mut bootstrap, 6, &identity(4));
    bytes(&mut bootstrap, 7, b"atelier-resources-session-test");
    assert_eq!(read_frame(stream).await?, envelope(1, 0, 0, &bootstrap));

    let mut accepted = Vec::new();
    for marker in 1..=3 {
        bytes(&mut accepted, marker, &identity(marker as u8));
    }
    accepted.extend_from_slice(&[32, 1, 48, 7, 56, 1, 64, 1, 72, 1]);
    scalar(
        &mut accepted,
        10,
        u64::from(wheel::CAPABILITY_WHEEL_ALLOCATION_V1),
    );
    if selected {
        scalar(
            &mut accepted,
            10,
            u64::from(atelier::resources::CAPABILITY_ATELIER_RESOURCES_V1),
        );
    }
    write_frame(stream, &envelope(2, 0, 0, &accepted)).await?;

    let mut spatial = Vec::new();
    bytes(&mut spatial, 1, &[0x11; 32]);
    bytes(&mut spatial, 2, &[]);
    let mut body = Vec::new();
    for (domain, revision, payload) in [(1, 5, spatial.as_slice()), (2, 2, &[][..])] {
        let mut entry = vec![8, domain, 16, revision, 24, 1];
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
